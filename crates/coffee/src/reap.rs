//! Detection and signalling of stale, orphaned developer processes.
//!
//! A candidate is a process that was adopted by `launchd` (PPID 1) after its
//! original parent exited, has been alive for a while, and is still burning CPU.
//! PPID 1 alone is not proof of a problem, so the age, CPU and command-line
//! filters are what keep this safe; see `docs/system-maintenance.md`.

use crate::cli::ReapArgs;
use crate::notify;
use crate::ui;
use colored::Colorize;
use std::time::Duration;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, Signal, System};

/// How long to wait between the two process samples.
///
/// CPU usage is a delta between samples, so a single refresh always reads zero.
const CPU_SAMPLE_DELAY: Duration = Duration::from_millis(500);

/// The refresh kind this tool needs.
///
/// Both fields matter, and getting either wrong makes the reaper a silent
/// no-op rather than an error:
///
/// - `cmd` must be refreshed, otherwise every command line is empty and the
///   target filter matches nothing. `System::refresh_processes` uses
///   `ProcessRefreshKind::nothing()`, which does not populate `cmd`.
/// - CPU usage must be measured, otherwise `--min-cpu` filters everything out.
///
/// `everything()` is the only kind verified to satisfy both on macOS; the
/// narrower `nothing().with_cmd(...)` leaves `cpu_usage()` at zero.
fn refresh_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::everything()
}

/// Executable prefixes owned by macOS itself.
///
/// This is the strongest safety net in the reaper: even if a target string is
/// mistyped so that it matches a system daemon, nothing running out of these
/// prefixes is ever signalled. Developer tools do not live here.
///
/// `/bin` is deliberately absent. A tool invoked as a shell script reports its
/// executable as `/bin/sh`, so protecting `/bin` would make every script-based
/// developer tool unreachable. Scripts are covered instead by the same-user
/// check below, and real daemons under `/bin` are covered by [`PROTECTED_NAMES`].
const SYSTEM_EXE_PREFIXES: &[&str] = &["/System/", "/usr/", "/sbin/", "/Library/Apple/"];

/// Process names that are never reapable, regardless of targets.
///
/// Belt and braces alongside [`SYSTEM_EXE_PREFIXES`]: a daemon's executable path
/// can be missing (unreadable by a non-root user), and this check does not
/// depend on that succeeding.
const PROTECTED_NAMES: &[&str] = &[
    "kernel_task",
    "launchd",
    "launchctl",
    "loginwindow",
    "WindowServer",
    "SystemUIServer",
    "Dock",
    "Finder",
    "securityd",
    "sshd",
    "sudo",
    "mds",
    "mds_stores",
    "mdworker_shared",
    "diskarbitrationd",
    "opendirectoryd",
    "cfprefsd",
    "coreaudiod",
    "coreaudiod_agent",
    "powerd",
    "hidd",
];

/// Whether an executable path belongs to the operating system.
pub fn is_system_executable(exe: &str) -> bool {
    SYSTEM_EXE_PREFIXES
        .iter()
        .any(|prefix| exe.starts_with(prefix))
}

/// Whether a process is structurally off-limits to the reaper.
pub fn is_protected_name(name: &str) -> bool {
    PROTECTED_NAMES.iter().any(|p| p.eq_ignore_ascii_case(name))
}

#[derive(Debug)]
pub struct Candidate {
    pub pid: Pid,
    pub start_time: u64,
    pub name: String,
    pub command: String,
    pub cpu_usage: f32,
    pub age: u64,
    pub memory: u64,
    pub exe: String,
}

/// The full command line of a process, joined with spaces.
///
/// `sysinfo` returns argv as a slice, and every caller in this module wanted
/// the same single string to substring-match against. Doing the join in one
/// place also keeps the lossy conversion applied once.
pub fn command_line(process: &sysinfo::Process) -> String {
    process
        .cmd()
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Case-insensitive substring match against any of `targets`.
///
/// Blank targets are ignored so that `--target ""` can never match everything.
pub fn command_matches(command: &str, targets: &[String]) -> bool {
    let command = command.to_lowercase();
    targets
        .iter()
        .map(|target| target.trim().to_lowercase())
        .any(|target| !target.is_empty() && command.contains(&target))
}

/// Sample the process table twice so CPU usage reflects a real delta.
pub fn sample_processes() -> System {
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind());
    std::thread::sleep(CPU_SAMPLE_DELAY);
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind());
    system
}

