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
  kacheDir = "${homeDir}/.cache/kache";
  cargo-plugins = with pkgs; [
    cargo-sweep # Cleanup build artifacts
    cargo-edit # cargo add/rm/upgrade
    cargo-machete # find unused deps
    cargo-expand # macro expansion
    cargo-deny # dependency linter
    bacon # background checker
    cargo-generate # cargo, make me a project
    kache # shared compilation cache
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
        ++ lib.optionals pkgs.stdenv.hostPlatform.isDarwin [ pkgs.libiconv ]
        ++ cargo-plugins;

      sessionVariables = {
        CARGO_HOME = cargoHome;
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        RUSTC_WRAPPER = "${pkgs.kache}/bin/kache";
        KACHE_DIR = kacheDir;
        KACHE_MAX_SIZE = "20GiB";
        KACHE_GC_MAX_AGE_HOURS = "168";
        KACHE_INPUT_PREDICTIONS = "true";
        KACHE_EXPLAIN_MISS = "true";
        CARGO_INCREMENTAL = "0";
      };

      sessionPath = [ "${cargoHome}/bin" ];

      file.".cargo/config.toml".source = (pkgs.formats.toml { }).generate "cargo-config" {
        install.root = cargoHome;
        net.git-fetch-with-cli = true;
        target = lib.optionalAttrs pkgs.stdenv.hostPlatform.isDarwin {
          "aarch64-apple-darwin".linker = "clang";
        };
      };

      file.".config/kache/config.toml".source = (pkgs.formats.toml { }).generate "kache-config" {
        cache = {
          cache_executables = true;
          local_max_size = "20GiB";
          gc_max_age_hours = 168;
          input_predictions = true;
          explain_miss = true;
          adaptive_incremental = true;
          scheduler = true;
          compression_level = 3;
          auto_gc = true;
          build_script_cache = true;
          clean_incremental = true;
        };
      };

      activation.initRust = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        $DRY_RUN_CMD mkdir -p ${cargoHome}/bin
        $DRY_RUN_CMD mkdir -p ${kacheDir}
      '';
    };
  };
}
