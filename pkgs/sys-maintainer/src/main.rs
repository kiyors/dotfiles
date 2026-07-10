use clap::{Parser, Subcommand};
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
        /// Target string to match in the command line (default: "jest-worker")
        #[arg(short, long, default_value = "jest-worker")]
        target: String,
    },
    /// Run system cleanup (nix store, etc.)
    SystemCleanup,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::ReapZombies { target } => reap_zombies(target),
        Commands::SystemCleanup => system_cleanup(),
    }
}

fn reap_zombies(target: &str) {
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
            if cmd_str.contains(target) {
                let cpu_usage = process.cpu_usage();
                println!(
                    "Found zombie process: {} (PID: {}) using {:.1}% CPU",
                    process.name().to_string_lossy(),
                    pid,
                    cpu_usage
                );
                if process.kill() {
                    println!("Successfully killed zombie process (PID: {})", pid);
                    killed_count += 1;
                } else {
                    eprintln!("Failed to kill zombie process (PID: {})", pid);
                }
            }
        }
    }

    if killed_count == 0 {
        println!("No zombie '{}' processes found.", target);
    } else {
        println!("Reaped {} zombie(s).", killed_count);
    }
}

fn system_cleanup() {
    println!("Running system garbage collection...");

    let cmds = vec![
        ("nh", vec!["clean", "darwin"]),
        ("nh", vec!["clean", "all"]),
        ("mo", vec!["clean"]),
    ];

    for (cmd, args) in cmds {
        println!("Executing: {} {:?}", cmd, args);
        match Command::new(cmd).args(&args).status() {
            Ok(status) if status.success() => {
                println!("Successfully executed {} {:?}", cmd, args);
            }
            Ok(status) => {
                eprintln!("Command {} {:?} exited with status: {}", cmd, args, status);
            }
            Err(e) => {
                eprintln!("Failed to execute {} {:?}: {}", cmd, args, e);
            }
        }
    }
    println!("System cleanup complete.");
}
