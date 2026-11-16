# Core reconciliation

The design cites the tau core at `render-core` `b6bc814` (2026-11-14 16:37 +0400). This file checks each protocol claim of `client.md` and `spec.md` §2, §3.2 against `render-core` `f6ed8a7` (2026-11-15 21:27 +0400), checked out at `/tmp/tau-ref/tau`.

Paths below are relative to that checkout. pi-durable, chord and pi-schema are 1.5.2 in both commits (`package-lock.json:475-566`). Their sources were read from the Nix store copy of `node_modules` (`@earendil-works/pi-durable/dist/...`, cited as `pi-durable:<file>:<line>`).

Verdicts:

- `same`: the behaviour and the cited path are both unchanged.
- `moved`: only the path or line changed.
- `changed`: the behaviour changed.
- `missing`: the design does not cover something the core does.

How the code changed between the two commits:

- These files are byte-identical apart from their path: `http.ts`, `streams.ts`, `devices.ts`, `options.ts` and `cursors.ts`.
- `faults.ts` and `refusals.ts` changed only in comments. Their line numbers are unchanged.
- `calls.ts` shifted by about 31 lines. It gained the structural queue.
- `inbox.ts` gained a startup fix.
- `projects.ts` gained extension reload.
- `store/decisions.ts` gained a check that refuses an empty subject.
- `docs/core/*` was rewritten into `docs/protocol.md`, `docs/host.md`, `docs/primitives.md` and `docs/design.md`.

---

## 1. Envelopes and transport (`client.md` §1, §7)

| Claim in the design | Current core | Verdict |
|---|---|---|
| A call is `POST /v1/<name>`. Its body is at most 1 MiB (`http.ts:11`). | `BODY_LIMIT = 1024 * 1024` is at `core/src/host/http.ts:11`, and `too_large` is at `:42`. | moved |
| Every call but `pair` sends `Authorization: Bearer tau.<device>.<secret>` (`devices.ts:23`). | The regex is `/^Bearer tau\.([a-z][a-z0-9-]{0,31})\.([A-Za-z0-9_-]{43})$/` at `core/src/host/devices.ts:23`. The secret must be exactly 43 base64url characters. | moved |
| Every reply has `Tau-Protocol: 1` (`http.ts:28`). | JSON replies set it at `core/src/host/http.ts:28`. Event streams set it at `core/src/host/streams.ts:56`. | moved |
| (not in the design) A call sends `Content-Type: application/json`. | `docs/protocol.md:7` says a call has this header. The server does not check it (`core/src/host/http.ts:35-56`). | missing (doc only) |
| `{result}` comes with 200. `{refused:{code,message,…}}` comes with 400–413. `{unknown:{code,message}}` comes with 500 (`http.ts:137-146`). | Statuses are set at `core/src/host/http.ts:18-31`. Refusal statuses are at `core/src/host/refusals.ts:4-26`. Any throw before headers become `unknown`/`storage_failed`/500 at `core/src/host/http.ts:137-146`. A throw after headers destroys the response (`:139`). | moved |
| Only the body decides the outcome (old `protocol.md:46`). | `docs/protocol.md:19` says this. | moved |
| A proxy 408/409 page is unknown. | `docs/protocol.md:19` says any unrecognized body leaves the outcome unknown. The proxy example is no longer in the docs. | moved |
| The order of checks is Host/Origin, `pair`, token, call name, body (`http.ts:78-135`). | Host/Origin are checked at `core/src/host/http.ts:79-83`. `pair` comes next at `:84-103`, then the token at `:104-109`, the call name at `:110-122` and the body at `:123-124`. A stream re-checks the token after it opens (`:129-134`). | moved |
| `POST /v1/` with a good token answers `unknown_call` with `calls`. This is the handshake. | The code still does this. The name regex `^/v1/([A-Za-z]*)$` gives `""`, which is not a call, so the reply is `unknown_call` with `calls: Object.keys(calls)` (`core/src/host/http.ts:84`, `:110-122`). The message is `This server has no call /v1/.` (`:119`). The docs no longer describe the handshake: old "Versions" rules 202-207 are gone, and `docs/protocol.md:141` keeps only "ignore unknown fields, unknown codes are refusals, unknown request fields are `bad_request`". `pair` is not in `calls` (`core/src/host/calls.ts:193-646`; `pair` is routed at `http.ts:85`). | same (code), undocumented |
| `Host` must be loopback or `--allow-host`. `Origin`, when present, must be allowed too. | The loopback names are `127.0.0.1`, `localhost` and `[::1]` (`core/src/host/http.ts:12`). The checks are at `:63-77`. | moved |

