{
  npmlock2nix,
  waterwheel-version,
  waterwheel-commit,
}:
npmlock2nix.build {
  src = ../ui;
  installPhase = "cp -r dist $out";
  preBuild = ''
    substituteInPlace webpack.config.js \
    --replace 'gitRevisionPlugin.version()' '"${waterwheel-version}"' \
    --replace 'gitRevisionPlugin.commithash()' '"${waterwheel-commit}"'
  '';
  buildCommands = ["npm run build"];
}
