//! The page (`spec.md` §5.6): the title block and its facts, the failed block, the basis as
//! Markdown (or source) block by block with the reader's cursor, the choices, and the notes margin
//! beside the column; the details panel's About.

use std::ops::Range;

use rune::{El, ScrollSpec, Span, col, el, icon, rich, row, text};
use tau_client::model::Liveness;
use tau_client::wire::DecisionView;
use tau_client::{DecisionId, DecisionRef, Known};

use super::doc::{Doc, Kind, byte_of};
use super::margin::{self, place};
use super::notes::Draft;
use super::reader::{self, record, shown};
use super::standing::{Standing, Then, ThenMark, standing, then};
use super::state::{Anchor, Cursor};
use super::{decision_layout, now};
use crate::app::{App, Focus};
use crate::assets::carbon;
use crate::content::{Paint, Tint, line, markdown};
use crate::frame::SidebarPlace;
use crate::screens::{NOTHING_RECEIVED, item_title, stub};
use crate::theme::Theme;
use crate::time;
use crate::ui::{Mark, Part, chip, facts, group_head, mark, notice};

/// A note in the margin, wherever it came from.
struct Item {
    /// Where its passage starts, in bytes of `text`; the block it is beside.
    at: usize,
    block: usize,
    /// Its tint's index in that block's Markdown, for the passage's first line.
    tint: Option<usize>,
    body: El,
    /// The note being typed sits 11 px higher, so its text lines up with the passage.
    lift: f32,
}

pub fn page(t: Theme, app: &App) -> El {
    let Some(d) = shown(app) else {
        return el();
    };
    let title = match &app.stage {
        crate::app::Stage::Item(item) => item_title(app, item),
        _ => String::new(),
    };
    match app.link.server().decisions.get(&d) {
        Some(Known::Have { value, .. }) => full(t, app, &d, value),
        Some(Known::Gone { .. }) => stub(t, &title, "gone from the server"),
        _ => stub(t, &title, NOTHING_RECEIVED),
    }
}

/// The stage's width left of the details panel.
fn stage_width(t: Theme, app: &App) -> f32 {
    let laid = rune::memo::layout_size("stage").map(|s| s[0]);
    let mut w = laid.unwrap_or_else(|| {
        let side = app.sidebar_place() == Some(SidebarPlace::Beside);
        app.window[0] - if side { t.l.sidebar.width } else { 0.0 }
    });
    if app.details_visible {
        w -= t.l.details.width;
    }
    w
}

fn keys_on_page(app: &App) -> bool {
    app.floats.is_empty() && app.focus == Focus::Stage
}

