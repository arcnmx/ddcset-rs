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
  shellBase = pkgs.shells.rust.stable or (rust.stable.mkShell { });
  shell = shellBase.overrideAttrs (old: with pkgs; {
    buildInputs = old.buildInputs or [] ++ [
      xorg.libxcb
      udev
    ];
    nativeBuildInputs = old.nativeBuildInputs or [] ++ [
      python3
      pkg-config
    ];
  });
in ddcset // {
  inherit ddcset windows shell;
}
