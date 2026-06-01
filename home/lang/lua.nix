{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "lang.lua";
  description = "Lua development environment (interpreter, LSP, linters, formatter)";
  config = {
    home.packages = with pkgs; [
      # Interpreter + package manager
      # Use luarocks-nix (nix-aware) over top-level luarocks to avoid
      # `.luarocks-wrapped` collision; lazy.nvim's rocks support uses it.
      lua
      lua51Packages.luarocks-nix

      # LSP + linters
      lua-language-server
      selene
      lua53Packages.luacheck

      # Formatter
      stylua
    ];
  };
}
