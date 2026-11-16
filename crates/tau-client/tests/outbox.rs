mod common;

use common::*;
use serde_json::{Value, json};
use tau_client::engine::{Effect, LinkEvent, Setup};
use tau_client::ids::{ConvRef, DecisionRef, EntryId, Project};
use tau_client::outbox::{Next, NotSent, Outbox, OutboxEntry, Retry, Write, WriteCall, WriteState};
use tau_client::wire::{CallName, InputMode, Outcome, Unknown};

fn nc4() -> ConvRef {
    ConvRef::new("notes-cli", 4)
}

fn input(text: &str) -> Write {
    Write::Input {
        conv: nc4(),
        text: text.into(),
        mode: InputMode::Send,
    }
}

fn timeout(h: &mut Harness) {
    h.idle(20.0);
}

fn state(h: &Harness, id: tau_client::WriteId) -> WriteState {
    h.e.outbox().get(id).expect("entry").state.clone()
}

#[test]
fn input_bodies_follow_the_mode() {
    let (mut h, _) = Harness::connected();
    for (mode, when) in [
        (InputMode::Send, None),
        (InputMode::Steer, Some("steer")),
        (InputMode::FollowUp, Some("followUp")),
    ] {
        h.e.write(
            Write::Input {
                conv: nc4(),
                text: "t".into(),
                mode,
            },
            h.now,
        )
        .unwrap();
        let (_, body) = h.call(CallName::Input);
        assert_eq!(body.get("whenBusy").and_then(Value::as_str), when);
        assert!(body.get("whenIdle").is_none());
        assert_eq!(
            (&body["project"], &body["conversation"], &body["text"]),
            (&json!("notes-cli"), &json!(4), &json!("t"))
        );
    }
}

#[test]
fn request_ids_are_fresh_per_send() {
    let (mut h, _) = Harness::connected();
    h.e.write(input("one"), h.now).unwrap();
    h.e.write(input("two"), h.now).unwrap();
    h.e.write(
        Write::Continue {
            conv: ConvRef::new("render-core", 5),
        },
        h.now,
    )
    .unwrap();
    let mut ids: Vec<String> = h
        .calls()
        .iter()
        .map(|(_, r)| r.body["requestId"].as_str().unwrap().to_owned())
        .collect();
    for id in &ids {
        let ulid = id.strip_prefix("r-").expect("r- prefix");
        assert_eq!(ulid.len(), 26, "{id}");
        assert!(
            ulid.chars()
                .all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c)),
            "{id}"
        );
    }
    ids.dedup();
    assert_eq!(ids.len(), 3);
}

#[test]
fn a_resend_sends_the_same_bytes() {
    let (mut h, _) = Harness::connected();
    let id = h.e.write(input("hello"), h.now).unwrap();
    let (_, first) = h.call(CallName::Input);
    timeout(&mut h);
    assert!(matches!(
        state(&h, id),
        WriteState::Unknown {
            reason: Unknown::Timeout,
            ..
        }
    ));
    assert_eq!(h.e.outbox().unknown_count(), 1);
    h.secs(1.0);
    let (_, again) = h.call(CallName::Input);
    assert_eq!(
        serde_json::to_vec(&again).unwrap(),
        serde_json::to_vec(&first).unwrap()
    );
}

#[test]
fn the_resend_table() {
    use Retry::*;
    let table = [
        (WriteCall::Input, true, Yes),
        (WriteCall::Continue, true, Yes),
        (WriteCall::SetModel, true, NotRestored),
        (WriteCall::SetOptions, true, NotRestored),
        (WriteCall::Bind, true, Yes),
        (WriteCall::AddExtensions, true, Yes),
        (WriteCall::Answer, false, No),
        (WriteCall::Create, false, No),
        (WriteCall::Stop, false, No),
    ];
    for (call, automatic, retry) in table {
        let r = call.resends();
        assert_eq!((r.automatic, r.retry), (automatic, retry), "{call:?}");
    }
}

#[test]
fn the_resend_schedule() {
    let (mut h, _) = Harness::connected();
    let id = h.e.write(input("hello"), h.now).unwrap();
    let (call, _) = h.call(CallName::Input);
    h.unknown(call);
    for wait in [1.0, 2.0, 4.0, 8.0, 16.0, 30.0] {
        h.idle(wait - 0.05);
        h.no_calls(CallName::Input);
        h.idle(0.05);
        let (call, _) = h.call(CallName::Input);
        h.unknown(call);
    }
    h.idle(600.0);
    h.no_calls(CallName::Input);
    assert!(matches!(
        state(&h, id),
        WriteState::Unknown {
            next: Next::Manual,
            ..
        }
    ));
    assert!(h.e.outbox().get(id).unwrap().can_retry());
}

