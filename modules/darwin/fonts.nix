{
  pkgs,
  config,
  lib,
  ...
}:
lib.mkModule {
  globalConfig = config;
  name = "darwin.fonts";
  description = "macOS system fonts configuration";
  config = {
    fonts.packages = lib.commonFontPkgs pkgs;
  };
}
