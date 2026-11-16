// Seeded faults: every check family must fail on its mutant, and pass on the unmutated state.
import { createRequire } from 'node:module';
import { homedir } from 'node:os';
import { join, dirname } from 'node:path';
import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
const HERE = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(HERE, 'package.json'));
const puppeteer = require('puppeteer-core');
const shellPath = process.env.CHROME_HEADLESS_SHELL || (() => { const root = join(homedir(), '.cache/puppeteer/chrome-headless-shell'); const v = readdirSync(root)[0]; const s = readdirSync(join(root, v)).find((d) => d.startsWith('chrome-headless-shell')); return join(root, v, s, 'chrome-headless-shell'); })();
const b = await puppeteer.launch({ executablePath: shellPath, headless: 'shell', args: ['--no-sandbox', '--disable-gpu', '--allow-file-access-from-files'] });
const checks = readFileSync(join(HERE, 'checks.js'), 'utf8');
const muted = (sel) => `document.querySelectorAll('${sel}').forEach((e) => e.style.setProperty('color', 'rgb(216,208,192)', 'important'))`;
const cases = [
  // CHIPS
  ['chips: a fourth chip in the stage', 'decision-long', "document.querySelector('.mhd').insertAdjacentHTML('beforeend', '<span class=\"kbd\" data-key=\"␣mn\">␣mn</span><span class=\"kbd\" data-key=\"␣mw\">␣mw</span><span class=\"kbd\" data-key=\"]n\">]n</span>')", (r) => r.chips.fails.length > 0],
  ['chips: a key the grammar does not print (␣ot on Activity)', 'home', "document.querySelector('.since').insertAdjacentHTML('beforeend', '<span class=\"kbd\" data-key=\"␣ot\">␣ot</span>')", (r) => r.chips.fails.length > 0],
  ['chips: a chip in the echo', 'server-stopped', "document.querySelector('.status .echo').insertAdjacentHTML('beforeend', '<span class=\"kbd\" data-key=\"␣rc\">␣rc</span>')", (r) => r.chips.fails.length > 0],
  ['chips: a third chip in the sidebar', 'home', "document.querySelectorAll('.side .place')[3].insertAdjacentHTML('beforeend', '<span class=\"kbd\" data-key=\"␣oh\">␣oh</span>')", (r) => r.chips.fails.length > 0],
  ['chips: the sidebar loses its ␣. chip', 'timeline', "document.querySelector('.side .kbd[data-key=\"␣.\"]').remove()", (r) => r.chips.fails.length > 0],
  ['chips: a hint printed where the grammar prints none (␣ot on the since-line, probe P13)', 'home', "document.querySelector('.since').insertAdjacentHTML('beforeend', '<span class=\"hint\" data-key=\"␣ot\">␣ot</span>')", (r) => r.chips.fails.length > 0],
  ['chips: y printed in the Connect form (probe P5, TC-07)', 'connect', "document.querySelector('.connect .go').insertAdjacentHTML('beforeend', '<span class=\"seg\"><span class=\"kbd\" data-key=\"y\">y</span><span>goes on</span></span>')", (r) => r.chips.fails.length > 0 && r.resolve.fails.length > 0],
  // RESOLVE
  ['resolve: ␣cx on a failed run', 'conversation-failed', "document.querySelector('.shead .seg .kbd').dataset.key = '␣cx'", (r) => r.resolve.fails.length > 0],
  ['resolve: ␣ad on an answered decision', 'decision-then-faulted', "document.querySelector('.shead .seg .kbd').dataset.key = '␣ad'", (r) => r.resolve.fails.length > 0],
  ['resolve: y printed in the which-key float', 'which-key-root', "document.querySelector('.float .hints').insertAdjacentHTML('beforeend', '<span class=\"kbd\" data-key=\"y\">y</span>')", (r) => r.resolve.fails.length > 0],
  ['resolve: ␣ad dropped from the keymap table', 'decision-long', "window.__keymap.binds = window.__keymap.binds.filter((b) => b.seq !== '<leader>ad')", (r) => r.resolve.fails.length > 0],
  ['resolve: the picker loses ⏎ (the bind checklist and the project picker judge it from the badge)', 'bind-float', "window.__keymap.binds = window.__keymap.binds.filter((b) => !(b.ctx === 'picker' && b.seq === '<CR>'))", (r) => r.resolve.fails.length > 0],
  ['resolve: a hint in the focused composer that no key in its context binds (Tab under steer)', 'conversation-steer-insert', "document.querySelector('.comp .ft').insertAdjacentHTML('beforeend', '<span class=\"hint on\" data-key=\"C-l\">C-l</span>')", (r) => r.resolve.fails.length > 0],
  ['resolve: M-p dropped from the start page\'s Insert keys', 'start', "window.__keymap.binds = window.__keymap.binds.filter((b) => !(b.mode === 'i' && b.seq === '<M-p>'))", (r) => r.resolve.fails.length > 0],
  ['resolve: the start composer\'s ⌘⏎ hint unbound', 'start', "window.__keymap.binds = window.__keymap.binds.filter((b) => !(b.ctx === 'start' && b.seq === '<D-CR>'))", (r) => r.resolve.fails.length > 0],
  ['resolve: an M-p hint printed in the conversation composer, where only the start editor has it (probe P3, TC-09)', 'conversation-steer-insert', "document.querySelector('.comp .ft').insertAdjacentHTML('beforeend', '<span class=\"hint on\" data-key=\"M-p\">M-p</span>')", (r) => r.resolve.fails.length > 0 && r.chips.fails.length > 0],
  ['resolve: M-w and Tab hints in the note being typed (probe P3b)', 'decision-note-insert', "document.querySelector('.note.act').insertAdjacentHTML('beforeend', '<span class=\"hint on\" data-key=\"M-w\">M-w</span><span class=\"hint on\" data-key=\"Tab\">Tab</span>')", (r) => r.resolve.fails.length > 0 && r.chips.fails.length > 0],
  // CORAL
  ['coral: a primary on a calm screen', 'home-calm', "document.querySelector('.lcol .h1').insertAdjacentHTML('afterend', '<span style=\"display:block;width:120px;height:26px;background:var(--coral)\"></span>')", (r) => r.coral.fails.length > 0],
  ['coral: a second solid mass beside the needs-you head', 'home', "document.querySelector('.since').style.background = 'var(--coral)'", (r) => r.coral.fails.length > 0],
  ['coral: agent text drawn coral', 'conversation-streaming', "document.querySelector('.msg [data-content] p').style.color = 'var(--coral)'", (r) => r.coral.fails.length > 0],
  ['coral: row text drawn coral', 'conversations', "document.querySelector('.lrow .tx[data-spoof]').style.color = 'var(--coral)'", (r) => r.coral.fails.length > 0],
  ['coral: coral text outside a page title (the breadcrumb)', 'project', "document.querySelector('.shead .crumb span').style.color = 'var(--coral)'", (r) => r.coral.fails.length > 0],
  // CURSOR
  ['cursor: two coral cursors (the sidebar current item gets the bar)', 'home', "document.querySelector('.side .row.sel').style.boxShadow = 'inset 2px 0 0 var(--coral)'", (r) => r.cursor.fails.length > 0],
  ['cursor: the stage keeps its coral head while the sidebar has the keys', 'sidebar-focus', "document.querySelector('.rw.need.sel .l1').style.background = 'var(--coral)'", (r) => r.cursor.fails.length > 0],
  ['cursor: no coral cursor (the last entry loses its bar)', 'conversation-streaming', "document.querySelector('.cur.keys').style.setProperty('box-shadow', 'none', 'important')", (r) => r.cursor.fails.length > 0],
  ['cursor: the float has the keys but the stage draws the coral bar', 'unknown-outcome', "document.querySelector('.tool.cur').style.setProperty('box-shadow', 'inset 2px 0 0 var(--coral)', 'important')", (r) => r.cursor.fails.length > 0],
  ['cursor: a clipped bar does not count (the cursor row pushed out of the column)', 'conversation-streaming', "const t = document.querySelector('.tool.cur'); t.style.marginLeft = '-700px'", (r) => r.cursor.fails.length > 0],
  ['cursor: a float with no selection, and the stage keeps its coral bar (TC-05)', 'readback-answer', "document.querySelectorAll('.float .prow.sel').forEach((e) => e.classList.remove('sel')); document.querySelector('.curbar').style.background = 'var(--coral)'", (r) => r.cursor.fails.length > 0],
  ['cursor: a coral caret left in the start composer under the project picker (probe P10)', 'start-project-picker', "document.querySelector('.comp .cb').insertAdjacentHTML('afterbegin', '<span class=\"bar\"></span>')", (r) => r.cursor.fails.length > 0],
  ['cursor: a coral caret in the sidebar while the stage has the keys (probe P10b)', 'home', "document.querySelector('.side .row').insertAdjacentHTML('beforeend', '<span class=\"bar\" style=\"display:inline-block;width:2px;height:16px;background:var(--coral)\"></span>')", (r) => r.cursor.fails.length > 0],
  ['coral: a 2 px coral edge on agent text (probe P16)', 'conversation-streaming', "document.querySelector('.msg [data-content] p').style.borderLeft = '2px solid var(--coral)'", (r) => r.coral.fails.length > 0],
  ['cursor: a caret left in the read-only start composer while create is in flight', 'start-starting', "document.querySelector('.start .comp .cb').insertAdjacentHTML('beforeend', '<span class=\"bar\"></span>')", (r) => r.cursor.fails.length > 0],
  ['cursor: Connect\'s code field keeps its coral edge and caret under the Confirm float', 'connect-http-warning', "const f = [...document.querySelectorAll('.connect .fld')].pop(); f.classList.add('focus'); f.insertAdjacentHTML('beforeend', '<span class=\"bar\"></span>')", (r) => r.cursor.fails.length > 0],
  ['cursor: the sidebar has the keys but the decision\'s document cursor stays coral', 'sidebar-focus-decision', "document.querySelector('.curbar').style.background = 'var(--coral)'", (r) => r.cursor.fails.length > 0],
  // CONTRAST
  ['contrast: muted text dropped a step', 'home', "document.querySelector('.since').style.color = '#5a564e'", (r) => r.contrast.fails.length > 0],
  ['contrast: a faint mark', 'faults', "document.querySelector('.lrow .mk .sm').style.color = '#3a3632'", (r) => r.contrast.fails.length > 0],
  ['contrast: Light composer placeholder too pale', 'start-light', "document.querySelector('.comp').insertAdjacentHTML('beforeend', '<span style=\"color:#c8bca4\">pale</span>')", (r) => r.contrast.fails.length > 0],
  // COUNTS
  ['counts: sidebar tag disagrees with the title tag', 'home', "document.querySelector('.side .tags [data-count=needs]').textContent = '4'", (r) => r.counts.fails.length > 0],
  ['counts: sidebar × tag disagrees with the Failed head', 'home', "document.querySelector('.side .tags [data-count=failed]').textContent = '5'", (r) => r.counts.fails.length > 0],
  ['counts: Failed head disagrees with its rows', 'home', "document.querySelectorAll('.rw.failed')[3].remove()", (r) => r.counts.fails.length > 0],
  ['counts: readback note count off by one', 'readback-answer', "document.querySelector('.float [data-count=notes]').textContent = '2'", (r) => r.counts.fails.length > 0],
  ['counts: the bottom bar\'s × tag off with the sidebar hidden', 'narrow-home', "document.querySelector('.status .tags [data-count=failed]').textContent = '3'", (r) => r.counts.fails.length > 0, 800],
  ['counts presence: the bottom bar loses its tags with the sidebar hidden (probe P1, FM9)', 'narrow-home', "document.querySelectorAll('.status .tags').forEach((e) => e.remove())", (r) => r.counts.fails.length > 0, 800],
  ['counts presence: the bottom bar loses the server\'s words with the sidebar hidden (probe P2, FC15)', 'server-stopped', "document.querySelectorAll('.status .srvb').forEach((e) => e.remove())", (r) => r.counts.fails.length > 0],
  ['counts presence: the bottom bar loses its tags in server-stopped (probe P2b)', 'server-stopped', "document.querySelectorAll('.status .tags').forEach((e) => e.remove())", (r) => r.counts.fails.length > 0],
  ['counts presence: the sidebar loses its tags (probe P6)', 'home', "document.querySelectorAll('.side .tags').forEach((e) => e.remove())", (r) => r.counts.fails.length > 0],
  ['counts presence: Home\'s title loses its count tag (probe P15)', 'home', "document.querySelectorAll('.h1 .ttag').forEach((e) => e.remove())", (r) => r.counts.fails.length > 0],
  ['counts presence: the server\'s words shown twice', 'offline', "document.querySelector('.status').insertAdjacentHTML('beforeend', '<span class=\"srvb\">offline 6 m</span>')", (r) => r.counts.fails.length > 0],
  ['presence: a ◆ tag drawn before anything arrived (home-connecting)', 'home-connecting', "document.querySelector('.side .place .rt, .side .tags').insertAdjacentHTML('beforeend', '<span class=\"tag t-needs\" data-tag=\"needs\"><span data-count=\"needs\">3</span></span>')", (r) => r.counts.fails.length > 0],
  ['presence: ○ connecting removed from the server line (home-connecting)', 'home-connecting', "document.querySelector('.side .srv').innerHTML = 'laptop → tau.tail4e2f.ts.net'", (r) => r.counts.fails.length > 0],
  ['presence: the hostile inbox\'s fourth decision missing from the ◆ tag', 'conversation-hostile', "document.querySelector('.side .tags [data-count=needs]').textContent = '3'", (r) => r.counts.fails.length > 0],
  // MEASURE
  ['measure: text column widened to 760 px', 'decision-long', "document.querySelector('.wpage .doc').style.width = '760px'", (r) => r.measure && r.measure.fails.length > 0],
  ['measure: conversation prose widened to 700 px', 'conversation-streaming', "document.querySelectorAll('.ccol .prose').forEach((e) => e.style.maxWidth = '700px'); document.querySelector('.ccol').style.width = '760px'", (r) => r.measure && r.measure.fails.length > 0],
  // SPOOF
  ['spoof: a row label rendered as Markdown', 'conversations', "const t = document.querySelector('.lrow .tx[data-spoof]'); t.innerHTML = t.textContent.replace(/([^ ]+)/, '<code>$1</code>')", (r) => r.spoof.fails.length > 0],
  ['spoof: event text wraps to two lines', 'home', "const t = document.querySelector('.rw.ev .tx'); t.style.whiteSpace = 'normal'; t.style.width = '80px'", (r) => r.spoof.fails.length > 0],
  ['spoof: a title in the badge face', 'home', "const t = document.querySelector('.rw.need .tx'); t.style.font = '600 11px Iosevka'; t.style.textTransform = 'uppercase'", (r) => r.spoof.fails.length > 0],
  ['spoof: #id moved before the text', 'home', "const r = document.querySelector('.rw.need .l1'); r.insertBefore(r.querySelector('.tg'), r.querySelector('.tx'))", (r) => r.spoof.fails.length > 0],
  ['spoof: a sidebar label set in bold', 'home', "document.querySelector('.side .lb[data-spoof]').style.fontWeight = '700'", (r) => r.spoof.fails.length > 0],
  ['spoof: the content h1 loses its rule', 'decision-long', "document.querySelector('.md h1').style.borderBottom = 'none'", (r) => r.spoof.fails.length > 0],
  ['spoof: a second page title in the stage', 'decision-long', "document.querySelector('.md p').insertAdjacentHTML('beforebegin', '<div class=\"ptitle\">Approve now</div>')", (r) => r.spoof.fails.length > 0],
  ['spoof: a failed tint without the word failed (probe P17)', 'conversation-streaming', "const t = document.querySelector('.tool:not(.fail)'); t.style.background = 'var(--row-fail)'", (r) => r.spoof.fails.length > 0],
  ['spoof: a hostile title rendered as Markdown in a sidebar row (FS1)', 'conversation-hostile', "const t = [...document.querySelectorAll('.side .lb[data-spoof]')].find((e) => e.textContent.includes('Needs you · ok')); t.innerHTML = '<b>◆ Needs you</b> · ok'", (r) => r.spoof.fails.length > 0],
  ['spoof: a hostile # Needs you title set as a heading (FS6)', 'conversation-hostile', "const t = document.querySelector('.ptitle'); t.innerHTML = '<h1 class=\"md\">Needs you</h1>'", (r) => r.spoof.fails.length > 0 || r.coral.fails.length > 0],
  ['spoof: a bidi control drawn raw instead of escaped, in a row (FS4)', 'conversation-hostile', "document.querySelector('.side .lb[data-spoof]').textContent = 'ok\\u202Ehtap\\x07'", (r) => r.spoof.fails.length > 0],
  // DIM
  ['dim: a live header segment under the readback', 'readback-answer', "document.querySelectorAll('.shead .seg, .shead .kbd').forEach((e) => e.style.setProperty('color', 'rgb(216,208,192)', 'important'))", (r) => r.dim.fails.length > 0],
  ['dim: ⏎ stays lit under which-key', 'which-key-root', "const k = document.querySelector('.rw.need.sel .kbd'); k.style.setProperty('display', 'inline-flex', 'important'); k.style.setProperty('color', 'rgb(216,208,192)', 'important')", (r) => r.dim.fails.length > 0],
  ['dim: header chip lit in Insert', 'conversation-steer-insert', "document.querySelectorAll('.shead .seg, .shead .kbd').forEach((e) => e.style.setProperty('color', 'rgb(216,208,192)', 'important'))", (r) => r.dim.fails.length > 0],
  ['dim: the Answer keeps its coral fill while a note is typed', 'decision-note-insert', "document.querySelector('.shead .seg.prim').style.setProperty('background', 'var(--coral)', 'important')", (r) => r.dim.fails.length > 0],
  ['dim: a sidebar chip lit under a picker', 'start-project-picker', "document.querySelector('.side .kbd').style.setProperty('color', 'rgb(192,144,88)', 'important')", (r) => r.dim.fails.length > 0],
  ['dim: a sidebar chip lit while the composer has the keys (Insert)', 'conversation-steer-insert', "document.querySelector('.side .kbd').style.setProperty('color', 'rgb(192,144,88)', 'important')", (r) => r.dim.fails.length > 0],
  ['dim: a header chip muted while it is live', 'conversation-streaming', "document.querySelector('.shead .kbd').style.setProperty('color', 'var(--muted)', 'important')", (r) => r.dim.fails.length > 0],
  ['dim: ⌘⏎ still lit while create is in flight (start-starting)', 'start-starting', "document.querySelector('.start .comp .ft').innerHTML = '<span class=\"hint on\" data-key=\"⌘⏎\">⌘⏎</span><span>start</span>'", (r) => r.dim.fails.length > 0],
  ['dim: M-p and M-w lit while create is in flight', 'start-starting', "document.querySelectorAll('.startline .hint').forEach((h) => h.classList.add('on'))", (r) => r.dim.fails.length > 0],
  ['dim: the Answer solid coral while the sidebar has the keys (TC-04)', 'sidebar-focus-decision', "const p = document.querySelector('.shead .seg.prim'); p.style.setProperty('background', 'var(--coral)', 'important')", (r) => r.dim.fails.length > 0],
  ['dim: ␣cn lit under the outbox (floats are root contexts)', 'unknown-outcome', "document.querySelector('.side .kbd').style.setProperty('color', 'rgb(192,144,88)', 'important')", (r) => r.dim.fails.length > 0],
  ['dim: the container\'s own hints drawn muted while it has the keys', 'start', "document.querySelectorAll('.startline .hint').forEach((h) => h.classList.remove('on'))", (r) => r.dim.fails.length > 0],
  // IDS
  ['ids: a #id cut', 'home', "const t = document.querySelector('.rw.need .tg'); t.style.width = '6px'; t.style.overflow = 'hidden'", (r) => r.ids.fails.length > 0],
  ['ids: the after <decision ID> span cut in a Failed row (TC-15)', 'home', "const a = document.querySelector('.rw.failed .aft'); a.style.flex = '0 1 auto'; a.style.minWidth = '0'; a.style.maxWidth = '60px'; a.style.overflow = 'hidden'", (r) => r.ids.fails.length > 0],
  ['ids: a sidebar #id cut', 'home', "const t = document.querySelector('.side .row.it .id'); t.style.width = '6px'; t.style.overflow = 'hidden'", (r) => r.ids.fails.length > 0],
];
const fam = (r) => ({ chips: r.chips.fails.length, resolve: r.resolve.fails.length, coral: r.coral.fails.length, cursor: r.cursor.fails.length, contrast: r.contrast.fails.length, counts: r.counts.fails.length, measure: r.measure ? r.measure.fails.length : 0, spoof: r.spoof.fails.length, dim: r.dim.fails.length, ids: r.ids.fails.length });
let caught = 0;
for (const [what, state, inject, hit, width] of cases) {
  const page = await b.newPage(); await page.setViewport({ width: width || 1512, height: 982 });
  await page.goto(pathToFileURL(join(HERE, 'index.html')).href + '#' + state, { waitUntil: 'networkidle0' });
  await page.waitForFunction('window.__ready === true');
  const base = await page.evaluate(`(${checks})()`);
  const clean = !hit(base);
  try { await page.evaluate(inject); } catch (e) { console.log(`INJECT ERROR  ${what}: ${e.message.slice(0, 100)}`); }
  const r = await page.evaluate(`(${checks})()`);
  const ok = clean && hit(r); if (ok) caught++;
  await page.close();
  console.log(`${ok ? 'CAUGHT' : clean ? 'MISSED' : 'BASE FAILS'}  ${what}  [${state}]`);
}
console.log(`${caught}/${cases.length} caught`);
await b.close();
