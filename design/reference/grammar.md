# Interaction grammar

One command language for the tau desktop client, and for a terminal client later. `../keymap.toml` is the single declarative table of every key; this file says what the table means, how keys resolve, and the lint rules (§12) that keep the table coherent. Screens are in `../spec.md`.

Notation: `␣` is the leader (Space). `⏎` Enter. `C-x` Ctrl-x, `M-x` Option-x, `⌘` Cmd (GUI only, always an alias).

## 1. Principles

1. **Verbs over tau's objects.** vim's operators, motions and text objects, with tau's nouns as objects: decision row, message, tool call, note, group, open item.
2. **Answering is a separate dialect.** Only `␣ad` (answer) and `␣ar` (send the answer again) send an answer. Both run in that decision's view only, both read back what they send, and only `y` sends, after the readback has been on screen for 400 ms. Nothing else sends an answer: not `␣rt`, counts, `.`, hints, links, `Esc`, a click, or a server event (§11).
3. **Keys act on what was drawn.** A key resolves against the last drawn frame, never against state that arrived after it (§7).
4. **Muscle memory carries over or fails safe.** Keys the user owns in nvim keep their meaning where tau has one; elsewhere they are unbound or local.
5. **Prefix-free, no timeouts.** which-key appears after 300 ms and decides nothing. The only timed binding is Insert `jk`.
6. **Same table, two renderers.** Every binding has a terminal form (KL9).
7. **A printed key works where it is printed, and few are printed** (§9).
8. **One region has the keys.** The stage, the sidebar or the top float; only it draws a coral cursor.

## 2. Modes

| Mode | Badge | Edge | Entered by | Left by | What keys do |
|---|---|---|---|---|---|
| Normal | `NORMAL` | coral | default; `Esc` | | motions, operators, the leader, `g z ] [` |
| Visual | `VISUAL`, `V-LINE` | steel | `v`, `V` (builtins in readers and editors) | `Esc`, an operator | extend a selection |
| Operator-pending | `NORMAL`, pending keys in the bottom bar | coral | `c`, `gc`, `y` | a motion or text object, `Esc` | wait for an object |
| Insert | `INSERT` | sage | `i a o` in editors and composers; the start page on open | `Esc`, `jk` | text entry; the leader is inactive |
| Command | `COMMAND` | orange | `:`, `␣;`, `⌘K`, `␣.`, every picker | `⏎`, `Esc`, `C-c` | the float's input |
| Hint | `HINT` | orange | `gs` | a label, `Esc` | every key picks a label |
| Readback | `READBACK`: solid coral, word knocked out, in the answer readback and send-again; the plain badge with a coral edge in Confirm and the not-recorded reply | coral | `␣ad`, `␣ar`; a Confirm; an answer's `conflict` reply | `y`, cancel | only the float's own keys: pick, confirm, cancel, details |
| Outbox | `OUTBOX` | coral | `␣ru`, the `?` tag | `Esc`, `q` | `j k`, `K`, `␣rt` retry, `x` forget |
| Server | `SERVER` | coral | `␣os`, the server line, the `Server` row | `Esc`, `q` | `j k` scroll |
| Peek | `PEEK` | coral | `K` on a list row under a 1000 px stage, or on an open item | `K`, `q`, `Esc` | `j k` move the cursor under it, `⏎` opens |

The badge is the bottom bar's first segment: square, 26 px tall, Iosevka 11/14 600 tracked +0.08 em on `surface.cursor.side`, with a 2 px left edge in the mode's colour. Cursor shapes: block (Normal), bar (Insert), underline (Operator-pending), none (Readback). Every float that takes the keys names its own mode, never `NORMAL`: the pickers (palette, find, project, workflow, model, thinking, filter) and the bind and add-extension checklists are `COMMAND`; the bind float's text fields are `INSERT`; the readback, send-again, the not-recorded reply and Confirm are `READBACK`; the outbox is `OUTBOX`, the Server float `SERVER`, the peek float `PEEK`. The sidebar and which-key are not modes: the badge stays `NORMAL`, and which-key shows the pending keys.

