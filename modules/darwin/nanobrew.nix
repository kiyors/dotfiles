{
  pkgs,
  inputs,
  config,
  lib,
  ...
}:
{
  # 1. Import the base nix-nanobrew logic from flake inputs
  imports = [ inputs.nix-nanobrew.darwinModules.nix-nanobrew ];

  inherit
    (lib.mkModule {
      globalConfig = config;
      name = "darwin.nanobrew";
      description = "macOS nanobrew package manager setup";
      config = {
        # Extra system setup
        environment.systemPackages = with pkgs; [ pkg-config ];
        environment.systemPath = [ "/opt/nanobrew/prefix/bin" ];

        # 2. Configure nix-nanobrew options directly as shown in homebrew.nix
        nix-nanobrew = {
          enable = true;
          user = "gaurav";
          autoMigrate = true;
          onActivation = {
            # autoUpdate = false;
            cleanup = "uninstall"; # Genuine declarative cleanup: remove from config = remove from system
            upgrade = true;
          };
          package = inputs.nix-nanobrew.packages.${pkgs.stdenv.hostPlatform.system}.default;

          # Declarative package lists hardcoded in the module
          casks = [
            "iina"
            "blip"
            # "bruno"
            # "motrix"
            "raycast"
            "spotify"
            "obsidian"
            "antigravity"
            # "google-drive"
            "google-chrome"
            # "brave-browser"
            # "helium-browser"
            "keyboardcleantool"
            # "netbirdio/tap/netbird-ui"
            "mhaeuser/mhaeuser/battery-toolkit"
          ];

          brews = [
            "mas"
            "mole"
            "sheets"
            # "colima"
            # "docker"
            "libiconv"
            "tesseract"
            "gemini-cli"
            "tree-sitter"
            # "docker-buildx"
            "tesseract-lang"
            # "docker-compose"
            "tree-sitter-cli"
            "netbirdio/tap/netbird"
            "Arthur-Ficial/tap/apfel"
          ];
        };

        # 3. Handle Xcode prerequisites (standard pattern)
        system.activationScripts.preActivation.text = ''
          echo "━━━ Checking Prerequisites ━━━"
          if ! xcode-select -p &> /dev/null; then
            echo "Installing Xcode Command Line Tools..."
            touch /tmp/.com.apple.dt.CommandLineTools.installondemand.in-progress
            PROD=$(softwareupdate -l | grep "\*.*Command Line" | tail -n 1 | sed 's/^[^C]* //')
            if [ -n "$PROD" ]; then
              softwareupdate -i "$PROD" --verbose
              echo "✓ Xcode Command Line Tools: Installed"
            fi
            rm -f /tmp/.com.apple.dt.CommandLineTools.installondemand.in-progress
          fi

          if ! /usr/bin/pgrep -q oahd; then
            echo "Installing Rosetta 2..."
            sudo softwareupdate --install-rosetta --agree-to-license 2>/dev/null || true
          fi
        '';
      };
    })
    options
    config
    ;
}
