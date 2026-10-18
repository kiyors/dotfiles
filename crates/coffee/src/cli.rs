//! Command-line surface for `coffee`.
//!
//! Every subcommand owns an `Args` struct so handlers receive one typed value
//! instead of a long positional list. This keeps the dispatch in `main` trivial
//! and makes the arguments directly constructible from tests.

use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(name = "coffee", author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Reap stale, CPU-heavy orphaned developer processes
    ReapZombies(ReapArgs),
    /// Clean old Nix generations, with optional deep cache cleanup
    #[command(name = "system-cleanup", visible_alias = "clean")]
    SystemCleanup(CleanupArgs),

    /// Clean stale temporary build and scratch files from /tmp
    CleanTmp(PassiveArgs),

    /// Report on the health of this machine's maintenance tooling
    Doctor(DoctorArgs),

    /// Report which directories are using the most disk space
    Space(SpaceArgs),

    /// Generate shell completions
    GenerateCompletion(CompletionArgs),
}

#[derive(Debug, Args)]
pub struct ReapArgs {
    /// Command-line fragments to match (can be provided multiple times)
    #[arg(
        short,
        long = "target",
        visible_alias = "targets",
        default_value = "jest-worker"
    )]
    pub targets: Vec<String>,

    /// Minimum process age in seconds
    #[arg(long, default_value_t = 300)]
    pub min_age: u64,

    /// Minimum CPU usage percentage
    #[arg(long, default_value_t = 10.0)]
    pub min_cpu: f32,

    /// Also reap candidates using at least this much memory, even when idle
    /// (0 disables the memory check). Accepts suffixes, e.g. 512M, 4G
    #[arg(long, default_value = "0", value_parser = parse_memory)]
    pub min_memory: u64,

    /// Seconds to wait after SIGTERM before sending SIGKILL
    #[arg(long, default_value_t = 2)]
    pub grace_period: u64,

    /// Only print matching processes; do not send signals
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Suppress start, summary, and no-match output
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,

    /// Do not send a macOS notification after reaping processes
    #[arg(long, default_value_t = false)]
    pub no_notify: bool,
}

impl ReapArgs {
    /// The exact policy the background launchd agent runs.
    ///
    /// `modules/darwin/zombie-reaper.nix` invokes `coffee reap-zombies` every
    /// 30 minutes with these thresholds. `coffee doctor --reap-candidates`
    /// reports against the same policy rather than an approximation of it, so
    /// the diagnostic cannot quietly disagree with what is actually killing
    /// processes. `background_policy_matches_agent` in the tests below fails
    /// if the two ever drift apart again.
    ///
    /// Read-only and quiet by construction: this describes what the agent
    /// *would* do, so it must never signal anything itself.
    pub fn background_agent_policy() -> Self {
        Self {
            targets: vec![
                "jest-worker".into(),
                "typescript/lib/tsc.js".into(),
                "node_modules/.bin/tsc".into(),
                "esbuild".into(),
                "rust-analyzer".into(),
            ],
            min_age: 900,
            min_cpu: 25.0,
            // The agent reaps idle multi-gigabyte renderers, so the memory
            // threshold is a live part of the policy — not zero.
            min_memory: 1024 * 1024 * 1024,
            grace_period: 5,
            dry_run: true,
            quiet: true,
            no_notify: true,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.min_cpu.is_finite() || self.min_cpu < 0.0 {
            return Err("--min-cpu must be a non-negative number".into());
        }
        if self.min_age == 0 {
            return Err("--min-age must be at least 1 second".into());
        }
        if self.min_cpu == 0.0 && self.min_memory == 0 {
            return Err(
                "--min-cpu and --min-memory cannot both be 0: that would match every \
                 process. Raise one of them."
                    .into(),
            );
        }
        Ok(())
    }
}

/// Parse a memory size such as `512M`, `4G` or a plain byte count.
pub fn parse_memory(raw: &str) -> Result<u64, String> {
    let trimmed = raw.trim();
    let (digits, multiplier) = match trimmed.chars().last() {
        Some('K' | 'k') => (&trimmed[..trimmed.len() - 1], 1024u64),
        Some('M' | 'm') => (&trimmed[..trimmed.len() - 1], 1024 * 1024),
        Some('G' | 'g') => (&trimmed[..trimmed.len() - 1], 1024 * 1024 * 1024),
        Some('T' | 't') => (&trimmed[..trimmed.len() - 1], 1024 * 1024 * 1024 * 1024),
        _ => (trimmed, 1),
    };

    digits
        .parse::<u64>()
        .map(|value| value.saturating_mul(multiplier))
        .map_err(|_| format!("invalid memory size: {raw} (try 512M, 4G or a byte count)"))
}

#[derive(Debug, Args)]
pub struct CleanupArgs {
    /// Number of Nix generations to retain
    #[arg(long, default_value_t = 5)]
    pub keep_generations: u32,

