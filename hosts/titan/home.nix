{
  config,
  pkgs,
  lib,
  ...
}:
{
  imports = [
    ../../home
    ./secrets
  ];

  lang = {
    rust.enable = true;
    node.enable = true;
    lua.enable = true;
  };

  versionControl.git.enable = true;
  shell = {
    zsh.enable = true;
    tools = {
      enable = true;
      starship.enable = true;
      direnv.enable = true;
      fastfetch.enable = true;
      bat.enable = true;
      atuin.enable = true;
    };
  };
  editors = {
    zed.enable = true;
    neovim.enable = true;
  };

  desktop = {
    hyprland.enable = true;
    vicinae.enable = true;
    waybar.enable = true;
    swaync.enable = true;
    swayosd.enable = true;
  };

  network.impala.enable = true;
  network.bluetui.enable = true;
  terminal.ghostty.enable = true;

  cli = {
    tmux.enable = true;
  };

  media = {
    mpv.enable = false;
  };

  secrets.sops.enable = true;

  home.username = "gaurav";
  home.homeDirectory = "/home/gaurav";

  home.packages = with pkgs; [
    kdePackages.kate
    gemini-cli
  ];

  xdg.userDirs = {
    enable = true;
    createDirectories = true;
  };

  home.customDirs = [
    ".config/sops/age"
    "personal"
    { "personal/media" = "Movies/media"; }
    "personal/obsidian"
    "personal/projects"
    "personal/playground"
    "workspace"
    "workspace/docs"
  ];

  home.sessionVariables = {
    XDG_CONFIG_HOME = "$HOME/.config";
    TERM = "ghostty";
    SHELL = "${pkgs.zsh}/bin/zsh";
  };

  home.sessionPath = [
    "$HOME/.nix-profile/bin"
    "$HOME/.cargo/bin"
  ];

  programs.home-manager.enable = true;
  home.stateVersion = "26.11";
}
