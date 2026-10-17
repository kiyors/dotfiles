//! Planning and execution of the Nix garbage-collection policy.
//!
//! The plan is built as pure data first (`plan`) and only then executed. That
//! separation is what makes the safety properties testable: a dry run can never
//! produce a live destructive command, and that is asserted directly.

use crate::cli::CleanupArgs;
use crate::deep;
use crate::notify;
use crate::reap;
use crate::tmp;
use crate::ui;
use colored::Colorize;

/// A child process that cleanup intends to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl PlannedCommand {
    fn new(program: &str, args: &[impl AsRef<str>]) -> Self {
        Self {
            program: program.to_string(),
            args: args.iter().map(|arg| arg.as_ref().to_string()).collect(),
        }
    }
}

/// Build the full command plan for this invocation without running anything.
///
/// `--deep` is a full reclaim: caches *and* project build artifacts, via
/// `mo purge`. Discarding every old Nix generation stays behind `-d` because it
/// is the one step that removes rollback history system-wide.
pub fn plan(args: &CleanupArgs) -> Vec<PlannedCommand> {
    let mut nh_args = vec![
        "clean".to_string(),
        "all".to_string(),
        "--keep".to_string(),
        args.effective_keep_generations().to_string(),
        "--keep-since".to_string(),
        args.effective_keep_since().to_string(),
    ];

    if args.optimise() {
        nh_args.push("--optimise".into());
    }
    if !args.clean_gcroots() {
        nh_args.push("--no-gcroots".into());
        nh_args.push("--no-direnv".into());
    }
    if args.dry_run {
        nh_args.push("--dry".into());
    }
    if args.quiet {
        nh_args.push("--quiet".into());
    }

    let mut commands = vec![PlannedCommand::new("nh", &nh_args)];

    if args.deep() {
        let mo_args: &[&str] = if args.dry_run {
            &["clean", "--dry-run"]
        } else {
            &["clean"]
        };
        commands.push(PlannedCommand::new("mo", mo_args));
    }

    if args.purge() {
        let purge_args: &[&str] = if args.dry_run {
            &["purge", "--dry-run"]
        } else {
            &["purge", "--yes"]
        };
        commands.push(PlannedCommand::new("mo", purge_args));
    }

    if args.gc() {
        let gc_args: &[&str] = if args.dry_run {
            &["nix-collect-garbage", "-d", "--dry-run"]
        } else {
            &["nix-collect-garbage", "-d"]
        };
        commands.push(PlannedCommand::new("sudo", gc_args));
    }

    // `nh --optimise` above only flags the store for deduplication; the actual
    // pass needs root. Never schedule it for real during a preview.
    if args.optimise() && !args.dry_run {
        commands.push(PlannedCommand::new("sudo", &["nix-store", "--optimise"]));
    }

    commands
}

/// What the run actually removed, for the closing summary.
#[derive(Debug, Default)]
struct CleanupStats {
    /// Bytes freed by `coffee` itself, measured before each deletion.
    direct_bytes: u64,
    tmp_items: usize,
    dev_cache_bytes: u64,
    /// Free space across all writable volumes when the run started.
    free_before: u64,
    /// Free space once the run finished, including any child tool's effect.
    free_after: u64,
}

impl CleanupStats {
    /// Net change in free space, clamped at zero.
    fn freed(&self) -> u64 {
        self.free_after.saturating_sub(self.free_before)
    }
}