## 2. Calls (`client.md` §4)

The design cited `calls.ts:162-616`. The list of calls is now at `core/src/host/calls.ts:193-646`. Every call takes a body that fits a closed schema. An unknown field is `bad_request` (`core/src/host/calls.ts:57-64`; `docs/protocol.md:141`).

| Call | Design: body / result | Current core | Verdict |
|---|---|---|---|
| `pair` | `{code}` → `{device, token}` | `core/src/host/http.ts:85-102`, `core/src/host/devices.ts:111-134` | moved |
| `extensions` | `{}` → `items:{name, workflow, schema?}` | `core/src/host/calls.ts:194-204`. The catalogue can now change at runtime (§8). Items come in name order, because modules are admitted sorted (`core/src/host/extensions.ts:69`). | changed (order; live catalogue) |
| `models` | `{}` → `items:{model, name}` | `core/src/host/calls.ts:205-212` | moved |
| `projects` | `{}` → `items:{project, workdir, extensions, missing}` | `core/src/host/calls.ts:213-215`, `core/src/host/projects.ts:133-146`. `missing` now reads the live catalogue (`projects.ts:137-146`). | moved (`missing` can shrink after a reload) |
| `bind` | `workdir, extensions` (can be empty) → `{project, repeated}` | `core/src/host/calls.ts:216-219`. `extensions` is `arr(str())`, so it can be empty. The result's `project` is a `ProjectView` object, not a name (`core/src/host/projects.ts:148-185`, `:245-255`; `docs/protocol.md:66`). The call runs on the structural queue and can answer `busy` (`docs/host.md:75`). | changed (`busy`) |
| `addExtensions` | `extensions` → `{project}` | `core/src/host/calls.ts:220-231`. `project` is a `ProjectView`. The call is structural and can answer `busy` (`docs/host.md:76`). | changed (`busy`) |
| `options` | → `items:{extension, schema, value, errors}`. A secret reads as `true`. | `core/src/host/calls.ts:232-250`, `core/src/host/options.ts:71-85`. The call is structural and can answer `busy` (`docs/host.md:77`). | changed (`busy`) |
| `setOptions` | `extension, patch` (`null` deletes a key) → `{value}` | `core/src/host/calls.ts:251-280`, `core/src/host/options.ts:87-100`. The call is structural and can answer `busy` (`docs/host.md:78`). | changed (`busy`) |
| `create` | `workflow` or `model` → `{conversation}` | `core/src/host/calls.ts:281-337`. The bare path is at `:285-299`. The workflow path is at `:300-336`. The call is structural and can answer `busy` (`docs/host.md:79`). | changed (`busy`) |
| `conversations` | `limit:500, cursor?` → page of `{conversation, owner?, state}`, oldest first | `core/src/host/calls.ts:338-378`. The scan is ascending (`core/src/host/cursors.ts:27-32`; `pi-durable:types.d.ts:744-748`). The event log is skipped (`calls.ts:359`). `limit` has no maximum (`calls.ts:181`). The default is 50 (`:176`). | moved |
| `open` | `conversation, after?` → stream of `ConversationView` | `core/src/host/calls.ts:379-384`. `after` is ≥ 0, and the watch starts `from: after + 1` (`:159-168`). | moved |
| `history` | `conversation, cursor?, limit?, minEntryId?, maxEntryId?, order?` → page of `EntryRecord` | `core/src/host/calls.ts:385-445`. Entry IDs are ≥ 1 (`:179`). The order defaults to descending. The bounds are inclusive (`docs/protocol.md:74`). Bounds sent with a cursor must equal the cursor's, or the call answers `bad_cursor` (`calls.ts:412-427`). | moved |
| `input` | `conversation, text, requestId, whenBusy?` → `{submission}` | `core/src/host/calls.ts:446-479`. The request ID is stored as `client:<device>:<requestId>` (`:461`). `whenBusy` defaults to `followUp` (`docs/host.md:103`). | moved |
| `continue` | `conversation, requestId` → `{submission}` | `core/src/host/calls.ts:480-498`. The prefix is set at `:487`. | moved |
| `stop` | `conversation` → `{}` | `core/src/host/calls.ts:499-505` | moved |
| `setModel` | `conversation, model, thinkingLevel?` (`off`…`max`) → `{}` | `core/src/host/calls.ts:506-531`. The levels are at `:182-190`. | moved |
| `inbox` | `{}` → stream | `core/src/host/calls.ts:532-536`. The open now goes through the structural queue, so it can be refused with `busy` while the server stops (`docs/host.md:87`). | changed (`busy`) |
| `decision` | `decision` → `{decision, text, then?}` | `core/src/host/calls.ts:537-569` | moved |
| `answer` | `decision, basis, choice, annotations` → `{decision, repeated}` | `core/src/host/calls.ts:570-604` | moved |
| `timeline` | `after?` → stream of the log's `ConversationView` | `core/src/host/calls.ts:605-613` | moved |
| `faults` | `cursor?, limit?` → page of `{task, kind, conversation, error, endedAt?, decision?}` | `core/src/host/calls.ts:614-646`, `core/src/host/faults.ts:14-21`. The newest task ID comes first (`faults.ts:74`; `docs/host.md:113`). Kinds of missing extensions are left out (`calls.ts:623-628`). | moved |
| No call was added, removed or renamed. | `Object.keys(calls)` gives the same 20 names in the same order (`core/src/host/calls.ts:193-646`). | same |

