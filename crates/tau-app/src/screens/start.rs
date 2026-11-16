//! The start page owns a device-local draft, its launch choice, and one pending create.
//! Server outcomes remain in tau-client; pickers snapshot the choices the user sees.

pub(crate) mod scenes;
#[cfg(test)]
mod tests;
mod view;

use super::{Screen, ScreenId};
use crate::app::{App, Focus, Handler, Invocation, Stage};
use crate::copy;
use crate::editor::{EditMode, Editor, Lines, Reply};
use crate::floats::{
    Float,
    project_picker::{self, Choice, Picker},
};
use crate::frame::{Crumb, Row};
use crate::keys::{Code, Ctx, Key, Mode};
use crate::open::Item;
use crate::theme::Theme;
use rune::{El, Tx};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tau_client::outbox::{StartWith, WriteCall, WriteState};
use tau_client::wire::{CallName, RefusalCode};
use tau_client::{LinkEvent, Write, WriteId};

#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
struct Saved {
    text: String,
    project: Option<String>,
    with: Option<StartWith>,
    last_project: Option<String>,
    last_with: BTreeMap<String, StartWith>,
}
pub struct State {
    pub editor: Editor,
    pub project: Option<String>,
    pub with: Option<StartWith>,
    pub pending: Option<WriteId>,
    pub failure: Option<String>,
    saved: Saved,
    loaded: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            editor: Editor::inserting("", Lines::Multi),
            project: None,
            with: None,
            pending: None,
            failure: None,
            saved: Saved::default(),
            loaded: false,
        }
    }
}
#[derive(Debug, PartialEq)]
pub(crate) enum Blocker {
    Ready,
    Loading,
    ChooseProject,
    NoProjects,
    NoModels,
    Missing(String),
    Options(String),
}

