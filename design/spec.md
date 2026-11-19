# tau desktop: specification

The desktop client of a tau server: what it shows, how it flows, and what each screen asks of the server. Visual rules are in `reference/visual-language.md`, keyboard rules in `reference/grammar.md` and `keymap.toml`, and connection behaviour in `client.md`. Test data shaped like the protocol is `fixture.json`.

**Words.** *The server* is a running `tau`: the host layer and the store layer of `docs/core`. *A call* is one `POST /v1/<name>`. *A stream* is a call answered by server-sent events (`open`, `inbox`, `timeline`). *Extension text* is text an extension's code wrote: a decision title, an event's `text`, a fault's `error`. *Agent text* is what a model wrote. *A place* is one of Home, Conversations, Timeline, Projects and Faults. *An open item* is a conversation, decision or project you opened; the sidebar lists them. The name is written `tau`; `τau` is the logo only. Keys: `␣` is Space (the leader), `⏎` Enter, `C-x` Ctrl, `M-x` Option, `⌘` Cmd.

---

## 1. Principles

1. **One window onto one server.** What needs you (open decisions), what broke (failed runs, faulted tasks), what runs (conversations), and what happened (events), across every project. Everything else is a detail of one of these.
2. **It knows no workflow.** It is complete for the bare agent, and it shows any workflow through the core's own shapes: decisions, events, conversations, faults, options.
3. **Quiet means nothing asks.** A calm screen has colour, a coral page title and the brand, but no coral fill and no `◆` tag.
4. **Answering is a separate dialect.** Only `␣ad` (answer) and `␣ar` (send the answer again) send an answer. Both run in that decision's view only, both read back what they send, and only `y` sends. An answer whose reply was lost is sent again only with `␣ar`, never from the outbox (owner defaults D5, D7).
5. **Say what is true, including "unknown".** Every call ends as done, refused or unknown, and the screen says which (`client.md`). Before the server has said anything, the screen says that, not "nothing" (§4.8).
6. **Chrome and content never mix.** Agent and extension text render only in content lanes, or as one plain line in a row or a page title. Content can't produce an icon, a fill, a chip, a container, a chrome face or a coral mark.
7. **One state, one number; one fact, one place.** A count reads the same everywhere it appears. On a view, each fact appears once; the breadcrumb, the title block and the details panel never repeat each other, except that the details panel gives the model in full.
8. **Few printed keys.** A printed key works where it is printed. which-key, the palette and `␣hk` find the rest.

---

## 2. The core as a client sees it

The calls and streams, with their bodies and results, are listed in `client.md` §4. What the core does not give, and what this client does instead:

| Missing | The client | Cost |
|---|---|---|
| A stream of conversation states | polls `conversations` every 10 s per project while the window is focused, every page (`client.md` §5); the `open` stream of the conversation the stage shows, and the peek's, are live | Running, Failed, the `×` tag, the sidebar's marks and the Project view's counts can be up to 10 s old; one call per 500 conversations per project each time (D1) |
| A conversation's name or creation time | labels it by the first line of its first `pi.user` entry (`client.md` §5.1); orders by ID, newest first | a workflow's role conversations are labelled by the workflow's instruction text (D2) |
| Who sent an input | every `pi.user` entry shows as **input**, never "you" | a workflow's input and yours look alike (D3) |
| A way to clear a fault or a failed run | hides either on this device (`x`); a failed run shows again when it fails again; a fault is hidden by itself after this device sends its answer again | another device still shows it (D4, D8) |
| A device list or a revoke call | none; the Server float names `tau device list` and `tau device revoke` | |
| A `stop` that names a run | `␣cx` only in the conversation's own view, while the drawn frame shows a run, and off after sending until the view shows that run gone or a new one | a second stop can still end a run that started in the gap |
| A `create` that is safe to repeat | `create` is never resent automatically, and `⌘⏎` is off while one is in flight (`client.md` §2.4) | an empty duplicate can stay; it shows as `empty conversation` |
| A time on a failed run | failed runs are counted, never timed | the since-line leaves them out |
| A tool call's start and end | a finished tool row shows its outcome word and no duration; a running row counts up from the `timestamp` of the `pi.assistant` message that made the call (D12) | the running count includes the time the model took to write that message |
| A pinned shape for `pi.usage` | not shown | no token or cost display in v1 |
| A distinct code for a refused `Host` | reads that one `message`, with a safe fallback (`client.md` §6.1) | fragile if the server rewords it |

---

## 3. Information model

### 3.1 Objects

| Object | Identity | Source | Where it lives |
|---|---|---|---|
| Server | its URL | config | the sidebar's server line; the Server float |
| Device | the name in the token (`laptop`) | `pair`; `by` on answers | the server line; `by` on answered decisions and notes |
| Project | `project` | `projects`, `bind` | Projects, **Project view**, the start line; a sage name on every row and over each group of open items |
| Extension | `name` | `extensions`; a project's `extensions` | the Project view; a slot on rows (`w1`) |
| Options | (project, extension) | `options`, `setOptions` | a form in the Project view |
| Conversation | (project, `#id`) | `conversations`, `open`, `history` | **Conversation view**; rows in Home and Conversations; an open item |
| Decision | (project, `subject#n`) | `inbox`, `decision`, `answer` | **Decision view**; Needs you rows; an open item |
| Event | (project, entry ID in the log) | `timeline`, `history` on the log | rows in Activity, Timeline, a conversation's details |
| Fault | (project, task ID) | `faults` | rows in Failed and Faults |

The screens you act in are the start page, Home, Conversation, Decision and Project. The other places (Conversations, Timeline, Faults, Projects) exist because Home shows only what changed or needs you.

### 3.2 Names the client derives, from core contracts only

