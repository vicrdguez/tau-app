{
  pkgs,
  release,
  root,
}:
let
  inherit (release) craneLib common cargoArtifacts;
in
{
  package = release.package;
  fmt = craneLib.cargoFmt {
    inherit (common) src pname version;
    cargoExtraArgs = "--all";
  };
  clippy = craneLib.cargoClippy (
    common
    // {
      inherit cargoArtifacts;
      cargoExtraArgs = "--locked --workspace";
      cargoClippyExtraArgs = "--all-targets -- -D warnings";
    }
  );
  test = craneLib.cargoTest (
    common
    // {
      inherit cargoArtifacts;
      cargoExtraArgs = "--locked --workspace";
    }
  );
  nixfmt =
    pkgs.runCommand "tau-nixfmt"
      {
        nativeBuildInputs = [ pkgs.nixfmt ];
        src = pkgs.lib.fileset.toSource {
          inherit root;
          fileset = pkgs.lib.fileset.unions [
            (root + /flake.nix)
            (root + /nix)
          ];
        };
      }
      ''
        cd "$src"
        nixfmt --check flake.nix nix/*.nix
        touch "$out"
      '';
}
// pkgs.lib.optionalAttrs pkgs.stdenv.isDarwin {
  app = release.bundle;
}
