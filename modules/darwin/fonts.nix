{
  pkgs,
  config,
  lib,
  ...
}:
lib.mkModule {
  globalConfig = config;
  name = "darwin.fonts";
  description = "macOS system fonts configuration";
  config = {
    fonts = {
      packages = with pkgs; [
        roboto
        work-sans
        cantarell-fonts
        comic-neue
        inter
        lato
        (google-fonts.override { fonts = [ "Inter" ]; })
        jetbrains-mono
        nerd-fonts.jetbrains-mono
        nerd-fonts.zed-mono
      ];
    };
  };
}
