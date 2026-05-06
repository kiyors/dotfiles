{
  myLib,
  config,
  lib,
  pkgs,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "versionControl.jujutsu";
  description = "Enable Jujutsu (jj) version control system";
  enableDefault = config.versionControl.git.enable;
  config = {
    home.packages = with pkgs; [
      jujutsu
      jjui
    ];

    programs.jujutsu = {
      enable = true;
      package = pkgs.jujutsu;
      # We intentionally leave `settings` out/empty so Home Manager
      # doesn't try to generate its own config.toml file.
    };
    home.file.".config/jj".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/jj";
  };
}
