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

  # Regenerate only when the binary is present. A cache produced by an earlier
  # run can reference the tool it was generated from (atuin's does, via
  # `ATUIN_SESSION=$(atuin uuid)`), so sourcing it after the tool is
  # uninstalled printed `command not found` on every shell start.
  #
  # An existing, valid cache is still sourced when the binary is missing —
  # most of these files (fzf keybindings, zoxide init, direnv hook) are
  # still useful — so this is not an early return, only a regeneration guard.
  if [[ -n "$bin_path" ]] && { [[ ! -s "$cache_file" ]] || [[ "$bin_path" -nt "$cache_file" ]]; }; then
    # Generation is atomic and validated: the output goes to a temporary file
    # that is only moved into place if it is non-empty. A plain `>` would create
    # a zero-byte cache on a failed run, and since `-s` would then be false
    # while `-f` stayed true, the empty file would never be retried and the
    # integration would be disabled until someone deleted it by hand.
    local tmp="$cache_file.tmp"
    if "$@" > "$tmp" 2>/dev/null && [[ -s "$tmp" ]]; then
      mv "$tmp" "$cache_file"
    else
      rm -f "$tmp"
    fi
  fi

  [[ -s "$cache_file" ]] && source "$cache_file"
  return 0
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

# Add custom completions to fpath
fpath=("$HOME/.config/zsh/completions" $fpath)

# Nix packages ship their zsh completions under <pkg>/share/zsh/site-functions.
# Profiles are already on PATH, so mirror those completions onto fpath: this is
# what makes `coffee <TAB>` (and `co <TAB>`) offer clean, doctor, reap-zombies,
# generate-completion and the flags. No subprocess is spawned, and the paths are
# resolved per shell start so a rebuild never leaves a stale store path behind.
for profile in ${(z)NIX_PROFILES}; do
  fpath+=("$profile"/share/zsh/site-functions(N/))
done
fpath=($fpath(N/))

# zstyle before compinit: the completion system reads these settings as it is
# initialised, and sourcing them afterwards left them applying only to
# completions that had not yet been set up.
#
# Note that `cache-path` here is the *results* cache, not the compdump. The
# dump location is chosen by zsh itself (currently ~/.zcompdump); moving it
# would mean setting ZSH_COMPDUMP.
source "$HOME/.config/zsh/zstyle.zsh"

# compinit (fast: -C uses cached dump, skips re-scan)
autoload -Uz compinit && compinit -C

# `coffee` must win the `co` name: zsh's own RCS completion (_rcs) also declares
# `#compdef co`, and whichever file compinit happens to read last wins. Binding
# both names explicitly means `co <TAB>` offers clean, doctor, reap-zombies,
# --deep and the rest, regardless of which completion file was scanned first.
if (( $+functions[_coffee] )); then
  compdef _coffee coffee
  compdef _coffee co
fi

# carapace's cache also shells out to `carapace` while being sourced, for the
# same reason atuin's does below.
if (( $+commands[carapace] )); then
  _cached_eval carapace carapace _carapace
fi

# Keybindings
# Atuin bindings (must be in array BEFORE vi-mode init runs)
# Registered only when atuin is present: a widget backed by a missing binary
# fails on the keystroke rather than at startup.
if (( $+commands[atuin] )); then
  zvm_after_init_commands+=('bindkey -M viins "^k" atuin-up-search')
  zvm_after_init_commands+=('bindkey -M vicmd "^k" atuin-up-search')
  zvm_after_init_commands+=('bindkey -M viins "^r" atuin-search')
  zvm_after_init_commands+=('bindkey -M vicmd "^r" atuin-search')
fi
# Source remaining keybindings after vi-mode is ready
zvm_after_init_commands+=('source "$HOME/.config/zsh/Keybindings.zsh"')
zvm_after_init_commands+=('[ -f ~/.fzf.zsh ] && source ~/.fzf.zsh')

# zstyle.zsh is sourced above, before compinit. Not sourced again here.

# Native Prompt
autoload -U colors && colors

PROMPT="%(?:%{$fg_bold[green]%}%1{➜%}:%{$fg_bold[red]%}%1{➜%} ) %{$fg[cyan]%}%c%{$reset_color%}"
PROMPT+=' $(git_prompt_info)'

# The four ZSH_THEME_GIT_PROMPT_* variables that used to live here are gone:
# `gitprompt` emits the colour codes itself, byte-for-byte what they expanded
# to. They are documented in crates/gitprompt/src/render.rs, where the exact
# escape sequences are pinned by tests.

# Shell integrations (sheldon already sourced from cache above — do not re-source)
#
# atuin is called out because its generated cache runs the binary while it is
# being sourced (`export ATUIN_SESSION=$(atuin uuid)`), so an existing cache
# still errors after atuin is uninstalled. fzf, zoxide and direnv emit pure
# shell and are safe to source either way. The check is `command -v`, not the
# cache file, because the cache outlives the tool it came from.
if (( $+commands[atuin] )); then
  _cached_eval atuin atuin init zsh --disable-up-arrow
fi

_cached_eval fzf fzf --zsh
_cached_eval zoxide zoxide init --cmd cd zsh
