# Metrics: a board of charts you pin

**Status:** proposal, 2026-11-23; revised 2026-11-24 after a UI review against `reference/visual-language.md`, `spec.md` and the built app. The mockups are `mockups/metrics-*.png`, rendered by `../tools/proto/` (states 48 to 55 in its `states.md`), and every check family passes on them. When the owner accepts it, its parts move into `spec.md` (§4.1 and a new §5.10), `client.md` §4 to §6, `keymap.toml`, `reference/grammar.md` §9, `reference/visual-language.md` and `fixture.py`, and this file is deleted.

tau records telemetry per project since tau PR #131 (`trunk`, `a371345`). Model cost, tokens and request time come from pi-ai. Task phase durations and reports come from pi-durable. tau records the wait for an answered decision, and each workflow can declare its own metrics. This proposal adds one place, **Metrics**, where you pick any declared metric, cut it by its attributes, see it drawn the way its kind calls for or as a chart type you choose, and pin the chart to a board.

The client still knows no workflow (`spec.md` §1, principle 2). Everything it draws comes from the server's metric schemas and rows. No metric name appears in the client's code.

## What the review changed

The first draft (`a60855d`) described the plot well but not how the page sits in the frame. The review fitted it to the frame and the visual language:

- The board is a place like the others: a breadcrumb with its project filter, pins grouped by project, and the peek. A chart is a view of that place, with an action segment and the details panel.
- The facts line is the chart's cut, and each part opens its picker. The description moved to the details panel.
- `␣pf` keeps meaning "project". The cut moved to the view's local leader, one short picker per part: `␣mr`, `␣mg`, `␣mf`.
- Bars are `fg.muted`, not `fg.alt`. The band is `border.strong`, not `border.hairline`, which fell under the fill-cue floor under the cursor.
- In a split chart the time cursor is a neutral column and the selected strip a neutral row. The coral bar shows only where they cross. Strips keep their order while a chart is open.
- A chart you leave unpinned is kept, and `u` takes back an unpin.
- Copy: `default` for `natural`, plain plot words for kind names in the facts line, and the copy rules' dates and range words.

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

**Answers:** how much this work costs and how long it takes, per project, over the last hour, day or week.

Metrics is a place. It opens with `␣om`, `:metrics` or its sidebar row under Projects (Carbon `chart--column`, 16 px, `fg.muted`). It shows the board, or one chart as a view of the board.

### The board

`mockups/metrics-board.png`, `metrics-board-light.png`, `metrics-board-empty.png`.

- **Frame.** Breadcrumb `Metrics › all projects`, or `Metrics › render-core` with the quiet `show all` while filtered (`spec.md` §4.2). `␣pf` filters the board by project, as in every place. The header has no action segments, as no place has. Title `Metrics`. The list column, the cursor band and the peek follow `spec.md` §4.7 and §5.8.
- **Groups.** Pins are grouped by project under sage group heads with their count (`render-core 3 charts`), as Conversations groups its rows. Groups come in the order of their first pin, and pins in the order you pinned them. A pin's head never repeats its project, because the group head says it.
- **A pin** is a 28 px head over a 48 px strip, with 12 px between pins. The head holds the metric name (`mono.small`, orange), the cut (`by model · kind output`, `ui.small` `fg.alt`), the range (`last 24 hours`, `fg.muted`) and, at the right, the whole-range figure (`mono.small` `fg.alt`, `fg` on the cursor pin). The figure is a counter's total (`38.4 k tokens`, `$14.20`), a histogram's mean with its max (`mean 3 m · max 8 m`, F7), or an upDownCounter's net change (`net +2`). The strip draws the chart with every group merged: counters summed, histograms merged as `Bucket` merges. It has no axes.
- **The cursor** pin gets the list's band and, while the stage has the keys, its 2 px coral bar.
- **The peek** shows the cursor pin, live: its name, a facts line (`render-core · by model · kind output · last 24 hours`), its summary sentence, its groups as strips on one scale with the scale named (`one scale, 0 to 2 k per 15 min`), and its newest bucket (`16:30 to now · so far 1.04 k tokens`). It has no cursor and prints no keys. Under a 1000 px stage it is the `K` float, as in every place.
- **Keys.** `⏎` or `l` opens the pin's chart. `x` unpins, and the echo reads `Unpinned pi.ai.tokens by model, kind output`. `u` puts the last unpinned pin back in its place. `␣mn` starts a new chart in the cursor pin's project, else the board's filter, else the project last used on this device.
- **Empty.** One line where the first pin would be: `No pinned charts yet · ␣mn makes one`, the Projects pattern (`spec.md` §5.8). No cursor shows.

