//! Server facts supplied by the screen controller, without connection logic.
use crate::{
    content,
    theme::Theme,
    ui::{self, Scope, Shell},
};
use rune::{El, col, text};
pub fn view(t: Theme, scope: Scope, url: &str, facts: &[(String, String)], offset: usize) -> El {
    let mut body = col().w_full();
    for (label, value) in facts.iter().skip(offset).take(8) {
        body = body.child(ui::kv_row(
            t,
            label,
            content::line(t, value, t.ty.ui, t.c.fg.default),
        ));
    }
    body=body.child(text("Devices are listed and revoked on the server: tau device list, tau device revoke <name>.").font(t.ty.ui_small).color(t.c.fg.alt).mx(24.0).mt(12.0))
        .child(text(":forget-server · type after closing this float").font(t.ty.mono_small).color(t.c.fg.muted).mx(24.0).mt(6.0));
    ui::shell(
        t,
        Shell {
            tag: None,
            title: ui::title(t, "Server"),
            subtitle: Some(url.into()),
            meta: None,
            body,
            input: None,
            hints: vec![ui::float_hint(t, scope, &["<Esc>"], "closes")],
        },
    )
}
