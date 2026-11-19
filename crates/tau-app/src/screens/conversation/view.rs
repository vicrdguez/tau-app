//! The conversation page (`spec.md` §5.5): the column, centred in the stage and moved only as far
//! as an open details panel needs (§4.7), holds the title block, the transcript and, at its foot,
//! the queue, the unsent rows and the composer. Every line of text starts on one axis, and every
//! box spans the text column and [`LANE`] on each side; left of the boxes is the [`GUTTER`].
//! Other sizes follow the prototype (`tools/proto/calm.css` `.msg`, `.tool`, `.cstack`, `.comp`).

use rune::{
    Color, El, Icon, ScrollSpec, TextStyle, Transition, col, el, icon, row, spacer, text,
    text_input,
};
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
use crate::theme::{Motion, Theme};
use crate::time;
use crate::ui::{Mark, Part as Fact, Scope, chip, facts, glyphs, mark};

/// The transcript's scroll viewport.
pub const TRANSCRIPT: &str = "conv-transcript";
/// What the transcript scrolls.
const DOC: &str = "conv-doc";

/// The column's left 24 px, where a message's speaker mark stands (`visual-language.md` §6).
const GUTTER: f32 = 24.0;
/// How far every box (a container, a band, a tint) reaches past the text on each side, so text
/// in a box and text outside one start on one x.
const LANE: f32 = 16.0;
/// A container's padding: its 1 px hairline takes the rest of [`LANE`].
const INSIDE: f32 = LANE - 1.0;
/// The content bar stands this far left of its text.
const BAR: f32 = 12.0;
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
        let band = el.fill(fill.unwrap_or(self.t.c.surface.cursor_read));
        let attn = self.t.c.state.attn;
        match (self.keys, stop) {
            (false, _) => band,
            // A hung code block covers its text block's left edge, so the bar draws over it.
            (true, Stop::Text(..)) => band.child(rune::el().w(2.0).fill(attn).pin(
                Some(0.0),
                Some(0.0),
                None,
                Some(0.0),
            )),
            (true, _) => band.border_sides([0.0, 0.0, 0.0, 2.0], attn),
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
    let ride = cursor
        .as_ref()
        .filter(|_| !following && app.float_drawn())
        .and_then(|c| ride(t, &c.key()));
    let stage = crate::screens::stage_width(t, app);
    let mut width = (stage - 32.0).min(t.read.conversation);
    let mut left = (stage - width) / 2.0;
    if app.details_visible {
        let room = stage - t.l.details.width;
        width = width.min(room - 32.0);
        left = crate::screens::beside_details(t, left, width, room);
    }
    let column = t
        .motion
        .moves(col().key("conv-column"), t.motion.page)
        .transition(Transition::none())
        .w(width)
        .h_full()
        .child(
            title_block(t, app, &s, &model)
                .pl(GUTTER + LANE)
                .pr(LANE)
                .instant_changes(),
        )
        .child(transcript(&cx, &s, &items, following, ride))
        .child(stack(t, app, &s, &model, keys && composing).pl(GUTTER));
    row().w_full().h_full().pl(left).child(column)
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

/// The `sticky_bottom` inset that holds the cursor's stop above the transcript's foot while the
/// foot rides a bottom float's edge (`spec.md` §4): the stop stays put until the foot comes
/// within `float.inset_x` of it, or of the room it had under it if that is less, then rides it.
fn ride(t: Theme, stop: &str) -> Option<f32> {
    use rune::memo::{layout_with, window_layout_with};
    let bottom = |r: [f32; 4]| r[1] + r[3];
    let foot = layout_with(TRANSCRIPT, bottom)? + layout_with(crate::frame::FLOAT_BAND, |r| r[3])?;
    let under = foot - window_layout_with(stop, bottom)?;
    let below = layout_with(DOC, bottom)? - layout_with(stop, bottom)?;
    Some(t.l.float.inset_x.min(under) - below)
}

fn transcript(cx: &Draw, s: &Shown, items: &[Item], following: bool, ride: Option<f32>) -> El {
    let t = cx.t;
    let mut doc = col().key(DOC).w_full().pl(GUTTER).pb(8.0);
    match s.held.map(|h| &h.docs) {
        Some(tau_client::Known::Gone { .. }) => {
            doc = doc.child(
                text("gone from the server")
                    .font(t.ty.ui)
                    .color(t.c.fg.alt)
                    .mx(LANE),
            )
        }
        _ if items.is_empty() && s.docs.is_none() => {
            doc = doc.child(
                text(NOTHING_RECEIVED)
                    .font(t.ty.ui)
                    .color(t.c.fg.alt)
                    .mx(LANE),
            )
        }
        _ => {}
    }
    for item in items {
        doc = doc.child(item_el(cx, item));
    }
    if let Some(inset) = ride {
        doc = doc.sticky_bottom(inset);
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
            let mut c = col()
                .key(format!("{}-row", stop.key()))
                .clip_rounded(0.0)
                .w_full()
                .mb(AFTER_MESSAGE)
                .child(
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
                c = c.child(
                    prose(t, src, LANE)
                        .mx(LANE)
                        .mt(6.0)
                        .transition(Transition::opacity().springs(Motion::pair(t.motion.page))),
                );
            }
            c
        }
        Item::Gap => text("fetching entries missed while away…")
            .font(t.ty.ui_small)
            .color(t.c.fg.muted)
            .mx(LANE)
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
                    .mx(LANE),
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
                .child(speaker(&carbon::BOT, t.c.fg.muted, 0.0))
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
                row().w_full().px(LANE).child(
                    markdown(t, src)
                        .streaming(true)
                        .hang(LANE)
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
                        .mx(LANE)
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

/// A header line: the robot in the gutter, `agent` and its time, and the model when it differs
/// from the facts line's.
fn agent_head(cx: &Draw, model: &str, at: Option<i64>) -> El {
    let t = cx.t;
    let mut h = row()
        .w_full()
        .center_y()
        .gap(8.0)
        .px(LANE)
        .py(4.0)
        .mb(1.0)
        .child(speaker(&carbon::BOT, t.c.fg.muted, 0.0))
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

/// A message's speaker mark (`visual-language.md` §6): a 16 px slot at the column's left edge,
/// 8 px clear of the box, centred on the header line, which starts `inset` inside the box edge.
/// It has no name: the header's word names the speaker.
fn speaker(glyph: &'static Icon, color: Color, inset: f32) -> El {
    row()
        .w(16.0)
        .center()
        .pin(Some(-GUTTER - inset), Some(0.0), None, Some(0.0))
        .child(icon(glyph, 14.0).color(color))
}

/// Your message: the container on `tint.human`, the person in the gutter, `input` in steel and its
/// time, then the body, whose code blocks keep the text's width: they can't reach past the
/// container.
fn input(cx: &Draw, head: EntryId, at: Option<i64>, body: &[Body]) -> El {
    let t = cx.t;
    let mut h = row()
        .w_full()
        .center_y()
        .gap(8.0)
        .px(INSIDE)
        .h(16.0)
        .child(speaker(&carbon::USER, t.c.state.stale, LANE - INSIDE))
        .child(
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
                        row().w_full().px(INSIDE).child(prose(t, &tx.source, 0.0)),
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
                        .mx(INSIDE)
                        .mt(AFTER_BLOCK),
                )
            }
        }
    }
    c
}

/// One Markdown block in the reading measure, its code blocks reaching `hang` past it.
fn prose(t: Theme, src: &str, hang: f32) -> El {
    markdown(t, src)
        .hang(hang)
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
                row().w_full().px(LANE).child(prose(cx.t, &b.source, LANE)),
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
                    cx.band(
                        &b.stop,
                        row().w_full().px(LANE).child(prose(t, &b.source, LANE)),
                    )
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
                let mut f = col()
                    .key(format!("{}-row", stop.key()))
                    .clip_rounded(0.0)
                    .w_full()
                    .child(
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
                            .transition(Transition::opacity().springs(Motion::pair(t.motion.page)))
                            .w_full()
                            .px(LANE)
                            .mt(4.0)
                            .child(prose(t, src, LANE)),
                    );
                }
                (f, AFTER_BLOCK)
            }
            Part::Tool(r) => (tool(cx, r), AFTER_ROW),
            Part::Fold { stop, rows, open } => {
                let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
                let mut f = col()
                    .key(format!("{}-row", stop.key()))
                    .clip_rounded(0.0)
                    .w_full()
                    .child(
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
        .transition(Transition::opacity().springs(Motion::pair(t.motion.page)))
        .clip_rounded(0.0)
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
        .transition(Transition::opacity().springs(Motion::pair(t.motion.page)))
        .w_full()
        .mt(AFTER_ROW)
        .mb(4.0)
        .pt(9.0)
        .pb(10.0)
        .px(INSIDE)
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
                    .px(LANE)
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
                    .px(LANE)
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
                .mx(LANE)
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
            .key(format!("unsent-{}", e.id.0))
            .clip_rounded(0.0)
            .transition(
                Transition::offset(0.0, 8.0)
                    .combined(Transition::opacity())
                    .springs(Motion::pair(t.motion.page)),
            )
            .w_full()
            .px(LANE)
            .mb(10.0)
            .child(head)
            .child(content_bar(t, e.text().unwrap_or_default()));
        if row_words.unknown {
            let mut words = row().center_y().gap(6.0).mt(4.0).child(
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

/// Content on the 2 px content bar, in `prose.small`, two lines at most. The bar hangs left of
/// the axis, inside the boxes' [`LANE`], so the text keeps the axis.
fn content_bar(t: Theme, src: &str) -> El {
    row()
        .ml(-BAR)
        .pl(BAR)
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
        .px(INSIDE)
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
        .child(cap.instant_changes())
        .child(
            row()
                .w_full()
                .mt(6.0)
                .min_h(24.0)
                .child(body.instant_changes()),
        );
    if composing {
        c = c.child(
            hints(
                t,
                app.scope(),
                insert,
                s.state == ConversationState::Running,
            )
            .instant_changes()
            .transition(Transition::none())
            .mt(6.0),
        );
    }
    c.on_press_tx(|a: &mut App, tx| {
        super::compose(a);
        tx.instant();
    })
}

/// `Esc jk leave Insert · ⌘⏎ steer · M-⏎ follow-up` (running) or `· ⌘⏎ send`; in the composer's
/// Normal mode `⌘⏎ send · Esc back to the conversation`.
fn hints(t: Theme, scope: Scope, insert: bool, running: bool) -> El {
    let key = |seq: &str, always_lit: bool| {
        let typed = t.keymap.chip(seq, scope.mode, scope.ctx);
        glyphs(
            &typed.printed,
            t.ty.hint,
            if always_lit || typed.lit {
                t.c.fg.alt
            } else {
                t.c.fg.muted
            },
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
            key("<D-CR>", false),
            word(if running { "steer" } else { "send" }),
        ]));
        if running {
            groups.push(group(vec![key("<M-CR>", false), word("follow-up")]));
        }
    } else {
        groups.push(group(vec![key("<D-CR>", false), word("send")]));
        groups.push(group(vec![
            key("<Esc>", false),
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
