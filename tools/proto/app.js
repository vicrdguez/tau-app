// tau desktop prototype, calm frame (e13): every screen is drawn from fixture.json (window.FX). location.hash names a state.
// The data layer is e11's; the frame and the views follow the e12 calm sketch.
(() => {
  const FX = window.FX, C = FX.calls, ST = FX.streams, L = FX.local;
  const PROJECTS_ALL = C.projects.result.items.map((p) => p.project);

  // ───────────── helpers ─────────────
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
  // one plain line: no Markdown, control characters escaped visibly (direction §4.2)
  // bidi controls and zero-width characters as visible escapes (mono.small on surface.bar), in content and in rows
  const INVIS = /[\u200B-\u200F\u202A-\u202E\u2066-\u2069\uFEFF]/g;
  const unInvis = (html) => html.replace(INVIS, (c) => `<span class="uesc">U+${c.charCodeAt(0).toString(16).toUpperCase().padStart(4, '0')}</span>`);
  const plain = (s) => unInvis(esc(String(s).replace(/`([^`\n]+)`/g, '$1')).replace(/[\x00-\x1f\x7f]/g, (c) => (c === '\n' ? '\\n' : `\\x${c.charCodeAt(0).toString(16).padStart(2, '0')}`)));
  const I = (id, cls = '') => `<svg class="i${cls ? ' ' + cls : ''}"><use href="#${id}"/></svg>`;
  // key glyphs: ␣ and ⏎ are drawn paths (tokens.json tau.icon.key_space, key_return)
  const glyphs = (k) => esc(k).replace(/␣/g, I('space')).replace(/⏎/g, I('key-return'));
  const K = (k) => `<span class="kbd" data-key="${esc(k)}">${glyphs(k)}</span>`;
  const dot = '<span class="dt">·</span>';
  const pad = (n) => String(n).padStart(2, '0');
  let NOW = FX.now;
  const hm = (ms) => { const d = new Date(ms); return `${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`; };
  const MON = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  const when = (ms) => { const d = new Date(ms), n = new Date(NOW); return d.toISOString().slice(0, 10) === n.toISOString().slice(0, 10) ? hm(ms) : `${MON[d.getUTCMonth()]} ${d.getUTCDate()} ${hm(ms)}`; };
  const age = (ms) => { const m = Math.round((NOW - ms) / 60000); return m < 60 ? `${m} m` : m < 1440 ? `${Math.floor(m / 60)} h ${m % 60} m` : `${Math.floor(m / 1440)} d ${Math.floor((m % 1440) / 60)} h`; };
  const modelShort = (m) => (m || '').replace(/^[^/]+\//, '');
  const modelName = (m) => (C.models.result.items.find((x) => x.model === m) || {}).name;
  const leaf = (p) => '…/' + p.split('/').filter(Boolean).pop();
  const clone = (v) => (v === undefined ? v : JSON.parse(JSON.stringify(v)));
  // the seven ops of a stream frame (those the fixture uses: r s d a p)
  function apply(v, ops) {
    for (const op of ops) {
      if (op[0] === 'r') { v = clone(op[1]); continue; }
      const [t, path, ...a] = op;
      if (path.some((k) => ['__proto__', 'constructor', 'prototype'].includes(k))) continue;
      const parent = path.slice(0, -1).reduce((o, k) => o[k], v), k = path[path.length - 1];
      if (t === 's') parent[k] = clone(a[0]);
      else if (t === 'd') { if (Array.isArray(parent)) parent.splice(k, 1); else delete parent[k]; }
      else if (t === 'a') parent[k] += a[0];
      else if (t === 'p') parent[k].splice(a[0], a[1], ...clone(a[2]));
    }
    return v;
  }
  marked.use({ gfm: true, breaks: true, renderer: { html({ text, block }) { return block ? `<p class="raw">${esc(text.replace(/\n+$/, ''))}</p>` : esc(text); } } });
  const mdInline = (s) => marked.parseInline(s);
  function mdBlocks(text, cls = '') {
    // top-level blocks, each with its UTF-16 source range, so a note's range finds its block (direction §4.5)
    const toks = marked.lexer(text);
    let off = 0, html = '';
    for (const t of toks) {
      const start = text.indexOf(t.raw, off); const s = start < 0 ? off : start; const e = s + t.raw.length; off = e;
      if (t.type === 'space') continue;
      let inner;
      if (t.type === 'code') {
        const wide = t.text.split('\n').some((l) => l.length > 70);
        inner = `<pre${wide ? ' data-wide' : ''}>${t.lang ? `<span class="lang">${esc(t.lang)}</span>` : ''}${esc(t.text)}</pre>`;
      } else if (t.type === 'list') {
        let io = s; const items = t.items.map((it) => { const is = text.indexOf(it.raw, io); io = is + it.raw.length; return `<li data-s="${is}" data-e="${io}">${marked.parser(it.tokens.length ? [{ type: 'paragraph', tokens: it.tokens.flatMap((x) => x.tokens || [x]), raw: it.text, text: it.text }] : []).replace(/^<p>|<\/p>\s*$/g, '')}</li>`; });
        inner = `<${t.ordered ? 'ol' : 'ul'}${t.ordered && t.start !== 1 ? ` start="${t.start}"` : ''}>${items.join('')}</${t.ordered ? 'ol' : 'ul'}>`;
      } else inner = marked.parser([t]);
      html += `<div class="blk" data-s="${s}" data-e="${e}" data-type="${t.type}">${inner}</div>`;
    }
    return `<div class="md ${cls}" data-content>${html}</div>`;
  }

  // ───────────── the world, from the fixture plus a state's overrides ─────────────
  function world(s) {
    let inbox = apply(null, ST.inbox.first.ops);
    for (const i of s.inboxLater || []) inbox = apply(inbox, ST.inbox.later[i].frame.ops);
    if (s.calm || s.noProjects) inbox = s.noProjects ? {} : { forecasts: {}, 'notes-cli': {}, 'render-core': {} };
    if (s.hostile) inbox = { ...inbox, ...FX.hostile.inbox };
    const projects = s.noProjects ? [] : [...C.projects.result.items, ...(s.scratch ? [C['bind scratch'].result.project] : []), ...(s.hostile ? [FX.hostile.project] : [])];
    const pnames = projects.map((p) => p.project);
    const convs = {};
    for (const p of pnames) convs[p] = clone((C[`conversations ${p}`] || (p === 'hostile' ? FX.hostile.conversations : null) || { result: { items: [] } }).result.items);
    for (const [p, id, st] of s.convState || []) convs[p].find((c) => c.conversation === id).state = st;
    const hidden = s.hiddenFaults || L.hiddenFaults;
    const faults = [];
    for (const p of pnames) for (const f of (C[`faults ${p}`] || { result: { items: [] } }).result.items) if (!hidden.includes(`${p}/${f.task}`)) faults.push({ project: p, ...f });
    const events = [];
    for (const p of pnames) {
      const tl = ST[`timeline ${p}`]; if (!tl) continue;
      for (const e of apply(null, tl.first.ops).entries) events.push({ project: p, ...e });
    }
    events.sort((a, b) => b.data.at - a.data.at);
    const decisions = [];
    for (const [p, subs] of Object.entries(inbox)) for (const [sub, d] of Object.entries(subs)) decisions.push({ project: p, subject: sub, ...d, n: +d.id.split('#')[1] });
    decisions.sort((a, b) => a.requestedAt - b.requestedAt);
    const outbox = [...L.outbox.filter((o) => o.state !== 'unknown' || s.unknown), ...(s.extraOutbox || [])];
    return { inbox, projects, pnames, convs, faults, events, decisions, outbox, base: L.sinceBase.at, showNew: !!s.since, hiddenRuns: s.hiddenRuns || L.hiddenFailedRuns || [] };
  }
  const label = (p, id) => { const t = ((p === 'hostile' ? FX.hostile.firstInput : L.firstInput[p]) || {})[id]; if (!t) return null; const l = t.split('\n')[0]; return l.length > 80 ? l.slice(0, 79).replace(/\s+\S*$/, '') + '…' : l; };
  const tabLabel = (t) => { if (t.length <= 26) return t; const c = t.slice(0, 27); return c.slice(0, c.lastIndexOf(' ')).replace(/[,.:;]$/, '') + '…'; };
  const extOf = (p, id) => ((L.extensionOf[p] || {})[id]) || null;
  const modelOf = (p, id) => ((L.modelOf[p] || {})[id]) || null;
  function failedRows(W) {
    const named = W.faults.filter((f) => f.decision).sort((a, b) => b.endedAt - a.endedAt);
    const other = W.faults.filter((f) => !f.decision).sort((a, b) => b.endedAt - a.endedAt);
    const conv = [];
    for (const p of W.pnames) for (const c of [...W.convs[p]].reverse()) if (c.state === 'failed' && !W.hiddenRuns.includes(`${p}/${c.conversation}`)) conv.push({ project: p, id: c.conversation });
    return [...named.map((f) => ({ fault: f })), ...conv.map((c) => ({ conv: c })), ...other.map((f) => ({ fault: f }))];
  }
  function runningRows(W) { const r = []; for (const p of W.pnames) for (const c of [...W.convs[p]].reverse()) if (c.state === 'running') r.push({ project: p, id: c.conversation }); return r; }
  const decisionRec = (p, id) => (C.decision[`${p}/${id}`] || FX.hostile[`decision ${p}/${id}`] || {}).result;

  // ───────────── a conversation, from its open stream ─────────────
  function convView(key, later = []) {
    const st = ST[`open ${key}`] || FX.hostile[`open ${key}`];
    let v = apply(null, st.first.ops);
    for (const i of later) v = apply(v, st.later[i].ops);
    return v;
  }
  function transcript(v) {
    const items = []; const tools = {};
    let turn = null;
    for (const e of v.entries) {
      const m = e.model && e.model[0];
      if (e.kind === 'pi.user') { items.push({ t: 'input', text: m.content, at: m.timestamp, id: e.id }); turn = null; }
      else if (e.kind === 'pi.system') items.push({ t: 'sys', tools: (m.toolsAdded || []).map((x) => x.name) });
      else if (e.kind === 'pi.assistant') {
        if (!turn) { turn = { t: 'agent', model: m.model, at: m.timestamp, blocks: [] }; items.push(turn); }
        for (const b of m.content) {
          if (b.type === 'text') turn.blocks.push({ t: 'text', text: b.text });
          else if (b.type === 'thinking') turn.blocks.push({ t: 'thinking', text: b.thinking || '' });
          else if (b.type === 'toolCall') { const tb = { t: 'tool', id: b.id, name: b.name, args: b.arguments, at: m.timestamp }; tools[b.id] = tb; turn.blocks.push(tb); }
        }
      } else if (e.kind === 'pi.tool-result') { const tb = tools[m.toolCallId]; if (tb) tb.result = { isError: m.isError, at: m.timestamp, text: m.content.map((c) => c.text || '').join('') }; }
      else { items.push({ t: 'rec', kind: e.kind, byTask: e.byTaskId, text: typeof (e.data || {}).text === 'string' ? e.data.text : null, id: e.id }); turn = null; }
    }
    const live = v.docs['pi.live'] || {};
    for (const lt of live.tools || []) if (tools[lt.callId]) tools[lt.callId].live = lt;
    if (live.generation) items.push({ t: 'gen', model: live.generation.message.model, at: live.generation.message.timestamp, text: live.generation.message.content.map((c) => c.text || '').join('') });
    return { items, live, agent: v.docs['pi.agent'] || {}, inbox: (v.docs['pi.inbox'] || {}).items || [] };
  }
  const toolArgs = (tb) => { const a = tb.args || {}; if (a.command) return a.command; if (a.query) return `"${a.query}"`; if (a.path) return a.path; if (a.text) return a.text.split('\n')[0]; return JSON.stringify(a); };
  // ───────────── calm primitives ─────────────
  // a muted key hint (inside containers, the start line); it carries its key so RESOLVE checks it like a chip
  const H = (k, key, lit = false) => `<span class="hint${lit ? ' on' : ''}" data-key="${esc(key || k)}">${glyphs(k)}</span>`;
  // small status marks: an 8-10 px glyph in a 14 px slot (tokens.json tau.icon.mark_small)
  const MK = {
    dec: '<span class="sm dec"><svg class="i"><use href="#diamond"/></svg></span>',
    fail: '<span class="sm fail"><svg class="i"><use href="#xmark"/></svg></span>',
    run: '<span class="sm run"><svg class="i"><use href="#tri"/></svg></span>',
    unk: '<span class="sm unk"><svg class="i"><use href="#help"/></svg></span>',
    ok: '<span class="sm ok"><svg class="i"><use href="#checkmark"/></svg></span>',
  };
  const dur = (ms) => (ms < 10000 ? `${(ms / 1000).toFixed(1)} s` : ms < 60000 ? `${Math.round(ms / 1000)} s` : `${Math.floor(ms / 60000)} m`);
  const crumbOf = (parts) => parts.map((p, i) => (i ? '<span class="s">›</span>' : '') + `<span>${p}</span>`).join('');
  const seg = (key, labelTxt) => `<span class="seg">${K(key)}<span>${labelTxt}</span></span>`;
  const segOff = (labelTxt) => `<span class="seg off"><span>${labelTxt}</span><span class="dt">·</span><span>off</span></span>`;
  const tagsHTML = (ctx) => {
    const c = ctx.counts;
    return (c.needs ? `<span class="tag t-needs" data-tag="needs">${I('diamond')}<span data-count="needs">${c.needs}</span></span>` : '')
      + (c.failed ? `<span class="tag t-failed" data-tag="failed">${I('xmark')}<span data-count="failed">${c.failed}</span></span>` : '')
      + (c.unknown ? `<span class="tag t-unk" data-tag="unknown">${I('help')}<span data-count="unknown">${c.unknown}</span></span>` : '');
  };
  // the title block: title.page (coral, one line), one facts line, details › at its right
  const titleBlock = (title, facts, { details = false, spoof = false, noDetails = false } = {}) => `<div class="tblock"><div class="ptitle"${spoof ? ' data-spoof' : ''}>${title}</div><div class="ff"><span class="fx">${facts}</span>${noDetails ? '' : `<span class="det">details${I(details ? 'chevron--left' : 'chevron--right')}</span>`}</div></div>`;

  // ───────────── tools in the transcript: quiet one-line rows ─────────────
  function toolOutcome(tb, running) {
    if (tb.live) return 'run';
    if (tb.result) return tb.result.isError ? 'fail' : 'ok';
    return running ? 'run' : 'unk';
  }
  const took = (tb) => (tb.result && tb.result.at ? tb.result.at - tb.at : NOW - tb.at);
  function toolHTML(tb, running) {
    const o = toolOutcome(tb, running);
    const ic = { ok: MK.ok, fail: MK.fail, run: MK.run, unk: MK.unk }[o];
    const w = { ok: 'ok', fail: 'failed', run: 'running', unk: 'no result' }[o];
    const open = o === 'run' && tb.live;
    let h = `<div class="tool ${o}" data-tool><span class="ic">${ic}</span><span class="tn">${esc(tb.name)}</span><span class="ta">${esc(toolArgs(tb))}</span><span class="oc"><span class="w">${w}</span><span class="du">${o === 'run' ? dur(took(tb)) : ''}</span></span><span class="cv">${I(open ? 'chevron--down' : 'chevron--right')}</span></div>`;
    if (open) { const lines = tb.live.output.replace(/\n$/, '').split('\n'); h += `<div class="tout">${esc(lines.slice(-6).join('\n'))}</div>`; }
    return h;
  }
  function agentBlocks(blocks, running) {
    let html = ''; let i = 0;
    while (i < blocks.length) {
      const b = blocks[i];
      if (b.t === 'tool' && toolOutcome(b, running) === 'ok') {
        let j = i; while (j < blocks.length && blocks[j].t === 'tool' && toolOutcome(blocks[j], running) === 'ok') j++;
        if (j - i >= 2) { const g = blocks.slice(i, j); html += `<div class="tool ok fold" data-tool><span class="ic">${MK.ok}</span><span class="tn et">${g.length} tool calls</span><span class="ta">${g.map((x) => x.name).join(', ')}</span><span class="oc"><span class="w">ok</span><span class="du"></span></span><span class="cv">${I('chevron--right')}</span></div>`; i = j; continue; }
      }
      if (b.t === 'tool') html += toolHTML(b, running);
      else if (b.t === 'thinking') html += `<div class="sysl">thinking · ${b.text.split(/\s+/).length} words</div>`;
      else html += `<div class="prose" data-content>${unInvis(marked.parse(b.text))}</div>`;
      i++;
    }
    return html;
  }
  function transcriptHTML(T, { recCur, factsModel } = {}) {
    const running = !!T.live.run;
    let recN = 0;
    const mdl = (m) => (m && m !== factsModel ? `<span class="mdl">${esc(m)}</span>` : '');
    return T.items.map((it) => {
      if (it.t === 'input') return `<div class="msg input" data-entry="${it.id}"><div class="mh"><span class="nm in">input</span><span class="tm">${hm(it.at)}</span></div><div class="mb md prose" data-content>${unInvis(marked.parse(it.text))}</div></div>`;
      if (it.t === 'sys') return `<div class="sysl">tools: ${it.tools.join(', ')}</div>`;
      if (it.t === 'agent') return `<div class="msg"><div class="mh"><span class="nm">agent</span>${mdl(it.model)}<span class="tm">${hm(it.at)}</span></div><div class="mb md">${agentBlocks(it.blocks, running)}</div></div>`;
      if (it.t === 'gen') {
        const parts = it.text.split(/\n\n/); const last = parts.pop();
        return `<div class="msg gen"><div class="mh"><span class="nm">agent</span><span class="dt">·</span><span>writing</span>${mdl(it.model)}</div><div class="mb md prose" data-content>${parts.map((p) => marked.parse(p)).join('')}<p class="writing">${mdInline(last)}</p></div></div>`;
      }
      if (it.t === 'rec') { recN++; return `<div class="msg rec${recCur === recN ? ' cur' : ''}" data-entry="${it.id}"><div class="mh"><span class="kd">${esc(it.kind)}</span>${it.byTask ? '<span>by a task</span>' : ''}</div><div class="mb prose">${it.text != null ? mdBlocks(it.text) : '<span class="mu">no text</span>'}</div></div>`; }
      return '';
    }).join('');
  }

  // ───────────── states (states.md) ─────────────
  const DRAFT = L.startDraftExample;
  const S = {
    connect: { screen: 'connect', mode: 'INSERT', noProjects: true, noOpen: true, noSide: true, focus: 'container' },
    'connect-http-warning': { of: 'connect', server: 'http://192.168.1.20:6283', float: 'confirm-http', mode: 'READBACK', plainBadge: true, focus: 'float' },
    'bind-float': { screen: 'home', noProjects: true, noOpen: true, float: 'bind', mode: 'COMMAND', focus: 'float' },
    'project-options-unset': { screen: 'project', project: 'scratch', scratch: true, cur: 'w1/designer', extraOpen: { project: 'scratch', kind: 'project' } },
    start: { screen: 'start', mode: 'INSERT', project: 'render-core', focus: 'container' },
    'start-project-picker': { screen: 'start', mode: 'COMMAND', project: 'render-core', float: 'project', focus: 'float' },
    'start-options-unset': { screen: 'start', mode: 'INSERT', project: 'scratch', scratch: true, text: DRAFT.text, focus: 'container' },
    'start-no-reply': { screen: 'start', mode: 'INSERT', project: 'render-core', text: DRAFT.text, noReply: true, focus: 'container', extraOutbox: [{ call: 'create', body: { project: 'render-core', workflow: 'w1' }, state: 'unknown', sentAt: FX.now - 20000 }] },
    'start-light': { of: 'start', light: true },
    home: { screen: 'home', since: true, cur: 'dec:0' },
    'home-peek-running': { screen: 'home', since: true, cur: 'run:notes-cli/4' },
    'home-calm': { screen: 'home', calm: true, convState: [['render-core', 5, 'idle'], ['notes-cli', 5, 'idle']], hiddenFaults: ['render-core/19', 'forecasts/9'], cur: 'run:notes-cli/4' },
    'home-1300': { of: 'home', width: 1300 },
    'sidebar-focus': { of: 'home', focus: 'sidebar', sideCur: 'decision render-core/w1:3:design#2' },
    'conversation-streaming': { screen: 'conv', conv: 'notes-cli/4', later: [0, 1] },
    'conversation-steer-insert': { screen: 'conv', conv: 'notes-cli/4', later: [0, 1], mode: 'INSERT', draft: "Use node:test's snapshot support for the TOML fixtures.", focus: 'container' },
    'conversation-failed': { screen: 'conv', conv: 'render-core/5' },
    'model-picker': { screen: 'start', mode: 'COMMAND', project: 'render-core', float: 'workflow', focus: 'float' },
    'conversation-joined': { screen: 'conv', conv: 'render-core/7', later: [0], extraOpen: { project: 'render-core', kind: 'conversation', id: 7 }, mode: 'INSERT', draft: '', focus: 'container' },
    'conversation-records': { screen: 'conv', conv: 'render-core/3', extraOpen: { project: 'render-core', kind: 'conversation', id: 3 }, recCur: 2, details: true },
    'conversation-owned': { screen: 'conv', conv: 'render-core/8', extraOpen: { project: 'render-core', kind: 'conversation', id: 8 } },
    'decision-long': { screen: 'dec', dec: 'render-core/w1:3:design#2', docCur: 0 },
    'decision-note-insert': { screen: 'dec', dec: 'render-core/w1:3:design#2', drafts: [0, 2], mode: 'INSERT', focus: 'container', composing: { text: 'Byte-equal only while history keeps its key order across core versions. Say whether it does.', sel: 'two exports of the same store are byte-equal.' } },
    'readback-answer': { screen: 'dec', dec: 'render-core/w1:3:design#2', docCur: 0, float: 'readback', pick: 'revise', mode: 'READBACK', noteAt: 150, echo: 'read it first', focus: 'float' },
    'decision-forecast': { screen: 'dec', dec: 'forecasts/w2:2:pi-durable-2#1', choiceCur: 'no', drafts: [] },
    'decision-superseded': { screen: 'dec', dec: 'render-core/w1:3:design#2', docCur: 0, inboxLater: [2] },
    'answer-conflict': { screen: 'dec', dec: 'render-core/w1:3:design#2', docCur: 0, float: 'conflict', noteAt: 150, mode: 'READBACK', plainBadge: true, focus: 'float', replyTaken: true },
    'decision-then-faulted': { screen: 'dec', dec: 'render-core/w1:6:design#1', extraOpen: { project: 'render-core', kind: 'decision', id: 'w1:6:design#1' }, drafts: [], blockCur: 0 },
    'readback-send-again': { screen: 'dec', dec: 'render-core/w1:6:design#1', extraOpen: { project: 'render-core', kind: 'decision', id: 'w1:6:design#1' }, drafts: [], blockCur: 0, float: 'again', mode: 'READBACK', focus: 'float' },
    timeline: { screen: 'timeline', since: true, cur: 'ev:render-core/35' },
    faults: { screen: 'faults', cur: 0 },
    conversations: { screen: 'convs', cur: 'render-core/7' },
    project: { screen: 'project', project: 'render-core', cur: 'w1/reviewer' },
    'options-error': { screen: 'project', project: 'scratch', scratch: true, cur: 'w1/designer', extraOpen: { project: 'scratch', kind: 'project' }, optDraft: { 'w1/designer': 'anthropic/claude-opus-9' }, optErr: true, echo: 'The options of w1 do not fit.' },
    'which-key-root': { screen: 'home', since: true, cur: 'dec:0', float: 'wk-root', showcmd: '␣' },
    'which-key-answer': { screen: 'dec', dec: 'render-core/w1:3:design#2', docCur: 0, float: 'wk-a', showcmd: '␣a' },
    'server-stopped': { screen: 'conv', conv: 'notes-cli/4', later: [0, 1], conn: 'stopped', down: true, sideOff: true, echo: 'The server is stopping. tau reconnects when it is back.' },
    offline: { screen: 'home', since: true, cur: 'dec:0', conn: 'offline', unknown: true, at: FX.now + 7 * 60000 },
    'unknown-outcome': { screen: 'conv', conv: 'notes-cli/4', later: [0, 1], conn: 'reconnecting', down: true, unknown: true, float: 'outbox', mode: 'OUTBOX', focus: 'float' },
    revoked: { screen: 'dec', dec: 'render-core/w1:3:design#2', docCur: 0, conn: 'revoked', down: true },
    // added by the calm challenge
    'home-connecting': { screen: 'home', connecting: true, conn: 'connecting', cur: '' },
    'start-starting': { screen: 'start', mode: 'INSERT', project: 'render-core', text: DRAFT.text, starting: true, focus: 'container' },
    'sidebar-focus-decision': { of: 'decision-long', focus: 'sidebar', sideCur: 'decision render-core/w1:3:design#2' },
    'conversation-hostile': { screen: 'conv', conv: 'hostile/2', hostile: true, extraOpenAll: [{ project: 'hostile', kind: 'conversation', id: 2 }, { project: 'hostile', kind: 'decision', id: 'w1:4:design#1' }, { project: 'hostile', kind: 'conversation', id: 3 }] },
    'home-light': { of: 'home', light: true },
    'decision-light': { of: 'decision-note-insert', light: true },
    'narrow-800': { sheet: ['narrow-home', 'narrow-conversation', 'narrow-decision'], width: 800, labels: ['home at 800 px · the sidebar hidden by itself', 'conversation at 800 px', 'decision at 800 px · notes inline'] },
    'narrow-home': { of: 'home', width: 800, sub: true },
    'narrow-conversation': { of: 'conversation-streaming', width: 800, sub: true },
    'narrow-decision': { of: 'decision-long', width: 800, sub: true },
  };
  const resolve = (name) => { const s = S[name]; if (!s || !s.of) return { ...s }; const base = resolve(s.of); return { ...base, ...s, of: undefined, sub: s.sub, focus: s.focus || base.focus }; };

  // ───────────── sidebar (spec §4.1) ─────────────
  const convState = (W, p, id) => ((W.convs[p] || []).find((c) => c.conversation === +id) || {}).state;
  function openGroups(s) {
    if (s.noOpen) return [];
    const groups = clone(L.openItems.groups);
    for (const x of [...(s.extraOpen ? [s.extraOpen] : []), ...(s.extraOpenAll || [])]) {
      let g = groups.find((y) => y.project === x.project);
      if (!g) { g = { project: x.project, items: [] }; groups.push(g); }
      if (!g.items.some((i) => i.kind === x.kind && i.id === x.id)) g.items.push({ kind: x.kind, id: x.id });
    }
    return groups;
  }
  function sidebarHTML(s, W, ctx) {
    const v = s.screen;
    const sideFocus = ctx.focus === 'sidebar';
    const curKey = (k) => (sideFocus && s.sideCur === k ? ' scur' : '');
    const place = (icon, lb, on, rt = '', key = '') => `<div class="row place${on ? ' sel' : ''}${curKey(key)}">${I(icon)}<span class="lbx">${lb}</span>${rt ? `<span class="rt">${rt}</span>` : ''}</div>`;
    const active = s.screen === 'conv' ? `conversation ${s.conv}` : s.screen === 'dec' ? `decision ${s.dec}` : s.screen === 'project' ? `project ${s.project}` : null;
    const open = openGroups(s).map(({ project: p, items }) => `<div class="pgl">${p}</div>` + items.map((it) => {
      const key = it.kind === 'project' ? `project ${p}` : `${it.kind} ${p}/${it.id}`;
      let icon, body, mark = '';
      if (it.kind === 'conversation') {
        icon = 'chat'; const st = convState(W, p, it.id);
        body = `<span class="id pre">#${it.id}</span><span class="lb" data-spoof>${plain(label(p, it.id) || 'empty conversation')}</span>`;
        mark = st === 'running' ? MK.run : st === 'failed' ? MK.fail : '';
      } else if (it.kind === 'decision') {
        const R = decisionRec(p, it.id); const d = R.decision; icon = 'diamond-o';
        body = `<span class="lb" data-spoof>${plain(d.title)}</span><span class="id">#${it.id.split('#')[1]}</span>`;
        const live = W.inbox[p] && W.inbox[p][d.subject] && W.inbox[p][d.subject].id === d.id;
        const tf = R.then && R.then.outcome;
        mark = live && !(s.replyTaken && `${p}/${it.id}` === s.dec) ? MK.dec : (d.status === 'answered' && tf && tf.status === 'faulted') ? MK.fail : '';
      } else { icon = 'folder'; body = `<span class="lb pjl">${p}</span>`; }
      if (s.connecting) mark = '';
      return `<div class="row it${key === active ? ' sel' : ''}${curKey(key)}">${I(icon)}${body}<span class="rt st">${mark}</span></div>`;
    }).join('')).join('');
    const c = s.conn || 'ok';
    const srv = c === 'connecting' ? `<span class="srv bad">${I('ring')}connecting</span>`
      : c === 'stopped' ? `<span class="srv bad">${I('time')}server stopped · retrying</span>`
      : c === 'offline' ? `<span class="srv bad">${I('ring')}offline 6 m</span>`
      : c === 'reconnecting' ? `<span class="srv bad">${I('ring')}reconnecting in 4 s</span>`
      : c === 'revoked' ? `<span class="srv bad">${MK.fail}revoked</span>`
      : `<span class="srv"><span class="hst">${L.config.device} → ${FX.server.url.replace(/^https?:\/\//, '')}</span>${I('chevron--down')}</span>`;
    const homeOn = v === 'home' || v === 'faults';
    return `<aside class="side${sideFocus ? ' focused' : ''}" data-ctx="${sideFocus ? 'sidebar' : 'global'}"><div class="brand"><img class="appicon" src="../../design/brand/tau-icon-small-coral.svg" alt=""><div class="bt"><div class="bn">tau</div>${srv}</div></div>
      <div class="nav">${place('add', 'New', v === 'start', K('␣cn'))}${place('search', 'Find', false, K('␣.'))}</div>
      <div class="nav">${place('home', 'Home', homeOn, `<span class="tags">${tagsHTML(ctx)}</span>`, 'home')}${place('chat', 'Conversations', v === 'convs')}${place('time', 'Timeline', v === 'timeline')}${place('folder', 'Projects', false)}</div>
      ${open ? `<div class="seclab">Open</div><div class="opens">${open}</div>` : ''}
      <div class="sfoot">${place('settings', 'Server')}</div></aside>`;
  }
  function headHTML(s, ctx, crumb, acts = '') {
    if (s.screen === 'connect') return '<div class="shead"></div>';
    return `<div class="shead" data-ctx="${ctx.viewCtx}"><span class="tog">${I(ctx.sideHidden ? 'side-panel--open' : 'side-panel--close')}</span><span class="crumb">${crumb}</span><span class="acts">${acts}</span></div>`;
  }
  // one bottom bar (spec §4.3): badge, pending keys, echo; at the right the save state, then (sidebar hidden) tags and server trouble
  function statusHTML(s, ctx) {
    const m = s.mode || 'NORMAL';
    const cls = { NORMAL: 'normal', INSERT: 'insert', VISUAL: 'visual', COMMAND: 'command', READBACK: s.plainBadge ? 'readback-plain' : 'readback', OUTBOX: 'outbox', SERVER: 'server', PEEK: 'peek' }[m];
    const c = s.conn || 'ok';
    const trouble = c === 'connecting' ? `${I('ring')}connecting` : c === 'stopped' ? `${I('time')}server stopped · retrying` : c === 'offline' ? `${I('ring')}offline 6 m` : c === 'reconnecting' ? `${I('ring')}reconnecting in 4 s` : c === 'revoked' ? `${MK.fail}revoked` : '';
    const right = ctx.sideHidden && s.screen !== 'connect' ? `<span class="tags">${tagsHTML(ctx)}</span>${trouble ? `<span class="srvb">${trouble}</span>` : ''}` : '';
    return `<footer class="status" data-ctx="${ctx.viewCtx}"><span class="mode m-${cls}">${m}</span>${s.showcmd ? `<span class="showcmd" data-key="${esc(s.showcmd)}">${s.showcmd.replace(/␣/g, I('space'))}</span>` : ''}<span class="echo">${s.echo ? esc(s.echo) : ''}</span><span class="spacer"></span>${ctx.save || ''}${right}</footer>`;
  }

  // ───────────── Home (spec §5.3) ─────────────
  function needRow(d, sel, W, ctx) {
    const isNew = d.requestedAt > W.base;
    const head = sel && ctx.focus === 'stage';
    return `<div class="rw two need${sel ? ' sel' : ''}${head ? ' head' : ''}" data-row="need"${isNew ? ' data-new' : ''}><div class="l1"><span class="mk">${MK.dec}</span><span class="tx mv" data-spoof style="flex:0 1 auto">${plain(d.title)}</span><span class="tg">#${d.n}</span><span class="tm">${age(d.requestedAt)}</span>${head ? K('⏎') : ''}</div>
      <div class="l2"><span class="pj">${d.project}</span> ${dot} <span class="sj">${esc(d.subject)}</span></div></div>`;
  }
  // fixed slots: project, #id, extension, reason (or model), time; dropped whole by layout() (spec §4.7)
  const slots = ({ p, id, e, rs, model, tm, hasTm }) => `<span class="sl pr">${p}</span><span class="sl tg">#${id}</span><span class="sl ch">${e || ''}</span><span class="sl ${model ? 'mdl' : 'rs'}">${rs || ''}</span>${hasTm ? `<span class="sl tm">${tm || ''}</span>` : ''}`;
  function failedRowHTML(r, sel) {
    if (r.fault) {
      const f = r.fault;
      return `<div class="rw one line failed${sel ? ' sel' : ''}" data-row="failed"><div class="l1"><span class="mk">${MK.fail}</span><span class="tx fl"><span class="kind">${esc(f.kind)}</span>${f.decision ? `<span class="aft">after ${esc(f.decision)}</span>` : ''}</span>${slots({ p: f.project, id: f.conversation, e: f.kind.split('.')[0], rs: 'task faulted', tm: when(f.endedAt), hasTm: true })}</div></div>`;
    }
    const { project: p, id } = r.conv; const e = extOf(p, id);
    return `<div class="rw one line failed${sel ? ' sel' : ''}" data-row="failed"><div class="l1"><span class="mk">${MK.fail}</span><span class="tx" data-spoof>${label(p, id) ? plain(label(p, id)) : '<span class="mu">empty conversation</span>'}</span>${slots({ p, id, e, rs: 'run failed', hasTm: true })}</div></div>`;
  }
  function runRowHTML(r, sel) {
    const { project: p, id } = r; const e = extOf(p, id), m = modelOf(p, id);
    return `<div class="rw one line run${sel ? ' sel' : ''}" data-row="running"><div class="l1"><span class="mk">${MK.run}</span><span class="tx" data-spoof>${plain(label(p, id) || 'empty conversation')}</span>${slots({ p, id, e, rs: m ? modelShort(m) : '', model: true })}</div></div>`;
  }
  function eventRow(e, W, sel) {
    const isNew = W.showNew && e.data.at > W.base;
    return `<div class="rw one ev${isNew ? ' new' : ''}${sel ? ' sel' : ''}" data-row="event"><div class="l1"><span class="tm">${when(e.data.at).replace(/^(\w+ \d+) \d\d:\d\d$/, '$1')}</span><span class="tx" data-spoof style="flex:0 1 auto">${plain(e.data.text)}</span><span class="pr">${e.project}</span>${e.data.conversation ? `<span class="tg">#${e.data.conversation}</span>` : ''}</div></div>`;
  }
  function sinceLine(W, s) {
    const nd = W.decisions.filter((d) => d.requestedAt > W.base).length, nf = W.faults.filter((f) => f.endedAt > W.base).length, ne = W.events.filter((e) => e.data.at > W.base).length;
    const away = Math.round(((s.at || FX.now) - W.base) / 60000);
    const part = (h) => `<span class="sp">${h}</span>`;
    return `<div class="since">${I('time')}${part(`Since ${hm(W.base)} on this device`)}${part(`${dot}away ${Math.floor(away / 60)} h ${away % 60} m`)}${nd ? part(`${dot}<span data-count="new-dec">${nd}</span> new for you`) : ''}${nf ? part(`${dot}<span data-count="new-fault">${nf}</span> new fault${nf > 1 ? 's' : ''}`) : ''}${ne ? part(`${dot}<span data-count="new-ev">${ne}</span> events`) : ''}</div>`;
  }
  function homeHTML(s, W, ctx) {
    const fr = failedRows(W), rr = runningRows(W);
    const [ck, cv] = (s.cur || '').split(':');
    let curRow = null, h = '';
    if (!W.projects.length) {
      h += `<div class="ptitle h1">Nothing needs you</div><div class="empty" data-ctx="list">No projects yet ${dot} ${K('␣pb')} binds one</div>`;
      ctx.crumb = crumbOf(['Home']);
      return `<div class="lcol" data-ctx="list">${h}</div>`;
    }
    if (s.connecting) {
      // nothing received yet (spec §4.8): no calm claim, no counts, the peek column empty
      ctx.crumb = crumbOf(['Home']);
      return `<div class="lcol" data-ctx="list"><div class="ptitle h1">Home</div><div class="empty">Nothing received from the server yet</div></div>${ctx.peek ? '<aside class="peek" data-ctx="list"></aside>' : ''}`;
    }
    h += W.decisions.length ? `<div class="ptitle h1">Needs you <span class="tag t-needs ttag">${I('diamond')}<span data-count="needs">${W.decisions.length}</span></span></div>` : `<div class="ptitle h1">Nothing needs you</div>`;
    if (s.since) h += sinceLine(W, s);
    if (W.decisions.length) h += `<div class="rows first">${W.decisions.map((d, i) => { const sel = ck === 'dec' && +cv === i; if (sel) curRow = { kind: 'dec', d }; return needRow(d, sel, W, ctx); }).join('')}</div>`;
    if (fr.length) {
      const shown = fr.slice(0, 8);
      const over = fr.slice(8), oRuns = over.filter((r) => r.conv).length, oFaults = over.length - oRuns;
      const more = (oRuns ? `<div class="more more-failed">${oRuns} more failed run${oRuns > 1 ? 's' : ''} · in Conversations</div>` : '') + (oFaults ? `<div class="more more-failed">${oFaults} more faulted task${oFaults > 1 ? 's' : ''} · in Faults</div>` : '');
      h += `<div class="hgrp"><div class="gh">Failed <span class="n" data-count="failed">${fr.length}</span></div><div class="rows slotted">${shown.map((r) => failedRowHTML(r, false)).join('')}</div>${more}</div>`;
    }
    if (rr.length) h += `<div class="hgrp"><div class="gh">Running <span class="n" data-count="running">${rr.length}</span></div><div class="rows slotted">${rr.map((r) => { const sel = ck === 'run' && cv === `${r.project}/${r.id}`; if (sel) curRow = { kind: 'run', r }; return runRowHTML(r, sel); }).join('')}</div></div>`;
    const ev = W.events.slice(0, 12);
    h += `<div class="hgrp"><div class="gh">Activity</div>${ev.length ? `<div class="rows">${ev.map((e) => eventRow(e, W, false)).join('')}</div>${W.events.length > 12 ? `<div class="more">${I('arrow--down')}${W.events.length - 12} older in the timeline</div>` : ''}` : '<div class="empty">No events yet. Bare conversations write none.</div>'}</div>`;
    ctx.crumb = crumbOf(curRow && curRow.kind === 'dec' ? ['Home', 'Needs you', `1 of ${W.decisions.length}`] : ['Home', 'Running', `${rr.findIndex((r) => `${r.project}/${r.id}` === cv) + 1} of ${rr.length}`]);
    const peek = ctx.peek && curRow ? `<aside class="peek" data-ctx="list">${curRow.kind === 'dec' ? decisionPeek(curRow.d, s) : convPeek(curRow.r.project, curRow.r.id, s)}</aside>` : '';
    return `<div class="lcol" data-ctx="list">${h}</div>${peek}`;
  }
  function decisionPeek(d, s) {
    const R = decisionRec(d.project, d.id), D = R.decision;
    return `<div class="pt"><span>${plain(D.title)}</span><span class="id">#${d.n}</span></div>
      <div class="pf"><span class="pj">${d.project}</span> ${dot} <span class="sj">${esc(D.subject)}</span> ${dot} requested ${hm(D.requestedAt)} ${dot} in <span class="id">#${D.conversation}</span> ${dot} basis entry ${D.basis}${s.conn === 'offline' ? ` ${dot} as of 16:41` : ''}</div>
      <div class="pc">${D.choices.map((c) => `<b>${esc(c)}</b>`).join(` ${dot} `)}</div>
      <div class="ph">basis<span class="rr">${R.text.length.toLocaleString('en')} characters</span></div>
      <div class="cbar md" data-content data-fill>${marked.parse(R.text)}</div><div class="cut" data-cut>${I('arrow--down')}the rest is in the decision</div>`;
  }
  function convPeek(p, id, s) {
    const key = `${p} #${id}`; const v = convView(key, []); const T = transcript(v);
    const ag = T.agent; const model = ag.model ? ag.model.modelId : modelShort(modelOf(p, id));
    const sel = ag.extensions == null ? 'project default' : Array.isArray(ag.extensions) ? `only ${ag.extensions.join(', ')}` : `project default + ${ag.extensions.add.join(', ')}`;
    const run = T.live.run; const fail = T.live.runFailure;
    const state = run ? `running · turn ${run.turns}` : fail ? 'run failed' : 'idle';
    let lastText = null; let runTool = null;
    for (const it of T.items) if (it.t === 'agent') for (const b of it.blocks) { if (b.t === 'text') lastText = b.text; if (b.t === 'tool' && b.live) runTool = b; }
    const gen = T.items.find((x) => x.t === 'gen');
    let h = `<div class="pt"><span data-spoof style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${plain(label(p, id) || 'empty conversation')}</span></div>
      <div class="pf"><span class="pj">${p}</span> ${dot} <span class="id">#${id}</span> ${dot} ${esc(model)}${ag.thinkingLevel ? ` ${dot} ${ag.thinkingLevel}` : ''} ${dot} ${sel}</div>
      <div class="pc" style="display:flex;align-items:center;gap:4px">${run ? MK.run : fail ? MK.fail : ''}<b>${state}</b>${s.conn === 'offline' ? ` ${dot} as of 16:41` : ''}</div>`;
    if (gen) h += `<div class="ph">agent ${dot} writing</div><div class="cbar md" data-content><p class="writing">${mdInline(gen.text)}</p></div>`;
    else if (lastText) h += `<div class="ph">last agent text</div><div class="cbar md" data-content>${marked.parse(lastText)}</div>`;
    if (runTool) { const lines = runTool.live.output.replace(/\n$/, '').split('\n').slice(-4); h += `<div class="tool run" data-tool><span class="ic">${MK.run}</span><span class="tn">${esc(runTool.name)}</span><span class="ta">${esc(toolArgs(runTool))}</span><span class="oc"><span class="w">running</span></span></div><div class="tout">${esc(lines.join('\n'))}</div>`; }
    if (fail) h += `<div class="kvp"><span class="k">reason</span><span>${esc(fail.reason)}</span><span class="k">message</span><span class="err">${esc(fail.detail)}</span></div>`;
    if (T.inbox.length) h += `<div class="pc">${T.inbox.length} queued ${dot} ${T.inbox.map((q) => (q.mode === 'followUp' ? 'follow-up' : q.mode)).join(', ')}</div>`;
    return h;
  }

  // ───────────── the start page (spec §5.2) ─────────────
  function startHTML(s, W, ctx) {
    const ins = s.mode === 'INSERT';
    ctx.crumb = crumbOf(['New conversation']); ctx.viewCtx = 'start';
    const P = W.projects.find((x) => x.project === s.project);
    const opts = (C[`options ${s.project}`] || { result: { items: [] } }).result.items;
    const wfOpt = opts.find((o) => o.extension === 'w1');
    const missing = wfOpt ? wfOpt.errors.map((e) => e.path.slice(1)) : [];
    const needs = missing.length ? `<span class="dt">·</span><span class="rosew">needs options: ${missing.join(', ')}</span>` : '';
    // the start editor has the keys in Insert; while create is in flight (TC-02) the text is read-only and its keys are off
    const keys = ins && !s.starting && ctx.focus === 'container';
    const hint = s.starting ? '<span class="starting">starting…</span>' : missing.length ? `${H('⌘⏎', '', keys)}<span>set its options</span>` : `${H('⌘⏎', '', keys)}<span>start</span>`;
    const caret = keys ? '<span class="bar"></span>' : '';
    const text = s.text ? `${esc(s.text)}${caret}` : `${caret}<span class="ph">What do you want to work on?</span>`;
    const noReply = s.noReply ? `<div class="startnote">${MK.unk}<span>No reply ${dot} a conversation may have been created; it would show as empty conversation in <span class="pj">${s.project}</span></span></div>` : '';
    ctx.starting = !!s.starting;
    return `<div class="start"><img class="wordmark" src="../../design/brand/tau-wordmark-on-${s.light ? 'light' : 'dark'}-coral.svg" alt="tau">
      <div class="comp box${keys ? ' ins' : ''}${s.starting ? ' ro' : ''}"><div class="cb">${text}</div><div class="ft">${ins ? hint : ''}</div></div>
      <div class="startline">${I('folder')}<span class="pj">${P.project}</span>${H('M-p', '', keys)}${dot}<span class="wf">w1</span><span class="al">workflow</span>${needs}${H('M-w', '', keys)}</div>${noReply}</div>`;
  }

  // ───────────── Conversation (spec §5.5) ─────────────
  function convHTML(s, W, ctx) {
    const [p, id] = s.conv.split('/'); const key = `${p} #${id}`;
    const v = convView(key, s.later || []); const T = transcript(v);
    const ag = T.agent, run = T.live.run, fail = T.live.runFailure;
    const model = ag.model ? ag.model.modelId : '';
    const toolsSel = ag.extensions == null ? 'project default' : Array.isArray(ag.extensions) ? `only ${ag.extensions.join(', ')}` : `project default + ${ag.extensions.add.join(', ')}`;
    const wf = ag.extensions && (Array.isArray(ag.extensions) ? ag.extensions : ag.extensions.add).find((x) => C.extensions.result.items.some((e) => e.name === x && e.workflow));
    ctx.conv = run ? 'running' : fail ? 'failed' : 'idle';
    const off = !!s.down || s.conn === 'stopped' || s.conn === 'offline' || s.conn === 'revoked';
    ctx.writesOff = off;
    const owner = (W.convs[p] || []).find((c) => c.conversation === +id)?.owner;
    let facts, acts = '';
    if (run) { facts = `${MK.run}<span>running</span>${dot}${esc(model)}${dot}turn ${run.turns}`; acts = off ? segOff('stop') : seg('␣cx', 'stop'); }
    else if (fail) { facts = `${MK.fail}<span>run failed</span>${dot}${esc(fail.reason)}${dot}${esc(model)}`; acts = off ? segOff('continue') : seg('␣cc', 'continue'); }
    else facts = `<span>idle</span>${dot}${esc(model)}`;
    if (owner) facts += `${dot}owned by <span class="id">#${owner.conversationId}</span>`;
    if (s.down) facts += `${dot}as of ${hm(FX.now)}`; // §4.9: the view's stream is down, its state is marked
    const decs = W.decisions.filter((d) => d.project === p && d.conversation === +id);
    const evs = W.events.filter((e) => e.project === p && e.data.conversation === +id).slice(0, 5);
    let dp = '';
    if (s.details) {
      const fullModel = `${ag.model.provider}/${model}`;
      dp = `<aside class="dpanel" data-ctx="conversation"><div class="lg"><div class="lh">Settings</div>
        <div class="li"><span class="k">model</span><span class="v"><span class="mono nobr">${esc(fullModel)}</span> ${dot} ${esc(modelName(fullModel) || '')}</span></div>
        <div class="li"><span class="k">thinking</span><span class="v">${ag.thinkingLevel || 'model default'}</span></div>
        <div class="li"><span class="k">tools</span><span class="v">${toolsSel}</span></div>
        <div class="li"><span class="k">cwd</span><span class="v mono">${esc(leaf(ag.cwd || ''))}</span></div>
        ${wf ? `<div class="note2">${wf} may set the model again when it next sends work here</div>` : ''}</div>`;
      if (decs.length) dp += `<div class="lg"><div class="lh">Decisions</div>${decs.map((d) => `<div class="lk">${MK.dec}<span class="x" data-spoof>${plain(d.title)}</span><span class="id">#${d.n}</span></div>`).join('')}</div>`;
      if (evs.length) dp += `<div class="lg"><div class="lh">Events</div>${evs.map((e) => `<div class="lk"><span class="tm">${when(e.data.at)}</span><span class="x" data-spoof>${plain(e.data.text)}</span></div>`).join('')}</div>`;
      dp += '</aside>';
    }
    let stack = '';
    if (T.inbox.length) stack += T.inbox.map((q) => `<div class="qrow"><div class="qm">${q.mode === 'followUp' ? 'follow-up' : q.mode} <span class="dt">· queued</span></div><div class="qt" data-content>${mdInline(q.content)}</div></div>`).join('') + `<div class="qnote">queued inputs can't be taken back</div>`;
    const unk = W.outbox.filter((o) => o.state === 'unknown' && o.body.conversation && `${o.body.project}/${o.body.conversation}` === s.conv);
    stack += unk.map((o) => `<div class="prow2" data-row="unknown"><div class="qm">${MK.unk}<span>${o.body.whenBusy === 'followUp' ? 'follow-up' : 'steer'}</span></div><div class="qt" data-content>${mdInline(o.body.text)}</div><div class="qw">no reply ${dot} resends when connected</div></div>`).join('');
    const ins = s.mode === 'INSERT';
    const head = run ? `<b>steer</b> ${dot} joins this run after the current tool round` : fail ? `<b>send</b> ${dot} starts a new run ${dot} the last run failed` : `<b>send</b> ${dot} starts a run on ${esc(model)}`;
    const lit = ins && ctx.focus === 'container';
    const foot = ins ? `<div class="ft">${H('Esc', '', lit)}${H('jk', '', lit)}<span>leave Insert</span><span class="gap"></span>${H('⌘⏎', '', lit)}<span>${run ? 'steer' : 'send'}</span>${run ? `<span class="gap"></span>${H('M-⏎', '', lit)}<span>follow-up</span>` : ''}</div>` : '';
    stack += `<div class="comp box${ins ? ' ins' : ''}"><div class="cap">${head}</div><div class="cb">${ins ? `${esc(s.draft || '')}<span class="bar"></span>` : ''}</div>${foot}</div>`;
    let tail = '';
    if (fail) tail = `<div class="rfail"><div class="rfh">${MK.fail}<span>Run failed</span><span class="w">· ${esc(fail.reason)}</span></div><div class="f2">${esc(fail.detail)}</div><div class="f3">${K('␣cc')}<span>continue runs it again on the same context</span></div></div>`;
    ctx.crumb = crumbOf([`<span class="pj">${p}</span>`, `<span class="id">#${id}</span>`]);
    ctx.viewCtx = 'conversation';
    const tb = titleBlock(`<span data-spoof class="tl">${plain(label(p, id) || 'empty conversation')}</span>`, facts, { details: s.details });
    return { acts, stage: `<div class="ccol">${tb}<div class="tscroll" data-follow><div class="tin"><div class="doc">${transcriptHTML(T, { recCur: s.recCur, factsModel: model })}${tail}</div></div></div><div class="cstack">${stack}</div></div>${dp}` };
  }

  // ───────────── Decision (spec §5.6) ─────────────
  function decHTML(s, W, ctx) {
    const slash = s.dec.indexOf('/'); const p = s.dec.slice(0, slash), id = s.dec.slice(slash + 1);
    const R = decisionRec(p, id); const ext = R.decision.subject.split(':')[0];
    // after a conflict reply the view takes the reply's record (TC-21)
    const D = s.replyTaken ? { ...R.decision, ...FX.refusals['conflict (answer w1:3:design#2 after the phone answered approve)'].reply.refused.decision } : R.decision;
    const openNow = W.inbox[p] && W.inbox[p][D.subject];
    const superseded = D.status === 'open' && openNow && openNow.id !== D.id;
    if (s.replyTaken) ctx.replyTaken = true;
    const drafts = D.status === 'open' ? (s.drafts ? (L.noteDrafts[s.dec] || []).filter((_, i) => s.drafts.includes(i)) : (L.noteDrafts[s.dec] || [])) : [];
    const range = drafts.filter((d) => d.range), whole = drafts.filter((d) => !d.range);
    const writesOff = s.conn === 'offline' || s.conn === 'revoked';
    const tf = !s.replyTaken && R.then && R.then.outcome; const thenFaulted = D.status === 'answered' && tf && tf.status === 'faulted';
    ctx.dec = { status: superseded ? 'superseded' : D.status, thenFaulted, writesOff };
    ctx.viewCtx = 'decision';
    let facts, acts = '';
    if (superseded) facts = `superseded by <span class="id">#${openNow.id.split('#')[1]}</span>`;
    else if (D.status === 'open') facts = `open${dot}requested ${hm(D.requestedAt)}${dot}${age(D.requestedAt)} ago`;
    else facts = `answered ${esc(D.choice)}${dot}by ${D.by} ${hm(D.at)}${R.then && !s.replyTaken ? `${dot}then <span class="id">${esc(D.then.task)}</span>${dot}${tf ? (tf.status === 'faulted' ? `${MK.fail}<span>faulted</span>` : `${MK.ok}<span>${tf.status}</span>`) : `${MK.run}running`}` : ''}`;
    if (s.down && D.status === 'open') facts += `${dot}as of ${hm(FX.now)}`; // §4.9
    if (D.status === 'open' && !superseded) acts = writesOff ? segOff('Answer') : `<span class="seg prim">${I('diamond')}${K('␣ad')}<span>Answer</span></span>`;
    if (thenFaulted) acts = writesOff ? segOff('send again') : seg('␣ar', 'send again');
    const earlier = [];
    for (let k = +id.split('#')[1] - 1; k >= 1; k--) { const e = decisionRec(p, `${D.subject}#${k}`); if (e) earlier.push(e.decision); }
    let doc = titleBlock(plain(D.title), facts, { details: s.details });
    if (thenFaulted) doc += `<div class="faultblk"><div class="rfh">${MK.fail}<span>The work after this answer failed</span></div><div class="f2">${esc(D.then.task)} ${dot} ${esc(tf.error.message)}</div></div>`;
    doc += `${mdBlocks(R.text, 'basis')}`;
    doc += `<div class="cgh">Choices</div><div class="choices">${D.choices.map((c, i) => `<div class="cho${s.choiceCur === c ? ' sel' : ''}"${D.choice === c ? ' data-chosen' : ''}>${D.choice === c ? `<span class="cm">●</span>` : ''}<span class="cn">${i + 1}</span><span>${esc(c)}</span>${D.choice === c ? `<span class="cb">chosen by ${D.by} ${dot} ${hm(D.at)}</span>` : ''}</div>`).join('')}</div>`;
    doc += '<div style="height:320px"></div>';
    // the notes margin, from the top of the title block down (spec §5.6)
    const nNotes = drafts.length + (s.composing ? 1 : 0);
    let mhead;
    if (D.status === 'open') mhead = nNotes ? `<div class="mhd">${I('edit')}<b>Your notes</b>${dot}<span data-count="notes">${nNotes}</span>${dot}not sent</div>` : `<div class="mhd">${I('edit')}<b>Your notes</b>${dot}none</div>`;
    else mhead = D.annotations.length ? `<div class="mhd"><b>Notes</b>${dot}${D.annotations.length}${dot}sent by ${D.by} ${hm(D.at)}</div>` : `<div class="mhd"><b>Notes</b>${dot}none${dot}answered by ${D.by} ${hm(D.at)}</div>`;
    const marg = `<div class="mtop">${mhead}
        ${whole.map((w) => `<div class="mwhole" data-note="whole"><div class="nl">on the whole</div><div class="nb" data-content>${esc(w.text)}</div></div>`).join('')}
        ${earlier.map((e) => `<div class="earl">${I('chevron--right')}<span>Earlier:</span><span class="id">#${e.id.split('#')[1]}</span><span>${esc(e.choice)}${dot}${e.annotations.length} note${e.annotations.length === 1 ? '' : 's'}</span></div>`).join('')}</div>`;
    // folded margin (TC-12): under 1016 px its head, the note on the whole and Earlier sit under the facts line
    if (ctx.narrow) doc = doc.replace('</div></div>', `</div></div><div class="mfold">${marg}</div>`);
    let dp = '';
    if (s.details) dp = `<aside class="dpanel"><div class="lg"><div class="lh">About</div><div class="li"><span class="k">extension</span><span class="v">${ext}</span></div><div class="li"><span class="k">conversation</span><span class="v"><span class="id">#${D.conversation}</span></span></div><div class="li"><span class="k">basis</span><span class="v">entry ${D.basis} · ${R.text.length.toLocaleString('en')} characters</span></div><div class="li"><span class="k">then</span><span class="v mono">${esc(D.then.task)}</span></div></div></aside>`;
    let notice = '';
    if (superseded) notice = `<div class="notice">${I('time', 'steel')}<span class="nt">Superseded by <span class="id">#${openNow.id.split('#')[1]}</span> ${dot} this request can no longer be answered</span></div>`;
    if (s.conn === 'revoked') notice = `<div class="notice rose">${MK.fail}<span class="nt">This device was revoked on the server. Nothing more is sent. To use it again, run <span class="mono">tau device pair &lt;new name&gt;</span> on the server (a revoked name can't be reused), then ${K('␣rp')}.</span></div>`;
    ctx.crumb = crumbOf([`<span class="pj">${p}</span>`, `<span class="id">${esc(D.id)}</span>`]);
    ctx.notes = { range, whole: whole.length };
    return { acts, notice, stage: `<div class="wpage"><div class="doc">${doc}</div><div class="dmarg">${ctx.narrow ? '' : marg}</div></div>${dp}` };
  }

  // ───────────── the places (spec §5.8) ─────────────
  function timelineHTML(s, W, ctx) {
    const [, ref] = s.cur.split(':'); const [cp, cid] = ref.split('/');
    let cur = null;
    const rows = W.events.map((e) => { const sel = e.project === cp && e.id === +cid; if (sel) cur = e; return `<div class="lrow${W.showNew && e.data.at > W.base ? ' new' : ''}${sel ? ' sel' : ''}"><span class="tm" style="width:96px">${when(e.data.at)}</span><span class="tx" data-spoof>${plain(e.data.text)}</span><span class="knd">${esc(e.kind)}</span><span class="pr">${e.project}</span><span class="idc">${e.data.conversation ? '#' + e.data.conversation : ''}</span></div>`; }).join('');
    ctx.crumb = crumbOf(['Timeline', 'all projects']);
    const peek = ctx.peek ? `<aside class="peek" data-ctx="list"><div class="pt"><span data-spoof>${plain(cur.data.text)}</span></div><div class="pf"><span class="sj mono">${esc(cur.kind)}</span> ${dot} ${hm(cur.data.at)} ${dot} <span class="pj">${cur.project}</span> ${dot} by task ${cur.byTaskId}</div><div class="sub2">${convPeek(cur.project, cur.data.conversation, s)}</div></aside>` : '';
    return `<div class="lcol" data-ctx="list"><div class="ptitle h1">Timeline</div>${sinceLine(W, s)}<div class="rows first">${rows}</div></div>${peek}`;
  }
  function faultsHTML(s, W, ctx) {
    const fs = [...W.faults].sort((a, b) => b.endedAt - a.endedAt);
    const rows = fs.map((f, i) => `<div class="lrow${s.cur === i ? ' sel' : ''}"><span class="mk">${MK.fail}</span><span class="kind">${esc(f.kind)}</span><span class="er" data-spoof>${plain(f.error)}</span><span class="pr">${f.project}</span><span class="id">#${f.conversation}</span>${f.decision ? `<span class="ds">after ${esc(f.decision)}</span>` : ''}<span class="tm" style="width:92px;text-align:right">${when(f.endedAt)}</span></div>`).join('');
    const f = fs[s.cur]; const R = f.decision && decisionRec(f.project, f.decision); const D = R && R.decision;
    ctx.crumb = crumbOf(['Faults', 'all projects']);
    const peek = ctx.peek ? `<aside class="peek" data-ctx="list"><div class="pt"><span class="mono" style="color:var(--orange);font-weight:400">${esc(f.kind)}</span><span class="al" style="font-weight:400">faulted</span></div>
      <div class="kvp"><span class="k">task</span><span class="mono">${f.task}</span><span class="k">error</span><span class="err" data-content>${esc(f.error)}</span><span class="k">conversation</span><span><span class="pj">${f.project}</span> <span class="id">#${f.conversation}</span></span><span class="k">ended</span><span class="mono">${when(f.endedAt)}</span></div>
      ${D ? `<div class="sub2"><div class="pt"><span>${plain(D.title)}</span><span class="id">#${D.id.split('#')[1]}</span></div><div class="pf"><span class="sj">${esc(D.subject)}</span> ${dot} answered <b style="font-weight:600;color:var(--fg)">${esc(D.choice)}</b> by ${D.by} at ${hm(D.at)}</div><div class="pc">Open the decision to send the same answer again. It starts the work after it again.</div></div>` : ''}</aside>` : '';
    return `<div class="lcol" data-ctx="list"><div class="ptitle h1">Faults</div><div class="lede">A faulted task never runs again by itself. If it names a decision, open it and send the answer again. Hiding a fault is local to this device.</div><div class="rows first" data-count-rows="failed-faults">${rows}</div></div>${peek}`;
  }
  function convsHTML(s, W, ctx) {
    let h = '';
    for (const p of W.pnames) {
      const cs = [...W.convs[p]].reverse();
      const nr = cs.filter((c) => c.state === 'running').length, nf = cs.filter((c) => c.state === 'failed').length;
      h += `<div class="lgh" data-group="${p}">${p} <span class="s">${nr ? `<span data-count="run-${p}">${nr}</span> running ${dot} ` : ''}${nf ? `<span data-count="fail-${p}">${nf}</span> failed ${dot} ` : ''}<span data-count="all-${p}">${cs.length}</span> in all</span></div><div class="rows">`;
      h += cs.map((c) => { const id = c.conversation; const sel = s.cur === `${p}/${id}`; const e = extOf(p, id), m = modelOf(p, id); const lb = label(p, id);
        return `<div class="lrow${sel ? ' sel' : ''}" data-crow="${p}" data-cstate="${c.state}"><span class="mk">${c.state === 'running' ? MK.run : c.state === 'failed' ? MK.fail : ''}</span><span class="tx" data-spoof>${lb ? plain(lb) : '<span class="mu">empty conversation</span>'}</span><span class="ext">${e || ''}</span><span class="md2">${m ? modelShort(m) : ''}</span><span class="idc">#${id}</span></div>`; }).join('') + '</div>';
    }
    const [cp, cid] = s.cur.split('/');
    ctx.crumb = crumbOf(['Conversations', 'all projects']);
    return `<div class="lcol" data-ctx="list"><div class="ptitle h1">Conversations</div><div style="margin-top:8px">${h}</div></div>${ctx.peek ? `<aside class="peek" data-ctx="list">${convPeek(cp, cid, s)}</aside>` : ''}`;
  }

  // ───────────── Project view (spec §5.9) ─────────────
  function projectHTML(s, W, ctx) {
    const P = W.projects.find((x) => x.project === s.project);
    const opts = (C[`options ${s.project}`] || { result: { items: [] } }).result.items;
    const extInfo = (n) => C.extensions.result.items.find((e) => e.name === n);
    const draft = s.optDraft || {};
    const cs = W.convs[P.project] || [];
    const nr = cs.filter((c) => c.state === 'running').length, nf = cs.filter((c) => c.state === 'failed').length;
    const nd2 = W.decisions.filter((d) => d.project === P.project).length, nfa = W.faults.filter((f) => f.project === P.project).length;
    let doc = titleBlock(P.project, `working directory <span class="mono fgc">${esc(P.workdir)}</span>${dot}a path on the server`, { noDetails: true });
    doc += `<div class="cgh first">Work</div><div class="fsec">
      <div class="frow"><span class="fw">Conversations</span><span class="fs">${cs.length}${nr ? ` ${dot} ${nr} running` : ''}${nf ? ` ${dot} ${nf} failed` : ''}</span></div>
      <div class="frow"><span class="fw">Open decisions</span><span class="fs">${nd2}</span></div>
      <div class="frow"><span class="fw">Faults</span><span class="fs">${nfa}</span></div></div>`;
    doc += `<div class="cgh">Extensions</div><div class="fsec">${P.extensions.map((n) => { const o = opts.find((x) => x.extension === n); const nerr = o ? o.errors.length : 0;
      return `<div class="frow"><span class="fk">${n}</span><span class="fv al">${extInfo(n).workflow ? 'workflow' : 'tool'}</span><span class="fs${nerr ? ' rosew' : ''}">${nerr ? `${nerr} to set` : 'all set'}</span></div>`; }).join('')}</div>
      <div class="fnote">An extension can't be removed from a project.</div>`;
    for (const o of [...opts].sort((a) => (a.extension === 'w1' ? -1 : 1))) {
      doc += `<div class="cgh">Options ${dot} ${o.extension}</div><div class="fsec">`;
      for (const [k, sch] of Object.entries(o.schema.properties)) {
        const key = `${o.extension}/${k}`; const dv = draft[key]; const v = dv !== undefined ? dv : o.value[k];
        const err = s.optErr && dv !== undefined ? FX.refusals['bad_options (setOptions designer to an unknown model)'].reply.refused.errors.find((e) => e.path === '/' + k) : o.errors.find((e) => e.path === '/' + k);
        let shown;
        if (sch.format === 'model') shown = v ? `<span class="m">${esc(v)}</span>${modelName(v) ? `<span class="nm">${esc(modelName(v))}</span>` : ''}` : '<span class="mu">not set</span>';
        else if (sch.format === 'secret') shown = v ? '<span>set</span> <span class="mu">· hidden</span>' : '<span class="mu">not set</span>';
        else if (v === undefined && sch.default !== undefined) shown = `<span class="mu">default ${esc(sch.default)}</span>`;
        else shown = v === undefined ? '<span class="mu">not set</span>' : `<span class="m">${esc(v)}</span>`;
        const missingNow = !dv && o.errors.some((e) => e.path === '/' + k);
        doc += `<div class="frow${err ? ' two' : ''}${s.cur === key ? ' sel' : ''}">${dv !== undefined ? `<span class="ed">${I('edit')}</span>` : ''}<span class="fk">${k}</span><span class="fv">${shown}</span>${err ? `<span class="ferr">${esc(missingNow ? 'required' : err.message)}</span>` : ''}</div>`;
      }
      doc += '</div>';
    }
    const nd = Object.keys(draft).length;
    ctx.drafts = nd; ctx.viewCtx = 'project';
    ctx.crumb = crumbOf(['Projects', `<span class="pj">${P.project}</span>`]);
    ctx.save = nd ? `<span class="save">${I('edit')}<span>${nd} not saved</span>${K(':w')}</span>` : '';
    return `<div class="wpage" data-ctx="project"><div class="doc">${doc}</div></div>`;
  }

  // ───────────── Connect (spec §5.1) ─────────────
  const insecure = (u) => { const m = /^http:\/\/(\[[^\]]+\]|[^/:]+)/.exec(u); return !!m && !['127.0.0.1', 'localhost', '[::1]'].includes(m[1]); };
  function connectHTML(s, ctx) {
    ctx.crumb = crumbOf(['Connect']); ctx.viewCtx = 'connect';
    const warn = insecure(s.server || 'http://127.0.0.1:6283');
    return `<div class="connect" data-ctx="connect"><img class="wordmark" src="../../design/brand/tau-wordmark-on-${s.light ? 'light' : 'dark'}-coral.svg" alt="tau">
      <div class="ptitle">Connect to a tau server</div>
      <div class="cf"><span class="k">Server</span><span class="fld">${esc(s.server || 'http://127.0.0.1:6283')}</span>${warn ? `<span></span><span class="cwarn">${I('warning--alt')}<span>This address is not encrypted. Anyone on the network path can read this device's token. Use https:// (Tailscale Serve or a TLS proxy) or an SSH tunnel.</span></span>` : ''}<span class="k">Pairing code</span><span class="fld${ctx.focus === 'container' ? ' focus' : ''}">K7QM-3XRT${ctx.focus === 'container' ? '<span class="bar"></span>' : ''}</span></div>
      <p>On the server's machine, run <code>tau device pair &lt;name&gt;</code>, with a new name for this device such as <code>laptop</code>; a name is never used twice. It prints a code that works once, for 10 minutes. The token it gives this device is kept in the macOS Keychain.</p>
      <div class="go"><span class="seg">${K('⏎')}<span>connect</span></span></div></div>`;
  }

  // ───────────── floats: one shell (spec §6) ─────────────
  const shell = ({ tag = '', title, sub = '', meta = '', body, input = '', hints = '' }) => `<div class="float on"><div class="fh">${tag}<span class="tt">${title}</span>${sub ? `<span class="sub">${sub}</span>` : ''}${meta ? `<span class="meta">${meta}</span>` : ''}</div>
    <div class="fb">${body}</div>${input || hints ? `<div class="ff2">${input}<span class="hints">${hints}</span></div>` : ''}</div>`;
  const fk = (keys, word) => `<span class="fkp">${keys.map(K).join('')}<span>${word}</span></span>`;
  const kv = (k, v, cls = '') => `<span class="rk">${k}</span><span class="rv${cls ? ' ' + cls : ''}">${v}</span>`;
  function floatHTML(s, W, ctx) {
    const f = s.float; if (!f) return '';
    ctx.float = f;
    if (f === 'readback') {
      const R = decisionRec('render-core', 'w1:3:design#2'), D = R.decision; const notes = L.noteDrafts[s.dec];
      const nr = notes.filter((x) => x.range).length, nw = notes.length - nr;
      return shell({ ctxName: 'readback', tag: `<span class="vtag">${I('diamond')}ANSWER</span>`, title: `${esc(D.title)} <span class="id">#2</span> <span class="dt">·</span> <span class="pj">render-core</span>`, meta: `open ${dot} requested ${hm(D.requestedAt)}`,
        body: `<div class="rb2"><div class="rbl">${D.choices.map((c, i) => `<div class="prow${s.pick === c ? ' sel' : ''}"><span class="cn">${i + 1}</span><span>${esc(c)}</span></div>`).join('')}<div class="rbhint">${H('j', '', true)} ${H('k', '', true)} or ${H('1–3', '1', true)}</div></div>
          <div class="rbg">${kv('notes', `<span data-count="notes">${notes.length}</span> sent with the answer ${dot} ${nr} on passages, ${nw} on the whole<span class="rkh">${K('K')}reads them</span>`)}
          ${kv('basis', `entry ${D.basis} ${dot} ${R.text.length.toLocaleString('en')} characters ${dot} shown to the end`)}
          ${kv('then', `<span class="id">${D.then.task}</span> runs in <span class="id">#${D.conversation}</span>`)}
          ${kv('from', `this device, ${L.config.device}`)}</div></div>`,
        hints: fk(['y'], 'confirm') + fk(['n', 'Esc', 'q'], 'cancel') });
    }
    if (f === 'again') {
      const R = decisionRec('render-core', 'w1:6:design#1'), D = R.decision;
      return shell({ ctxName: 'readback', tag: `<span class="vtag">${I('diamond')}SEND AGAIN</span>`, title: `${esc(D.title)} <span class="id">#1</span> <span class="dt">·</span> <span class="pj">render-core</span>`, meta: `answered ${esc(D.choice)} ${dot} ${D.by} ${hm(D.at)}`,
        body: `<div class="rbg kv96">${kv('its work', `<span class="id">${D.then.task}</span> faulted ${dot} <span class="mono">${esc(R.then.outcome.error.message)}</span>`, 'cut')}
          ${kv('sends', `the same answer: ${esc(D.choice)}, ${D.annotations.length} notes, basis entry ${D.basis}`)}
          ${kv('then', `a new <span class="id">${D.then.task}</span> starts in <span class="id">#${D.conversation}</span>`)}
          ${kv('from', `this device, ${L.config.device}`)}</div>`,
        hints: fk(['y'], 'confirm') + fk(['n', 'Esc', 'q'], 'cancel') });
    }
    if (f === 'conflict') {
      const R = FX.refusals['conflict (answer w1:3:design#2 after the phone answered approve)'].reply.refused, D = R.decision;
      return shell({ ctxName: 'readback', tag: `<span class="ntag">${I('diamond-o')}NOT RECORDED</span>`, title: `${esc(D.title)} <span class="id">#2</span> <span class="dt">·</span> <span class="pj">render-core</span>`, meta: `answered ${dot} ${D.by} ${hm(D.at)}`,
        body: `<div class="rbg kv96">${kv('reply', `<b>Not recorded. ${D.by} answered ${esc(D.choice)} at ${hm(D.at)}.</b>`)}
          ${kv('their choice', `<span class="cn">${D.choices.indexOf(D.choice) + 1}</span> ${esc(D.choice)}`)}
          ${kv('their notes', `${D.annotations.length} notes`)}
          ${kv('yours', 'your choice revise and your 3 notes were not sent; they stay on this device')}</div>`,
        hints: fk(['Esc'], 'closes · your notes stay on this device') });
    }
    if (f === 'wk-root') {
      const G = [['a', `${I('diamond-o')} answer`], ['c', 'conversation'], ['o', 'open'], ['p', 'project'], ['m', 'this view'], ['r', 'recover'], ['t', 'toggle'], ['h', 'help']];
      const Cm = [['n', 'next decision'], ['.', 'find anything'], [';', 'commands'], ['y', 'yank to clipboard']];
      const cell = ([k, d], g) => `<div class="wk${g ? ' grp' : ''}"><span class="k">${esc(k)}</span><span class="d">${d}</span></div>`;
      return shell({ ctxName: 'global', title: `<span class="ldr">${I('space')}leader</span>`,
        body: `<div class="wkg">${G.map((x) => cell(x, 1)).join('')}</div><div class="wkr"></div><div class="wkg c">${Cm.map((x) => cell(x, 0)).join('')}</div>`, hints: fk(['Esc'], 'closes') });
    }
    if (f === 'wk-a') {
      return shell({ ctxName: 'global', title: 'answer', sub: 'every command reads back what it sends before y',
        body: `<div class="wkg two"><div class="wk grp"><span class="k">d</span><span class="d">${I('diamond-o')} answer this decision</span></div><div class="wk dim"><span class="k">r</span><span class="d">${I('diamond-o')} send the answer again <span class="why">· its work did not fail</span></span></div></div>`, hints: fk(['Esc'], 'closes') });
    }
    if (f === 'confirm-http') {
      return shell({ ctxName: 'confirm', title: 'Confirm', body: '<div class="cfm">Pair over plain HTTP? Anyone on the network path can read this device\'s token.</div>', hints: fk(['y'], 'pairs') + fk(['n'], 'cancels') });
    }
    if (f === 'workflow') {
      // one flat list (TC-32): the project's workflows, then one bare-agent row per model
      const rows = [`<div class="prow sel"><span><span class="mono">w1</span> ${dot} workflow</span></div>`, ...C.models.result.items.map((m) => `<div class="prow"><span>bare agent ${dot} ${esc(m.name)}</span></div>`)];
      return shell({ ctxName: 'picker', title: 'Workflow', sub: `for the new conversation in <span class="pj">${s.project}</span>`, body: rows.join(''),
        input: `<span class="fin"><span class="bc"></span><span class="cnt">${rows.length} rows</span></span>`, hints: fk(['⏎'], 'choose') + fk(['Esc'], 'back') });
    }
    if (f === 'model') {
      const cur = 'anthropic/claude-sonnet-4-5';
      return shell({ ctxName: 'picker', title: 'Model', sub: 'for <span class="id">#5</span>', meta: 'w1 may set the model again when it next sends work here',
        body: C.models.result.items.map((m) => `<div class="prow${m.model === 'anthropic/claude-opus-4-5' ? ' sel' : ''}"><span class="n">${esc(m.name)}</span><span class="s mono">${esc(m.model)}</span>${m.model === cur ? '<span class="x">current</span>' : ''}</div>`).join(''),
        input: `<span class="fin"><span class="bc"></span><span class="cnt">${C.models.result.items.length} models</span></span>`, hints: fk(['⏎'], 'choose') + fk(['Esc'], 'back') });
    }
    if (f === 'project') {
      const rows = W.projects.map((P) => [P.project, P.extensions.length ? P.extensions.slice().sort((a) => (a.startsWith('w') ? -1 : 1)).join(' · ') : 'bare agent only', P.workdir]).sort((a, b) => (a[0] === s.project ? -1 : b[0] === s.project ? 1 : a[0] === 'notes-cli' ? -1 : 1));
      return shell({ ctxName: 'picker', title: 'Project', sub: 'for the new conversation',
        body: rows.map(([n, w, p], i) => `<div class="prow${i === 0 ? ' sel' : ''}">${I('folder')}<span class="n pj">${n}</span><span class="w">${w}</span><span class="s mono">${p}</span><span class="x">${n === L.lastProject ? 'last used' : ''}</span></div>`).join(''),
        input: `<span class="fin"><span class="bc"></span><span class="cnt">${rows.length} projects</span></span>`, hints: fk(['⏎'], 'choose') + fk(['Esc'], 'back') });
    }
    if (f === 'bind') {
      const exts = C.extensions.result.items;
      return shell({ ctxName: 'field', mode: 'i', title: 'Bind a project',
        body: `<div class="fgrid"><span class="k">name</span><span class="v"><span class="fld">scratch</span></span>
          <span class="k">working directory</span><span class="v"><span class="fld" style="min-width:380px">/home/vrod/src/scratch</span><span class="mu">a path on the server</span></span>
          <span class="k" style="align-self:start;padding-top:2px">extensions</span><span class="v"><div class="chk">${exts.map((e) => `<div class="ci${e.name === 'w1' ? ' on sel' : ''}">${I(e.name === 'w1' ? 'checkbox--checked' : 'checkbox')}<span class="n">${e.name}</span><span class="s">${e.workflow ? 'workflow' : 'tool'}</span></div>`).join('')}</div></span></div>`,
        hints: fk(['Tab'], 'toggle') + fk(['⏎'], 'bind') + fk(['Esc'], 'cancel') });
    }
    if (f === 'outbox') {
      const ob = W.outbox;
      const rows = ob.map((o) => `<div class="prow${o.state === 'unknown' ? ' sel' : ''}"><span class="m">${o.state === 'unknown' ? MK.unk : MK.ok}</span><span class="n mono" style="min-width:44px;width:44px">${o.call}</span><span class="s" style="width:120px"><span class="pj">${o.body.project}</span> <span class="id">#${o.body.conversation}</span></span><span class="n" style="flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis" data-spoof>${plain(o.body.text)}</span><span class="x">${o.state === 'unknown' ? '<b>no reply</b> · resends when connected' : 'done'} · ${hm(o.sentAt)}</span></div>`).join('');
      return shell({ ctxName: 'list', title: 'Outbox', sub: 'every write, kept until done', meta: `<span data-count="unknown">${ob.filter((o) => o.state === 'unknown').length}</span> without a reply`, body: rows,
        hints: fk(['␣rt'], 'retry') + fk(['x'], 'forget') + fk(['Esc'], 'closes') });
    }
    return '';
  }

  // ───────────── render ─────────────
  function render(name) {
    const s = resolve(name);
    if (s.at) NOW = s.at;
    const W = world(s);
    const sideHidden = !!(s.noSide || s.sideOff || innerWidth < 1000);
    const sw = innerWidth - (sideHidden ? 0 : 248);
    const focus = s.focus || 'stage';
    const ctx = { narrow: sw < 1016, peek: sw >= 1000, sideHidden, sw, focus, viewCtx: 'list', state: name, mode: s.mode || 'NORMAL', conn: s.conn || 'ok', calm: !W.decisions.length };
    ctx.counts = { needs: W.decisions.length, failed: failedRows(W).length, unknown: W.outbox.filter((o) => o.state === 'unknown').length };
    if (s.screen === 'connect' || !W.projects.length || s.connecting) ctx.counts = { needs: 0, failed: 0, unknown: 0 };
    document.documentElement.classList.toggle('light', !!s.light);
    const st = document.documentElement.style; st.setProperty('--side', sideHidden ? '0px' : '248px'); st.setProperty('--sw', `${sw}px`);
    const dpw = s.details ? 240 : 0, avail = sw - dpw;
    const narrowDec = avail < 1016;
    st.setProperty('--dx', `${narrowDec ? Math.round((avail - Math.min(600, avail - 32)) / 2) : Math.round((avail - 968) / 2)}px`);
    st.setProperty('--cw', `${Math.min(640, avail - 32)}px`);
    st.setProperty('--cx', `${Math.round((avail - Math.min(640, avail - 32)) / 2)}px`);
    ctx.narrow = narrowDec;
    let body = '', acts = '', notice = '';
    if (s.screen === 'connect') body = connectHTML(s, ctx);
    else if (s.screen === 'start') body = startHTML(s, W, ctx);
    else if (s.screen === 'home') {
      if (s.conn === 'offline') notice = `<div class="notice">${I('ring', 'steel')}<span class="nt">Offline since 16:41 ${dot} showing the state as of then ${dot} 3 notes and 1 unsent input are safe ${dot} writes resume when connected</span><span class="act">${K('␣rc')}<span>reconnect</span></span></div>`;
      body = homeHTML(s, W, ctx);
    } else if (s.screen === 'conv') { const r = convHTML(s, W, ctx); acts = r.acts; body = r.stage; }
    else if (s.screen === 'dec') { const r = decHTML(s, W, ctx); acts = r.acts; notice = r.notice; body = r.stage; }
    else if (s.screen === 'timeline') body = timelineHTML(s, W, ctx);
    else if (s.screen === 'faults') body = faultsHTML(s, W, ctx);
    else if (s.screen === 'convs') body = convsHTML(s, W, ctx);
    else if (s.screen === 'project') body = projectHTML(s, W, ctx);
    const fl = floatHTML(s, W, ctx);
    const app = document.getElementById('app');
    app.className = [ctx.narrow ? 'narrow' : '', fl ? 'floatopen' : '', ctx.mode === 'INSERT' ? 'insert' : '', sideHidden ? 'sidehidden' : '', !ctx.peek ? 'nopeek' : '', `focus-${focus}`].join(' ');
    const lights = '<div class="lights"><i style="background:#ff5f57"></i><i style="background:#febc2e"></i><i style="background:#28c840"></i></div>';
    app.innerHTML = lights + (sideHidden ? '' : sidebarHTML(s, W, ctx))
      + `<main class="stage">${headHTML(s, ctx, ctx.crumb || '', acts)}<div class="sbody${notice ? ' hasnotice' : ''}">${notice}<div class="sin">${body}</div></div>${fl}</main>` + statusHTML(s, ctx);
    window.__ctx = { ...ctx, float: ctx.float || null };
    const faces = ['400 14px Etoile', '600 14px Etoile', '700 20px Etoile', '700 24px Etoile', 'italic 400 14px Etoile', '400 13px Iosevka', '500 13px Iosevka', '600 11px Iosevka', '700 13px Iosevka'];
    Promise.all(faces.map((f) => document.fonts.load(f))).then(() => document.fonts.ready).then(() => layout(s, ctx));
  }

  function wrapText(root, needle, cls) {
    const tw = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    for (let n = tw.nextNode(); n; n = tw.nextNode()) {
      const i = n.textContent.indexOf(needle); if (i < 0) continue;
      const rg = document.createRange(); rg.setStart(n, i); rg.setEnd(n, i + needle.length);
      const sp = document.createElement('span'); sp.className = cls; rg.surroundContents(sp); return sp;
    }
    return null;
  }
  // slots drop whole, per group, while a label would get less than 200 px: time, model, extension, reason (spec §4.7)
  function dropSlots() {
    for (const g of document.querySelectorAll('.rows.slotted')) {
      for (const cls of ['tm', 'mdl', 'ch', 'rs']) {
        const tight = [...g.querySelectorAll('.rw .tx')].some((t) => t.getBoundingClientRect().width < 200);
        if (!tight) break;
        g.querySelectorAll(`.sl.${cls}`).forEach((e) => { e.style.display = 'none'; });
      }
    }
  }
  function layout(s, ctx) {
    const flo = document.querySelector('.float.on');
    if (flo) { const st = document.querySelector('.stage').getBoundingClientRect(); document.querySelector('.sbody').style.bottom = `${st.bottom - flo.getBoundingClientRect().top}px`; }
    const sin = document.querySelector('.sin');
    dropSlots();
    // the since-line drops whole parts from its end, never cutting a word (TC-15)
    for (const sl of document.querySelectorAll('.since')) { const parts = [...sl.querySelectorAll(':scope > .sp')]; while (sl.scrollWidth > sl.clientWidth + 1 && parts.length > 1) parts.pop().remove(); }
    for (const cb of document.querySelectorAll('.peek .cbar[data-fill]')) {
      const peek = cb.closest('.peek'); const cut = peek.querySelector('[data-cut]');
      const avail = peek.getBoundingClientRect().bottom - cb.getBoundingClientRect().top - 40;
      cb.style.maxHeight = `${Math.max(60, Math.floor(avail / 19) * 19)}px`;
      if (cut) cut.style.display = cb.scrollHeight > cb.clientHeight ? 'flex' : 'none';
    }
    const ts = document.querySelector('.tscroll');
    if (ts) {
      const stack = document.querySelector('.cstack'); const fl = document.querySelector('.float.on');
      const tb = document.querySelector('.ccol .tblock');
      const top = tb.offsetHeight + 24;
      ts.style.top = top + 'px';
      ts.style.height = `${sin.clientHeight - top - stack.offsetHeight - parseInt(stack.style.bottom || 16) - 14}px`;
      const tin = ts.querySelector('.tin'); const doc = tin.querySelector('.doc');
      const recCur = tin.querySelector('.msg.rec.cur');
      let y = Math.min(0, ts.clientHeight - tin.scrollHeight);
      if (recCur) y = -Math.max(0, recCur.offsetTop - 40);
      // never clip a line under the title block: start the view at the first whole entry line at or below the cut
      if (y < 0) {
        const tinTop = tin.getBoundingClientRect().top;
        const marks = [...doc.querySelectorAll('.msg, .msg > .mh, .mb > *, .mb .prose > *, .tool, .tout, .sysl, .rfail')].map((e) => e.getBoundingClientRect().top - tinTop).filter((t) => t >= -y - 0.5).sort((a, b) => a - b);
        if (marks.length) y = -marks[0];
      }
      tin.style.top = y + 'px';
      // the stage's cursor: on the last entry (follow on) unless the composer or a float holds the keys
      // the transcript's cursor (TC-06): a whole row on a tool row, a fold or a header; a line inside message text.
      // It sits on the last entry while follow is on; the band spans the column and its bar is the column's left edge.
      const keys = ctx.focus === 'stage' ? 'keys' : 'nokeys';
      if (recCur) recCur.querySelector(':scope > .mh').classList.add('cur', keys);
      else if (s.screen === 'conv') {
        const cands = [...doc.querySelectorAll('.tool, .msg .prose, .msg > .mh')].filter((e) => !e.closest('.msg.gen'));
        const body = cands.filter((e) => !e.matches('.mh'));
        const last = body.length ? body[body.length - 1] : cands[cands.length - 1];
        if (last && last.matches('.prose')) {
          const tw = document.createTreeWalker(last, NodeFilter.SHOW_TEXT); let tn = null; for (let n = tw.nextNode(); n; n = tw.nextNode()) if (n.textContent.trim()) tn = n;
          const rg = document.createRange(); rg.selectNodeContents(tn); const rects = rg.getClientRects(); const lr = rects[rects.length - 1];
          const dr = doc.getBoundingClientRect();
          const band = document.createElement('div'); band.className = `curline ${keys}`;
          band.style.cssText = `top:${lr.top - dr.top - 1}px;height:${lr.height + 2}px`;
          doc.appendChild(band);
        } else if (last) last.classList.add('cur', keys);
      }
    }
    const page = document.querySelector('.wpage');
    if (page && s.screen === 'dec') {
      const doc = page.querySelector('.doc'), marg = page.querySelector('.dmarg');
      const blocks = [...doc.querySelectorAll('.md.basis [data-s]')];
      const blockAt = (pos) => { const li = blocks.filter((b) => b.tagName === 'LI' && +b.dataset.s <= pos && pos < +b.dataset.e); if (li.length) return li[li.length - 1]; return blocks.find((b) => +b.dataset.s <= pos && pos < +b.dataset.e); };
      const R = decisionRec(s.dec.slice(0, s.dec.indexOf('/')), s.dec.slice(s.dec.indexOf('/') + 1));
      const ytop = (el) => el.getBoundingClientRect().top - page.getBoundingClientRect().top;
      const notes = ctx.notes ? ctx.notes.range : [];
      // ranges first: a faint sage tint; the active one (the cursor's note, or the one being typed) stronger
      const items = notes.map((n, i) => { const needle = R.text.slice(n.range.start, n.range.end).replace(/`/g, ''); const active = !s.composing && s.docCur === i; return { text: n.text, active: false, cursorNote: active, span: wrapText(doc, needle, `rng${active ? ' on' : ''}`) || blockAt(n.range.start) }; });
      if (s.composing) items.push({ text: s.composing.text, span: wrapText(doc, s.composing.sel, 'rng on'), active: true });
      let off = 0;
      if (s.scrollTo) { const hd = [...doc.querySelectorAll('.md h2')].find((h) => h.textContent.trim() === s.scrollTo); if (hd) off = ytop(hd) - 24; }
      if (s.noteAt != null && notes[0]) off = ytop(blockAt(notes[0].range.start)) - s.noteAt;
      page.style.top = `${-off}px`;
      // the document cursor: band and 2 px bar (coral only while the stage has the keys), block caret
      const curBlock = s.docCur != null && notes[s.docCur] ? blockAt(notes[s.docCur].range.start) : s.blockCur != null ? blocks[s.blockCur] : null;
      if (curBlock) {
        const band = document.createElement('div'); band.className = 'curband';
        band.style.cssText = `position:absolute;left:calc(var(--dx) - 40px);width:640px;top:${ytop(curBlock) - 1}px;height:23px;background:var(--cursor-read);z-index:0`;
        const bar = document.createElement('div'); bar.className = 'curbar'; bar.style.cssText = `position:absolute;left:0;top:0;width:2px;height:100%;background:${ctx.focus === 'stage' ? 'var(--coral)' : 'var(--muted)'}`;
        band.appendChild(bar); page.insertBefore(band, page.firstChild); doc.style.zIndex = 1;
        const tw = document.createTreeWalker(curBlock, NodeFilter.SHOW_TEXT); const tn = tw.nextNode();
        if (tn && ctx.focus === 'stage') { const r = document.createRange(); r.setStart(tn, 0); r.setEnd(tn, 1); const sp = document.createElement('span'); sp.className = 'caret'; r.surroundContents(sp); }
      }
      const pageTop = page.getBoundingClientRect().top;
      if (ctx.narrow) {
        for (const it of items) { const d = document.createElement('div'); d.className = `note inline${it.active ? ' act box' : ''}`; if (it.active) { d.dataset.ctx = 'note'; d.dataset.mode = 'i'; } d.innerHTML = noteInner(it); (it.span.closest('[data-s]') || it.span).after(d); }
      } else {
        const mtop = marg.querySelector('.mtop');
        let last = mtop.offsetHeight + 12;
        const lead = document.createElementNS('http://www.w3.org/2000/svg', 'svg'); lead.setAttribute('class', 'leader'); lead.style.cssText = `position:absolute;left:0;top:0;width:100%;height:${page.scrollHeight}px;overflow:visible;pointer-events:none`;
        page.appendChild(lead);
        for (const it of items.sort((a, b) => ytop(a.span) - ytop(b.span))) {
          const d = document.createElement('div'); d.className = `note${it.active ? ' act box' : ''}`; if (it.active) { d.dataset.ctx = 'note'; d.dataset.mode = 'i'; }
          d.innerHTML = noteInner(it); marg.appendChild(d);
          const first = it.span.getClientRects()[0];
          const want = first.top - pageTop - (it.active ? 11 : 0);
          const y = Math.max(want, last); d.style.top = `${y}px`;
          last = y + d.offsetHeight + 12;
          if (y - want > 8) {
            const pr = page.getBoundingClientRect();
            const x0 = doc.getBoundingClientRect().right - pr.left + 6, y0 = first.top - pr.top + first.height / 2;
            const x1 = marg.getBoundingClientRect().left - pr.left - 6, y1 = y + 8, xm = x0 + 10;
            lead.insertAdjacentHTML('beforeend', `<path d="M${x0} ${y0} H${xm} V${y1} H${x1}" fill="none" stroke="var(--leader)" stroke-width="1"/>`);
          }
        }
      }
    }
    if (page && s.screen === 'project') page.style.top = '0px';
    window.__ready = true;
  }
  const noteInner = (it) => (it.active
    ? `<div class="nh">${I('edit')}<span>note on the selection ${dot} not sent</span></div><div class="nb">${esc(it.text)}<span class="bar"></span></div><div class="ft">${H('Esc', '', true)}${H('jk', '', true)}<span>leave Insert</span></div>`
    : `<div class="nh">${I('edit')}<span>not sent</span></div><div class="nb" data-content>${esc(it.text)}</div>`);

  const names = Object.keys(S).filter((n) => !S[n].sub);
  window.__states = names;
  window.__meta = Object.fromEntries(Object.keys(S).map((n) => [n, { width: resolve(n).width || 1512, sheet: S[n].sheet, labels: S[n].labels }]));
  const name = location.hash.slice(1);
  if (name && S[name] && !S[name].sheet) render(name);
  else window.__ready = true;
})();
