use super::{Blocker, StartWith, blocker};
use crate::{
    app::{App, Focus},
    assets::carbon,
    content::line,
    theme::Theme,
    ui,
};
use rune::{El, col, icon, row, text, text_input};

pub(super) fn view(t: Theme, a: &App) -> El {
    let s = &a.screens.start;
    let active = a.focus == Focus::Stage && a.floats.is_empty() && s.pending.is_none();
    let insert = active && super::editor_mode(&s.editor) == crate::keys::Mode::Insert;
    let block = blocker(a);
    let mut edit = s.editor.edit.clone();
    edit.focused = insert;
    let input = text_input(&edit, t.ty.prose)
        .wrap_anywhere()
        .caret(t.c.state.attn, t.c.surface.selection)
        .color(t.c.fg.default)
        .placeholder("What do you want to work on?", t.c.fg.muted)
        .on_edit(|a: &mut App| &mut a.screens.start.editor.edit)
        .on_input_key(crate::app::input::on_field_key)
        .label("What do you want to work on?")
        .key("start-composer")
        .w_full()
        .min_h(60.0)
        .max_h(252.0);
    let input = if active { input } else { input.inert() };
    let scope = if active {
        a.scope()
    } else {
        ui::Scope {
            ctx: crate::keys::Ctx::Confirm,
            mode: crate::keys::Mode::Readback,
        }
    };
    let footer = if s.pending.is_some() {
        text("starting…").font(t.ty.ui_small).color(t.c.fg.alt)
    } else if !a.floats.is_empty() {
        rune::el().h(16.0)
    } else {
        let word = if matches!(block, Blocker::Options(_)) {
            "set its options"
        } else {
            "start"
        };
        let mut r = row()
            .center_y()
            .gap(6.0)
            .child(ui::hint(t, scope, "<D-CR>"))
            .child(text(word).font(t.ty.ui_small).color(t.c.fg.muted));
        if super::editor_mode(&s.editor) == crate::keys::Mode::Normal {
            r = r.child(text("· i writes").font(t.ty.hint).color(t.c.fg.muted));
        }
        r
    };
    let composer = col()
        .w_full()
        .min_h(112.0)
        .px(15.0)
        .pt(19.0)
        .pb(11.0)
        .gap(6.0)
        .fill(t.c.surface.raised)
        .border(
            1.0,
            if insert {
                t.c.state.attn
            } else {
                t.c.border.hairline
            },
        )
        .radius(t.l.radius.surface)
        .child(input)
        .child(row().child(rune::spacer()).child(footer));
    let mut startline = row().center_y().gap(8.0).mt(12.0).px(4.0);
    match block {
        Blocker::NoProjects => {
            startline = startline
                .child(
                    text("No projects yet ·")
                        .font(t.ty.ui_small)
                        .color(t.c.fg.alt),
                )
                .child(ui::hint(t, scope, "<D-CR>"))
                .child(text("binds one").font(t.ty.ui_small).color(t.c.fg.alt))
        }
        Blocker::ChooseProject => {
            startline = startline
                .child(
                    text("choose a project")
                        .font(t.ty.ui_small)
                        .color(t.c.fg.alt),
                )
                .child(ui::hint(t, scope, "<M-p>"))
        }
        _ => {
            startline = startline
                .child(icon(&carbon::FOLDER, 13.0).color(t.c.fg.muted))
                .child(line(
                    t,
                    s.project.as_deref().unwrap_or(""),
                    t.ty.ui_small,
                    t.c.kind.worker,
                ))
                .child(ui::hint(t, scope, "<M-p>"))
                .child(text("·").font(t.ty.ui_small).color(t.c.fg.muted));
            match &s.with {
                Some(StartWith::Workflow(w)) => {
                    startline = startline
                        .child(line(t, w, t.ty.mono_small, t.c.fg.default))
                        .child(text("workflow").font(t.ty.ui_small).color(t.c.fg.alt));
                }
                Some(StartWith::Model(m)) if block != Blocker::NoModels => {
                    startline = startline
                        .child(text("bare agent ·").font(t.ty.ui_small).color(t.c.fg.alt))
                        .child(line(
                            t,
                            m.rsplit('/').next().unwrap_or(m),
                            t.ty.mono_small,
                            t.c.fg.default,
                        ));
                }
                _ => {
                    startline = startline.child(
                        text(if block == Blocker::Loading {
                            super::super::NOTHING_RECEIVED
                        } else {
                            "bare agent · no models on the server"
                        })
                        .font(t.ty.ui_small)
                        .color(t.c.fg.alt),
                    );
                }
            }
            let error = match &block {
                Blocker::Options(s) => Some(s.as_str()),
                Blocker::Missing(_) => Some("missing on the server"),
                _ => None,
            };
            if let Some(error) = error {
                startline = startline
                    .child(text("·").font(t.ty.ui_small).color(t.c.fg.muted))
                    .child(line(t, error, t.ty.ui_small, t.c.text.failed));
            }
            startline = startline.child(ui::hint(t, scope, "<M-w>"));
        }
    }
    let mut content = col()
        .w(640.0)
        .max_w((a.window[0] - 32.0).max(0.0))
        .mt(-80.0)
        .child(row().justify_center().child(ui::wordmark(t, 44.0)).mb(36.0))
        .child(composer)
        .child(startline);
    if let Some(failure) = &s.failure {
        content = content.child(
            row()
                .gap(6.0)
                .mt(10.0)
                .px(4.0)
                .child(icon(&carbon::HELP, 12.0).color(t.c.state.unknown))
                .child(
                    line(
                        t,
                        failure.strip_prefix("? ").unwrap_or(failure),
                        t.ty.ui_small,
                        t.c.fg.default,
                    )
                    .wrap()
                    .grow(1.0)
                    .min_w(0.0),
                ),
        );
    }
    // 32.5% of the page's available height, including when a float takes room below it.
    col()
        .w_full()
        .h_full()
        .child(col().grow(0.325).min_h(104.0))
        .child(col().grow(0.675).min_h(0.0).items_center().child(content))
}
