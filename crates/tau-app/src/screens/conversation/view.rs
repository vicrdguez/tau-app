//! The conversation page (`spec.md` §5.5): the 880 px column, centred left of the details panel,
//! holds the title block, the transcript and, at its foot, the queue, the unsent rows and the
//! composer. Sizes follow the prototype (`tools/proto/calm.css` `.ccol`, `.msg`, `.tool`,
//! `.cstack`, `.comp`).

use rune::{El, ScrollSpec, TextStyle, col, el, icon, row, spacer, text, text_input};
use tau_client::shapes::{ConversationState, QueueMode};
use tau_client::wire::InputMode;
use tau_client::{EntryId, Label, Wall};

use super::caption;
use super::state::{self, Shown, composer_edit, current_stop, shown};
use super::transcript::{Body, Item, Outcome, Part, Stop, Text, ToolRow};
use crate::app::{App, Focus, commands, input};
use crate::assets::carbon;
use crate::content::{line, markdown};
use crate::copy;
use crate::editor::EditMode;
use crate::screens::NOTHING_RECEIVED;
use crate::theme::Theme;
use crate::time;
use crate::ui::{Mark, Part as Fact, Scope, chip, facts, glyphs, mark, printed};

/// The transcript's scroll viewport.
pub const TRANSCRIPT: &str = "conv-transcript";

/// Agent text, headers and rows sit 12 px in from the column, so the cursor's band and bar span
/// it whole (prototype `.ccol .tool`, `.ccol .mh`).
const LANE: f32 = 12.0;
/// Below a message, a record or a chrome line.
const AFTER_MESSAGE: f32 = 18.0;
/// Below a tool row (`tokens.json` `layout.tool_row.gap`).
const AFTER_ROW: f32 = 4.0;
/// Between paragraphs (`tokens.json` `layout.markdown.paragraph_gap`).
const AFTER_BLOCK: f32 = 12.0;
/// Between the items of one list.
const BETWEEN_ITEMS: f32 = 4.0;
const ROW: f32 = 30.0;
/// A running row's count (`tokens.json` `layout.tool_row.duration`).
const COUNT: f32 = 44.0;

/// What every part of the transcript needs to draw.
struct Draw<'a> {
    t: Theme,
    /// Where the keys are, for whether a printed key is lit.
    scope: Scope,
    cursor: Option<&'a Stop>,
    /// The stage has the keys, so the cursor's bar is coral.
    keys: bool,
    /// The model the facts line names: a header names its own only when it differs.
    model: &'a str,
    now: Wall,
}

impl Draw<'_> {
    /// A cursor stop's element: the band and, while the stage has the keys, the 2 px coral bar.
    fn band(&self, stop: &Stop, el: El) -> El {
        self.band_on(stop, el, None)
    }

    fn fold_line(&self, stop: &Stop, el: El, fill: Option<rune::Color>) -> El {
        let target = stop.clone();
        self.band_on(stop, el, fill)
            .on_press(move |a: &mut App| super::toggle(a, target.clone()))
    }

    /// The same over a row with a fill of its own (the failed tint), which the cursor keeps.
    fn band_on(&self, stop: &Stop, el: El, fill: Option<rune::Color>) -> El {
        let el = el.key(stop.key());
        if self.cursor != Some(stop) {
            return match fill {
                Some(f) => el.fill(f),
                None => el,
            };
        }
        let el = el.fill(fill.unwrap_or(self.t.c.surface.cursor_read));
        if self.keys {
            el.border_sides([0.0, 0.0, 0.0, 2.0], self.t.c.state.attn)
        } else {
            el
        }
    }
}

