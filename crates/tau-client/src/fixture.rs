//! `design/fixture.json` as a server, for the UI's demo mode, headless renders and tests: every call and stream is
//! answered in place from the fixture's replies and frames.

use crate::engine::StreamEvent;
use crate::sse::Frame;
use crate::transport::{Deliver, Reply, StreamHandle, Target, Transport};
use crate::wire::{CallName, Outcome, Refusal, RefusalCode, Request, Token, parse_reply};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The design fixture, parsed.
#[derive(Clone)]
pub struct Fixture {
    value: Arc<Value>,
}

impl Fixture {
    /// The fixture this crate was built with.
    pub fn design() -> Fixture {
        Fixture::parse(include_str!("../../../design/fixture.json"))
            .expect("the design fixture is JSON")
    }

    pub fn parse(text: &str) -> serde_json::Result<Fixture> {
        Ok(Fixture {
            value: Arc::new(serde_json::from_str(text)?),
        })
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn url(&self) -> String {
        self.value["server"]["url"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }

    pub fn device(&self) -> String {
        self.value["thisDevice"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }

    pub fn token(&self) -> Token {
        Token::new(
            self.value["pair"]["reply"]["result"]["token"]
                .as_str()
                .unwrap_or_default(),
        )
    }
}

/// Answers from a [`Fixture`], in place: a call's reply and a stream's first frame arrive before the call returns.
/// Writes answer with the shapes the server would send; nothing they do changes later answers.
pub struct FixtureTransport {
    fixture: Fixture,
    next: Mutex<HashMap<String, u64>>,
}

impl FixtureTransport {
    pub fn new(fixture: Fixture) -> FixtureTransport {
        FixtureTransport {
            fixture,
            next: Mutex::default(),
        }
    }

    fn answer(&self, request: &Request) -> Outcome {
        let f = self.fixture.value();
        let body = &request.body;
        let project = body["project"].as_str().unwrap_or_default();
        let named = |key: String, missing: Outcome| match f["calls"].get(&key) {
            Some(call) => Outcome::Done(call["result"].clone()),
            None => missing,
        };
        let no_project = || refused("unknown_project", format!("No project is named {project}."));
        match request.call {
            CallName::Handshake => envelope(&f["server"]["discovery"]["reply"]),
            CallName::Pair => envelope(&f["pair"]["reply"]),
            CallName::Extensions | CallName::Models | CallName::Projects => {
                Outcome::Done(f["calls"][request.call.path()]["result"].clone())
            }
            CallName::Options | CallName::Conversations | CallName::Faults => {
                named(format!("{} {project}", request.call.path()), no_project())
            }
            CallName::Decision => {
                let id = body["decision"].as_str().unwrap_or_default();
                match f["calls"]["decision"].get(format!("{project}/{id}")) {
                    Some(d) => Outcome::Done(d["result"].clone()),
                    None => refused(
                        "unknown_decision",
                        format!("{project} has no decision {id}."),
                    ),
                }
            }
            CallName::History => Outcome::Done(self.history(body)),
            CallName::Input | CallName::Continue => {
                let kind = request.call.path();
                let status = if request.call == CallName::Input {
                    "queued"
                } else {
                    "placed"
                };
                Outcome::Done(json!({"submission": {
                    "id": self.next("submission", 100), "conversationId": body["conversation"],
                    "requestId": format!("client:{}:{}", self.fixture.device(), body["requestId"].as_str().unwrap_or_default()),
                    "type": kind, "status": status,
                }}))
            }
            CallName::Stop | CallName::SetModel => Outcome::Done(json!({})),
            CallName::Create => {
                let rows = f["calls"][format!("conversations {project}")]["result"]["items"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                let newest = rows
                    .iter()
                    .filter_map(|r| r["conversation"].as_u64())
                    .max()
                    .unwrap_or(1);
                Outcome::Done(
                    json!({ "conversation": self.next(&format!("conversation {project}"), newest + 1) }),
                )
            }
            CallName::Answer => {
                let id = body["decision"].as_str().unwrap_or_default();
                let Some(d) = f["calls"]["decision"].get(format!("{project}/{id}")) else {
                    return refused("unknown_decision", format!("No decision has the ID {id}."));
                };
                let mut record = d["result"]["decision"].clone();
                record["status"] = "answered".into();
                record["choice"] = body["choice"].clone();
                record["annotations"] = body["annotations"].clone();
                record["by"] = self.fixture.device().into();
                record["at"] = f["now"].clone();
                Outcome::Done(json!({ "decision": record, "repeated": false }))
            }
            CallName::SetOptions => {
                let items = &f["calls"][format!("options {project}")]["result"]["items"];
                let item = items
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|i| i["extension"] == body["extension"]);
                let mut value = item
                    .map(|i| i["value"].clone())
                    .unwrap_or_else(|| json!({}));
                if let (Some(map), Some(patch)) = (value.as_object_mut(), body["patch"].as_object())
                {
                    for (key, v) in patch {
                        match v {
                            Value::Null => map.shift_remove(key),
                            v => map.insert(key.clone(), v.clone()),
                        };
                    }
                }
                Outcome::Done(json!({ "value": value }))
            }
            CallName::Bind | CallName::AddExtensions => {
                let known = f["calls"]["projects"]["result"]["items"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|p| p["project"] == project);
                let mut view = known.cloned().unwrap_or_else(|| json!({"project": project, "workdir": body["workdir"], "extensions": [], "missing": []}));
                let mut extensions: Vec<Value> =
                    view["extensions"].as_array().cloned().unwrap_or_default();
                for e in body["extensions"].as_array().into_iter().flatten() {
                    if !extensions.contains(e) {
                        extensions.push(e.clone());
                    }
                }
                view["extensions"] = Value::Array(extensions);
                match request.call {
                    CallName::Bind => {
                        Outcome::Done(json!({ "project": view, "repeated": known.is_some() }))
                    }
                    _ => Outcome::Done(json!({ "project": view })),
                }
            }
            CallName::Open | CallName::Inbox | CallName::Timeline => {
                refused("unknown_call", "Streams are not calls.".into())
            }
        }
    }

    fn next(&self, key: &str, first: u64) -> u64 {
        let mut next = self.next.lock().unwrap_or_else(|e| e.into_inner());
        let n = next.entry(key.to_owned()).or_insert(first);
        *n += 1;
        *n - 1
    }

    /// A conversation's entries as the fixture has them: its `open` frame's, or else its first input alone.
    fn entries(&self, project: &str, conversation: u64) -> Vec<Value> {
        let f = self.fixture.value();
        let view = &f["streams"][format!("open {project} #{conversation}")]["first"]["ops"][0][1];
        let log = &f["streams"][format!("timeline {project}")]["first"]["ops"][0][1];
        let from = if log["conversation"]["id"] == conversation {
            log
        } else {
            view
        };
        let mut entries = from["entries"].as_array().cloned().unwrap_or_default();
        let first = &f["local"]["firstInput"][project][conversation.to_string()];
        let has_input = entries.iter().any(|e| e["kind"] == "pi.user");
        if let (Some(text), false) = (first.as_str(), has_input) {
            let id = entries
                .iter()
                .filter_map(|e| e["id"].as_u64())
                .min()
                .map_or(1, |m| m.saturating_sub(1).max(1));
            let input = json!({"id": id, "conversationId": conversation, "kind": "pi.user",
                "model": [{"role": "user", "content": text, "timestamp": f["now"]}]});
            entries.insert(0, input);
        }
        entries
    }

    fn history(&self, body: &Value) -> Value {
        let (project, conversation) = (
            body["project"].as_str().unwrap_or_default(),
            body["conversation"].as_u64().unwrap_or(0),
        );
        let (min, max) = (
            body["minEntryId"].as_u64().unwrap_or(0),
            body["maxEntryId"].as_u64().unwrap_or(u64::MAX),
        );
        let mut items: Vec<Value> = self
            .entries(project, conversation)
            .into_iter()
            .filter(|e| (min..=max).contains(&e["id"].as_u64().unwrap_or(0)))
            .collect();
        if body["order"] != "ascending" {
            items.reverse();
        }
        items.truncate(body["limit"].as_u64().unwrap_or(50) as usize);
        json!({ "items": items })
    }

    fn first_frame(&self, request: &Request) -> Value {
        let f = self.fixture.value();
        let project = request.body["project"].as_str().unwrap_or_default();
        match request.call {
            CallName::Inbox => f["streams"]["inbox"]["first"].clone(),
            CallName::Timeline => f["streams"][format!("timeline {project}")]["first"].clone(),
            _ => {
                let conversation = request.body["conversation"].as_u64().unwrap_or(0);
                let first = &f["streams"][format!("open {project} #{conversation}")]["first"];
                if first.is_object() {
                    return first.clone();
                }
                let view = json!({"conversation": {"id": conversation}, "entries": self.entries(project, conversation),
                    "docs": {"pi.live": {}, "pi.inbox": {"items": []}}});
                json!({ "ops": [["r", view]] })
            }
        }
    }
}

impl Transport for FixtureTransport {
    fn call(&self, _target: &Target, request: Request, reply: Reply) {
        reply(self.answer(&request));
    }

    fn open(&self, _target: &Target, request: Request, deliver: Deliver) -> Box<dyn StreamHandle> {
        let frame = self.first_frame(&request);
        match frame["ops"].as_array() {
            Some(ops) => deliver(StreamEvent::Frame(Frame::Ops(ops.clone()))),
            None => deliver(StreamEvent::Refused(refused(
                "unknown_project",
                format!("No project is named {}.", request.body["project"]),
            ))),
        }
        Box::new(Quiet)
    }
}

/// A fixture stream: its first frame was delivered, and nothing else ever comes.
struct Quiet;

impl StreamHandle for Quiet {
    fn close(&self) {}
}

fn envelope(reply: &Value) -> Outcome {
    parse_reply(
        200,
        Some(crate::wire::PROTOCOL),
        &serde_json::to_vec(reply).unwrap_or_default(),
    )
}

fn refused(code: &str, message: String) -> Outcome {
    Outcome::Refused(Refusal {
        code: RefusalCode::parse(code),
        message,
        extra: Map::new(),
    })
}
