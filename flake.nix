{
  description = "Embedded Rust on stm32-f3";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        from-rust-toolchain-file = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      with pkgs;
      {
        devShells.default = mkShell {
          buildInputs = [
            pkg-config
            openssl
            from-rust-toolchain-file
            flip-link # Adds zero-cost stack overflow protection to your embedded programs
            rust-analyzer
            probe-rs # (cargo-embed) can be used to flash binaries onto microcontrollers
            cargo-binutils # tools for examining rust binaries (`cargo-size`, `cargo-strip`, `cargo-objdump`)
            libusb
            gdb
            picocom
            gcc-arm-embedded
          ];

          RUST_SRC_PATH = "${from-rust-toolchain-file}/lib/rustlib/src/rust/library";

          shellHook = ''
            export PS1="[\e[1;32mNucleo-F303ZE-Rust-env\e[0m] $PS1"
          '';
        };
      }
    );
}

