//! Links for tests: the design's fixture as a server that a test can silence, refuse every call
//! to, or end a stream of, on a clock that moves when told.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tau_client::clock::{ManualClock, Mono, Now, Wall};
use tau_client::engine::StreamEvent;
use tau_client::fixture::FixtureTransport;
use tau_client::sse::{EndReason, Frame};
use tau_client::store::{ClientConfig, Secrets, Store};
use tau_client::transport::{Deliver, Reply, StreamHandle, Target, Transport};
use tau_client::wire::{CallName, Outcome, Request};
use tau_client::{Fixture, Link, LinkConfig};

/// The fixture's server, which tests can silence, refuse every call to, or end a stream of.
pub(crate) struct Wire {
    fixture: FixtureTransport,
    pub silent: AtomicBool,
    pub refuse: Mutex<Option<Outcome>>,
    pub model_refresh: Mutex<Option<Outcome>>,
    streams: Mutex<Vec<Arc<Deliver>>>,
}

struct Held;

impl StreamHandle for Held {
    fn close(&self) {}
}

impl Transport for Wire {
    fn call(&self, target: &Target, request: Request, reply: Reply) {
        if self.silent.load(Ordering::SeqCst) {
            return;
        }
        if request.call == CallName::Models
            && request.body == serde_json::json!({"refresh": true})
            && let Some(outcome) = self.model_refresh.lock().unwrap().clone()
        {
            reply(outcome);
            return;
        }
        match self.refuse.lock().unwrap().clone() {
            Some(outcome) => reply(outcome),
            None => self.fixture.call(target, request, reply),
        }
    }

    fn open(&self, target: &Target, request: Request, deliver: Deliver) -> Box<dyn StreamHandle> {
        if self.silent.load(Ordering::SeqCst) {
            return Box::new(Held);
        }
        let deliver = Arc::new(deliver);
        self.streams.lock().unwrap().push(deliver.clone());
        self.fixture
            .open(target, request, Box::new(move |e| deliver(e)));
        Box::new(Held)
    }
}

impl Wire {
    pub fn end_every_stream(&self, reason: EndReason) {
        self.deliver_to_all(StreamEvent::Frame(Frame::End(reason)));
    }

    pub fn deliver_to_all(&self, event: StreamEvent) {
        for deliver in self.streams.lock().unwrap().drain(..) {
            deliver(event.clone());
        }
    }
}

pub(crate) struct Setup {
    pub link: Link,
    pub wire: Arc<Wire>,
    pub clock: ManualClock,
    _dir: tempfile::TempDir,
}

pub(crate) fn refreshed_models() -> Outcome {
    let mut models = fixture().value()["calls"]["models"]["result"].clone();
    models["items"].as_array_mut().unwrap().insert(
        0,
        serde_json::json!({"model": "openai/gpt-6.2", "name": "GPT-6.2"}),
    );
    Outcome::Done(models)
}

pub(crate) fn fixture() -> Fixture {
    Fixture::design()
}

/// A link paired with the fixture's server, its files in a temp dir, on a clock that moves when told.
pub(crate) fn paired(silent: bool) -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture();
    let store = Store::at(dir.path().to_path_buf());
    store
        .save_client(&ClientConfig {
            server: Some(f.url()),
            device: Some(f.device()),
        })
        .unwrap();
    let secrets = Secrets::memory();
    secrets.save(&f.url(), &f.token()).unwrap();
    linked(dir, secrets, silent)
}

/// A link over the fixture's server with `secrets`: unpaired unless they hold the token.
pub(crate) fn linked(dir: tempfile::TempDir, secrets: Secrets, silent: bool) -> Setup {
    let f = fixture();
    let clock = ManualClock::new(Now {
        mono: Mono(1_000_000),
        wall: Wall(f.value()["now"].as_i64().unwrap()),
    });
    let wire = Arc::new(Wire {
        fixture: FixtureTransport::new(f),
        silent: AtomicBool::new(silent),
        refuse: Mutex::new(None),
        model_refresh: Mutex::new(None),
        streams: Mutex::new(Vec::new()),
    });
    let config = LinkConfig {
        dir: Some(dir.path().to_path_buf()),
        secrets,
        clock: Arc::new(clock.clone()),
    };
    Setup {
        link: Link::with_transport(config, wire.clone()),
        wire,
        clock,
        _dir: dir,
    }
}

/// The app as launched on a device whose files are in `dir`, paired with the fixture's server at
/// `url`.
pub(crate) fn launch(dir: &std::path::Path, url: &str) -> crate::app::App {
    let store = Store::at(dir.to_path_buf());
    store
        .save_client(&ClientConfig {
            server: Some(url.into()),
            device: Some("laptop".into()),
        })
        .unwrap();
    let f = fixture();
    let secrets = Secrets::memory();
    secrets.save(url, &f.token()).unwrap();
    crate::app::App::launch(Link::with_transport(
        LinkConfig {
            dir: Some(dir.to_path_buf()),
            secrets,
            ..LinkConfig::standard()
        },
        Arc::new(FixtureTransport::new(f)),
    ))
}

/// Pumps until nothing changes, as frames would.
pub(crate) fn settle(link: &mut Link) {
    for _ in 0..30 {
        if !link.pump() {
            return;
        }
    }
    panic!("the link never settled");
}

impl Setup {
    pub fn clock_now(&self) -> Wall {
        use tau_client::clock::Clock;
        self.clock.now().wall
    }
}

/// The app in a headless window `size` points large, laid out once.
pub(crate) fn window_at(app: crate::app::App, size: [f32; 2]) -> rune::Runtime<crate::app::App> {
    let fonts =
        rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = rune::Runtime::with_text(
        app,
        crate::app::view,
        (size[0], size[1]),
        rune::Env::new(true, 1.0),
        fonts,
    );
    rt.frame(1.0);
    rt
}

/// The app in a 1512 × 982 headless window.
pub(crate) fn window(app: crate::app::App) -> rune::Runtime<crate::app::App> {
    window_at(app, [1512.0, 982.0])
}

/// A click at the centre of the element keyed `key`, and the frames that run its handler.
pub(crate) fn click(rt: &mut rune::Runtime<crate::app::App>, key: &str) {
    let e = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("no {key} on screen"));
    let (x, y) = (e.rect[0] + e.rect[2] / 2.0, e.rect[1] + e.rect[3] / 2.0);
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.02);
    rt.frame(0.02);
}

/// One node of the accessibility tree: its key, role, the name a reader says, and its bounds.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spoken {
    pub key: String,
    pub role: String,
    pub name: String,
    pub bounds: [f64; 4],
}

/// The accessibility tree, depth first, as the OS gets it.
pub(crate) fn spoken(rt: &rune::Runtime<crate::app::App>) -> Vec<Spoken> {
    fn walk(v: &serde_json::Value, out: &mut Vec<Spoken>) {
        let s = |k: &str| v[k].as_str().unwrap_or_default().to_string();
        out.push(Spoken {
            key: s("key"),
            role: s("role"),
            name: s("name"),
            bounds: std::array::from_fn(|i| v["bounds"][i].as_f64().unwrap_or_default()),
        });
        for c in v["children"].as_array().into_iter().flatten() {
            walk(c, out);
        }
    }
    let mut out = Vec::new();
    walk(&rt.accessibility_json(), &mut out);
    out
}
