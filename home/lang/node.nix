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
        biome
      ];

      sessionVariables = {
        # Suppress experimental warnings (e.g., when using newer node features)
        NODE_OPTIONS = "--disable-warning=ExperimentalWarning";
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
    home.file.".config/pnpm".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/pnpm";
  };
}
