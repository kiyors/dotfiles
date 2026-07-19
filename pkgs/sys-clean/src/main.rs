use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use colored::*;
use std::process::{Command, ExitCode};
use std::time::Duration;
use sysinfo::{Pid, ProcessesToUpdate, Signal, System};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Reap stale, CPU-heavy orphaned developer processes
    ReapZombies {
        /// Command-line fragments to match (can be provided multiple times)
        #[arg(
            short,
            long = "target",
            visible_alias = "targets",
            default_value = "jest-worker"
        )]
        targets: Vec<String>,

        /// Minimum process age in seconds
        #[arg(long, default_value_t = 300)]
        min_age: u64,

        /// Minimum CPU usage percentage
        #[arg(long, default_value_t = 10.0)]
        min_cpu: f32,

        /// Seconds to wait after SIGTERM before sending SIGKILL
        #[arg(long, default_value_t = 2)]
        grace_period: u64,

        /// Only print matching processes; do not send signals
        #[arg(long, default_value_t = false)]
        dry_run: bool,

        /// Suppress start, summary, and no-match output
        #[arg(short, long, default_value_t = false)]
        quiet: bool,

        /// Do not send a macOS notification after reaping processes
        #[arg(long, default_value_t = false)]
        no_notify: bool,
    },
    /// Clean old Nix generations, with optional deep cache cleanup
    SystemCleanup {
        /// Number of Nix generations to retain
        #[arg(long, default_value_t = 5)]
        keep_generations: u32,

        /// Also retain Nix generations newer than this duration (for example, 14d)
        #[arg(long, default_value = "14d")]
        keep_since: String,

        /// Also remove caches, logs, temporary files, and application leftovers
        #[arg(long, default_value_t = false)]
        deep: bool,

        /// Also remove stale project and direnv GC roots
        #[arg(long, default_value_t = false)]
        clean_gcroots: bool,

        /// Deduplicate the Nix store after garbage collection (can be slow)
        #[arg(long, default_value_t = false)]
        optimise: bool,

        /// Preview cleanup without changing anything
        #[arg(long, default_value_t = false)]
        dry_run: bool,

        /// Suppress routine progress output
        #[arg(short, long, default_value_t = false)]
        quiet: bool,
    },
    /// Generate shell completions
    GenerateCompletion {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Debug)]
struct Candidate {
    pid: Pid,
    start_time: u64,
    name: String,
    command: String,
    cpu_usage: f32,
    age: u64,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let succeeded = match &cli.command {
        Commands::ReapZombies {
            targets,
            min_age,
            min_cpu,
            grace_period,
            dry_run,
            quiet,
            no_notify,
        } => reap_zombies(
            targets,
            *min_age,
            *min_cpu,
            *grace_period,
            *dry_run,
            *quiet,
            *no_notify,
        ),
        Commands::SystemCleanup {
            keep_generations,
            keep_since,
            deep,
            clean_gcroots,
            optimise,
            dry_run,
            quiet,
        } => system_cleanup(
            *keep_generations,
            keep_since,
            *deep,
            *clean_gcroots,
            *optimise,
            *dry_run,
            *quiet,
        ),
        Commands::GenerateCompletion { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(*shell, &mut cmd, name, &mut std::io::stdout());
            true
        }
    };