### The chart view

`mockups/metrics-chart.png`, `metrics-chart-split.png`, `metrics-chart-histogram.png`.

A chart is a view of the Metrics place, as Faults is of Home. While it shows, Metrics is the sidebar's current row. It never becomes an open item, because it is no server object. `C-o` goes back to the board.

- **Frame.** Breadcrumb `Metrics › render-core › pi.ai.tokens` (the project sage, the metric orange mono). The column is the places' list column, 40 px in on each side. With the details panel open, its right edge clears the panel's hairline by 24 px (`spec.md` §4.5).
- **Action segment**, at the header's right: `␣mp pin` for a chart no pin holds, `␣mp update pin` for a chart opened from a pin and changed since, and none for a chart that equals its pin.
- **Title block.** The title is the metric name (`title.page`). The facts line is the chart's cut, four parts with steel labels and `fg` values: `range last 7 days · split model · keep kind output · plot default: total per 2 h as bars`. Each part is a hint target (`gs`) and pressable, and opens its picker. A part that differs from the chart's pin carries the sage `edit` mark, as a drafted option does (`spec.md` §5.9). `details ›` stands at its right.
- **Summary**, 24 px under the title block (`ui`, `fg.alt`, its figures `fg`). It names the total, the range, every kept value and every summed attribute (F9): `1.24 M tokens in the last 7 days, summed over every provider, model and kind`. A histogram's pairs the count with the mean, the max and the min (F7): `297 measurements in the last 24 hours where phase is implement, merged over every kind · mean 3 m · max 8 m · min 1 m`. A split chart adds one muted line that names the scale: `one strip per model, on one scale from 0 to 5 k tokens per 2 h`.
- **The plot**, 18 px under the summary: 280 px tall, or one 48 px strip per value.
- **The readout**, under the x labels: `Nov 13 12:00 to 14:00 · 76.5 k tokens`. In a split chart it names the selected strip's value and every value's: `Nov 13 14:00 to 16:00 · claude-sonnet-4-5 2.73 k tokens · every model 7.61 k tokens`. On the newest bucket: `16:30 to now · so far 2 measurements · mean 3 m · max 4 m`. On an empty bucket: `… · no measurements`. An unknown bucket in range adds a line with the small unknown mark: `09:15 to 09:30 · unknown: a measurement was not a finite number`.
- **Details panel** (`␣td`, `details ›`): **About** (metric, from, kind, unit, bucket, refresh), **Description** (the schema's text as plain lines), **Attributes**, and **It can't show** (what `K` says, `Each kind has a default plot` below). The facts line names no kind and no unit. The panel holds them.
- **Keys.** `h` and `l` move the time cursor, which starts on the newest bucket. `j` and `k` move between strips. `K` says what the chart can't show and why, in the server's terms.

### A typical session

| Keys | Result |
|---|---|
| `␣om` | The board, filtered as you left it, with the peek on the pin under the cursor. |
| `␣mn` | The **Metric** picker rises. It lists `render-core`'s metrics under a group head per source, workflow extensions first, then `tau`, `pi-ai` and `pi-durable`. Each row shows the name, the description and `counter · tokens`. |
| `tokens` `⏎` | The chart view draws `pi.ai.tokens` as bars of tokens per 2 hours over 7 days, with the time cursor on the newest bucket. The header shows `␣mp pin`. |
| `␣mg` `model` `⏎` | The **Split** picker. The chart becomes one strip per model on one scale, and the facts line reads `split model`. |
| `␣mf` `output` `⏎` | The **Keep only** picker. Keeps rows whose `kind` is `output`. |
| `␣mp` | The echo reads `Pinned to the board`, and the segment goes. |
| `C-o` | Back on the board, with the cursor on the new pin under `render-core`. |

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
	/// Follows the kind's default plot, so a pin improves when the rule does. The page calls it `default`.
	#[default]
	Natural,
	Chosen { stat: Stat, mark: Mark },
}
pub enum Stat { Total, Count, Mean, Min, Max }
pub enum Mark { Bars, Line, Band }
```

The board never holds two equal charts, so a chart is its own pin's identity and the board needs no IDs. A chart opened from a pin remembers that pin. `␣mp` then replaces the pin in place, the echo says `Pin updated`, and the segment goes. Pinning an equal chart twice changes nothing.

A chart holds a question, never an answer. It stores no rows, no absolute times and no colours. A pin opened next week asks the same question about the last 7 days.

A chart you leave unpinned is kept, as every place keeps its filter and cursor (`spec.md` §5.8). The Metric picker leads with it under a `Not pinned 1` group head (`pi.ai.tokens · by model · kind output · last 7 days`). Pinning it, or leaving another unpinned chart, replaces it.

### Each kind has a default plot, and the picker offers only plots that are true

| Kind | Default plot | Also offered (`␣mt`) | It can't show |
|---|---|---|---|
| `counter` | total per bucket, bars | total as a line; number of measurements as bars | the value of one measurement, a rate finer than a bucket |
| `histogram` | mean per bucket as a line over a band from min to max | mean, min or max as a line; measurements as bars; total of the values as bars (total wait, for example) | percentiles, a median, the distribution's shape |
| `upDownCounter` | net change per bucket, signed bars about zero | net change as a line | the level (for example, tasks active now) |

`default` is the app's word for a value the client chooses (`model default`, `project default`, `default main`). The `␣mt` picker reads `default: total per 2 h as bars` first, then the plots its kind allows, and marks the one in use `current`, as the Model picker does.

Bars appear only for a statistic that adds up (`Total`, `Count`). A bar of means or maxes would invite the reader to add them, so `plots(kind)` never offers one. A histogram's summary always pairs the mean with the max and the count, so a skewed mean can't hide the slow outlier. `K` on a chart, and the details panel's **It can't show**, say what the chart can't show and why, in the server's terms. The kind words (`counter`, `histogram`, `upDownCounter`) appear in the Metric picker and the details panel, never in the facts line (`spec.md` §8).

The unit sets how numbers read and never sets the plot. `ms` reads `340 ms` under a second, then through `time::span` (`4.2 s`, `12 s`, `4 m`). `USD` reads `$14.20`, with three significant digits under a dollar. `token` reads `1.24 M tokens`. `By` reads `1.2 MB`. A metric with no unit reads `1,204`. Any other unit prints as sent. A scale's numbers drop trailing zeros (`5 k`, `50 k`, `10 m`). A scale tops out at the first of 1, 2, 2.5, 5 and 10 times a power of ten at or above the largest value, counted in minutes for `ms` over a minute.

### Groups are strips, not colours

A split chart draws one strip per attribute value, on one shared scale that the summary names. Each strip is 48 px: 6 px, a 34 px plot, 8 px. Its value's label sits in a 180 px column at the left (`ui`, `fg.alt`, plain text) and its whole-range figure in a 64 px column at the right (`mono.small` `fg.alt`). Rows that lack the attribute form their own `not set` strip. `j` and `k` move between strips. `h` and `l` move the time cursor, which is one column across every strip.

Strips are sorted by their figure when the chart opens or its cut changes. A refresh never reorders them, so the strip under the cursor stays put (F16). A value new since the sort joins the end.

When the plot would be under 400 px wide, the label column narrows to 120 px and labels are cut with `…`. Figures never cut.

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

| Range | Words | Bucket | Buckets | Refresh while shown and focused |
|---|---|---|---|---|
| `1 h` | `last hour` | 1 min | 60 | every 10 s |
| `24 h` | `last 24 hours` | 15 min | 96 | every 1 min |
| `7 d` | `last 7 days` | 2 h | 84 | every 5 min |

The page writes a range in its words everywhere: the pin's head, the facts line, the summary and the `␣mr` picker.

`30 d` and `90 d` wait for phase 0. A busy metric over 90 days could exceed the client's 20 s call timeout or its 64 MiB reply cap (`crates/tau-client/src/http.rs`). That size is unmeasured. If it is too large, the fix is a server-side step parameter (request 1 below), not a client workaround.

The newest bucket is still filling. It draws as an outline, and the readout says `so far`.

### The screen declares what it shows, and the engine does the rest

```rust
// screens/metrics.rs, in poll: the set of queries the stage draws now.
app.link.watch_metrics(wanted(app));
```

`watch_metrics` is idempotent. The engine sends a watched query once the link is ready and the window focused, refreshes it on the range's cadence, and asks again on focus, on reconnect and on showing the place. A query left out of the set stops refreshing. This closes two gaps the existing on-demand reads have. `fetch_options` drops a request made before the handshake (`engine/fetch.rs:51-56`), and a reconnect does not re-ask it.

`telemetry` rides the catalogue refresh. `unknown_call` on it means the server predates telemetry.

### Editing uses the existing picker, one per part of the cut

`mockups/metrics-metric-picker.png`, `metrics-keep-picker.png`.

Every edit goes through `floats/project_picker.rs::Picker`. The Picker gains the group heads the Settings float draws (`group_head`): a steel head with its count of rows the query keeps, 12 px below the rows above it, never selectable (`spec.md` §6).

| Picker | Key | Rows |
|---|---|---|
| Metric | `␣mn` (new chart), `␣mm` (change metric) | the chart left unpinned, if any, under `Not pinned`; then the project's metrics under a head per source: name (orange mono), description cut with `…`, `kind · unit` at the right |
| Project | `␣pf` | on the board, `all projects` and each project, the board's filter; on a chart, each project, the chart's project |
| Range | `␣mr` | `last hour`, `last 24 hours`, `last 7 days`; `current` on the one in use |
| Split | `␣mg` | `no split`, then each attribute with its number of values |
| Keep only | `␣mf` | a head per attribute the chart doesn't split by, then `any <attribute>` and each value, with its whole-range figure; `current` on the values in use; the meta says which attribute is the split, which isn't listed |
| Plot | `␣mt` | `default: <what it is>`, then the plots its kind allows; `current` on the one in use |

One choice per visit, then the picker closes. `␣pf` keeps the meaning it has in every place, so the chart's other parts get keys of their own under the view's local leader (`␣m`, "this view"). `␣mr` resets an option field in the Project view and `␣mn` writes a note in a decision. The local leader means what the view under it means, and which-key shows the chart's. A `:chart` line that parses into the same `Chart` is a later addition with no new state.

The keymap gains two contexts: `metrics` (the board, a child of `list`) and `chart` (the chart view, a child of `reader`). `reference/grammar.md` §9 gains two printed keys: `␣mp` in a chart's header segment and `␣mn` in the empty board's line.

### Pins live per server on this device

`metrics.json` sits beside `places.json` in the server's directory and is saved through `link.store()`. It holds the pins and the chart left unpinned. The board's filter and cursor live in `places.json` with the other places'. Metric names and projects belong to one server, so a pin means nothing on another. `:forget-server` deletes the pins with the rest. Optional fields default, so a file from an older client still loads. `Store::load` treats an unreadable file as missing, and a save would then overwrite a hand-built board. Metrics therefore reads its file with a new `Store::try_load`. If the file exists but can't be read, Metrics leaves it untouched and says `Pinned charts on this device couldn't be read.`