fn full(t: Theme, app: &App, d: &DecisionRef, v: &DecisionView) -> El {
    let l = decision_layout();
    let s = &app.screens.decision;
    let live = app.link.server().inbox.value().map(|i| i.liveness(d));
    let unsettled = app.link.outbox().answer_unsettled(d);
    let st = standing(v, live.as_ref(), unsettled);
    let then_state = v.then.as_ref().map(then);
    let doc = reader::doc(app, d).unwrap_or_else(|| Doc::rendered(""));
    let source = s.page_of(d).is_some_and(|p| p.source);
    let cursor = reader::cursor(app, d);
    let keys = keys_on_page(app) && s.editing.is_none();
    let selection = reader::selection(app, d);
    let stage_w = stage_width(t, app);
    let folded = stage_w < l.fold_notes_below;
    let column_w = if folded {
        (stage_w - 32.0).min(l.text)
    } else {
        l.text
    };
    let left = if folded {
        (stage_w - column_w) / 2.0
    } else {
        (stage_w - l.block) / 2.0
    }
    .max(16.0);

    let answered = matches!(st, Standing::Answered { .. });
    // The notes the margin shows: the answer's once answered, else this device's drafts.
    let shown_notes: Vec<Draft> = if answered {
        v.decision
            .annotations
            .iter()
            .map(Draft::from_annotation)
            .collect()
    } else {
        s.notes.of(d).to_vec()
    };
    let bytes = |r: &Range<u64>| -> Option<Range<usize>> {
        Some(byte_of(&v.text, r.start as usize)?..byte_of(&v.text, r.end as usize)?)
    };
    let editing = s.editing.as_ref().filter(|e| &e.decision == d);
    let editing_range = editing.and_then(|e| match &e.anchor {
        Anchor::Range { range, .. } => bytes(range),
        Anchor::Whole => None,
    });
    let cursor_at = match cursor {
        Some(Cursor::Text(at)) => Some(at),
        _ => None,
    };
    // The active passage: the note being typed, else the one under the cursor.
    let active = editing_range.clone().or_else(|| {
        let at = cursor_at?;
        shown_notes
            .iter()
            .filter_map(|n| bytes(n.range.as_ref()?))
            .find(|r| r.contains(&at))
            .or_else(|| {
                let block = doc.block_of(at);
                shown_notes
                    .iter()
                    .filter_map(|n| bytes(n.range.as_ref()?))
                    .find(|r| doc.block_of(r.start) == block)
            })
    });

    // Tints per block, and where each passage note's tint is.
    let mut tints: Vec<Vec<Tint>> = vec![Vec::new(); doc.blocks.len()];
    let mut passages: Vec<(usize, usize, Option<usize>, Option<&Draft>)> = Vec::new();
    let mut ranges: Vec<(Range<usize>, Option<&Draft>)> = shown_notes
        .iter()
        .filter_map(|n| Some((bytes(n.range.as_ref()?)?, Some(n))))
        .collect();
    if let Some(r) = editing_range.clone()
        && !ranges.iter().any(|(x, _)| *x == r)
    {
        ranges.push((r, None));
    }
    for (r, note) in &ranges {
        let mut first = None;
        for (k, b) in doc.blocks.iter().enumerate() {
            let a = r.start.max(b.range.start);
            let z = r.end.min(b.range.end);
            if a < z {
                tints[k].push(Tint {
                    range: a - b.range.start..z - b.range.start,
                    active: active.as_ref() == Some(r),
                });
                first.get_or_insert((k, tints[k].len() - 1));
            }
        }
        let (k, i) = first.map_or((doc.block_of(r.start).unwrap_or(0), None), |(k, i)| {
            (k, Some(i))
        });
        passages.push((r.start, k, i, *note));
    }

    // The margin's items beside passages.
    let sent_words = || match &st {
        Standing::Answered { by, at, .. } => {
            let mut w = "sent".to_string();
            if let Some(by) = by {
                w.push_str(&format!(" by {by}"));
            }
            if let Some(at) = at {
                w.push_str(&format!(" · {}", time::clock(*at)));
            }
            w
        }
        _ => "not sent".to_string(),
    };
    let mut items: Vec<Item> = Vec::new();
    for (at, block, tint, note) in &passages {
        let typed_here = editing_range.as_ref().is_some_and(|r| r.start == *at);
        let body = match (editing.filter(|_| typed_here), note) {
            (Some(e), _) => Some((margin::editing(t, app.scope(), e, keys_on_page(app)), 11.0)),
            (None, Some(n)) => Some((margin::note(t, &sent_words(), !answered, &n.text), 0.0)),
            (None, None) => None,
        };
        if let Some((body, lift)) = body {
            items.push(Item {
                at: *at,
                block: *block,
                tint: *tint,
                body,
                lift,
            });
        }
    }
    // The ghost `+ note c` at the cursor line, until three notes are written here.
    let open = matches!(st, Standing::Open { .. });
    if open
        && keys
        && s.notes.written() < 3
        && let Some(at) = cursor_at
        && let Some(block) = doc.block_of(at)
        && !items.iter().any(|i| i.block == block)
    {
        items.push(Item {
            at,
            block,
            tint: None,
            body: row()
                .center_y()
                .gap(6.0)
                .h(18.0)
                .child(text("+ note").font(t.ty.ui_small).color(t.c.fg.muted))
                .child(chip(t, app.scope(), "c", false)),
            lift: 0.0,
        });
    }
    items.sort_by_key(|i| i.at);

    // The margin's top: head, the note on the whole, the earlier requests.
    let whole_note = shown_notes.iter().find(|n| n.range.is_none());
    // The note being typed counts once it is new.
    let count = shown_notes.len() + usize::from(editing.is_some_and(|e| e.draft.is_none()));
    let head_words: Vec<String> = match &st {
        Standing::Answered { by, at, .. } => {
            let who = by.clone().map_or(String::new(), |b| format!(" by {b}"));
            let when = at.map_or(String::new(), |a| format!(" {}", time::clock(a)));
            if count == 0 {
                vec!["none".into(), format!("answered{who}{when}")]
            } else {
                vec![count.to_string(), format!("sent{who}{when}")]
            }
        }
        _ if count == 0 => vec!["none".into()],
        _ => vec![count.to_string(), "not sent".into()],
    };
    let mut top = col()
        .w_full()
        .child(margin::head(t, !answered, &head_words));
    match editing.filter(|e| e.anchor == Anchor::Whole) {
        Some(e) => {
            top = top.child(
                margin::editing(t, app.scope(), e, keys_on_page(app))
                    .mt(14.0)
                    .mx(-15.0),
            )
        }
        None => {
            if let Some(n) = whole_note {
                top = top.child(margin::whole(t, &n.text).mt(14.0));
            }
        }
    }
    for e in earlier(t, app, d) {
        top = top.child(e.mt(14.0));
    }

    // The column.
    let mut column =
        col()
            .w(column_w)
            .pb(120.0)
            .child(title_block(t, app, v, &st, then_state.as_ref()));
    if let Some(th) = then_state.as_ref().filter(|th| th.faulted && answered) {
        column = column.child(failed_block(t, v, th));
    }
    let top = if folded {
        column = column.child(top.mb(24.0));
        None
    } else {
        Some(top)
    };
    let bar = if keys { t.c.state.attn } else { t.c.fg.muted };
    let mut inline: Vec<Vec<El>> = (0..doc.blocks.len()).map(|_| Vec::new()).collect();
    if folded {
        for it in items.drain(..) {
            if let Some(slot) = inline.get_mut(it.block) {
                slot.push(it.body);
            }
        }
    }
    for (k, b) in doc.blocks.iter().enumerate() {
        let src = &v.text[b.range.clone()];
        let in_block = |at: usize| doc.block_of(at) == Some(k);
        let mut paints = Vec::new();
        if let Some(sel) = &selection {
            let a = sel.start.max(b.range.start);
            let z = sel.end.min(b.range.end);
            if a < z {
                paints.push(Paint {
                    range: a - b.range.start..z - b.range.start,
                    fg: None,
                    bg: t.c.surface.selection,
                });
            }
        }
        // While a note is typed its caret is the cursor; the reader's band steps aside.
        let here = cursor_at.is_some_and(in_block) && editing.is_none();
        if here
            && keys
            && let Some(at) = cursor_at
        {
            paints.push(Paint {
                range: at - b.range.start..doc.after(at) - b.range.start,
                fg: Some(t.c.surface.read),
                bg: t.c.state.attn,
            });
        }
        let body = if source {
            source_line(t, src, &tints[k], &paints)
        } else {
            markdown(t, src)
                .tint(tints[k].clone())
                .paint(paints)
                .build()
        };
        let mut block = col().w_full().key(format!("b{k}")).mt(gap_before(
            &doc,
            k,
            source.then_some((v.text.as_str(), t.ty.mono.line_height)),
        ));
        if here {
            block = block
                .child(
                    el().absolute(-40.0, -1.0)
                        .size(column_w + 40.0, 23.0)
                        .fill(t.c.surface.cursor_read)
                        .key("decision-cursor"),
                )
                .child(el().absolute(-40.0, -1.0).size(2.0, 23.0).fill(bar));
        }
        block = block.child(body);
        column = column.child(block);
        for n in std::mem::take(&mut inline[k]) {
            column = column.child(n.mt(4.0).mb(14.0));
        }
    }
    column = column.child(choices(t, v, &st, cursor, keys, column_w));

    let mut content = row().key("decision-row").pl(left).child(column);
    if let Some(top) = top {
        let origin = rune::memo::layout("decision-row").map_or(0.0, |r| r[1]);
        let top_h = rune::memo::layout_size("decision-mtop").map_or(90.0, |s| s[1]);
        let mut want = Vec::new();
        let mut heights = Vec::new();
        for (j, it) in items.iter().enumerate() {
            let line = it
                .tint
                .and_then(|i| rune::memo::layout(&format!("b{}/note-{i}", it.block)))
                .or_else(|| rune::memo::layout(&format!("b{}", it.block)));
            want.push(line.map_or(0.0, |r| r[1] - origin) - it.lift);
            heights.push(rune::memo::layout_size(&format!("mn-{j}")).map_or(40.0, |s| s[1]));
        }
        let placed = place(&want, &heights, 34.0 + top_h + margin::GAP);
        let mut m = el().w(l.notes_margin).ml(l.gap).child(
            top.key("decision-mtop")
                .absolute(0.0, 34.0)
                .w(l.notes_margin),
        );
        let mut leaders = Vec::new();
        for (j, (it, p)) in items.into_iter().zip(&placed).enumerate() {
            m = m.child(
                it.body
                    .key(format!("mn-{j}"))
                    .absolute(if it.lift > 0.0 { -15.0 } else { 0.0 }, p.y)
                    .w(l.notes_margin + if it.lift > 0.0 { 30.0 } else { 0.0 }),
            );
            if p.leader {
                let y0 = want[j] + it.lift + 10.0;
                leaders.extend(margin::leader(
                    t,
                    left + column_w + 6.0,
                    y0,
                    left + column_w + l.gap - 6.0 - if it.lift > 0.0 { 15.0 } else { 0.0 },
                    p.y + 8.0,
                ));
            }
        }
        content = content.child(m);
        for ld in leaders {
            content = content.child(ld);
        }
    }

    let scroll = col()
        .key("decision-page")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(content.w_full())
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .scroll_margin(24.0);
    let mut page = col().w_full().h_full().fill(t.c.surface.read);
    if let Some(b) = band(t, app, d, &st, live.as_ref()) {
        page = page.child(b);
    }
    page = page.child(scroll);
    if let Some(h) = rune::memo::layout_size("decision-readback").map(|s| s[1])
        && crate::screens::holder(app) == Some(crate::screens::ScreenId::Decision)
        && !app.floats.is_empty()
    {
        page = page.child(el().h(h + t.l.float.inset_bottom).w_full());
    }
    page
}

