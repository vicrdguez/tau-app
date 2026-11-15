# Open questions

## 1. Owner defaults

Each is in force as written in `spec.md` and `client.md`. Each lists its alternative.

### 1.1 Accepted (2026-11-15)

| # | Question | Default | Alternative |
|---|---|---|---|
| D1 | The core has no stream of conversation states. | The client polls `conversations` every 10 s per project while the window is focused (every page), and `faults` every 5 min. | Add a `conversations` watch to the host (a core change, through the admission rule). |
| D2 | Conversations have no name or creation time. | Label each by its first input line; cache a found label for good; order by ID. A workflow's role conversations carry the workflow's instruction text as label. | Add an optional label (and a time) to the core. |
| D4 | Faults can't be cleared. | Hide per device (`x`), and hide automatically after this device sends the answer again. | A stored "seen" list on the server (the next step `docs/core/host.md` names). |
| D5 | Resending after an unknown outcome. | Inputs, `continue`, option and model changes, `bind` and `addExtensions` resend by themselves on one schedule (at the next connection, then on the stream backoff, at most 6 tries; never after 10 minutes or a restart). Answers never resend by themselves. | Resend answers automatically too, accepting that a resend after a fault restarts the work. |
| D6 | May a burst such as `␣ad2y` at typing speed send an answer? (challenge TD-08) | No: `y` counts only after the readback has been on screen for 400 ms; an earlier `y` is discarded and echoes `read it first`. | Count `y` as soon as the readback is drawn, accepting a fifth of a second on screen. |
| D7 | Where can an answer whose reply was lost be sent again? (TD-07) | Only with `␣ar`, in that decision's view, through the readback. `␣rt` never sends an answer; the outbox entry says `open the decision to send it again`. The client first tries to settle it from the inbox and `decision`. | Also from the outbox with `␣rt`, on any screen, without a new readback. |
| D8 | A failed run stays failed until a new run. (TD-13) | Hide it per device with `x`; it shows again when it fails again. Home's Failed holds failed runs and faulted tasks; the `×` tag counts exactly what it shows. | Leave it in Failed and the `×` tag until a new run is spent on it. |
| D9 | May a project scope ever filter Home? (TD-14) | No. Home always shows every project; rows name their project. Conversations, Timeline and Faults each keep one filter of their own. `␣cn`, `␣pa` and `␣po` share one rule: the project the current view names (a Project view, a conversation, a decision, or the sidebar item under the cursor), else the last project used on this device (calm rulings §2, refined by the calm challenge TC-17). | A global scope that filters Home, the `◆` tag and every place. |
| D10 | The v1 cut list. (TD-17) | Cut: Mermaid diagrams (a `mermaid` fence is a code block), the measure and prose-face toggles (`␣tw`, `␣tp`), `␣'`, `␣:`, `␣/` (keep `/`), `␣rr` (keep `␣rc`), and the second Home key `␣q` (keep `␣oh`; `1gt` went with the tabs in the calm pass). | Keep any of them, each with a defined behaviour, a fixture and a state. |

### 1.2 Open for the owner

| # | Question | Default | Alternative |
|---|---|---|---|
| D3 | Inputs don't say who sent them. | Every `pi.user` entry is `input`, never "you". The owner is checking how the core could record the sender. | A pi-durable or host change that records the sender. |
| D11 | What does the app open on? (calm rulings §2) | The start page. The sidebar's Home row carries the `◆`, `×` and `?` tags whenever they are above 0, and the bottom bar does while the sidebar is not drawn, so nothing that needs you is hidden; before the first frame the server line says `○ connecting` (`spec.md` §4.8). **Cost:** Home is not "no keys" at launch: it is `Esc ␣oh` away, and the since-line is first seen when you go there. | Open on Home, as before; `␣cn` still opens the start page. Home then needs the same "nothing received yet" state at launch. |
| D12 | The core records no tool call's start or end; how does a tool row show time? (calm challenge TC-16) | A finished row shows its outcome word and no duration. A running row counts up from the `timestamp` of the `pi.assistant` message that made the call; the error is the model's writing time, small against a call long enough to watch. A fold never shows a figure (`spec.md` §5.5). | The previous default: a finished row shows its `pi.tool-result` `timestamp` minus its message's; a fold shows the last result minus the first message's timestamp, never a sum. It includes the model's writing time, and calls of one message overlap. |

