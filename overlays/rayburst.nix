final: prev:
let
  rayburst = (prev.rayburst or prev.motrix-next).overrideAttrs (oldAttrs: rec {
    pname = "rayburst";
    version = "4.0.1";

    src = prev.fetchFromGitHub {
      owner = "AnInsomniacy";
      repo = "rayburst";
      tag = "v${version}";
      hash = "sha256-ViMwZA3HsouYH7bV5BWPuNrOxT1ySfOruBRIyPrasgw=";
    };

    cargoDeps = prev.rustPlatform.fetchCargoVendor {
      inherit src;
      sourceRoot = "${pname}-${version}/${oldAttrs.cargoRoot}";
      hash = "sha256-5OkOO9jnI9/CSGN7Bsmbe7S5MGfAPWBGYz1PPYzHWm0=";
    };

    pnpmDeps = prev.fetchPnpmDeps {
      inherit pname version src;
      pnpm = prev.pnpm_11;
      hash = "sha256-zFV87FMMtJQXcus3p0OQDUQ1yVgJ0RgN2EangBw6Y1s=";
      fetcherVersion = 4;
    };

    patches = [ ];

    postPatch =
      (oldAttrs.postPatch or "")
      + ''
        substituteInPlace scripts/build-native-messaging-launcher.mjs \
          --replace-fail "join(tauriDir, 'target'" "join(root, 'target'"

        if [ -f src-tauri/tauri.macos.conf.json ]; then
          jq '
            del(.build.beforeBundleCommand) |
            .bundle.icon = (.bundle.icon | map(select(. != "generated-icons/Assets.car")))
          ' src-tauri/tauri.macos.conf.json | sponge src-tauri/tauri.macos.conf.json
        fi

        echo "exit 0" > scripts/compile-macos-icon.sh

        if [ -f src-tauri/tauri.release.json ]; then
          jq '.bundle.createUpdaterArtifacts = false' src-tauri/tauri.release.json | sponge src-tauri/tauri.release.json
        fi
      '';

    # Explicitly support darwin build dependencies if needed
    buildInputs =
      (oldAttrs.buildInputs or [ ])
      ++ prev.lib.optionals prev.stdenv.hostPlatform.isDarwin [
        prev.apple-sdk
      ];

    postInstall =
      (oldAttrs.postInstall or "")
      + prev.lib.optionalString prev.stdenv.hostPlatform.isDarwin ''
        mkdir -p $out/bin
        ln -s $out/Applications/Rayburst.app/Contents/MacOS/rayburst $out/bin/rayburst
      '';

    meta = (oldAttrs.meta or { }) // {
      homepage = "https://github.com/AnInsomniacy/rayburst";
      changelog = "https://github.com/AnInsomniacy/rayburst/releases/tag/v${version}";
      mainProgram = "rayburst";
    };
  });
in
{
  inherit rayburst;
  # Backward compatibility alias
  motrix-next = rayburst;
}
