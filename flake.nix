{
  description = "DDC/CI display control application for Windows and Linux";
  inputs = {
    flakelib.url = "github:flakelib/fl";
    nixpkgs = { };
    rust = {
      url = "github:arcnmx/nixexprs-rust";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, flakelib, nixpkgs, rust, ... }@inputs: let
    nixlib = nixpkgs.lib;
  in flakelib {
    inherit inputs;
    systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
    devShells = {
      plain = {
        mkShell, writeShellScriptBin, hostPlatform
      , udev
      , pkg-config, python3
      , libiconv
      , CoreGraphics ? darwin.apple_sdk.frameworks.CoreGraphics, darwin
      , enableRust ? true, cargo
      , rustTools ? [ ]
      }: mkShell {
        inherit rustTools;
        buildInputs =
          nixlib.optional hostPlatform.isLinux udev
          ++ nixlib.optionals hostPlatform.isDarwin [ libiconv CoreGraphics ];
        nativeBuildInputs = [ pkg-config python3 ]
          ++ nixlib.optional enableRust cargo
          ++ [
            (writeShellScriptBin "generate" ''nix run .#generate "$@"'')
          ];
      };
      stable = { rust'stable, outputs'devShells'plain }: outputs'devShells'plain.override {
        inherit (rust'stable) mkShell;
        enableRust = false;
      };
      dev = { rust'unstable, rust-w64-overlay, outputs'devShells'plain }: let
        channel = rust'unstable.override {
          channelOverlays = [ rust-w64-overlay ];
        };
      in outputs'devShells'plain.override {
        inherit (channel) mkShell;
        enableRust = false;
        rustTools = [ "rust-analyzer" ];
      };
      default = { outputs'devShells }: outputs'devShells.plain;
    };
    packages = {
      ddcset = {
        __functor = _: import ./derivation.nix;
        fl'config.args = {
          crate.fallback = self.lib.crate;
        };
      };
      ddcset-w64 = { pkgsCross'mingwW64, rust-w64, source }: pkgsCross'mingwW64.callPackage ./derivation.nix {
        inherit (rust-w64.latest) rustPlatform;
        inherit source;
      };
      ddcset-static = { pkgsCross'musl64'pkgsStatic, eudev-musl64, source }: (pkgsCross'musl64'pkgsStatic.callPackage ./derivation.nix {
        inherit ((import inputs.rust { pkgs = pkgsCross'musl64'pkgsStatic; }).latest) rustPlatform;
        udev = eudev-musl64;
        inherit source;
      }).overrideAttrs (old: {
        # XXX: why is this needed?
        NIX_LDFLAGS = old.NIX_LDFLAGS or "" + " -static";
        RUSTFLAGS = old.RUSTFLAGS or "" + " -C default-linker-libraries=yes";
      });
      default = { ddcset }: ddcset;
    };
    legacyPackages = { callPackageSet }: callPackageSet {
      source = { rust'builders }: rust'builders.wrapSource self.lib.crate.src;

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

      generate = { rust'builders, outputHashes }: rust'builders.generateFiles {
        paths = {
          "lock.nix" = outputHashes;
        };
      };
      outputHashes = { rust'builders }: rust'builders.cargoOutputHashes {
        inherit (self.lib) crate;
      };
    } { };
    checks = {
    };
    lib = with nixlib; {
      crate = rust.lib.importCargo {
        path = ./Cargo.toml;
        inherit (import ./lock.nix) outputHashes;
      };
      inherit (self.lib.crate) version;
      releaseTag = "v${self.lib.version}";
    };
    config = rec {
      name = "ddcset-rs";
    };
  };
}
