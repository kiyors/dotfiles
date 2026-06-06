{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "desktop.swaync";
  description = "SwayNC notification center with custom config symlink";
  config = {
    services.swaync = {
      enable = true;
    };

    # Symlink the swaync directory
    xdg.configFile."swaync".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/swaync";
  };
}
