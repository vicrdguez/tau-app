//! Connect (`spec.md` §5.1, `client.md` §7): which server, and is this device allowed on it. The
//! first run, and after `:forget-server` or a revoke (`␣rp`). Two single-line fields, the server's
//! URL and the pairing code; `⏎` pairs, after the Confirm float when the URL is plain HTTP to
//! another machine; a failure says why under the fields; once paired, Home with the sidebar.

use rune::{El, PillStyle, Span, TextEdit, Tx, col, icon, rich, row, text, text_input};
use tau_client::engine::PairFailure;
use tau_client::wire::{RefusalCode, Unknown};
use tau_client::{LinkEvent, LinkPhase};

use super::Screen;
use crate::app::scenes::Scene;
use crate::app::{App, Handler, Invocation, Stage, input};
use crate::assets::{carbon, marks};
use crate::copy;
use crate::editor::{EditMode, Editor, Lines, Reply};
use crate::floats::{Confirm, Float};
use crate::frame::Place;
use crate::keys::{Code, Ctx, Key, Mode};
use crate::theme::Theme;
use crate::ui::{Scope, chip, text_field, wordmark};

/// The server field's text until one is kept (`client.md` §7: the server listens there).
pub const DEFAULT_URL: &str = "http://127.0.0.1:6283";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Field {
    #[default]
    Server,
    Code,
}

pub struct State {
    pub server: Editor,
    pub code: Editor,
    failure: Option<Failure>,
    /// The URL and the code the last `pair` sent.
    tried: (String, String),
}

/// Why the last `pair` failed, in the copy's words, and the field whose value it was about.
struct Failure {
    words: String,
    about: Option<Field>,
}

impl Default for State {
    fn default() -> Self {
        let mut s = State {
            server: Editor::inserting(DEFAULT_URL, Lines::Single),
            code: Editor::inserting("", Lines::Single),
            failure: None,
            tried: Default::default(),
        };
        s.focus(Field::Server);
        s
    }
}

impl State {
    /// The field with the caret: the one whose model rune keeps focused, a click's too.
    pub fn field(&self) -> Field {
        if self.code.edit.focused {
            Field::Code
        } else {
            Field::Server
        }
    }

    fn focus(&mut self, field: Field) {
        self.server.edit.focused = field == Field::Server;
        self.code.edit.focused = field == Field::Code;
    }

    fn editor(&mut self) -> &mut Editor {
        match self.field() {
            Field::Server => &mut self.server,
            Field::Code => &mut self.code,
        }
    }

    fn focused(&self) -> &Editor {
        match self.field() {
            Field::Server => &self.server,
            Field::Code => &self.code,
        }
    }

    /// Why the last `pair` failed, while the field it was about still holds what was sent.
    pub fn failure(&self) -> Option<&str> {
        let f = self.failure.as_ref()?;
        let kept = match f.about {
            Some(Field::Server) => self.server.text().trim() == self.tried.0,
            Some(Field::Code) => self.code.text().trim() == self.tried.1,
            None => true,
        };
        kept.then_some(f.words.as_str())
    }

    /// The focused field's text, for its `on_edit`.
    pub fn focused_edit(&mut self) -> Option<&mut TextEdit> {
        Some(&mut self.editor().edit)
    }
}

/// An `http://` URL whose host is not this machine: the token would cross the network in the
/// clear (`client.md` §7).
pub fn plain_http_elsewhere(url: &str) -> bool {
    let url = url.trim();
    let Some(rest) = url
        .get(..7)
        .filter(|s| s.eq_ignore_ascii_case("http://"))
        .map(|_| &url[7..])
    else {
        return false;
    };
    let host = if rest.starts_with('[') {
        rest.split_inclusive(']').next().unwrap_or(rest)
    } else {
        rest.split(['/', ':']).next().unwrap_or(rest)
    };
    !["127.0.0.1", "localhost", "[::1]"].contains(&host.to_ascii_lowercase().as_str())
}

/// The wordmark's height on Connect and the start page (`tokens.json` `layout.start`).
const WORDMARK_HEIGHT: f32 = 44.0;

const WARNING: &str = "This address is not encrypted. Anyone on the network path can read this \
                       device's token. Use https:// (Tailscale Serve or a TLS proxy) or an SSH \
                       tunnel.";
