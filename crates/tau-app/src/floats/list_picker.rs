//! Find, Filter, and list search share the same plain rows and input layout.
use crate::{
    content,
    theme::Theme,
    ui::{self, Scope, Shell},
};
use rune::{El, TextEdit, col, row, text, text_input};

pub fn view(
    t: Theme,
    scope: Scope,
    title: &str,
    subtitle: &str,
    rows: &[String],
    cursor: usize,
    edit: &TextEdit,
) -> El {
    let mut body = col().w_full();
    for (i, label) in rows.iter().enumerate() {
        body = body.child(ui::float_row(
            t,
            i == cursor,
            content::line(t, label, t.ty.ui, t.c.fg.default),
        ));
    }
    let input = row()
        .center_y()
        .gap(10.0)
        .child(
            text_input(edit, t.ty.float_input)
                .single_line()
                .caret(t.c.state.attn, t.c.surface.selection)
                .color(t.c.fg.default)
                .on_edit(|a: &mut crate::app::App| a.float_edit())
                .on_input_key(crate::app::input::on_field_key)
                .label(format!("{title} query"))
                .key("list-picker-input")
                .min_w(2.0)
                .shrink(),
        )
        .child(
            text(format!("{} rows", rows.len()))
                .font(t.ty.ui_small)
                .color(t.c.fg.muted),
        );
    ui::shell(
        t,
        Shell {
            tag: None,
            title: ui::title(t, title),
            subtitle: Some(subtitle.into()),
            meta: None,
            body,
            input: Some(input),
            hints: vec![
                ui::float_hint(t, scope, &["<CR>"], "choose"),
                ui::float_hint(t, scope, &["<Esc>"], "back"),
            ],
        },
    )
}
