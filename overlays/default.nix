{ inputs, ... }:
{
  # Overlay custom derivations into nixpkgs so you can use pkgs.<name>
  additions = final: _prev: import ../pkgs { pkgs = final; };

  # https://wiki.nixos.org/wiki/Overlays
  modifications = final: prev: {
    # rayburst = (import ./rayburst.nix final prev).rayburst;
    # motrix-next = final.rayburst;

    stable = import inputs.nixpkgs-stable {
      system = final.stdenv.hostPlatform.system;
      config.allowUnfree = true;
    };

    # Fix direnv hanging on aarch64-darwin during checkPhase
    # https://github.com/NixOS/nixpkgs/issues/507531
    direnv = prev.direnv.overrideAttrs (oldAttrs: {
      doCheck = if final.stdenv.hostPlatform.isDarwin then false else (oldAttrs.doCheck or true);
    });

    # Fix afdko test failures on Darwin (SIGTRAP)
    afdko = prev.afdko.overrideAttrs (oldAttrs: {
      doCheck = if final.stdenv.hostPlatform.isDarwin then false else (oldAttrs.doCheck or true);
    });

    # Fix go-mockery checkPhase failure (incompatible export data format with newer Go)
    go-mockery = prev.go-mockery.overrideAttrs (_: {
      doCheck = false;
    });

    # Fix sesh build failure: mockery fails under Go 1.27, but sesh compiles without it
    sesh = prev.sesh.overrideAttrs (oldAttrs: {
      preBuild = "";
      doCheck = false;
      nativeBuildInputs = builtins.filter (x: (x.pname or "") != "go-mockery") (
        oldAttrs.nativeBuildInputs or [ ]
      );
    });

    pythonPackagesExtensions = prev.pythonPackagesExtensions ++ [
      (python-final: python-prev: {
        # Fix afdko test failures on Darwin (SIGTRAP) in pythonPackages
        afdko = python-prev.afdko.overridePythonAttrs (oldAttrs: {
          doCheck = if final.stdenv.hostPlatform.isDarwin then false else (oldAttrs.doCheck or true);
        });
      })
    ];
  };
}
