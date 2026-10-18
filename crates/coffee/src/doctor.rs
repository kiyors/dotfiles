//! `coffee doctor` — a read-only health report for this machine's maintenance
//! setup.
//!
//! Everything here is non-destructive. It answers the questions you otherwise
//! have to guess at: is the Nix store running out of room, are the launchd jobs
//! actually installed *and* loaded, and are there stale processes waiting to be
//! reaped?

use crate::cli::{DoctorArgs, ReapArgs};
use crate::cmd;
use crate::reap;
use crate::space;
use crate::tmp;
use crate::tool;
use crate::ui::{self, Printer};
use colored::Colorize;
use std::path::PathBuf;
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Ok,
    Info,
    Warn,
    Fail,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Ok => "✅",
            Status::Info => "ℹ️",
            Status::Warn => "⚠️",
            Status::Fail => "❌",
        }
    }

    /// Informational notes are not a reason to call a machine unhealthy.
    fn is_problem(self) -> bool {
        matches!(self, Status::Warn | Status::Fail)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Host,
    Disks,
    Tools,
    Mole,
    Tmp,
    Agents,
    Reap,
}

const SECTIONS: [Section; 7] = [
    Section::Host,
    Section::Disks,
    Section::Tools,
    Section::Mole,
    Section::Tmp,
    Section::Agents,
    Section::Reap,
];

#[derive(Debug)]
struct Check {
    section: Section,
    label: String,
    status: Status,
    detail: String,
}

/// Accumulates checks so the summary is derived rather than tracked twice.
#[derive(Debug, Default)]
struct Report {
    checks: Vec<Check>,
}

impl Report {
    fn push(
        &mut self,
        section: Section,
        label: impl Into<String>,
        status: Status,
        detail: impl Into<String>,
    ) {
        self.checks.push(Check {
            section,
            label: label.into(),
            status,
            detail: detail.into(),
        });
    }

    fn in_section(&self, section: Section) -> Vec<&Check> {
        self.checks
            .iter()
            .filter(|check| check.section == section)
            .collect()
    }

    fn count(&self, status: Status) -> usize {
        self.checks.iter().filter(|c| c.status == status).count()
    }

    fn problems(&self) -> usize {
        self.checks.iter().filter(|c| c.status.is_problem()).count()
    }

    fn is_healthy(&self) -> bool {
        self.problems() == 0
    }
}

struct ToolCheck {
    name: &'static str,
    required: bool,
}

const TOOLS: &[ToolCheck] = &[
    ToolCheck {
        name: "nix",
        required: true,
    },
    ToolCheck {
        name: "nh",
        required: true,
    },
    ToolCheck {
        name: "mo",
        required: false,
    },
    ToolCheck {
        name: "brew",
        required: false,
    },
    ToolCheck {
        name: "launchctl",
        required: false,
    },
];

struct AgentCheck {
    /// Short name used in the report.
    name: &'static str,
    /// Full launchd label, which nix-darwin prefixes with `org.nixos.`.
    label: &'static str,
    plist: &'static str,
    /// `gui` agents need the numeric uid appended to the domain.
    domain: &'static str,
}

const AGENTS: &[AgentCheck] = &[
    AgentCheck {
        name: "zombie-reaper",
        label: "org.nixos.zombie-reaper",
        plist: "Library/LaunchAgents/org.nixos.zombie-reaper.plist",
        domain: "gui",
    },
    AgentCheck {
        name: "system-cleanup",
        label: "org.nixos.system-cleanup",
        plist: "/Library/LaunchDaemons/org.nixos.system-cleanup.plist",
        domain: "system",
    },
];

/// Numeric user id, needed to address the `gui/<uid>` launchd domain.
fn uid() -> Option<String> {
    cmd::first_line("id", &["-u"])
}