const CONFIRM: &str =
    "Pair over plain HTTP? Anyone on the network path can read this device's token.";

pub struct Connect;

impl Screen for Connect {
    fn ctx(&self, _: &App) -> Ctx {
        Ctx::Connect
    }

    fn mode(&self, app: &App) -> Mode {
        match app.screens.connect.focused().mode() {
            EditMode::Insert => Mode::Insert,
            EditMode::Normal => Mode::Normal,
            EditMode::Visual | EditMode::VisualLine => Mode::Visual,
        }
    }

    fn framed(&self) -> bool {
        false
    }

    fn view(&self, t: Theme, app: &App) -> El {
        view(t, app)
    }

    fn commands(&self) -> Vec<(&'static str, Handler)> {
        vec![("connect.submit", submit)]
    }

    fn globals(&self) -> Vec<(&'static str, Handler)> {
        vec![("server.forget", forget), ("recover.pair", pair_again)]
    }

    fn keys(&self, app: &mut App, keys: &[Key], count: Option<u32>) -> bool {
        let s = &mut app.screens.connect;
        if let [key] = keys
            && key.code == Code::Tab
        {
            s.focus(match s.field() {
                Field::Server => Field::Code,
                Field::Code => Field::Server,
            });
            return true;
        }
        let now = app.now;
        let ed = s.editor();
        if ed.mode() == EditMode::Insert {
            return keys.iter().all(|k| ed.insert(*k, now) == Reply::Handled);
        }
        for k in keys {
            ed.normal(count, *k);
        }
        true
    }

    fn shown(&self, app: &mut App) {
        let s = &mut app.screens.connect;
        let field = if s.server.text().trim().is_empty() {
            Field::Server
        } else {
            s.field()
        };
        s.focus(field);
        s.editor().enter_insert();
    }

    fn on_event(&self, app: &mut App, event: &LinkEvent) {
        match event {
            LinkEvent::Paired { .. } => {
                let s = &mut app.screens.connect;
                s.failure = None;
                s.code.edit.replace_text("");
                app.show(Stage::Place(Place::Home));
            }
            LinkEvent::PairFailed(failure) => {
                let s = &mut app.screens.connect;
                s.failure = Some(Failure {
                    words: copy::pair_failure(failure, &s.tried.0, None),
                    about: about(failure),
                });
            }
            _ => {}
        }
    }

    fn scenes(&self) -> Vec<Scene> {
        vec![
            ("connect", |a| typed(a, DEFAULT_URL)),
            ("connect-http-warning", |a| {
                typed(a, "http://192.168.1.20:6283");
                submit_now(a);
            }),
            ("connect-no-server", |a| {
                typed(a, "http://192.168.1.20:6283");
                let s = &mut a.screens.connect;
                s.tried = (s.server.text().to_string(), s.code.text().to_string());
                s.focus(Field::Server);
                Connect.on_event(
                    a,
                    &LinkEvent::PairFailed(PairFailure::NoReply(Unknown::Unreachable(
                        "Connection refused (os error 61)".into(),
                    ))),
                );
            }),
        ]
    }

    fn poll(&self, app: &mut App, now: f64, tx: &mut Tx) -> bool {
        let ed = app.screens.connect.editor();
        if let Some(due) = ed.jk_deadline() {
            tx.wake_after(std::time::Duration::from_secs_f64((due - now).max(0.0)));
        }
        ed.tick(now)
    }
}

/// Connect with `url` and the fixture's pairing code typed, the caret after the code.
fn typed(a: &mut App, url: &str) {
    a.show(Stage::Connect);
    let s = &mut a.screens.connect;
    s.server.edit.replace_text(url);
    s.code.edit.replace_text("K7QM-3XRT");
    s.focus(Field::Code);
}

fn submit_now(a: &mut App) {
    crate::app::commands::run_id(a, "connect.submit", &mut Tx::default());
}

