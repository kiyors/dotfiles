{
  myLib,
  inputs,
  pkgs,
  config,
  lib,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "desktop.vicinae";
  description = "Vicinae launcher and clipboard manager";
  imports = [ inputs.vicinae.homeManagerModules.default ];
  config = {
    programs.vicinae = {
      enable = true;
      package = pkgs.vicinae;
      systemd = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
        enable = true;
        autoStart = true;
      };
      launchd = lib.mkIf pkgs.stdenv.hostPlatform.isDarwin {
        enable = true;
        autoStart = true;
        environment = {
          PATH = "${config.home.homeDirectory}/.nix-profile/bin:/run/current-system/sw/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin";
        };
      };
      settings = {
        "$schema" = "https://vicinae.com/schemas/config.json";
        close_on_focus_loss = true;
        consider_preedit = true;
        pop_to_root_on_close = false;
        escape_key_behavior = "close_window";
        favicon_service = "twenty";
        search_files_in_root = true;
        global_shortcuts = {
          toggle = if pkgs.stdenv.hostPlatform.isDarwin then "cmd+SPACE" else "alt+SPACE";
        };
        font = {
          normal = {
            family = "JetBrainsMono Nerd Font Propo";
            size = 13;
          };
        };
        theme = {
          light = {
            name = "vicinae-light";
            icon_theme = "default";
          };
          dark = {
            name = "vicinae-dark";
            icon_theme = "default";
          };
        };
        launcher_window = {
          compact_mode = {
            enabled = true;
          };
          material = "liquid_glass";
          opacity = 0.83;
        };
        providers = {
          applications = {
            enabled = true;
          };
          clipboard = {
            entrypoints = {
              history = {
                shortcut = if pkgs.stdenv.hostPlatform.isDarwin then "alt+v" else "super+v";
              };
            };
          };
          "@knoopx/vicinae-extension-nix-0" = {
            preferences = {
              homeManagerOptionsUrl = "https://home-manager-options.extranix.com/data/options-master.json";
              searchUrl = "https://search.nixos.org/backend/latest-48-nixos-unstable/_search";
            };
          };
        };
      };
      extensions =
        let
          exts = inputs.vicinae-extensions.packages.${pkgs.stdenv.hostPlatform.system};
        in
        [ exts.nix ]
        ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux (
          map (name: exts.${name}) (
            lib.filter (name: exts ? ${name}) [
              "power-profile"
              "wifi-commander"
            ]
          )
        );
    };
    systemd.user.services.vicinae.Service.Environment = lib.mkIf pkgs.stdenv.hostPlatform.isLinux [
      "USE_LAYER_SHELL=1"
      "XDG_DATA_DIRS=${config.home.homeDirectory}/.nix-profile/share:/etc/profiles/per-user/${config.home.username}/share:/run/current-system/sw/share:/usr/share:${config.home.homeDirectory}/.local/share"
      "PATH=${
        lib.makeBinPath [ pkgs.pulseaudio ]
      }:${pkgs.vicinae}/libexec/vicinae:${config.home.homeDirectory}/.nix-profile/bin:/etc/profiles/per-user/${config.home.username}/bin:/run/current-system/sw/bin:/run/wrappers/bin"
    ];
    home.packages = lib.mkIf pkgs.stdenv.hostPlatform.isLinux [ pkgs.pulseaudio ];
  };
}
