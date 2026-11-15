# Prototype

An HTML model of the desktop design in `../../design/`. It renders every state in `states.md` headlessly in Chromium and checks each render against the design's rules. It is a model of the layout and the rules, not the client.

Its inputs are the design's own files, read and never written: `fixture.json`, `keymap.toml` and `tokens.json`, plus the brand files in `../../design/brand/`. A build reproduces `../../design/mockups/` and `../../design/wireframes/` byte-identically. Change a design file, rebuild, and look at the diff.

## Run

Needs Node 20 or later, Python 3, and Chrome Headless Shell 156.0.8078.47.

```sh
npm install
npx @puppeteer/browsers install chrome-headless-shell@156.0.8078.47
npm run build      # every state to png/, the check results to png/checks.json (about 1 minute)
npm run build -- home start   # only these states
npm run wire       # the wireframes to png/wire/
npm run mutants    # seeds faults into renders; every one must be caught
```

The build looks for Chrome Headless Shell under `~/.cache/puppeteer/`. Set `CHROME_HEADLESS_SHELL` to the binary's path to use another copy.

The build generates `index.html`, `tokens.css` (from `tokens.json`), `fixture.js` (from `fixture.json`) and `keymap.js` (from `keymap.toml`, by `tools/keymap.py`). Don't edit those; they are git-ignored, like `png/`.

## What it checks

Each render must pass every family: CHIPS, RESOLVE, CORAL, CURSOR, PRESENCE, CONTRAST, COUNTS, MEASURE, SPOOF, DIM and IDS. `../../design/evidence.md` §1 says what each one fails on, and its results. The checks read only what a render shows: the mode from the badge, the region that has the keys, and visible pixels. They take nothing a view says about itself. `mutants.mjs` holds the seeded faults, at least one per family.

## The keymap lint

`tools/lint.py` checks `keymap.toml` against the grammar's rules. Run it from `../../design/`:

```sh
python3 -B ../tools/proto/tools/lint.py keymap.toml --protocol client.md --cheatsheet /tmp/cheatsheet.md
python3 -B tools/keymap_answer_rule.py --mutants
```

## Files

| File | What it is |
|---|---|
| `states.md` | the states to render, each with its window, theme, mode, focus, data and what it must show |
| `app.js`, `calm.css`, `index.tmpl.html` | the model of the frame and the views |
| `build.mjs` | generates the inputs, renders the states and the wireframes, runs `checks.js` |
| `checks.js` | the check families, run in the page |
| `mutants.mjs` | the seeded faults |
| `tools/keymap.py` | turns `keymap.toml` into the key table that RESOLVE and DIM use |
| `tools/lint.py` | the keymap lint |
| `vendor/fonts/` | the Iosevka and Iosevka Etoile weights the design uses (SIL OFL 1.1, `LICENSE.md`) |
