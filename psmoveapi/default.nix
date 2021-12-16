{ stdenv
, fetchFromGitHub
, pkg-config
, cmake
, swig
, dbus
, udev
, bluez
, libusb
, libusb-compat-0_1
, python3
, tree
}:

stdenv.mkDerivation rec {
  pname = "psmoveapi";
  version = "4.0.12";

  src = fetchFromGitHub {
    owner = "thp";
    repo = pname;
    rev = version;
    sha256 = "0p328hkmbzlqwqh9pdy6vrig12y564q5yh02sj5x59rk2kl2ka65";
    fetchSubmodules = true;
  };

  nativeBuildInputs = [
    pkg-config
    cmake
    swig
  ];

  buildInputs = [
    dbus
    udev
    bluez
    libusb
    libusb-compat-0_1
    python3
  ];

  cmakeFlags = "-DPSMOVE_BUILD_JAVA_BINDINGS=off -DPSMOVE_BUILD_CSHARP_BINDINGS=off -DPSMOVE_BUILD_PROCESSING_BINDINGS=off -DCMAKE_SWIG_OUTDIR=./bindings";
  
  patches = [
    ./fix-build.patch
  ];

  postInstall = ''
    mkdir -p $out/${python3.sitePackages}
    cp _psmove.so bindings/psmove.py $out/${python3.sitePackages}
  '';
}

