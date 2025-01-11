{
  description = "My rust devenv nix flake";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }@inputs:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
        in {
          devShells.default = pkgs.mkShell rec {
            buildInputs = with pkgs; [
              (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              clang

              # command automation
              just

              # for the crate graph
              graphviz

              openssl
              openssl.dev

              lld
              udev alsa-lib vulkan-loader
              xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr # To use the x11 feature
              libxkbcommon wayland # To use the wayland feature
            ];
            nativeBuildInputs = with pkgs; [ pkg-config ] ;
            LIBCLANG_PATH = pkgs.lib.makeLibraryPath [pkgs.llvmPackages_latest.libclang.lib];
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          };
        }
      );
}
