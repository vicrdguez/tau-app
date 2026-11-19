use super::*;
use crate::{
    app::Invocation,
    copy,
    editor::Lines,
    floats::{
        Confirm,
        project_bind::{Bind, Extension, Part},
        project_picker::{Choice, Picker},
    },
    screens::{ScreenFloat, ScreenId, top_float},
    ui,
};
use tau_client::Write;

pub(crate) fn open_options(a: &mut App, p: String, first: bool) {
    a.screens.project.first_error = first;
    a.show(Stage::Item(Item::Project(p)));
}
pub(super) fn choose_project(a: &mut App, adding: bool) {
    if let Some(p) = start::named_project(a) {
        if adding {
            add(a, p)
        } else {
            open_options(a, p, false)
        }
    } else {
        start::project_picker(
            a,
            if adding {
                start::ProjectPurpose::Add
            } else {
                start::ProjectPurpose::Options
            },
        );
    }
}
pub(crate) fn bind(a: &mut App) {
    a.link.refresh_catalogue();
    let es = extensions(a, None);
    a.open_float(Float::Screen(Box::new(Bind::new(None, es))));
}
pub(crate) fn add(a: &mut App, p: String) {
    a.link.refresh_catalogue();
    let es = extensions(a, Some(&p));
    a.open_float(Float::Screen(Box::new(Bind::new(Some(p), es))));
}
fn extensions(a: &App, p: Option<&str>) -> Vec<Extension> {
    let bound = p.and_then(|p| {
        a.link
            .server()
            .projects
            .value()?
            .iter()
            .find(|x| x.project.as_str() == p)
    });
    a.link
        .server()
        .extensions
        .value()
        .into_iter()
        .flatten()
        .filter(|e| !bound.is_some_and(|p| p.extensions.contains(&e.name)))
        .map(|e| Extension {
            name: e.name.clone(),
            workflow: e.workflow,
            checked: false,
        })
        .collect()
}
pub(super) fn bind_keys(a: &mut App, inv: &Invocation, _: &mut Tx) {
    let Some(k) = inv.keys.last().copied() else {
        return;
    };
    let now = a.now;
    let Some(f) = top_float::<Bind>(a) else {
        return;
    };
    if f.part == Part::Extensions {
        checklist_key(a, k);
        return;
    }
    match k.code {
        Code::Esc => f.part = Part::Extensions,
        Code::Tab => next_part(f, k.mods.shift),
        Code::Enter => next_part(f, false),
        Code::Backspace => {
            f.editor().edit.key(
                &rune::KeyEvent {
                    key: rune::Key::Backspace,
                    mods: rune::Mods::default(),
                },
                None,
            );
        }
        _ => {
            f.editor().insert(k, now);
        }
    }
}
/// `Tab` and `S-Tab`: the next or previous of the float's regions, wrapping. Bind has the name,
/// the working directory and the checklist; Add extensions only the checklist.
fn next_part(f: &mut Bind, back: bool) {
    let parts: &[Part] = if f.project.is_some() {
        &[Part::Extensions]
    } else {
        &[Part::Name, Part::Directory, Part::Extensions]
    };
    let at = parts.iter().position(|p| *p == f.part).unwrap_or(0);
    let by = if back { parts.len() - 1 } else { 1 };
    f.part = parts[(at + by) % parts.len()];
}

