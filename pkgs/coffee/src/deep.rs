//! Optional, destructive deep cleanup: developer caches, Homebrew and macOS.
//!
//! None of this runs by default. Deleting warm caches trades disk space for CPU
//! and slower first launches, so every caller must pass `--deep` explicitly.

use crate::reap;
use crate::tool;
use crate::ui;
use std::path::PathBuf;

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
pub fn clean_dev_caches(dry_run: bool, quiet: bool) -> u64 {
    let mut freed = 0;

    for cache in dev_caches() {
        if !cache.path.exists() {
            continue;
        }

        if reap::any_process_matches(cache.busy_marker) {
            if !quiet {
                ui::warn(format!(
                    "Skipping {} — a `{}` process is running",
                    cache.path.display(),
                    cache.busy_marker
                ));
            }
            continue;
        }

        let bytes = crate::space::dir_size(&cache.path);

        if dry_run {
            if !quiet {
                ui::dry_run(format!(
                    "Would remove {} ({})",
                    cache.path.display(),
                    ui::human_bytes(bytes)
                ));
            }
            freed += bytes;
            continue;
        }

        match std::fs::remove_dir_all(&cache.path) {
            Err(error) => {
                if !quiet {
                    ui::failure(format!(
                        "Failed to remove {}: {error}",
                        cache.path.display()
                    ));
                }
            }
            Ok(()) => {
                freed += bytes;
                if !quiet {
                    ui::ok(format!(
                        "Removed {} ({})",
                        cache.path.display(),
                        ui::human_bytes(bytes)
                    ));
                }
            }
        }
    }

    freed
}

/// Drop stale downloads and prune all outdated formulae.
pub fn brew_cleanup(dry_run: bool, quiet: bool) {
    let mut args = vec!["cleanup".to_string(), "-s".to_string()];
    if dry_run {
        args.push("-n".to_string());
    } else {
        args.push("--prune=all".to_string());
    }

    if !quiet {
        ui::step("brew", &args);
    }

    // Homebrew is optional, so a missing binary is not an error.
    let Ok(status) = std::process::Command::new("brew").args(&args).status() else {
        return;
    };

    if quiet {
        return;
    }

    if status.success() {
        ui::ok("Successfully executed brew cleanup");
    } else {
        ui::failure(format!("brew cleanup exited with status: {status}"));
    }
}

/// Rebuild macOS DNS and QuickLook caches.
pub fn flush_macos_caches(dry_run: bool, quiet: bool) {
    if !tool::is_macos() {
        return;
    }

    if dry_run {
        if !quiet {
            ui::dry_run("Would flush macOS DNS & thumbnail caches");
        }
        return;
    }

    let _ = std::process::Command::new("dscacheutil")
        .arg("-flushcache")
        .status();
    let _ = std::process::Command::new("killall")
        .args(["-HUP", "mDNSResponder"])
        .status();
    let _ = std::process::Command::new("qlmanage")
        .args(["-r", "cache"])
        .status();

    if !quiet {
        ui::ok("Flushed macOS DNS & QuickLook caches");
    }
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
