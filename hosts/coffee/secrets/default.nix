# hosts/coffee/secrets/default.nix
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
      indiefluence-vps = {
        path = "${sshDir}/indiefluence_vps";
        mode = "0600";
      };
      indiefluence-vps-pub = {
        path = "${sshDir}/indiefluence_vps.pub";
      };
      hermes_api_key = { };
      hermes_api_base_url = { };
    };

  sops.templates."hermes-config.yaml" = {
    path = "${config.home.homeDirectory}/.hermes/config.yaml";
    content = ''
      api_server:
        base_url: "${config.sops.placeholder.hermes_api_base_url}"
        key: "${config.sops.placeholder.hermes_api_key}"
    '';
  };
}
