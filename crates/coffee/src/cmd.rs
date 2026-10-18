//! Spawning external child processes.
//!
//! Every child process `coffee` runs follows the same shape: resolve a program,
//! pass argv (never shell source, never an interpolated string), then either
//! let it write straight to the terminal or capture its output. Six modules
//! were repeating that shape inline, so it lives here once.
//!
//! The single most important distinction this module draws is between "the
//! program could not be started" and "the program ran and exited non-zero".
//! `nh` missing is a broken installation; `nh` exiting 1 is a cleanup problem.
//! Collapsing the two into one boolean is what made missing optional tools look
//! like failures.

use std::process::ExitStatus;

/// Build a [`std::process::Command`] for `program` with `args` appended.
///
/// Takes `impl AsRef<str>` so callers can pass a mix of literals and owned
/// `String`s (`cleanup` plans are `Vec<String>`) without converting first.
fn command(program: &str, args: &[impl AsRef<str>]) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    command.args(args.iter().map(AsRef::as_ref));
    command
}

/// Why a child process did not run to completion.
///
/// Spawning can fail for several genuinely different reasons, and they are not
/// interchangeable in a report: "no such file" means the tool is not installed,
/// "permission denied" means it is installed but unusable, and a bad shebang
/// means the file is corrupt. Collapsing them all into "not found" sends the
/// reader looking in the wrong place.
#[derive(Debug)]
pub enum Launch {
    /// The program ran and exited; `success()` reflects how.
    Ran(ExitStatus),
    /// The program could not be started at all.
    Failed(std::io::Error),
}

impl Launch {
    /// The exit status, or `None` when the program never started.
    pub fn status(&self) -> Option<&ExitStatus> {
        match self {
            Launch::Ran(status) => Some(status),
            Launch::Failed(_) => None,
        }
    }

    /// Whether the program ran and exited successfully.
    pub fn is_success(&self) -> bool {
        self.status().is_some_and(ExitStatus::success)
    }
}

/// Run `program`, letting it inherit this process's stdout and stderr.
///
/// Keeps the underlying [`std::io::Error`] so callers can report *why* a tool
/// could not start, rather than guessing.
pub fn run(program: &str, args: &[impl AsRef<str>]) -> Launch {
    command(program, args)
        .status()
        .map_or_else(Launch::Failed, Launch::Ran)
}

