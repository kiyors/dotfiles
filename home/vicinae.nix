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
    services.vicinae = {
      enable = true;
      systemd = {
        enable = true;
        autoStart = true;
        environment = {
          USE_LAYER_SHELL = 1;
        };
      };
      settings = {
        close_on_focus_loss = true;
        consider_preedit = true;
        pop_to_root_on_close = true;
        favicon_service = "twenty";
        search_files_in_root = true;
        font = {
          normal = {
            size = 12;
            family = "Maple Mono";
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
          opacity = 0.98;
        };
        # Import secrets if they exist (handled in host-specific secrets module)
        imports =
          let
            secretsPath = "${config.home.homeDirectory}/.config/vicinae/secrets.json";
          in
          pkgs.lib.optional (builtins.pathExists secretsPath) secretsPath;
      };
      extensions = with inputs.vicinae-extensions.packages.${pkgs.stdenv.hostPlatform.system}; [
        bluetooth
        nix
        power-profile
      ];
    };
  };
}
