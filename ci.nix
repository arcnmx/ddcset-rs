{ config, pkgs, lib, ... }: with pkgs; with lib; let
  ddcset = import ./. { inherit pkgs; };
  artifactRoot = ".ci/artifacts";
  artifacts = "${artifactRoot}/bin/ddcset*";
  musl64 = pkgsCross.musl64.pkgsStatic;
  ddcset-static = musl64.callPackage ./derivation.nix {
    udev = (musl64.eudev.override {
      glib = null; gperf = null; util-linux = null; kmod = null;
    }).overrideAttrs (old: {
      # XXX: apply hack to fix https://github.com/NixOS/nixpkgs/pull/145819
      nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.gperf ];
    });
  };
  ddcset-checked = ddcset.overrideAttrs (_: {
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
      };
      rust = "master";
    };
    tasks = {
      build.inputs = singleton ddcset-checked;
    };
    jobs = {
      nixos = {
        tasks = {
          build-windows.inputs = singleton ddcset.windows;
          build-static.inputs = singleton ddcset-static;
        };
        artifactPackage = symlinkJoin {
          name = "ddcset-artifacts";
          paths = [ ddcset-static ddcset.windows ];
          postBuild = ''
            mv $out/bin/ddcset $out/bin/ddcset-musl64
          '';
        };
      };
      macos = {
        system = "x86_64-darwin";
        artifactPackage = runCommand "ddcset-artifact" { } ''
          mkdir -p $out/bin
          ln -s ${ddcset-checked}/bin/ddcset${hostPlatform.extensions.executable} $out/bin/ddcset-macos64${hostPlatform.extensions.executable}
        '';
      };
    };

    artifactPackage = mkDefault ddcset-checked;

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
              uses.path = "actions/upload-artifact@v2";
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
  options.artifactPackage = mkOption {
    type = types.package;
  };
}
