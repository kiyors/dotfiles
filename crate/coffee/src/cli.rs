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

    /// Also purge project build artifacts (node_modules, target/, build/, dist/)
    #[arg(long, default_value_t = false)]
    pub purge_projects: bool,
    /// Delete old Nix generations with sudo nix-collect-garbage -d (this discards rollback history)
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

    /// Full clean: no generation retention, and every phase (caches, project
    /// artifacts, garbage collection, store optimisation, /tmp). Destructive.
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
        if self.all { 0 } else { self.keep_generations }
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
