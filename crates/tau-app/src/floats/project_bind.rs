//! Bind's two Insert fields and the Command extension checklist. No server calls.
use crate::{
    app::App,
    assets::carbon,
    content::line,
    editor::{Editor, Lines},
    keys::{Ctx, Mode},
    screens::{ScreenFloat, ScreenId},
    theme::Theme,
    ui::{self, Badge, Scope, Shell},
};
use rune::{El, TextEdit, col, icon, row, text, text_input};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Name,
    Directory,
    Extensions,
}
pub struct Extension {
    pub name: String,
    pub workflow: bool,
    pub checked: bool,
}
pub struct Bind {
    pub project: Option<String>,
    pub name: Editor,
    pub directory: Editor,
    pub part: Part,
    pub extensions: Vec<Extension>,
    pub cursor: usize,
    pub error: Option<String>,
}
impl Bind {
    pub fn new(project: Option<String>, extensions: Vec<Extension>) -> Self {
        let part = if project.is_some() {
            Part::Extensions
        } else {
            Part::Name
        };
        Self {
            project,
            name: Editor::inserting("", Lines::Single),
            directory: Editor::inserting("", Lines::Single),
            part,
            extensions,
            cursor: 0,
            error: None,
        }
    }
    pub fn editor(&mut self) -> &mut Editor {
        if self.part == Part::Name {
            &mut self.name
        } else {
            &mut self.directory
        }
    }
    pub fn selected(&self) -> Vec<String> {
        self.extensions
            .iter()
            .filter(|e| e.checked)
            .map(|e| e.name.clone())
            .collect()
    }
}
impl ScreenFloat for Bind {
    fn owner(&self) -> ScreenId {
        ScreenId::Project
    }
    fn scope(&self) -> Scope {
        if self.part == Part::Extensions {
            Scope {
                ctx: Ctx::Bind,
                mode: Mode::Command,
            }
        } else {
            Scope {
                ctx: Ctx::Bind,
                mode: Mode::Insert,
            }
        }
    }
    fn badge(&self) -> Badge {
        if self.part == Part::Extensions {
            Badge::Command
        } else {
            Badge::Insert
        }
    }
    fn edit_mut(&mut self) -> Option<&mut TextEdit> {
        if self.part == Part::Extensions {
            None
        } else {
            Some(&mut self.editor().edit)
        }
    }
    fn view(&self, t: Theme, a: &App) -> El {
        let active = matches!(a.floats.last(), Some((crate::floats::Float::Screen(_), _)));
        let mut body = col().w_full().gap(8.0);
        if self.project.is_none() {
            for (label, part, ed) in [
                ("name", Part::Name, &self.name),
                ("working directory", Part::Directory, &self.directory),
            ] {
                let mut edit = ed.edit.clone();
                edit.focused = active && self.part == part;
                let input = text_input(&edit, t.ty.float_input)
                    .single_line()
                    .caret(t.c.state.attn, t.c.surface.selection)
                    .color(t.c.fg.default)
                    .on_edit(move |a: &mut App| {
                        let f = crate::screens::top_float::<Bind>(a).expect("bind field");
                        if part == Part::Name {
                            &mut f.name.edit
                        } else {
                            &mut f.directory.edit
                        }
                    })
                    .on_input_key(crate::app::input::on_field_key)
                    .label(label)
                    .key(format!("bind-{label}"));
                let mut value = row().center_y().gap(12.0).child(
                    ui::text_field(t, edit.focused, input)
                        .w(if part == Part::Name { 220.0 } else { 380.0 })
                        .on_press(move |a: &mut App| {
                            if let Some(f) = crate::screens::top_float::<Bind>(a) {
                                f.part = part;
                            }
                        }),
                );
                if part == Part::Directory {
                    value = value.child(
                        text("a path on the server")
                            .font(t.ty.ui_small)
                            .color(t.c.fg.muted),
                    );
                }
                let field = ui::kv_row(t, label, value);
                body = body.child(if edit.focused {
                    field.key(ui::FLOAT_CURSOR)
                } else {
                    field
                });
            }
        }
        let mut checklist = col().w_full();
        for (i, e) in self.extensions.iter().enumerate() {
            let content = row()
                .center_y()
                .gap(10.0)
                .child(
                    icon(
                        if e.checked {
                            &carbon::CHECKBOX_CHECKED
                        } else {
                            &carbon::CHECKBOX
                        },
                        14.0,
                    )
                    .color(t.c.fg.alt),
                )
                .child(line(t, &e.name, t.ty.mono, t.c.fg.default).w(84.0))
                .child(
                    text(if e.workflow { "workflow" } else { "tool" })
                        .font(t.ty.ui)
                        .color(t.c.fg.alt),
                );
            checklist = checklist.child(
                ui::float_row(
                    t,
                    active && self.part == Part::Extensions && self.cursor == i,
                    content,
                )
                .on_press(move |a: &mut App| {
                    if let Some(f) = crate::screens::top_float::<Bind>(a) {
                        f.part = Part::Extensions;
                        f.cursor = i;
                        f.extensions[i].checked = !f.extensions[i].checked;
                    }
                }),
            );
        }
        body = body.child(ui::kv_list(t, "extensions", checklist));
        if let Some(error) = &self.error {
            body = body.child(line(t, error, t.ty.ui_small, t.c.text.failed).px(24.0));
        }
        let scope = self.scope();
        let hints = if self.part == Part::Extensions {
            vec![
                ui::float_hint(t, scope, &["<Space>"], "toggle"),
                ui::float_hint(
                    t,
                    scope,
                    &["<CR>"],
                    if self.project.is_some() {
                        "add"
                    } else {
                        "bind"
                    },
                ),
                ui::float_hint(t, scope, &["<Esc>"], "cancel"),
            ]
        } else {
            vec![
                ui::float_hint(t, scope, &["<Tab>"], "next"),
                ui::float_hint(t, scope, &["<Esc>", "<Esc>"], "cancel"),
            ]
        };
        ui::shell(
            t,
            Shell {
                tag: None,
                title: ui::title(
                    t,
                    if self.project.is_some() {
                        "Add extensions"
                    } else {
                        "Bind a project"
                    },
                ),
                subtitle: self.project.as_ref().map(|p| format!("to {p}")),
                meta: None,
                body,
                input: None,
                hints,
            },
        )
    }
}
