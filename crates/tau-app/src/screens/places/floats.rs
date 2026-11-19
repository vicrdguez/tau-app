//! Controllers for list floats. Renderers under `floats/` take only plain text and marks.
use super::{RowId, home, list, place, reconcile, save, settings_mut};
use crate::app::{App, Invocation};
use crate::floats::list_picker::{Kind as Found, Line};
use crate::floats::{self as draw, Float, Noun};
use crate::keys::{Code, Ctx, Key, Mode};
use crate::screens::{ScreenFloat, ScreenId, top_float};
use crate::theme::Theme;
use crate::ui::{Badge, Scope};
use rune::{El, TextEdit, Tx};
use tau_client::{
    WriteId,
    streams::{Phase, StreamKey},
};

#[derive(Clone, Debug)]
pub enum Choice {
    Object(RowId),
    Project(Option<String>),
    State(Option<String>),
}
#[derive(Clone, Debug)]
pub struct PickRow {
    pub label: String,
    pub target: Choice,
}
#[derive(Clone, Copy)]
enum Kind {
    Find,
    Filter,
    Search,
}
struct Picker {
    kind: Kind,
    query: TextEdit,
    cursor: usize,
    frozen: Option<Vec<PickRow>>,
}
impl Picker {
    fn rows(&self, a: &App) -> Vec<PickRow> {
        if let Some(rows) = &self.frozen {
            return rows.clone();
        }
        self.matching(a, &self.query.text)
    }
    fn matching(&self, a: &App, query: &str) -> Vec<PickRow> {
        match self.kind {
            Kind::Find => find_rows(a, query),
            Kind::Search => super::matching_rows(a, query)
                .into_iter()
                .map(|r| PickRow {
                    label: r.label,
                    target: Choice::Object(r.id),
                })
                .collect(),
            Kind::Filter => {
                let mut rows = vec![PickRow {
                    label: "all projects".into(),
                    target: Choice::Project(None),
                }];
                rows.extend(
                    a.link
                        .server()
                        .projects
                        .value()
                        .into_iter()
                        .flatten()
                        .map(|p| PickRow {
                            label: p.project.to_string(),
                            target: Choice::Project(Some(p.project.to_string())),
                        }),
                );
                if place(a) == crate::frame::Place::Conversations {
                    rows.push(PickRow {
                        label: "all states".into(),
                        target: Choice::State(None),
                    });
                    rows.extend(["failed", "running", "idle"].map(|s| PickRow {
                        label: s.into(),
                        target: Choice::State(Some(s.into())),
                    }));
                }
                rows.into_iter()
                    .filter(|r| r.label.to_lowercase().contains(&query.to_lowercase()))
                    .collect()
            }
        }
    }
}
impl ScreenFloat for Picker {
    fn owner(&self) -> ScreenId {
        ScreenId::Places
    }
    fn scope(&self) -> Scope {
        Scope {
            ctx: Ctx::Picker,
            mode: Mode::Command,
        }
    }
    fn badge(&self) -> Badge {
        Badge::Command
    }
    fn edit_mut(&mut self) -> Option<&mut TextEdit> {
        Some(&mut self.query)
    }
    fn view(&self, t: Theme, a: &App) -> El {
        let rows = self.rows(a);
        let q = &self.query.text;
        let total = self.matching(a, "").len();
        let (title, subtitle, label, noun) = match self.kind {
            Kind::Find => (
                "Find",
                "projects, conversations, decisions, events",
                "Search projects, conversations, decisions and events".to_string(),
                Noun::new("result", "results"),
            ),
            Kind::Filter => (
                "Filter",
                place(a).name(),
                if place(a) == crate::frame::Place::Conversations {
                    "Filter projects and states".to_string()
                } else {
                    "Filter projects".to_string()
                },
                Noun::new("filter", "filters"),
            ),
            Kind::Search => {
                let noun = match place(a) {
                    crate::frame::Place::Timeline => Noun::new("event", "events"),
                    crate::frame::Place::Faults => Noun::new("fault", "faults"),
                    _ => Noun::new("conversation", "conversations"),
                };
                (
                    "Search",
                    place(a).name(),
                    format!("Search {}", noun.many),
                    noun,
                )
            }
        };
        let find = matches!(self.kind, Kind::Find);
        draw::list_picker::view(
            t,
            self.scope(),
            draw::list_picker::List {
                title,
                subtitle,
                rows: &rows
                    .iter()
                    .map(|r| match &r.target {
                        Choice::Object(id) if find => found(id, &r.label),
                        _ => Line::Plain(r.label.clone()),
                    })
                    .collect::<Vec<_>>(),
                cursor: self.cursor,
                input: draw::Input {
                    edit: &self.query,
                    label: &label,
                    placeholder: if find { "type to search" } else { draw::FILTER },
                    count: noun.kept(q, rows.len(), total),
                },
                none: if find {
                    format!("Nothing matches \"{q}\"")
                } else {
                    noun.none(q)
                },
            },
        )
    }
}
fn picker(a: &mut App, kind: Kind) {
    let mut query = TextEdit::new("");
    query.focused = true;
    a.open_float(Float::Screen(Box::new(Picker {
        kind,
        query,
        cursor: 0,
        frozen: None,
    })));
}
pub fn find(a: &mut App) {
    picker(a, Kind::Find)
}
pub fn filter(a: &mut App) {
    picker(a, Kind::Filter)
}
pub fn search(a: &mut App) {
    picker(a, Kind::Search)
}
/// A Find row as the list picker draws it: its kind and its project come from what it opens.
fn found(id: &RowId, label: &str) -> Line {
    let (kind, project) = match id {
        RowId::Project(_) => return Line::Project(label.into()),
        RowId::Conversation(c) => (Found::Conversation, c.project.to_string()),
        RowId::Decision(d) => (Found::Decision, d.project.to_string()),
        RowId::Event(p, _) => (Found::Event, p.to_string()),
        _ => return Line::Plain(label.into()),
    };
    Line::In {
        kind,
        label: label.into(),
        project,
    }
}
pub fn find_rows(a: &App, query: &str) -> Vec<PickRow> {
    let (prefix, query) = query
        .split_once(':')
        .filter(|(p, _)| matches!(*p, "p" | "c" | "d" | "e"))
        .map_or(("", query), |(p, q)| (p, q));
    let query = query.trim().to_lowercase();
    let hit = |s: &str| s.to_lowercase().contains(&query);
    let mut rows = Vec::new();
    if prefix.is_empty() || prefix == "p" {
        rows.extend(
            a.link
                .server()
                .projects
                .value()
                .into_iter()
                .flatten()
                .filter(|p| hit(p.project.as_str()))
                .map(|p| PickRow {
                    label: p.project.to_string(),
                    target: Choice::Object(RowId::Project(p.project.clone())),
                }),
        );
    }
    for (p, rs) in [
        ("c", list::conversations(a)),
        (
            "d",
            home::rows(a)
                .into_iter()
                .filter(|r| matches!(r.id, RowId::Decision(_)))
                .collect(),
        ),
        ("e", list::events(a)),
    ] {
        if prefix.is_empty() || prefix == p {
            rows.extend(
                rs.into_iter()
                    .filter(|r| hit(&r.label) || hit(&r.project))
                    .map(|r| PickRow {
                        label: r.label,
                        target: Choice::Object(r.id),
                    }),
            );
        }
    }
    rows
}
pub fn typing(a: &mut App, _: &[Key]) -> bool {
    if let Some(p) = top_float::<Picker>(a) {
        p.frozen = None;
        p.cursor = 0;
        return false;
    }
    true
}
pub fn picker_keys(a: &mut App, inv: &Invocation, tx: &mut Tx) {
    let Some(&key) = inv.keys.first() else { return };
    if key == Key::plain(Code::Esc) || key == Key::ctrl('c') {
        a.close_float();
        return;
    }
    let Some((Float::Screen(f), opener)) = a.floats.pop() else {
        return;
    };
    let mut f = f;
    let Some(p) = (f.as_mut() as &mut dyn std::any::Any).downcast_mut::<Picker>() else {
        a.floats.push((Float::Screen(f), opener));
        return;
    };
    let rows = p.rows(a);
    if key == Key::plain(Code::Enter) && p.cursor < rows.len() {
        if let Some(r) = rows.get(p.cursor) {
            match &r.target {
                Choice::Object(id) => {
                    if matches!(p.kind, Kind::Search) {
                        settings_mut(a).search = p.query.text.clone();
                        home::select(a, id.clone());
                    } else {
                        list::open(a, id, false);
                    }
                }
                Choice::Project(project) => {
                    settings_mut(a).project = project.clone();
                    reconcile(a);
                    save(a);
                }
                Choice::State(state) => {
                    settings_mut(a).state = state.clone();
                    reconcile(a);
                    save(a);
                }
            }
        }
        return;
    }
    if key == Key::ctrl('n') || key == Key::ctrl('p') {
        tx.scroll_page("picker-preview", if key == Key::ctrl('n') { 1 } else { -1 });
    }
    if key == Key::ctrl('j') || key == Key::ctrl('k') {
        let by = if key == Key::ctrl('j') { 1 } else { -1 };
        p.cursor = (p.cursor as i64 + by).clamp(0, rows.len().saturating_sub(1) as i64) as usize;
        p.frozen = Some(rows);
    }
    a.floats.push((Float::Screen(f), opener));
}