## 3. Contexts and focus

A command applies in the contexts it lists and their descendants. An inner context may shadow an outer binding only when it says so (`shadows = "parent"`). **A float that takes the keys is a root of its own:** nothing of `global` acts under it (no leader, no `:`, no `⌘K`, no `K`), only its own keys, with `Esc` (and `q` where it has no input) to close it.

```
global
├─ list            Home, Conversations, Timeline, Faults, Projects
│  └─ sidebar      the sidebar while it has the keys
├─ reader
│  ├─ conversation
│  ├─ decision
│  └─ project      the Project view's option fields
├─ editor          never hosts an answer
│  ├─ composer     a conversation's input
│  ├─ note         a decision note
│  ├─ field        an option value in the Project view
│  ├─ connect      the Connect screen
│  └─ start        the start page's composer
└─ hint
picker             palette, find, project, workflow, model, thinking, filter
bind               the bind float's two text fields, and the bind and add-extension checklists
readback           the answer readback, send again
reply              the not-recorded reply
confirm            before bind, addExtensions and pairing over plain HTTP
outbox             the outbox float
server             the Server float
peek               the K float
```

**Focus.** One region has the keys: the stage (the context of its page), the sidebar (`sidebar`), or the top float. The peek column is not a region: it has no cursor and never takes the keys. Only the region with the keys draws a coral cursor, and a chip, hint or segment is lit only where its key resolves for that region and mode (`../spec.md` §4.6).
- `C-h` gives the sidebar the keys (window left); `C-l`, `Esc` or `q` give them back (window right). A sidebar that is not drawn (hidden by choice, or under a 1000 px window) is drawn over the stage while it, or a float opened from it, has the keys. Under 1000 px `␣tb` does what `C-h` does.
- The sidebar is a list: `j k gg G` move, `]g [g` move between its sections, `⏎`/`l` open the row in the stage and give the stage the keys, `K` peeks an open item in the float. `x` closes the open item under the cursor; it shadows the list's `x`.
- Any command that changes what the stage shows gives the stage the keys. The leader works in the sidebar.
- A float takes the keys while it is open and hands them back to the region that had them; meanwhile the stage's and the sidebar's cursors are neutral, and a float without a selection shows no coral cursor. which-key takes no keys.

## 4. Verbs and objects

```ebnf
sequence   = [ count ] , ( motion | operation | command )
           | ":" , exname , [ " " , args ] , "⏎" ;
operation  = operator , ( motion | textobject | operator ) ;   (* doubled = linewise *)
operator   = "c" | "gc" | "y" | "␣y" ;
motion     = basic | ( "]" | "[" ) , objchar | "/" , pattern , "⏎" | "?" , pattern , "⏎" | "n" | "N" ;
objchar    = "q" | "g" | "m" | "M" | "t" | "n" | "b" ;
textobject = ( "i" | "a" ) , ( "w" | "s" | "p" | "m" ) ;
```

**Verbs**

| Key | Meaning | Tier |
|---|---|---|
| `c{obj}`, `gc{obj}` | in a decision: a note on that range of the basis; in an editor: vim's change | local (a draft) |
| `y{obj}`, `␣y{obj}` | yank text; to the system clipboard | local |
| `dd` | decision: discard the note draft under the cursor; Project view: reset the field to its default (a draft) | local |
| `u`, `C-r` | undo, redo a draft change | local |
| `⏎`, `l` | open the row's object; in the sidebar, the place or item; in a reader, the link, `#id`, field or choice under the cursor | local |
| `K` | details: the peek float of a list row (under a 1000 px stage) or an open item, a field's schema, a record's data, a truncated value in full | local |
| `gd` | go to the linked object: a decision's or event's conversation, a fault's decision, an owner | local, a jump |
| `gx` | open an external link, after a confirm showing the whole URL | local |
| `gs` | hint mode | local |
| `x` | a faulted-task or failed-run row: hide on this device; an outbox entry: forget; an open item in the sidebar: close | local |
| `q`, `Esc` | close the top float; in the sidebar, back to the stage; in an editor's Normal mode, back to the view | local |
| `i a o` | conversation: write in the composer | local |
| `C-h`, `C-l` | the sidebar takes the keys; the stage takes them back | local |
| `⌘⏎`, `M-⏎` | composer, Insert or Normal: send or steer; queue as follow-up. Start page: start. Never on empty text | write |

