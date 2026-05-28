{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
let
  homeDir = config.home.homeDirectory;
  cargoHome = "${homeDir}/.cargo";
  sccacheDir = "${homeDir}/.cache/sccache";
  cargo-plugins = with pkgs; [
    cargo-sweep # Cleanup build artifacts
    cargo-edit # cargo add/rm/upgrade
    cargo-machete # find unused deps
    cargo-expand # macro expansion
    cargo-deny # dependency linter
    bacon # background checker
    cargo-generate # cargo, make me a project
    sccache # shared compilation cache
  ];
in
myLib.mkHomeModule {
  globalConfig = config;
  name = "lang.rust";
  description = "Rust development environment";
  config = {
    home = {
      packages =
        with pkgs;
        [
          # Core Toolchain
          rustc
          cargo
          clippy
          rustfmt
          rust-analyzer

          # System dependencies often needed for building crates
          pkg-config
          wasm-pack
          openssl
          cargo-shear
          cargo-vet

          rustlings
        ]
        ++ lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ]
        ++ cargo-plugins;

      sessionVariables = {
        CARGO_HOME = cargoHome;
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
        SCCACHE_DIR = sccacheDir;
        CARGO_INCREMENTAL = "0";
      };

      sessionPath = [ "${cargoHome}/bin" ];

      file.".cargo/config.toml".source = (pkgs.formats.toml { }).generate "cargo-config" {
        install.root = cargoHome;
        net.git-fetch-with-cli = true;
        target = lib.optionalAttrs pkgs.stdenv.isDarwin {
          "aarch64-apple-darwin".linker = "clang";
        };
      };

      activation.initRust = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        $DRY_RUN_CMD mkdir -p ${cargoHome}/bin
        $DRY_RUN_CMD mkdir -p ${sccacheDir}
      '';
    };
  };
}
