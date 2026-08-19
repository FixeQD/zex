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
    }) // {
      nixosModule = {
        config,
        lib,
        ...
      }: {
        options.zex.pam = {
          enable = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Install the PAM service the zex lockscreen authenticates against";
          };
          text = lib.mkOption {
            type = lib.types.str;
            default = ''
              account required pam_unix.so

              auth optional pam_unix.so likeauth nullok
              auth sufficient pam_unix.so likeauth nullok try_first_pass
              auth required pam_deny.so

              password sufficient pam_unix.so nullok sha512

              session required pam_env.so conffile=/etc/security/pam_env.conf readenv=0
              session required pam_unix.so
              session required pam_loginuid.so
              session required pam_limits.so conf=/etc/security/limits.conf
            '';
            description = "Contents of /etc/pam.d/zex";
          };
        };

        config = lib.mkIf config.zex.pam.enable {
          security.pam.services.zex.text = config.zex.pam.text;
        };
      };

      nixosModules.default = self.nixosModule;
    };
}
