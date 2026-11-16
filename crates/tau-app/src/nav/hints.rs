//! Hint mode (`gs`, `grammar.md` §13): a home-row label on every target the window shows in the
//! region that has the keys. A label moves that region's cursor and records a jump; it never
//! opens or acts. `Esc` cancels.
//!
//! The targets are the page's (`Screen::targets`) or the sidebar's rows, taken when `gs` is
//! pressed. Which of them the window shows is known only when the frame lays out, so the labels
//! are dealt on the first frame drawn in hint mode and kept from then on: a label typed before
//! that frame picks nothing (`grammar.md` §7).

use std::sync::atomic::{AtomicU64, Ordering};

use rune::{Cx, El, Tx, el, row, text};

use crate::app::{App, Focus, Handler, Invocation};
use crate::frame::Row;
use crate::keys::{Code, Key};
use crate::screens::{self, HintTarget};
use crate::theme::Theme;

pub struct Hints {
    /// Keys the overlay, so its first layout is this session's.
    session: u64,
    /// The sidebar's rows rather than the page's targets.
    sidebar: bool,
    targets: Vec<HintTarget>,
    /// Each label as first drawn, with its target's index.
    drawn: Option<Vec<(String, usize)>>,
    typed: String,
}

pub fn handlers() -> Vec<(&'static str, Handler)> {
    vec![("hint.pane", start), ("hint.select", select)]
}

/// `n` labels from `alphabet`: one letter each up to its length, else two letters each, so no
/// label starts another.
pub fn labels(n: usize, alphabet: &str) -> Vec<String> {
    let a: Vec<char> = alphabet.chars().collect();
    if n <= a.len() {
        return a.iter().take(n).map(char::to_string).collect();
    }
    a.iter()
        .flat_map(|x| a.iter().map(move |y| format!("{x}{y}")))
        .take(n)
        .collect()
}

fn start(app: &mut App, inv: &Invocation, _: &mut Tx) {
    if !inv.applies {
        return;
    }
    let sidebar = app.focus == Focus::Sidebar;
    let targets = if sidebar {
        sidebar_targets(app)
    } else {
        screens::page(app).targets(app)
    };
    if targets.is_empty() {
        app.say("nothing to label here");
        return;
    }
    static SESSIONS: AtomicU64 = AtomicU64::new(0);
    app.hints = Some(Hints {
        session: SESSIONS.fetch_add(1, Ordering::Relaxed),
        sidebar,
        targets,
        drawn: None,
        typed: String::new(),
    });
}

/// The sidebar's rows, top to bottom; each anchor is the row's place in that order.
fn sidebar_targets(app: &App) -> Vec<HintTarget> {
    let l = app.theme().l.sidebar;
    let (rows, _) = app.sidebar_rows();
    rows.iter()
        .enumerate()
        .map(|(i, r)| {
            // Open items have their own icon size and gap (`frame::sidebar`).
            let x = match r {
                Row::Item(_) => l.row_padding_x + l.item_icon + 8.0,
                _ => l.row_padding_x + l.row_icon + 10.0,
            };
            HintTarget {
                key: crate::frame::row_key(r),
                at: [x, (l.row - LABEL_H) / 2.0],
                within: "sidebar",
                anchor: i.into(),
            }
        })
        .collect()
}

/// A label key, `Esc` or `BS`.
fn select(app: &mut App, inv: &Invocation, _: &mut Tx) {
    let Some(key) = inv.keys.first().copied() else {
        return;
    };
    let Some(h) = app.hints.as_mut() else { return };
    if key == Key::plain(Code::Esc) {
        app.hints = None;
        return;
    }
    if key == Key::plain(Code::Backspace) {
        h.typed.pop();
        return;
    }
    let (Some(c), Some(drawn)) = (key.typed(), h.drawn.as_ref()) else {
        return;
    };
    let typed = format!("{}{c}", h.typed);
    if let Some(&(_, i)) = drawn.iter().find(|(l, _)| *l == typed) {
        pick(app, i);
    } else if drawn.iter().any(|(l, _)| l.starts_with(&typed)) {
        h.typed = typed;
    }
}

