{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "desktop.waybar";
  description = "Waybar status bar with custom config symlink";
  config = {
    home.packages = with pkgs; [
      waybar
      networkmanagerapplet
      blueman
      playerctl
      vivid
    ];

    programs.waybar = {
      enable = true;
      systemd.enable = true;
    };

    # Symlink the waybar directory
    xdg.configFile."waybar".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/waybar";

    wayland.windowManager.hyprland.settings.exec-once = [
      "waybar"
      "nm-applet"
      "blueman-applet"
    ];
  };
}