/// Space above block `k`: the prose's paragraph gap, list items closer, headings further; in
/// the source view, one line per blank line.
fn gap_before(doc: &Doc, k: usize, source: Option<(&str, f32)>) -> f32 {
    if k == 0 {
        return 0.0;
    }
    if let Some((text, line)) = source {
        let between = &text[doc.blocks[k - 1].range.end..doc.blocks[k].range.start];
        return between.matches('\n').count().saturating_sub(1) as f32 * line;
    }
    let (prev, kind) = (doc.blocks[k - 1].kind, doc.blocks[k].kind);
    match (prev, kind) {
        (Kind::Heading(1), _) => 12.0,
        (_, Kind::Heading(2)) => 26.0,
        (_, Kind::Heading(_)) => 24.0,
        (Kind::Item, Kind::Item) => 4.0,
        (Kind::Heading(_), _) => 8.0,
        _ => 12.0,
    }
}

/// A line of the source view: mono, every character as typed, with the same tints and paints.
fn source_line(t: Theme, src: &str, tints: &[Tint], paints: &[Paint]) -> El {
    let mut spans = Vec::new();
    for tint in tints {
        let bg = if tint.active {
            t.c.tint.note_active
        } else {
            t.c.tint.note_range
        };
        spans.push(Span::new(tint.range.start, tint.range.end).bg(bg, 0.0));
    }
    for p in paints {
        let mut s = Span::new(p.range.start, p.range.end).bg(p.bg, 0.0);
        if let Some(fg) = p.fg {
            s = s.color(fg);
        }
        spans.push(s);
    }
    rich(src.to_string(), spans)
        .font(t.ty.mono)
        .color(t.c.fg.default)
        .wrap()
        .w_full()
}

