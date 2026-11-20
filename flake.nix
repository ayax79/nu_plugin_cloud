{
  description = "A nushell plugin for working with cloud storage services";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      inherit (nixpkgs) lib;
      # nixpkgs 26.11 dropped x86_64-darwin.
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      cargoToml = lib.importTOML ./Cargo.toml;
      forAllSystems =
        f:
        lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );
      # Everything else is derived from the package set and toolchain, so the
      # package, dev shell and checks all build with the same toolchain.
      perSystem =
        pkgs:
        let
          toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          devToolchain = toolchain.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
          };
          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
          rustSrc = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./src
              (lib.fileset.maybeMissing ./rustfmt.toml)
            ];
          };
          package = rustPlatform.buildRustPackage {
            pname = cargoToml.package.name;
            inherit (cargoToml.package) version;
            src = rustSrc;
            cargoLock.lockFile = ./Cargo.lock;
            # nixpkgs' cargo setup hook (cargoUseSystemLibs) forces -sys crates
            # to link system libraries instead of vendored copies
            # (AWS_LC_SYS_USE_SYSTEM, LIBGIT2_NO_VENDOR,
            # LIBSQLITE3_SYS_USE_PKG_CONFIG, ...), so they must be found via
            # pkg-config. libgit2 comes in through nu-command's dev-dependency.
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [
              pkgs.aws-lc
              pkgs.libgit2
              pkgs.sqlite
              pkgs.zlib
            ];
            # Tests construct object_store/reqwest TLS clients, which load the
            # system CA store when the client is built. The nix build sandbox
            # has no system CA store, so provide one for the test phase.
            nativeCheckInputs = [ pkgs.cacert ];
            SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
            meta = {
              inherit (cargoToml.package) description;
              homepage = cargoToml.package.repository;
              license = lib.licenses.mit;
              mainProgram = cargoToml.package.name;
            };
          };
          runCheck =
            name: src: packages: script:
            pkgs.runCommand "${cargoToml.package.name}-${name}" { nativeBuildInputs = packages; } ''
              cd ${src}
              ${script}
              touch $out
            '';
        in
        {
          inherit package;
          devShell = pkgs.mkShell {
            # Listed before inputsFrom so the dev toolchain (with rust-src and
            # rust-analyzer) comes first on PATH.
            packages = [ devToolchain ];
            inputsFrom = [ package ];
          };
          checks = {
            inherit package;

            rustfmt = runCheck "rustfmt" rustSrc [ toolchain ] "cargo fmt --check";

            nixfmt =
              runCheck "nixfmt"
                (lib.fileset.toSource {
                  root = ./.;
                  fileset = lib.fileset.fileFilter (file: file.hasExt "nix") ./.;
                })
                [
                  pkgs.nixfmt
                  pkgs.findutils
                ]
                "find . -name '*.nix' -exec nixfmt --check {} +";

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
          };
        };
      outputs = forAllSystems perSystem;
    in
    {
      packages = lib.mapAttrs (_: o: {
        default = o.package;
        ${cargoToml.package.name} = o.package;
      }) outputs;
      devShells = lib.mapAttrs (_: o: { default = o.devShell; }) outputs;
      checks = lib.mapAttrs (_: o: o.checks) outputs;
      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