pub fn page(t: Theme, app: &App) -> El {
    let Some(s) = shown(app) else {
        return el();
    };
    let st = &app.screens.conversation;
    let conv = st.conv(&s.conv);
    let composing = conv.is_some_and(|c| c.composing);
    let keys = app.floats.is_empty() && app.focus == Focus::Stage;
    let items = state::items(app, &s);
    let cursor = current_stop(app, &s);
    let model = s
        .docs
        .and_then(|d| d.agent.model.as_deref())
        .map(short_model)
        .unwrap_or_default();
    let cx = Draw {
        t,
        scope: app.scope(),
        cursor: cursor.as_ref(),
        keys: keys && !composing,
        model: &model,
        now: if s.up {
            st.now()
        } else {
            s.at.unwrap_or_else(|| st.now())
        },
    };
    let following = conv.is_none_or(|c| c.cursor.is_none());
    let column = col()
        .w_full()
        .key("conv-column")
        .max_w(t.read.conversation)
        .h_full()
        .child(title_block(t, app, &s, &model))
        .child(transcript(&cx, &s, &items, following))
        .child(stack(t, app, &s, &model, keys && composing));
    row()
        .w_full()
        .h_full()
        .justify_center()
        .px(16.0)
        .child(column)
}

/// `provider/model` → `model`, as the facts line and the caption name it.
fn short_model(m: &str) -> String {
    m.rsplit_once('/').map_or(m, |(_, id)| id).to_string()
}

/// The title (the conversation's label, one coral line) and its facts line with `details ›`.
fn title_block(t: Theme, app: &App, s: &Shown, model: &str) -> El {
    let tb = t.l.title_block;
    let label = app.server.items.iter().find_map(|(item, f)| {
        (crate::app::conv_ref(item).as_ref() == Some(&s.conv))
            .then(|| f.label.clone())
            .flatten()
    });
    let title = match label {
        Some(l) => line(t, &l, t.ty.title_page, t.c.state.attn),
        None if app.link.server().label(&s.conv) == Label::Empty => text("empty conversation")
            .font(t.ty.title_page)
            .color(t.c.fg.muted),
        None => text(s.conv.project.to_string())
            .font(t.ty.title_page)
            .color(t.c.state.attn),
    };
    let open = app.details_visible;
    let details = row()
        .center_y()
        .gap(4.0)
        .child(text("details").font(t.ty.ui_small).color(t.c.fg.muted))
        .child(
            icon(&carbon::CHEVRON_RIGHT, 12.0)
                .color(t.c.fg.muted)
                .icon_rotation(if open { std::f32::consts::PI } else { 0.0 }),
        )
        .on_press_tx(|a: &mut App, tx| commands::run_id(a, "toggle.details", tx));
    col()
        .w_full()
        .pt(tb.top)
        .child(
            row()
                .h(tb.title)
                .center_y()
                .w_full()
                .child(title.shrink().min_w(0.0)),
        )
        .child(
            row()
                .mt(tb.facts_gap)
                .h(tb.facts)
                .center_y()
                .w_full()
                .child(facts_line(t, s, model).shrink().min_w(0.0))
                .child(spacer())
                .child(details),
        )
}

/// `▶ running · claude-sonnet-4-5 · turn 6`, by state; `owned by #3`; `as of 16:41` while the
/// view's stream is down (`spec.md` §5.5, §4.9).
fn facts_line(t: Theme, s: &Shown, model: &str) -> El {
    let Some(docs) = s.docs else {
        return text(NOTHING_RECEIVED).font(t.ty.ui_small).color(t.c.fg.alt);
    };
    let marked = |m: Mark, words: &str| Fact {
        el: row().center_y().gap(2.0).child(mark(t, m)).child(
            text(words.to_string())
                .font(t.ty.ui_small)
                .color(t.c.fg.alt),
        ),
    };
    let mut parts = match &s.state {
        ConversationState::Running => vec![marked(Mark::Running, "running")],
        ConversationState::Failed { reason } => {
            vec![marked(Mark::Failed, "run failed"), Fact::text(t, reason)]
        }
        ConversationState::Stopped => vec![Fact::text(t, "stopped")],
        ConversationState::Idle => vec![Fact::text(t, "idle")],
    };
    if !model.is_empty() {
        parts.push(Fact::text(t, model));
    }
    if let Some(run) = &docs.live.run {
        parts.push(Fact::text(t, &format!("turn {}", run.turns)));
    }
    if let Some(owner) = s.owner {
        parts.push(Fact {
            el: row()
                .center_y()
                .gap(4.0)
                .child(text("owned by").font(t.ty.ui_small).color(t.c.fg.alt))
                .child(
                    text(format!("#{owner}"))
                        .font(t.ty.mono_small)
                        .color(t.c.kind.literal),
                ),
        });
    }
    if !s.up
        && let Some(at) = s.at
    {
        parts.push(Fact::text(t, &format!("as of {}", time::clock(at))));
    }
    facts(t, parts)
}

