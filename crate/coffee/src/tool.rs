//! Locating and interrogating external command-line tools.

use std::path::PathBuf;
use std::process::Command;

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
    which(name)?;
    let output = Command::new(name).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next().unwrap_or_default().trim().to_owned();
    (!line.is_empty()).then_some(line)
}

/// Whether `name` is a macOS-only tool, and therefore only a hard requirement
/// on macOS.
pub fn is_macos() -> bool {
    std::env::consts::OS == "macos"
}

#[cfg(test)]
mod tests {
    use super::which;

    #[test]
    fn which_finds_a_tool_that_is_always_present() {
        assert!(which("sh").is_some(), "expected `sh` to resolve on PATH");
    }

    #[test]
    fn which_returns_none_for_a_tool_that_does_not_exist() {
        assert!(which("definitely-not-a-real-tool-xyz").is_none());
    }
}
