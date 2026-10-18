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
//! The branch lookup. Reading `.git/HEAD` directly removes a fork and an exec,
//! measured at roughly 8ms.
//!
//! The dirty check still calls `git status`, because git's own index caching
//! already makes that scan close to optimal and reimplementing it would be
//! slower to write and easier to get wrong. That is also where almost all the
//! remaining time goes: `git status` alone costs ~149ms in a 68k-file
//! repository, so the honest end-to-end win over the shell version is six
//! processes down to three, not an order of magnitude.
//!
//! ## Memoisation is not used by the shell
//!
//! `$(gitprompt)` spawns a fresh process for every prompt draw, so the caches in
//! [`head::HeadCache`] and [`dirty::Cached`] always start empty and always miss.
//! They exist for tests and for any future in-process caller. `GITPROMPT_TTL_MS`
//! tunes them and has no effect on what you see in a shell.

mod dirty;
mod head;
mod render;
mod repo;

use std::io::Write;
use std::process::ExitCode;

/// Reads `GITPROMPT_TTL_MS`, falling back to 150ms.
///
/// An unparseable value falls back to the default rather than erroring: a
/// prompt must never be the thing that breaks a shell. Note that `0` disables
/// caching while a negative or garbage value *enables* the default — the
/// distinction only matters to an in-process caller, since the shell creates a
/// new process (and therefore a new cache) on every prompt.
///
/// Split from the `env::var` read so the parsing rules can be tested without
/// mutating process-global state, which needs `unsafe` under this workspace's
/// `unsafe_code = "forbid"`.
fn ttl_from_env() -> std::time::Duration {
    match std::env::var("GITPROMPT_TTL_MS") {
        Ok(raw) => parse_ttl(&raw),
        Err(_) => std::time::Duration::from_millis(150),
    }
}

/// Turns a raw `GITPROMPT_TTL_MS` value into a duration.
fn parse_ttl(raw: &str) -> std::time::Duration {
    match raw.trim().parse::<u64>() {
        Ok(0) => std::time::Duration::ZERO,
        Ok(ms) => std::time::Duration::from_millis(ms),
        Err(_) => std::time::Duration::from_millis(150),
    }
}

fn main() -> ExitCode {
    // A cwd we cannot read means the directory was removed out from under us,
    // which happens with `cd`-then-`rmdir`. There is nothing to resolve `.`
    // against at that point, so give up rather than guess a repository.
    let Ok(cwd) = std::env::current_dir() else {
        return ExitCode::SUCCESS;
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_disables_caching() {
        assert_eq!(parse_ttl("0"), std::time::Duration::ZERO);
    }

    #[test]
    fn a_numeric_value_is_honoured() {
        assert_eq!(parse_ttl("42"), std::time::Duration::from_millis(42));
        // Surrounding whitespace is tolerated rather than treated as garbage.
        assert_eq!(parse_ttl("  250  "), std::time::Duration::from_millis(250));
    }

    #[test]
    fn garbage_falls_back_rather_than_disabling() {
        // Regression guard: the doc once claimed an unparseable value disabled
        // caching, which was the opposite of what the code did.
        for bad in ["abc", "-5", "", "   "] {
            assert_eq!(
                parse_ttl(bad),
                std::time::Duration::from_millis(150),
                "input {bad:?} should fall back to the default"
            );
        }
    }
}