fn transcript(cx: &Draw, s: &Shown, items: &[Item], following: bool) -> El {
    let t = cx.t;
    let mut doc = col().w_full().pb(8.0);
    match s.held.map(|h| &h.docs) {
        Some(tau_client::Known::Gone { .. }) => {
            doc = doc.child(
                text("gone from the server")
                    .font(t.ty.ui)
                    .color(t.c.fg.alt)
                    .pl(LANE),
            )
        }
        _ if items.is_empty() && s.docs.is_none() => {
            doc = doc.child(
                text(NOTHING_RECEIVED)
                    .font(t.ty.ui)
                    .color(t.c.fg.alt)
                    .pl(LANE),
            )
        }
        _ => {}
    }
    for item in items {
        doc = doc.child(item_el(cx, item));
    }
    col()
        .key(TRANSCRIPT)
        .w_full()
        .mt(t.l.title_block.after)
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: following,
        })
        .child(doc)
}

fn item_el(cx: &Draw, item: &Item) -> El {
    let t = cx.t;
    match item {
        Item::Input { head, at, body } => input(cx, *head, *at, body),
        Item::Chrome {
            stop,
            words,
            text: opened,
            open,
        } => {
            let mut c = col().w_full().mb(AFTER_MESSAGE).child(
                cx.band(
                    stop,
                    row().w_full().h(16.0).center_y().px(LANE).child(
                        text(words.clone())
                            .font(t.ty.ui_small)
                            .color(t.c.fg.muted)
                            .truncate(),
                    ),
                ),
            );
            if *open && let Some(src) = opened {
                c = c.child(prose(t, src).pl(LANE).mt(6.0));
            }
            c
        }
        Item::Gap => text("fetching entries missed while away…")
            .font(t.ty.ui_small)
            .color(t.c.fg.muted)
            .pl(LANE)
            .mb(AFTER_MESSAGE),
        Item::Turn {
            head,
            at,
            model,
            parts,
        } => {
            let mut c = col()
                .w_full()
                .mb(AFTER_MESSAGE)
                .child(cx.band(&Stop::Head(*head), agent_head(cx, model, *at)));
            c = c.child(turn_parts(cx, parts));
            c
        }
        Item::Record {
            head,
            kind,
            by_task,
            body,
        } => {
            let mut h = row()
                .w_full()
                .center_y()
                .gap(8.0)
                .px(LANE)
                .py(4.0)
                .mb(1.0)
                .child(line(t, kind, t.ty.mono_small, t.c.kind.literal));
            if by_task.is_some() {
                h = h.child(text("by a task").font(t.ty.ui_small).color(t.c.fg.alt));
            }
            let body = match body {
                Some(blocks) => texts(cx, blocks),
                None => text("no text")
                    .font(t.ty.prose)
                    .color(t.c.fg.muted)
                    .pl(LANE),
            };
            col()
                .w_full()
                .mb(AFTER_MESSAGE)
                .child(cx.band(&Stop::Head(*head), h))
                .child(body)
        }
        Item::Writing { model, text: src } => {
            let mut h = row()
                .w_full()
                .center_y()
                .gap(6.0)
                .px(LANE)
                .py(4.0)
                .mb(1.0)
                .child(text("agent").font(t.ty.ui_small).color(t.c.fg.alt))
                .child(text("·").font(t.ty.ui_small).color(t.c.fg.muted))
                .child(text("writing").font(t.ty.ui_small).color(t.c.fg.alt));
            if short_model(model) != cx.model && !model.is_empty() {
                h = h.child(
                    text(short_model(model))
                        .font(t.ty.ui_small)
                        .color(t.c.fg.alt),
                );
            }
            col().w_full().mb(AFTER_MESSAGE).child(h).child(
                row().w_full().pl(LANE).child(
                    markdown(t, src)
                        .streaming(true)
                        .build()
                        .max_w(t.read.measure)
                        .grow(1.0),
                ),
            )
        }
        Item::Failure { reason, detail } => {
            let mut c = col().w_full().mt(4.0).mb(22.0).child(
                row()
                    .w_full()
                    .h(ROW)
                    .center_y()
                    .gap(8.0)
                    .px(LANE)
                    .radius(t.l.radius.pill)
                    .fill(t.c.tint.row_fail)
                    .child(mark(t, Mark::Failed))
                    .child(
                        text("Run failed")
                            .font(TextStyle {
                                weight: 600.0,
                                ..t.ty.ui
                            })
                            .color(t.c.fg.default),
                    )
                    .child(text(format!("· {reason}")).font(t.ty.ui).color(t.c.fg.alt)),
            );
            if let Some(d) = detail {
                c = c.child(
                    text(d.clone())
                        .font(t.ty.mono_small)
                        .color(t.c.fg.alt)
                        .wrap()
                        .px(LANE)
                        .mt(3.0),
                );
            }
            c.child(continue_line(cx, "continue runs it again on the same context").mt(8.0))
        }
        Item::Stopped => col().w_full().mt(4.0).mb(22.0).child(continue_line(
            cx,
            "Stopped · continue runs it again on the same context",
        )),
    }
}

