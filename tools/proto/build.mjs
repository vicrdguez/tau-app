#!/usr/bin/env node
// tau desktop prototype: builds index.html (base CSS from the Brief's style frame, Carbon sprite, tau brand symbols, the fixture)
// and renders the states of states.md headlessly to png/. Adapted from e10-modern/proto/build.mjs.
// Usage: node build.mjs [state ...]          (no args: every state; writes png/checks.json from checks.js)
//        node build.mjs --wire [state ...]   greyscale renders with each frame region outlined and labelled, to png/wire/
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, readdirSync, mkdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
// The design package and the brand in this repository, read only.
const DESIGN = join(HERE, '..', '..', 'design');
const BRAND = join(HERE, '..', '..', 'brand', 'svg');
const require = createRequire(join(HERE, 'package.json'));
const puppeteer = require('puppeteer-core');

const ICONS = ['list--checked', 'chat', 'time', 'folder', 'search', 'bot', 'user', 'document', 'edit', 'help', 'help--filled', 'subtract',
  'checkmark', 'stop-sign--filled', 'play--filled--alt', 'chevron--right', 'chevron--down', 'close', 'arrow--down', 'checkbox', 'checkbox--checked',
  'launch', 'warning--alt', 'locked', 'send', 'circle-dash', 'add', 'information', 'renew', 'home', 'settings', 'side-panel--close', 'side-panel--open', 'chevron--left'];
const CARBON = join(HERE, 'node_modules/@carbon/icons/svg/32');
const symbols = ICONS.map((n) => {
  const inner = readFileSync(join(CARBON, `${n}.svg`), 'utf8').replace(/^[\s\S]*?<svg[^>]*>/, '').replace(/<\/svg>\s*$/, '').replace(/<title>.*?<\/title>/g, '');
  return `<symbol id="${n}" viewBox="0 0 32 32">${inner}</symbol>`;
});
// The Brief's custom glyphs: the diamond (filled, outline), the drawn leader mark, the offline ring.
symbols.push('<symbol id="diamond" viewBox="0 0 16 16"><path d="M8 .7 15.3 8 8 15.3.7 8z"/></symbol>');
symbols.push('<symbol id="diamond-o" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M8 .7 15.3 8 8 15.3.7 8zm0 2.6L3.3 8 8 12.7 12.7 8z"/></symbol>');
symbols.push('<symbol id="space" viewBox="0 0 16 16"><path d="M1 8h2v3h10V8h2v5H1z"/></symbol>');
// tokens.json tau.icon.key_return: ⏎ drawn as a path, so no font fallback is involved
symbols.push('<symbol id="key-return" viewBox="0 0 16 16"><path d="M13 3h2v7H5.4l2.3 2.3-1.4 1.4L1.6 9l4.7-4.7 1.4 1.4L5.4 8H13z"/></symbol>');
symbols.push('<symbol id="ring" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M8 2.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zm0 1.5a4 4 0 1 1 0 8 4 4 0 0 1 0-8z"/></symbol>');
// The failed mark: Carbon's stop-sign--filled silhouette, redrawn to fill its box with square corners. Carbon's own glyph
// sits 3 px inside a 32 px box with rounded corners, and at 14 px it antialiases into a circle (looked at, 2026-11-15).
const si = symbols.findIndex((x) => x.startsWith('<symbol id="stop-sign--filled"'));
symbols[si] = '<symbol id="stop-sign--filled" viewBox="0 0 14 14"><path d="M4 0h6l4 4v6l-4 4H4L0 10V4z"/></symbol>';
// calm small marks: a heavy × (failed) and a solid triangle (running), drawn for 8-10 px
symbols.push('<symbol id="xmark" viewBox="0 0 10 10"><path d="M1.6 0 5 3.4 8.4 0 10 1.6 6.6 5 10 8.4 8.4 10 5 6.6 1.6 10 0 8.4 3.4 5 0 1.6z"/></symbol>');
symbols.push('<symbol id="tri" viewBox="0 0 10 10"><path d="M1 0 10 5 1 10z"/></symbol>');
symbols.push('<symbol id="dot" viewBox="0 0 16 16"><circle cx="8" cy="8" r="3.5"/></symbol>');
// tau brand, currentColor versions only (direction.md §7: inside the window the mark is always ink)
const brand = (file, id) => {
  const s = readFileSync(join(BRAND, file), 'utf8');
  const vb = s.match(/viewBox="([^"]+)"/)[1];
  const inner = s.replace(/^[\s\S]*?<svg[^>]*>/, '').replace(/<\/svg>\s*$/, '').replace(/<title>.*?<\/title>/g, '');
  return `<symbol id="${id}" viewBox="${vb}">${inner}</symbol>`;
};
symbols.push(brand('tau-mark-small.svg', 'tau-small'), brand('tau-wordmark.svg', 'tau-wordmark'));
const sprite = `<svg width="0" height="0" style="position:absolute" aria-hidden="true">${symbols.join('')}</svg>`;

