{
  rustPlatform,
  openssl,
  rust-bin,
  pkg-config,
  waterwheel-version,
  waterwheel-ui,
  lib,
  stdenv,
}:
rustPlatform.buildRustPackage {
  pname = "waterwheel";
  version = waterwheel-version;
  buildInputs = [openssl];
  nativeBuildInputs = [
    rust-bin.nightly.latest.default
    pkg-config
  ];

  src = stdenv.mkDerivation {
    name = "src";
    builder = builtins.toFile "builder.sh" ''
      source $stdenv/setup
      mkdir $out
      cp -rT --no-preserve=mode,ownership $src $out/src/
      cp $cargoLock $out/Cargo.lock
      cp $cargoToml $out/Cargo.toml
      mkdir $out/ui
      cp -rT --no-preserve=mode,ownership $ui $out/ui/dist
    '';
    cargoLock = ../Cargo.lock;
    cargoToml = ../Cargo.toml;
    ui = waterwheel-ui;
    src = ../src;
  };

  preBuild = ''
    substituteInPlace src/lib.rs \
    --replace 'git_version::git_version!()' '"${waterwheel-version}"'
  '';

  cargoLock.lockFile = ../Cargo.lock;

  doCheck = false;

  meta = with lib; {
    description = "A workflow scheduler based on petri-nets";
    homepage = "https://github.com/sphenlee/waterwheel";
    license = lib.licenses.mit;
    maintainers = [lib.maintainers.gtrunsec];
    platforms = lib.systems.doubles.all;
  };
}
