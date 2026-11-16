//! One delayed preview target. The peek borrows the client cache and owns only Slot::Peek.
#[cfg(test)]
mod tests;
use super::home::list::{self, RowId};
use super::{Screen, ScreenFloat, ScreenId, home, places};
use crate::app::{App, Focus, Handler, Invocation, Stage};
use crate::content;
use crate::floats::Float;
use crate::frame::{Place, Row};
use crate::keys::{Code, Ctx, Key, Mode};
use crate::open::Item;
use crate::theme::Theme;
use crate::ui::{self, Badge, Scope, Shell};
use rune::{El, Tx, col, row, text};
use tau_client::shapes::{Block, EntryBody};
use tau_client::{ConvRef, DecisionRef, LinkEvent, Slot, Wall};

#[derive(Default)]
pub struct State {
    pub shown: Option<RowId>,
    pub as_of: Option<Wall>,
    pending: Option<RowId>,
    since: f64,
}
pub struct Peek;
struct PeekFloat;
impl ScreenFloat for PeekFloat {
    fn owner(&self) -> ScreenId {
        ScreenId::Peek
    }
    fn scope(&self) -> Scope {
        Scope {
            ctx: Ctx::Peek,
            mode: Mode::Peek,
        }
    }
    fn badge(&self) -> Badge {
        Badge::Peek
    }
    fn view(&self, t: Theme, a: &App) -> El {
        ui::shell(
            t,
            Shell {
                tag: None,
                title: ui::title(t, "Peek"),
                subtitle: None,
                meta: None,
                body: body(t, a).px(24.0).max_h(280.0).scroll(rune::ScrollSpec {
                    stick_bottom: false,
                }),
                input: None,
                hints: vec![
                    ui::float_hint(t, self.scope(), &["<CR>"], "open"),
                    ui::float_hint(t, self.scope(), &["K", "<Esc>"], "closes"),
                ],
            },
        )
    }
}
fn from_item(item: &Item) -> RowId {
    match item {
        Item::Conversation { project, id } => {
            RowId::Conversation(ConvRef::new(project.clone(), *id))
        }
        Item::Decision { project, id } => {
            RowId::Decision(DecisionRef::new(project.clone(), id.clone()))
        }
        Item::Project(p) => RowId::Project(p.clone().into()),
    }
}
pub fn target(a: &App) -> Option<RowId> {
    if a.scope().ctx == Ctx::Peek && a.floats.last().is_some_and(|(_, f)| *f == Focus::Sidebar) {
        return match &a.sidebar_cursor {
            Row::Item(i) => Some(from_item(i)),
            _ => None,
        };
    }
    if list::stage_width(a) < 1000.0 && a.scope().ctx != Ctx::Peek {
        return None;
    }
    match &a.stage {
        Stage::Place(Place::Home) => home::selected(a).map(|r| r.id),
        Stage::Place(Place::Conversations | Place::Timeline | Place::Faults) => {
            places::selected(a).map(|r| r.id)
        }
        _ => None,
    }
}
fn request(a: &mut App, id: Option<RowId>) {
    a.link.show(Slot::Peek, None);
    if let Some(id) = &id {
        match id {
            RowId::Conversation(c) => {
                a.link.want_label(c.clone());
                a.link.show(Slot::Peek, Some(c.clone()));
            }
            RowId::Event(..) => {
                if let Some(Item::Conversation { project, id }) = list::item(id, a, false) {
                    a.link.show(Slot::Peek, Some(ConvRef::new(project, id)));
                }
            }
            RowId::Decision(d) => {
                if !a.link.server().decisions.contains_key(d) {
                    a.link.fetch_decision(d.clone());
                }
            }
            RowId::Fault(..) => {
                if let Some(Item::Decision { project, id }) = list::item(id, a, false) {
                    a.link.fetch_decision(DecisionRef::new(project, id));
                }
            }
            RowId::Project(p) => a.link.fetch_options(p.clone()),
            _ => {}
        }
    }
    a.screens.peek.shown = id;
}
pub fn settle_scene(a: &mut App) {
    let id = target(a);
    a.screens.peek.pending = id.clone();
    request(a, id);
    crate::app::link::settle(a);
}
fn open_float(a: &mut App, _: &Invocation, _: &mut Tx) {
    if a.focus != Focus::Sidebar
        && (list::stage_width(a) >= 1000.0 || !matches!(a.stage, Stage::Place(_)))
    {
        return;
    }
    if a.focus == Focus::Sidebar && !matches!(a.sidebar_cursor, Row::Item(_)) {
        return;
    }
    a.open_float(Float::Screen(Box::new(PeekFloat)));
    let target = target(a);
    a.screens.peek.pending = target.clone();
    request(a, target);
}
fn keys(a: &mut App, inv: &Invocation, _: &mut Tx) {
    let Some(&k) = inv.keys.first() else { return };
    if [Key::char('K'), Key::char('q'), Key::plain(Code::Esc)].contains(&k) {
        a.close_float();
        return;
    }
    if k == Key::plain(Code::Enter) {
        let id = target(a);
        a.close_float();
        if let Some(id) = id {
            list::open(a, &id, false);
        }
        return;
    }
    let sidebar = a.floats.last().is_some_and(|(_, f)| *f == Focus::Sidebar);
    if sidebar {
        let (rows, _) = a.sidebar_rows();
        let at = rows
            .iter()
            .position(|r| *r == a.sidebar_cursor)
            .unwrap_or(0);
        let to = if k == Key::char('j') {
            (at + 1).min(rows.len() - 1)
        } else {
            at.saturating_sub(1)
        };
        a.sidebar_cursor = rows[to].clone();
    } else {
        home::motion(a, &[k], None);
    }
    let id = target(a);
    request(a, id.clone());
    a.screens.peek.pending = id;
}
impl Screen for Peek {
    fn ctx(&self, _: &App) -> Ctx {
        Ctx::Peek
    }
    fn view(&self, t: Theme, a: &App) -> El {
        body(t, a).px(28.0).pt(28.0).h_full().clip_rounded(0.0)
    }
    fn globals(&self) -> Vec<(&'static str, Handler)> {
        vec![("peek", open_float)]
    }
    fn commands(&self) -> Vec<(&'static str, Handler)> {
        vec![("peek.keys", keys)]
    }
    fn poll(&self, a: &mut App, now: f64, tx: &mut Tx) -> bool {
        let id = target(a);
        if id != a.screens.peek.pending {
            a.screens.peek.pending = id.clone();
            a.screens.peek.since = now;
            a.link.show(Slot::Peek, None);
            if id.is_none() {
                a.screens.peek.shown = None;
                return true;
            }
        }
        if id != a.screens.peek.shown {
            let left = 0.300 - (now - a.screens.peek.since);
            if left > 0.000_001 {
                tx.wake_after(std::time::Duration::from_secs_f64(left));
                return false;
            }
            request(a, id);
            return true;
        }
        false
    }
    fn on_event(&self, a: &mut App, event: &LinkEvent) {
        let d = match event {
            LinkEvent::DecisionLeft { decision, .. } | LinkEvent::Superseded { decision, .. } => {
                Some(decision)
            }
            _ => None,
        };
        if let Some(d) = d
            && a.screens.peek.shown == Some(RowId::Decision(d.clone()))
        {
            a.link.fetch_decision(d.clone());
        }
    }
}
fn facts(t: Theme, words: &str) -> El {
    text(words.to_string())
        .w_full()
        .wrap()
        .font(t.ty.ui_small)
        .color(t.c.fg.alt)
        .mt(4.0)
}
fn heading(t: Theme, words: &str) -> El {
    content::line(t, words, t.ty.float_title, t.c.fg.default)
}
fn as_of(a: &App) -> String {
    if let Some(at) = a.screens.peek.as_of {
        return format!(" · as of {}", crate::time::clock(at));
    }
    if a.server.notice.is_some() || a.link.status().phase != tau_client::LinkPhase::Live {
        a.link
            .server()
            .inbox
            .at()
            .map(|at| format!(" · as of {}", crate::time::clock(at)))
            .unwrap_or_default()
    } else {
        String::new()
    }
}
fn body(t: Theme, a: &App) -> El {
    match &a.screens.peek.shown {
        Some(RowId::Decision(d)) => decision(t, a, d),
        Some(RowId::Conversation(c)) => conversation(t, a, c),
        Some(RowId::Event(p, id)) => {
            let Some(e) = a
                .link
                .server()
                .timelines
                .get(p)
                .and_then(|t| t.entries.get(id))
            else {
                return col();
            };
            let data = &e.raw["data"];
            let mut out = col()
                .w_full()
                .min_h(0.0)
                .child(heading(t, data["text"].as_str().unwrap_or("no text")))
                .child(facts(
                    t,
                    &format!(
                        "{} · {} · {}{}",
                        e.kind,
                        data["at"]
                            .as_i64()
                            .map(|at| crate::time::clock(Wall(at)))
                            .unwrap_or_default(),
                        p,
                        e.by_task
                            .map(|id| format!(" · by task {id}"))
                            .unwrap_or_default()
                    ),
                ));
            if let Some(id) = data["conversation"].as_u64() {
                out = out.child(conversation(t, a, &ConvRef::new(p.clone(), id)).mt(22.0));
            }
            out
        }
        Some(RowId::Fault(p, id)) => {
            let Some(f) = a
                .link
                .server()
                .faults
                .get(p)
                .and_then(|k| k.value())
                .and_then(|fs| fs.iter().find(|f| f.task == *id))
            else {
                return col();
            };
            let mut out = col()
                .w_full()
                .min_h(0.0)
                .child(heading(t, &f.kind))
                .child(facts(
                    t,
                    &format!(
                        "{p} · task {id} · #{} · {}{}",
                        f.conversation,
                        f.ended_at
                            .map(|at| crate::time::clock(Wall(at)))
                            .unwrap_or_default(),
                        as_of(a)
                    ),
                ))
                .child(
                    text(f.error.clone())
                        .wrap()
                        .font(t.ty.mono)
                        .color(t.c.fg.default)
                        .mt(12.0),
                );
            if let Some(id) = &f.decision {
                let d = DecisionRef::new(p.clone(), id.clone());
                if let Some(v) = a.link.server().decisions.get(&d).and_then(|k| k.value()) {
                    out = out
                        .child(heading(t, &v.decision.title).mt(22.0))
                        .child(facts(
                            t,
                            &format!(
                                "{} · by {} · {}",
                                v.decision.choice.as_deref().unwrap_or("not answered"),
                                v.decision
                                    .by
                                    .as_ref()
                                    .map(ToString::to_string)
                                    .unwrap_or_default(),
                                v.decision
                                    .at
                                    .map(|at| crate::time::clock(Wall(at)))
                                    .unwrap_or_default()
                            ),
                        ));
                }
                out=out.child(facts(t,"Open the decision to send the same answer again. It starts the work after it again.").mt(12.0));
            }
            out
        }
        Some(RowId::Project(p)) => {
            let Some(project) = a
                .link
                .server()
                .projects
                .value()
                .and_then(|ps| ps.iter().find(|v| v.project == *p))
            else {
                return col();
            };
            let mut out = col()
                .w_full()
                .child(heading(t, p.as_str()))
                .child(facts(t, &project.workdir));
            for ext in &project.extensions {
                let kind = a
                    .link
                    .server()
                    .extensions
                    .value()
                    .and_then(|es| es.iter().find(|e| e.name == *ext))
                    .map(|e| if e.workflow { "workflow" } else { "tool" })
                    .unwrap_or("missing on the server");
                let state = a
                    .link
                    .server()
                    .options
                    .get(p)
                    .and_then(|k| k.value())
                    .and_then(|os| os.iter().find(|o| o.extension == *ext))
                    .map(|o| {
                        let problems = tau_client::shapes::options_problems(o);
                        if problems.missing.is_empty() && problems.misfits.is_empty() {
                            "all set".into()
                        } else {
                            format!(
                                "{} to set · {} don't fit",
                                problems.missing.len(),
                                problems.misfits.len()
                            )
                        }
                    })
                    .unwrap_or("not received yet".into());
                out = out.child(facts(t, &format!("{ext} · {kind} · {state}")).mt(12.0));
            }
            out
        }
        _ => col(),
    }
}
fn decision(t: Theme, a: &App, d: &DecisionRef) -> El {
    let Some(v) = a.link.server().decisions.get(d).and_then(|k| k.value()) else {
        return col().child(facts(t, "Nothing received from the server yet"));
    };
    let r = &v.decision;
    let chars = v.text.chars().count();
    let count = if chars >= 1000 {
        format!("{},{:03}", chars / 1000, chars % 1000)
    } else {
        chars.to_string()
    };
    col()
        .w_full()
        .h_full()
        .min_h(0.0)
        .child(
            row()
                .center_y()
                .gap(8.0)
                .child(heading(t, &r.title).shrink().min_w(0.0))
                .child(content::line(
                    t,
                    &format!("#{}", d.id.request_number().unwrap_or(1)),
                    t.ty.mono_small,
                    t.c.kind.literal,
                )),
        )
        .child(facts(
            t,
            &format!(
                "{} · {} · requested {} · in #{} · basis entry {}{}",
                d.project,
                r.subject,
                crate::time::clock(Wall(r.requested_at)),
                r.conversation,
                r.basis,
                as_of(a)
            ),
        ))
        .child(facts(t, &r.choices.join(" · ")).mt(10.0))
        .child(
            row()
                .center_y()
                .mt(20.0)
                .child(text("basis").font(t.ty.ui_small).color(t.c.kind.you))
                .child(rune::spacer())
                .child(
                    text(format!("{count} characters"))
                        .font(t.ty.ui_small)
                        .color(t.c.fg.muted),
                ),
        )
        .child(
            col()
                .mt(8.0)
                .pl(12.0)
                .border_sides([0.0, 0.0, 0.0, 2.0], t.c.border.strong)
                .grow(1.0)
                .min_h(0.0)
                .clip_rounded(0.0)
                .child(content::markdown(t, &v.text).peek().build()),
        )
        .child(facts(t, "the rest is in the decision").my(10.0))
}
fn last_text(tr: &tau_client::model::Transcript) -> Option<&str> {
    let generation = tr
        .docs
        .value()
        .and_then(|d| d.live.generation.as_ref())
        .and_then(|g| g.message.as_ref());
    generation
        .into_iter()
        .chain(tr.entries.values().rev().filter_map(|e| match &e.body {
            EntryBody::Assistant(m) => Some(m),
            _ => None,
        }))
        .flat_map(|m| m.blocks.iter().rev())
        .find_map(|b| match b {
            Block::Text(s) => Some(s.as_str()),
            _ => None,
        })
}