struct Outbox {
    cursor: Option<WriteId>,
    details: bool,
}
impl ScreenFloat for Outbox {
    fn owner(&self) -> ScreenId {
        ScreenId::Places
    }
    fn scope(&self) -> Scope {
        Scope {
            ctx: Ctx::Outbox,
            mode: Mode::Outbox,
        }
    }
    fn badge(&self) -> Badge {
        Badge::Outbox
    }
    fn view(&self, t: Theme, a: &App) -> El {
        let entries = a.link.outbox().entries();
        let cursor = entries
            .iter()
            .position(|e| Some(e.id) == self.cursor)
            .unwrap_or(0);
        let rows = entries
            .iter()
            .map(|e| {
                let copy = crate::copy::outbox_row(e, home::now(a));
                let about = match e.decision() {
                    Some(d) => Some(match decision_title(a, &d) {
                        Some(title) => draw::outbox::About::Decision {
                            title,
                            number: d.id.request_number().unwrap_or(1),
                        },
                        None => draw::outbox::About::DecisionId(d.id.to_string()),
                    }),
                    None => e
                        .text()
                        .filter(|s| !s.is_empty())
                        .map(|s| draw::outbox::About::Text(s.into())),
                };
                draw::outbox::Row {
                    call: copy.call,
                    project: e.project().map(|p| p.to_string()),
                    conversation: e.conversation().map(|c| c.id.0),
                    about,
                    state: copy.state,
                    time: crate::time::clock(e.created),
                    unknown: copy.unknown,
                    opens: opens(e).is_some(),
                }
            })
            .collect::<Vec<_>>();
        let current = entries.get(cursor);
        let details = self.details.then(|| {
            current
                .map(|e| serde_json::to_string_pretty(&e.body).unwrap_or_default())
                .unwrap_or_default()
        });
        draw::outbox::view(
            t,
            self.scope(),
            &rows,
            cursor,
            a.link.outbox().unknown_count(),
            current.is_some_and(|e| e.can_retry()) && a.link.status().writes_on,
            details.as_deref(),
        )
    }
}
/// Where `⏎` on an outbox entry goes: the object it names. It never sends.
enum Opens {
    Item(crate::open::Item),
    Start,
}

