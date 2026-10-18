#!/bin/sh

if command -v bat &> /dev/null; then
  alias cat="bat -pp --theme \"Catppuccin Mocha\""
  alias catt="bat --theme \"Catppuccin Mocha\""
fi

function y() {
	local tmp="$(mktemp -t "yazi-cwd.XXXXXX")" cwd
	yazi "$@" --cwd-file="$tmp"
	if cwd="$(command cat -- "$tmp")" && [ -n "$cwd" ] && [ "$cwd" != "$PWD" ]; then
		builtin cd -- "$cwd"
	fi
	rm -f -- "$tmp"
}



# Direnv hook (cached)
_cached_eval direnv direnv hook zsh

# Base URL for the templates
# NIX_TEMPLATE_URL="https://flakehub.com/f/the-nix-way/dev-templates/*"
NIX_TEMPLATE_URL="github:kiyors/templates"

# Initialize the current directory with a template
function nix-init {
  local template="${1:-empty}"
  nix flake init --template "$NIX_TEMPLATE_URL#$template"
  direnv allow
}

# Create a new project with a template
function nix-new {
  local dir="$1"
  local template="${2:-empty}"

  if [[ -z "$dir" ]]; then
    echo "Usage: nix-new <directory> [template]"
    echo "Example: nix-new my-project node"
    return 1
  fi

  if [[ -d "$dir" ]]; then
    echo "Directory \"$dir\" already exists!"
    return 1
  fi

  nix flake new "$dir" --template "$NIX_TEMPLATE_URL#$template"
  cd "$dir"
  direnv allow
}

# ── Shell history backup ─────────────────────────────────────────────
# Bundles zsh history, atuin db, and zoxide db into a timestamped tarball
# under ~/Documents/shell-backups (override via $1).
#
# Usage:
#   backup-shell-history                 # → ~/Documents/shell-backups/...
#   backup-shell-history ~/Dropbox/bak   # custom destination
function backup-shell-history {
  emulate -L zsh
  setopt pipefail

  local dest_dir="${1:-$HOME/Documents/shell-backups}"
  local timestamp
  timestamp="$(date +%Y%m%d-%H%M%S)"
  local host_tag="${HOST:-$(hostname -s)}"
  local out="$dest_dir/shell-history-${host_tag}-${timestamp}.tar.gz"

  mkdir -p "$dest_dir" || return 1

  local staging
  staging="$(mktemp -d)" || { print -u2 "backup-shell-history: mktemp failed"; return 1; }

  local copied=0
  local src
  # zsh history
  src="${HISTFILE:-$HOME/.zsh_history}"
  if [[ -f "$src" ]]; then
    cp -p -- "$src" "$staging/zsh_history" && (( copied++ ))
  fi
  # atuin sqlite db (and key, if present, so restore reuses the same encryption)
  local atuin_dir="${XDG_DATA_HOME:-$HOME/.local/share}/atuin"
  if [[ -f "$atuin_dir/history.db" ]]; then
    cp -p -- "$atuin_dir/history.db" "$staging/atuin_history.db" && (( copied++ ))
    [[ -f "$atuin_dir/key" ]] && cp -p -- "$atuin_dir/key" "$staging/atuin_key"
  fi
  # zoxide db
  local zo_dir="${_ZO_DATA_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/zoxide}"
  if [[ -f "$zo_dir/db.zo" ]]; then
    cp -p -- "$zo_dir/db.zo" "$staging/zoxide_db.zo" && (( copied++ ))
  fi

  if (( copied == 0 )); then
    rm -rf -- "$staging"
    print -u2 "backup-shell-history: nothing found to back up"
    return 1
  fi

  if tar -czf "$out" -C "$staging" .; then
    rm -rf -- "$staging"
    print -- "backup-shell-history: $copied file(s) → $out"
  else
    rm -rf -- "$staging"
    print -u2 "backup-shell-history: tar failed"
    return 1
  fi
}

# Short alias
alias bkhist='backup-shell-history'

# ── Git Prompt Helpers ───────────────────────────────────────────────
# The prompt segment is produced by `gitprompt` (crates/gitprompt), which
# reads .git/HEAD directly and formats the escape sequences itself.
#
# The previous implementation shelled out six times per prompt — two command
# substitutions, `git symbolic-ref`, `git status`, and a pipe to `tail` — to
# compute a branch name and a dirty flag. Measured here at 29ms in an ordinary
# repository and 161ms in a 68k-file one, on every prompt draw.
#
# `gitprompt` prints nothing outside a repository, which is exactly what a
# prompt substitution wants, so no guard is needed here.
function git_prompt_info() {
  command gitprompt 2> /dev/null
}

function portkill() {
  if [ -z "$1" ]; then
    echo "Usage: portkill <port_number>"
    return 1
  fi
  kill -9 $(lsof -t -i:$1)
}
