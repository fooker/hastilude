{ pkgs ? import <nixpkgs> {} }:

let
  psmoveapi = pkgs.callPackage ./psmoveapi {};

  py = pkgs.python3.withPackages (p: with p; [
    psmoveapi
  ]);

in
  pkgs.mkShell {
    buildInputs = [
      py
      psmoveapi
      pkgs.python3Packages.bpython
    ];
  }


