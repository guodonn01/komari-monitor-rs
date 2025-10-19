{
  description = "Komari Monitor Agent in Rust";
  inputs = {
    utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };
  outputs = { nixpkgs, utils, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        toolchain = pkgs.rustPlatform;
      in rec {
        packages = let
          p = {
            pname = "komari-monitor-rs";
            version = "0.2.7";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildType = "minimal";
            # For other makeRustPlatform features see:
            # https://github.com/NixOS/nixpkgs/blob/master/doc/languages-frameworks/rust.section.md#cargo-features-cargo-features
          };
        in rec {
          default = ureq;
          ureq = toolchain.buildRustPackage
            (p // { buildFeatures = [ "ureq-support" ]; });
          nyquest-support = toolchain.buildRustPackage
            (p // { buildFeatures = [ "nyquest-support" ]; });
        };

        # Executed by `nix run`
        apps.default = utils.lib.mkApp { drv = packages.default; };

        # Used by `nix develop`
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            (with toolchain; [ cargo rustc rustLibSrc ])
            clippy
            rustfmt
            pkg-config
          ];

          # Specify the rust-src path (many editors rely on this)
          RUST_SRC_PATH = "${toolchain.rustLibSrc}";
        };
      });
}
