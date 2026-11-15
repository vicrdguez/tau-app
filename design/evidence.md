# Evidence

What backs this design, labelled **measured** (a check ran and gave a number), **looked** (a person looked at a render), or **reasoned** (argued from the core's documents and source, not run). Dates are 2026-11-14 and 2026-11-15.

The design went through two passes. The first built the screens and was challenged once (§4, §5). The **calm pass** replaced the frame and the look after the owner's review: a sidebar instead of tabs, a stage header, one bottom bar, the float shell, the start page, small marks and new tokens. §1 to §3 describe the calm pass. §4 and §5 are the first pass's record; their image names refer to that pass's renders, which are kept in the owner's design workbench, not in this repository.

## 1. The HTML prototype

An HTML prototype built from `spec.md`, `fixture.json`, `keymap.toml` and `tokens.json`, rendered headlessly in Chromium at 1512 × 982 (and 1300 × 982 and 800 × 982), in Ember dark and Ember Light. 47 states, 49 renders (the 800 px state is a sheet of three frames), plus 4 wireframes, in `mockups/` and `wireframes/`; `../tools/proto/states.md` lists each state and what it shows. The prototype is `../tools/proto/` (`npm run build` takes about 56 s, `npm run wire` about 15 s, `npm run mutants` about 50 s; its README says how to run it), and it reproduces `mockups/` and `wireframes/` byte-identically. Its colours and sizes are generated from `tokens.json` at build time, all three variants, so CONTRAST measures the shipped tokens. It is a model of the layout and the rules, not the rune client. Ember Soft was not rendered: no state asks for it.

**How the checks read a render.** The checks take nothing a view says about itself. The mode comes from the badge the bottom bar shows; the region with the keys from the open float, the focused container, or whichever of the sidebar and the stage shows the visible coral cursor; the key context from what that region draws; and a binding counts only where its command applies. A mark counts only if its pixels are visible (hit-tested, so a bar clipped by its column or covered by a float does not count).

**Measured on the final pass, after the calm challenge (§6), every family passing on all 49 renders:**

| Check | Fails when | Result |
|---|---|---|
| CHIPS | a region prints more chips than it may (stage 3; sidebar exactly `␣cn` and `␣.`; bottom bar 1; echo none), or a chip or hint sits where `reference/grammar.md` §9 does not allow it | 0 failures over 137 chips (stage 26, sidebar 86 on 43 renders, bottom bar 1, floats 24) and 33 hints |
| RESOLVE | a lit chip or hint is not bound for its command in the context and mode that have the keys | 0 failures; 170 chips and hints, 118 judged (lit), 52 muted and not judged |
| CORAL | a solid coral mass outside the allowed kinds (count tags, the Needs you head, Answer, the verb tag and badge), coral text outside a page title, or thin coral outside a keyboard cue | 0 failures over 65 masses |
| CURSOR | not exactly one visible coral cursor in the region with the keys, or one anywhere else; zero is allowed only under a float with no selection, while starting, and before anything is received (`spec.md` §4.6) | 0 failures: stage 29, container 10, float 8, sidebar 2; zero on 5 renders, each an allowed case |
| PRESENCE | a count above 0 without exactly one visible tag (the Home row, or the bottom bar while the sidebar is hidden), a tag at 0, Home's title without its tag, or an abnormal link whose words show in other than exactly one place | 0 failures: `◆`, `×` and `?` on all 49 renders (147 checks), the server's words on the 5 abnormal-link renders |
| CONTRAST | text under 4.5:1 (3:1 large) or a mark under 3:1 on its surface | 0 failures over 5,421 samples (4,170 text, 1,251 marks); 3 renders in Ember Light |
| COUNTS | a count tag, Home's title tag, a group head and the drawn rows or notes disagree | 0 failures over 321 counted values |
| MEASURE | a prose line wider than 600 px | 0 failures over 126 blocks; the widest 600 px (85 characters) |
| SPOOF | a row label rendered as Markdown or wrapped, a bidi control in a row, an `#id` out of its slot, a page title that is not one plain line, a content `h1` without its rule, a failed tint without the word | 0 failures over 395 row texts, the fixture's `hostile` block included |
| DIM | a chip, hint or segment lit while its key does not act, or muted while it does; Answer solid coral when `␣ad` would not open the readback | 0 failures over 181 chips, hints and segments on 22 renders |
| IDS | an ID clipped or cut, the `after <decision ID>` span and the sidebar included | 0 failures |

**RESOLVE runs on `keymap.toml`.** The prototype generates its key table from this package's keymap (165 bindings, 78 ex names, 22 contexts, the floats as root contexts, and the builtin sets including `start_insert`). The list of keys that may be printed at all comes from `reference/grammar.md` §9, because the keymap has no printed-key field.

**Mutants:** 79 seeded faults, at least one per family, each family passing on its unmutated state first; 79/79 caught. Among them: two coral cursors; no visible cursor where one is due; a cursor bar clipped out of view; a coral cursor kept in the stage under a float; a `◆` tag drawn before anything arrived; the bottom bar's tags or the server's words removed while the sidebar is hidden; `⌘⏎`, `M-p` or `M-w` lit while `create` is in flight; Answer solid while the sidebar has the keys; `M-p` hinted in a conversation's composer; a hostile title rendered as Markdown in a sidebar row; a bidi control in a row; a cut `after` span. The challenger's 15 extra faults (§6) were rerun against these checks: 14 are caught and kept as mutants; the 15th only rewrote labels the checks no longer read.

**Not rendered:** `○ connecting` at the bottom bar's right end while the sidebar is hidden (`spec.md` §4.8) is implemented in the prototype and covered by PRESENCE's rule, but no state draws it.

**Looked:** the prototype's builder opened the four states the challenge added (`home-connecting`, `start-starting`, `sidebar-focus-decision`, `conversation-hostile`) and `connect-http-warning`, `answer-conflict`, `readback-send-again`, `model-picker`, `server-stopped`, `unknown-outcome`, `conversation-records`, `home-1300`, `narrow-800`, `decision-light`, `decision-then-faulted` and `conversation-owned`; in the pass before, the six start and sidebar states and `home`, `conversation-streaming`, `decision-note-insert`, `readback-answer`, `project`, `bind-float` and `wire-readback-answer`, among others. The coordinator opened `sidebar-focus` and `start-options-unset`. The owner judged the look on a three-round sketch, kept in the design workbench, not on this set.

## 2. The keymap

**Measured, final pass:** `keymap.toml` (89 commands, 12 of them modal key sets, 165 bindings, 22 contexts) passes the keymap lint with 0 violations: KL0, KL1, KL2, KL4, KL5, KL7 (against `client.md` §4), KL8, KL9, KL12, KL14, KL16. It also passes KL4g, the package's own rule that any command sending `answer` is decide-tier under `␣a` (`tools/keymap_answer_rule.py`, 0 violations). KL10 was run against a cheat sheet generated from the same file, so it proves only that generation works; no cheat sheet is kept.

**Mutants,** kept in `tools/keymap-mutants.toml`, 9 of 9 caught: an unknown `wire` (KL7), an answer offered from a list (KL4), an answer outside `␣a` (KL4), a prefix clash under `␣o` (KL2), `x` at the write tier (KL5), the leader in Insert (KL14), a retry command that sends `answer` (KL4g), the sidebar's `x` bound without shadowing the list's (KL2), and `start.project` bound in Insert (KL14). `keymap_answer_rule.py --mutants` checks the KL4g one itself; each of the other eight is caught when the keymap lint runs on it (`../tools/proto/tools/lint.py`; from this folder, run `python3 -B ../tools/proto/tools/lint.py keymap.toml --protocol client.md --cheatsheet <a generated sheet>`).

**Not measured by the lint:** that a float's context does not inherit `global` (FK13): bindings in different modes never overlap, so no lint rule can see it; it is a unit check for the client. KL17 and KL20 run on renders; the prototype's CHIPS, RESOLVE and DIM families are their stand-ins.

## 3. The fixture

**Measured:** `fixture.py` regenerates `fixture.json` byte-identically. The calm pass added the device-local state of the new frame (`local.openItems`, `local.places`, `local.lastProject`, `local.lastWorkflow`, `local.sidebarVisible`, `local.detailsVisible`, `local.startDraft`) and the start page's `create` and `input` calls. Its call results follow `packages/host/src/calls.ts`; its stream frames follow `streams.ts` and `inbox.ts`; `Fault` follows `faults.ts`; the answer refusals' messages are the ones `packages/store/src/decisions.ts` writes; the extensions' schemas equal what `packages/tau/test/modules.test.ts` asserts; entries, `pi.live`, `pi.inbox` and `pi.agent` follow frames captured from pi-durable 1.5.2 in `docs/core/arena-wire/candidate-1.md` and `candidate-3.md`.

**Inferred, not captured** (labelled in the fixture's `_about`): the `pi.tool-result` entry shape (from `packages/w1/test/setup.ts`), `runFailure.message`, the error object of a faulted task's outcome, and both the path and the message of a `required` error, which is why the client computes missing options from the schema. **Invented:** the owned conversation `render-core #8`, since no shipped extension creates one.

## 4. Rulings on the prototype (first pass)

The prototype's builder listed 18 ambiguities and 5 weak spots. Each was ruled on 2026-11-15; `spec.md` holds the result. Those that changed a render, applied in the final pass:

1. Failed and Running rows on Home become one 28 px line, so Activity rises above the fold.
2. The failed mark is drawn as Carbon's octagon at 14 px, not a circle.
3. Rows drop the backticks of inline code; content-bar text (queue, unsent rows) renders inline Markdown.
4. A single newline is a line break in every rendered Markdown (the forecast basis keeps its lines).
5. Markdown headings inside the peek step down (`peek.h1` 14/20).
6. The Project view gets a left margin of counts that open lists filtered to the project.
7. which-key lists groups first, a hairline, then single commands.

**Accepted differences** found in the final pass, now written into the spec:

- Carbon's `stop-sign--filled` still read as a circle at 14 px (its glyph is inset, with rounded corners), so the mark is the client's own octagon on a whole-pixel 14 × 14 grid with square corners (`reference/visual-language.md` §6, `tokens.json` `tau.icon.failed_mark`).
- In the Project view's 200 px margin, `Conversations` puts its counts on a second line (`spec.md` §5.10).
- Home's Failed and Running rows keep a fixed 132 px reason or model column, which leaves a gap before the time on short reasons (`spec.md` §5.2, `tokens.json` `tau.layout.row_slots`).
- The which-key heads read `leader · display only · decides nothing` and `answer · every command reads back what it sends before y` (`reference/grammar.md` §5).

**Measured** while ruling: Iosevka 34 has its own glyph for `ℹ` (U+2139), a bold i in its cell, the same 24 px advance as `i` at 48 px. At 12 px it reads close to `1`. It is not a font fallback; the client shows tool output's bytes unchanged.

## 5. The challenge (first pass)

One challenger read the whole package against the core's source on 2026-11-15 and reported 26 findings (1 block, 16 major, 9 minor), each settled in the text the same day by the coordinator's rulings. Five became owner defaults (D6 to D10 in `open-questions.md`). The challenger reproduced every number in §1 to §3 as they stood then. The final prototype pass (§1) applied every image change the rulings made:

- **Decision states:** the provenance line and the title's `#n` are gone (one fact, one place).
- **Project states:** `server path` and the project name are gone from the margin.
- **Conversation states:** `exit 1` is gone from failed tool blocks, and the state and model are said once, in the action row.
- **`conversation-records`:** records read `by a task`.
- **Home:** fault rows read `task faulted`.
- **Pickers:** the badge is `COMMAND`, and workflows have no diamond.
- **which-key:** the root no longer lists the cut commands, and the answer group's greyed reason reads `not answered`.
- **Outbox:** its footer reads `␣rt retry`.

## 6. The challenge (calm pass)

One challenger read the calm package against the core's source on 2026-11-15 and reported 32 findings (2 block, 14 major, 16 minor). Every one was accepted and settled in the text the same day; the review and the rulings are kept in the design workbench. The challenger reproduced every number of the pass before (45 renders, byte-identical images, 45/45 mutants, the lint, the fixture), and a probe of 15 extra seeded faults showed that 13 passed every family unnoticed; the checks were then rebuilt (§1) and catch 14 of them.

- **Block:** before the server's first frame the client looked calm (no tags, `Nothing needs you`): now "nothing received yet" (`spec.md` §4.8). `⌘⏎` on the start page stayed live while `create` was in flight, so a double press started two agents: now `starting…` locks the page, and empty text sends nothing.
- **Changed defaults:** D12 now shows no duration on a finished tool row; a running row counts up (`open-questions.md`). `␣cn`, `␣pa` and `␣po` take the project the current view names, else the last used.
- **Simpler:** one `open` stream for the shown conversation and one for the peek, the others' marks from the poll; one flat list on `M-w` for workflows and the bare agent's models.
- **Added with the settlement:** a float that takes the keys has its own badge and inherits no `global` keys; four states (`home-connecting`, `start-starting`, `sidebar-focus-decision`, `conversation-hostile`); the fixture's `hostile` block and new refusals.

The final prototype pass (§1) rendered every image change the rulings made; five small gaps it found between text and states were settled in the text (zero cursors while starting and before any data; a lit hint's colour; a faulted kind cut before its `after` span drops; `as of` on a revoked decision; the hidden-sidebar `○ connecting` noted as not rendered).

## 7. Reasoned only

- Everything about rune: wrap with pills, the virtual list at long transcripts, key timestamps against the drawn frame, IME behaviour. The earlier rune spike (E-RUNE-1) measured wrapping and fonts for the Render client, not these screens.
- Every behaviour of the server, from `docs/core` and `packages/host/src`; no client has run against a real `tau`.
- The cost of polling `conversations` (it reads each conversation's `pi.live`) and `faults` (it scans every finished task of the workflow kinds) on a large store.
- Mapping a Visual selection in rendered Markdown back to exact UTF-16 source ranges.
- That the since-line's window-focus rule matches how the owner leaves and returns.
- No human trial has run on this design.