    if succeeded {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn send_macos_notification(title: &str, message: &str) {
    // Pass values as argv instead of interpolating them into AppleScript source.
    let script = r#"on run argv
display notification (item 2 of argv) with title (item 1 of argv)
end run"#;
    let _ = Command::new("osascript")
        .args(["-e", script, title, message])
        .status();
}

fn command_matches(command: &str, targets: &[String]) -> bool {
    let command = command.to_lowercase();
    targets
        .iter()
        .map(|target| target.trim().to_lowercase())
        .any(|target| !target.is_empty() && command.contains(&target))
}

#[allow(clippy::too_many_arguments)]
fn reap_zombies(
    targets: &[String],
    min_age: u64,
    min_cpu: f32,
    grace_period: u64,
    dry_run: bool,
    quiet: bool,
    no_notify: bool,
) -> bool {
    if !min_cpu.is_finite() || min_cpu < 0.0 {
        eprintln!("{} --min-cpu must be a non-negative number", "❌".red());
        return false;
    }

    if !quiet {
        println!(
            "{}",
            "🧟 Scanning for stale orphaned processes...".cyan().bold()
        );
    }

    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    // Process CPU usage is a delta and needs two samples.
    std::thread::sleep(Duration::from_millis(500));
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let candidates: Vec<Candidate> = sys
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let cpu_usage = process.cpu_usage();
            let age = process.run_time();
            let is_orphan = process.parent().is_some_and(|parent| parent.as_u32() == 1);

            if !(is_orphan && age >= min_age && cpu_usage >= min_cpu) {
                return None;
            }

            let command = process
                .cmd()
                .iter()
                .map(|part| part.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");

            if !command_matches(&command, targets) {
                return None;
            }

            Some(Candidate {
                pid: *pid,
                start_time: process.start_time(),
                name: process.name().to_string_lossy().into_owned(),
                command,
                cpu_usage,
                age,
            })
        })
        .collect();

    if candidates.is_empty() {
        if !quiet {
            println!(
                "{} No candidates found (age ≥ {}s, CPU ≥ {:.1}%, targets: {:?})",
                "💤".blue(),
                min_age,
                min_cpu,
                targets
            );
        }
        return true;
    }

    for candidate in &candidates {
        println!(
            "{} {} (PID {}, age {}s, CPU {:.1}%)\n   {}",
            "⚠️  Found orphaned process:".yellow(),
            candidate.name.bold(),
            candidate.pid.to_string().cyan(),
            candidate.age,
            candidate.cpu_usage,
            candidate.command.bright_black()
        );
    }

    if dry_run {
        if !quiet {
            println!(
                "{} Would reap {} process(es).",
                "[DRY RUN]".magenta().bold(),
                candidates.len()
            );
        }
        return true;
    }

    let mut pending = Vec::new();
    let mut failed = 0;
    for candidate in &candidates {
        let still_same_process = sys
            .process(candidate.pid)
            .is_some_and(|process| process.start_time() == candidate.start_time);
        let sent = still_same_process
            && sys
                .process(candidate.pid)
                .and_then(|process| process.kill_with(Signal::Term))
                .unwrap_or(false);

        if sent {
            pending.push(candidate);
        } else {
            failed += 1;
            eprintln!(
                "{} Failed to send SIGTERM to PID {}",
                "❌".red(),
                candidate.pid
            );
        }
    }

    if !pending.is_empty() && grace_period > 0 {
        std::thread::sleep(Duration::from_secs(grace_period));
    }

    let pids: Vec<Pid> = pending.iter().map(|candidate| candidate.pid).collect();
    if !pids.is_empty() {
        sys.refresh_processes(ProcessesToUpdate::Some(&pids), true);
    }

    let mut reaped = 0;
    let mut forced = 0;
    for candidate in pending {
        match sys.process(candidate.pid) {
            None => reaped += 1,
            Some(process) if process.start_time() != candidate.start_time => reaped += 1,
            Some(process) if process.kill_with(Signal::Kill).unwrap_or(false) => {
                reaped += 1;
                forced += 1;
            }
            Some(_) => {
                failed += 1;
                eprintln!(
                    "{} Failed to send SIGKILL to PID {}",
                    "❌".red(),
                    candidate.pid
                );
            }
        }
    }

    if !quiet {
        println!("{}", "─".repeat(50).bright_black());
        println!(
            "{} Reaped {} orphaned process(es) ({} required SIGKILL).",
            "✨".green(),
            reaped.to_string().bold(),
            forced
        );
    }
    if reaped > 0 && !no_notify {
        send_macos_notification(
            "System Maintainer",
            &format!("Reaped {reaped} stale orphaned process(es) 🧟"),
        );
    }

    failed == 0
}

fn cleanup_commands(
    keep_generations: u32,
    keep_since: &str,
    deep: bool,
    clean_gcroots: bool,
    optimise: bool,
    dry_run: bool,
    quiet: bool,
) -> Vec<(String, Vec<String>)> {
    let mut nh_args = vec![
        "clean".into(),
        "all".into(),
        "--keep".into(),
        keep_generations.to_string(),
        "--keep-since".into(),
        keep_since.into(),
    ];
    if optimise {
        nh_args.push("--optimise".into());
    }
    if !clean_gcroots {
        nh_args.push("--no-gcroots".into());
        nh_args.push("--no-direnv".into());
    }
    if dry_run {
        nh_args.push("--dry".into());
    }
    if quiet {
        nh_args.push("--quiet".into());
    }

    let mut commands = vec![("nh".into(), nh_args)];
    if deep {
        let mut mo_args = vec!["clean".into()];
        if dry_run {
            mo_args.push("--dry-run".into());
        }
        commands.push(("mo".into(), mo_args));
    }
    commands
}

fn system_cleanup(
    keep_generations: u32,
    keep_since: &str,
    deep: bool,
    clean_gcroots: bool,
    optimise: bool,
    dry_run: bool,
    quiet: bool,
) -> bool {
    if keep_generations == 0 {
        eprintln!("{} --keep-generations must be at least 1", "❌".red());
        return false;
    }
    if keep_since.trim().is_empty() {
        eprintln!("{} --keep-since cannot be empty", "❌".red());
        return false;
    }

    if !quiet {
        println!(
            "{}",
            if dry_run {
                "🧹 Previewing system cleanup..."
            } else {
                "🧹 Running system cleanup..."
            }
            .cyan()
            .bold()
        );
        println!("{}", "─".repeat(50).bright_black());
    }

    let commands = cleanup_commands(
        keep_generations,
        keep_since,
        deep,
        clean_gcroots,
        optimise,
        dry_run,
        quiet,
    );
    let mut succeeded = true;

    for (cmd, args) in commands {
        if !quiet {
            println!("{} {} {:?}", "▶".blue(), cmd.bold(), args);
        }
        match Command::new(&cmd).args(&args).status() {
            Ok(status) if status.success() => {
                if !quiet {
                    println!(
                        "  {} Successfully executed {} {:?}",
                        "✅".green(),
                        cmd.bold(),
                        args
                    );
                }
            }
            Ok(status) => {
                succeeded = false;
                eprintln!(
                    "  {} Command {} {:?} exited with status: {}",
                    "❌".red(),
                    cmd.bold(),
                    args,
                    status.to_string().red()
                );
            }
            Err(error) => {
                succeeded = false;
                eprintln!(
                    "  {} Failed to execute {} {:?}: {}",
                    "💥".red().bold(),
                    cmd.bold(),
                    args,
                    error.to_string().red()
                );
            }
        }
    }

    if !quiet {
        println!("{}", "─".repeat(50).bright_black());
    }
    if succeeded && !quiet {
        let message = if dry_run {
            "✨ System cleanup preview complete."
        } else {
            "✨ System cleanup complete."
        };
        println!("{}", message.green().bold());
    } else if !succeeded {
        eprintln!("{}", "System cleanup completed with errors.".red().bold());
    }
    succeeded
}

#[cfg(test)]
mod tests {
    use super::{cleanup_commands, command_matches};

