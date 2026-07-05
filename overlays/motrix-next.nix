final: prev: {
  motrix-next = prev.motrix-next.overrideAttrs (oldAttrs: rec {
    version = "3.9.6";
    
    src = prev.fetchFromGitHub {
      owner = "AnInsomniacy";
      repo = "motrix-next";
      tag = "v${version}";
      hash = "sha256-ynLi+biCdjU7EOq556YuFonghWaxDV7UtHWiKImq7WE=";
    };

    # The cargoHash and pnpmDeps hash from nixos-unstable
    cargoHash = "sha256-c17GTD9Wcy9LYLfBcwECNS1Tek5hTWPmie2lXtrbtFc=";

    pnpmDeps = prev.fetchPnpmDeps {
      inherit (oldAttrs) pname;
      inherit version src;
      pnpm = prev.pnpm_10; # The original package uses pnpm_10
      hash = "sha256-WAuHoLAnFLP6i+rJSegt/hI6sb1SDhm7LWgsup70o9E=";
      fetcherVersion = 3;
    };

    # Explicitly support darwin build dependencies if needed
    buildInputs = oldAttrs.buildInputs or [] ++ prev.lib.optionals prev.stdenv.hostPlatform.isDarwin (with prev.darwin.apple_sdk.frameworks; [
      AppKit
      CoreServices
      Security
      WebKit
    ]);
  });
}
