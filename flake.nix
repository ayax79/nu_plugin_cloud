{
  description = "Nix flake for nu_plugin_cloud Rust project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system};
      in {
        packages.default = pkgs.mkShell {
          buildInputs = [ pkgs.rust pkgs.cargo ];
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [ pkgs.rust pkgs.cargo ];
          shellHook = ''
            echo "Development shell for nu_plugin_cloud initialized!"
          '';
        };

        apps.default = {
          type = "app";
          program = "${self.packages.default}/bin/nu_plugin_cloud";
        };
      });
}

