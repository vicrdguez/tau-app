{
  description = "tau app: the native desktop client of a tau server";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    # The tau server the end-to-end test runs against (nix run .#e2e).
    tau.url = "github:vicrdguez/tau/render-core";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      crane,
      tau,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      root = ./.;
      supportedSystems = [
        "aarch64-darwin"
        "x86_64-linux"
        "aarch64-linux"
      ];
      perSystem = lib.genAttrs supportedSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          assets = import ./nix/assets.nix { inherit pkgs root; };
          release = import ./nix/package.nix {
            inherit
              pkgs
              crane
              toolchain
              assets
              root
              ;
          };
          launchers = {
            default = pkgs.writeShellScriptBin "tau" ''
              exec ${release.package}/bin/tau-app "$@"
            '';
            demo = pkgs.writeShellScriptBin "demo" ''
              exec ${release.package}/bin/tau-app --demo "$@"
            '';
          };
          commands = import ./nix/commands.nix {
            inherit
              pkgs
              toolchain
              assets
              ;
            inherit (release.common) buildInputs;
            inherit (release) runtimeLibraries;
            tau = tau.packages.${system};
          };
        in
        {
          inherit
            pkgs
            toolchain
            assets
            commands
            release
            launchers
            ;
        }
      );
      app = program: {
        type = "app";
        inherit program;
      };
    in
    {
      apps = lib.mapAttrs (
        _: s:
        (lib.mapAttrs (name: command: app "${command}/bin/${name}") s.commands.apps)
        // {
          default = app "${s.launchers.default}/bin/tau";
          demo = app "${s.launchers.demo}/bin/demo";
        }
      ) perSystem;
      devShells = lib.mapAttrs (_: s: { default = s.commands.shell; }) perSystem;
      packages = lib.mapAttrs (
        _: s:
        {
          inherit (s) assets;
          default = s.release.package;
        }
        // lib.optionalAttrs s.pkgs.stdenv.isDarwin {
          app = s.release.bundle;
        }
      ) perSystem;
      checks = lib.mapAttrs (
        _: s:
        import ./nix/checks.nix {
          inherit (s) pkgs release;
          inherit root;
        }
      ) perSystem;
      formatter = lib.mapAttrs (_: s: s.pkgs.nixfmt) perSystem;
    };
}