fn conversation(t: Theme, a: &App, c: &ConvRef) -> El {
    let label = match a.link.server().label(c) {
        tau_client::Label::Found(s) => s,
        _ => "empty conversation".into(),
    };
    let mut out = col().w_full().min_h(0.0).child(heading(t, &label));
    let Some(tr) = a.link.server().transcripts.get(c) else {
        return out;
    };
    let Some(docs) = tr.docs.value() else {
        return out;
    };
    let model = docs
        .agent
        .model
        .as_deref()
        .unwrap_or("model default")
        .split('/')
        .next_back()
        .unwrap_or_default();
    let tools = match &docs.agent.extensions {
        tau_client::shapes::Selection::ProjectDefault => "project default".into(),
        tau_client::shapes::Selection::Only(names) => format!("only {}", names.join(", ")),
        tau_client::shapes::Selection::Change { add, .. } => {
            format!("project default + {}", add.join(", "))
        }
    };
    out = out.child(facts(
        t,
        &format!(
            "{} · #{} · {model} · {} · {tools}",
            c.project,
            c.id,
            docs.agent
                .thinking_level
                .as_deref()
                .unwrap_or("model default")
        ),
    ));
    let state = if let Some(run) = &docs.live.run {
        format!("running · turn {}", run.turns)
    } else if let Some(f) = &docs.live.run_failure {
        format!("run failed · {}", f.reason)
    } else {
        "idle".into()
    };
    out = out.child(facts(t, &format!("{state}{}", as_of(a))).mt(10.0));
    if let Some(f) = &docs.live.run_failure {
        out = out.child(
            text(f.detail.clone().unwrap_or_default())
                .wrap()
                .font(t.ty.mono_small)
                .color(t.c.fg.alt)
                .mt(12.0),
        );
    }
    let generation = docs
        .live
        .generation
        .as_ref()
        .and_then(|g| g.message.as_ref());
    if let Some(text) = last_text(tr) {
        out = out.child(
            rune::text("last agent text")
                .font(t.ty.ui_small)
                .color(t.c.kind.you)
                .mt(22.0),
        );
        out = out.child(
            content::markdown(t, text)
                .peek()
                .streaming(generation.is_some())
                .build()
                .pl(12.0)
                .mt(8.0)
                .border_sides([0.0, 0.0, 0.0, 2.0], t.c.border.strong),
        );
    }
    for tool in &docs.live.tools {
        let args = tr
            .entries
            .values()
            .rev()
            .filter_map(|e| match &e.body {
                EntryBody::Assistant(m) => Some(m),
                _ => None,
            })
            .flat_map(|m| &m.blocks)
            .find_map(|b| match b {
                Block::ToolCall { id, arguments, .. } if *id == tool.call_id => {
                    arguments["command"].as_str()
                }
                _ => None,
            })
            .unwrap_or_default();
        out = out.child(
            row()
                .center_y()
                .gap(8.0)
                .mt(22.0)
                .child(ui::mark(t, ui::Mark::Running))
                .child(
                    content::line(
                        t,
                        &format!("{} {args}", tool.name),
                        t.ty.mono_small,
                        t.c.fg.alt,
                    )
                    .shrink()
                    .min_w(0.0),
                )
                .child(rune::spacer())
                .child(rune::text("running").font(t.ty.ui_small).color(t.c.fg.alt)),
        );
        let lines = tool.output.lines().collect::<Vec<_>>();
        let tail = lines
            .iter()
            .skip(lines.len().saturating_sub(4))
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        out = out.child(
            text(tail)
                .font(t.ty.mono_output)
                .color(t.c.fg.alt)
                .fill(t.c.surface.sunken)
                .px(10.0)
                .py(10.0)
                .mt(8.0)
                .radius(4.0)
                .border(1.0, t.c.border.hairline),
        );
    }
    if !docs.queue.is_empty() {
        out = out.child(facts(t, &format!("{} queued · steer", docs.queue.len())).mt(12.0));
    }
    out
}
