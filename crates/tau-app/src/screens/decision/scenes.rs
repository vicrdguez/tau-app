//! The decision's mockup states on the fixture link (`tools/proto/states.md` 22 to 29, 42, 46).
//! States the plain fixture link can't reach (a later inbox frame, a refused answer) run on
//! [`Wire`]: the same fixture as a server that also delivers the fixture's later inbox frames and
//! answers `answer` with one of its refusals.

use std::sync::{Arc, LazyLock, Mutex};

use rune::Tx;
use serde_json::Value;
use tau_client::engine::StreamEvent;
use tau_client::fixture::FixtureTransport;
use tau_client::sse::Frame;
use tau_client::store::{ClientConfig, Secrets, Store};
use tau_client::transport::{Deliver, Reply, StreamHandle, Target, Transport};
use tau_client::wire::{CallName, Outcome, Request, parse_reply};
use tau_client::{DecisionRef, Fixture, Link, LinkConfig, Wall};

use super::notes::Draft;
use super::readback::{Kind, Phase, Readback};
use super::state::Cursor;
use crate::app::scenes::{Scene, on_fixture};
use crate::app::{App, Focus, Invocation, Stage, link};
use crate::floats::Float;
use crate::frame::Row;
use crate::keys::keymap;
use crate::open::Item;
use crate::theme::Variant;

static FIXTURE: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../../../design/fixture.json"))
        .expect("design/fixture.json is JSON")
});

pub fn all() -> Vec<Scene> {
    vec![
        ("decision-long", |a| long(a)),
        ("decision-note-insert", note_insert),
        ("decision-light", |a| {
            note_insert(a);
            a.variant = Variant::EmberLight;
        }),
        ("readback-answer", |a| {
            long(a);
            open_answer(a, Some(1), false);
            run(a, "readback.confirm");
        }),
        ("decision-forecast", |a| {
            shown(a, "forecasts", "w2:2:pi-durable-2#1");
            a.screens
                .decision
                .page(&dref("forecasts", "w2:2:pi-durable-2#1"))
                .cursor = Some(Cursor::Choice(1));
        }),
        ("decision-superseded", |a| {
            let wire = long_on_wire(a, None);
            wire.inbox(FIXTURE["streams"]["inbox"]["later"][2]["frame"]["ops"].clone());
            frames(a);
        }),
        ("answer-conflict", |a| {
            let refusal = &FIXTURE["refusals"]["conflict (answer w1:3:design#2 after the phone answered approve)"]
                ["reply"];
            long_on_wire(a, Some(refusal.clone()));
            open_answer(a, Some(1), true);
            run(a, "readback.confirm");
            frames(a);
        }),
        ("decision-then-faulted", |a| {
            shown(a, "render-core", "w1:6:design#1");
        }),
        ("readback-send-again", |a| {
            shown(a, "render-core", "w1:6:design#1");
            run(a, "auth.again");
            if let Some(f) = crate::screens::top_float::<Readback>(a) {
                f.drawn = Some(f.pick);
            }
        }),
        ("which-key-answer", |a| {
            long(a);
            let scope = a.scope();
            for key in crate::keys::parse_seq("<leader>a").unwrap_or_default() {
                a.keys
                    .press(keymap(), scope.mode, scope.ctx, key, false, -1.0);
            }
            a.which_key_shown = true;
        }),
        ("sidebar-focus-decision", |a| {
            long(a);
            a.focus = Focus::Sidebar;
            a.sidebar_cursor = Row::Item(review());
        }),
    ]
}

fn dref(project: &str, id: &str) -> DecisionRef {
    DecisionRef::new(project, id)
}

fn review() -> Item {
    Item::Decision {
        project: "render-core".into(),
        id: "w1:3:design#2".into(),
    }
}

/// The fixture's `now`: ages read as the mockups'.
fn freeze(a: &mut App) {
    a.screens.decision.frozen_now = FIXTURE["now"].as_i64().map(Wall);
}

fn shown(a: &mut App, project: &str, id: &str) {
    freeze(a);
    on_fixture(
        a,
        Stage::Item(Item::Decision {
            project: project.into(),
            id: id.into(),
        }),
    );
    // Now that the link is ready, the earlier requests the margin names.
    crate::screens::Screen::shown(&super::Decision, a);
    link::settle(a);
}

/// The fixture's drafts of `w1:3:design#2` (`local.noteDrafts`).
fn drafts() -> Vec<Draft> {
    serde_json::from_value(FIXTURE["local"]["noteDrafts"]["render-core/w1:3:design#2"].clone())
        .unwrap_or_default()
}

/// State 22: the drafts, the cursor on the bullet whose passage has a note.
fn long(a: &mut App) {
    shown(a, "render-core", "w1:3:design#2");
    seed(a, drafts());
    cursor_on(a, "A line is at most");
}

fn seed(a: &mut App, d: Vec<Draft>) {
    let s = &mut a.screens.decision;
    s.notes.seed(&dref("render-core", "w1:3:design#2"), d);
}

