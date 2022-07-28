{ rustPlatform
, nix-gitignore
, udev
, python3, pkg-config
, hostPlatform
, lib
, libiconv, CoreGraphics ? darwin.apple_sdk.frameworks.CoreGraphics, darwin
, ...
}: with lib; let
  cargoToml = importTOML ./Cargo.toml;
in rustPlatform.buildRustPackage {
  pname = "ddcset";
  version = cargoToml.package.version;

  buildInputs =
    optionals hostPlatform.isLinux [ udev ]
    ++ optionals hostPlatform.isDarwin [ libiconv CoreGraphics ];
  nativeBuildInputs = [ pkg-config python3 ];

  src = nix-gitignore.gitignoreSourcePure [ ./.gitignore ''
    /.github
    /.git
    *.nix
  '' ] ./.;

  cargoSha256 = "sha256-o+E5JZivfq7Ya0QsvLUBOoAFtzIahgnjjQDGCafaVP4=";
  doCheck = false;

  meta = {
    platforms = platforms.unix ++ platforms.windows;
  };
}
