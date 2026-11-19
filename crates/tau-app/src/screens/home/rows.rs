//! List row layout, fixed slots, and the responsive slot-dropping rule.
use super::list::{ListRow, RowId, open};
use crate::app::{App, Focus};
use crate::content;
use crate::frame::SidebarPlace;
use crate::theme::Theme;
use crate::ui::{self, Mark};
use rune::{El, col, el, row, text};
use tau_client::Wall;

pub fn stage_width(app: &App) -> f32 {
    app.window[0]
        - if app.sidebar_place() == Some(SidebarPlace::Beside) {
            app.theme().l.sidebar.width
        } else {
            0.0
        }
}

pub fn width(app: &App) -> f32 {
    let stage = stage_width(app);
    stage - 80.0 - if stage >= 1000.0 { 440.0 } else { 0.0 }
}

/// The optional slots of Failed and Running rows. `detail` is the 132 px column Failed fills
/// with its reason and Running with its model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slots {
    pub extension: bool,
    pub detail: bool,
    pub time: bool,
}
impl Slots {
    /// One decision for every group present, so their columns stand on the same lines. Slots drop
    /// whole until the label keeps 200 px: time, model, extension, reason. While Failed is
    /// present its reason holds the detail column, so a model alone doesn't drop it.
    pub fn home(width: f32, failed: bool, running: bool) -> Self {
        let mut s = Self {
            extension: true,
            detail: failed || running,
            time: failed,
        };
        let label = |s: Self| {
            width
                - 14.0
                - 96.0
                - 26.0
                - 36.0
                - if s.extension { 42.0 } else { 0.0 }
                - if s.detail { 144.0 } else { 0.0 }
                - if s.time { 96.0 } else { 0.0 }
        };
        if label(s) < 200.0 {
            s.time = false;
        }
        if label(s) < 200.0 && !failed {
            s.detail = false;
        }
        if label(s) < 200.0 {
            s.extension = false;
        }
        if label(s) < 200.0 {
            s.detail = false;
        }
        s
    }
    pub fn running(width: f32) -> Self {
        Self::home(width, false, true)
    }
}

/// The columns one list draws its rows on, decided once from every row it shows.
pub struct Grid {
    pub home: bool,
    pub slots: Slots,
    /// The width of the event time column: the widest time drawn.
    pub stamp: f32,
    /// The width of Faults' `after <decision ID>` column, the widest drawn; none when it doesn't
    /// fit, so it shows whole or not at all.
    pub after: Option<f32>,
}

impl Grid {
    pub fn home(app: &App, rows: &[ListRow]) -> Self {
        let now = super::now(app);
        let stamp = rows
            .iter()
            .filter(|r| matches!(r.id, RowId::Event(..)))
            .filter_map(|r| r.at)
            .map(|at| mono_width(app.theme().ty.mono_small, &stamp(at, now)))
            .fold(0.0, f32::max);
        let has = |group: &str| rows.iter().any(|r| r.group == group);
        Self {
            home: true,
            slots: Slots::home(width(app), has("Failed"), has("Running")),
            stamp,
            after: None,
        }
    }
    /// A place's grid. Faults rows keep their error at least 200 px by dropping the time, then
    /// the `after` column.
    pub fn place(app: &App, rows: &[ListRow]) -> Self {
        let ty = app.theme().ty;
        let width = width(app);
        let widest =
            |w: &dyn Fn(&ListRow) -> Option<f32>| rows.iter().filter_map(w).fold(0.0, f32::max);
        let kind =
            widest(&|r| matches!(r.id, RowId::Fault(..)).then(|| mono_width(ty.mono, &r.label)));
        let after = widest(&|r| {
            r.after
                .as_ref()
                .map(|d| mono_width(ty.mono_small, &format!("after {d}")))
        });
        let mut after = (after > 0.0).then_some(after);
        let mut time = true;
        let error = |after: Option<f32>, time: bool| {
            width
                - 14.0
                - 12.0
                - kind
                - 12.0
                - 96.0
                - 12.0
                - 26.0
                - 12.0
                - after.map_or(0.0, |a| a + 12.0)
                - if time { 96.0 } else { 0.0 }
        };
        if error(after, time) < 200.0 {
            time = false;
        }
        if error(after, time) < 200.0 {
            after = None;
        }
        Self {
            home: false,
            slots: Slots {
                time,
                ..Slots::running(width)
            },
            stamp: 94.0,
            after,
        }
    }
}

/// Where a row's label starts inside the row: its 12 px padding, the mark's slot, the gap.
pub fn label_inset() -> f32 {
    12.0 + crate::theme::tokens().marks.slot + 12.0
}

