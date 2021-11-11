{ rustPlatform
, nix-gitignore
, libxcb, udev
, python3, pkg-config
, hostPlatform
, lib
, ...
}: let
  cargoToml = lib.importTOML ./Cargo.toml;
in rustPlatform.buildRustPackage {
  pname = "ddcset";
  version = cargoToml.package.version;

  buildInputs = lib.optionals hostPlatform.isLinux [ libxcb udev ];
  nativeBuildInputs = [ pkg-config python3 ];

  src = nix-gitignore.gitignoreSourcePure [ ./.gitignore ''
    *.nix
  '' ] ./.;

  cargoSha256 = "1bif5sbccsp48qh4vwqx28ifricj1i40f7ddhdv9pnqva5py0fl2";
}
