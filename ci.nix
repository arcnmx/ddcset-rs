{ pkgs, lib, ... }: with pkgs; with lib; let
  ddcset = import ./. { inherit pkgs; };
in {
  name = "ddcset";
  ci.gh-actions.enable = true;
  cache.cachix.arc.enable = true;
  channels = {
    nixpkgs = {
      # see https://github.com/arcnmx/nixexprs-rust/issues/10
      args.config.checkMetaRecursively = false;
    };
    rust = "master";
  };
  tasks = {
    build.inputs = singleton (ddcset.overrideAttrs (_: {
      doCheck = true;
    }));
  };
  jobs = {
    nixos.tasks.build-windows.inputs = singleton ddcset.windows;
    macos.system = "x86_64-darwin";
  };
}
