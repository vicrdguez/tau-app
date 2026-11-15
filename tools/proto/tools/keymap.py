"""Writes keymap.js from the package's keymap.toml: contexts, every binding (mode, context, sequence), ex names, builtins.
checks.js resolves printed chips against this table instead of a hand-written one. Run with python3 -B."""
import json, sys, tomllib
src, out = sys.argv[1], sys.argv[2]
k = tomllib.load(open(src, 'rb'))
binds, ex = [], {}
for c in k['command']:
    for b in c['keys']:
        binds.append({'mode': b['mode'], 'ctx': b['ctx'], 'seq': b['seq'], 'id': c['id'], 'tier': c.get('tier'), 'cctx': c.get('ctx', [])})  # cctx: where the command applies
    if c.get('ex'):
        ex[c['ex']] = {'id': c['id'], 'ctx': c.get('ctx', [])}
table = {'contexts': k['contexts'], 'binds': binds, 'ex': ex, 'builtin': k['builtin'], 'leader': k['meta']['leader']}
open(out, 'w').write('window.__keymap = ' + json.dumps(table) + ';\n')
print(f'keymap.js: {len(binds)} bindings, {len(ex)} ex names, {len(k["contexts"])} contexts')
