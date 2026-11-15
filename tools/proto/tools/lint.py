#!/usr/bin/env python3
"""Keymap lint, phase 2: KL0 KL1 KL2 KL4 KL5 KL7 KL8 KL9 KL10 KL12 KL14 KL16, implemented from the written rules in design/grammar.md section 15
(E-PROTO-2). KL0 absorbs KL3/KL11/KL13; KL5 absorbs KL6; KL15 is dropped. Numbers kept for traceability with the phase-1 mutants.

    python3 tools/lint.py [keymap.toml] [--protocol design/protocol-requirements.md] [--cheatsheet cheatsheet.md]
                          [--write-cheatsheet] [--report results/lint.txt]

Exit status 0 only if the keymap has no violations and (with --report) every seeded mutant newly trips its own rule.
Phase 5 (E-PROTO-5): KL4c now requires `wire` on every authority command and forbids a `readback` field (the host's Basis.fields is the only
source); KL4f forbids a list context on ␣aa ␣ar ␣ad ␣ac (only auth.cancel may be offered from a list row). Failure cases, written before the
code: a mutant with a readback field passes; a mutant with no wire passes; a mutant that adds `list` to auth.approve passes; the count check
does not report 116 commands and 184 bindings.
Requires Python >= 3.11 (stdlib tomllib).
"""
from __future__ import annotations

import argparse
import pathlib
import re
import sys
import tomllib
from dataclasses import dataclass

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_PROTOCOL = ROOT.parent.parent / "design" / "protocol-requirements.md"


@dataclass(frozen=True)
class Violation:
    rule: str
    msg: str
    where: str = ""

    def __str__(self):
        return f"{self.rule}  {self.msg}" + (f"  [{self.where}]" if self.where else "")


class Keymap:
    def __init__(self, data: dict):
        self.raw = data
        self.meta = data.get("meta", {})
        self.contexts: dict[str, str] = data.get("contexts", {})
        self.groups: dict[str, str] = data.get("groups", {})
        self.builtin: dict = data.get("builtin", {})
        self.commands: list[dict] = data.get("command", [])
        self.leader = self.meta.get("leader", "<Space>")

    # ---- context tree
    def chain(self, ctx: str) -> list[str]:
        out, seen, c = [], set(), ctx
        while c and c not in seen:
            out.append(c)
            seen.add(c)
            c = self.contexts.get(c, "")
        return out

    def applies_in(self, cmd: dict, ctx: str) -> bool:
        ch = self.chain(ctx)
        return any(c in ch for c in cmd.get("ctx", []))

    def effective(self, cmd: dict) -> set[str]:
        return {c for c in self.contexts if self.applies_in(cmd, c)}

    def overlap(self, a: str, b: str) -> bool:
        """Contexts overlap when equal or one is an ancestor of the other."""
        return a == b or a in self.chain(b) or b in self.chain(a)

    def descendants_of(self, ctx: str) -> set[str]:
        return {c for c in self.contexts if ctx in self.chain(c)}


def load_text(text: str) -> Keymap:
    return Keymap(tomllib.loads(text))


def load(path) -> Keymap:
    return load_text(pathlib.Path(path).read_text())


# ------------------------------------------------------------------ key syntax
_NAMED = r"CR|Esc|Tab|BS|Space"
_TOKEN = re.compile(rf"<(?:(?:C|M|S|D)-)*(?:[A-Za-z0-9\[\]\\;'`,./=\-]|{_NAMED})>|<(?:{_NAMED}|leader)>")  # D = Cmd: GUI-only alias (KL9)
_PLACEHOLDER = re.compile(r"<(?:code|label)>")