/// `› Earlier: #1 revise · 2 notes`, newest first; opened, each note with the passage it quoted.
fn earlier(t: Theme, app: &App, d: &DecisionRef) -> Vec<El> {
    let Some(n) = d.id.request_number() else {
        return Vec::new();
    };
    let subject = d.id.subject();
    let open = app
        .screens
        .decision
        .page_of(d)
        .map(|p| p.earlier.clone())
        .unwrap_or_default();
    (1..n)
        .rev()
        .map(|i| {
            let id = DecisionId::from(format!("{subject}#{i}"));
            let r = DecisionRef::new(d.project.clone(), id.clone());
            let rec = record(app, &r);
            let is_open = open.contains(&id);
            let mut head = row()
                .center_y()
                .gap(6.0)
                .h(16.0)
                .child(
                    icon(
                        if is_open {
                            &carbon::CHEVRON_DOWN
                        } else {
                            &carbon::CHEVRON_RIGHT
                        },
                        12.0,
                    )
                    .color(t.c.fg.muted),
                )
                .child(text("Earlier:").font(t.ty.ui_small).color(t.c.fg.alt))
                .child(
                    text(format!("#{i}"))
                        .font(t.ty.mono_small)
                        .color(t.c.kind.literal),
                );
            if let Some(v) = rec {
                if let Some(c) = &v.decision.choice {
                    head = head.child(text(c.clone()).font(t.ty.ui_small).color(t.c.fg.alt));
                }
                let k = v.decision.annotations.len();
                head = head
                    .child(text("·").font(t.ty.ui_small).color(t.c.fg.muted))
                    .child(
                        text(format!("{k} {}", if k == 1 { "note" } else { "notes" }))
                            .font(t.ty.ui_small)
                            .color(t.c.fg.alt),
                    );
            }
            let toggle = id.clone();
            let dd = d.clone();
            let mut out = col().w_full().child(head.on_press(move |a: &mut App| {
                let p = a.screens.decision.page(&dd);
                if !p.earlier.remove(&toggle) {
                    p.earlier.insert(toggle.clone());
                }
            }));
            if is_open && let Some(v) = rec {
                for a in &v.decision.annotations {
                    let quote = a
                        .range
                        .as_ref()
                        .and_then(|r| {
                            Some(
                                v.text[byte_of(&v.text, r.start as usize)?
                                    ..byte_of(&v.text, r.end as usize)?]
                                    .to_string(),
                            )
                        })
                        .unwrap_or_else(|| "on the whole".into());
                    out = out.child(
                        col()
                            .w_full()
                            .pl(18.0)
                            .mt(8.0)
                            .child(text(quote).font(t.ty.ui_small).color(t.c.fg.muted).wrap())
                            .child(
                                text(a.text.clone())
                                    .font(t.ty.ui)
                                    .color(t.c.fg.default)
                                    .wrap()
                                    .mt(2.0),
                            ),
                    );
                }
            }
            out
        })
        .collect()
}