#[test]
fn unknown_while_offline_waits_for_a_connection() {
    let (mut h, c) = Harness::connected();
    h.opens();
    h.e.write(input("hello"), h.now).unwrap();
    let (call, _) = h.call(CallName::Input);
    h.cut(c.inbox);
    for id in c.timelines.values() {
        h.cut(*id);
    }
    h.unknown(call);
    h.secs(1.0);
    let opens = h.opens();
    h.no_calls(CallName::Input);
    let inbox = opens
        .iter()
        .find(|(_, r)| r.call == CallName::Inbox)
        .unwrap()
        .0;
    h.secs(5.0);
    h.no_calls(CallName::Input);
    h.frame(inbox, &fixture()["streams"]["inbox"]["first"]);
    h.call(CallName::Input);
}

#[test]
fn ten_minutes_ends_automatic_resends() {
    for (minutes, sent) in [(9, true), (11, false)] {
        let mut h = Harness::paired();
        let id = h.e.write(input("hello"), h.now).unwrap();
        assert!(matches!(
            state(&h, id),
            WriteState::Waiting {
                next: Next::AtConnection
            }
        ));
        h.secs(minutes as f64 * 60.0);
        h.calls();
        h.e.reconnect(h.now);
        h.connect();
        assert_eq!(
            !h.calls_named(CallName::Input).is_empty(),
            sent,
            "{minutes} minutes"
        );
        if !sent {
            assert!(matches!(
                state(&h, id),
                WriteState::Waiting { next: Next::Manual }
            ));
        }
    }
}

fn restored(call: &str, body: Value) -> OutboxEntry {
    serde_json::from_value(json!({
        "id": 1, "call": call, "body": body, "created": 0, "sent_at": 0, "tries": 0, "restored": false, "from_start": false,
        "state": {"Unknown": {"reason": "Timeout", "next": "Manual"}}
    }))
    .unwrap()
}

#[test]
fn restored_settings_cannot_be_retried() {
    let entries = vec![
        restored(
            "setModel",
            json!({"project": "render-core", "conversation": 5, "model": "anthropic/claude-opus-4-5"}),
        ),
        restored(
            "setOptions",
            json!({"project": "render-core", "extension": "w1", "patch": {}}),
        ),
        restored(
            "input",
            json!({"project": "notes-cli", "conversation": 4, "text": "x", "requestId": "r-1"}),
        ),
    ];
    let outbox = Outbox::restore(entries);
    let can: Vec<bool> = outbox
        .entries()
        .iter()
        .map(OutboxEntry::can_retry)
        .collect();
    assert_eq!(can, [false, false, true]);
    let mut h = Harness::new(Setup {
        outbox: outbox.entries().to_vec(),
        ..Harness::setup()
    });
    h.connect();
    let first = h.e.outbox().entries()[0].id;
    assert_eq!(h.e.retry(first, h.now), Err(NotSent::NoRetry));
    h.no_calls(CallName::SetModel);
}

#[test]
fn answers_creates_and_stops_are_never_resent() {
    let (mut h, _) = Harness::connected();
    let decision = DecisionRef::new("forecasts", "w2:2:pi-durable-2#1");
    let answer =
        h.e.write(
            Write::Answer {
                decision,
                basis: EntryId(17),
                choice: "no".into(),
                annotations: vec![],
            },
            h.now,
        )
        .unwrap();
    let stop = h.e.write(Write::Stop { conv: nc4() }, h.now).unwrap();
    h.calls();
    timeout(&mut h);
    h.idle(600.0);
    assert!(h.calls_named(CallName::Answer).is_empty() && h.calls_named(CallName::Stop).is_empty());
    let (a, s) = (
        h.e.outbox().get(answer).unwrap(),
        h.e.outbox().get(stop).unwrap(),
    );
    assert!(a.is_unknown() && s.is_unknown());
    assert!(!a.can_retry() && a.can_send_again());
    assert!(!s.can_retry() && !s.can_send_again());
    assert_eq!(h.e.retry(stop, h.now), Err(NotSent::NoRetry));
    assert_eq!(
        h.e.retry(answer, h.now),
        Ok(()),
        "␣ar sends the stored body"
    );
    assert_eq!(h.calls_named(CallName::Answer).len(), 1);
}

#[test]
fn retry_sends_once_now() {
    let (mut h, _) = Harness::connected();
    let id = h.e.write(input("hello"), h.now).unwrap();
    let (call, body) = h.call(CallName::Input);
    h.unknown(call);
    h.e.retry(id, h.now).unwrap();
    let (_, again) = h.call(CallName::Input);
    assert_eq!(again, body);
    assert_eq!(state(&h, id), WriteState::Sending);
    h.secs(0.5);
    h.no_calls(CallName::Input);
}