## 2. Copy still owed

| Where | What |
|---|---|
| Decision, withdrawn | the notice sentence (now `Withdrawn`) |
| An open item whose object is gone | the exact line (now `gone from the server`) |
| Empty Conversations, Timeline, Faults and Projects places, and a place whose filter matches nothing | one sentence each |
| The sidebar with no open items | whether `OPEN` shows with a sentence or is left out (now left out) |
| The start page | the line when `create` was refused for a reason other than options (now the refusal's copy of `client.md` §6) |
| Bind float | errors for a bad name and a relative path (checked on the device before `bind`; zero extensions is valid) |
| Server float | everything beyond the devices sentence |
| `␣hk` | the cheat sheet's layout (generated from `keymap.toml`) |
| Dock badge | whether it shows only `◆` or also `×` |

## 3. Unmeasured; first implementation tests

- **Does `stop` leave a `runFailure`?** If it does, `conversations` reports a stopped conversation `failed`; the client must read the reason and show it as stopped (`spec.md` §5.5). Test: stop a run against a real `tau`, read `conversations` and `pi.live`. Settle the word the reason uses (`aborted` or another).
- The shape of a `then` task blocked by `missing_task` (`spec.md` §5.6 shows it as `blocked`); reopen a store with an extension removed and read `decision`.
- Whether an inbox entry that changes ID arrives as one `s` op on the subject or as field ops; the client applies ops generically either way.
- **Tool durations** (D12): whether pi's `pi.assistant` `timestamp` marks the start of the message or its end, which decides how much model time a running count holds, and how honest D12's alternative would be. The `pi.tool-result` shape itself is inferred (`fixture.json` `_about`).
- The rune client itself: no screen of this design has been built in rune. In particular: the float shell's radius with its upward shadow (`box_shadow` at a negative y); whether rune's in-text pill takes `radius.pill` or keeps its fixed metrics; the note leader as a drawn line; the drawn `␣` and `⏎` glyphs at 9 px; the sidebar's scrolling `OPEN` section; the sidebar drawn over the stage under 1000 px; a read-only composer while `create` is in flight, and suppressing auto-repeat on `⌘⏎`.
- A real `tau`: every server behaviour is from `docs/core` and `packages/host/src`.
- Key timing for the readback's 400 ms arm and the discard of early keys; IME with `jk` and `M-⏎`; `M-p` and `M-w` on keyboard layouts where Option-p or Option-w types a letter the user needs.
- Long transcripts: the virtual list, the 2,000-line cap. One `open` stream follows the shown conversation and one the peek: the cost of a reopen with `after` on each `]b` and `⏎` (one round trip).
- The cost of polling `conversations` (every page, every 10 s, each conversation's `pi.live`) and `faults` on a large store.
- Mapping Visual selections in rendered Markdown to exact source ranges.
- Pointer behaviour beyond "a click never confirms".
- Human trials: answering a design with notes unaided; whether `input` instead of "you" confuses; whether the since-line's focus rule matches how the owner leaves; whether `C-h` and `]b` are found without the cheat sheet; whether opening on the start page (D11) hides what needs you.

## 4. Requests to the core

- **A distinct refusal code for a refused `Host` or `Origin`.** Today it is `unauthorized`, like a bad token (`packages/host/src/http.ts:82`), so the client reads the message, which is fragile (`client.md` §6.1).
- **Who sent an input** (D3), which the owner is checking.
- **A tool call's start and end,** if the owner takes D12's alternative: from pi-durable, through the host's protocol rule for its shapes.
- D1 and D2 above, if the owner chooses their alternatives.

## 5. Later, not for v1

- **Extension-specific UI:** renderers by entry kind, a per-project panel on Home, commands under `␣m`, maybe a sidebar section (`spec.md` §9). The owner has his own plans for malleable UI.
- **Mermaid diagrams** (D10): a renderer, its fit rule, a full view with pan and zoom, with a fixture and a state.
- Showing tokens and cost once pi-durable pins `pi.usage`.
- Splits: the keys stay reserved (`reference/grammar.md` §12); `C-h` and `C-l` already move between the sidebar and the stage.
- Reordering open items by hand.
