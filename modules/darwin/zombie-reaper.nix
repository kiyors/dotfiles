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
      # Every flag MUST be joined with a trailing backslash.
      #
      # launchd runs this whole block as `sh -c "<one string>"`, and a newline
      # inside that string ENDS the command. Written with bare newlines, only
      # `reap-zombies` reached the binary: every flag below it became its own
      # shell command and died with "command not found" (buried nowhere, since
      # stderr only captured those), so the agent silently ran with clap's
      # DEFAULTS — reaping any orphaned `jest-worker` every 30 minutes regardless
      # of CPU or age, and ignoring `--quiet`, which is why the log filled up.
      #
      # `cli::tests::every_agent_flag_survives_the_shell_invocation` re-runs this
      # block through /bin/sh and fails if a flag ever stops arriving, so a
      # reformat cannot silently reintroduce it. `scripts/verify-reaper.sh`
      # checks the same thing on the live agent.
      #
      # The `system-cleanup` daemon below is written the same way, for the same
      # reason.
      command = ''
        ${pkgs.coffee}/bin/coffee reap-zombies \
          --quiet \
          --min-age 900 \
          --min-cpu 25 \
          --min-memory 1G \
          --grace-period 5 \
          -t jest-worker \
          -t typescript/lib/tsc.js \
          -t node_modules/.bin/tsc \
          -t esbuild \
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
