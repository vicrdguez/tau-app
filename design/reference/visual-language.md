# Visual language

How the tau desktop client looks. Every value is a token in `../tokens.json` (the gaps and the Markdown metrics this file names are in `tau.layout.gaps` and `tau.layout.markdown`); this file says where each goes and why. Screens are in `../spec.md`.

**Identity: Ember, calm.** Warm graphite, ivory text, and coral, which marks what needs you, where your keys act, the brand and the page titles. Real type with a scale, small line icons, a left sidebar and a quiet stage. Things are told apart by spacing and type; one container style, used rarely, holds what you write and what floats over the page. It keeps the terminal's good cues: a mode badge, `:w`, the `:` line and pickers rising from the bottom, a block caret, few printed keys. It drops the limits of a text grid and the clutter of every fact and key on every screen. Ember's other accents name kinds of things, so text differs by colour as well as brightness, without a rainbow.

**Refused:** cards, boxes around rows or groups, everything-in-a-rounded-box, radii above 6 px, round buttons and big buttons, sheets, vibrancy, translucency, blur, gradient blobs, emoji in chrome, spinners, colour on body text, a different hue per project.

## 1. Layout

**Spacing** on a 4 px base: `space.1` 2 · `space.2` 4 · `space.3` 8 · `space.4` 12 · `space.5` 16 · `space.6` 24 · `space.7` 32.

**The frame** is in `../spec.md` §4: the sidebar 248, the stage with its 44 px header, the peek 440, the details panel 240, the bottom floats, the bottom bar 26.

**Rows:** dense rows are 28 px, with no rules between them. Needs you entries are a 26 px head and a 22 px byline, 6 px apart. Group heads use `group` (13/18 600 steel), 26 px above. Float rows are 30 px. Rows, bands and the sidebar are square.

**Hairlines:** one under the stage header, one at the left of the peek and of the details panel, the dividers inside a float (above its foot, between the readback's columns, between which-key's groups and commands), and the containers' edges. All are 1 px `border.hairline`. Every other line belongs to content (heading rules, table rules, the code block's gold rule, quote bars), to the 2 px content bar, or is the note leader.

**Selection:** a square band (`surface.cursor.read` in the stage, `surface.current` in the sidebar, `surface.cursor.side` in floats) with a 2 px coral bar on its left edge, inside the column, full band height, while that region has the keys. Without the keys it keeps the band and loses the bar (§10). On a Needs you row the stage's cursor head is solid coral instead, text knocked out in `surface.read`.

### 1.1 The one container style

`surface.raised`, a 1 px `border.hairline` edge, `radius.surface` 6, padding 12 to 16 px. It holds only:

- **your message** (a `pi.user` entry), on `tint.human` instead of `surface.raised`;
- **the composer**, and a note being typed;
- **opened tool output**, on `surface.sunken` (recessed);
- **floats**, which add a 2 px coral top rule and an upward shadow.

A **text field** (Connect's fields, the bind float's name and working directory) is the same style made small: `surface.sunken`, a 1 px `border.hairline` edge, `radius.pill`, 26 px tall, 10 px in (`tau.layout.text_field`). Its edge turns coral while it has the caret in Insert.

Everything else (agent text, records, tool rows, groups, the peek, the details panel, notes that are not being typed) has no box. In Insert, the focused container's edge turns coral, and its muted keys sit inside at the bottom right in `hint`.

## 2. Type

Two families, bundled, SIL OFL 1.1, version 34.9.1: **Iosevka Etoile** for chrome and prose, **Iosevka** for code, IDs, keys and fixed numeric columns. No third family. Etoile and Iosevka keep one advance width across weights, so bold never reflows a line. Ligatures and contextual alternates are off in mono runs.

