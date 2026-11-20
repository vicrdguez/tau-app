# Connection truth

How the desktop client talks to a tau server, and what it tells the user when the link or the server does not cooperate. `spec.md` says where each of these shows on screen. The protocol itself is `docs/core/protocol.md`; the server's behaviour is `packages/host/src`. Where this file and those disagree, those win, and this file is wrong.

Citations are `file:line` in the tau core at `render-core` `b6bc814` (2026-11-14); `README.md` says where those files live now.

---

## 1. One call

A call is one `POST /v1/<name>` with a JSON body of at most 1 MiB (`packages/host/src/http.ts:11`). Every call but `pair` carries `Authorization: Bearer tau.<device>.<secret>` (`devices.ts:23`). Every reply carries `Tau-Protocol: 1` (`http.ts:28`).

**The body decides the outcome, never the status** (`docs/core/protocol.md:46`):

| Reply body | Outcome | The client |
|---|---|---|
| `{"result": …}` | done | applies it |
| `{"refused": {code, message, …}}` (status 400–413) | not done; the server decided | shows it (§6) |
| `{"unknown": {code, message}}` (status 500, `http.ts:137-146`) | unknown | §2 |
| anything else, no reply, a cut, or no reply within **20 s** | unknown | §2 |

A proxy can answer 408 or 409 with a page of its own after the server committed; that is unknown, not refused.

**Order of checks on the server** (`http.ts:78-135`, build note `docs/core/build-notes/unit-2-host.md`): `Host` and `Origin`, then `pair`, then the token, then the call name, then the body. So `POST /v1/` with a good token answers `unknown_call` with the list of calls: the client uses that as its handshake (§7).

---

## 2. The outbox and resends

Every write call (tier `write` or `decide` in `keymap.toml`) is written to the **outbox** with its exact body **before** it is sent. The outbox lives on this device and survives a restart of the client.

| State | Meaning | Shown |
|---|---|---|
| `sending` | sent, no reply yet | `sending…` where the act happened |
| `done` | a `result` came | the entry leaves the outbox after 1 minute |
| `refused` | a `refused` came | the refusal's copy (§6), where the act happened; the entry stays until opened |
| `unknown` | §1 | the mauve `?` mark and `no reply`; counted by the `?` count tag |

**Request IDs.** `input` and `continue` carry a `requestId` the client makes once per send (a ULID, `r-01JCV8Q6W3M2`). The server stores it as `client:<device>:<requestId>` (`calls.ts:431`, `calls.ts:457`; `protocol.md:87-91`), so a resend from this device returns the first submission, and another device's ID can never collide with it.

### 2.1 On an unknown outcome

| Call | What the client does | Why it is safe |
|---|---|---|
| `input`, `continue` | resends the same body on the schedule of §2.2 | same request ID: the server returns the first submission (`docs/core/host.md:55`) |
| `setModel`, `setOptions` | resends the same body on the schedule of §2.2, but never after a restart of the client: a restored entry reads `may not have been applied · set it again` and `␣rt` is off for it | the same value comes back (`host.md` table, "A resend"); after a restart another device or the workflow may have set a newer value |
| `bind`, `addExtensions` | resends the same body on the schedule of §2.2 | `bind` returns `repeated: true` (`projects.ts:211-214`); the same names change nothing |
| `answer` | **never by the outbox.** The entry reads `? answer · render-core · Review the design #2 · no reply · open the decision to send it again` (the stored decision ID, `w1:3:design#2`, when the title is not held), and `␣rt` is off for it; `⏎` on it opens that decision. §2.3 | an equal resend after a fault starts the `then` task again (`docs/core/primitives.md:138`), so only the human sends it, through the readback (`spec.md` §5.7) |
| `create` | **never automatically**, and its `input` is not sent (§2.4). `␣rt` is off for it: the start page keeps the text, and `⌘⏎` there tries again. Entry: `No reply · a conversation may have been created · try again from the start page` | a retried `create` makes a second conversation, an accepted limit (`docs/core/README.md`, "Accepted limits") |
| `stop` | **never**. Entry: `No reply · the run may have stopped` | a second `stop` ends whatever runs then (`host.md:56`) |
| `pair` | never; §7 | a code works once, and a landed `pair` has used the name |