/// Run `program` with its output captured rather than printed, discarding it.
///
/// The exit status is preserved exactly, including a failure. That distinction
/// is the point: [`capture`] treats a non-zero exit as `None`, which is right when
/// the caller wants a *value* and wrong when the caller wants to know whether
/// the cleanup step worked.
pub fn run_captured(program: &str, args: &[impl AsRef<str>]) -> Launch {
    command(program, args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_or_else(Launch::Failed, Launch::Ran)
}

/// Run `program`, capture stdout, and return it as lossy UTF-8.
///
/// `None` covers both "could not start" and "produced no stdout", so callers
/// that only want a best-effort value do not need to tell them apart.
pub fn capture(program: &str, args: &[impl AsRef<str>]) -> Option<String> {
    let output = command(program, args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// First line of `program`'s stdout, trimmed.
///
/// Used for `--version` and `id -u` style probes, where a multi-line or
/// multi-paragraph answer is noise and only the leading summary is wanted.
pub fn first_line(program: &str, args: &[impl AsRef<str>]) -> Option<String> {
    let line = capture(program, args)?
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned();
    (!line.is_empty()).then_some(line)
}

#[cfg(test)]
mod tests {
    use super::{Launch, capture, first_line, run};

    #[test]
    fn a_missing_program_cannot_be_started() {
        // The distinction that matters: no exit status, because it never ran.
        let launch = run("definitely-not-a-real-tool-xyz", &["--version"]);
        assert!(launch.status().is_none());
        assert!(matches!(launch, Launch::Failed(_)));
    }

    #[test]
    fn a_program_that_ran_and_failed_is_distinguishable_from_a_missing_one() {
        // Reporting "not found" for a tool that ran and exited 3 would send the
        // reader hunting for a missing binary instead of a failing one.
        let launch = run("/bin/sh", &["-c", "exit 3"]);
        let status = launch.status().expect("sh runs");
        assert!(!status.success());
        assert_eq!(status.code(), Some(3));
        assert!(matches!(launch, Launch::Ran(_)));
    }

    #[test]
    fn the_reason_a_program_could_not_start_is_preserved() {
        // "No such file", "permission denied" and a bad shebang are three
        // different problems, so the error must not be flattened into one.
        match run("definitely-not-a-real-tool-xyz", &["--version"]) {
            Launch::Failed(error) => assert_eq!(error.kind(), std::io::ErrorKind::NotFound),
            Launch::Ran(_) => panic!("the program does not exist"),
        }
    }

    #[test]
    fn a_program_that_is_not_executable_is_not_reported_as_missing() {
        use std::os::unix::fs::PermissionsExt;

        let path = std::env::temp_dir().join(format!("coffee-cmd-notexec-{}", std::process::id()));
        std::fs::write(&path, b"#!/bin/sh\ntrue\n").expect("write a non-executable file");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("chmod");

        let path = path.to_str().expect("utf-8 path").to_string();
        match run(path.as_str(), &[] as &[&str]) {
            Launch::Failed(error) => assert_eq!(
                error.kind(),
                std::io::ErrorKind::PermissionDenied,
                "a present-but-unexecutable file must not look like a missing one"
            ),
            Launch::Ran(_) => panic!("the file is not executable"),
        }

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn run_reports_the_exit_status_of_a_program_that_ran() {
        assert!(
            run("/bin/sh", &["-c", "exit 0"])
                .status()
                .expect("sh runs")
                .success()
        );
        assert!(
            !run("/bin/sh", &["-c", "exit 1"])
                .status()
                .expect("sh runs")
                .success()
        );
    }

    #[test]
    fn captured_output_is_returned_for_successes_only() {
        assert_eq!(
            capture("/bin/sh", &["-c", "printf hello"]).as_deref(),
            Some("hello")
        );
        assert_eq!(capture("/bin/sh", &["-c", "exit 1"]), None);
    }

    #[test]
    fn first_line_keeps_only_the_leading_summary() {
        assert_eq!(
            first_line("/bin/sh", &["-c", "printf 'nh 4.1.2\\nextra\\n'"]).as_deref(),
            Some("nh 4.1.2")
        );
        // An empty line is not a value.
        assert_eq!(first_line("/bin/sh", &["-c", "printf ''"]), None);
        assert_eq!(first_line("/bin/sh", &["-c", "printf '   \\n'"]), None);
    }

    #[test]
    fn arguments_are_passed_as_argv_rather_than_through_a_shell() {
        // If these were interpolated into a shell command, `$(echo pwned)` and
        // the glob `*` would both be expanded before the child ever saw them.
        let evil = "$(echo pwned) *";
        let out = capture("/bin/echo", &["-n", evil]).expect("echo runs");
        assert_eq!(out, evil);
    }

    #[test]
    fn owned_strings_and_literals_can_be_mixed() {
        // `cleanup` plans `Vec<String>` arguments; `doctor` and `notify` pass
        // literals. One signature has to serve both. `echo` joins its arguments
        // with spaces, which is exactly the shape we want to see preserved.
        let owned: Vec<String> = vec!["one".into(), "two".into()];

        assert_eq!(
            capture("/bin/echo", &["-n", "lit", owned[0].as_str()]).as_deref(),
            Some("lit one")
        );

        let mut flags = vec!["-n".to_string()];
        flags.extend(owned);
        assert_eq!(capture("/bin/echo", &flags).as_deref(), Some("one two"));
    }
}
