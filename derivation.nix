{ rustPlatform
, nix-gitignore
, udev
, python3, pkg-config
, hostPlatform
, lib
, libiconv, CoreGraphics ? darwin.apple_sdk.frameworks.CoreGraphics, darwin
, buildType ? "release"
, cargoLock ? {
  lockFile = ./Cargo.lock;
  outputHashes = {
    "ddc-hi-0.5.0" = "sha256-qjjn5OAqNfKZmW5buzwEc752WgSDU+yVGrM3e6VFMHs=";
    "nvapi-0.2.0" = "sha256-wKaYVG7i7yJ/0ot+ISRJ8/Vnygti72FRqm3LQBy9UCc=";
  };
}, _arg'ddcset ? nix-gitignore.gitignoreSourcePure [ ./.gitignore ''
  /.github
  /.git
  *.nix
'' ] ./.
}: with lib; let
  cargoToml = importTOML ./Cargo.toml;
in rustPlatform.buildRustPackage {
  pname = "ddcset";
  version = if buildType == "release"
    then cargoToml.package.version
    else _arg'ddcset.lastModifiedDate or cargoToml.package.version;

  buildInputs =
    optionals hostPlatform.isLinux [ udev ]
    ++ optionals hostPlatform.isDarwin [ libiconv CoreGraphics ];
  nativeBuildInputs = [ pkg-config python3 ];

  src = _arg'ddcset;
  inherit cargoLock buildType;
  ${if cargoLock == null then "cargoSha256" else null} = "sha256-vt9XpVTPCR2e3g5O2ChPRws4CiiWV0jj1xQ17kiHSJM=";
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
