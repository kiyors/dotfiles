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
            "homebrew/homebrew-core" = inputs.homebrew-core;
            "homebrew/homebrew-cask" = inputs.homebrew-cask;
            "homebrew/homebrew-bundle" = inputs.homebrew-bundle;
          };
        };

        # Activation scripts run as root, allowing native path manipulation
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

          # Configure full Xcode path if it has finished downloading via Homebrew mas
          if [ -d "/Applications/Xcode.app" ]; then
            echo "Configuring Xcode developer directory..."
            xcode-select --switch /Applications/Xcode.app/Contents/Developer
            xcodebuild -license accept || true

            # ⬇️ ADD THIS LINE HERE ⬇️
            echo "Ensuring Metal Toolchain component is installed..."
            xcodebuild -downloadComponent MetalToolchain || true

            # Optional: Uncomment this to keep Xcode footprint minimal (~20GB savings)
            # echo "Purging unneeded simulation runtimes..."
            # rm -rf /Library/Developer/CoreSimulator/Profiles/Runtimes/*
          else
            echo "ℹ️  Xcode.app not found yet. It will be configured on the next switch after Homebrew finishes downloading it."
          fi
        '';

        homebrew = {
          enable = true;
          global.brewfile = true;
          onActivation = {
            autoUpdate = false;
            cleanup = "uninstall";
            extraFlags = [ "--force" ];
            upgrade = true;
          };
          taps = [
            "homebrew/core"
            "homebrew/cask"
            "mhaeuser/mhaeuser"
            "netbirdio/tap"
            "Arthur-Ficial/tap"
          ];
          casks = [
            "iina"
            "blip"
            "bruno"
            "ghostty"
            # "steam"
            "raycast"
            "codex-app"
            "t3-code"
            "spotify"
            "obsidian"
            # "motrix"
            # "gcloud-cli"
            "antigravity"
            "antigravity-ide"
            "antigravity-cli"
            "codex"
            "claude-code"
            # "google-drive"
            # "epic-games"
            "google-chrome"
            "brave-browser"
            # "helium-browser"
            "keyboardcleantool"
            # "netbirdio/tap/netbird-ui"
            # "mhaeuser/mhaeuser/battery-toolkit"
          ];
          brews = [
            "mas"
            "mole"
            "bun"
            # "sheets"
            "opencode"
            # "colima"
            # "docker"
            "libiconv"
            # "tesseract"
            # "gemini-cli"
            "tree-sitter"
            # "docker-buildx"
            # "tesseract-lang"
            # "docker-compose"
            "tree-sitter-cli"
            # "netbirdio/tap/netbird"
            # "Arthur-Ficial/tap/apfel"
          ];

          # This declarative block takes over App Store duties perfectly
          masApps = {
            "WhatsApp Messenger" = 310633997;
            "Xcode" = 497799835;
            "Apple Developer" = 640199958;
          };
        };
      };
    })
    options
    config
    ;
}
