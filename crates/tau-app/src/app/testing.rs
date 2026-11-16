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
use tau_client::wire::{Outcome, Request};
use tau_client::{Fixture, Link, LinkConfig};

/// The fixture's server, which tests can silence, refuse every call to, or end a stream of.
pub(crate) struct Wire {
    fixture: FixtureTransport,
    pub silent: AtomicBool,
    pub refuse: Mutex<Option<Outcome>>,
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
            theme: None,
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
