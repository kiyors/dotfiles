//! Locating the git repository that owns a directory.
//!
//! Only the on-disk layout is handled here — no `git` process is spawned.
//! That is the entire reason this crate exists: `git rev-parse --show-toplevel`
//! costs a fork and an exec on every single prompt.

use std::path::{Path, PathBuf};

/// The git directory and worktree root for the repository containing `start`.
///
/// Returns `None` when `start` is not inside a repository, which is the common
/// case in `~` and `/tmp` and must stay silent and cheap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// The `.git` directory. May live outside the worktree for worktrees and
    /// submodules, so it is tracked separately from `root`.
    git_dir: PathBuf,
    /// The worktree root — the directory whose contents belong to the repo.
    root: PathBuf,
}

impl Repo {
    /// Walks up from `start` looking for a `.git` entry.
    ///
    /// Symlinks are *not* followed while walking. Resolving them would make
    /// the walk depend on filesystem layout in ways that are hard to reason
    /// about, and `~` is frequently a symlink on some systems.
    #[must_use]
    pub fn discover(start: &Path) -> Option<Self> {
        let mut dir = Some(start);

        while let Some(current) = dir {
            let candidate = current.join(".git");

            // `.git` is a directory in a normal clone, and a *file* containing
            // `gitdir: <path>` in a worktree or submodule. Both are valid.
            if candidate.is_dir() {
                return Some(Self {
                    git_dir: candidate,
                    root: current.to_path_buf(),
                });
            }

            if candidate.is_file() {
                return read_gitdir_pointer(&candidate).map(|git_dir| Self {
                    git_dir,
                    root: current.to_path_buf(),
                });
            }

            dir = current.parent();
        }

        None
    }

    /// Absolute path of the `.git` directory.
    #[must_use]
    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    /// Absolute path of the worktree root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Resolves the `gitdir: <path>` indirection used by worktrees and submodules.
///
/// The path in the file may be relative, in which case it is interpreted
/// against the directory containing the `.git` file itself — which is exactly
/// what git does.
fn read_gitdir_pointer(pointer_file: &Path) -> Option<PathBuf> {
    let contents = std::fs::read_to_string(pointer_file).ok()?;
    let target = contents
        .lines()
        .find_map(|line| line.trim().strip_prefix("gitdir:"))?
        .trim();

    if target.is_empty() {
        return None;
    }

    let path = Path::new(target);
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        pointer_file.parent()?.join(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Creates a temporary directory that cleans itself up.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            // PID keeps parallel test threads from colliding.
            let base = std::env::temp_dir().join(format!("gitprompt-{tag}-{}", std::process::id()));
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

    #[test]
    fn finds_repository_from_the_root() {
        let tmp = TempDir::new("root");
        fs::create_dir(tmp.path().join(".git")).unwrap();

        let repo = Repo::discover(tmp.path()).expect("should discover repo");

        assert_eq!(repo.root(), tmp.path());
        assert_eq!(repo.git_dir(), tmp.path().join(".git"));
    }

    #[test]
    fn finds_repository_from_a_nested_directory() {
        let tmp = TempDir::new("nested");
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        let deep = tmp.path().join("src/main/lib");
        fs::create_dir_all(&deep).unwrap();

        let repo = Repo::discover(&deep).expect("should walk up to repo root");

        assert_eq!(repo.root(), tmp.path());
    }

    #[test]
    fn returns_none_outside_a_repository() {
        let tmp = TempDir::new("outside");
        let deep = tmp.path().join("a/b/c");
        fs::create_dir_all(&deep).unwrap();

        assert_eq!(Repo::discover(&deep), None);
    }

    #[test]
    fn resolves_a_worktree_gitdir_pointer() {
        let tmp = TempDir::new("worktree");
        let worktree = tmp.path().join("wt");
        let real_git_dir = tmp.path().join("real/.git/worktrees/wt");
        fs::create_dir_all(&worktree).unwrap();
        fs::create_dir_all(&real_git_dir).unwrap();

        // Relative pointer, which is what `git worktree add` writes.
        fs::write(worktree.join(".git"), "gitdir: ../real/.git/worktrees/wt\n").unwrap();

        let repo = Repo::discover(&worktree).expect("should follow gitdir pointer");

        assert_eq!(repo.root(), worktree);
        // `wt/../real/.git/worktrees/wt` normalises to `tmp/real/...`.
        assert!(repo.git_dir().ends_with("real/.git/worktrees/wt"));
    }

    #[test]
    fn rejects_an_empty_gitdir_pointer() {
        let tmp = TempDir::new("empty-pointer");
        fs::write(tmp.path().join(".git"), "gitdir:   \n").unwrap();

        assert_eq!(Repo::discover(tmp.path()), None);
    }
}