`x` on an outbox entry forgets it on this device; nothing is sent. `⏎` opens what the entry names and closes the outbox: an answer's decision, an input's conversation, the start page for a `create`; it never sends. Forgetting an `answer` entry also turns `␣ad` back on for that decision (§2.3): if the lost answer did land, a different one is refused with `conflict`.

### 2.2 The resend schedule

Automatic resends (the calls marked so in §2.1) follow one schedule per entry:

- The first try is at the next stream connection, or 1 s after the unknown outcome if the streams are connected (a 20 s timeout or a `storage_failed` leaves them up).
- Then on the stream backoff: 2, 4, 8, 16, 30 s, at most 6 tries in all. After that the entry waits for `␣rt`.
- **An entry older than 10 minutes, or one restored from an earlier launch, is never sent automatically.** It waits for `␣rt`, and its row says `not sent automatically · ␣rt retries`. An input not delivered on Friday does not start a run on Monday.
- `␣rt` on an entry sends it once, now, with the same body.

### 2.3 An answer with an unknown outcome

- **Drafts stay** until a `result` comes; a lost reply never loses the notes.
- **`␣ad` is off** for that decision while the entry is unknown (`answer sent 16:42 · no reply`).
- **The client settles the entry itself** when the server proves the outcome:
  - **Landed:** the inbox no longer lists that decision ID, and `decision` shows it `answered` with `by` this device and the same choice and notes. The entry turns `done`; the drafts clear; echo `Answered revise · the reply was lost, but it was recorded`.
  - **Not landed:** a fresh first frame of `inbox` (after a reconnect) still lists that decision ID open. The entry turns `not recorded`, `␣ad` is live again, and its readback opens with the same pick and the same notes when the user presses it. No server event opens it.
  - **Answered by another device, superseded or withdrawn:** the entry takes the copy of `conflict` or `not_open` (§6).
- **Otherwise** the entry stays unknown, and `␣ar` in that decision's view sends the stored body again, through the readback (`spec.md` §5.7).

**Offline writes.** With no stream connected, one write still works: ⌘⏎ in a composer puts the `input` in the outbox as `waiting for the server`, and it goes with its request ID when a stream connects (the first try of §2.2). Every other write is off (`· off`), and `␣ad`/`␣ar` echo `offline · answer when connected`. The start page's `⌘⏎` echoes `offline · start when connected`: it needs `create`, which is never queued.

### 2.4 Starting a conversation: `create`, then `input`

The start page's `⌘⏎` (`spec.md` §5.2) makes two calls, in order, each its own outbox entry:

1. `create {project, workflow}`, or `create {project, model}` for the bare agent (`calls.ts:249-305`). Its result is `{conversation}`. For a bare agent the server sets the model and the project's working directory (`calls.ts:261-266`); for a workflow, the working directory and whatever the workflow's entry sets, after it has checked the workflow's options (`calls.ts:287-303`).
2. Only on `create`'s `result`: `input {project, conversation, text, requestId}`, with a new request ID, exactly as a composer sends it (`calls.ts:416-449`).

Before `create` the client checks the text: empty or white space sends nothing, and over 1 MiB sends nothing and echoes `Over 1 MiB · nothing was sent` (the body limit, `http.ts:11`), so a too-large `input` can't be refused after `create` has made the conversation. From the press until `create` replies or times out, the start page reads `starting…`, its text is read-only, and `⌘⏎`, `M-p` and `M-w` are off; auto-repeat never fires `⌘⏎`. So one press makes at most one `create`. If the stage has left the start page when the `result` comes, the conversation joins the sidebar and the `input` is sent; the stage stays where it is.

| What happens | The client |
|---|---|
| both done | the page is the conversation; the input's entry arrives on its `open` stream |
| `create` refused | nothing was created and no `input` is sent. The start page stays with its text, and the refusal's copy (§6) shows under the start line; `options_invalid` also opens the Project view at the first missing key |
| `create` unknown (no reply, a cut, 20 s, `storage_failed`) | **never resent**, and no `input` is sent. The start page stays with its text and reads `? No reply · a conversation may have been created; it would show as empty conversation in render-core`. The outbox entry counts in `?`. `⌘⏎` again sends a new `create`, then the `input` |
| `create` done, `input` unknown | the page is already the conversation. The input resends on §2.2's schedule with the same request ID, so it lands once |
| `create` done, `input` refused | the conversation stays, empty (`empty conversation`); the text returns to its composer as a draft, with the refusal's copy above it |

