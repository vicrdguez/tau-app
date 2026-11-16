# Checkout commands: each runs from the repository root, whatever directory it is called from,
# with the pinned toolchain, the platform's C toolchain and SDK, and TAU_ASSETS.
{
  pkgs,
  toolchain,
  assets,
  tau,
  buildInputs,
  runtimeLibraries,
}:
let
  inherit (pkgs) lib;
  shell = pkgs.mkShell (
    {
      packages = [
        toolchain
        pkgs.nixfmt
      ];
      inherit buildInputs;
      TAU_ASSETS = assets;
    }
    // lib.optionalAttrs pkgs.stdenv.isLinux {
      LD_LIBRARY_PATH = lib.makeLibraryPath runtimeLibraries;
    }
  );
  enterRoot = ''
    while [ ! -f flake.nix ] || [ ! -f Cargo.toml ]; do
      if [ "$PWD" = / ]; then
        printf '%s\n' 'Run this command inside the tau-app repository.' >&2
        exit 1
      fi
      cd ..
    done
  '';
  mkCommand =
    name: text:
    pkgs.writeShellApplication {
      inherit name;
      runtimeInputs = [
        toolchain
        pkgs.stdenv.cc
        pkgs.nixfmt
        pkgs.findutils
      ];
      text = ''
        ${enterRoot}
        export TAU_ASSETS=${assets}
        ${lib.optionalString pkgs.stdenv.isLinux ''
          export LD_LIBRARY_PATH="${lib.makeLibraryPath runtimeLibraries}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
        ''}
        ${lib.optionalString pkgs.stdenv.isDarwin ''
          export SDKROOT=${pkgs.apple-sdk.sdkroot}
          export DEVELOPER_DIR=${pkgs.apple-sdk}
          export LIBRARY_PATH="${pkgs.libiconv}/lib''${LIBRARY_PATH:+:$LIBRARY_PATH}"
        ''}
        ${text}
      '';
    };
