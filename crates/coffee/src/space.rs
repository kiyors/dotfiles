//! Disk usage accounting for cleanup reporting.
//!
//! Everything `coffee` deletes it also measures, so the summary it prints is a
//! real byte count rather than an estimate. Directory sizes are computed with
//! `std::fs` only; `du` is never spawned.

use crate::ui;
use colored::Colorize;
use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use sysinfo::Disks;

/// Files smaller than this are counted but not walked into, to keep the walk
/// cheap on trees with many tiny files.
const MIN_REPORTABLE: u64 = 1024 * 1024;

/// A mounted volume's space situation.
///
/// This lives here rather than in `doctor` because both `space` and `doctor`
/// need it: it is disk accounting, not a health check. Keeping it in `doctor`
/// meant the accounting module had to depend on the reporting module, which is
/// backwards and made the dependency impossible to remove.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        self.free_fraction() < LOW_FREE_FRACTION || self.available < LOW_FREE_BYTES
    }
}

impl Display for Volume {
    /// `91.7 GiB free of 228.3 GiB (40%)`.
    ///
    /// A `Display` impl rather than a `format!` at each call site: `doctor` and
    /// `space` both report volumes, and they used to carry two copies of this
    /// exact string that would drift apart the moment either wording changed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} free of {} ({:.0}%)",
            ui::human_bytes(self.available),
            ui::human_bytes(self.total),
            self.free_fraction() * 100.0
        )
    }
}

/// Free-space fraction below which a volume is called out.
pub const LOW_FREE_FRACTION: f64 = 0.10;

/// Absolute free-space floor, independent of volume size.
pub const LOW_FREE_BYTES: u64 = 20 * 1024 * 1024 * 1024;

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
    volumes()
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

    std::fs::read_dir(path).map_or(0, |entries| {
        entries
            .flatten()
            .map(|entry| dir_size(&entry.path()))
            .sum::<u64>()
    })
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

/// Rows for the biggest immediate children of `roots`, largest first.
///
/// `limit` is applied **once, after merging**, not per root. Truncating per
/// root first is what previously let `space` print a headline total covering up
/// to `limit × roots` rows while showing only `limit` of them, so the summary
/// contradicted the table directly above it.
pub fn usage_rows(roots: &[PathBuf], limit: usize) -> Vec<Usage> {
    let mut rows: Vec<Usage> = roots
        .iter()
        .flat_map(|root| largest_children(root, usize::MAX))
        .collect();

    rows.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.path.cmp(&b.path)));
    rows.truncate(limit);
    rows
}

/// Render a table of `path | size`, right-aligned, descending.
pub fn render_table(rows: &[Usage]) {
    let table_rows: Vec<Vec<String>> = rows
        .iter()
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
    use super::{LOW_FREE_BYTES, Usage, Volume, dir_size, largest_children, usage_rows, volumes};
    use crate::ui::human_bytes;
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

    #[test]
    fn the_limit_applies_across_all_roots_not_per_root() {
        // Regression guard. `usage_rows` used to truncate to `limit` inside
        // each root and then merge, so 2 roots at `--top 2` produced 4 rows
        // while the table rendered 2 of them — and the reported total summed
        // all 4, so the headline number contradicted the table above it.
        let a = scratch("limit-a");
        let b = scratch("limit-b");
        for root in [&a, &b] {
            for index in 0..3 {
                let dir = root.join(format!("c{index}"));
                std::fs::create_dir_all(&dir).expect("mkdir");
                std::fs::write(dir.join("f"), vec![0u8; 4 * 1024 * 1024]).expect("write");
            }
        }

        let rows = usage_rows(&[a.clone(), b.clone()], 2);
        assert_eq!(rows.len(), 2, "--top is a total, not a per-root budget");

        // The headline total must be the sum of exactly the rows on screen.
        let total: u64 = rows.iter().map(|row| row.bytes).sum();
        assert_eq!(total, 2 * 4 * 1024 * 1024);

        std::fs::remove_dir_all(&a).expect("clean a");
        std::fs::remove_dir_all(&b).expect("clean b");
    }

    #[test]
    fn a_volume_renders_its_free_space_the_same_way_everywhere() {
        let volume = Volume {
            mount_point: "/".into(),
            available: 30 * 1024 * 1024 * 1024,
            total: 80 * 1024 * 1024 * 1024,
        };
        assert_eq!(volume.to_string(), "30.0 GiB free of 80.0 GiB (38%)");
    }

    #[test]
    fn a_zero_sized_volume_reports_zero_percent_instead_of_dividing_by_zero() {
        let volume = Volume {
            mount_point: "/".into(),
            available: 0,
            total: 0,
        };
        assert_eq!(volume.free_fraction(), 0.0);
        assert!(volume.is_low(), "an unreadable volume is not healthy");
        assert_eq!(volume.to_string(), "0 B free of 0 B (0%)");
    }

    #[test]
    fn the_volume_thresholds_render_in_binary_units() {
        assert_eq!(human_bytes(LOW_FREE_BYTES), "20.0 GiB");
    }

    #[test]
    fn only_readable_volumes_with_a_real_size_are_reported() {
        // Read-only APFS snapshots mirror the data volume, so counting them
        // would double the apparent capacity in every report.
        for volume in volumes() {
            assert!(
                volume.total > 0,
                "{} was reported with zero size",
                volume.mount_point
            );
            assert!(
                volume.available <= volume.total,
                "{} claims more free space than it has",
                volume.mount_point
            );
            assert!(
                volume.mount_point.starts_with('/'),
                "mount points are absolute: {}",
                volume.mount_point
            );
        }
    }
}
