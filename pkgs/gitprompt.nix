{ pkgs, rustPlatform, ... }:

let
  inherit (pkgs) lib;

  # Mirrors pkgs/coffee.nix: only the manifests, lockfile and this member's
  # sources are needed. Keeping `target/` out of the store matters even more
  # here — gitprompt is on every prompt draw, so a bloated binary is felt.
  #
  # The root manifest names every workspace member, and cargo refuses to load a
  # workspace whose members are missing from disk. So the *other* member's
  # manifest and sources are included too, even though nothing here uses them:
  # without `crates/coffee/Cargo.toml` the build fails with "failed to load
  # manifest for workspace member". Sources are a few kilobytes; correctness
  # of the build is worth more than the bytes.
  src = lib.fileset.toSource {
    root = ./..;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../crates/gitprompt/Cargo.toml
      ../crates/gitprompt/src
      ../crates/coffee/Cargo.toml
      ../crates/coffee/src
    ];
  };

in
rustPlatform.buildRustPackage {
  pname = "gitprompt";
  version = "0.1.0";

  inherit src;

  # Build only this crate's binary, for the same reason as pkgs/coffee.nix: a
  # bare `cargo build` would also produce coffee and install it here.
  buildAndTestSubdir = "crates/gitprompt";

  cargoLock = {
    lockFile = ./../Cargo.lock;
  };

  # The tests shell out to a real `git` and build real repositories, so the
  # sandbox needs git available at build time. Without it they skip silently
  # and the escape-sequence pinning in `render` is all that gets checked.
  nativeBuildInputs = [ pkgs.git ];

  # `render` pins the exact bytes of the prompt segment against what the shell
  # used to emit, and `dirty` verifies the indicator's meaning. Both are cheap
  # and both are the whole contract with config/zsh, so run them every build.
  doCheck = true;

  meta = {
    description = "Fast git branch and dirty indicator for shell prompts";
    homepage = "https://github.com/gaurav/dotfiles";
    license = lib.licenses.mit;
    mainProgram = "gitprompt";
    platforms = lib.platforms.unix;
  };
}
