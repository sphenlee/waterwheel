{inputs}: final: prev: let
  inherit (inputs) nix-filter;
  commit = inputs.self.shortRev or "dirty";
  date = inputs.self.lastModifiedDate or inputs.self.lastModified or "19700101";
  version = "${builtins.substring 0 8 date}.${commit}";
  npmlock2nix = import inputs.npmlock2nix {pkgs = prev;};
in rec {
  waterwheel = prev.callPackage ./waterwheel.nix {inherit version waterwheel-ui nix-filter;};
  waterwheel-ui = prev.callPackage ./waterwheel-ui.nix {inherit version npmlock2nix commit;};
}
