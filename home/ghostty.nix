{
  myLib,
  lib,
  config,
  pkgs,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "terminal.ghostty";
  description = "ghostty terminal";
  config = {
    home.packages = lib.optionals pkgs.stdenv.hostPlatform.isLinux [
      pkgs.ghostty
    ];

    home.file = {
      ".config/ghostty/config".source =
        config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/ghostty/${
          if pkgs.stdenv.hostPlatform.isDarwin then "darwin-config" else "linux-config"
        }";
    };
  };
}
