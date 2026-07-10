{
  pkgs,
  inputs,
  self,
  ...
}:
{
  imports = [
    ./homebrew.nix
    ./nanobrew.nix
    ./settings.nix
    ./packages.nix
    ./fonts.nix
    ./determinateNix.nix
    ./nix.nix
    ./zombie-reaper.nix
  ];
}
