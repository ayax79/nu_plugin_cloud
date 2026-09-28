{
  inputs.rust-flake.url = "github:KaiSforza/rust-flake";
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
