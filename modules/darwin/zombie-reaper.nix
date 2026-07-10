{ pkgs, ... }:

{
  launchd.user.agents = {
    
    # 1. The Zombie Reaper (Runs every 30 minutes)
    "zombie-jest-reaper" = {
      command = pkgs.writeShellScript "reap-zombies" ''
        ${pkgs.procps}/bin/ps -eo pid,ppid,command | \
          ${pkgs.gnugrep}/bin/grep '[j]est-worker/processChild.js' | \
          ${pkgs.gawk}/bin/awk '$2 == 1 {print $1}' | \
          ${pkgs.findutils}/bin/xargs -I {} kill -9 {} 2>/dev/null || true
      '';
      
      serviceConfig = {
        StartInterval = 1800; # 30 minutes
        RunAtLoad = true;
        StandardErrorPath = "/tmp/zombie-reaper.err";
        StandardOutPath = "/tmp/zombie-reaper.out";
        ProcessType = "Background";
      };
    };

    # 2. System Garbage Collector (Runs every 3 days)
    "system-cleanup" = {
      command = pkgs.writeShellScript "system-cleanup" ''
        # Load the nix environment variables just in case launchd doesn't have them
        if [ -e '/nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh' ]; then
          source '/nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh'
        fi
        
        # Run your cleanup commands
        nh clean darwin
        nh clean all
        mo clean
      '';
      
      serviceConfig = {
        # 3 days in seconds (3 * 24 * 60 * 60 = 259200)
        StartInterval = 259200;
        
        # Run it once at startup as well, just to ensure it fires if you reboot often
        RunAtLoad = true; 
        
        # Logs are helpful to verify it's working
        StandardErrorPath = "/tmp/system-cleanup.err";
        StandardOutPath = "/tmp/system-cleanup.out";
        
        # Let macOS throttle this during heavy usage so it doesn't slow you down
        ProcessType = "Background"; 
      };
    };
  };
}
