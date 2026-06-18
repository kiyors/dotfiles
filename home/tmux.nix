{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "cli.tmux";
  description = "Tmux with custom dotfiles symlink";
  config = {
    home.packages = with pkgs; [
      (sesh.overrideAttrs (old: {
        src = pkgs.fetchFromGitHub {
          owner = "joshmedeski";
          repo = "sesh";
          rev = "main";
          hash = "sha256-MqNwbWQ+zLFRQpwDBsiQSDj83Ef2OQAF5lRMSsWHUMI=";
        };
        vendorHash = "sha256-9IiDp/HaxXQAyNzuVBLiO+oIijBbdKBjssCmj8WV9V4=";
      }))
      tmuxinator
      yq
    ];
    programs.tmux = {
      enable = true;
      plugins = with pkgs.tmuxPlugins; [
        sessionist
      ];
    };

    # HM 26.11's programs.tmux writes xdg.configFile."tmux/tmux.conf", which
    # conflicts with the symlinked directory below. Disable it so our
    # dotfiles-managed tmux.conf wins. Source the sessionist plugin manually
    # (or via TPM) from within tmux.conf.
    xdg.configFile."tmux/tmux.conf".enable = lib.mkForce false;

    home.file.".config/tmux".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/tmux";

    home.file.".config/sesh".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/sesh";
  };
}
