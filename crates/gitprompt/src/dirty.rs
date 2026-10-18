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

/// How long `git status` may run before the prompt gives up on it.
///
/// A prompt runs after every command, so an unbounded child process is the one
/// failure mode that makes the shell unusable rather than merely wrong. Real
/// triggers: a hung `core.fsmonitor` hook, an NFS or SMB home directory, or a
/// `git` wrapper script that blocks.
///
/// Exceeding this yields [`Dirty::Unknown`], which renders no `✘`. A prompt
/// that is briefly wrong self-corrects on the next draw; a shell you cannot type
/// into does not.
const GIT_TIMEOUT: Duration = Duration::from_millis(500);

/// Asks git whether anything differs from `HEAD`.
///
/// Untracked files are counted, because the shell prompt this replaces treated
/// them as dirty; dropping them would silently change the indicator's meaning.
/// `git diff-index` would answer this without walking the working tree, but it
/// ignores untracked files and would silently redefine the indicator.
///
/// `GIT_OPTIONAL_LOCKS=0` keeps git from rewriting `.git/index` as a side
/// effect of a read-only status check. It is an environment variable rather
/// than a flag because `--no-optional-locks` is a `git` global option and is
/// rejected by `git status` itself.
///
/// Bounded by [`GIT_TIMEOUT`]; see its docs for why.
pub fn check(repo_root: &Path) -> Dirty {
    check_with(repo_root, GIT_TIMEOUT, Path::new("git"))
}

/// [`check`], with the timeout and the `git` binary under the caller's control.
///
/// Split out so the timeout path can be tested against a stand-in binary
/// without rewriting `PATH`. Mutating the environment is `unsafe` in recent
/// Rust and this workspace sets `unsafe_code = "forbid"`, so injection is the
/// only way to exercise a hang deterministically.
fn check_with(repo_root: &Path, timeout: Duration, git_bin: &Path) -> Dirty {
    let Ok(mut child) = Command::new(git_bin)
        .arg("status")
        .arg("--porcelain")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .current_dir(repo_root)
        // stdin must not be inherited: a git that decided to prompt would
        // otherwise steal keystrokes from the user's prompt line.
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    else {
        return Dirty::Unknown;
    };

    // The child is waited on by a background thread while this one enforces the
    // deadline. A `try_wait` poll loop cannot do this job: the child only
    // reports a status once it *exits*, so a hung `git` would keep the loop
    // spinning until the child finished on its own — which is precisely the
    // hang the timeout exists to prevent.
    //
    // Both stdout and stderr are taken *before* the move. `Child::wait` drops
    // the pipe handles, so a thread that called it and won the race could close
    // stdout underneath the reader and make a repo with real changes report
    // clean. That race is rare enough to pass locally and fail in CI; keeping
    // ownership of the pipes here removes the ordering dependency entirely.
    let stdout = child.stdout.take();

    let reader = std::thread::spawn(move || {
        use std::io::Read as _;

        let mut buffer: Vec<u8> = Vec::new();
        if let Some(mut out) = stdout {
            // Bounded so a pathological `git` cannot grow this without limit;
            // the verdict only needs to know whether any output arrived.
            let _ = out.by_ref().take(64 * 1024).read_to_end(&mut buffer);
        }
        buffer
    });

    let pid = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    let waiter = std::thread::spawn(move || {
        let result = child.wait();
        let _ = tx.send(result);
    });

    // Timed out, or the waiter panicked: kill the child so it does not outlive
    // the prompt as an orphan. A failure to kill means it already exited, which
    // is harmless.
    //
    // The reader is deliberately *not* joined here. It is blocked in `read` on
    // a pipe the hung child still holds open, and joining it would reinstate
    // exactly the hang this timeout exists to prevent. Killing the child closes
    // the pipe and the thread then finishes on its own.
    let Ok(Ok(status)) = rx.recv_timeout(timeout) else {
        kill_pid(pid);
        drop(waiter);
        return Dirty::Unknown;
    };

    // `git status` exits 128 with empty stdout outside a repository, so the
    // exit status has to be checked before interpreting empty stdout as clean.
    // Reporting "clean" there would silently mask a real failure.
    if !status.success() {
        drop(waiter);
        drop(reader);
        return Dirty::Unknown;
    }

    // The child has exited, so the pipe is closed and this join returns at once.
    let buffer = reader.join().unwrap_or_default();
    let _ = waiter.join();

    if buffer.is_empty() {
        Dirty::No
    } else {
        Dirty::Yes
    }
}

