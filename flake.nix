{
  description = "A workflow scheduler based on petri-nets";

  inputs = {
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

    npmlock2nix.url = "github:tweag/npmlock2nix";
    npmlock2nix.flake = false;

    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  };

  outputs = inputs: let
    commit = inputs.self.shortRev or "dirty";
    date = inputs.self.lastModifiedDate or inputs.self.lastModified or "19700101";
    version = "${builtins.substring 0 8 date}.${commit}";
    nixpkgsForHost = host:
      import inputs.nixpkgs {
        overlays = [
          inputs.rust-overlay.overlay
          (final: prev: let
            npmlock2nix = import inputs.npmlock2nix {pkgs = prev;};
          in {
            waterwheel-ui = npmlock2nix.build {
              src = ./ui;
              installPhase = "cp -r dist $out";
              preBuild = ''
                substituteInPlace webpack.config.js \
                --replace 'gitRevisionPlugin.version()' '"${version}"' \
                --replace 'gitRevisionPlugin.commithash()' '"${commit}"'
              '';
              buildCommands = ["npm run build"];
            };

            waterwheel = prev.rustPlatform.buildRustPackage {
              pname = "waterwheel";
              inherit version;
              buildInputs = [prev.openssl];
              nativeBuildInputs = [
                final.rust-bin.nightly.latest.default
                prev.pkg-config
              ];

              src = ./.;

              preBuild = ''
                substituteInPlace src/lib.rs \
                --replace 'git_version::git_version!()' '"${version}"'

                cp -rf ${final.waterwheel-ui} ui/dist
              '';

              cargoLock.lockFile = ./Cargo.lock;

              doCheck = false;

              meta = with prev; {
                description = "The Uncompromising Nix Code Formatter.";
                homepage = "https://github.com/sphenlee/waterwheel";
                license = lib.licenses.mit;
                maintainers = [lib.maintainers.gtrunsec];
                platforms = lib.systems.doubles.all;
              };
            };
          })
        ];
        system = host;
      };

    nixpkgs."aarch64-darwin" = nixpkgsForHost "aarch64-darwin";
    nixpkgs."aarch64-linux" = nixpkgsForHost "aarch64-linux";
    nixpkgs."i686-linux" = nixpkgsForHost "i686-linux";
    nixpkgs."x86_64-darwin" = nixpkgsForHost "x86_64-darwin";
    nixpkgs."x86_64-linux" = nixpkgsForHost "x86_64-linux";

    buildBinariesForHost = host: pkgs: let
      binaries = builtins.listToAttrs (
        builtins.map (pkg: {
          name = "waterwheel-${pkg.stdenv.targetPlatform.config}";
          value = pkg;
        })
        pkgs
      );
    in
      binaries
      // {
        "waterwheel-binaries" = nixpkgs.${host}.linkFarm "waterwheel-binaries" (
          nixpkgs.${host}.lib.mapAttrsToList
          (name: binary: {
            inherit name;
            path = "${binary}/bin/waterwheel";
          })
          binaries
        );
      };
  in rec {
    checks."aarch64-darwin" = packages."aarch64-darwin";
    checks."aarch64-linux" = packages."aarch64-linux";
    checks."i686-linux" = packages."i686-linux";
    checks."x86_64-darwin" = packages."x86_64-darwin";
    checks."x86_64-linux" = packages."x86_64-linux";

    defaultPackage."aarch64-darwin" = packages."aarch64-darwin"."waterwheel-aarch64-apple-darwin";
    defaultPackage."aarch64-linux" = packages."aarch64-linux"."waterwheel-aarch64-unknown-linux-gnu";
    defaultPackage."i686-linux" = packages."i686-linux"."waterwheel-i686-unknown-linux-gnu";
    defaultPackage."x86_64-darwin" = packages."x86_64-darwin"."waterwheel-x86_64-apple-darwin";
    defaultPackage."x86_64-linux" = packages."x86_64-linux"."waterwheel-x86_64-unknown-linux-gnu";

    packages."aarch64-darwin" = with nixpkgs."aarch64-darwin";
      buildBinariesForHost "aarch64-darwin" [
        waterwheel
      ];
    packages."aarch64-linux" = with nixpkgs."aarch64-linux";
      buildBinariesForHost "aarch64-linux" [
        waterwheel
        pkgsStatic.waterwheel
      ];
    packages."i686-linux" = with nixpkgs."i686-linux";
      buildBinariesForHost "i686-linux" [
        waterwheel
      ];
    packages."x86_64-darwin" = with nixpkgs."x86_64-darwin";
      buildBinariesForHost "x86_64-darwin" [
        waterwheel
      ];
    packages."x86_64-linux" = with nixpkgs."x86_64-linux";
      (buildBinariesForHost "x86_64-linux" [
        waterwheel

        pkgsStatic.waterwheel

        pkgsCross.aarch64-multiplatform.pkgsStatic.waterwheel

        pkgsCross.armv7l-hf-multiplatform.pkgsStatic.waterwheel

        pkgsCross.gnu32.pkgsStatic.waterwheel

        pkgsCross.raspberryPi.pkgsStatic.waterwheel
      ])
      // {
        waterwheel-ui = nixpkgs.x86_64-linux.waterwheel-ui;
      };
  };
}