The start page's draft clears, and this device records the project and workflow as last used (§8), only when `create` succeeds; an earlier `create` entry of the start page still unknown in the outbox is then forgotten, so `?` doesn't count a retry that worked. No call says which device created a conversation, so a duplicate from a lost reply can't be found and removed: it stays as `empty conversation`.

---

## 3. Streams

`open`, `inbox` and `timeline` answer with `text/event-stream` (`streams.ts:53-57`). A refusal comes first as an ordinary JSON reply. A frame is one line `data: <json>` and a blank line; a line `:` is a heartbeat.

### 3.1 What the client holds

| Stream | Held | Body |
|---|---|---|
| `inbox` | always, while connected | `{}` |
| `timeline` | always, one per project | `{project, after?}` |
| `open` | one, for the conversation the stage shows. An open item that isn't shown holds no stream: it keeps its entries, reopens with `after` when shown again (§3.4), and its sidebar mark comes from the poll (§5) | `{project, conversation, after?}` |
| `open` | one more for the peek, while the cursor rests on a conversation, event or running row | as above |

### 3.2 Frames and ops

- The first frame is `{"ops":[["r", value]]}` and **replaces the value** (`streams.ts:58`, `protocol.md:127`). That is the state now.
- Later frames are `{"ops": [ … ]}`. Apply them in order: `r` replace, `s` set, `d` delete, `a` append text, `t` drop leading text, `p` splice an array, `m` reorder (`protocol.md:131-139`). Refuse the path segments `__proto__`, `constructor`, `prototype`. Apply ops generically: a change to an inbox entry may come as one `s` on the subject or as field-level ops.
- `inbox`'s value is `{ "<project>": { "<subject>": {id, title, requestedAt, conversation} } }`. A project bound while the stream is open arrives as an `s` op on `[project]` (`inbox.ts:69-77`). One store's failed watch ends the whole stream with `failed` (`inbox.ts:59-66`).
- `open`'s and `timeline`'s value is pi-durable 1.5.2's `ConversationView`: `conversation`, `entries` (a window: the newest 50, growing to 100, then dropping its oldest), `before`, and `docs` (`pi.agent`, `pi.inbox`, `pi.live`, `pi.provider`, `pi.usage`) (`protocol.md:146-150`).

### 3.3 Entries are kept by ID; the gap rule

The client keeps every entry it has seen, by ID. The window dropping its oldest from the value never drops them from the view.

