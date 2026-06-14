{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
let
  homeDir = config.home.homeDirectory;
  npmGlobalDir = "${homeDir}/.npm";
  pnpmHome = "${homeDir}/.local/share/pnpm";
in
myLib.mkHomeModule {
  globalConfig = config;
  name = "lang.node";
  description = "Node.js, Bun, and PNPM development environment";
  config = {
    home = {
      packages = with pkgs; [
        nodejs_24
        pnpm
        # bun
        npm-check-updates
        npkill
        husky
        deno
        biome
      ];

      sessionVariables = {
        # Suppress experimental and diagnostic warnings (common in Node 24+ with pnpm)
        NODE_OPTIONS = "--disable-warning=ExperimentalWarning --no-warnings";
        PNPM_HOME = pnpmHome;
      };

      sessionPath = [
        "${npmGlobalDir}/bin"
        pnpmHome
      ];

      # Ensure the global npm/pnpm directories exist for manual global installs
      activation.initNode = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        $DRY_RUN_CMD mkdir -p ${npmGlobalDir}/bin ${npmGlobalDir}/lib
        $DRY_RUN_CMD mkdir -p ${pnpmHome}
      '';

      file = {
        ".npmrc".text = ''
          prefix=${npmGlobalDir}
        '';

        ".config/pnpm/config.yaml".text = ''
          prefix: ${pnpmHome}
          storeDir: ${pnpmHome}/store
        '';

        # ".bunfig.toml".text = ''
        #   [runtime]
        #   logLevel = "debug"
        #   telemetry = false
        #
        #   [install]
        #   optional = true
        #   dev = true
        #   peer = true
        #   production = false
        #   exact = true
        #   auto = "fallback"
        # '';
      };
    };
  };
}
