# Systems to build for. nixpkgs 26.11 dropped x86_64-darwin, so it is
# excluded from the nix-systems/default list rust-flake uses by default.
[
  "aarch64-darwin"
  "aarch64-linux"
  "x86_64-linux"
]
