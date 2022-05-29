{ pkgs ? import <nixpkgs> {} }:

let
  mozillaOverlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  rust = pkgs.extend mozillaOverlay;
  rustChannel = rust.rustChannelOf { date = "2022-04-23"; channel = "nightly"; };

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
  ];

  RUST_BACKTRACE = 1;
  RUST_SRC = "${rustChannel.rust-src}/lib/rustlib/src/rust";
}


