{ pkgs, ... }:
{
  moviebox-tui = pkgs.callPackage ./moviebox-tui.nix { };
  multica = pkgs.callPackage ./multica.nix { };
  multica-cli = pkgs.callPackage ./multica-cli.nix { };
  spotidownloader = pkgs.callPackage ./spotidownloader.nix { };
  spotiflac = pkgs.callPackage ./spotiflac.nix { };
  superCmd = pkgs.callPackage ./supercmd.nix { };
  sysClean = pkgs.callPackage ./sys-clean.nix { };
  recordly = pkgs.callPackage ./recordly.nix { };
  kache = pkgs.callPackage ./kache.nix { };
  # tinycast = pkgs.callPackage ./tinycast.nix { };
}