**Gap rule** (`protocol.md:152`, as the host's own test client does it, `packages/host/test/helpers.ts:235-243`): for each frame of `open` or `timeline`,

1. take `held`, the newest entry ID of that conversation the client held **before** applying the frame;
2. if nothing of that conversation was held (`held` is none), **skip**: a first open shows the window and pages older entries only when the user scrolls back;
3. otherwise, if the frame's `before` is newer than `held`, call `history {project, conversation, minEntryId: held + 1, maxEntryId: before, order: "ascending", limit: 100}` and follow `next` until it is absent.

While it runs, the view shows one chrome line where the gap is: `fetching entries missed while away…`. For `timeline` the conversation is the log's, named by the value's `conversation.id` (`protocol.md:158`).

**Scrolling back** past the oldest held entry pages `history` with `order: "descending"`, `maxEntryId` the oldest held minus one, and the cursor `next`. A cursor is opaque; send it back unchanged to the same call (`protocol.md` "A cursor is an opaque string").

### 3.4 Reopening: state, not replay

A stream is reopened with `after` = the newest entry ID held (`protocol.md:154`). The first frame then carries only newer entries (at most 100, else the gap rule fills in) and the documents as they are now. Nothing is replayed, and the client never pretends otherwise: a generation that a restart cut off begins again, so its partial text can get shorter.

### 3.5 Silence, sleep, backoff

- The server writes a heartbeat after 15 s without a frame (`streams.ts:6`, `streams.ts:29-37`).
- **45-second rule** (`protocol.md:171`): a stream that delivers nothing, frame or heartbeat, for 45 s, by a clock that counts sleep (`mach_continuous_time` on macOS), is closed and reopened.
- On wake from sleep every stream reopens at once.
- **Backoff** per stream after a cut or an `end`: 1, 2, 4, 8, 16, 30 s, then every 30 s (`protocol.md:172`). It resets when a first frame arrives. `␣rc` reconnects every stream now.
- A stream the client stops reading for 60 s is cut by the server with no `end` (`streams.ts:7`, `streams.ts:43`); that is a cut like any other.

### 3.6 `end` reasons

A clean end is a last frame `{"end":{"reason": …}}` (`streams.ts:26`; reasons in `devices.ts:42`). "Server line" below is the sidebar's (`spec.md` §4.1); while the sidebar is hidden, the same words show at the bottom bar's right end.

| Reason | When | Shown | Then |
|---|---|---|---|
| `stopping` | the server is shutting down cleanly (`docs/core/build-notes/unit-2-host.md`, "tau and a signal") | server line `◷ server stopped · retrying`; echo `The server is stopping. tau reconnects when it is back.` | backoff |
| `revoked` | `tau device revoke <this device>` ran on the server (`devices.ts:136-141`) | rose notice band `This device was revoked on the server. Nothing more is sent. To use it again, run tau device pair <new name> on the server (a revoked name can't be reused), then ␣rp.`; server line `× revoked`; views frozen, writes off, drafts kept | no reconnect. **This is the only way the client concludes "revoked".** |
| `failed` on `open` or `timeline` | that store's watch failed (`streams.ts:59-65`) | echo `A stream failed on the server and is reopening.` | backoff; after 3 failures in a row of the same stream, a notice band `The server keeps failing to stream <project>. Its last state is shown.` |
| `failed` on `inbox` | one store's watch failed, and the server does not say which (`inbox.ts:59-66`) | echo `The list of open decisions failed on the server and is reopening.` | backoff; after 3 failures in a row, a notice band `Open decisions may be out of date for every project: the server keeps failing to send them.` The `◆` tag and Needs you stay as last received, marked `as of 16:41` |
| none: a cut | a crash or the network | server line `○ reconnecting in 4 s`; the Server float lists each stream | backoff |

### 3.7 Offline

No stream connected for 60 s is **offline**: notice band `Offline since 16:41 · showing the state as of then · 3 notes and 1 unsent input are safe · writes resume when connected` (when nothing was received since launch, without `showing the state as of then`: `spec.md` §4.8) with `␣rc` reconnect; server line `○ offline 6 m`. Views stay readable. State facts (open, running, failed) carry `as of 16:41` in the peek; ages (`2 h 52 m`) keep counting on the local clock, because the time since a decision was requested does not depend on the link.

---

## 4. The calls

All calls of protocol 1 (`packages/host/src/calls.ts:162-616`), with what this client sends and uses. "Stream" rows answer as §3.

| Call | Body (besides `project`) | Result | Used by |
|---|---|---|---|
| `pair` | `code` (no token, no project) | `{device, token}` | Connect |
| `extensions` | (no project) | `items: {name, workflow, schema?}` | bind float, the start page's workflow picker, Project view, "is this selection a workflow" |
| `models` | (no project) | `items: {model, name}` | model pickers, `model` option fields, the start page's bare agent |
| `projects` | (no project) | `items: {project, workdir, extensions, missing}` | Home, Projects, project pickers, the start page |
| `bind` | `workdir`, `extensions` (may be empty, `docs/core/host.md:73`) | `{project, repeated}` | bind float |
| `addExtensions` | `extensions` | `{project}` | add-extensions picker |
| `options` | | `items: {extension, schema, value, errors}`; a secret's value is `true` (`options.ts:71-77`) | Project view, the start page (missing options) |
| `setOptions` | `extension`, `patch` (`null` resets a key, `options.ts:93`) | `{value}` | Project view `:w` |
| `create` | `workflow` or `model` | `{conversation}` (`calls.ts:249-305`) | the start page (§2.4) |
| `conversations` | `limit: 500`, `cursor?` | page of `{conversation, owner?, state}`, **oldest first**; `owner` is `{conversationId, taskId}`; state from `pi.live` (`calls.ts:308-346`) | Home, Conversations, the sidebar's marks, the Project view's counts (polled, §5) |
| `open` | `conversation`, `after?` | stream of `ConversationView` | the conversation the stage shows, peek |
| `history` | `conversation`, `cursor?`, `limit?`, `minEntryId?`, `maxEntryId?`, `order?` | page of `EntryRecord` | labels, gap rule, scrolling back, older events |
| `input` | `conversation`, `text`, `requestId`, `whenBusy?` | `{submission}` | composer, the start page (§2.4) |
| `continue` | `conversation`, `requestId` | `{submission}` | `␣cc` |
| `stop` | `conversation` | `{}` | `␣cx` |
| `setModel` | `conversation`, `model`, `thinkingLevel?` (`off minimal low medium high xhigh max`, `calls.ts:151-159`) | `{}` | `␣cm`, `␣ct` |
| `inbox` | (no project) | stream of open decisions | Home, the `◆` tag, the sidebar's decision marks, decision liveness |
| `decision` | `decision` | `{decision, text, then?}`; `then` is the `then` task's state when answered (`calls.ts:505-537`) | Decision view, peek, earlier requests |
| `answer` | `decision`, `basis`, `choice`, `annotations` | `{decision, repeated}`; no `then` (`calls.ts:563-570`) | readback `y` (`␣ad`, `␣ar`) |
| `timeline` | `after?` | stream of the event log's `ConversationView` | Home Activity, Timeline, conversation margins |
| `faults` | `cursor?`, `limit?` | page of `{task, kind, conversation, error, endedAt?, decision?}` (`faults.ts:14-21`) | Home Failed, Faults (polled, §5) |

The client never sends `whenIdle` (`calls.ts:423-437`): a steer whose run ended in flight starts a run, and the transcript shows that. It never opens the event log as a conversation, so `event_log` is never provoked.

---

## 5. Polling

The core has no stream of conversation states or of faults, so the client polls them **while the window is focused, whatever the view**: the `×` tag is on every screen.

| Call | When |
|---|---|
| `conversations {project, limit: 500}`, **following `next` to the end** | every 10 s per project while the window is focused; at once on focus and on reconnect; 1 s after this device's `input`, `continue`, `stop` or `create` in that project |
| `faults {project}`, following `next` to the end | every 5 min while focused; at once on focus, on reconnect, and when Home or Faults opens |
| `projects`, `extensions`, `models` | on connect, and when a view, picker or the start page that shows them opens |
| `options {project}` | when the Project view opens, when the start page's project is set or changes, and after `setOptions` |
| `history` for a label | §5.1 |
| `decision` for an answered decision whose `then` task is not terminal | `spec.md` §5.6 |

`conversations` pages oldest first, and a page may come one short while `next` still pages on (`cursors.ts:30`, `docs/core/build-notes/unit-2-host.md`). The client reads every page and reverses the order for display, newest first. **Cost:** one call per 500 conversations per project every 10 s, and the server reads each conversation's `pi.live` on every call; a project with thousands of conversations pays that each time (unmeasured). Home's Running and Failed, the `×` tag, the sidebar's conversation marks and the Project view's counts all come from this one poll.

The `open` stream of the conversation the stage shows, and the peek's, are live, so a conversation that is on screen never waits for a poll; an open item that isn't shown takes its mark from the poll.

### 5.1 Labels

A conversation's label is the first line of its first `pi.user` entry. The client asks `history {project, conversation, order: "ascending", limit: 10}` and follows `next` until it finds a `pi.user` entry, at most 5 pages (50 entries). **Only a found label is cached**, and then for good, because entries never change. A conversation with no `pi.user` yet (just created) is asked again the next time its row is drawn, at most once a minute, and reads `empty conversation` meanwhile. A conversation whose `open` stream shows a first `pi.user` takes its label from there.

---

## 6. Refusals in plain words

The client branches on `code`, shows `message` as the server wrote it where the table says so, and treats an unknown code as "not done" with its message (`protocol.md` "Versions"). Status codes are in `packages/host/src/refusals.ts:4-26`. Where a server sentence is quoted inside the client's copy, the client adds no full stop after it.

| Code | Status | From | Shown |
|---|---|---|---|
| `bad_request` | 400 | any | `The server did not accept the request: <message>` and each `errors` item (`path`, `message`), in the outbox entry. A client bug. |
| `bad_cursor` | 400 | paging | nothing: the client drops the cursor and pages from the start |
| `unauthorized` | 401 | `pair` | Connect: the server's sentence, then `A code works once, for 10 minutes, and five wrong tries cancel every pending code. Run tau device pair <name> again.` (`devices.ts:20-21`, `devices.ts:114-124`) |
| `unauthorized` | 401 | any other call | §6.1 |
| `unknown_call` | 404 | any | `This server has no call <name>; it may be older than this client.` |
| `unknown_project`, `unknown_conversation`, `unknown_decision` | 404 | any | the message (`render-core has no conversation 99.`); an open item on that object keeps its label in `fg.muted`, and its view reads `gone from the server` |
| `too_large` | 413 | `input`, `answer` | `Over 1 MiB · the server refused it · nothing was done`; the draft stays |
| `conflict` | 409 | `answer` | the view takes the reply's `decision` at once (answered, no Answer), and the readback turns into the reply: `Not recorded. <by> answered <choice> at <time>.` with that record's choice and notes; your notes stay as drafts (`primitives.md:139`, `packages/store/src/types.ts` `AnswerRefusal`) |
| `conflict` | 409 | `bind` | `<project> is already bound with other fields:` and the stored project (`projects.ts:214-216`) |
| `not_open` | 409 | `answer` | `Not recorded. This request is no longer open.`, and `#n` of the subject's open decision when `open` is given (`primitives.md:140`) |
| `stale_basis`, `unknown_choice`, `bad_annotation` | 409 | `answer` | `Not recorded. The server refused it: <message>` (no added full stop). A client bug: the client quotes the basis it drew, offers only the record's choices, and checks ranges itself. |
| `extension_not_loaded` | 409 | `answer` | `Not recorded. The server has no code for <extension> now; the decision stays open.` On `␣ar` after a fault the decision is already answered and the refusal comes from that case (`packages/store/src/decisions.ts:194-199`, `:250-251`): `Not sent again. The server has no code for <extension> now; the answer stays recorded and its work is not started.` |
| `extension_not_loaded` | 409 | `create`, `setOptions` | `The server has no code for <extension> now.` (`calls.ts:279-283`). The start page says it before sending only when the remembered workflow is in the project's `missing`: `extensions` lists only what the server has, so nothing else tells a missing workflow from a missing tool |
| `bad_options` | 409 | `setOptions` | each error under its field by `path`; drafts stay (`options.ts:96-98`) |
| `options_invalid` | 409 | `create` | `<workflow> can't start: its options are not all set or don't fit`, then the Project view opens at the first field with an error, and the start page keeps the text. The server refuses while the workflow's `options` item has any `errors`, missing keys or values that don't fit (`calls.ts:287-298`, `options.ts:66-69`); `spec.md` §3.2 names both |
| `not_a_workflow` | 409 | `create` | the server's sentence: `websearch is not a workflow.`, or `render-core has no workflow w3.` (`calls.ts:274-286`). The workflow picker never offers one, so this is a race |
| `unknown_model` | 409 | `create`, `setModel` | `This server has no model <model>.` (`calls.ts:255-260`); the picker refetches `models` |
| `busy` | 409 | `continue` | `A run is working; there is nothing to continue.` |
| `idle` | 409 | `input` with `whenIdle` | never: this client never sends `whenIdle` |
| `event_log` | 409 | writes to the log | never: this client never opens the log as a conversation |
| `storage_failed` | 500, in `unknown` | any write | the call's unknown row of §2 |

### 6.1 `unauthorized` in the middle of a session

The server uses `unauthorized` for four cases: a refused `Host` or `Origin`, a wrong pairing code, a bad or revoked token, and a device revoked while its stream was opening (`http.ts:82`, `:100`, `:108`, `:133`). The client tells them apart as far as the protocol allows, and no further:

- **One branch reads `message`, and it is fragile:** when the message is exactly `This Host or Origin is not allowed.`, the client adds `If you reach the server through a proxy, start it with tau --allow-host <name>`. If the server rewords it, the client falls back to the next rule, which is safe. A distinct code is an open request to the core (`open-questions.md` §4).
- **Any `unauthorized` on a call:** show the server's sentence in a notice band (`The server refused this device: <message>`), stop sending writes (they wait in the outbox), keep the streams' backoff running, and keep every draft. Server line `× refused by the server`.
- **"Revoked" is claimed only after a stream ended with `end: revoked`** (§3.6). An `unauthorized` alone never puts the client in the revoked state and never tells the user to pair a new name.
- When a call succeeds again, the band goes and writes resume.

---

## 7. Pairing, the token, the network

- **First run.** The owner runs `tau device pair <name>` on the server; it prints a code `XXXX-XXXX` that works once for 10 minutes (`devices.ts:97-110`). The client sends `pair {code}` with no token; case, dashes and spaces in the code are ignored (`devices.ts:45`). The reply is `{device, token}`.
- **A lost `pair` reply.** The server records the device before it replies (`devices.ts:127-132`), so if the call landed the name is used and the token is gone. Connect shows: `No reply from the server. If the code was accepted, the name <name> is now used and this device never got its token. Run tau device pair <new name> on the server, pair with the new code, then revoke the old name: tau device revoke <name>.`
- **No server at the URL.** Connect shows: `No tau server answered at <url>: <error>. Check the address, and that tau is running there.`
- **Plain HTTP to another machine.** The transport is all that protects the token (`docs/core/protocol.md` "The network"). Before pairing over `http://` to a host that is not `127.0.0.1`, `localhost` or `[::1]`, Connect shows a warning under the field: `This address is not encrypted. Anyone on the network path can read this device's token. Use https:// (Tailscale Serve or a TLS proxy) or an SSH tunnel.` `⏎` then opens the Confirm float, `Pair over plain HTTP? Anyone on the network path can read this device's token.`, with hints `y pairs · n cancels`. The form never prints `y`: in a field it is a letter, and a wrong code counts toward the five tries that cancel every pending code (`devices.ts:118-122`).
- **Handshake.** Then `POST /v1/` with the token: the client expects `unknown_call` with `calls`, and the header `Tau-Protocol: 1`. Another protocol number: `This server speaks protocol <n>; this client speaks 1.` and nothing else is sent.
- **Storage.** The token goes in the macOS Keychain, item `tau <server url>`. The server URL and the device name go in `~/.config/tau/client.toml`. The token is never written to a file, a log or a URL.
- **Names.** A device name matches `^[a-z][a-z0-9-]{0,31}$` and is never used twice, even after a revoke (`devices.ts:19`, `devices.ts:99-100`).
- **The network.** The server listens on `127.0.0.1:6283`. A device on another machine comes through Tailscale Serve, a TLS proxy or an SSH tunnel; the proxy's host name must be allowed with `tau --allow-host` (`protocol.md` "The network"). The client sends no `Origin`.
- **What the client cannot do.** List or revoke devices: that is `tau device list` and `tau device revoke` on the server. The Server float says so.

---

## 8. Device-local state

Kept on this device per server URL, beside the outbox. It survives a restart, is never sent, and `:forget-server` deletes it with the token. `fixture.json` → `local` shows one instance.

| State | Shape | Rules |
|---|---|---|
| Sidebar visible | a boolean, `true` at first | the only sidebar state: `␣tb` sets it above 1000 px. Under a 1000 px window the sidebar is drawn only while it has the keys, and nothing changes the choice (`spec.md` §4.7) |
| Details panel visible | a boolean, `false` at first | `␣td` sets it, for every view |
| Open items | groups in order, each `{project, items}`; an item is `{kind, id, cursor?}`: a conversation and its number, a decision and its ID, or the project itself; `cursor` is where this device left the view's cursor | groups in the order of each project's first item, items in the order opened (`spec.md` §4.1). Added when the stage shows the object; removed only by `x` in the sidebar or `:q` |
| Places | per place, its `filter` (Conversations `{project, state}`; Timeline and Faults `{project}`; Home and Projects none) and its `cursor` (an object ID) | kept when you leave the place and across launches. A cursor whose object is gone moves to the nearest row |
| Last project | a project name | set when `create` from the start page succeeds |
| Last workflow per project | per project, `{workflow}` or `{model}` for the bare agent | set with the last project |
| Start page draft | `{text, project, workflow}` or `{text, project, model}`, or none | kept until `create` succeeds (§2.4) |

The tabs and each list tab's filter of the previous design are gone. The rest of the device-local state is unchanged: composer and note drafts, the outbox (§2), the label cache (§5.1), hidden faults and failed runs, the since-you-left base, the jumplist (`spec.md` §3.3). The app opens on the start page (D11), so the last view shown is not kept.

## 9. Conformance against the fixture

`fixture.json` holds replies and frames in the protocol's shapes. An implementation passes when, fed these:

| Input | Must show |
|---|---|
| `streams.inbox.first` | `◆ 3`; three Needs you rows, oldest first |
| `streams.inbox.later[0]` | `◆ 2`; the row leaves; the cursor, if it was there, moves on with the echo `answered elsewhere` |
| `streams.inbox.later[2]` while `decision render-core/w1:3:design#2` is open | the superseded banner; `␣ad` off; drafts kept |
| `streams.open notes-cli #4` first, then `later[0..5]` in order | a value equal to a fresh first frame after the same changes; the steer leaves the queue and appears as input; the tool block turns `ok` |
| `streams.ends.revoked` on any stream | the revoked state; no reconnect attempt |
| `refusals."unauthorized (revoked)"` on a call, with no `end` | the refused band of §6.1, writes held, streams still retrying; **not** the revoked state |
| `streams.ends.stopping` | backoff from 1 s; the server line `◷ server stopped · retrying` |
| `examples."gap rule"`: a reopen whose first frame has `before` above the newest held entry | exactly one `history` sequence for the gap, then no duplicate entries |
| `examples."gap rule, first open"`: a first frame with `before` set and nothing of that conversation held | **no** `history` call |
| `calls."conversations render-core (two pages)"` | both pages read; rows newest first; `#8`'s facts line ends `owned by #3` |
| `refusals.*` | the copy of §6 for each code, with no doubled full stop |
| an `answer` that times out, then `streams.inbox` without that decision and `calls."answer forecasts/w2:2:pi-durable-2#1 (resent after no reply)"`'s record from `decision` | `? 1`, then settled as landed with no `␣rt`: `? ` gone, drafts cleared |
| `local.outbox` with an unknown `input` | the unsent row above the composer; resent at the next connection; one submission |
| the same entry restored after a restart | `not sent automatically · ␣rt retries`; nothing sent until `␣rt` |
| `local.openItems` | the sidebar's `OPEN`: `notes-cli` with `#4` (steel triangle); `render-core` with `Review the design #2` (coral diamond), `#5` (rose `×`) and `render-core`; `forecasts` with `Resolve forecast #1` (coral diamond) |
| `local.lastProject`, `local.lastWorkflow`, no `local.startDraft` | `␣cn`: the line `render-core M-p · w1 workflow M-w` |
| the start page on `scratch` with `calls."options scratch"` | `needs options: designer, implementer, reviewer` before any text; `⌘⏎` opens the Project view at designer and sends nothing |
| `⌘⏎` with text on render-core and w1, then `calls."create w1 render-core"` and `calls."input start render-core #9"` | one `create`, then one `input` to conversation 9; the page reads `render-core › #9`; `#9` joins the sidebar under render-core; `lastWorkflow` for render-core stays `w1` |
| the same `create` timing out | no `input`, no resend; the text stays on the start page; `? 1` |
| `local.sidebarVisible` false, then `streams.ends.stopping` on every stream | the count tags `◆ 3 × 4` and `◷ server stopped · retrying` at the bottom bar's right end |
| launch, before any frame (no stream has delivered) | the server line `○ connecting`; no count tags; Home titled `Home` with `Nothing received from the server yet`; open items with no marks. Then `streams.inbox.first`: `◆ 3` and Needs you, with the line still in place of Failed and Running until `calls."conversations *"` and `calls."faults *"` have answered |
| the same launch, offline after 60 s | the band without `showing the state as of then` |
| `⌘⏎` twice on the start page before `calls."create w1 render-core"` replies | one `create`; the second press does nothing; `starting…` until the reply |
| the start page on render-core with `calls."options render-core (reviewer model gone)"` | the line `w1 workflow · options don't fit: reviewer`; the Project view's Extensions row `1 doesn't fit`; `⌘⏎` opens the Project view at `reviewer` and sends nothing |
| `refusals."options_invalid (create w1 in render-core, a model the server no longer has)"` | `w1 can't start: its options are not all set or don't fit`; the Project view at `reviewer`; the text kept |
| `hostile` | every case in its `cases` list shows as its `expect` says (`failure-catalogue.md` FS1–FS6) |
