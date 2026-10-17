//! Disk usage accounting for cleanup reporting.
//!
//! Everything `coffee` deletes it also measures, so the summary it prints is a
//! real byte count rather than an estimate. Directory sizes are computed with
//! `std::fs` only; `du` is never spawned.

use crate::ui;
use colored::Colorize;
use std::path::{Path, PathBuf};

/// Files smaller than this are counted but not walked into, to keep the walk
/// cheap on trees with many tiny files.
const MIN_REPORTABLE: u64 = 1024 * 1024;

/// A single row of a usage report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    pub path: PathBuf,
    pub bytes: u64,
}

impl Usage {
    pub fn new(path: impl Into<PathBuf>, bytes: u64) -> Self {
        Self {
            path: path.into(),
            bytes,
        }
    }
}

/// Free space on each mounted writable volume, keyed by mount point.
pub fn free_space() -> Vec<(String, u64)> {
    crate::doctor::volumes()
        .into_iter()
        .map(|volume| (volume.mount_point, volume.available))
        .collect()
}

/// Total free space across all writable volumes.
pub fn total_free() -> u64 {
    free_space().into_iter().map(|(_, free)| free).sum()
}

/// Recursive size of a path. Symlinks are measured, never followed, so a link
/// into a large tree cannot inflate the result or loop forever.
pub fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };

    if !meta.is_dir() || meta.file_type().is_symlink() {
        return meta.len();
    }

    std::fs::read_dir(path)
        .map(|entries| entries.flatten().map(|e| dir_size(&e.path())).sum())
        .unwrap_or(0)
}

/// Immediate children of `root`, largest first.
///
/// This is the "what is taking up space" view: one level deep, so the answer is
/// actionable rather than a list of individual files.
pub fn largest_children(root: &Path, limit: usize) -> Vec<Usage> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };

    let mut children: Vec<Usage> = entries
        .flatten()
        .map(|entry| Usage::new(entry.path(), dir_size(&entry.path())))
        .filter(|usage| usage.bytes >= MIN_REPORTABLE)
        .collect();

    children.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.path.cmp(&b.path)));
    children.truncate(limit);
    children
}

/// Rows for the biggest immediate children of each of `roots`.
pub fn usage_rows(roots: &[PathBuf], limit: usize) -> Vec<Usage> {
    let mut rows = Vec::new();
    for root in roots {
        rows.extend(largest_children(root, limit));
    }
    rows.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.path.cmp(&b.path)));
    rows
}

/// Render a table of `path | size`, right-aligned, descending.
pub fn render_table(rows: &[Usage], limit: usize) {
    let table_rows: Vec<Vec<String>> = rows
        .iter()
        .take(limit)
        .map(|usage| {
            vec![
                usage.path.display().to_string(),
                ui::human_bytes(usage.bytes),
            ]
        })
        .collect();

    if table_rows.is_empty() {
        println!("{}", "  (nothing above 1 MiB)".dimmed());
        return;
    }

    ui::table(
        &["Path", "Size"],
        &table_rows,
        &[ui::Align::Left, ui::Align::Right],
    );
}

#[cfg(test)]
mod tests {
    use super::{Usage, dir_size, largest_children, usage_rows};
    use std::path::{Path, PathBuf};

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("coffee-space-test-{}-{name}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    #[test]
    fn dir_size_sums_nested_files() {
        let dir = scratch("sizes-nested");
        std::fs::create_dir_all(dir.join("a/b")).expect("nested dirs");
        std::fs::write(dir.join("a/b/file"), vec![0u8; 2048]).expect("write file");
        std::fs::write(dir.join("top"), vec![0u8; 1024]).expect("write file");

        assert_eq!(dir_size(&dir), 3072);
        std::fs::remove_dir_all(&dir).expect("clean up");
    }

    #[test]
    fn dir_size_of_missing_path_is_zero() {
        assert_eq!(dir_size(Path::new("/definitely/not/here")), 0);
    }

    #[test]
    fn dir_size_does_not_follow_symlinks() {
        let dir = scratch("sizes-symlink");
        let big = dir.join("big");
        std::fs::create_dir_all(&big).expect("create big dir");
        std::fs::write(big.join("filler"), vec![0u8; 8192]).expect("write filler");

        std::os::unix::fs::symlink(&big, dir.join("link")).expect("symlink");

        // The symlink itself is negligible; following it would double the total.
        let size = dir_size(&dir);
        assert!(size < 16_384, "symlink was followed, size={size}");

        std::fs::remove_dir_all(&dir).expect("clean up");
    }

    #[test]
    fn largest_children_sorts_descending_and_skips_tiny_entries() {
        let dir = scratch("largest-children");
        std::fs::create_dir_all(dir.join("medium")).expect("mkdir");
        std::fs::create_dir_all(dir.join("small")).expect("mkdir");
        std::fs::write(dir.join("medium/f"), vec![0u8; 4 * 1024 * 1024]).expect("write");
        std::fs::write(dir.join("small/f"), vec![0u8; 2 * 1024 * 1024]).expect("write");
        std::fs::write(dir.join("tiny"), b"x").expect("write tiny");

        let rows = largest_children(&dir, 10);
        let names: Vec<String> = rows
            .iter()
            .map(|u| u.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();

        assert_eq!(names, ["medium", "small"], "must be sorted and skip tiny");
        assert_eq!(rows[0].bytes, 4 * 1024 * 1024);

        std::fs::remove_dir_all(&dir).expect("clean up");
    }

    #[test]
    fn usage_rows_merges_roots_by_size() {
        let a = scratch("usage-a");
        let b = scratch("usage-b");
        std::fs::create_dir_all(a.join("x")).expect("mkdir");
        std::fs::write(a.join("x/f"), vec![0u8; 3 * 1024 * 1024]).expect("write");
        std::fs::write(b.join("y"), vec![0u8; 5 * 1024 * 1024]).expect("write");

        let roots = vec![a.clone(), b.clone()];
        let rows = usage_rows(&roots, 10);
        assert_eq!(rows[0].path.file_name().unwrap(), "y");
        assert_eq!(rows[0].bytes, 5 * 1024 * 1024);

        std::fs::remove_dir_all(&a).expect("clean a");
        std::fs::remove_dir_all(&b).expect("clean b");
    }

    #[test]
    fn usage_is_constructible_for_reporting() {
        let usage = Usage::new("/tmp/example", 42);
        assert_eq!(usage.bytes, 42);
        assert_eq!(usage.path, PathBuf::from("/tmp/example"));
    }
}
