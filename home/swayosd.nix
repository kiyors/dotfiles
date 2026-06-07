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

    # Disable HM-managed configs to allow symlinking the whole swayosd directory
    xdg.configFile."swayosd/style.css".enable = lib.mkForce false;

    # Symlink the swayosd directory
    xdg.configFile."swayosd".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/swayosd";
  };
}
