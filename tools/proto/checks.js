() => {
  // Render checks for the tau prototype, calm frame (e13, phase 2: the package's calm text), run in the page after it is drawn.
  // Families follow states.md "Checks to run on every render". Adapted from e11-tau/proto/checks.js.
  const W = innerWidth, H = innerHeight;
  const CTX = window.__ctx || {};
  const parse = (s) => { const m = String(s).match(/rgba?\(([^)]+)\)/); if (!m) return null; const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number); return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 }; };
  const hexc = (h) => { h = h.trim().replace('#', ''); return { r: parseInt(h.slice(0, 2), 16), g: parseInt(h.slice(2, 4), 16), b: parseInt(h.slice(4, 6), 16), a: 1 }; };
  const lum = ({ r, g, b }) => { const f = (c) => { c /= 255; return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; }; return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b); };
  const ratio = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
  const over = (top, under) => ({ r: top.r * top.a + under.r * (1 - top.a), g: top.g * top.a + under.g * (1 - top.a), b: top.b * top.a + under.b * (1 - top.a), a: 1 });
  const hex = (c) => '#' + [c.r, c.g, c.b].map((v) => Math.round(v).toString(16).padStart(2, '0')).join('');
  const near = (a, b, t = 6) => !!a && !!b && Math.abs(a.r - b.r) + Math.abs(a.g - b.g) + Math.abs(a.b - b.b) < t;
  const shown = (el) => { const r = el.getBoundingClientRect(); if (r.width < 1 || r.height < 1 || r.bottom <= 0 || r.top >= H || r.right <= 0 || r.left >= W) return false; const cs = getComputedStyle(el); return cs.visibility !== 'hidden' && cs.display !== 'none' && +cs.opacity > 0; };
  const onTop = (el) => { const r = el.getBoundingClientRect(); const x = Math.min(W - 1, Math.max(0, r.left + Math.min(r.width / 2, 6))), y = Math.min(H - 1, Math.max(0, r.top + r.height / 2)); const hit = document.elementFromPoint(x, y); return !!hit && (hit === el || el.contains(hit) || hit.contains(el) || (hit.closest('svg') && el.contains(hit.closest('svg')))); };
  const root = getComputedStyle(document.documentElement);
  const TOK = (n) => hexc(root.getPropertyValue(n));
  const CORAL = TOK('--coral'), MUTED = TOK('--muted');
  // the surface an element sits on: its own fill, else what lies beneath it on screen
  function bgOf(el, under) {
    const r = el.getBoundingClientRect();
    const x = Math.min(W - 1, Math.max(0, r.left + Math.min(r.width / 2, 5))), y = Math.min(H - 1, Math.max(0, r.top + r.height / 2));
    const chain = [...(under ? [] : [el]), ...document.elementsFromPoint(x, y).filter((e) => e !== el && !el.contains(e) && !e.closest('svg'))];
    const layers = [];
    for (const e of chain) { const c = parse(getComputedStyle(e).backgroundColor); if (c && c.a > 0) { layers.push(c); if (c.a >= 1) break; } }
    let base = parse(getComputedStyle(document.body).backgroundColor) || TOK('--read');
    for (let i = layers.length - 1; i >= 0; i--) base = layers[i].a >= 1 ? layers[i] : over(layers[i], base);
    return base;
  }
  const chips = [...document.querySelectorAll('.kbd')].filter((c) => shown(c) && onTop(c));
  const keyOf = (c) => c.dataset.key || c.textContent.trim();
  const inFloat = (el) => !!el.closest('.float');
  const isMuted = (el) => near(parse(getComputedStyle(el).color), MUTED);

  // ── THE KEYS, as the render shows them (challenge TC-14): the badge gives the mode, the pending keys the prefix, and the region
  // that has the keys is the open float (unless pending keys show which-key, which takes none), the focused container in Insert,
  // the sidebar when its cursor is the coral one, else the stage. Each region's context comes from what it draws, never from a
  // label the view declares.
  const MODES = { NORMAL: 'n', INSERT: 'i', COMMAND: 'c', READBACK: 'rb', VISUAL: 'v', OUTBOX: 'ob', SERVER: 'sv', PEEK: 'ho', HINT: 'h' };
  const badgeEl = document.querySelector('.status .mode');
  const MODE = MODES[(badgeEl && badgeEl.textContent.trim()) || 'NORMAL'] || 'n';
  const pendEl = document.querySelector('.status .showcmd');
  const PEND = pendEl && shown(pendEl) ? (pendEl.dataset.key || pendEl.textContent.trim()) : '';
  const floatEl = [...document.querySelectorAll('.float')].find(shown) || null;
  const coralOf = (cs) => { const m = cs.boxShadow && cs.boxShadow.match(/rgba?\([^)]+\)/); return /inset/.test(cs.boxShadow) && /2px 0px 0px/.test(cs.boxShadow) && m && near(parse(m[0]), CORAL, 4); };
  // is this element's pixel at (x, y) on screen: inside every clip (elementsFromPoint skips clipped content) and not under an opaque
  // fill or a float (transparent text boxes above it don't hide it)
  const visibleAt = (el, x, y) => {
    if (x < 0 || y < 0 || x >= W || y >= H) return false;
    for (const e of document.elementsFromPoint(x, y)) {
      if (e === el || el.contains(e)) return true;
      if (e.contains(el)) continue;
      const bg = parse(getComputedStyle(e).backgroundColor);
      if ((bg && bg.a >= 0.9) || e.closest('.float')) return false;
    }
    return false;
  };
  const edgeEl = [...document.querySelectorAll('.box, .fld')].find((b) => shown(b) && near(parse(getComputedStyle(b).borderLeftColor), CORAL, 4)) || null;
  const sideCursor = [...document.querySelectorAll('.side *')].some((e) => shown(e) && coralOf(getComputedStyle(e)) && (() => { const r = e.getBoundingClientRect(); return visibleAt(e, r.left + 1, r.top + r.height / 2); })());
  const FOCUS = floatEl && !PEND ? 'float' : MODE === 'i' ? 'container' : sideCursor ? 'sidebar' : 'stage';
  const pageCtx = () => (document.querySelector('.sin .connect') ? 'connect' : document.querySelector('.sin .start') ? 'start' : document.querySelector('.sin .ccol') ? 'conversation' : document.querySelector('.sin .wpage .dmarg') ? 'decision' : document.querySelector('.sin .wpage') ? 'project' : 'list');
  // floats are root contexts (the float-keys ruling): each is told by what it draws and the badge it names
  const floatCtx = (f) => (f.querySelector('.ntag') ? 'reply' : f.querySelector('.vtag') ? 'readback' : MODE === 'rb' ? 'confirm' : MODE === 'ob' ? 'outbox' : MODE === 'sv' ? 'server' : MODE === 'ho' ? 'peek' : MODE === 'i' ? 'bind' : MODE === 'c' ? 'picker' : 'global');
  const containerCtx = (b) => (!b ? pageCtx() : b.closest('.start') ? 'start' : b.closest('.cstack') ? 'composer' : b.closest('.note') ? 'note' : b.closest('.connect') ? 'connect' : b.closest('.float') ? 'bind' : 'editor');
  const KCTX = FOCUS === 'float' ? floatCtx(floatEl) : FOCUS === 'container' ? containerCtx(edgeEl) : FOCUS === 'sidebar' ? 'sidebar' : pageCtx();
  // the part of the screen whose hints speak for the keys: the float, or the focused container's page section
  const focusRegion = FOCUS === 'float' ? floatEl : FOCUS === 'container' ? (edgeEl && (edgeEl.closest('.start, .connect') || edgeEl)) : null;

  const KM = window.__keymap || { contexts: {}, binds: [], ex: {}, builtin: {} };
  const SEQ = { '␣': '<leader>', '⏎': '<CR>', '⌘⏎': '<D-CR>', 'M-⏎': '<M-CR>', 'Esc': '<Esc>', 'Tab': '<Tab>', 'S-Tab': '<S-Tab>' };
  const seqOf = (key) => SEQ[key] || (/^[CMSD]-.$/.test(key) ? `<${key}>` : key.replace(/^␣/, '<leader>'));
  const chain = (ctx) => { const out = []; for (let c = ctx; c && !out.includes(c); c = KM.contexts[c]) out.push(c); return out; };
  // does this key act now: bound for the region and mode that have the keys, after any pending prefix
  const bound = (key) => {
    const ch = chain(KCTX), B = KM.builtin || {};
    if (key.startsWith(':')) { const e = KM.ex[key.slice(1)]; return !PEND && MODE === 'n' && !!e && e.ctx.some((x) => ch.includes(x)); }
    const one = seqOf(key);
    if (PEND) return (B.pending || []).includes(one);
    // a key resolves when a binding reaches this context and its command applies here (the command's own ctx list)
    if (KM.binds.some((b) => b.seq === one && b.mode === MODE && ch.includes(b.ctx) && (!b.cctx || !b.cctx.length || b.cctx.some((x) => ch.includes(x))))) return true;
    if (MODE === 'i' && ch.includes('editor')) {
      const sets = ['editor_insert', ...ch.map((c) => `${c}_insert`)];
      if (sets.some((k) => (B[k] || []).includes(one)) || (B.timeout_ok || []).includes(one)) return true;
    }
    return false;
  };
  const conn = CTX.conn || 'ok';
  const writes = conn === 'ok' || conn === 'reconnecting';
  // whether the command can run in the drawn state (the facts the view draws: open, running, failed, drafts, the link)
  const LIVE = {
    '␣ad': () => CTX.dec && CTX.dec.status === 'open' && !CTX.dec.writesOff && writes,
    '␣ar': () => CTX.dec && CTX.dec.thenFaulted && !CTX.dec.writesOff && writes,
    '␣cx': () => CTX.conv === 'running' && !CTX.writesOff,
    '␣cc': () => CTX.conv === 'failed' && !CTX.writesOff,
    '␣rc': () => conn !== 'ok' && conn !== 'revoked',
    '␣rp': () => conn === 'revoked',
    '␣rt': () => (CTX.counts || {}).unknown > 0,
    ':w': () => CTX.drafts > 0 && conn === 'ok',
    // while create is in flight the start page's keys are off (TC-02)
    '⌘⏎': () => !CTX.starting, 'M-p': () => !CTX.starting, 'M-w': () => !CTX.starting,
    '␣pb': () => conn === 'ok',
  };
  const acts = (key) => bound(key) && (!LIVE[key] || !!LIVE[key]());

  // ── CHIPS: the budget per region: stage (header, body, notice) ≤ 3 at first sight, the sidebar's fixed two, bottom bar ≤ 1,
  // the echo none; each chip and each hint printed only where reference/grammar.md §9 prints one. Floats print their own keys.
  const EXEMPT = '.float';
  const PRINTABLE = [['⏎', '.rw.need.sel .l1'], ['␣ad', '.shead .seg.prim'], ['␣ar', '.shead'], ['␣cx', '.shead'], ['␣cc', '.shead, .rfail'], [':w', '.status .save'],
    ['␣cn', '.side'], ['␣.', '.side'], ['␣rc', '.notice'], ['␣rp', '.notice'], ['␣rt', '.notice, .shead, .prow2'], ['␣pb', '.empty, .startline'],
    ['⏎', '.connect .go'], ['c', '.ghost']];
  const HINTABLE = [['⌘⏎', '.start .comp'], ['M-p', '.startline'], ['M-w', '.startline'], ['Esc', '.cstack .comp, .note.act'], ['jk', '.cstack .comp, .note.act'],
    ['⌘⏎', '.cstack .comp'], ['M-⏎', '.cstack .comp']]; // the start line, the start composer, the composer's and the note's Insert keys
  const echo = chips.filter((c) => c.closest('.status .echo'));
  const status = chips.filter((c) => c.closest('.status') && !c.closest('.status .echo'));
  const side = chips.filter((c) => c.closest('.side'));
  const stage = chips.filter((c) => !c.closest('.status, .side') && !c.closest(EXEMPT));
  const hints = [...document.querySelectorAll('.hint')].filter((c) => shown(c));
  const chipFails = [];
  if (stage.length > 3) chipFails.push(`stage ${stage.length} > 3`);
  if (document.querySelector('.side') && (side.length !== 2 || !side.some((c) => keyOf(c) === '␣cn') || !side.some((c) => keyOf(c) === '␣.'))) chipFails.push(`sidebar prints ${side.map(keyOf).join(' ') || 'nothing'}, not its fixed ␣cn and ␣.`);
  if (status.length > 1) chipFails.push(`bottom bar ${status.length} > 1`);
  if (echo.length) chipFails.push(`echo prints ${echo.map(keyOf).join(' ')}`);
  for (const c of [...stage, ...side, ...status]) if (!PRINTABLE.some(([k, sel]) => k === keyOf(c) && c.closest(sel))) chipFails.push(`not printable here: ${keyOf(c)}`);
  for (const c of hints) if (!c.closest(EXEMPT) && !HINTABLE.some(([k, sel]) => k === keyOf(c) && c.closest(sel))) chipFails.push(`hint not printable here: ${keyOf(c)}`);
  const chipsOut = { stage: stage.length, side: side.length, status: status.length, hints: hints.length, exempt: chips.length - stage.length - side.length - status.length - echo.length, keys: chips.map(keyOf), fails: chipFails };

  // ── RESOLVE: every chip drawn lit, and every hint in the region that has the keys, acts there (bound in keymap.toml for that
  // region's context and the badge's mode, and runnable in the drawn state). Muted chips and hints outside the focus are counted,
  // not judged: DIM proves they are muted because they don't act.
  const resolveFails = []; let judged = 0, skipped = 0;
  for (const c of [...chips, ...hints]) {
    const lit = !isMuted(c);
    if (!lit) { skipped++; continue; }
    judged++;
    if (c.closest('.status .echo')) { resolveFails.push(`${keyOf(c)} (echo)`); continue; }
    const key = keyOf(c);
    if (!bound(key)) { resolveFails.push(`${key} unbound in ${KCTX} (${MODE}${PEND ? ', pending ' + PEND : ''})`); continue; }
    if (LIVE[key] && !LIVE[key]()) resolveFails.push(`${key} cannot run here`);
  }
  const modal = !!floatEl;
  const insert = MODE === 'i';

  // ── CORAL: solid coral areas larger than the caret and wider than a 3 px rule; one coral cursor; no coral in agent or row text.
  // Allowed coral beyond the masses below: the brand (an image, not measured), page titles (text).
  const masses = [], coralFails = [];
  for (const el of document.querySelectorAll('body *')) {
    if (el.closest('svg')) continue;
    const c = parse(getComputedStyle(el).backgroundColor);
    if (!c || c.a < 0.9 || !near(c, CORAL, 4) || !shown(el)) continue;
    const r = el.getBoundingClientRect();
    if (Math.min(r.width, r.height) <= 3 || r.width * r.height <= 8 * 21 || el.matches('.caret, .bar, .bc')) continue;
    const what = el.closest('.tags') || el.matches('.ttag') ? 'tag' : el.matches('.mode.m-readback') ? 'badge' : el.matches('.vtag') ? 'verb tag' : el.closest('.rw.need.head') ? 'needs-you head' : el.matches('.shead .seg.prim') ? 'primary' : (el.className || el.tagName);
    masses.push({ what, w: Math.round(r.width), h: Math.round(r.height) });
  }
  const ALLOWED = ['tag', 'badge', 'verb tag', 'needs-you head', 'primary'];
  for (const m of masses) if (!ALLOWED.includes(m.what)) coralFails.push(`solid coral on ${m.what}`);
  let solo = masses.filter((m) => m.what !== 'tag');
  if (solo.some((m) => m.what === 'verb tag')) solo = solo.filter((m) => m.what !== 'badge'); // a pair, counted once
  if (solo.length > 1) coralFails.push(`${solo.length} solid masses besides the tag`);
  if (CTX.calm && masses.length) coralFails.push(`calm screen has ${masses.map((m) => m.what).join(', ')}`);
  if (CTX.calm && document.querySelector('.h1 .ttag, .rw.need, .sm.dec')) coralFails.push('calm screen has a needs-you cue');
  // coral text only in title.page: never in agent, extension or row text, nor anywhere else
  for (const el of document.querySelectorAll('body *')) {
    if (!shown(el) || el.closest('.ptitle, svg')) continue;
    const own = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
    const cs = getComputedStyle(el);
    if (own && near(parse(cs.color), CORAL, 4) && !el.matches('.caret, .bar')) coralFails.push(`coral text in ${el.closest('[data-spoof]') ? 'row text' : el.closest('[data-content]') ? 'content' : 'chrome'}: ${el.textContent.trim().slice(0, 24)}`);
    if (!el.closest('[data-content], [data-spoof]')) continue;
    const bg = parse(cs.backgroundColor);
    if (bg && bg.a > 0.5 && near(bg, CORAL, 4) && !el.matches('.caret, .bar')) coralFails.push('coral fill in content');
  }
  // thin coral (≤ 2 px) is the keyboard only: a float's top rule, the focused container's edge, cursor bars on selectable rows
  for (const el of document.querySelectorAll('body *')) {
    if (el.closest('svg') || !shown(el)) continue;
    const cs = getComputedStyle(el);
    for (const sideN of ['Top', 'Right', 'Bottom', 'Left']) {
      if (parseFloat(cs[`border${sideN}Width`]) < 1 || !near(parse(cs[`border${sideN}Color`]), CORAL, 4)) continue;
      const ok = (el.matches('.float') && sideN === 'Top') || el.matches('.box.ins, .note.act, .fld.focus');
      if (!ok) coralFails.push(`thin coral ${sideN.toLowerCase()} edge on ${String(el.className).slice(0, 24) || el.tagName}`);
    }
    if (coralOf(cs) && (el.closest('[data-content], [data-spoof]') || !el.matches('.mode, .sel, .scur, .cur, .curline, .prow, .ci, .cho, .frow, .lrow, .rw, .row, .tool, .mh'))) coralFails.push(`coral bar on ${String(el.className).slice(0, 24) || el.tagName}`);
  }
  const coralOut = { masses, fails: coralFails };
  // ── CURSOR (TC-05, TC-06): coral cursor marks counted only where they show on screen (hit-tested at the bar's pixels, so a bar
  // clipped by overflow or covered by a float doesn't count): 2 px bars, the coral Needs you head, the document cursor bar, a coral
  // caret, a focused container's coral edge. The region that has the keys draws exactly one; a float that takes the keys and has
  // no selection draws none; every other region draws none.
  const marks = [], cursorFails = [];
  for (const el of document.querySelectorAll('body *')) {
    if (el.closest('svg') || el.matches('.mode') || !shown(el)) continue;
    const cs = getComputedStyle(el); const r = el.getBoundingClientRect();
    let kind = null;
    if (coralOf(cs) && visibleAt(el, r.left + 1, r.top + r.height / 2)) kind = 'bar';
    else if (el.matches('.curbar') && near(parse(cs.backgroundColor), CORAL, 4) && visibleAt(el, r.left + r.width / 2, r.top + r.height / 2)) kind = 'bar';
    else if (el.matches('.rw.need .l1') && near(parse(cs.backgroundColor), CORAL, 4) && visibleAt(el, r.left + r.width / 2, r.top + r.height / 2)) kind = 'bar';
    else if (r.width <= 10 && near(parse(cs.backgroundColor), CORAL, 4) && visibleAt(el, r.left + r.width / 2, r.top + r.height / 2)) kind = 'caret';
    else if (el.matches('.box, .fld') && near(parse(cs.borderLeftColor), CORAL, 4) && visibleAt(el, r.left + 0.5, r.top + r.height / 2)) kind = 'edge';
    if (!kind) continue;
    const region = el.closest('.float') ? 'float' : el.closest('.side') ? 'sidebar' : (el.closest('.box, .fld') && kind !== 'bar') ? 'container' : el.closest('.stage') ? 'stage' : 'other';
    marks.push({ kind, region, cls: String(el.className).slice(0, 30) });
  }
  // the region that has the keys, from the state (the states list says who has them) and checked against the render's own badge
  const want = CTX.focus === 'sidebar' && FOCUS === 'stage' ? 'sidebar' : FOCUS;
  const sel = floatEl && [...floatEl.querySelectorAll('.prow.sel, .ci.sel')].some(shown);
  const inWant = marks.filter((m) => m.region === want), elsewhere = marks.filter((m) => m.region !== want);
  if (elsewhere.length) cursorFails.push(`coral cursor in the ${[...new Set(elsewhere.map((m) => m.region))].join(', ')}, but the ${want} has the keys`);
  const bars = inWant.filter((m) => m.kind === 'bar').length, edges = inWant.filter((m) => m.kind === 'edge').length, carets = inWant.filter((m) => m.kind === 'caret').length;
  const inFlight = MODE === 'i' && !edges && [...document.querySelectorAll('.start .starting')].some(shown);
  const nothing = want === 'stage' && ![...document.querySelectorAll('.sin .rw, .sin .lrow, .sin .frow, .sin .cho, .sin .msg, .sin .md [data-s], .sin .tool')].some(shown);
  if ((want === 'float' && !sel) || inFlight || nothing) { if (marks.length) cursorFails.push(`${marks.length} coral cursor mark(s) where none may show`); }
  else if (want === 'container') { if (edges !== 1 || carets < 1) cursorFails.push(`focused container: ${edges} coral edge(s), ${carets} caret(s)`); }
  else if (bars !== 1) cursorFails.push(`${bars} coral cursor(s) in the ${want}`);
  const cursorOut = { focus: want, marks: marks.length, zero: (want === 'float' && !sel) || inFlight || nothing, fails: cursorFails };

  // ── CONTRAST: text 4.5:1 (3:1 when large), marks 3:1, against the surface they sit on
  const fails = [], seen = new Set();
  let nText = 0, nMarks = 0;
  const test = (el, fgStr, need, kind, sample, under) => {
    const fg = parse(fgStr); if (!fg) return;
    const bg = bgOf(el, under); const f = fg.a < 1 ? over(fg, bg) : fg; const r = ratio(f, bg);
    const key = `${hex(f)}|${hex(bg)}|${kind}`;
    if (r < need && !seen.has(key)) { seen.add(key); fails.push({ kind, fg: hex(f), bg: hex(bg), ratio: +r.toFixed(2), need, sample: String(sample).slice(0, 40) }); }
  };
  for (const el of document.querySelectorAll('body *')) {
    if (el.closest('svg') || el.closest('.lights')) continue;
    const txt = [...el.childNodes].filter((n) => n.nodeType === 3).map((n) => n.textContent).join('').trim();
    if (!txt || !shown(el) || !onTop(el)) continue;
    const cs = getComputedStyle(el); const size = parseFloat(cs.fontSize), bold = +cs.fontWeight >= 700;
    nText++;
    test(el, cs.color, size >= 24 || (size >= 18.66 && bold) ? 3 : 4.5, 'text', txt);
  }
  for (const el of document.querySelectorAll('svg.i, svg.tau, .connect .wm svg')) {
    if (!shown(el) || el.closest('.lights') || !onTop(el)) continue;
    nMarks++;
    test(el, getComputedStyle(el).color, 3, 'mark', (el.querySelector('use') || el).getAttribute('href') || el.className.baseVal);
  }
  const contrastOut = { text: nText, marks: nMarks, fails };

  // ── COUNTS: every count printed more than once agrees, and a count printed over rows equals the rows drawn
  const countFails = []; const groups = {}; let checked = 0;
  for (const el of document.querySelectorAll('[data-count]')) {
    if (!shown(el) && !el.closest('.tags')) continue;
    (groups[el.dataset.count] ||= []).push(parseInt(el.textContent, 10)); checked++;
  }
  for (const [k, vs] of Object.entries(groups)) if (new Set(vs).size > 1) countFails.push(`${k}: ${vs.join(' vs ')}`);
  const complete = (sel) => !document.querySelector(sel);
  const ROWS = { needs: ['.rw.need'], failed: ['.rw.failed', '.more-failed'], running: ['.rw.run'], 'new-dec': ['.rw.need[data-new]'], 'new-ev': ['.rw.ev.new, .lrow.new', '.more'],
    notes: ['.note, .mwhole[data-note]'], 'failed-faults': ['.lrow'] };
  for (const [k, vs] of Object.entries(groups)) {
    let sel = ROWS[k] && ROWS[k][0], unless = ROWS[k] && ROWS[k][1];
    const m = k.match(/^(run|fail|all)-(.+)$/);
    if (m) sel = `[data-crow="${m[2]}"]${m[1] === 'run' ? '[data-cstate=running]' : m[1] === 'fail' ? '[data-cstate=failed]' : ''}`;
    if (!sel || !document.querySelector(sel.split(',')[0].replace(/\[data-new\]|\.new/, '')) || (unless && !complete(unless) && k !== 'new-ev')) continue;
    const n = document.querySelectorAll(sel).length;
    if (vs.some((v) => v !== n)) countFails.push(`${k}: prints ${vs[0]}, draws ${n}`);
  }

  // presence (TC-13): a count above 0 has exactly one visible tag, on the sidebar's Home row or, while the sidebar is hidden, in the
  // bottom bar; Home's title carries the Needs you count; an abnormal link shows the server's words in exactly one visible place
  const sideShown = [...document.querySelectorAll('.side')].some(shown);
  const N = CTX.counts || {};
  for (const kind of ['needs', 'failed', 'unknown']) {
    const tags = [...document.querySelectorAll(`.tag[data-tag=${kind}]`)].filter((t) => shown(t) && onTop(t));
    const want = N[kind] || 0; checked++;
    if (!want) { if (tags.length) countFails.push(`${kind} tag shown at 0`); continue; }
    if (tags.length !== 1) { countFails.push(`${kind} ${want}: ${tags.length} visible tags`); continue; }
    const where = tags[0].closest('.side') ? 'sidebar' : tags[0].closest('.status') ? 'bar' : 'elsewhere';
    if (where !== (sideShown ? 'sidebar' : 'bar')) countFails.push(`${kind} tag in the ${where} while the sidebar is ${sideShown ? 'shown' : 'hidden'}`);
    if (parseInt(tags[0].textContent, 10) !== want) countFails.push(`${kind} tag reads ${tags[0].textContent.trim()}, the data says ${want}`);
  }
  const homeTitle = [...document.querySelectorAll('.ptitle')].find((t) => shown(t) && /^(Needs you|Nothing needs you)/.test(t.textContent.trim()));
  if (homeTitle) { checked++; const tt = homeTitle.querySelector('.ttag'); if ((N.needs || 0) > 0 && (!tt || !shown(tt) || parseInt(tt.textContent, 10) !== N.needs)) countFails.push('Home title without its Needs you count'); if (!(N.needs > 0) && tt) countFails.push('Home title tag at 0'); }
  const WORDS = { connecting: 'connecting', stopped: 'server stopped', offline: 'offline', reconnecting: 'reconnecting', revoked: 'revoked' };
  const wordsAt = [...document.querySelectorAll('.side .srv, .status .srvb')].filter((e) => shown(e) && onTop(e));
  if (WORDS[conn]) { checked++; const n = wordsAt.filter((e) => e.textContent.includes(WORDS[conn])).length; if (n !== 1) countFails.push(`the link is ${conn}: its words show in ${n} places`); }
  else if (wordsAt.some((e) => e.matches('.bad, .srvb'))) countFails.push('server trouble shown on a calm link');

  // ── MEASURE: prose lines in the text column stay within the 600 px measure
  let measureOut = null;
  const prose = [...document.querySelectorAll('.doc .md p, .doc .md li, .connect p')].filter((p) => shown(p) && !p.closest('.peek, .float'));
  if (prose.length) {
    const ctx2 = document.createElement('canvas').getContext('2d');
    let maxW = 0, maxChars = 0; const mf = [];
    for (const p of prose) {
      const w = p.getBoundingClientRect().width; maxW = Math.max(maxW, w);
      if (w > 600.5) mf.push(`${Math.round(w)} px: ${p.textContent.slice(0, 30)}`);
      const lines = new Map(); const tw = document.createTreeWalker(p, NodeFilter.SHOW_TEXT);
      for (let n = tw.nextNode(); n; n = tw.nextNode()) for (let i = 0; i < n.length; i++) { const rg = document.createRange(); rg.setStart(n, i); rg.setEnd(n, i + 1); const rc = rg.getClientRects()[0]; if (!rc) continue; const k = Math.round(rc.top / 6); lines.set(k, (lines.get(k) || 0) + 1); }
      const counts = [...lines.values()].slice(0, -1); for (const c of counts) maxChars = Math.max(maxChars, c);
    }
    if (maxChars > 92) mf.push(`a line of ${maxChars} characters`);
    ctx2.font = '400 14px Etoile';
    measureOut = { blocks: prose.length, maxWidth: Math.round(maxW), maxChars, fails: mf };
  }

  // ── SPOOF: agent and extension text in rows is one plain line in a fixed slot (direction §4.2)
  const spoofFails = []; const sp = [...document.querySelectorAll('[data-spoof]')].filter(shown);
  for (const el of sp) {
    const cs = getComputedStyle(el); const t = el.textContent.slice(0, 24);
    const lh = parseFloat(cs.lineHeight) || 18;
    if (el.children.length && !el.querySelector(':scope > .mu:only-child')) spoofFails.push(`markup inside: ${t}`);
    if (cs.whiteSpace !== 'nowrap' && el.getClientRects().length > 1) spoofFails.push(`wraps: ${t}`);
    if (el.getBoundingClientRect().height > lh * 1.5) spoofFails.push(`more than one line: ${t}`);
    if (/[\x00-\x1f\x7f]/.test(el.textContent)) spoofFails.push(`raw control character: ${t}`);
    if (!cs.fontFamily.startsWith('Etoile') && !cs.fontFamily.startsWith('"Etoile"')) spoofFails.push(`chrome face ${cs.fontFamily}: ${t}`);
    const size = parseFloat(cs.fontSize), wt = +cs.fontWeight;
    const okFace = (size === 12 && wt === 400 && el.closest('.dpanel')) || (size === 24 && wt === 700 && el.closest('.ptitle')) || (size === 13 && (wt === 400 || (wt === 600 && el.closest('.new')))) || (size === 14 && wt === 600 && (el.matches('.mv') || el.closest('.peek .pt')));
    if (!okFace) spoofFails.push(`face ${size}/${wt}: ${t}`);
    if (cs.textTransform !== 'none' || (cs.letterSpacing !== 'normal' && parseFloat(cs.letterSpacing) !== 0)) spoofFails.push(`case or tracking: ${t}`);
    for (const e of [el, ...el.querySelectorAll('*')]) { const b = parse(getComputedStyle(e).backgroundColor); if (b && b.a > 0) spoofFails.push(`fill: ${t}`); }
    // fixed slots: the client's mark before the text, the #id after it
    const row = el.parentElement;
    const kids = [...row.children]; const at = kids.indexOf(el);
    const mk = kids.findIndex((k) => k.matches('.mk')); const idx = kids.findIndex((k) => k.matches('.tg, .idc, .id'));
    if (mk >= 0 && mk > at) spoofFails.push(`mark after the text: ${t}`);
    if (idx >= 0 && idx < at && !(row.closest('.side') && kids[idx].matches('.id.pre'))) spoofFails.push(`#id before the text: ${t}`); // the sidebar's conversation item prints #4 first, in its own slot (spec §4.1)
  }

  // ── DIM: with a float open or in Insert, chips, hints and segments outside the focused float or container are fg.muted,
  // and the Answer is its muted outline
  const dimFails = [];
  // one rule (TC-04): a chip or segment is lit only when its key acts for the region and mode that have the keys, else muted;
  // the Answer is solid only when ␣ad would open the readback. Hints are muted by design; RESOLVE judges those that speak.
  let dimChecked = 0;
  for (const c of [...chips, ...hints]) {
    if (!shown(c)) continue; dimChecked++;
    const a2 = acts(keyOf(c)), lit = !isMuted(c);
    if (lit && !a2) dimFails.push(`lit but inert: ${keyOf(c)} (${KCTX}, ${MODE}${PEND ? ', pending' : ''})`);
    if (!lit && a2) dimFails.push(`muted but live: ${keyOf(c)} (${KCTX}, ${MODE})`);
  }
  for (const sg of document.querySelectorAll('.shead .seg')) { if (!shown(sg) || sg.querySelector('.kbd')) continue; dimChecked++; if (!isMuted(sg)) dimFails.push(`a segment without a key is lit: ${sg.textContent.trim().slice(0, 20)}`); }
  for (const pr of document.querySelectorAll('.shead .seg.prim')) {
    if (!shown(pr)) continue; dimChecked++;
    const solid = near(parse(getComputedStyle(pr).backgroundColor), CORAL, 4);
    if (solid !== acts('␣ad')) dimFails.push(solid ? 'the Answer is solid coral but ␣ad would not open the readback' : 'the Answer is muted but ␣ad would open the readback');
  }

  // ── IDS: #ids, decision IDs, subjects and kinds are never cut
  const idFails = [];
  for (const el of document.querySelectorAll('.id, .tg, .idc, .sj, .did, .kind, .tid, .aft')) {
    if (!shown(el) || el.matches('.rw.line .tx.fl .kind')) continue; // spec §4.7: in a faulted task's label the kind is cut first
    const r = el.getBoundingClientRect();
    for (let a = el.parentElement; a && a !== document.body; a = a.parentElement) {
      const cs = getComputedStyle(a); if (cs.overflow === 'visible' && cs.overflowX === 'visible') continue;
      const ar = a.getBoundingClientRect(); if (r.right > ar.right + 0.5 || r.left < ar.left - 0.5) { idFails.push(el.textContent.trim()); break; }
    }
    if (el.scrollWidth > el.clientWidth + 1 && getComputedStyle(el).overflow !== 'visible') idFails.push(el.textContent.trim());
  }

  // a tint always comes with its word: every failed-row tint says failed
  const FAILT = TOK('--row-fail');
  for (const el of document.querySelectorAll('body *')) { if (!shown(el) || el.closest('svg')) continue; if (near(parse(getComputedStyle(el).backgroundColor), FAILT, 3) && !/failed/.test(el.textContent)) spoofFails.push(`failed tint without its word: ${el.textContent.trim().slice(0, 24)}`); }
  // title.page only on the stage's first line (one per view, never inside content); a content h1 is rose with its rule
  const titles = [...document.querySelectorAll('.ptitle')].filter(shown);
  if (titles.length > 1) spoofFails.push(`${titles.length} page titles`);
  for (const t of titles) { if (t.closest('[data-content]')) spoofFails.push('page title inside content'); if (t.querySelector('h1, h2, h3, h4, h5, h6, strong, b, em, code, pre')) spoofFails.push('page title carries markup: it is one plain line'); const r = t.getBoundingClientRect(); const col = t.closest('.lcol, .ccol, .doc, .connect'); if (col && [...col.querySelectorAll('*')].some((e) => !t.contains(e) && shown(e) && onTop(e) && [...e.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim()) && e.getBoundingClientRect().top < r.top - 1)) spoofFails.push('page title not on the first line'); }
  for (const h of [...document.querySelectorAll('[data-content] h1, .md h1')].filter(shown)) { const cs = getComputedStyle(h); if (!near(parse(cs.color), TOK('--h1'), 4)) spoofFails.push('content h1 not rose'); if (parseFloat(cs.borderBottomWidth) < 1) spoofFails.push('content h1 without its rule'); }

  return { chips: chipsOut, resolve: { fails: resolveFails, judged, skipped, ctx: KCTX, mode: MODE, focus: FOCUS }, coral: coralOut, cursor: cursorOut, contrast: contrastOut, counts: { checked, fails: countFails }, measure: measureOut,
    spoof: { items: sp.length, fails: spoofFails }, dim: { applies: modal || insert || FOCUS !== 'stage', checked: dimChecked, fails: dimFails }, ids: { fails: idFails } };
}
