{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "desktop.hyprland";
  description = "Hyprland compositor with custom config symlink";
  config = {
    home.packages = with pkgs; [
      qt5.qtwayland
      qt6.qtwayland
      libsForQt5.qt5ct
      qt6Packages.qt6ct
      hyprshot
      hyprpicker
      swappy
      imv
      wf-recorder
      wlr-randr
      wl-clipboard
      brightnessctl
      gnome-themes-extra
      libva
      dconf
      wayland-utils
      wayland-protocols
      glib
      direnv
      meson
      hyprpolkitagent
      google-chrome
      hypridle
      hyprpaper
      wl-clip-persist
    ];

    wayland.windowManager.hyprland = {
      enable = true;
      xwayland.enable = true;
      systemd.enable = false; # Handled by custom launch or lua config
    };

    # Symlink the hypr directory to allow Lua/Conf based configuration
    xdg.configFile."hypr".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/hypr";
  };
}