/// `␣cc continue runs it again on the same context`.
fn continue_line(cx: &Draw, words: &str) -> El {
    let t = cx.t;
    let chip = chip(t, cx.scope, "<leader>cc", false);
    row().center_y().gap(6.0).px(LANE).child(chip).child(
        text(words.to_string())
            .font(t.ty.ui_small)
            .color(t.c.fg.alt),
    )
}

/// A header line: `agent` and its time, and the model when it differs from the facts line's.
fn agent_head(cx: &Draw, model: &str, at: Option<i64>) -> El {
    let t = cx.t;
    let mut h = row()
        .w_full()
        .center_y()
        .gap(8.0)
        .px(LANE)
        .py(4.0)
        .mb(1.0)
        .child(text("agent").font(t.ty.ui_small).color(t.c.fg.alt));
    let m = short_model(model);
    if !m.is_empty() && m != cx.model {
        h = h.child(text(m).font(t.ty.ui_small).color(t.c.fg.alt));
    }
    if let Some(at) = at {
        h = h.child(
            text(time::clock(Wall(at)))
                .font(t.ty.mono_small)
                .color(t.c.fg.muted),
        );
    }
    h
}

/// Your message: the container on `tint.human`, `input` in steel and its time, then the body.
fn input(cx: &Draw, head: EntryId, at: Option<i64>, body: &[Body]) -> El {
    let t = cx.t;
    let mut h = row().w_full().center_y().gap(8.0).px(16.0).h(16.0).child(
        text("input")
            .font(TextStyle {
                weight: 600.0,
                ..t.ty.ui_small
            })
            .color(t.c.kind.you),
    );
    if let Some(at) = at {
        h = h.child(
            text(time::clock(Wall(at)))
                .font(t.ty.mono_small)
                .color(t.c.fg.muted),
        );
    }
    let mut c = col()
        .w_full()
        .pt(10.0)
        .pb(12.0)
        .mb(AFTER_MESSAGE)
        .fill(t.c.tint.human)
        .border(1.0, t.c.border.hairline)
        .border_layout()
        .radius(t.l.radius.surface)
        .child(cx.band(&Stop::Head(head), h).mb(5.0));
    let mut prev: Option<&str> = None;
    for b in body {
        match b {
            Body::Text(tx) => {
                let gap = prev.map_or(0.0, |p| gap_between(p, &tx.source));
                c = c.child(
                    cx.band(
                        &tx.stop,
                        row().w_full().px(16.0).child(prose(t, &tx.source)),
                    )
                    .mt(gap),
                );
                prev = Some(&tx.source);
            }
            Body::Image(mime) => {
                c = c.child(
                    text(format!("image · {mime}"))
                        .font(t.ty.ui_small)
                        .color(t.c.fg.muted)
                        .px(16.0)
                        .mt(AFTER_BLOCK),
                )
            }
        }
    }
    c
}

