{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
let
  cargoHome = "${config.home.homeDirectory}/.cargo";
  cargo-plugins = with pkgs; [
    cargo-sweep # Cleanup build artifacts
    cargo-edit # cargo add/rm/upgrade
    cargo-machete # find unused deps
    cargo-expand # macro expansion
    cargo-deny # dependency linter
    bacon # background checker
    cargo-generate # cargo, make me a project
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
          openssl

          rustlings
        ]
        ++ lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ]
        ++ cargo-plugins;

      sessionVariables = {
        CARGO_HOME = cargoHome;
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
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
      '';
    };
  };
}
