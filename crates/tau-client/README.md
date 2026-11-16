# tau-client

The desktop app's link to one tau server. It owns everything between the UI and the wire: pairing and the token,
the handshake, calls and their outcomes, the outbox and its resends, the `inbox`, `timeline` and `open` streams and
their supervision, polling, labels, and one read-only model of what the server last said. It implements
`design/client.md`, corrected by `design/core-reconciliation.md` where the core differs. It does not depend on
rune, and it holds no UI words: it reports domain outcomes (refusal codes and the server's messages, unknown
reasons, stream states) and the UI writes the copy.

## Public surface

The UI owns one `Link`.

| Group | Calls |
|---|---|
| Make | `Link::open(LinkConfig) -> Link` (paired or not, from `client.toml` and the Keychain), `LinkConfig::standard()`, `Link::fixture(Fixture::design())` for demo mode and headless renders, `Link::with_transport(config, Arc<dyn Transport>)` |
| Frame loop | `set_waker(Fn() + Send + Sync)`, `pump() -> bool` (applied anything the UI shows?), `next_deadline() -> Option<Duration>` |
| Read | `server() -> &Server`, `outbox() -> &Outbox`, `status() -> &LinkStatus`, `take_events() -> Vec<LinkEvent>`, `store() -> &Store` for the UI's own device-local state |
| Write | `write(Write) -> Result<WriteId, NotSent>`, `retry(WriteId) -> Result<(), NotSent>` (`␣rt`), `forget(WriteId)` (`x`) |
| Watch and fetch | `show(Slot::Stage \| Slot::Peek, Option<ConvRef>)`, `watch_decision(Option<DecisionRef>)`, `fetch_decision(DecisionRef)`, `fetch_options(Project)`, `refresh_catalogue()`, `refresh_faults()`, `page_older(TranscriptKey)`, `want_label(ConvRef)`, `set_focused(bool)`, `reconnect()` (`␣rc`) |
| Server | `pair(url, code)`, `forget_server()` |

`Write` has one arm per write call: `Input {conv, text, mode}`, `Continue`, `Stop`, `SetModel`, `SetOptions`,
`Bind`, `AddExtensions`, `Answer`, and `Start {project, with: Workflow | Model, text}`, which makes `create` and,
only on its result, `input`.

`Server` holds each fetched thing as `Known::NotYet`, `Known::Have {value, at}` or `Known::Gone {message}`, so
the UI can say "Nothing received from the server yet" instead of drawing an empty list. `LinkStatus.phase` is the
server line: not paired, pairing, other protocol, revoked, refused by the server, offline since, connecting,
server stopped, reconnecting in N s, live.

Server messages change `server()`, `outbox()` and `status()` only inside `pump`, so keys resolve against the frame
the user saw (`design/reference/grammar.md` §7). Call `pump` when the waker fires and once per frame.

## Isolated storage

`LinkConfig::standard()` reads `TAU_CLIENT_DIR` instead of `~/.config/tau` when set.
That directory holds both `client.toml` and per-server state. The macOS Keychain remains the default token store.
Set `TAU_TOKEN_DIR` explicitly to use private token files in that directory, including on macOS.
The real-server UI check sets both paths to a fresh temporary directory.

## Engine and runtime

| Module | Owns |
|---|---|
| `ids` | branded identifiers: `Project`, `ConversationId`, `EntryId`, `DecisionId`, `ConvRef`, `DecisionRef`, ... |
| `wire` | request bodies, the three reply envelopes, refusal codes, result shapes; replies are parsed here and trusted inside |
| `sse`, `ops` | bytes to frames; stream ops applied to a JSON value, refusing prototype path segments |
| `shapes` | lenient typed views of pi entries and documents, and the names `spec.md` §3.2 derives |
| `model` | `Server`, the read-only model |
| `outbox` | entries with their exact bodies, states, the resend table and schedule |
| `streams` | one stream's supervision: phases, backoff, silence, `end` causes |
| `engine` | the pure core: UI commands, replies, frames and ticks in; calls, stream opens and closes, saves and deadlines out. It never reads a clock or does I/O |
| `link` | the runtime: carries effects out on threads, queues what comes back for `pump`, and runs a timer on a clock that counts sleep |
| `transport`, `http`, `fixture` | one small trait, carried out over HTTP/1.1 (plain or TLS) or answered from `design/fixture.json` |
| `store`, `clock` | device-local files per server URL, `client.toml`, the token in the Keychain; the two clocks |

## Tests

`crates/tau-client/failure-modes.md` lists every failure the tests guard against, written before the code.

- `nix run .#test` runs the workspace tests. This crate's tests feed the engine and the `Link` from
  `design/fixture.json`; they need no server.
- `nix run .#e2e` starts a real tau server from the pinned `tau` flake input, with the fake model `faux/echo` and
  the w2 extension (`tools/e2e/`), and runs `tests/e2e.rs` against it.
- `nix run .#lint` runs rustfmt, clippy with `-D warnings`, and nixfmt.
