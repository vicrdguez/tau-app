//! Outbox layout. Its screen controller supplies plain copy and decides retry eligibility.
use crate::{
    content,
    theme::Theme,
    ui::{self, Mark, Scope, Shell},
};
use rune::{El, col, row};

pub struct Row {
    pub label: String,
    pub state: String,
    pub time: String,
    pub unknown: bool,
}
pub fn view(
    t: Theme,
    scope: Scope,
    rows: &[Row],
    cursor: usize,
    unknown: usize,
    retry: bool,
    details: Option<&str>,
) -> El {
    let mut body = col().w_full();
    for (i, r) in rows
        .iter()
        .enumerate()
        .skip(cursor.saturating_sub(5))
        .take(8)
    {
        let line = ui::slot_row(
            Some(if r.unknown {
                ui::mark(t, Mark::Unknown)
            } else {
                rune::el().w(14.0)
            }),
            content::line(t, &r.label, t.ty.ui, t.c.fg.default),
            vec![
                content::line(t, &r.state, t.ty.ui_small, t.c.fg.alt).w(340.0),
                content::line(t, &r.time, t.ty.mono_small, t.c.fg.muted).w(44.0),
            ],
            12.0,
        );
        body = body.child(ui::float_row(t, i == cursor, line));
    }
    if rows.is_empty() {
        body = body.child(ui::kv_row(t, "", ui::title(t, "No writes in the outbox.")));
    }
    if let Some(details) = details {
        body = body.child(
            rune::text(details.to_string())
                .font(t.ty.mono_small)
                .color(t.c.fg.alt)
                .wrap()
                .mx(24.0)
                .max_h(140.0)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                }),
        );
    }
    let mut hints = Vec::new();
    if retry {
        hints.push(ui::float_hint(t, scope, &["<leader>rt"], "retry"));
    }
    hints.push(ui::float_hint(t, scope, &["x"], "forget"));
    hints.push(ui::float_hint(t, scope, &["<Esc>"], "closes"));
    ui::shell(
        t,
        Shell {
            tag: None,
            title: ui::title(t, "Outbox"),
            subtitle: Some("every write, kept until done".into()),
            meta: Some(format!("{unknown} without a reply")),
            body: row().w_full().child(body),
            input: None,
            hints,
        },
    )
}