fn marked(t: Theme, m: El, s: &str) -> Part {
    Part {
        el: row()
            .center_y()
            .gap(4.0)
            .child(m)
            .child(text(s.to_string()).font(t.ty.ui_small).color(t.c.fg.alt)),
    }
}

fn then_mark(t: Theme, m: ThenMark) -> El {
    match m {
        ThenMark::Waiting => icon(&carbon::HOURGLASS, 11.0).color(t.c.fg.muted),
        ThenMark::Blocked => icon(&carbon::HOURGLASS, 11.0).color(t.c.fg.default),
        ThenMark::Running => mark(t, Mark::Running),
        ThenMark::Sleeping => icon(&carbon::TIME, 11.0).color(t.c.state.stale),
        ThenMark::Done => mark(t, Mark::Ok),
        ThenMark::Failed => mark(t, Mark::Failed),
        ThenMark::Aborted => icon(&carbon::SUBTRACT, 11.0).color(t.c.fg.muted),
    }
}

/// The facts line's parts by the decision's standing.
fn fact_parts(
    t: Theme,
    app: &App,
    v: &DecisionView,
    st: &Standing,
    th: Option<&Then>,
) -> Vec<Part> {
    let mut parts = Vec::new();
    match st {
        Standing::Open { requested } => {
            parts.push(Part::text(t, "open"));
            parts.push(Part::text(
                t,
                &format!("requested {}", time::clock(*requested)),
            ));
            let ago = time::span(time::between(*requested, now(app)));
            parts.push(Part::text(t, &format!("{ago} ago")));
        }
        Standing::Answered { choice, by, at } => {
            parts.push(Part::text(t, &format!("answered {choice}")));
            let mut w = String::new();
            if let Some(by) = by {
                w.push_str(&format!("by {by}"));
            }
            if let Some(at) = at {
                w.push_str(&format!(" {}", time::clock(*at)));
            }
            if !w.is_empty() {
                parts.push(Part::text(t, w.trim()));
            }
        }
        Standing::Superseded { by } => match by {
            Some(n) => parts.push(Part {
                el: row()
                    .center_y()
                    .gap(4.0)
                    .child(text("superseded by").font(t.ty.ui_small).color(t.c.fg.alt))
                    .child(
                        text(n.clone())
                            .font(t.ty.mono_small)
                            .color(t.c.kind.literal),
                    ),
            }),
            None => parts.push(Part::text(t, "superseded")),
        },
        Standing::Withdrawn => parts.push(Part::text(t, "withdrawn")),
        Standing::Unknown { sent } => {
            parts.push(marked(
                t,
                mark(t, Mark::Unknown),
                &format!("answer sent {}", time::clock(*sent)),
            ));
            parts.push(Part::text(t, "no reply"));
        }
    }
    if matches!(st, Standing::Answered { .. })
        && let (Some(task), Some(th)) = (v.decision.then.as_ref(), th)
    {
        parts.push(Part {
            el: row()
                .center_y()
                .gap(4.0)
                .child(text("then").font(t.ty.ui_small).color(t.c.fg.alt))
                .child(
                    text(task.task.clone())
                        .font(t.ty.mono_small)
                        .color(t.c.kind.literal),
                ),
        });
        parts.push(match th.mark {
            Some(m) => marked(t, then_mark(t, m), &th.word),
            None => Part::text(t, &th.word),
        });
    }
    if app.link.status().phase != tau_client::LinkPhase::Live
        && let Some(at) = app.link.server().inbox.at()
    {
        parts.push(Part::text(t, &format!("as of {}", time::clock(at))));
    }
    parts
}

