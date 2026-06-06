#!/bin/sh

alias c='clear'
alias vi='nvim'
alias vim='nvim'
alias lzg='lazygit'
alias lzd='lazydocker'
alias ff='fastfetch'
alias logs='git log --graph --all --pretty=format:"%C(magenta)%h %C(white) %an  %ar%C(auto)  %D%n%s%n"'

# Colorize grep output (good for log files)
alias grep='rg'

# confirm before overwriting something
alias cp="cp -i"
alias mv='mv -i'
alias rm='rm -i'

# Create alias override commands using 'eza'
alias ls='eza --icons=always --color=auto'
alias ll='eza -lh --git --icons=always'
alias la='eza -a --icons=always'
alias tree='eza --tree --level=2 --icons=always'
