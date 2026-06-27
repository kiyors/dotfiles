{ inputs }:
let
  inherit (inputs)
    self
    nixpkgs
    nix-darwin
    home-manager
    ;
  lib = nixpkgs.lib;

  # Define systems as an attrset for easy access
  systems = {
    x86_64-linux = "x86_64-linux";
    aarch64-linux = "aarch64-linux";
    aarch64-darwin = "aarch64-darwin";
  };

  # Helper to generate attributes for all systems
  forAllSystems = lib.genAttrs (builtins.attrValues systems);

  # Import overlays once and reuse
  overlays = import ../overlays { inherit inputs; };

  # Standard overlays for all hosts
  standardOverlays = [
    overlays.additions
    overlays.modifications
  ];

  # Platform helpers
  isDarwin = system: lib.hasSuffix "darwin" system;

  # Helper to create pkgs with overlays
  mkPkgs =
    system:
    import nixpkgs {
      inherit system;
      inherit (nixpkgs) lib;
      config.allowUnfree = true;
      overlays = standardOverlays ++ (lib.optional (isDarwin system) inputs.brew-nix.overlays.default);
    };

in
rec {
  inherit
    systems
    forAllSystems
    standardOverlays
    overlays
    isDarwin
    mkPkgs
    ;

  # Helper for NixOS configurations
  mkNixosHost =
    {
      hostname,
      system ? systems.x86_64-linux,
      username ? "gaurav",
      withHomeManager ? true,
      extraModules ? [ ],
      extraOverlays ? [ ],
      ... # Ignore other flags
    }:
    lib.nixosSystem {
      inherit system;
      specialArgs = {
        inherit inputs self;
        lib = self.lib;
        isNixOS = true;
        isDarwin = false;
      };

      modules = [
        ../hosts/${hostname}/default.nix
        { nixpkgs.overlays = standardOverlays ++ extraOverlays; }
      ]
      ++ lib.optionals withHomeManager [
        home-manager.nixosModules.home-manager
        {
          home-manager.useGlobalPkgs = false;
          home-manager.useUserPackages = true;
          home-manager.users.${username} = {
            imports = [ ../hosts/${hostname}/home.nix ];
            home.username = lib.mkForce username;
            home.homeDirectory = lib.mkForce "/home/${username}";
          };
          home-manager.extraSpecialArgs = {
            inherit inputs self;
            myLib = self.lib;
          };
          home-manager.backupFileExtension = "backup";
        }
      ]
      ++ extraModules;
    };

  # Helper for Darwin configurations
  mkDarwinHost =
    {
      hostname,
      system ? systems.aarch64-darwin,
      username ? "gaurav",
      withHomeManager ? true,
      extraModules ? [ ],
      extraOverlays ? [ ],
      ... # Ignore other flags
    }:
    nix-darwin.lib.darwinSystem {
      inherit system;
      specialArgs = {
        inherit inputs self;
        lib = self.lib;
        isDarwin = true;
        isNixOS = false;
      };

      modules = [
        ../hosts/${hostname}/default.nix
        { nixpkgs.overlays = [ inputs.brew-nix.overlays.default ] ++ standardOverlays ++ extraOverlays; }
      ]
      ++ lib.optionals withHomeManager [
        home-manager.darwinModules.home-manager
        inputs.determinate.darwinModules.default
        {
          home-manager.useGlobalPkgs = true;
          home-manager.useUserPackages = true;
          home-manager.extraSpecialArgs = {
            inherit inputs self;
            myLib = self.lib;
          };
          home-manager.backupFileExtension = "backup";
          home-manager.users.${username} = {
            imports = [ ../hosts/${hostname}/home.nix ];
            home.username = lib.mkForce username;
            home.homeDirectory = lib.mkForce "/Users/${username}";
          };
        }
      ]
      ++ extraModules;
    };

  # Unified System Helper
  mkSystem =
    { hostname, system, ... }@args:
    if args.isDarwin or (isDarwin system) then
      self.lib.mkDarwinHost args
    else
      self.lib.mkNixosHost args;

  # Standalone Home Manager configurations
  mkHomeConfig =
    {
      hostname,
      system,
      username ? "gaurav",
      homeDirectory ? null,
      extraHomeModules ? [ ],
      ... # Ignore extra args from mkConfigurations like extraModules, isNixos, isDarwin
    }:
    let
      pkgs = mkPkgs system;
      finalHomeDir =
        if homeDirectory != null then
          homeDirectory
        else if isDarwin system then
          "/Users/${username}"
        else
          "/home/${username}";
    in
    home-manager.lib.homeManagerConfiguration {
      inherit pkgs;
      extraSpecialArgs = {
        inherit inputs self;
        myLib = self.lib;
      };
      modules = [
        ../hosts/${hostname}/home.nix
        {
          home.username = lib.mkForce username;
          home.homeDirectory = lib.mkForce finalHomeDir;
        }
      ]
      ++ extraHomeModules;
    };

  # Generate all configurations from a single attrset of hosts
  mkConfigurations =
    hosts:
    let
      # Filter by system type (using flags if present, otherwise auto-detect)
      nixosHosts = lib.filterAttrs (
        name: host: host.isNixos or (!(host.isDarwin or (isDarwin host.system)))
      ) hosts;
      darwinHosts = lib.filterAttrs (name: host: host.isDarwin or (isDarwin host.system)) hosts;

      # Create system configurations
      nixosConfigurations = lib.mapAttrs (
        name: host: mkNixosHost ({ hostname = name; } // host)
      ) nixosHosts;
      darwinConfigurations = lib.mapAttrs (
        name: host: mkDarwinHost ({ hostname = name; } // host)
      ) darwinHosts;

      # Filter hosts that want standalone Home Manager
      homeHosts = lib.filterAttrs (
        name: host: host.withStandaloneHome or host.withHomeManager or true
      ) hosts;

      # Create Home Manager configurations mapped by "username@hostname"
      homeConfigurations = lib.mapAttrs' (
        name: host:
        lib.nameValuePair "${host.username or "gaurav"}@${name}" (
          mkHomeConfig ({ hostname = name; } // host)
        )
      ) homeHosts;
    in
    {
      inherit nixosConfigurations darwinConfigurations homeConfigurations;
    };
}
