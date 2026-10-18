{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "cli.nh";
  description = "nh Nix helper tools";
  enableDefault = config.cli.enable or false;
  config = {
    programs.nh = {
      enable = true;
      clean = {
        enable = true;
        # No extraArgs: on darwin the home-manager module passes the whole
        # string as a single argv element, so a value like
        # "--delete-older-than 0d --gc" reached nh as one bogus argument (and
        # neither flag exists on `nh clean`). nh's defaults already keep the
        # current generation of each user profile and run the store gc.
      };
      flake = "${config.home.homeDirectory}/dotfiles";
    };
  };
}
