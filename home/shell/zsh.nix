{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "shell.zsh";
  description = "Zsh shell environment";
  config = {
    home.packages = with pkgs; [
      ripgrep
      tldr
      yq
      fd
      zoxide
      yazi
      fzf
      eza
      bat
      carapace
      vivid
      sheldon
    ];

    programs.zsh = {
      enable = true;
      initContent = ''source "$HOME/.config/zsh/.zshrc"'';
    };

    home.file.".config/zsh".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/zsh";

    home.file.".config/sheldon".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/sheldon";

    home.activation.initZsh = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      $DRY_RUN_CMD mkdir -p $HOME/.cache/zsh
      # Fix for stty issue on macOS when coreutils is installed
      if [ "$(uname)" = "Darwin" ]; then
        $DRY_RUN_CMD mkdir -p $HOME/.local/bin
        $DRY_RUN_CMD ln -sf /bin/stty $HOME/.local/bin/stty
      fi
    '';
  };
}