    /// Also retain Nix generations newer than this duration (for example, 14d)
    #[arg(long, default_value = "14d")]
    pub keep_since: String,

    /// Reclaimable caches AND project build artifacts: mo clean, mo purge,
    /// Gradle/Xcode caches, Homebrew, macOS caches and stale /tmp entries
    #[arg(long, default_value_t = false)]
    pub deep: bool,

    /// Also purge project build artifacts (`node_modules/`, `target/`, `build/`, `dist/`)
    #[arg(long, default_value_t = false)]
    pub purge_projects: bool,
    /// Delete old Nix generations with `sudo nix-collect-garbage -d` (this discards rollback history)
    #[arg(short = 'd', long, default_value_t = false)]
    pub collect_garbage: bool,

    /// Also clean stale temporary build and scratch files in /tmp
    #[arg(long, default_value_t = false)]
    pub clean_tmp: bool,

    /// Also remove stale project and direnv GC roots
    #[arg(long, default_value_t = false)]
    pub clean_gcroots: bool,

    /// Deduplicate the Nix store after garbage collection (runs sudo nix-store --optimise)
    #[arg(long, default_value_t = false)]
    pub optimise: bool,

    /// Preview cleanup without changing anything
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Suppress routine progress output
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,

    /// Full clean: only the current generation of each profile survives, and
    /// every phase runs (caches, project artifacts, garbage collection, store
    /// optimisation, /tmp). Destructive.
    #[arg(long, default_value_t = false)]
    pub all: bool,

    /// Do not print the closing summary table of space reclaimed
    #[arg(long, default_value_t = false)]
    pub no_stats: bool,

    /// Only remove /tmp entries older than this many seconds (0 disables the guard)
    #[arg(long, default_value_t = crate::tmp::DEFAULT_MIN_AGE)]
    pub tmp_min_age: u64,

    /// Do not send a macOS notification after cleanup
    #[arg(long, default_value_t = false)]
    pub no_notify: bool,
}

impl CleanupArgs {
    pub fn validate(&self) -> Result<(), String> {
        if !self.all && self.keep_generations == 0 {
            return Err("--keep-generations must be at least 1 (or use --all)".into());
        }
        if !self.all && self.keep_since.trim().is_empty() {
            return Err("--keep-since cannot be empty".into());
        }
        Ok(())
    }

    /// `--all` implies every destructive phase and full generation removal.
    pub fn deep(&self) -> bool {
        self.all || self.deep
    }

    pub fn purge(&self) -> bool {
        self.all || self.deep || self.purge_projects
    }

    pub fn gc(&self) -> bool {
        self.all || self.collect_garbage
    }

    pub fn optimise(&self) -> bool {
        self.all || self.optimise
    }

    pub fn clean_tmp(&self) -> bool {
        self.all || self.clean_tmp
    }

    pub fn clean_gcroots(&self) -> bool {
        self.all || self.clean_gcroots
    }

    pub fn effective_keep_generations(&self) -> u32 {
        // Never 0. `nh clean --keep 0` deletes every generation of every
        // profile, including the ones the `default` and `system` profile
        // symlinks point at — that removes the Nix binaries themselves
        // (nix, nix-store, ...) and the running system generation. Keeping 1
        // still discards all rollback history, which is what `--all` is for.
        if self.all { 1 } else { self.keep_generations }
    }

