{ pkgs ? import <nixpkgs> {} }:

with pkgs;

mkShell {
  buildInputs = [
    rust-bin.nightly.latest.default
    nodejs
  ];
}
