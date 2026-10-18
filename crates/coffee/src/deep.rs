//! Optional, destructive deep cleanup: developer caches, Homebrew and macOS.
//!
//! None of this runs by default. Deleting warm caches trades disk space for CPU
//! and slower first launches, so every caller must pass `--deep` explicitly.

use crate::log::Mode;
use crate::reap;
use crate::space;
use crate::tool;
use crate::ui::{self, Printer};
use std::path::PathBuf;
use sysinfo::System;

/// A global build cache that is safe to remove only when nothing is using it.
pub struct DevCache {
    pub path: PathBuf,
    /// A process command-line fragment that means the cache is in use.
    pub busy_marker: &'static str,
}

/// Global build caches that are expensive to regenerate, paired with the
/// process that indicates each one is currently in use.
pub fn dev_caches() -> Vec<DevCache> {
    let Ok(home) = std::env::var("HOME") else {
        return Vec::new();
    };

    let entries: &[(&str, &str)] = &[
        (".gradle/caches", "gradle"),
        (".gradle/daemon", "gradle"),
        ("Library/Developer/Xcode/DerivedData", "xcodebuild"),
        ("Library/Developer/CoreSimulator/Caches", "CoreSimulator"),
    ];

    entries
        .iter()
        .map(|(relative, busy_marker)| DevCache {
            path: PathBuf::from(&home).join(relative),
            busy_marker,
        })
        .collect()
}

/// Remove global Gradle and Xcode build caches.
///
/// Caches belonging to a tool that is currently running are skipped: deleting
/// `~/.gradle/daemon` under a live Gradle daemon breaks in-flight builds.
/// Returns the total bytes that were (or in a dry run, would be) freed.
pub fn clean_dev_caches(dry_run: bool, quiet: bool, system: &System) -> u64 {
    let out = Printer::quiet(quiet);
    let mut freed = 0;

    for cache in dev_caches() {
        if !cache.path.exists() {
            continue;
        }

        if reap::any_process_matches(system, cache.busy_marker) {
            out.warn(format!(
                "Skipping {} — a `{}` process is running",
                cache.path.display(),
                cache.busy_marker
            ));
            continue;
        }

        let bytes = space::dir_size(&cache.path);

        if dry_run {
            out.dry_run(format!(
                "Would remove {} ({})",
                cache.path.display(),
                ui::human_bytes(bytes)
            ));
            freed += bytes;
            continue;
        }

        if let Err(error) = std::fs::remove_dir_all(&cache.path) {
            out.failure(format!(
                "Failed to remove {}: {error}",
                cache.path.display()
            ));
        } else {
            freed += bytes;
            out.ok(format!(
                "Removed {} ({})",
                cache.path.display(),
                ui::human_bytes(bytes)
            ));
        }
    }

    freed
}

/// Drop stale downloads and prune all outdated formulae.
pub fn brew_cleanup(dry_run: bool, quiet: bool) {
    let out = Printer::quiet(quiet);

    let mut args = vec!["cleanup".to_string(), "-s".to_string()];
    if dry_run {
        args.push("-n".to_string());
    } else {
        args.push("--prune=all".to_string());
    }

    out.step("brew", &args);

    // Homebrew is optional, so a missing binary is not an error.
    let launch = Mode::from_quiet(quiet).run("brew", &args);

    let Some(status) = launch.status() else {
        return;
    };

    if launch.is_success() {
        out.ok("Successfully executed brew cleanup");
    } else {
        out.failure(format!("brew cleanup exited with status: {status}"));
    }
}

/// Rebuild the macOS DNS caches (`dscacheutil`, `mDNSResponder`) and the
/// macOS thumbnail cache (`qlmanage`).
#[allow(clippy::needless_pass_by_value)] // `quiet` is forwarded to the printer
pub fn flush_macos_caches(dry_run: bool, quiet: bool) {
    let out = Printer::quiet(quiet);

    if !tool::is_macos() {
        return;
    }

    if dry_run {
        out.dry_run("Would flush macOS DNS & thumbnail caches");
        return;
    }

    // All three are best-effort: a machine missing one of them still benefits
    // from flushing the other two, so failures are not surfaced. Their output is
    // discarded either way — it is progress noise, not a result.
    for (program, args) in [
        ("dscacheutil", &["-flushcache"][..]),
        ("killall", &["-HUP", "mDNSResponder"][..]),
        ("qlmanage", &["-r", "cache"][..]),
    ] {
        Mode::Quiet.run_discarding(program, args);
    }

    out.ok("Flushed macOS DNS & QuickLook caches");
}

#[cfg(test)]
mod tests {
    use super::dev_caches;

    #[test]
    fn dev_caches_are_absolute_and_have_a_busy_marker() {
        let caches = dev_caches();
        // Without HOME the list is empty rather than relative to the CWD.
        if std::env::var("HOME").is_ok() {
            assert_eq!(caches.len(), 4);
            for cache in &caches {
                assert!(
                    cache.path.is_absolute(),
                    "{} should be absolute",
                    cache.path.display()
                );
                assert!(
                    !cache.busy_marker.is_empty(),
                    "{} needs a busy marker",
                    cache.path.display()
                );
            }
        }
    }

    #[test]
    fn every_dev_cache_points_inside_the_user_home_directory() {
        let Ok(home) = std::env::var("HOME") else {
            return;
        };

        for cache in dev_caches() {
            assert!(
                cache.path.starts_with(&home),
                "{} escaped the home directory",
                cache.path.display()
            );
        }
    }
}
