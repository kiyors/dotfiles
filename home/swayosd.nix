{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "desktop.swayosd";
  description = "SwayOSD on-screen display with custom config symlink";
  config = {
    services.swayosd = {
      enable = true;
    };

    # Symlink the swayosd directory
    xdg.configFile."swayosd".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/swayosd";
  };
}
