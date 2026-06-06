{
  pkgs,
  config,
  lib,
  ...
}:
lib.mkModule {
  globalConfig = config;
  name = "nixos.fonts";
  description = "NixOS system fonts";
  config = {
    fonts = {
      packages =
        (lib.commonFontPkgs pkgs)
        ++ (with pkgs; [
          source-sans
          comfortaa
          lexend
          jost
          dejavu_fonts
          noto-fonts
          noto-fonts-cjk-sans
          noto-fonts-color-emoji
          openmoji-color
          twemoji-color-font
          material-symbols
          libertinus
          nerd-fonts.symbols-only
          nerd-fonts.caskaydia-cove
          nerd-fonts.bigblue-terminal
          nerd-fonts.victor-mono
          nerd-fonts.mononoki
          nerd-fonts.heavy-data
          nerd-fonts.inconsolata
          nerd-fonts.fira-code
          nerd-fonts.meslo-lg
          maple-mono.truetype
          maple-mono.NF-unhinted
        ]);
      enableDefaultPackages = false;
    };
  };
}
