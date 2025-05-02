{ lib
, stdenv
, fetchFromGitHub
, rustPlatform
, pkgconfig
, bzip2
, zstd
}:

rustPlatform.buildRustPackage rec {
  pname = "nu_plugin_cloud";
  version = "0.2.5";

  src = fetchFromGitHub {
    owner = "ayax79";
    repo = "nu_plugin_cloud";
    rev = "v${version}";
    hash = "sha256-e3xsyd2n2KwEmTVSdEYFze1JqArdmf5RU7oozjrgamk=";
  };

  # configurePhase = ''
  #   export BZIP2_SYS_USE_PKG_CONFIG=1 ZSTD_SYS_USE_PKG_CONFIG=1
  # '';
  
  # buildInputs = [ bzip2 zstd ];
  # nativeBuildInputs = [ pkgconfig ];

  cargoLock = {
    lockFile = src + /Cargo.lock;
  };

  meta = let inherit (lib) licenses platforms; in {
    description = "A nushell plugin for working with cloud storage services";
    homepage = "https://github.com/ayax79 ";
    license = licenses.mit;
    platforms = platforms.unix ++ platforms.windows;
  };
}

