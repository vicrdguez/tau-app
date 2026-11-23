# Metrics: a board of charts you pin

**Status:** proposal, 2026-11-23. When the owner accepts it, its parts move into `spec.md` (a new §5.10), `client.md` §4 to §6, `keymap.toml`, `reference/visual-language.md` and `fixture.py`, and this file is deleted.

tau records telemetry per project since tau PR #131 (`trunk`, `a371345`). Model cost, tokens and request time come from pi-ai. Task phase durations and reports come from pi-durable. tau records the wait for an answered decision, and each workflow can declare its own metrics. This proposal adds one place, **Metrics**, where you pick any declared metric, cut it by its attributes, see it drawn the way its kind calls for or as a chart type you choose, and pin the chart to a board.

The client still knows no workflow (`spec.md` §1, principle 2). Everything it draws comes from the server's metric schemas and rows. No metric name appears in the client's code.

## What the server gives

| Call | Body | Result |
|---|---|---|
| `telemetry` | `{}` | `items: { source, schema }[]`: every metric definition on the server, with kind, unit, description and attributes |
| `metrics` | `project, name, startMin?, startMax?, attributes?` | `items: StoredMetric[]`: one row per 10-second interval per attribute set, with `count`, `sum`, `min`, `max` |

These facts shape the design:

- `metrics` reads one metric of one project. It has no rollup, no limit and no paging.
- A histogram row keeps count, sum, min and max. The server keeps no bucket counts, so no client can show a percentile or a median.
- An upDownCounter row holds deltas. The server keeps no opening value, and retention deletes rows after 90 days, so the client can't prove a level such as "3 tasks active now".
- `telemetry` is server-wide. A project's metrics come from `pi-durable`, `pi-ai`, `tau` and the extensions bound to that project (`ProjectView.extensions`).
- A row appears after its 10-second interval ends, plus up to 1 s for the flush.
- A nonfinite measurement makes its row's `sum`, `min` and `max` null.

## The page

Metrics opens with `␣om`, `:metrics` or its sidebar row under Projects. The page shows the board, or one chart large.

**The board** lists your pins, top to bottom in the order you pinned them. Each pin is a 28 px head and a 48 px strip under it. The head holds the metric name (`mono.small`, orange), the chart's words (`by model · kind output`), the project (sage), the range (`fg.muted`), and the whole-range figure at the right (`$14.20`). The strip draws the chart with every group summed. `⏎` opens the pin large, `x` unpins it, and `␣pf` filters the board by project, like the other places. With no pins, the page reads `No pinned charts. Choose a metric, then pin its chart here.` and the cursor sits on `+ new chart`.

**The chart view** shows one chart at the stage's width. The page title is the metric name. Under it come the facts line (`counter · tokens · render-core · 7 days`), the metric's description as content, the summary (`1.24 M tokens in 7 days, summed over every provider and kind`), the plot, and the readout for the time cursor (`Tue 11 Nov 14:00 to 16:00 · 31.2 k tokens`). A grouped chart draws one strip per value instead of one plot.

A typical session:

| Keys | Result |
|---|---|
| `␣om` | The board. |
| `␣mn` | The **Metric** picker rises. It lists `render-core`'s metrics, workflow extensions first, then `tau`, `pi-ai` and `pi-durable`. Each row shows the name, the description and `counter · tokens`. |
| `tokens` `⏎` | The chart view draws `pi.ai.tokens` as bars of tokens per 2 hours over 7 days, with the time cursor on the newest bucket. |
| `␣pf` `by model` `⏎` | The **Filter** picker. The chart becomes one strip per model on a shared scale. |
| `␣pf` `output` `⏎` | Keeps rows whose `kind` is `output`. |
| `␣mp` | The echo reads `Pinned to the board`. |
| `C-o` | Back on the board, with the cursor on the new pin. |

## Decisions

### A chart is one value, and a pin is that value