## 3. Refusals (`client.md` §6)

| Code / claim | Current core | Verdict |
|---|---|---|
| Status table (`refusals.ts:4-26`) | `core/src/host/refusals.ts:4-26` has the same 21 codes and statuses. | moved |
| `errors` items `{path, message}` (`bad_request`, `bad_options`) | `{path, keyword, message}`. `path` is a JSON pointer, and `""` is the root (`docs/protocol.md:21`). | same |
| `bad_request` | Body schema failures (`core/src/host/calls.ts:61`). Body not JSON or not an object (`core/src/host/http.ts:47-54`). `pair` body (`http.ts:89-95`). | moved |
| `bad_cursor` | `core/src/host/calls.ts:174-175`. pi-durable cursor errors map to it at `:65-72`. | moved |
| `unauthorized`: Host/Origin, code, token, revoked mid-open | The messages are `This Host or Origin is not allowed.` (`core/src/host/http.ts:82`), `That code does not work.` (`:100`), `Send a valid device token.` (`:108`) and `The device was revoked.` (`:133`). There is still no distinct code for Host. | moved |
| `unknown_call` carries `calls` | `core/src/host/http.ts:119-121`, `docs/protocol.md:28` | moved |
| `unknown_project` / `unknown_conversation` / `unknown_decision` | `core/src/host/calls.ts:88`, `:152-155`, `:553-556`. `answer` uses `No decision has the ID <id>.` (`core/src/store/decisions.ts:191-194`). | moved |
| `too_large` 413 | `core/src/host/http.ts:42` | moved |
| `conflict` on `answer` carries `decision` (answered) | `core/src/store/decisions.ts:234-240`, `core/src/store/types.ts:59` | moved |
| `conflict` on `bind` carries `project` (`ProjectView`) | `core/src/host/projects.ts:252-254`, `docs/protocol.md:31` | moved |
| `not_open` carries `open?` | `core/src/store/decisions.ts:219-229`, `core/src/store/types.ts:60` | moved |
| `stale_basis`, `unknown_choice`, `bad_annotation` | `core/src/store/decisions.ts:262-281` | moved |
| `extension_not_loaded` on `answer` (open or faulted resend) | `core/src/store/decisions.ts:196-201`, used at `:253` (faulted resend) and `:283` (open) | moved |
| `extension_not_loaded` on `create` / `setOptions` | `core/src/host/calls.ts:309-313`, `:255-263` | moved |
| `bad_options` carries `errors` | `core/src/host/calls.ts:264-277`, `core/src/host/options.ts:96-97` | moved |
| `options_invalid` (the design shows no extra field) | It carries `errors`, the same list the `options` call returns (`core/src/host/calls.ts:318-331`; `docs/protocol.md:36`). The message is always `The options of <w> are not all set.`, even for format errors. | same (the design can use `errors` directly) |
| `not_a_workflow` | `core/src/host/calls.ts:305-308`, `:315-316` | moved |
| `unknown_model` | `core/src/host/calls.ts:288-291`, `:515-519` | moved |
| `busy` comes only from `continue` ("A run is working") | `continue` still answers it (`core/src/host/calls.ts:493-494`). **New:** `bind`, `addExtensions`, `options`, `setOptions`, `create` and the `inbox` open answer `busy` with `The server is stopping.` once shutdown begins (`core/src/host/structural.ts:9-12`; `core/src/host/index.ts:131-134`; `docs/host.md:130`, `:75-87`). | **changed** |
| `idle` (only with `whenIdle`) | `core/src/host/calls.ts:474-475` | moved |
| `event_log` | `core/src/host/calls.ts:146` | moved |
| `storage_failed` in `unknown` | `core/src/host/http.ts:137-146` | moved |

