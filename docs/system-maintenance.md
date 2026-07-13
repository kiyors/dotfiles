# System Maintenance & Background Agents

This document describes the macOS background tasks configured through `launchd` in
`modules/darwin/zombie-reaper.nix`.

## 1. Orphan Reaper (`zombie-reaper`)

The command retains the historical name `sys-clean reap-zombies`, but it does not
look for Unix zombie processes. A real zombie has already exited and consumes no
CPU. This tool looks for stale, CPU-heavy developer processes whose original parent
has exited and which have consequently been adopted by `launchd` (PID 1).

### Background policy

The launch agent runs every 30 minutes and once when it is loaded. A process is a
candidate only when all of these conditions hold:

- its parent PID is 1;
- its command line contains one of the configured developer-tool fragments;
- it has been running for at least 15 minutes; and
- it is using at least 25% CPU during the sample window.

The background target list intentionally uses developer-tool fragments:
`jest-worker`, `tsc`, `esbuild`, and `rust-analyzer`. Generic runtime names such as
`node`, `python`, `go`, and `ruby` are excluded because legitimate daemonized
programs can also have PID 1 as their parent.

When a candidate is found, the tool sends `SIGTERM`, waits five seconds, and sends
`SIGKILL` only if the process is still alive. Successful runs with no candidates are
quiet, so the launchd log does not grow every 30 minutes. Runs use background process
scheduling, low-priority I/O, and a positive nice value.

> PPID 1 is a useful signal, not a proof that a process is unwanted. The age, CPU,
> and command filters reduce false positives, but review target changes with
> `--dry-run` before deploying them.

### Running it manually

The interactive defaults are less strict than the background policy: minimum age
five minutes, minimum CPU 10%, target `jest-worker`, and a two-second graceful
shutdown window.

```sh
# Preview the default policy.
sys-clean reap-zombies --dry-run

# Preview the exact background policy.
sys-clean reap-zombies --dry-run \
  --min-age 900 --min-cpu 25 --grace-period 5 \
  -t jest-worker -t tsc -t esbuild -t rust-analyzer

# Include an additional project-specific command fragment.
sys-clean reap-zombies --dry-run -t jest-worker -t my-project/dev-worker
```

Relevant options:

- `-t, --target <TEXT>` adds command-line fragments to match (repeatable; `--targets` is also accepted);
- `--min-age <SECONDS>` ignores newly started processes;
- `--min-cpu <PERCENT>` ignores idle processes (use `0` intentionally to include them);
- `--grace-period <SECONDS>` controls the delay between `SIGTERM` and `SIGKILL`;
- `--dry-run` lists candidates without signaling them;
- `--quiet` suppresses routine start and summary output; and
- `--no-notify` disables the macOS notification after a successful reap.

The command exits non-zero if validation fails or a signal cannot be delivered.

## 2. System Garbage Collector (`system-cleanup`)

This launch agent runs weekly, on Sunday at 04:00 (or after wake if the Mac was
asleep). It no longer runs at load, avoiding cleanup work around login and system
activation. The job runs this conservative policy:

```sh
sys-clean system-cleanup --quiet --keep-generations 5 --keep-since 14d
```

That cleans unreferenced Nix store paths while retaining at least five generations
and everything from the last 14 days for rollback. It preserves project and direnv
GC roots by default. The job is a system daemon because Nix system generations are
root-owned, and it uses an explicit Nix-store path for `nh` rather than depending on
an interactive shell's `PATH`. Background scheduling, low-priority I/O, and a
positive nice value reduce interference with interactive work.

Broad cache cleanup is deliberately not automated. Warm caches generally improve
application startup and rebuild speed; deleting them repeatedly trades disk space
for more CPU, network traffic, and slower first launches. Use deep cleanup manually
when disk pressure justifies it:

```sh
# Always preview first.
sys-clean system-cleanup --deep --dry-run

# Remove old Nix paths plus caches, logs, temporary files, and app leftovers.
sys-clean system-cleanup --deep

# Optionally deduplicate the Nix store; useful for space, but potentially slow.
sys-clean system-cleanup --optimise

# Explicitly include stale project and direnv GC roots in cleanup.
sys-clean system-cleanup --clean-gcroots --dry-run
```

The command exits non-zero if validation or any child command fails, allowing
launchd logs or future monitoring to expose partial failures.

## Expected impact

- The orphan reaper can improve responsiveness, thermals, and battery life when a
  matched abandoned process is consuming CPU.
- Conservative Nix cleanup primarily preserves disk space and rollback safety; it
  does not make a healthy machine inherently faster.
- Deep cleanup is a disk-recovery tool, not routine performance maintenance.
- Neither task replaces macOS updates, adequate free disk space, or investigating
  consistently high memory pressure and CPU usage in Activity Monitor.

## Logs

The agents write to:

- `/tmp/zombie-reaper.out` and `/tmp/zombie-reaper.err`
- `/tmp/system-cleanup.out` and `/tmp/system-cleanup.err`

Inspect the current launchd job with:

```sh
launchctl print "gui/$(id -u)/zombie-reaper"
```