/// One Markdown block in the reading measure.
fn prose(t: Theme, src: &str) -> El {
    markdown(t, src)
        .build()
        .max_w(t.read.measure)
        .grow(1.0)
        .min_w(0.0)
}

/// Blocks of a message body, each a cursor stop.
fn texts(cx: &Draw, blocks: &[Text]) -> El {
    let mut c = col().w_full();
    let mut prev: Option<&str> = None;
    for b in blocks {
        let gap = prev.map_or(0.0, |p| gap_between(p, &b.source));
        c = c.child(
            cx.band(
                &b.stop,
                row().w_full().pl(LANE).child(prose(cx.t, &b.source)),
            )
            .mt(gap),
        );
        prev = Some(&b.source);
    }
    c
}

/// The space between two blocks, as the prototype's collapsed margins put it: list items of one
/// list 4 px, a heading its own space above and below (`calm.css` `.md h2`, `.md h3`), else 12.
fn gap_between(prev: &str, next: &str) -> f32 {
    if list_item(prev) && list_item(next) {
        return BETWEEN_ITEMS;
    }
    let below = match heading(prev) {
        Some(2) => 8.0,
        Some(3..) => 6.0,
        _ => AFTER_BLOCK,
    };
    let above = match heading(next) {
        Some(2) => 26.0,
        Some(3..) => 24.0,
        _ => 0.0,
    };
    below.max(above)
}

/// An ATX heading's level.
fn heading(src: &str) -> Option<usize> {
    let s = src.trim_start();
    let level = s.chars().take_while(|c| *c == '#').count();
    ((1..=6).contains(&level) && s[level..].starts_with([' ', '\t'])).then_some(level)
}

fn list_item(src: &str) -> bool {
    let s = src.trim_start();
    let digits = s.chars().take_while(char::is_ascii_digit).count();
    s.starts_with(['-', '*', '+']) && s.chars().nth(1).is_some_and(char::is_whitespace)
        || digits > 0 && s[digits..].starts_with(['.', ')'])
}

/// A turn's parts: text blocks, thinking folds, tool rows and folds, in their order.
fn turn_parts(cx: &Draw, parts: &[Part]) -> El {
    let t = cx.t;
    let mut c = col().w_full();
    let mut after: Option<f32> = None;
    let mut prev_text: Option<&str> = None;
    for p in parts {
        let (el, below) = match p {
            Part::Text(b) => {
                let gap = match prev_text {
                    Some(prev) => gap_between(prev, &b.source),
                    None => after.unwrap_or(0.0),
                };
                prev_text = Some(&b.source);
                after = Some(AFTER_BLOCK);
                c = c.child(
                    cx.band(&b.stop, row().w_full().pl(LANE).child(prose(t, &b.source)))
                        .mt(gap),
                );
                continue;
            }
            Part::Thinking {
                stop,
                words,
                text: src,
                open,
            } => {
                let mut f = col().key(format!("{}-row", stop.key())).w_full().child(
                    cx.fold_line(
                        stop,
                        row().w_full().h(16.0).center_y().px(LANE).child(
                            text(format!("thinking · {words} words"))
                                .font(t.ty.ui_small)
                                .color(t.c.fg.muted),
                        ),
                        None,
                    ),
                );
                if *open {
                    f = f.child(
                        row()
                            .key(format!("{}-body", stop.key()))
                            .w_full()
                            .pl(LANE)
                            .mt(4.0)
                            .child(prose(t, src)),
                    );
                }
                (f, AFTER_BLOCK)
            }
            Part::Tool(r) => (tool(cx, r), AFTER_ROW),
            Part::Fold { stop, rows, open } => {
                let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
                let mut f = col().key(format!("{}-row", stop.key())).w_full().child(
                    cx.fold_line(
                        stop,
                        tool_line(
                            cx,
                            mark(t, Mark::Ok),
                            text(format!("{} tool calls", rows.len()))
                                .font(t.ty.ui)
                                .color(t.c.fg.default),
                            names.join(", "),
                            Outcome::Ok,
                            None,
                            *open,
                        ),
                        None,
                    ),
                );
                if *open {
                    for r in rows {
                        f = f.child(tool(cx, r).mt(AFTER_ROW));
                    }
                }
                (f, AFTER_ROW)
            }
        };
        prev_text = None;
        let gap = after.unwrap_or(0.0);
        c = c.child(el.mt(gap));
        after = Some(below);
    }
    c
}