/// A key in the checklist: the cursor moves, `Space` ticks its row, `⏎` goes on to Confirm.
pub(super) fn checklist_key(a: &mut App, k: Key) {
    let Some(f) = top_float::<Bind>(a) else {
        return;
    };
    let down = [
        Key::char('j'),
        Key::ctrl('j'),
        Key::ctrl('n'),
        Key::plain(Code::Down),
    ];
    let up = [
        Key::char('k'),
        Key::ctrl('k'),
        Key::ctrl('p'),
        Key::plain(Code::Up),
    ];
    if k.code == Code::Esc || k == Key::ctrl('c') {
        a.close_float();
    } else if k.code == Code::Tab {
        next_part(f, k.mods.shift);
    } else if k == Key::char(' ') {
        if let Some(e) = f.extensions.get_mut(f.cursor) {
            e.checked = !e.checked;
        }
    } else if down.contains(&k) {
        f.cursor = (f.cursor + 1).min(f.extensions.len().saturating_sub(1));
    } else if up.contains(&k) {
        f.cursor = f.cursor.saturating_sub(1);
    } else if k.code == Code::Enter {
        confirm(a);
    }
}
fn confirm(a: &mut App) {
    let f = top_float::<Bind>(a).unwrap();
    let extensions = f.selected();
    let (write, sentence, yes) = if let Some(p) = &f.project {
        if extensions.is_empty() {
            return;
        }
        (
            Write::AddExtensions {
                project: p.as_str().into(),
                extensions: extensions.clone(),
            },
            format!(
                "Add {} to {p}? An extension can't be removed from a project.",
                extensions.join(", ")
            ),
            "adds",
        )
    } else {
        let p = f.name.text().trim();
        let dir = f.directory.text().trim();
        let valid = !p.is_empty()
            && p.len() <= 63
            && p.bytes()
                .enumerate()
                .all(|(i, b)| b.is_ascii_lowercase() || b.is_ascii_digit() || (i > 0 && b == b'-'));
        if !valid {
            f.error = Some(
                "Use 1–63 lowercase letters, digits or hyphens; begin with a letter or digit."
                    .into(),
            );
            f.part = Part::Name;
            return;
        }
        if !dir.starts_with('/') {
            f.error = Some("Use an absolute path on the server.".into());
            f.part = Part::Directory;
            return;
        }
        let sentence = if extensions.is_empty() {
            format!(
                "Bind {p} with no extensions? Its conversations get the coding tools only. A project can't be unbound; extensions can be added later but never removed."
            )
        } else {
            format!(
                "Bind {p} with {}? A project can't be unbound and an extension can't be removed.",
                extensions.join(", ")
            )
        };
        (
            Write::Bind {
                project: p.into(),
                workdir: dir.into(),
                extensions,
            },
            sentence,
            "binds",
        )
    };
    a.open_float(Float::Confirm(Confirm {
        sentence,
        yes,
        on_yes: Some(Box::new(move |a, _| match a.link.write(write) {
            Ok(_) => {
                a.close_float();
            }
            Err(e) => {
                if let Some(s) = copy::project_not_sent(&e, false) {
                    a.say(s);
                }
            }
        })),
    }));
}
fn targets(a: &App, p: &str) -> Vec<Target> {
    let mut out = vec![
        Target::Work(Place::Conversations),
        Target::Work(Place::Home),
        Target::Work(Place::Faults),
    ];
    out.extend(fields(a, p).into_iter().map(|f| Target::Field(f.id)));
    out
}
pub(super) fn keep_cursor(a: &mut App) -> bool {
    if let Some(p) = shown(a).map(str::to_owned) {
        if matches!(a.screens.project.cursor, Some(Target::Field(_)))
            && a.link
                .server()
                .options
                .get(&tau_client::Project::new(&p))
                .is_none_or(|v| matches!(v, tau_client::Known::NotYet))
        {
            return false;
        }
        if a.screens.project.first_error
            && let Some(f) = fields(a, &p).into_iter().find(|f| !f.errors.is_empty())
        {
            a.screens.project.cursor = Some(Target::Field(f.id));
            a.screens.project.first_error = false;
            return true;
        }
        let ts = targets(a, &p);
        if !a
            .screens
            .project
            .cursor
            .as_ref()
            .is_some_and(|c| ts.contains(c))
        {
            a.screens.project.cursor = ts.first().cloned();
            return true;
        }
    } else if let Some(ps) = a.link.server().projects.value()
        && !a
            .screens
            .project
            .list_cursor
            .as_ref()
            .is_some_and(|p| ps.iter().any(|x| x.project.as_str() == p))
    {
        a.screens.project.list_cursor = ps.first().map(|p| p.project.to_string());
        return true;
    }
    false
}
pub(super) fn motion(a: &mut App, keys: &[Key], count: u32) {
    let delta = match keys {
        [k] if *k == Key::char('j') => count as isize,
        [k] if *k == Key::char('k') => -(count as isize),
        [k] if *k == Key::char('G') => isize::MAX,
        [g, h] if *g == Key::char('g') && *h == *g => isize::MIN,
        _ => return,
    };
    if let Some(p) = shown(a) {
        let ts = targets(a, p);
        let at = ts
            .iter()
            .position(|x| Some(x) == a.screens.project.cursor.as_ref())
            .unwrap_or(0);
        let to = (at as isize)
            .saturating_add(delta)
            .clamp(0, ts.len().saturating_sub(1) as isize) as usize;
        a.screens.project.cursor = ts.get(to).cloned();
    } else if let Some(ps) = a.link.server().projects.value() {
        let at = ps
            .iter()
            .position(|p| Some(p.project.as_str()) == a.screens.project.list_cursor.as_deref())
            .unwrap_or(0);
        let to = (at as isize)
            .saturating_add(delta)
            .clamp(0, ps.len().saturating_sub(1) as isize) as usize;
        a.screens.project.list_cursor = ps.get(to).map(|p| p.project.to_string());
        let _ = a
            .link
            .store()
            .save("projects-cursor.json", &a.screens.project.list_cursor);
    }
}
pub(super) fn open(a: &mut App, inv: &Invocation, tx: &mut Tx) {
    if !inv.applies {
        return;
    }
    if shown(a).is_none() {
        if let Some(p) = a.screens.project.list_cursor.clone() {
            open_options(a, p, false);
        }
        return;
    }
    match a.screens.project.cursor.clone() {
        Some(Target::Field(_)) => edit(a, inv, tx),
        Some(Target::Work(p)) => {
            a.screens.project.work_project = shown(a).map(str::to_owned);
            a.show(Stage::Place(p));
        }
        None => {}
    }
}
fn selected(a: &App) -> Option<Field> {
    let Target::Field(id) = a.screens.project.cursor.as_ref()? else {
        return None;
    };
    fields(a, &id.project).into_iter().find(|f| &f.id == id)
}
pub(super) fn edit(a: &mut App, inv: &Invocation, _: &mut Tx) {
    if !inv.applies {
        return;
    }
    let Some(f) = selected(a) else {
        return;
    };
    if f.schema["type"] == "boolean"
        && !f.schema["enum"].is_array()
        && f.schema["format"] != "model"
        && !f.secret()
    {
        a.screens.project.drafts.insert(
            f.id,
            Value::Bool(!f.value.and_then(|v| v.as_bool()).unwrap_or(false)),
        );
        return;
    }
    let choices = if f.schema["format"] == "model" {
        a.link.refresh_catalogue();
        Some(
            a.link
                .server()
                .models
                .value()
                .into_iter()
                .flatten()
                .map(|m| Choice::new(m.model.clone(), m.name.clone(), m.model.clone()))
                .collect::<Vec<_>>(),
        )
    } else {
        f.schema["enum"].as_array().map(|es| {
            es.iter()
                .map(|v| Choice::new(v.to_string(), value_text(v), ""))
                .collect()
        })
    };
    if let Some(rows) = choices {
        let model = f.schema["format"] == "model";
        let id = f.id.clone();
        let selected = f
            .value
            .map(|v| if model { value_text(&v) } else { v.to_string() });
        a.open_float(Float::Screen(Box::new(Picker::new(
            ScreenId::Project,
            if model { "Model" } else { "Choice" },
            format!("for {} · {}", id.extension, id.key),
            "choices",
            rows,
            selected,
            Box::new(move |a, v| {
                let value = if model {
                    Value::String(v)
                } else {
                    serde_json::from_str(&v).unwrap_or(Value::Null)
                };
                a.screens.project.drafts.insert(id, value);
            }),
        ))));
        return;
    }
    if f.secret()
        || matches!(
            f.schema["type"].as_str(),
            Some("string" | "number" | "integer")
        )
    {
        let value = if f.secret() {
            String::new()
        } else {
            f.value
                .as_ref()
                .filter(|v| !v.is_null())
                .or_else(|| f.schema.get("default"))
                .map(value_text)
                .unwrap_or_default()
        };
        let mut editor = Editor::inserting(&value, Lines::Single);
        editor.edit.focused = true;
        editor.edit.set_secure(f.secret());
        a.screens.project.editing = Some(Editing {
            id: f.id,
            editor,
            schema: f.schema,
        });
    }
}
pub(super) fn reset(a: &mut App, inv: &Invocation, _: &mut Tx) {
    if inv.applies
        && let Some(f) = selected(a)
    {
        a.screens.project.drafts.insert(f.id, Value::Null);
    }
}
pub(super) fn commit(a: &mut App, _: &Invocation, _: &mut Tx) {
    let Some(e) = &a.screens.project.editing else {
        return;
    };
    let text = e.editor.text();
    let value = match e.schema["type"].as_str() {
        Some("integer") => match text.parse::<i64>() {
            Ok(n) => Value::from(n),
            Err(_) => {
                a.say("Enter an integer");
                return;
            }
        },
        Some("number") => match text
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .and_then(serde_json::Number::from_f64)
        {
            Some(n) => Value::Number(n),
            None => {
                a.say("Enter a finite number");
                return;
            }
        },
        _ => Value::String(text.into()),
    };
    let id = e.id.clone();
    a.screens.project.drafts.insert(id, value);
    a.screens.project.editing = None;
}
pub(super) fn save(a: &mut App, inv: &Invocation, _: &mut Tx) {
    if !inv.applies {
        return;
    }
    let Some(p) = shown(a).map(str::to_owned) else {
        return;
    };
    let mut patches: BTreeMap<String, serde_json::Map<String, Value>> = BTreeMap::new();
    for (id, value) in &a.screens.project.drafts {
        if id.project == p {
            patches
                .entry(id.extension.clone())
                .or_default()
                .insert(id.key.clone(), value.clone());
        }
    }
    for (extension, patch) in patches {
        if a.screens
            .project
            .saving
            .values()
            .any(|s| s.project == p && s.extension == extension)
        {
            continue;
        }
        match a.link.write(Write::SetOptions {
            project: p.as_str().into(),
            extension: extension.clone(),
            patch: patch.clone(),
        }) {
            Ok(id) => {
                a.screens.project.saving.insert(
                    id,
                    Saving {
                        project: p.clone(),
                        extension,
                        patch,
                    },
                );
            }
            Err(e) => {
                if let Some(s) = copy::project_not_sent(&e, false) {
                    a.say(s);
                }
            }
        }
    }
}
pub(super) fn schema(a: &mut App, _: &Invocation, _: &mut Tx) {
    if let Some(f) = selected(a) {
        a.open_float(Float::Screen(Box::new(Schema(f))));
    }
}
struct Schema(Field);
impl ScreenFloat for Schema {
    fn owner(&self) -> ScreenId {
        ScreenId::Project
    }
    fn scope(&self) -> ui::Scope {
        ui::Scope {
            ctx: Ctx::Reply,
            mode: Mode::Readback,
        }
    }
    fn badge(&self) -> ui::Badge {
        ui::Badge::ReadbackPlain
    }
    fn view(&self, t: Theme, _: &App) -> El {
        ui::shell(
            t,
            ui::Shell {
                tag: None,
                title: ui::title(t, &format!("Schema · {}", self.0.id.key)),
                subtitle: Some(self.0.id.extension.clone()),
                meta: None,
                body: rune::text(serde_json::to_string_pretty(&self.0.schema).unwrap_or_default())
                    .font(t.ty.mono)
                    .color(t.c.fg.default)
                    .wrap()
                    .px(24.0),
                input: None,
                hints: vec![ui::float_hint(t, self.scope(), &["<Esc>"], "closes")],
            },
        )
    }
}