// The fixture is read, never written: everything the screens show comes from it.
// TC-30: the CSS variables are generated from the package's tokens.json, all three variants, so CONTRAST measures the shipped tokens
{
  const T = JSON.parse(readFileSync(join(DESIGN, 'tokens.json'), 'utf8'));
  const MAP = { read: 'surface.read', side: 'surface.side', raised: 'surface.raised', bar: 'surface.bar', 'cursor-read': 'surface.cursor.read', 'cursor-side': 'surface.cursor.side',
    current: 'surface.current', code: 'surface.code', sunken: 'surface.sunken', sel: 'surface.selection', hl: 'border.hairline', hair: 'border.hairline', strong: 'border.strong',
    focus: 'rule.focus', leader: 'rule.note_leader', fg: 'fg.default', alt: 'fg.alt', muted: 'fg.muted', coral: 'state.attn', blocked: 'state.blocked', 'steel-mark': 'state.stale',
    unknown: 'state.unknown', 'sage-mark': 'state.local', olive: 'state.ok', 'olive-t': 'text.ok', rose: 'text.failed', h1: 'content.h1', h2: 'content.h2', strongc: 'content.strong',
    steel: 'kind.you', sage: 'kind.worker', orange: 'kind.literal', gold: 'kind.code', link: 'kind.link', 'link-active': 'kind.link_active', human: 'tint.human',
    'row-fail': 'tint.row_fail', 'note-range': 'tint.note_range', 'note-active': 'tint.note_active' };
  const get = (v, path) => { const [g, ...rest] = path.split('.'); return v[g][rest.join('.')]; };
  const block = (sel, v) => `${sel}{${Object.entries(MAP).map(([k, path]) => { const x = get(v, path); if (!x) throw new Error(`tokens.json has no ${path}`); return `--${k}:${x}`; }).join(';')}}`;
  const R = T.radius, Lt = T.tau.layout;
  const sizes = `:root{--r-pill:${R.pill}px;--r-surface:${R.surface}px;--tf-h:${Lt.text_field.height}px;--tf-px:${Lt.text_field.padding_x}px;--side-w:${Lt.sidebar.width}px;--peek-w:${Lt.peek.width}px;--details-w:${Lt.details.width}px}`;
  writeFileSync(join(HERE, 'tokens.css'), `/* generated by build.mjs from ${join(DESIGN, 'tokens.json')}; do not edit */\n${block(':root', T.variants.ember)}\n${block('html.soft', T.variants['ember-soft'])}\n${block('html.light', T.variants['ember-light'])}\n${sizes}\n`);
}
writeFileSync(join(HERE, 'fixture.js'), `window.FX = ${readFileSync(join(DESIGN, 'fixture.json'), 'utf8')};\n`);
// the key table for RESOLVE and CHIPS, generated from the package's keymap.toml
process.stdout.write(execFileSync('python3', ['-B', join(HERE, 'tools/keymap.py'), join(DESIGN, 'keymap.toml'), join(HERE, 'keymap.js')]));
const out = join(HERE, 'index.html');
writeFileSync(out, readFileSync(join(HERE, 'index.tmpl.html'), 'utf8').replace('<!--SPRITE-->', sprite));

