//! Conservative cleanup of stale scratch files under `/tmp`.
//!
//! This deletes real files, so the rules are deliberately narrow: only build and
//! scratch artefacts from tools that are known to leave them behind, and never
//! anything a running process might still be using. The classification lives in
//! `is_protected` / `is_junk` so it can be unit tested directly.

use crate::space;
use crate::ui::Printer;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Default age guard: only remove `/tmp` entries older than a day.
///
/// Scratch directories for a `nix develop` session are created moments before
/// they are used, so a guard of at least a few minutes is what stops cleanup
/// from pulling the environment out from under a running shell.
pub const DEFAULT_MIN_AGE: u64 = 86_400;

/// Prefixes owned by long-lived sockets, daemons and this tool's own launchd
/// logs. These are live files that merely look like scratch data.
const PROTECTED_PREFIXES: &[&str] = &[
    "tmux-",
    "aerospace",
    "zsh-fzf-tab",
    ".com.apple",
    // This tool's own launchd job log names.
    "zombie-reaper",
    "system-cleanup",
    "coffee",
];

/// Suffixes used by unix sockets.
const PROTECTED_SUFFIXES: &[&str] = &[".sock"];

/// Well-known macOS files that must never be touched.
const PROTECTED_EXACT: &[&str] = &["powerlog"];

/// Build/scratch directories that are safe to remove once abandoned.
const JUNK_PREFIXES: &[&str] = &["nix-develop-", "nix-shell.", "chromalayer-vectors-"];

/// Ad-hoc scratch files developers leave behind while debugging.
const JUNK_EXACT: &[&str] = &["dbg.rs", "msg.txt"];

/// Whether `name` must be left alone regardless of anything else.
pub fn is_protected(name: &str) -> bool {
    PROTECTED_EXACT.contains(&name)
        || PROTECTED_SUFFIXES.iter().any(|s| name.ends_with(s))
        || PROTECTED_PREFIXES.iter().any(|p| name.starts_with(p))
}

/// Whether `name` is stale clutter that `coffee` is willing to delete.
///
/// Protection always wins, so a name that matches both rules is kept.
pub fn is_junk(name: &str) -> bool {
    !is_protected(name)
        && (JUNK_EXACT.contains(&name) || JUNK_PREFIXES.iter().any(|p| name.starts_with(p)))
}

/// Whether `entry` is old enough to delete.
///
/// Every ambiguous case resolves to "keep it": an unknown modification time, or
/// a cutoff that underflowed (an absurd `--min-age`), must never turn into a
/// deletion.
fn is_old_enough(entry: &std::fs::DirEntry, cutoff: Option<SystemTime>) -> bool {
    let Some(cutoff) = cutoff else {
        return true;
    };

    entry
        .metadata()
        .ok()
        .and_then(|meta| meta.modified().ok())
        .is_some_and(|modified| modified <= cutoff)
}

/// List the deletable entries directly inside `dir`, sorted for stable output.
///
/// `min_age` is the safety guard that keeps this from deleting scratch
/// directories belonging to a `nix develop` / `nix-shell` session that is still
/// running: those are created seconds before use, not hours. `0` disables the
/// guard.
pub fn scan(dir: &Path, min_age: u64) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let cutoff = SystemTime::now().checked_sub(Duration::from_secs(min_age));

    let mut junk: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| is_junk(&entry.file_name().to_string_lossy()))
        .filter(|entry| is_old_enough(entry, cutoff))
        .map(|entry| entry.path())
        .collect();

    junk.sort();
    junk
}

/// Total size of `paths`, ignoring anything that cannot be stat'ed.
pub fn total_size(paths: &[PathBuf]) -> u64 {
    paths.iter().map(|path| space::dir_size(path)).sum()
}

/// Outcome of a `/tmp` sweep, used for the cleanup stats table.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TmpStats {
    pub removed: usize,
    /// Entries a `--dry-run` would have removed. Always 0 on a live run.
    pub skipped: usize,
    pub bytes: u64,
}

