#!/usr/bin/env bash
# Verify the background zombie-reaper agent end to end.
#
# Checks, in order:
#   1. the agent is registered with launchd
#   2. it has actually run, and when
#   3. the flags the agent passes are the intended ones (not clap defaults)
#   4. it honours --quiet
#   5. the run count is keeping up with the interval
#
# Usage: scripts/verify-reaper.sh

set -uo pipefail

LABEL="org.nixos.zombie-reaper"
DOMAIN="gui/$(id -u)"
fail=0

say() { printf '\n\033[1m%s\033[0m\n' "$*"; }
ok() { printf '  \033[32mok\033[0m    %s\n' "$*"; }
bad() {
  printf '  \033[31mFAIL\033[0m  %s\n' "$*"
  fail=1
}

agent_state() { launchctl print "$DOMAIN/$LABEL" 2>/dev/null; }

say "1. registered with launchd"
if agent_state >/dev/null; then
  ok "loaded as $DOMAIN/$LABEL"
  agent_state | grep -E '^\tpath = ' | sed 's/^/        /'
else
  bad "not loaded — run: launchctl bootstrap $DOMAIN ~/Library/LaunchAgents/$LABEL.plist"
  exit 1
fi

say "2. run history"
runs=$(agent_state     | awk -F'= ' '/^\truns =/{print $2; exit}')
interval=$(agent_state | awk -F'= ' '/run interval =/{print $2+0; exit}')
last=$(agent_state     | awk -F'= ' '/last exit code =/{print $2; exit}')
signal=$(agent_state   | awk -F'= ' '/last terminating signal/{print $2; exit}')
printf '  runs=%s  interval=%ss  last_exit=%s\n' "$runs" "$interval" "${last:-n/a}"
case "${last:-0}" in
  0) ok "last run exited cleanly" ;;
  n/a|'') if [ -n "$signal" ]; then
         bad "no exit code; last terminating signal: $signal"
       else
         ok "no run recorded yet this session"
       fi ;;
  *) bad "last run exited with $last — see /tmp/zombie-reaper.err" ;;
esac

say "3. flags the agent actually passes"
cmd=$(agent_state | sed -n '/arguments = {/,/^\t}/p')
for expected in "--min-age 900" "--min-cpu 25" "--min-memory 1G" "--grace-period 5" "--quiet"; do
  if printf '%s' "$cmd" | grep -qF -- "$expected"; then
    ok "passes $expected"
  else
    bad "missing $expected — the agent is running with clap DEFAULTS"
  fi
done
targets=$(printf '%s' "$cmd" | grep -c '^\s*-t ' || true)
printf '        %s targets configured\n' "$targets"
[ "$targets" -eq 5 ] && ok "all 5 targets present" || bad "expected 5 targets, found $targets"

say "4. binary honours --quiet"
# The plist wraps the real command as `/bin/wait4path /nix/store && exec
# <store-path>/bin/coffee …`, so match the coffee binary rather than the first
# word of the line.
bin=$(printf '%s' "$cmd" | grep -oE '/nix/store/[^ ]*coffee[^ ]*/bin/coffee' | head -1)
if [ -x "$bin" ]; then
  ok "binary: $bin"
  loud=$("$bin" reap-zombies --dry-run 2>&1 | wc -l | tr -d ' ')
  quiet=$("$bin" reap-zombies --quiet --dry-run 2>&1 | wc -l | tr -d ' ')
  printf '        verbose=%s lines, quiet=%s lines\n' "$loud" "$quiet"
  [ "$quiet" -eq 0 ] && ok "--quiet suppresses output" || bad "--quiet produced $quiet lines"
  [ "$loud" -gt 0 ] && ok "verbose mode reports its policy" || bad "verbose mode is silent too"
else
  bad "binary $bin is not executable"
fi

say "5. schedule is keeping up"
# `launchctl kickstart` is deliberately NOT used here. The job sets
# `minimum runtime = 600`, so launchd holds the slot for 10 minutes and
# `kickstart` blocks until it can respawn — it looks like a hang and makes this
# check unreliable. The counter is inspected instead, which is deterministic.
#
# The window is measured from the plist's own mtime, NOT from boot: a `darwin-
# rebuild` re-bootstraps the agent and resets `runs`, while uptime keeps counting
# from long before, which would make every rebuilt machine look broken.
runs=$(agent_state     | awk -F'= ' '/^\truns =/{print $2; exit}')
interval=$(agent_state | awk -F'= ' '/run interval =/{print $2+0; exit}')
plist="$HOME/Library/LaunchAgents/$LABEL.plist"
since=$(stat -f%m "$plist" 2>/dev/null || echo 0)
elapsed=$(( $(date +%s) - since ))
expected=$(( elapsed / interval ))

printf '        agent installed %ss ago  interval=%ss  runs=%s  expected≥%s\n' \
  "$elapsed" "$interval" "$runs" "$expected"

# Allow a grace window: a fresh bootstrap fires RunAtLoad immediately, then the
# first interval has not elapsed yet.
if [ "$runs" -gt "$expected" ] || [ "$elapsed" -lt $(( interval * 2 )) ]; then
  ok "run count is consistent with the interval"
else
  bad "$runs run(s) in ${elapsed}s at a ${interval}s interval — expected ≥$expected"
fi

cat <<EOF

  To force a run by hand (this blocks until launchd respawns the job, which
  takes up to 'minimum runtime' seconds):

      launchctl kickstart gui/\$(id -u)/$LABEL
EOF

say "logs"
printf '        /tmp/zombie-reaper.out  %s bytes\n' "$(stat -f%z /tmp/zombie-reaper.out 2>/dev/null || echo '?')"
printf '        /tmp/zombie-reaper.err  %s bytes\n' "$(stat -f%z /tmp/zombie-reaper.err 2>/dev/null || echo '?')"
if [ -s /tmp/zombie-reaper.err ]; then
  bad "stderr is not empty:"
  tail -20 /tmp/zombie-reaper.err | sed 's/^/        /'
else
  ok "stderr empty"
fi

printf '\n'
[ "$fail" -eq 0 ] && printf '\033[32mall checks passed\033[0m\n' || printf '\033[31mone or more checks failed\033[0m\n'
exit "$fail"
