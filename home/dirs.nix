{
  myLib,
  config,
  lib,
  ...
}:
let
  cfg = config.home.customDirs;
in
{
  options.home.customDirs = lib.mkOption {
    type = with lib.types; listOf (either str (attrsOf str));
    default = [ ];
    description = ''
      A list of directories (relative to $HOME) to create.
      Items can be simple strings (path only) or attribute sets (path = linkTo).
      Idempotent: existing paths are left alone.
    '';
    example = [
      "personal"
      { "personal/media" = "Movies/media"; }
      "workspace"
    ];
  };

  config = lib.mkIf (cfg != [ ]) {
    home.activation.createCustomDirs = lib.hm.dag.entryAfter [ "writeBoundary" ] (
      let
        normalized = lib.flatten (map (item:
          if builtins.isString item then
            [{ path = item; linkTo = null; }]
          else
            lib.mapAttrsToList (path: linkTo: { inherit path linkTo; }) item
        ) cfg);

        mkCmds = d:
          let
            create = ''$DRY_RUN_CMD mkdir -p "$HOME/${d.path}"'';
            link = lib.optionalString (d.linkTo != null) ''
              $DRY_RUN_CMD mkdir -p "$(dirname "$HOME/${d.linkTo}")"
              $DRY_RUN_CMD ln -sfn "$HOME/${d.path}" "$HOME/${d.linkTo}"
            '';
          in
          "${create}\n${link}";
      in
      lib.concatMapStringsSep "\n" mkCmds normalized
    );
  };
}