def tokenize(seq: str, leader: str = "<Space>") -> list[str]:
    """Parse a sequence into tokens; raises ValueError on a malformed <...> token."""
    out, i = [], 0
    while i < len(seq):
        if seq[i] == "<" and i + 1 < len(seq) and (seq[i + 1].isalpha() or seq[i + 1] in "CMS"):
            j = seq.find(">", i)
            if j < 0:
                raise ValueError(f"unterminated token in {seq!r}")
            tok = seq[i : j + 1]
            if _PLACEHOLDER.fullmatch(tok):
                out.append(tok)
            elif _TOKEN.fullmatch(tok):
                out.append(leader if tok == "<leader>" else tok)
            else:
                raise ValueError(f"bad key token {tok!r} in {seq!r}")
            i = j + 1
        else:
            out.append(seq[i])
            i += 1
    return out


def bindings(km: Keymap):
    """Yield (cmd, key_dict, tokens or None, syntax_error or None)."""
    for cmd in km.commands:
        for k in cmd.get("keys", []):
            try:
                yield cmd, k, tokenize(k.get("seq", ""), km.leader), None
            except ValueError as e:
                yield cmd, k, None, str(e)


def _is_prefix(a: list[str], b: list[str]) -> bool:
    return len(a) <= len(b) and b[: len(a)] == a


# ------------------------------------------------------------------ protocol catalogue (KL7)
def protocol_wire_types(text: str) -> set[str]:
    start = text.find("## 4.")
    sec = text[start:] if start >= 0 else text
    end = sec.find("\n## ", 5)
    sec = sec[: end if end > 0 else None]
    return {m.group(1) for m in re.finditer(r"^\|\s*`([A-Za-z*\-]+)`\s*\|", sec, re.M)}


# ------------------------------------------------------------------ cheat sheet (KL10)
def _safe_tokens(seq: str, leader: str) -> list[str]:
    try:
        return tokenize(seq, leader)
    except ValueError:
        return list(seq)


def cheatsheet(km: Keymap) -> str:
    def keys(cmd):
        return " · ".join(f"`{_disp(_safe_tokens(k['seq'], km.leader))}`" for k in cmd.get("keys", []) if cmd.get("kind") != "modal" and k.get("mode") in ("n", "v", "i")) or "—"

    def row(cmd):
        mark = "◆ " if cmd.get("tier") in ("decide", "bind") else ""
        return f"| {keys(cmd)} | `:{cmd['ex']}` | {cmd['desc']} | {cmd.get('tier', 'local')} |"

    L = ["# Render keymap cheat sheet", "", "Generated from `keymap.toml` by `tools/lint.py --write-cheatsheet`; do not edit by hand (KL10 fails when it drifts).",
         "Leader is `␣`. Tiers: local · write · decide ◆ (readback + y) · bind ◆ (readback + check code + ⏎).", ""]
    head = "| keys | ex | description | tier |\n|---|---|---|---|"
    cmds = [c for c in km.commands if c.get("kind") != "modal"]
    leader_groups: dict[str, list] = {}
    other, ex_only = [], []
    for c in cmds:
        lk = [_safe_tokens(k["seq"], km.leader) for k in c.get("keys", []) if k.get("mode") == "n"]
        lead = [t for t in lk if t and t[0] == km.leader and len(t) > 1]
        if lead:
            g = lead[0][1] if len(lead[0]) > 2 else "·"
            leader_groups.setdefault(g, []).append(c)
        elif c.get("keys"):
            other.append(c)
        else:
            ex_only.append(c)
    L += ["## Leader tree", ""]
    for g in sorted(leader_groups):
        label = km.groups.get(f"<leader>{g}", "") if g != "·" else "single-key leader commands"
        L += [f"### `␣{g if g != '·' else ''}` {label}".rstrip(), "", head] + [row(c) for c in sorted(leader_groups[g], key=lambda c: c["id"])] + [""]
    L += ["## Direct keys (`g` `z` `]` `[` `<C-w>` and single keys)", "", head] + [row(c) for c in sorted(other, key=lambda c: c["id"])] + [""]
    L += ["## Ex only (reachable from `:` and the palette)", "", head] + [row(c) for c in sorted(ex_only, key=lambda c: c["id"])] + [""]
    modal = [c for c in km.commands if c.get("kind") == "modal"]
    L += ["## Modal keys (readback, hint, picker)", "", "| keys | description |", "|---|---|"]
    for c in modal:
        L.append("| " + " · ".join(f"`{_disp(_safe_tokens(k['seq'], km.leader))}`" for k in c["keys"]) + f" | {c['desc']} |")
    return "\n".join(L) + "\n"