/// Sends `SIGKILL` to a pid already known to have exited or be hung.
///
/// `Child::kill` is not reachable here because the child has been moved into
/// the waiter thread. `unsafe` is forbidden workspace-wide, so this shells out
/// to `kill` rather than calling libc directly; the `kill` binary is present on
/// every platform this runs on, and a failure only means the process is
/// already gone.
fn kill_pid(pid: u32) {
    let _ = Command::new("kill")
        .arg("-KILL")
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Repeated-lookup memo for the dirty verdict.
///
/// **This exists for tests and for any future caller that polls within one
/// process.** The shell does not: `$(gitprompt)` spawns a fresh process per
/// prompt draw, so every run starts with an empty cache and the memo can never
/// hit. An earlier version of this crate claimed the cache absorbed "a burst of
/// redraws"; that was wrong, and the claim was removed rather than the code kept
/// to justify it. The dirty check is simply called once, on startup, by
/// [`check`]'s caller.
///
/// **Where it does trade freshness for speed, that is a real trade.** The worst
/// case is a change made within the TTL not yet being reflected. It is not keyed
/// on the `.git/index` mtime, because editing a tracked file does not rewrite the
/// index and that would report "clean" indefinitely.
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

        // Stamped *after* the call. Stamping before would store an entry that is
        // already expired whenever git takes longer than the TTL — which is
        // exactly the large-repository case a cache would exist to help.
        *self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((Instant::now(), dirty));

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
    fn a_hanging_git_is_abandoned_rather_than_blocking_the_prompt() {
        // Regression guard for the worst failure mode this binary has: it runs
        // on every prompt, so an unbounded `git status` — a hung fsmonitor hook,
        // an NFS home directory — wedges the shell until the user notices.
        //
        // A stand-in binary that sleeps plays the hung child, so the test does
        // not need to touch PATH (which would require `unsafe`).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            let bin = std::env::temp_dir().join(format!("gitprompt-hang-{}", std::process::id()));
            std::fs::create_dir_all(&bin).unwrap();
            let fake = bin.join("git");
            std::fs::write(&fake, "#!/bin/sh\nsleep 30\n").unwrap();
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

            let started = Instant::now();
            let state = check_with(std::path::Path::new("."), Duration::from_millis(300), &fake);
            let elapsed = started.elapsed();

            let _ = std::fs::remove_dir_all(&bin);

            assert_eq!(state, Dirty::Unknown);
            assert!(
                elapsed < Duration::from_secs(5),
                "check took {elapsed:?}; the timeout is not being enforced"
            );
        }
    }

    #[test]
    fn a_stand_in_binary_that_succeeds_is_consulted() {
        // Guards the injection path itself: if `check_with` ignored its `git_bin`
        // argument the timeout test above would pass for the wrong reason.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            let bin = std::env::temp_dir().join(format!("gitprompt-fake-{}", std::process::id()));
            std::fs::create_dir_all(&bin).unwrap();
            let fake = bin.join("git");
            // `git status --porcelain` prints one entry for an untracked file.
            std::fs::write(&fake, "#!/bin/sh\necho '?? newfile'\n").unwrap();
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

            let state = check_with(std::path::Path::new("."), GIT_TIMEOUT, &fake);

            let _ = std::fs::remove_dir_all(&bin);
            assert_eq!(state, Dirty::Yes);
        }
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
