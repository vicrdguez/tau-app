# tau app

The desktop app for tau: a native window, built on rune, onto one tau server. This repository holds the app's design, its brand and the tools that check the design. The app's code will live here too. The tau core is in another repository.

## What is here

| Folder | What it holds |
|---|---|
| [`design/`](design/README.md) | The design package: the spec, the connection rules, the keymap and its grammar, the visual language and tokens, test data in the protocol's shapes, the failure catalogue, open questions, evidence, mockups and wireframes. **Start here.** |
| [`brand/`](brand/README.md) | The tau logo: mark, wordmark and app icons, as SVG and PNG, in each colour version. The client uses a few of these; `design/brand/` lists which and where. |
| [`tools/proto/`](tools/proto/README.md) | An HTML prototype that renders every state of the design and checks it against the design's rules, plus the keymap lint. It reproduces the design's mockups byte-identically. |

## The core

The app talks to the tau core over HTTP and server-sent events. The core lives in the `render-core` branch of the tau repository; read `docs/protocol.md` and `docs/host.md` there first. Where the design and the core disagree, the core wins.

The design was made against the core as of 2026-11-14. The core's files have moved since and its docs are being reworded, but its substance is unchanged. `design/README.md` maps the old paths the design cites to the current ones. Reconcile the citations with the current core before building.

## Status

- **Design:** finished, 2026-11-15. It went through two passes, each challenged once, with every finding settled. The prototype passes every check on all 47 states.
- **Open:** three owner defaults (`design/open-questions.md`): D3 (an input shows as `input`, not "you"), D11 (the app opens on the start page) and D12 (no duration on a finished tool row).
- **Not measured:** no rune build and no real tau server have run against the design.
- **Not built:** no app code exists yet.
