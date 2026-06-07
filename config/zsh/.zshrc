#!/bin/sh

# Should be called before compinit
zmodload zsh/complist

# Init cache for heavy eval tools
# Writes static cache files so we avoid fork+exec on every startup.
typeset -g ZSH_CACHE_DIR="$HOME/.cache/zsh"
[[ -d "$ZSH_CACHE_DIR" ]] || mkdir -p "$ZSH_CACHE_DIR"

function _cached_eval {
  local name=$1; shift
  local cache_file="$ZSH_CACHE_DIR/$name.zsh"
  local bin_path="${commands[$1]:-}"

  # Rebuild cache if missing or if the binary is newer
  if [[ ! -f "$cache_file" ]] || [[ -n "$bin_path" && "$bin_path" -nt "$cache_file" ]]; then
    "$@" > "$cache_file" 2>/dev/null
  fi
  source "$cache_file"
}

# Catppuccin theme (must be sourced BEFORE syntax-highlighting loads)
source "$HOME/.config/zsh/catppuccin_mocha-zsh-syntax-highlighting.zsh"

# Source config modules
source "$HOME/.config/zsh/exports.zsh"
source "$HOME/.config/zsh/aliases.zsh"
source "$HOME/.config/zsh/Keybindings.zsh"
source "$HOME/.config/zsh/functions.zsh"

# Sheldon – plugin manager (cached: regenerates when lock file changes)
ZVM_VI_INSERT_ESCAPE_BINDKEY=jk
ZVM_SYSTEM_CLIPBOARD_ENABLED=true
# Load ZVM synchronously so keys pressed right after the prompt aren't dropped.
ZVM_INIT_MODE=sourcing
if (( $+commands[sheldon] )); then
  () {
    local sheldon_cache="$ZSH_CACHE_DIR/sheldon.zsh"
    local sheldon_toml="$HOME/.config/sheldon/plugins.toml"
    local sheldon_lock="$HOME/.local/share/sheldon/plugins.lock"
    [[ -f "$sheldon_lock" ]] || sheldon_lock="$HOME/.config/sheldon/plugins.lock"

    # Regenerate cache if missing, if config is newer, or if lockfile is newer.
    if [[ ! -s "$sheldon_cache" || "$sheldon_toml" -nt "$sheldon_cache" || ( -f "$sheldon_lock" && "$sheldon_lock" -nt "$sheldon_cache" ) ]]; then
      local tmp="$sheldon_cache.tmp"
      if [[ ! -s "$sheldon_cache" ]]; then
        echo "Sheldon: Initializing plugins (first run)..."
        if sheldon source > "$tmp" && [[ -s "$tmp" ]]; then
          mv "$tmp" "$sheldon_cache"
        else
          rm -f "$tmp"
        fi
      else
        # Silent update for subsequent runs
        sheldon source > "$tmp" 2>/dev/null && [[ -s "$tmp" ]] && mv "$tmp" "$sheldon_cache" || rm -f "$tmp"
      fi
    fi

    if [[ -s "$sheldon_cache" ]]; then
      source "$sheldon_cache"
    else
      # Fallback: source live if cache generation failed
      eval "$(sheldon source)"
    fi
  }
fi

# compinit (fast: -C uses cached dump, skips re-scan)
autoload -Uz compinit && compinit -C
_cached_eval carapace carapace _carapace

# Keybindings
# Atuin bindings (must be in array BEFORE vi-mode init runs)
zvm_after_init_commands+=('bindkey -M viins "^k" atuin-up-search')
zvm_after_init_commands+=('bindkey -M vicmd "^k" atuin-up-search')
zvm_after_init_commands+=('bindkey -M viins "^r" atuin-search')
zvm_after_init_commands+=('bindkey -M vicmd "^r" atuin-search')
# Source remaining keybindings after vi-mode is ready
zvm_after_init_commands+=('source "$HOME/.config/zsh/Keybindings.zsh"')
zvm_after_init_commands+=('[ -f ~/.fzf.zsh ] && source ~/.fzf.zsh')

# zstyle (completion styling)
source "$HOME/.config/zsh/zstyle.zsh"


# Native Prompt
autoload -U colors && colors

PROMPT="%(?:%{$fg_bold[green]%}%1{➜%}:%{$fg_bold[red]%}%1{➜%} ) %{$fg[cyan]%}%c%{$reset_color%}"
PROMPT+=' $(git_prompt_info)'

ZSH_THEME_GIT_PROMPT_PREFIX="%{$fg_bold[blue]%}git:(%{$fg[red]%}"
ZSH_THEME_GIT_PROMPT_SUFFIX="%{$reset_color%} "
ZSH_THEME_GIT_PROMPT_DIRTY="%{$fg[blue]%}) %{$fg[yellow]%}%1{✘%}"
ZSH_THEME_GIT_PROMPT_CLEAN="%{$fg[blue]%})"

# Shell integrations (sheldon already sourced from cache above — do not re-source)
_cached_eval fzf fzf --zsh
_cached_eval zoxide zoxide init --cmd cd zsh
_cached_eval atuin atuin init zsh --disable-up-arrow