const WIRE = process.argv[2] === '--wire';
const ARGS = process.argv.slice(WIRE ? 3 : 2);
const WIRE_STATES = ['home', 'conversation-streaming', 'decision-long', 'readback-answer'];
// Frame regions (direction.md §3) in paint order; later outlines draw over earlier ones.
const REGIONS = [['Sidebar', '.side', '#1f5fbf'], ['Stage header', '.shead', '#0f7a55'], ['Stage', '.sbody', '#b3261e'], ['Notice band', '.notice', '#00708a'], ['Peek', '.peek', '#8a3ab9'],
  ['Details panel', '.dpanel', '#7a6a00'], ['Bottom float', '.float.on', '#a35a00'], ['Bottom bar', '.status', '#6a3fd0']];
const wireOverlay = async (regions, shot) => {
  const W = innerWidth, H = innerHeight;
  const vis = (el) => { const r = el.getBoundingClientRect(); const cs = getComputedStyle(el); return r.width >= 1 && r.height >= 1 && r.bottom > 0 && r.top < H && cs.display !== 'none' && cs.visibility !== 'hidden'; };
  const onTop = (el, r) => { const h = document.elementFromPoint(Math.min(W - 1, Math.max(0, r.left + r.width / 2)), Math.min(H - 1, Math.max(0, r.top + r.height / 2))); return !!h && (h === el || el.contains(h) || h.contains(el)); };
  const obst = [];
  for (const el of document.querySelectorAll('#app *')) {
    if (el.closest('svg') && !el.matches('svg')) continue;
    const txt = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
    if (!(txt || el.matches('svg, .kbd, .tag, .seg, .lights i'))) continue;
    if (!vis(el)) continue;
    if (txt) { for (const n of el.childNodes) if (n.nodeType === 3 && n.textContent.trim()) { const rg = document.createRange(); rg.selectNodeContents(n); for (const r of rg.getClientRects()) if (onTop(el, r)) obst.push(r); } }
    else { const r = el.getBoundingClientRect(); if (onTop(el, r)) obst.push(r); }
  }
  const img = new Image(); img.src = shot; await img.decode();
  const cv = document.createElement('canvas'); cv.width = img.width; cv.height = img.height;
  const cx = cv.getContext('2d'); cx.drawImage(img, 0, 0);
  const px = cx.getImageData(0, 0, cv.width, cv.height), d = px.data;
  for (let i = 0; i < d.length; i += 4) d[i] = d[i + 1] = d[i + 2] = Math.round(0.2126 * d[i] + 0.7152 * d[i + 1] + 0.0722 * d[i + 2]);
  cx.putImageData(px, 0, 0);
  cv.style.cssText = `position:fixed;left:0;top:0;width:${W}px;height:${H}px;z-index:99998`;
  document.body.appendChild(cv);
  const layer = document.createElement('div');
  layer.style.cssText = 'position:fixed;inset:0;z-index:99999;pointer-events:none;font:600 13px/18px Iosevka, monospace';
  document.body.appendChild(layer);
  const hit = (a, b) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;
  const area = (a, b) => Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left)) * Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top));
  const placed = [], found = [];
  for (const [name, sel] of regions) {
    const el = [...document.querySelectorAll(sel)].find(vis); if (!el) continue;
    const r = el.getBoundingClientRect();
    found.push({ name, x: Math.round(r.left), y: Math.round(r.top), w: Math.round(r.width), h: Math.round(Math.min(r.bottom, H) - r.top), r });
  }
  for (const f of found) {
    const box = document.createElement('div');
    box.style.cssText = `position:absolute;left:${f.x}px;top:${f.y}px;width:${f.w}px;height:${f.h}px;box-sizing:border-box;border:2px solid ${regions.find((x) => x[0] === f.name)[2]}`;
    layer.appendChild(box);
  }
  for (const f of found) {
    const col = regions.find((x) => x[0] === f.name)[2];
    const lab = document.createElement('div');
    lab.textContent = `${f.name}  x ${f.x}  y ${f.y}  w ${f.w}  h ${f.h}`;
    lab.style.cssText = `position:absolute;white-space:nowrap;padding:1px 7px;background:${col};color:#fff;box-shadow:0 0 0 1px rgba(255,255,255,.85)`;
    layer.appendChild(lab);
    const lw = lab.offsetWidth, lh = lab.offsetHeight;
    const covers = found.filter((o) => o !== f && found.indexOf(o) > found.indexOf(f) && hit(o.r, f.r)).map((o) => o.r);
    const avoid = [...obst.filter((o) => hit(o, f.r)), ...covers, ...placed];
    const inner = { l: f.x + 6, r: f.x + f.w - 6 - lw };
    const ys = f.h < 50 ? [Math.round(f.y + (f.h - lh) / 2)] : [];
    if (f.h >= 50) { for (let y = f.y + f.h - lh - 8; y >= f.y + 6; y -= 6) ys.push(y); }
    let best = null;
    for (const y of ys) {
      for (let x = Math.round(inner.r); x >= inner.l; x -= 6) {
        const c = { left: x - 12, top: y - 3, right: x + lw + 12, bottom: y + lh + 3 };
        const cost = avoid.reduce((s, o) => s + area(c, o), 0);
        if (!best || cost < best.cost) best = { x, y, cost };
        if (cost === 0) break;
      }
      if (best && best.cost === 0) break;
    }
    lab.style.left = best.x + 'px'; lab.style.top = best.y + 'px';
    placed.push({ left: best.x - 4, top: best.y - 4, right: best.x + lw + 4, bottom: best.y + lh + 4 });
    f.cost = best.cost;
  }
  return found.map(({ name, x, y, w, h, cost }) => ({ name, x, y, w, h, overlap: cost }));
};

