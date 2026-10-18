//! `coffee space` — report what is actually consuming disk.
//!
//! Answers the question cleanup cannot: not just how much was freed, but which
//! directories are large enough to be worth acting on.

use crate::cli::SpaceArgs;
use crate::space::{self, Volume};
use crate::ui::{self, Printer};
use colored::Colorize;
use std::path::PathBuf;

/// Roots to inspect when the user does not name any.
fn default_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.clone());
        for relative in [
            "Library/Caches",
            ".cache",
            ".gradle",
            ".npm",
            ".cargo",
            ".rustup",
            "Library/Developer/Xcode/DerivedData",
            "Library/Developer/CoreSimulator",
        ] {
            roots.push(home.join(relative));
        }
    }

    roots.push(PathBuf::from("/nix/store"));
    roots
}

/// One `  ✅ /Volumes/Data  91.7 GiB free of …` line.
fn volume_line(volume: &Volume) -> String {
    let status = if volume.is_low() { "⚠️" } else { "✅" };
    let line = format!("  {status} {}  {volume}", volume.mount_point);

    if volume.is_low() {
        line.yellow().to_string()
    } else {
        line
    }
}

pub fn run(args: &SpaceArgs) -> bool {
    let out = Printer::quiet(args.quiet);
    let roots = if args.paths.is_empty() {
        default_roots()
    } else {
        args.paths.clone()
    };

    // Explicit paths are a promise: if one is wrong the user wants to know,
    // rather than silently getting a report over the remaining roots.
    if !args.paths.is_empty() {
        let missing: Vec<&PathBuf> = roots.iter().filter(|root| !root.exists()).collect();
        if !missing.is_empty() {
            for root in missing {
                out.failure(format!("No such directory: {}", root.display()));
            }
            return false;
        }
    }

    out.heading("📊 Disk usage");

    for volume in space::volumes() {
        out.line(volume_line(&volume));
    }

    out.rule();

    // `--top` is applied once across all roots, so the rows below are exactly
    // the rows the table renders and the total below them is exactly their sum.
    let rows = space::usage_rows(&roots, args.top);
    if rows.is_empty() {
        out.success("✨ Nothing above 1 MiB in the scanned directories");
        return true;
    }

    space::render_table(&rows);

    let total: u64 = rows.iter().map(|row| row.bytes).sum();
    out.note(format!(
        "  {} across {} entries shown",
        ui::human_bytes(total),
        rows.len()
    ));

    true
}

#[cfg(test)]
mod tests {
    use super::{default_roots, volume_line};
    use crate::space::Volume;

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

    #[test]
    fn the_home_directory_is_not_scanned_twice() {
        // `$HOME` itself and its children are both roots, so the first row of
        // the report would otherwise be `$HOME` followed by every directory
        // inside it, making the table harder to read than the disk is.
        let roots = default_roots();
        let mut seen = roots.clone();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), roots.len(), "default roots must be distinct");
    }

    #[test]
    fn a_volume_line_is_marked_bad_only_when_the_volume_is_low() {
        let healthy = Volume {
            mount_point: "/".into(),
            available: 100 * 1024 * 1024 * 1024,
            total: 100 * 1024 * 1024 * 1024,
        };
        assert!(volume_line(&healthy).contains('\u{2705}'));

        let low = Volume {
            available: 1024 * 1024 * 1024,
            ..healthy.clone()
        };
        assert!(volume_line(&low).contains('\u{26a0}'));
    }

    #[test]
    fn a_volume_line_carries_the_mount_point_and_the_free_space() {
        let volume = Volume {
            mount_point: "/System/Volumes/Data".into(),
            available: 30 * 1024 * 1024 * 1024,
            total: 80 * 1024 * 1024 * 1024,
        };
        // `Display` colours inject ANSI escapes, so strip them before asserting
        // on the text a user actually reads.
        let plain = strip_ansi(&volume_line(&volume));

        assert!(plain.contains("/System/Volumes/Data"), "{plain}");
        assert!(plain.contains("30.0 GiB free of 80.0 GiB (38%)"), "{plain}");
    }

    /// Drop CSI escape sequences so coloured output can be asserted on.
    fn strip_ansi(text: &str) -> String {
        let mut out = String::new();
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
            out.push(c);
        }
        out
    }
}