def _disp(toks: list[str]) -> str:
    parts = [("␣" if t == "<Space>" else "⏎" if t == "<CR>" else t[1:-1].replace("CR", "⏎") if t.startswith("<") and len(t) > 2 else t) for t in toks]
    return "".join(("" if i == 0 else " " if (len(p) > 1 or len(parts[i - 1]) > 1) else "") + p for i, p in enumerate(parts))


# ------------------------------------------------------------------ the rules
RESERVED_LOCAL = {"<Esc>", "<C-c>", "<C-[>", "q", "u", "<C-r>", "."}
TRAP_KEYS = {"dd", "D", "x", "X", "s", "S", "r", "R", "p", "P", "@", "ZQ"}
TRAP_EX_LOCAL = {"q!", "qa"}
TRAP_EX_WRITE = {"w", "wq", "x"}
TIER_RANK = {"local": 0, "write": 1, "decide": 2, "bind": 3}
NOT_LEGACY = {"<C-h>", "<C-i>", "<C-m>", "<C-[>", "<C-CR>", "<S-CR>"}
INSERT_OK = {"<Esc>", "jk", "<C-CR>", "<M-CR>", "<D-CR>", "<C-w>", "<C-u>", "<CR>", "<C-t>", "<C-d>", "<C-j>", "<C-k>", "<C-f>", "<C-n>", "<C-p>", "<Tab>", "<BS>"}
HOST_TRIGGER_KEYS = {"trigger", "on_event", "on_host_event", "host_event", "event", "subscribe", "when"}
RULES = ["KL0", "KL1", "KL2", "KL4", "KL5", "KL7", "KL8", "KL9", "KL10", "KL12", "KL14", "KL16"]
# grammar.md §15: the [[command]] fields; anything else is a schema error
CMD_FIELDS = {"id", "ex", "desc", "tier", "kind", "ctx", "wire", "readback", "keys", "countable", "repeatable", "alias_of", "shadows", "confirm_local", "effect", "local_only", "object", "jump", "chars", "tui", "tui_alt", "ex_aliases", "note"}
KEY_FIELDS = {"mode", "ctx", "seq", "tui", "tui_alt", "shadows", "alias_of"}
REQUIRED_FIELDS = ["id", "ex", "desc", "tier", "kind", "ctx"]
NON_NORMAL = {"i", "o", "h", "rb", "c", "ho"}  # ho: the handover overlay (CD-08)
EDITOR_CTXS = ("editor",)


