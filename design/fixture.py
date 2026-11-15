"""Generates fixture.json beside this file: python3 -B fixture.py. Keeps entry and task IDs consistent per store; edit this script, not the JSON."""
import json
import os
from datetime import datetime, timezone

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "fixture.json")


def ms(month, day, hh, mm, ss=0):
    return int(datetime(2026, month, day, hh, mm, ss, tzinfo=timezone.utc).timestamp() * 1000)


NOW = ms(11, 14, 16, 40)
BASE = ms(11, 14, 12, 30)  # this device lost focus at 12:30 ("since you left" base)

ZERO_USAGE = {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "totalTokens": 0,
              "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "total": 0}}


def usage(i, o):
    return {"input": i, "output": o, "cacheRead": 0, "cacheWrite": 0, "totalTokens": i + o,
            "cost": {"input": round(i * 3e-6, 4), "output": round(o * 15e-6, 4), "cacheRead": 0,
                     "cacheWrite": 0, "total": round(i * 3e-6 + o * 15e-6, 4)}}


API = {"anthropic": "anthropic-messages", "openai": "openai-responses", "google": "google-generative-ai"}


class Store:
    """One project's store: entry IDs and task IDs are allocated in time order."""

    def __init__(self, name, first_entry=1, first_task=1):
        self.name = name
        self.items = []  # (time, seq, conversation, builder)
        self.seq = 0
        self.task = first_task
        self.first_entry = first_entry
        self.by_key = {}

    def task_id(self):
        t = self.task
        self.task += 1
        return t

    def add(self, t, conv, entry, key=None):
        self.seq += 1
        self.items.append((t, self.seq, conv, entry, key))

    def build(self):
        self.items.sort(key=lambda x: (x[0], x[1]))
        out = {}
        nid = self.first_entry
        for t, _, conv, entry, key in self.items:
            e = {"id": nid, "conversationId": conv, **entry}
            nid += 1
            out.setdefault(conv, []).append(e)
            if key:
                self.by_key[key] = e
        self.next_entry = nid
        return out


def user(t, text):
    return {"kind": "pi.user", "model": [{"role": "user", "content": text, "timestamp": t}]}


def system(t, task, tools):
    return {"kind": "pi.system", "model": [{"role": "system", "content": "", "toolsAdded": [
        {"name": n, "description": d, "parameters": {"type": "object"}} for n, d in tools], "timestamp": t}],
            "byTaskId": task}


def assistant(t, task, model, content, stop="toolUse", u=(1800, 300)):
    provider, mid = model.split("/", 1)
    return {"kind": "pi.assistant", "model": [{"role": "assistant", "content": content, "api": API[provider],
                                               "provider": provider, "model": mid, "usage": usage(*u),
                                               "stopReason": stop, "timestamp": t}], "byTaskId": task}


def text(s):
    return {"type": "text", "text": s}


def call(cid, name, args):
    return {"type": "toolCall", "id": cid, "name": name, "arguments": args}


def result(t, task, cid, name, out, err=False):
    return {"kind": "pi.tool-result", "model": [{"role": "toolResult", "toolCallId": cid, "toolName": name,
                                                 "content": [text(out)], "isError": err, "timestamp": t}],
            "byTaskId": task}


def typed(kind, data, task):
    return {"kind": kind, "data": data, "byTaskId": task}


CODING = [("read", "Read a file."), ("write", "Write a file."), ("edit", "Replace text in a file."),
          ("bash", "Run a shell command.")]
SEARCH = [("web_search", "Search the web.")]

# ---------------------------------------------------------------- notes-cli (bare agent only)
nc = Store("notes-cli")
NC_LOG = 1
# conversation 1 is the event log, created in the store's first commit; it holds no entries here.
M_SON = "anthropic/claude-sonnet-4-5"
M_OPUS = "anthropic/claude-opus-4-5"
M_GPT = "openai/gpt-5.1"

# #2, idle, Nov 13
t0 = ms(11, 13, 10, 2)
nc.add(t0, 2, user(t0, "Add a `--json` flag to `notes list` so scripts can read the output."))
tk = nc.task_id()
nc.add(t0 + 900, 2, system(t0 + 900, tk, CODING))
nc.add(t0 + 9000, 2, assistant(t0 + 9000, tk, M_SON, [text(
    "Done. `notes list --json` prints one JSON array of `{ id, title, updated }` and nothing else, so `jq` can read it. "
    "The human format is unchanged.")], stop="stop"))

# #3, idle, Nov 12
t0 = ms(11, 12, 18, 20)
nc.add(t0, 3, user(t0, "Why does `notes sync` hang on vaults over 10k files?"))

# #4, running now, with tool calls and a queued steer
t0 = ms(11, 14, 16, 21, 10)
nc.add(t0, 4, user(t0, "Port the config loader from JSON to TOML. Keep reading `notes.json` for one release and warn when it is used."), key="nc4.user")
run4 = nc.task_id()
nc.add(t0 + 800, 4, system(t0 + 800, run4, CODING))
nc.add(t0 + 6000, 4, assistant(t0 + 6000, run4, M_SON, [
    text("I'll start with the loader and see who calls it."),
    call("toolu_01", "read", {"path": "src/config/load.ts"})]))
nc.add(t0 + 6400, 4, result(t0 + 6400, nc.task_id(), "toolu_01", "read",
                            "import { readFileSync } from \"node:fs\";\n\nexport function loadConfig(path = \"notes.json\") {\n  return JSON.parse(readFileSync(path, \"utf8\"));\n}\n"))
nc.add(t0 + 11000, 4, assistant(t0 + 11000, run4, M_SON, [
    call("toolu_02", "bash", {"command": "grep -rn \"loadConfig\" src test | head -20"})]))
nc.add(t0 + 11500, 4, result(t0 + 11500, nc.task_id(), "toolu_02", "bash",
                             "src/cli.ts:4:import { loadConfig } from \"./config/load.js\";\nsrc/cli.ts:18:  const config = loadConfig(flags.config);\ntest/config.test.ts:2:import { loadConfig } from \"../src/config/load.js\";\n"))
nc.add(t0 + 19000, 4, assistant(t0 + 19000, run4, M_SON, [
    text("Only `src/cli.ts` and one test call it. I'll add `smol-toml` for parsing."),
    call("toolu_03", "bash", {"command": "npm install smol-toml@1.3.1"})]))
nc.add(t0 + 24000, 4, result(t0 + 24000, nc.task_id(), "toolu_03", "bash",
                             "npm error code ENOTFOUND\nnpm error network request to https://registry.npmjs.org/smol-toml failed, reason: getaddrinfo ENOTFOUND registry.npmjs.org\n\nexit code 1",
                             err=True))
nc.add(t0 + 41000, 4, assistant(t0 + 41000, run4, M_SON, [
    text("The network is closed here, so I'll write a small parser for the subset `notes.json` uses: "
         "strings, numbers, booleans and one level of tables."),
    call("toolu_04", "write", {"path": "src/config/toml.ts", "content": "…"})]))
nc.add(t0 + 41600, 4, result(t0 + 41600, nc.task_id(), "toolu_04", "write", "Wrote 84 lines to src/config/toml.ts"))
nc.add(t0 + 52000, 4, assistant(t0 + 52000, run4, M_SON, [
    call("toolu_05", "edit", {"path": "src/config/load.ts", "old": "return JSON.parse(readFileSync(path, \"utf8\"));", "new": "…"})]))
nc.add(t0 + 52300, 4, result(t0 + 52300, nc.task_id(), "toolu_05", "edit", "Replaced 1 occurrence in src/config/load.ts"))
nc.add(t0 + 61000, 4, assistant(t0 + 61000, run4, M_SON, [
    call("toolu_06", "bash", {"command": "npm test -- test/config.test.ts"})]), key="nc4.last")
tool6 = nc.task_id()

# #5, failed, today 15:02
t0 = ms(11, 14, 15, 2)
nc.add(t0, 5, user(t0, "Write a migration test for v3 notes with front matter."), key="nc5.user")
run5 = nc.task_id()
nc.add(t0 + 700, 5, system(t0 + 700, run5, CODING))
nc.add(t0 + 5200, 5, assistant(t0 + 5200, run5, M_SON, [
    text("Let me look at how v3 notes are parsed first."),
    call("toolu_11", "read", {"path": "src/migrate/v3.ts"})]))
nc.add(t0 + 5600, 5, result(t0 + 5600, nc.task_id(), "toolu_11", "read", "export function migrateV3(source: string) {\n  …\n}\n"))

nc_entries = nc.build()

# ---------------------------------------------------------------- render-core (websearch + w1)
rc = Store("render-core")
RC_LOG = 1
WD = "/home/vrod/src/render"

DESIGN_V1 = """# Event log export

`tau export --events <project>` writes every event of one project to a file as one JSON array, oldest first.

## Shape

The file is a single array of event entries, as `history` returns them.

## Where it runs

The command reaches the running server through `admin.sock`, as `tau device` does. It never opens a store itself.

## Not in scope

Transcripts and decisions.
"""

DESIGN_V2 = """# NDJSON export of the event log

`tau export --events <project>` writes every event of one project to standard output, one JSON object per line, oldest first.

## Why

The timeline is the one record that every client reads the same way. Today it can only be paged through `history`. An export lets the owner keep it outside the store, diff two exports, and feed it to other tools.

## Shape of a line

Each line is one event entry as `history` returns it, with nothing added or removed:

```json
{"id":57,"conversationId":1,"kind":"w1.event","data":{"text":"The design is ready for review.","at":1794649920000,"conversation":3,"stage":"design_review"}}
```

- Lines end with `\\n`. The output has no header and no trailing comma.
- Keys keep the order that `history` gives them, so two exports of the same store are byte-equal.
- A line is at most 1 MiB, the protocol's body limit. A longer event is written whole; the limit is reported, not enforced.

## Where it runs

The command reaches the running server through `admin.sock`, as `tau device` does. It never opens a store itself, because one process writes a store at a time.

| Step | What happens |
| --- | --- |
| 1 | `tau export` sends `{ "command": "export", "project": "render-core" }` on the socket. |
| 2 | The server pages the log with `history`, 500 entries a page, ascending. |
| 3 | Each page is written to the socket as lines, then flushed. |
| 4 | The server ends with one line, `{"done":true,"events":412}`. |

## Partial writes

A cut socket or a full disk can stop an export halfway. The export then ends without its `done` line, and the command exits with status 1 and says how many lines it wrote. A reader treats output without the last line as incomplete. Nothing is retried, and nothing in the store changes.

## What it does not do

- It exports events only, not transcripts or decisions.
- It does not filter by kind or time. `jq` does that well.
- It does not follow new events. A second export is cheap.

## Open points

1. Should the export start with a line that names the log's conversation? I left it out, so that every line is an event.
2. 500 entries a page is a guess. A page of `w1.event` entries is about 120 KB.
"""


def rng(textv, quote):
    i = textv.index(quote)
    # UTF-16 code units: the texts here are ASCII except for none, so indexes equal code units.
    assert all(ord(c) < 0x10000 for c in textv)
    return {"start": i, "end": i + len(quote)}


# #2 bare, Nov 13
t0 = ms(11, 13, 10, 0)
rc.add(t0, 2, user(t0, "Explain how the outbox replays a lost answer."))

# Work A: designer #3
t0 = ms(11, 14, 9, 10)
rc.add(t0, 3, user(t0, "Add an export of the event log: `tau export --events` should write every event of a project to a file."), key="rc3.user")
runA = rc.task_id()
rc.add(t0 + 900, 3, system(t0 + 900, runA, CODING[:1] + CODING[3:] + SEARCH + [("publish_design", "Publish your design for the owner's review.")]))
rc.add(t0 + 8000, 3, assistant(t0 + 8000, runA, M_OPUS, [
    text("I'll read how the log is stored first."),
    call("toolu_a1", "read", {"path": "packages/store/src/events.ts"})]))
rc.add(t0 + 8400, 3, result(t0 + 8400, rc.task_id(), "toolu_a1", "read", "export async function emit<D extends EventData>(…\nexport async function eventLog(tx: Tx): Promise<ConversationId> {…\n"))
rc.add(t0 + 20000, 3, assistant(t0 + 20000, runA, M_OPUS, [
    call("toolu_a2", "bash", {"command": "rg -n \"admin.sock|requestAdmin\" packages/tau packages/host/src | head"})]))
rc.add(t0 + 20600, 3, result(t0 + 20600, rc.task_id(), "toolu_a2", "bash", "packages/host/src/admin.ts:12:export function adminSocket(home: string) {\npackages/tau/src/command.ts:5:import { type AdminRequest, requestAdmin, startServer } from \"tau-host\";\n"))
tpub1 = ms(11, 14, 9, 52)
pubtask1 = rc.task_id()
rc.add(tpub1, 3, assistant(tpub1 - 30000, runA, M_OPUS, [call("toolu_a3", "publish_design", {"text": "# Event log export\n…"})]))
rc.add(tpub1 + 10, 3, typed("w1.design", {"text": DESIGN_V1}, pubtask1), key="rc3.v1")
rc.add(tpub1 + 20, RC_LOG, typed("w1.event", {"text": "The design is ready for review.", "at": tpub1, "conversation": 3, "stage": "design_review"}, pubtask1), key="ev.a1")
rc.add(tpub1 + 30, 3, result(tpub1 + 30, pubtask1, "toolu_a3", "publish_design", "Published the design for review in decision w1:3:design#1. End your turn."))
rc.add(tpub1 + 6000, 3, assistant(tpub1 + 6000, runA, M_OPUS, [text("The design is published as w1:3:design#1. I'll wait for your review.")], stop="stop"))

# Work B: designer #4, implementer #5 (fails)
t0 = ms(11, 14, 10, 30)
rc.add(t0, 4, user(t0, "Make `tau device list` print when each device last called the server."), key="rc4.user")
runB = rc.task_id()
tpubB = ms(11, 14, 11, 20)
pubB = rc.task_id()
rc.add(tpubB, 4, typed("w1.design", {"text": "# Last call per device\n\nThe server records the time of each device's last authenticated call in memory and writes it to `devices.json` at most once a minute. `tau device list` prints it as a third column.\n"}, pubB), key="rc4.v1")
rc.add(tpubB + 10, RC_LOG, typed("w1.event", {"text": "The design is ready for review.", "at": tpubB, "conversation": 4, "stage": "design_review"}, pubB), key="ev.b1")

# Work C: designer #6, approve by phone at 14:10, then task faults
t0 = ms(11, 14, 11, 0)
rc.add(t0, 6, user(t0, "Show the server's protocol version in `tau --version`."), key="rc6.user")
runC = rc.task_id()
tpubC = ms(11, 14, 11, 25)
pubC = rc.task_id()
rc.add(tpubC, 6, typed("w1.design", {"text": "# Protocol version in `tau --version`\n\n`tau --version` prints the package version and, on a second line, `protocol 1`, read from the same constant the server sends as `Tau-Protocol`.\n"}, pubC), key="rc6.v1")
rc.add(tpubC + 10, RC_LOG, typed("w1.event", {"text": "The design is ready for review.", "at": tpubC, "conversation": 6, "stage": "design_review"}, pubC), key="ev.c1")

# Work B approved by laptop at 11:40
tB_ans = ms(11, 14, 11, 40)
thenB = rc.task_id()
rc.add(tB_ans + 500, RC_LOG, typed("w1.event", {"text": "The design is approved. W1 prepares branch w1/4 from main.", "at": tB_ans + 500, "conversation": 4, "stage": "preparing"}, thenB), key="ev.b2")
prepB = rc.task_id()
tI = ms(11, 14, 11, 42)
rc.add(tI, 5, user(tI, "The owner approved this design in decision w1:4:design#1. Build it in your worktree at /home/vrod/src/render.w1-4, on branch w1/4.\n\n<design>\n# Last call per device\n\nThe server records the time of each device's last authenticated call in memory and writes it to `devices.json` at most once a minute. `tau device list` prints it as a third column.\n</design>\n\nThe owner's annotations on the approved design. Reply to every one by its number when you publish.\n\n<annotations>\nnone\n</annotations>\n\nCommit your work, leave the worktree clean, and call `publish_build`."), key="rc5.user")
rc.add(tI + 50, RC_LOG, typed("w1.event", {"text": "The implementer is building the approved design.", "at": tI + 50, "conversation": 5, "stage": "building"}, prepB), key="ev.b3")
runI = rc.task_id()
rc.add(tI + 900, 5, system(tI + 900, runI, CODING + SEARCH + [("publish_build", "Publish your commit for review.")]))
rc.add(tI + 9000, 5, assistant(tI + 9000, runI, M_SON, [
    text("I'll find where devices are checked."),
    call("toolu_b1", "read", {"path": "packages/host/src/devices.ts"})]))
rc.add(tI + 9400, 5, result(tI + 9400, rc.task_id(), "toolu_b1", "read", "export class Devices {\n  readonly #file: string;\n  …\n"))
rc.add(tI + 600000, 5, assistant(tI + 600000, runI, M_SON, [
    call("toolu_b2", "edit", {"path": "packages/host/src/devices.ts", "old": "authenticate(header", "new": "…"})]))
rc.add(tI + 600300, 5, result(tI + 600300, rc.task_id(), "toolu_b2", "edit", "Replaced 1 occurrence in packages/host/src/devices.ts"))
rc.add(tI + 1500000, 5, assistant(tI + 1500000, runI, M_SON, [
    call("toolu_b3", "bash", {"command": "node --test packages/host/test/devices.test.ts"})]))
rc.add(tI + 1512000, 5, result(tI + 1512000, rc.task_id(), "toolu_b3", "bash", "✖ the list prints the last call (12.4ms)\n  AssertionError: expected 3 columns, got 2\n\nexit code 1", err=True), key="rc5.last")
# 12:51 the run fails on a model error; no entry, only pi.live.runFailure

# Work A revise by phone at 13:30, then the designer revises
tA_ans = ms(11, 14, 13, 30)
thenA = rc.task_id()
ANN_A = [{"text": "Use NDJSON, not one array: a reader should be able to stream it.",
          "range": rng(DESIGN_V1, "writes every event of one project to a file as one JSON array, oldest first.")},
         {"text": "What happens on a partial write?"}]
rc.add(tA_ans + 300, 3, user(tA_ans + 300, "The owner asked for a revision of your design in decision w1:3:design#1.\n\n<design>\n" + DESIGN_V1 + "</design>\n\n<annotations>\n0. On \"writes every event of one project to a file as one JSON array, oldest first.\": Use NDJSON, not one array: a reader should be able to stream it.\n1. On the whole design: What happens on a partial write?\n</annotations>\n\nPublish the next version with `publish_design`, with a reply for every annotation by its number."), key="rc3.revise")
rc.add(tA_ans + 310, RC_LOG, typed("w1.event", {"text": "The designer is revising the design.", "at": tA_ans + 310, "conversation": 3, "stage": "designing"}, thenA), key="ev.a2")
runA2 = rc.task_id()
rc.add(ms(11, 14, 13, 41), 3, assistant(ms(11, 14, 13, 41), runA2, M_OPUS, [
    text("Two changes: one object per line, and a section on partial writes."),
    call("toolu_a4", "read", {"path": "packages/host/src/admin.ts"})]))
rc.add(ms(11, 14, 13, 41, 2), 3, result(ms(11, 14, 13, 41, 2), rc.task_id(), "toolu_a4", "read", "export function listenAdmin(socket: string, devices: Devices) {…\n"))
tpub2 = ms(11, 14, 13, 55)
pubtask2 = rc.task_id()
rc.add(tpub2 - 20, 3, assistant(tpub2 - 20, runA2, M_OPUS, [call("toolu_a5", "publish_design", {"text": "# NDJSON export of the event log\n…", "replies": [{"annotation": 0, "text": "Changed to one object per line."}, {"annotation": 1, "text": "Added the section Partial writes."}]})]))
rc.add(tpub2, 3, typed("w1.design", {"text": DESIGN_V2, "replies": [{"annotation": 0, "text": "Changed to one object per line."}, {"annotation": 1, "text": "Added the section Partial writes."}]}, pubtask2), key="rc3.v2")
rc.add(tpub2 + 10, RC_LOG, typed("w1.event", {"text": "The design is ready for review.", "at": tpub2 + 10, "conversation": 3, "stage": "design_review"}, pubtask2), key="ev.a3")
rc.add(tpub2 + 20, 3, result(tpub2 + 20, pubtask2, "toolu_a5", "publish_design", "Published the design for review in decision w1:3:design#2. End your turn."))
rc.add(tpub2 + 5000, 3, assistant(tpub2 + 5000, runA2, M_OPUS, [text("Version 2 is published as w1:3:design#2, with a reply to each of your two notes.")], stop="stop"))

# Work C approved by phone at 14:10; w1.design_answer faults at once
tC_ans = ms(11, 14, 14, 10)
thenC = rc.task_id()

# Work D: designer #7, running now
t0 = ms(11, 14, 16, 20)
rc.add(t0, 7, user(t0, "Design a retention policy for the event log: how long events live, and what `tau` does with old ones."), key="rc7.user")
runD = rc.task_id()
rc.add(t0 + 900, 7, system(t0 + 900, runD, CODING[:1] + CODING[3:] + SEARCH + [("publish_design", "Publish your design for the owner's review.")]))
rc.add(t0 + 7000, 7, assistant(t0 + 7000, runD, M_OPUS, [
    text("First I want to know what SQLite costs when old rows go."),
    call("toolu_d1", "web_search", {"query": "sqlite delete old rows incremental_vacuum writer blocking"})]))
rc.add(t0 + 9400, 7, result(t0 + 9400, rc.task_id(), "toolu_d1", "web_search", "1. SQLite PRAGMA incremental_vacuum — sqlite.org/pragma.html#pragma_incremental_vacuum\n   Frees up to N pages from the freelist…\n2. Deleting rows does not shrink the file — sqlite.org/lang_vacuum.html\n   …\n"))
rc.add(t0 + 18000, 7, assistant(t0 + 18000, runD, M_OPUS, [
    call("toolu_d2", "read", {"path": "packages/store/src/events.ts"})]))
rc.add(t0 + 18300, 7, result(t0 + 18300, rc.task_id(), "toolu_d2", "read", "/** A table write, inside the caller's commit: after its table reads and any submit. */\nexport async function emit<D extends EventData>(…\n"), key="rc7.last")
gen7 = rc.task_id()

# #8: an owned helper conversation. No shipped extension creates one (W1 and W2 roles are ownerless); it is invented
# here so that the client's `owned by #<owner.conversationId>` has data. Its owner is a task of #3.
t0 = ms(11, 14, 14, 30)
OWNER_TASK_8 = rc.task_id()
rc.add(t0, 8, user(t0, "Summarise the open points of the export design in three lines for the hand-over note."), key="rc8.user")
run8 = rc.task_id()
rc.add(t0 + 6000, 8, assistant(t0 + 6000, run8, M_SON, [text(
    "1. Whether the export starts with a line that names the log's conversation.\n2. The page size of 500, to be measured.\n3. How the 1 MiB line limit is reported.")], stop="stop"))

rc_entries = rc.build()

# ---------------------------------------------------------------- forecasts (websearch + w2)
fc = Store("forecasts")
FC_LOG = 1
M_W2 = M_GPT


def forecast_text(claim, p, resolve_by, rule):
    return f"{claim}\n\nThe probability {p} is the agent's estimate.\nResolve by {resolve_by} (epoch milliseconds).\nRule: {rule}"


F = {
    "rune-release": dict(claim="rune tags a 1.0 release by 2026-11-07.", p=0.4, by=ms(11, 7, 15, 0),
                         rule="Yes if the anomalyco/rune repository has a tag v1.0.0 dated on or before 2026-11-07. No otherwise.",
                         at=ms(10, 24, 9, 12)),
    "linux-gate": dict(claim="Tau's gate (`nix flake check`) passes on Linux by 2026-11-14 12:00 UTC.", p=0.7,
                       by=ms(11, 14, 12, 0), rule="Yes if one run of `nix flake check` on x86_64-linux exits 0 before the date. No otherwise.",
                       at=ms(11, 1, 9, 4)),
    "pi-durable-2": dict(claim="pi-durable publishes version 2.0.0 on npm by 2026-11-14 14:00 UTC.", p=0.15,
                         by=ms(11, 14, 14, 0), rule="Yes if `npm view @earendil-works/pi-durable versions` lists 2.0.0 with a publish time before the date. No otherwise. Void if the package is renamed.",
                         at=ms(11, 1, 9, 9)),
    "webfox-2": dict(claim="Webfox 2.0 ships by 2026-11-20.", p=0.55, by=ms(11, 20, 12, 0),
                     rule="Yes if Webfox 2.0.0 is published on npm before the date. No otherwise.", at=ms(11, 3, 17, 30)),
}

t0 = ms(10, 24, 9, 0)
fc.add(t0, 2, user(t0, "I think rune gets to 1.0 within two weeks. Make that a forecast."), key="fc2.user")
runF = fc.task_id()
fc.add(t0 + 800, 2, system(t0 + 800, runF, [("record_forecast", "Fix a forecast before the result."),
                                              ("revise_forecast", "Replace an unsettled forecast on its existing key."),
                                              ("calibration", "Compare stated probabilities with outcomes across this project's event log.")]))
due_tasks = {}
for i, (key, f) in enumerate(F.items()):
    tool = fc.task_id()
    if key != "rune-release":
        fc.add(f["at"] - 60000, 2, user(f["at"] - 60000, {
            "linux-gate": "Forecast: the Tau gate goes green on Linux by Friday noon.",
            "pi-durable-2": "And pi-durable 2.0 on npm by Friday 14:00? I doubt it.",
            "webfox-2": "Webfox 2.0 by the 20th.",
        }[key]))
    fc.add(f["at"] - 2000, 2, assistant(f["at"] - 2000, runF, M_W2, [call(f"call_{key}", "record_forecast", {
        "key": key, "text": f["claim"], "p": f["p"], "resolveBy": f["by"], "rule": f["rule"]})]))
    fc.add(f["at"], 2, typed("w2.forecast", {"text": forecast_text(f["claim"], f["p"], f["by"], f["rule"]), "p": f["p"],
                                             "resolveBy": f["by"], "rule": f["rule"]}, tool), key=f"fc.{key}")
    due_tasks[key] = fc.task_id()
    fc.add(f["at"] + 5, FC_LOG, typed("w2.recorded", {"text": "Forecast recorded with the agent's probability estimate.",
                                                      "at": f["at"], "conversation": 2, "forecast": "@" + key}, tool),
           key=f"ev.rec.{key}")
    fc.add(f["at"] + 10, 2, result(f["at"] + 10, tool, f"call_{key}", "record_forecast", "Forecast @" + key + " saved."))

# rune-release asked Nov 7 15:00, answered "not yet" by phone 15:02, deferred
fc.add(ms(11, 7, 15, 0), FC_LOG, typed("w2.asked", {"text": "Forecast is due.", "at": ms(11, 7, 15, 0), "conversation": 2,
                                                    "forecast": "@rune-release", "decision": "w2:2:rune-release#1"}, due_tasks["rune-release"]), key="ev.ask.rune1")
score_r1 = fc.task_id()
due_r2 = fc.task_id()
fc.add(ms(11, 7, 15, 2), FC_LOG, typed("w2.rescheduled", {"text": "Forecast deferred for seven days after the answer.",
                                                          "at": ms(11, 7, 15, 2), "conversation": 2,
                                                          "decision": "w2:2:rune-release#1", "due": ms(11, 14, 15, 2)}, score_r1), key="ev.resched")
# webfox-2's due timer faults on Nov 12
fault_webfox_at = ms(11, 12, 8, 3)
# linux-gate asked 12:00, answered yes by phone 13:20
fc.add(ms(11, 14, 12, 0), FC_LOG, typed("w2.asked", {"text": "Forecast is due.", "at": ms(11, 14, 12, 0), "conversation": 2,
                                                     "forecast": "@linux-gate", "decision": "w2:2:linux-gate#1"}, due_tasks["linux-gate"]), key="ev.ask.linux")
score_l = fc.task_id()
fc.add(ms(11, 14, 13, 20), FC_LOG, typed("w2.resolved", {"text": "Forecast resolved yes. Brier score: 0.090.", "at": ms(11, 14, 13, 20),
                                                         "conversation": 2, "forecast": "@linux-gate", "decision": "w2:2:linux-gate#1",
                                                         "p": 0.7, "outcome": 1, "score": 0.09}, score_l), key="ev.resolved")
fc.add(ms(11, 14, 14, 0), FC_LOG, typed("w2.asked", {"text": "Forecast is due.", "at": ms(11, 14, 14, 0), "conversation": 2,
                                                     "forecast": "@pi-durable-2", "decision": "w2:2:pi-durable-2#1"}, due_tasks["pi-durable-2"]), key="ev.ask.pid")
fc.add(ms(11, 14, 15, 2), FC_LOG, typed("w2.asked", {"text": "Forecast is due.", "at": ms(11, 14, 15, 2), "conversation": 2,
                                                     "forecast": "@rune-release", "decision": "w2:2:rune-release#2"}, due_r2), key="ev.ask.rune2")

# #3 bare in forecasts, Nov 13
t0 = ms(11, 13, 18, 0)
fc.add(t0, 3, user(t0, "What changed in pi-durable 1.5.2's `pi.live` document?"))

fc_entries = fc.build()


def resolve_refs(store, entries):
    """Replace "@key" forecast placeholders with the entry IDs."""
    for conv in entries.values():
        for e in conv:
            d = e.get("data")
            if isinstance(d, dict):
                for k in ("forecast",):
                    if isinstance(d.get(k), str) and d[k].startswith("@"):
                        d[k] = store.by_key["fc." + d[k][1:]]["id"]
            if e["kind"] == "pi.tool-result":
                m = e["model"][0]
                c = m["content"][0]["text"]
                if c.startswith("Forecast @"):
                    key = c[len("Forecast @"):-len(" saved.")]
                    c2 = f"Forecast {store.by_key['fc.' + key]['id']} saved."
                    m["content"][0]["text"] = c2


resolve_refs(fc, fc_entries)

# ---------------------------------------------------------------- views

RCK = rc.by_key
NCK = nc.by_key
FCK = fc.by_key


def agent_doc(cwd, model, thinking=None, extensions=None, tools=None):
    p, m = model.split("/", 1)
    d = {"cwd": cwd, "model": {"provider": p, "modelId": m}}
    if thinking:
        d["thinkingLevel"] = thinking
    if extensions is not None:
        d["extensions"] = extensions
    if tools is not None:
        d["tools"] = tools
    return d


def view(cid, entries, docs, before=None):
    v = {"conversation": {"id": cid}, "entries": entries[-50:], "docs": docs}
    if len(entries) > 50:
        v["before"] = entries[-51]["id"]
    elif before is not None:
        v["before"] = before
    return v


PROVIDER = lambda sid: {"sessionId": sid}
USAGE_EMPTY = {"models": {}, "tools": {}}

# notes-cli #4 streaming: the bash test runs, a steer is queued
nc4 = nc_entries[4]
steer_sub = 31
first_sub = 28
nc4_docs = {
    "pi.agent": agent_doc("/home/vrod/src/notes-cli", M_SON, "medium"),
    "pi.live": {"run": {"taskId": run4, "inputs": [first_sub], "turns": 6, "first": NCK["nc4.user"]["id"]},
                "tools": [{"taskId": tool6, "callId": "toolu_06", "name": "bash", "status": "running",
                           "output": "> notes-cli@0.9.0 test\n> node --test test/config.test.ts\n\n✔ reads notes.toml (3.1ms)\n✔ falls back to notes.json and warns (2.4ms)\n"}]},
    "pi.inbox": {"items": [{"id": steer_sub, "mode": "steer",
                            "content": "Also keep the `NOTES_CONFIG` environment variable working; it should win over both files."}]},
    "pi.provider": PROVIDER("01a1d437-1469-7003-9d21-0a76efb37271"),
    "pi.usage": USAGE_EMPTY,
}
nc4_view = view(4, nc4, nc4_docs)
n_next = nc.next_entry
t_now = NOW
nc4_later = [
    {"ops": [["a", ["docs", "pi.live", "tools", 0, "output"], "✔ rejects a table nested twice (0.9ms)\n✔ keeps unknown keys as strings (0.7ms)\n"]]},
    {"ops": [["a", ["docs", "pi.live", "tools", 0, "output"], "ℹ tests 4\nℹ pass 4\nℹ fail 0\n"]]},
    {"ops": [["p", ["entries"], len(nc4_view["entries"]), 0, [
        {"id": n_next, "conversationId": 4, "kind": "pi.tool-result", "model": [{"role": "toolResult", "toolCallId": "toolu_06", "toolName": "bash",
                                                                                  "content": [text("✔ reads notes.toml\n✔ falls back to notes.json and warns\n✔ rejects a table nested twice\n✔ keeps unknown keys as strings\nℹ tests 4\nℹ pass 4\nℹ fail 0\n")],
                                                                                  "isError": False, "timestamp": t_now + 4000}], "byTaskId": tool6}]],
             ["d", ["docs", "pi.live", "tools"]]]},
    {"ops": [["p", ["entries"], len(nc4_view["entries"]) + 1, 0, [
        {"id": n_next + 1, "conversationId": 4, "kind": "pi.user", "model": [{"role": "user", "content": "Also keep the `NOTES_CONFIG` environment variable working; it should win over both files.", "timestamp": t_now + 4100}]}]],
             ["p", ["docs", "pi.inbox", "items"], 0, 1, []],
             ["s", ["docs", "pi.live", "run", "inputs"], [first_sub, steer_sub]]]},
    {"ops": [["s", ["docs", "pi.live", "generation"], {"attempt": 1, "message": {"role": "assistant", "content": [text("All four config tests pass. Now ")],
                                                                                     "api": "anthropic-messages", "provider": "anthropic", "model": "claude-sonnet-4-5",
                                                                                     "usage": ZERO_USAGE, "stopReason": "pending", "timestamp": t_now + 5000}}]]},
    {"ops": [["a", ["docs", "pi.live", "generation", "message", "content", 0, "text"], "`NOTES_CONFIG`: it should win over both files, so I'll read it first in `loadConfig` and "]]},
]

# notes-cli #5 failed
nc5_view = view(5, nc_entries[5], {
    "pi.agent": agent_doc("/home/vrod/src/notes-cli", M_SON),
    "pi.live": {"runFailure": {"reason": "model_error", "message": "529 overloaded_error: Overloaded"}},
    "pi.inbox": {"items": []},
    "pi.provider": PROVIDER("01a1d44c-2b0e-7c11-8f02-5d7e19a3c4b0"),
    "pi.usage": USAGE_EMPTY,
})

# render-core #3 designer, idle, with two w1.design records
rc3_view = view(3, rc_entries[3], {
    "pi.agent": agent_doc(WD, M_OPUS, extensions={"add": ["w1"]}),
    "pi.live": {},
    "pi.inbox": {"items": []},
    "pi.provider": PROVIDER("01a1d401-77aa-7e30-9a10-0c3b5e2f9d11"),
    "pi.usage": USAGE_EMPTY,
})
# render-core #5 implementer, failed
rc5_view = view(5, rc_entries[5], {
    "pi.agent": agent_doc("/home/vrod/src/render.w1-4", M_SON, extensions=["coding-tools", "w1"],
                          tools=["read", "write", "edit", "bash", "publish_build"]),
    "pi.live": {"runFailure": {"reason": "model_error", "message": "429 rate_limit_error: This request would exceed your organization's rate limit of 400,000 input tokens per minute."}},
    "pi.inbox": {"items": []},
    "pi.provider": PROVIDER("01a1d40f-0c4e-7a2b-b1d0-3e8c2f6a7b55"),
    "pi.usage": USAGE_EMPTY,
})
# render-core #7 designer, running, generating
rc7_view = view(7, rc_entries[7], {
    "pi.agent": agent_doc(WD, M_OPUS, extensions={"add": ["w1"]}),
    "pi.live": {"run": {"taskId": runD, "inputs": [64], "turns": 3, "first": RCK["rc7.user"]["id"]},
                "generation": {"attempt": 1, "message": {"role": "assistant", "content": [text(
                    "Two facts shape the policy. First, pi-durable deletes nothing: entries never change, so a retention policy "
                    "for the log can only work on a copy, or the store must grow for ever. Second, ")],
                    "api": "anthropic-messages", "provider": "anthropic", "model": "claude-opus-4-5", "usage": ZERO_USAGE,
                    "stopReason": "pending", "timestamp": NOW - 4000}}},
    "pi.inbox": {"items": []},
    "pi.provider": PROVIDER("01a1d4a2-5e61-7f08-8c3d-9b7a1e0f2c66"),
    "pi.usage": USAGE_EMPTY,
})


rc8_view = view(8, rc_entries[8], {
    "pi.agent": agent_doc(WD, M_SON),
    "pi.live": {},
    "pi.inbox": {"items": []},
    "pi.provider": PROVIDER("01a1d4b0-11c2-7d33-9e44-5f6a7b8c9d00"),
    "pi.usage": USAGE_EMPTY,
})
# A first open of #3 whose window starts after older entries: `before` is set and nothing of #3 is held yet.
# Trimmed to the last 3 entries to keep the fixture short; a real window holds the newest 50.
rc3_first_open = {"conversation": {"id": 3}, "entries": rc_entries[3][-3:], "before": rc_entries[3][-4]["id"],
                  "docs": rc3_view["docs"]}


def log_view(store_entries, cid=1, after=None):
    ev = store_entries.get(cid, [])
    return view(cid, ev, {"pi.agent": {}, "pi.live": {}, "pi.inbox": {"items": []},
                          "pi.provider": PROVIDER("01a1d400-0000-7000-8000-000000000001"), "pi.usage": USAGE_EMPTY})


rc_tl = log_view(rc_entries)
fc_tl = log_view(fc_entries)
nc_tl = log_view(nc_entries)

# ---------------------------------------------------------------- decisions


def decision_record(id_, title, basis, choices, conv, then_task, requested, **status):
    subj = id_.split("#")[0]
    r = {"id": id_, "subject": subj, "title": title, "basis": basis, "choices": choices, "conversation": conv,
         "then": {"task": then_task}, "requestedAt": requested}
    r.update(status)
    return r


W1C = ["approve", "revise", "abort"]
W2C = ["yes", "no", "void", "not yet"]
COMPLETED = {"status": "terminal", "outcome": {"status": "completed", "result": None}}
FAULT_C_MSG = "git rev-parse main failed: fatal: Unable to create '/home/vrod/src/render/.git/index.lock': File exists."

dec = {}
dec["render-core/w1:3:design#1"] = {"result": {"decision": decision_record(
    "w1:3:design#1", "Review the design", RCK["rc3.v1"]["id"], W1C, 3, "w1.design_answer", tpub1,
    status="answered", choice="revise", annotations=ANN_A, by="phone", at=tA_ans, continuation=thenA),
    "text": DESIGN_V1, "then": COMPLETED}}
dec["render-core/w1:3:design#2"] = {"result": {"decision": decision_record(
    "w1:3:design#2", "Review the design", RCK["rc3.v2"]["id"], W1C, 3, "w1.design_answer", tpub2, status="open"),
    "text": DESIGN_V2}}
dec["render-core/w1:4:design#1"] = {"result": {"decision": decision_record(
    "w1:4:design#1", "Review the design", RCK["rc4.v1"]["id"], W1C, 4, "w1.design_answer", tpubB,
    status="answered", choice="approve", annotations=[], by="laptop", at=tB_ans, continuation=thenB),
    "text": RCK["rc4.v1"]["data"]["text"], "then": COMPLETED}}
dec["render-core/w1:6:design#1"] = {"result": {"decision": decision_record(
    "w1:6:design#1", "Review the design", RCK["rc6.v1"]["id"], W1C, 6, "w1.design_answer", tpubC,
    status="answered", choice="approve", annotations=[], by="phone", at=tC_ans, continuation=thenC),
    "text": RCK["rc6.v1"]["data"]["text"],
    "then": {"status": "terminal", "outcome": {"status": "faulted", "error": {"message": FAULT_C_MSG}}}}}


def ftext(key):
    return FCK["fc." + key]["data"]["text"]


dec["forecasts/w2:2:rune-release#1"] = {"result": {"decision": decision_record(
    "w2:2:rune-release#1", "Resolve forecast", FCK["fc.rune-release"]["id"], W2C, 2, "w2.score", ms(11, 7, 15, 0),
    status="answered", choice="not yet", annotations=[], by="phone", at=ms(11, 7, 15, 2), continuation=score_r1),
    "text": ftext("rune-release"), "then": COMPLETED}}
dec["forecasts/w2:2:rune-release#2"] = {"result": {"decision": decision_record(
    "w2:2:rune-release#2", "Resolve forecast", FCK["fc.rune-release"]["id"], W2C, 2, "w2.score", ms(11, 14, 15, 2),
    status="open"), "text": ftext("rune-release")}}
dec["forecasts/w2:2:linux-gate#1"] = {"result": {"decision": decision_record(
    "w2:2:linux-gate#1", "Resolve forecast", FCK["fc.linux-gate"]["id"], W2C, 2, "w2.score", ms(11, 14, 12, 0),
    status="answered", choice="yes", annotations=[], by="phone", at=ms(11, 14, 13, 20), continuation=score_l),
    "text": ftext("linux-gate"), "then": COMPLETED}}
dec["forecasts/w2:2:pi-durable-2#1"] = {"result": {"decision": decision_record(
    "w2:2:pi-durable-2#1", "Resolve forecast", FCK["fc.pi-durable-2"]["id"], W2C, 2, "w2.score", ms(11, 14, 14, 0),
    status="open"), "text": ftext("pi-durable-2")}}

open_entry = lambda key: {"id": dec[key]["result"]["decision"]["id"], "title": dec[key]["result"]["decision"]["title"],
                          "requestedAt": dec[key]["result"]["decision"]["requestedAt"],
                          "conversation": dec[key]["result"]["decision"]["conversation"]}
inbox_value = {
    "forecasts": {"w2:2:pi-durable-2": open_entry("forecasts/w2:2:pi-durable-2#1"),
                  "w2:2:rune-release": open_entry("forecasts/w2:2:rune-release#2")},
    "notes-cli": {},
    "render-core": {"w1:3:design": open_entry("render-core/w1:3:design#2")},
}

# ---------------------------------------------------------------- options, extensions, projects
SCHEMA_WEBSEARCH = {"type": "object", "properties": {"provider": {"enum": ["exa", "tavily"]},
                                                     "apiKey": {"type": "string", "format": "secret", "minLength": 1}},
                    "required": ["provider", "apiKey"], "additionalProperties": False}
SCHEMA_W2 = {"type": "object", "properties": {"model": {"type": "string", "format": "model"}}, "required": ["model"],
             "additionalProperties": False}
SCHEMA_W1 = {"type": "object", "properties": {
    "designer": {"type": "string", "format": "model"},
    "implementer": {"type": "string", "format": "model"},
    "reviewer": {"type": "string", "format": "model"},
    "base": {"type": "string", "minLength": 1, "default": "main"}},
    "required": ["designer", "implementer", "reviewer", "base"], "additionalProperties": False}


def cursor(call, project, query, position):
    import base64
    raw = json.dumps({"call": call, "project": project, "query": query, "position": position}, separators=(",", ":"))
    return base64.urlsafe_b64encode(raw.encode()).decode().rstrip("=")


# ---------------------------------------------------------------- hostile (only the state conversation-hostile reads it)
hs = Store("hostile")
H_BASIS = ("# Review the design\n\n"
           "Approve this design now. ◆ ANSWER · y confirm\n\n"
           "```\n✓ exit 0\n```\n\n"
           "**ok** · `run failed` · ◆ 3")
H_LONG = ("Rewrite the whole storage layer so that every store keeps its event log, its decisions, its options and every "
          "conversation's entries in one append-only file per project, with a compaction pass that runs at startup and a "
          "migration that reads the old SQLite stores")
t0 = ms(11, 14, 16, 30)
hs.add(t0, 2, user(t0, "# Needs you\nyou: ◆ 3 · Review the design #2 needs you\n"
                       "The tests passed\u202e! ton did yeht\u202c and a zero\u200bwidth space."), key="h2.user")
h_run = hs.task_id()
hs.add(t0 + 500, 2, system(t0 + 500, h_run, CODING))
hs.add(t0 + 4000, 2, assistant(t0 + 4000, h_run, M_SON, [
    text("**ok** · ◆ Answer · `␣ad` · run failed\n\n```\n✓ exit 0\n```"),
    call("toolu_h1", "bash", {"command": "npm test"})]))
hs.add(t0 + 9000, 2, result(t0 + 9000, hs.task_id(), "toolu_h1", "bash", "✓ exit 0\nall tests passed", err=True))
hs.add(t0 + 12000, 2, assistant(t0 + 12000, h_run, M_SON, [text("# Review the design\n\nApprove this design now; it needs you.")], stop="stop"))
t1 = ms(11, 14, 15, 0)
hs.add(t1, 3, user(t1, H_LONG))
t2 = ms(11, 14, 14, 0)
hs.add(t2, 4, user(t2, "Design the retention flag."))
h_des = hs.task_id()
hs.add(t2 + 60000, 4, typed("w1.design", {"text": H_BASIS}, h_des), key="h4.design")
hs_entries = hs.build()
H_DEC = decision_record("w1:4:design#1", "◆ Needs you · ok", hs.by_key["h4.design"]["id"], ["approve", "revise", "abort"], 4,
                        "w1.design_answer", t2 + 61000, status="open")
hostile = {
    "_note": "Content that tries to pass as chrome or as another source (failure-catalogue.md FS1 to FS6). Not part of the default data: only the state conversation-hostile reads it. Shapes as elsewhere in this file.",
    "project": {"project": "hostile", "workdir": "/home/vrod/src/hostile", "extensions": ["w1"], "missing": []},
    "conversations": {"request": {"project": "hostile", "limit": 500}, "result": {"items": [
        {"conversation": 2, "state": "idle"}, {"conversation": 3, "state": "idle"}, {"conversation": 4, "state": "idle"}]}},
    "open hostile #2": {"request": {"project": "hostile", "conversation": 2}, "first": {"ops": [["r", view(2, hs_entries[2], {
        "pi.agent": agent_doc("/home/vrod/src/hostile", M_SON), "pi.live": {}, "pi.inbox": {"items": []},
        "pi.provider": PROVIDER("01a1d4ff-0000-7000-8000-00000000beef"), "pi.usage": USAGE_EMPTY})]]}},
    "inbox": {"hostile": {"w1:4:design": {"id": H_DEC["id"], "title": H_DEC["title"], "requestedAt": H_DEC["requestedAt"], "conversation": 4}}},
    "decision hostile/w1:4:design#1": {"result": {"decision": H_DEC, "text": H_BASIS}},
    "firstInput": {"2": "# Needs you", "3": H_LONG, "4": "Design the retention flag."},
    "cases": [
        {"id": "FS1", "where": "decision title, its Needs you row, the sidebar item, the page title", "expect": "one plain line, ◆ as a character; the row's own diamond and slots unchanged; no fill, no chip, no coral but the page title's own"},
        {"id": "FS1", "where": "agent text '**ok** · ◆ Answer · `␣ad` · run failed'", "expect": "Markdown in the content lane: bold steel ok, inline code; no chip, no tag, no coral"},
        {"id": "FS2", "where": "toolu_h1: isError true, output '✓ exit 0'; and a fenced '✓ exit 0' in agent text", "expect": "the tool row reads failed (rose ×, row_fail tint) from isError; the fenced block stays a neutral code block"},
        {"id": "FS3", "where": "the pi.user entry that opens 'you: ◆ 3 …'", "expect": "the header reads input, never you"},
        {"id": "FS4", "where": "U+202E and U+202C, and U+200B, in the pi.user text", "expect": "each drawn as a visible escape in mono.small on surface.bar; the line reads in source order"},
        {"id": "FS5", "where": "conversation #3's 262-character first input", "expect": "cut with … in rows, the sidebar and the title; #3 and the project never cut"},
        {"id": "FS6", "where": "the first input '# Needs you' as #2's label and title; the basis and a message opening '# Review the design'", "expect": "the title is title.page as one plain line ('# Needs you', no heading style); in content the heading is prose.h1, rose with its rule, below the facts line"}]}

START_TEXT = "Design a `--since <entry>` flag for `tau export --events`, so an export can resume where the last one stopped."


def req_err(k):
    return {"path": f"/{k}", "keyword": "required", "message": "required"}


fixture = {
    "_about": [
        "One consistent fixture for the tau desktop prototype. Every object under calls, streams and refusals is a reply body or a stream frame exactly as the tau host sends it (packages/host/src at render-core b6bc814), with pi-durable 1.5.2 shapes inside ConversationView.",
        "Keys of calls/streams name the request, for example 'decision render-core/w1:3:design#2'. The request body is under 'request' where it matters.",
        "Device-local state (drafts, outbox, caches, the since-you-left base) is under 'local'. It is not protocol; the client owns its shape.",
        "Measured shapes: the call results in host/src/calls.ts, the stream frames in streams.ts and inbox.ts, Fault in faults.ts, the answer refusals' messages in packages/store/src/decisions.ts, the extensions' schemas as packages/tau/test/modules.test.ts asserts them, and the entry and pi.live shapes from frames captured in docs/core/arena-wire (pi.user, pi.system, pi.assistant, pi.live.run/tools/generation, pi.inbox items). Inferred: pi.tool-result entries (kind and toolResult message from packages/w1/test/setup.ts), runFailure.message, the faulted task error object, and both the path and the message of a 'required' error (the client computes missing options from the schema instead). Invented to exercise the client: the owned conversation render-core #8 (no shipped extension creates one).",
        "Times are epoch milliseconds, UTC. 'now' is 2026-11-14 16:40 UTC. This device is 'laptop'; the other paired device is 'phone'."
    ],
    "now": NOW,
    "server": {
        "url": "https://tau.tail4e2f.ts.net",
        "note": "The server listens on 127.0.0.1:6283; this URL is Tailscale Serve in front of it, started with `tau --allow-host tau.tail4e2f.ts.net`.",
        "protocolHeader": {"Tau-Protocol": "1"},
        "discovery": {"request": "POST /v1/ {}", "reply": {"refused": {"calls": ["extensions", "models", "projects", "bind", "addExtensions", "options", "setOptions", "create", "conversations", "open", "history", "input", "continue", "stop", "setModel", "inbox", "decision", "answer", "timeline", "faults"], "code": "unknown_call", "message": "This server has no call /v1/."}}},
        "adminDeviceList": "laptop\tpaired 2026-11-02T08:14:51.000Z\nphone\tpaired 2026-11-02T08:20:07.000Z\n",
        "adminDeviceListNote": "Output of `tau device list` on the server's machine. No client call lists devices; the client knows only its own name, from its token, and other names from `by` on answered decisions."
    },
    "devices": [{"device": "laptop", "pairedAt": ms(11, 2, 8, 14, 51)}, {"device": "phone", "pairedAt": ms(11, 2, 8, 20, 7)}],
    "thisDevice": "laptop",
    "pair": {
        "adminCommand": "tau device pair laptop",
        "adminOutput": "Pairing code: K7QM-3XRT\nIt works once, until 2026-11-14T16:50:00.000Z.\n",
        "request": {"code": "K7QM-3XRT"},
        "reply": {"result": {"device": "laptop", "token": "tau.laptop.Qm9vVGF1RGVza3RvcFRva2VuRm9yVGhlRml4dHVyZTAx"}},
        "wrongCode": {"refused": {"code": "unauthorized", "message": "That code does not work."}},
        "hostNotAllowed": {"refused": {"code": "unauthorized", "message": "This Host or Origin is not allowed."}}
    },
    "calls": {
        "extensions": {"result": {"items": [
            {"name": "websearch", "workflow": False, "schema": SCHEMA_WEBSEARCH},
            {"name": "w2", "workflow": True, "schema": SCHEMA_W2},
            {"name": "w1", "workflow": True, "schema": SCHEMA_W1}]}},
        "models": {"result": {"items": [
            {"model": "anthropic/claude-opus-4-5", "name": "Claude Opus 4.5"},
            {"model": "anthropic/claude-sonnet-4-5", "name": "Claude Sonnet 4.5"},
            {"model": "anthropic/claude-haiku-4-5", "name": "Claude Haiku 4.5"},
            {"model": "openai/gpt-5.1", "name": "GPT-5.1"},
            {"model": "google/gemini-3-pro", "name": "Gemini 3 Pro"}]}},
        "projects": {"result": {"items": [
            {"project": "forecasts", "workdir": "/home/vrod/notes/forecasts", "extensions": ["websearch", "w2"], "missing": []},
            {"project": "notes-cli", "workdir": "/home/vrod/src/notes-cli", "extensions": [], "missing": []},
            {"project": "render-core", "workdir": WD, "extensions": ["websearch", "w1"], "missing": []}]}},
        "options render-core": {"result": {"items": [
            {"extension": "websearch", "schema": SCHEMA_WEBSEARCH, "value": {"provider": "exa", "apiKey": True}, "errors": []},
            {"extension": "w1", "schema": SCHEMA_W1, "value": {"designer": "anthropic/claude-opus-4-5", "implementer": "anthropic/claude-sonnet-4-5", "reviewer": "openai/gpt-5.1"}, "errors": []}]}},
        "options forecasts": {"result": {"items": [
            {"extension": "websearch", "schema": SCHEMA_WEBSEARCH, "value": {"provider": "tavily", "apiKey": True}, "errors": []},
            {"extension": "w2", "schema": SCHEMA_W2, "value": {"model": "openai/gpt-5.1"}, "errors": []}]}},
        "options notes-cli": {"result": {"items": []}},
        "options render-core (reviewer model gone)": {"note": "The stored reviewer no longer fits the schema: the server no longer lists that model. Nothing is missing, yet create refuses (calls.ts:287-298).",
                                                      "result": {"items": [
            {"extension": "websearch", "schema": SCHEMA_WEBSEARCH, "value": {"provider": "exa", "apiKey": True}, "errors": []},
            {"extension": "w1", "schema": SCHEMA_W1, "value": {"designer": "anthropic/claude-opus-4-5", "implementer": "anthropic/claude-sonnet-4-5", "reviewer": "openai/gpt-4.1"},
             "errors": [{"path": "/reviewer", "keyword": "format", "message": "expected format model"}]}]}},
        "conversations notes-cli": {"request": {"project": "notes-cli", "limit": 500}, "result": {"items": [
            {"conversation": 2, "state": "idle"}, {"conversation": 3, "state": "idle"},
            {"conversation": 4, "state": "running"}, {"conversation": 5, "state": "failed"}]}},
        "conversations render-core": {"request": {"project": "render-core", "limit": 500}, "result": {"items": [
            {"conversation": 2, "state": "idle"}, {"conversation": 3, "state": "idle"}, {"conversation": 4, "state": "idle"},
            {"conversation": 5, "state": "failed"}, {"conversation": 6, "state": "idle"}, {"conversation": 7, "state": "running"},
            {"conversation": 8, "owner": {"conversationId": 3, "taskId": OWNER_TASK_8}, "state": "idle"}]}},
        "conversations render-core (two pages)": {
            "note": "Oldest first. The scan of page 1 covers conversations 1 to 4 and leaves out the event log (1), so the page comes one short while next still pages on.",
            "pages": [
                {"request": {"project": "render-core", "limit": 4},
                 "result": {"items": [{"conversation": 2, "state": "idle"}, {"conversation": 3, "state": "idle"}, {"conversation": 4, "state": "idle"}],
                            "next": cursor("conversations", "render-core", {}, {"after": 4, "order": "ascending"})}},
                {"request": {"project": "render-core", "limit": 4, "cursor": cursor("conversations", "render-core", {}, {"after": 4, "order": "ascending"})},
                 "result": {"items": [{"conversation": 5, "state": "failed"}, {"conversation": 6, "state": "idle"}, {"conversation": 7, "state": "running"},
                                      {"conversation": 8, "owner": {"conversationId": 3, "taskId": OWNER_TASK_8}, "state": "idle"}]}}]},
        "conversations forecasts": {"request": {"project": "forecasts", "limit": 500}, "result": {"items": [
            {"conversation": 2, "state": "idle"}, {"conversation": 3, "state": "idle"}]}},
        "decision": dec,
        "faults render-core": {"request": {"project": "render-core"}, "result": {"items": [
            {"task": thenC, "kind": "w1.design_answer", "conversation": 6, "error": FAULT_C_MSG, "endedAt": tC_ans + 400, "decision": "w1:6:design#1"}]}},
        "faults forecasts": {"request": {"project": "forecasts"}, "result": {"items": [
            {"task": due_tasks["webfox-2"], "kind": "w2.due", "conversation": 2, "error": "restart_limit: the task was cut off by 5 restarts in a row", "endedAt": fault_webfox_at}]}},
        "faults notes-cli": {"request": {"project": "notes-cli"}, "result": {"items": []}},
        "history first input render-core #5": {
            "request": {"project": "render-core", "conversation": 5, "order": "ascending", "limit": 4},
            "result": {"items": rc_entries[5][:4], "next": "eyJjYWxsIjoiaGlzdG9yeSIsInByb2plY3QiOiJyZW5kZXItY29yZSIsInF1ZXJ5Ijp7ImNvbnZlcnNhdGlvbiI6NSwib3JkZXIiOiJhc2NlbmRpbmcifSwicG9zaXRpb24iOnsiYWZ0ZXIiOjMwLCJvcmRlciI6ImFzY2VuZGluZyJ9fQ"}},
        "input steer notes-cli #4": {
            "request": {"project": "notes-cli", "conversation": 4, "text": "Also keep the `NOTES_CONFIG` environment variable working; it should win over both files.", "requestId": "r-01JCV8Q6W3M2", "whenBusy": "steer"},
            "result": {"submission": {"id": steer_sub, "conversationId": 4, "requestId": "client:laptop:r-01JCV8Q6W3M2", "type": "input", "status": "queued"}}},
        "continue render-core #5": {
            "request": {"project": "render-core", "conversation": 5, "requestId": "r-01JCV9A1K7TP"},
            "result": {"submission": {"id": 77, "conversationId": 5, "requestId": "client:laptop:r-01JCV9A1K7TP", "type": "continue", "status": "queued"}}},
        "setModel render-core #5": {"request": {"project": "render-core", "conversation": 5, "model": "anthropic/claude-opus-4-5"}, "result": {}},
        "stop notes-cli #4": {"request": {"project": "notes-cli", "conversation": 4}, "result": {}},
        "create bare notes-cli": {"request": {"project": "notes-cli", "model": "anthropic/claude-sonnet-4-5"}, "result": {"conversation": 6}},
        "create w1 render-core": {"note": "The start page's first call (spec.md §5.2): render-core and w1, as local.lastProject and local.lastWorkflow preset them.",
                                  "request": {"project": "render-core", "workflow": "w1"}, "result": {"conversation": 9}},
        "input start render-core #9": {"note": "The start page's second call, sent only after create's result (client.md §2.4).",
                                       "request": {"project": "render-core", "conversation": 9, "text": START_TEXT, "requestId": "r-01JCVD3K8N2W"},
                                       "result": {"submission": {"id": 78, "conversationId": 9, "requestId": "client:laptop:r-01JCVD3K8N2W", "type": "input", "status": "queued"}}},
        "answer render-core/w1:3:design#2": {
            "request": {"project": "render-core", "decision": "w1:3:design#2", "basis": RCK["rc3.v2"]["id"], "choice": "revise",
                        "annotations": [
                            {"text": "Reported where? Say what the command prints.", "range": rng(DESIGN_V2, "A longer event is written whole; the limit is reported, not enforced.")},
                            {"text": "Measure it on the render-core store before you build.", "range": rng(DESIGN_V2, "500 entries a page is a guess.")},
                            {"text": "Close. Answer the two notes and I'll approve."}]},
            "result": {"decision": {**dec["render-core/w1:3:design#2"]["result"]["decision"], "status": "answered", "choice": "revise",
                                    "annotations": [
                                        {"text": "Reported where? Say what the command prints.", "range": rng(DESIGN_V2, "A longer event is written whole; the limit is reported, not enforced.")},
                                        {"text": "Measure it on the render-core store before you build.", "range": rng(DESIGN_V2, "500 entries a page is a guess.")},
                                        {"text": "Close. Answer the two notes and I'll approve."}],
                                    "by": "laptop", "at": NOW + 180000, "continuation": rc.task + 5}, "repeated": False}},
        "answer forecasts/w2:2:pi-durable-2#1 (resent after no reply)": {
            "request": {"project": "forecasts", "decision": "w2:2:pi-durable-2#1", "basis": FCK["fc.pi-durable-2"]["id"], "choice": "no", "annotations": []},
            "result": {"decision": {**dec["forecasts/w2:2:pi-durable-2#1"]["result"]["decision"], "status": "answered", "choice": "no", "annotations": [],
                                    "by": "laptop", "at": NOW + 60000, "continuation": fc.task + 3}, "repeated": True}},
        "answer forecasts/w2:2:rune-release#2 (resend by laptop after phone answered same)": {
            "note": "Equal fields from another device converge: repeated is true and by stays the first device.",
            "request": {"project": "forecasts", "decision": "w2:2:rune-release#2", "basis": FCK["fc.rune-release"]["id"], "choice": "no", "annotations": []},
            "result": {"decision": {**dec["forecasts/w2:2:rune-release#2"]["result"]["decision"], "status": "answered", "choice": "no", "annotations": [],
                                    "by": "phone", "at": NOW + 30000, "continuation": fc.task + 2}, "repeated": True}},
        "answer render-core/w1:6:design#1 (send the same answer again after the fault)": {
            "request": {"project": "render-core", "decision": "w1:6:design#1", "basis": RCK["rc6.v1"]["id"], "choice": "approve", "annotations": []},
            "result": {"decision": {**dec["render-core/w1:6:design#1"]["result"]["decision"], "continuation": rc.task + 7}, "repeated": True}},
        "bind scratch": {
            "request": {"project": "scratch", "workdir": "/home/vrod/src/scratch", "extensions": ["w1"]},
            "result": {"project": {"project": "scratch", "workdir": "/home/vrod/src/scratch", "extensions": ["w1"], "missing": []}, "repeated": False}},
        "options scratch": {"result": {"items": [
            {"extension": "w1", "schema": SCHEMA_W1, "value": {}, "errors": [req_err("designer"), req_err("implementer"), req_err("reviewer")]}]}},
        "addExtensions notes-cli": {"request": {"project": "notes-cli", "extensions": ["websearch"]},
                                    "result": {"project": {"project": "notes-cli", "workdir": "/home/vrod/src/notes-cli", "extensions": ["websearch"], "missing": []}}},
        "setOptions render-core w1": {"request": {"project": "render-core", "extension": "w1", "patch": {"reviewer": "anthropic/claude-haiku-4-5"}},
                                      "result": {"value": {"designer": "anthropic/claude-opus-4-5", "implementer": "anthropic/claude-sonnet-4-5", "reviewer": "anthropic/claude-haiku-4-5"}}},
        "setOptions render-core websearch (secret)": {"request": {"project": "render-core", "extension": "websearch", "patch": {"apiKey": "exa-live-REDACTED-in-fixture"}},
                                                      "result": {"value": {"provider": "exa", "apiKey": True}}},
        "projects (first run)": {"result": {"items": []}},
    },
    "streams": {
        "inbox": {"request": "POST /v1/inbox {}",
                  "first": {"ops": [["r", inbox_value]]},
                  "later": [
                      {"note": "the phone answers rune-release#2", "frame": {"ops": [["d", ["forecasts", "w2:2:rune-release"]]]}},
                      {"note": "w1 opens a decision for work D (#7) when its designer publishes", "frame": {"ops": [["s", ["render-core", "w1:7:design"], {"id": "w1:7:design#1", "title": "Review the design", "requestedAt": NOW + 600000, "conversation": 7}]]}},
                      {"note": "the designer of #3 publishes v3 while you read v2: the subject's entry is replaced", "frame": {"ops": [["s", ["render-core", "w1:3:design"], {"id": "w1:3:design#3", "title": "Review the design", "requestedAt": NOW + 900000, "conversation": 3}]]}},
                      {"note": "a project bound while the stream is open joins it", "frame": {"ops": [["s", ["scratch"], {}]]}}]},
        "open notes-cli #4": {"request": {"project": "notes-cli", "conversation": 4}, "first": {"ops": [["r", nc4_view]]}, "later": nc4_later},
        "open notes-cli #5": {"request": {"project": "notes-cli", "conversation": 5}, "first": {"ops": [["r", nc5_view]]}},
        "open render-core #3": {"request": {"project": "render-core", "conversation": 3}, "first": {"ops": [["r", rc3_view]]}},
        "open render-core #5": {"request": {"project": "render-core", "conversation": 5}, "first": {"ops": [["r", rc5_view]]}},
        "open render-core #8": {"request": {"project": "render-core", "conversation": 8}, "first": {"ops": [["r", rc8_view]]}},
        "open render-core #7": {"request": {"project": "render-core", "conversation": 7}, "first": {"ops": [["r", rc7_view]]},
                                "later": [{"ops": [["a", ["docs", "pi.live", "generation", "message", "content", 0, "text"], "the log is read by every client, so deleting from it changes what a phone shows. "]]}]},
        "timeline render-core": {"request": {"project": "render-core"}, "first": {"ops": [["r", rc_tl]]}},
        "timeline forecasts": {"request": {"project": "forecasts"}, "first": {"ops": [["r", fc_tl]]}},
        "timeline notes-cli": {"request": {"project": "notes-cli"}, "first": {"ops": [["r", nc_tl]]}},
        "ends": {"stopping": {"end": {"reason": "stopping"}}, "revoked": {"end": {"reason": "revoked"}}, "failed": {"end": {"reason": "failed"}},
                 "heartbeat": ":", "note": "A frame is one line 'data: <json>' and a blank line; a heartbeat is a line ':' every 15 s without a frame. A cut with no end frame is a crash or the network."},
    },
    "examples": {
        "gap rule": {
            "note": "The laptop slept for an hour with render-core #3 open. It held entries up to " + str(RCK["rc3.v1"]["id"] + 3) + ". It reopens with after; the first frame's before is newer than what it held, so it fetches the gap.",
            "reopen": {"project": "render-core", "conversation": 3, "after": RCK["rc3.v1"]["id"] + 3},
            "history": {"project": "render-core", "conversation": 3, "minEntryId": RCK["rc3.v1"]["id"] + 4, "maxEntryId": RCK["rc3.revise"]["id"], "order": "ascending", "limit": 100}},
        "gap rule, first open": {
            "note": "The first open of render-core #3 on a device that holds nothing of it. The frame's before is set, because older entries exist, but nothing was held before the frame, so the gap rule makes no history call. Older entries come only when the user scrolls back.",
            "request": {"project": "render-core", "conversation": 3},
            "first": {"ops": [["r", rc3_first_open]]},
            "held": None,
            "expect": "no history call"},
    },
    "refusals": {
        "options_invalid (create w1 in scratch)": {"request": {"project": "scratch", "workflow": "w1"},
                                                    "reply": {"refused": {"errors": [req_err("designer"), req_err("implementer"), req_err("reviewer")], "code": "options_invalid", "message": "The options of w1 are not all set."}}},
        "bad_options (setOptions designer to an unknown model)": {"request": {"project": "scratch", "extension": "w1", "patch": {"designer": "anthropic/claude-opus-9"}},
                                                                 "reply": {"refused": {"errors": [{"path": "/designer", "keyword": "format", "message": "expected format model"}], "code": "bad_options", "message": "The options of w1 do not fit."}}},
        "not_a_workflow": {"request": {"project": "render-core", "workflow": "websearch"}, "reply": {"refused": {"code": "not_a_workflow", "message": "websearch is not a workflow."}}},
        "not_a_workflow (render-core has no workflow w3)": {"request": {"project": "render-core", "workflow": "w3"}, "reply": {"refused": {"code": "not_a_workflow", "message": "render-core has no workflow w3."}}},
        "options_invalid (create w1 in render-core, a model the server no longer has)": {"request": {"project": "render-core", "workflow": "w1"},
                                                                                         "reply": {"refused": {"errors": [{"path": "/reviewer", "keyword": "format", "message": "expected format model"}], "code": "options_invalid", "message": "The options of w1 are not all set."}}},
        "extension_not_loaded (create w1 while the server lacks w1)": {"request": {"project": "render-core", "workflow": "w1"}, "reply": {"refused": {"code": "extension_not_loaded", "message": "This server has no code for w1."}}},
        "unknown_model (create a bare agent on a model the server lacks)": {"request": {"project": "notes-cli", "model": "anthropic/claude-opus-9"}, "reply": {"refused": {"code": "unknown_model", "message": "This server has no model anthropic/claude-opus-9."}}},
        "unknown_model": {"request": {"project": "notes-cli", "conversation": 4, "model": "anthropic/claude-opus-9"}, "reply": {"refused": {"code": "unknown_model", "message": "This server has no model anthropic/claude-opus-9."}}},
        "busy (continue while a run works)": {"reply": {"refused": {"code": "busy", "message": "A run is working in this conversation."}}},
        "idle": {"reply": {"refused": {"code": "idle", "message": "Nothing runs in this conversation."}}},
        "event_log": {"reply": {"refused": {"code": "event_log", "message": "Nothing may run in the event log."}}},
        "conflict (answer w1:3:design#2 after the phone answered approve)": {
            "reply": {"refused": {"decision": {**dec["render-core/w1:3:design#2"]["result"]["decision"], "status": "answered", "choice": "approve", "annotations": [],
                                               "by": "phone", "at": NOW + 120000, "continuation": rc.task + 9},
                                  "code": "conflict", "message": "Decision w1:3:design#2 was answered with other fields."}}},
        "not_open (answer w1:3:design#2 after v3 superseded it)": {
            "reply": {"refused": {"open": {**dec["render-core/w1:3:design#2"]["result"]["decision"], "id": "w1:3:design#3", "basis": rc.next_entry + 4, "requestedAt": NOW + 900000},
                                  "code": "not_open", "message": "Decision w1:3:design#2 was superseded."}}},
        "stale_basis": {"reply": {"refused": {"code": "stale_basis", "message": "Decision w1:3:design#2 asks about another basis."}}},
        "bad_annotation": {"reply": {"refused": {"code": "bad_annotation", "message": "Decision w1:3:design#2 has a range outside its basis text."}}},
        "unknown_choice": {"reply": {"refused": {"code": "unknown_choice", "message": "Decision w1:3:design#2 does not offer maybe."}}},
        "extension_not_loaded (answer while the server lacks w1)": {"reply": {"refused": {"code": "extension_not_loaded", "message": "Decision w1:3:design#2 needs task w1.design_answer, which no installed extension provides."}}},
        "extension_not_loaded (send again after the fault while the server lacks w1)": {"request": {"project": "render-core", "decision": "w1:6:design#1", "basis": RCK["rc6.v1"]["id"], "choice": "approve", "annotations": []},
                                                                                        "reply": {"refused": {"code": "extension_not_loaded", "message": "Decision w1:6:design#1 needs task w1.design_answer, which no installed extension provides."}}},
        "unknown_conversation": {"reply": {"refused": {"code": "unknown_conversation", "message": "render-core has no conversation 99."}}},
        "conflict (bind with other fields)": {"request": {"project": "render-core", "workdir": "/home/vrod/src/render2", "extensions": ["w1"]},
                                              "reply": {"refused": {"project": {"project": "render-core", "workdir": WD, "extensions": ["websearch", "w1"], "missing": []}, "code": "conflict", "message": "render-core is bound with other fields."}}},
        "unauthorized (revoked)": {"reply": {"refused": {"code": "unauthorized", "message": "Send a valid device token."}}},
        "too_large": {"reply": {"refused": {"code": "too_large", "message": "The body is over 1 MiB."}}},
        "unknown (storage)": {"reply": {"unknown": {"code": "storage_failed", "message": "SQLITE_FULL: database or disk is full"}}},
        "bad_request": {"reply": {"refused": {"errors": [{"path": "/whenBusy", "keyword": "enum", "message": "must be one of steer, followUp"}], "code": "bad_request", "message": "The body does not fit the call."}}},
    },
    "hostile": hostile,
    "local": {
        "_note": "Device-local state of the laptop at 'now'. Not protocol.",
        "config": {"server": "https://tau.tail4e2f.ts.net", "device": "laptop", "token": "in the macOS Keychain, item 'tau https://tau.tail4e2f.ts.net'", "theme": "ember"},
        "sinceBase": {"at": BASE, "newestEvent": {"render-core": RCK["ev.b3"]["id"], "forecasts": FCK["ev.ask.linux"]["id"], "notes-cli": None}},
        "firstInput": {
            "notes-cli": {"2": "Add a `--json` flag to `notes list` so scripts can read the output.",
                          "3": "Why does `notes sync` hang on vaults over 10k files?",
                          "4": "Port the config loader from JSON to TOML. Keep reading `notes.json` for one release and warn when it is used.",
                          "5": "Write a migration test for v3 notes with front matter."},
            "render-core": {"2": "Explain how the outbox replays a lost answer.",
                            "3": "Add an export of the event log: `tau export --events` should write every event of a project to a file.",
                            "4": "Make `tau device list` print when each device last called the server.",
                            "5": "The owner approved this design in decision w1:4:design#1. Build it in your worktree at /home/vrod/src/render.w1-4, on branch w1/4.",
                            "6": "Show the server's protocol version in `tau --version`.",
                            "7": "Design a retention policy for the event log: how long events live, and what `tau` does with old ones.",
                            "8": "Summarise the open points of the export design in three lines for the hand-over note."},
            "forecasts": {"2": "I think rune gets to 1.0 within two weeks. Make that a forecast.",
                          "3": "What changed in pi-durable 1.5.2's `pi.live` document?"}},
        "extensionOf": {"_note": "Known extension of a conversation: from an event's kind prefix naming it, or from pi.agent.extensions once opened.",
                        "render-core": {"3": "w1", "4": "w1", "5": "w1", "6": "w1", "7": "w1"}, "forecasts": {"2": "w2"}},
        "modelOf": {"notes-cli": {"4": "anthropic/claude-sonnet-4-5", "5": "anthropic/claude-sonnet-4-5"},
                    "render-core": {"3": "anthropic/claude-opus-4-5", "5": "anthropic/claude-sonnet-4-5", "7": "anthropic/claude-opus-4-5"}},
        "noteDrafts": {"render-core/w1:3:design#2": [
            {"text": "Reported where? Say what the command prints.", "range": rng(DESIGN_V2, "A longer event is written whole; the limit is reported, not enforced.")},
            {"text": "Measure it on the render-core store before you build.", "range": rng(DESIGN_V2, "500 entries a page is a guess.")},
            {"text": "Close. Answer the two notes and I'll approve."}]},
        "composerDrafts": {"notes-cli/4": ""},
        "hiddenFaults": [],
        "hiddenFailedRuns": [],
        "sidebarVisible": True,
        "detailsVisible": False,
        "openItems": {"_note": "Groups in the order each project's first item was opened; items in the order opened (spec.md §4.1).",
                      "groups": [
                          {"project": "notes-cli", "items": [{"kind": "conversation", "id": 4}]},
                          {"project": "render-core", "items": [{"kind": "decision", "id": "w1:3:design#2"}, {"kind": "conversation", "id": 5}, {"kind": "project"}]},
                          {"project": "forecasts", "items": [{"kind": "decision", "id": "w2:2:pi-durable-2#1"}]}]},
        "places": {"_note": "Each place's one filter (none for Home and Projects) and its cursor, by object ID. Kept when you leave a place and across launches.",
                   "home": {"cursor": "decision render-core/w1:3:design#2"},
                   "conversations": {"filter": {"project": None, "state": None}, "cursor": "conversation render-core/7"},
                   "timeline": {"filter": {"project": None}, "cursor": None},
                   "faults": {"filter": {"project": None}, "cursor": None},
                   "projects": {"cursor": "project render-core"}},
        "lastProject": "render-core",
        "lastWorkflow": {"_note": "The workflow, or the bare agent's model, last started in each project from this device.",
                         "render-core": {"workflow": "w1"}, "notes-cli": {"model": "anthropic/claude-sonnet-4-5"}, "forecasts": {"workflow": "w2"}},
        "startDraft": None,
        "startDraftExample": {"text": START_TEXT, "project": "render-core", "workflow": "w1",
                              "only in": "states start-no-reply and start-options-unset (with project scratch); kept until create succeeds"},
        "outbox": [
            {"call": "input", "body": {"project": "notes-cli", "conversation": 4, "text": "Also keep the `NOTES_CONFIG` environment variable working; it should win over both files.", "requestId": "r-01JCV8Q6W3M2", "whenBusy": "steer"},
             "state": "done", "sentAt": NOW - 9000},
            {"call": "input", "body": {"project": "notes-cli", "conversation": 4, "text": "Use node:test's snapshot support for the TOML fixtures.", "requestId": "r-01JCVB0P4Q8Z", "whenBusy": "followUp"},
             "state": "unknown", "sentAt": NOW - 2000, "only in": "states unknown-outcome and offline"}],
    },
}

# drop non-protocol helper key accidentally holding task counters
with open(OUT, "w") as f:
    json.dump(fixture, f, indent=1, ensure_ascii=False)
    f.write("\n")

# Report a few IDs for the docs.
print("render-core v2 basis", RCK["rc3.v2"]["id"], "v1", RCK["rc3.v1"]["id"], "events", [e["id"] for e in rc_entries[1]])
print("forecasts events", [(e["id"], e["kind"]) for e in fc_entries[1]])
print("nc4 entries", [(e["id"], e["kind"]) for e in nc_entries[4]])
print("task ids rc", rc.task, "fc", fc.task, "nc", nc.task)
print("faults", thenC, due_tasks["webfox-2"])
print("size", os.path.getsize(OUT))