const checksSrc = readFileSync(join(HERE, 'checks.js'), 'utf8');
const shell = process.env.CHROME_HEADLESS_SHELL || (() => {
  const shellRoot = join(homedir(), '.cache/puppeteer/chrome-headless-shell');
  const ver = readdirSync(shellRoot)[0];
  const sub = readdirSync(join(shellRoot, ver)).find((d) => d.startsWith('chrome-headless-shell'));
  return join(shellRoot, ver, sub, 'chrome-headless-shell');
})();
const browser = await puppeteer.launch({ executablePath: shell, headless: 'shell', args: ['--no-sandbox', '--disable-gpu', '--force-color-profile=srgb', '--allow-file-access-from-files'] });
mkdirSync(join(HERE, WIRE ? 'png/wire' : 'png'), { recursive: true });
const results = {};

async function open(name, width) {
  const page = await browser.newPage();
  page.on('pageerror', (e) => console.log(name, 'PAGEERROR', e.message));
  page.on('console', (m) => { if (m.type() === 'error' || m.type() === 'warn') console.log(name, 'console', m.text()); });
  await page.setViewport({ width, height: 982, deviceScaleFactor: 1 });
  // one retry: a busy machine has timed out a navigation once (2026-11-15)
  for (let i = 0; ; i++) { try { await page.goto(pathToFileURL(out).href + `#${name}`, { waitUntil: 'networkidle0', timeout: 60000 }); break; } catch (e) { if (i) throw e; console.log(name, 'retrying after', e.message); } }
  await page.waitForFunction('window.__ready === true');
  await page.evaluate(() => document.fonts.ready);
  const fonts = await page.evaluate(async () => { const f = ['400 13px Etoile', '600 13px Etoile', '700 22px Etoile', '400 13px Iosevka', '600 11px Iosevka']; await Promise.all(f.map((x) => document.fonts.load(x))); return f.every((x) => document.fonts.check(x)); });
  return { page, fonts };
}
const line = (name, r, fonts) => {
  const c = r.coral;
  return `${name.padEnd(27)} ${fonts ? '' : 'FONTS MISSING '}chips ${r.chips.stage}/${r.chips.side}/${r.chips.status}${r.chips.fails.length ? ' FAIL ' + r.chips.fails.join(',') : ''}` +
    ` | resolve ${r.resolve.judged}j/${r.resolve.skipped}s ${r.resolve.fails.length ? 'FAIL ' + r.resolve.fails.join(',') : 'ok'}` +
    ` | coral ${c.masses.map((m) => m.what).join('+') || '-'}${c.fails.length ? ' FAIL ' + c.fails.join('; ') : ''}` +
    ` | cursor ${r.cursor.focus}${r.cursor.fails.length ? ' FAIL ' + r.cursor.fails.join('; ') : ''}` +
    ` | contrast ${r.contrast.text}t/${r.contrast.marks}m${r.contrast.fails.length ? ' FAIL ' + r.contrast.fails.length : ''}` +
    ` | counts ${r.counts.checked}${r.counts.fails.length ? ' FAIL ' + r.counts.fails.join('; ') : ''}` +
    `${r.measure ? ` | measure ${r.measure.maxWidth}px ${r.measure.maxChars}ch${r.measure.fails.length ? ' FAIL' : ''}` : ''}` +
    ` | spoof ${r.spoof.items}${r.spoof.fails.length ? ' FAIL ' + r.spoof.fails.join('; ') : ''}` +
    ` | ids ${r.ids.fails.length ? 'FAIL ' + r.ids.fails.join(',') : 'ok'}` +
    ` | dim ${r.dim.applies ? (r.dim.fails.length ? 'FAIL ' + r.dim.fails.join(',') : 'ok') : '-'}`;
};
try {
  const probe = await browser.newPage();
  await probe.goto(pathToFileURL(out).href, { waitUntil: 'networkidle0' });
  await probe.waitForFunction('window.__ready === true');
  const all = await probe.evaluate(() => window.__states);
  const meta = await probe.evaluate(() => window.__meta);
  await probe.close();
  const jobs = ARGS.length ? ARGS : WIRE ? WIRE_STATES : all;
  for (const name of jobs) {
    const m = meta[name] || {};
    if (m.sheet) { // several frames rendered side by side into one PNG
      const shots = [];
      for (const sub of m.sheet) {
        const { page, fonts } = await open(sub, m.width);
        shots.push((await page.screenshot({ encoding: 'base64' })));
        const r = await page.evaluate(`(${checksSrc})()`);
        results[sub] = r; console.log(line(sub, r, fonts));
        await page.close();
      }
      const sheet = await browser.newPage();
      const gap = 24, W = m.width * shots.length + gap * (shots.length + 1);
      await sheet.setViewport({ width: W, height: 982 + gap * 2 + 28, deviceScaleFactor: 1 });
      await sheet.setContent(`<body style="margin:0;background:#0b0b0a;font:500 13px/28px Menlo,monospace;color:#999489">${shots.map((s, i) => `<div style="position:absolute;left:${gap + i * (m.width + gap)}px;top:${gap - 4}px">${m.labels[i]}</div><img style="position:absolute;left:${gap + i * (m.width + gap)}px;top:${gap + 24}px" src="data:image/png;base64,${s}">`).join('')}</body>`);
      await sheet.screenshot({ path: join(HERE, 'png', `${name}.png`) });
      await sheet.close();
      continue;
    }
    const { page, fonts } = await open(name, m.width || 1512);
    if (WIRE) {
      const shot = 'data:image/png;base64,' + (await page.screenshot({ encoding: 'base64' }));
      const regs = await page.evaluate(`(${wireOverlay})(${JSON.stringify(REGIONS)}, ${JSON.stringify(shot)})`);
      await page.screenshot({ path: join(HERE, 'png', 'wire', `wire-${name}.png`) });
      console.log(name.padEnd(26), regs.map((g) => `${g.name} ${g.x},${g.y},${g.w},${g.h}${g.overlap ? ` (label overlaps ${Math.round(g.overlap)} px²)` : ''}`).join(' | '));
      await page.close(); continue;
    }
    await page.screenshot({ path: join(HERE, 'png', `${name}.png`) });
    const r = await page.evaluate(`(${checksSrc})()`);
    results[name] = r;
    console.log(line(name, r, fonts));
    await page.close();
  }
} finally {
  await browser.close();
}
if (WIRE) process.exit(0);
let prev = {}; try { prev = JSON.parse(readFileSync(join(HERE, 'png', 'checks.json'), 'utf8')); } catch {}
writeFileSync(join(HERE, 'png', 'checks.json'), JSON.stringify({ ...prev, ...results }, null, 1));
