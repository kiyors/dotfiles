{
  config,
  pkgs,
  inputs,
  ...
}:
{
  imports = [
    # Include the results of the hardware scan.
    ./hardware-configuration.nix
    ../../modules
    inputs.vicinae.nixosModules.default
  ];

  # Bootloader.
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  networking.hostName = "grax";

  # Enable networking
  networking.networkmanager.enable = true;

  # Enable the X11 windowing system.
  services.xserver.enable = true;

  # Enable the KDE Plasma Desktop Environment.
  services.displayManager.sddm.enable = true;
  services.desktopManager.plasma6.enable = true;

  # Configure keymap in X11
  services.xserver.xkb = {
    layout = "us";
    variant = "";
  };

  # Enable CUPS to print documents.
  services.printing.enable = true;

  # Define a user account.
  users.users."jogi" = {
    isNormalUser = true;
    description = "jogi";
    extraGroups = [
      "networkmanager"
      "wheel"
      "docker"
    ];
    shell = pkgs.zsh;
  };

  # Install firefox.
  programs.firefox.enable = true;
  programs.zsh.enable = true;

  # Add swap file for heavy builds
  swapDevices = [
    {
      device = "/var/lib/swapfile";
      size = 8 * 1024; # 8GB
    }
  ];

  # Allow unfree packages
  nixpkgs.config.allowUnfree = true;

  system.stateVersion = "26.05";

  modules = {
    common.packages.enable = true;
    nixos = {
      docker.enable = true;
      fonts.enable = true;
      nix.enable = true;
      locale.enable = true;
      audio.enable = true;
      bluetooth.enable = true;
      desktop.hyprland.enable = true;
      nvidia.enable = true;
    };
  };
}
