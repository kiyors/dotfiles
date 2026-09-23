# Contributing to Dotfiles

This document explains the architecture, conventions, and workflows for this Nix-based dotfiles repository.

## Architecture Overview

```
dotfiles/
├── flake.nix              # Flake entry point: inputs + host definitions
├── lib/                   # Custom Nix library (helpers, module system)
│   ├── moduleHelper.nix   # mkModule / mkHomeModule system
│   ├── configHelper.nix   # mkNixosHost / mkDarwinHost / mkConfigurations
│   └── fonts.nix          # Shared font packages
├── modules/               # System-level NixOS/darwin modules
│   ├── common/            # Shared across all platforms
│   ├── nixos/             # NixOS-specific modules
│   └── darwin/            # macOS-specific modules
├── home/                  # Home Manager modules (user-level config)
│   ├── cli/               # CLI tools (bat, btop, eza, tmux, etc.)
│   ├── editor/            # Editors (neovim, zed, vscode)
│   ├── git/               # Git, gh, lazygit, jujutsu
│   ├── lang/              # Language environments (node, rust, go, etc.)
│   └── shell/             # Shell config (zsh, nushell, starship, etc.)
├── hosts/                 # Per-host configurations
│   ├── atlas/             # NixOS desktop
│   ├── titan/             # NixOS desktop
│   ├── hades/             # NixOS server
│   └── coffee/            # macOS laptop
├── config/                # Raw config files (symlinked into ~/.config)
├── pkgs/                  # Custom package derivations
├── overlays/              # Nixpkgs overlays
├── secrets/               # SOPS-encrypted secrets
└── docs/                  # Documentation
```

## The Module System

This repo uses a custom module system built on top of NixOS/Home Manager modules. Every configuration is wrapped in a module that auto-creates an `.enable` option, making features trivially toggleable per host.

### How It Works

The core logic lives in `lib/moduleHelper.nix`:

```nix
# System modules: options live under `modules.<name>.enable`
mkModule = args: mkModuleAt (args // { prefix = "modules"; });

# Home Manager modules: options live under `<name>.enable`
mkHomeModule = args: mkModuleAt (args // { prefix = ""; });
```

When you call `mkHomeModule`, it:
1. Creates an option at `<name>.enable` (e.g., `cli.bat.enable`)
2. Wraps your config in `mkIf` so it only applies when enabled
3. Returns both `options` and `config` attrsets

### Module Anatomy

Every module follows this pattern:

```nix
{
  myLib,           # Your custom library (provides mkHomeModule)
  lib,             # Nixpkgs lib
  config,          # Current config attrset (for reading other options)
  pkgs,            # Package set
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;          # Required: passed through for mkIf evaluation
  name = "category.feature";      # Dot-separated path for the enable option
  description = "What this does"; # Human-readable description
  enableDefault = false;          # Optional: auto-enable when parent is enabled
  config = {
    # Your actual configuration goes here
    home.packages = [ pkgs.somePackage ];
  };
  imports = [ ];                  # Optional: sub-modules to import
}
```

### Enable Option Hierarchy

The `name` parameter uses dot-separated paths to create nested options:

| Module `name` | Generated option path | Example usage in `home.nix` |
|----------------|----------------------|----------------------------|
| `"cli.bat"` | `cli.bat.enable` | `cli.bat.enable = true;` |
| `"lang.node"` | `lang.node.enable` | `lang.node.enable = true;` |
| `"terminal.ghostty"` | `terminal.ghostty.enable` | `terminal.ghostty.enable = true;` |
| `"shell.zsh"` | `shell.zsh.enable` | `shell.zsh.enable = true;` |

### Cascading Enables with `enableDefault`

Use `enableDefault` to auto-enable child modules when a parent is enabled:

```nix
# In home/cli/bat.nix
myLib.mkHomeModule {
  globalConfig = config;
  name = "cli.bat";
  description = "Bat cat clone";
  enableDefault = config.cli.enable or false;  # Auto-enable when cli.enable = true
  config = { ... };
}
```

Now when you set `cli.enable = true;` in a host's `home.nix`, all children with `enableDefault = config.cli.enable or false` are automatically enabled.

### System vs Home Manager Modules

| Feature | System Modules (`mkModule`) | Home Manager Modules (`mkHomeModule`) |
|---------|----------------------------|--------------------------------------|
| Option prefix | `modules.<name>.enable` | `<name>.enable` |
| Used for | System services, hardware, boot | User packages, dotfiles, shell |
| Location | `modules/nixos/`, `modules/darwin/` | `home/` |
| Config scope | System-level NixOS/darwin options | Home Manager options |

## Adding a New Home Manager Module

1. Create the file in the appropriate directory:

```
home/
├── cli/
│   └── my-tool.nix      # For CLI tools
├── lang/
│   └── my-lang.nix      # For language environments
├── editor/
│   └── my-editor.nix    # For editors
└── my-feature.nix       # For top-level features
```