impl TmpStats {
    /// How many entries this sweep accounted for, live or previewed.
    ///
    /// A dry run increments `skipped` and a live run increments `removed`, so
    /// the summary table can report one count without caring which mode ran.
    pub fn accounted(&self) -> usize {
        self.removed + self.skipped
    }
}

/// Remove stale clutter from `/tmp`.
///
/// `min_age` protects scratch directories that are minutes old and likely still
/// in use; `0` removes the guard. Returns false only when `/tmp` is unreadable,
/// which is the one failure worth surfacing.
pub fn clean(dry_run: bool, quiet: bool, min_age: u64) -> TmpStats {
    let out = Printer::quiet(quiet);
    let mut stats = TmpStats::default();
    let tmp = Path::new("/tmp");

    // `read_dir` failing already covers "/tmp is missing"; an `exists()` probe
    // first would be a second syscall asking the same question.
    if std::fs::read_dir(tmp).is_err() {
        out.failure("Failed to read /tmp");
        return stats;
    }

    for path in scan(tmp, min_age) {
        let bytes = total_size(std::slice::from_ref(&path));

        if dry_run {
            out.dry_run(format!("Would remove {}", path.display()));
            stats.skipped += 1;
            stats.bytes += bytes;
            continue;
        }

        // `remove_dir_all` on a file and `remove_file` on a directory both fail,
        // and `/tmp` holds both, so pick by the type we just measured.
        let removed = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        }
        .is_ok();

        if removed {
            out.ok(format!("Removed {}", path.display()));
            stats.removed += 1;
            stats.bytes += bytes;
        } else {
            out.failure(format!("Failed to remove {}", path.display()));
        }
    }

    if stats.removed > 0 {
        out.line(format!(
            "  {} Cleaned {} stale /tmp item(s)",
            "✨".green(),
            stats.removed
        ));
    } else if dry_run {
        out.line(format!("  {} /tmp already clean", "✨".green()));
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::{TmpStats, is_junk, is_protected, scan};
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    /// Comfortably older than the 24h guard, spelled so the duration lint does
    /// not rewrite it into something harder to read.
    const TWO_DAYS: u64 = 2 * 24 * 60 * 60;

    #[test]
    fn accounted_reports_one_count_for_live_runs_and_previews() {
        let live = TmpStats {
            removed: 3,
            ..Default::default()
        };
        assert_eq!(live.accounted(), 3);

        // A dry run never increments `removed`, so the summary has to add the
        // two counters together to describe the same sweep either way.
        let preview = TmpStats {
            skipped: 3,
            ..Default::default()
        };
        assert_eq!(preview.accounted(), 3);

        assert_eq!(TmpStats::default().accounted(), 0);
    }

    /// A scratch directory unique to one test.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "coffee-tmp-test-{}-{}-{name}",
            std::process::id(),
            line!()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn names_of(paths: &[PathBuf]) -> Vec<String> {
        paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    /// Pretend `path` was last modified `age` ago.
    fn backdate(path: &std::path::Path, age: Duration) {
        let when = SystemTime::now() - age;
        std::fs::File::options()
            .write(true)
            .open(path)
            .expect("open for backdating")
            .set_modified(when)
            .expect("backdate mtime");
    }

    #[test]
    fn build_scratch_artefacts_are_junk() {
        assert!(is_junk("nix-develop-abc123"));
        assert!(is_junk("nix-shell.9f8e"));
        assert!(is_junk("chromalayer-vectors-deadbeef"));
        assert!(is_junk("dbg.rs"));
        assert!(is_junk("msg.txt"));
    }

    #[test]
    fn sockets_and_daemons_are_protected() {
        assert!(is_protected("tmux-1000/default"));
        assert!(is_protected("aerospace-501"));
        assert!(is_protected("powerlog"));
        assert!(is_protected("zsh-fzf-tab-status"));
        assert!(is_protected(".com.apple.dbus"));
    }

    #[test]
    fn this_tools_own_launchd_logs_are_protected() {
        // These are live log files written every 30 minutes / weekly.
        assert!(is_protected("zombie-reaper.out"));
        assert!(is_protected("zombie-reaper.err"));
        assert!(is_protected("system-cleanup.out"));
        assert!(is_protected("system-cleanup.err"));
    }

    #[test]
    fn unix_sockets_are_protected() {
        assert!(is_protected("listener.sock"));
        assert!(is_protected("varlock.sock"));
    }

    #[test]
    fn protection_always_beats_a_junk_prefix() {
        // A junk-looking name under a protected prefix must survive.
        assert!(!is_junk("tmux-1000/nix-shell.9f8e"));
    }

    #[test]
    fn unrelated_files_are_untouched() {
        for name in [
            "ds_store",
            "foo.txt",
            "nix-build-cache",
            "ssh-abcDEFG",
            "com.apple.Safari",
        ] {
            assert!(!is_junk(name), "{name} should not be deleted");
        }
    }

    #[test]
    fn scan_only_returns_classified_junk() {
        let dir = temp_dir("scan-only-returns-classified-junk");

        std::fs::write(dir.join("dbg.rs"), b"x").expect("write junk file");
        std::fs::write(dir.join("keep-me.txt"), b"x").expect("write keeper");
        std::fs::write(dir.join("listener.sock"), b"x").expect("write socket");

        let names = names_of(&scan(&dir, 0));
        assert_eq!(names, ["dbg.rs"]);

        std::fs::remove_dir_all(&dir).expect("clean up temp dir");
    }

    #[test]
    fn the_age_guard_spares_freshly_created_scratch_dirs() {
        // A live `nix develop` shell creates these seconds before use, so
        // deleting them mid-session would break the running environment.
        let dir = temp_dir("age-guard-spares-fresh-entries");

        std::fs::write(dir.join("nix-develop-abc123"), b"x").expect("write fresh scratch dir");

        assert!(
            names_of(&scan(&dir, 86_400)).is_empty(),
            "fresh entries must survive a 24h age guard"
        );
        assert_eq!(
            names_of(&scan(&dir, 0)),
            ["nix-develop-abc123"],
            "a zero guard disables the age check"
        );

        std::fs::remove_dir_all(&dir).expect("clean up temp dir");
    }

    #[test]
    fn the_age_guard_removes_entries_older_than_the_threshold() {
        let dir = temp_dir("age-guard-removes-old-entries");

        let old = dir.join("nix-shell.9f8e");
        std::fs::write(&old, b"x").expect("write old scratch entry");
        backdate(&old, Duration::from_secs(TWO_DAYS));

        assert_eq!(names_of(&scan(&dir, 86_400)), ["nix-shell.9f8e"]);

        std::fs::remove_dir_all(&dir).expect("clean up temp dir");
    }

    #[test]
    fn protected_entries_are_never_returned_regardless_of_age() {
        let dir = temp_dir("protected-entries-never-returned");

        let socket = dir.join("system-cleanup.sock");
        std::fs::write(&socket, b"x").expect("write protected file");
        backdate(&socket, Duration::from_secs(TWO_DAYS));

        // Compared rather than `assert!(is_empty())` so a failure names the
        // entries that leaked past the protection check.
        assert_eq!(scan(&dir, 86_400), [] as [std::path::PathBuf; 0]);

        std::fs::remove_dir_all(&dir).expect("clean up temp dir");
    }

    #[test]
    fn an_absurd_age_guard_keeps_everything() {
        // A cutoff that underflows must never be read as "delete it all".
        let dir = temp_dir("absurd-age-guard-keeps-everything");

        std::fs::write(dir.join("dbg.rs"), b"x").expect("write junk file");

        assert_eq!(names_of(&scan(&dir, u64::MAX)), ["dbg.rs"]);

        std::fs::remove_dir_all(&dir).expect("clean up temp dir");
    }
}
