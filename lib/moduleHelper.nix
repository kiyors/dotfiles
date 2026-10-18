{ lib }:
let
  # Internal builder shared by mkModule / mkHomeModule.
  # `prefix` is "" for HM modules and "modules" for system modules.
  mkModuleAt =
    {
      prefix,
      name,
      description ? "Enable ${name} module",
      config,
      globalConfig,
      imports ? [ ],
      # Extra options declared alongside the generated `enable` flag. Keys
      # are absolute paths from the config root, so they merge onto the
      # `enable` attrset rather than replacing it. `recursiveUpdate` is
      # required: `//` is a shallow merge and would drop `enable`.
      options ? { },
      enableDefault ? false,
    }:
    let
      fullPath = if prefix == "" then name else "${prefix}.${name}";
      pathParts = lib.splitString "." fullPath;
      enablePath = pathParts ++ [ "enable" ];
      enableOption =
        if enableDefault then
          lib.mkOption {
            type = lib.types.bool;
            default = true;
            inherit description;
          }
        else
          lib.mkEnableOption description;
    in
    {
      options = lib.recursiveUpdate (lib.setAttrByPath pathParts { enable = enableOption; }) options;
      config = lib.mkIf (lib.attrByPath enablePath enableDefault globalConfig) config;
    }
    // lib.optionalAttrs (imports != [ ]) { inherit imports; };
in
{
  # System modules: options live under `modules.<name>.enable`
  mkModule = args: mkModuleAt (args // { prefix = "modules"; });

  # Home Manager modules: options live under `<name>.enable`
  mkHomeModule = args: mkModuleAt (args // { prefix = ""; });

  # Helper for boolean options
  mkBoolOpt =
    default: description:
    lib.mkOption {
      inherit default description;
      type = lib.types.bool;
    };

  # Helper for string options
  mkStrOpt =
    default: description:
    lib.mkOption {
      inherit default description;
      type = lib.types.str;
    };

  # Helper for package lists
  mkPkgOpt =
    default: description:
    lib.mkOption {
      inherit default description;
      type = lib.types.listOf lib.types.package;
    };
}