/// The `state = …` value from `launchctl print` output.
///
/// `launchctl print` exits 0 for any service launchd knows about, loaded or
/// not, so its exit status alone cannot tell "running" from "registered but
/// stopped". Reading `state` is what actually distinguishes them, and this runs
/// under `--quiet` in launchd jobs, so the dump is captured rather than printed.
fn launchd_state(target: &str) -> Option<String> {
    cmd::capture("launchctl", &["print", target])
        .and_then(|dump| {
            dump.lines()
                .filter_map(|line| line.trim().strip_prefix("state = "))
                .map(|state| state.trim().trim_matches('"').to_string())
                .next()
        })
        .filter(|state| !state.is_empty())
}

/// Whether a launchd agent's state means it is actually doing its job.
///
/// A `StartInterval` job only runs during its slot, so `waiting` is its normal
/// resting state. `not running` on an installed agent means it was never
/// successfully started, which is worth surfacing differently.
fn is_live_state(state: Option<&str>) -> bool {
    matches!(state, Some("running" | "waiting" | "active" | "kickstart"))
}

fn check_disks(report: &mut Report) {
    for volume in space::volumes() {
        let status = if volume.is_low() {
            Status::Warn
        } else {
            Status::Ok
        };

        report.push(
            Section::Disks,
            volume.mount_point.clone(),
            status,
            volume.to_string(),
        );
    }
}

/// Classify one external tool: is it present, and does it report a version?
fn tool_status(name: &str, required: bool) -> (Status, String) {
    if let Some(version) = tool::version_of(name) {
        return (Status::Ok, version);
    }

    if tool::which(name).is_some() {
        return (Status::Ok, "installed (version unavailable)".to_string());
    }

    if required {
        (
            Status::Fail,
            "not found on PATH — required by this tool".to_string(),
        )
    } else {
        (Status::Info, "not installed (optional)".to_string())
    }
}

fn check_tools(report: &mut Report) {
    for entry in TOOLS {
        let (status, detail) = tool_status(entry.name, entry.required);
        report.push(Section::Tools, entry.name, status, detail);
    }
}

fn check_tmp(report: &mut Report) {
    let root = PathBuf::from("/tmp");
    let everything = tmp::scan(&root, 0);
    let eligible = tmp::scan(&root, tmp::DEFAULT_MIN_AGE);

    if everything.is_empty() {
        report.push(
            Section::Tmp,
            "clutter",
            Status::Ok,
            "/tmp has no stale items",
        );
        return;
    }

    report.push(
        Section::Tmp,
        "clutter",
        Status::Info,
        format!(
            "{} item(s) using {}; {} are older than {} and would be cleaned",
            everything.len(),
            ui::human_bytes(tmp::total_size(&everything)),
            eligible.len(),
            humanize_duration(tmp::DEFAULT_MIN_AGE),
        ),
    );
}

fn check_agents(report: &mut Report) {
    let home = std::env::var("HOME").unwrap_or_default();

    for agent in AGENTS {
        let plist = if agent.plist.starts_with('/') {
            PathBuf::from(agent.plist)
        } else {
            PathBuf::from(&home).join(agent.plist)
        };

        if !plist.is_file() {
            report.push(
                Section::Agents,
                agent.name,
                Status::Info,
                format!("not installed (no {})", plist.display()),
            );
            continue;
        }

        // `gui` agents are addressed as `gui/<uid>/<label>`, and without the
        // numeric uid there is nothing to ask launchd about.
        let target = match (agent.domain, uid()) {
            ("gui", Some(uid)) => format!("gui/{uid}/{}", agent.label),
            ("gui", None) => {
                report.push(
                    Section::Agents,
                    agent.name,
                    Status::Warn,
                    "installed, but the user id could not be read",
                );
                continue;
            }
            (domain, _) => format!("{domain}/{}", agent.label),
        };

        let state = launchd_state(&target);

        let (status, detail) = match (state.as_deref(), is_live_state(state.as_deref())) {
            (Some(state), true) => (Status::Ok, format!("installed and {state} ({target})")),
            // Registered with launchd but not running right now. For a
            // `StartInterval` agent this is the normal state between runs, so
            // it stays informational rather than becoming a warning.
            (Some(state), false) => (
                Status::Info,
                format!("installed at {} but {state}", plist.display()),
            ),
            // The plist is on disk yet launchd has never heard of it: the agent
            // was never bootstrapped, or its name does not match the label.
            (None, _) => (
                Status::Warn,
                format!("installed at {} but unknown to launchd", plist.display()),
            ),
        };

        report.push(Section::Agents, agent.name, status, detail);
    }
}

