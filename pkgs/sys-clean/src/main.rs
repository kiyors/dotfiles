use clap::{Parser, Subcommand};
use colored::*;
use std::process::Command;
use sysinfo::System;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Kill orphaned node processes that are eating CPU
    ReapZombies {
        /// Target strings to match in the command line (can provide multiple)
        #[arg(short, long, default_value = "jest-worker")]
        targets: Vec<String>,

        /// If set, only prints what would be killed without actually killing
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Run system cleanup (nix store, etc.)
    SystemCleanup,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::ReapZombies { targets, dry_run } => reap_zombies(targets, *dry_run),
        Commands::SystemCleanup => system_cleanup(),
    }
}

fn send_macos_notification(title: &str, message: &str) {
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        message, title
    );
    let _ = Command::new("osascript").arg("-e").arg(&script).status();
}

fn reap_zombies(targets: &[String], dry_run: bool) {
    println!("{}", "🧟 Starting Zombie Reaper...".cyan().bold());

    let mut sys = System::new_all();
    sys.refresh_all();

    // Refresh again slightly later to get accurate CPU usage readings
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_all();

    let mut killed_count = 0;

    for (pid, process) in sys.processes() {
        if process.parent().is_some_and(|p| p.as_u32() == 1) {
            let cmd: Vec<String> = process
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect();
            let cmd_str = cmd.join(" ");

            let matches_target = targets.iter().any(|t| cmd_str.contains(t));

            if matches_target {
                let cpu_usage = process.cpu_usage();
                println!(
                    "{} {} (PID: {}) using {} CPU",
                    "⚠️  Found zombie process:".yellow(),
                    process.name().to_string_lossy().bold(),
                    pid.to_string().cyan(),
                    format!("{:.1}%", cpu_usage).red().bold()
                );

                if dry_run {
                    println!(
                        "   {} Would have killed zombie process (PID: {})",
                        "[DRY RUN]".magenta().bold(),
                        pid.to_string().cyan()
                    );
                } else if process.kill() {
                    println!(
                        "   {} Successfully killed zombie process (PID: {})",
                        "✅".green(),
                        pid.to_string().cyan()
                    );
                    killed_count += 1;
                } else {
                    eprintln!(
                        "   {} Failed to kill zombie process (PID: {})",
                        "❌".red(),
                        pid.to_string().cyan()
                    );
                }
            }
        }
    }

    println!("{}", "─".repeat(50).bright_black());
    if killed_count > 0 {
        println!(
            "{} Reaped {} zombie(s).",
            "✨".green(),
            killed_count.to_string().bold()
        );
        send_macos_notification(
            "System Maintainer",
            &format!("Reaped {} orphaned background process(es) 🧟", killed_count),
        );
    } else {
        println!(
            "{} No zombies found matching targets: {:?}",
            "💤".blue(),
            targets
        );
    }
}

fn system_cleanup() {
    println!(
        "{}",
        "🧹 Running system garbage collection...".cyan().bold()
    );
    println!("{}", "─".repeat(50).bright_black());

    let cmds = vec![
        ("nh", vec!["clean", "darwin"]),
        ("nh", vec!["clean", "all"]),
        ("mo", vec!["clean"]),
    ];

    for (cmd, args) in cmds {
        println!("{} {} {:?}", "▶".blue(), cmd.bold(), args);
        match Command::new(cmd).args(&args).status() {
            Ok(status) if status.success() => {
                println!(
                    "  {} Successfully executed {} {:?}",
                    "✅".green(),
                    cmd.bold(),
                    args
                );
            }
            Ok(status) => {
                eprintln!(
                    "  {} Command {} {:?} exited with status: {}",
                    "❌".red(),
                    cmd.bold(),
                    args,
                    status.to_string().red()
                );
            }
            Err(e) => {
                eprintln!(
                    "  {} Failed to execute {} {:?}: {}",
                    "💥".red().bold(),
                    cmd.bold(),
                    args,
                    e.to_string().red()
                );
            }
        }
    }

    println!("{}", "─".repeat(50).bright_black());
    println!("{}", "✨ System cleanup complete.".green().bold());
}
