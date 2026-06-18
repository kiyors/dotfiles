# hosts/coffee/home.nix
{
  config,
  pkgs,
  lib,
  ...
}:
{

  home.packages = with pkgs; [
    # spotiflac
    # spotidownloader
    nyaa
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
    nvf.enable = true;
  };

  cli = {
    tmux.enable = true;
    lazydocker.enable = true;
  };

  shell = {
    zsh.enable = true;
    tools = {
      enable = true;
      bat.enable = true;
      atuin.enable = true;
      direnv.enable = true;
      fastfetch.enable = true;
    };
  };

  media = {
    mpv.enable = false;
  };

  versionControl = {
    git.enable = true;
    # jujutsu.enable = false;
  };
  secrets.sops.enable = true;

  wm.aerospace.enable = true;

  terminal.ghostty.enable = true;

  home.username = "gaurav";
  home.homeDirectory = "/Users/gaurav";

  xdg.userDirs.enable = false;

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

  home.stateVersion = "26.11";
  home.sessionVariables = {
    XDG_CONFIG_HOME = "$HOME/.config";
    TERM = "ghostty";
    SHELL = "zsh";
  };
  home.sessionPath = [
    "/run/current-system/sw/bin"
    "$HOME/.nix-profile/bin"
    "$HOME/.cargo/bin"
    "/opt/homebrew/bin"
  ];
  programs.home-manager.enable = true;
}
