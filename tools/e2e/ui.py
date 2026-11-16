"""Drive the real app through Rune input scripts, then assert its rendered tree."""

import json
from pathlib import Path
import subprocess
import sys

binary, url, code, state = sys.argv[1:]
state = Path(state)
out = Path("target/snaps")
out.mkdir(parents=True, exist_ok=True)
message = "tau UI real server proof"


def key(t, name, **mods):
    return {"t": t, "type": "key", "name": name, **mods}


def typing(t, text):
    return {"t": t, "type": "type", "text": text, "every": 0.001}


def run(name, events, at, expected):
    script = state / f"{name}.json"
    script.write_text(json.dumps(events))
    tree = out / f"e2e-ui-{name}.json"
    subprocess.run(
        [binary, "--live", "--realtime", "--script", str(script),
         "--snap", str(out / f"e2e-ui-{name}.png"), "--tree", str(tree),
         "--at", str(at), "--size", "1512x982", "--scale", "1"],
        check=True, timeout=at + 45,
    )
    data = json.loads(tree.read_text())
    texts = []

    def visit(value):
        if isinstance(value, dict):
            if isinstance(value.get("text"), str):
                texts.append(value["text"])
            for child in value.values():
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(data)
    for text in expected:
        if not any(text in actual for actual in texts):
            raise SystemExit(f"{name}: tree does not show {text!r}; inspect {tree}")
    print(f"e2e-ui {name}: passed", flush=True)


run("connect", [], 0.2, ["Connect"])
run("home", [
    key(0.2, "a", cmd=True), typing(0.3, url), key(0.4, "Tab"),
    typing(0.5, code), key(0.6, "Enter"),
], 3, ["Home", "ui → 127.0.0.1", "No projects yet"])

workdir = state / "project"
workdir.mkdir()
run("conversation", [
    key(0.8, "Escape"), typing(1, " cn"),
    key(1.3, "Enter", cmd=True),
    typing(1.5, "ui-proof"), key(1.7, "Tab"),
    typing(1.9, str(workdir)), key(2.1, "Tab"),
    key(2.3, "Enter"), typing(2.5, "y"),
    key(4, "w", alt=True), typing(4.2, "Faux echo"), key(4.4, "Enter"),
    typing(4.6, message), key(4.8, "Enter", cmd=True),
], 10, [f"echo: {message}", "agent", "ui-proof"])
