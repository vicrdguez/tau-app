//! Object identities and row projections shared by Home, places, and the peek.
pub use super::rows::{Slots, line, row_view, stage_width};
use serde::{Deserialize, Serialize};
use tau_client::shapes::event_extension;
use tau_client::wire::RunState;
use tau_client::{ConvRef, DecisionRef, EntryId, Label, Project, TaskId};

use crate::app::{App, Stage};
use crate::frame::Place;
use crate::open::Item;
use crate::ui::Mark;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RowId {
    Decision(DecisionRef),
    Conversation(ConvRef),
    Event(Project, EntryId),
    Fault(Project, TaskId),
    Project(Project),
    MoreRuns,
    MoreFaults,
    MoreEvents,
}

#[derive(Clone, Debug)]
pub struct ListRow {
    pub id: RowId,
    pub group: String,
    pub project: String,
    pub label: String,
    pub conversation: Option<u64>,
    pub extension: String,
    pub model: String,
    pub reason: String,
    pub after: Option<String>,
    pub at: Option<i64>,
    pub mark: Option<Mark>,
    pub hidden: bool,
}

impl ListRow {
    pub fn new(id: RowId, group: &str, project: &str, label: String) -> Self {
        Self {
            id,
            group: group.into(),
            project: project.into(),
            label,
            conversation: None,
            extension: String::new(),
            model: String::new(),
            reason: String::new(),
            after: None,
            at: None,
            mark: None,
            hidden: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Cursor {
    pub id: Option<RowId>,
    #[serde(skip)]
    previous: Vec<RowId>,
}

impl Cursor {
    pub fn reconcile(&mut self, ids: &[RowId]) -> bool {
        let before = self.id.clone();
        if !self.id.as_ref().is_some_and(|id| ids.contains(id)) {
            let at = self
                .id
                .as_ref()
                .and_then(|id| self.previous.iter().position(|r| r == id))
                .unwrap_or(0);
            self.id = self
                .previous
                .iter()
                .skip(at + 1)
                .find(|id| ids.contains(id))
                .cloned()
                .or_else(|| ids.get(at.min(ids.len().saturating_sub(1))).cloned());
        }
        self.previous = ids.to_vec();
        self.id != before
    }
    pub fn index(&self, rows: &[ListRow]) -> usize {
        rows.iter()
            .position(|r| Some(&r.id) == self.id.as_ref())
            .unwrap_or(0)
    }
}

pub fn conversations(app: &App) -> Vec<ListRow> {
    let s = app.link.server();
    let mut out = Vec::new();
    for (p, known) in &s.conversations {
        let mut rows: Vec<_> = known.value().into_iter().flatten().collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.conversation));
        for r in rows {
            let conv = ConvRef::new(p.clone(), r.conversation.0);
            let label = match s.label(&conv) {
                Label::Found(s) => s,
                _ => "empty conversation".into(),
            };
            let mut row = ListRow::new(
                RowId::Conversation(conv.clone()),
                p.as_str(),
                p.as_str(),
                label,
            );
            row.conversation = Some(r.conversation.0);
            row.mark = match r.state {
                RunState::Running => Some(Mark::Running),
                RunState::Failed => Some(Mark::Failed),
                _ => None,
            };
            row.reason = match r.state {
                RunState::Running => "running",
                RunState::Failed => "run failed",
                _ => "idle",
            }
            .into();
            row.hidden = app.hidden.runs.contains(&conv);
            if let Some(docs) = s.transcripts.get(&conv).and_then(|t| t.docs.value()) {
                row.model = docs
                    .agent
                    .model
                    .as_deref()
                    .unwrap_or_default()
                    .split('/')
                    .next_back()
                    .unwrap_or_default()
                    .into();
                row.extension = tau_client::shapes::selection_workflow(
                    &docs.agent.extensions,
                    s.extensions.value().map(Vec::as_slice).unwrap_or_default(),
                )
                .unwrap_or_default()
                .into();
            }
            if row.extension.is_empty() {
                row.extension = s
                    .timelines
                    .get(p)
                    .into_iter()
                    .flat_map(|t| t.entries.values())
                    .rev()
                    .find(|e| e.raw["data"]["conversation"].as_u64() == Some(r.conversation.0))
                    .and_then(|e| event_extension(&e.kind))
                    .unwrap_or_default()
                    .into();
            }
            out.push(row);
        }
    }
    out
}

pub fn faults(app: &App) -> Vec<ListRow> {
    let mut out = Vec::new();
    for (p, known) in &app.link.server().faults {
        for f in known.value().into_iter().flatten() {
            let mut row = ListRow::new(
                RowId::Fault(p.clone(), f.task),
                "Failed",
                p.as_str(),
                f.kind.clone(),
            );
            row.conversation = Some(f.conversation.0);
            row.extension = event_extension(&f.kind).unwrap_or_default().into();
            row.reason = "task faulted".into();
            row.after = f.decision.as_ref().map(ToString::to_string);
            row.at = f.ended_at;
            row.mark = Some(Mark::Failed);
            row.hidden = app.hidden.faults.contains(&(p.clone(), f.task));
            out.push(row);
        }
    }
    out.sort_by_key(|r| std::cmp::Reverse(r.at));
    out
}

pub fn events(app: &App) -> Vec<ListRow> {
    let mut out = Vec::new();
    for (p, timeline) in &app.link.server().timelines {
        for e in timeline.entries.values() {
            let data = &e.raw["data"];
            let Some(label) = data["text"].as_str() else {
                continue;
            };
            let mut row = ListRow::new(
                RowId::Event(p.clone(), e.id),
                "Activity",
                p.as_str(),
                label.into(),
            );
            row.at = data["at"].as_i64();
            row.conversation = data["conversation"].as_u64();
            row.extension = e.kind.clone();
            out.push(row);
        }
    }
    out.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| a.project.cmp(&b.project)));
    out
}

