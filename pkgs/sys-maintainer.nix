{ pkgs, rustPlatform, ... }:

rustPlatform.buildRustPackage {
  pname = "sys-maintainer";
  version = "0.1.0";

  src = ./sys-maintainer;

  cargoLock = {
    lockFile = ./sys-maintainer/Cargo.lock;
  };

  meta = with pkgs.lib; {
    description = "Custom system maintenance utility for macOS to reap orphaned Node processes and run GC";
    homepage = "https://github.com/gaurav/dotfiles";
    license = licenses.mit;
    maintainers = [ ];
    mainProgram = "sys-maintainer";
  };
}
