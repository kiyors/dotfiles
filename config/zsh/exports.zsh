#!/bin/sh

# History
HISTFILE=~/.zsh_history
HISTSIZE=1000000
SAVEHIST=$HISTSIZE
HISTDUP=erase

setopt APPEND_HISTORY
setopt SHARE_HISTORY
setopt HIST_IGNORE_SPACE
setopt HIST_IGNORE_DUPS
setopt INC_APPEND_HISTORY
setopt HIST_SAVE_NO_DUPS
setopt HIST_FIND_NO_DUPS
export HISTORY_IGNORE="(ls|cd|pwd|exit|sudo reboot|history|cd -|cd ..)"

# Shell Options
setopt autocd
setopt correct
setopt interactivecomments
setopt magicequalsubst
setopt nonomatch
setopt notify
setopt numericglobsort
setopt promptsubst
setopt MENU_COMPLETE
setopt AUTO_LIST
setopt COMPLETE_IN_WORD

# Editors & Tools
export EDITOR="nvim"
export VISUAL="nvim"
export SUDO_EDITOR="nvim"
export TERMINAL="ghostty"
export MANPAGER='nvim +Man!'
export MANWIDTH=999

# XDG & Data Dirs
export XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}"
export XDG_CACHE_HOME="${XDG_CACHE_HOME:-$HOME/.cache}"
export XDG_DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
export XDG_STATE_HOME="${XDG_STATE_HOME:-$HOME/.local/state}"
export _ZO_DATA_DIR="$XDG_CONFIG_HOME/zoxide"
export SOPS_AGE_KEY_FILE="$XDG_CONFIG_HOME/sops/age/keys.txt"
export EZA_CONFIG_DIR="$XDG_CONFIG_HOME/eza"
export GOPATH="$XDG_CONFIG_HOME/go"

# PATH
export PATH="$HOME/.local/bin":$PATH
export PATH=$PATH:/usr/bin
export PATH="$HOME/.npm/bin:$PATH"
export PATH="$HOME/.docker/bin":$PATH
export PATH="$HOME/.local/nvim-macos-arm64/bin":$PATH
export PATH=$HOME/.cargo/bin:$PATH
export PATH=$HOME/.local/share/go/bin:$PATH
export PATH=$HOME/.cache/.bun/bin:$PATH
export PATH="$HOME/.local/share/neovim/bin":$PATH

# pnpm
export PNPM_HOME="$HOME/.local/share/pnpm"
path=("$PNPM_HOME" $path)

# PHP / Composer
export COMPOSER_HOME="$XDG_CONFIG_HOME/composer"
export PATH="$COMPOSER_HOME/vendor/bin:$PATH"

# Nix
export NIX_CONFIG="experimental-features = nix-command flakes"

# FZF
export FZF_DEFAULT_COMMAND='rg --files --no-ignore-vcs --hidden'
export FZF_DEFAULT_OPTS=" \
--color=bg+:#313244,bg:#1E1E2E,spinner:#F5E0DC,hl:#F38BA8 \
--color=fg:#CDD6F4,header:#F38BA8,info:#CBA6F7,pointer:#F5E0DC \
--color=marker:#B4BEFE,fg+:#CDD6F4,prompt:#CBA6F7,hl+:#F38BA8 \
--color=selected-bg:#45475A \
--color=border:#6C7086,label:#CDD6F4"

# Carapace Theme (Catppuccin Mocha)
export CARAPACE_BRIDGES='zsh,fish,bash,inshellisense'
export CARAPACE_HIGHLIGHT_DESCRIPTION='38;5;243:italic'
export CARAPACE_HIGHLIGHT_FLAG='38;5;116'
export CARAPACE_HIGHLIGHT_FLAGARG='38;5;222'
export CARAPACE_HIGHLIGHT_FLAGMULTIARG='38;5;222'
export CARAPACE_HIGHLIGHT_FLAGNOARG='38;5;147'
export CARAPACE_HIGHLIGHT_DEFAULT='38;5;188'
export CARAPACE_HIGHLIGHT_KEYWORDNEGATIVE='38;5;210'
export CARAPACE_HIGHLIGHT_KEYWORDPOSITIVE='38;5;156'
export CARAPACE_HIGHLIGHT_KEYWORDAMBIGUOUS='38;5;222'
export CARAPACE_HIGHLIGHT_KEYWORDUNKNOWN='38;5;243'
export CARAPACE_HIGHLIGHT_VALUE='38;5;213'

# LS_COLORS (cached: avoids forking vivid every shell).
# Atomic write — only commit a non-empty result so a transient vivid failure
# never wedges all future shells with a broken cache.
if [[ -n "${commands[vivid]:-}" ]]; then
  _vivid_cache="$XDG_CACHE_HOME/zsh/vivid_ls_colors.zsh"
  if [[ ! -s "$_vivid_cache" || "${commands[vivid]}" -nt "$_vivid_cache" ]]; then
    [[ -d "$XDG_CACHE_HOME/zsh" ]] || mkdir -p "$XDG_CACHE_HOME/zsh"
    if _vivid_out="$(vivid generate molokai 2>/dev/null)" && [[ -n "$_vivid_out" ]]; then
      print -r -- "export LS_COLORS=\"$_vivid_out\"" > "$_vivid_cache.tmp" \
        && mv "$_vivid_cache.tmp" "$_vivid_cache"
    fi
    unset _vivid_out
  fi
  [[ -s "$_vivid_cache" ]] && source "$_vivid_cache"
fi

# macOS-specific
case "$(uname -s)" in
Darwin)
  export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib
  # Homebrew
  export HOMEBREW_NO_ANALYTICS=1
  export HOMEBREW_NO_ENV_HINTS=1

  # Use native stty to avoid conflicts with GNU coreutils on macOS.
  # Alias covers interactive shell; the $HOME/.local/bin/stty symlink
  # ensures subprocesses (fzf, atuin, etc.) also see /bin/stty via PATH.
  alias stty='/bin/stty'
  if [[ ! -e "$HOME/.local/bin/stty" ]]; then
    [[ -d "$HOME/.local/bin" ]] || mkdir -p "$HOME/.local/bin"
    ln -sf /bin/stty "$HOME/.local/bin/stty"
  fi

  # Ensure standard system tools are explicitly discoverable
  export PATH="/usr/bin:/usr/sbin:/usr/local/bin:$PATH"

  # Ensure your local environment always clears a path for the standalone toolchain
  unset SDKROOT
  ;;
esac