2. Write the module:

```nix
{
  myLib,
  lib,
  config,
  pkgs,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "my-category.my-tool";
  description = "My awesome tool";
  config = {
    home.packages = [ pkgs.my-tool ];

    home.file.".config/my-tool/config.toml".text = ''
      # configuration here
    '';
  };
}
```

3. Import it in the parent `default.nix` or directly in a host's `home.nix`:

```nix
# Option A: Import in parent default.nix (auto-included for all hosts)
imports = [ ./my-tool.nix ];

# Option B: Import only in specific hosts
# In hosts/myhost/home.nix:
imports = [ ../../home/my-feature.nix ];
my-category.my-tool.enable = true;
```

## Adding a New Host

1. Create the host directory:

```bash
mkdir -p hosts/myhost
```

2. Create `hosts/myhost/default.nix` (system config):

```nix
{ config, pkgs, lib, ... }:
{
  # System-level configuration
  networking.hostName = "myhost";
  
  # Import modules as needed
  imports = [
    ../../modules
  ];

  # Enable specific system modules
  modules.docker.enable = true;
}
```

3. Create `hosts/myhost/home.nix` (user config):

```nix
{ config, pkgs, lib, ... }:
{
  imports = [
    ../../home        # Import all home modules
    ./secrets         # Optional: host-specific secrets
  ];

  # Enable features for this host
  lang = {
    rust.enable = true;
    node.enable = true;
  };
  
  shell.zsh.enable = true;
  editors.neovim.enable = true;
  terminal.ghostty.enable = true;

  # Host-specific settings
  home.username = "myuser";
  home.homeDirectory = "/home/myuser";
  home.stateVersion = "26.11";
}
```

4. Register the host in `flake.nix`:

```nix
hosts = {
  # ... existing hosts ...
  myhost = {
    isNixos = true;        # or isDarwin = true for macOS
    hostname = "myhost";
    username = "myuser";
    system = lib.systems.x86_64-linux;
    # extraModules = [ ];  # Optional: additional system modules
  };
};
```

The `lib.mkConfigurations` function automatically generates `nixosConfigurations`, `darwinConfigurations`, and `homeConfigurations` from this attrset.

## Adding a System Module (NixOS/darwin)

1. Create the file in `modules/nixos/` or `modules/darwin/`:

```nix
# modules/nixos/my-service.nix
{
  config,
  lib,
  pkgs,
  ...
}:
lib.mkModule {
  globalConfig = config;
  name = "nixos.my-service";
  description = "My custom service";
  config = {
    services.my-service = {
      enable = true;
      package = pkgs.my-service;
    };
  };
}
```

2. Import it in `modules/nixos/default.nix`:

```nix
imports = [
  # ... existing imports ...
  ./my-service.nix
];
```

3. Enable it in a host's system config:

```nix
modules.my-service.enable = true;
```

## Configuration Hierarchy

The evaluation order is:

1. **flake.nix** — Defines hosts and their basic properties
2. **lib/configHelper.nix** — Generates system configurations
3. **hosts/<name>/default.nix** — System-level config (NixOS/darwin)
4. **hosts/<name>/home.nix** — User-level config (Home Manager)
5. **home/default.nix** — Shared home modules (imported by all hosts)
6. **home/** subdirectories — Individual feature modules

## Code Conventions

### File Naming

- Use `kebab-case` for file names: `my-feature.nix`
- Use dot-separated paths for module names: `category.feature`
- Group related modules in directories: `home/cli/`, `home/lang/`

### Module Parameters

Always accept these parameters in this order:

```nix
{
  myLib,
  lib,
  config,
  pkgs,
  ...
}:
```

### Configuration Files

Raw config files live in `config/` and are symlinked into `~/.config/`:

```nix
home.file.".config/tool/config".source =
  config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/tool/config";
```

This allows live editing without rebuilding.

### Secrets

Use SOPS for sensitive data:

```nix
sops.secrets.my-secret = {
  sopsFile = ./secrets.yaml;
  path = "${config.home.homeDirectory}/.config/my-tool/secret.key";
};
```

See `docs/secrets.md` for details.

## Useful Commands

```bash
# Build and switch (NixOS)
sudo nixos-rebuild switch --flake .

# Build and switch (macOS)
darwin-rebuild switch --flake .

# Build home manager only
home-manager switch --flake .

# Check for errors
nix flake check

# Update all inputs
nix flake update

# Update a specific input
nix flake update nixpkgs

# Format code
nix fmt

# Show dependency tree
nix flake metadata
```

## Tips

- **Read existing modules** before writing new ones — follow the patterns you see
- **Use `config.lib.file.mkOutOfStoreSymlink`** for configs you want to edit live
- **Test changes** with `nix build` before switching
- **Keep modules focused** — one module per feature/tool
- **Use `enableDefault`** for logical groupings (e.g., all CLI tools under `cli`)
- **Check `lib/moduleHelper.nix`** if you're confused about how options are generated