#[test]
fn done_leaves_after_a_minute_refused_stays() {
    let (mut h, _) = Harness::connected();
    let done = h.e.write(input("one"), h.now).unwrap();
    let refused = h.e.write(input("two"), h.now).unwrap();
    let calls = h.calls_named(CallName::Input);
    h.done(
        calls[0].0,
        &fixture()["calls"]["input steer notes-cli #4"]["result"],
    );
    h.reply(
        calls[1].0,
        outcome(&fixture()["refusals"]["too_large"]["reply"]),
    );
    let events = h.e.take_events();
    assert!(events.iter().any(
        |e| matches!(e, LinkEvent::Write { id, state: WriteState::Done { .. }, .. } if *id == done)
    ));
    assert!(events.iter().any(|e| matches!(e, LinkEvent::Write { id, state: WriteState::Refused(_), .. } if *id == refused)));
    h.idle(59.0);
    assert!(h.e.outbox().get(done).is_some());
    h.idle(2.0);
    assert!(h.e.outbox().get(done).is_none());
    h.idle(600.0);
    assert!(h.e.outbox().get(refused).is_some());
}

#[test]
fn forget_sends_nothing() {
    let (mut h, _) = Harness::connected();
    let id = h.e.write(input("hello"), h.now).unwrap();
    let (call, _) = h.call(CallName::Input);
    h.unknown(call);
    h.effects();
    h.e.forget(id, h.now);
    assert!(h.e.outbox().get(id).is_none());
    h.idle(60.0);
    let effects = h.effects();
    assert!(effects.contains(&Effect::SaveOutbox));
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::Call { request, .. } if request.call == CallName::Input))
    );
}

#[test]
fn a_late_reply_changes_nothing() {
    let (mut h, _) = Harness::connected();
    let id = h.e.write(input("hello"), h.now).unwrap();
    let (first, _) = h.call(CallName::Input);
    timeout(&mut h);
    h.idle(1.0);
    let (second, _) = h.call(CallName::Input);
    h.done(
        first,
        &fixture()["calls"]["input steer notes-cli #4"]["result"],
    );
    assert_eq!(state(&h, id), WriteState::Sending);
    h.e.take_events();
    h.done(
        second,
        &fixture()["calls"]["input steer notes-cli #4"]["result"],
    );
    assert!(matches!(state(&h, id), WriteState::Done { .. }));
    let settled =
        h.e.take_events()
            .into_iter()
            .filter(|e| matches!(e, LinkEvent::Write { .. }))
            .count();
    assert_eq!(settled, 1);
}

#[test]
fn persisted_and_restored() {
    let (mut h, _) = Harness::connected();
    h.effects();
    let sending = h.e.write(input("in flight"), h.now).unwrap();
    assert!(
        h.effects().contains(&Effect::SaveOutbox),
        "saved before it is sent"
    );
    let refused = h.e.write(input("refused"), h.now).unwrap();
    let (call, _) = h.call(CallName::Input);
    h.reply(call, outcome(&fixture()["refusals"]["too_large"]["reply"]));
    let saved = h.e.outbox().persisted();
    let json = serde_json::to_string(&saved).unwrap();
    let back: Vec<OutboxEntry> = serde_json::from_str(&json).unwrap();
    let outbox = Outbox::restore(back);
    let get = |id| outbox.get(id).unwrap();
    assert!(get(sending).restored);
    assert!(matches!(
        get(sending).state,
        WriteState::Unknown {
            reason: Unknown::Restart,
            next: Next::Manual
        }
    ));
    assert!(matches!(get(refused).state, WriteState::Refused(_)));
    let mut h = Harness::new(Setup {
        outbox: outbox.entries().to_vec(),
        ..Harness::setup()
    });
    h.connect();
    h.idle(120.0);
    h.no_calls(CallName::Input);
    let next = h.e.write(input("new"), h.now).unwrap();
    assert!(
        next.0 > sending.0.max(refused.0),
        "IDs don't repeat across launches"
    );
}

#[test]
fn offline_input_waits() {
    let mut h = Harness::paired();
    let id = h.e.write(input("hello"), h.now).unwrap();
    assert_eq!(
        state(&h, id),
        WriteState::Waiting {
            next: Next::AtConnection
        }
    );
    h.no_calls(CallName::Input);
    let (hs, _) = h.call(CallName::Handshake);
    h.reply(hs, outcome(&fixture()["server"]["discovery"]["reply"]));
    h.no_calls(CallName::Input);
    let (inbox, _) = h.open(CallName::Inbox);
    h.frame(inbox, &fixture()["streams"]["inbox"]["first"]);
    let (_, body) = h.call(CallName::Input);
    assert_eq!(body["text"], "hello");
}

#[test]
fn offline_writes_other_than_input_are_refused() {
    let mut h = Harness::paired();
    let decision = DecisionRef::new("forecasts", "w2:2:pi-durable-2#1");
    let writes = [
        Write::Stop { conv: nc4() },
        Write::Continue { conv: nc4() },
        Write::Answer {
            decision,
            basis: EntryId(17),
            choice: "no".into(),
            annotations: vec![],
        },
        Write::Bind {
            project: Project::from("scratch"),
            workdir: "/w".into(),
            extensions: vec![],
        },
    ];
    for w in writes {
        assert_eq!(h.e.write(w.clone(), h.now), Err(NotSent::Offline), "{w:?}");
    }
    assert!(h.e.outbox().entries().is_empty());
    let _ = Outcome::Done(json!({}));
}