fn check_reap_candidates(report: &mut Report, out: &Printer) {
    // The policy is shared with modules/darwin/zombie-reaper.nix via
    // `ReapArgs::background_agent_policy`, so this reports what the 30-minute
    // agent would actually reap rather than a looser approximation of it.
    let args = ReapArgs::background_agent_policy();

    let sampled = reap::sample_processes();
    let candidates = reap::find_candidates(&sampled, &args);

    report.push(
        Section::Reap,
        "candidates",
        if candidates.is_empty() {
            Status::Ok
        } else {
            Status::Info
        },
        format!(
            "{} process(es) match the background reaper policy (age ≥ {}s, CPU ≥ {:.0}%, memory ≥ {}, targets: {})",
            candidates.len(),
            args.min_age,
            args.min_cpu,
            ui::human_bytes(args.min_memory),
            args.targets.len(),
        ),
    );

    for candidate in candidates {
        out.line(format!(
            "{} {} (PID {}, age {}s, CPU {:.1}%, memory {})",
            "⚠️  Would reap:".yellow(),
            candidate.name.bold(),
            candidate.pid.to_string().cyan(),
            candidate.age,
            candidate.cpu_usage,
            ui::human_bytes(candidate.memory)
        ));
    }
}

fn check_mole(report: &mut Report) {
    if tool::which("mo").is_none() {
        report.push(
            Section::Mole,
            "mole",
            Status::Info,
            "not installed (optional)",
        );
        return;
    }

    report.push(Section::Mole, "mole", Status::Ok, "installed");

    let Some(home) = std::env::var("HOME").ok() else {
        report.push(
            Section::Mole,
            "whitelist",
            Status::Warn,
            "HOME is unset, so the Mole config cannot be located",
        );
        return;
    };

    let config_dir = PathBuf::from(&home).join(".config/mole");

    // `mo clean` empties the Trash unless the whitelist protects it, and
    // `coffee clean --deep` runs `mo clean`. An empty whitelist means the
    // Trash has no extra protection, so this is worth calling out.
    match std::fs::read_to_string(config_dir.join("whitelist")) {
        Ok(contents) => {
            let protected = contents
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .count();

            if protected == 0 {
                report.push(
                    Section::Mole,
                    "whitelist",
                    Status::Warn,
                    "no protected paths — `mo clean` will empty the Trash",
                );
            } else {
                report.push(
                    Section::Mole,
                    "whitelist",
                    Status::Ok,
                    format!("{protected} protected path(s)"),
                );
            }
        }
        Err(_) => report.push(
            Section::Mole,
            "whitelist",
            Status::Info,
            format!("no whitelist at {}", config_dir.join("whitelist").display()),
        ),
    }

    // Purge has no whitelist; the only guard is which directories get scanned.
    match std::fs::read_to_string(config_dir.join("purge_paths")) {
        Ok(contents) => {
            let roots: Vec<&str> = contents
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .collect();

            if roots.is_empty() {
                report.push(
                    Section::Mole,
                    "purge paths",
                    Status::Warn,
                    "none configured — `mo purge` would fall back to its default scan roots",
                );
            } else {
                report.push(Section::Mole, "purge paths", Status::Ok, roots.join(", "));
            }
        }
        Err(_) => report.push(
            Section::Mole,
            "purge paths",
            Status::Warn,
            "not configured — `mo purge` uses its default scan roots",
        ),
    }
}

