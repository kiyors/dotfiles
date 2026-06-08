{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
let
  goPath = "${config.xdg.configHome}/go";
in
myLib.mkHomeModule {
  globalConfig = config;
  name = "lang.go";
  description = "Go development environment";
  config = {
    home = {
      packages = with pkgs; [
        go
        gopls
        gotools
        golangci-lint
        delve
      ];

      sessionVariables = {
        GOPATH = goPath;
      };

      sessionPath = [ "${goPath}/bin" ];

      activation.initGo = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        $DRY_RUN_CMD mkdir -p ${goPath}/bin
      '';
    };
  };
}