**Objects**

| Char | Object | Motion | Text object | Where |
|---|---|---|---|---|
| `q` | open-decision row, from anywhere | `]q [q` | | global |
| `b` | open item, in the sidebar's order, wrapping | `]b [b` | | global |
| `g` | group in a list | `]g [g` | | lists, the sidebar |
| `m` | message | `]m [m ]M [M` | `im am` | conversation |
| `t` | tool call | `]t [t` | | conversation |
| `n` | note on a passage, in text order | `]n [n` | | decision |
| `p s w` | paragraph, sentence, word | `{ } ( ) w b` | `ip ap is as iw aw` | readers |

`]x` moves to the start of the next object; `[x` to the start of the one the cursor is in, or the previous one. `]x` motions are not jumps, except `]q` and `]b`, which change the view; `␣n`, `gd`, `gg`, `G`, searches and every open are jumps too. `]b` follows unimpaired's buffer motion: open items are tau's buffers.

**Display forms** stay whole when text wraps: `#4` (conversation), `w1:3:design#2` (decision), `w1:3:design` (subject), `#2` (request number), `w1.design_answer` (task or entry kind), `anthropic/claude-opus-4-5` (model), `entry 34` (entry). The client matches objects by their IDs, never by parsing what it displayed.

## 5. Leader and which-key

```
groups:    a ◇ answer     c conversation     o open     p project
           m this view    r recover          t toggle   h help
commands:  n next decision   . find anything   ; commands   y yank to clipboard
```

| Group | Leaves |
|---|---|
| `␣a` ◇ answer | `d` answer this decision · `r` send the answer again |
| `␣c` conversation | `n` new conversation (the start page) · `x` stop the run · `c` continue a failed run · `m` switch model · `t` thinking level · `f` queue as follow-up |
| `␣o` open | `h` home · `c` conversations · `t` timeline · `f` faults · `p` projects · `s` server and streams |
| `␣p` project | `b` bind a project · `a` add extensions · `o` this project's options · `f` filter this place |
| `␣m` this view | decision: `n` note on paragraph or selection · `w` note on the whole basis. project: `s` save option drafts (`:w`) · `r` reset field to default |
| `␣r` recover | `u` outbox · `t` retry the call, same body · `c` reconnect now · `p` pair again |
| `␣t` toggle | `f` follow the run · `s` basis as source text · `h` show hidden failures · `b` sidebar · `d` details panel |
| `␣h` help | `k` keys cheat sheet · `t` theme · `s` settings (also `⌘,`) |

Home is `␣oh` and the sidebar's Home row; there is no other key for it.

**which-key** opens in the float shell once a proper prefix (`␣`, `␣a`, `g`, `z`, `]`, `[`, `d`) has waited 300 ms since its last key. Each prefix key typed before then starts the wait again, so a sequence typed without such a pause never shows it. Once up, it stays for the whole sequence: a further prefix key swaps its table at once, with no new wait, and it leaves when the sequence ends. It is display only and consumes no key but `Esc`, which cancels the prefix; the bottom bar shows the pending keys. Body: the key grid, group entries first in `fg` 600, a 1 px hairline, then single commands in `fg.alt` 400. Each entry is its key (orange `mono`) and its `desc`. The root's head is `␣ leader` alone, with the hint `Esc closes`. The `␣a` head is `answer` with the subtitle `every command reads back what it sends before y`; its leaves carry an outlined `◇` in `fg.alt`, never coral. A leaf that can't run here stays listed, greyed, with the reason: `open a decision first`, `not answered`, `its work did not fail`, `offline`, `an answer is waiting for a reply`.

## 6. Normal keys by context