/// Every PID this process must never signal: itself and all of its ancestors.
///
/// Signalling an ancestor would terminate whatever launched the reaper — the
/// login session, tmux, or the terminal. Since candidates are only ever PIDs
/// whose parent is 1, an ancestor cannot normally match, but a recycled PID
/// could, so the set is computed explicitly.
fn protected_pids(system: &System) -> Vec<Pid> {
    let self_pid = Pid::from_u32(std::process::id());
    let mut chain = Vec::new();
    let mut current = self_pid;
    let mut hops = 0;

    while let Some(process) = system.process(current) {
        chain.push(current);
        match process.parent() {
            Some(parent) if parent != current && hops < 128 => {
                current = parent;
                hops += 1;
            }
            _ => break,
        }
    }

    chain
}

/// Collect the processes matching the reaping policy.
///
/// Guards are applied in layers, each one cheap and independently sufficient to
/// prevent damage: ancestry, then executable provenance, then process name,
/// then ownership, and only then the user-supplied filters.
pub fn find_candidates(system: &System, args: &ReapArgs) -> Vec<Candidate> {
    let ancestors = protected_pids(system);
    let own_uid = system
        .process(Pid::from_u32(std::process::id()))
        .map(|process| process.user_id());

    let mut candidates: Vec<Candidate> = system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            // 1. Never signal ourselves or an ancestor.
            if ancestors.contains(pid) {
                return None;
            }

            // 2. Never signal anything shipped by the operating system.
            let exe = process.exe().map(|path| path.display().to_string());
            if exe.as_deref().is_some_and(is_system_executable) {
                return None;
            }

            // 3. Never signal a known-critical daemon by name.
            let name = process.name().to_string_lossy().into_owned();
            if is_protected_name(&name) {
                return None;
            }

            // 4. Only ever touch processes owned by the same user. This is what
            //    keeps the tool harmless if it is ever run as root.
            if let (Some(own), Some(theirs)) = (own_uid, Some(process.user_id()))
                && own != theirs
            {
                return None;
            }

            // 5. Only processes adopted by launchd.
            if process.parent().is_none_or(|parent| parent.as_u32() != 1) {
                return None;
            }

            let cpu_usage = process.cpu_usage();
            let age = process.run_time();
            let memory = process.memory();

            // `min_cpu` and `min_memory` are alternatives: a process must trip
            // at least one of them, so a 4 GB leaked renderer is still caught
            // even while idle, and a busy compile is caught at low memory.
            //
            // A threshold of 0 means "this check is disabled", not "everything
            // passes it": comparing against an unset threshold would be
            // vacuously true (`x >= 0`), which would silently make the *other*
            // threshold irrelevant. With `--min-memory 0` — the default — that
            // turned `--min-cpu` into a no-op and reduced the policy to
            // "orphan + target match", killing idle long-lived dev servers.
            let cpu_hot = args.min_cpu > 0.0 && cpu_usage >= args.min_cpu;
            let memory_hungry = args.min_memory > 0 && memory >= args.min_memory;
            // Reject unless every layer of the policy agrees.
            if age < args.min_age || !(cpu_hot || memory_hungry) {
                return None;
            }

            let command = command_line(process);

            if !command_matches(&command, &args.targets) {
                return None;
            }

            Some(Candidate {
                pid: *pid,
                start_time: process.start_time(),
                name,
                command,
                cpu_usage,
                age,
                memory,
                exe: exe.unwrap_or_default(),
            })
        })
        .collect();

    // Busiest and hungriest first, with the PID as a stable tie-breaker so the
    // output order does not depend on hash-map iteration.
    candidates.sort_by(|a, b| {
        b.cpu_usage
            .total_cmp(&a.cpu_usage)
            .then(b.memory.cmp(&a.memory))
            .then(a.pid.cmp(&b.pid))
    });

    candidates
}

