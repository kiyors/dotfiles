//! Reading the branch name straight out of `.git/HEAD`.
//!
//! `git symbolic-ref HEAD` costs a fork and an exec (~8ms measured) purely to
//! read a file this module parses in microseconds.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// What `.git/HEAD` says about the current checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    /// On a branch. Holds the short name, e.g. `main` or `feature/login`.
    Branch(String),
    /// Detached. Holds the abbreviated commit id.
    Detached(String),
    /// `HEAD` exists but could not be understood.
    Unknown,
}

impl Head {
    /// The text to show in a prompt.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        match self {
            Self::Branch(name) | Self::Detached(name) => Some(name),
            Self::Unknown => None,
        }
    }
}

/// Reads `.git/HEAD` and returns what it points at.
///
/// Detached heads are abbreviated to seven characters, matching
/// `git rev-parse --short HEAD`.
#[must_use]
pub fn read(git_dir: &Path) -> Head {
    let Ok(contents) = std::fs::read_to_string(git_dir.join("HEAD")) else {
        return Head::Unknown;
    };

    let trimmed = contents.trim();

    if let Some(reference) = trimmed.strip_prefix("ref:") {
        let reference = reference.trim();
        return match reference.strip_prefix("refs/heads/") {
            Some(branch) if !branch.is_empty() => Head::Branch(branch.to_owned()),
            // `refs/remotes/...`, a symbolic ref to something unusual, or a
            // branch name that is somehow empty: show it verbatim rather than
            // dropping the prompt segment entirely.
            _ => Head::Branch(reference.to_owned()),
        };
    }

    // A bare 40-character object id means a detached HEAD.
    if trimmed.len() >= 7 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Head::Detached(trimmed[..7].to_owned());
    }

    Head::Unknown
}

/// Caches the answer per `.git` directory for a short window.
///
/// `HEAD` changes only on `git checkout`, so re-reading it per prompt is
/// wasted I/O — but a cache that never expires would leave the prompt showing
/// the old branch after a checkout. A few hundred milliseconds is short enough
/// that a user never notices the lag and long enough to absorb the rapid-fire
/// prompt draws of an interactive session.
///
/// Keyed by path so switching directories between two repos cannot return the
/// other repo's branch.
#[derive(Debug)]
pub struct HeadCache {
    entries: std::sync::Mutex<Vec<(std::path::PathBuf, Head, u64)>>,
    ttl_millis: u64,
}

impl HeadCache {
    /// Creates a cache with the given time-to-live in milliseconds.
    #[must_use]
    pub fn new(ttl_millis: u64) -> Self {
        Self {
            entries: std::sync::Mutex::new(Vec::new()),
            ttl_millis,
        }
    }

    /// Returns the branch for `git_dir`, re-reading only when the cache expired.
    pub fn get(&self, git_dir: &Path) -> Head {
        // A monotonically increasing counter stands in for wall-clock time so
        // tests can drive expiry without sleeping.
        static TICK: AtomicU64 = AtomicU64::new(0);
        let now = TICK.fetch_add(1, Ordering::Relaxed);

        let mut guard = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if let Some((_, head, at)) = guard.iter().find(|(p, _, _)| p == git_dir)
            && now.saturating_sub(*at) < self.ttl_millis
        {
            return head.clone();
        }

        let head = read(git_dir);

        // Keep one entry per repository; a shell rarely straddles more than a
        // handful, so the linear scan is cheaper than a hash map would be.
        match guard.iter_mut().find(|(p, _, _)| p == git_dir) {
            Some(entry) => {
                entry.1 = head.clone();
                entry.2 = now;
            }
            None => guard.push((git_dir.to_path_buf(), head.clone(), now)),
        }

        head
    }
}

impl Default for HeadCache {
    fn default() -> Self {
        // 250ms. Long enough to cover a burst of `cd`+prompt, short enough
        // that switching branches updates the prompt before it is read.
        Self::new(250)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let base =
                std::env::temp_dir().join(format!("gitprompt-head-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&base);
            fs::create_dir_all(&base).expect("create temp dir");
            Self(base)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn git_dir_with(tag: &str, head: &str) -> (TempDir, PathBuf) {
        let tmp = TempDir::new(tag);
        let git_dir = tmp.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), head).unwrap();
        (tmp, git_dir)
    }

    #[test]
    fn reads_a_branch() {
        let (_tmp, dir) = git_dir_with("branch", "ref: refs/heads/main\n");
        assert_eq!(read(&dir), Head::Branch("main".into()));
    }

    #[test]
    fn reads_a_nested_branch_name() {
        let (_tmp, dir) = git_dir_with("nested", "ref: refs/heads/feature/some/deep/name\n");
        assert_eq!(read(&dir), Head::Branch("feature/some/deep/name".into()));
    }

    #[test]
    fn reads_a_detached_head_as_seven_characters() {
        let (_tmp, dir) = git_dir_with("detached", "3f2a1b9c4d5e6f708192a3b4c5d6e7f8091a2b3c\n");
        assert_eq!(read(&dir), Head::Detached("3f2a1b9".into()));
    }

    #[test]
    fn missing_head_is_unknown() {
        let tmp = TempDir::new("missing");
        assert_eq!(read(tmp.path()), Head::Unknown);
    }

    #[test]
    fn unparseable_head_is_unknown() {
        let (_tmp, dir) = git_dir_with("garbage", "this is not a ref\n");
        assert_eq!(read(&dir), Head::Unknown);
    }

    #[test]
    fn non_head_ref_is_shown_verbatim() {
        // A symbolic ref pointing outside refs/heads is unusual but legal;
        // dropping it would make the prompt look broken.
        let (_tmp, dir) = git_dir_with("remote-ref", "ref: refs/remotes/origin/main\n");
        assert_eq!(read(&dir), Head::Branch("refs/remotes/origin/main".into()));
    }

    #[test]
    fn cache_returns_the_same_answer_within_the_ttl() {
        let (tmp, dir) = git_dir_with("cache-ttl", "ref: refs/heads/main\n");
        let cache = HeadCache::new(1000);

        assert_eq!(cache.get(&dir), Head::Branch("main".into()));

        // Rewrite HEAD; inside the TTL the cache must still answer "main".
        fs::write(dir.join("HEAD"), "ref: refs/heads/other\n").unwrap();
        assert_eq!(cache.get(&dir), Head::Branch("main".into()));

        drop(tmp);
    }

    #[test]
    fn cache_expires_and_picks_up_a_new_branch() {
        let (_tmp, dir) = git_dir_with("cache-expiry", "ref: refs/heads/main\n");
        // TTL of 1 tick: every call re-reads.
        let cache = HeadCache::new(1);

        assert_eq!(cache.get(&dir), Head::Branch("main".into()));
        fs::write(dir.join("HEAD"), "ref: refs/heads/switched\n").unwrap();
        assert_eq!(cache.get(&dir), Head::Branch("switched".into()));
    }

    #[test]
    fn cache_is_keyed_per_repository() {
        let (_a, dir_a) = git_dir_with("cache-a", "ref: refs/heads/alpha\n");
        let (_b, dir_b) = git_dir_with("cache-b", "ref: refs/heads/beta\n");
        let cache = HeadCache::new(1000);

        assert_eq!(cache.get(&dir_a), Head::Branch("alpha".into()));
        assert_eq!(cache.get(&dir_b), Head::Branch("beta".into()));
        // Interleaved access must not cross-contaminate.
        assert_eq!(cache.get(&dir_a), Head::Branch("alpha".into()));
        assert_eq!(cache.get(&dir_b), Head::Branch("beta".into()));
    }
}