/// Produce and print the report. Returns true when nothing warned or failed.
pub fn run(args: &DoctorArgs) -> bool {
    let out = Printer::quiet(args.quiet);
    let mut report = Report::default();

    report.push(
        Section::Host,
        "coffee",
        Status::Ok,
        format!(
            "{} {} on {} ({})",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
    );

    report.push(
        Section::Host,
        "uptime",
        Status::Ok,
        format!("{} uptime", humanize_duration(System::uptime())),
    );

    check_disks(&mut report);
    check_tools(&mut report);
    check_mole(&mut report);
    check_tmp(&mut report);
    check_agents(&mut report);

    if args.reap_candidates {
        check_reap_candidates(&mut report, &out);
    }

    print_report(&report, &out);

    let healthy = report.is_healthy();
    if !healthy {
        out.failure(format!(
            "{} problem(s) found — see the warnings above.",
            report.problems()
        ));
    }

    healthy || !args.strict
}

fn print_report(report: &Report, out: &Printer) {
    let quiet = out.is_quiet();
    out.heading("☕ coffee doctor");

    for section in SECTIONS {
        let checks = report.in_section(section);
        if checks.is_empty() || (quiet && checks.iter().all(|c| !c.status.is_problem())) {
            continue;
        }

        for check in checks {
            if quiet && !check.status.is_problem() {
                continue;
            }
            out.line(format!(
                "{} {}: {}",
                check.status.label(),
                check.label,
                check.detail.dimmed()
            ));
        }
    }

    if quiet {
        return;
    }

    out.rule();
    let summary = format!(
        "{} passed, {} info, {} warning(s), {} failure(s)",
        report.count(Status::Ok),
        report.count(Status::Info),
        report.count(Status::Warn),
        report.count(Status::Fail)
    );

    if report.is_healthy() {
        out.success(format!("✨ {summary}"));
    } else {
        out.line(summary.yellow());
    }
}

/// Render a second count as `3d 4h 5m`.
fn humanize_duration(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if days > 0 || hours > 0 {
        parts.push(format!("{hours}h"));
    }
    parts.push(format!("{minutes}m"));
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::{Report, Section, Status, humanize_duration};
    use crate::space;

    #[test]
    fn durations_drop_empty_leading_units() {
        assert_eq!(humanize_duration(0), "0m");
        assert_eq!(humanize_duration(90), "1m");
        assert_eq!(humanize_duration(3_600), "1h 0m");
        assert_eq!(humanize_duration(90_061), "1d 1h 1m");
    }

    #[test]
    fn informational_checks_are_not_problems() {
        let mut report = Report::default();
        assert!(report.is_healthy());

        report.push(Section::Host, "x", Status::Info, "note");
        assert!(report.is_healthy(), "informational notes are not unhealthy");

        report.push(Section::Host, "y", Status::Warn, "careful");
        assert!(!report.is_healthy());
        assert_eq!(report.problems(), 1);
    }

    #[test]
    fn a_failure_is_a_problem() {
        let mut report = Report::default();
        report.push(Section::Tools, "nix", Status::Fail, "missing");
        assert!(!report.is_healthy());
        assert_eq!(report.count(Status::Fail), 1);
    }

    #[test]
    fn checks_are_grouped_by_section() {
        let mut report = Report::default();
        report.push(Section::Tools, "nix", Status::Ok, "2.x");
        report.push(Section::Tools, "nh", Status::Ok, "4.x");
        report.push(Section::Agents, "zombie-reaper", Status::Info, "absent");

        assert_eq!(report.in_section(Section::Tools).len(), 2);
        assert_eq!(report.in_section(Section::Agents).len(), 1);
        assert_eq!(report.in_section(Section::Disks).len(), 0);
    }

    #[test]
    fn a_stopped_but_registered_agent_is_not_reported_as_broken() {
        use super::is_live_state;

        // A `StartInterval` agent spends most of its life waiting. Reporting
        // that as a warning would make `doctor --strict` permanently fail.
        assert!(is_live_state(Some("running")));
        assert!(is_live_state(Some("waiting")));
        assert!(is_live_state(Some("active")));

        assert!(!is_live_state(Some("not running")));
        assert!(!is_live_state(None), "an unknown state is not a live state");
    }

    #[test]
    fn the_disk_thresholds_are_documented_where_they_are_enforced() {
        // These moved to `space`, next to the code that actually applies them.
        assert_eq!(crate::ui::human_bytes(space::LOW_FREE_BYTES), "20.0 GiB");
        assert!((space::LOW_FREE_FRACTION - 0.10).abs() < f64::EPSILON);
    }
}
