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

    nativeBuildInputs = with pkgs; [pkg-config wrapGAppsHook3 libclang];

    buildInputs = with pkgs; [
      wayland
      libxkbcommon
      fontconfig
      freetype
      pipewire
      linux-pam
      vulkan-loader
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

    packages.zex-shell = pkgs.rustPlatform.buildRustPackage {
      pname = "zex-shell";
      inherit version;

      src = self;
      cargoLock = {
        lockFile = ./Cargo.lock;
        allowBuiltinFetchGit = true;
      };

      inherit nativeBuildInputs buildInputs;

      meta = with pkgs.lib; {
        mainProgram = "zex-shell";
        description = "Zex shell daemon";
        license = licenses.gpl3Plus;
        platforms = platforms.linux;
      };

      postFixup = ''
        for f in $out/bin/*; do
          patchelf --add-rpath ${pkgs.wayland}/lib "$f"
        done
      '';
    };

    packages.zexctl = pkgs.rustPlatform.buildRustPackage {
      pname = "zexctl";
      inherit version;

      src = self;
      cargoLock = {
        lockFile = ./Cargo.lock;
        allowBuiltinFetchGit = true;
      };

      inherit nativeBuildInputs buildInputs;

      meta = with pkgs.lib; {
        mainProgram = "zexctl";
        description = "Zex shell control CLI";
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
      # Required for iced_wgpu
      env.VK_ICD_FILENAMES = "${pkgs.vulkan-loader}/share/vulkan/icd.d/lvp_icd.x86_64.json";
    };
  }) // {
    homeManagerModules.default = { config, lib, pkgs, ... }:
      let
        cfg = config.programs.zex;
        shellPkg = self.packages.${pkgs.system}.zex-shell;
        ctlPkg = self.packages.${pkgs.system}.zexctl;
      in {
        options.programs.zex = {
          enable = lib.mkEnableOption "zex shell";

          settings = lib.mkOption {
            type = lib.types.attrsOf (lib.types.either lib.types.str (lib.types.either lib.types.int (lib.types.either lib.types.bool (lib.types.attrsOf lib.types.str))));
            default = { };
            description = "Zex shell settings (mirrors ~/.config/zex/settings.json schema)";
          };

          package = lib.mkOption {
            type = lib.types.package;
            default = shellPkg;
            description = "The zex-shell package to use";
          };

          extraPackages = lib.mkOption {
            type = lib.types.listOf lib.types.package;
            default = [ pkgs.matugen pkgs.ghostty pkgs.gpu-screen-recorder ];
            description = "Extra packages to install alongside zex";
          };

          pam = {
            enable = lib.mkEnableOption "PAM service for lockscreen";
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
        };

        config = lib.mkIf cfg.enable {
          environment.systemPackages = [ shellPkg ctlPkg ] ++ cfg.extraPackages;

          # PAM service for lockscreen
          security.pam.services.zex = lib.mkIf cfg.pam.enable {
            text = cfg.pam.text;
          };

          # Generate settings.json from Nix options
          xdg.configFile."zex/settings.json" = {
            text = lib.toJSON cfg.settings;
          };

          # Autostart zex-shell via XDG autostart (no systemd)
          xdg.configFile."autostart/zex.desktop" = {
            text = ''
              [Desktop Entry]
              Type=Application
              Name=Zex Shell
              Exec=${shellPkg}/bin/zex-shell
              X-GNOME-Autostart-enabled=true
            '';
          };
        };
      };

    nixosModules.default = { config, lib, ... }:
      self.homeManagerModules.default { config = config; inherit lib; };
  };
}
