{ lib, pkgs, ... }:

{
  launchd.user.agents = {

    # 1. Reap stale, CPU-heavy developer processes adopted by launchd.
    "zombie-reaper" = {
      # Generic runtime names (node, python, ruby, etc.) are deliberately omitted:
      # PPID 1 is not, by itself, proof that a process is unwanted.
      #
      # `tsc` is spelled as a path fragment rather than the bare word, because
      # matching is a substring test: a bare "tsc" would also match any command
      # line that merely contains those three letters. `coffee reap-zombies`
      # additionally refuses to signal anything running out of /System, /usr or
      # /sbin, and refuses well-known daemon names, so a mistyped target here
      # degrades to "match nothing" rather than "kill something important".
      command = ''
        ${pkgs.coffee}/bin/coffee reap-zombies
        --quiet
        --min-age 900
        --min-cpu 25
        --min-memory 1G
        --grace-period 5
        -t jest-worker
        -t typescript/lib/tsc.js
        -t node_modules/.bin/tsc
        -t esbuild
        -t rust-analyzer
      '';

      serviceConfig = {
        StartInterval = 1800; # 30 minutes
        RunAtLoad = true;
        ThrottleInterval = 600;
        StandardErrorPath = "/tmp/zombie-reaper.err";
        StandardOutPath = "/tmp/zombie-reaper.out";
        ProcessType = "Background";
        LowPriorityIO = true;
        Nice = 10;
        # Let launchd clean up the whole process group if a run overruns, so a
        # stuck run cannot outlive its 30-minute slot.
        AbandonProcessGroup = true;
      };
    };

  };

  # System generations are root-owned, so this must not be a user agent.
  launchd.daemons."system-cleanup" = {
    command = pkgs.writeShellScript "system-cleanup" ''
      export PATH=${lib.makeBinPath [ pkgs.nh ]}:/usr/bin:/bin

      # Broad cache removal is intentionally not automated: warm caches improve
      # application performance and are recreated after deletion.
      #
      # --no-notify: this runs as root, where osascript cannot post a user
      # notification, so the attempt would only spawn a process for nothing.
      ${pkgs.coffee}/bin/coffee system-cleanup \
        --quiet \
        --keep-generations 5 \
        --keep-since 14d \
        --no-notify
    '';

    serviceConfig = {
      # Sunday at 04:00. StartCalendarInterval jobs delayed by sleep run on wake.
      StartCalendarInterval = {
        Weekday = 0;
        Hour = 4;
        Minute = 0;
      };

      StandardErrorPath = "/tmp/system-cleanup.err";
      StandardOutPath = "/tmp/system-cleanup.out";
      ProcessType = "Background";
      LowPriorityIO = true;
      Nice = 15;
      AbandonProcessGroup = true;
    };
  };
}
