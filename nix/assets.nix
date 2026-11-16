# The files the app reads at run time: the two Iosevka families the design names (v34.9.1, SIL OFL 1.1),
{ pkgs, root }:
let
  iosevka = pkgs.fetchzip {
    url = "https://github.com/be5invis/Iosevka/releases/download/v34.9.1/PkgTTF-Iosevka-34.9.1.zip";
    hash = "sha256-prLvuNpU8BomZlyMHTyDU35rfWy20+oD8e7YwdJdciQ=";
    stripRoot = false;
  };
  etoile = pkgs.fetchzip {
    url = "https://github.com/be5invis/Iosevka/releases/download/v34.9.1/PkgTTF-IosevkaEtoile-34.9.1.zip";
    hash = "sha256-Q8yAcqciqary1uubOU0o0vsmne8rCAW1aaqqxxB2wVI=";
    stripRoot = false;
  };
  carbon = pkgs.fetchzip {
    url = "https://registry.npmjs.org/@carbon/icons/-/icons-11.90.0.tgz";
    hash = "sha256-D2Bm9F3SiuFRS3f5js9k0ZW9gdxKTJi7Umj0OOpPNwE=";
  };
  faces = [
    {
      dir = etoile;
      names = [
        "IosevkaEtoile-Regular"
        "IosevkaEtoile-Italic"
        "IosevkaEtoile-SemiBold"
        "IosevkaEtoile-SemiBoldItalic"
        "IosevkaEtoile-Bold"
        "IosevkaEtoile-BoldItalic"
      ];
    }
    {
      dir = iosevka;
      names = [
        "Iosevka-Regular"
        "Iosevka-Italic"
        "Iosevka-Medium"
        "Iosevka-SemiBold"
        "Iosevka-Bold"
      ];
    }
  ];
  icons = [
    "add"
    "arrow--down"
    "caret--right"
    "chat"
    "checkbox"
    "checkbox--checked"
    "checkmark"
    "chevron--down"
    "chevron--right"
    "edit"
    "folder"
    "help"
    "home"
    "hourglass"
    "search"
    "settings"
    "side-panel--close"
    "side-panel--open"
    "subtract"
    "time"
    "warning--alt"
  ];
  copyFaces = pkgs.lib.concatMapStrings (
    f: pkgs.lib.concatMapStrings (n: "cp ${f.dir}/${n}.ttf $out/fonts/\n") f.names
  ) faces;
in
pkgs.runCommand "tau-app-assets" { } ''
  mkdir -p $out/fonts $out/icons
  ${copyFaces}
  cp ${root + "/tools/proto/vendor/fonts/LICENSE.md"} $out/fonts/LICENSE.md
  ${pkgs.lib.concatMapStrings (n: "cp ${carbon}/svg/32/${n}.svg $out/icons/\n") icons}
  cp ${carbon}/LICENSE $out/icons/LICENSE
''