/// One tool row and, when open, its full input and output in sunken containers.
fn tool(cx: &Draw, r: &ToolRow) -> El {
    let t = cx.t;
    let lead = match r.outcome {
        Outcome::Ok => mark(t, Mark::Ok),
        Outcome::Failed => mark(t, Mark::Failed),
        Outcome::Running => mark(t, Mark::Running),
        Outcome::NoResult => mark(t, Mark::Unknown),
    };
    let count = (r.outcome == Outcome::Running)
        .then_some(r.started)
        .flatten()
        .map(|at| time::span(time::between(Wall(at), cx.now)));
    let stop = Stop::Tool(r.call.clone());
    let row_el = tool_line(
        cx,
        lead,
        line(t, &r.name, t.ty.mono, t.c.fg.default),
        r.args.clone(),
        r.outcome,
        count,
        r.open,
    );
    let tint = (r.outcome == Outcome::Failed).then_some(t.c.tint.row_fail);
    let mut c = col()
        .key(format!("{}-row", stop.key()))
        .w_full()
        .child(cx.fold_line(&stop, row_el, tint));
    if let Some(out) = r.shown_output() {
        let both = r.input.is_some() && !out.is_empty();
        if let Some(input) = &r.input {
            c = c.child(tool_block(
                t,
                format!("{}-input", stop.key()),
                input,
                both.then_some("input"),
            ));
        }
        if r.input.is_none() || !out.is_empty() {
            c = c.child(tool_block(
                t,
                format!("{}-output", stop.key()),
                &out,
                both.then_some("output"),
            ));
        }
    }
    c
}

fn tool_block(t: Theme, key: String, source: &str, label: Option<&str>) -> El {
    col()
        .key(key)
        .w_full()
        .mt(AFTER_ROW)
        .mb(4.0)
        .pt(9.0)
        .pb(10.0)
        .px(14.0)
        .fill(t.c.surface.sunken)
        .border(1.0, t.c.border.hairline)
        .border_layout()
        .radius(t.l.radius.surface)
        .clip_rounded(t.l.radius.surface)
        .children(label.map(|label| {
            text(label.to_string())
                .font(t.ty.ui_small)
                .color(t.c.fg.muted)
                .mb(4.0)
        }))
        .child(
            text(source.to_string())
                .font(t.ty.mono_output)
                .color(t.c.fg.alt)
                .min_w(0.0),
        )
}

/// A tool row's line: mark, name, arguments cut with `…`, then the outcome word, the count slot
/// and the chevron.
fn tool_line(
    cx: &Draw,
    lead: El,
    name: El,
    args: String,
    outcome: Outcome,
    count: Option<String>,
    open: bool,
) -> El {
    let t = cx.t;
    let word = match outcome {
        Outcome::Ok => text("ok").font(t.ty.ui_small).color(t.c.text.ok),
        Outcome::Failed => text("failed")
            .font(TextStyle {
                weight: 700.0,
                ..t.ty.ui_small
            })
            .color(t.c.text.failed),
        Outcome::Running => text("running").font(t.ty.ui_small).color(t.c.fg.alt),
        Outcome::NoResult => text("no result")
            .font(TextStyle {
                weight: 700.0,
                ..t.ty.ui_small
            })
            .color(t.c.fg.default),
    };
    row()
        .w_full()
        .h(ROW)
        .center_y()
        .gap(10.0)
        .px(LANE)
        .radius(t.l.radius.pill)
        .child(lead)
        .child(name)
        .child(
            line(t, &args, t.ty.mono, t.c.fg.alt)
                .grow(1.0)
                .shrink()
                .min_w(0.0),
        )
        .child(word)
        .child(
            row()
                .w(COUNT)
                .justify_end()
                .children(count.map(|c| text(c).font(t.ty.mono_small).color(t.c.fg.muted))),
        )
        .child(
            icon(
                if open {
                    &carbon::CHEVRON_DOWN
                } else {
                    &carbon::CHEVRON_RIGHT
                },
                12.0,
            )
            .color(t.c.fg.muted),
        )
}