## 4. Streams (`client.md` §3)

| Claim | Current core | Verdict |
|---|---|---|
| The reply is `text/event-stream` (`streams.ts:53-57`). A refusal comes first as JSON. | `core/src/host/streams.ts:53-57`. The refusal is sent at `core/src/host/http.ts:127-128`. | moved |
| A frame is `data: <json>` and a blank line. A heartbeat is the line `:`. | `core/src/host/streams.ts:41`, `:33`. The heartbeat is `:\n\n`. | moved |
| The first frame is `{"ops":[["r",value]]}` (`streams.ts:58`). | `core/src/host/streams.ts:58` | moved |
| The op kinds are `r s d a t p m`, with their argument shapes. | `docs/protocol.md:82-92`. The type is at `chord:delta/index.d.ts:19-21`. `s d a t` cannot target the root. `p` and `m` can. | moved |
| Refuse `__proto__`, `constructor`, `prototype`. | `docs/protocol.md:97` | moved |
| The heartbeat comes after 15 s without a frame (`streams.ts:6`, `:29-37`). | `core/src/host/streams.ts:6`, `:29-37` | moved |
| An unread stream is cut after 60 s with no `end` (`streams.ts:7`, `:43`). | `core/src/host/streams.ts:7`, `:43`. The cut destroys the socket (`:24`). | moved |
| The 45-second rule. | `docs/protocol.md:99` | moved |
| Backoff of 1, 2, 4 … 30 s (old `protocol.md:172`). | The core docs no longer give a schedule. It is the client's own policy. | changed (doc only) |
| `end` reasons `stopping`, `revoked`, `failed` (`devices.ts:42`, `streams.ts:26`). | `core/src/host/devices.ts:42`, `core/src/host/streams.ts:26`. `failed` is sent at `streams.ts:59-65`. `revoked` is sent at `devices.ts:136-141`. `stopping` is sent at `core/src/host/index.ts:131-134`. **New:** a reload whose publication fails closes the server, so streams also end `stopping` then (`core/src/host/index.ts:115-128`). | changed (one more `stopping` cause) |
| The `inbox` value is `{project:{subject:{id,title,requestedAt,conversation}}}`. | `core/src/store/types.ts:41-49`, `core/src/host/inbox.ts:24-32` | moved |
| A project bound during the stream arrives as an `s` on `[project]` (`inbox.ts:69-77`). | `core/src/host/inbox.ts:69-81`. A project bound between the first frame and `start` is now inside the first `r` value rather than lost (`:75`, `:79-81`, `:87-91`). A per-project `r` becomes `s` on `[project]` (`:17-20`). | moved (bug fixed) |
| One store's failed watch ends the inbox with `failed` (`inbox.ts:59-66`). | `core/src/host/inbox.ts:57-68`. A failed join after a bind also ends it (`:79-81`). | moved |
| `ConversationView` is `{conversation, entries, before?, docs}`. The docs are `pi.agent`, `pi.inbox`, `pi.live`, `pi.provider` and `pi.usage`. | `docs/protocol.md:174-179`, `pi-durable:harness/view.d.ts:6-17`, `pi-durable:harness/view.js:14-20`. Only documents that exist appear (`docs/protocol.md:388`). | moved |
| The window holds the newest 50, then drops its oldest at 100. With `after`, it starts there when at most 100 entries follow. | `docs/protocol.md:103`, `pi-durable:harness/view.d.ts:19-34`, `view.js:13`. With `from` past the newest entry, the window is empty and `before` is the newest entry (`view.d.ts:27`). | moved |
| The gap rule: `minEntryId: held+1`, `maxEntryId: before`. | `docs/protocol.md:105`. The test client is at `core/test/host/helpers.ts:235-245`. | moved |
| `timeline` uses the same window and gap rule. Its `conversation.id` names the log. | `docs/protocol.md:107`, `core/src/host/calls.ts:605-613` | moved |

