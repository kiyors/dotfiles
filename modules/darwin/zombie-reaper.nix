{ lib, pkgs, ... }:

{
  launchd.user.agents = {
    
    # 1. Reap stale, CPU-heavy developer processes adopted by launchd.
    "zombie-reaper" = {
      # Generic runtime names (node, python, ruby, etc.) are deliberately omitted:
      # PPID 1 is not, by itself, proof that a process is unwanted.
      command = "${pkgs.sysClean}/bin/sys-clean reap-zombies --quiet --min-age 900 --min-cpu 25 --grace-period 5 -t jest-worker -t tsc -t esbuild -t rust-analyzer";
      
      serviceConfig = {
        StartInterval = 1800; # 30 minutes
        RunAtLoad = true;
        StandardErrorPath = "/tmp/zombie-reaper.err";
        StandardOutPath = "/tmp/zombie-reaper.out";
        ProcessType = "Background";
        LowPriorityIO = true;
        Nice = 10;
      };
    };

  };

  # System generations are root-owned, so this must not be a user agent.
  launchd.daemons."system-cleanup" = {
    command = pkgs.writeShellScript "system-cleanup" ''
      export PATH=${lib.makeBinPath [ pkgs.nh ]}:/usr/bin:/bin

      # Broad cache removal is intentionally not automated: warm caches improve
      # application performance and are recreated after deletion.
      ${pkgs.sysClean}/bin/sys-clean system-cleanup --quiet --keep-generations 5 --keep-since 14d
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
    };
  };
}