In `--demo`, pins last for the session.

## How it looks

The chart draws on `rune::el::canvas` with `Shape::view_box(buckets, 1000)`, so x counts buckets and y counts thousandths of the scale. Stroke widths stay in points. Axis labels and the readout are ordinary text beside the canvas, because a `Shape` draws no text. No colour token is new.

- **Marks.** Bars fill `fg.muted`, 6.09 / 6.30 / 5.95 : 1 on `surface.read` (Ember / Soft / Light), with a 1 px gap between bars. The bar under the time cursor fills `fg`, so the cursor shows in the data as well as under it. A line is 1.5 px `fg`. A histogram's band fills `border.strong`, with the mean line over it, and the band under the time cursor fills `fg.muted`. The first draft's `fg.alt` bars stood 7.8:1 against the stage and 1.5:1 from body text, so a board of strips would have been the loudest thing in the app. Its `border.hairline` band was 1.30:1 on the stage and 1.11:1 under the cursor column, below `tau.fill_cue_min` (1.15). `border.strong` is 1.67 / 1.72 / 1.84 on the stage and 1.43 / 1.45 / 1.56 under the cursor (F17).
- **Signed bars** stand about a zero line (`border.strong`) at half height. Both signs fill `fg.muted`. Olive and rose label outcomes only (`reference/visual-language.md` §4.2, rule 5).
- **The newest bucket** draws as a 1 px `fg.muted` outline. A histogram's last line segment is dashed.
- **Axes.** A 1 px `border.strong` baseline and two `border.hairline` rules at the middle and top ticks. The y ticks stand in a 56 px column at the plot's right (`mono.small` `fg.muted`), so every number on the page sits at the right: the ticks, the strips' figures and the pins' figures. The x labels (`mono.small` `fg.muted`) read in the copy rules' forms (`spec.md` §8): `Nov 8` at each midnight for 7 days; `20:00` every 4 hours for 24 hours, with `Nov 14` at midnight; and every 10 minutes for 1 hour. A label too close to the right edge is left out.
- **The time cursor.** Its column gets a `surface.cursor.read` band behind the marks, through every strip. In a split chart the selected strip gets the same band as a row, and its label and figure turn `fg`. While the stage has the keys, a 2 px coral bar sits under the column, 2 px below the selected strip's baseline. That bar is the page's one coral cursor (`reference/visual-language.md` §4.3). Without the keys, the column and the row keep their bands and the bar goes (§1). On the board, the pin under the cursor gets the list's usual band and coral bar. The peek has no cursor.
- **Unknown.** A bucket with a null value draws no mark. The outline `?` (Carbon `help`, 11 px, `state.unknown`) sits under the baseline at that bucket, the silhouette the visual language gives unknown (§4.6), so it can't read as an empty bucket. The readout's unknown line says why.
- **Content.** Descriptions, attribute values and strip labels are extension text. They render as plain lines, never as chips or coloured text.
- **Motion.** None. A refresh, a new cut, a range and every cursor move land within one frame (`reference/visual-language.md` §7). No bar grows in.
- **Accessibility.** A pin reads as its head and its figure: `pi.ai.tokens by model, kind output, last 24 hours, 38.4 k tokens`. A plot's description is its summary sentence, a strip reads as its label and figure, and a time cursor move announces the readout.

