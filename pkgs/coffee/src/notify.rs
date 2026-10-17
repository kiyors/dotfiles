//! Best-effort macOS user notifications.
//!
//! Notifications are a convenience, never a failure mode: if `osascript` is
//! missing or the call fails, the underlying command still reports its result.

/// Send a user notification on macOS. No-op on every other platform.
pub fn send(title: &str, message: &str) {
    if std::env::consts::OS != "macos" {
        return;
    }

    // Pass values as argv instead of interpolating them into AppleScript source.
    let script = r#"on run argv
display notification (item 2 of argv) with title (item 1 of argv)
end run"#;

    let _ = std::process::Command::new("osascript")
        .args(["-e", script, title, message])
        .status();
}
