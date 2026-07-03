{
  myLib,
  lib,
  config,
  inputs,
  pkgs,
  ...
}:
myLib.mkHomeModule {
  globalConfig = config;
  name = "hermes-agent";
  description = "Hermes Agent configuration";
  config = {
    home.packages = [
      inputs.hermes-agent.packages."${pkgs.stdenv.hostPlatform.system}".default
    ];
  };
}
