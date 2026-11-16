# tau app

The desktop app for tau, a native rune window onto one tau server. This repository holds the Rust app, its client library, the design, and the tools that check them. The tau core is in another repository.

## What is here

| Folder | What it holds |
|---|---|
| [`crates/tau-app/`](crates/tau-app/README.md) | The native UI, screens, keymap, theme, and headless scenes. |
| [`crates/tau-client/`](crates/tau-client/README.md) | Pairing, HTTP, streams, persistence, and the read-only server model. |
| [`nix/`](nix/README.md) | Release packaging, assets, checks, and checkout commands. |
| [`design/`](design/README.md) | The design package: the spec, the connection rules, the keymap and its grammar, the visual language and tokens, test data in the protocol's shapes, the failure catalogue, open questions, evidence, mockups and wireframes. **Start here.** |
| [`brand/`](brand/README.md) | The tau logo: mark, wordmark and app icons, as SVG and PNG, in each colour version. The client uses a few of these; `design/brand/` lists which and where. |
| [`tools/proto/`](tools/proto/README.md) | An HTML prototype that renders every state of the design and checks it against the design's rules, plus the keymap lint. It reproduces the design's mockups byte-identically. |

## The core

The app talks to the tau core over HTTP and server-sent events. The core lives in the `render-core` branch of the tau repository; read `docs/protocol.md` and `docs/host.md` there first. Where the design and the core disagree, the core wins.

The design was made against the core as of 2026-11-14. The core's files have moved since and its docs are being reworded, but its substance is unchanged. `design/README.md` maps the old paths the design cites to the current ones. `design/core-reconciliation.md` compares the design's protocol claims with the current core; the client follows the core where they differ.

## Status

- **Design:** finished, 2026-11-15. It went through two passes, each challenged once, with every finding settled. The prototype passes every check on all 47 states.
- **Open:** three owner defaults (`design/open-questions.md`): D3 (an input shows as `input`, not "you"), D11 (the app opens on the start page) and D12 (no duration on a finished tool row).
- The native app and client library are implemented. Rune renders the design's fixture scenes and the UI talks to a real tau server.
- The real-server UI gate pairs through Connect, binds a project, starts a bare conversation on `faux/echo`, and checks the rendered echo reply.
- Nix provides the release package, macOS bundle, isolated checks, and the cumulative verification command below. Linux outputs are defined but have not been built or run on Linux in this work.
- Visual differences and pinned Rune renderer limits are recorded in [`crates/tau-app/README.md`](crates/tau-app/README.md#pinned-rune-kit-limits).

## Build and run

Nix with flakes enabled is required. The flake pins dependencies and reads the Rust version from `rust-toolchain.toml`.
Run checkout commands inside this repository. Use `--` before arguments passed through `nix run`.

| Command | Result |
|---|---|
| `nix build` | Release binary at `result/bin/tau-app`, with fonts and icons in its Nix closure. No dev shell is needed to run it. |
| `nix build .#app` | On macOS, `result/Applications/tau.app` with the executable, Info.plist, and coral brand icon. The bundle depends on its Nix store closure and is not a standalone distribution. |
| `nix run` | Launch the packaged app against your configured server, or show Connect on first use. |
| `nix run .#demo` | Launch the packaged app on `design/fixture.json`, without using your server. |
| `nix run .#dev` | Build and launch the checkout in debug mode. Add `-- --demo` for fixture data. |
| `nix run .#run` | Build and launch the checkout in release mode. |
| `nix develop` | Shell with the pinned Rust toolchain, native build inputs, nixfmt, and assets. |
| `nix flake check` | Isolated builds of tracked source. Checks the package, macOS bundle, rustfmt, clippy with `-D warnings`, workspace tests, and nixfmt. Real-server tests do not run here. |
| `nix run .#fmt` | Format Rust and Nix source in the checkout. |
| `nix run .#lint` | Check rustfmt, clippy with `-D warnings`, and nixfmt in the checkout. |
| `nix run .#test` | Run workspace tests. Add a Cargo filter, for example `-- content::`. |
| `nix run .#e2e` | Test the client against the pinned real server, `faux/echo`, and w2 in temporary state. |
| `nix run .#e2e-ui` | Drive the real UI with Rune scripts against a temporary real server. Save Connect, Home, and conversation PNGs and element trees as `target/snaps/e2e-ui-*`. Fail unless the tree shows the assistant echo. |
| `nix run .#snap` | Render every named fixture scene into `target/snaps/`. Add `-- <scene> [WxH]` for one scene. |
| `nix run .#script -- <name> <at> [scene]` | Play `tools/scripts/<name>.json` and save the frame at `<at>` seconds. |
| `nix run .#compare -- <scene> [mockup]` | Compare a rendered scene with its design mockup. Write side-by-side and difference images into `target/snaps/compare/`. |
| `nix run .#verify` | Run lint, workspace tests, client e2e, UI e2e, and all fixture snapshots in order. Stop on the first failure. |

Stage new source files before `nix build` or `nix flake check`. Flakes omit untracked files.
The checkout commands use the working tree and are intended for the edit loop.
Headless renders still require a GPU or a working software graphics driver.

The app defaults to `~/.config/tau` for configuration and per-server state, and to the macOS Keychain for tokens.
`TAU_CLIENT_DIR` overrides the configuration and state root. `TAU_TOKEN_DIR` explicitly selects private token files instead of the Keychain.
The UI gate sets both to temporary directories and removes them afterward. It never needs your server or provider credentials.

Normal headless runs use fixture data. `--live` selects the real client for headless tooling, and cannot be combined with `--demo`.
The UI gate uses Rune's existing `--realtime` flag so network threads and server replies can progress between scripted inputs.
`TAU_ASSETS` optionally overrides the asset path baked into the binary.
