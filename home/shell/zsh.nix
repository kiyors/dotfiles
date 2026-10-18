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

      # Backs `git_prompt_info` in config/zsh/functions.zsh. Put in
      # `home.packages` rather than referenced through the `pkgs` overlay
      # attribute, so it resolves without depending on the overlay being
      # applied.
      gitprompt
    ];

    programs.zsh = {
      enable = true;

      # Home Manager emits `autoload -U compinit && compinit` ahead of
      # `initContent`, so the uncached run happens *before* our own
      # `compinit -C` in config/zsh/.zshrc can take effect. Measured on this
      # machine that redundant scan is ~930ms of a ~1.1s interactive startup:
      # it walks all 24 fpath entries and re-runs every `_compdef`.
      #
      # Emptying it leaves config/zsh/.zshrc as the single `compinit`, which
      # already passes `-C`. Verified: both configurations load an identical
      # completion set (2566 entries, `_comps[coffee]` present).
      #
      # This is deliberately a Nix-layer setting. A non-Nix user copying
      # config/zsh/.zshrc gets the correct `-C` behaviour with no extra help,
      # so the portable half of the config stays portable.
      completionInit = "";

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

    home.file = {
      ".config/zsh".source =
        config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/zsh";

      ".config/sheldon".source =
        config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/sheldon";
    }
    // lib.optionalAttrs pkgs.stdenv.hostPlatform.isDarwin {
      ".config/mole".source =
        config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/mole";
    };

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