fn title_block(t: Theme, app: &App, v: &DecisionView, st: &Standing, th: Option<&Then>) -> El {
    let tb = t.l.title_block;
    let details = row()
        .center_y()
        .gap(4.0)
        .child(text("details").font(t.ty.ui_small).color(t.c.fg.muted))
        .child(icon(&carbon::CHEVRON_RIGHT, 12.0).color(t.c.fg.muted))
        .on_press(|a: &mut App| a.details_visible = !a.details_visible);
    col()
        .w_full()
        .pt(tb.top)
        .pb(tb.after)
        .child(line(t, &v.decision.title, t.ty.title_page, t.c.state.attn))
        .child(
            row()
                .center_y()
                .w_full()
                .mt(tb.facts_gap)
                .h(tb.facts)
                .child(
                    facts(t, fact_parts(t, app, v, st, th))
                        .grow(1.0)
                        .shrink()
                        .min_w(0.0),
                )
                .child(details),
        )
}

/// `The work after this answer failed`, on `tint.row_fail`, over the task kind and its error.
fn failed_block(t: Theme, v: &DecisionView, th: &Then) -> El {
    let task = v
        .decision
        .then
        .as_ref()
        .map_or(String::new(), |x| x.task.clone());
    col()
        .w_full()
        .mb(24.0)
        .child(
            row()
                .center_y()
                .gap(10.0)
                .h(30.0)
                .px(10.0)
                .radius(t.l.radius.pill)
                .fill(t.c.tint.row_fail)
                .child(mark(t, Mark::Failed))
                .child(
                    text("The work after this answer failed")
                        .font(t.ty.ui_strong)
                        .color(t.c.fg.default),
                ),
        )
        .child(
            text(format!("{task} · {}", th.error.clone().unwrap_or_default()))
                .font(t.ty.mono_small)
                .color(t.c.fg.alt)
                .wrap()
                .pl(10.0)
                .mt(6.0),
        )
}

