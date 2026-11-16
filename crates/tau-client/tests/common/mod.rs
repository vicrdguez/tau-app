#![allow(dead_code)]

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::Duration;
use tau_client::clock::{Mono, Now, Wall};
use tau_client::engine::{CallId, Effect, Engine, Input, LinkStatus, Setup, StreamEvent};
use tau_client::ids::{Device, Project};
use tau_client::sse::{EndReason, Frame};
use tau_client::streams::{Phase, StreamId};
use tau_client::wire::{CallName, Outcome, Request, Unknown, parse_reply};

/// `design/fixture.json`: replies and frames in the protocol's shapes.
pub fn fixture() -> &'static Value {
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let path = format!("{}/../../design/fixture.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
    })
}

pub const URL: &str = "https://tau.tail4e2f.ts.net";
pub const PROJECTS: [&str; 3] = ["forecasts", "notes-cli", "render-core"];

/// A reply envelope from the fixture, read as the client reads replies.
pub fn outcome(envelope: &Value) -> Outcome {
    parse_reply(200, Some("1"), &serde_json::to_vec(envelope).unwrap())
}

pub fn result(v: &Value) -> Outcome {
    Outcome::Done(v.clone())
}

pub fn stream_first(name: &str) -> Value {
    fixture()["streams"][name]["first"].clone()
}

/// Drives an engine as the runtime would, with a clock that moves only when told.
pub struct Harness {
    pub e: Engine,
    pub now: Now,
    pending: Vec<Effect>,
}

/// The stream IDs of a connected harness.
pub struct Connected {
    pub inbox: StreamId,
    pub timelines: BTreeMap<String, StreamId>,
}

impl Harness {
    pub fn new(setup: Setup) -> Harness {
        let now = Now {
            mono: Mono(1_000_000),
            wall: Wall(fixture()["now"].as_i64().unwrap()),
        };
        Harness {
            e: Engine::new(setup, now),
            now,
            pending: Vec::new(),
        }
    }

    pub fn setup() -> Setup {
        Setup {
            server: Some((URL.into(), Device::from("laptop"))),
            seed: 7,
            ..Setup::default()
        }
    }

    pub fn paired() -> Harness {
        Harness::new(Harness::setup())
    }

    /// Paired, handshake done, catalogue answered, inbox and timelines at their first frames.
    pub fn connected() -> (Harness, Connected) {
        let mut h = Harness::paired();
        let c = h.connect();
        (h, c)
    }

    pub fn status(&self) -> LinkStatus {
        self.e.status(self.now)
    }

    fn collect(&mut self) {
        self.pending.extend(self.e.take_effects());
    }

    /// Every effect not looked at yet.
    pub fn effects(&mut self) -> Vec<Effect> {
        self.collect();
        std::mem::take(&mut self.pending)
    }

    /// Calls not looked at yet; other effects stay pending.
    pub fn calls(&mut self) -> Vec<(CallId, Request)> {
        self.collect();
        let mut calls = Vec::new();
        self.pending.retain(|e| match e {
            Effect::Call { id, request, .. } => {
                calls.push((*id, request.clone()));
                false
            }
            _ => true,
        });
        calls
    }

    pub fn calls_named(&mut self, name: CallName) -> Vec<(CallId, Value)> {
        self.collect();
        let mut calls = Vec::new();
        self.pending.retain(|e| match e {
            Effect::Call { id, request, .. } if request.call == name => {
                calls.push((*id, request.body.clone()));
                false
            }
            _ => true,
        });
        calls
    }

    pub fn call(&mut self, name: CallName) -> (CallId, Value) {
        let mut calls = self.calls_named(name);
        assert_eq!(calls.len(), 1, "one {name:?} call, got {calls:?}");
        calls.remove(0)
    }

    pub fn call_for(&mut self, name: CallName, project: &str) -> (CallId, Value) {
        self.collect();
        let at = self
            .pending
            .iter()
            .position(|e| matches!(e, Effect::Call { request, .. } if request.call == name && request.body["project"] == project))
            .unwrap_or_else(|| panic!("no {name:?} call for {project}: {:?}", self.pending));
        let Effect::Call { id, request, .. } = self.pending.remove(at) else {
            unreachable!()
        };
        (id, request.body)
    }

    /// How many calls of this name wait for a reply, without answering them.
    pub fn pending(&mut self, name: CallName) -> usize {
        self.collect();
        self.pending
            .iter()
            .filter(|e| matches!(e, Effect::Call { request, .. } if request.call == name))
            .count()
    }

    pub fn no_calls(&mut self, name: CallName) {
        let calls = self.calls_named(name);
        assert!(calls.is_empty(), "no {name:?} call expected, got {calls:?}");
    }

    pub fn opens(&mut self) -> Vec<(StreamId, Request)> {
        self.collect();
        let mut opens = Vec::new();
        self.pending.retain(|e| match e {
            Effect::Open { id, request } => {
                opens.push((*id, request.clone()));
                false
            }
            _ => true,
        });
        opens
    }

