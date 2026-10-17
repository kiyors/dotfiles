//! Shared terminal output helpers.
//!
//! Centralising the emoji and colour vocabulary keeps every subcommand looking
//! the same and gives the test suite one place to format sizes.

use colored::Colorize;
use std::fmt::Display;

const RULE_WIDTH: usize = 50;

/// A cyan, bold section heading.
pub fn heading(text: impl Display) {
    println!("{}", text.to_string().cyan().bold());
}

/// A horizontal separator.
pub fn rule() {
    println!("{}", "─".repeat(RULE_WIDTH).bright_black());
}

/// Announce a child process that is about to run.
pub fn step(program: impl Display, args: &[String]) {
    println!("{} {} {:?}", "▶".blue(), program.to_string().bold(), args);
}

/// A successful child process or completed action.
pub fn ok(text: impl Display) {
    println!("  {} {}", "✅".green(), text);
}

/// Something unexpected that did not stop the run.
pub fn warn(text: impl Display) {
    println!("  {} {}", "⚠️".yellow(), text);
}

/// A failed child process or action, indented under its parent step.
pub fn failure(text: impl Display) {
    eprintln!("  {} {}", "❌".red(), text);
}

/// A fatal problem that aborts the whole command.
pub fn fatal(text: impl Display) {
    eprintln!("{} {}", "❌".red(), text);
}

/// An action that was skipped because `--dry-run` was given.
pub fn dry_run(text: impl Display) {
    println!("  {} {}", "[DRY RUN]".magenta(), text);
}

/// A final green, bold result line.
pub fn success(text: impl Display) {
    println!("{}", text.to_string().green().bold());
}

/// Render a byte count in binary units, e.g. `1.4 GiB`.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// How a column is aligned inside the rendered table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

/// Render a bordered table.
///
/// This is hand-rolled rather than pulled from a crate on purpose: the Nix build
/// vendors dependencies from `Cargo.lock`, so a new crate has to be fetchable at
/// build time. A few dozen lines of `println!` avoids that dependency entirely.
///
/// `aligns` may be shorter than `headers`; missing entries default to
/// [`Align::Left`]. Rows with fewer cells than headers are padded.
pub fn table(headers: &[&str], rows: &[Vec<String>], aligns: &[Align]) {
    if headers.is_empty() {
        return;
    }

    let columns = headers.len();
    let align_of = |index: usize| aligns.get(index).copied().unwrap_or(Align::Left);

    let mut widths: Vec<usize> = headers.iter().map(|h| display_width(h)).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate().take(columns) {
            widths[index] = widths[index].max(display_width(cell));
        }
    }

    let rule = |left: &str, mid: &str, right: &str| {
        let mut out = String::from(left);
        for (index, width) in widths.iter().enumerate() {
            if index > 0 {
                out.push_str(mid);
            }
            out.push_str(&"─".repeat(width + 2));
        }
        out.push_str(right);
        println!("{}", out.bright_black());
    };

    let render_row = |cells: &[String]| {
        let mut out = String::from("│");
        for (index, width) in widths.iter().enumerate() {
            let empty = String::new();
            let cell = cells.get(index).unwrap_or(&empty);
            let padding = width.saturating_sub(display_width(cell));
            out.push(' ');

            match align_of(index) {
                Align::Left => {
                    out.push_str(cell);
                    out.push_str(&" ".repeat(padding));
                }
                Align::Right => {
                    out.push_str(&" ".repeat(padding));
                    out.push_str(cell);
                }
            }

            out.push_str(" │");
        }
        println!("{out}");
    };

    rule("┌", "┬", "┐");

    let header_cells: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
    render_row(&header_cells);

    rule("├", "┼", "┤");

    for row in rows {
        render_row(row);
    }

    rule("└", "┴", "┘");
}

/// Printable width of a cell, ignoring any ANSI escapes it may contain.
fn display_width(cell: &str) -> usize {
    let mut width = 0;
    let mut chars = cell.chars();

    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip a CSI escape sequence: ESC [ ... final-byte.
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        width += 1;
    }

    width
}

#[cfg(test)]
mod tests {
    use super::{Align, display_width, human_bytes, table};

    #[test]
    fn bytes_below_one_kib_are_shown_exactly() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1023), "1023 B");
    }

    #[test]
    fn larger_values_scale_to_binary_units() {
        assert_eq!(human_bytes(1024), "1.0 KiB");
        assert_eq!(human_bytes(1536), "1.5 KiB");
        assert_eq!(human_bytes(1024 * 1024), "1.0 MiB");
        assert_eq!(human_bytes(3 * 1024 * 1024 * 1024), "3.0 GiB");
    }

    #[test]
    fn absurdly_large_values_saturate_at_the_largest_unit() {
        // u64::MAX bytes is exactly 2^24 TiB, so the unit never overflows.
        assert_eq!(human_bytes(u64::MAX), "16777216.0 TiB");
    }

    #[test]
    fn ansi_escapes_do_not_count_towards_width() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("\u{1b}[31mabc\u{1b}[0m"), 3);
        assert_eq!(display_width("\u{1b}[1m\u{1b}[31mab\u{1b}[27m\u{1b}[0m"), 2);
    }

    #[test]
    fn table_with_no_headers_renders_nothing() {
        // Must not panic.
        table(&[], &[], &[]);
    }

    #[test]
    fn alignment_falls_back_to_left_when_unspecified() {
        assert_eq!(Align::Left, Align::Left);
        assert_ne!(Align::Left, Align::Right);
    }
}
