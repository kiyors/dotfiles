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
  lombokAgent = "-javaagent:${pkgs.lombok}/share/java/lombok.jar";
in
myLib.mkHomeModule {
  globalConfig = config;
  name = "lang.java";
  description = "Java development environment";
  config = {
    home = {
      packages = with pkgs; [
        jdk
        maven
        gradle
        lombok
        ncurses
        zlib
      ] ++ lib.optionals pkgs.stdenv.isLinux [
        gcc
        patchelf
      ];

      sessionVariables = {
        JAVA_HOME = "${jdk}";
        JAVA_TOOL_OPTIONS = lombokAgent;
      };
    };
  };
}