    pub fn open(&mut self, name: CallName) -> (StreamId, Value) {
        let mut opens: Vec<_> = self
            .opens()
            .into_iter()
            .filter(|(_, r)| r.call == name)
            .collect();
        assert_eq!(opens.len(), 1, "one {name:?} open, got {opens:?}");
        let (id, r) = opens.remove(0);
        (id, r.body)
    }

    pub fn closes(&mut self) -> Vec<StreamId> {
        self.collect();
        let mut closes = Vec::new();
        self.pending.retain(|e| match e {
            Effect::Close { id } => {
                closes.push(*id);
                false
            }
            _ => true,
        });
        closes
    }

    pub fn reply(&mut self, call: CallId, outcome: Outcome) {
        self.e.step(Input::Reply { call, outcome }, self.now);
    }

    pub fn done(&mut self, call: CallId, v: &Value) {
        self.reply(call, Outcome::Done(v.clone()));
    }

    pub fn unknown(&mut self, call: CallId) {
        self.reply(
            call,
            Outcome::Unknown(Unknown::Transport("connection reset".into())),
        );
    }

    pub fn stream(&mut self, id: StreamId, event: StreamEvent) {
        self.e.step(Input::Stream { id, event }, self.now);
    }

    pub fn frame(&mut self, id: StreamId, frame: &Value) {
        let f = match (frame.get("ops"), frame.get("end")) {
            (Some(ops), _) => Frame::Ops(ops.as_array().unwrap().clone()),
            (_, Some(end)) => Frame::End(EndReason::parse(end["reason"].as_str().unwrap())),
            _ => panic!("not a frame: {frame}"),
        };
        self.stream(id, StreamEvent::Frame(f));
    }

    pub fn end(&mut self, id: StreamId, reason: &str) {
        self.frame(id, &json!({"end": {"reason": reason}}));
    }

    pub fn heartbeat(&mut self, id: StreamId) {
        self.stream(id, StreamEvent::Frame(Frame::Heartbeat));
    }

    pub fn cut(&mut self, id: StreamId) {
        self.stream(id, StreamEvent::Closed(Some("connection reset".into())));
    }

    pub fn advance(&mut self, d: Duration) {
        self.now = self.now.after(d);
        self.e.step(Input::Tick, self.now);
    }

    pub fn secs(&mut self, s: f64) {
        self.advance(Duration::from_millis((s * 1000.0) as u64));
    }

    /// Lets time pass while the server keeps every live stream alive with heartbeats.
    pub fn idle(&mut self, s: f64) {
        let mut left = (s * 1000.0) as u64;
        while left > 0 {
            let step = left.min(10_000);
            self.advance(Duration::from_millis(step));
            left -= step;
            for (_, stream) in self.status().streams {
                if let Phase::Live { id, .. } = stream.phase {
                    self.heartbeat(id);
                }
            }
        }
    }

    /// Answers the handshake and the catalogue from the fixture, then gives every stream its first frame.
    pub fn connect(&mut self) -> Connected {
        let (id, body) = self
            .calls_named(CallName::Handshake)
            .pop()
            .expect("a handshake");
        assert_eq!(body, json!({}));
        self.reply(id, outcome(&fixture()["server"]["discovery"]["reply"]));
        let (projects, _) = self.call(CallName::Projects);
        self.done(projects, &fixture()["calls"]["projects"]["result"]);
        let (extensions, _) = self.call(CallName::Extensions);
        self.done(extensions, &fixture()["calls"]["extensions"]["result"]);
        let (models, _) = self.call(CallName::Models);
        self.done(models, &fixture()["calls"]["models"]["result"]);
        let opens = self.opens();
        let inbox = opens
            .iter()
            .find(|(_, r)| r.call == CallName::Inbox)
            .expect("inbox opened")
            .0;
        self.frame(inbox, &fixture()["streams"]["inbox"]["first"]);
        let mut timelines = BTreeMap::new();
        for (id, r) in opens.iter().filter(|(_, r)| r.call == CallName::Timeline) {
            let project = r.body["project"].as_str().unwrap().to_owned();
            self.frame(*id, &stream_first(&format!("timeline {project}")));
            timelines.insert(project, *id);
        }
        assert_eq!(timelines.len(), 3, "a timeline per project");
        Connected { inbox, timelines }
    }

    /// Answers every pending `conversations` and `faults` poll from the fixture.
    pub fn answer_polls(&mut self) {
        for (id, body) in self.calls_named(CallName::Conversations) {
            let p = body["project"].as_str().unwrap();
            self.done(
                id,
                &fixture()["calls"][format!("conversations {p}")]["result"],
            );
        }
        for (id, body) in self.calls_named(CallName::Faults) {
            let p = body["project"].as_str().unwrap();
            self.done(id, &fixture()["calls"][format!("faults {p}")]["result"]);
        }
    }
}

pub fn project(name: &str) -> Project {
    Project::from(name)
}

pub fn secs(s: u64) -> Duration {
    Duration::from_secs(s)
}
