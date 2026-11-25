// Metrics mockups (design/metrics.md, with the UI review's changes): made-up telemetry for the Metrics states, and the keys the
// proposal adds. Phase 1 of the proposal moves the data into fixture.py and the keys into keymap.toml; until it is accepted they
// live here, so the client's embedded fixture.json and keymap.toml stay as they are. Everything is seeded: a build is repeatable.
window.MX = (() => {
  const NOW = window.FX.now;
  const MIN = 60e3, HOUR = 60 * MIN;
  // the proposal's ranges: bucket width, bucket count, words
  const R = {
    '1h': { b: MIN, n: 60, words: 'last hour', per: '1 min' },
    '24h': { b: 15 * MIN, n: 96, words: 'last 24 hours', per: '15 min' },
    '7d': { b: 2 * HOUR, n: 84, words: 'last 7 days', per: '2 h' },
  };
  // bucket edges sit on local time (UTC in the prototype); the newest bucket holds now and is still filling
  const start = (r) => Math.floor(NOW / R[r].b) * R[r].b - (R[r].n - 1) * R[r].b;
  const filled = (r) => (NOW - (start(r) + (R[r].n - 1) * R[r].b)) / R[r].b;
  const rng = (seed) => () => { seed = (seed + 0x6d2b79f5) | 0; let t = Math.imul(seed ^ (seed >>> 15), 1 | seed); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
  // how busy a working day is at time t
  const work = (t) => {
    const d = new Date(t), h = d.getUTCHours() + d.getUTCMinutes() / 60, dow = d.getUTCDay();
    const day = h < 7 ? 0.03 : h < 9 ? 0.35 : h < 12 ? 1 : h < 13 ? 0.55 : h < 18 ? 1.1 : h < 21 ? 0.45 : 0.1;
    return (dow === 0 || dow === 6 ? 0.4 : 1) * day;
  };
  // a counter: one total per bucket, summing to `total` over the range
  function counter(seed, r, total) {
    const g = rng(seed), s = start(r), { b, n } = R[r];
    const raw = [];
    for (let i = 0; i < n; i++) { let v = work(s + i * b + b / 2) * (0.55 + 0.9 * g()); if (g() < 0.07) v *= 1.9; raw.push(v); }
    raw[n - 1] *= filled(r);
    const sum = raw.reduce((a, c) => a + c, 0);
    return raw.map((v) => (v / sum) * total);
  }
  // one counter split by an attribute: each bucket's total shared out with some noise
  function split(values, shares, seed) {
    const g = rng(seed), out = Object.fromEntries(Object.keys(shares).map((k) => [k, []]));
    for (const v of values) {
      const w = Object.entries(shares).map(([k, sh]) => [k, sh * (0.6 + 0.8 * g())]);
      const tw = w.reduce((a, [, x]) => a + x, 0);
      for (const [k, x] of w) out[k].push((v * x) / tw);
    }
    return out;
  }
  // a histogram: count, sum, min and max per bucket. `unknown` marks buckets where a nonfinite measurement poisoned sum, min, max
  function hist(seed, r, { rate, median, unknown = [] }) {
    const g = rng(seed), s = start(r), { b, n } = R[r];
    const out = [];
    for (let i = 0; i < n; i++) {
      let c = Math.round(work(s + i * b + b / 2) * rate * (0.4 + 1.2 * g()) * (i === n - 1 ? filled(r) : 1));
      if (i === n - 1 || unknown.includes(i)) c = Math.max(c, 2);
      if (!c) { out.push({ count: 0, sum: 0, min: null, max: null }); continue; }
      let sum = 0, mn = Infinity, mx = 0;
      for (let k = 0; k < c; k++) { const x = median * Math.exp((g() + g() + g() - 1.5) * 0.7); sum += x; mn = Math.min(mn, x); mx = Math.max(mx, x); }
      out.push(unknown.includes(i) ? { count: c, sum: null, min: null, max: null } : { count: c, sum, min: mn, max: mx });
    }
    return out;
  }
  // an upDownCounter: net change per bucket, signed
  function updown(seed, r) {
    const g = rng(seed), s = start(r), { b, n } = R[r];
    const out = [];
    for (let i = 0; i < n; i++) { const w = work(s + i * b + b / 2); const up = Math.round(w * 3 * g()), down = Math.round(w * 3 * g()); out.push(i === n - 1 ? Math.round((up - down) * filled(r)) : up - down); }
    return out;
  }

  const MODELS = { 'claude-opus-4-5': 0.41, 'claude-sonnet-4-5': 0.34, 'gpt-5.1': 0.13, 'gemini-3-pro': 0.08, 'claude-haiku-4-5': 0.04 };
  // every metric definition on the server (the `telemetry` call), in the picker's order: workflow extensions, tau, pi-ai, pi-durable
  const catalogue = [
    { source: 'w1', name: 'w1.designs', kind: 'counter', unit: '', desc: 'Designs the reviewer finished, by verdict', attrs: ['verdict'] },
    { source: 'tau', name: 'tau.decision.wait', kind: 'histogram', unit: 'ms', desc: "Time from a decision's request to its answer", attrs: ['extension', 'subject'] },
    { source: 'pi-ai', name: 'pi.ai.cost', kind: 'counter', unit: 'USD', desc: "Cost of model requests, from the provider's prices", attrs: ['provider', 'model'] },
    { source: 'pi-ai', name: 'pi.ai.request.duration', kind: 'histogram', unit: 'ms', desc: 'Time from a model request to its last token', attrs: ['provider', 'model', 'outcome'] },
    { source: 'pi-ai', name: 'pi.ai.tokens', kind: 'counter', unit: 'token', desc: 'Tokens sent to and received from a model provider, per request', attrs: ['provider', 'model', 'kind'] },
    { source: 'pi-durable', name: 'pi.durable.phase.duration', kind: 'histogram', unit: 'ms', desc: 'Time a task spends in one phase', attrs: ['phase', 'kind'] },
    { source: 'pi-durable', name: 'pi.durable.reports', kind: 'counter', unit: '', desc: 'Reports that tasks wrote', attrs: ['kind'] },
    { source: 'pi-durable', name: 'pi.durable.tasks.active', kind: 'upDownCounter', unit: '', desc: 'Tasks started minus tasks ended', attrs: ['kind'] },
  ];
  const series = {
    // render-core
    tokensAll7d: counter(11, '7d', 1240000),
    tokensOut7d: split(counter(12, '7d', 223200), MODELS, 13),
    tokensOut24h: split(counter(14, '24h', 38400), MODELS, 15),
    cost7d: counter(16, '7d', 14.2),
    phase24h: hist(17, '24h', { rate: 12, median: 3.6 * MIN, unknown: [66] }),
    // notes-cli, forecasts
    notesTokens24h: counter(18, '24h', 412000),
    active24h: updown(19, '24h'),
  };
  // the values each attribute holds in render-core's replies, with their whole-range figure (pi.ai.tokens, 7 days, kind output)
  const values = {
    provider: [['anthropic', 0.79], ['openai', 0.13], ['google', 0.08]],
    kind: [['input', 0.52], ['output', 0.18], ['cacheRead', 0.28], ['cacheWrite', 0.02]],
  };
  return { NOW, MIN, HOUR, R, start, filled, catalogue, series, values, MODELS };
})();

// The keys the proposal adds, with the review's changes: ␣pf keeps meaning "project" in every place; the chart's own cuts move to
// the view's local leader (␣mr range, ␣mg split, ␣mf keep only), beside ␣mn, ␣mm, ␣mt and ␣mp. Two new contexts: metrics (the
// board, a list) and chart (the chart view, where h l move the time cursor and j k the strip).
(() => {
  const KM = window.__keymap;
  KM.contexts.metrics = 'list';
  KM.contexts.chart = 'reader';
  const add = (seq, ctx, id, cctx = [ctx]) => KM.binds.push({ mode: 'n', ctx, seq, id, tier: 'local', cctx });
  add('<leader>om', 'global', 'metrics.open', ['global']);
  for (const c of ['metrics', 'chart']) add('<leader>mn', c, 'chart.new', ['metrics', 'chart']);
  add('<leader>mm', 'chart', 'chart.metric');
  add('<leader>mr', 'chart', 'chart.range');
  add('<leader>mg', 'chart', 'chart.split');
  add('<leader>mf', 'chart', 'chart.keep');
  add('<leader>mt', 'chart', 'chart.plot');
  add('<leader>mp', 'chart', 'chart.pin');
  add('x', 'metrics', 'pin.remove');
  add('u', 'metrics', 'pin.restore');
  for (const k of ['h', 'l']) add(k, 'chart', 'chart.time');
  for (const k of ['j', 'k']) add(k, 'chart', 'chart.strip');
  KM.ex.metrics = { id: 'metrics.open', ctx: ['global'] };
})();
