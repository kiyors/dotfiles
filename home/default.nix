{ ... }:
{
  imports = [
    # ./nushell.nix
    # ./zen
    # ./zen.nix

    ./lang
    ./editor
    ./tmux.nix
    ./git
    ./shell
    ./lazydocker.nix
    ./ghostty.nix
    ./impala.nix
    ./bluetui.nix
    ./hyprland.nix
    ./vicinae.nix
    ./waybar.nix
    ./swaync.nix
    ./swayosd.nix
    ./aerospace.nix
    ./mpv.nix
    ./spicetify.nix
    ./sops.nix
    ./dirs.nix
    ./hermes.nix
  ];

  editors = {
    neovim.enable = true;
  };

  # Lua dev tooling backs the Neovim config (LSP, selene, stylua, luarocks)
  lang.lua.enable = true;

  # Baseline session defaults shared across hosts. Per-host home.nix may
  # extend this attrset; it won't conflict with single-value defaults here.
  home.sessionVariables = {
    EDITOR = "nvim";
  };

  # Disable manual generation to avoid options.json warning
  manual.manpages.enable = false;
  manual.json.enable = false;
  manual.html.enable = false;
}
