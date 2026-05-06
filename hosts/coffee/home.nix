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
  };

  editors = {
    zed.enable = false;
    neovim.enable = true;
  };

  cli = {
    tmux.enable = true;
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

  # Create directories
  home.activation.createCustomDirs = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    $DRY_RUN_CMD mkdir -p "$HOME/.config/sops/age"
    $DRY_RUN_CMD mkdir -p "$HOME/personal"
    $DRY_RUN_CMD mkdir -p "$HOME/personal/media"
    $DRY_RUN_CMD mkdir -p "$HOME/personal/obsidian"
    $DRY_RUN_CMD mkdir -p "$HOME/personal/projects"
    $DRY_RUN_CMD mkdir -p "$HOME/personal/playground"
    $DRY_RUN_CMD mkdir -p "$HOME/workspace"
    $DRY_RUN_CMD mkdir -p "$HOME/workspace/docs"
  '';

  home.stateVersion = "25.05";
  home.sessionVariables = {
    XDG_CONFIG_HOME = "$HOME/.config";
    TERM = "ghostty";
    EDITOR = "nvim";
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
