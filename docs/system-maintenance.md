# System Maintenance & Background Agents

This document outlines the automated background tasks configured in macOS (`launchd`) via our `nix-darwin` setup.

These services are located in `modules/darwin/zombie-reaper.nix`.

## 1. Zombie Reaper (`zombie-jest-reaper`)

### The Problem
Sometimes, especially during web development with frameworks like Next.js or testing suites like Jest, closing a terminal or aborting a test doesn't properly kill the child processes. On macOS, these "orphaned" `jest-worker` Node processes get adopted by the system's root process (`launchd`, PID 1). Because they are stuck in a loop, they max out the CPU, generating heat and drastically draining the battery.

### What the agent does
The `zombie-jest-reaper` is a launchd agent that runs automatically every **30 minutes** in the background. It looks for `jest-worker` processes that have been orphaned and terminates them.

### Is it safe? Will it kill my active `pnpm dev`?
**Yes, it is 100% safe. It will NEVER kill your active dev environments.** 
The script uses a very specific safeguard:
```bash
awk '$2 == 1 {print $1}'
```
This command checks the Parent Process ID (PPID) of the Node process. 
- When you run `pnpm dev` or `npm test` normally, its parent is your terminal shell (e.g., `zsh`, `tmux`), so the PPID might be `40952`. The script **ignores** these.
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