/// Print the closing summary: what was removed and how much space it freed.
///
/// A real before/after free-space reading is used so the number accounts for
/// `nh`, `mo` and `brew` as well, which this tool does not delete itself.
fn print_stats(args: &CleanupArgs, stats: &CleanupStats) {
    if args.quiet || args.no_stats {
        return;
    }

    ui::rule();
    ui::heading(if args.dry_run {
        "📊 What a real run would free"
    } else {
        "📊 What was freed"
    });

    let action = if args.dry_run { "would free" } else { "freed" };

    let mut rows: Vec<Vec<String>> = Vec::new();

    if stats.tmp_items > 0 || stats.direct_bytes > 0 {
        rows.push(vec![
            format!("/tmp clutter ({} item(s))", stats.tmp_items),
            ui::human_bytes(stats.direct_bytes),
        ]);
    }

    if stats.dev_cache_bytes > 0 {
        rows.push(vec![
            "Gradle / Xcode caches".to_string(),
            ui::human_bytes(stats.dev_cache_bytes),
        ]);
    }

    // The headline number: measured, and covering every child command.
    // Only meaningful after a real run — in a preview the free-space delta is
    // just whatever else happened to touch the disk while scanning.
    if !args.dry_run {
        rows.push(vec![
            format!("Disk free space {action} in total"),
            ui::human_bytes(stats.freed()),
        ]);
    }

    ui::table(
        &["Category", "Size"],
        &rows,
        &[ui::Align::Left, ui::Align::Right],
    );

    if args.dry_run {
        println!("{}", "  Preview only — nothing was deleted.".magenta());
    } else if stats.freed() > 0 {
        ui::success(format!(
            "✨ Reclaimed {} of disk space",
            ui::human_bytes(stats.freed())
        ));
    } else {
        println!(
            "{}",
            "  No measurable change (already clean, or another process reclaimed it).".dimmed()
        );
    }

    if !args.dry_run {
        println!(
            "{}",
            "  Run `coffee space` to see what is still using space.".dimmed()
        );
    }
}
///
/// Returns false if validation fails or any child command fails, so launchd
/// logs expose partial failures.
pub fn run(args: &CleanupArgs) -> bool {
    if let Err(error) = args.validate() {
        ui::fatal(error);
        return false;
    }

    if !args.quiet {
        ui::heading(if args.dry_run {
            "🧹 Previewing system cleanup..."
        } else {
            "🧹 Running system cleanup..."
        });
        ui::rule();
    }

    // Say what is about to become unrecoverable *before* prompting for sudo,
    // not after the fact.
    if (args.deep || args.purge_projects) && !args.dry_run {
        ui::warn(
            "`mo purge --yes` runs unattended, which skips the interactive review \
             that normally protects artifacts used in the last 7 days; preview with \
             `co clean --deep --dry-run`",
        );
    }
    if args.gc() && !args.dry_run {
        ui::warn(
            "`nix-collect-garbage -d` deletes every old Nix generation system-wide; \
             rolling back to a previous system or profile will no longer be possible",
        );
    }

    let commands = plan(args);
    let mut succeeded = true;

    // Captured before anything runs, so the closing summary measures the whole
    // run rather than only what this tool deleted by hand.
    let mut stats = CleanupStats {
        free_before: crate::space::total_free(),
        ..Default::default()
    };

    for command in &commands {
        if !args.quiet {
            ui::step(&command.program, &command.args);
        }

        match std::process::Command::new(&command.program)
            .args(&command.args)
            .status()
        {
            Ok(status) if status.success() => {
                if !args.quiet {
                    ui::ok(format!(
                        "Successfully executed {} {:?}",
                        command.program, command.args
                    ));
                }
            }
            Ok(status) => {
                succeeded = false;
                eprintln!(
                    "  {} Command {} {:?} exited with status: {}",
                    "❌".red(),
                    command.program.bold(),
                    command.args,
                    status.to_string().red()
                );
            }
            Err(error) => {
                succeeded = false;
                eprintln!(
                    "  {} Failed to execute {} {:?}: {}",
                    "💥".red().bold(),
                    command.program.bold(),
                    command.args,
                    error.to_string().red()
                );
            }
        }
    }

    // Call out anything a dry run deliberately withheld.
    if args.optimise() && args.dry_run && !args.quiet {
        ui::step("sudo", &["nix-store".into(), "--optimise".into()]);
        ui::dry_run("Would execute sudo nix-store --optimise");
    }

    if args.deep() && succeeded {
        if !args.quiet {
            ui::rule();
            ui::heading(if args.dry_run {
                "🧹 Previewing deep developer caches cleanup..."
            } else {
                "🧹 Cleaning deep developer caches..."
            });
            // `mo clean` empties the Trash unless it is whitelisted, which is
            // the one irreversible thing in this phase.
            ui::warn(
                "`mo clean` also empties the Trash unless it is protected — \
                 add it via `mo clean --whitelist`",
            );
        }

        // One process snapshot for the whole phase. Sampling per cache would
        // walk the entire process table once per cache, and this is the slowest
        // part of a deep clean.
        let system = reap::sample_processes();
        stats.dev_cache_bytes += deep::clean_dev_caches(args.dry_run, args.quiet, &system);
        deep::brew_cleanup(args.dry_run, args.quiet);
        deep::flush_macos_caches(args.dry_run, args.quiet);
    }

    if (args.deep() || args.clean_tmp()) && succeeded {
        if !args.quiet {
            ui::rule();
            ui::heading(if args.dry_run {
                "🧹 Previewing /tmp cleanup..."
            } else {
                "🧹 Cleaning /tmp junk..."
            });
        }

        let tmp_stats = tmp::clean(args.dry_run, args.quiet, args.tmp_min_age);
        stats.tmp_items = tmp_stats.removed + tmp_stats.skipped;
        stats.direct_bytes += tmp_stats.bytes;
    }

    if args.all && !args.quiet {
        ui::warn(
            "`--all` removed every Nix generation and every project build \
             artifact. There is no undo beyond what `co space` can re-scan.",
        );
    }

    stats.free_after = crate::space::total_free();

    if !args.quiet {
        ui::rule();
    }

    if succeeded && !args.quiet {
        ui::success(if args.dry_run {
            "✨ System cleanup preview complete."
        } else {
            "✨ System cleanup complete."
        });
    } else if !succeeded {
        eprintln!("{}", "System cleanup completed with errors.".red().bold());
    }

    print_stats(args, &stats);

    if succeeded && !args.dry_run && !args.no_notify {
        notify::send(
            "System Maintainer",
            if args.deep {
                "Deep system cleanup complete 🧹"
            } else {
                "System cleanup complete 🧹"
            },
        );
    }

    succeeded
}

