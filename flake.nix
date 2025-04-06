{

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { flake-utils, nixpkgs, naersk, fenix, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ fenix.overlays.default ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config = { permittedInsecurePackages = [ ]; };
        };

        toolchain = with fenix.packages.${system};
          combine [
            minimal.cargo
            minimal.rustc
            latest.clippy
            latest.rust-src
            latest.rustfmt
            targets.wasm32-unknown-unknown.latest.rust-std
          ];
        min-pkgs = with pkgs; [
          pkg-config
          openssl
          gcc
          udev
          llvmPackages.libclang
          geckodriver
          wayland
          libxkbcommon
          wayland
          xorg.libxcb
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          pkg-config
          libGL
          libGLU
        ];
      in {
        defaultPackage = (naersk.lib.${system}.override {
          cargo = toolchain;
          rustc = toolchain;
        }).buildPackage {
          src = ./.;
          nativeBuildInputs = with pkgs; [ ] ++ min-pkgs;
        };

        devShell = (naersk.lib.${system}.override {
          cargo = toolchain;
          rustc = toolchain;
        }).buildPackage {
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang}/lib";
          src = ./.;
          mode = "fmt";
          dontPatchELF = true;
          nativeBuildInputs = with pkgs; [ ] ++ min-pkgs;
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${
              pkgs.lib.makeLibraryPath min-pkgs
            }"'';
        };
      });
}
