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

  cargoSha256 = "0ym9mcva3f2nmxhl91dh3fcnpyahn5xyxj8519l8sp404lvs370k";
}
