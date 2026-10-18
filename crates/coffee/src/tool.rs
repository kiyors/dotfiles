//! Locating and interrogating external command-line tools.

use crate::cmd;
use std::path::PathBuf;

/// Find `name` on `PATH`, mirroring what a shell would resolve.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    which_in(name, &path)
}

/// [`which`], against an explicit search path.
///
/// Split out so tests can point the lookup at a directory they control. Doing
/// that by mutating `PATH` needs `unsafe` under this workspace's
/// `unsafe_code = "forbid"`, and a test that depends on some other tool being
/// installed is not a test.
fn which_in(name: &str, path: &std::ffi::OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
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
        // The tool under test is created here rather than borrowed from the
        // system, because every real candidate proved unreliable in some
        // environment: macOS ships BSD `sed`, which rejects `--version`, and
        // the Nix build sandbox has no `git` on PATH at all. Both cases made
        // `version_of` return `None`, and with the body wrapped in
        // `if let Some(..)` the test passed without asserting anything — dead
        // code that looked like coverage.
        //
        // A local script keeps the assertion unconditional and hermetic, and
        // the lookup goes through `which_in` so `PATH` never has to be mutated
        // (which would need `unsafe` under this workspace's forbid).
        let dir = std::env::temp_dir().join(format!("coffee-tool-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let name = "coffee-fake-version";
        let tool = dir.join(name);
        std::fs::write(&tool, "#!/bin/sh\nprintf 'fake 1.2.3\\nsecond line\\n'\n")
            .expect("write fake tool");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755))
                .expect("make fake tool executable");
        }

        let found = super::which_in(name, dir.as_os_str());
        assert_eq!(
            found.as_deref(),
            Some(tool.as_path()),
            "the script should be found on an explicit search path"
        );

        // `first_line` is what `version_of` delegates to. It is driven with an
        // absolute path so no PATH lookup is involved at all.
        let version = crate::cmd::first_line(tool.to_str().expect("utf-8 path"), &["--version"])
            .expect("the fake tool should report a version");

        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(
            version, "fake 1.2.3",
            "only the first line of `--version` should be returned"
        );
        assert!(
            !version.contains('\n'),
            "version should be a single line, got {version:?}"
        );
    }

    #[test]
    fn which_in_ignores_a_directory_that_does_not_contain_the_tool() {
        let empty = std::env::temp_dir().join(format!("coffee-tool-empty-{}", std::process::id()));
        std::fs::create_dir_all(&empty).expect("create temp dir");

        assert_eq!(super::which_in("nothing-here", empty.as_os_str()), None);

        let _ = std::fs::remove_dir_all(&empty);
    }
}
