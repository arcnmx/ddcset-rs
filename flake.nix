{
  description = "DDC/CI display control application for Windows and Linux";
  inputs = {
    flakelib.url = "github:flakelib/fl";
    nixpkgs = { };
    rust = {
      url = "github:arcnmx/nixexprs-rust";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    arc = {
      url = "github:arcnmx/nixexprs";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, flakelib, nixpkgs, ... }@inputs: let
    nixlib = nixpkgs.lib;
    impure = builtins ? currentSystem;
  in flakelib {
    inherit inputs;
    systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
    devShells = {
      plain = {
        mkShell, hostPlatform
      , udev
      , pkg-config, python3
      , libiconv
      , CoreGraphics ? darwin.apple_sdk.frameworks.CoreGraphics, darwin
      , enableRust ? true, cargo
      , rustTools ? [ ], rust'latest
      }: mkShell {
        inherit rustTools;
        buildInputs =
          nixlib.optional hostPlatform.isLinux udev
          ++ nixlib.optional hostPlatform.isDarwin CoreGraphics;
        nativeBuildInputs = [ pkg-config python3 ]
          ++ nixlib.optional enableRust cargo;
      };
      stable = { rust'stable, rust'latest, outputs'devShells'plain }: let
        stable = if impure then rust'stable else rust'latest;
      in outputs'devShells'plain.override {
        inherit (stable) mkShell;
        enableRust = false;
      };
      dev = { arc'rustPlatforms, rust'nightly, rust-w64-overlay, outputs'devShells'plain }: let
        nightly = arc'rustPlatforms.nightly.hostChannel;
        channel = rust'nightly.override {
          inherit (nightly) date manifestPath;
          rustcDev = true;
          channelOverlays = [ rust-w64-overlay ];
        };
      in outputs'devShells'plain.override {
        inherit (channel) mkShell;
        enableRust = false;
        rustTools = [ "rust-analyzer" "rustfmt" ];
      };
      default = { outputs'devShells }: outputs'devShells.plain;
    };
    packages = {
      ddcset = {
        __functor = _: import ./derivation.nix;
        fl'config.args = {
          _arg'ddcset.fallback = self;
        };
      };
      ddcset-w64 = { pkgsCross'mingwW64, rust-w64 }: pkgsCross'mingwW64.callPackage ./derivation.nix {
        inherit (rust-w64.latest) rustPlatform;
        cargoLock = null;
        _arg'ddcset = self;
      };
      ddcset-static = { pkgsCross'musl64'pkgsStatic, eudev-musl64 }: (pkgsCross'musl64'pkgsStatic.callPackage ./derivation.nix {
        inherit ((import inputs.rust { pkgs = pkgsCross'musl64'pkgsStatic; }).latest) rustPlatform;
        cargoLock = null;
        udev = eudev-musl64;
        _arg'ddcset = self;
      }).overrideAttrs (old: {
        # XXX: why is this needed?
        NIX_LDFLAGS = old.NIX_LDFLAGS or "" + " -static";
        RUSTFLAGS = old.RUSTFLAGS or "" + " -C default-linker-libraries=yes";
      });
      default = { ddcset }: ddcset;
    };
    legacyPackages = { callPackageSet }: callPackageSet {
      rust-w64 = { pkgsCross'mingwW64 }: import inputs.rust { inherit (pkgsCross'mingwW64) pkgs; };
      rust-w64-overlay = { rust-w64 }: let
        target = rust-w64.lib.rustTargetEnvironment {
          inherit (rust-w64) pkgs;
          rustcFlags = [ "-L native=${rust-w64.pkgs.windows.pthreads}/lib" ];
        };
      in cself: csuper: {
        sysroot-std = csuper.sysroot-std ++ [ cself.manifest.targets.${target.triple}.rust-std ];
        cargo-cc = csuper.cargo-cc // cself.context.rlib.cargoEnv {
          inherit target;
        };
        rustc-cc = csuper.rustc-cc // cself.context.rlib.rustcCcEnv {
          inherit target;
        };
      };
      eudev-musl64 = { pkgsCross'musl64'pkgsStatic, gperf }: (pkgsCross'musl64'pkgsStatic.eudev.override {
        glib = null; gperf = null; util-linux = null; kmod = null;
      }).overrideAttrs (old: {
        # XXX: apply hack to fix https://github.com/NixOS/nixpkgs/pull/145819
        nativeBuildInputs = old.nativeBuildInputs ++ [ gperf ];
        patches = old.patches or [ ] ++ [ ./eudev-gettid.patch ];
      });
    } { };
    checks = {
      rustfmt = { rustfmt, cargo, ddcset, runCommand }: runCommand "ddcset-cargo-fmt-check" {
        nativeBuildInputs = [ cargo (rustfmt.override { asNightly = true; }) ];
        inherit (ddcset) src;
        meta.name = "cargo fmt (nix run .#wpdev-fmt)";
      } ''
        cargo fmt --check \
          --manifest-path $src/Cargo.toml
        touch $out
      '';
    };
    lib = with nixlib; {
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      inherit (self.lib.cargoToml.package) version;
      releaseTag = "v${self.lib.version}";
    };
    config = rec {
      name = "ddcset-rs";
      packages.namespace = [ name ];
      inputs.arc = {
        lib.namespace = [ "arc" ];
        packages.namespace = [ "arc" ];
      };
    };
  };
}
