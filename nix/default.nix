{
  naersk,
  rust-bin,
  waterwheel-version,
  waterwheel-ui,
  lib,
  openssl,
  pkg-config,
  runCommand,
}:
(naersk.override {
  rustc = rust-bin.nightly.latest.default;
  cargo = rust-bin.nightly.latest.default;
})
.buildPackage {
  version = waterwheel-version;
  root =
    runCommand "${waterwheel-version}-src" {
      cargoLock = ../Cargo.lock;
      cargoToml = ../Cargo.toml;
      ui = waterwheel-ui;
      src = ../src;
    } ''
      mkdir -p "$out"
      cp -rT --no-preserve=mode,ownership $src $out/src/
      cp $cargoLock $out/Cargo.lock
      cp $cargoToml $out/Cargo.toml
      mkdir $out/ui
      cp -rT --no-preserve=mode,ownership $ui $out/ui/dist
    '';

  nativeBuildInputs = [openssl.dev pkg-config];

  GIT_HASH = waterwheel-version;

  doCheck = true;
  checkPhase = ''
    cargo test
  '';

  meta = with lib; {
    description = "A workflow scheduler based on petri-nets";
    homepage = "https://github.com/sphenlee/waterwheel";
    license = lib.licenses.mit;
    maintainers = [lib.maintainers.gtrunsec];
    platforms = lib.systems.doubles.all;
  };
}