## 5. Paging and cursors (`client.md` §3.3, §5)

| Claim | Current core | Verdict |
|---|---|---|
| Pages are `{items, next?}`. A missing `next` means the end. | `core/src/host/calls.ts:169-173`, `docs/host.md:68` | moved |
| `conversations` is oldest first. A page can come one short while `next` still pages on. | `core/src/host/cursors.ts:30` (ascending only), `core/src/host/calls.ts:359` (log skipped) | moved |
| `limit` maximum | None in tau (`core/src/host/calls.ts:181`) or in pi-durable sqlite (`pi-durable:storage/sqlite/storage.js:179-195`). The default is 50. | same |
| A cursor is opaque. Send it back to the same call. | It is base64url JSON `{call, project, query, position}` (`core/src/host/cursors.ts:61-70`). A cursor is refused for another call or project (`:73-91`). It works after a restart (`docs/protocol.md:74`). | moved |
| `history` cursor bounds | Bounds sent with a cursor must equal the stored ones. `order` must match too (`core/src/host/calls.ts:412-427`, `core/src/host/cursors.ts:97-101`). | same (clarification) |
| `faults` shape | `core/src/host/faults.ts:14-21`. It is newest first (`:74`). The cursor holds one position for each kind (`:22-24`). | moved |

## 6. Pairing and network (`client.md` §7)

