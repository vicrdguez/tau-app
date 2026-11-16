# tau-app

The tau desktop app's UI: a rune window onto one tau server, built to `design/`. This crate draws and handles keys. The link to the server lives in `crates/tau-client`; `App` owns one `Link` and pumps it every frame.

## Commands

All run through Nix from the repository root.

| Command | What it does |
|---|---|
| `nix run .#dev` | the window, debug build; `nix run .#dev -- --demo` runs it on the design's fixture instead of a server |
| `nix run .#run` | the window, release build |
| `nix run .#test` | `cargo test --workspace` |
| `nix run .#lint` | `cargo fmt --check`, clippy with `-D warnings`, `nixfmt --check` |
| `nix run .#snap [scene [WxH]]` | headless PNGs into `target/snaps/`, every scene or one, in UTC (the fixture's clock); a scene named `…-800` renders 800 px wide |
| `nix run .#script <name> <at> [scene]` | plays `tools/scripts/<name>.json` (keys and clicks, as a window gets them) from a scene and renders the frame at `<at>` seconds |
| `nix run .#compare <scene> [mockup]` | the render beside its mockup and their difference, into `target/snaps/compare/` |

Motion replays use the same keys as the window.

| Replay | Shows |
|---|---|
| `nix run .#script sidebar-slide 0.3 home` | sidebar hiding at 100 ms; 0.7 is settled and 1.0 shows the return |
| `nix run .#script details-slide 0.3 conversation-streaming` | details opening beside the conversation; 1.0 shows closing |
| `nix run .#script send-enter 0.3 conversation-steer-insert` | a three-line draft sent above the shrinking composer |
| `nix run .#script fold-glide 0.45 conversation-streaming` | a tool fold opening at 100 ms; 1.15 shows closing |

Fonts load at run time from `$TAU_ASSETS/fonts`, which every command sets. The path baked in at build time is the default.

## Modules

| Module | Holds |
|---|---|
| `theme` | `design/tokens.json` parsed once: three colour variants (`t.c`), the type scale (`t.ty`), layout and timings (`t.l`), derived reading widths (`t.read`), and motion policy (`t.motion`). `Motion::moves` creates user-triggered animation islands for floats, panels, folds and sends. Motion off uses `Spring::NONE` and no islands. `Theme::of(variant)` swaps colours only, so `␣ht` never reflows. |
| `assets` | the fonts, the Carbon icons, the client's small marks and key glyphs, the brand SVGs |
| `keys` | the keymap engine over `design/keymap.toml`: contexts, modes, prefix-free resolution with `shadows`, counts, pending sequences, which-key entries, `:` names, lit and muted chips. It returns an `Outcome` and never runs anything. |
| `editor` | one modal editor (Insert, Normal, Visual) layered over rune's `TextEdit`, for the composer, notes, fields, Connect and the start page |
| `copy` | the copy's words for what the link reports: refusals, outbox rows, link echoes, Connect's failures. Pure functions; key hints come apart from the words |
| `time` | clock times in local time (`16:41`) and spans (`4.2 s`, `17 m`, `1 h 5 m`) |
| `content` | agent and extension text: one plain line for rows and titles, and tau Markdown (§ Content) |
| `open` | the open items in the sidebar's order: open, close, cycle |
| `ui` | primitives that take the theme and plain data: rows, chips and hints, count tags, the mode badge, group heads, facts lines, the container, the text field, the notice band, the float shell, the wordmark |
| `frame` | the sidebar, the stage header, the bottom bar, the notice band and the layout of every region at any window size |
| `floats` | which-key, the palette, the `:` line, `:messages`, Confirm (and `gx`'s, `open_url`), and the sheet (`:jumps`, the `␣hk` cheat sheet generated from `keymap.toml`); `Float::Screen` holds a screen's own float |
| `nav` | the jumplist (`C-o C-i :jumps`, kept on this device) and hint mode (`gs`), over each screen's anchor and targets |
| `yank` | `y{obj}` and `␣y{obj}` from the readers: the operator waiting for its object, plain-text sentences, the register |
| `app` | the state (`App`), the rune wiring (`spec`, `view`, `poll`), key dispatch (`input`), the command tables (`commands`), the link in the app (`link`), what the frame shows of the server (`server`) and the headless scenes (`scenes`) |
| `screens` | one family per file or folder: `connect`, `start`, `home`, `places`, `project`, `peek`, `conversation`, `decision` (§ Screens) |

## The link in the app

`App::launch(link)` opens the window on the start page (D11), or on Connect while the link isn't paired. `main` gives it `Link::open(LinkConfig::standard())`, or the fixture link (`app::link::demo()`) with `--demo` and in headless renders. `AppSpec::setup` hands rune's waker to the link; window focus and blur call `Link::set_focused` (the link polls only while focused); `poll` calls `app::link::pump`, which:

1. pumps the link, so server messages change the model only at the frame boundary;
2. echoes each `LinkEvent` once in the copy's words (`copy::event_echo`), then hands it to every screen's `on_event`;
3. refreshes `App::server` (`ServerView`): the server line and its mark, the count tags (each `None` until received: `◆` the inbox's size, `×` Home's Failed head minus `App::hidden`, `?` the outbox's unknown entries), the open items' labels, marks and `gone`, and the link's notice band (offline, refused, revoked, another protocol, a stream that keeps failing; superseded is the decision screen's). An open item keeps the label it had when the server stops naming it;
4. asks for what the open items lack: a conversation's label, a decision's title once a launch;
5. shows Connect while the link isn't paired.

`App::show(stage)` also points the link at the stage: the `open` stream of the conversation it shows (`Slot::Stage`), and the decision it watches. The peek's `Slot::Peek` is the peek's to set.

**Who may read client types.** `frame`, `ui`, `floats` and `content` take plain data and never name `tau_client` (a test checks it). Screens may read the client's domain model read-only: `app.link.server()` (`Server`, `Known`, `shapes`), `app.link.outbox()`, `app.link.status()`; duplicating it would only add a copy to keep in step. They write through the link's calls (`app.link.write(..)`, `show`, `fetch_*`) and say outcomes with `copy`. `ServerView` is the frame's: a screen doesn't add fields to it.

## Screens

A screen family is one file, or one folder with a `mod.rs`, under `src/screens/`. The app reaches every family through the `Screen` trait and two tables in `screens/mod.rs` that already list all eight: `of(stage)` names the family that draws a stage, and `get(id)` the family itself. `Stage` (in `app/mod.rs`) has every case: `Connect`, `Start`, `Place(Home | Conversations | Timeline | Faults | Projects)` and `Item(Conversation | Decision | Project)`.

| Family | Draws | Built by |
|---|---|---|
| `connect` | Connect | done |
| `start` | the start page | the start and project screens |
| `project` | the Projects place and the Project view | the start and project screens |
| `home` | Home | Home and the places |
| `places` | Conversations, Timeline, Faults | Home and the places |
| `peek` | the peek column of Home and the places, and the peek float | Home and the places |
| `conversation` | a conversation | the conversation view |
| `decision` | a decision and its readback floats | the decision view |

**A screen agent edits only its family's file or folder.** Everything a family declares is a `Screen` method with a default, so a stub fills in only `ctx` and `view`:

1. **Its state.** The family's `State` struct, in its own file. `App::screens.<family>` holds it (`Screens` in `screens/mod.rs` already has a field per family). Per-object state (drafts per decision, a cursor per place) lives in maps inside it.
2. **Keys and frame parts.** `ctx` and `mode` (a page with an editor reports the editor's mode and its editor context; `badge` follows `mode`), `crumbs`, `segments`, `has_peek`, `has_details`, `details`, and `framed` (false draws no sidebar, toggle or breadcrumb: Connect).
3. **Its view.** `view(t, app)`, built from `ui`, `content` and the theme. It fills the stage below the header and the notice band; the frame clips it at the top of any float. The peek's `view` is the peek column.
4. **Its commands.** `commands()` returns `(id, handler)` pairs, keyed by the command `id` in `keymap.toml`, that run while the family has the keys: its page, or a float it opened. Several families may handle one id (`list.open`). `globals()` returns handlers for commands whose keymap context is `global` and that the family owns (`conv.new` is the start page's); they run wherever the keys are, and one id has one global handler. A command resolves to the handler of the family that has the keys, else the global one, else it echoes `<desc> isn't built yet`. A handler gets `&mut App`, the `Invocation` (count, the keys that ran it, and `applies`, false where the command has nothing to act on) and the `Tx`.
5. **Its other keys.** `keys(app, keys, count)` gets the keys no command took while the family has the keys: list motions, an editor's keys. Return false to let the focused field type the key.
6. **An editor.** Give the editor's `edit` to `text_input` with `on_edit` (reaching the field in your `State`) and `on_input_key(app::input::on_field_key)`. In `keys`, hand the keys to `Editor::insert` or `Editor::normal`; `Reply::Pass` returns false. Call `Editor::tick` from `poll` for Insert's timed `jk`. Connect is the worked example.
7. **Its floats.** Implement `ScreenFloat` (owner, scope, badge, view, `edit_mut`) and open it with `app.open_float(Float::Screen(Box::new(..)))`. Its keys go to the owner's `commands` and `keys`; `screens::top_float::<T>(app)` gets it back to change it. Confirm is the frame's (`Float::Confirm` with an `on_yes` step).
8. **Jumps, hints, yanks.** `anchor` says where the page's cursor stands, as JSON only the family reads back, and `land` puts it there: the jumplist keeps anchors, and a jump records one when a command the keymap marks `jump` (or `gg`, `G`) moves the stage. `targets` lists what hint mode labels: each target's rune key, where its label sits, the scrolling element that clips it, and its anchor. `yank` returns the source text of a text object at the cursor. A family without a cursor leaves all four at their defaults.
The defaulted `Screen::moving` fingerprints the page state a key may animate. The conversation includes its folds and unsent input ids, but not its cursor or server entries. `app::input::Moving` compares that fingerprint with float count, sidebar placement and details visibility. A page change or float close stays instant.

9. **Link hooks.** `shown` runs when `App::show` puts one of the family's stages on the stage, `on_event` for every `LinkEvent`, `on_focus` for window focus, `poll` every frame.
10. **Its scenes.** `scenes()` returns named states built with `app::scenes::on_fixture(app, stage)` (the fixture's open items, `stage` shown, the fixture link settled) or `before_any_frame`. Name a scene after its mockup, render it with `nix run .#snap <name>` and compare with `nix run .#compare <name>`.
11. **Its tests.** In the family's own file or folder (`connect/tests.rs`). `app::testing` makes links on the fixture that a test can silence, refuse every call to, or end a stream of, on a clock that moves when told.

Document a screen in its module doc comment, not in this README, so screen work never edits a shared file. A new Carbon icon goes in both lists, `nix/assets.nix` and `carbon!` in `assets.rs`, in alphabetical order; the prototype's icons are all there already.

## Working on a screen

- **Sources, in order of authority.** `design/spec.md`, then `design/reference/visual-language.md`, `design/reference/grammar.md`, `design/keymap.toml` (the command ids handlers implement), `design/client.md` for what each call means, and `design/failure-catalogue.md` for the defences a screen carries. The text wins over any image.
- **Exact sizes.** `tools/proto/states.md` describes every mockup state, and `tools/proto/app.js` with `calm.css` is the prototype that rendered `design/mockups/`. Read them when the prose leaves a size, gap or colour open.
- **Rune.** The pinned rev's `crates/rune/README.md`, `crates/rune-kit/README.md` and `examples/` at https://github.com/anomalyco/rune (the rev is in the workspace `Cargo.toml`).
- **The render loop.** Build each state as a named scene on the fixture link, render it with `nix run .#snap <scene>`, compare it with `nix run .#compare <scene> <mockup>`, and look at both images. Iterate until layout, type, colour and marks match; note what still differs and why. The pixel share covers the whole window, so it understates a local mistake.
- **Tests.** Behaviour at the screen's seam: what a key does to state, which `Write` a command sends, what a view shows for a fixture state. Write the failure modes first, then the tests, then the code. Keep them fast.
- **Boundaries.** `frame`, `ui`, `floats` and `content` never import `tau_client`; a test enforces it. Screens read the client's model read-only and act only through `Link` methods.
- **Commands.** Run cargo only through the Nix commands, so the toolchain, SDK and assets are right.

## Content

`content` draws agent and extension text without app state or client types.

```rust
use tau_app::content::{line, markdown, Tint};

let label = line(t, label_source, t.ty.ui, t.c.fg.default);
let title = line(t, title_source, t.ty.title_page, t.c.state.attn);
let body = markdown(t, source).build();
let preview = markdown(t, source).peek().streaming(true).build();
let noted = markdown(t, source)
    .tint([Tint { range: 12..28, active: true }])
    .build();
```

`line(Theme, &str, TextStyle, Color) -> El` returns one rich-text element with rune's end truncation. Give it a bounded width, or a shrinking slot such as `ui::slot_row`. `runs(&str) -> Vec<Run>` is the pure parser. `Run` is `Text(String)` or `Escape(String)`. Equal-length paired backtick delimiters disappear. Other Markdown, unmatched delimiters, and ordinary Unicode stay literal. Invisible characters use uppercase Unicode escapes such as `U+000A`, `U+202E`, and `U+200B`, in `mono.small` on `surface.bar`.

`markdown(Theme, &str) -> Markdown<'_>` borrows the source until `build(self) -> El`. The consuming options are `peek()`, `streaming(bool)`, and `tint(impl IntoIterator<Item = Tint>)`. Full Markdown uses `prose`; the peek uses `prose.small` and the peek heading tokens. The caller owns the reading column's width, outer spacing, scrolling, and the gap below its page title.

`Tint { range: Range<usize>, active: bool }` uses UTF-8 bytes of the original source, not UTF-16 or preprocessed bytes. Invalid and empty ranges paint nothing. Endpoints inside a character expand to its boundaries, ranges crossing the cap clip, and ranges wholly beyond it disappear. Tints use `tint.note_range` or `tint.note_active` without a ring. Their local rune keys are `note-0`, `note-1`, and so on, under the caller's keyed content element. Note conversion from server UTF-16 ranges belongs to the decision screen.

`source.rs` caps each supplied content string at 2,000 source lines. A terminal newline does not add a line. The omission label is a separate element, so an unclosed fence cannot swallow it. It exposes controls, bidi controls, zero-width characters, and invisible characters decoded by rune-kit's parser. Markdown's structural LF and CRLF remain line boundaries. Tabs become `U+0009`, including in code. Rune-kit's `Rendered::source_ranges` identifies prose soft breaks, which become hard breaks without changing code or list boundaries. The same source map supplies heading, bold, and inline-code colours. Sparse edits map the caller's original note ranges into the prepared source.

`outline.rs` turns the prepared source into the blocks this module draws itself: headings, lists, quotes, code and images, around the prose rune-kit draws. A list item's or a quote's body is its source with the markers and indentation cut out, parsed again as a document of its own, so nesting needs no second Markdown renderer. A `Cut` is the pieces of the prepared source a block shows; it maps note ranges, paints and colours into the block's own text. GFM alerts are read from a quote's first line, since `source.rs` has already turned its newline into a hard break.

`render.rs` draws the outline. Gaps follow the prototype's margins, collapsed as a page collapses them: 12 px after a paragraph, list, table, code block, quote or image; `h1` 12 below its rule (6 px padding and 1 px `border.strong`); `h2` 26 above and 8 below; `h3` to `h6` 24 and 6; list items 4 apart, and a list nested in an item 4 under the item's text. The peek steps down to 8, `h1` 6 (4 px padding), `h2` 12 and 4, `h3` to `h6` 10 and 3 (`spec.md` §5.4). List markers hang in a 24 px column (18 in the peek) plus 2 px: `• ◦ ▪` by level in `fg.muted`, numerals right-aligned in `mono` `fg.alt`, or a task's 14 px box. A quote has its 3 px `border.strong` bar and its prose in italic `fg.alt`; an alert's kind word heads it in 600. Every fenced block, `diff`, `patch` and `mermaid` included, is the same square `surface.code` block with a 2 px gold rule, `mono` 13/20 (12/18 in the peek), padding 20 above, 7 below and 14 at the sides, and its language in gold `ui.small` pinned 3 px from its top and 10 from its right; in the peek the code has 5 px above it, so the label sits over the first line's end. Code scrolls sideways under a 32 px fade and is coloured by rune-kit's scanners, except the line still streaming in. An image loads nothing and is a 32 px placeholder, `image · <alt> · <host>`.

`style.rs` owns the rune-kit style for prose, the fonts of both sizes, and the syntax colours. It reads Markdown metrics and the peek's width from `t.l`, the reading measure from `t.read`, and syntax colours from `t.c`. Colours otherwise come from the supplied theme. A heading's level picks rune-kit's fourth heading slot, so `h5` (uppercase, +0.04 em) and `h6` (italic) keep their own fonts; `h5` is uppercased only where a capital takes the same bytes, so marks keep their place. Tables fit the reading measure (the peek's column in the peek) less any list or quote inset: their columns shrink to their longest word, then the table scrolls. Inline-code pills are padded 5 px (4 in the peek). Links have no opener, tasks have no handler, and images have no loader. Content never receives coral from this module. The block holding the source's end streams (`streaming(true)`), and the final unterminated source line is muted through a source highlight.

### Pinned rune-kit limits

These are unmet design rules, not substitutes for future rune-kit hooks.

| Rule | Current rendering and missing hook |
|---|---|
| 1 px table rules | Rule colours and header weight are supported, but rune-kit fixes rule thickness at 0.5 pt. |
| Literal raw HTML | Shown as typed. `content::source` escapes each `<` that opens HTML before rune-kit parses, since rune-kit drops HTML and an HTML block would swallow the prose after it. Note ranges stay mapped to the original bytes. |
| Markdown escape font | Escape backgrounds and colour are supported through highlights. Highlights cannot set a font, so Markdown escapes retain their surrounding font rather than `mono.small`. Plain-line escapes have the required font. |
| Inline-code metrics | The pill has rune's fixed radius rather than 4 px; rune-kit's prose text exposes no `span_radius`. Its font scales with the surrounding heading. |
| Wide-block edge indicators | Code has the 32 px fade. Tables scroll without one, and neither has the 2 px edge rule or the caret: whether a block is clipped is known only after layout. |
| Images inside a link or emphasis | Their alt text stays in the line, as rune-kit draws them. Only an image standing in a paragraph becomes the placeholder. |
| Reference links across blocks | Each prose block is parsed on its own, so a reference link whose definition sits in another block shows as typed. |
| Tail-only incremental work | The block holding the end streams, but the outline and preprocessing read the capped source on every build. This adapter does not promise tail-only total parsing cost. |

### Render specimen

`src/content/walkthrough.md` holds headings, nested lists, a task, a table, long Rust code, a quote, a link, an image, raw HTML and hostile characters. Render it through a page's `view` to look at it; the pure cores are tested in `line_tests.rs`, `source_tests.rs` and `outline_tests.rs` (`nix run .#test -- content::`).
