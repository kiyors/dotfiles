# System Maintenance & Background Agents

This document describes the macOS background tasks configured through `launchd` in
`modules/darwin/zombie-reaper.nix`.

## 1. Orphan Reaper (`zombie-reaper`)

The command retains the historical name `coffee reap-zombies` (or `co reap-zombies`), but it does not
look for Unix zombie processes. A real zombie has already exited and consumes no
CPU. This tool looks for stale, CPU-heavy developer processes whose original parent
has exited and which have consequently been adopted by `launchd` (PID 1).

A candidate is only signalled when every one of these holds:

- it runs the same user as `coffee` (nothing else is ever touched, which is what
  keeps the tool harmless even if it is ever run as root);
- its executable is **not** under `/System`, `/usr`, `/sbin` or `/Library/Apple`;
- its name is not a known-critical daemon (`launchd`, `WindowServer`, `Dock`,
  `loginwindow`, `sshd`, `kernel_task`, …);
- it is neither `coffee` itself nor one of its ancestors;
- its parent PID is 1;
- its command line contains one of the configured developer-tool fragments; and
- it is older than `--min-age` **and** either using more than `--min-cpu` CPU or
  holding more than `--min-memory`.

A threshold of `0` means *that check is switched off*, not "everything passes
it". This distinction matters: the CPU and memory checks are alternatives
combined with OR, so treating an unset threshold as `>= 0` — which is always
true — would silently make the other threshold dead. With `--min-memory 0`, the
default, `--min-cpu` would then do nothing at all and the policy would collapse
to "orphaned and target-matched", taking idle long-lived dev servers with it.

The provenance and name checks mean a mistyped target degrades to "match
nothing" rather than "signal something important". For example
`coffee reap-zombies -t launchd` matches nothing at all.

Matching is a case-insensitive substring test, so bare short words are avoided:
`tsc` is configured as `typescript/lib/tsc.js` and `node_modules/.bin/tsc`
because a bare `tsc` would match any command line containing those letters.

### Background policy

The launch agent runs every 30 minutes and once when it is loaded. A process is a
candidate only when all of these conditions hold:

- its parent PID is 1;
- its command line contains one of the configured developer-tool fragments;
- it has been running for at least 15 minutes; and
- it is using at least 25% CPU, or holding at least 1 GiB of memory.

The background target list intentionally uses developer-tool fragments:
`jest-worker`, `typescript/lib/tsc.js`, `node_modules/.bin/tsc`, `esbuild`, and
`rust-analyzer`. Generic runtime names such as `node`, `python`, `go`, and
`ruby` are excluded because legitimate daemonized programs can also have PID 1 as
their parent.

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
# Preview the default policy (callable as `coffee` or `co`).
coffee reap-zombies --dry-run

# Preview the exact background policy.
co reap-zombies --dry-run \
  --min-age 900 --min-cpu 25 --grace-period 5 \
  -t jest-worker -t tsc -t esbuild -t rust-analyzer

# Include an additional project-specific command fragment.
co reap-zombies --dry-run -t jest-worker -t my-project/dev-worker
```

Relevant options:

- `-t, --target <TEXT>` adds command-line fragments to match (repeatable; `--targets` is also accepted);
- `--min-age <SECONDS>` ignores newly started processes;
- `--min-cpu <PERCENT>` ignores idle processes (use `0` intentionally to include them);
- `--min-memory <SIZE>` also reaps candidates holding at least this much memory, accepting suffixes such as `512M` and `4G`. This catches idle-but-leaking processes; `0` disables it;
- `--grace-period <SECONDS>` controls the delay between `SIGTERM` and `SIGKILL`;
- `--dry-run` lists candidates without signaling them;
- `--quiet` suppresses routine start and summary output; and
- `--no-notify` disables the macOS notification after a successful reap.

`--min-cpu` and `--min-memory` cannot both be `0`, because that combination
would match every orphaned process on the machine.

Matches are reported as a table sorted by CPU, then memory:

```
🧟 Scanning for stale orphaned processes...
⚠️  Found orphaned processes:
┌─────────┬───────┬─────┬──────┬─────────┬──────────────────────┐
│ Process │   PID │ Age │  CPU │  Memory │ Executable          │
├─────────┼───────┼─────┼──────┼─────────┼──────────────────────┤
│ node    │ 41207 │ 62s │ 99.1% │ 412 MiB │ /opt/homebrew/bin/node│
└─────────┴───────┴─────┴──────┴─────────┴──────────────────────┘
   node /Users/g/p/node_modules/.bin/jest-worker/index.js
  ✨ Reaped 1 orphaned process(es) (0 required SIGKILL).
