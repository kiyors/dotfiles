{
  inputs,
  config,
  lib,
  ...
}:
lib.mkModule {
  globalConfig = config;
  name = "nixos.nix";
  description = "NixOS package manager subsystem";
  config = {
    nixpkgs.config.allowUnfree = true;
    nixpkgs.config.allowBroken = true;
    nix = {
      channel.enable = false;
      extraOptions = "warn-dirty = false";
      optimise.automatic = true;
      settings = {
        download-buffer-size = 262144000;
        experimental-features = [
          "nix-command"
          "flakes"
        ];
        substituters = [
          "https://cache.nixos.org?priority=10"
          "https://nix-community.cachix.org"
          "https://hyprland.cachix.org"
          "https://numtide.cachix.org"
        ];
        trusted-public-keys = [
          "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
          "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
          "hyprland.cachix.org-1:G7vB76S94yH3766xZ1tTcs0H6Qh4/7i5v4u1tA1+39Y="
          "numtide.cachix.org-1:2ps1kLBUWjxIneOy1Ik6cQjb41X0iXVXeHigGmycPPE="
        ];
        extra-substituters = [ "https://vicinae.cachix.org" ];
        extra-trusted-public-keys = [ "vicinae.cachix.org-1:1kDrfienkGHPYbkpNj1mWTr7Fm1+zcenzgTizIcI3oc=" ];
        builders-use-substitutes = true;
        max-jobs = 1;
        cores = 1;
        fallback = true;
        connect-timeout = 60;
        stalled-download-timeout = 300;
      };
      gc = {
        automatic = true;
        persistent = true;
        dates = "weekly";
        options = "--delete-older-than 3d";
      };
    };
    zramSwap = {
      enable = true;
      priority = 100;
    };
  };
}
