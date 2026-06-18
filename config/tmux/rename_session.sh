#!/usr/bin/env bash

session_name="$1"
session_path=$(tmux display-message -t "$session_name" -p "#{session_path}")

# Calculate desired name
desired_name=""
if [[ "$session_path" == "$HOME/workspace" ]]; then
  desird_name="ndow -c "#{pane_current_path}" && tmux send-keys -t 2 nvim C-m
    workspace"
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
if grep -qE "^name[[:space:]]*=[[:space:]]*[\"']${session_name}[\"']" "$HOME/dotfiles/config/sesh/sesh.toml" 2>/dev/null; then
  exit 0
fi

# Only rename if the current name is different and the new name doesn't already exist
if [[ "$session_name" != "$desired_name" ]] && ! tmux has-session -t "$desired_name" 2>/dev/null; then
  tmux rename-session -t "$session_name" "$desired_name"
fi
