# tau desktop

The design of tau's desktop client: a native window, built on rune, onto one tau server. It shows what needs you (open decisions), what broke (failed runs and faulted tasks), what runs (conversations) and what happened (events), across every project, and it knows no workflow: it is complete for the bare agent and shows any workflow through the core's own shapes. It is keyboard-first, with nvim and Doom Emacs habits (modes, a leader with which-key, the `:` line, pickers rising from the bottom), and it looks calm and modern in the Ember theme: a left sidebar of places and open items, a quiet stage, one container style used rarely, small coloured marks.

The core it talks to is tau, in the `render-core` branch of the tau repository: start with `docs/protocol.md` and `docs/host.md`, then `core/src/host`. Where this folder and the core disagree, the core wins.

The design was made against `render-core` as of 2026-11-14 (`b6bc814`), and its `file:line` citations point there. Since then the core's files have moved and its docs are being reworded; its substance is unchanged. To follow a citation, map its old path:

| Cited as | Now in `render-core` |
|---|---|
| `docs/core/protocol.md`, `docs/core/host.md` | `docs/protocol.md`, `docs/host.md` |
| other `docs/core/*.md` (design notes) | removed or folded into `docs/` |
| `packages/host/src/*` | `core/src/host/*` |
| `packages/store/src/*` | `core/src/store/*` |
| `packages/kit`, `packages/tau` | under `core/src/` (`extension`, `command`) and `core/test/` |
| `packages/w1`, `packages/w2`, `packages/websearch` | `extensions/w1`, `extensions/w2`, `extensions/websearch` |

Before building, reconcile the citations with the current core once its docs settle.

## Files

| File | What it holds |
|---|---|
| `spec.md` | the frame (sidebar, stage header, bottom bar, peek, details panel, floats), the start page and every screen with the calls and streams behind it, the float shell, the main flows as keystrokes, the copy rules. **Start here.** |
| `client.md` | connection truth: call outcomes, the outbox and resends, starting a conversation (`create` then `input`), streams (gap rule, 45-second rule, backoff, `end` reasons), polling, every refusal code in plain words, pairing, device-local state, and conformance checks against the fixture |
| `keymap.toml` | every key: one declarative table, lint-clean |
| `tools/` | `keymap_answer_rule.py`, the package's own keymap rule (KL4g), and `keymap-mutants.toml`, the seeded faults each lint rule must catch |
| `reference/grammar.md` | what the keymap means: modes, contexts and focus (the sidebar, the start page), verbs and objects, the leader, resolution, the float shell, the answer dialect, printed keys, the lint rules |
| `reference/visual-language.md` | type (`title.page`), colour, coral and its one-cursor rule, the container style, radii, tints and the note leader, small marks, chrome versus content, brand use |
| `tokens.json` | every colour, size and timing, in three variants: Ember (default), Ember Soft, Ember Light |
| `brand/` | the logo files the app uses (the coral avatar and wordmarks, the Dock icon), the macOS icon sizes, and where each goes |
| `fixture.json`, `fixture.py` | test data in the protocol's exact shapes: three projects, conversations in every state, open and answered decisions, two timelines, faults, options with a secret and models, the start page's `create` and `input`, refusals (the start page's `create` ones included), a `hostile` block of content that tries to pass as chrome, device-local state (open items, places, last project and workflow). `python3 -B fixture.py` regenerates it |
| `failure-catalogue.md` | how this client could mislead its user, the defence, and the check for each |
| `evidence.md` | what was measured, looked at, or only reasoned |
| `open-questions.md` | the owner's defaults (accepted and open), copy still owed, what is unmeasured, requests to the core |
| `mockups/`, `wireframes/` | renders of the HTML prototype of the calm pass, made before the calm challenge's rulings; the text wins over any image |

## How to read it

1. `spec.md` §1–§4 for the model and the frame, then §5 for the screen you build and §6 for the floats.
2. `client.md` beside any screen that calls the server.
3. `reference/grammar.md` and `keymap.toml` for keys; `reference/visual-language.md` and `tokens.json` for how it looks.
4. Build against `fixture.json` first; `client.md` §9 lists what the client must show for each piece of it.

**Notation:** `␣` is Space (the leader), `⏎` Enter, `C-x` Ctrl, `M-x` Option, `⌘` Cmd. `#4` is conversation 4 of a project; `w1:3:design#2` is a decision ID.

## Status

- **Designed** 2026-11-14 for the tau core on `render-core` as of that date (`b6bc814`), replacing the earlier "Brief" design for Render, which assumed a core with one workflow built in.
- **First pass** prototyped in HTML (38 states) and **challenged** on 2026-11-15 (26 findings, each settled in the text; `evidence.md` §4–§5).
- **Calm pass** 2026-11-15: after the owner's review, a three-round sketch settled a calmer look and a new frame, and this text was rewritten to it: the sidebar replaces the tab bar, list tabs and per-tab filters; the start page replaces the New conversation picker; one bottom bar replaces the statusline and the echo row; every bottom float shares one shell; containers, radii, small marks, coral page titles and the note leader are new. The keymap stays lint-clean and the fixture regenerates byte-identically.
- **Prototyped, calm pass:** 47 states (49 renders) and 4 wireframes in `mockups/` and `wireframes/`, rendered after the challenge's rulings; every check family passes on every render, the checks read only what a render shows (mode, region, visible pixels), printed keys and hints resolve against `keymap.toml`, and 79 of 79 seeded faults are caught (`evidence.md` §1).
- **Challenged, calm pass** 2026-11-15: 32 findings (2 block, 14 major, 16 minor), each ruled by the coordinator and settled in the text, plus one ruling from the prototype's checks (no `global` under a float). The keymap stays lint-clean (9 of 9 seeded faults caught) and the fixture regenerates byte-identically (`evidence.md` §2, §3, §6).
- **Open:** D3, D11 and D12 (`open-questions.md` §1.2), a short list of owed copy, the requests to the core, and everything that only a rune build and a real server can measure.
- **Not built.** No rune code exists for this design yet.
