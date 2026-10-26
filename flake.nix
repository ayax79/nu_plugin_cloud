{
  description = "A nushell plugin for working with cloud storage services";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-flake.url = "github:KaiSforza/rust-flake";
  inputs.rust-flake.inputs.nixpkgs.follows = "nixpkgs";
  inputs.systems = {
    url = "path:./systems.nix";
    flake = false;
  };
  inputs.rust-flake.inputs.systems.follows = "systems";
  outputs = inputs:
    let inherit (inputs.rust-flake.lib) rust-flake rust-flakes;
    in rust-flakes [
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
}
