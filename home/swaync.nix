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

    # Disable HM-managed configs to allow symlinking the whole swaync directory
    xdg.configFile."swaync/config.json".enable = lib.mkForce false;
    xdg.configFile."swaync/style.css".enable = lib.mkForce false;

    # Symlink the swaync directory
    xdg.configFile."swaync".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/swaync";
  };
}