```rust
// crates/tau-app/src/screens/metrics/chart.rs
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chart {
	pub project: String,
	pub metric: String,
	/// Exact equality per attribute, all at once.
	#[serde(default)]
	pub filters: BTreeMap<String, AttrValue>,
	/// One attribute to split by, drawn as one strip per value.
	#[serde(default)]
	pub by: Option<String>,
	#[serde(default)]
	pub range: Range,
	#[serde(default)]
	pub plot: Plot,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Plot {
	/// Follows the kind's natural plot, so a pin improves when the rule does.
	#[default]
	Natural,
	Chosen { stat: Stat, mark: Mark },
}
pub enum Stat { Total, Count, Mean, Min, Max }
pub enum Mark { Bars, Line, Band }
```

The board never holds two equal charts, so a chart is its own pin's identity and the board needs no IDs. A chart opened from a pin remembers that pin. `␣mp` then replaces the pin in place and says `Pin updated`. Pinning an equal chart twice changes nothing.

A chart holds a question, never an answer. It stores no rows, no absolute times and no colours. A pin opened next week asks the same question about the last 7 days.

### Each kind has a natural plot, and the picker offers only plots that are true

| Kind | Natural plot | Also offered (`␣mt`) | It can't show |
|---|---|---|---|
| `counter` | total per bucket, bars | total as a line; number of adds as bars | the value of one add, a rate finer than a bucket |
| `histogram` | mean per bucket as a line over a band from min to max | mean, min or max as a line; measurements as bars; total of the values as bars (total wait, for example) | percentiles, a median, the distribution's shape |
| `upDownCounter` | net change per bucket, signed bars about zero | net change as a line | the level (for example, tasks active now) |

Bars appear only for a statistic that adds up (`Total`, `Count`). A bar of means or maxes would invite the reader to add them, so `plots(kind)` never offers one. A histogram's summary always pairs the mean with the max and the count, so a skewed mean can't hide the slow outlier. `K` on a chart says what it can't show and why, in the server's terms.

The unit sets how numbers read and never sets the plot. `ms` reads `340 ms` under a second, then through `time::span` (`4.2 s`, `12 s`, `4 m`). `USD` reads `$14.20`, with three significant digits under a dollar. `token` reads `1.24 M tokens`. `By` reads `1.2 MB`. A metric with no unit reads `1,204`. Any other unit prints as sent.

### Groups are strips, not colours

A grouped chart draws one strip per attribute value, sorted by the whole-range figure, on one shared scale, with the value's label at the left and its figure at the right. Rows that lack the attribute form their own `not set` strip. `j` and `k` move between strips. `h` and `l` move the time cursor, which is one column across every strip.

The alternative was one plot with a colour per series. `reference/visual-language.md` §4.2 gives every hue one meaning: olive means ok, rose means failed, sage means a project. A palette of six series colours would either break those meanings or need six new neutral tones that look alike. Strips need no new colour, and they read like the app's lists. They also need no "top 4 and others" rule that could hide a group. Strips take more height than stacked bars, and when a group is tiny its strip lies flat on the shared scale. Its figure at the right still gives the number.

### The client fetches per metric and range, then cuts locally

The client asks `metrics {project, name, startMin}` with no attribute filter. It rolls the reply into display buckets once, keeping one track per attribute set. Filters, grouping and the plot are cuts of that roll-up. Changing any of them redraws at once, with no fetch. Pins of one metric and range share one fetch. This costs a larger reply when a filter would have narrowed it. Phase 0 measures whether that matters.