def check(km: Keymap, protocol_text: str | None = None, cheatsheet_text: str | None = None) -> list[Violation]:
    V: list[Violation] = []
    add = lambda r, m, w="": V.append(Violation(r, m, w))  # noqa: E731
    cmds = km.commands
    byid = {}
    # KL1 reachability ------------------------------------------------------
    seen_ex: dict[str, str] = {}
    for c in cmds:
        cid = c.get("id", "?")
        if cid in byid:
            add("KL1", f"duplicate id {cid}", cid)
        byid[cid] = c
        if c.get("kind") == "modal":
            continue
        ex = c.get("ex")
        if not ex:
            add("KL1", f"{cid} has no ex name, so it is unreachable from ':' and the palette", cid)
            continue
        for name in [ex, *c.get("ex_aliases", [])]:
            if name in seen_ex and seen_ex[name] != cid:
                add("KL1", f"ex name :{name} is used by both {seen_ex[name]} and {cid}", cid)
            seen_ex[name] = cid
        if c.get("tier", "local") != "local" and c.get("palette", True) is False:
            add("KL1", f"non-local command {cid} is missing from the palette", cid)

    # KL0 schema (phase 2: absorbs KL3 key syntax, KL11 contexts, KL13 no host triggers) -------------
    for c in cmds:
        cid = c.get("id", "?")
        for key in c:
            if key not in CMD_FIELDS and key not in HOST_TRIGGER_KEYS:
                add("KL0", f"{cid} has unknown field {key!r} (schema)", cid)
        for need in REQUIRED_FIELDS:
            if need == "ex" and c.get("kind") == "modal":
                continue
            if need not in c:
                add("KL0", f"{cid} lacks required field {need!r}", cid)
        if c.get("tier") not in ("local", "write", "decide"):
            add("KL0", f"{cid} tier {c.get('tier')!r} is not local, write or decide", cid)
        for k in c.get("keys", []):
            for key in k:
                if key not in KEY_FIELDS and key not in HOST_TRIGGER_KEYS:
                    add("KL0", f"{cid} binding {k.get('seq')} has unknown field {key!r} (schema)", cid)
            if "<label>" in str(k.get("seq", "")) and c.get("kind") != "modal":
                add("KL0", f"{cid}: the <label> placeholder is only allowed in kind = 'modal'", cid)
    for ctx, parent in km.contexts.items():
        if parent and parent not in km.contexts:
            add("KL0", f"context {ctx} has unknown parent {parent}", ctx)
        seen, c = set(), ctx
        while c:
            if c in seen:
                add("KL0", f"context tree has a cycle through {ctx}", ctx)
                break
            seen.add(c)
            c = km.contexts.get(c, "")
    for c in cmds:
        for x in c.get("ctx", []):
            if x not in km.contexts:
                add("KL0", f"{c.get('id')} references unknown context {x}", c.get("id", ""))
        for k in c.get("keys", []):
            if k.get("ctx") not in km.contexts:
                add("KL0", f"{c.get('id')} binds {k.get('seq')} in unknown context {k.get('ctx')}", c.get("id", ""))

    # KL3 key syntax --------------------------------------------------------
    B = []
    for cmd, k, toks, err in bindings(km):
        if err:
            add("KL0", f"{cmd.get('id')}: {err}", cmd.get("id", ""))
            continue
        if k.get("tui_alt"):
            try:
                tokenize(k["tui_alt"], km.leader)
            except ValueError as e:
                add("KL0", f"{cmd.get('id')} tui_alt: {e}", cmd.get("id", ""))
        B.append((cmd, k, toks))

    # KL2 prefix-freedom ----------------------------------------------------
    builtin_sets = []   # (context, mode, tokens, is_operator)
    for name, ctx, mode in (("reader_motions", "reader", "n"), ("list_motions", "list", "n"), ("editor_normal", "editor", "n"), ("editor_textobjects", "editor", "n"), ("editor_insert", "editor", "i")):
        for s in km.builtin.get(name, []):
            try:
                builtin_sets.append((ctx, mode, tokenize(s, km.leader)))
            except ValueError:
                pass
    timeout_ok = {tuple(tokenize(s, km.leader)) for s in km.builtin.get("timeout_ok", [])}
    ops = [(cmd, k, toks) for cmd, k, toks in B if cmd.get("kind") in ("operator", "prefixarg")]
    for i, (ca, ka, ta) in enumerate(B):
        for cb, kb, tb in B[i + 1:]:
            if ka.get("mode") != kb.get("mode") or not km.overlap(ka.get("ctx", ""), kb.get("ctx", "")):
                continue
            if ca.get("id") == cb.get("id") and ta == tb:
                continue
            if tuple(ta) in timeout_ok or tuple(tb) in timeout_ok:
                continue
            if _is_prefix(ta, tb) or _is_prefix(tb, ta):
                (inner, ik), outer = ((ca, ka), cb) if len(km.chain(ka["ctx"])) >= len(km.chain(kb["ctx"])) else ((cb, kb), ca)
                if km.chain(ka["ctx"]) != km.chain(kb["ctx"]) and (outer.get("id") in (inner.get("shadows"), ik.get("shadows")) or ik.get("shadows") == "parent"):     # `shadows` may sit on the command or on the binding (keymap.toml uses both)
                    continue
                add("KL2", f"{ca.get('id')} {ka['seq']} ({ka['ctx']}) and {cb.get('id')} {kb['seq']} ({kb['ctx']}) are not prefix-free in mode {ka['mode']}", ca.get("id", ""))
    for co, ko, to in ops:       # an operator / prefix-arg key reserves every o.x in overlapping contexts
        for cb, kb, tb in B:
            if cb is co and kb is ko:
                continue
            if kb.get("mode") != ko.get("mode") or not km.overlap(ko.get("ctx", ""), kb.get("ctx", "")):
                continue
            if len(tb) > len(to) and tb[: len(to)] == to:
                add("KL2", f"{cb.get('id')} {kb['seq']} starts with the {co.get('kind')} key {ko['seq']} of {co.get('id')}, which reserves every {ko['seq']}·x", cb.get("id", ""))
    for cmd, k, toks in B:       # literal bindings against the reserved builtin sets
        for bctx, bmode, btoks in builtin_sets:
            if k.get("mode") != bmode or not km.overlap(k.get("ctx", ""), bctx) or tuple(toks) in timeout_ok or tuple(btoks) in timeout_ok:
                continue
            if _is_prefix(toks, btoks) or _is_prefix(btoks, toks):
                add("KL2", f"{cmd.get('id')} {k['seq']} ({k['ctx']}) collides with the builtin {''.join(btoks)} ({bctx})", cmd.get("id", ""))

    # KL4 authority ---------------------------------------------------------
    editors = set().union(*[km.descendants_of(e) for e in EDITOR_CTXS if e in km.contexts]) if km.contexts else set()
    for cmd in cmds:
        tier = cmd.get("tier", "local")
        cid = cmd.get("id", "")
        if tier == "decide":
            for c, k, toks in B:
                if c is not cmd:
                    continue
                if k.get("mode") in NON_NORMAL:
                    add("KL4", f"{cid} has a binding in mode {k['mode']} (Insert/Operator-pending/Hint/Readback/Picker): authority never lives there", cid)
                elif not (len(toks) >= 2 and toks[0] == km.leader and toks[1] == "a"):
                    add("KL4", f"{cid} binding {k['seq']} does not start with <leader>a", cid)
            # phase 5 (grammar §15, challenge 4): KL4c — the host's Basis.fields is the only readback source, so every authority command names its
            # host decidable (wire) and lists no readback fields of its own
            if not cmd.get("wire"):
                add("KL4", f"KL4c {cid} has no host decidable (wire)", cid)
            if "readback" in cmd:
                add("KL4", f"KL4c {cid} lists readback fields; the host basis is the only source", cid)
            # phase 5 (challenge 2, FC-T18): KL4f — only auth.cancel may be offered from a list row
            if cid != "auth.cancel" and "list" in (cmd.get("ctx") or []):
                add("KL4", f"KL4f {cid} offered from a list row", cid)
            if cmd.get("repeatable") or cmd.get("countable"):
                add("KL4", f"{cid} must be neither repeatable nor countable", cid)
            leak = sorted(km.effective(cmd) & editors)
            if leak:
                add("KL4", f"{cid} applies in editor contexts {leak}: ctx must exclude editor and its children", cid)
        else:
            for c, k, toks in B:
                if c is cmd and len(toks) >= 2 and toks[0] == km.leader and toks[1] == "a":
                    add("KL4", f"{cid} is tier {tier} but sits under <leader>a (only decide belongs there)", cid)

    # KL5 reserved local keys ----------------------------------------------
    for cmd, k, toks in B:
        key = "".join(toks)
        if cmd.get("tier", "local") != "local" and (key in RESERVED_LOCAL or (k.get("mode") == "i" and key == "jk")):
            add("KL5", f"{cmd.get('id')} binds the reserved local key {key} at tier {cmd.get('tier')}", cmd.get("id", ""))

    # KL6 trap keys ---------------------------------------------------------
    for cmd, k, toks in B:
        key = "".join(toks)
        if k.get("mode") in ("n", "v") and key in TRAP_KEYS and cmd.get("tier", "local") != "local":
            add("KL5", f"{cmd.get('id')} binds the trap key {key} at tier {cmd.get('tier')}", cmd.get("id", ""))
        if k.get("mode") in ("n", "v") and key == "ZZ" and TIER_RANK.get(cmd.get("tier", "local"), 0) > TIER_RANK["write"]:
            add("KL5", f"{cmd.get('id')} binds ZZ above write", cmd.get("id", ""))
    for cmd in cmds:
        names = {cmd.get("ex"), *cmd.get("ex_aliases", [])} - {None}
        tier = cmd.get("tier", "local")
        for n in names & TRAP_EX_LOCAL:
            if tier != "local":
                add("KL5", f"ex :{n} ({cmd.get('id')}) must be local, is {tier}", cmd.get("id", ""))
        for n in names & TRAP_EX_WRITE:
            if TIER_RANK.get(tier, 0) > TIER_RANK["write"]:
                add("KL5", f"ex :{n} ({cmd.get('id')}) may be at most write, is {tier}", cmd.get("id", ""))
        if cmd.get("effect") == "discard" and cmd.get("local_only") is not True:
            add("KL5", f"{cmd.get('id')} discards but does not declare local_only = true", cmd.get("id", ""))

    # KL7 wire coverage -----------------------------------------------------
    if protocol_text is not None:
        known = protocol_wire_types(protocol_text)
        for cmd in cmds:
            for w in cmd.get("wire", []):
                if w not in known:
                    add("KL7", f"{cmd.get('id')} wire type {w!r} is not in the protocol-requirements.md section 4 catalogue", cmd.get("id", ""))

    # KL8 which-key ---------------------------------------------------------
    group_toks = {tuple(tokenize(g, km.leader)) for g in km.groups}
    per_group: dict[tuple, dict[str, str]] = {}
    for cmd, k, toks in B:
        if cmd.get("kind") == "modal":
            continue
        if toks and toks[0] == km.leader and len(toks) > 1:
            for n in range(1, len(toks)):
                if tuple(toks[:n]) not in group_toks:
                    add("KL8", f"{cmd.get('id')} {k['seq']}: prefix {''.join(toks[:n])} has no [groups] label", cmd.get("id", ""))
            g = per_group.setdefault(tuple(toks[:-1]), {})
            if cmd.get("desc") in g and g[cmd["desc"]] != cmd.get("id"):
                add("KL8", f"desc {cmd['desc']!r} is used by {g[cmd['desc']]} and {cmd.get('id')} in the same which-key group {''.join(toks[:-1])}", cmd.get("id", ""))
            g[cmd.get("desc")] = cmd.get("id")
    for cmd in cmds:     # desc length is per command, reported once, bound or ex-only
        if cmd.get("kind") != "modal" and len(cmd.get("desc", "")) > 32:
            add("KL8", f"{cmd.get('id')} desc is {len(cmd['desc'])} chars (max 32)", cmd.get("id", ""))

    # KL9 terminal representability ----------------------------------------
    for cmd in cmds:
        mine = [(k, t) for c, k, t in B if c is cmd and k.get("mode") in ("n", "v", "i")]
        def legacy(t):
            return not any(x in NOT_LEGACY or x.startswith("<M-") or x.startswith("<D-") or re.match(r"<C-S-", x) for x in t)
        has_legacy = any(legacy(t) for k, t in mine)
        for k, t in mine:
            if legacy(t):
                continue
            if k.get("tui_alt"):
                continue
            if k.get("tui") == "legacy-as-tab" and "".join(t) == "<C-i>" and not any("<Tab>" in tt for kk, tt in [(kk, tt) for c, kk, tt in B if kk.get("mode") == "n"]):
                continue
            if has_legacy:
                continue
            add("KL9", f"{cmd.get('id')} binding {k['seq']} has no legacy terminal encoding and no tui_alt or other legacy binding", cmd.get("id", ""))

    # KL10 docs drift -------------------------------------------------------
    if cheatsheet_text is not None and cheatsheet_text.strip() != cheatsheet(km).strip():
        add("KL10", "the committed cheat sheet differs from the one generated from the table")

    # KL12 objects ----------------------------------------------------------
    names: dict[str, set[str]] = {}
    for cmd in cmds:
        if cmd.get("kind") == "motion":
            if "object" in cmd:
                names.setdefault(cmd["object"], set()).add(cmd["id"].split(".")[0])
            if "jump" not in cmd:
                add("KL12", f"motion {cmd.get('id')} does not declare jump", cmd.get("id", ""))
    for ch, ns in names.items():
        if len(ns) > 1:
            add("KL12", f"object char {ch} means {sorted(ns)} in different ]x motions", ch)
    for cmd in cmds:
        if cmd.get("kind") == "textobject":
            for ch, nm in cmd.get("chars", {}).items():
                if ch in names and nm not in names[ch]:
                    add("KL12", f"object char {ch} is {sorted(names[ch])} in ]{ch}/[{ch} motions but {nm!r} as a text object", cmd.get("id", ""))
    # phase 2 (D3): [builtin].editor_textobjects are vim's and mini.ai's lexicon, scoped to editors, and exempt from KL12

    # KL13 no host triggers ---------------------------------------------------
    for cmd in cmds:
        for key in cmd:
            if key in HOST_TRIGGER_KEYS:
                add("KL0", f"{cmd.get('id')} declares a host-event trigger ({key})", cmd.get("id", ""))
        for k in cmd.get("keys", []):
            for key in k:
                if key in HOST_TRIGGER_KEYS:
                    add("KL0", f"{cmd.get('id')} binding {k.get('seq')} declares a host-event trigger ({key})", cmd.get("id", ""))

    # KL14 Insert whitelist ---------------------------------------------------
    ok = INSERT_OK | {"".join(tokenize(s, km.leader)) for s in km.builtin.get("editor_insert", [])}
    for cmd, k, toks in B:
        if k.get("mode") == "i":
            key = "".join(toks)
            if km.leader in toks or key not in ok:
                add("KL14", f"{cmd.get('id')} Insert binding {k['seq']} is not editing, completion, <Esc>, jk, <C-CR>, <M-CR> or <D-CR> (no leader)", cmd.get("id", ""))

    # KL15 dropped in phase 2 (grammar.md §15): pinning is a resolver property, checked by TRAP

    # KL16 aliases ------------------------------------------------------------
    for cmd, k, toks in B:
        a = k.get("alias_of")
        if a is None:
            continue
        t = byid.get(a)
        if t is None:
            add("KL16", f"{cmd.get('id')} binding {k['seq']} is alias_of unknown command {a}", cmd.get("id", ""))
            continue
        if t.get("tier", "local") != cmd.get("tier", "local"):
            add("KL16", f"{cmd.get('id')} alias_of {a} but tiers differ ({cmd.get('tier')} vs {t.get('tier')})", cmd.get("id", ""))
        if not any(km.overlap(x, y) for x in cmd.get("ctx", []) for y in t.get("ctx", [])):
            add("KL16", f"{cmd.get('id')} alias_of {a} but their contexts do not overlap", cmd.get("id", ""))
    return V


