//! Project options are keyed drafts, never writes until :w. Project rows read the same polls as Home.
mod actions;
mod scenes;
#[cfg(test)]
mod tests;
mod view;

use super::{Screen, start};
use crate::{
    app::{App, Handler, Stage},
    editor::{EditMode, Editor, Reply},
    floats::{Float, project_picker},
    frame::{Crumb, Place, Room},
    keys::{Code, Ctx, Key, Mode},
    open::Item,
    theme::Theme,
};
use rune::{El, Tx};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use tau_client::outbox::{WriteCall, WriteState};
use tau_client::wire::{FieldError, OptionsItem, RunState};
use tau_client::{LinkEvent, WriteId};

pub(crate) use actions::{add, bind, open_options};
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FieldKey {
    pub project: String,
    pub extension: String,
    pub key: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Work(Place),
    Field(FieldKey),
}
pub struct Editing {
    pub id: FieldKey,
    pub editor: Editor,
    pub schema: Value,
}
pub struct Saving {
    pub project: String,
    pub extension: String,
    pub patch: serde_json::Map<String, Value>,
}
#[derive(Default)]
pub struct State {
    pub cursor: Option<Target>,
    pub cursors: BTreeMap<String, Target>,
    pub list_cursor: Option<String>,
    pub drafts: BTreeMap<FieldKey, Value>,
    pub editing: Option<Editing>,
    pub saving: BTreeMap<WriteId, Saving>,
    pub errors: BTreeMap<(String, String), Vec<FieldError>>,
    pub work_project: Option<String>,
    first_error: bool,
    loaded: bool,
    persisted: Vec<(FieldKey, Value)>,
}
#[derive(Clone)]
pub struct Field {
    pub id: FieldKey,
    pub schema: Value,
    pub value: Option<Value>,
    pub errors: Vec<String>,
    pub draft: bool,
}
impl Field {
    pub fn secret(&self) -> bool {
        self.schema["format"] == "secret"
    }
    pub fn display(&self) -> String {
        if self.secret() {
            return if self
                .value
                .as_ref()
                .is_some_and(|v| !v.is_null() && v != &Value::Bool(false))
            {
                "set · hidden"
            } else {
                "not set"
            }
            .into();
        }
        match self.value.as_ref().filter(|v| !v.is_null()) {
            Some(Value::Bool(v)) => if *v { "on" } else { "off" }.into(),
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => self
                .schema
                .get("default")
                .map(|v| format!("default {}", value_text(v)))
                .unwrap_or_else(|| "not set".into()),
        }
    }
}
pub(crate) fn value_text(v: &Value) -> String {
    v.as_str()
        .map(str::to_string)
        .unwrap_or_else(|| v.to_string())
}
pub(crate) fn shown(a: &App) -> Option<&str> {
    match &a.stage {
        Stage::Item(Item::Project(p)) => Some(p),
        _ => None,
    }
}
/// A successful write is already server truth while the follow-up options read is in flight.
pub(crate) fn options(a: &App, p: &str) -> Option<Vec<OptionsItem>> {
    let known = a.link.server().options.get(&p.into())?;
    let mut options = known.value()?.clone();
    for option in &mut options {
        let latest = a.link.outbox().entries().iter().rev().find_map(|entry| {
            if entry.call != WriteCall::SetOptions
                || entry.body["project"] != p
                || entry.body["extension"] != option.extension
            {
                return None;
            }
            if let tau_client::WriteState::Done { result, at } = &entry.state
                && Some(*at) >= known.at()
            {
                return result["value"].as_object();
            }
            None
        });
        if let Some(value) = latest {
            option.value = value.clone();
            option.errors.clear();
        }
    }
    Some(options)
}
pub(crate) fn fields(a: &App, p: &str) -> Vec<Field> {
    let Some(options) = options(a, p) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut options: Vec<_> = options.iter().collect();
    options.sort_by_key(|o| {
        !a.link
            .server()
            .extensions
            .value()
            .into_iter()
            .flatten()
            .any(|e| e.name == o.extension && e.workflow)
    });
    for o in options {
        let problems = tau_client::shapes::options_problems(o);
        for (k, schema) in o.schema["properties"].as_object().into_iter().flatten() {
            let id = FieldKey {
                project: p.into(),
                extension: o.extension.clone(),
                key: k.clone(),
            };
            let draft = a.screens.project.drafts.get(&id);
            let errors = a
                .screens
                .project
                .errors
                .get(&(p.into(), o.extension.clone()))
                .unwrap_or(&o.errors);
            let mut messages: Vec<_> = errors
                .iter()
                .filter(|e| pointer_field(&e.path) == *k)
                .map(|e| e.message.clone())
                .collect();
            if problems.missing.contains(k) && messages.is_empty() {
                messages.push("required".into());
            }
            if problems.missing.contains(k) && draft.is_none() {
                messages = vec!["required".into()];
            }
            out.push(Field {
                id,
                schema: schema.clone(),
                value: draft.cloned().or_else(|| o.value.get(k).cloned()),
                errors: messages,
                draft: draft.is_some(),
            });
        }
    }
    out
}
fn pointer_field(path: &str) -> String {
    path.trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("")
        .replace("~1", "/")
        .replace("~0", "~")
}
pub(crate) fn option_errors(o: &OptionsItem) -> String {
    let p = tau_client::shapes::options_problems(o);
    let mut parts = Vec::new();
    if !p.missing.is_empty() {
        parts.push(format!("needs options: {}", p.missing.join(", ")));
    }
    if !p.misfits.is_empty() {
        parts.push(format!("options don't fit: {}", p.misfits.join(", ")));
    }
    parts.join(" · ")
}
#[derive(Debug, PartialEq)]
pub struct Counts {
    pub conversations: Option<(usize, usize, usize)>,
    pub decisions: Option<usize>,
    pub faults: Option<usize>,
}
fn counts(a: &App, p: &str) -> Counts {
    let s = a.link.server();
    let id = p.into();
    Counts {
        conversations: s.conversations.get(&id).and_then(|v| v.value()).map(|cs| {
            (
                cs.len(),
                cs.iter().filter(|c| c.state == RunState::Running).count(),
                cs.iter().filter(|c| c.state == RunState::Failed).count(),
            )
        }),
        decisions: s
            .inbox
            .value()
            .map(|i| i.projects.get(&id).map_or(0, |x| x.len())),
        faults: s.faults.get(&id).and_then(|v| v.value()).map(Vec::len),
    }
}
pub struct Project;
impl Screen for Project {
    fn ctx(&self, a: &App) -> Ctx {
        if a.screens.project.editing.is_some() {
            Ctx::Field
        } else if shown(a).is_some() {
            Ctx::Project
        } else {
            Ctx::List
        }
    }
    fn mode(&self, a: &App) -> Mode {
        a.screens
            .project
            .editing
            .as_ref()
            .map_or(Mode::Normal, |e| start::editor_mode(&e.editor))
    }
    fn crumbs(&self, a: &App) -> Vec<Crumb> {
        let mut c = vec![Crumb::Text("Projects".into())];
        c.extend(shown(a).map(|p| Crumb::Project(p.into())));
        c
    }
    fn has_peek(&self, a: &App) -> bool {
        shown(a).is_none()
    }
    fn view(&self, t: Theme, a: &App) -> El {
        view::view(t, a)
    }
    fn room(&self, a: &App) -> Room {
        let p = &a.screens.project;
        Room::Lift(if shown(a).is_none() {
            p.list_cursor
                .as_ref()
                .map(|name| format!("project-row-{name}"))
        } else {
            p.cursor.as_ref().map(|c| match c {
                Target::Field(f) => format!("option-{}-{}", f.extension, f.key),
                Target::Work(place) => format!("work-{place:?}"),
            })
        })
    }
    fn save(&self, t: Theme, a: &App) -> Option<El> {
        view::save(t, a)
    }
    fn globals(&self) -> Vec<(&'static str, Handler)> {
        vec![
            ("project.bind", |a, _, _| bind(a)),
            ("project.add", |a, _, _| actions::choose_project(a, true)),
            ("project.options", |a, _, _| {
                actions::choose_project(a, false)
            }),
        ]
    }
    fn commands(&self) -> Vec<(&'static str, Handler)> {
        vec![
            ("field.edit", actions::edit),
            ("field.reset", actions::reset),
            ("options.save", actions::save),
            ("field.commit", actions::commit),
            ("list.open", actions::open),
            ("picker.keys", project_picker::keys),
            ("bind.keys", actions::bind_keys),
            ("peek", actions::schema),
            ("float.close", |a, _, _| {
                a.screens.project.editing = None;
            }),
            ("reply.keys", |a, _, _| {
                a.close_float();
            }),
        ]
    }
    fn shown(&self, a: &mut App) {
        if !a.screens.project.loaded {
            let saved: Vec<(FieldKey, Value)> = a
                .link
                .store()
                .load("project-options.json")
                .unwrap_or_default();
            a.screens.project.drafts = saved.iter().cloned().collect();
            a.screens.project.persisted = saved;
            a.screens.project.loaded = true;
            a.screens.project.list_cursor = a.link.store().load("projects-cursor.json");
        }
        a.link.refresh_catalogue();
        if let Some(p) = shown(a).map(str::to_owned) {
            a.screens.project.cursor = a.screens.project.cursors.get(&p).cloned();
            a.link.fetch_options(p.as_str().into());
        }
        actions::keep_cursor(a);
    }
    fn keys(&self, a: &mut App, keys: &[Key], count: Option<u32>) -> bool {
        if !a.floats.is_empty() {
            if let Some(f) = super::top_float::<crate::floats::project_bind::Bind>(a) {
                if f.part != crate::floats::project_bind::Part::Extensions {
                    return false;
                }
                if let [k] = keys {
                    actions::checklist_key(a, *k);
                }
                return true;
            }
            return project_picker::typing(a);
        }
        if a.screens.project.editing.is_some()
            && let [key] = keys
            && key.code == Code::Tab
        {
            let back = key.mods.shift;
            crate::app::commands::run_id(a, "field.commit", &mut Tx::default());
            if a.screens.project.editing.is_none() {
                actions::motion(a, &[Key::char(if back { 'k' } else { 'j' })], 1);
            }
            return true;
        }
        if let Some(e) = a.screens.project.editing.as_mut() {
            if e.editor.mode() == EditMode::Insert {
                return keys
                    .iter()
                    .all(|k| e.editor.insert(*k, a.now) == Reply::Handled);
            }
            if keys == [Key::plain(Code::Esc)] || keys == [Key::char('q')] {
                a.screens.project.editing = None;
                return true;
            }
            for k in keys {
                e.editor.normal(count, *k);
            }
            return true;
        }
        actions::motion(a, keys, count.unwrap_or(1));
        true
    }
    fn poll(&self, a: &mut App, now: f64, tx: &mut Tx) -> bool {
        let mut changed = false;
        if let Some(e) = &mut a.screens.project.editing {
            if let Some(d) = e.editor.jk_deadline() {
                tx.wake_after(std::time::Duration::from_secs_f64((d - now).max(0.0)));
            }
            changed |= e.editor.tick(now);
        }
        if super::of(&a.stage) == super::ScreenId::Project {
            changed |= actions::keep_cursor(a);
        }
        changed
    }
    fn on_focus(&self, a: &mut App, focused: bool) {
        if focused {
            return;
        }
        if a.screens.project.loaded {
            let saved: Vec<_> = a
                .screens
                .project
                .drafts
                .iter()
                .filter(|(id, _)| {
                    !fields(a, &id.project)
                        .iter()
                        .any(|f| f.id == **id && f.secret())
                })
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            if saved != a.screens.project.persisted {
                match a.link.store().save("project-options.json", &saved) {
                    Ok(()) => a.screens.project.persisted = saved,
                    Err(e) => a.say(format!("Could not keep option drafts on this device: {e}")),
                }
            }
        }
    }
    fn on_event(&self, a: &mut App, event: &LinkEvent) {
        if let LinkEvent::Write {
            id,
            call: WriteCall::SetOptions,
            state,
        } = event
            && let Some(sent) = a.screens.project.saving.remove(id)
        {
            match state {
                WriteState::Done { .. } => {
                    for (key, value) in sent.patch {
                        let id = FieldKey {
                            project: sent.project.clone(),
                            extension: sent.extension.clone(),
                            key,
                        };
                        if a.screens.project.drafts.get(&id) == Some(&value) {
                            a.screens.project.drafts.remove(&id);
                        }
                    }
                    a.screens
                        .project
                        .errors
                        .remove(&(sent.project, sent.extension));
                }
                WriteState::Refused(r) => {
                    a.screens
                        .project
                        .errors
                        .insert((sent.project, sent.extension), r.errors());
                }
                WriteState::Unknown { .. } => {
                    a.screens.project.saving.insert(*id, sent);
                }
                _ => {}
            }
        }
    }
    fn scenes(&self) -> Vec<crate::app::scenes::Scene> {
        scenes::all()
    }
}