```rust
// crates/tau-client/src/telemetry.rs: pure, no clock, no I/O
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MetricQuery {
	pub project: String,
	pub metric: String,
	pub range: Range,
	/// Bucket edges sit on local time, so a 2-hour bucket starts at an even local hour.
	pub utc_offset_min: i16,
}

pub enum MetricRead {
	Have { cube: Cube, at: Wall, no_reply_since: Option<Wall> },
	Refused { code: RefusalCode, message: String },
	NoReply { since: Wall },
}

/// A reply rolled into its range's buckets, one track per attribute set.
pub struct Cube { /* kind, unit, start, width, buckets, settled, tracks */ }

/// Merges exactly: counts and sums add, min and max take the extreme.
/// `None` means a null arrived, which poisons that statistic but never the count.
pub struct Bucket { pub count: u64, pub sum: Option<f64>, pub min: Option<f64>, pub max: Option<f64> }

/// Cuts a cube into strips. A bucket with no measurements is `None`.
pub fn series(cube: &Cube, filters: &Attrs, by: Option<&str>) -> Series;
```

`Server` gains `telemetry: Known<Catalogue>` and `metrics: BTreeMap<MetricQuery, MetricRead>`. The whole query is the slot's key, and each key has at most one call in flight. A slow reply to a 7-day query fills the 7-day slot, never the slot of the 24-hour query that replaced it on screen.

### Ranges stop at 7 days until a measurement says otherwise

| Range | Bucket | Buckets | Refresh while shown and focused |
|---|---|---|---|
| `1 h` | 1 min | 60 | every 10 s |
| `24 h` | 15 min | 96 | every 1 min |
| `7 d` | 2 h | 84 | every 5 min |

`30 d` and `90 d` wait for phase 0. A busy metric over 90 days could exceed the client's 20 s call timeout or its 64 MiB reply cap (`crates/tau-client/src/http.rs`). That size is unmeasured. If it is too large, the fix is a server-side step parameter (request 1 below), not a client workaround.

The newest bucket is still filling. It draws as an outline, and the readout says `so far`.

### The screen declares what it shows, and the engine does the rest

```rust
// screens/metrics.rs, in poll: the set of queries the stage draws now.
app.link.watch_metrics(wanted(app));
```

`watch_metrics` is idempotent. The engine sends a watched query once the link is ready and the window focused, refreshes it on the range's cadence, and asks again on focus, on reconnect and on showing the place. A query left out of the set stops refreshing. This closes two gaps the existing on-demand reads have. `fetch_options` drops a request made before the handshake (`engine/fetch.rs:51-56`), and a reconnect does not re-ask it.

`telemetry` rides the catalogue refresh. `unknown_call` on it means the server predates telemetry.

### Editing uses the existing picker three times

Every edit goes through `floats/project_picker.rs::Picker`, unchanged:

| Picker | Key | Rows |
|---|---|---|
| Metric | `␣mn` (new chart), `␣mm` (change metric) | the project's metrics: name, description, `kind · unit` |
| Filter | `␣pf` on a chart | projects; ranges; `no grouping` and `by <attribute>`; `any <attribute>` and each value the schema declares or the reply holds |
| Chart type | `␣mt` | `natural · <what it is>`, then the plots its kind allows |

One choice per visit, then the picker closes. Three edits are three visits. A `:chart` line that parses into the same `Chart` is a later addition with no new state.

### Pins live per server on this device

`metrics.json` sits beside `places.json` in the server's directory and is saved through `link.store()`. Metric names and projects belong to one server, so a pin means nothing on another. `:forget-server` deletes the pins with the rest. Optional fields default, so a file from an older client still loads. `Store::load` treats an unreadable file as missing, and a save would then overwrite a hand-built board. Metrics therefore reads its file with a new `Store::try_load`. If the file exists but can't be read, Metrics leaves it untouched and says `Pinned charts on this device couldn't be read.`

In `--demo`, pins last for the session.

## How it looks

The chart draws on `rune::el::canvas` with `Shape::view_box(buckets, 1000)`, so x counts buckets and y counts thousandths of the scale. Stroke widths stay in points. Axis labels and the readout are ordinary text beside the canvas, because a `Shape` draws no text.

