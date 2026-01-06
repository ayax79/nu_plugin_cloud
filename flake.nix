{
  inputs.rust-flake.url = "github:KaiSforza/rust-flake";
  outputs = inputs:
    let inherit (inputs.rust-flake.lib) rust-flake rust-flakes;
    in rust-flakes [ (rust-flake { root = ./.; }) ];
}