pub fn item(id: &RowId, app: &App, linked: bool) -> Option<Item> {
    match id {
        RowId::Conversation(c) => Some(Item::Conversation {
            project: c.project.to_string(),
            id: c.id.0,
        }),
        RowId::Decision(d) if !linked => Some(Item::Decision {
            project: d.project.to_string(),
            id: d.id.to_string(),
        }),
        RowId::Decision(d) => {
            let id = app
                .link
                .server()
                .inbox
                .value()
                .and_then(|i| i.projects.get(&d.project))
                .and_then(|s| s.get(&d.id.subject()))
                .map(|r| r.conversation.0)
                .or_else(|| {
                    app.link
                        .server()
                        .decisions
                        .get(d)
                        .and_then(|v| v.value())
                        .map(|v| v.decision.conversation.0)
                })?;
            Some(Item::Conversation {
                project: d.project.to_string(),
                id,
            })
        }
        RowId::Fault(p, task) => {
            let f = app
                .link
                .server()
                .faults
                .get(p)?
                .value()?
                .iter()
                .find(|f| f.task == *task)?;
            if let Some(d) = &f.decision {
                Some(Item::Decision {
                    project: p.to_string(),
                    id: d.to_string(),
                })
            } else {
                Some(Item::Conversation {
                    project: p.to_string(),
                    id: f.conversation.0,
                })
            }
        }
        RowId::Event(p, id) => {
            let e = app.link.server().timelines.get(p)?.entries.get(id)?;
            Some(Item::Conversation {
                project: p.to_string(),
                id: e.raw["data"]["conversation"].as_u64()?,
            })
        }
        RowId::Project(p) => Some(Item::Project(p.to_string())),
        _ => None,
    }
}

pub fn open(app: &mut App, id: &RowId, linked: bool) {
    match id {
        RowId::MoreRuns => {
            app.screens.places.conversations.state = Some("failed".into());
            app.show(Stage::Place(Place::Conversations));
        }
        RowId::MoreFaults => app.show(Stage::Place(Place::Faults)),
        RowId::MoreEvents => app.show(Stage::Place(Place::Timeline)),
        _ => {
            if let Some(i) = item(id, app, linked) {
                app.show(Stage::Item(i));
            }
        }
    }
}

pub fn hide(app: &mut App, id: &RowId) {
    match id {
        RowId::Fault(p, task) => {
            let key = (p.clone(), *task);
            if !app.hidden.faults.remove(&key) {
                app.hidden.faults.insert(key);
            }
        }
        RowId::Conversation(conv)
            if conversations(app)
                .iter()
                .any(|r| r.id == *id && r.mark == Some(Mark::Failed)) =>
        {
            if !app.hidden.runs.remove(conv) {
                app.hidden.runs.insert(conv.clone());
            }
        }
        _ => return,
    }
    super::save(app);
    crate::app::link::refresh(app);
}