- **Marks.** Bars fill `fg.alt`, with a 1 px gap between bars. A line is 1.5 px `fg`. A histogram's band fills `border.hairline`, with the mean line over it. No new colour tokens.
- **Axes.** A 1 px `border.strong` baseline and two `border.hairline` rules at the middle and top ticks. Ticks are `mono.small` `fg.muted`. The x labels read `16:40` for 1 hour and 24 hours, and `Tue 14:00` for 7 days.
- **The cursor.** The time cursor's column gets a `surface.cursor.read` band behind the marks. While the stage has the keys, a 2 px coral bar sits under that column on the baseline. That bar is the page's one coral cursor (`reference/visual-language.md` §4.3). On the board, the pin under the cursor gets the list's usual band and coral bar.
- **Unknown.** A bucket with a null value draws no mark. A 2 by 6 px `state.unknown` tick sits under the baseline, and the readout says `unknown: a measurement was not a finite number`.
- **Content.** Descriptions, attribute values and strip labels are extension text. They render as plain lines, never as chips or coloured text.

## What each state says

| State | Board row | Chart view |
|---|---|---|
| nothing received yet | `Nothing received from the server yet` | same |
| no reply, nothing held | `No reply from the server. Asking again.` | same |
| refresh failed, data held | the chart, with `as of 16:40` in the facts | same |
| no rows in the range | `No measurements in the last 7 days` | `No measurements of pi.ai.tokens in render-core in the last 7 days.` |
| empty bucket | | readout `no measurements` |
| `telemetry_unavailable` | `render-core opened without telemetry` | `The server opened render-core without telemetry, so it keeps no metrics for it. Its log names the cause in a line that starts with tau: telemetry render-core.` |
| `unknown_project` | `render-core is gone from the server` | same |
| server without telemetry | `This server has no telemetry. It may be older than this client.` | same |
| metric in no schema now | the chart, with `not in the server's schemas now` in the facts | same, and `K` says why |

## Failure list

Each line is a way this view could mislead you. The tests named here are written before the code they test.

| # | How it could mislead | Proof |
|---|---|---|
| F1 | The newest bucket looks like a collapse because it is still filling. | Cube test: a reply built 3 s after a bucket edge marks the newest bucket in progress. A scene shows the outline. |
| F2 | A slow reply for the old selection draws under the new one. | Link test: watch query A, then B. A's reply lands after B's watch. B's slot stays empty and the screen draws B's state. |
| F3 | `telemetry_unavailable` reads as "no measurements" or "loading". | Link test: the refusal becomes `MetricRead::Refused`. A screen test on the fixture's `scratch` project checks the copy. |
| F4 | A null value is skipped silently, and a total looks exact. | Series test: a null row makes its bucket unknown, keeps its count, and the notes name the unknown bucket. |
| F5 | A histogram mean is an average of averages. | Series test: 3 measurements with sum 60, min 10 and max 30, plus 1 measurement of 100, give count 4, mean 40, min 10 and max 100. |
| F6 | Bars of means or maxes invite adding them. | Test over every kind: `plots(kind)` never pairs `Bars` with `Mean`, `Min` or `Max`. |
| F7 | A percentile is implied. | No percentile `Stat` exists. Copy test: a histogram summary always holds the max and the count. |
| F8 | An upDownCounter's net change reads as a level. | Copy test: the facts and summary say `net change`, and no plot says `level`. A scene shows negative bars below zero. |
| F9 | A summed attribute is hidden, for example token kinds added together. | Copy test: the summary names every attribute that is neither filtered nor grouped. |
| F10 | A chart asked before the handshake never loads, or stays dead after a reconnect. | Link test: a watch before `Ready` sends one `metrics` call after the handshake, and a reconnect asks again. |
| F11 | Charts poll while hidden or while the window is unfocused. | Link test: no calls with focus off or an empty watch set, and one call per cadence while focused. |
| F12 | Old numbers look current after the link dropped. | Link test: a failed refresh keeps the cube and sets `no_reply_since`. A screen test checks `as of 16:40`. |
| F13 | Pins are lost, or one server's pins show on another. | Store tests: round trip, two servers with separate files, `forget_server` deleting the file, and an unreadable file left unchanged. |
| F14 | Coral appears on data, or two coral cursors show. | Scenes with the stage holding the keys and with a picker holding them. |
| F15 | A key collides, or a handler has no keymap ID. | Keymap lint with its mutants, and the existing handler-to-ID test. |

