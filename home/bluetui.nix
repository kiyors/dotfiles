{
  myLib,
  lib,
  config,
  pkgs,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "network.bluetui";
  description = "TUI for managing bluetooth";
  config = {
    home.packages = lib.optionals pkgs.stdenv.isLinux [
      pkgs.bluetui
    ];

    home.file.".config/bluetui/config.toml".source = lib.mkIf pkgs.stdenv.isLinux (
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/bluetui/config.toml"
    );
  };
}