- **Global:** `j k gg G C-d C-u`, `C-o C-i` (jumps), `/ ? n N`, `:`, `⌘K`, `gs`, `]b [b`, `C-h`, `q`/`Esc` close the top float, `K`, `gd`, `]q [q`; `:q` closes the open item the stage shows (drafts stay), `:qa` quits.
- **List:** `⏎`/`l` open; `]g [g`; `K`; `x` (failures, outbox entries); `␣pf` filter (Conversations, Timeline, Faults).
- **Sidebar:** the list's keys over its rows; `x` closes an open item; `C-l`, `Esc`, `q` back to the stage.
- **Conversation:** `i a o` compose; `]m [m ]t [t`; `za zo zc zR zM` fold tool rows and thinking; `G` on the last entry resumes follow; `␣c…`; `␣td`.
- **Decision:** reader motions; `v V`; `c{motion}`, `gc`, `␣mn`, `␣mw`; `]n [n`; `i` edits the note under the cursor; `dd`, `u`, `C-r`; `⏎` on a choice moves the cursor there; `␣ad`, `␣ar`; `␣ts`; `␣td`.
- **Project:** `j k` field by field (the field under the cursor is bold); `⏎`/`i` edit; `dd`/`␣mr` reset; `:w`/`␣ms` save.
- **Start page:** Insert on open (§10); in its Normal mode the editor's keys, `⌘⏎` starts, `M-p` and `M-w` (`:project`, `:workflow`) change the project and the workflow, `i a o` return to Insert.

## 7. Resolution and pinning

```text
on frame boundary:  apply queued server messages (frames, call replies); draw
on key(e):          resolve against the last drawn frame
  first key of a sequence pins {view, object ID, anchor, the drawn state of the target}
  dispatch resolves by object ID, never by row index
```

- **Server messages apply at frame boundaries only;** a key meets exactly what the last drawn frame showed.
- **The cursor is attached to an object ID.** An insert above it scrolls the view, not the cursor. When its row leaves (a decision answered on another device), the cursor moves to the next row and the echo says why (`answered elsewhere`).
- **No server event moves focus, opens a float, opens a readback, or adds, closes or reorders an open item.** A server event may only turn an action off or change a mark.
- **The send mode is the drawn one.** A composer whose caption said `steer` sends a steer; if the run ended in flight, the server starts a run with it, and the transcript shows that.
- **The start page sends what its line drew:** the project and the workflow (or model) as drawn when `⌘⏎` was pressed, even if a poll changed them after. From the press until `create` replies or times out, `⌘⏎`, `M-p` and `M-w` are off and the text is read-only; auto-repeat never fires `⌘⏎`. A `create` result that arrives after the stage left the page adds the conversation to the sidebar and moves nothing.
- **`␣cx` and `␣cc` act only in the conversation's own view,** where `pi.live` is live. `␣cx` names the drawn run: it is on only while the drawn frame shows a run, and off after sending until the view shows that run gone or a new one. A list row's state comes from a poll and never drives them.
- **Every bottom float keeps its target in view:** the cursor's row, line or field rides the float's top edge once the edge reaches it, and a conversation's queue, unsent rows and composer ride it too (`../spec.md` §4); nothing else on the page moves.

## 8. The float shell and the bottom bar

