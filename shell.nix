{ pkgs ? import <nixpkgs> {} }:

let
  mozillaOverlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  rust = pkgs.extend mozillaOverlay;
  rustChannel = rust.rustChannelOf { date = "2021-11-23"; channel = "nightly"; };

  psmoveapi = pkgs.callPackage ./psmoveapi {};

  pythonEnv = pkgs.python3.withPackages (p: with p; [
    psmoveapi
  ]);

in pkgs.mkShell {
  buildInputs = [
    rustChannel.rust
    rustChannel.rust-src
    rustChannel.cargo
    pkgs.pkg-config
    pkgs.openssl
    pkgs.udev
    pkgs.dbus
    pkgs.alsaLib
    pkgs.llvmPackages.llvm
    pkgs.llvmPackages.clang
    psmoveapi
  ];

  LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib";

  RUST_BACKTRACE = 1;
  RUST_SRC = "${rustChannel.rust-src}/lib/rustlib/src/rust";
}


