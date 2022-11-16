{ pkgs ? import <nixpkgs> { } }: let
  hasPkgs = pkgs ? path;
  lockData = builtins.fromJSON (builtins.readFile ./flake.lock);
  sourceInfo = lockData.nodes.std.locked;
  src = fetchTarball {
    url = "https://github.com/${sourceInfo.owner}/${sourceInfo.repo}/archive/${sourceInfo.rev}.tar.gz";
    sha256 = sourceInfo.narHash;
  };
  inherit (import src) Flake;
  inputs = Flake.Lock.Node.inputs (Flake.Lock.root (Flake.Lock.New (lockData // {
    override.sources = if hasPkgs then {
      nixpkgs = pkgs.path;
    } else { };
  })));
  ddcset-rs = Flake.CallDir ./. inputs;
  checks = ddcset-rs.checks.${pkgs.system};
  packages = ddcset-rs.packages.${pkgs.system};
  devShells = ddcset-rs.devShells.${pkgs.system};
in (if hasPkgs then packages.ddcset // {
  inherit packages checks devShells;
  inherit (packages) ddcset;
  windows = packages.ddcset-w64;
  static = packages.ddcset-static;
  shell = devShells.default;
} else { }) // {
  inherit inputs;
  outputs = ddcset-rs;
}
