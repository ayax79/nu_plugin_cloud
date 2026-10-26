{
  description = "A nushell plugin for working with cloud storage services";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-flake.url = "github:KaiSforza/rust-flake";
  inputs.rust-flake.inputs.nixpkgs.follows = "nixpkgs";
  # Pin rust-overlay at the top level so `nix flake update` advances it
  # rather than resetting it to rust-flake's (much older) locked revision.
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  inputs.rust-flake.inputs.rust-overlay.follows = "rust-overlay";
  inputs.systems = {
    url = "path:./systems.nix";
    flake = false;
  };
  inputs.rust-flake.inputs.systems.follows = "systems";
  outputs =
    inputs:
    let
      inherit (inputs.rust-flake.lib) rust-flake rust-flakes;
      outputs = rust-flakes [
        (rust-flake {
          root = ./.;
          # Tests construct object_store/reqwest TLS clients, which load the
          # system CA store when the client is built. The nix build sandbox
          # has no system CA store, so provide one for the test phase.
          pkg-overrides = final: prev: {
            nativeCheckInputs = [ final.cacert ];
            SSL_CERT_FILE = "${final.cacert}/etc/ssl/certs/ca-bundle.crt";
          };
        })
      ];
      # Same package set rust-flake builds with, so fmt/clippy come from the
      # toolchain pinned in rust-toolchain.toml.
      forAllSystems =
        f:
        inputs.nixpkgs.lib.genAttrs (import inputs.systems) (
          system:
          f (
            import inputs.nixpkgs {
              inherit system;
              overlays = [
                inputs.rust-overlay.overlays.default
                outputs.overlays.default
              ];
            }
          )
        );
    in
    outputs
    // {
      checks = forAllSystems (
        pkgs:
        let
          src = pkgs.lib.cleanSource ./.;
          package = pkgs.nu_plugin_cloud;
          runCheck =
            name: packages: script:
            pkgs.runCommand "nu_plugin_cloud-${name}" { nativeBuildInputs = packages; } ''
              cd ${src}
              ${script}
              touch $out
            '';
        in
        {
          inherit package;

          rustfmt = runCheck "rustfmt" [ pkgs.rustToolchain' ] "cargo fmt --check";

          nixfmt = runCheck "nixfmt" [
            pkgs.nixfmt
            pkgs.findutils
          ] "find . -name '*.nix' -exec nixfmt --check {} +";

          clippy = package.overrideAttrs (old: {
            pname = "${old.pname}-clippy";
            buildPhase = ''
              runHook preBuild
              cargo clippy --offline --all-targets -- -D warnings
              runHook postBuild
            '';
            doCheck = false;
            installPhase = "touch $out";
            dontFixup = true;
          });
        }
      );

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
