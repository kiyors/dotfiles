# System Maintenance & Background Agents

This document outlines the automated background tasks configured in macOS (`launchd`) via our `nix-darwin` setup.

These services are located in `modules/darwin/zombie-reaper.nix`.

## 1. Zombie Reaper (`zombie-reaper`)

### The Problem
Sometimes, especially during web or backend development, closing a terminal or aborting a test doesn't properly kill the child processes. On macOS, these "orphaned" developer processes (like Node, Python, Rust, etc.) get adopted by the system's root process (`launchd`, PID 1). Because they are stuck in a loop, they max out the CPU, generating heat and drastically draining the battery.

### What the agent does
The `zombie-reaper` is a launchd agent that runs automatically every **30 minutes** in the background. It uses a custom Rust tool (`sys-maintainer`) that scans system processes, looks for orphaned developer nodes (e.g., `jest-worker`, `node`, `tsc`, `python`, `rust-analyzer`), and terminates them.

### Key Features
- **Native macOS Notifications**: When the tool finds and reaps a zombie, it triggers a native macOS desktop notification (e.g. "Reaped 3 orphaned background process(es) 🧟") so you know it's working. If no zombies are found, it stays completely silent.
- **Multiple Targets**: The `--target` (or `-t`) flag can be passed multiple times. The Nix configuration easily tracks multiple developer tools at once (like `node`, `python`, `cargo`, `go`, etc.).
- **Dry Run Mode**: You can test the tool manually without killing anything by running `sys-maintainer reap-zombies --dry-run`. It will scan the system and print exactly what it *would* have killed, including the PID and CPU usage.

### Is it safe? Will it kill my active `pnpm dev`?
**Yes, it is 100% safe. It will NEVER kill your active dev environments.** 
The Rust program uses a very specific safeguard:
It checks the Parent Process ID (PPID) of the Node process using the cross-platform `sysinfo` crate.
- When you run `pnpm dev` or `npm test` normally, its parent is your terminal shell (e.g., `zsh`, `tmux`), so the PPID might be `40952`. The program **ignores** these.
- It only targets processes where the PPID is exactly `1`. A process only gets a PPID of `1` when its original parent window is destroyed but the process refused to close (an orphan/zombie). 

## 2. System Garbage Collector (`system-cleanup`)

### What the agent does
This agent runs automatically every **3 days** (259200 seconds) in the background to keep the Nix store and macOS system clean. 

It executes:
- `nh clean darwin`
- `nh clean all`
- `mo clean`

This ensures that old, unused Nix packages and system generations don't slowly fill up your hard drive over time. It runs silently as a `ProcessType = "Background"` so macOS automatically throttles it and ensures it doesn't interrupt your active work.

### Logs
If you ever need to check if these are working, you can view their output logs at:
- `/tmp/zombie-reaper.out` & `/tmp/zombie-reaper.err`
- `/tmp/system-cleanup.out` & `/tmp/system-cleanup.err`