Every bottom float is one component (`../spec.md` §6), and one that takes the keys is a root context: only its own keys act under it (§3). It spans the stage, inset 12 px at the sides and 10 px at the foot, on `surface.raised` with a hairline, a 2 px coral top rule, `radius.surface` and an upward shadow, up to 40 % of the window tall. It has a **head** (an optional tag, a title, a subtitle, right-aligned meta), a **body** (rows, a key-value grid, a key grid, or the readback's two columns), and a **foot** above a hairline (an optional input at the left, hints at the right). While it is open, chips, hints and segments outside it are muted and inert, and the badge names its mode.

- `:` takes ex names from the table (`:w`, `:answer`, `:theme`, `:bind`). `Tab` completes; Up/Down walk the history.
- **Palette** (`␣;`, `⌘K`): every command as `desc · :ex · keys`, with `◇` on answers; one that can't run here is listed with the reason. Choosing an answer command opens its readback.
- **Pickers** (`␣.`, `M-p` and `M-w` on the start page, `␣cm`, `␣ct`, `␣pf`, `␣pa` when no view names a project): `C-j C-k` move, `C-n C-p` page the preview, `⏎` chooses, and in the start page's and option fields' pickers a press on a row chooses it; `Esc C-c` cancel. Results freeze once you move. Every picker is one step: `M-w` lists the workflows and the models together.
- **The bind and add-extension checklists** (`bind.keys`): `j k`, `C-j C-k`, `C-n C-p` and Down Up move, `␣` ticks or unticks the row under the cursor, `⏎` goes on to Confirm, `Esc C-c` cancel. `Tab` and `S-Tab` walk the bind float's regions (name, working directory, extensions), wrapping, and never tick.
- **The bottom bar** (26 px, always there) holds the badge, pending keys, the echo and the save state; while the sidebar is hidden, the count tags and abnormal server state at its right end. The echo is the last transient message: it never repeats the bar and prints no key chips. Anything the user must act on is a notice band, never only an echo. `:messages` keeps the history.

## 9. Printed keys

**A key printed as an invitation is a chip and works where it is printed.** It is lit only while its key resolves for the region and mode that have the keys, and muted otherwise (`../spec.md` §4.6). At first sight a view prints at most 3 chips in the stage (its header included), 1 in the bottom bar, and none in the echo. The sidebar always prints its two (`␣cn` on `+ New`, `␣.` on `Find`). Floats, composer and note footers, the start line and form footers print their own keys as hints and are exempt from the budget, not from resolving. Printed:

- `⏎` on the Needs you cursor head;
- the header's segments: `◆ ␣ad Answer`, `␣ar send again`, `␣cx stop`, `␣cc continue`;
- `␣cc` in the run-failure and stopped lines;
- `:w` with option drafts: in the bottom bar, and as `:w save` on the head of each Options group that has drafts. The Project view prints no other chip in the stage, so it keeps the budget while drafts span at most three groups;
- `c` in the first-note ghost (`+ note c`);
- `␣rc` on the offline notice, `␣rp` on the revoked notice, `␣rt` on an unknown row that isn't an answer;
- `␣pb` on Home and the Projects place with no projects, pressable (the start page's composer says `⌘⏎ bind a project` instead, since it is in Insert);
- `␣th` on a place whose filtered rows are all hidden on this device (`2 faulted tasks hidden on this device ␣th show hidden`);
- `⏎ connect` in the Connect form's footer; over plain HTTP its `y` is in the Confirm float, never in the form, where `y` is a letter;
- in the sidebar, `␣cn` and `␣.`;
- as hints: `M-p` and `M-w` on the start line (lit in Insert and Normal), `⌘⏎ start` or `starting…` in the start composer, the composer's and the note's Insert keys, the float hints. The start line's two parts and the start composer's hint take a press too, as their keys do and only while lit.

A chip or hint prints Option as `⌥` with the key in capitals (`⌥P`, `⌥⏎`), as it prints Command (`⌘K`); `keymap.toml`, `settings.toml` and this file write it `M-`. Assistive technology reads a printed key in words, from the keys this device types: `Space c n`, `Command-Return`, `Option-P`, `Control-H`, `Escape`, in a chip, a hint, which-key, the palette, the Settings key cells and `␣hk` alike; a list of keys reads `Space Semicolon or Command-K`; a control of words and a key reads as one name, `start, Command-Return`. A disabled action prints no chip (`stop · off`). Server text never names a client key. The one plain-word key is the echo after a click on the readback's `y` hint, `press y to send this answer` (`press y to send it again` in send-again). which-key, the palette and `␣hk` carry the rest.

## 10. Editors

The composer, a note, an option field, the Connect fields and the start page's composer share one modal editor with a vim subset: `h j k l w b e W B E 0 ^ $ gg G f t F T ; ,`, the operators `d c y` with `iw aw ip ap is as i" a" i( a( i[ a[ i{ a{ iq aq ib ab`, `dd yy cc D C x p P J u C-r .`, `v V`, `i a I A o O`, `jk`/`Esc`, and Insert `C-w C-u`.

- **The composer:** `⌘⏎` sends (or steers) from Insert and from Normal mode; `M-⏎` queues a follow-up. **Bare `⏎` never sends:** in Insert it is a new line and continues a Markdown list item; in Normal mode it does nothing. In a terminal client, `⏎` in the composer's Normal mode sends, and `␣cf` replaces `M-⏎`.
- **The start page's composer:** opens in Insert. `⌘⏎` starts (`create`, then `input`), never on empty text; `⏎` is a new line. `M-p` and `M-w` open the project picker and the workflow list, in Insert and in Normal: they are the commands `start.project` (`:project`) and `start.workflow` (`:workflow`), bound in both modes of the start page only, so in every other editor Option-p and Option-w type `π` and `∑` as usual. In a terminal they need the kitty keyboard protocol, as `M-⏎` does; in Normal, `␣pp` and `␣pw` are their terminal forms.
- **A note:** `⏎` is a new line; the note is sent only with its answer.
- **Single-line editors** (field, Connect): `⏎` commits the value; `Tab` and `S-Tab` move to the next and previous field; on the last field `⏎` submits the form. The bind float's two fields are not editors in this sense: they live in the float's own context with their own keys (`bind.keys`) and no Normal mode, so nothing of `global` reaches them; `Esc` moves to the checklist.
- A composer opens in Insert. The first `Esc` returns to its Normal mode; a second `Esc` (or `q`) returns to the view, keeping the text as a local draft (sage `edit`, `not sent`). The start page has no view behind its composer, so a second `Esc` does nothing there.
- `Esc` always leaves Insert; `jk` is unavailable while an input method composes. Apart from the start page's two, no Option + printable key binds in an editor.
- Editors never host an answer: `␣a` is inert in Insert.

## 11. The answer dialect

1. Two decide-tier commands exist: `␣ad` answer, `␣ar` send the answer again. Nothing else sends an answer: `␣rt` never does, and an outbox entry for an answer says `open the decision to send it again`.
2. They run only in that decision's view, where the basis is drawn. On a list row or in the sidebar they echo `open it first · ⏎`.
3. They open a readback that shows everything the call will send: the choice, the notes, the basis entry and how much of it was drawn, the `then` task, this device.
4. **`y` counts only after the readback has been on screen for 400 ms** (`tau.client_ms.readback_arm`), and confirms the readback as last drawn. An earlier `y` is discarded and echoes `read it first`, so a burst such as `␣ad2y` never sends. Auto-repeat and `⏎` never confirm; a click never confirms.
5. In the readback `j k`, or `1`–`9` while there are at most nine choices, pick the choice in its left column; each pick redraws, and `y` waits for that draw. `K` reads every note. `n Esc q C-c` cancel.
6. No server event opens a readback; a server event can only turn `y` off (the request is no longer open, the link is down, the device is refused or revoked).
7. `␣ar` is live when the decision's `then` task ended `faulted`, and when this device's answer has an unknown outcome that the client could not settle. `␣ad` is off while such an outcome is pending.

A decision's basis is one entry that never changes, and a revised basis is a new decision ID, so "you can only answer what you were shown" holds by construction; the client tracks no read basis.

## 12. The keymap file and its lint

`keymap.toml` has `[meta]`, `[contexts]`, `[groups]`, `[builtin]` and one `[[command]]` per command with `id`, `ex`, `desc` (≤ 32), `tier` (`local`, `write`, `decide`), `kind` (`command`, `motion`, `operator`, `textobject`, `modal`), `ctx`, optional `wire` (the calls it makes), and `keys` with `mode`, `ctx`, `seq` and optional `shadows`, `alias_of`, `tui`, `tui_alt`. Modes: `n v i c h rb ho ob sv` (`rb` the readback, send-again, reply and confirm floats, `ho` the peek float, `ob` the outbox float, `sv` the Server float).

| Rule | Checks |
|---|---|
| KL0 | schema: known fields, keys parse, the context tree is acyclic, no binding is triggered by a server event |
| KL1 | every command has a unique `id` and ex name and is in the palette |
| KL2 | prefix-freedom per mode in overlapping contexts, unless `shadows`; no clash with the builtin sets |
| KL4 | the decide tier: only under `␣a`, only in context `decision` (never a list), never countable or repeatable, never in an editor, always with `wire`, never Insert, picker, hint or readback modes |
| KL4g | any command whose `wire` includes `answer` is tier `decide`, in context `decision` only, bound only under `␣a` (`../tools/keymap_answer_rule.py`) |
| KL5 | reserved keys (`Esc C-c q u C-r .`, trap keys such as `dd x`) bind only local commands; `:w` at most write |
| KL7 | every `wire` names a call in `../client.md` §4 |
| KL8 | every prefix has a which-key label; `desc` ≤ 32 and unique in its group |
| KL9 | every binding has a terminal encoding, a `tui_alt`, or another binding that has one (`<D-x>` is an alias only) |
| KL10 | a generated cheat sheet, if one is kept, matches the table |
| KL12 | one object char means one object |
| KL14 | Insert binds only editing keys, `Esc`, `jk`, `<CR>`, `<C-CR>`, `<M-CR>`, `<D-CR>`, `Tab`, `S-Tab`, the editor's builtin Insert keys, and an Option or Command chord its command binds in Normal in the same context; never the leader |
| KL16 | `alias_of` names a command of the same tier with overlapping contexts |
| KL17 | at render: every visible chip and hint resolves in its active context; strict in Insert |
| KL20 | at render: the chip budget of §9 |

KL0–KL16 run on the file; KL17 and KL20 run on rendered views, from a table generated from `keymap.toml` (with its builtin sets). KL17 takes each chip's context and mode from the region and the badge the render shows, never from a label the render declares.

**A builtin set the workbench lint doesn't read.** `[builtin] visual` (`Esc` leaving Visual) is documented here and generated into KL17's table, but the lint doesn't check it. The start page's `M-p` and `M-w` are command bindings in Insert and in Normal, allowed in Insert by KL14's twin clause; two mutants keep the clause narrow: `start.project` bound in Insert with no Normal twin, and its Insert key moved into the composer, each trip KL14. `v` and `V` are in `reader_motions`, so KL2 guards them in readers. `../tools/keymap-mutants.toml` holds seeded faults that each must trip its rule; `../evidence.md` has the latest result.

**Reserved for splits,** which v1 doesn't have: `C-j C-k` in Normal mode, the `␣w` group and the `C-w` group, all bound to nothing; and `C-v`/`C-s` in pickers. `C-h` and `C-l` already move between the sidebar and the stage, and will move between splits too; `C-w h` is `C-h`'s terminal form. In pickers `C-j` and `C-k` move the cursor (the fzf habit); that is the only place they are bound.

## 13. Hint mode and jumps

- **Hint mode** (`gs`) labels every target visible in the region that has the keys (rows, notes, choices, links, the details panel's links, segments) with home-row labels `a s d f g h j k l`, two letters when there are more than nine. Labels are chrome chips placed at the start of each target's text after its state mark, shifted right to avoid overlaps, frozen when `gs` is pressed. A label moves the cursor and records a jump; it never opens or acts. `Esc` cancels.
- **Jumplist:** one per device, 100 entries of `{object, anchor}`; `C-o C-i` move through it; `:jumps` lists it. Landing on a decision that is no longer open shows it as it is, with its state in the facts line. A jump to an object that isn't an open item opens it.

## 14. Terminal mapping

A future terminal client shares this grammar and `keymap.toml`, not the layout. `⌘` bindings are aliases (`⌘⏎` falls back to `⏎` in a composer's Normal mode and `C-⏎` in Insert); `M-⏎` falls back to `␣cf`; `C-i` is sent as `Tab`; `C-h` falls back to `C-w h`; the start page's `M-p` and `M-w` need the kitty keyboard protocol, or `␣pp` and `␣pw` in its Normal mode. The rest of the table is terminal-representable (KL9).