```

The command exits non-zero if validation fails or a signal cannot be delivered.

## 2. System Garbage Collector (`system-cleanup`)

This launch agent runs weekly, on Sunday at 04:00 (or after wake if the Mac was
asleep). It no longer runs at load, avoiding cleanup work around login and system
activation. The job runs this conservative policy:

```sh
coffee system-cleanup --quiet --keep-generations 5 --keep-since 14d
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
# `clean` is a direct alias for `system-cleanup` (use `coffee` or shortcut `co`)

# Reclaimable caches AND project build artifacts:
# - nh clean all
# - mo clean (app, browser & system caches)
# - mo purge (node_modules, target/, build/, dist/)
# - Gradle/Xcode cache removal (skipped while a build tool is running)
# - brew cleanup --prune=all
# - macOS DNS & QuickLook cache flush
# - /tmp clutter cleanup (only items older than 24h)
co clean --deep --dry-run
co clean --deep

# Project build artifacts only:
co clean --purge-projects --dry-run
co clean --purge-projects

# Delete every old Nix generation system-wide (discards rollback history).
# This is NOT part of --deep and needs its own flag:
co clean -d --dry-run
co clean -d

# Deduplicate the Nix store only:
co clean --optimise

# Everything at once: only the current generation of each profile survives,
# plus project artifacts, global garbage collection, store deduplication and
# /tmp. Destructive and slow:
co clean --all --dry-run
co clean --all

# Clean stale /tmp clutter (nix-shell, nix-develop, scratch files) only:
co clean-tmp --dry-run
co clean-tmp
co clean-tmp --min-age 0   # ignore the age guard
```

### What is destructive

`--deep` runs `mo purge --yes`, which is unattended and therefore skips the
interactive review Mole uses to protect artifacts touched in the last 7 days.
`coffee` prints a warning before it runs, and `--dry-run` previews it.

Discarding Nix generations stays behind `-d` because it is the only step that
removes rollback history system-wide; rolling back to a previous system or
profile is not possible afterwards.

### Other safety guards

- `/tmp` entries are only removed when they are older than `--min-age` (default
  24h), so scratch directories belonging to a running `nix develop` shell are
  left alone. Sockets, `tmux-*`, and this tool's own launchd logs are never
  removed.
- Gradle and Xcode caches are skipped when a matching build process is running.
- `mo clean` empties the Trash unless it is protected. `coffee` warns before
  running it; add Trash to `mo clean --whitelist` to keep it.
- `coffee doctor` reports whether the Mole whitelist protects anything and which
  directories `mo purge` would scan.

The command exits non-zero if validation or any child command fails, allowing
launchd logs or future monitoring to expose partial failures.

## 3. Reporting (`co clean` summary and `coffee space`)

`coffee clean` prints what it did and how much space it reclaimed:

```
📊 What was freed
┌─────────────────────────────────────┬──────────┐
│ Category                            │     Size │
├─────────────────────────────────────┼──────────┤
│ /tmp clutter (3 item(s))            │  10.0 MiB│
│ Disk free space freed in total      │  10.4 MiB│
└─────────────────────────────────────┴──────────┘
  ✨ Reclaimed 10.4 MiB of disk space
```

The per-category rows count only what `coffee` deleted itself, measured before
each removal. The total row is a real free-space reading taken before and after
the whole run, so it also covers `nh`, `mo` and `brew`. In `--dry-run` the total
row is omitted, since no space was actually freed. Use `--no-stats` to suppress
the table, or `--quiet`, which already implies it.

`--quiet` suppresses *routine* output only. Failures and fatals still go to
stderr, because a swallowed error in a launchd log is worse than a noisy one.

`coffee space` answers "what is still using space" by listing the largest
directories:

```
$ coffee space --top 8
📊 Disk usage
  ✅ /System/Volumes/Data  91.7 GiB free of 228.3 GiB (40%)
