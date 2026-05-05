{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "editors.neovim";
  description = "Nvim Editor with custom dotfiles symlink";
  config = {
    home.packages = with pkgs; [
      nixfmt
      statix
    ];

    programs.neovim = {
      enable = true;
      withRuby = false;
      withPython3 = true;
      extraPython3Packages =
        ps: with ps; [
          pynvim
          pip
        ];

      extraPackages = with pkgs; [
        tree-sitter
        lua54Packages.jsregexp
        nodejs_24
        vscode-langservers-extracted
        tailwindcss-language-server
        fzf
        unzip
        lua
        lua-language-server
        lua53Packages.luacheck
        luajitPackages.jsregexp
        lua51Packages.luarocks-nix
        luarocks
        nixd
        selene
        biome
        uv
        gopls
        gofumpt
        stylua
        rustfmt
        harper
        gnumake
        go
        gcc
        cargo
        rustc
        rustup
        ripgrep
        wordnet
        imagemagick
        libiconv
      ];

      extraWrapperArgs = [
        "--suffix"
        "LIBRARY_PATH"
        ":"
        "${lib.makeLibraryPath [ pkgs.stdenv.cc.cc.lib ]}"
      ];
    };

    # Disable HM-managed init.lua to allow symlinking the whole nvim directory
    xdg.configFile."nvim/init.lua".enable = lib.mkForce false;
    xdg.configFile."nvim".source =
      config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/nvim";
  };
}
