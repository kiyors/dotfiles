{
  description = "NixOS from Scratch";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs-stable.url = "github:nixos/nixpkgs/nixos-26.05";
    nix-darwin = {
      url = "github:nix-darwin/nix-darwin/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    home-manager = {
      url = "github:nix-community/home-manager/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixos-hardware.url = "github:oxodx/nixos-hardware";
    nixcord.url = "github:kaylorben/nixcord";
    nix-nanobrew = {
      url = "github:kiyors/nix-nanobrew";
    };
    nix-homebrew = {
      url = "github:zhaofengli-wip/nix-homebrew";
      inputs.brew-src.url = "github:Homebrew/brew";
    };
    homebrew-core = {
      url = "github:homebrew/homebrew-core";
      flake = false;
    };
    homebrew-cask = {
      url = "github:homebrew/homebrew-cask";
      flake = false;
    };
    homebrew-bundle = {
      url = "github:homebrew/homebrew-bundle";
      flake = false;
    };
    brew-nix = {
      url = "github:BatteredBunny/brew-nix";
      inputs.nix-darwin.follows = "nix-darwin";
      inputs.brew-api.follows = "brew-api";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    brew-api = {
      url = "github:BatteredBunny/brew-api";
      flake = false;
    };
    disko = {
      url = "github:nix-community/disko/latest";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    zen-browser = {
      url = "github:0xc000022070/zen-browser-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hyprland = {
      url = "github:hyprwm/Hyprland";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    vicinae.url = "github:vicinaehq/vicinae";
    vicinae-extensions = {
      url = "github:vicinaehq/extensions";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    betterfox = {
      url = "github:yokoffing/Betterfox";
      flake = false;
    };
    spicetify-nix = {
      url = "github:Gerg-L/spicetify-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    determinate = {
      url = "https://flakehub.com/f/DeterminateSystems/determinate/3";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nh = {
      url = "github:kiyors/nh";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nvf = {
      url = "github:notashelf/nvf";
    };
    ghostty = {
      url = "github:ghostty-org/ghostty";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      disko,
      ...
    }@inputs:
    let
      # Initialize our custom library
      lib = import ./lib { inherit inputs; };

      # Define our hosts
      hosts = {
        atlas = {
          isNixos = true;
          hostname = "atlas";
          username = "gaurav";
          system = lib.systems.x86_64-linux;
        };
        hades = {
          isNixos = true;
          hostname = "hades";
          username = "gaurav";
          system = lib.systems.x86_64-linux;
          extraModules = [
            disko.nixosModules.disko
            ./hosts/hades/disko-config.nix
          ];
        };
        coffee = {
          isDarwin = true;
          hostname = "coffee";
          username = "gaurav";
          system = lib.systems.aarch64-darwin;
        };
      };

      # Generate all configurations
      configs = lib.mkConfigurations hosts;
    in
    {
      # Expose lib for use within the flake and by other flakes
      inherit lib;

      # Standard formatter for all supported systems
      formatter = lib.forAllSystems (system: nixpkgs.legacyPackages.${system}.nixfmt);

      # Common overlays
      inherit (lib) overlays;

      # Automatically unpack generated configurations
      inherit (configs) nixosConfigurations darwinConfigurations homeConfigurations;
    };
}