fn text(a: &App) -> String {
    super::reader::record(a, &dref("render-core", "w1:3:design#2"))
        .map(|v| v.text.clone())
        .unwrap_or_default()
}

fn cursor_on(a: &mut App, words: &str) {
    let at = text(a).find(words).unwrap_or(0);
    a.screens
        .decision
        .page(&dref("render-core", "w1:3:design#2"))
        .cursor = Some(Cursor::Text(at));
}

/// State 23: Visual over `two exports of the same store are byte-equal.`, `␣mn`, and the note
/// typed.
fn note_insert(a: &mut App) {
    shown(a, "render-core", "w1:3:design#2");
    let mut d = drafts();
    d.remove(1);
    seed(a, d);
    let body = text(a);
    let from = body.find("two exports of the same").unwrap_or(0);
    let to = body
        .find("byte-equal.")
        .map_or(from, |i| i + "byte-equal".len());
    let page = a
        .screens
        .decision
        .page(&dref("render-core", "w1:3:design#2"));
    page.visual = Some((from, false));
    page.cursor = Some(Cursor::Text(to));
    run(a, "note.here");
    if let Some(e) = a.screens.decision.editing.as_mut() {
        e.editor.edit.replace_text(
            "Byte-equal only while history keeps its key order across core versions. Say whether it does.",
        );
    }
}

/// The answer readback with `pick`; `armed`, it has been on screen past the arm time.
fn open_answer(a: &mut App, pick: Option<usize>, armed: bool) {
    a.open_float(Float::Screen(Box::new(Readback {
        decision: dref("render-core", "w1:3:design#2"),
        kind: Kind::Answer,
        pick,
        drawn: Some(pick),
        shown_at: armed.then_some(-10.0),
        phase: Phase::Reading,
        notes_open: false,
    })));
}

/// Runs a command as its keys would where the keys are.
fn run(a: &mut App, id: &str) {
    if let Some(command) = keymap().command(id) {
        let applies = keymap().applies(command, a.scope().ctx);
        let inv = Invocation {
            command,
            count: None,
            applies,
            keys: Vec::new(),
        };
        crate::app::commands::run(a, &inv, &mut Tx::default());
    }
}

/// Applies what the link received and hands its events to the screens, as frames would.
fn frames(a: &mut App) {
    let live = a.live;
    a.live = true;
    for _ in 0..10 {
        link::pump(a, &mut Tx::default());
    }
    a.live = live;
}

/// State 22 on [`Wire`], answering `answer` with `refusal` when given.
fn long_on_wire(a: &mut App, refusal: Option<Value>) -> Arc<Wire> {
    let wire = Arc::new(Wire {
        fixture: FixtureTransport::new(Fixture::design()),
        answer: refusal.map(|r| {
            parse_reply(
                200,
                Some(tau_client::wire::PROTOCOL),
                &serde_json::to_vec(&r).unwrap_or_default(),
            )
        }),
        inbox: Mutex::new(Vec::new()),
    });
    let f = Fixture::design();
    let dir = std::env::temp_dir().join(format!("tau-scene-{}", std::process::id()));
    let store = Store::at(dir.clone());
    let _ = store.save_client(&ClientConfig {
        server: Some(f.url()),
        device: Some(f.device()),
    });
    let secrets = Secrets::memory();
    let _ = secrets.save(&f.url(), &f.token());
    a.link = Link::with_transport(
        LinkConfig {
            dir: Some(dir),
            secrets,
            clock: Arc::new(tau_client::clock::SystemClock),
        },
        wire.clone(),
    );
    long(a);
    wire
}

/// The fixture as a server that can also send a later inbox frame and refuse `answer`.
struct Wire {
    fixture: FixtureTransport,
    answer: Option<Outcome>,
    inbox: Mutex<Vec<Arc<Deliver>>>,
}

struct Held;

impl StreamHandle for Held {
    fn close(&self) {}
}

impl Wire {
    fn inbox(&self, ops: Value) {
        let ops = ops.as_array().cloned().unwrap_or_default();
        for d in self.inbox.lock().unwrap_or_else(|e| e.into_inner()).iter() {
            d(StreamEvent::Frame(Frame::Ops(ops.clone())));
        }
    }
}

impl Transport for Wire {
    fn call(&self, target: &Target, request: Request, reply: Reply) {
        match (&request.call, &self.answer) {
            (CallName::Answer, Some(outcome)) => reply(outcome.clone()),
            _ => self.fixture.call(target, request, reply),
        }
    }

    fn open(&self, target: &Target, request: Request, deliver: Deliver) -> Box<dyn StreamHandle> {
        let inbox = request.call == CallName::Inbox;
        let deliver = Arc::new(deliver);
        if inbox {
            self.inbox
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(deliver.clone());
        }
        self.fixture
            .open(target, request, Box::new(move |e| deliver(e)));
        Box::new(Held)
    }
}