pub struct Start;
impl Screen for Start {
    fn ctx(&self, _: &App) -> Ctx {
        Ctx::Start
    }
    fn mode(&self, a: &App) -> Mode {
        editor_mode(&a.screens.start.editor)
    }
    fn crumbs(&self, _: &App) -> Vec<Crumb> {
        vec![Crumb::Text("New conversation".into())]
    }
    fn view(&self, t: Theme, a: &App) -> El {
        view::view(t, a)
    }
    fn globals(&self) -> Vec<(&'static str, Handler)> {
        vec![("conv.new", new)]
    }
    fn commands(&self) -> Vec<(&'static str, Handler)> {
        vec![
            ("start.send", send),
            ("start.project", |a, _, _| {
                project_picker(a, ProjectPurpose::Start)
            }),
            ("start.workflow", |a, _, _| workflow_picker(a)),
            ("editor.newline", |a, _, _| {
                if a.screens.start.pending.is_none() {
                    a.screens.start.editor.newline();
                }
            }),
            ("picker.keys", project_picker::keys),
        ]
    }
    fn keys(&self, a: &mut App, keys: &[Key], count: Option<u32>) -> bool {
        if !a.floats.is_empty() {
            return project_picker::typing(a);
        }
        if a.screens.start.pending.is_some() {
            if keys == [Key::plain(Code::Esc)] {
                a.screens.start.editor.insert(Key::plain(Code::Esc), a.now);
            }
            return true;
        }
        let ed = &mut a.screens.start.editor;
        if ed.mode() == EditMode::Insert {
            keys.iter().all(|k| ed.insert(*k, a.now) == Reply::Handled)
        } else {
            for k in keys {
                ed.normal(count, *k);
            }
            true
        }
    }
    fn shown(&self, a: &mut App) {
        load(a);
        a.screens.start.editor.enter_insert();
        a.screens.start.editor.edit.focused = true;
        a.link.refresh_catalogue();
        resolve(a);
        if let Some(p) = &a.screens.start.project {
            a.link.fetch_options(p.as_str().into());
        }
    }
    fn poll(&self, a: &mut App, now: f64, tx: &mut Tx) -> bool {
        if !a.screens.start.loaded {
            return false;
        }
        let changed = resolve(a);
        let ed = &mut a.screens.start.editor;
        if let Some(due) = ed.jk_deadline() {
            tx.wake_after(std::time::Duration::from_secs_f64((due - now).max(0.0)));
        }
        ed.tick(now) || changed
    }
    fn on_focus(&self, a: &mut App, focused: bool) {
        if !focused && a.screens.start.loaded {
            persist(a);
        }
    }
    fn on_event(&self, a: &mut App, event: &LinkEvent) {
        match event {
            LinkEvent::Started {
                conv, with, create, ..
            } if a.screens.start.pending == Some(*create) => {
                let s = &mut a.screens.start;
                s.pending = None;
                s.failure = None;
                s.editor.edit.replace_text("");
                s.saved.last_project = Some(conv.project.to_string());
                s.saved
                    .last_with
                    .insert(conv.project.to_string(), with.clone());
                persist(a);
                let item = Item::Conversation {
                    project: conv.project.to_string(),
                    id: conv.id.0,
                };
                if a.stage == Stage::Start {
                    a.show(Stage::Item(item));
                } else {
                    a.open.open(item);
                }
            }
            LinkEvent::Write {
                id,
                call: WriteCall::Create,
                state,
            } if a.screens.start.pending == Some(*id) => match state {
                WriteState::Unknown { .. } => {
                    a.screens.start.pending = None;
                    a.screens.start.failure = Some(copy::start_unknown(
                        &serde_json::json!({"project":a.screens.start.project}),
                    ));
                }
                WriteState::Refused(r) => {
                    a.screens.start.pending = None;
                    let body = a
                        .link
                        .outbox()
                        .get(*id)
                        .map(|e| e.body.clone())
                        .unwrap_or_default();
                    a.screens.start.failure =
                        copy::refusal(CallName::Create, &body, r, copy::AnswerAction::Answer)
                            .message;
                    if r.code == RefusalCode::OptionsInvalid
                        && a.stage == Stage::Start
                        && let Some(p) = a.screens.start.project.clone()
                    {
                        super::project::open_options(a, p, true);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn scenes(&self) -> Vec<crate::app::scenes::Scene> {
        scenes::all()
    }
}

pub(crate) fn editor_mode(ed: &Editor) -> Mode {
    match ed.mode() {
        EditMode::Insert => Mode::Insert,
        EditMode::Normal => Mode::Normal,
        _ => Mode::Visual,
    }
}
fn load(a: &mut App) {
    let s = &mut a.screens.start;
    if s.loaded {
        return;
    }
    s.loaded = true;
    s.saved = a.link.store().load("start.json").unwrap_or_default();
    s.project = s.saved.project.clone();
    s.with = s.saved.with.clone();
    s.editor.edit.replace_text(&s.saved.text);
}
fn persist(a: &mut App) {
    let s = &mut a.screens.start;
    let next = Saved {
        text: s.editor.text().into(),
        project: s.project.clone(),
        with: s.with.clone(),
        last_project: s.saved.last_project.clone(),
        last_with: s.saved.last_with.clone(),
    };
    if next != s.saved {
        let result = a.link.store().save("start.json", &next);
        s.saved = next;
        if let Err(e) = result {
            a.say(format!(
                "Could not keep the start draft on this device: {e}"
            ));
        }
    }
}
/// Project selection shared by New, Options, and Add extensions.
pub(crate) fn named_project(a: &App) -> Option<String> {
    if a.focus == Focus::Sidebar {
        if let Row::Item(i) = &a.sidebar_cursor {
            return Some(i.project().into());
        }
    } else if let Stage::Item(i) = &a.stage {
        return Some(i.project().into());
    }
    if a.stage == Stage::Start
        && let Some(p) = &a.screens.start.project
    {
        return Some(p.clone());
    }
    a.screens.start.saved.last_project.clone().or_else(|| {
        let ps = a.link.server().projects.value()?;
        (ps.len() == 1).then(|| ps[0].project.to_string())
    })
}
fn new(a: &mut App, _: &Invocation, _: &mut Tx) {
    load(a);
    let context = if a.focus == Focus::Sidebar {
        match &a.sidebar_cursor {
            Row::Item(i) => Some(i.project().to_string()),
            _ => None,
        }
    } else {
        match &a.stage {
            Stage::Item(i) => Some(i.project().to_string()),
            _ => None,
        }
    };
    let project = context
        .or_else(|| a.screens.start.project.clone())
        .or_else(|| named_project(a));
    if a.screens.start.pending.is_none()
        && let Some(p) = project
    {
        set_project(a, p);
    }
    a.show(Stage::Start);
}
fn resolve(a: &mut App) -> bool {
    if a.screens.start.pending.is_some() {
        return false;
    }
    if a.screens.start.project.is_none() {
        let p = a.screens.start.saved.last_project.clone().or_else(|| {
            let ps = a.link.server().projects.value()?;
            (ps.len() == 1).then(|| ps[0].project.to_string())
        });
        if let Some(p) = p {
            set_project(a, p);
            return true;
        }
    }
    if a.screens.start.with.is_none()
        && let Some(p) = a.screens.start.project.clone()
    {
        a.screens.start.with = default_with(a, &p);
        return a.screens.start.with.is_some();
    }
    false
}
fn default_with(a: &App, p: &str) -> Option<StartWith> {
    if let Some(w) = a.screens.start.saved.last_with.get(p) {
        return Some(w.clone());
    }
    let server = a.link.server();
    let project = server
        .projects
        .value()?
        .iter()
        .find(|x| x.project.as_str() == p)?;
    let extensions = server.extensions.value()?;
    let workflows: Vec<_> = extensions
        .iter()
        .filter(|e| e.workflow && project.extensions.contains(&e.name))
        .collect();
    if workflows.len() == 1 {
        Some(StartWith::Workflow(workflows[0].name.clone()))
    } else {
        server
            .models
            .value()?
            .first()
            .map(|m| StartWith::Model(m.model.clone()))
    }
}
pub(crate) fn set_project(a: &mut App, p: String) {
    if a.screens.start.project.as_ref() != Some(&p) {
        a.screens.start.with = default_with(a, &p);
        a.screens.start.failure = None;
    }
    a.screens.start.project = Some(p.clone());
    a.link.fetch_options(p.as_str().into());
}
pub(crate) fn blocker(a: &App) -> Blocker {
    let s = &a.screens.start;
    let Some(projects) = a.link.server().projects.value() else {
        return Blocker::Loading;
    };
    if projects.is_empty() {
        return Blocker::NoProjects;
    }
    let Some(p) = s.project.as_ref() else {
        return Blocker::ChooseProject;
    };
    let Some(project) = projects.iter().find(|x| x.project.as_str() == p) else {
        return Blocker::ChooseProject;
    };
    match &s.with {
        Some(StartWith::Workflow(w)) => {
            if project.missing.contains(w) {
                return Blocker::Missing(w.clone());
            }
            let Some(opts) = super::project::options(a, project.project.as_str()) else {
                return Blocker::Loading;
            };
            if let Some(o) = opts.iter().find(|o| &o.extension == w)
                && !o.errors.is_empty()
            {
                return Blocker::Options(super::project::option_errors(o));
            }
            Blocker::Ready
        }
        Some(StartWith::Model(_)) => match a.link.server().models.value() {
            None => Blocker::Loading,
            Some(models) if models.is_empty() => Blocker::NoModels,
            Some(_) => Blocker::Ready,
        },
        None if a.link.server().models.value().is_none() => Blocker::Loading,
        None => Blocker::NoModels,
    }
}
fn send(a: &mut App, _: &Invocation, _: &mut Tx) {
    let s = &a.screens.start;
    if s.pending.is_some() {
        return;
    }
    match blocker(a) {
        Blocker::NoProjects => {
            super::project::bind(a);
            return;
        }
        Blocker::ChooseProject => {
            project_picker(a, ProjectPurpose::Start);
            return;
        }
        Blocker::Options(_) => {
            if let Some(p) = s.project.clone() {
                super::project::open_options(a, p, true);
            }
            return;
        }
        Blocker::Missing(w) => {
            a.say(format!("the server has no code for {w}"));
            return;
        }
        Blocker::NoModels => {
            a.say("bare agent · no models on the server");
            return;
        }
        Blocker::Loading => {
            a.say(super::NOTHING_RECEIVED);
            return;
        }
        Blocker::Ready => {}
    }
    let s = &a.screens.start;
    let (Some(p), Some(w)) = (&s.project, &s.with) else {
        return;
    };
    let write = Write::Start {
        project: p.as_str().into(),
        with: w.clone(),
        text: s.editor.text().into(),
    };
    match a.link.write(write) {
        Ok(id) => {
            a.screens.start.pending = Some(id);
            a.screens.start.failure = None;
            persist(a);
        }
        Err(e) => {
            if let Some(words) = copy::project_not_sent(&e, true) {
                a.say(words);
            }
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) enum ProjectPurpose {
    Start,
    Options,
    Add,
}
pub(crate) fn project_picker(a: &mut App, purpose: ProjectPurpose) {
    if matches!(purpose, ProjectPurpose::Start) && a.screens.start.pending.is_some() {
        return;
    }
    a.link.refresh_catalogue();
    let current = named_project(a);
    let mut rows = Vec::new();
    for p in a.link.server().projects.value().into_iter().flatten() {
        let mut r = Choice::new(
            p.project.to_string(),
            p.project.to_string(),
            if p.extensions.is_empty() {
                "bare agent only".into()
            } else {
                p.extensions.join(" · ")
            },
        );
        r.path = p.workdir.clone();
        r.project = true;
        if Some(p.project.as_str()) == a.screens.start.saved.last_project.as_deref() {
            r.note = "last used".into();
        }
        rows.push(r);
    }
    rows.sort_by_key(|r| Some(&r.value) != current.as_ref());
    let (owner, sub) = match purpose {
        ProjectPurpose::Start => (ScreenId::Start, "for the new conversation"),
        ProjectPurpose::Options => (ScreenId::Project, "to set options"),
        ProjectPurpose::Add => (ScreenId::Project, "to add extensions"),
    };
    a.open_float(Float::Screen(Box::new(Picker::new(
        owner,
        "Project",
        sub.into(),
        "projects",
        rows,
        current,
        Box::new(move |a, p| match purpose {
            ProjectPurpose::Start => set_project(a, p),
            ProjectPurpose::Options => super::project::open_options(a, p, false),
            ProjectPurpose::Add => super::project::add(a, p),
        }),
    ))));
}
fn workflow_picker(a: &mut App) {
    if a.screens.start.pending.is_some() {
        return;
    }
    let Some(p) = a.screens.start.project.clone() else {
        project_picker(a, ProjectPurpose::Start);
        return;
    };
    a.link.refresh_catalogue();
    a.link.fetch_options(p.as_str().into());
    let server = a.link.server();
    let Some(project) = server
        .projects
        .value()
        .and_then(|ps| ps.iter().find(|x| x.project.as_str() == p))
    else {
        return;
    };
    let mut rows = Vec::new();
    for e in server.extensions.value().into_iter().flatten().filter(|e| {
        e.workflow && project.extensions.contains(&e.name) && !project.missing.contains(&e.name)
    }) {
        let mut r = Choice::new(
            format!("w:{}", e.name),
            format!("{} · workflow", e.name),
            "",
        );
        if let Some(o) = server
            .options
            .get(&project.project)
            .and_then(|o| o.value())
            .and_then(|os| os.iter().find(|o| o.extension == e.name))
        {
            r.detail = super::project::option_errors(o);
            r.error = !o.errors.is_empty();
        }
        rows.push(r);
    }
    for m in server.models.value().into_iter().flatten() {
        rows.push(Choice::new(
            format!("m:{}", m.model),
            "bare agent",
            m.name.clone(),
        ));
    }
    let selected = a.screens.start.with.as_ref().map(|w| match w {
        StartWith::Workflow(w) => format!("w:{w}"),
        StartWith::Model(m) => format!("m:{m}"),
    });
    a.open_float(Float::Screen(Box::new(Picker::new(
        ScreenId::Start,
        "Workflow",
        format!("for the new conversation in {p}"),
        "choices",
        rows,
        selected,
        Box::new(|a, v| {
            a.screens.start.with = Some(if let Some(w) = v.strip_prefix("w:") {
                StartWith::Workflow(w.into())
            } else {
                StartWith::Model(v[2..].into())
            });
            a.screens.start.failure = None;
        }),
    ))));
}
