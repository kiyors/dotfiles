//! `coffee doctor` — a read-only health report for this machine's maintenance
//! setup.
//!
//! Everything here is non-destructive. It answers the questions you otherwise
//! have to guess at: is the Nix store running out of room, are the launchd jobs
//! actually installed *and* loaded, and are there stale processes waiting to be
//! reaped?

use crate::cli::{DoctorArgs, ReapArgs};
use crate::reap;
use crate::tmp;
use crate::tool;
use crate::ui;
use colored::Colorize;
use std::path::PathBuf;
use sysinfo::{Disks, System};

/// Free-space fraction below which a volume is called out.
const DISK_WARN_FRACTION: f64 = 0.10;
/// Absolute free-space floor, independent of volume size.
const DISK_WARN_BYTES: u64 = 20 * 1024 * 1024 * 1024;

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
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

/// A mounted volume's space situation.
#[derive(Debug, Clone)]
pub struct Volume {
    pub mount_point: String,
    pub available: u64,
    pub total: u64,
}

impl Volume {
    /// Fraction of the volume that is free.
    pub fn free_fraction(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.available as f64 / self.total as f64
    }

    /// Whether this volume is running low enough to be worth reporting.
    pub fn is_low(&self) -> bool {
        self.free_fraction() < DISK_WARN_FRACTION || self.available < DISK_WARN_BYTES
    }
}

/// Every writable volume, skipping APFS read-only snapshots that merely mirror
/// the data volume.
pub fn volumes() -> Vec<Volume> {
    Disks::new_with_refreshed_list()
        .list()
        .iter()
        .filter(|disk| !disk.is_read_only() && disk.total_space() > 0)
        .map(|disk| Volume {
            mount_point: disk.mount_point().display().to_string(),
            available: disk.available_space(),
            total: disk.total_space(),
        })
        .collect()
}

fn check_disks(report: &mut Report) {
    for volume in volumes() {
        let detail = format!(
            "{} free of {} ({:.0}%)",
            ui::human_bytes(volume.available),
            ui::human_bytes(volume.total),
            volume.free_fraction() * 100.0
        );

        let status = if volume.is_low() {
            Status::Warn
        } else {
            Status::Ok
        };

        report.push(Section::Disks, volume.mount_point, status, detail);
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

        let target = if agent.domain == "gui" {
            match uid() {
                Some(uid) => format!("gui/{uid}/{}", agent.label),
                None => {
                    report.push(
                        Section::Agents,
                        agent.name,
                        Status::Warn,
                        "installed, but the user id could not be read",
                    );
                    continue;
                }
            }
        } else {
            format!("{}/{}", agent.domain, agent.label)
        };

        let loaded = std::process::Command::new("launchctl")
            .arg("print")
            .arg(&target)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if loaded {
            report.push(
                Section::Agents,
                agent.name,
                Status::Ok,
                format!("installed and loaded ({target})"),
            );
        } else {
            report.push(
                Section::Agents,
                agent.name,
                Status::Warn,
                format!("installed at {} but not loaded", plist.display()),
            );
        }
    }
}

fn check_reap_candidates(report: &mut Report) {
    // The same policy modules/darwin/zombie-reaper.nix runs every 30 minutes.
    let args = ReapArgs {
        targets: vec!["jest-worker".into()],
        min_age: 900,
        min_cpu: 25.0,
        min_memory: 0,
        grace_period: 5,
        dry_run: true,
        quiet: true,
        no_notify: true,
    };

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
            "{} process(es) match the background reaper policy",
            candidates.len()
        ),
    );

    for candidate in candidates {
        println!(
            "{} {} (PID {}, age {}s, CPU {:.1}%)",
            "⚠️  Would reap:".yellow(),
            candidate.name.bold(),
            candidate.pid.to_string().cyan(),
            candidate.age,
            candidate.cpu_usage
        );
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
        check_reap_candidates(&mut report);
    }

    print_report(&report, args.quiet);

    let healthy = report.is_healthy();
    if !healthy {
        eprintln!(
            "{}",
            format!(
                "{} problem(s) found — see the warnings above.",
                report.problems()
            )
            .red()
            .bold()
        );
    }

    healthy || !args.strict
}

fn print_report(report: &Report, quiet: bool) {
    ui::heading("☕ coffee doctor");

    for section in SECTIONS {
        let checks = report.in_section(section);
        if checks.is_empty() || (quiet && checks.iter().all(|c| !c.status.is_problem())) {
            continue;
        }

        for check in checks {
            if quiet && !check.status.is_problem() {
                continue;
            }
            println!(
                "{} {}: {}",
                check.status.label(),
                check.label,
                check.detail.dimmed()
            );
        }
    }

    if quiet {
        return;
    }

    ui::rule();
    let summary = format!(
        "{} passed, {} info, {} warning(s), {} failure(s)",
        report.count(Status::Ok),
        report.count(Status::Info),
        report.count(Status::Warn),
        report.count(Status::Fail)
    );

    if report.is_healthy() {
        ui::success(format!("✨ {summary}"));
    } else {
        println!("{}", summary.yellow());
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
    use crate::ui::human_bytes;

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
    fn disk_warn_threshold_renders_in_binary_units() {
        assert_eq!(human_bytes(super::DISK_WARN_BYTES), "20.0 GiB");
    }
}
