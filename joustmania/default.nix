{ stdenv
, makeWrapper
, fetchFromGitHub
, python3
, psmoveapi
, alsaLib
}:

let
  src = fetchFromGitHub {
    owner = "adangert";
    repo = "JoustMania";
    rev = "393fefcf7675a1b8b396a43450b9cbf9dc23a9e9";
    sha256 = "0my4r1la9vsl4d1c3x5ljp9a5ph3wpbw7k4lhhrhpg4vs06aa6w6";
  };

  pyalsaaudio = python3.pkgs.buildPythonPackage rec {
    pname = "pyalsaaudio";
    version = "0.9.0";

    src = python3.pkgs.fetchPypi {
      inherit pname version;
      sha256 = "0s8yw1h601cw2sqgxm7sjh7li8l3v5l380xm8wq2mbf86v3nk81w";
    };

    doCheck = false;

    buildInputs = [
      alsaLib
    ];
  };

  pythonEnv = python3.withPackages (ps: [
    pythonEnv
    (python3.pkgs.toPythonModule psmoveapi)
    ps.psutil
    ps.numpy
    ps.scipy
    ps.pygame
    pyalsaaudio
    ps.pydub
    ps.flask
    ps.wtforms
    ps.dbus-python
  ]);
in
  stdenv.mkDerivation {
    name = "joustmania";

    inherit src;

    nativeBuildInputs = [ makeWrapper ];

    buildPhase = "";
    installPhase = ''
      mkdir -p $out/bin
      makeWrapper '${pythonEnv.interpreter}' $out/bin/piparty \
        --add-flags ${src}/piparty.py \
        --prefix PYTHONPATH : '${src}'
    '';
  }

