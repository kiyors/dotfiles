//! `coffee space` — report what is actually consuming disk.
//!
//! Answers the question cleanup cannot: not just how much was freed, but which
//! directories are large enough to be worth acting on.

use crate::cli::SpaceArgs;
use crate::doctor;
use crate::space;
use crate::ui;
use colored::Colorize;
use std::path::PathBuf;

/// Roots to inspect when the user does not name any.
fn default_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.clone());
        roots.push(home.join("Library/Caches"));
        roots.push(home.join(".cache"));
        roots.push(home.join(".gradle"));
        roots.push(home.join(".npm"));
        roots.push(home.join(".cargo"));
    }

    roots.push(PathBuf::from("/nix/store"));
    roots
}

pub fn run(args: &SpaceArgs) -> bool {
    let roots = if args.paths.is_empty() {
        default_roots()
    } else {
        args.paths.clone()
    };

    let missing: Vec<&PathBuf> = roots.iter().filter(|root| !root.exists()).collect();
    if !missing.is_empty() && !args.paths.is_empty() {
        for root in missing {
            ui::failure(format!("No such directory: {}", root.display()));
        }
        return false;
    }

    if !args.quiet {
        ui::heading("📊 Disk usage");
    }

    for volume in doctor::volumes() {
        let low = volume.is_low();
        let status = if low { "⚠️" } else { "✅" };
        let line = format!(
            "  {status} {}  {} free of {} ({:.0}%)",
            volume.mount_point,
            ui::human_bytes(volume.available),
            ui::human_bytes(volume.total),
            volume.free_fraction() * 100.0
        );
        if low {
            println!("{}", line.yellow());
        } else {
            println!("{line}");
        }
    }

    if !args.quiet {
        ui::rule();
    }

    let rows = space::usage_rows(&roots, args.top);
    if rows.is_empty() {
        if !args.quiet {
            ui::success("✨ Nothing above 1 MiB in the scanned directories");
        }
        return true;
    }

    space::render_table(&rows, args.top);

    let total: u64 = rows.iter().map(|row| row.bytes).sum();
    if !args.quiet {
        println!(
            "{}",
            format!(
                "  {} across {} entries shown",
                ui::human_bytes(total),
                rows.len().min(args.top)
            )
            .dimmed()
        );
    }

    true
}

#[cfg(test)]
mod tests {
    use super::default_roots;

    #[test]
    fn default_roots_are_absolute_and_include_the_nix_store() {
        let roots = default_roots();
        assert!(
            roots.iter().all(|root| root.is_absolute()),
            "every default root must be absolute"
        );
        assert!(
            roots.iter().any(|root| root == "/nix/store"),
            "the nix store should be reported by default"
        );
    }
}
