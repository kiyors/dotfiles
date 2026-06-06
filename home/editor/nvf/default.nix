{
  myLib,
  config,
  lib,
  inputs,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "editors.nvf";
  description = "Neovim Flake (NVF)";
  imports = [ inputs.nvf.homeManagerModules.default ];
  config = {
    programs.nvf = {
      enable = true;
      settings = {
        imports = [
          ./options.nix
          ./languages.nix
          ./picker.nix
          ./snacks.nix
          ./keymaps.nix
          ./utils.nix
          ./mini.nix
        ];
      };
    };

    home.shellAliases = {
      nvf = "${config.programs.nvf.finalPackage}/bin/nvim";
      ni = "nvf";
    };
  };
}
