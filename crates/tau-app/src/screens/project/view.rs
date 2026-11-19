use super::*;
use crate::{app::Focus, assets::carbon, content::line, frame::SidebarPlace, ui};
use rune::{col, icon, row, text, text_input};

pub(super) fn save(t: Theme, a: &App) -> Option<El> {
    let p = shown(a)?;
    let n = a
        .screens
        .project
        .drafts
        .keys()
        .filter(|id| id.project == p)
        .count();
    if n == 0 {
        return None;
    }
    Some(not_saved(t, n).child(save_key(t, a)).mr(14.0))
}
/// The sage `edit` mark and how many drafts are not saved.
fn not_saved(t: Theme, n: usize) -> El {
    row()
        .center_y()
        .gap(7.0)
        .child(icon(&carbon::EDIT, 12.0).color(t.c.state.local))
        .child(
            text(format!("{n} not saved"))
                .font(t.ty.ui_small)
                .color(t.c.fg.alt),
        )
}
/// The `:w` chip, lit while the save resolves where the keys are.
fn save_key(t: Theme, a: &App) -> El {
    ui::glyphs(
        ":w",
        t.ty.key,
        if a.scope().lit(t.keymap, "<leader>ms") {
            t.c.kind.literal
        } else {
            t.c.fg.muted
        },
    )
    .h(18.0)
    .px(5.0)
    .radius(t.l.radius.pill)
    .fill(t.c.surface.bar)
}
/// An Options group's head; with drafts in the group, `✎ 1 not saved :w save` at its right,
/// which saves as `:w` does.
fn options_head(t: Theme, a: &App, p: &str, extension: &str) -> El {
    let n = a
        .screens
        .project
        .drafts
        .keys()
        .filter(|id| id.project == p && id.extension == extension)
        .count();
    let mut r = row()
        .center_y()
        .pl(40.0)
        .pr(12.0)
        .mt(26.0)
        .mb(4.0)
        .child(ui::group_head(t, &format!("Options · {extension}"), None));
    if n > 0 {
        r = r.child(rune::spacer()).child(
            not_saved(t, n)
                .child(save_key(t, a))
                .child(text("save").font(t.ty.ui_small).color(t.c.fg.alt))
                .key(format!("options-save-{extension}"))
                .on_press_tx(|a: &mut App, tx| crate::app::commands::run_id(a, "options.save", tx)),
        );
    }
    r
}
fn selected(t: Theme, a: &App, target: &Target, body: El) -> El {
    if a.screens.project.cursor.as_ref() != Some(target) {
        return body;
    }
    body.fill(t.c.surface.cursor_read).border_sides(
        [0.0, 0.0, 0.0, 2.0],
        if a.focus == Focus::Stage && a.floats.is_empty() {
            t.c.state.attn
        } else {
            t.c.surface.cursor_read
        },
    )
}
fn head(t: Theme, title: &str, first: bool) -> El {
    ui::group_head(t, title, None)
        .pl(40.0)
        .mt(if first { 0.0 } else { 26.0 })
        .mb(4.0)
}
fn work_row(t: Theme, a: &App, label: &str, value: String, place: Place) -> El {
    let r = row()
        .center_y()
        .h(30.0)
        .pl(40.0)
        .pr(12.0)
        .child(text(label.to_string()).font(t.ty.ui).color(t.c.fg.default))
        .child(rune::spacer())
        .child(text(value).font(t.ty.ui_small).color(t.c.fg.alt));
    selected(t, a, &Target::Work(place), r)
        .key(format!("work-{place:?}"))
        .on_press(move |a: &mut App| {
            a.screens.project.cursor = Some(Target::Work(place));
        })
        .on_double_press_tx(|a: &mut App, tx| crate::app::commands::run_id(a, "list.open", tx))
}
pub(super) fn view(t: Theme, a: &App) -> El {
    let Some(p) = shown(a) else {
        return projects(t, a);
    };
    let Some(project) = a
        .link
        .server()
        .projects
        .value()
        .and_then(|ps| ps.iter().find(|x| x.project.as_str() == p))
    else {
        return super::super::stub(t, p, super::super::NOTHING_RECEIVED);
    };
    let c = counts(a, p);
    let stage_w = a.window[0]
        - if a.sidebar_place() == Some(SidebarPlace::Beside) {
            t.l.sidebar.width
        } else {
            0.0
        };
    let left = if stage_w >= 1000.0 {
        (stage_w - 968.0) / 2.0
    } else {
        ((stage_w - 600.0) / 2.0).max(40.0)
    };
    let mut doc = col()
        .w(640.0)
        .pt(28.0)
        .pb(32.0)
        .child(line(t, p, t.ty.title_page, t.c.state.attn).pl(40.0))
        .child(
            row()
                .center_y()
                .gap(6.0)
                .pl(40.0)
                .mt(4.0)
                .mb(24.0)
                .child(
                    text("working directory")
                        .font(t.ty.ui_small)
                        .color(t.c.fg.alt),
                )
                .child(line(t, &project.workdir, t.ty.mono_small, t.c.fg.default))
                .child(
                    text("· a path on the server")
                        .font(t.ty.ui_small)
                        .color(t.c.fg.alt),
                ),
        )
        .child(head(t, "Work", true));
    let conv = c
        .conversations
        .map(|(all, running, failed)| {
            let mut s = all.to_string();
            if running > 0 {
                s.push_str(&format!(" · {running} running"));
            }
            if failed > 0 {
                s.push_str(&format!(" · {failed} failed"));
            }
            s
        })
        .unwrap_or_else(|| "not received".into());
    doc = doc
        .child(work_row(t, a, "Conversations", conv, Place::Conversations))
        .child(work_row(
            t,
            a,
            "Open decisions",
            c.decisions
                .map(|n| n.to_string())
                .unwrap_or_else(|| "not received".into()),
            Place::Home,
        ))
        .child(work_row(
            t,
            a,
            "Faults",
            c.faults
                .map(|n| n.to_string())
                .unwrap_or_else(|| "not received".into()),
            Place::Faults,
        ))
        .child(head(t, "Extensions", false));
    let known_options = options(a, p);
    for name in &project.extensions {
        let ext = a
            .link
            .server()
            .extensions
            .value()
            .into_iter()
            .flatten()
            .find(|e| &e.name == name);
        let opts = known_options
            .as_ref()
            .and_then(|os| os.iter().find(|o| &o.extension == name));
        let missing = project.missing.contains(name);
        let status = if missing {
            "missing on the server · its tasks wait and its tools are not offered".into()
        } else if let Some(o) = opts {
            let e = tau_client::shapes::options_problems(o);
            let mut parts = Vec::new();
            if !e.missing.is_empty() {
                parts.push(format!("{} to set", e.missing.len()));
            }
            if !e.misfits.is_empty() {
                parts.push(format!("{} doesn't fit", e.misfits.len()));
            }
            if parts.is_empty() {
                "all set".into()
            } else {
                parts.join(" · ")
            }
        } else {
            "not received".into()
        };
        doc = doc.child(
            row()
                .center_y()
                .h(30.0)
                .pl(40.0)
                .pr(12.0)
                .gap(14.0)
                .child(line(t, name, t.ty.mono, t.c.fg.default).w(120.0))
                .child(
                    text(ext.map_or("", |e| if e.workflow { "workflow" } else { "tool" }))
                        .font(t.ty.ui)
                        .color(t.c.fg.alt),
                )
                .child(rune::spacer())
                .child(line(
                    t,
                    &status,
                    t.ty.ui_small,
                    if missing || opts.is_some_and(|o| !o.errors.is_empty()) {
                        t.c.text.failed
                    } else {
                        t.c.fg.alt
                    },
                )),
        );
    }
    doc = doc.child(
        text("An extension can't be removed from a project.")
            .font(t.ty.prose_small)
            .color(t.c.fg.alt)
            .pl(40.0)
            .mt(8.0),
    );
    let mut extension = String::new();
    for f in fields(a, p) {
        if extension != f.id.extension {
            extension = f.id.extension.clone();
            doc = doc.child(options_head(t, a, p, &extension));
        }
        doc = doc.child(field(t, a, &f));
    }
    col()
        .w_full()
        .h_full()
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        })
        .child(doc.ml((left - 40.0).max(0.0)))
}
fn field(t: Theme, a: &App, f: &Field) -> El {
    let focused = a.focus == Focus::Stage && a.floats.is_empty();
    let editing = a.screens.project.editing.as_ref().filter(|e| e.id == f.id);
    let mut body = row()
        .center_y()
        .gap(14.0)
        .child(line(t, &f.id.key, t.ty.mono, t.c.fg.default).w(120.0));
    let value = if let Some(e) = editing {
        let mut edit = e.editor.edit.clone();
        edit.focused = focused;
        text_input(&edit, t.ty.mono)
            .single_line()
            .caret(t.c.state.attn, t.c.surface.selection)
            .color(t.c.fg.default)
            .on_edit(|a: &mut App| {
                &mut a
                    .screens
                    .project
                    .editing
                    .as_mut()
                    .expect("option editor")
                    .editor
                    .edit
            })
            .on_input_key(crate::app::input::on_field_key)
            .label(f.id.key.clone())
            .key("option-editor")
            .w_full()
    } else {
        let default = f.value.as_ref().is_none_or(|v| v.is_null());
        let mut value = row().center_y().gap(8.0).child(line(
            t,
            &f.display(),
            if default { t.ty.ui } else { t.ty.mono },
            if default {
                t.c.fg.muted
            } else {
                t.c.fg.default
            },
        ));
        if f.schema["format"] == "model"
            && let Some(name) = a
                .link
                .server()
                .models
                .value()
                .into_iter()
                .flatten()
                .find(|m| Some(m.model.as_str()) == f.value.as_ref().and_then(Value::as_str))
                .map(|m| &m.name)
        {
            value = value.child(line(t, name, t.ty.ui_small, t.c.fg.alt));
        }
        value
    };
    body = body.child(value.grow(1.0).min_w(0.0));
    let mut r = col()
        .min_h(30.0)
        .justify_center()
        .pl(40.0)
        .pr(12.0)
        .child(body);
    if !f.errors.is_empty() {
        r = r.py(6.0);
        for error in &f.errors {
            r = r.child(line(t, error, t.ty.ui_small, t.c.text.failed).pl(134.0));
        }
    }
    if f.draft {
        r = r.child(
            icon(&carbon::EDIT, 12.0)
                .color(t.c.state.local)
                .pin(Some(14.0), Some(7.0), None, None)
                .key("draft-mark"),
        );
    }
    let id = f.id.clone();
    selected(t, a, &Target::Field(f.id.clone()), r)
        .key(format!("option-{}-{}", f.id.extension, f.id.key))
        .on_press(move |a: &mut App| {
            a.screens.project.cursor = Some(Target::Field(id.clone()));
        })
        .on_double_press_tx(|a: &mut App, tx| crate::app::commands::run_id(a, "field.edit", tx))
}
fn projects(t: Theme, a: &App) -> El {
    let mut body = col()
        .w_full()
        .h_full()
        .pl(t.l.list_column.left)
        .pr(t.l.list_column.right_gap)
        .pt(28.0)
        .child(
            text("Projects")
                .font(t.ty.title_page)
                .color(t.c.state.attn)
                .mb(24.0),
        );
    match a.link.server().projects.value() {
        None => {
            body = body.child(
                text(super::super::NOTHING_RECEIVED)
                    .font(t.ty.ui)
                    .color(t.c.fg.alt),
            )
        }
        Some(ps) if ps.is_empty() => {
            body = body.child(crate::screens::home::no_projects(t, a.scope()))
        }
        Some(ps) => {
            for p in ps {
                let name = p.project.to_string();
                let c = counts(a, &name);
                let mut r = row()
                    .center_y()
                    .h(28.0)
                    .gap(12.0)
                    .px(8.0)
                    .child(line(t, &name, t.ty.ui, t.c.kind.worker).w(130.0))
                    .child(
                        line(
                            t,
                            &format!("…/{}", p.workdir.rsplit('/').next().unwrap_or(&p.workdir)),
                            t.ty.mono,
                            t.c.fg.alt,
                        )
                        .w(170.0),
                    )
                    .child(line(
                        t,
                        &p.extensions.join(" · "),
                        t.ty.mono_small,
                        t.c.fg.alt,
                    ));
                if let Some(n) = c.decisions.filter(|n| *n > 0) {
                    r = r.child(text(format!("◆ {n}")).font(t.ty.ui_small).color(t.c.fg.alt));
                }
                if !p.missing.is_empty() {
                    r = r.child(
                        text(format!("{} missing", p.missing.len()))
                            .font(t.ty.ui_small)
                            .color(t.c.text.failed),
                    );
                }
                if Some(&name) == a.screens.project.list_cursor.as_ref() {
                    r = r.fill(t.c.surface.cursor_read).border_sides(
                        [0.0, 0.0, 0.0, 2.0],
                        if a.focus == Focus::Stage && a.floats.is_empty() {
                            t.c.state.attn
                        } else {
                            t.c.surface.cursor_read
                        },
                    );
                }
                body = body.child(
                    r.key(format!("project-row-{name}"))
                        .on_press(move |a: &mut App| {
                            a.screens.project.list_cursor = Some(name.clone())
                        })
                        .on_double_press_tx(|a: &mut App, tx| {
                            crate::app::commands::run_id(a, "list.open", tx)
                        }),
                );
            }
        }
    }
    body
}
