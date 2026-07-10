{ pkgs, rustPlatform, ... }:

rustPlatform.buildRustPackage {
  pname = "sys-maintainer";
  version = "0.1.0";

  src = ./sys-maintainer;

  cargoLock = {
    lockFile = ./sys-maintainer/Cargo.lock;
  };
}