/// Whether any process already in `system` has a command line containing
/// `fragment`.
///
/// Used by deep cleanup to avoid deleting a build cache out from under a build
/// that is still running. Takes a snapshot the caller already holds rather than
/// sampling its own, because deep cleanup checks several caches and each sample
/// walks the whole process table.
pub fn any_process_matches(system: &System, fragment: &str) -> bool {
    let targets = [fragment.to_string()];
    system
        .processes()
        .values()
        .any(|process| command_matches(&command_line(process), &targets))
}

impl Candidate {
    /// A short, human-meaningful label: the executable name when the process
    /// name is generic, otherwise the reported name.
    fn label(&self) -> String {
        if self.name.chars().count() > 2 {
            return self.name.clone();
        }

        std::path::Path::new(&self.exe)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| self.name.clone())
    }
}

/// Render the candidates as a table, newest metrics first.
fn candidate_table(candidates: &[Candidate], out: &ui::Printer) {
    if out.is_quiet() {
        return;
    }

    let rows: Vec<Vec<String>> = candidates
        .iter()
        .map(|candidate| {
            vec![
                candidate.label(),
                candidate.pid.as_u32().to_string(),
                format!("{}s", candidate.age),
                format!("{:.1}%", candidate.cpu_usage),
                ui::human_bytes(candidate.memory),
                candidate.exe.clone(),
            ]
        })
        .collect();

    ui::table(
        &["Process", "PID", "Age", "CPU", "Memory", "Executable"],
        &rows,
        &[
            ui::Align::Left,
            ui::Align::Right,
            ui::Align::Right,
            ui::Align::Right,
            ui::Align::Right,
            ui::Align::Left,
        ],
    );
}

/// Terminate every candidate: `SIGTERM`, wait, then `SIGKILL` what is left.
///
/// Returns whether every signal was delivered successfully.
pub fn run(args: &ReapArgs) -> bool {
    let out = ui::Printer::quiet(args.quiet);

    if let Err(error) = args.validate() {
        out.fatal(error);
        return false;
    }

    out.heading("🧟 Scanning for stale orphaned processes...");

    let mut system = sample_processes();
    let candidates = find_candidates(&system, args);

    if candidates.is_empty() {
        out.line(format!(
            "{} No candidates found (age ≥ {}s, CPU ≥ {:.1}%, memory ≥ {}, targets: {:?})",
            "💤".blue(),
            args.min_age,
            args.min_cpu,
            ui::human_bytes(args.min_memory),
            args.targets
        ));
        return true;
    }

    out.line(format!("{}", "⚠️  Found orphaned processes:".yellow()));
    candidate_table(&candidates, &out);
    for candidate in &candidates {
        out.line(format!("   {}", candidate.command.bright_black()));
    }

    if args.dry_run {
        out.dry_run(format!("Would reap {} process(es).", candidates.len()));
        return true;
    }

    let mut pending = Vec::new();
    let mut failed = 0;

    for candidate in &candidates {
        // Confirm the PID was not recycled between scanning and signalling.
        let still_same_process = system
            .process(candidate.pid)
            .is_some_and(|process| process.start_time() == candidate.start_time);

        let sent = still_same_process
            && system
                .process(candidate.pid)
                .and_then(|process| process.kill_with(Signal::Term))
                .unwrap_or(false);

        if sent {
            pending.push(candidate);
        } else {
            failed += 1;
            out.failure(format!("Failed to send SIGTERM to PID {}", candidate.pid));
        }
    }

    if !pending.is_empty() && args.grace_period > 0 {
        std::thread::sleep(Duration::from_secs(args.grace_period));
    }

    let pids: Vec<Pid> = pending.iter().map(|candidate| candidate.pid).collect();
    if !pids.is_empty() {
        system.refresh_processes(ProcessesToUpdate::Some(&pids), true);
    }

    let mut reaped = 0;
    let mut forced = 0;

    for candidate in pending {
        match system.process(candidate.pid) {
            None => reaped += 1,
            Some(process) if process.start_time() != candidate.start_time => reaped += 1,
            Some(process) if process.kill_with(Signal::Kill).unwrap_or(false) => {
                reaped += 1;
                forced += 1;
            }
            Some(_) => {
                failed += 1;
                out.failure(format!("Failed to send SIGKILL to PID {}", candidate.pid));
            }
        }
    }

    out.rule();
    out.line(format!(
        "{} Reaped {} orphaned process(es) ({} required SIGKILL).",
        "✨".green(),
        reaped.to_string().bold(),
        forced
    ));

    if forced > 0 {
        out.warn(format!(
            "{forced} process(es) ignored SIGTERM and were force-killed"
        ));
    }

    if reaped > 0 && !args.no_notify {
        notify::send(
            "System Maintainer",
            &format!("Reaped {reaped} stale orphaned process(es) 🧟"),
        );
    }

    failed == 0
}

