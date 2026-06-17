{
  description = "percy dev shell (Joy fork)";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    # Pinned Rust toolchain + the wasm target the percy-dom browser tests build for.
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Pinned to match the Joy repo's toolchain (rustc 1.94.0) so the two clones share
        # one toolchain in the nix store. percy itself is edition 2018 and not version-
        # sensitive; the wasm32 target is for the `wasm-pack test` browser suite.
        rustToolchain = pkgs.rust-bin.stable."1.94.0".default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };

      in
      {
        formatter = pkgs.nixpkgs-fmt;

        devShells = {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              rustToolchain # rustc + cargo + rustfmt, pinned, with the wasm32 target
              rust-analyzer
              lld

              # `cargo bench -p percy-dom --bench diff` is native Criterion — needs nothing
              # beyond the toolchain. The rest is for the wasm-pack browser test suite
              # (`./test.sh`): wasm-pack drives wasm-bindgen + a headless browser, and
              # `.cargo/config` sets the wasm32 test runner to wasm-bindgen-test-runner.
              wasm-pack
              wasm-bindgen-cli

              # Headless browsers + their webdrivers for `wasm-pack test`.
              # test.sh defaults to --firefox; most per-test header comments use --chrome.
              # Both are provided so either flag works out of the box.
              chromium
              chromedriver
              firefox
              geckodriver
            ];

            shellHook = ''
              # wasm-pack otherwise tries to download a chromedriver/geckodriver that won't
              # run under nix; point it at the ones from this shell instead.
              export CHROMEDRIVER="${pkgs.chromedriver}/bin/chromedriver"
              export GECKODRIVER="${pkgs.geckodriver}/bin/geckodriver"
            '';
          };
        };
      });
}
