//! Filterable choices for the start page and option fields. Model rows follow the link's catalogue.

use super::{FILTER, Input, Noun, input, no_match};
use crate::app::{App, Invocation};
use crate::assets::carbon;
use crate::content::line;
use crate::keys::{Code, Ctx, Key, Mode};
use crate::screens::{ScreenFloat, ScreenId, top_float, top_float_ref};
use crate::theme::Theme;
use crate::ui::{self, Badge, Scope, Shell};
use rune::{El, TextEdit, col, icon, row};

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
    pub noun: Noun,
    rows: Vec<Choice>,
    model_rows: Option<fn(&App) -> Vec<Choice>>,
    pub query: TextEdit,
    pub selected: Option<String>,
    pub choose: Option<Choose>,
}
impl Picker {
    pub fn new(
        owner: ScreenId,
        title: &str,
        subtitle: String,
        noun: Noun,
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
            model_rows: None,
            query,
            selected,
            choose: Some(choose),
        }
    }
    pub fn with_model_rows(mut self, rows: fn(&App) -> Vec<Choice>) -> Self {
        self.model_rows = Some(rows);
        self
    }
    fn models(&self, a: &App) -> Vec<Choice> {
        self.model_rows.map_or_else(Vec::new, |rows| rows(a))
    }
    fn filtered<'a>(&'a self, models: &'a [Choice]) -> Vec<&'a Choice> {
        let q = self.query.text.to_lowercase();
        self.rows
            .iter()
            .chain(models)
            .filter(|r| {
                format!("{} {} {}", r.label, r.detail, r.path)
                    .to_lowercase()
                    .contains(&q)
            })
            .collect()
    }
    fn current<'a>(&self, rows: &[&'a Choice]) -> Option<&'a Choice> {
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
    fn offers_model_refresh(&self) -> bool {
        self.model_rows.is_some()
    }
    fn view(&self, t: Theme, a: &App) -> El {
        let models = self.models(a);
        let total = self.rows.len() + models.len();
        let rows = self.filtered(&models);
        let selected = self.current(&rows).map(|r| &r.value);
        let mut body = col().w_full();
        for r in &rows {
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
            let value = r.value.clone();
            // The press sits on a wrapper: the selected row keeps the key the shell scrolls to.
            body = body.child(
                row()
                    .w_full()
                    .key(format!("pick-{}", r.value))
                    .on_press(move |a: &mut App| choose(a, value.clone()))
                    .child(ui::float_row(t, Some(&r.value) == selected, content)),
            );
        }
        if rows.is_empty() && total != 0 {
            body = body.child(no_match(t, self.noun.none(&self.query.text)));
        }
        let input = input(
            t,
            Input {
                edit: &self.query,
                label: &format!("Filter {}", self.noun.many),
                placeholder: FILTER,
                count: self.noun.kept(&self.query.text, rows.len(), total),
            },
        );
        let mut hints = Vec::new();
        if self.offers_model_refresh() {
            hints.push(super::model_refresh(t, self.scope(), a));
        }
        hints.push(ui::float_hint_if(
            t,
            self.scope(),
            &["<CR>"],
            "choose",
            !rows.is_empty(),
        ));
        hints.push(ui::float_hint(t, self.scope(), &["<Esc>"], "back"));
        ui::shell(
            t,
            Shell {
                tag: None,
                title: ui::title(t, &self.title),
                subtitle: Some(self.subtitle.clone()),
                meta: None,
                body,
                input: Some(input),
                hints,
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
    let Some(p) = top_float_ref::<Picker>(a) else {
        return;
    };
    let models = p.models(a);
    let rows = p.filtered(&models);
    if k.code == Code::Enter {
        if let Some(value) = p.current(&rows).map(|r| r.value.clone()) {
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
        let at = rows
            .iter()
            .position(|r| Some(&r.value) == p.selected.as_ref())
            .unwrap_or(0);
        let to = (at as isize + delta).clamp(0, rows.len().saturating_sub(1) as isize) as usize;
        let selected = rows.get(to).map(|r| r.value.clone());
        if let Some(p) = top_float::<Picker>(a) {
            p.selected = selected;
        }
    }
}

/// `⏎` on the selected row, or a click on any row.
fn choose(a: &mut App, value: String) {
    if let Some(choose) = top_float::<Picker>(a).and_then(|p| p.choose.take()) {
        a.close_float();
        choose(a, value);
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
