# hosts/coffee/default.nix
{
  config,
  pkgs,
  inputs,
  ...
}:
{
  imports = [
    ../../modules
  ];

  ids.gids.nixbld = 350;
  networking.hostName = "coffee";
  system.primaryUser = "gaurav";

  modules = {
    common.packages.enable = true;
    darwin = {
      homebrew.enable = true;
      nanobrew.enable = false;
      settings.enable = true;
      packages.enable = true;
      fonts.enable = true;
      determinateNix.enable = true;
    };
  };

  environment.systemPackages = with pkgs; [
    darwin.cctools
    llvmPackages.bintools
    # nushell
  ];
  environment.shells = [
    # pkgs.nushell
    pkgs.zsh
  ];

  # system.applications = with pkgs; [
  #   superCmd
  # ];

  users.users.gaurav = {
    uid = 501;
    description = "Gaurav";
    home = "/Users/gaurav";
    shell = pkgs.zsh;
  };
}
