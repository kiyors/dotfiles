{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
let
  composerHome = "${config.xdg.configHome}/composer";
in
myLib.mkHomeModule {
  globalConfig = config;
  name = "lang.php";
  description = "PHP, Composer, and Laravel development environment";
  config = {
    home = {
      packages = with pkgs; [
        php
        phpPackages.composer
        phpactor # PHP refactoring + LSP fallback
        php-cs-fixer # alternative formatter to pint
      ];

      sessionVariables = {
        COMPOSER_HOME = composerHome;
      };

      sessionPath = [ "${composerHome}/vendor/bin" ];

      activation.initPhp = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        $DRY_RUN_CMD mkdir -p ${composerHome}/vendor/bin
        if [ ! -e ${composerHome}/vendor/bin/laravel ]; then
          $DRY_RUN_CMD ${pkgs.phpPackages.composer}/bin/composer global require laravel/installer --no-interaction || true
        fi
      '';
    };
  };
}
