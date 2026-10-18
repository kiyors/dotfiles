#!/usr/bin/env bash

session_name="$1"
session_path=$(tmux display-message -t "$session_name" -p "#{session_path}")

# Locate sesh.toml relative to this script rather than assuming the repo lives
# at ~/dotfiles. SCRIPT_DIR survives being symlinked into ~/.config/tmux, which
# is how the Nix layer installs this file.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SESH_CONFIG="$SCRIPT_DIR/../sesh/sesh.toml"

# Calculate desired name
desired_name=""
if [[ "$session_path" == "$HOME/workspace" ]]; then
  desired_name="workspace"
elif [[ "$session_path" == "$HOME/workspace/"* ]]; then
  parent=$(basename "$(dirname "$session_path")")
  base=$(basename "$session_path")
  desired_name="$parent/$base"
else
  parent=$(basename "$(dirname "$session_path")")
  base=$(basename "$session_path")
  if [[ "$parent" == $(basename "$HOME") ]] || [[ "$parent" == "/" ]]; then
    desired_name="$base"
  else
    desired_name="$parent/$base"
  fi
fi

desired_name=${desired_name//./_}
desired_name=${desired_name//:/_}

# Check if the session is explicitly named in sesh.toml
if [[ -f "$SESH_CONFIG" ]] && grep -qE "^name[[:space:]]*=[[:space:]]*[\"']${session_name}[\"']" "$SESH_CONFIG" 2>/dev/null; then
  exit 0
fi

# Only rename if the current name is different and the new name doesn't already exist
if [[ "$session_name" != "$desired_name" ]] && ! tmux has-session -t "$desired_name" 2>/dev/null; then
  tmux rename-session -t "$session_name" "$desired_name"
fi
