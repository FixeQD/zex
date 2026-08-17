{
  description = "zex - a native Rust wayland shell for Niri and Hyprland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [rust-overlay.overlays.default];
      };
      inherit ((pkgs.lib.importTOML ./Cargo.toml).workspace.package) version;

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = [
          "rust-src"
          "rust-analyzer"
          "clippy"
          "rustfmt"
        ];
      };

      nativeBuildInputs = with pkgs; [pkg-config wrapGAppsHook3];

      buildInputs = with pkgs; [
        gtk4
        gtk4-layer-shell
        wayland
        libxkbcommon
        fontconfig
        freetype
        gdk-pixbuf
        pipewire
        linux-pam
      ];
    in {
      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "zex";
        inherit version;

        src = self;
        cargoLock = {
          lockFile = ./Cargo.lock;
          allowBuiltinFetchGit = true;
        };

        inherit nativeBuildInputs buildInputs;

        meta = with pkgs.lib; {
          mainProgram = "zex";
          description = "Native Rust wayland shell for Niri and Hyprland";
          license = licenses.gpl3Plus;
          platforms = platforms.linux;
        };

        postFixup = ''
          for f in $out/bin/*; do
            patchelf --add-rpath ${pkgs.wayland}/lib "$f"
          done
        '';
      };

      devShells.default = pkgs.mkShell {
        inherit buildInputs;

        nativeBuildInputs = with pkgs; [
          pkg-config
          libclang.lib
        ];

        packages = [
          rustToolchain
          pkgs.matugen
          pkgs.ghostty
          pkgs.gpu-screen-recorder
        ];

        env.LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        env.LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
      };
    });
}
