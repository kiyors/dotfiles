{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
let
  javaVersion = 25;
  jdk = pkgs."jdk${toString javaVersion}";
  # kotlin-language-server 1.3.13 doesn't support Java 25 yet
  # Use Java 21 for the LSP while keeping Java 25 as default
  jdk21 = pkgs.jdk21;
in
myLib.mkHomeModule {
  globalConfig = config;
  name = "lang.kotlin";
  description = "Kotlin development environment";
  config = {
    home = {
      packages = with pkgs; [
        jdk
        jdk21
        gradle
        kotlin
        ncurses
        zlib
      ] ++ lib.optionals pkgs.stdenv.isLinux [
        gcc
        patchelf
      ];

      sessionVariables = {
        JAVA_HOME = "${jdk}";
      };
    };
  };
}
