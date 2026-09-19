# hosts/coffee/home.nix
{
  config,
  pkgs,
  lib,
  ...
}:
{

  home.packages = with pkgs; [
    # multica
    # multica-cli
    # spotiflac
    # spotidownloader
    # nyaa
    # motrix-next
    moviebox-tui
    # recordly
  ];

  imports = [
    ../../home
    ./secrets
  ];

  lang = {
    zig.enable = true;
    rust.enable = true;
    node.enable = true;
    python.enable = true;
    lua.enable = true;
  };

  editors = {
    zed.enable = true;
    neovim.enable = true;
  };

  cli = {
    enable = true;
    tmux.enable = true;
    herdr.enable = true;
    lazydocker.enable = true;
  };

  shell = {
    zsh.enable = true;
    tools = {
      enable = true;
      atuin.enable = true;
      direnv.enable = true;
    };
  };

  media = {
    mpv.enable = false;
  };

  versionControl = {
    git.enable = true;
    jujutsu.enable = false;
  };
  secrets.sops.enable = true;

  desktop = {
    vicinae.enable = true;
  };

  wm.aerospace.enable = true;

  terminal.ghostty.enable = true;

  home = {
    username = "gaurav";
    homeDirectory = "/Users/gaurav";
    customDirs = [
      ".config/sops/age"
      "personal"
      { "personal/media" = "Movies/media"; }
      "personal/obsidian"
      "personal/projects"
      "personal/projects/learn"
      "personal/playground"
      "workspace"
      "workspace/docs"
    ];
    sessionVariables = {
      XDG_CONFIG_HOME = "$HOME/.config";
      TERM = "ghostty";
      SHELL = "zsh";
    };
    stateVersion = "26.11";
    sessionPath = [
      "/run/current-system/sw/bin"
      "$HOME/.nix-profile/bin"
      "$HOME/.cargo/bin"
      "/opt/homebrew/bin"
    ];
  };

  xdg.userDirs.enable = false;
  programs.home-manager.enable = true;
}