# ------------------------------------------------------------------ CLI / report
def run_report(keymap_path: pathlib.Path, protocol_path: pathlib.Path, sheet_path: pathlib.Path, mutants_dir: pathlib.Path, out: pathlib.Path) -> int:
    proto = protocol_path.read_text()
    sheet = sheet_path.read_text() if sheet_path.exists() else None
    km = load(keymap_path)
    base = check(km, proto, sheet)
    lines = [f"# keymap lint, phase 2 ({' '.join(RULES)}), implemented from design/grammar.md section 15",
             f"# keymap: {keymap_path.name}  commands: {len(km.commands)}  python: {sys.version.split()[0]}", "",
             "## real keymap"]
    by_rule: dict[str, list[Violation]] = {}
    for v in base:
        by_rule.setdefault(v.rule, []).append(v)
    for r in RULES:
        lines.append(f"{r:5} {'FAIL' if r in by_rule else 'PASS'}" + (f"  ({len(by_rule[r])})" if r in by_rule else ""))
        for v in by_rule.get(r, []):
            lines.append(f"        - {v.msg}")
    real_ok = not base
    lines += ["", f"real keymap: {'PASS' if real_ok else 'FAIL'} ({len(base)} violation{'s' if len(base) != 1 else ''})", "", "## seeded mutants (each is the real keymap plus one seeded violation; it must newly trip its own rule)"]
    base_set = {(v.rule, v.msg) for v in base}
    mut_ok = True
    for n in [r[2:] for r in RULES] + ["6", "4c", "4f"]:
        p = mutants_dir / f"KL{n}.toml"
        if not p.exists():
            lines.append(f"KL{n:<3} MISSING {p.name}")
            mut_ok = False
            continue
        vs = check(load(p), proto, sheet)
        new = {(v.rule, v.msg) for v in vs} - base_set
        tripped = sorted({r for r, _ in new}, key=lambda x: int(x[2:]))
        own = "KL5" if n == "6" else "KL4" if n in ("4c", "4f") else f"KL{n}"                                           # the phase-1 KL6 mutant (trap key) now trips KL5, which absorbed KL6
        ok = own in tripped
        mut_ok &= ok
        lines.append(f"KL{n:<3} {'ok  ' if ok else 'FAIL'} {p.name}: newly trips {', '.join(tripped) or 'nothing'}")
        for r, m in sorted(new):
            if r == own:
                lines.append(f"        - {m}")
                break
    nb = sum(len(c.get("keys") or []) for c in km.commands); count_ok = len(km.commands) == 116 and nb == 184      # phase 6: 116 commands, 184 bindings
    lines += ["", f"mutants: {'all ' + str(len(RULES) + 3) + ' trip their own rule' if mut_ok else 'SOME DID NOT TRIP THEIR RULE'}", f"keymap size: {len(km.commands)} commands, {nb} bindings ({'as designed' if count_ok else 'NOT 116 / 184'})", f"overall: {'PASS' if real_ok and mut_ok and count_ok else 'FAIL'}"]
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    return 0 if real_ok and mut_ok and count_ok else 1