fn opens(e: &tau_client::outbox::OutboxEntry) -> Option<Opens> {
    use crate::open::Item;
    if e.call == tau_client::WriteCall::Create {
        return Some(Opens::Start);
    }
    if let Some(d) = e.decision() {
        return Some(Opens::Item(Item::Decision {
            project: d.project.to_string(),
            id: d.id.to_string(),
        }));
    }
    if let Some(c) = e.conversation() {
        return Some(Opens::Item(Item::Conversation {
            project: c.project.to_string(),
            id: c.id.0,
        }));
    }
    e.project()
        .map(|p| Opens::Item(Item::Project(p.to_string())))
}

/// The decision's title, from the inbox or a fetched decision.
fn decision_title(a: &App, d: &tau_client::DecisionRef) -> Option<String> {
    let s = a.link.server();
    s.inbox
        .value()
        .and_then(|i| i.projects.get(&d.project))
        .and_then(|subjects| subjects.get(&d.id.subject()))
        .filter(|open| open.id == d.id)
        .map(|open| open.title.clone())
        .or_else(|| {
            s.decisions
                .get(d)
                .and_then(|v| v.value())
                .map(|v| v.decision.title.clone())
        })
}

pub fn outbox(a: &mut App) {
    let cursor = a
        .link
        .outbox()
        .entries()
        .iter()
        .find(|e| e.is_unknown())
        .or_else(|| a.link.outbox().entries().first())
        .map(|e| e.id);
    a.open_float(Float::Screen(Box::new(Outbox {
        cursor,
        details: false,
    })));
}
pub fn outbox_keys(a: &mut App, inv: &Invocation, tx: &mut Tx) {
    let Some(&key) = inv.keys.first() else { return };
    if key == Key::plain(Code::Esc) || key == Key::char('q') {
        a.close_float();
        return;
    }
    if key == Key::plain(Code::Enter) {
        let cursor = top_float::<Outbox>(a).and_then(|o| o.cursor);
        let target = cursor
            .and_then(|id| a.link.outbox().get(id))
            .and_then(opens);
        match target {
            Some(Opens::Item(item)) => {
                a.close_float();
                a.show(crate::app::Stage::Item(item));
            }
            Some(Opens::Start) => {
                a.close_float();
                crate::app::commands::run_id(a, "conv.new", tx);
            }
            None => {}
        }
        return;
    }
    let ids = a
        .link
        .outbox()
        .entries()
        .iter()
        .map(|e| e.id)
        .collect::<Vec<_>>();
    if let Some(o) = top_float::<Outbox>(a) {
        if key == Key::char('K') {
            o.details = !o.details;
            return;
        }
        let at = ids.iter().position(|id| Some(*id) == o.cursor).unwrap_or(0);
        let to = if key == Key::char('j') {
            (at + 1).min(ids.len().saturating_sub(1))
        } else {
            at.saturating_sub(1)
        };
        o.cursor = ids.get(to).copied();
    }
}
pub fn forget(a: &mut App) {
    let Some(id) = top_float::<Outbox>(a).and_then(|o| o.cursor) else {
        return;
    };
    let ids = a
        .link
        .outbox()
        .entries()
        .iter()
        .map(|e| e.id)
        .collect::<Vec<_>>();
    let at = ids.iter().position(|i| *i == id).unwrap_or(0);
    a.link.forget(id);
    let ids = a
        .link
        .outbox()
        .entries()
        .iter()
        .map(|e| e.id)
        .collect::<Vec<_>>();
    if let Some(o) = top_float::<Outbox>(a) {
        o.cursor = ids.get(at.min(ids.len().saturating_sub(1))).copied();
    }
    crate::app::link::refresh(a);
}
pub fn retry(a: &mut App, _: &Invocation, _: &mut Tx) {
    let Some(id) = top_float::<Outbox>(a).and_then(|o| o.cursor) else {
        return;
    };
    if a.link.outbox().get(id).is_some_and(|e| e.can_retry()) && a.link.status().writes_on {
        let _ = a.link.retry(id);
    }
}
struct Server {
    offset: usize,
}
impl ScreenFloat for Server {
    fn owner(&self) -> ScreenId {
        ScreenId::Places
    }
    fn scope(&self) -> Scope {
        Scope {
            ctx: Ctx::Server,
            mode: Mode::Server,
        }
    }
    fn badge(&self) -> Badge {
        Badge::Server
    }
    fn view(&self, t: Theme, a: &App) -> El {
        let s = a.link.server();
        let mut facts = vec![
            (
                "device".into(),
                s.device
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or("not paired".into()),
            ),
            ("protocol".into(), "1".into()),
        ];
        let since = a
            .link
            .status()
            .streams
            .iter()
            .filter_map(|(_, s)| {
                if let Phase::Live { since, .. } = s.phase {
                    Some(since)
                } else {
                    None
                }
            })
            .min();
        if let Some(since) = since {
            facts.push((
                "connected".into(),
                format!("since {}", crate::time::clock(since)),
            ));
        }
        for (key, stream) in &a.link.status().streams {
            let name = match key {
                StreamKey::Inbox => "inbox".into(),
                StreamKey::Timeline(_) => "timeline".into(),
                StreamKey::Open(_) => "open".into(),
            };
            let state = match stream.phase {
                Phase::Live { .. } => "live",
                Phase::Connecting { .. } => "connecting",
                Phase::Backoff { .. } => "reconnecting",
                Phase::Revoked => "revoked",
            };
            let detail = match key {
                StreamKey::Inbox => state.into(),
                StreamKey::Timeline(p) => format!("{p} · {state}"),
                StreamKey::Open(c) => format!("{} #{} · {state}", c.project, c.id),
            };
            facts.push((name, detail));
        }
        draw::server::view(
            t,
            self.scope(),
            s.url.as_deref().unwrap_or("not connected"),
            &facts,
            self.offset,
        )
    }
}
pub fn server(a: &mut App) {
    a.open_float(Float::Screen(Box::new(Server { offset: 0 })));
}
pub fn server_keys(a: &mut App, inv: &Invocation, _: &mut Tx) {
    let Some(&key) = inv.keys.first() else { return };
    if key == Key::plain(Code::Esc) || key == Key::char('q') {
        a.close_float();
        return;
    }
    let max = a.link.status().streams.len().saturating_sub(4);
    if let Some(s) = top_float::<Server>(a) {
        s.offset = if key == Key::char('j') {
            (s.offset + 1).min(max)
        } else {
            s.offset.saturating_sub(1)
        };
    }
}
