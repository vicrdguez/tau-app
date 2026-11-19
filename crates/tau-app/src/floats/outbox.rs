//! Outbox layout. Its screen controller supplies plain copy and decides retry eligibility.
use crate::{
    content,
    theme::Theme,
    ui::{self, Mark, Scope, Shell},
};
use rune::{El, col, row, text};

/// One entry: `answer · render-core · Review the design #2`. An absent part is left out with its
/// separator.
pub struct Row {
    pub call: &'static str,
    pub project: Option<String>,
    pub conversation: Option<u64>,
    pub about: Option<About>,
    pub state: String,
    pub time: String,
    pub unknown: bool,
    /// `⏎` opens an object the row names.
    pub opens: bool,
}

/// What an entry is about, after its project.
pub enum About {
    /// The input's text, cut with `…`.
    Text(String),
    /// The decision an answer belongs to, when its title is held.
    Decision { title: String, number: u32 },
    /// The stored decision ID, when the title is not held; never cut.
    DecisionId(String),
}

fn label(t: Theme, r: &Row) -> El {
    let sep = || text("·").font(t.ty.ui).color(t.c.fg.muted);
    let number = |n: String| text(n).font(t.ty.mono_small).color(t.c.kind.literal);
    let mut line = row()
        .center_y()
        .gap(6.0)
        .min_w(0.0)
        .child(text(r.call).font(t.ty.ui).color(t.c.fg.default));
    if let Some(p) = &r.project {
        line = line
            .child(sep())
            .child(text(p.clone()).font(t.ty.ui).color(t.c.kind.worker));
        if let Some(n) = r.conversation {
            line = line.child(number(format!("#{n}")));
        }
    }
    match &r.about {
        None => line,
        Some(About::Text(s)) => line.child(sep()).child(
            content::line(t, s, t.ty.ui, t.c.fg.default)
                .shrink()
                .min_w(0.0),
        ),
        Some(About::Decision { title, number: n }) => line
            .child(sep())
            .child(
                content::line(t, title, t.ty.ui, t.c.fg.default)
                    .shrink()
                    .min_w(0.0),
            )
            .child(number(format!("#{n}"))),
        Some(About::DecisionId(id)) => line.child(sep()).child(
            text(id.clone())
                .font(t.ty.mono_small)
                .color(t.c.kind.literal)
                .min_w(id.chars().count() as f32 * t.ty.mono_small.size / 2.0),
        ),
    }
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
    for (i, r) in rows.iter().enumerate() {
        let line = ui::slot_row(
            Some(if r.unknown {
                ui::mark(t, Mark::Unknown)
            } else {
                rune::el().w(14.0)
            }),
            label(t, r),
            vec![
                content::line(t, &r.state, t.ty.ui_small, t.c.fg.alt).w(340.0),
                content::line(t, &r.time, t.ty.mono_small, t.c.fg.muted).w(44.0),
            ],
            12.0,
        );
        body = body.child(ui::float_row(t, i == cursor, line));
    }
    if rows.is_empty() {
        body = body.child(
            row()
                .h(30.0)
                .center_y()
                .px(24.0)
                .child(ui::title(t, "No writes in the outbox.")),
        );
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
    hints.push(ui::float_hint_if(
        t,
        scope,
        &["<CR>"],
        "open",
        rows.get(cursor).is_some_and(|r| r.opens),
    ));
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