/// `⏎`: pair with the code, after the Confirm float for plain HTTP to another machine. Without a
/// code, the caret goes to it.
fn submit(app: &mut App, _: &Invocation, _: &mut Tx) {
    if app.link.status().phase == LinkPhase::Pairing {
        return;
    }
    let s = &mut app.screens.connect;
    let url = s.server.text().trim().to_string();
    let code = s.code.text().trim().to_string();
    if url.is_empty() {
        s.focus(Field::Server);
        return;
    }
    if code.is_empty() {
        s.focus(Field::Code);
        return;
    }
    if plain_http_elsewhere(&url) {
        app.open_float(Float::Confirm(Confirm {
            sentence: CONFIRM.into(),
            yes: "pairs",
            on_yes: Some(Box::new(move |a: &mut App, _: &mut Tx| pair(a, url, code))),
        }));
    } else {
        pair(app, url, code);
    }
}

fn pair(app: &mut App, url: String, code: String) {
    let s = &mut app.screens.connect;
    s.failure = None;
    s.tried = (url.clone(), code.clone());
    app.link.pair(&url, &code);
}

/// The field whose value a failure was about: the address for no server, another protocol or a
/// host the server doesn't allow; the code for a wrong or expired one; neither for the rest.
fn about(failure: &PairFailure) -> Option<Field> {
    match failure {
        PairFailure::NoReply(Unknown::Unreachable(_) | Unknown::OtherProtocol(_)) => {
            Some(Field::Server)
        }
        PairFailure::Refused(r) if copy::host_refused(r) => Some(Field::Server),
        PairFailure::Refused(r) if r.code == RefusalCode::Unauthorized => Some(Field::Code),
        _ => None,
    }
}

/// `:forget-server`: after a confirm, the token and this server's state go, and Connect opens with
/// the URL kept.
fn forget(app: &mut App, _: &Invocation, _: &mut Tx) {
    let Some(url) = app.link.server().url.clone() else {
        return;
    };
    let host = url.split_once("://").map_or(url.as_str(), |(_, h)| h);
    app.open_float(Float::Confirm(Confirm {
        sentence: format!(
            "Forget {host}? This device's token and its state for this server are deleted. The \
             server lists the device until it is revoked there."
        ),
        yes: "forgets",
        on_yes: Some(Box::new(move |a: &mut App, _: &mut Tx| {
            crate::app::local::forget(a);
            a.screens.connect.server.edit.replace_text(&url);
            a.show(Stage::Connect);
        })),
    }));
}

/// `␣rp`: after a revoke, Connect with the URL kept; the revoked token is forgotten first.
fn pair_again(app: &mut App, _: &Invocation, _: &mut Tx) {
    match app.link.status().phase {
        LinkPhase::Revoked => {
            let url = app.link.server().url.clone().unwrap_or_default();
            crate::app::local::forget(app);
            app.screens.connect.server.edit.replace_text(&url);
            app.show(Stage::Connect);
        }
        LinkPhase::NotPaired | LinkPhase::Pairing => app.show(Stage::Connect),
        _ => app.say("This device is paired; :forget-server pairs it anew"),
    }
}

/// The page: a centred 600 px column of the wordmark, the title, the two fields, the paragraph and
/// the form's footer. Its sizes are the prototype's (`tools/proto/calm.css` `.connect`).
fn view(t: Theme, app: &App) -> El {
    let s = &app.screens.connect;
    let scope = app.scope();
    let keys = app.floats.is_empty();
    let insert = keys && scope.mode == Mode::Insert;
    let url = s.server.text();
    let mut fields = col().gap(10.0).mt(24.0).w_full();
    fields = fields.child(field_row(
        t,
        "Server",
        field(
            t,
            &s.server,
            keys,
            insert && s.field() == Field::Server,
            "Server",
            |a| &mut a.screens.connect.server.edit,
        ),
    ));
    if plain_http_elsewhere(url) {
        fields = fields.child(under(warning(t)));
    }
    fields = fields.child(field_row(
        t,
        "Pairing code",
        field(
            t,
            &s.code,
            keys,
            insert && s.field() == Field::Code,
            "Pairing code",
            |a| &mut a.screens.connect.code.edit,
        ),
    ));
    if let Some(failure) = s.failure() {
        fields = fields.child(under(failed(t, failure)));
    }
    let pairing = app.link.status().phase == LinkPhase::Pairing;
    col().w_full().h_full().items_center().child(
        col()
            .w(600.0)
            .pt(96.0)
            .child(wordmark(t, WORDMARK_HEIGHT))
            .child(
                text("Connect to a tau server")
                    .font(t.ty.title_page)
                    .color(t.c.state.attn)
                    .mt(32.0),
            )
            .child(fields)
            .child(paragraph(t).mt(22.0))
            .child(footer(t, scope, pairing).mt(22.0)),
    )
}