| Claim | Current core | Verdict |
|---|---|---|
| The code is `XXXX-XXXX`, from an 8-character alphabet of 32 symbols. It works once and lasts 10 minutes. | `core/src/host/devices.ts:20`, `:22`, `:96-109` | moved |
| Case, dashes and spaces are ignored. | `core/src/host/devices.ts:45` | moved |
| Five wrong codes in a row clear every pending code. | `core/src/host/devices.ts:21`, `:117-124` | moved |
| The device is recorded before the reply. | `core/src/host/devices.ts:126-133` | moved |
| The device-name regex is `^[a-z][a-z0-9-]{0,31}$`, and a name is never reused. | `core/src/host/devices.ts:19`, `:99-100` | moved |
| The token is `tau.<device>.<secret>`, with 32 bytes in base64url. | `core/src/host/devices.ts:126`, `:133`, `docs/protocol.md:113` | moved |
| The server listens on `127.0.0.1:6283`. `--allow-host` adds names. | `core/src/host/index.ts:104`, `core/src/command/command.ts:11`, `:35-39`, `docs/host.md:18` | moved |
| `tau device pair` output | `Pairing code: <code>\nIt works once, until <ISO>.\n` (`core/src/command/command.ts:116-118`) | moved |
| `revoke` ends that device's streams with `revoked`. | `core/src/host/devices.ts:136-141` | moved |

## 7. Entry and document shapes (`client.md` §3.2, `spec.md` §2, §3.2)

All of these come from pi-durable 1.5.2, which is unchanged. The core now writes them out in `docs/protocol.md:139-506`.

| Claim | Current core | Verdict |
|---|---|---|
| `EntryRecord` `{id, conversationId, kind, model?, data?, head?, edits?, byTaskId?}` | `docs/protocol.md:164-173` | same |
| `pi.user`: `model[0]` is a user message. `content` is a string. | `pi-durable:entries.js:11`. `content` is `string \| (TextContent \| ImageContent)[]` (`docs/protocol.md:345`). | same, but the design assumes a string |
| `pi.assistant`: content blocks `text`, `thinking`, `toolCall`, and `timestamp` | `docs/protocol.md:346-371`, `:256-276`. It also has `stopReason`, `usage`, `errorMessage?` and `durationMs?`. | same |
| `pi.tool-result`: `toolCallId`, `isError` | `docs/protocol.md:372-384`. It also has `toolName` and `durationMs?`. `data` is always `{diagnostics: […]}` (`pi-durable:harness/tool.js:865`). | same |
| `pi.system` | `content` is empty, with `toolsAdded?` and `toolsRemoved?` (`pi-durable:entries.js:14-15`; `docs/protocol.md:336-343`). | same |
| Entry kinds the transcript renders: `pi.user`, `pi.assistant`, `pi.tool-result`, `pi.system` | Built-in kinds also include `pi.reset` and `pi.compaction` (`docs/protocol.md:188`; `pi-durable:entries.js:21-30`). | **missing** |
| `pi.live` `run`, `runFailure`, `generation`, `tools` | `docs/protocol.md:464-506`. `runFailure` is `{taskId, inputs, reason, detail?, first?}`, and the reason can be `aborted` (`:488-503`). `ToolSlot.status` is `pending`, `running` or `done` (`:442`). | same |
| State is `running` when `run` is present, else `failed` when `runFailure` is present, else `idle`. | `core/src/host/calls.ts:364-369` | moved |
| `pi.inbox.items` | `docs/protocol.md:426-436` | same |
| `pi.agent` `model`, `thinkingLevel`, `extensions`, `cwd` | `docs/protocol.md:391-425`. `extensions` is `string[]` or `{add?, remove?}`. | same |
| `spec.md` §2: "A pinned shape for `pi.usage`" is missing. | It is now documented as `{models: Record<string,Usage>, tools: Record<string,Usage>}` (`docs/protocol.md:437`). | changed (now pinned) |
| `spec.md` §2 D12: there is no tool duration. | A tool result's message has `durationMs?` (`docs/protocol.md:382`). A running slot still has no start time (`:439-451`). | changed (doc now exposes it) |
| A subject is `${extension}:${key}`. | This is a convention ("should"), not enforced (`docs/primitives.md:40`). An empty subject is refused (`core/src/store/decisions.ts:107-108`). | same |
| A decision ID is `${subject}#${n}`. | `core/src/store/types.ts:8`, `docs/primitives.md:40` | moved |
| Options that stop a workflow come from `options` `errors`, with defaults filled in. | `core/src/host/options.ts:66-69`, `:80-85`, `core/src/host/calls.ts:318-331` | moved |
| `stop` ends what runs then. | `docs/host.md:105`. `stop` also withdraws queued inputs. | moved |
| A repeated `create` makes a second conversation (an accepted limit). | `docs/design.md:74`, `docs/host.md:99` | moved |

