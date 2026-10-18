//! Working out whether the worktree has uncommitted changes.
//!
//! This is the expensive half of a git prompt. Measured on this machine in a
//! 68k-file repository: `git status --porcelain` costs 145ms, of which ~91ms
//! is walking for untracked files.
//!
//! The check itself is delegated to git rather than reimplemented. Git already
//! caches each tracked file's stat data in `.git/index`, so its scan is close
//! to optimal; a Rust reimplementation would be more code and easier to get
//! subtly wrong (stat comparison, skip-worktree bits, sparse checkout,
//! submodules, `.gitignore` rules for untracked files).
//!
//! What this module removes is *process* overhead, not git work — see
//! [`Cached`] for the optional time-based memo.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// Whether the worktree differs from `HEAD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dirty {
    /// Modified, staged, or with untracked files present.
    Yes,
    /// No differences.
    No,
    /// The check could not be performed — not a repository, or git is missing.
    /// Treated as clean so the prompt does not show a spurious `✘`.
    Unknown,
}

impl Dirty {
    /// Whether this state should render the dirty marker.
    #[must_use]
    pub fn is_dirty(self) -> bool {
        matches!(self, Self::Yes)
    }
}

/// Asks git whether anything differs from `HEAD`.
///
/// Untracked files are counted, because the shell prompt this replaces treated
/// them as dirty; dropping them would silently change the indicator's meaning.
///
/// `GIT_OPTIONAL_LOCKS=0` keeps git from rewriting `.git/index` as a side
/// effect of a read-only status check. It is an environment variable rather
/// than a flag because `--no-optional-locks` is a `git` global option and is
/// rejected by `git status` itself.
pub fn check(repo_root: &Path) -> Dirty {
    let Ok(output) = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .current_dir(repo_root)
        .output()
    else {
        return Dirty::Unknown;
    };

    // `git status` exits 128 with empty stdout outside a repository, so the
    // exit status has to be checked before interpreting empty stdout as clean.
    // Reporting "clean" there would silently mask a real failure.
    if !output.status.success() {
        return Dirty::Unknown;
    }

    if output.stdout.is_empty() {
        Dirty::No
    } else {
        Dirty::Yes
    }
}

/// Short-lived memo for the dirty verdict.
///
/// **This trades freshness for speed, deliberately.** An earlier design keyed
/// the cache on the `.git/index` mtime, which is wrong: editing a tracked file
/// does not rewrite the index, so the prompt would keep reporting "clean"
/// indefinitely. A time bound has no such failure mode — the worst case is that
/// a change made in the last few milliseconds is not yet reflected.
///
/// The default TTL is short enough that the staleness is not perceptible
/// between two commands, and long enough to absorb the burst of redraws zsh
/// performs while a prompt is being drawn.
#[derive(Debug)]
pub struct Cached {
    inner: std::sync::Mutex<Option<(Instant, Dirty)>>,
    ttl: Duration,
}

impl Cached {
    /// Creates a memo with the given time-to-live.
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: std::sync::Mutex::new(None),
            ttl,
        }
    }

    /// Returns the dirty state, re-running git once the TTL has elapsed.
    pub fn get(&self, repo_root: &Path) -> Dirty {
        let now = Instant::now();

        {
            let guard = self
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some((at, dirty)) = *guard
                && now.duration_since(at) < self.ttl
            {
                return dirty;
            }
        }

        let dirty = check(repo_root);
        *self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((now, dirty));
        dirty
    }
}

impl Default for Cached {
    fn default() -> Self {
        Self::new(Duration::from_millis(150))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_available() -> bool {
        Command::new("git").arg("--version").output().is_ok()
    }

    /// Builds a throwaway repository with one committed file.
    fn scratch_repo(tag: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("gitprompt-dirty-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();

        let run = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&path)
                .output()
                .expect("git should run");
        };

        run(&["init", "-q"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "test"]);
        std::fs::write(path.join("tracked.txt"), "original\n").unwrap();
        run(&["add", "tracked.txt"]);
        run(&["commit", "-q", "-m", "initial"]);

        path
    }

    #[test]
    fn clean_repository_reports_clean() {
        if !git_available() {
            return;
        }
        let repo = scratch_repo("clean");
        assert_eq!(check(&repo), Dirty::No);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn modified_tracked_file_reports_dirty() {
        if !git_available() {
            return;
        }
        let repo = scratch_repo("modified");
        std::fs::write(repo.join("tracked.txt"), "changed\n").unwrap();
        assert_eq!(check(&repo), Dirty::Yes);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn staged_file_reports_dirty() {
        if !git_available() {
            return;
        }
        let repo = scratch_repo("staged");
        std::fs::write(repo.join("tracked.txt"), "changed\n").unwrap();
        Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(&repo)
            .output()
            .unwrap();
        assert_eq!(check(&repo), Dirty::Yes);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn untracked_file_reports_dirty() {
        if !git_available() {
            return;
        }
        let repo = scratch_repo("untracked");
        std::fs::write(repo.join("new.txt"), "hello\n").unwrap();
        // The shell prompt this replaces counted untracked files, so this must.
        assert_eq!(check(&repo), Dirty::Yes);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn non_repository_directory_is_unknown_and_not_dirty() {
        let dir = std::env::temp_dir().join(format!("gitprompt-norepo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let state = check(&dir);
        assert_eq!(state, Dirty::Unknown);
        // A spurious ✘ on every prompt outside a repo would be worse than
        // showing nothing.
        assert!(!state.is_dirty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_within_ttl_returns_the_memoised_value() {
        if !git_available() {
            return;
        }
        let repo = scratch_repo("cache-ttl");
        let cache = Cached::new(Duration::from_secs(60));

        assert_eq!(cache.get(&repo), Dirty::No);

        // Dirty the worktree while the entry is still fresh. The memo is
        // expected to hold the old answer; this documents the trade-off.
        std::fs::write(repo.join("tracked.txt"), "changed\n").unwrap();
        assert_eq!(cache.get(&repo), Dirty::No);

        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn cache_after_ttl_rechecks() {
        if !git_available() {
            return;
        }
        let repo = scratch_repo("cache-expire");
        // Zero TTL: every call re-runs git.
        let cache = Cached::new(Duration::from_millis(0));

        assert_eq!(cache.get(&repo), Dirty::No);
        std::fs::write(repo.join("tracked.txt"), "changed\n").unwrap();
        assert_eq!(cache.get(&repo), Dirty::Yes);

        let _ = std::fs::remove_dir_all(&repo);
    }
}
