{ pkgs, ... }:
{
  spotidownloader = pkgs.callPackage ./spotidownloader.nix { };
  spotiflac = pkgs.callPackage ./spotiflac.nix { };
  superCmd = pkgs.callPackage ./supercmd.nix { };
  sysClean = pkgs.callPackage ./sys-clean.nix { };
}