| Token | Family | Size / line | Weight | Use |
|---|---|---|---|---|
| `title.page` | Etoile | 24/30 | 700 | page titles: Home, the places, and the conversation, decision and project titles; coral (`state.attn`); no rule |
| `prose.h1` | Etoile | 20/28 | 700 | content `h1`; rose (`content.h1`); always with a 1 px `border.strong` rule under it |
| `prose.h2` | Etoile | 17/24 | 600 | orange |
| `prose.h3` / `h4` | Etoile | 15/22 · 14/20 | 600 | `fg` |
| `prose.h5` / `h6` | Etoile | 13/19 | 600 | `fg.alt`; uppercase +0.04 em / italic |
| `move` | Etoile | 14/20 | 600 | Needs you heads |
| `group` | Etoile | 13/18 | 600 | group heads, steel; their count `fg.alt` |
| `prose` | Etoile | 14/21 | 400 | body |
| `prose.small` | Etoile | 13/19 | 400 | content on the 2 px content bar: peek bodies, queue items, unsent rows |
| `peek.h1` / `h2` / `h3` | Etoile | 14/20 · 13/19 · 13/19 | 700 · 600 · 600 | headings inside the peek: rose with rule · orange · `fg` |
| `ui` | Etoile | 13/18 | 400 | rows, sidebar rows, the breadcrumb, float rows |
| `ui.strong` | Etoile | 13/18 | 600 | an event row new since you left; author names |
| `ui.small` | Etoile | 12/16 | 400 | bylines, facts lines, the bottom bar and its echo, captions, field labels (steel) |
| `float.title` | Etoile | 14/20 | 600 | a float's head title |
| `sidebar.brand` · `sidebar.server` | Etoile | 14/18 · 11.5/16 | 600 · 400 | `tau` beside the avatar · the server line |
| `sidebar.section` | Etoile | 11/14 | 600 | `OPEN`: uppercase, +0.1 em, `fg.muted` |
| `sidebar.group` | Etoile | 12/16 | 400 | a project's name over its open items, sage |
| `mono` | Iosevka | 13/20 | 400 | code blocks, tool rows |
| `float.input` | Iosevka | 14/20 | 400 | a float's input, the `:` line included |
| `mono.small` | Iosevka | 12/16 | 400 | times, IDs in rows, every fixed numeric column |
| `mono.output` | Iosevka | 12/18 | 400 | opened tool output, `fg.alt` |
| `prose.code` | Iosevka | 13/21 | 400 | inline code: gold on a `surface.code` pill |
| `key` | Iosevka | 11.5/16 | 500 | key chips: orange on `surface.bar` (`surface.cursor.side` inside a float), `radius.pill` |
| `hint` | Iosevka | 11.5/16 | 400 | keys inside a container and on the start line, no fill: `fg.alt` while lit, `fg.muted` while inert (`spec.md` §4.6) |
| `badge` | Iosevka | 11/14 | 600 | the mode badge and the readback verb tag; uppercase, +0.08 em |

Chrome is told from content by lane, surface, size and case, not by face: content can't produce the badge style, a chip, a fill, a container or a page title.

**A page title is not a heading.** `title.page` is coral, 24/30, has no rule, and is always the stage's first line, with its facts line under it. A content `h1` is rose, 20/28, has a rule, and sits at least 24 px below the title block. Content renders only in `prose.*`, so a heading in agent or extension text can never take the page title's style.

## 3. Reading layout

