{
  lib,
  stdenv,
  fetchurl,
  undmg,
  makeWrapper,
  git,
  nodejs,
}:

let
  pname = "supercmd";
  version = "1.0.19";

  # Define architecture-specific sources
  sources = {
    "aarch64-darwin" = {
      url = "https://github.com/SuperCmdLabs/SuperCmd/releases/download/${version}/SuperCmd-${version}-arm64.dmg";
      sha256 = "12pf9a0l89gnd57b11hs96ycdcyc4ax9apbzwcwc022k2x056b4g";
    };
  };

  srcMeta =
    sources.${stdenv.hostPlatform.system}
      or (throw "Unsupported system: ${stdenv.hostPlatform.system}");

in
stdenv.mkDerivation {
  inherit pname version;

  src = fetchurl {
    inherit (srcMeta) url sha256;
  };

  nativeBuildInputs = [
    undmg
    makeWrapper
  ];

  # Add git and nodejs so we can reference them
  buildInputs = [
    git
    nodejs
  ];

  sourceRoot = ".";

  # Allow access to /usr/bin/hdiutil
  __noChroot = true;

  unpackPhase = ''
    runHook preUnpack
    # Mount the DMG
    hdiutil_output=$(/usr/bin/hdiutil attach -nobrowse $src)
    echo "hdiutil output: $hdiutil_output"
    mount_point=$(echo "$hdiutil_output" | grep -o '/Volumes/.*')
    echo "Mount point: $mount_point"
    # List the content of the mounted volume
    ls -l "$mount_point"
    # Copy the .app directory
    cp -R "$mount_point"/*.app .
    # Unmount the DMG
    /usr/bin/hdiutil detach "$mount_point"
    runHook postUnpack
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out/Applications
    cp -r *.app $out/Applications

    # 1. Identify the executable path inside the .app bundle
    APP_BIN="$out/Applications/SuperCmd.app/Contents/MacOS/SuperCmd"

    # 2. Wrap the executable to include git and nodejs (npm) in its PATH
    # This prevents the need for Homebrew when installing Raycast extensions
    wrapProgram "$APP_BIN" \
      --prefix PATH : "${
        lib.makeBinPath [
          git
          nodejs
        ]
      }"

    runHook postInstall
  '';

  meta = with lib; {
    description = "Open-source launcher for macOS with Raycast-compatible extensions, voice workflows, and AI-native actions";
    homepage = "https://supercmd.sh";
    license = licenses.mit;
    maintainers = [ ];
    platforms = platforms.darwin;
  };
}
