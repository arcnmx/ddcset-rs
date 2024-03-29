{ config, pkgs, lib, ... }: with pkgs; with lib; let
  ddcset-rs = import ./. { inherit pkgs; };
  inherit (ddcset-rs) checks packages;
  artifactRoot = ".ci/artifacts";
  artifacts = "${artifactRoot}/bin/ddcset*";
  ddcset-checked = (packages.ddcset.override {
    buildType = "debug";
  }).overrideAttrs (_: {
    doCheck = true;
  });
in {
  config = {
    name = "ddcset";
    ci.gh-actions.enable = true;
    cache.cachix.arc.enable = true;
    channels = {
      nixpkgs = {
        # see https://github.com/arcnmx/nixexprs-rust/issues/10
        args.config.checkMetaRecursively = false;
        version = "22.11";
      };
    };
    tasks = {
      build.inputs = singleton ddcset-checked;
      fmt.inputs = singleton checks.rustfmt;
    };
    jobs = {
      nixos = {
        tasks = {
          build-windows.inputs = singleton packages.ddcset-w64;
          build-static.inputs = singleton packages.ddcset-static;
        };
        artifactPackages = {
          musl64 = packages.ddcset-static;
          win64 = packages.ddcset-w64;
        };
      };
      macos = {
        system = "x86_64-darwin";
        artifactPackages.macos = ddcset-checked;
      };
    };

    # XXX: symlinks are not followed, see https://github.com/softprops/action-gh-release/issues/182
    artifactPackage = runCommand "ddcset-artifacts" { } (''
      mkdir -p $out/bin
    '' + concatStringsSep "\n" (mapAttrsToList (key: ddcset: ''
        cp ${ddcset}/bin/ddcset${ddcset.stdenv.hostPlatform.extensions.executable} $out/bin/ddcset-${key}${ddcset.stdenv.hostPlatform.extensions.executable}
    '') config.artifactPackages));

    gh-actions = {
      jobs = mkIf (config.id != "ci") {
        ${config.id} = {
          permissions = {
            contents = "write";
          };
          step = {
            artifact-build = {
              order = 1100;
              name = "artifact build";
              uses = {
                # XXX: a very hacky way of getting the runner
                inherit (config.gh-actions.jobs.${config.id}.step.ci-setup.uses) owner repo version;
                path = "actions/nix/build";
              };
              "with" = {
                file = "<ci>";
                attrs = "config.jobs.${config.jobId}.artifactPackage";
                out-link = artifactRoot;
              };
            };
            artifact-upload = {
              order = 1110;
              name = "artifact upload";
              uses.path = "actions/upload-artifact@v3";
              "with" = {
                name = "ddcset";
                path = artifacts;
              };
            };
            release-upload = {
              order = 1111;
              name = "release";
              "if" = "startsWith(github.ref, 'refs/tags/')";
              uses.path = "softprops/action-gh-release@v1";
              "with".files = artifacts;
            };
          };
        };
      };
    };
  };
  options = {
    artifactPackage = mkOption {
      type = types.package;
    };
    artifactPackages = mkOption {
      type = with types; attrsOf package;
    };
  };
}