## 8. New in the core since `b6bc814`, visible to clients

| What | Where | Effect on a client |
|---|---|---|
| Structural queue | `core/src/host/structural.ts:1-21`. Its users are at `core/src/host/calls.ts:96-126` and `:532-536`. | `bind`, `addExtensions`, `options`, `setOptions`, `create` and the `inbox` open run one at a time with reload. They wait behind a running reload. Once shutdown starts, they answer `busy`. |
| `tau extension reload` | `core/src/host/reload.ts`, `core/src/host/projects.ts:216-237`, `core/src/host/index.ts:115-129`, `docs/extensions.md:374-390` | These can change on a live server with no stream event: the list from `extensions`, schemas, `projects[].missing`, and the items from `options`. A reload adds or replaces extensions. It never removes one. A failed publication stops the server, so streams end `stopping`. |
| Empty subject refused | `core/src/store/decisions.ts:107-108` | None. Inbox subjects are never `""`. |

## 9. Fixture shapes that do not match the current core

None of these come from a core change; they did not match at `b6bc814` either. They should still be fixed before the client tests against them.

| Fixture path | Fixture | Current core | Fix |
|---|---|---|---|
| `calls."continue render-core #5".result.submission.status` | `"queued"` | A `continue` submission is never `queued`. It is `placed`, `done` or `unanswered` (`pi-durable:types.d.ts:514-537`; `docs/protocol.md:215-222`). | `"placed"` |
| `streams."open render-core #5"` and `"open notes-cli #5"`, `docs.pi.live.runFailure` | `{reason, message}` | `{taskId, inputs, reason, detail?, first?}`. There is no `message`. The error text is in `detail` (`docs/protocol.md:488-503`; `pi-durable:harness/live.d.ts:96-109`). | Rename `message` to `detail`. Add `taskId` and `inputs`. |
| Every `pi.tool-result` entry (20 entries) | no `data` | `data: {diagnostics: […]}` is always written (`pi-durable:harness/tool.js:865`). | Add `"data":{"diagnostics":[]}` |
| `pair.reply.result.token` | The secret has 44 characters. | The secret must be exactly 43 characters (`core/src/host/devices.ts:23`). This token would be `unauthorized`. | Use a 43-character secret. |
| `calls.extensions.result.items` | Ordered `websearch`, `w2`, `w1` | Admitted in sorted name order: `w1`, `w2`, `websearch` (`core/src/host/extensions.ts:69`). | Reorder. The client must not depend on the order. |
| `options_invalid` / `options scratch` `required` errors | `"message":"required"` | pi-schema says `"missing"`. The path `/designer` and keyword `required` match (`pi-schema:explain.js:178`). | `"message":"missing"` |
| `refusals.bad_request` | `"message":"must be one of steer, followUp"` | pi-schema says `expected one of ["steer","followUp"]` (`pi-schema:explain.js:67`, `:358-363`). | Use the pi-schema text. |
| `examples."gap rule"` | Reopen `after: 12`, then `history` with `minEntryId: 13`, `maxEntryId: 29`. | Entry IDs are scoped to the store, so IDs 13–29 hold at most 17 entries. With at most 100 entries after `after`, the window starts at `after + 1` and `before` ≤ 12, so no gap occurs (`docs/protocol.md:103`; `pi-durable:harness/view.d.ts:19-29`). | Use a gap of more than 100 entries, or a folded frame. |
| `server.discovery`, `pair.*`, `refusals.*` messages, `conversations` cursors, `history` cursor, inbox ops, `open notes-cli #4` ops, `faults`, `decision`, `bind`, `addExtensions`, `setOptions` | — | These match the current code: `core/src/host/http.ts:119`, `core/src/host/cursors.ts:61-70`, `core/src/host/inbox.ts:17-20`, `core/src/host/faults.ts:82-107` and `core/src/store/decisions.ts:191-281`. | none |