## Build order

Each phase ends in a check on the real artifact.

| Phase | Builds | Check |
|---|---|---|
| 0 | Nothing. Call `metrics` for `pi.ai.tokens` and `pi.durable.phase.duration` on a server with real use, over 1 hour, 24 hours, 7 days, 30 days and 90 days. | Rows, bytes and milliseconds per range recorded in `design/evidence.md`. The range list is decided. |
| 1 | The design package: `spec.md` §5.10, `client.md` (calls, refresh, `telemetry_unavailable`), `keymap.toml`, `fixture.py` with schemas, rows and a refusing project. | Keymap lint with mutants passes. `fixture.json` regenerates. |
| 2 | `telemetry.rs`: catalogue parsing, `Cube`, `series`. Tests F1 and F4 to F7 first. | `cargo test -p tau-client --test telemetry` |
| 3 | Wire, engine and link: two calls, one refusal code, the watch, refresh and states; fixture answers. Tests F2, F3 and F10 to F12 first. | `cargo test -p tau-client` |
| 4 | `ui/chart.rs` with scenes built from hand-made figures: bars, line, band, signed bars, strips, in progress, unknown, Ember Light. | `nix run .#snap` of those scenes, reviewed by the owner |
| 5 | The Metrics screen family: place and sidebar row, board, chart view, pickers, keys, `metrics.json`. Tests F8, F9 and F13 to F15 first. | `cargo test -p tau-app metrics`, the scenes, then `nix run .#verify` |
| 6 | Move the flake's e2e tau input to a revision with telemetry. Add one real-server check that reads `telemetry` and one metric. | `nix run .#e2e` |

The fixture's rows carry fixed dates. Scenes run on a `ManualClock` at the fixture's `now`. For `--demo`, the fixture transport shifts each row's `intervalMs` by the gap between the session's clock and the fixture's `now`, so demo charts are not empty.

## Left out of v1

- **Charts across projects**, such as cost for every project this week. Each project is a separate `metrics` call, and a chart would need a merge rule and a note for every refusing project.
- **More than one metric on a chart**, and ratios such as failures per run.
- **Reordering pins.** The sidebar's open items can't be reordered by hand either (`open-questions.md` §5).
- **Metrics without a schema.** The server can't list them.
- **Traces.** `traces` and `trace` deserve their own view: the error traces of a project, newest first, opening one as a span tree.
- **A `:chart` command line** that parses into a `Chart`.

## Open questions for the owner

1. Should the board show live charts, as proposed, or a list of saved charts that each open on `⏎`? A list fetches nothing until opened. The board shows every pin at a glance, which is the dashboard you asked for, at one fetch per pinned metric and range.
2. Should pins stay on this device per server, or should the server keep them so every device sees one board? A server-kept board is a core change.
3. Do you want charts across projects in v1?
4. Should the Project view's Work group gain a `Metrics` row that opens the board filtered to that project?
5. Are neutral strips acceptable for groups, or do you want a categorical palette in the Ember tokens?

## Requests to the core

1. A `step` for `metrics` that returns one merged row per step per attribute set. It removes the reply-size risk, and the client's roll-up shrinks to a copy.
2. An opening value for an upDownCounter series, so the client can draw a level.
3. Histogram bucket counts per interval, using the schema's `boundaries`, so percentiles become possible.
4. A `workflow` attribute on `pi.ai.cost` and `pi.ai.tokens`. Cost per project exists today. Cost per workflow inside a project with several workflows does not.
5. A call that lists the metric names with data in a project.