/// The drawn width of `s` in a mono style: Iosevka advances half an em per character.
fn mono_width(font: rune::TextStyle, s: &str) -> f32 {
    s.chars().count() as f32 * font.size / 2.0
}

pub fn active(app: &App) -> bool {
    app.focus == Focus::Stage
        && (app.floats.is_empty() || app.scope().ctx == crate::keys::Ctx::Peek)
}

pub fn line(t: Theme, s: &str, color: rune::Color) -> El {
    content::line(t, s, t.ty.ui, color)
}
fn small(t: Theme, s: &str, color: rune::Color, w: f32) -> El {
    content::line(t, s, t.ty.mono_small, color).w(w).min_w(w)
}

fn stamp(at: i64, now: Wall) -> String {
    crate::time::stamp(Wall(at), now)
}

pub fn row_view(t: Theme, app: &App, r: &ListRow, selected: bool, grid: &Grid) -> El {
    let home = grid.home;
    let keys = active(app);
    let mut slots = Vec::new();
    let lead = if r.hidden {
        text("−").font(t.ty.ui).color(t.c.fg.muted).w(14.0)
    } else {
        r.mark
            .map(|m| ui::mark(t, m))
            .unwrap_or_else(|| el().w(14.0))
    };
    let mut label = line(
        t,
        &r.label,
        if r.hidden {
            t.c.fg.muted
        } else {
            t.c.fg.default
        },
    );
    let body = match &r.id {
        RowId::Decision(d) => {
            let solid = selected && keys;
            let fg = if solid {
                t.c.surface.read
            } else {
                t.c.fg.default
            };
            let n = format!("#{}", d.id.request_number().unwrap_or(1));
            let head = row()
                .center_y()
                .gap(10.0)
                .h(26.0)
                .px(12.0)
                .child(ui::mark_in(
                    Mark::Decision,
                    if solid { fg } else { t.c.state.attn },
                ))
                .child(
                    content::line(t, &r.label, t.ty.move_, fg)
                        .shrink()
                        .min_w(0.0),
                )
                .child(text(n).font(t.ty.mono_small).color(if solid {
                    fg
                } else {
                    t.c.kind.literal
                }))
                .child(rune::spacer())
                .child(
                    row().w(84.0).min_w(84.0).justify_end().child(content::line(
                        t,
                        &r.at
                            .map(|at| {
                                crate::time::span(crate::time::between(Wall(at), super::now(app)))
                            })
                            .unwrap_or_default(),
                        t.ty.mono_small,
                        if solid { fg } else { t.c.fg.alt },
                    )),
                );
            // Every head keeps the chip's width, drawn or not, so the ages line up.
            let chip = ui::chip(t, app.scope(), "<CR>", false);
            let head = head.child(if solid {
                chip
            } else {
                chip.opacity(0.0).a11y_hidden()
            });
            let head = if solid {
                head.fill(t.c.state.attn)
            } else {
                head
            };
            col()
                .w_full()
                .child(head)
                .child(
                    row()
                        .center_y()
                        .h(22.0)
                        .pl(36.0)
                        .child(line(t, &r.project, t.c.kind.worker))
                        .child(text(" · ").font(t.ty.ui_small).color(t.c.fg.muted))
                        .child(content::line(
                            t,
                            d.id.subject().as_str(),
                            t.ty.mono_small,
                            t.c.fg.alt,
                        )),
                )
                .mb(6.0)
        }
        RowId::Event(..) => {
            let new = super::is_new(app, r);
            label = content::line(
                t,
                &r.label,
                if new { t.ty.ui_strong } else { t.ty.ui },
                t.c.fg.default,
            );
            let date =
                r.at.map(|at| stamp(at, super::now(app)))
                    .unwrap_or_default();
            let lead = small(t, &date, t.c.fg.muted, grid.stamp);
            let number = r
                .conversation
                .map(|id| format!("#{id}"))
                .unwrap_or_default();
            if home {
                row()
                    .center_y()
                    .gap(12.0)
                    .w_full()
                    .child(lead)
                    .child(label.shrink().min_w(0.0))
                    .child(line(t, &r.project, t.c.kind.worker))
                    .child(small(t, &number, t.c.kind.literal, 26.0))
                    .h(28.0)
                    .px(12.0)
            } else {
                slots.push(small(t, &r.extension, t.c.fg.muted, 86.0));
                slots.push(line(t, &r.project, t.c.kind.worker).w(96.0));
                slots.push(small(t, &number, t.c.kind.literal, 26.0));
                ui::slot_row(Some(lead), label, slots, 12.0)
                    .h(28.0)
                    .px(12.0)
            }
        }
        RowId::MoreRuns | RowId::MoreFaults | RowId::MoreEvents => {
            let inset = if r.id == RowId::MoreEvents {
                12.0 + grid.stamp + 12.0
            } else {
                label_inset()
            };
            row()
                .center_y()
                .h(28.0)
                .pl(inset)
                .child(line(t, &r.label, t.c.fg.muted).shrink().min_w(0.0))
        }
        RowId::Fault(p, task) if !home => {
            let error = app
                .link
                .server()
                .faults
                .get(p)
                .and_then(|k| k.value())
                .and_then(|fs| fs.iter().find(|f| f.task == *task))
                .map(|f| f.error.as_str())
                .unwrap_or_default();
            let lead = row().center_y().gap(12.0).child(lead).child(
                content::line(t, &r.label, t.ty.mono, t.c.kind.literal)
                    .min_w(mono_width(t.ty.mono, &r.label)),
            );
            slots.push(line(t, &r.project, t.c.kind.worker).w(96.0));
            slots.push(small(
                t,
                &format!("#{}", r.conversation.unwrap_or_default()),
                t.c.kind.literal,
                26.0,
            ));
            if let Some(w) = grid.after {
                let after = r.after.as_ref().map(|d| format!("after {d}"));
                slots.push(small(t, &after.unwrap_or_default(), t.c.fg.alt, w));
            }
            if grid.slots.time {
                slots.push(row().w(84.0).min_w(84.0).justify_end().child(content::line(
                    t,
                    &r.at.map(|a| stamp(a, super::now(app))).unwrap_or_default(),
                    t.ty.mono_small,
                    t.c.fg.muted,
                )));
            }
            let error = content::line(t, error, t.ty.ui, t.c.fg.alt);
            ui::slot_row(Some(lead), error, slots, 12.0)
                .h(28.0)
                .px(12.0)
        }
        _ => {
            let slots_on = grid.slots;
            if matches!(r.id, RowId::Fault(..)) {
                label = content::line(t, &r.label, t.ty.mono, t.c.kind.literal);
                if let Some(after) = &r.after {
                    // A whole identity suffix gets its own nonshrinking slot; the kind cuts first.
                    let suffix = format!("after {after}");
                    if width(app) >= 500.0 {
                        label = row()
                            .center_y()
                            .gap(10.0)
                            .child(label.shrink().min_w(0.0))
                            .child(
                                content::line(t, &suffix, t.ty.mono_small, t.c.fg.alt)
                                    .min_w(suffix.chars().count() as f32 * 6.0),
                            );
                    }
                }
            }
            if home || matches!(r.id, RowId::Fault(..)) {
                slots.push(line(t, &r.project, t.c.kind.worker).w(96.0));
                slots.push(small(
                    t,
                    &r.conversation
                        .map(|id| format!("#{id}"))
                        .unwrap_or_default(),
                    t.c.kind.literal,
                    26.0,
                ));
            }
            if slots_on.extension {
                slots.push(small(t, &r.extension, t.c.fg.muted, 30.0));
            }
            if slots_on.detail {
                let detail = if home && r.mark != Some(Mark::Running) {
                    &r.reason
                } else {
                    &r.model
                };
                slots.push(line(t, detail, t.c.fg.alt).w(132.0));
            }
            if slots_on.time && home {
                slots.push(row().w(84.0).justify_end().child(content::line(
                    t,
                    &r.at.map(|a| stamp(a, super::now(app))).unwrap_or_default(),
                    t.ty.mono_small,
                    t.c.fg.muted,
                )));
            }
            if !home && matches!(r.id, RowId::Conversation(..)) {
                slots.push(small(
                    t,
                    &r.conversation
                        .map(|id| format!("#{id}"))
                        .unwrap_or_default(),
                    t.c.kind.literal,
                    30.0,
                ));
            }
            ui::slot_row(Some(lead), label, slots, 12.0)
                .h(28.0)
                .px(12.0)
        }
    };
    let body = if selected {
        let b = body.fill(t.c.surface.cursor_read);
        if keys && !matches!(r.id, RowId::Decision(_)) {
            b.border_sides([0.0, 0.0, 0.0, 2.0], t.c.state.attn)
        } else {
            b
        }
    } else {
        body
    };
    let id = r.id.clone();
    let click = id.clone();
    let double = id.clone();
    let body = body
        .key(format!("list-{id:?}"))
        .on_press(move |a: &mut App| {
            if a.floats.is_empty() {
                super::select(a, click.clone());
            }
        })
        .on_double_press(move |a: &mut App| {
            if a.floats.is_empty() {
                open(a, &double, false);
            }
        });
    if let RowId::Conversation(conv) = id {
        body.on_layout(move |a: &mut App, rect| {
            if rect[1] + rect[3] > 44.0 && rect[1] < a.window[1] - 26.0 {
                a.link.want_label(conv.clone());
            }
        })
    } else {
        body
    }
}