---

## What the client must do differently

These are only the substantive changes. Path and line moves are not listed.

1. **`busy` is not only `continue`.** `bind`, `addExtensions`, `options`, `setOptions` and `create` can be refused with `busy` / `The server is stopping.` (`core/src/host/structural.ts:9-12`; `docs/host.md:130`). The client must branch on the call, not on the code alone:
   - `busy` on `continue` keeps the current copy.
   - `busy` on a structural call means "not done, the server is stopping". It is safe to send again after the server is back. For `create`, nothing was created.
   - The `inbox` open can also be refused with a JSON `busy` (`core/src/host/calls.ts:532-536`). Treat that like `end: stopping` and back off.
2. **The extension catalogue is live.** `tau extension reload` can change these on a running server with no stream event: the list from `extensions`, each workflow's `schema`, `projects[].missing`, and the items from `options` (`core/src/host/projects.ts:216-237`; `docs/extensions.md:380-390`). Do not cache these for the session. Refetch them on reconnect, when a view opens, and after any `extension_not_loaded` or `not_a_workflow` refusal. A failed reload stops the server, so streams end `stopping` (`core/src/host/index.ts:115-128`).
3. **Structural calls queue behind a reload.** A `create` that waits longer than the client's 20 s timeout becomes "unknown", and it may still run (`core/src/host/structural.ts:9-14`). §2.4's rule of never resending `create` already covers this. Expect such cases more often during a reload.
4. **`options_invalid` carries `errors`** (`core/src/host/calls.ts:318-331`). The client can name the failing fields from the refusal itself rather than refetch `options`.
5. **Render or skip `pi.reset` and `pi.compaction`.** Both are built-in entry kinds (`docs/protocol.md:188`).
   - `pi.reset` has `model` absent, or holds a user handoff message.
   - `pi.compaction` holds a user summary message and has `head` (`pi-durable:entries.js:21-30`).
   - `pi.user` `content` can be an array of text and image blocks, not only a string (`docs/protocol.md:345`). Take labels from the first text block.
6. **The handshake rests on code, not on the docs.** `POST /v1/` still answers `unknown_call` with `calls` (`core/src/host/http.ts:84`, `:110-122`). The current `docs/protocol.md` no longer promises it. Also check `Tau-Protocol` on every reply. If the handshake ever comes back as something other than `unknown_call`, judge the server by the header alone.
7. **Send `Content-Type: application/json`** on every call (`docs/protocol.md:7`). The server does not enforce it today.
8. **`history` with a cursor:** send the same bounds and `order` as the first page, or omit them all. Any difference is `bad_cursor` (`core/src/host/calls.ts:412-427`). `maxEntryId` must be ≥ 1 (`:179`), so stop scrolling back when the oldest entry held is 1.
9. **Fix the fixture** before conformance tests (§9). The fixes are:
   - `continue` status
   - the `runFailure` shape
   - `pi.tool-result` `data`
   - the 43-character token
   - the order of `extensions`
   - the pi-schema messages
   - the gap-rule example
10. **The design text should absorb two points:**
    - The backoff schedule is the client's own policy; the core docs no longer give one.
    - `pi.usage` and the tool result's `durationMs` are now documented (`docs/protocol.md:437`, `:382`). `spec.md` §2's "no pinned shape for `pi.usage`" is out of date.
