let
  self = import ./. { pkgs = null; system = null; };
in {
  rustPlatform
, nix-gitignore
, udev
, python3, pkg-config
, hostPlatform
, lib
, libiconv, CoreGraphics ? darwin.apple_sdk.frameworks.CoreGraphics, darwin
, buildType ? "release"
, cargoLock ? crate.cargoLock
, source ? crate.src
, crate ? self.lib.crate
}: with lib; rustPlatform.buildRustPackage {
  pname = crate.name;
  inherit (crate) version;

  buildInputs =
    optionals hostPlatform.isLinux [ udev ]
    ++ optionals hostPlatform.isDarwin [ libiconv CoreGraphics ];
  nativeBuildInputs = [ pkg-config python3 ];

  src = source;
  inherit cargoLock buildType;
  doCheck = false;

  meta = {
    description = "DDC/CI display control application";
    homepage = "https://github.com/arcnmx/ddcset-rs";
    license = licenses.mit;
    maintainers = [ maintainers.arcnmx ];
    platforms = platforms.unix ++ platforms.windows;
    mainProgram = "ddcset";
  };
}