/// The column's foot: `12 new ↓`, the queue, the unsent rows and the composer.
fn stack(t: Theme, app: &App, s: &Shown, model: &str, composing: bool) -> El {
    let mut c = col().w_full().pt(14.0).pb(16.0);
    let conv = app.screens.conversation.conv(&s.conv);
    if let (Some(paused), Some(h)) = (conv.and_then(|c| c.paused_at), s.held) {
        let new = h
            .entries
            .range(EntryId(paused.0 + 1)..)
            .filter(|(_, e)| !matches!(e.body, tau_client::shapes::EntryBody::ToolResult { .. }))
            .count();
        if new > 0 {
            c = c.child(
                row()
                    .center_y()
                    .gap(4.0)
                    .mb(10.0)
                    .child(
                        text(format!("{new} new"))
                            .font(t.ty.ui_small)
                            .color(t.c.fg.alt),
                    )
                    .child(icon(&carbon::ARROW_DOWN, 12.0).color(t.c.fg.alt)),
            );
        }
    }
    if let Some(docs) = s.docs
        && !docs.queue.is_empty()
    {
        for q in &docs.queue {
            let word = match &q.mode {
                QueueMode::Steer => "steer".to_string(),
                QueueMode::FollowUp => "follow-up".to_string(),
                QueueMode::Write => "write".to_string(),
                QueueMode::Other(o) => o.clone(),
            };
            c = c.child(
                col()
                    .w_full()
                    .mb(10.0)
                    .child(
                        row()
                            .center_y()
                            .gap(4.0)
                            .mb(4.0)
                            .child(text(word).font(t.ty.ui_small).color(t.c.kind.you))
                            .child(text("· queued").font(t.ty.ui_small).color(t.c.fg.muted)),
                    )
                    .child(content_bar(t, &q.text)),
            );
        }
        c = c.child(
            text("queued inputs can't be taken back")
                .font(t.ty.ui_small)
                .color(t.c.fg.muted)
                .ml(14.0)
                .mb(14.0),
        );
    }
    let now = app.screens.conversation.now();
    for e in app.link.outbox().unsent_inputs(&s.conv) {
        let row_words = copy::outbox_row(e, now);
        let word = match e.mode() {
            InputMode::Steer => "steer",
            InputMode::FollowUp => "follow-up",
            InputMode::Send => "send",
        };
        let mut head = row().center_y().gap(5.0).mb(4.0);
        if row_words.unknown {
            head = head.child(mark(t, Mark::Unknown));
        }
        head = head.child(text(word).font(t.ty.ui_small).color(t.c.fg.default));
        if !row_words.unknown {
            head = head.child(
                text(format!("· {}", row_words.state))
                    .font(t.ty.ui_small)
                    .color(t.c.fg.muted),
            );
        }
        let mut unsent = col()
            .w_full()
            .mb(10.0)
            .child(head)
            .child(content_bar(t, e.text().unwrap_or_default()));
        if row_words.unknown {
            let mut words = row().center_y().gap(6.0).ml(14.0).mt(4.0).child(
                text(row_words.state.clone())
                    .font(t.ty.ui_small)
                    .color(t.c.fg.alt),
            );
            if let Some(hint) = row_words.retry {
                words = words
                    .child(text("·").font(t.ty.ui_small).color(t.c.fg.muted))
                    .child(chip(t, app.scope(), hint.key, false))
                    .child(text(hint.words).font(t.ty.ui_small).color(t.c.fg.alt));
            }
            unsent = unsent.child(words);
        }
        c = c.child(unsent);
    }
    c.child(composer(t, app, s, model, composing))
}

/// Content on the 2 px content bar, in `prose.small`, two lines at most.
fn content_bar(t: Theme, src: &str) -> El {
    row()
        .w_full()
        .pl(12.0)
        .border_sides([0.0, 0.0, 0.0, 2.0], t.c.border.strong)
        .max_h(2.0 * t.ty.prose_small.line_height)
        .clip_rounded(0.0)
        .child(markdown(t, src).peek().build().grow(1.0).min_w(0.0))
}