    pub fn effective_keep_since(&self) -> &str {
        if self.all {
            "0h"
        } else {
            self.keep_since.trim()
        }
    }
}

#[derive(Debug, Args)]
pub struct PassiveArgs {
    /// Preview cleanup without changing anything
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Suppress routine progress output
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,

    /// Only remove entries older than this many seconds (0 disables the guard)
    #[arg(long, default_value_t = crate::tmp::DEFAULT_MIN_AGE)]
    pub min_age: u64,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Also list processes that `reap-zombies` would target right now
    #[arg(long, default_value_t = false)]
    pub reap_candidates: bool,

    /// Exit non-zero when a warning or failure is reported
    #[arg(long, default_value_t = false)]
    pub strict: bool,

    /// Only print warnings and failures
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,
}

#[derive(Debug, Args)]
pub struct SpaceArgs {
    /// Directories to inspect (default: home, caches and the nix store)
    #[arg(value_name = "PATH")]
    pub paths: Vec<std::path::PathBuf>,

    /// Maximum rows to show
    #[arg(long, default_value_t = 20)]
    pub top: usize,

    /// Only print warnings
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,
}

#[derive(Debug, Args)]
pub struct CompletionArgs {
    #[arg(value_enum)]
    pub shell: Shell,

    /// Command name to generate completions for (default: "coffee")
    #[arg(long, default_value = "coffee")]
    pub bin_name: String,
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands, ReapArgs, parse_memory};
    use clap::{CommandFactory, Parser};

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("arguments must parse")
    }

    /// The clap definition itself was never validated: a bad `default_value`,
    /// a conflicting long name or an unreachable alias would only surface as a
    /// runtime panic or a broken shell completion. This is clap's own check for
    /// exactly that, and it costs nothing to run on every build.
    #[test]
    fn the_command_definition_is_internally_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn the_subcommands_users_are_documented_to_have_all_exist() {
        // `docs/system-maintenance.md` tells users to run these by name; if a
        // rename lands without updating the docs, this is what catches it.
        for name in [
            "reap-zombies",
            "system-cleanup",
            "clean",
            "clean-tmp",
            "doctor",
            "space",
            "generate-completion",
        ] {
            let argv: Vec<&str> = if name == "generate-completion" {
                vec!["coffee", name, "bash"]
            } else {
                vec!["coffee", name]
            };
            assert!(
                Cli::try_parse_from(&argv).is_ok(),
                "documented subcommand `{name}` does not parse"
            );
        }
    }

    #[test]
    fn clean_is_an_alias_for_system_cleanup() {
        let Cli { command } = parse(&["coffee", "clean"]);
        assert!(matches!(command, Commands::SystemCleanup(_)));
    }

    #[test]
    fn reap_zombies_defaults_are_the_documented_ones() {
        let Cli { command } = parse(&["coffee", "reap-zombies"]);
        let Commands::ReapZombies(args) = command else {
            panic!("expected reap-zombies, got {command:?}");
        };

        assert_eq!(args.targets, ["jest-worker"]);
        assert_eq!(args.min_age, 300);
        assert_eq!(args.min_cpu, 10.0);
        // 0 must mean "memory check disabled", not "every process matches".
        // `a_zero_threshold_disables_its_check` pins the behaviour this relies on.
        assert_eq!(args.min_memory, 0);
        assert!(!args.dry_run);
        assert!(!args.quiet);
        assert!(!args.no_notify);
    }

    #[test]
    fn reap_zombies_accepts_repeated_targets() {
        let Cli { command } = parse(&[
            "coffee",
            "reap-zombies",
            "-t",
            "jest-worker",
            "--target",
            "esbuild",
        ]);
        let Commands::ReapZombies(args) = command else {
            panic!("expected reap-zombies");
        };
        assert_eq!(args.targets, ["jest-worker", "esbuild"]);
    }

    #[test]
    fn the_destructive_cleanup_flags_combine_without_erroring() {
        // These reach clap together in practice (a "full" run plus a preview),
        // and clap conflicts are enforced at parse time — so if anyone ever adds
        // `conflicts_with` here, this is where it surfaces.
        let parse_ok = |args: &[&str]| Cli::try_parse_from(args).is_ok();
        assert!(parse_ok(&[
            "coffee",
            "system-cleanup",
            "--all",
            "--dry-run"
        ]));
        assert!(parse_ok(&[
            "coffee",
            "system-cleanup",
            "--deep",
            "--dry-run"
        ]));
        assert!(parse_ok(&[
            "coffee",
            "system-cleanup",
            "--deep",
            "--purge-projects",
            "--collect-garbage",
            "--optimise",
            "--clean-tmp",
            "--dry-run",
        ]));
    }

    #[test]
    fn space_takes_paths_positionally_and_a_top_limit() {
        let Cli { command } = parse(&["coffee", "space", "/tmp", "/var", "--top", "5"]);
        let Commands::Space(args) = command else {
            panic!("expected space");
        };
        assert_eq!(args.paths.len(), 2);
        assert_eq!(args.top, 5);
    }

    #[test]
    fn generate_completion_defaults_the_binary_name_to_coffee() {
        let Cli { command } = parse(&["coffee", "generate-completion", "zsh"]);
        let Commands::GenerateCompletion(args) = command else {
            panic!("expected generate-completion");
        };
        assert_eq!(args.bin_name, "coffee");
    }

    #[test]
    fn a_zero_threshold_disables_its_check_rather_than_matching_everything() {
        // The reaper's policy is `cpu_hot || memory_hungry`, so a threshold of
        // `0` must read as "this check is off". Treating it as a live threshold
        // makes the comparison vacuously true, which silently turns the *other*
        // threshold into dead code.
        let cpu_off = ReapArgs {
            min_cpu: 0.0,
            min_memory: 1024,
            ..ReapArgs::background_agent_policy()
        };
        assert!(
            cpu_off.validate().is_ok(),
            "memory-only matching is allowed"
        );

        let memory_off = ReapArgs {
            min_cpu: 10.0,
            min_memory: 0,
            ..ReapArgs::background_agent_policy()
        };
        assert!(
            memory_off.validate().is_ok(),
            "CPU-only matching is allowed"
        );

        // Both off is the only genuinely unsafe combination.
        let both_off = ReapArgs {
            min_cpu: 0.0,
            min_memory: 0,
            ..ReapArgs::background_agent_policy()
        };
        assert!(both_off.validate().is_err());
    }

    #[test]
    fn the_background_policy_never_signals_anything() {
        // `doctor` reports candidates with this policy. If it could ever send a
        // signal, running a diagnostic would start killing processes.
        let policy = ReapArgs::background_agent_policy();
        assert!(policy.dry_run, "the reporting policy must be a dry run");
        assert!(policy.quiet);
        assert!(policy.no_notify);
        assert!(policy.validate().is_ok());
    }

    /// The one test that stops the Rust side and the launchd agent from drifting
    /// apart again — which is exactly how `doctor` came to report a policy
    /// (`--min-memory 0`) that the real agent (`--min-memory 1G`) never used.
    #[test]
    fn background_policy_matches_the_launchd_agent() {
        use std::path::PathBuf;

        // `CARGO_MANIFEST_DIR` is `<repo>/crates/coffee`. Inside the Nix build
        // sandbox only the Rust files are present, so this legitimately skips
        // rather than failing there.
        let agent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/darwin/zombie-reaper.nix");
        let Ok(source) = std::fs::read_to_string(&agent) else {
            eprintln!(
                "skipping: {} is not available in this build",
                agent.display()
            );
            return;
        };

        let policy = ReapArgs::background_agent_policy();
        let tokens: Vec<&str> = source.split_whitespace().collect();
        let flag = |name: &str| {
            tokens.windows(2).find(|pair| pair[0] == name).map_or_else(
                || panic!("{name} is not set in {}", agent.display()),
                |pair| pair[1].trim_end_matches('\\').to_string(),
            )
        };

        assert_eq!(policy.min_age.to_string(), flag("--min-age"));
        assert_eq!(policy.min_cpu.to_string(), flag("--min-cpu"));
        assert_eq!(
            policy.min_memory,
            parse_memory(&flag("--min-memory")).expect("--min-memory parses")
        );
        assert_eq!(policy.grace_period.to_string(), flag("--grace-period"));

        // Every `-t` in the agent must appear in the shared policy, and the
        // policy must not claim targets the agent does not use.
        //
        // The trailing `\` is stripped because launchd runs the agent's command
        // block as a single `sh -c` string, where every flag has to be joined by
        // a backslash continuation. It is shell punctuation, not part of the
        // target.
        let agent_targets: Vec<&str> = source
            .lines()
            .filter_map(|line| line.trim().strip_prefix("-t "))
            .map(|target| target.trim().trim_end_matches('\\').trim())
            .filter(|target| !target.is_empty())
            .collect();

        assert!(
            !agent_targets.is_empty(),
            "no -t targets found in the agent"
        );
        for target in &agent_targets {
            assert!(
                policy.targets.iter().any(|mine| mine == target),
                "agent target `{target}` is missing from ReapArgs::background_agent_policy()"
            );
        }
        assert_eq!(
            policy.targets.len(),
            agent_targets.len(),
            "the policy and the agent disagree about the target list"
        );
    }

    /// Regression guard for the bug that made the whole reaper policy inert.
    ///
    /// launchd runs the agent's command block as `sh -c "<one string>"`. A
    /// newline inside that string terminates the command, so a block written
    /// with bare newlines instead of `\` continuations passed only
    /// `reap-zombies` to the binary: the rest of the flags became separate
    /// "command not found" failures, and the agent silently ran with clap's
    /// defaults — reaping every orphaned `jest-worker` regardless of CPU or age,
    /// and ignoring `--quiet`.
    ///
    /// This asserts the shell actually receives all of them, so the mistake
    /// cannot come back through a reformat.
    #[test]
    fn every_agent_flag_survives_the_shell_invocation() {
        use std::path::PathBuf;

        let agent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/darwin/zombie-reaper.nix");
        let Ok(source) = std::fs::read_to_string(&agent) else {
            return;
        };

        // The command block is the indented `coffee reap-zombies ...` region,
        // which ends at the closing quote of the Nix string. Comments inside it
        // are prose, not flags, so they are excluded rather than mistaken for a
        // broken continuation.
        let block: Vec<&str> = source
            .lines()
            .map(str::trim)
            // Anchor on the binary invocation itself. Matching the subcommand
            // name would instead hit the prose in the comment above it.
            .skip_while(|line| !line.starts_with("${pkgs.coffee}"))
            // The Nix string closes with `'';`, which ends the block. Stopping
            // only on a blank line would run on into `serviceConfig`.
            .take_while(|line| !line.is_empty() && !line.ends_with("'';"))
            .collect();

        assert!(
            block.len() > 5,
            "failed to locate the reaper command block in {}",
            agent.display()
        );

        // Every line but the last must continue the one above it; the final one
        // ends the command and so carries no backslash.
        let (continuations, last) = block.split_at(block.len() - 1);
        let last = last[0];
        assert!(
            !last.ends_with('\\'),
            "a trailing backslash would swallow the next command: {last}"
        );

        for line in continuations {
            assert!(
                line.ends_with('\\'),
                "`{line}` would be executed as its own shell command; \
                 every flag in a launchd `sh -c` block needs a trailing backslash"
            );
        }

        // And prove the real shell agrees, using the flags this test cares about.
        // `${pkgs.coffee}` is a Nix interpolation, not a shell word, so it is
        // replaced with a placeholder path before the block reaches `/bin/sh`.
        let joined = block
            .iter()
            .map(|line| line.trim_end_matches('\\').trim())
            .map(|line| line.replace("${pkgs.coffee}", "/bin/coffee"))
            .collect::<Vec<_>>()
            .join(" ");

        let script = format!("set -- {joined}; printf '%s\\n' \"$@\"");
        let output = std::process::Command::new("/bin/sh")
            .args(["-c", &script])
            .output()
            .expect("/bin/sh runs");

        let argv: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect();

        for expected in [
            "reap-zombies",
            "--quiet",
            "--min-age",
            "--min-cpu",
            "--min-memory",
            "--grace-period",
            "-t",
        ] {
            assert!(
                argv.iter().any(|arg| arg == expected),
                "`{expected}` never reaches the binary; the agent would run with defaults. argv={argv:?}"
            );
        }
    }
}