## What each state says

| State | Board row | Chart view |
|---|---|---|
| no pins | `No pinned charts yet · ␣mn makes one`, no cursor | |
| nothing received yet | `Nothing received from the server yet` | same |
| no reply, nothing held | `No reply from the server. Asking again.` | same |
| refresh failed, data held | the chart, with `as of 16:40` before the figure | the chart, with the summary ending `as of 16:40` |
| no rows in the range | `No measurements in the last 7 days` | `No measurements of pi.ai.tokens in render-core in the last 7 days.` |
| empty bucket | | readout `no measurements` |
| unknown bucket | | the outline `?` under it, and the readout's unknown line |
| `telemetry_unavailable` | `render-core opened without telemetry` | `The server opened render-core without telemetry, so it keeps no metrics for it. Its log names the cause in a line that starts with tau: telemetry render-core.` |
| `unknown_project` | `render-core is gone from the server` | same |
| server without telemetry | `This server has no telemetry. It may be older than this client.` | same |
| metric in no schema now | the chart, with `not in the server's schemas now` before the figure | the chart, with the summary ending `not in the server's schemas now`, and `K` says why |

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
| F7 | A percentile is implied. | No percentile `Stat` exists. Copy test: a histogram summary and a histogram pin's figure always hold the max, and the summary the count. |
| F8 | An upDownCounter's net change reads as a level. | Copy test: the facts and summary say `net change`, and no plot says `level`. A scene shows negative bars below zero. |
| F9 | A summed attribute is hidden, for example token kinds added together. | Copy test: the summary names every attribute that is neither filtered nor grouped. |
| F10 | A chart asked before the handshake never loads, or stays dead after a reconnect. | Link test: a watch before `Ready` sends one `metrics` call after the handshake, and a reconnect asks again. |
| F11 | Charts poll while hidden or while the window is unfocused. | Link test: no calls with focus off or an empty watch set, and one call per cadence while focused. |
| F12 | Old numbers look current after the link dropped. | Link test: a failed refresh keeps the cube and sets `no_reply_since`. A screen test checks `as of 16:40`. |
| F13 | Pins are lost, or one server's pins show on another. | Store tests: round trip, two servers with separate files, `forget_server` deleting the file, and an unreadable file left unchanged. |
| F14 | Coral appears on data, or two coral cursors show. | Scenes with the stage holding the keys, with a picker holding them, and with the sidebar holding them: one coral mark at most, and the time cursor's bar only where its column crosses the selected strip. The prototype's CORAL and CURSOR checks pass on states 48 to 55. |
| F15 | A key collides, or a handler has no keymap ID. | Keymap lint with its mutants, and the existing handler-to-ID test. |
| F16 | Strips reorder under the cursor when a refresh changes their totals. | Screen test: a refresh that swaps two strips' totals keeps their order and the cursor's strip. Reopening the chart sorts again. |
| F17 | The min-to-max band vanishes under the time cursor. | Token test over the three variants: the band's fill stays at or above `tau.fill_cue_min` on `surface.read` and on `surface.cursor.read`. |
| F18 | A chart built but not pinned is lost when you leave it. | Store test: a chart left unpinned survives a restart and leads the Metric picker. Pinning it clears it. |
| F19 | An unpin can't be taken back. | Screen test: `x` then `u` puts the pin back at its place, and a second `u` does nothing. |