- **Extension of a decision:** a subject is `${extension}:${key}` (`docs/core/primitives.md`, `Decision.subject`), so the text before its first `:`. `w1:3:design` → `w1`.
- **Request number:** a decision ID is `${subject}#${n}`; `#n` follows the title in rows and the sidebar (`Review the design #2`). `n > 1` means the subject was asked before; `#1 … #(n−1)` are readable with `decision`.
- **Extension of an event:** an event's `kind` starts with its extension's name and a dot (`w1.event`, `w2.asked`).
- **Extension of a conversation:** known when an event names the conversation (its kind's prefix), or once the client has seen its `pi.agent.extensions` name a workflow (`extensions` says which names are workflows). Unknown otherwise: no slot.
- **Label of a conversation:** the first line of the first `pi.user` entry's text, at most 80 characters, cut with `…` (`client.md` §5.1 has the query and the caching rule). While this device is sending a new conversation's first input from the start page, the first line of that text (§5.2). Otherwise none yet: `empty conversation` in `fg.muted`.
- **Owner of a conversation:** `conversations` gives `owner: {conversationId, taskId}` for an owned one; the client shows `owned by #<owner.conversationId>`.
- **State of a conversation:** from `conversations`; in a watched view from `pi.live`: `run` present is running, else `runFailure` present is failed, else idle (`packages/host/src/calls.ts:334-339`). A `runFailure` whose reason names an abort is shown as **stopped**, if a stopped run leaves one (§5.5).
- **Options that stop a workflow:** the server refuses `create` while the workflow's `options` item has any `errors` (`packages/host/src/calls.ts:287-298`). Those errors are every validation error of the stored value with the defaults filled, not only missing keys (`options.ts:66-69`, `:80-85`); the `options` call returns the same list (`calls.ts:200-217`). So the trigger is `errors.length > 0`, and the client names the errors in two kinds:
  - **missing keys:** the schema's `required` keys, minus the keys set in `value`, minus the keys with a `default`. The client computes this list itself; it does not read it from the `path` of the server's `required` errors, whose form is inferred (`fixture.json` `_about`). Words: `needs options: designer, implementer, reviewer`; in the Project view `3 to set`.
  - **values that don't fit:** every other error, named by its `path`, as `bad_options` errors are (§5.9). Words: `options don't fit: reviewer`; in the Project view `1 doesn't fit`.

### 3.3 What the client holds

| Held | From | Fresh by |
|---|---|---|
| Open decisions, all projects | `inbox` | live |
| Each project's events | `timeline` per project; older pages by `history` on the log | live |
| Conversation states | `conversations {project, limit: 500}`, every page | polled while focused |
| Faults | `faults {project}`, every page | polled while focused |
| An open conversation | one `open` for the conversation the stage shows, plus one for the peek; an open item that isn't shown reopens with `after` when shown (`client.md` §3.4), and its sidebar mark comes from the poll | live while shown |
| A decision record | `decision` | §5.6, "Refreshing" |
| Projects, extensions, models, options | the calls | on connect, on opening their views and pickers, after each write |

**Device-local state**, never sent (`client.md` §8): the server URL and device name (`~/.config/tau/client.toml`); the token (macOS Keychain); the theme, the reading width, motion and any key moved (`settings.toml` beside `client.toml`, changed in the settings float, `␣hs` `:settings` `⌘,`, and by `␣ht`); the sidebar's visibility and the details panel's; the open items, their order and each one's cursor; each place's filter and cursor; the last project used and the last workflow per project; the start page's draft; each conversation's composer draft; each decision's note drafts; the outbox; the label cache; hidden faults and hidden failed runs; the since-you-left base; the jumplist. `fixture.json` → `local` shows one instance.

---

## 4. The frame

Window 1512 × 982 logical px (a 14″ MacBook Pro, maximised), with the sidebar shown. The smallest window is 800 × 600.

| Region | Box (x, y, w, h) | Surface | Holds |
|---|---|---|---|
| Sidebar | 0, 0, 248, 956 | `surface.side` | the window controls; brand and server; `+ New`, `Find`; the places; `OPEN`, the open items by project; `Server` (§4.1) |
| Stage header | 248, 0, 1264, 44; a 1 px `border.hairline` at its foot | `surface.read` | left: the sidebar toggle and the breadcrumb; right: the view's action segments (§4.2) |
| Stage | 248, 44, 1264, 912 | `surface.read` | one page |
| Notice band (when a state needs one) | top of the stage, its width, 38 px for one line, 56 px when its sentence takes two; pushes the page down | `surface.side` | offline, refused, revoked, superseded, a stream that keeps failing |
| Peek (Home and the places, stage ≥ 1000 px) | 1072, 44, 440, 912; a hairline at its left and no line inside | `surface.read` | the cursor row, live (§5.4) |
| Details panel (`␣td`) | 1272, 44, 240, 912; a hairline at its left | `surface.read` | a view's secondary facts (§4.5) |
| Bottom float | 260 to 1500, foot at y 946, up to 40 % of the window tall | `surface.raised`, the float shell (§6) | which-key, palette, `:` line, settings, find, pickers, filter, bind, confirm, readback, outbox, Server, the peek float |
| Bottom bar | 0, 956, 1512, 26 | `surface.bar` | mode badge · pending keys · echo · save state of option drafts; at its right end, while the sidebar is hidden, the count tags and any abnormal server state (§4.3) |

With the sidebar hidden, the stage and its header start at x 0; the window controls sit at the header's left and its content starts at x 88. A float covers the stage only, never the sidebar or the bottom bar; the page is clipped at the float's top edge, and the insets beside and under the float show `surface.read`.

**Every bottom float keeps what you act on in view.** When a float opens, the page scrolls so the cursor's row or target sits fully above it; in a conversation the queue, the unsent rows and the composer lift above it too.

### 4.1 The sidebar

248 px, full height above the bottom bar, square. Top to bottom:

1. **The window controls** at x 16, y 16.
2. **Brand and server,** from y 44: the avatar (`brand/tau-icon-small-coral.svg`, 24 px) and beside it two lines, `tau` (`sidebar.brand`) and the **server line** (`sidebar.server`, 11.5/16). A click on the server line opens the Server float.
3. **`+ New`** with its chip `␣cn`, and **`Find`** with its chip `␣.`, at the right. `+ New` is the current row while the start page shows.
4. **The places:** **Home**, with the count tags at its right (§4.4); **Conversations**; **Timeline**; **Projects**. Faults has no row: it is reached with `␣of` and from the overflow line of Home's Failed group, and while it shows, Home is the current row.
5. **`OPEN`** (`sidebar.section`, 24 px above), then the open items, grouped by project. Each group opens with the project's name in sage (`sidebar.group`, 22 px). Each item is one 28 px row: a kind icon (`chat` conversation, outline diamond decision, `folder` project), a label cut with `…`, and a small state mark at the right. The section scrolls; everything above it and the `Server` row stay put.
6. **`Server`** at the bottom (`settings` icon): opens the Server float.

**Rows** are 28 px, `ui` in `fg.alt`, with a 16 px icon in `fg.muted` (14 px for open items). **The current row** (what the stage shows) has a neutral `surface.cursor.side` fill, `fg` text and no bar.

**Open items.**

| Item | Label | Mark at the right |
|---|---|---|
| Conversation | `#4` (orange) and its label: `#4 Port the config loader…` | running: steel triangle; failed: rose `×`; stopped and idle: none |
| Decision | its title and `#n` (orange): `Review the design #2` | open: coral diamond; answered, superseded, withdrawn: none; its `then` task faulted: rose `×`, following Failed, so a fault hidden on this device draws none |
| Project | the project's name | none |

- **Opening adds an item.** Showing a conversation, decision or project in the stage adds it, whatever opened it: `⏎` on a row, `gd`, `␣n`, `]q`, Find, `␣po`, or the start page sending. An item already open becomes current; it never appears twice. The peek adds nothing.
- **Order.** Groups appear in the order their first item was opened; items within a group in the order they were opened. A new item joins the end of its group; a new project's group joins the end of the section. Reopening moves nothing.
- **Closing.** `x` on an item in the sidebar, or `:q` on the one the stage shows. Drafts stay on this device. Closing the shown item shows the next item in the sidebar's order, else the previous one, else Home. A group with no items left goes. Places can't be closed.
- **Items stay** until you close them, whatever happens to their object: an answered, superseded or withdrawn decision keeps its item and loses its mark; an object the server no longer has keeps its label in `fg.muted`, and the stage reads `gone from the server`.
- **They survive** a restart (`client.md` §8).
- **Cycling:** `]b` and `[b` show the next and the previous open item in the sidebar's order, wrapping; while the stage shows a place, they count from the item shown last. The places are not in the cycle.
- **None open:** `OPEN` and its section are left out.

**Focus.** The stage has the keys by default.
- `C-h` gives them to the sidebar (window left). Its cursor starts on the current row and takes the 2 px coral bar; the stage's cursor keeps its band and loses its bar (a Needs you head turns from solid coral to the neutral band).
- In the sidebar: `j k gg G` move over every row, `+ New` to `Server`; `]g [g` move between its sections (the top rows, the places, each project's group, `Server`); `⏎` or `l` opens the row and gives the stage the keys (`+ New` and `Find` run `␣cn` and `␣.`); `x` closes the open item under the cursor; `K` on an open item opens its peek in the float; `C-l`, `Esc` or `q` give the stage the keys back without opening anything.
- Any command that changes what the stage shows (`␣oh`, `]b`, `␣cn`, a float's choice) gives the stage the keys. The badge stays `NORMAL` in the sidebar.
- A sidebar that is not drawn (hidden by its stored choice, or under a 1000 px window) is drawn while it, or a float opened from it, has the keys: over the stage's left 248 px, with a hairline at its right, and nothing reflows. It goes when the keys go back to the stage.

**Hiding.** The sidebar slides on a 200 ms spring when a key or press shows or hides it. Beside the stage, both move together. Over the stage, the sidebar stays opaque during exit. Motion off and Reduce Motion make this instant (`reference/visual-language.md` §7). `␣tb` (or the toggle in the stage header) shows and hides the sidebar; the choice kept on this device is its only state. Under a 1000 px window the sidebar is not drawn whatever the choice, and `␣tb` does what `C-h` does (§4.7). While it is not drawn, the count tags and any abnormal server state sit at the right end of the bottom bar (§4.3), so nothing that needs you and nothing wrong with the link is out of sight.

**Server line.** Calm, `fg.muted`: `laptop → tau.tail4e2f.ts.net ⌄`; the host is cut at its end with `…` when it doesn't fit. Before pairing: `not connected`. Not calm, `fg` with its mark: `○ connecting` (from launch until the first `inbox` frame, §4.8), `○ reconnecting in 4 s`, `○ offline 6 m`, `◷ server stopped · retrying`, `× refused by the server`, `× revoked`, `× server speaks protocol 2`. The Server float has the detail (each stream and its state).

### 4.2 The stage header

44 px, the stage's width, a hairline at its foot.

**Left:** the sidebar toggle (16 px `side-panel--close` while the sidebar shows, `side-panel--open` while hidden, `fg.muted`), then the **breadcrumb** (`ui`, separators `›` in `fg.muted`). The breadcrumb lives only here. Connect draws neither.
- Start page: `New conversation`.
- Home: the group and the cursor's place in it (`Home › Needs you › 1 of 3`). Home has no filter, and the breadcrumb never claims one.
- A place: its filter (`Timeline › all projects`, `Conversations › render-core · failed`, `Projects`).
- A view: its object (`notes-cli › #4`, `render-core › w1:3:design#2`, `Projects › render-core`). The project is sage, `#4` and the decision ID orange mono.

**Right:** the view's action segments, 16 px apart: quiet `ui` in `fg.alt` with a key chip.
- Conversation: running `␣cx stop`; failed or stopped `␣cc continue`; idle none.
- Decision: open `◆ ␣ad Answer`: solid coral (`radius.pill`, 26 px tall, text knocked out in `surface.read`) only when `␣ad` would open the readback (§4.6); otherwise a muted outline (`border.hairline`, `fg.muted` text and chip). `␣ar send again` (quiet) when it is live (§5.7).
- Every other page: none.

A disabled segment reads `<verb> · off` with a dashed `border.hairline` edge and no chip; a write segment is also off while the view's stream is down (§4.9). A segment that is on but whose key doesn't resolve where the keys are is muted (§4.6).

### 4.3 The bottom bar

One 26 px bar on `surface.bar` across the window. From the left:
1. **The mode badge** (`reference/grammar.md` §2), 26 px, square.
2. **Pending keys** (orange `mono.small`), while a sequence waits.
3. **The echo** (`ui.small` `fg`): one transient message. It never repeats the bar, prints no chips, and is cut with `…` before the right-hand segments. `:messages` keeps the history. Anything the user must act on is a notice band, never only an echo.
4. **At the right:** the save state of option drafts (`✎ 2 not saved :w`); then, while the sidebar is hidden, the count tags (§4.4) and, when not calm, the server state in the server line's words.

### 4.4 Count tags

`radius.pill`, 16 px tall (18 px on Home's title), at least 30 px wide; mark and number in Iosevka 11.5/16 500, knocked out in `surface.read`. Each shows only while above 0, and a count above 0 always has exactly one visible tag: at the right of the sidebar's Home row, or at the bottom bar's right end while the sidebar is not drawn. A count not yet received draws no tag; the server line's `○ connecting` says why (§4.8).

- **`◆ 3` coral:** open decisions, all projects (the inbox's size). It equals Home's title count. Click: the oldest.
- **`× 4` rose:** exactly the count in Home's Failed head: failed runs and faulted tasks, minus those hidden on this device. Click: Home at Failed, whose head it equals; Faults is one step on.
- **`? 1` mauve:** outbox entries with an unknown outcome. Click: the outbox.

### 4.5 The peek and the details panel

**The peek** is the right column of Home and the places while the stage is at least 1000 px wide: 440 px, a hairline at its left and none inside, 28 px padding. It has no cursor and never takes the keys. Narrower, `K` opens it as a float (§5.4); wider, `K` on a list row does nothing.

**The details panel** is a view's secondary facts, at the stage's right edge: 240 px, a hairline at its left, 24 px padding, groups under `group` heads; each label (`ui.small` steel) sits over its value (`ui.small`), so a model ID never breaks. `␣td` or `details ›` in the title block shows and hides it; it is hidden at first, and the choice is kept on this device for every view. With it open, the view's column centres in the stage left of it.
- **Conversation:** **Settings**: the model in full (`anthropic/claude-sonnet-4-5 · Claude Sonnet 4.5`), the thinking level (`model default` when `pi.agent` sets none), tools worded from `pi.agent.extensions` (absent `project default`; `{add:["w1"]}` `project default + w1`; a list `only coding-tools, w1`), `cwd` as `…/leaf`, and, when the selection names a workflow, `w1 may set the model again when it next sends work here`. **Decisions**: open decisions whose `conversation` is this one (from the inbox), `◆ title #n`. **Events**: the newest 5 that name it, time and text, cut with `…`.
- **Decision:** **About**: `extension w1`, `conversation #3`, `basis entry 34 · 2,203 characters`, `then w1.design_answer`.

Links in the panel and in facts lines (`#3`, `owned by #3`, a decision, an event) are hint targets (`gs`) and clickable; `gd` in a decision opens its conversation, and in an owned conversation its owner.

### 4.6 Keys, coral and muting

One region has the keys: the stage, the sidebar, or the top float. Two rules follow from it, and every other place in this package cites them.

**One coral cursor.** Only the region that has the keys draws its cursor in coral: the 2 px bar, the Needs you head, or a caret with its container's coral edge. Every other cursor keeps its band and draws it neutral.
- A float that takes the keys turns the stage's and the sidebar's cursors neutral. If it has a selection (a picker, the bind or add-extension checklist, the answer readback, the outbox), that selection is the one coral cursor. If it has none (the send-again readback, the not-recorded reply, Confirm, Server), no coral cursor shows; its coral top rule marks it.
- The peek float's `j k` move the list's or the sidebar's cursor, so that cursor is its selection and stays coral.
- which-key takes no keys and leaves every cursor as it is.
- Outside a float, no coral cursor shows in two cases only: while the start page is `starting…`, when its text is read-only and its edge is back to the hairline (§5.2), and while nothing has been received yet, when there is nothing to point at (§4.8).

**Lit or muted.** A chip, hint or segment is lit only when its key resolves for the region and the mode that have the keys; otherwise it is `fg.muted` and inert. A lit chip keeps its chip colours; a lit hint is `fg.alt`. So:
- in Insert every leader chip is muted (the sidebar's `␣cn` and `␣.`, the header's segments), and a container's own hints are lit;
- with the sidebar holding the keys, the header's segments are muted (`␣cx`, `␣cc`, `␣ad`, `␣ar` don't apply there);
- under a float that takes the keys, everything outside it is muted.

**The coral Answer** is solid only when `␣ad` would open the readback: the decision's stage has the keys in Normal mode, no float is open, and the decision is open with `␣ad` on. Otherwise it is its muted outline.

### 4.7 Widths

- **The smallest window** is 800 × 600.
- **The sidebar** is drawn when its stored choice says so and the window is at least 1000 px wide. Otherwise it is drawn only while it, or a float opened from it, has the keys, over the stage (§4.1); `␣tb` under 1000 px does what `C-h` does. The stored choice is the only state: no width ever changes it.
- **The peek** is a column while the stage is at least 1000 px wide (a 1248 px window with the sidebar, 1000 px without); narrower, it is the `K` float.
- **Home and the places:** the list column runs from 40 px inside the stage's left edge to 40 px before the peek's hairline (at 1512 with the sidebar: window x 288 to 1032). The cursor band runs 12 px beyond the column on each side.
- **Row slots** (§5.3) are laid out once for Home's Failed and Running together, so they stay aligned at every width. When a row's label would get less than 200 px, the slots drop whole, in this order: time, model, extension, reason. The model and the reason share one column, so while Failed shows, that column stands until the reason drops. At 1300 px with the sidebar, both groups keep project, `#id` and the reason / model column; Running alone keeps project, `#id` and extension. In a faulted task's label, `after <decision ID>` shows whole or not at all, and the kind is cut first.
- **Conversation:** an 880 px column, `min(880, stage − 32)`, centred left of the details panel; prose in it wraps at 840 px.
- **Decision:** the 840 px text column, a 48 px gap and the 320 px notes margin centre together (1208 px). When the stage left of the details panel is under 1256 px, the margin folds: its head, the note on the whole and the Earlier lines sit under the facts line, in that order, and each passage's note sits under its paragraph; the text column is `min(840, stage − 32)`.
- **Project view:** a 600 px column, `min(600, stage − 32)`.
- **Start page:** the composer is 640 px, `min(640, stage − 32)`.

### 4.8 Nothing received yet

The client keeps no server state across launches, so from launch until the first `inbox` frame it knows nothing about decisions, and until the first `conversations` and `faults` polls nothing about failed and running work. It says so instead of drawing calm:
- the server line reads `○ connecting`; that isn't calm, so it also shows at the bottom bar's right end while the sidebar is not drawn;
- no count tag is drawn for a count not yet received;
- Home's title is `Home`, and its groups are replaced by one line, `Nothing received from the server yet` (`ui`, `fg.alt`). Once the inbox has arrived, Needs you shows as usual and the line stays in place of Failed and Running until the first poll; Activity shows each project's events as their `timeline` arrives;
- the places read the same line until their data arrives, and open items draw no mark;
- if the link fails before anything arrived, the offline band drops `showing the state as of then` (`client.md` §3.7).

### 4.9 A view whose stream is down

While the stream a view depends on is down (a conversation's `open`; a decision's `inbox`), the view shows its last state, marked: its write segments read `· off` (`stop · off`, `continue · off`, `Answer · off`), its state facts end `as of 16:41`, and a running tool call's count stops. An input typed meanwhile still goes to the outbox (`client.md` §2).

---

## 5. Screens

Each screen: the question it answers, what it shows, the calls and streams behind it, its keys, its states. A view opens with its cursor on the first line of its content, or where this device left it on that open item.

### 5.1 Connect (first run, and after `:forget-server` or a revoke)

**Answers:** which server, and is this device allowed on it.

- The sidebar is not drawn: there is nothing yet to list. No sidebar toggle, no breadcrumb, no count tags; the bottom bar reads `INSERT`.
- A centred 600 px column: the wordmark (`brand/tau-wordmark-on-dark-coral.svg`, `-on-light-coral` in Ember Light, 44 px tall), then the page title `Connect to a tau server` (`title.page`).
- Field **Server**, prefilled `http://127.0.0.1:6283`. Field **Pairing code**, `XXXX-XXXX`. Both are text fields (`reference/visual-language.md` §1.1).
- One paragraph (`prose`, `fg.alt`): "On the server's machine, run `tau device pair <name>`, with a new name for this device such as `laptop`; a name is never used twice. It prints a code that works once, for 10 minutes. The token it gives this device is kept in the macOS Keychain."
- The form's footer segment `⏎ connect` (neutral; a form footer prints its own keys, as a composer's does).
- **Plain HTTP to another machine:** an `http://` URL whose host is not `127.0.0.1`, `localhost` or `[::1]` shows the warning of `client.md` §7 under the field. `⏎` then opens the Confirm float, `Pair over plain HTTP? Anyone on the network path can read this device's token.`, hints `y pairs · n cancels`; `y` there pairs. The form never prints `y`, so a `y` typed in a field is a letter.
- **Calls:** `pair {code}`; then `POST /v1/` (expects `unknown_call` and `Tau-Protocol: 1`); then `projects`; then Home, with the sidebar.
- **States** (copy in `client.md` §6–§7): wrong or expired code; Host not allowed; no server at the URL; a lost `pair` reply; another protocol.
- **Keys:** Insert on open; `Tab` / `S-Tab` between fields; `⏎` connects.

### 5.2 The start page (`␣cn`)

**Answers:** what do you want to work on, and where.

A page in the stage, not a float. The app opens on it (D11). Breadcrumb `New conversation`; no actions; the sidebar's `+ New` is the current row.

**Layout.** Only three things, centred in the stage:
- the wordmark (`tau-wordmark-on-dark-coral.svg`, `-on-light-coral` in Ember Light), 44 px tall, 36 px above the composer;
- **the composer**, the container style, 640 px wide and at least 112 px tall, growing to 12 lines and then scrolling; its top edge at 0.325 of the stage's height below the header (296 px at 982). Placeholder `What do you want to work on?` in `fg.muted`. In Insert its edge is coral and `⌘⏎ start` sits inside at the bottom right (`hint`);
- **the start line**, 12 px under the composer (`ui.small`): a `folder` icon, the project (sage) and its hint `M-p`, `·`, then the workflow, `w1` (`mono`, `fg`) and `workflow` (`fg.alt`), and its hint `M-w`: `render-core M-p · w1 workflow M-w`. For the bare agent the line names the model: `notes-cli M-p · bare agent · claude-sonnet-4-5 M-w`. The two hints are lit in Insert and in Normal: both keys work in both modes (**Keys**, below).

**What it starts.** A start page with no draft takes:
- **the project:** the one the current view names (a Project view, a conversation, a decision, or the sidebar item under the cursor when the sidebar has the keys), else the last project used on this device, else the only project. `␣pa` and `␣po` take their project by the same rule. With none of these, the line reads `choose a project M-p`, and `⌘⏎` opens the project picker and keeps the text.
- **the workflow:** the last one used in that project on this device; else the project's only workflow; else the bare agent, with the model last used for a bare agent in that project, else the first that `models` lists. With no models at all, the line reads `bare agent · no models on the server`, and `⌘⏎` echoes it.

The page keeps its project, workflow and text as one draft on this device until a conversation starts from it; `␣cn` returns to that draft. Opening it from any view that names a project sets that project and keeps the text.

**Before you type, the line says what can't start:**
- a workflow whose options have errors (§3.2): `scratch M-p · w1 workflow · needs options: designer, implementer, reviewer M-w`, or `· options don't fit: reviewer` (the last part in rose). The composer's hint reads `⌘⏎ set its options`; `⌘⏎` opens the Project view at the first field with an error and keeps the text.
- a workflow the server lacks: `extensions` lists only what the server has, and `projects` gives a project's `missing` as bare names, so the workflow picker never lists a missing extension. The line reads `· missing on the server` (rose) only when the remembered workflow (`lastWorkflow`) is in the project's `missing`; `⌘⏎` then echoes `the server has no code for w1`.
- no projects at all: the line reads `No projects yet · ⌘⏎ binds one`, and `⌘⏎` opens the bind float.

**Sending.** `⌘⏎` sends what the drawn line names (keys act on the drawn frame):
1. Empty or white-space text: nothing happens. Text over 1 MiB: echo `Over 1 MiB · nothing was sent`; nothing is sent.
2. `create {project, workflow}`, or `create {project, model}` for the bare agent. From the press until `create` replies or times out, the composer's hint reads `starting…`, the text is read-only, and `⌘⏎`, `M-p` and `M-w` are off; auto-repeat never fires `⌘⏎`.
3. On its `result`, the page becomes that conversation in place: the breadcrumb reads `render-core › #9`, your text moves to the top as the unsent row `sending…`, the composer drops to the column's foot, empty, and `#9` joins the sidebar under its project, current. The mode is Normal, the cursor is on the unsent row, and follow is on. Until the label is known, the title is the first line of the text being sent.
4. `input {project, conversation, text, requestId}`, with the outbox's rules (`client.md` §2).

If the stage no longer shows the start page when the `result` comes, `#9` joins the sidebar, the `input` is sent, and the stage stays where it is.

The draft clears when `create` succeeds. This device then remembers the project as the last used and the workflow (or the bare agent and its model) as that project's last, and forgets any earlier `create` entry of the start page left in the outbox without a reply.

**Failures** (`client.md` §2.4):
- `create` with no reply: never resent. The page stays, the text stays in the composer, editable again, and a line under the start line reads `? No reply · a conversation may have been created; it would show as empty conversation in render-core`. The outbox holds the entry (`? 1`), with no `␣rt`. `⌘⏎` again tries once more, `create` then `input`; if the first `create` had landed, there are now two conversations.
- `create` refused: the page stays with the text, and the refusal's copy (`client.md` §6) shows under the start line. `options_invalid` reads `w1 can't start: its options are not all set or don't fit` and opens the Project view at the first field with an error.
- `input` with no reply, after `create` succeeded: the page is already the conversation; the unsent row reads `no reply · resends when connected` and resends with the same request ID.
- `input` refused: the text returns to the conversation's composer as a draft, with the refusal's copy above it; the conversation stays, empty, and its title reads `empty conversation`.
- Offline: `⌘⏎` echoes `offline · start when connected`; nothing is queued, because `create` isn't.

**Calls:** `projects`, `extensions` and `models` when the page opens; `options {project}` when its project is set or changes; then `create`, `input`.

**Keys:** Insert on open. `⏎` is a new line; `⌘⏎` starts; `M-p` opens the project picker; `M-w` opens the workflow list (Option-p and Option-w can't type `π` and `∑` here). In both modes they are the commands `start.project` (`:project`) and `start.workflow` (`:workflow`). `Esc` or `jk` leaves Insert; in Normal the composer's hint reads `⌘⏎ start · i writes`, and `i a o` return to Insert.

### 5.3 Home

**Answers:** what needs me, what broke, what runs, and what happened while I was away. Once you are there it needs no key. The app opens on the start page (D11), so at launch Home is `Esc ␣oh` away, and the since-line is first seen there. Home always shows every project; each row names its project.

**Title and Needs you.** The page title (`title.page`, coral) is the Needs you group's head and carries its count once, as the coral count tag at 18 px after it: `Needs you ◆ 3`. Calm: `Nothing needs you`, no tag, and no Needs you rows. Before the first `inbox` frame: `Home`, and the line `Nothing received from the server yet` in place of the groups (§4.8).

**Since-line** under the title (`ui.small`, `fg.alt`, steel `time` mark): `Since 12:30 on this device · away 4 h 10 m · 3 new for you · 1 new fault · 5 events`. When it doesn't fit, it drops whole parts from its end, never a cut word.
- **Base:** the moment this window last lost focus or the app quit, stored with the newest event ID per project. The line shows when the window gains focus after at least 5 minutes away, and stays until it loses focus again. Never left: no line.
- **Counts** only what carries a time: decisions with `requestedAt`, faults with `endedAt`, events with `at` after the base. Failed runs carry no time and are not counted.
- **New event rows** take `ui.strong` until the cursor passes them. Bold means "new since you left" and nothing else in lists.

**Groups,** in `j` order, each head 26 px below the group before:

| Group | Rows | Order | Row form |
|---|---|---|---|
| **Needs you** (under the title) | every open decision (`inbox`) | oldest `requestedAt` first | a 26 px head and a 22 px byline, 6 px between entries. Head (`move`): the coral diamond, the title, `#n` (orange `mono.small`), the age at the right (`mono.small` `fg.alt`). Byline (`ui.small`, 22 px in): project (sage) · subject (`mono.small` `fg.alt`, `w1:3:design`) |
| **Failed N** | failed runs and faulted tasks, minus those hidden on this device | faults that name a decision; then failed runs (project order, newest ID first); then other faults, newest first. At most 8 rows; then one line per kind that overflows: `2 more failed runs · in Conversations` (opens Conversations filtered to failed) and `1 more faulted task · in Faults` (opens Faults) | one 28 px line: the rose `×`, the label, then fixed slots at the right: project (sage, 96), `#id` (orange, 26), extension (`mono.small` `fg.muted`, 30), reason (`run failed` / `task faulted`, `fg.alt`, 132), time (`mono.small` `fg.muted`, 84, right-aligned; faulted tasks only). A faulted task's label is its kind (`mono` orange) and, when it names one, `after w1:6:design#1` (`mono.small` `fg.alt`); that part shows whole or not at all, and the kind is cut first |
| **Running N** | running conversations | project order, newest ID first | one 28 px line: the steel triangle, the label, then project, `#id`, extension, and the model (`fg.alt`, in the 132 px column Failed's reason uses) when known. The core gives no time; while Failed shows, Running keeps Failed's time slot empty so both groups stand on one grid |
| **Activity** | events of every project | newest `at` first; at most 12, then `N older in the timeline` | one 28 px line: time as the Timeline writes it (`15:02` today, `Nov 14 19:02` on another day; `mono.small` `fg.muted`) in a column as wide as the widest time in the group, so every text starts on one line; text (`ui`), project (sage), `#id` (orange) when the event names a conversation |

Slots are fixed columns 12 px apart; the label takes the rest and is cut with `…`. Failed and Running share one grid: project, `#id`, extension and the reason / model column stand on the same lines in both groups. A row with a short reason leaves a gap before the time, on purpose. Slots drop by the rule of §4.7. The mark, the label, the project and `#id` never drop. An overflow line starts where the text of the rows above it starts: `2 more failed runs · in Conversations` on Failed's label column, `N older in the timeline` on Activity's text column. Rows have no rules between them.

- **Rows are plain text.** Extension and agent text in a row (titles, labels, event text, fault errors) is one `ui` line: no Markdown, no bold, control characters escaped, cut at the end with `…`. Paired backticks of an inline code span are dropped, so `` `notes.json` `` reads `notes.json`. The client's mark and `#id` sit in fixed slots, so text can't pose as a head or a tag.
- **The cursor:** on a Needs you entry, while the stage has the keys, its head is a solid coral band, text and diamond knocked out in `surface.read`, with the `⏎` chip at the right, and its byline sits on `surface.cursor.read`. On other rows: `surface.cursor.read` with the 2 px coral bar. Without the keys: the band alone (§4.6).
- **Hiding a failure** (`x`): a faulted task stays hidden on this device; a failed run stays hidden while its state stays failed, and shows again once a poll has seen it running or idle and it fails again. `␣th` shows hidden ones in the places.
- **Empty groups** are left out, except Activity, which reads `No events yet. Bare conversations write none.` No projects at all: `No projects yet · ␣pb binds one`.

**Calls and streams:** `inbox`, `timeline` per project, `conversations` per project (polled), `faults` per project (polled), `history` for missing labels; the peek adds one `open` or one `decision`.

**Keys:** `j k`; `⏎`/`l` open; `]g [g` groups; `]q [q` decision rows; `K` the peek float under a 1000 px stage (wider, the peek is already there and `K` does nothing); `x` hide the failure under the cursor (this device); `␣n` the oldest decision; `gd` on an event or fault row opens its conversation.

### 5.4 Peek (Home and every place)

**Answers:** what is this row, without opening it. It follows the cursor after the cursor rests 300 ms. While the stage is at least 1000 px wide it is the right column; narrower, `K` opens it in the float with its own keys (`j k` move the list cursor and the float follows, `⏎` opens, `K q Esc` close). `K` on an open item in the sidebar opens the same float.

| Row | Peek shows | Calls |
|---|---|---|
| Decision | title + `#n`; facts `render-core · w1:3:design · requested 13:55 · in #3 · basis entry 34`; the choices separated by `·`, wrapping onto more lines when they don't fit; `basis` and its size (`2,203 characters`); then the basis as Markdown in `prose.small` on the 2 px content bar, cut at the peek's foot with `the rest is in the decision` | `decision`, cached per ID, refetched when the inbox changes the subject |
| Conversation | label; facts (project, `#id`, model, thinking, tools selection); state; then the live tail: the last agent text block, the running tool with its last 4 output lines, `1 queued · steer`; failed: `runFailure` reason and message | one `open`, closed when the cursor leaves |
| Event | the event text, kind (`w1.event`), time, project, `by task 18` when the entry has `byTaskId`; then the named conversation's peek | `open` for that conversation |
| Fault | kind, task ID, the whole error in `mono`, conversation, `endedAt`; if it names a decision: that decision's title, choice, `by`, `at`, and `Open the decision to send the same answer again. It starts the work after it again.` | `decision` when named |
| Project | workdir, extensions with kind, options state per extension (`all set` / `3 to set`), missing extensions | `options` |

**Markdown in the peek steps down** so it stays a preview: `h1` 14/20 700 rose with its rule, `h2` 13/19 600 orange, `h3`–`h6` 13/19 600 `fg`, body `prose.small` 13/19.

Offline, the peek's state facts end with `as of 16:41`; ages keep counting (`client.md` §3.7).

### 5.5 Conversation

**Answers:** what this agent is doing, and how I steer it.

**Header:** breadcrumb `notes-cli › #4`; running `␣cx stop`, failed or stopped `␣cc continue`. No coral: nothing in a conversation is a decision. `␣cx` and `␣cc` work only here, in the conversation's own view, where `pi.live` is live; while its stream is down they read `· off` (§4.9).

**Title block,** 28 px below the header: the title is the conversation's label (`title.page`, coral, one line, cut with `…`), so the first input stays at the top even when it has scrolled far away. Under it, 4 px down, one facts line (`ui.small` `fg.alt`) with `details ›` at its right end (`details ‹` while the panel shows):
- running: `▶ running · claude-sonnet-4-5 · turn 6` (steel triangle; the turn from `pi.live.run.turns`);
- idle: `idle · claude-sonnet-4-5`;
- failed: `× run failed · model_error · claude-sonnet-4-5` (rose `×`, words in `fg.alt`);
- stopped: `stopped · claude-sonnet-4-5` (below);
- owned: the line ends `· owned by #3`.

The project and `#4` are in the breadcrumb, and the rest of the settings in the details panel (§4.5). The transcript starts 24 px below.

**The cursor.** Inside message text (your message, agent text, a record's body) the cursor is a line: a full-width band with the 2 px coral bar, as in any reader, so `v`, `V`, the motions and the yanks work on text. On a tool row, a fold, a thinking fold or a header line it is the whole row. Band and bar are as `reference/visual-language.md` §1 says, and the bar is drawn inside the column, never in its outset. `]m [m ]t [t` move it by entry and by tool call.

**Transcript** (the 880 px column; prose wraps at 840):

| Entry | Renders as |
|---|---|
| `pi.user` | **your message**, the container style on `tint.human`: a header line, `input` (steel 600) and its time (`mono.small` `fg.muted`); the body as Markdown with single line breaks kept; tags such as `<design>` show as literal text. Left-aligned like everything else |
| `pi.assistant` | a header line, `agent` (`fg.alt`) and its time, and the model only when it differs from the facts line's; no box. Consecutive agent entries with no input or record between them share one header (one turn). Text blocks as Markdown; `thinking` blocks folded to `thinking · 214 words`; each `toolCall` becomes a tool row in its place |
| `pi.tool-result` | not drawn alone; it sets the outcome of the tool row with the same `toolCallId` |
| `pi.system` | one chrome line, `fg.muted`: `tools: read, bash, web_search, publish_design` |
| any other kind (`w1.design`, `w2.forecast`) | a **record**: a header line with the kind (`mono.small` orange) and `by a task` when it has `byTaskId` (`K` gives the task ID); the body is `data.text` as Markdown when it is a string, else `no text`; no box. `K` shows the entry's `data` as JSON |

**Tool rows:** one 30 px line each, 4 px apart, with no box: a small mark, the tool name (`mono` `fg`), the arguments (`mono` `fg.alt`, cut with `…`), and at the right the outcome word (`ui.small`) and a 44 px slot (`mono.small` `fg.muted`, right-aligned) that only a running row fills, then a chevron (`fg.muted`). The outcome comes from the core's records only, never from parsing output:
- a `pi.tool-result` with `isError: false` → olive `checkmark`, `ok` (olive text);
- `isError: true` → rose `×`, **`failed`** (bold, rose), and the row on `tint.row_fail` (`radius.pill`, no edge), the only tinted row;
- an item in `pi.live.tools` → steel triangle, `running` (`fg.alt`); its output shows open, the last 6 lines, streaming;
- a call with neither while no run is live → mauve `help`, **`no result`** (bold, `fg`).

**Duration (D12):** a finished row shows its outcome word and no duration. A running row counts up from the `timestamp` of the `pi.assistant` message that made the call: `4.2 s` (one decimal under 10 s), `12 s`, `17 m`; while the view's stream is down the count stops (§4.9). Two or more adjacent `ok` rows fold into one, `✓ 2 tool calls · read, bash   ok`, with no figure; failed, running and no-result rows never fold. A press, Enter or `za` opens and closes a tool row, tool group or thinking fold and keeps the cursor on it. Enter acts only in the reader. `]t [t` move. **Opened input and output** sit under the row in the container style on `surface.sunken`, in `mono.output`, showing the bytes as sent (Iosevka draws `ℹ` as a bold i in its cell; the client substitutes nothing). The input appears above the output when the summary omits fields or lines, exceeds 48 characters, or falls back to JSON for a non-empty object. A sole multiline string shows as itself; other omitted arguments show as pretty JSON. When both blocks show, small muted `input` and `output` labels tell them apart.

**Live run.** `pi.live.generation.message` renders as an agent message in progress with its own header, `agent · writing` (and the model when it differs); its last block is `fg.muted` (the streaming tail, never coral). When it is committed as an entry it joins the turn's header.

**Run failure** (`pi.live.runFailure`), after the last entry: a head line on `tint.row_fail` with the rose `×` and `Run failed · model_error` (`fg` 600); under it, unboxed, the message in `mono.small` and `␣cc continue runs it again on the same context`. It goes when a new run starts.

**A stopped run.** Not verified: whether `stop` leaves a `runFailure`. If it does, `conversations` reports the conversation `failed`. The client then reads the reason (one `open` of that conversation, closed after its first frame) and, when the reason names an abort (`aborted`), shows **stopped**: no rose mark, not in Failed, not counted by `×`; the facts line reads `stopped`, the header `␣cc continue`; the line after the last entry reads `Stopped · continue runs it again on the same context`, untinted. If a stopped run leaves no `runFailure`, it is simply idle. This is a first implementation test (`open-questions.md` §3).

**Queue** (`pi.inbox.items`), directly above the composer: the mode word and its state (`steer · queued`, `follow-up · queued`, steel and `fg.muted`) and the text on the content bar (`prose.small`, inline Markdown, two lines at most). Under the last: `queued inputs can't be taken back`. A row leaves when the core places it.

**Send motion.** A locally accepted send enters above the composer with an 8 px rise and fade on a 180 ms spring. The composer shrinks and the transcript moves with it. A refused send keeps the draft and starts no motion. The server's replacement of the unsent row with a transcript message is instant (`reference/visual-language.md` §7).

**Unsent rows,** between the queue and the composer: `sending…`; on an unknown outcome the mauve `?`, the mode word, the text on the content bar, and `no reply · resends when connected` (or `not sent automatically · ␣rt retries`, `client.md` §2.2).

**Composer,** pinned 16 px above the stage's foot, the column's width: the container style, with a caption inside at the top (`ui.small` steel, the mode word 600) that says what `⌘⏎` does, from the drawn state:
- idle: `send · starts a run on claude-sonnet-4-5`;
- running: `steer · joins this run after the current tool round`;
- failed: `send · starts a new run · the last run failed`.

**⌘⏎ sends** in both the composer's Insert and Normal modes; **M-⏎** queues a follow-up. On empty or white-space text neither does anything. **Bare `⏎` never sends from the composer:** in Insert it is a new line; in the composer's Normal mode it does nothing. In Insert the edge is coral and the hints inside at the bottom right read `Esc jk leave Insert · ⌘⏎ steer · M-⏎ follow-up` (running) or `Esc jk leave Insert · ⌘⏎ send`; in the composer's Normal mode `⌘⏎ send · Esc back to the conversation`; without the keys, none. Sends: idle or failed `input {text, requestId}`; running ⌘⏎ adds `whenBusy: "steer"`; M-⏎ `whenBusy: "followUp"`.

**Follow:** on while the cursor is on the last entry; an upward motion pauses it, `12 new ↓` sits at the column's foot above the queue, `G` resumes.

**Calls and streams:** `open`, `history`, `input`, `continue`, `stop`, `setModel`, `models`.

**Keys:** `i a o` compose; ⌘⏎ / M-⏎ in the composer; `]m [m`, `]t [t`, `za`; `␣cx`, `␣cc`, `␣cm`, `␣ct`, `␣cf`; `␣td`; `gd`.

### 5.6 Decision

**Answers:** what am I asked, on exactly which text, and what happens when I answer.

**One fact, one place.** The breadcrumb holds the project and the decision ID; the title block holds the title and the state; the margin holds your notes; the details panel holds the extension, conversation, basis entry and `then` task. Nothing appears twice.

**Header:** breadcrumb `render-core › w1:3:design#2`. Right: open → `◆ ␣ad Answer` (§4.2); `␣ar send again` when the `then` task ended `faulted` or this device's answer has an unknown outcome. Otherwise nothing.

**Title block:** the title (`title.page`, coral; extension text as one plain line), with no `#n`: it is in the ID. Facts line with `details ›` at its right: `open · requested 13:55 · 2 h 45 m ago`; or `answered revise · by phone 13:30`; or `superseded by #3`; or `withdrawn`; or `? answer sent 16:42 · no reply`. After an answer the line adds the `then` task's state (table below).

**Text column** (840 px), 24 px below the title block:
- Answered with a faulted `then` task: a head line on `tint.row_fail` with the rose `×` and `The work after this answer failed`; under it, unboxed, the task kind and error in `mono.small`.
- **The basis:** `text` as Markdown, single line breaks kept. Its first heading is a content `h1` (rose 20/28 with its rule), never the page title's style. This is the exact text the answer quotes; nothing else is shown as basis.
- **Choices** (group head, steel): one 28 px row per choice in the core's order: number (`mono.small` `fg.alt`, while there are at most nine) and the string (`ui` `fg`), wrapping within the column when long. No recommendation, no colour per choice, no special `abort`. Answered: the chosen row carries `●` and `chosen by phone · 14:10`.

**The `then` task's state,** from the `then` field of `decision` (pi-durable's task state):

| `status` | `outcome.status` | Shown | Mark |
|---|---|---|---|
| `pending` | | `then w1.design_answer · queued` | `hourglass`, `fg.muted` |
| `running` | | `· running` | steel triangle |
| `waiting` | | `· waiting` | `hourglass`, `fg.muted` |
| `sleeping` | | `· sleeping` (a timer) | `time`, steel |
| `completing` | | `· finishing` | steel triangle |
| `terminal` | `completed` | `· completed` | `checkmark`, olive |
| `terminal` | `failed` | `· failed` (the workflow's own outcome; the core never runs it again, and `␣ar` stays off) | rose `×` |
| `terminal` | `aborted` | `· aborted` | `subtract`, `fg.muted` |
| `terminal` | `faulted` | `· faulted` and the failed block; `␣ar send again` | rose `×` |
| a state that says `missing_task` (blocked) | | `· blocked · its extension is not on the server` | `hourglass`, `fg` |
| anything else | | the raw `status` word | none |

**Refreshing.** The client refetches `decision` 1 s after this device's own `answer` (the reply carries no `then`), when the view is shown again, when a `faults` poll names this decision, and every 30 s while the view is shown and the state is not `terminal`.

**The notes margin** (320 px, 48 px right of the text column), from the top of the title block down. When the stage is too narrow for it, items 1 to 3 sit under the facts line, in that order, and each passage's note under its paragraph (§4.7):
1. **Head:** the sage `edit` mark and `Your notes · 3 · not sent` (`Your notes` in `fg.alt` 600, the rest `fg.muted`); with none, `Your notes · none`. After an answer the margin shows the answer's notes, with no mark: `Notes · 3 · sent by phone 13:30`, or `Notes · none · answered by phone 14:10`.
2. **The note on the whole:** `on the whole` (`ui.small` `fg.muted`) and its text (`ui` `fg`); left out when there is none.
3. **Earlier requests,** folded, one line each, newest first: `› Earlier: #1 revise · 2 notes`. Opening one (`za`, or a click) lists that request's notes with the passages they quoted; they can't be pinned to this text. `gd` on its `#1` opens that decision.
4. **Notes on passages,** each beside its passage: its head (`edit`, `not sent` or `sent by phone · 13:30`) aligned to the passage's first line, its text under it (`ui` 13/19 `fg`). Notes stack 12 px apart without overlapping; a note pushed below its passage gets a **leader**, a 1 px `rule.note_leader` line from the column's right edge at the passage's first line, across the gap and down to the note's head.

**Passages with notes** carry `tint.note_range` behind their text, inline; the passage of the note under the cursor, or of the note being typed, carries `tint.note_active`.

**The note being typed** is the container style with a coral edge, in the margin at its place: head `note on the selection · not sent` (or `note on the whole paragraph`, `note on the whole`), the text with a coral caret, and the hint `Esc jk leave Insert` inside at the bottom right. Until 3 notes are written on this device, a ghost `+ note c` shows at the cursor line.

**Notes and ranges.** A range counts UTF-16 code units of `text`, as `text.slice(start, end)` (the server checks it, `primitives.md` "annotations replace notes"). The rendered Markdown keeps each block's source range.
- `␣mn` on a block notes that block's source range, trimmed of blank lines.
- A Visual selection inside one block's plain text maps to its exact source range. A selection that crosses inline markup or blocks takes the range of the blocks it touches, and the note's head says `note on the whole paragraph`.
- `␣ts` shows the basis as plain source text (`mono`, no Markdown); there any character range is exact.
- `␣mw` writes the note on the whole (no range).
- Drafts stay on this device until a `result` of `answer` carries them (a review does not move between devices).
- `]n` and `[n` move the cursor to the next and the previous note's passage, in text order; they are not printed. The note on the whole is reached with `␣mw`.

**Liveness,** from the inbox entry of the subject:
- its open ID changes → notice band (steel `time` mark): `Superseded by #3 · this request can no longer be answered`; `␣ad` off; the drafts stay with `#2`.
- it leaves the inbox → refetch `decision`; notice `Answered approve by phone at 16:42` or `Withdrawn`.

**An answer whose reply was lost** (`client.md` §2.3): `␣ad` is off and the drafts stay; the client settles it from the inbox and `decision` when they prove it landed or not; until then `␣ar` sends the stored body again through the readback.

**Calls:** `decision` (this one, and `#1 … #(n−1)`), `answer`; the `inbox` stream already held.

**Keys:** reader motions; `v V`; `c{motion}`, `gc`, `␣mn` note; `␣mw`; `]n [n`; `i` edits the note under the cursor; `dd` discards a draft, `u` undoes; `⏎` on a choice moves the cursor there; `␣ad`; `␣ar`; `␣ts`; `␣td`; `gd`.

### 5.7 Readback: answering, and sending again

The one authority dialect: two commands, both under `␣a`, both only in the Decision view, both confirmed with `y`. Both use the float shell (§6).

**`␣ad` Answer** opens the readback: mode badge `READBACK` solid coral, verb tag `◆ ANSWER` solid coral (the one pair). It spans the stage like every float: its right column's values can be long, and one width for every float keeps the coral rule and the hints in one place.

```
◆ ANSWER   Review the design #2 · render-core                                open · requested 13:55
  1  approve         │  notes   3 sent with the answer · 2 on passages, 1 on the whole   K reads them
▌ 2  revise          │  basis   entry 34 · 2,203 characters · shown to the end
  3  abort           │  then    w1.design_answer runs in #3
  j k or 1–3         │  from    this device, laptop
─────────────────────────────────────────────────────────────────────────────────────────────────────
                                                                              y confirm   n Esc q cancel
```

- **Left column** (300 px): the choices as 30 px picker rows, `1 approve` (number `mono.small` `fg.alt`, choice `ui`), the pick with the selection (fill, 2 px coral bar, 600), and the hint `j k or 1–3` under them. Up to nine carry their number and `1`–`9` pick; past nine they carry none, and only `j k` move the pick; when they don't fit the float's height the column scrolls.
- **A 1 px hairline** between the columns.
- **Right column:** a 72 px steel label column and the values, on 30 px rows: `notes`, `basis`, `then`, `from`, with `K reads them` after the notes' value. Its first row sits on the left column's first baseline.
- Its echoes (`read it first`, `pick a choice first`) show in the bottom bar.
- **Choice:** preselected when the page cursor was on a choice row; otherwise none, and `y` echoes `pick a choice first`. Each pick redraws the float.
- **Basis line:** how much of the basis this view has drawn: `shown to the end`, or `not shown to the end · 62 % shown`. It informs; it never blocks.
- **`y` counts only after the readback has been on screen for 400 ms** (`tokens.json` `tau.client_ms.readback_arm`), and confirms what the float last drew. An earlier `y` is discarded and echoes `read it first`. A burst such as `␣ad2y` at typing speed therefore never sends. Auto-repeat and `⏎` never confirm; a click on `y` echoes `press y to confirm`.
- **A host event can only turn `y` off:** if the inbox says the request is no longer open, the float becomes the not-open reply.
- **On `y`:** `sending…`, then `answer {project, decision, basis, choice, annotations}` with the drafts in order:

| Reply | Shown |
|---|---|
| `result`, `repeated: false` | float closes; echo `Answered revise · w1.design_answer runs in #3`; the view turns answered; drafts clear |
| `result`, `repeated: true` | the same; echo `Already recorded`, plus `· by phone` when `by` is another device |
| `conflict` | the view takes the reply's `decision` at once: facts line `answered approve · by phone 16:42`, no Answer, no diamond in the sidebar. The float becomes the reply, tag `◇ NOT RECORDED` (neutral, outline) and one key-value grid: `reply Not recorded. phone answered approve at 16:42.` · `their choice 1 approve` · `their notes 0 notes` · `yours your choice revise and your 3 notes were not sent; they stay on this device`; hints `Esc closes · your notes stay on this device`; badge `READBACK` in its plain form; only `Esc` and `q` act |
| `not_open` | `Not recorded. This request is no longer open.`, and `#3` when the reply names the open one |
| `extension_not_loaded` | `Not recorded. The server has no code for w1 now; the decision stays open.` |
| `stale_basis`, `unknown_choice`, `bad_annotation`, `bad_request` | `Not recorded. The server refused it: <message>` (a client bug) |
| unknown | float closes; drafts stay; outbox `? answer · no reply · open the decision to send it again`; facts line `? answer sent 16:42 · no reply` with `␣ar send again`; `␣ad` off (`client.md` §2.3) |

**`␣ar` Send again** is the one way to send an answer a second time. It is live in two cases. There is nothing to pick, so its body is the key-value grid alone (96 px labels), and no coral cursor shows while it holds the keys (§4.6):

```
◆ SEND AGAIN   Review the design #1 · render-core                     answered approve · phone 14:10
its work   w1.design_answer faulted · git rev-parse main failed: fatal: Unable to create …
sends      the same answer: approve, 0 notes, basis entry 17
then       a new w1.design_answer starts in #6
from       this device, laptop
                                                                              y confirm   n Esc q cancel
```

- **The `then` task ended `faulted`:** `y` sends `answer` with the record's own `basis`, `choice` and `annotations`. Expected: `repeated: true` and a new `continuation`; echo `Sent again · w1.design_answer runs again in #6`; the old fault is hidden on this device.
- **This device's answer has an unknown outcome:** the first row reads `last send   had no reply · it may have been recorded`, and `y` sends the stored body. Its reply settles the outbox entry by the table above (`repeated: true` means it had landed).
- **`extension_not_loaded` on a send-again after a fault:** the decision is answered, and the refusal comes from the answered case (`packages/store/src/decisions.ts:194-199`, `:250-251`). The float becomes the reply: `Not sent again. The server has no code for w1 now; the answer stays recorded and its work is not started.`

The same 400 ms rule applies.

### 5.8 The places

Each place is one page with one filter and one cursor. It keeps both on this device when you leave it, and across launches. Filters: Conversations, a project and a state; Timeline and Faults, a project; Home and Projects, none. `␣pf` opens the filter float; the breadcrumb shows the filter. Each place has its page title (`Conversations`, `Timeline`, `Faults`, `Projects`), rows of 28 px under `group` heads, and the peek (§5.4).

**Conversations** (`␣oc`, its sidebar row). **Answers:** every conversation, to continue or join one.
- Grouped by project; the group head is the project (sage) and `1 running · 1 failed · 7 in all`; rows newest ID first.
- Row: the small mark (running triangle, failed `×`, stopped and idle none), label, then the slots: extension when known, model when known (`fg.alt`), `#id` (orange).
- Opening Conversations from the Project view sets its project; Home's `more failed runs` line sets `failed`.
- `/` searches labels. `x` hides a failed run on this device; `␣th` shows hidden ones.
- **Calls:** `conversations` per project (polled while focused, every page); `history` for labels of drawn rows only.

**Timeline** (`␣ot`, its sidebar row). **Answers:** what happened, in order, across projects.
- Rows like Activity's, plus the event's kind (`mono.small` `fg.muted`, `w1.event`) before the project; newest first; the since-line and bold new rows. Scrolling to the end pages back with `history` on each project's log (`order: "descending"`), merged by `at`.
- `⏎` opens the named conversation.
- **Calls and streams:** `timeline` per project, `history`.

**Faults** (`␣of`; Home's `more faulted task` line; no sidebar row). **Answers:** which tasks died, and which one I can restart.
- Under the title, once: `A faulted task never runs again by itself. If it names a decision, open it and send the answer again. Hiding a fault is local to this device.`
- Rows: rose `×` and kind (`mono` orange) lead; the error (`ui` `fg.alt`) takes the rest and is cut with `…`; then fixed slots at the right, 12 px apart: project (sage, 96), `#id` (orange, 26), `after w1:6:design#1` when named (`mono.small` `fg.alt`, as wide as the widest drawn), `endedAt` (`mono.small` `fg.muted`, 84, right-aligned). When the error would get less than 200 px, the time drops, then the `after` slot; each drops whole for every row.
- Hidden faults are left out; `␣th` shows them with a `subtract` mark and `hidden on this device`.
- `⏎`: its decision when named, else its conversation. `x` hides; `x` on a shown hidden one unhides it.
- **Calls:** `faults` per project, following `next`.

**Projects** (`␣op`, its sidebar row): project (sage) · workdir (`mono` `fg.alt`, `…/leaf`) · extensions (`mono.small`) · `◆ 1` when it has open decisions · `1 missing` (rose) when the server lacks an extension. `⏎` opens the Project view; `␣pb` binds one.

**Empty:** one line where the first row would be (`ui`, `fg.alt`), saying which of three cases it is.
- **The server has none:** `No faulted tasks.` · `No conversations yet.` · `No events yet. Bare conversations write none.` · `No projects yet · ␣pb binds one`.
- **The filter keeps none:** it names the filter, `No faulted tasks in notes-cli.` · `No failed conversations in render-core.` · `No events in notes-cli yet.`, then a quiet pressable `show all projects` (`ui.small`, `fg.muted`; `show all` in Conversations with a state) that resets this place's filter and no other place's.
- **The filter keeps some, all hidden on this device:** the count for the filter, `2 faulted tasks hidden on this device` or `1 failed run hidden on this device`, then the `␣th` chip and `show hidden`; a click on either runs `␣th`.

A search that matches nothing reads `No matching rows.`

### 5.9 Project view

**Answers:** where this project's agents work, what they can use, and how much is going on. Breadcrumb `Projects › render-core`; no actions. A 600 px column.

- **Title block:** the project's name (`title.page`); facts line `working directory /home/vrod/src/render · a path on the server`.
- **Work** (group head): three rows that open a place filtered to this project, setting that place's filter: `Conversations` with `7 · 1 running · 1 failed` at the right, `Open decisions 1` (opens Home at Needs you; Home itself is never filtered), `Faults 1`. The counts come from the same polls as Home (`client.md` §5) and refresh with them.
- **Extensions** (group head): `name` (`mono`), kind (`workflow` / `tool`), options state (`all set`, or in rose `3 to set` and `1 doesn't fit`, from the `options` item's errors as §3.2 names them); a missing one reads `missing on the server · its tasks wait and its tools are not offered`. Footer line: `An extension can't be removed from a project.` `␣pa` adds.
- **Options · w1**, **Options · websearch** (group heads): one row per schema property, in schema order:

| Schema | Row shows | `⏎` or `i` |
|---|---|---|
| `format: "model"` | `anthropic/claude-opus-4-5` (`mono`) and its name (`Claude Opus 4.5`, `fg.alt`) | model picker (`models`) |
| `format: "secret"` | `set · hidden` (value `true`) or `not set` | a masked editor, never prefilled; saving sends the new value once |
| `enum` | the value | choice picker |
| `type: "string"` | the value, or `default main` in `fg.muted` | inline editor |
| `type: "number"` / `"integer"` | the value | inline editor |
| `type: "boolean"` | `on` / `off` | toggles |
| anything else | the value as JSON | `K` shows the schema; not editable in v1 |

  A missing option (§3.2) reads `not set` with `required` in rose under it. Other errors from `options` show in rose under the field their `path` names (`expected format model`). Opened from the start page or after `options_invalid`, the view's cursor starts on the first field with an error.
- **Saving:** edits are drafts (sage `edit` mark per field; bottom bar `✎ 2 not saved :w`). `dd` or `␣mr` resets a field to its default as a draft. `:w` or `␣ms` sends one `setOptions {project, extension, patch}` per extension with changed keys (a reset is `null`). `bad_options`: each error under its field by `path` (`expected format model`), drafts kept, echo `The options of w1 do not fit.` Success: drafts clear; the values are the reply's `value`.
- `␣cn`, `␣pa` and `␣po` here take this project.
- **Calls:** `projects`, `options`, `setOptions`, `models`, `extensions`, `addExtensions`; the polled `conversations` and `faults`, and the `inbox`, for the Work counts.

---

## 6. Floats

**One shell for every bottom float.**
- **Frame:** spans the stage, inset 12 px at the sides and 10 px at the foot; `surface.raised`, a 1 px `border.hairline`, a 2 px coral top rule, `radius.surface`, the upward shadow; up to 40 % of the window tall; 12 px padding above, 10 below. The page is clipped at the float's top edge, so the insets beside and under it show `surface.read`, never a line of the page.
- **Head,** one line, 24 px in: an optional **tag** (the solid coral verb tag, or the neutral outline `◇ NOT RECORDED`), the **title** (`float.title`; it may carry an `#id` in orange mono, a project in sage, or a key glyph), a **subtitle** (`ui.small` `fg.muted`), and **meta** right-aligned (`ui.small` `fg.muted`).
- **Body,** one of: **rows** (30 px, 24 px in, the selection a `surface.cursor.side` fill with the 2 px coral bar); a **key-value grid** (96 px steel labels, `fg` values, muted hints at the right); a **key grid** (which-key: 200 px columns of 26 px entries); or the readback's two columns (§5.7). Results freeze once you move.
- **Foot,** above a hairline, 24 px in: an optional **input** at the left (`float.input`, Iosevka 14/20, a coral block caret, then a muted count such as `3 projects`), and **hints** at the right (key chips on `surface.cursor.side`, each followed by a muted word, 16 px apart).
- **While a float holds the keys:** only its own keys act, with `Esc` (and `q` where it has no input) to close it; nothing of `global` does, so no leader, `:`, `⌘K` or `K` reaches the page under it (`reference/grammar.md` §3). Chips, hints and segments outside it draw `fg.muted` and are inert, the coral Answer turns to its outline, the stage's and the sidebar's cursors turn neutral, and the mode badge names the float's own mode, never `NORMAL` (§4.6).

`q` or `Esc` closes any float that is in Normal mode.

| Float | Opens with | Head | Body | Calls |
|---|---|---|---|---|
| which-key | 300 ms after a prefix | `␣ leader` alone; for `␣a`: `answer` · `every command reads back what it sends before y` | key grid: group entries first in `fg` 600, a hairline, single commands in `fg.alt` 400 (`reference/grammar.md` §5); hints `Esc closes`; display only | none |
| Palette | `␣;`, `⌘K` | `Commands` | rows: desc, `:ex`, keys; unavailable ones listed with the reason; input | none |
| `:` line | `:` | `Command line` | completions as rows; input with `:`; history on Up/Down | the command's |
| Find | `␣.`, the sidebar's `Find` | `Find` · `projects, conversations, decisions, events` | rows; prefixes `p: c: d: e:` in the input | held data |
| Project | `M-p` or `:project` on the start page; `␣pa`, `␣po` when no view names a project and none was used on this device | `Project` · `for the new conversation` (or `to add extensions`, `to set options`) | rows: `folder`, name (sage), its extensions (`w1 · websearch`, or `bare agent only`), workdir (`mono.small` `fg.muted`), `last used` at the right; cursor on the current one; input `3 projects`; hints `⏎ choose · Esc back` | `projects` |
| Workflow | `M-w` or `:workflow` on the start page | `Workflow` · `for the new conversation in render-core` | one flat list: the project's workflows, each `w1 · workflow`, with no icon, and in rose `needs options: designer, implementer, reviewer` or `options don't fit: reviewer` when its options have errors; then one row per model, `bare agent · Claude Sonnet 4.5`. Typing filters both, so `son ⏎` picks a bare agent on Sonnet. An extension the server lacks is never listed | `extensions`, `options`, `models` |
| Model, thinking | `␣cm`, `␣ct`, a model field | `Model` · `for #5`; meta `w1 may set the model again when it next sends work here` when the selection names a workflow | rows: `models` with the current one marked `current`; the seven thinking levels | `models`, `setModel` |
| Filter | `␣pf` in a place | `Filter` · the place | rows: `all projects` and each one; in Conversations, a second group of states (`failed`, `running`, `idle`) | none |
| Bind | `␣pb`, `:bind`, `⌘⏎` on a start page with no projects | `Bind a project` | key-value grid: `name` (`^[a-z0-9][a-z0-9-]{0,62}$`) and `working directory` (an absolute path on the server, with the note `a path on the server`), both text fields; `extensions` (a checklist from `extensions`, with kind words; none is valid). The hints follow the focused part: in a field, `INSERT`, `Tab next · Esc Esc cancel` (the fields have no Normal mode: the first `Esc` goes to the checklist); in the checklist, `COMMAND`, `Tab toggle · ⏎ bind · Esc cancel` | `extensions`, then `bind` |
| Add extensions | `␣pa` | `Add extensions` · `to render-core` | a checklist of the server's extensions not in the project, in `COMMAND`; hints `Tab toggle · ⏎ add · Esc cancel` | `addExtensions` |
| Confirm | before `bind` and `addExtensions`, and before pairing over plain HTTP to another machine | `Confirm` | pairing: `Pair over plain HTTP? Anyone on the network path can read this device's token.`, hints `y pairs · n cancels`; binding with extensions: `Bind scratch with w1? A project can't be unbound and an extension can't be removed.`; with none: `Bind scratch with no extensions? Its conversations get the coding tools only. A project can't be unbound; extensions can be added later but never removed.`; hints `y binds · n cancels` | none |
| Readback | `␣ad`, `␣ar` | §5.7 | §5.7 | `answer` |
| Outbox | `␣ru`, the `?` tag (badge `OUTBOX`) | `Outbox` · `every write, kept until done`; meta `1 without a reply` | rows: mark, call, project and `#id`, the text, state, time; an answer entry reads `open the decision to send it again` and a create entry `try again from the start page`, and neither has `␣rt`; hints `␣rt retry · x forget · Esc closes` | the entry's call |
| Server | `␣os`, the sidebar's server line or `Server` row (badge `SERVER`) | `Server` · the URL | key-value grid: this device's name; `protocol 1`; connected since; each stream and its state; then `Devices are listed and revoked on the server: tau device list, tau device revoke <name>.` and `:forget-server`, typed after closing the float | none |
| Peek | `K` on a list row under a 1000 px stage, or on an open item in the sidebar | the row's title | §5.4 | as §5.4 |

**Badges.** Every float that takes the keys names its own mode, never `NORMAL`: the pickers (Palette, Find, Project, Workflow, Model, thinking, Filter) and the bind and add-extension checklists are `COMMAND` (the pickers have an input that filters as you type); the bind float's text fields are `INSERT`, with no Normal mode (`Esc` moves to the checklist); the answer readback and send-again are `READBACK`, solid coral; Confirm and the not-recorded reply are `READBACK` in the plain badge with a coral edge; the outbox is `OUTBOX`, the Server float `SERVER`, the peek float `PEEK`. which-key takes no keys: it keeps `NORMAL` with the pending key in the bottom bar.

---

## 7. Flows, as keystrokes

| Flow | Keys |
|---|---|
| First run | launch · `Tab` (URL prefilled) · type the code · `⏎` (over plain HTTP to another machine: then `y` in the Confirm float) |
| Bind a W1 project and set its options | `␣pb` · name `Tab` path `Tab` · `C-j` to w1 · `Tab` (toggles it) · `⏎` · `y` → Project view: `j` to designer, `⏎`, `opus`, `⏎`; the same for implementer and reviewer; `:w⏎` |
| Bind a bare-agent project | `␣pb` · name `Tab` path `Tab` · `⏎` (no extension ticked) · `y` |
| Start work where you last started | launch, or `␣cn` · type · `⌘⏎` |
| Start in another project | `␣cn` · `M-p` · `notes` · `⏎` · type · `⌘⏎` |
| Start a bare agent on another model | `␣cn` · `M-w` · `son` · `⏎` · type · `⌘⏎` |
| Start in a W1 project whose options aren't set | `␣cn` (the line names the missing options) · type · `⌘⏎` → Project view at designer · set the three · `:w⏎` · `␣cn` (the text is kept) · `⌘⏎` |
| Come back and answer a W1 design with notes | Home (cursor on the oldest decision) `⏎` · read with `C-d` · `␣mn` type `Esc Esc` · `V` over two lines `␣mn` type `Esc Esc` · `␣mw` type `Esc Esc` · `␣ad` `2`, read the float, `y` |
| Resolve a forecast | `]q` to it · `⏎` · `␣ad` `2`, read, `y` |
| Join a running W1 agent and steer it | Home `]g` to Running · `j` · `⏎` · `i` · type · `⌘⏎` |
| Find a workflow's agent from what happened | Home `]g` to Activity · `j` to `The implementer is building the approved design. #5` · `⏎` |
| Continue a failed run on another model | Home `]g` to Failed · `⏎` · `␣cm` `opus` `⏎` · `␣cc` |
| Put a failed run aside | Home `]g` to Failed · `x` |
| Restart work after a faulted `then` task | Home Failed, the fault row · `⏎` · `␣ar` · read · `y` |
| An answer whose reply was lost | wait: the client settles it from the server; if it stays unknown, in the decision `␣ar` · read · `y` |
| Retry another call with no reply | `␣ru` · `j` to it · `␣rt` |
| Stop a run | in the conversation `␣cx` |
| Switch between open items | `]b` / `[b`; or `C-h` · `j k` to it · `⏎` |
| Close an open item | `C-h` · `j` to it · `x`; or `:q⏎` in it |
| Read a conversation's settings | `␣td` |
| Read without the sidebar | `␣tb` |
| Go home | `␣oh` |

---

## 8. Copy rules

- **Plain words.** Say what happened and what the user can do: `Not recorded. phone answered approve at 16:42.` Internal names (`storage_failed`, `runFailure`) appear only in `K` details and outbox entries.
- **No bare dash in a field.** An empty value is words: `none`, `not set`, `default main`, `no text`, `empty conversation`, `no result`.
- **Plurals by count:** `1 note`, `3 notes`, `1 more faulted task`, `2 more failed runs`.
- **One state, one number; one fact, one place.** A facts line never repeats what the breadcrumb says.
- **Two failure words:** a conversation's run `run failed`; a task `task faulted`. The group and the tag that hold both are **Failed** and `×`.
- **Page titles** are a place's name, `Home`, `Needs you` or `Nothing needs you`, or the object's own label or title as one plain line (while a new conversation's first input is sending, its first line). A title is never a sentence the client wrote about the object.
- **Server and extension text is shown as sent,** never rewritten, except that rows and titles drop the backticks of inline code (§5.3). When the client's copy quotes a server sentence, it adds no full stop after it. Agent text is content.
- **Ages and times:** ages (`2 h 45 m`) count on the local clock; clock times (`13:55`) are local time; a time that hasn't happened isn't printed. Ages and running counts are written one way: one decimal under 10 s (`4.2 s`), whole seconds under a minute (`12 s`), `17 m` under an hour, `1 h 5 m` under a day, days and hours under a week (`4 d 15 h`; a day exactly is `1 d 0 h`), days alone after (`12 d`). A clock time from another day carries its date (`Nov 14 13:55`), the form the Timeline uses, also where an age stands beside it (`requested Nov 14 13:55 · 4 d 15 h ago`).
- **Disabled actions** read `<verb> · off` with a dashed edge, and say why on `K`.
- **An action names the act,** never the choice: `◆ ␣ad Answer`, `␣ar send again`, `␣rt retry`, `␣cc continue`. The readback names the choice.
- **Keys in copy** are chips, or muted hints inside a container, and only where `reference/grammar.md` §9 allows; the echo prints none. Server text never names a client key.
- **Every quoted sentence in this file is the copy.** Copy not yet written is listed in `open-questions.md`.

---

## 9. Extension-specific UI: none in v1

**What it costs.**
- **W1:** its stage shows only as event text. That a designer, an implementer and a reviewer serve one piece of work shows only through the events that name them. The designer's replies to your notes (`data.replies` on `w1.design`) are not drawn next to your notes; `K` on the record shows them as JSON.
- **W2:** a forecast's probability and dates are raw basis text (`Resolve by 1794664800000 (epoch milliseconds)`); calibration exists only as a tool's output; there is no list of open forecasts.
- **Any workflow:** options are a generic form, errors are JSON paths, choices are bare strings.

**Where a hook would go** (not designed): a renderer per entry kind, used by the transcript's record, the decision's basis and the peek; one panel per project on Home, between Running and Activity, and in the Project view; commands an extension adds under `␣m` and to the palette. The sidebar may become one more slot (an extension's own section under `OPEN`); that is a note, not a design. Whether a hook is client-side code or a declarative hint from the extension module is a core question, through the admission rule. The owner has his own plans for malleable UI; this section only keeps the slots named.

**Deferred with it:** Mermaid diagrams. In v1 a `mermaid` fence renders as an ordinary code block with its language label (`reference/visual-language.md` §9).
