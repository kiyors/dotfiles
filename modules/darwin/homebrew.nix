{
  pkgs,
  inputs,
  config,
  lib,
  ...
}:
{
  imports = [ inputs.nix-homebrew.darwinModules.nix-homebrew ];

  inherit
    (lib.mkModule {
      globalConfig = config;
      name = "darwin.homebrew";
      description = "macOS Homebrew package manager setup";
      config = {
        environment.systemPackages = with pkgs; [ pkg-config ];
        environment = {
          systemPath = [ "/opt/homebrew/bin" ];
          pathsToLink = [ "/Applications" ];
        };
        environment.variables = {
          HOMEBREW_PREFIX = "/opt/homebrew";
          HOMEBREW_CELLAR = "/opt/homebrew/Cellar";
          HOMEBREW_REPOSITORY = "/opt/homebrew";
          HOMEBREW_NO_INSTALL_FROM_API = "0";
          HOMEBREW_INSTALL_FROM_API = "1";
        };
        nix-homebrew = {
          enable = true;
          user = "gaurav";
          enableRosetta = true;
          autoMigrate = false;
          taps = {
            # "homebrew/homebrew-core" = inputs.homebrew-core;
            # "homebrew/homebrew-cask" = inputs.homebrew-cask;
            # "homebrew/homebrew-bundle" = inputs.homebrew-bundle;
          };
        };

        # 🚨 FIXED: Removed the broken `mas` bash commands from this block
        system.activationScripts.preActivation.text = ''
          echo "━━━ Checking Prerequisites ━━━"
          # Check and install Xcode Command Line Tools
          if ! xcode-select -p &> /dev/null; then
            echo "Installing Xcode Command Line Tools..."
            touch /tmp/.com.apple.dt.CommandLineTools.installondemand.in-progress
            PROD=$(softwareupdate -l | grep "\*.*Command Line" | tail -n 1 | sed 's/^[^C]* //')
            if [ -n "$PROD" ]; then
              softwareupdate -i "$PROD" --verbose
              echo "✓ Xcode Command Line Tools: Installed"
            else
              echo "⚠️  Could not auto-install. Run manually: xcode-select --install"
            fi
            rm -f /tmp/.com.apple.dt.CommandLineTools.installondemand.in-progress
          else
            echo "✓ Xcode Command Line Tools: $(xcode-select -p)"
          fi

          # Check and install Rosetta 2
          if /usr/bin/pgrep -q oahd; then
            echo "✓ Rosetta 2: Installed"
          else
            echo "Installing Rosetta 2..."
            if /usr/sbin/softwareupdate --install-rosetta --agree-to-license 2>/dev/null; then
              echo "✓ Rosetta 2: Installed successfully"
            else
              echo "⚠️  Rosetta 2 installation failed. Run manually:"
              echo "   sudo softwareupdate --install-rosetta --agree-to-license"
            fi
          fi
        '';

        homebrew = {
          enable = true;
          global.brewfile = true;
          onActivation = {
            autoUpdate = false;
            cleanup = "uninstall";
            upgrade = true;
          };
          taps = [
            # "homebrew/core"
            # "homebrew/cask"
            "mhaeuser/mhaeuser"
            "netbirdio/tap"
            "Arthur-Ficial/tap"
          ];
          casks = [
            "iina"
            "blip"
            "bruno"
            # "steam"
            "raycast"
            "spotify"
            "obsidian"
            # "motrix"
            "gcloud-cli"
            "antigravity"
            "google-drive"
            # "epic-games"
            "google-chrome"
            "brave-browser"
            # "helium-browser"
            "keyboardcleantool"
            # "netbirdio/tap/netbird-ui"
            "mhaeuser/mhaeuser/battery-toolkit"
          ];
          brews = [
            "mas"
            "mole"
            "sheets"
            "opencode"
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

          # This declarative block takes over App Store duties perfectly
          masApps = {
            "WhatsApp Messenger" = 310633997;
          };
        };
      };
    })
    options
    config
    ;
}