#[cfg(test)]
mod tests {
    use super::{
        command_matches, is_protected_name, is_system_executable, protected_pids, refresh_kind,
        sample_processes,
    };
    use crate::cli::{ReapArgs, parse_memory};
    use std::time::Duration;
    use sysinfo::{ProcessesToUpdate, System};

    fn reap_args() -> ReapArgs {
        ReapArgs {
            targets: vec!["jest-worker".into()],
            min_age: 300,
            min_cpu: 10.0,
            min_memory: 0,
            grace_period: 2,
            dry_run: false,
            quiet: false,
            no_notify: false,
        }
    }

    #[test]
    fn command_lines_are_actually_populated() {
        // Regression guard. `System::refresh_processes` uses
        // `ProcessRefreshKind::nothing()`, which leaves every `cmd()` empty on
        // macOS. With empty command lines the target filter matches nothing and
        // the reaper silently becomes a no-op.
        let mut system = System::new();
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind());

        let own = sysinfo::Pid::from_u32(std::process::id());
        let cmd = system
            .process(own)
            .expect("the current process must be listed")
            .cmd()
            .to_vec();

        assert!(
            !cmd.is_empty(),
            "command line must be refreshed; the reaper's matching depends on it"
        );

        let joined = cmd
            .iter()
            .map(|part| part.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            joined.contains("coffee"),
            "expected our own binary in the command line, got: {joined}"
        );
    }

    #[test]
    fn sampling_sees_a_live_cpu_burner() {
        // This is the strongest guarantee that reaping actually works: the
        // command line must be visible (otherwise the target filter matches
        // nothing) and CPU usage must be non-zero (otherwise --min-cpu filters
        // everything out). Both were silently zero before `everything()` was
        // used as the refresh kind.
        let mut burner = std::process::Command::new("/bin/sh")
            .args(["-c", "while :; do :; done"])
            .spawn()
            .expect("spawn a CPU burner");

        // Give the burner a moment to actually start burning CPU, then retry:
        // a single sample can land before the process shows measurable usage,
        // and this test runs inside `nix build`, so it must not be flaky.
        let pid = sysinfo::Pid::from_u32(burner.id());
        let mut seen = None;

        for attempt in 0..5 {
            std::thread::sleep(Duration::from_millis(300));
            let system = sample_processes();

            if let Some(process) = system.process(pid) {
                let joined = process
                    .cmd()
                    .iter()
                    .map(|part| part.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join(" ");

                if !joined.is_empty() && process.cpu_usage() > 0.0 {
                    seen = Some(joined);
                    break;
                }

                if attempt == 4 {
                    seen = Some(joined);
                }
            }
        }

        let _ = burner.kill();
        let _ = burner.wait();

        let joined = seen.expect("the burner must appear in the process table");
        assert!(
            !joined.is_empty(),
            "command line was empty after repeated sampling: {joined:?}"
        );
    }

    #[test]
    fn target_matching_is_case_insensitive() {
        assert!(command_matches(
            "/opt/homebrew/bin/node Jest-Worker/index.js",
            &["jest-worker".into()]
        ));
    }

    #[test]
    fn empty_targets_do_not_match_everything() {
        assert!(!command_matches("node server.js", &["  ".into()]));
        assert!(!command_matches("node server.js", &[]));
    }

    #[test]
    fn any_non_empty_target_can_match() {
        assert!(command_matches(
            "node node_modules/.bin/tsc --watch",
            &["jest-worker".into(), "node_modules/.bin/tsc".into()]
        ));
    }

    #[test]
    fn targets_are_trimmed_before_matching() {
        assert!(command_matches(
            "node esbuild --service",
            &["  esbuild ".into()]
        ));
    }

    #[test]
    fn system_executables_are_never_reapable() {
        for exe in [
            "/usr/libexec/sshd-keygen-wrapper",
            "/System/Library/CoreServices/Finder.app/Contents/MacOS/Finder",
            "/usr/bin/securityd",
            "/sbin/launchd",
            "/Library/Apple/usr/bin/SoftwareUpdate",
        ] {
            assert!(is_system_executable(exe), "{exe} should be protected");
        }

        for exe in [
            "/opt/homebrew/bin/node",
            "/Users/gaurav/dotfiles/node_modules/.bin/esbuild",
            "/nix/store/abc-coffee-0.2.0/bin/coffee",
            // A script-based dev tool reports /bin/sh; it must stay reapable.
            "/bin/sh",
            "/bin/bash",
            "",
        ] {
            assert!(!is_system_executable(exe), "{exe} should be reapable");
        }
    }

    #[test]
    fn critical_daemon_names_are_never_reapable() {
        for name in [
            "launchd",
            "kernel_task",
            "WindowServer",
            "Dock",
            "sshd",
            "LAUNCHD",
            "dock",
        ] {
            assert!(is_protected_name(name), "{name} should be protected");
        }

        for name in ["node", "jest-worker", "esbuild", "rust-analyzer", "tsc"] {
            assert!(!is_protected_name(name), "{name} should be reapable");
        }
    }

    #[test]
    fn the_reaper_never_targets_itself_or_an_ancestor() {
        let system = sample_processes();
        let ancestors = protected_pids(&system);

        assert!(
            ancestors.contains(&sysinfo::Pid::from_u32(std::process::id())),
            "the reaper's own PID must be protected"
        );
        // Compared rather than `assert!(!is_empty())` so a failure prints the
        // chain that was built, which is what makes the failure diagnosable.
        assert_ne!(ancestors, [] as [sysinfo::Pid; 0]);
        assert!(
            !ancestors.windows(2).any(|pair| pair[0] == pair[1]),
            "the ancestor chain must not contain cycles"
        );
    }

    #[test]
    fn memory_sizes_are_parsed_with_suffixes() {
        assert_eq!(parse_memory("0").unwrap(), 0);
        assert_eq!(parse_memory("1024").unwrap(), 1024);
        assert_eq!(parse_memory("512M").unwrap(), 512 * 1024 * 1024);
        assert_eq!(parse_memory("4G").unwrap(), 4 * 1024 * 1024 * 1024);
        assert_eq!(parse_memory("1T").unwrap(), 1024 * 1024 * 1024 * 1024);
        assert_eq!(parse_memory(" 512m ").unwrap(), 512 * 1024 * 1024);

        assert!(parse_memory("abc").is_err());
        assert!(parse_memory("12x").is_err());
        assert!(parse_memory("M").is_err());
    }

    #[test]
    fn matching_nothing_at_all_is_rejected() {
        // min_cpu = 0 and min_memory = 0 would match every orphaned process on
        // the machine, which is exactly the "system gets fucked" case.
        let mut args = reap_args();
        args.min_cpu = 0.0;
        args.min_memory = 0;
        assert!(args.validate().is_err());

        args.min_cpu = 0.0;
        args.min_memory = 1024;
        assert!(args.validate().is_ok(), "memory-only matching is allowed");

        args.min_cpu = 5.0;
        args.min_memory = 0;
        assert!(args.validate().is_ok(), "CPU-only matching is allowed");
    }

    #[test]
    fn args_reject_invalid_thresholds() {
        let mut args = reap_args();

        assert!(args.validate().is_ok());

        args.min_cpu = f32::NAN;
        assert!(args.validate().is_err());

        args.min_cpu = -1.0;
        assert!(args.validate().is_err());

        args.min_cpu = 10.0;
        args.min_age = 0;
        assert!(args.validate().is_err());
    }
}