def main(argv=None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("keymap", nargs="?", default=str(ROOT / "keymap.toml"))
    ap.add_argument("--protocol", default=str(DEFAULT_PROTOCOL))
    ap.add_argument("--cheatsheet", default=str(ROOT / "cheatsheet.md"))
    ap.add_argument("--write-cheatsheet", action="store_true")
    ap.add_argument("--report")
    ap.add_argument("--mutants", default=str(ROOT / "tools" / "mutants"))
    a = ap.parse_args(argv)
    km = load(a.keymap)
    if a.write_cheatsheet:
        pathlib.Path(a.cheatsheet).write_text(cheatsheet(km))
        print(f"wrote {a.cheatsheet}")
        return 0
    if a.report:
        return run_report(pathlib.Path(a.keymap), pathlib.Path(a.protocol), pathlib.Path(a.cheatsheet), pathlib.Path(a.mutants), pathlib.Path(a.report))
    sheet = pathlib.Path(a.cheatsheet).read_text() if pathlib.Path(a.cheatsheet).exists() else None
    vs = check(km, pathlib.Path(a.protocol).read_text(), sheet)
    for v in vs:
        print(v)
    print(f"{len(vs)} violation(s)")
    return 1 if vs else 0


if __name__ == "__main__":
    sys.exit(main())