    #[test]
    fn target_matching_is_case_insensitive() {
        assert!(command_matches(
            "/opt/homebrew/bin/node Jest-Worker/index.js",
            &["jest-worker".into()]
        ));
    }

    #[test]
    fn empty_targets_do_not_match_everything() {
        assert!(!command_matches("node server.js", &["  ".into()]));
    }

    #[test]
    fn any_non_empty_target_can_match() {
        assert!(command_matches(
            "node node_modules/.bin/tsc --watch",
            &["jest-worker".into(), "node_modules/.bin/tsc".into()]
        ));
    }

    #[test]
    fn conservative_cleanup_is_the_default_shape() {
        let commands = cleanup_commands(5, "14d", false, false, false, false, true);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].0, "nh");
        assert_eq!(
            commands[0].1,
            [
                "clean",
                "all",
                "--keep",
                "5",
                "--keep-since",
                "14d",
                "--no-gcroots",
                "--no-direnv",
                "--quiet"
            ]
        );
    }

    #[test]
    fn deep_dry_run_never_runs_a_live_cache_cleanup() {
        let commands = cleanup_commands(5, "14d", true, true, true, true, false);
        assert!(commands[0].1.iter().any(|arg| arg == "--optimise"));
        assert!(commands[0].1.iter().any(|arg| arg == "--dry"));
        assert_eq!(commands[1].0, "mo");
        assert!(commands[1].1.iter().any(|arg| arg == "--dry-run"));
    }
}