## Build order

Each phase ends in a check on the real artifact.

| Phase | Builds | Check |
|---|---|---|
| 0 | Nothing. Call `metrics` for `pi.ai.tokens` and `pi.durable.phase.duration` on a server with real use, over 1 hour, 24 hours, 7 days, 30 days and 90 days. | Rows, bytes and milliseconds per range recorded in `design/evidence.md`. The range list is decided. |
| 1 | The design package: `spec.md` §4.1 and §5.10, `client.md` (calls, refresh, `telemetry_unavailable`), `keymap.toml` and `reference/grammar.md` §9, `fixture.py` with schemas, rows and a refusing project. The mockups' made-up data and keys in `../tools/proto/metrics.js` move into `fixture.py` and `keymap.toml`, and the prototype's sidebar draws the Metrics row in every state. | Keymap lint with mutants passes. `fixture.json` regenerates. The prototype's build passes every check family. |
| 2 | `telemetry.rs`: catalogue parsing, `Cube`, `series`. Tests F1 and F4 to F7 first. | `cargo test -p tau-client --test telemetry` |
| 3 | Wire, engine and link: two calls, one refusal code, the watch, refresh and states; fixture answers. Tests F2, F3 and F10 to F12 first. | `cargo test -p tau-client` |
| 4 | `ui/chart.rs` with scenes built from hand-made figures: bars, line, band, signed bars, strips, the crossing cursor, in progress, unknown, Ember Light. | `nix run .#snap` of those scenes, compared with `mockups/metrics-*.png` through `nix run .#compare`, reviewed by the owner |
| 5 | The Metrics screen family: place and sidebar row, board and its peek, chart view and its details panel, the six pickers, keys, `metrics.json`. Tests F8, F9, F13 to F16, F18 and F19 first. | `cargo test -p tau-app metrics`, the scenes, then `nix run .#verify` |
| 6 | Move the flake's e2e tau input to a revision with telemetry. Add one real-server check that reads `telemetry` and one metric. | `nix run .#e2e` |

