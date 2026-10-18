//! Deciding where a child's output should go.
//!
//! `coffee --quiet` promises to suppress routine progress, but most of the work
//! is done by `nh`, `mo`, `brew` and `sudo` — and those write to stdout directly.
//! A `--quiet` flag on the parent therefore used to change almost nothing about
//! the output the user actually saw, because the child printed anyway.
//!
//! The fix is to stop letting a quiet child inherit the terminal. Its output is
//! discarded instead, which both honours `--quiet` and means `coffee`'s own
//! progress lines are no longer interleaved with a tool's raw dump.

/// How to run one external tool, given whether the user asked for quiet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Let the child write straight to the terminal.
    Inherit,
    /// Discard the child's output, keeping only its exit status.
    Quiet,
}

impl Mode {
    /// The mode implied by the `--quiet` flag.
    pub fn from_quiet(quiet: bool) -> Self {
        if quiet { Mode::Quiet } else { Mode::Inherit }
    }

    /// Run `program`, returning how the launch went.
    ///
    /// The exit status is preserved in both modes. That matters because cleanup
    /// tools signal failure through it: discarding a child's output must not
    /// also discard the fact that it failed.
    pub fn run(self, program: &str, args: &[impl AsRef<str>]) -> crate::cmd::Launch {
        match self {
            Mode::Inherit => crate::cmd::run(program, args),
            Mode::Quiet => crate::cmd::run_captured(program, args),
        }
    }

    /// Run `program` and discard how it went.
    ///
    /// For fire-and-forget tools where neither the exit status nor the reason is
    /// worth reporting.
    pub fn run_discarding(self, program: &str, args: &[impl AsRef<str>]) {
        let _ = self.run(program, args);
    }
}

#[cfg(test)]
mod tests {
    use super::Mode;
    use std::process::ExitStatus;

    #[test]
    fn the_quiet_flag_picks_the_mode() {
        assert_eq!(Mode::from_quiet(true), Mode::Quiet);
        assert_eq!(Mode::from_quiet(false), Mode::Inherit);
    }

    #[test]
    fn a_quiet_child_still_reports_its_real_exit_status() {
        // `nh clean` signals failure through its exit code. If a quiet run
        // flattened that to success, `coffee clean --quiet` would report a
        // successful cleanup that never happened.
        let failed = Mode::Quiet.run("/bin/sh", &["-c", "exit 2"]);
        assert!(!failed.is_success());
        assert!(Mode::Quiet.run("/bin/sh", &["-c", "exit 0"]).is_success());
    }

    #[test]
    fn a_child_that_was_never_found_reports_no_status_but_keeps_the_reason() {
        for mode in [Mode::Inherit, Mode::Quiet] {
            let launch = mode.run("definitely-not-a-real-tool-xyz", &["--version"]);

            assert!(
                launch.status().is_none(),
                "a missing program has no exit status in either mode"
            );
            assert!(
                matches!(launch, crate::cmd::Launch::Failed(_)),
                "and the reason it could not start must survive"
            );
        }
    }

    #[test]
    fn a_failure_to_start_is_not_reported_as_a_nonzero_exit() {
        // These are different problems: one is "the tool is missing", the other
        // is "the tool ran and failed". Collapsing them hides which.
        let missing = Mode::Inherit.run("definitely-not-a-real-tool-xyz", &["-x"]);
        let failing = Mode::Inherit.run("/bin/sh", &["-c", "exit 1"]);

        assert!(missing.status().is_none());
        assert_eq!(failing.status().map(ExitStatus::code), Some(Some(1)));
    }

    #[test]
    fn a_missing_program_is_never_a_success() {
        // The dangerous direction: treating "could not start" as success would
        // let a missing tool masquerade as a working one.
        for mode in [Mode::Inherit, Mode::Quiet] {
            assert!(
                !mode
                    .run("definitely-not-a-real-tool-xyz", &["-x"])
                    .is_success()
            );
        }
    }

    #[test]
    fn discarding_the_result_cannot_change_the_exit_status() {
        // `run_discarding` is for tools whose failure is genuinely unimportant
        // (cache flushes), but it must not change how any *other* call reports.
        Mode::Quiet.run_discarding("/bin/sh", &["-c", "exit 1"]);
        assert!(!Mode::Quiet.run("/bin/sh", &["-c", "exit 1"]).is_success());
    }

    #[test]
    fn both_modes_agree_on_the_exit_status() {
        for code in [0, 1, 2] {
            let loud = Mode::Inherit.run("/bin/sh", &[&format!("exit {code}")]);
            let quiet = Mode::Quiet.run("/bin/sh", &[&format!("exit {code}")]);

            assert_eq!(
                loud.status().map(ExitStatus::code),
                quiet.status().map(ExitStatus::code)
            );
        }
    }
}
