{ pkgs, rustPlatform, ... }:

let
  inherit (pkgs) lib;

  # Only the workspace manifests, the lockfile and the member sources are needed
  # to build. A fileset keeps `target/`, editor droppings and stray scratch files
  # out of the store, which `cleanSourceWith` filtering by basename does not
  # reliably guarantee.
  #
  # This is a Cargo workspace: the root manifest and `Cargo.lock` live at the
  # flake root, so both must be included alongside the member's own files.
  # The Rust package itself lives in crates/coffee, keeping Rust sources
  # separate from the Nix packaging in pkgs/.
  src = lib.fileset.toSource {
    root = ./..;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../crates/coffee/Cargo.toml
      ../crates/coffee/src
    ];
  };

in
rustPlatform.buildRustPackage {
  pname = "coffee";
  version = "0.2.0";

  inherit src;

  cargoLock = {
    lockFile = ./../Cargo.lock;
  };

  nativeBuildInputs = [ pkgs.installShellFiles ];

  # `background_policy_matches_the_launchd_agent` reads this module through
  # CARGO_MANIFEST_DIR to cross-check the thresholds the launchd agent actually
  # runs. `src` above does not contain it, so without this the test would skip
  # and a drift between the Rust policy and the agent would go unnoticed — which
  # is exactly how `doctor` came to report thresholds the agent never used.
  #
  # One extra file in the source closure; cheap insurance for a safety property.
  postPatch = ''
    mkdir -p modules/darwin
    cp ${../modules/darwin/zombie-reaper.nix} modules/darwin/zombie-reaper.nix
  '';

  # The unit tests assert the cleanup safety invariants (notably that a dry run
  # can never schedule a destructive command, and that the reaper's thresholds
  # match the launchd agent's), so they must run on every build.
  doCheck = true;

  postInstall = ''
    # Install 'co' shortcut symlink.
    ln -s $out/bin/coffee $out/bin/co

    # Completions are generated for both names. Writing to a real file keeps this
    # working regardless of what shell `postInstall` happens to run under.
    for bin in coffee co; do
      for shell in bash zsh fish; do
        $out/bin/coffee generate-completion "$shell" --bin-name "$bin" \
          > "$TMPDIR/coffee-completion-$bin-$shell"
        installShellCompletion \
          --cmd "$bin" \
          --"$shell" "$TMPDIR/coffee-completion-$bin-$shell"
      done
    done
  '';

  meta = {
    description = "Developer cleanup & maintenance utility (callable via coffee or co)";
    homepage = "https://github.com/gaurav/dotfiles";
    license = lib.licenses.mit;
    mainProgram = "coffee";
    platforms = lib.platforms.unix;
  };
}
