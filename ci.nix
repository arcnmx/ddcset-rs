{ config, pkgs, lib, ... }: with pkgs; with lib; let
  ddcset = import ./. { inherit pkgs; };
  artifactRoot = ".ci/artifacts";
  artifacts = "${artifactRoot}/bin/ddcset*";
  musl64 = pkgsCross.musl64.pkgsStatic;
  ddcset-static = (musl64.callPackage ./derivation.nix {
    udev = (musl64.eudev.override {
      glib = null; gperf = null; util-linux = null; kmod = null;
    }).overrideAttrs (old: {
      # XXX: apply hack to fix https://github.com/NixOS/nixpkgs/pull/145819
      nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.gperf ];
    });
    inherit ((import config.channels.rust.path { pkgs = musl64; }).stable) rustPlatform;
  }).overrideAttrs (old: {
    # XXX: why is this needed?
    NIX_LDFLAGS = old.NIX_LDFLAGS or "" + " -static";
    RUSTFLAGS = old.RUSTFLAGS or "" + " -C default-linker-libraries=yes";
  });
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
        artifactPackages = {
          musl64 = ddcset-static;
          win64 = ddcset.windows;
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
  options = {
    artifactPackage = mkOption {
      type = types.package;
    };
    artifactPackages = mkOption {
      type = with types; attrsOf package;
    };
  };
}
