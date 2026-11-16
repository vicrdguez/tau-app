mod common;

use common::*;
use serde_json::json;
use tau_client::engine::{LinkPhase, Notice, Slot, StreamEvent};
use tau_client::ids::ConvRef;
use tau_client::outbox::{NotSent, Write};
use tau_client::sse::EndReason;
use tau_client::streams::StreamKey;
use tau_client::wire::{CallName, InputMode, Outcome, Unknown};
use tau_client::{LinkEvent, Project};

#[test]
fn the_handshake_comes_first() {
    let mut h = Harness::paired();
    let calls = h.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(
        (calls[0].1.call, &calls[0].1.body),
        (CallName::Handshake, &json!({}))
    );
    assert!(h.opens().is_empty());
    assert_eq!(h.status().phase, LinkPhase::Connecting);
    h.reply(
        calls[0].0,
        outcome(&fixture()["server"]["discovery"]["reply"]),
    );
    assert_eq!(
        h.opens()
            .iter()
            .filter(|(_, r)| r.call == CallName::Inbox)
            .count(),
        1
    );
    for name in [CallName::Projects, CallName::Extensions, CallName::Models] {
        h.call(name);
    }
}

#[test]
fn another_protocol_stops_everything() {
    let mut h = Harness::paired();
    let (id, _) = h.call(CallName::Handshake);
    h.reply(id, Outcome::Unknown(Unknown::OtherProtocol("2".into())));
    assert_eq!(h.status().phase, LinkPhase::OtherProtocol("2".into()));
    h.secs(600.0);
    assert!(h.calls().is_empty() && h.opens().is_empty());
    let conv = ConvRef::new("notes-cli", 4);
    assert_eq!(
        h.e.write(
            Write::Input {
                conv,
                text: "hi".into(),
                mode: InputMode::Send
            },
            h.now
        ),
        Err(NotSent::OtherProtocol)
    );
}

#[test]
fn an_unreachable_server_is_asked_again_on_backoff() {
    let mut h = Harness::paired();
    let (id, _) = h.call(CallName::Handshake);
    h.unknown(id);
    h.secs(0.9);
    h.no_calls(CallName::Handshake);
    h.secs(0.1);
    let (id, _) = h.call(CallName::Handshake);
    h.unknown(id);
    h.secs(1.9);
    h.no_calls(CallName::Handshake);
    h.secs(0.1);
    h.call(CallName::Handshake);
}

#[test]
fn silence_reopens_the_stream() {
    let (mut h, c) = Harness::connected();
    h.opens();
    h.secs(44.9);
    assert!(h.closes().is_empty());
    h.secs(0.2);
    let closes = h.closes();
    assert!(closes.contains(&c.inbox), "{closes:?}");
    let (reopened, body) = h.open(CallName::Inbox);
    assert_ne!(reopened, c.inbox);
    assert_eq!(body, json!({}));
}

#[test]
fn heartbeats_keep_a_stream_alive() {
    let (mut h, c) = Harness::connected();
    h.opens();
    h.secs(30.0);
    h.heartbeat(c.inbox);
    h.secs(30.0);
    assert!(!h.closes().contains(&c.inbox));
    h.secs(16.0);
    assert!(h.closes().contains(&c.inbox));
}

#[test]
fn backoff_steps_and_resets() {
    let (mut h, c) = Harness::connected();
    h.opens();
    h.cut(c.inbox);
    let mut id = c.inbox;
    for wait in [1.0, 2.0, 4.0, 8.0, 16.0, 30.0, 30.0] {
        h.secs(wait - 0.05);
        assert!(
            h.opens().iter().all(|(_, r)| r.call != CallName::Inbox),
            "nothing before {wait} s"
        );
        h.secs(0.05);
        let (next, _) = h.open(CallName::Inbox);
        assert_ne!(next, id);
        id = next;
        h.cut(id);
    }
    h.secs(30.0);
    let (id, _) = h.open(CallName::Inbox);
    h.frame(id, &fixture()["streams"]["inbox"]["first"]);
    h.cut(id);
    h.secs(1.0);
    h.open(CallName::Inbox);
}

#[test]
fn busy_inbox_is_a_stopping_server() {
    let (mut h, c) = Harness::connected();
    h.opens();
    h.cut(c.inbox);
    h.secs(1.0);
    let (id, _) = h.open(CallName::Inbox);
    let busy = json!({"refused": {"code": "busy", "message": "The server is stopping."}});
    h.stream(id, StreamEvent::Refused(outcome(&busy)));
    assert_eq!(h.status().phase, LinkPhase::ServerStopped);
    h.secs(2.0);
    h.open(CallName::Inbox);
}