fn field_row(t: Theme, label: &str, input: El) -> El {
    row()
        .center_y()
        .w_full()
        .child(
            text(label.to_string())
                .font(t.ty.ui_small)
                .color(t.c.kind.you)
                .w(120.0),
        )
        .child(input.grow(1.0).min_w(0.0))
}

/// One field: Iosevka 14 on `surface.sunken`, 30 px tall, its edge coral with the caret in
/// Insert. Its caret shows only while the page has the keys. While a float has them the field is
/// drawn without `on_edit`: rune clears the focus of a model it can reach when focus leaves it,
/// and that focus is what returns the caret to this field when the float closes.
fn field(
    t: Theme,
    ed: &Editor,
    keys: bool,
    coral: bool,
    name: &'static str,
    on_edit: fn(&mut App) -> &mut TextEdit,
) -> El {
    let mut edit = ed.edit.clone();
    edit.focused &= keys;
    let input = text_input(&edit, t.ty.float_input)
        .single_line()
        .caret(t.c.state.attn, t.c.surface.selection)
        .color(t.c.fg.default)
        .on_input_key(input::on_field_key)
        .label(name)
        .key(name);
    let input = if keys { input.on_edit(on_edit) } else { input };
    text_field(t, coral, input).h(30.0)
}

/// A line under a field, in its column.
fn under(body: El) -> El {
    row().w_full().pl(120.0).child(body.grow(1.0).min_w(0.0))
}

fn warning(t: Theme) -> El {
    row()
        .gap(8.0)
        .child(icon(&carbon::WARNING_ALT, 14.0).color(t.c.fg.alt).mt(2.0))
        .child(
            text(WARNING)
                .font(t.ty.prose_small)
                .color(t.c.fg.default)
                .wrap()
                .grow(1.0)
                .min_w(0.0),
        )
}

fn failed(t: Theme, words: &str) -> El {
    row()
        .gap(8.0)
        .child(icon(&marks::FAILED, 8.0).color(t.c.state.blocked).mt(5.0))
        .child(
            text(words.to_string())
                .font(t.ty.prose_small)
                .color(t.c.fg.default)
                .wrap()
                .grow(1.0)
                .min_w(0.0),
        )
}

/// The paragraph, `prose` in `fg.alt`, its commands as inline code: gold on a `surface.code` pill.
fn paragraph(t: Theme) -> El {
    let parts = [
        ("On the server's machine, run ", false),
        ("tau device pair <name>", true),
        (", with a new name for this device such as ", false),
        ("laptop", true),
        (
            "; a name is never used twice. It prints a code that works once, for 10 minutes. The \
             token it gives this device is kept in the macOS Keychain.",
            false,
        ),
    ];
    let mut content = String::new();
    let mut code = Vec::new();
    for (words, is_code) in parts {
        let start = content.len();
        content.push_str(words);
        if is_code {
            code.push(start..content.len());
        }
    }
    let spans = code
        .iter()
        .map(|r| Span::new(r.start, r.end).mono().size(13.0))
        .collect();
    let pill = PillStyle::new().bg(t.c.surface.code).fg(t.c.kind.code);
    rich(content, spans)
        .font(t.ty.prose)
        .color(t.c.fg.alt)
        .wrap()
        .w_full()
        .pills(code.into_iter().map(|r| (r, pill)))
}

/// `⏎ connect`, or `pairing…` while the `pair` call is out.
fn footer(t: Theme, scope: Scope, pairing: bool) -> El {
    if pairing {
        return row().child(text("pairing…").font(t.ty.ui).color(t.c.fg.muted));
    }
    row()
        .center_y()
        .gap(7.0)
        .child(chip(t, scope, "<CR>", false))
        .child(text("connect").font(t.ty.ui).color(t.c.fg.alt))
}

#[cfg(test)]
mod tests;
