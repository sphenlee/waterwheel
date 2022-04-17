{inputs}: let
  inherit (inputs) nix-filter;
  waterwheel-commit = inputs.self.shortRev or "dirty";
  waterwheel-date = inputs.self.lastModifiedDate or inputs.self.lastModified or "19700101";
  waterwheel-version = "${builtins.substring 0 8 waterwheel-date}.${waterwheel-commit}";
in {
  default = final: prev: rec {
    npmlock2nix = import inputs.npmlock2nix {pkgs = prev;};
    waterwheel = prev.callPackage ./waterwheel.nix {inherit waterwheel-version waterwheel-ui;};
    waterwheel-ui = prev.callPackage ./waterwheel-ui.nix {inherit waterwheel-version npmlock2nix waterwheel-commit;};
  };
}
