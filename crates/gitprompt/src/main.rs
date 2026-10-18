//! `gitprompt` — the git segment of a shell prompt.
//!
//! Prints one line to stdout and exits. That is the entire contract, because
//! the caller is a `$(...)` substitution inside `PROMPT`.
//!
//! ## Why this exists
//!
//! The shell function it replaces ran, on every prompt draw:
//!
//! ```sh
//! git symbolic-ref HEAD          # fork + exec
//! git status --porcelain | tail  # fork + exec + pipe + tail exec
//! ```
//!
//! plus two command substitutions for the nested calls — six processes to
//! produce two facts. Measured on this machine that is 29ms in a normal
//! repository and 161ms in a 68k-file one.
//!
//! ## What actually got faster
//!
//! The branch lookup. Reading `.git/HEAD` directly removes a fork and an exec
//! (~8ms), and the result is memoised per repository for a few hundred
//! milliseconds so a burst of redraws costs one read.
//!
//! The dirty check still calls `git status`, because git's own index caching
//! already makes that scan close to optimal and reimplementing it would be
//! slower to write and easier to get wrong. What disappears is the extra
//! process overhead around it: one exec instead of six.
//!
//! Set `GITPROMPT_TTL_MS=0` to disable memoisation entirely and always get a
//! freshly computed answer.

mod dirty;
mod head;
mod render;
mod repo;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

/// Reads `GITPROMPT_TTL_MS`, falling back to 150ms.
///
/// An unparseable or negative value disables caching rather than erroring:
/// a prompt must never be the thing that breaks a shell.
fn ttl_from_env() -> std::time::Duration {
    let Ok(raw) = std::env::var("GITPROMPT_TTL_MS") else {
        return std::time::Duration::from_millis(150);
    };

    match raw.trim().parse::<u64>() {
        Ok(0) => std::time::Duration::from_millis(0),
        Ok(ms) => std::time::Duration::from_millis(ms),
        Err(_) => std::time::Duration::from_millis(150),
    }
}

fn main() -> ExitCode {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Not in a repository is the common case (~, /tmp) and must be silent.
    let Some(repo) = repo::Repo::discover(&cwd) else {
        return ExitCode::SUCCESS;
    };

    let head = head::HeadCache::default();
    let dirt = dirty::Cached::new(ttl_from_env());

    let branch = head.get(repo.git_dir());
    let Some(branch) = branch.label().map(render::sanitize) else {
        return ExitCode::SUCCESS;
    };

    let segment = render::segment(&branch, dirt.get(repo.root()).is_dirty());

    // A closed stdout means the prompt was interrupted mid-draw. That is not
    // an error worth reporting, and writing to it would just raise SIGPIPE.
    let _ = std::io::stdout().write_all(segment.as_bytes());

    ExitCode::SUCCESS
}
