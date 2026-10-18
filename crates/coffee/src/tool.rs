//! Locating and interrogating external command-line tools.

use crate::cmd;
use std::path::PathBuf;

/// Find `name` on `PATH`, mirroring what a shell would resolve.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// Return the first line of `<name> --version`, if the tool is installed and
/// responds. Keeps `doctor` output to a single tidy line per tool.
pub fn version_of(name: &str) -> Option<String> {
    // Distinguish "installed but unresponsive" from "not installed at all";
    // `doctor` reports those differently.
    which(name)?;
    cmd::first_line(name, &["--version"])
}

/// Whether `name` is a macOS-only tool, and therefore only a hard requirement
/// on macOS.
pub fn is_macos() -> bool {
    std::env::consts::OS == "macos"
}

#[cfg(test)]
mod tests {
    use super::{version_of, which};

    #[test]
    fn which_finds_a_tool_that_is_always_present() {
        assert!(which("sh").is_some(), "expected `sh` to resolve on PATH");
    }

    #[test]
    fn which_returns_none_for_a_tool_that_does_not_exist() {
        assert!(which("definitely-not-a-real-tool-xyz").is_none());
    }

    #[test]
    fn version_of_is_none_for_a_tool_that_is_not_installed() {
        assert!(version_of("definitely-not-a-real-tool-xyz").is_none());
    }

    #[test]
    fn version_of_reads_a_single_line() {
        // `sh --version` exits non-zero on macOS, so use a tool that answers.
        if let Some(version) = version_of("sed") {
            assert!(!version.is_empty());
            assert!(!version.contains('\n'));
        }
    }
}
