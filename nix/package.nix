{
  pkgs,
  crane,
  toolchain,
  assets,
  root,
}:
let
  inherit (pkgs) lib;
  craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
  src = lib.fileset.toSource {
    inherit root;
    fileset = lib.fileset.unions [
      (root + /Cargo.toml)
      (root + /Cargo.lock)
      (root + /crates)
      (root + /design/fixture.json)
      (root + /design/tokens.json)
      (root + /design/keymap.toml)
      (root + /design/brand)
    ];
  };
  runtimeLibraries = lib.optionals pkgs.stdenv.isLinux [
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.vulkan-loader
    pkgs.xorg.libX11
    pkgs.xorg.libXcursor
    pkgs.xorg.libXi
    pkgs.xorg.libXrandr
  ];
  common = {
    inherit src;
    pname = "tau-app";
    version = "0.1.0";
    strictDeps = true;
    TAU_ASSETS = assets;
    cargoVendorDir = craneLib.vendorCargoDeps {
      inherit src;
      overrideVendorGitCheckout =
        _: drv:
        drv.overrideAttrs (old: {
          # Rune checks in registry archives whose Cargo.toml.orig cannot be repackaged.
          postPatch = (old.postPatch or "") + ''
            find . -name Cargo.toml.orig -delete
          '';
        });
    };
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs =
      lib.optionals pkgs.stdenv.isDarwin [
        pkgs.apple-sdk
        pkgs.libiconv
      ]
      ++ lib.optionals pkgs.stdenv.isLinux [
        pkgs.wayland
        pkgs.libxkbcommon
      ];
  };
  cargoArtifacts = craneLib.buildDepsOnly common;
  package = craneLib.buildPackage (
    common
    // {
      inherit cargoArtifacts;
      cargoExtraArgs = "--locked -p tau-app";
      doCheck = false;
      nativeBuildInputs =
        common.nativeBuildInputs ++ lib.optionals pkgs.stdenv.isLinux [ pkgs.makeWrapper ];
      postFixup = lib.optionalString pkgs.stdenv.isLinux ''
        wrapProgram "$out/bin/tau-app" --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibraries}
      '';
      postInstall = ''
        mkdir -p "$out/share/tau"
        ln -s ${assets}/fonts "$out/share/tau/fonts"
        ln -s ${assets}/icons "$out/share/tau/icons"
      '';
      meta.mainProgram = "tau-app";
    }
  );
  bundle = pkgs.runCommand "tau-app-bundle" { } ''
    app="$out/Applications/tau.app/Contents"
    mkdir -p "$app/MacOS" "$app/Resources"
    cp ${package}/bin/tau-app "$app/MacOS/tau-app"
    cp ${root}/brand/png/tau-icon-coral-1024.png "$app/Resources/tau.png"
    cp ${./Info.plist} "$app/Info.plist"
  '';
in
{
  inherit
    craneLib
    common
    cargoArtifacts
    package
    bundle
    runtimeLibraries
    ;
}
