{
  lib,
  stdenv,
  fetchurl,
  autoPatchelfHook,
}:

let
  pname = "kache";
  version = "0.28.0";

  sources = {
    "aarch64-darwin" = {
      url = "https://github.com/kunobi-ninja/kache/releases/download/v${version}/kache-aarch64-apple-darwin.tar.gz";
      sha256 = "1g0h0gd5s5xvy8g0hm245f96cmibni92ad3vjlixx148mcvszdyw";
    };
    "x86_64-darwin" = {
      url = "https://github.com/kunobi-ninja/kache/releases/download/v${version}/kache-x86_64-apple-darwin.tar.gz";
      sha256 = "0000000000000000000000000000000000000000000000000000000000000000";
    };
    "aarch64-linux" = {
      url = "https://github.com/kunobi-ninja/kache/releases/download/v${version}/kache-aarch64-unknown-linux-gnu.tar.gz";
      sha256 = "0000000000000000000000000000000000000000000000000000000000000000";
    };
    "x86_64-linux" = {
      url = "https://github.com/kunobi-ninja/kache/releases/download/v${version}/kache-x86_64-unknown-linux-gnu.tar.gz";
      sha256 = "0000000000000000000000000000000000000000000000000000000000000000";
    };
  };

  source = sources.${stdenv.hostPlatform.system} or (throw "Unsupported system: ${stdenv.hostPlatform.system}");
in
stdenv.mkDerivation {
  inherit pname version;

  src = fetchurl {
    inherit (source) url sha256;
  };

  nativeBuildInputs = lib.optionals stdenv.hostPlatform.isLinux [ autoPatchelfHook ];

  sourceRoot = ".";

  installPhase = ''
    mkdir -p $out/bin
    cp kache $out/bin/kache
    chmod +x $out/bin/kache
  '';

  meta = with lib; {
    description = "Zero-copy, content-addressed build cache for Rust, C/C++ and more";
    homepage = "https://github.com/kunobi-ninja/kache";
    license = licenses.asl20;
    platforms = platforms.darwin ++ platforms.linux;
    mainProgram = "kache";
  };
}
