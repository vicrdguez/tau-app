#!/usr/bin/env python3
"""KL4g for ../keymap.toml: a command whose `wire` includes "answer" must be tier "decide", bound only under <leader>a,
in context "decision" only. The Brief's keymap lint checks KL4 on decide-tier commands; this closes the other side:
a write-tier command that sends an answer (as `␣rt` once did) passes KL4 and fails here.

    python3 -B keymap_answer_rule.py [keymap.toml]          # exit 0 when clean
    python3 -B keymap_answer_rule.py --mutants              # also checks keymap-mutants.toml; writes the others to a temp folder for the workbench lint

Requires Python >= 3.11 (tomllib)."""
import pathlib
import sys
import tempfile
import tomllib

HERE = pathlib.Path(__file__).resolve().parent


def kl4g(data: dict) -> list[str]:
    out = []
    for c in data.get("command", []):
        if "answer" not in (c.get("wire") or []):
            continue
        cid = c.get("id", "?")
        if c.get("tier") != "decide":
            out.append(f"KL4g  {cid} sends answer but is tier {c.get('tier')!r}, not decide")
        if set(c.get("ctx") or []) != {"decision"}:
            out.append(f"KL4g  {cid} sends answer outside context decision: {c.get('ctx')}")
        for k in c.get("keys", []):
            if not str(k.get("seq", "")).startswith("<leader>a"):
                out.append(f"KL4g  {cid} binding {k.get('seq')} is not under <leader>a")
    return out


def main(argv: list[str]) -> int:
    path = pathlib.Path(argv[0]) if argv and not argv[0].startswith("--") else HERE.parent / "keymap.toml"
    text = path.read_text()
    vs = kl4g(tomllib.loads(text))
    for v in vs:
        print(v)
    print(f"KL4g: {len(vs)} violation(s) in {path.name}")
    if "--mutants" not in argv:
        return 1 if vs else 0
    muts = tomllib.loads((HERE / "keymap-mutants.toml").read_text())["mutant"]
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="tau-keymap-mutants-"))
    caught = 0
    for i, m in enumerate(muts, 1):
        assert m["find"] in text, m["name"]
        mutated = text.replace(m["find"], m["replace"], 1)
        if m["rule"] == "KL4g":
            hit = bool(kl4g(tomllib.loads(mutated)))
            print(f"{'caught' if hit else 'MISSED'}  {m['rule']:5} {m['name']}")
        else:
            out = scratch / f"mutant-{i}-{m['rule']}.toml"
            out.write_text(mutated)
            print(f"written {out}: the workbench lint must report {m['rule']}  ({m['name']})")
            hit = True
        caught += hit
    print(f"{caught}/{len(muts)} mutants: KL4g ones checked here, the others written for the workbench lint")
    return 0 if not vs and caught == len(muts) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