┌─────────────────────────────────────────────────┬──────────┐
│ Path                                            │     Size │
├─────────────────────────────────────────────────┼──────────┤
│ /Users/gaurav/personal                          │  30.8 GiB│
│ /Users/gaurav/Library/Caches/com.spotify.client │   2.5 GiB│
└─────────────────────────────────────────────────┴──────────┘
```

Pass paths to inspect something specific, e.g. `coffee space ~/workspace`.

`--top` is a limit on the whole report, not per scanned directory, so the
"across N entries shown" line is always the exact sum of the rows in the table
above it.

## 4. Diagnostics (`coffee doctor`)
`coffee doctor` is read-only and changes nothing. It reports:

- free space on every writable volume, warning below 10% free or 20 GiB free;
- whether `nix`, `nh`, `mo`, `brew` and `launchctl` are on `PATH`;
- the Mole whitelist contents and the `mo purge` scan roots;
- stale `/tmp` items, and how many are old enough for `--min-age` to remove; and
- whether both launchd jobs are installed *and* registered with launchd.

Use `--reap-candidates` to also list the processes `reap-zombies` would target
right now, and `--strict` to exit non-zero when anything warned or failed.

`--reap-candidates` reports against the *same* policy the 30-minute agent runs,
not a looser approximation: `ReapArgs::background_agent_policy` in
`crates/coffee/src/cli.rs` holds the thresholds, and a test cross-checks them
against `modules/darwin/zombie-reaper.nix` so the two cannot silently drift
apart. If you change one, change the other — `cargo test` will say so.

An installed agent that launchd knows about but is not currently running is
reported as informational, not a warning: a `StartInterval` job spends almost all
its life waiting between runs. Only an agent launchd has never heard of is a
warning, because that means it was never bootstrapped.

## 5. Verifying the background agents

Run the checker after any change to a launchd job, or when you want to confirm
the agents are still doing their job:

```sh
./scripts/verify-reaper.sh
```

It checks that the reaper is registered, that it has run and exited cleanly,
that **the flags the agent actually passes are the intended ones**, that
`--quiet` is honoured by the binary it points at, that the run count is keeping
up with the interval, and that stderr is empty. It exits non-zero on failure, so
it is usable from CI or a pre-commit hook.

The flag check is the important one. launchd runs the agent's command block as a
single `sh -c` string, and a newline inside that string ends the command — so a
block written with bare newlines instead of `\` continuations passes only the
subcommand, drops every flag on the floor, and the agent runs with clap's
defaults without saying so. That is not hypothetical: the reaper was configured
that way, so `--min-cpu 25` and `--min-memory 1G` never took effect and it
reaped any orphaned `jest-worker` every 30 minutes. Two guards now prevent a
regression:

- `cli::tests::every_agent_flag_survives_the_shell_invocation` replays the block
  through `/bin/sh` and fails if any flag fails to arrive;
- `background_policy_matches_the_launchd_agent` compares the thresholds in
  `ReapArgs::background_agent_policy` against the values in the Nix file.

To force a run by hand:

```sh
launchctl kickstart gui/$(id -u)/org.nixos.zombie-reaper
```

This blocks for up to `minimum runtime` (600s), because launchd holds the slot
open after a run — that is normal, not a hang. To watch what it would do without
signalling anything:

```sh
coffee reap-zombies --dry-run          # same flags the agent uses
coffee doctor --reap-candidates         # reports against the agent's policy
```

`/tmp/zombie-reaper.err` should stay empty; a non-empty stderr means a tool is
missing or failing. `/tmp/zombie-reaper.out` only grows when the agent finds
candidates to reap, because a normal run is `--quiet`.

## 6. Shell completion

`coffee` is installed as `coffee` and `co`, and the package generates zsh, bash
and fish completions at build time. After a `darwin-rebuild`, both names complete
on <kbd>Tab</kbd>:

```sh
coffee <TAB>      # clean-tmp  doctor  reap-zombies  system-cleanup  generate-completion
co clean <TAB>    # --deep  --purge-projects  --optimise  -d  --dry-run  --min-age ...
```

The completions live in each profile's `share/zsh/site-functions`, so
`config/zsh/.zshrc` mirrors those directories onto `fpath` before `compinit`.
Because zsh's own RCS completion also declares `#compdef co`, the `.zshrc`
re-binds `co` to `_coffee` explicitly — otherwise `co <TAB>` completes RCS
checkout options instead.

Aliases for the subcommands are in `config/zsh/aliases.zsh`:

| Alias   | Expands to                 |
| ------- | -------------------------- |
| `co`    | `coffee`                   |
| `cod`   | `coffee doctor`            |
| `cor`   | `coffee reap-zombies`      |
| `cot`   | `coffee clean-tmp`         |
| `cop`   | `coffee clean --dry-run`   |
| `codp`  | `coffee clean --deep --dry-run` |

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

