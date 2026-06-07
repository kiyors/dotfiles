{
  myLib,
  lib,
  config,
  pkgs,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "network.impala";
  description = "TUI for managing wifi";
  config = {
    home.packages = lib.optionals pkgs.stdenv.isLinux [
      pkgs.impala
    ];

    home.file.".config/impala/config.toml".source = lib.mkIf pkgs.stdenv.isLinux (
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/impala/config.toml"
    );
  };
}