#[cfg(test)]
mod tests {
    use super::{CleanupStats, PlannedCommand, plan};
    use crate::cli::CleanupArgs;

    fn args() -> CleanupArgs {
        CleanupArgs {
            keep_generations: 5,
            keep_since: "14d".into(),
            deep: false,
            purge_projects: false,
            collect_garbage: false,
            clean_tmp: false,
            clean_gcroots: false,
            optimise: false,
            dry_run: false,
            quiet: false,
            no_stats: true,
            tmp_min_age: 86_400,
            all: false,
            no_notify: true,
        }
    }

    #[test]
    fn conservative_cleanup_is_the_default_shape() {
        let commands = plan(&args());
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].program, "nh");
        assert_eq!(
            commands[0].args,
            [
                "clean",
                "all",
                "--keep",
                "5",
                "--keep-since",
                "14d",
                "--no-gcroots",
                "--no-direnv"
            ]
        );
    }

    #[test]
    fn quiet_is_forwarded_to_nh() {
        let mut a = args();
        a.quiet = true;
        assert!(plan(&a)[0].args.contains(&"--quiet".to_string()));
    }

    #[test]
    fn gcroots_are_preserved_by_default_and_can_be_requested() {
        let default = plan(&args());
        assert!(default[0].args.contains(&"--no-gcroots".to_string()));
        assert!(default[0].args.contains(&"--no-direnv".to_string()));

        let mut a = args();
        a.clean_gcroots = true;
        let commands = plan(&a);
        assert!(!commands[0].args.contains(&"--no-gcroots".to_string()));
        assert!(!commands[0].args.contains(&"--no-direnv".to_string()));
    }

    #[test]
    fn keep_since_is_trimmed_before_being_forwarded() {
        let mut a = args();
        a.keep_since = "  14d ".into();
        let commands = plan(&a);
        let value = commands[0]
            .args
            .iter()
            .position(|a| a == "--keep-since")
            .expect("--keep-since present");
        assert_eq!(commands[0].args[value + 1], "14d");
    }

    #[test]
    fn deep_dry_run_never_runs_a_live_cache_cleanup() {
        let mut a = args();
        a.deep = true;
        a.purge_projects = true;
        a.collect_garbage = true;
        a.optimise = true;
        a.dry_run = true;

        let commands = plan(&a);
        assert!(commands[0].args.contains(&"--optimise".to_string()));
        assert!(commands[0].args.contains(&"--dry".to_string()));
        assert_eq!(
            commands[1],
            PlannedCommand::new("mo", &["clean", "--dry-run"])
        );
        assert_eq!(
            commands[2],
            PlannedCommand::new("mo", &["purge", "--dry-run"])
        );
        assert_eq!(
            commands[3],
            PlannedCommand::new("sudo", &["nix-collect-garbage", "-d", "--dry-run"])
        );

        // `nix-store --optimise` must never appear as a live sudo command.
        assert!(
            !commands
                .iter()
                .any(|c| c.program == "sudo" && c.args.contains(&"--optimise".to_string())),
            "dry run must not schedule a live store optimisation"
        );
    }

    #[test]
    fn live_deep_cleanup_includes_mo_purge_yes_and_sudo_nix_commands() {
        let mut a = args();
        a.deep = true;
        a.purge_projects = true;
        a.collect_garbage = true;
        a.optimise = true;

        let commands = plan(&a);
        assert_eq!(commands[1], PlannedCommand::new("mo", &["clean"]));
        assert_eq!(commands[2], PlannedCommand::new("mo", &["purge", "--yes"]));
        assert_eq!(
            commands[3],
            PlannedCommand::new("sudo", &["nix-collect-garbage", "-d"])
        );
        assert_eq!(
            commands[4],
            PlannedCommand::new("sudo", &["nix-store", "--optimise"])
        );
    }

    #[test]
    fn deep_reclaims_caches_and_project_artifacts_but_never_collects_garbage() {
        // `--deep` is a full reclaim, so it runs `mo purge`. Discarding Nix
        // generations stays behind `-d`, because that is the one step that
        // removes rollback history system-wide.
        let mut a = args();
        a.deep = true;

        let commands = plan(&a);
        assert_eq!(commands.len(), 3, "unexpected plan: {commands:?}");
        assert_eq!(
            commands[1],
            PlannedCommand::new("mo", &["clean"]),
            "--deep should run mo clean"
        );
        assert_eq!(
            commands[2],
            PlannedCommand::new("mo", &["purge", "--yes"]),
            "--deep should run mo purge"
        );

        assert!(
            !commands
                .iter()
                .any(|c| c.args.iter().any(|a| a.contains("garbage"))),
            "--deep must never collect garbage: {commands:?}"
        );
        assert!(
            !commands
                .iter()
                .any(|c| c.program == "sudo" && c.args.contains(&"--optimise".to_string())),
            "--deep alone must not schedule a root store optimisation"
        );
    }

    #[test]
    fn garbage_collection_needs_its_own_flag() {
        let mut a = args();
        a.collect_garbage = true;

        let commands = plan(&a);
        assert_eq!(
            commands[1],
            PlannedCommand::new("sudo", &["nix-collect-garbage", "-d"])
        );
    }

    #[test]
    fn purge_projects_alone_does_not_run_mo_clean_or_garbage_collection() {
        let mut a = args();
        a.purge_projects = true;

        let commands = plan(&a);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[1], PlannedCommand::new("mo", &["purge", "--yes"]));
    }

    #[test]
    fn optimise_alone_schedules_store_deduplication() {
        let mut a = args();
        a.optimise = true;

        let commands = plan(&a);
        assert!(commands[0].args.contains(&"--optimise".to_string()));
        assert_eq!(
            commands[1],
            PlannedCommand::new("sudo", &["nix-store", "--optimise"])
        );
    }

    #[test]
    fn freed_never_reports_a_negative_change() {
        // Another process writing to disk mid-run must not make the summary
        // claim negative reclaim.
        let stats = CleanupStats {
            direct_bytes: 100,
            tmp_items: 2,
            dev_cache_bytes: 0,
            free_before: 10_000,
            free_after: 9_000,
        };
        assert_eq!(stats.freed(), 0);
    }

    #[test]
    fn freed_reports_the_measured_increase() {
        let stats = CleanupStats {
            direct_bytes: 0,
            tmp_items: 0,
            dev_cache_bytes: 0,
            free_before: 1_000,
            free_after: 4_500,
        };
        assert_eq!(stats.freed(), 3_500);
    }

    #[test]
    fn invalid_arguments_are_rejected_before_anything_runs() {
        let mut zero_generations = args();
        zero_generations.keep_generations = 0;
        assert!(zero_generations.validate().is_err());

        let mut empty_since = args();
        empty_since.keep_since = "   ".into();
        assert!(empty_since.validate().is_err());

        assert!(args().validate().is_ok());
    }
}
