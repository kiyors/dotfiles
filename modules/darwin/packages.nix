{
  pkgs,
  config,
  lib,
  ...
}:
lib.mkModule {
  globalConfig = config;
  name = "darwin.packages";
  description = "macOS system packages configuration";
  config = {
    environment.systemPackages = with pkgs; [
      luarocks
      nixfmt
      harper
      sysClean
    ];
  };
}
