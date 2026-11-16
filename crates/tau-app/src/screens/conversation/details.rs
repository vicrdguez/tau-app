//! The conversation's details panel (`spec.md` §4.5): Settings (the model in full, the thinking
//! level, the tools selection, `cwd`, and the workflow that may set the model again), the open
//! decisions this conversation asked, and the newest events that name it. Each label sits over
//! its value, so a model ID never breaks.

use rune::{El, TextStyle, col, row, text};
use serde_json::Value;
use tau_client::Wall;
use tau_client::shapes::Selection;

use super::floats::workflow_note;
use super::state::{Shown, shown};
use crate::app::App;
use crate::content::line;
use crate::theme::Theme;
use crate::time;
use crate::ui::{Mark, mark};

/// `tokens.json` `layout.details_events_max`.
const EVENTS: usize = 5;
/// `tokens.json` `layout.gaps.details_label_value`.
const LABEL_GAP: f32 = 2.0;

pub fn view(t: Theme, app: &App) -> El {
    let base = col()
        .w_full()
        .h_full()
        .fill(t.c.surface.read)
        .pt(28.0)
        .px(t.l.details.padding);
    let Some(s) = shown(app) else {
        return base;
    };
    let mut c = base.child(settings(t, app, &s));
    let decisions = decisions(t, app, &s);
    if !decisions.is_empty() {
        c = c.child(group(t, "Decisions", decisions).mt(20.0));
    }
    let events = events(t, app, &s);
    if !events.is_empty() {
        c = c.child(group(t, "Events", events).mt(20.0));
    }
    c
}

fn group(t: Theme, head: &str, rows: Vec<El>) -> El {
    col()
        .w_full()
        .child(
            row()
                .h(24.0)
                .center_y()
                .child(text(head.to_string()).font(t.ty.group).color(t.c.kind.you)),
        )
        .children(rows)
}

/// A label in steel over its value.
fn field(t: Theme, label: &str, value: El) -> El {
    col()
        .w_full()
        .mb(8.0)
        .gap(LABEL_GAP)
        .child(
            text(label.to_string())
                .font(t.ty.ui_small)
                .color(t.c.kind.you),
        )
        .child(value)
}

fn value(t: Theme, s: &str) -> El {
    text(s.to_string())
        .font(t.ty.ui_small)
        .color(t.c.fg.default)
        .wrap()
        .w_full()
}

fn settings(t: Theme, app: &App, s: &Shown) -> El {
    let Some(agent) = s.docs.map(|d| &d.agent) else {
        return group(
            t,
            "Settings",
            vec![value(t, crate::screens::NOTHING_RECEIVED)],
        );
    };
    let mut rows = Vec::new();
    let model = match &agent.model {
        Some(id) => {
            let name = app
                .link
                .server()
                .models
                .value()
                .and_then(|ms| ms.iter().find(|m| m.model == *id))
                .map(|m| m.name.clone());
            let mut v = row().flex_wrap().w_full().child(
                text(match &name {
                    Some(_) => format!("{id} ·"),
                    None => id.clone(),
                })
                .font(TextStyle {
                    size: t.ty.ui_small.size,
                    line_height: t.ty.ui_small.line_height,
                    ..t.ty.mono_small
                })
                .color(t.c.fg.default)
                .mr(4.0),
            );
            if let Some(n) = name {
                v = v.child(text(n).font(t.ty.ui_small).color(t.c.fg.default));
            }
            v
        }
        None => value(t, "not set"),
    };
    rows.push(field(t, "model", model));
    rows.push(field(
        t,
        "thinking",
        value(
            t,
            agent.thinking_level.as_deref().unwrap_or("model default"),
        ),
    ));
    rows.push(field(t, "tools", value(t, &tools(&agent.extensions))));
    if let Some(cwd) = &agent.cwd {
        rows.push(field(
            t,
            "cwd",
            text(leaf(cwd)).font(t.ty.mono_small).color(t.c.fg.default),
        ));
    }
    if let Some(note) = workflow_note(app, &s.conv) {
        rows.push(
            text(note)
                .font(t.ty.ui_small)
                .color(t.c.fg.muted)
                .wrap()
                .w_full()
                .mt(4.0),
        );
    }
    group(t, "Settings", rows)
}

/// `pi.agent.extensions` in words: absent `project default`, a change `project default + w1`, a
/// list `only coding-tools, w1`.
fn tools(sel: &Selection) -> String {
    match sel {
        Selection::ProjectDefault => "project default".into(),
        Selection::Only(names) => format!("only {}", names.join(", ")),
        Selection::Change { add, remove } => {
            let mut words = "project default".to_string();
            if !add.is_empty() {
                words.push_str(&format!(" + {}", add.join(", ")));
            }
            if !remove.is_empty() {
                words.push_str(&format!(" − {}", remove.join(", ")));
            }
            words
        }
    }
}

/// A path as `…/leaf`.
fn leaf(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rsplit_once('/') {
        Some((_, leaf)) if !leaf.is_empty() => format!("…/{leaf}"),
        _ => trimmed.to_string(),
    }
}

/// Open decisions whose `conversation` is this one: `◆ title #n`.
fn decisions(t: Theme, app: &App, s: &Shown) -> Vec<El> {
    let Some(inbox) = app.link.server().inbox.value() else {
        return Vec::new();
    };
    inbox
        .projects
        .get(&s.conv.project)
        .into_iter()
        .flat_map(|subjects| subjects.values())
        .filter(|d| d.conversation == s.conv.id)
        .map(|d| {
            let mut r = row()
                .w_full()
                .min_h(22.0)
                .center_y()
                .gap(8.0)
                .child(mark(t, Mark::Decision))
                .child(
                    line(t, &d.title, t.ty.ui_small, t.c.fg.default)
                        .grow(1.0)
                        .shrink(),
                );
            if let Some(n) = d.id.request_number() {
                r = r.child(
                    text(format!("#{n}"))
                        .font(t.ty.mono_small)
                        .color(t.c.kind.literal),
                );
            }
            r
        })
        .collect()
}

/// The newest events of the project's log that name this conversation: time and text.
fn events(t: Theme, app: &App, s: &Shown) -> Vec<El> {
    let Some(log) = app.link.server().timelines.get(&s.conv.project) else {
        return Vec::new();
    };
    let mut named: Vec<(i64, String)> = log
        .entries
        .values()
        .filter_map(|e| {
            let data = e.raw.get("data")?;
            (data.get("conversation").and_then(Value::as_u64) == Some(s.conv.id.0)).then(|| {
                (
                    data.get("at").and_then(Value::as_i64).unwrap_or_default(),
                    data.get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                )
            })
        })
        .collect();
    named.sort_by_key(|e| std::cmp::Reverse(e.0));
    named
        .into_iter()
        .take(EVENTS)
        .map(|(at, words)| {
            row()
                .w_full()
                .min_h(22.0)
                .center_y()
                .gap(8.0)
                .child(
                    text(time::clock(Wall(at)))
                        .font(t.ty.mono_small)
                        .color(t.c.fg.muted)
                        .w(36.0),
                )
                .child(
                    line(t, &words, t.ty.ui_small, t.c.fg.default)
                        .grow(1.0)
                        .shrink(),
                )
        })
        .collect()
}