F17 is a token test and runs with the theme tests from phase 4.

The fixture's rows carry fixed dates. Scenes run on a `ManualClock` at the fixture's `now`. For `--demo`, the fixture transport shifts each row's `intervalMs` by the gap between the session's clock and the fixture's `now`, so demo charts are not empty.

## Left out of v1

- **Charts across projects**, such as cost for every project this week. Each project is a separate `metrics` call, and a chart would need a merge rule and a note for every refusing project.
- **More than one metric on a chart**, and ratios such as failures per run.
- **Reordering pins.** The sidebar's open items can't be reordered by hand either (`open-questions.md` §5). Grouping by project gives the board its order meanwhile.
- **A categorical palette** for groups. Strips stand in for it.
- **Metrics without a schema.** The server can't list them.
- **Traces.** `traces` and `trace` deserve their own view: the error traces of a project, newest first, opening one as a span tree.
- **A `:chart` command line** that parses into a `Chart`.

## Open questions for the owner

1. Should the board show live charts, as proposed, or a list of saved charts that each open on `⏎`? A list fetches nothing until opened. The board shows every pin at a glance, which is the dashboard you asked for, at one fetch per pinned metric and range. The review recommends the live board: with the peek it reads like every other place, and the peek shows what a strip leaves out.
2. Should pins stay on this device per server, or should the server keep them so every device sees one board? A server-kept board is a core change.
3. Do you want charts across projects in v1?
4. Should the Project view's Work group gain a `Metrics` row that opens the board filtered to that project? The review recommends it: the Work rows already open places filtered to the project.
5. Are neutral strips acceptable for groups, or do you want a categorical palette in the Ember tokens? The review recommends strips, because a palette would break `reference/visual-language.md` §4.2.
6. The board prints no key, as no place does, so `␣mn` is found through which-key (`␣m`, "this view"), the palette and the empty board's line. Should the board break that rule with a `␣mn new chart` segment?
7. The cut has a key per part (`␣mr`, `␣mg`, `␣mf`), and `␣pf` keeps meaning "project". The alternative is one Filter float with a group head each for project, range, split and keep only, at the cost of `␣pf` meaning more on a chart than in the places. Which do you want?

## Requests to the core

1. A `step` for `metrics` that returns one merged row per step per attribute set. It removes the reply-size risk, and the client's roll-up shrinks to a copy.
2. An opening value for an upDownCounter series, so the client can draw a level.
3. Histogram bucket counts per interval, using the schema's `boundaries`, so percentiles become possible.
4. A `workflow` attribute on `pi.ai.cost` and `pi.ai.tokens`. Cost per project exists today. Cost per workflow inside a project with several workflows does not.
5. A call that lists the metric names with data in a project.
