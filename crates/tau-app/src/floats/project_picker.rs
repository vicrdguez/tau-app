//! Frozen, filterable choices for the start page and option fields.

use crate::app::{App, Invocation};
use crate::assets::carbon;
use crate::content::line;
use crate::keys::{Code, Ctx, Key, Mode};
use crate::screens::{ScreenFloat, ScreenId, top_float};
use crate::theme::Theme;
use crate::ui::{self, Badge, Scope, Shell};
use rune::{El, TextEdit, col, icon, row, text, text_input};

pub struct Choice {
    pub value: String,
    pub label: String,
    pub detail: String,
    pub path: String,
    pub note: String,
    pub project: bool,
    pub error: bool,
}
impl Choice {
    pub fn new(
        value: impl Into<String>,
        label: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            detail: detail.into(),
            path: String::new(),
            note: String::new(),
            project: false,
            error: false,
        }
    }
}
type Choose = Box<dyn FnOnce(&mut App, String)>;
pub struct Picker {
    pub owner: ScreenId,
    pub title: String,
    pub subtitle: String,
    pub noun: &'static str,
    pub rows: Vec<Choice>,
    pub query: TextEdit,
    pub selected: Option<String>,
    pub choose: Option<Choose>,
}
impl Picker {
    pub fn new(
        owner: ScreenId,
        title: &str,
        subtitle: String,
        noun: &'static str,
        rows: Vec<Choice>,
        selected: Option<String>,
        choose: Choose,
    ) -> Self {
        let mut query = TextEdit::new("");
        query.focused = true;
        Self {
            owner,
            title: title.into(),
            subtitle,
            noun,
            rows,
            query,
            selected,
            choose: Some(choose),
        }
    }
    pub fn filtered(&self) -> Vec<&Choice> {
        let q = self.query.text.to_lowercase();
        self.rows
            .iter()
            .filter(|r| {
                format!("{} {} {}", r.label, r.detail, r.path)
                    .to_lowercase()
                    .contains(&q)
            })
            .collect()
    }
    pub fn current(&self) -> Option<&Choice> {
        let rows = self.filtered();
        rows.iter()
            .copied()
            .find(|r| Some(&r.value) == self.selected.as_ref())
            .or_else(|| rows.first().copied())
    }
}
impl ScreenFloat for Picker {
    fn owner(&self) -> ScreenId {
        self.owner
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
    fn view(&self, t: Theme, _: &App) -> El {
        let rows = self.filtered();
        let selected = self.current().map(|r| &r.value);
        let at = rows
            .iter()
            .position(|r| Some(&r.value) == selected)
            .unwrap_or(0);
        let mut body = col().w_full();
        for r in rows.iter().skip(at.saturating_sub(7)).take(9) {
            let mut content = row().center_y().gap(12.0).w_full();
            if r.project {
                content = content.child(icon(&carbon::FOLDER, 14.0).color(t.c.fg.muted));
            }
            content = content.child(
                line(
                    t,
                    &r.label,
                    t.ty.ui,
                    if r.project {
                        t.c.kind.worker
                    } else {
                        t.c.fg.default
                    },
                )
                .min_w(130.0),
            );
            content = content.child(
                line(
                    t,
                    &r.detail,
                    t.ty.ui,
                    if r.error { t.c.text.failed } else { t.c.fg.alt },
                )
                .min_w(160.0),
            );
            if !r.path.is_empty() {
                content = content.child(
                    line(t, &r.path, t.ty.mono_small, t.c.fg.muted)
                        .shrink()
                        .min_w(0.0),
                );
            }
            content =
                content
                    .child(rune::spacer())
                    .child(line(t, &r.note, t.ty.ui_small, t.c.fg.muted));
            body = body.child(ui::float_row(t, Some(&r.value) == selected, content));
        }
        let input = row()
            .center_y()
            .gap(10.0)
            .child(
                text_input(&self.query, t.ty.float_input)
                    .single_line()
                    .caret(t.c.state.attn, t.c.surface.selection)
                    .color(t.c.fg.default)
                    .on_edit(|a: &mut App| a.float_edit())
                    .on_input_key(crate::app::input::on_field_key)
                    .label(format!("Filter {}", self.title))
                    .key("project-picker-input")
                    .min_w(2.0)
                    .shrink(),
            )
            .child(
                text(format!("{} {}", rows.len(), self.noun))
                    .font(t.ty.ui_small)
                    .color(t.c.fg.muted),
            );
        ui::shell(
            t,
            Shell {
                tag: None,
                title: ui::title(t, &self.title),
                subtitle: Some(self.subtitle.clone()),
                meta: None,
                body,
                input: Some(input),
                hints: vec![
                    ui::float_hint(t, self.scope(), &["<CR>"], "choose"),
                    ui::float_hint(t, self.scope(), &["<Esc>"], "back"),
                ],
            },
        )
    }
}
pub fn keys(a: &mut App, inv: &Invocation, _: &mut rune::Tx) {
    let Some(k) = inv.keys.last() else { return };
    if *k == Key::plain(Code::Esc) || *k == Key::ctrl('c') {
        a.close_float();
        return;
    }
    let Some(p) = top_float::<Picker>(a) else {
        return;
    };
    if k.code == Code::Enter {
        let value = p.current().map(|r| r.value.clone());
        if let Some(value) = value
            && let Some(choose) = p.choose.take()
        {
            a.close_float();
            choose(a, value);
        }
    } else {
        let delta = if *k == Key::ctrl('j') {
            1
        } else if *k == Key::ctrl('k') {
            -1
        } else if *k == Key::ctrl('n') {
            8
        } else if *k == Key::ctrl('p') {
            -8
        } else {
            0
        };
        let rows = p.filtered();
        let at = rows
            .iter()
            .position(|r| Some(&r.value) == p.selected.as_ref())
            .unwrap_or(0);
        let to = (at as isize + delta).clamp(0, rows.len().saturating_sub(1) as isize) as usize;
        p.selected = rows.get(to).map(|r| r.value.clone());
    }
}

pub fn typing(a: &mut App) -> bool {
    if let Some(p) = top_float::<Picker>(a) {
        p.selected = None;
        false
    } else {
        true
    }
}
