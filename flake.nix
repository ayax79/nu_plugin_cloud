{
  description = ''
    A nushell plugin for working with cloud storage services.
  '';

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      forAllSystems = lib.genAttrs lib.systems.flakeExposed;
    in
    {
      packages = forAllSystems (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in rec {
          default = verify-archive;
          verify-archive = pkgs.callPackage ./default.nix { };
        });

      apps = forAllSystems (system: rec {
        default = verify-archive;
        verify-archive = {
          type = "app";
          program = "${lib.getBin self.packages.${system}.verify-archive}/bin/nu_plugin_cloud";
        };
      });
    };
}
