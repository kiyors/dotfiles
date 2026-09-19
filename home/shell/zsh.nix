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
      fzf
      eza
      bat
      carapace
      vivid
      sheldon
    ];

    programs.zsh = {
      enable = true;
      initContent = ''
        # >>> dory cli >>>
        DORY_CLI_BIN="$HOME/.dory/bin"
        case ":$PATH:" in
          *":$DORY_CLI_BIN:"*) ;;
          *) export PATH="$DORY_CLI_BIN:$PATH" ;;
        esac
        # <<< dory cli <<<

        source "$HOME/.config/zsh/.zshrc"
      '';
      profileExtra = ''
        # >>> dory cli >>>
        DORY_CLI_BIN="$HOME/.dory/bin"
        case ":$PATH:" in
          *":$DORY_CLI_BIN:"*) ;;
          *) export PATH="$DORY_CLI_BIN:$PATH" ;;
        esac
        # <<< dory cli <<<
      '';
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
