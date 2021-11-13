{ pkgs, lib, ... }: with pkgs; with lib; let
  ddcset = import ./. { inherit pkgs; };
in {
  name = "ddcset";
  ci.gh-actions.enable = true;
  cache.cachix.arc.enable = true;
  channels = {
    nixpkgs = "21.11";
    rust = "master";
  };
  tasks = {
    build.inputs = with ddcset; [
      ddcset windows
    ];
  };
}