in
{
  inherit shell;
  apps = lib.mapAttrs mkCommand {
    test = ''exec cargo test --workspace "$@"'';
    verify = ''
      for command in lint test e2e e2e-ui snap; do
        printf '\n==> %s\n' "$command"
        start=$SECONDS
        ${pkgs.nix}/bin/nix run ".#$command"
        printf '==> %s passed in %s s\n' "$command" "$((SECONDS - start))"
      done
    '';
    run = ''exec cargo run -p tau-app --release -- "$@"'';
    # A real tau server with the fake model faux/echo and the w2 extension, on a free port in a short temp dir
    # (its admin socket path must fit in 104 bytes), then the end-to-end test against it.
    e2e = ''
      export PATH=${
        lib.makeBinPath [
          pkgs.coreutils
          pkgs.curl
          pkgs.gnugrep
          pkgs.gnused
        ]
      }:$PATH
      state=$(mktemp -d /tmp/tau-e2e.XXXXXX)
      cleanup() {
        status=$?
        kill "''${TAU_PID:-}" 2>/dev/null || true
        if [ "$status" -ne 0 ] && [ -f "$state/server.log" ]; then
          printf '%s\n' '--- server log' >&2
          cat "$state/server.log" >&2
        fi
        rm -rf "$state"
      }
      trap cleanup EXIT
      export TAU_FAUX_SCRIPT=$state/faux-script.json
      server_env=$(TAU_STATE=$state TAU_BIN=${tau.tau}/bin/tau TAU_FAUX=1 \
        TAU_EXTENSIONS="w2=${tau.extension-w2}" tools/e2e/run-server.sh 0 script)
      eval "$server_env"
      export TAU_BIN TAU_STATE TAU_PID TAU_URL
      cargo test -p tau-client --test e2e -- --nocapture "$@"
    '';
    e2e-ui = ''
      export PATH=${
        lib.makeBinPath [
          pkgs.coreutils
          pkgs.curl
          pkgs.gnugrep
          pkgs.gnused
          pkgs.python3
        ]
      }:$PATH
      cargo build -p tau-app --quiet
      state=$(mktemp -d /tmp/tau-ui.XXXXXX)
      cleanup() {
        status=$?
        kill "''${TAU_PID:-}" 2>/dev/null || true
        if [ "$status" -ne 0 ] && [ -f "$state/server.log" ]; then
          cat "$state/server.log" >&2
        fi
        rm -rf "$state"
      }
      trap cleanup EXIT
      server_env=$(TAU_STATE=$state TAU_BIN=${tau.tau}/bin/tau TAU_FAUX=1 \
        TAU_EXTENSIONS="w2=${tau.extension-w2}" tools/e2e/run-server.sh 0 observer)
      eval "$server_env"
      code=$(HOME="$state" "$TAU_BIN" device pair ui | sed -n 's/^Pairing code: //p')
      export TAU_CLIENT_DIR="$state/client" TAU_TOKEN_DIR="$state/tokens"
      python tools/e2e/ui.py "$PWD/target/debug/tau-app" "$TAU_URL" "$code" "$state"
    '';
    dev = ''exec cargo run -p tau-app -- "$@"'';
    fmt = ''
      cargo fmt --all
      nixfmt flake.nix nix/*.nix
    '';
    lint = ''
      cargo fmt --all --check
      cargo clippy --workspace --all-targets -- -D warnings
      nixfmt --check flake.nix nix/*.nix
    '';
    # `compare <scene> [mockup]`: the render beside its mockup, their difference, and the share of
    # pixels that differ by more than 6 %, into target/snaps/compare.
    compare = ''
      scene="$1"
      mock="design/mockups/''${2:-$1}.png"
      snap="target/snaps/$scene.png"
      out=target/snaps/compare
      mkdir -p "$out"
      m=${pkgs.imagemagick}/bin/magick
      "$m" "$mock" "$snap" +append "$out/$scene.png"
      "$m" "$mock" "$snap" -compose difference -composite -evaluate multiply 4 "$out/$scene-diff.png"
      differ=$("$m" compare -metric AE -fuzz 6% "$mock" "$snap" null: 2>&1 || true)
      differ=''${differ%% *}
      total=$("$m" identify -format '%w*%h' "$mock")
      printf '%s: %s pixels differ, %s %%\n' "$scene" "$differ" "$(echo "scale=3; 100 * $differ / ($total)" | ${pkgs.bc}/bin/bc)"
    '';
    # `script <name> <at> [scene]`: plays tools/scripts/<name>.json (keys and clicks, as a window
    # gets them) from a scene, and renders the frame at <at> seconds into target/snaps/script-<name>.png.
    script = ''
      export TZ=UTC
      cargo build -p tau-app --quiet
      mkdir -p target/snaps
      target/debug/tau-app --script "tools/scripts/$1.json" --scene "''${3:-home-connecting}" \
        --snap "target/snaps/script-$1.png" --at "$2" --size 1512x982 --scale 1
    '';
    # Headless renders into target/snaps at 1x, the mockups' scale, in UTC, the fixture's clock: `snap` renders every scene,
    # `snap <scene> [WxH]` one. A scene named `…-<width>` renders at that width, 982 tall.
    snap = ''
      export TZ=UTC
      cargo build -p tau-app --quiet
      bin=target/debug/tau-app
      mkdir -p target/snaps
      render() {
        "$bin" --snap "target/snaps/$1.png" --scene "$1" --size "$2" --scale 1
      }
      size_of() {
        case "$1" in
          *-[0-9][0-9][0-9]|*-[0-9][0-9][0-9][0-9]) printf '%sx982' "''${1##*-}" ;;
          *) printf '1512x982' ;;
        esac
      }
      if [ $# -gt 0 ]; then
        render "$1" "''${2:-$(size_of "$1")}"
      else
        "$bin" --list-scenes | while read -r scene; do
          render "$scene" "$(size_of "$scene")"
        done
      fi
    '';
  };
}