/// The composer: the container with its caption, the draft, and its hints while it has the keys.
fn composer(t: Theme, app: &App, s: &Shown, model: &str, composing: bool) -> El {
    let conv = app.screens.conversation.conv(&s.conv);
    let ed = conv.map(|c| &c.composer);
    let insert = composing && ed.is_some_and(|e| e.mode() == EditMode::Insert);
    let (word, rest) = caption(&s.state, model);
    let cap = row()
        .center_y()
        .gap(5.0)
        .child(
            text(word)
                .font(TextStyle {
                    weight: 600.0,
                    ..t.ty.ui_small
                })
                .color(t.c.kind.you),
        )
        .child(
            text(format!("· {rest}"))
                .font(t.ty.ui_small)
                .color(t.c.kind.you),
        );
    let draft = ed.map(|e| e.text()).unwrap_or_default();
    let body = match ed {
        Some(e) if composing => {
            let mut edit = e.edit.clone();
            edit.focused = true;
            text_input(&edit, t.ty.prose)
                .wrap_anywhere()
                .caret(t.c.state.attn, t.c.surface.selection)
                .color(t.c.fg.default)
                .on_edit(composer_edit)
                .on_input_key(input::on_field_key)
                .label("Composer")
                .key("composer")
                .w_full()
        }
        _ if !draft.is_empty() => text(draft.to_string())
            .font(t.ty.prose)
            .color(t.c.fg.default)
            .wrap()
            .w_full(),
        _ => el().h(24.0),
    };
    let mut c = col()
        .key("composer-container")
        .w_full()
        .min_h(68.0)
        .pt(11.0)
        .px(14.0)
        .pb(10.0)
        .fill(t.c.surface.raised)
        .border(
            1.0,
            if insert {
                t.c.state.attn
            } else {
                t.c.border.hairline
            },
        )
        .border_layout()
        .radius(t.l.radius.surface)
        .child(cap)
        .child(row().w_full().mt(6.0).min_h(24.0).child(body));
    if composing {
        c = c.child(
            hints(
                t,
                app.scope(),
                insert,
                s.state == ConversationState::Running,
            )
            .mt(6.0),
        );
    }
    c.on_press(|a: &mut App| super::compose(a))
}

/// `Esc jk leave Insert · ⌘⏎ steer · M-⏎ follow-up` (running) or `· ⌘⏎ send`; in the composer's
/// Normal mode `⌘⏎ send · Esc back to the conversation`.
fn hints(t: Theme, scope: Scope, insert: bool, running: bool) -> El {
    let key = |seq: &str, lit: bool| {
        glyphs(
            &printed(seq),
            t.ty.hint,
            if lit { t.c.fg.alt } else { t.c.fg.muted },
        )
    };
    let word = |w: &str| text(w.to_string()).font(t.ty.ui_small).color(t.c.fg.muted);
    let group = |parts: Vec<El>| row().center_y().gap(6.0).children(parts);
    let mut groups = Vec::new();
    if insert {
        // Esc and jk are the editor's own Insert keys, so they are lit wherever it has the keys.
        groups.push(group(vec![
            key("<Esc>", true),
            key("jk", true),
            word("leave Insert"),
        ]));
        groups.push(group(vec![
            key("<D-CR>", scope.lit("<D-CR>")),
            word(if running { "steer" } else { "send" }),
        ]));
        if running {
            groups.push(group(vec![
                key("<M-CR>", scope.lit("<M-CR>")),
                word("follow-up"),
            ]));
        }
    } else {
        groups.push(group(vec![
            key("<D-CR>", scope.lit("<D-CR>")),
            word("send"),
        ]));
        groups.push(group(vec![
            key("<Esc>", scope.lit("<Esc>")),
            word("back to the conversation"),
        ]));
    }
    row()
        .w_full()
        .justify_end()
        .center_y()
        .gap(t.l.gaps.hint)
        .children(groups)
}
