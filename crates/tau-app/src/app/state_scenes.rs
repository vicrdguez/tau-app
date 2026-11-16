//! Connection-state scenes driven through a fixture transport. The link, bands, actions and
//! outbox agree because the scenes deliver real stream endings and lost write replies.
use super::{
    App, Stage, link,
    scenes::{self, Scene},
};
use crate::open::Item;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tau_client::clock::{ManualClock, Mono, Now, Wall};
use tau_client::engine::StreamEvent;
use tau_client::fixture::FixtureTransport;
use tau_client::sse::{EndReason, Frame};
use tau_client::store::Secrets;
use tau_client::transport::{Deliver, Reply, StreamHandle, Target, Transport};
use tau_client::wire::{CallName, InputMode, Outcome, Request, Unknown};
use tau_client::{ConvRef, DecisionRef, EntryId, Fixture, Link, LinkConfig, Write};

pub const SCENES: &[Scene] = &[
    ("server-stopped", |a| {
        let (wire, _) = prepare(a, true);
        a.screens.conversation.clock =
            Some(Wall(Fixture::design().value()["now"].as_i64().unwrap()));
        scenes::on_fixture(
            a,
            Stage::Item(Item::Conversation {
                project: "notes-cli".into(),
                id: 4,
            }),
        );
        a.sidebar_visible = false;
        wire.end("stopping");
        a.live = true;
        link::pump(a, &mut rune::Tx::default());
        a.live = false;
    }),
    ("revoked", |a| {
        let (wire, _) = prepare(a, false);
        named(a, "decision-long");
        wire.end("revoked");
        link::settle(a);
    }),
    ("unknown-outcome", unknown),
    ("model-picker", |a| named(a, "start-workflow-picker")),
];

fn named(a: &mut App, name: &str) {
    scenes::all()
        .iter()
        .find(|(n, _)| *n == name)
        .expect("scene exists")
        .1(a);
}

fn unknown(a: &mut App) {
    let (wire, clock) = prepare(a, true);
    let f = Fixture::design();
    a.screens.conversation.clock = Some(Wall(f.value()["now"].as_i64().unwrap()));
    scenes::on_fixture(
        a,
        Stage::Item(Item::Conversation {
            project: "notes-cli".into(),
            id: 4,
        }),
    );
    for entry in f.value()["local"]["outbox"].as_array().unwrap() {
        wire.unknown
            .store(entry["state"] == "unknown", Ordering::SeqCst);
        let body = &entry["body"];
        a.link
            .write(Write::Input {
                conv: ConvRef::new(
                    body["project"].as_str().unwrap(),
                    body["conversation"].as_u64().unwrap(),
                ),
                text: body["text"].as_str().unwrap().into(),
                mode: if body["whenBusy"] == "steer" {
                    InputMode::Steer
                } else {
                    InputMode::FollowUp
                },
            })
            .unwrap();
        a.link.pump();
    }
    let decision = DecisionRef::new("render-core", "w1:3:design#2");
    a.link
        .write(Write::Answer {
            decision,
            basis: EntryId(34),
            choice: "revise".into(),
            annotations: Vec::new(),
        })
        .unwrap();
    a.link.pump();
    wire.disconnected.store(true, Ordering::SeqCst);
    wire.deliver(StreamEvent::Closed(None));
    a.link.pump();
    for seconds in [1, 2] {
        clock.advance(Duration::from_secs(seconds));
        a.link.pump();
        a.link.pump();
    }
    link::refresh(a);
    crate::screens::places::floats::outbox(a);
}

fn prepare(a: &mut App, streaming: bool) -> (Arc<Wire>, ManualClock) {
    let mut value = Fixture::design().value().clone();
    if streaming {
        let later: Vec<_> = value["streams"]["open notes-cli #4"]["later"]
            .as_array()
            .unwrap()
            .iter()
            .take(2)
            .flat_map(|f| f["ops"].as_array().cloned().unwrap_or_default())
            .collect();
        value["streams"]["open notes-cli #4"]["first"]["ops"]
            .as_array_mut()
            .unwrap()
            .extend(later);
    }
    let f = Fixture::parse(&value.to_string()).unwrap();
    let clock = ManualClock::new(Now {
        mono: Mono(1_000_000),
        wall: Wall(value["now"].as_i64().unwrap()),
    });
    let wire = Arc::new(Wire {
        fixture: FixtureTransport::new(f.clone()),
        streams: Mutex::default(),
        unknown: AtomicBool::new(false),
        disconnected: AtomicBool::new(false),
    });
    a.link = Link::with_transport(
        LinkConfig {
            dir: None,
            secrets: Secrets::memory(),
            clock: Arc::new(clock.clone()),
        },
        wire.clone(),
    );
    a.link.pair(&f.url(), "K7QM-3XRT");
    link::settle(a);
    a.live = false;
    (wire, clock)
}

struct Wire {
    fixture: FixtureTransport,
    streams: Mutex<Vec<Arc<Deliver>>>,
    unknown: AtomicBool,
    disconnected: AtomicBool,
}
struct Held;
impl StreamHandle for Held {
    fn close(&self) {}
}
impl Wire {
    fn deliver(&self, event: StreamEvent) {
        for deliver in self.streams.lock().unwrap().drain(..) {
            deliver(event.clone());
        }
    }
    fn end(&self, name: &str) {
        let f = Fixture::design();
        let reason = f.value()["streams"]["ends"][name]["end"]["reason"]
            .as_str()
            .unwrap();
        self.deliver(StreamEvent::Frame(Frame::End(EndReason::parse(reason))));
    }
}
impl Transport for Wire {
    fn call(&self, target: &Target, request: Request, reply: Reply) {
        if self.unknown.load(Ordering::SeqCst)
            && matches!(request.call, CallName::Input | CallName::Answer)
        {
            reply(Outcome::Unknown(Unknown::Timeout));
        } else {
            self.fixture.call(target, request, reply);
        }
    }
    fn open(&self, target: &Target, request: Request, deliver: Deliver) -> Box<dyn StreamHandle> {
        if self.disconnected.load(Ordering::SeqCst) {
            deliver(StreamEvent::Closed(None));
            return Box::new(Held);
        }
        let deliver = Arc::new(deliver);
        self.streams.lock().unwrap().push(deliver.clone());
        self.fixture
            .open(target, request, Box::new(move |event| deliver(event)));
        Box::new(Held)
    }
}
