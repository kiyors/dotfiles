//! Best-effort macOS user notifications.
//!
//! Notifications are a convenience, never a failure mode: if `osascript` is
//! missing or the call fails, the underlying command still reports its result.

use crate::log::Mode;
use crate::tool;

/// Send a user notification on macOS. No-op on every other platform.
pub fn send(title: &str, message: &str) {
    if !tool::is_macos() {
        return;
    }

    // Pass values as argv instead of interpolating them into AppleScript source.
    let script = r"on run argv
display notification (item 2 of argv) with title (item 1 of argv)
end run";

    // `osascript` prints nothing useful on success and the notification is a
    // convenience, so neither its output nor its exit status is worth acting on.
    Mode::Quiet.run_discarding("osascript", &["-e", script, title, message]);
}
