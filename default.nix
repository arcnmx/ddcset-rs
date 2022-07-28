{ pkgs ? import <nixpkgs> { } }: let
  rustPath = let
    local = builtins.tryEval <rust>;
    remote = builtins.fetchTarball {
      url = "https://github.com/arcnmx/nixexprs-rust/archive/master.tar.gz";
    };
  in if local.success then local.value else remote;
  inherit (pkgs.pkgsCross) mingwW64;
  rustW64 = import rustPath { inherit (mingwW64) pkgs; };
  rust = import rustPath { inherit pkgs; };
  ddcset = pkgs.callPackage ./derivation.nix { };
  windows = mingwW64.callPackage ./derivation.nix {
    inherit (rustW64.stable) rustPlatform;
  };
  mingwW64-target = rust.lib.targetForConfig.${mingwW64.hostPlatform.config};
  rustChannel = rust.stable.override {
    channelOverlays = [
      (cself: csuper: {
        sysroot-std = csuper.sysroot-std ++ [ cself.manifest.targets.${mingwW64-target}.rust-std ];
      })
    ];
  };
  shell = with pkgs; rustChannel.mkShell {
    buildInputs = [
      udev
    ];
    nativeBuildInputs = [
      mingwW64.stdenv.cc
      python3
      pkg-config
    ];
  };
in ddcset // {
  inherit ddcset windows shell;
}