/// The choices in the core's order: number, string, `●` and `chosen by phone · 14:10` once
/// answered; the cursor's band and bar on the row it is on.
fn choices(
    t: Theme,
    v: &DecisionView,
    st: &Standing,
    cursor: Option<Cursor>,
    keys: bool,
    column_w: f32,
) -> El {
    let r = &v.decision;
    let numbered = r.choices.len() <= 9;
    let mut out = col()
        .w_full()
        .mt(26.0)
        .child(group_head(t, "Choices", None))
        .child(el().h(4.0));
    for (i, c) in r.choices.iter().enumerate() {
        let chosen = matches!(st, Standing::Answered { choice, .. } if choice == c);
        let mut line_el = row().center_y().w_full().min_h(28.0);
        if cursor == Some(Cursor::Choice(i)) {
            line_el = line_el
                .child(
                    el().absolute(-40.0, 0.0)
                        .size(column_w + 40.0, 28.0)
                        .fill(t.c.surface.cursor_read)
                        .key("decision-cursor"),
                )
                .child(el().absolute(-40.0, 0.0).size(2.0, 28.0).fill(if keys {
                    t.c.state.attn
                } else {
                    t.c.fg.muted
                }));
        }
        if chosen {
            line_el = line_el.child(
                text("●")
                    .font(t.ty.ui_small)
                    .color(t.c.fg.default)
                    .absolute(-44.0, 5.0),
            );
        }
        if numbered {
            line_el = line_el.child(
                text((i + 1).to_string())
                    .font(t.ty.mono_small)
                    .color(t.c.fg.alt)
                    .absolute(-26.0, 6.0),
            );
        }
        line_el = line_el.child(
            text(c.clone())
                .font(t.ty.ui)
                .color(t.c.fg.default)
                .wrap()
                .grow(1.0)
                .shrink()
                .min_w(0.0),
        );
        if chosen && let Standing::Answered { by, at, .. } = st {
            let mut w = "chosen".to_string();
            if let Some(by) = by {
                w.push_str(&format!(" by {by}"));
            }
            if let Some(at) = at {
                w.push_str(&format!(" · {}", time::clock(*at)));
            }
            line_el = line_el.child(text(w).font(t.ty.ui_small).color(t.c.fg.alt));
        }
        out = out.child(line_el.key(format!("choice-{i}")));
    }
    out
}

/// The view's own band (`spec.md` §5.6 "Liveness"): superseded while shown, or left the inbox.
fn band(
    t: Theme,
    app: &App,
    d: &DecisionRef,
    st: &Standing,
    live: Option<&Liveness>,
) -> Option<El> {
    let time_mark = icon(&carbon::TIME, 14.0).color(t.c.state.stale);
    if let (Standing::Superseded { by: Some(n) }, Some(Liveness::Superseded { .. })) = (st, live) {
        return Some(notice(
            t,
            app.scope(),
            Some(time_mark),
            &format!("Superseded by {n} · this request can no longer be answered"),
            None,
            false,
        ));
    }
    if !app.screens.decision.left.contains(d) {
        return None;
    }
    let words = match st {
        Standing::Answered { choice, by, at } => {
            let mut w = format!("Answered {choice}");
            if let Some(by) = by {
                w.push_str(&format!(" by {by}"));
            }
            if let Some(at) = at {
                w.push_str(&format!(" at {}", time::clock(*at)));
            }
            w
        }
        Standing::Withdrawn => "Withdrawn".into(),
        _ => return None,
    };
    Some(notice(t, app.scope(), Some(time_mark), &words, None, false))
}

/// The details panel: About.
pub fn details(t: Theme, app: &App) -> El {
    let p = t.l.details.padding;
    let mut out = col()
        .w_full()
        .h_full()
        .fill(t.c.surface.read)
        .padding(p)
        .pt(28.0);
    let Some(d) = shown(app) else { return out };
    let Some(v) = record(app, &d) else { return out };
    let r = &v.decision;
    let item = |label: &str, value: El| {
        col()
            .w_full()
            .mt(8.0)
            .child(
                text(label.to_string())
                    .font(t.ty.ui_small)
                    .color(t.c.kind.you),
            )
            .child(value)
    };
    let plain = |s: String| text(s).font(t.ty.ui_small).color(t.c.fg.default).wrap();
    out = out
        .child(group_head(t, "About", None))
        .child(item(
            "extension",
            plain(d.id.subject().extension().to_string()),
        ))
        .child(item(
            "conversation",
            text(format!("#{}", r.conversation))
                .font(t.ty.mono_small)
                .color(t.c.kind.literal),
        ))
        .child(item(
            "basis",
            plain(format!(
                "entry {} · {} characters",
                r.basis,
                super::readback::thousands(v.text.chars().count())
            )),
        ));
    if let Some(th) = &r.then {
        out = out.child(item(
            "then",
            text(th.task.clone())
                .font(t.ty.mono_small)
                .color(t.c.kind.literal),
        ));
    }
    out
}
