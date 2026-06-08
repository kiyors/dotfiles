# hosts/grax/secrets/default.nix
{
  config,
  pkgs,
  inputs,
  ...
}:
{
  sops.secrets =
    let
      sshDir = "${config.home.homeDirectory}/.ssh";
    in
    {
      sshconfig = {
        path = "${sshDir}/config";
      };
      github-key = {
        path = "${sshDir}/github";
        mode = "0600";
      };
      signing-key = {
        path = "${sshDir}/key";
        mode = "0600";
      };
      signing-pub-key = {
        path = "${sshDir}/key.pub";
      };
      allowed-signers = {
        path = "${sshDir}/allowed_signers";
      };
      nix_access_tokens = { };
    };

  sops.templates."vicinae-secrets.json" = {
    path = "${config.home.homeDirectory}/.config/vicinae/secrets.json";
    content = builtins.toJSON {
      providers = {
        "@knoopx/nix" = {
          preferences = {
            githubToken = config.sops.placeholder.nix_access_tokens;
          };
        };
      };
    };
  };

  # Ensure sops-nix is started on NixOS (for things like mbsync or SSH config)
  # systemd.user.services.mbsync.Unit.After = [ "sops-nix.service" ];
}