- **Measure:** prose in reading columns wraps at **840 px**, about 112 average characters of Etoile at 14 px. The conversation's column is 880 px so that containers and tool rows have room around their text. Forms keep their 600/640 px columns. Wider panes centre the column.
- **Wrapping:** rune's native wrap (UAX #14), no inserted hyphens, no inserted characters: copied text equals the source bytes. IDs, paths, key names and inline code up to 40 characters never break: inline code is a drawn pill; IDs and paths in prose are transparent pills, only when they contain a break opportunity (`-`, `/`, `.`). A longer token wraps anywhere.
- **Line breaks:** in every rendered Markdown (basis, inputs, agent text, records, peek), a single newline is a line break, as in a chat or a GitHub comment. Paragraphs are separated by a blank line.
- **Blocks that don't wrap** (tables) start at the text column's left edge and grow right to the pane's content edge, or to the column's edge where a margin or a panel sits beside it (the decision's notes margin, the details panel); beyond it they scroll, and each clipped edge shows a 32 px fade, a 2 px `fg.muted` edge rule and a `caret--right`.
- **Truncation:** labels end in `…`; paths keep their leaf (`…/notes-cli`); IDs never truncate, and a span that holds one (a faulted task's `after w1:6:design#1`) shows whole or not at all; the one exception is a faulted task's kind in a Failed row, which is cut with `…` before its `after` span drops (`spec.md` §4.7); a facts or since-line drops whole parts from its end; numbers and times drop whole, never cut. Rows drop whole fields from the right.
- **Rows are plain text.** Extension and agent text in a row is one `ui` line: no Markdown, no bold, control characters escaped, paired inline-code backticks dropped, cut at the end.
- **Markdown blocks** (CommonMark plus GFM tables, task lists, strikethrough, autolinks, footnotes; GFM alerts; a `mermaid` fence is an ordinary code block in v1, §9):

| Block | Rendering |
|---|---|
| Paragraph | `prose`, 12 px between paragraphs |
| Headings | `h1` rose with its rule; `h2` orange; `h3`–`h6` `fg`; size, weight and rule carry rank |
| Lists | markers `• ◦ ▪` in `fg.muted`, 24 px per level, hanging indent; numerals right-aligned, tabular |
| Task list | a 14 px box in the marker column; display only |
| Table | header 600 `fg.alt` with a 1 px `border.strong` rule; 1 px `border.hairline` row rules; no vertical rules; cells 6 × 12 px; grows right, then scrolls |
| Blockquote | a 3 px `border.strong` bar, Etoile italic `fg.alt` |
| Alert | the quote shape plus the kind word; no icon, no hue |
| Code block | `surface.code`, a 2 px gold left rule, the language label top-right in gold `ui.small`, `mono` 13/20, wraps at spaces and inside tokens wider than the column; square |
| Inline code | gold on a `surface.code` pill, `radius.pill` |
| Link | prose face, salmon, 1 pt solid underline; `kind.link_active` under the cursor; `gx` opens after a confirm showing the URL |
| Emphasis | italic neutral; **bold** 700 in steel |
| Image | not loaded: a 32 px placeholder `image · <alt> · <host>` |
| Raw HTML, math, wiki links | literal text (tags such as `<design>` show as typed) |

- **Streaming:** the renderer re-parses only the last top-level block; an unclosed fence renders as a code block so far. The streaming tail is `fg.muted`, never coral.

## 4. Colour

### 4.1 Surfaces

The stage is the surface with the most contrast against text; chrome regions are one step quieter.

| Token | Ember (default) | Ember Soft | Ember Light | Used for |
|---|---|---|---|---|
| `surface.read` | `#151412` | `#1c1b19` | `#f0e8d8` | the stage, its header, the peek, the details panel |
| `surface.side` | `#1c1b19` | `#242320` | `#ede3d0` | the sidebar, the notice band |
| `surface.raised` | `#1e1d1b` | `#262522` | `#f8f3ea` | the container style: the composer, a note being typed, floats |
| `surface.bar` | `#252422` | `#2c2b28` | `#d8ccb0` | the bottom bar, key chips |
| `surface.cursor.read` | `#242320` | `#2a2927` | `#e2d6be` | the cursor band in the stage |
| `surface.cursor.side` | `#2e2d2a` | `#353430` | `#ddd0b8` | the mode badge; the selection in floats; key chips inside a float |
| `surface.current` | `#2e2d2a` | `#353430` | `#d8ccb0` | the sidebar's current row and its cursor band |
| `surface.code` | `#242320` | `#2a2927` | `#e6dac4` | code blocks, inline-code pills |
| `surface.sunken` | `#11100f` | `#181715` | `#e9e1d0` | opened tool output |
| `border.hairline` / `strong` | `#2c2b28` / `#3e3c38` | `#33322e` / `#444240` | `#d0c4aa` / `#b8ac96` | hairlines and container edges / content rules, the content bar |
| `fg` / `fg.alt` / `fg.muted` | `#d8d0c0` / `#b0a898` / `#999489` | same, muted `#a19c92` | `#282418` / `#585040` / `#5e5646` | text |

**Derivation.** Ember's new values come from the sketch (`e12-tau-calm`). Soft keeps Ember's step from the surface below: `raised` is `side` plus 2 per channel; `sunken` is `read` minus 4; `hairline` is `subtle`'s old value minus 2. Light lifts: `raised` is lighter than `read`, so a float or the composer reads as a sheet of paper on the page, and `sunken` is darker. Light's hairline is the old `subtle` plus 2. Light's `side`, `cursor.read` and `current` were moved in the calm challenge so that its fill cues reach Ember's step (§4.4); `current` is a token of its own because Light's `cursor.side` can't get darker without losing the badge against the bar.

The theme is the `theme` key in `~/.config/tau/settings.toml` (`ember`, `ember-soft`, `ember-light`); `␣ht` and the settings float (`␣hs`) switch it live within one frame and save it, and nothing reflows.

### 4.2 Hues: one hue, one family

Each hue has a token for marks (icons, bars, fills) and a token for text; they differ, and text always uses the text token, because the mark tokens fall under 4.5:1 on some surfaces (`state.blocked` 3.81:1 and `state.ok` 4.44:1 on `surface.cursor.side` in Ember). Both are in `../tokens.json`; `tau.hues` maps each hue name to them.

| Hue | Means | Mark token | Text token | Goes on |
|---|---|---|---|---|
| **Coral** | what needs you; where your keys act; the brand | `state.attn` | `state.attn`, only in `title.page` | §4.3 |
| **Steel** | a label; running | `state.stale` (the `time` mark, the running triangle) | `kind.you` | group heads, field labels, `input`, the composer's caption, the readback's labels, **bold** in content (`content.strong`) |
| **Orange** | literal machine text | none | `kind.literal` | `#4`, `#2`, decision IDs and subjects, task and entry kinds, key chips; content `h2` (`content.h2`) |
| **Gold** | code as a thing | `kind.code` (the code block's rule) | `kind.code` | inline code, the code block's rule and label |
| **Salmon** | you can go there | none | `kind.link`, `kind.link_active` under the cursor | links in content |
| **Sage** | where it lives; your draft | `state.local` (the `edit` mark) | `kind.worker` | project names (rows, the sidebar's groups, the start line), the server's host name; the draft mark; the note tints |
| **Olive** | it passed | `state.ok` | `text.ok` | `ok`-type outcome words and marks |
| **Rose** | it failed | `state.blocked` | `text.failed`; content headings use `content.h1` | the failed `×` and the `×` tag; words such as `failed`, `required`, `3 to set`, `needs options`; content `h1` |
| **Mauve** | unknown | `state.unknown` | none: unknown words stay `fg` | the unknown mark and the `?` tag |

Stays neutral: body prose, row labels, extension and agent text, field values, times (`fg.muted`), counts (`fg.alt`), the agent's voice. The colour budget:
1. Solid coral only as §4.3 lists.
2. Thin coral follows the keyboard and never touches agent content.
3. A hue goes on a short field whose type the client knows from a server field, never found by parsing a sentence.
4. Neutral leads: values and prose stay neutral. Page titles are coral and content headings rose; nothing else in a title block is coloured but the small mark, IDs and project names.
5. Olive and rose label nothing in chrome except outcomes.
6. A tint always comes with a word or a note: the failed-row tint with **`failed`**, a note range with its note. Note ranges are the one inline tint.
7. Syntax colour lives only inside code blocks.

### 4.3 Coral

Coral means "this needs you" or "your keys act here". It also marks the brand and page titles. It never appears in agent or extension text.

**Solid coral, only here:**
- the `◆` count tag, while above 0 (in the sidebar's Home row, on Home's title, or in the bottom bar when the sidebar is hidden);
- the stage's cursor head on a Needs you row, while the stage has the keys;
- `◆ ␣ad Answer` in an open decision's header, only when `␣ad` would open the readback (`../spec.md` §4.6); otherwise its muted outline;
- the answer readback's and send-again's verb tag and their solid `READBACK` badge (one pair); Confirm and the not-recorded reply show `READBACK` in the plain badge, with a coral edge;
- the brand: the sidebar's avatar (`tau-icon-small-coral`), and the τ of the wordmark on the start page and Connect.

Outside a readback, at most one solid coral mass shows besides the tags and the avatar: the Needs you head or the Answer. A calm screen has none: no tag, no head, no Answer. Not coral: Connect's action, continue, send again, stop, any agent or extension text.

**Coral text** is `title.page` only.

**Thin coral (≤ 2 px) is the keyboard:** the caret, the cursor bar, the edge of the focused container in Insert, a float's top rule, the `NORMAL` badge's left edge, the current search match.

**One coral cursor at a time.** Only the region that has the keys draws its cursor in coral: the bar, the Needs you head, or the caret with its container's edge. Every other region's cursor keeps its band and draws it neutral. A float that takes the keys turns the stage's and the sidebar's cursors neutral; if it has no selection (send-again, a reply, Confirm, Server), no coral cursor shows and its top rule marks it. which-key takes no keys and leaves every cursor as it is. The rule is `../spec.md` §4.6.

### 4.4 Tints

| Tint | Ember | Soft | Light | With |
|---|---|---|---|---|
| `tint.human` (your message) | `#1d1e1d` | `#232424` | `#e1ddd1` | `input` in steel |
| `tint.row_fail` | `#241918` | `#2b201f` | `#ecd9cc` | **`failed`**, bold: a failed tool row, the head of the run-failure block, the head of "The work after this answer failed"; `radius.pill`, no edge |
| `tint.note_range` | `#1f2420` | `#262b27` | `#dcd9c2` | a passage that has a note, inline behind its text |
| `tint.note_active` | `#272e28` | `#2e352f` | `#d1d4bc` | the passage of the note under the cursor, or of the note being typed |

**Derivation.** `tint.row_fail` is the midpoint of `surface.read` and the old `tint.tool_fail` in each variant. Note tints are sage at a low weight: Soft adds Ember's offsets from `surface.read`; Light mixes Light's sage into `surface.read` until the range tint reaches Ember's step (below), and the active tint is as dark as every text token allows at 4.5:1 (`kind.link` 4.57). `tint.note_active` is darker than the sketch's `#2c352e`: at that value steel, olive and rose text in a noted passage fell to 4.1:1. The active passage is also marked by the cursor, so its step from the range tint (1.13 / 1.14 / 1.06) is not a fill cue.

**Fill cues.** A fill that is the only sign of a state is at least **1.15:1** against the surface under it (`tau.fill_cue_min`). Three are: the sidebar's current row (`surface.current` on `surface.side`: 1.25 / 1.26 / 1.25), a neutral cursor band (`surface.cursor.read` on `surface.read`: 1.17 / 1.18 / 1.18), and a passage that has a note (`tint.note_range` on `surface.read`: 1.17 / 1.19 / 1.17). Light reaches Ember's step on each. Fills that come with a word, a mark or an edge (the failed row, your message, containers) are exempt.

**Leader:** `rule.note_leader` (Ember `#6b675f`, Soft `#736f67`, Light `#8c8270`), 1 px, the line from a displaced note to its passage (`../spec.md` §5.6). 3.27 / 3.44 / 3.11 : 1 on `surface.read`.

### 4.5 Contrast

Neutral text, worst case over every surface and tint it can sit on (the nine surfaces of §4.1, `tint.human`, `tint.row_fail` and both note tints): `fg` 8.99 / 8.13 / 9.72; `fg.alt` 5.84 / 5.28 / 5.00; `fg.muted` 4.56 / 4.56 / 4.55 (Ember / Soft / Light). The worst surface is still `surface.cursor.side` in the dark variants and `surface.bar` in Light; the new surfaces and tints are all easier. Each hue's text token reaches 4.5:1 on all of them, except `kind.link` in Light on `surface.bar` (4.35), where a link never sits. Marks are ≥ 3:1 (worst: `state.blocked` 3.44 on `surface.cursor.side` in Soft). Coral as `title.page` on `surface.read`: 6.51 / 6.09 / 4.55. Count tags knock their text out in `surface.read`: 6.51 / 6.09 / 4.55 on coral, 5.09 / 4.76 / 5.00 on rose, 5.11 / 4.77 / 4.79 on mauve. Hairlines are 1.2 to 1.6:1 by design: a container is told by its surface step as well as its edge. Under `surface.selection` coloured text falls to `fg`.

### 4.6 Shape before colour

Every state mark has its own silhouette, so colour-vision deficiency loses nothing: failed is a **heavy `×`**, unknown an **outline `?`** (`help`), running a **triangle**, needs-you a **filled diamond**, ok a **checkmark**. Every tint carries its word. References and code differ by shape: inline code is a pill, a link is the prose face underlined.

### 4.7 Syntax

rune-kit's scanners, colour only (never weight or slant): keyword orange, string olive, number and constant mauve, comment `fg.muted`, type and function gold, property and punctuation `fg.alt`, diff `+` olive and `−` rose. No coral and no background. Languages the kit doesn't know render plain, with the label still shown.

## 5. Surfaces, elevation, radius

| Level | What | Edge | Shadow |
|---|---|---|---|
| 0 | the sidebar, the stage and its header, the peek, the details panel, the bottom bar, rows, agent text, records, tool rows | none, or a hairline where §1 lists one | none |
| 1 | the containers (§1.1); chips, count tags, code blocks, the notice band | containers 1 px `border.hairline`; code a 2 px gold left rule | none |
| 2 | floats | 1 px `border.hairline` and a 2 px coral top rule | upward: `0 -14px 40px rgba(0,0,0,.45)` (Light `rgba(60,48,24,.22)`) |

**Radius:** `radius.surface` 6 on containers and floats; `radius.pill` 4 on key chips, count tags, hint labels, inline code, the coral Answer and the failed-row tint; 0 on rows, bands, the sidebar and every region. Inline code takes `radius.pill` where rune's in-text pill accepts a radius; at the pinned rune version that pill has fixed metrics (about 5 pt), unmeasured (`../open-questions.md` §3). No translucency, no blur.

## 6. Icons and marks

IBM Carbon icons (`@carbon/icons` 11.90.0, Apache-2.0), drawn as SVG paths and tinted: 16 px in sidebar rows and the header, 14 px in open items and rows, 12 px in small multiples. Icons supplement labels and never stand alone.

**Small marks** are the client's own drawings: an 8 to 10 px glyph centred in a 14 px slot (`tau.icon.mark_small`, paths in `../tokens.json`).

| Meaning | Mark | Colour |
|---|---|---|
| open decision, needs you | filled diamond, 9 px | coral (`state.attn`); knocked out in `surface.read` on the coral Needs you head and in the `◆` tag |
| failed (a run, a task, a tool call) | heavy `×`, 8 px, drawn for this size (`viewBox 0 0 10 10`, arms 1.6 px wide) | rose (`state.blocked`) |
| running | solid triangle, 8 px | steel (`state.stale`) |
| idle | none | |
| ok | Carbon `checkmark`, 12 px | olive |
| unknown | Carbon `help`, 12 px | mauve |
| hidden | Carbon `subtract` | `fg.muted` |
| stale, since you left; a sleeping task | Carbon `time` | steel |
| a waiting or queued task | Carbon `hourglass` | `fg.muted` |
| local draft | Carbon `edit` | sage |
| offline, reconnecting | ring | `fg.muted` |
| sidebar rows and kinds | `add` new, `search` find, `home`, `chat` conversation and Conversations, `time` Timeline, `folder` project and Projects, outline diamond decision, `settings` Server | `fg.muted`; `fg.alt` on the current row |
| the sidebar toggle | `side-panel--close` (shown), `side-panel--open` (hidden) | `fg.muted` |

The failed `×` replaces the octagon everywhere: the sidebar, Home, the lists, tool rows, facts lines and the `×` tag. Workflows in pickers have no icon (`w1 · workflow`): the outline diamond means a decision and nothing else.

**Key glyphs.** Letters, digits, punctuation and `⌘` in chips and hints come from Iosevka. `␣` and `⏎` are the client's own drawings (`tau.icon.key_space`, `tau.icon.key_return`), 9 px, in the chip's text colour: the sketch's `⏎` came from a fallback font and drew greyer than its neighbours, and a path needs no font.

## 7. Motion

Motion shows what your own key or press just did. Each motion uses a critically damped rune spring, with no overshoot. The `motion_ms` tokens specify perceptual durations, not a deadline for the last settling frame.

- Floats fade in on the 90 ms `float_fade` spring. Their coral top rule is outside the fading surface and has full opacity from the first frame. A key that closes a float removes it at once.
- The sidebar slides from the window's left edge and back on the 200 ms `panel` spring, beside the stage or over it. Beside, the stage moves with it. The sidebar stays opaque on exit, so its text never mixes with the page beneath it.
- The details panel slides from the stage's right edge on the same `panel` spring. The page beside it moves to its new position. Panel contents take their new layout without separate entrance fades.
- A sent message enters above the composer, rising 8 px as it fades in on the 180 ms `page` spring. The composer shrinks to its empty height and the transcript moves with it. When the server takes the input, the unsent row becomes a transcript message at once.
- A fold, including a tool row, tool group, thinking block or chrome line, opens and closes on the `page` spring. Its moving edge clips the content while the rows below follow.

Everything else lands within one frame. This includes cursor moves and their scrolls, page changes, mode changes, the theme, reading width, which-key, the echo, hint labels, typing, and anything the server changes. Streaming text, new entries, state words and unsent rows becoming messages never start motion. A motion already running may finish.

Inside a conversation, only folds, the unsent stack and the layout they displace animate. A press that focuses the composer lands at once. A page change or a key that closes a float lands the whole change, even if panel state changes with it.

Motion off, or the system's Reduce Motion, sets every duration to 0. There are no animation islands or animated key commits with motion off.

## 8. Chrome and content

1. **Lanes.** The sidebar, the stage header, the bottom bar, floats, hint labels, message and record headers, tool rows, title blocks, row marks and slots are chrome. Content renders only in content containers: message bodies, record bodies, the decision's basis, notes, and the 2 px content bar.
2. **Type.** Content renders only in `prose.*`, `prose.code` and `mono` (code). It can't choose a face, size, tracking or colour.
3. **Icons, chips, fills, tints and containers are chrome.** A code block is always neutral with a gold rule, whatever it prints (`✓ exit 0`). Tool rows come only from `pi.assistant` tool calls, `pi.tool-result` entries and `pi.live.tools`, and sit between message bodies, never inside one.
4. **Provenance comes from structure.** Your message's container only from a `pi.user` entry; the agent header only from `pi.assistant`; a record only from an entry of another kind. Hue is not provenance.
5. **Inert rendering.** No HTML, no remote images, no command links. Bidi, zero-width and control characters render as visible escapes in `mono.small` on `surface.bar`.
6. **Size cap:** a content block clips at 2,000 lines, with `… N more lines`.
7. **Rows** hold extension and agent text only as one plain line after the client's own mark (`../spec.md` §5.3).

## 9. Diagrams: deferred

Mermaid diagrams are not in v1. A `mermaid` fence renders as an ordinary code block: `surface.code`, the gold rule, the label `mermaid`, the source as text, unhighlighted. A diagram renderer, its fit rule and a full view come back with a design, a fixture and a state of their own (`../open-questions.md` §5).

## 10. Focus, cursor, hint labels

- One keyboard focus per window: the stage, the sidebar, or the top float. The focused region's cursor is coral (§4.3); the others keep a neutral band.
- **The sidebar's current item** (what the stage shows) has a neutral `surface.current` fill and no bar. When the sidebar has the keys, its cursor row adds the 2 px coral bar, and the stage's cursor drops its bar (a Needs you head turns from solid coral to the neutral band).
- In documents: a full-width current-line band, the 2 px coral bar at its left, a coral block caret (drawn as an element).
- In Insert: a bar caret in coral, and the edge of its container in coral.
- Hint labels: chips in `mono.small`, `surface.read` text on an `fg` fill, `radius.pill`, at the start of the target's text after its mark.
- Key chips: `key` orange on `surface.bar` (on `surface.cursor.side` inside a float), `radius.pill`. A chip, hint or segment is lit only while its key resolves for the region and the mode that have the keys; otherwise it draws `fg.muted` and is inert (`../spec.md` §4.6). So in Insert the leader chips are muted, with the sidebar holding the keys the header's segments are, and under a float everything outside it is.

## 11. Brand

From `../brand/`. The name is written `tau` everywhere but the logo.

- **The sidebar's avatar:** `tau-icon-small-coral.svg` at 24 px, beside `tau` and the server line. It is the same in every variant.
- **The wordmark** on the start page and on Connect: 44 px tall, `tau-wordmark-on-dark-coral.svg` in Ember and Ember Soft, `tau-wordmark-on-light-coral.svg` in Ember Light. Its τ is coral and its "au" is `fg`, as drawn; the client doesn't tint it.
- **The Dock icon:** `tau-icon-coral`, with a badge carrying the `◆` count while it is above 0.

Coral in the brand is the one coral that doesn't follow the keyboard or mean "needs you". It sits only in these three places, never beside a count, so it can't be read as one.