/// Moves the region's cursor to target `i` and leaves hint mode.
fn pick(app: &mut App, i: usize) {
    let Some(h) = app.hints.take() else { return };
    let anchor = h.targets[i].anchor.clone();
    if h.sidebar {
        let (rows, _) = app.sidebar_rows();
        if let Some(row) = anchor.as_u64().and_then(|n| rows.get(n as usize)) {
            app.sidebar_cursor = row.clone();
        }
        return;
    }
    let from = super::here(app);
    screens::page(app).land(app, &anchor);
    super::left(app, from);
}

/// A label's height: `mono.small` on a pill.
const LABEL_H: f32 = 16.0;

/// The labels over the window while hint mode is on. The first frame deals them to the targets
/// it shows inside the region; later frames keep that deal.
pub fn overlay(t: Theme, app: &App, cx: &Cx<App>) -> Option<El> {
    let h = app.hints.as_ref()?;
    let dealt = match &h.drawn {
        Some(d) => d.clone(),
        None => {
            let region = cx.window_layout(if h.sidebar { "sidebar" } else { "stage-body" })?;
            let shown: Vec<usize> = (0..h.targets.len())
                .filter(|&i| {
                    let tg = &h.targets[i];
                    let clip = cx.window_layout(tg.within).unwrap_or(region);
                    cx.window_layout(&tg.key).is_some_and(|r| {
                        let p = [r[0] + tg.at[0], r[1] + tg.at[1]];
                        inside(p, region) && inside(p, clip)
                    })
                })
                .collect();
            labels(shown.len(), app.keymap.hint_alphabet())
                .into_iter()
                .zip(shown)
                .collect()
        }
    };
    let char_w = t.ty.mono_small.size * 0.6;
    let mut placed: Vec<[f32; 4]> = Vec::new();
    let mut root = el()
        .key(format!("hints-{}", h.session))
        .absolute(0.0, 0.0)
        .size(app.window[0], app.window[1])
        .inert();
    for (label, i) in &dealt {
        let tg = &h.targets[*i];
        let Some(r) = cx.window_layout(&tg.key) else {
            continue;
        };
        let w = label.chars().count() as f32 * char_w + 6.0;
        let y = r[1] + tg.at[1];
        let mut x = r[0] + tg.at[0];
        // Shifted right past any label already placed where it would sit.
        while let Some(p) = placed
            .iter()
            .find(|p| x < p[0] + p[2] && p[0] < x + w && y < p[1] + p[3] && p[1] < y + LABEL_H)
        {
            x = p[0] + p[2] + 2.0;
        }
        placed.push([x, y, w, LABEL_H]);
        if !label.starts_with(&h.typed) {
            continue;
        }
        root = root.child(
            row()
                .center_y()
                .h(LABEL_H)
                .px(3.0)
                .fill(t.c.fg.default)
                .radius(t.l.radius.pill)
                .absolute(x, y)
                .child(
                    text(label.clone())
                        .font(t.ty.mono_small)
                        .color(t.c.surface.read),
                ),
        );
    }
    if h.drawn.is_none() {
        let session = h.session;
        root = root.on_layout(move |a: &mut App, _| deal(a, session, dealt.clone()));
    }
    Some(root)
}

fn inside(p: [f32; 2], r: [f32; 4]) -> bool {
    p[0] >= r[0] && p[0] < r[0] + r[2] && p[1] >= r[1] && p[1] + LABEL_H <= r[1] + r[3]
}

/// Keeps the labels the first frame of `session` drew.
fn deal(app: &mut App, session: u64, dealt: Vec<(String, usize)>) {
    if let Some(h) = app.hints.as_mut()
        && h.session == session
        && h.drawn.is_none()
    {
        h.drawn = Some(dealt);
    }
}

/// A frame on which every target shows: deals the labels if no frame has, and returns each with
/// its target's anchor.
#[cfg(test)]
pub(crate) fn draw(app: &mut App) -> Vec<(String, screens::Anchor)> {
    let Some(h) = app.hints.as_mut() else {
        return Vec::new();
    };
    let n = h.targets.len();
    let dealt = h.drawn.get_or_insert_with(|| {
        labels(n, app.keymap.hint_alphabet())
            .into_iter()
            .zip(0..n)
            .collect()
    });
    dealt
        .iter()
        .map(|(l, i)| (l.clone(), h.targets[*i].anchor.clone()))
        .collect()
}