#[test]
fn three_failures_in_a_row_are_noticed() {
    let (mut h, c) = Harness::connected();
    h.opens();
    let key = StreamKey::Timeline(Project::from("render-core"));
    let first = stream_first("timeline render-core");
    let mut id = c.timelines["render-core"];
    for n in 1..=3 {
        assert!(
            !h.status()
                .notices
                .contains(&Notice::KeepsFailing(key.clone())),
            "not before failure {n}"
        );
        h.end(id, "failed");
        assert!(h.e.take_events().contains(&LinkEvent::StreamEnded {
            key: key.clone(),
            reason: EndReason::Failed
        }));
        h.secs(1.0);
        id = h.open(CallName::Timeline).0;
        h.frame(id, &first);
    }
    assert!(
        h.status()
            .notices
            .contains(&Notice::KeepsFailing(key.clone()))
    );
    h.cut(id);
    assert!(
        !h.status().notices.contains(&Notice::KeepsFailing(key)),
        "a cut breaks the run"
    );

    let mut id = c.inbox;
    for _ in 0..3 {
        h.end(id, "failed");
        h.secs(1.0);
        id = h.open(CallName::Inbox).0;
        h.frame(id, &fixture()["streams"]["inbox"]["first"]);
    }
    assert!(
        h.status()
            .notices
            .contains(&Notice::KeepsFailing(StreamKey::Inbox))
    );
}

#[test]
fn a_stale_stream_changes_nothing() {
    let (mut h, _) = Harness::connected();
    let nc4 = ConvRef::new("notes-cli", 4);
    h.e.show(Slot::Stage, Some(nc4.clone()), h.now);
    let (old, _) = h.open(CallName::Open);
    h.e.show(Slot::Stage, Some(ConvRef::new("render-core", 3)), h.now);
    assert!(h.closes().contains(&old));
    h.frame(old, &stream_first("open notes-cli #4"));
    assert!(
        h.e.server()
            .transcripts
            .get(&nc4)
            .is_none_or(|t| t.entries.is_empty())
    );
}

#[test]
fn a_bad_frame_reopens_the_stream() {
    let (mut h, _) = Harness::connected();
    let nc4 = ConvRef::new("notes-cli", 4);
    h.e.show(Slot::Stage, Some(nc4.clone()), h.now);
    let (id, _) = h.open(CallName::Open);
    h.frame(id, &stream_first("open notes-cli #4"));
    h.frame(id, &json!({"ops": [["s", ["docs", "__proto__", "x"], 1]]}));
    assert!(h.closes().contains(&id));
    h.secs(1.0);
    let (_, body) = h.open(CallName::Open);
    assert_eq!(
        body,
        json!({"project": "notes-cli", "conversation": 4, "after": 21})
    );
    assert_eq!(
        h.e.server().transcripts[&nc4].entries.len(),
        13,
        "held entries stay"
    );
}

#[test]
fn show_holds_one_stream_per_conversation() {
    let (mut h, _) = Harness::connected();
    h.opens();
    let c4 = ConvRef::new("notes-cli", 4);
    h.e.show(Slot::Stage, Some(c4.clone()), h.now);
    let (stage, body) = h.open(CallName::Open);
    assert_eq!(body, json!({"project": "notes-cli", "conversation": 4}));
    h.e.show(Slot::Peek, Some(c4), h.now);
    assert!(h.opens().is_empty(), "the peek shares the stage's stream");
    h.e.show(Slot::Peek, Some(ConvRef::new("notes-cli", 5)), h.now);
    let (peek, _) = h.open(CallName::Open);
    h.e.show(Slot::Stage, None, h.now);
    assert_eq!(h.closes(), vec![stage]);
    h.e.show(Slot::Peek, None, h.now);
    assert_eq!(h.closes(), vec![peek]);
}

#[test]
fn reopen_asks_after_the_newest_held() {
    let (mut h, c) = Harness::connected();
    h.opens();
    let c4 = ConvRef::new("notes-cli", 4);
    h.e.show(Slot::Stage, Some(c4.clone()), h.now);
    let (id, _) = h.open(CallName::Open);
    h.frame(id, &stream_first("open notes-cli #4"));
    h.e.show(Slot::Stage, None, h.now);
    h.e.show(Slot::Stage, Some(c4), h.now);
    let (_, body) = h.open(CallName::Open);
    assert_eq!(body["after"], 21);
    h.cut(c.timelines["render-core"]);
    h.secs(1.0);
    let (_, body) = h.open(CallName::Timeline);
    let log = &fixture()["streams"]["timeline render-core"]["first"]["ops"][0][1]["entries"];
    let newest = log.as_array().unwrap().last().unwrap()["id"].clone();
    assert_eq!(body, json!({"project": "render-core", "after": newest}));
}

#[test]
fn reconnect_reopens_now() {
    let (mut h, c) = Harness::connected();
    h.opens();
    h.cut(c.inbox);
    h.secs(0.5);
    h.e.reconnect(h.now);
    let opens = h.opens();
    assert_eq!(
        opens
            .iter()
            .filter(|(_, r)| r.call == CallName::Inbox)
            .count(),
        1
    );
    assert_eq!(
        opens
            .iter()
            .filter(|(_, r)| r.call == CallName::Timeline)
            .count(),
        3,
        "live streams reopen too"
    );
    assert_eq!(h.closes().len(), 3);
}
