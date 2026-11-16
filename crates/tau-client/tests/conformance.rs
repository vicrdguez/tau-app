//! `design/client.md` §9: what the client must show when fed `design/fixture.json`, for every row this crate owns.

mod common;

use common::*;
use serde_json::{Value, json};
use tau_client::TranscriptKey;
use tau_client::engine::{LinkEvent, LinkPhase, Setup, Slot, StreamEvent};
use tau_client::ids::{ConvRef, DecisionId, DecisionRef, EntryId, Project, WriteId};
use tau_client::model::{Gap, Known, Label, Liveness, Older};
use tau_client::outbox::{
    NotSent, OutboxEntry, Settlement, StartWith, Write, WriteCall, WriteState,
};
use tau_client::shapes::{ConversationState, EntryBody, label};
use tau_client::sse::EndReason;
use tau_client::streams::StreamKey;
use tau_client::wire::{CallName, InputMode, RefusalCode, RunState, Unknown};

fn f(path: &[&str]) -> &'static Value {
    path.iter().fold(fixture(), |v, k| &v[*k])
}

fn later(stream: &str, i: usize) -> Value {
    let frame = &fixture()["streams"][stream]["later"][i];
    frame.get("frame").unwrap_or(frame).clone()
}

// ---------------------------------------------------------------- launch and the inbox

#[test]
fn nothing_received_is_not_calm() {
    let mut h = Harness::paired();
    let (hs, _) = h.call(CallName::Handshake);
    h.reply(hs, outcome(f(&["server", "discovery", "reply"])));
    let (projects, _) = h.call(CallName::Projects);
    h.done(projects, f(&["calls", "projects", "result"]));
    assert_eq!(h.status().phase, LinkPhase::Connecting);
    assert!(
        !h.status().received_anything,
        "replies are not the server's state"
    );
    assert_eq!(h.e.server().inbox, Known::NotYet);
    assert_eq!(
        h.e.server().open_decisions(),
        None,
        "no ◆ tag before the inbox says"
    );
    for p in PROJECTS {
        assert!(
            matches!(
                h.e.server().conversations.get(&project(p)),
                None | Some(Known::NotYet)
            ),
            "{p}"
        );
        assert!(
            matches!(
                h.e.server().faults.get(&project(p)),
                None | Some(Known::NotYet)
            ),
            "{p}"
        );
    }
    let (inbox, _) = h.open(CallName::Inbox);
    h.frame(inbox, f(&["streams", "inbox", "first"]));
    assert_eq!(h.status().phase, LinkPhase::Live);
    assert_eq!(h.e.server().open_decisions(), Some(3));
    assert!(
        matches!(
            h.e.server().conversations.get(&project("render-core")),
            None | Some(Known::NotYet)
        ),
        "Failed and Running wait for the poll"
    );
    h.e.set_focused(true, h.now);
    h.answer_polls();
    assert!(matches!(
        h.e.server().conversations[&project("render-core")],
        Known::Have { .. }
    ));
    assert!(matches!(
        h.e.server().faults[&project("render-core")],
        Known::Have { .. }
    ));
}

#[test]
fn offline_after_sixty_seconds() {
    let mut h = Harness::paired();
    let (hs, _) = h.call(CallName::Handshake);
    h.unknown(hs);
    h.secs(59.0);
    assert_eq!(h.status().phase, LinkPhase::Connecting);
    h.secs(1.0);
    assert!(
        matches!(
            h.status().phase,
            LinkPhase::Offline {
                received_anything: false,
                ..
            }
        ),
        "{:?}",
        h.status().phase
    );

    let (mut h, c) = Harness::connected();
    let down = h.now.wall;
    h.cut(c.inbox);
    for id in c.timelines.values() {
        h.cut(*id);
    }
    h.secs(59.0);
    assert!(!matches!(h.status().phase, LinkPhase::Offline { .. }));
    h.secs(1.0);
    assert_eq!(
        h.status().phase,
        LinkPhase::Offline {
            since: down,
            received_anything: true
        }
    );
    assert_eq!(
        h.e.server().open_decisions(),
        Some(3),
        "the state as of then stays readable"
    );
}

#[test]
fn inbox_first_frame() {
    let (h, _) = Harness::connected();
    let inbox = h.e.server().inbox.value().unwrap();
    assert_eq!(inbox.count(), 3);
    let order: Vec<&str> = inbox
        .oldest_first()
        .iter()
        .map(|(_, d)| d.id.as_str())
        .collect();
    assert_eq!(
        order,
        [
            "w1:3:design#2",
            "w2:2:pi-durable-2#1",
            "w2:2:rune-release#2"
        ]
    );
}

#[test]
fn a_decision_answered_elsewhere_leaves() {
    let (mut h, c) = Harness::connected();
    h.e.take_events();
    h.frame(c.inbox, &later("inbox", 0));
    assert_eq!(h.e.server().open_decisions(), Some(2));
    let left = DecisionRef::new("forecasts", "w2:2:rune-release#2");
    assert_eq!(
        h.e.take_events(),
        vec![LinkEvent::DecisionLeft {
            decision: left,
            ours: false
        }]
    );
}

#[test]
fn superseded_while_open() {
    let (mut h, c) = Harness::connected();
    let v2 = DecisionRef::new("render-core", "w1:3:design#2");
    h.e.watch_decision(Some(v2.clone()), h.now);
    let (id, body) = h.call(CallName::Decision);
    assert_eq!(
        body,
        json!({"project": "render-core", "decision": "w1:3:design#2"})
    );
    h.done(
        id,
        f(&["calls", "decision", "render-core/w1:3:design#2", "result"]),
    );
    h.e.take_events();
    h.frame(c.inbox, &later("inbox", 2));
    let by = DecisionId::from("w1:3:design#3");
    assert!(h.e.take_events().contains(&LinkEvent::Superseded {
        decision: v2.clone(),
        by: by.clone()
    }));
    assert_eq!(
        h.e.server().inbox.value().unwrap().liveness(&v2),
        Liveness::Superseded { by }
    );
    assert_eq!(h.e.server().open_decisions(), Some(3));
}

// ---------------------------------------------------------------- open, ops, the gap rule

#[test]
fn open_frames_converge_to_the_fresh_state() {
    let (mut h, _) = Harness::connected();
    let nc4 = ConvRef::new("notes-cli", 4);
    h.e.show(Slot::Stage, Some(nc4.clone()), h.now);
    let (id, _) = h.open(CallName::Open);
    h.frame(id, &stream_first("open notes-cli #4"));
    let t = &h.e.server().transcripts[&nc4];
    assert_eq!(t.docs.value().unwrap().queue.len(), 1);
    assert_eq!(
        t.docs.value().unwrap().live.state(),
        ConversationState::Running
    );
    for i in 0..6 {
        h.frame(id, &later("open notes-cli #4", i));
    }
    let t = &h.e.server().transcripts[&nc4];
    let docs = t.docs.value().unwrap();
    assert!(docs.queue.is_empty(), "the steer left the queue");
    let steer = t.entries.values().last().unwrap();
    assert!(matches!(&steer.body, EntryBody::User { .. }));
    assert!(
        steer.user_text().unwrap().contains("NOTES_CONFIG"),
        "and appears as input"
    );
    let ok = t
        .entries
        .values()
        .find(|e| matches!(&e.body, EntryBody::ToolResult { call_id, .. } if call_id == "toolu_06"))
        .unwrap();
    assert!(
        matches!(
            ok.body,
            EntryBody::ToolResult {
                is_error: false,
                ..
            }
        ),
        "the tool block turns ok"
    );
    assert!(docs.live.tools.is_empty());
    let text = match &docs
        .live
        .generation
        .as_ref()
        .unwrap()
        .message
        .as_ref()
        .unwrap()
        .blocks[0]
    {
        tau_client::shapes::Block::Text(t) => t.clone(),
        other => panic!("{other:?}"),
    };
    assert!(text.starts_with("All four config tests pass. Now `NOTES_CONFIG`: it should win"));
    assert_eq!(t.entries.len(), 15);
}

#[test]
fn entries_outlive_the_window() {
    let (mut h, _) = Harness::connected();
    let nc4 = ConvRef::new("notes-cli", 4);
    h.e.show(Slot::Stage, Some(nc4.clone()), h.now);
    let (id, _) = h.open(CallName::Open);
    h.frame(id, &stream_first("open notes-cli #4"));
    h.frame(
        id,
        &json!({"ops": [["p", ["entries"], 0, 5, []], ["s", ["before"], 13]]}),
    );
    let t = &h.e.server().transcripts[&nc4];
    assert_eq!(t.entries.len(), 13);
    assert_eq!(t.before, Some(EntryId(13)));
    assert!(
        h.calls_named(CallName::History).is_empty(),
        "dropped entries were held: no gap"
    );
}

#[test]
fn gap_rule_fetches_once() {
    let gap = f(&["examples", "gap rule"]);
    let (mut h, _) = Harness::connected();
    let rc7 = ConvRef::new("render-core", 7);
    h.e.show(Slot::Stage, Some(rc7.clone()), h.now);
    let (id, _) = h.open(CallName::Open);
    h.frame(id, &stream_first("open render-core #7"));
    h.no_calls(CallName::History);
    h.e.show(Slot::Stage, None, h.now);
    h.e.show(Slot::Stage, Some(rc7.clone()), h.now);
    let (id, body) = h.open(CallName::Open);
    assert_eq!(body, gap["reopen"]);
    h.frame(id, &gap["first"]);
    let (page1, body) = h.call(CallName::History);
    assert_eq!(body, gap["history"][0]["request"]);
    assert_eq!(
        h.e.server().transcripts[&rc7].gap,
        Some(Gap {
            min: EntryId(46),
            max: EntryId(165)
        })
    );
    h.done(page1, &gap["history"][0]["result"]);
    let (page2, body) = h.call(CallName::History);
    assert_eq!(body, gap["history"][1]["request"]);
    h.done(page2, &gap["history"][1]["result"]);
    h.no_calls(CallName::History);
    let t = &h.e.server().transcripts[&rc7];
    assert_eq!(t.gap, None);
    assert_eq!(t.entries.len(), 6 + 160, "every entry once");
    let held: Vec<u64> = t.entries.keys().map(|k| k.0).collect();
    let mut sorted = held.clone();
    sorted.dedup();
    assert_eq!(held, sorted);
    h.frame(
        id,
        &json!({"ops": [["s", ["docs", "pi.live", "run", "turns"], 84]]}),
    );
    h.no_calls(CallName::History);
}

#[test]
fn gap_rule_skips_a_first_open() {
    let first = f(&["examples", "gap rule, first open"]);
    let (mut h, _) = Harness::connected();
    let rc3 = ConvRef::new("render-core", 3);
    h.e.show(Slot::Stage, Some(rc3.clone()), h.now);
    let (id, body) = h.open(CallName::Open);
    assert_eq!(body, first["request"]);
    h.frame(id, &first["first"]);
    h.no_calls(CallName::History);
    assert_eq!(h.e.server().transcripts[&rc3].entries.len(), 3);
}

#[test]
fn scrolling_back_keeps_the_cursor_bounds() {
    let first = f(&["examples", "gap rule, first open"]);
    let (mut h, _) = Harness::connected();
    let rc3 = ConvRef::new("render-core", 3);
    h.e.show(Slot::Stage, Some(rc3.clone()), h.now);
    let (id, _) = h.open(CallName::Open);
    h.frame(id, &first["first"]);
    let key = TranscriptKey::Conversation(rc3.clone());
    h.e.page_older(key.clone(), h.now);
    let (call, body) = h.call(CallName::History);
    let bounds = json!({"project": "render-core", "conversation": 3, "maxEntryId": 33, "order": "descending", "limit": 100});
    assert_eq!(body, bounds);
    h.e.page_older(key.clone(), h.now);
    h.no_calls(CallName::History);
    let older: Vec<Value> = f(&["streams", "open render-core #3", "first"])["ops"][0][1]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["id"].as_u64().unwrap() <= 33)
        .rev()
        .cloned()
        .collect();
    h.done(call, &json!({"items": older[..3], "next": "c1"}));
    assert!(matches!(
        h.e.server().transcripts[&rc3].older,
        Older::More { .. }
    ));
    h.e.page_older(key.clone(), h.now);
    let (call, body) = h.call(CallName::History);
    let mut with_cursor = bounds.clone();
    with_cursor["cursor"] = json!("c1");
    assert_eq!(body, with_cursor, "the same bounds with the cursor");
    h.done(call, &json!({"items": older[3..]}));
    assert_eq!(h.e.server().transcripts[&rc3].older, Older::Complete);
    h.e.page_older(key, h.now);
    h.no_calls(CallName::History);
    assert!(h.e.server().transcripts[&rc3].has_start());
}

#[test]
fn bad_cursor_on_scroll_back_starts_over() {
    let first = f(&["examples", "gap rule, first open"]);
    let (mut h, _) = Harness::connected();
    let rc3 = ConvRef::new("render-core", 3);
    h.e.show(Slot::Stage, Some(rc3.clone()), h.now);
    let (id, _) = h.open(CallName::Open);
    h.frame(id, &first["first"]);
    let key = TranscriptKey::Conversation(rc3);
    h.e.page_older(key.clone(), h.now);
    let (call, _) = h.call(CallName::History);
    h.done(call, &json!({"items": [], "next": "c1"}));
    h.e.page_older(key, h.now);
    let (call, _) = h.call(CallName::History);
    h.reply(call, outcome(&json!({"refused": {"code": "bad_cursor", "message": "This cursor was not made by this call."}})));
    let (_, body) = h.call(CallName::History);
    assert!(
        body.get("cursor").is_none(),
        "the cursor is dropped and paging starts again"
    );
}

// ---------------------------------------------------------------- polls

#[test]
fn conversations_read_every_page_newest_first() {
    let pages = f(&["calls", "conversations render-core (two pages)", "pages"]);
    let (mut h, _) = Harness::connected();
    h.e.set_focused(true, h.now);
    let (call, body) = h.call_for(CallName::Conversations, "render-core");
    assert_eq!(body, json!({"project": "render-core", "limit": 500}));
    h.done(call, &pages[0]["result"]);
    assert!(
        matches!(
            h.e.server().conversations.get(&project("render-core")),
            None | Some(Known::NotYet)
        ),
        "not before the last page"
    );
    let (call, body) = h.call_for(CallName::Conversations, "render-core");
    assert_eq!(
        body,
        json!({"project": "render-core", "limit": 500, "cursor": pages[0]["result"]["next"]})
    );
    h.done(call, &pages[1]["result"]);
    let rows = h.e.server().conversations[&project("render-core")]
        .value()
        .unwrap()
        .clone();
    let ids: Vec<u64> = rows.iter().map(|r| r.conversation.0).collect();
    assert_eq!(ids, [8, 7, 6, 5, 4, 3, 2]);
    assert_eq!(
        rows[0].owner.as_ref().unwrap().conversation.0,
        3,
        "#8 is owned by #3"
    );
    assert_eq!(rows[1].state, RunState::Running);
}

#[test]
fn bad_cursor_restarts_paging() {
    let pages = f(&["calls", "conversations render-core (two pages)", "pages"]);
    let (mut h, _) = Harness::connected();
    h.e.set_focused(true, h.now);
    let (call, _) = h.call_for(CallName::Conversations, "render-core");
    h.done(call, &pages[0]["result"]);
    let (call, _) = h.call_for(CallName::Conversations, "render-core");
    h.reply(call, outcome(&json!({"refused": {"code": "bad_cursor", "message": "This cursor was not made by this call."}})));
    let (call, body) = h.call_for(CallName::Conversations, "render-core");
    assert_eq!(body, json!({"project": "render-core", "limit": 500}));
    h.done(call, f(&["calls", "conversations render-core", "result"]));
    assert_eq!(
        h.e.server().conversations[&project("render-core")]
            .value()
            .unwrap()
            .len(),
        7,
        "no rows doubled"
    );
}

#[test]
fn polls_follow_focus() {
    let (mut h, _) = Harness::connected();
    h.idle(60.0);
    h.no_calls(CallName::Conversations);
    h.no_calls(CallName::Faults);
    h.e.set_focused(true, h.now);
    assert_eq!(h.pending(CallName::Conversations), 3);
    assert_eq!(h.pending(CallName::Faults), 3);
    h.idle(0.1);
    h.answer_polls();
    h.idle(9.8);
    h.no_calls(CallName::Conversations);
    h.idle(0.2);
    assert_eq!(h.pending(CallName::Conversations), 3);
    h.no_calls(CallName::Faults);
    h.answer_polls();
    h.e.set_focused(false, h.now);
    h.idle(400.0);
    h.no_calls(CallName::Conversations);
    h.no_calls(CallName::Faults);
    h.e.set_focused(true, h.now);
    assert_eq!(h.pending(CallName::Faults), 3, "at once on focus");
}

#[test]
fn faults_poll_every_five_minutes() {
    let (mut h, _) = Harness::connected();
    h.e.set_focused(true, h.now);
    h.answer_polls();
    let mut seen = 0;
    for _ in 0..30 {
        h.idle(10.0);
        seen += h.pending(CallName::Faults);
        h.answer_polls();
    }
    assert_eq!(seen, 3, "one round in five minutes");
}

#[test]
fn a_poll_follows_this_devices_writes() {
    let (mut h, _) = Harness::connected();
    h.e.set_focused(true, h.now);
    h.answer_polls();
    h.secs(3.0);
    h.e.write(
        Write::Input {
            conv: ConvRef::new("notes-cli", 4),
            text: "go".into(),
            mode: InputMode::Send,
        },
        h.now,
    )
    .unwrap();
    let (call, _) = h.call(CallName::Input);
    h.done(call, f(&["calls", "input steer notes-cli #4", "result"]));
    h.secs(0.9);
    h.no_calls(CallName::Conversations);
    h.secs(0.1);
    let polled = h.calls_named(CallName::Conversations);
    assert_eq!(polled.len(), 1);
    assert_eq!(polled[0].1["project"], "notes-cli");
}

#[test]
fn faults_and_reconnects_refetch() {
    let (mut h, c) = Harness::connected();
    h.e.set_focused(true, h.now);
    h.answer_polls();
    h.e.refresh_faults(h.now);
    assert_eq!(h.pending(CallName::Faults), 3, "Home or Faults opened");
    h.answer_polls();
    h.cut(c.inbox);
    h.secs(1.0);
    let (inbox, _) = h.open(CallName::Inbox);
    h.frame(inbox, f(&["streams", "inbox", "first"]));
    assert_eq!(
        h.pending(CallName::Conversations),
        3,
        "at once on reconnect"
    );
    assert_eq!(h.pending(CallName::Faults), 3);
    for name in [CallName::Projects, CallName::Extensions, CallName::Models] {
        h.call(name);
    }
}

// ---------------------------------------------------------------- labels

#[test]
fn labels_are_found_once_and_cached() {
    let (mut h, _) = Harness::connected();
    let rc5 = ConvRef::new("render-core", 5);
    h.effects();
    h.e.want_label(rc5.clone(), h.now);
    let (call, body) = h.call(CallName::History);
    assert_eq!(
        body,
        json!({"project": "render-core", "conversation": 5, "order": "ascending", "limit": 10})
    );
    let found = f(&["calls", "history first input render-core #5", "result"]);
    h.done(call, found);
    let first = found["items"][0]["model"][0]["content"].as_str().unwrap();
    assert_eq!(h.e.server().label(&rc5), Label::Found(label(first)));
    assert!(
        h.effects()
            .contains(&tau_client::engine::Effect::SaveLabels)
    );
    h.e.want_label(rc5, h.now);
    h.no_calls(CallName::History);

    let empty = ConvRef::new("render-core", 9);
    h.e.want_label(empty.clone(), h.now);
    for page in 0..5 {
        let (call, body) = h.call(CallName::History);
        assert_eq!(body.get("cursor").is_some(), page > 0);
        h.done(call, &json!({"items": [{"id": 50 + page, "conversationId": 9, "kind": "pi.system"}], "next": format!("c{page}")}));
    }
    h.no_calls(CallName::History);
    assert_eq!(h.e.server().label(&empty), Label::Empty);
    h.idle(30.0);
    h.e.want_label(empty.clone(), h.now);
    h.no_calls(CallName::History);
    h.idle(31.0);
    h.e.want_label(empty, h.now);
    h.call(CallName::History);
}

#[test]
fn labels_come_from_the_start_only() {
    let (mut h, _) = Harness::connected();
    let rc3 = ConvRef::new("render-core", 3);
    h.e.show(Slot::Stage, Some(rc3.clone()), h.now);
    let (id, _) = h.open(CallName::Open);
    h.frame(id, &f(&["examples", "gap rule, first open"])["first"]);
    assert_eq!(h.e.server().label(&rc3), Label::NotYet);
    let nc4 = ConvRef::new("notes-cli", 4);
    h.e.show(Slot::Peek, Some(nc4.clone()), h.now);
    let (id, _) = h.open(CallName::Open);
    h.frame(id, &stream_first("open notes-cli #4"));
    let want = label(
        f(&["local", "firstInput", "notes-cli", "4"])
            .as_str()
            .unwrap(),
    );
    assert_eq!(h.e.server().label(&nc4), Label::Found(want));
}

#[test]
fn gone_from_the_server() {
    let (mut h, _) = Harness::connected();
    h.e.fetch_options(Project::from("nope"), h.now);
    let (call, _) = h.call(CallName::Options);
    h.reply(call, outcome(&json!({"refused": {"code": "unknown_project", "message": "No project is named nope."}})));
    assert_eq!(
        h.e.server().options[&project("nope")],
        Known::Gone {
            message: "No project is named nope.".into()
        }
    );
    let c99 = ConvRef::new("render-core", 99);
    h.e.show(Slot::Stage, Some(c99.clone()), h.now);
    let (id, _) = h.open(CallName::Open);
    h.stream(
        id,
        StreamEvent::Refused(outcome(f(&["refusals", "unknown_conversation", "reply"]))),
    );
    assert!(matches!(
        h.e.server().transcripts[&c99].docs,
        Known::Gone { .. }
    ));
    h.idle(120.0);
    assert!(
        h.opens().iter().all(|(_, r)| r.call != CallName::Open),
        "a gone conversation is not reopened"
    );
    h.e.fetch_decision(DecisionRef::new("render-core", "w1:3:design#9"), h.now);
    let (call, _) = h.call(CallName::Decision);
    h.reply(call, outcome(&json!({"refused": {"code": "unknown_decision", "message": "render-core has no decision w1:3:design#9."}})));
    assert!(matches!(
        h.e.server().decisions[&DecisionRef::new("render-core", "w1:3:design#9")],
        Known::Gone { .. }
    ));
}

// ---------------------------------------------------------------- ends and refusals

#[test]
fn revoked_ends_everything() {
    let (mut h, c) = Harness::connected();
    h.e.show(Slot::Stage, Some(ConvRef::new("notes-cli", 4)), h.now);
    h.opens();
    h.e.take_events();
    h.frame(c.inbox, f(&["streams", "ends", "revoked"]));
    assert_eq!(h.status().phase, LinkPhase::Revoked);
    assert!(h.e.take_events().contains(&LinkEvent::Revoked));
    assert_eq!(
        h.closes().len(),
        4,
        "the timelines and the open stream close"
    );
    h.e.set_focused(true, h.now);
    h.e.reconnect(h.now);
    h.secs(600.0);
    assert!(h.opens().is_empty(), "no reconnect attempt");
    assert!(h.calls().is_empty());
    let w = Write::Input {
        conv: ConvRef::new("notes-cli", 4),
        text: "x".into(),
        mode: InputMode::Send,
    };
    assert_eq!(h.e.write(w, h.now), Err(NotSent::Revoked));
    assert_eq!(
        h.e.server().open_decisions(),
        Some(3),
        "views stay as they were"
    );
}

#[test]
fn unauthorized_on_a_call_is_not_revoked() {
    let (mut h, c) = Harness::connected();
    h.e.set_focused(true, h.now);
    let (call, _) = h.call_for(CallName::Conversations, "render-core");
    h.reply(
        call,
        outcome(f(&["refusals", "unauthorized (revoked)", "reply"])),
    );
    assert_eq!(
        h.status().phase,
        LinkPhase::Refused {
            message: "Send a valid device token.".into()
        }
    );
    assert!(!h.status().writes_on);
    let id =
        h.e.write(
            Write::Input {
                conv: ConvRef::new("notes-cli", 4),
                text: "held".into(),
                mode: InputMode::Send,
            },
            h.now,
        )
        .unwrap();
    h.no_calls(CallName::Input);
    assert_eq!(
        h.e.write(
            Write::Stop {
                conv: ConvRef::new("notes-cli", 4)
            },
            h.now
        ),
        Err(NotSent::Refused)
    );
    h.opens();
    h.cut(c.inbox);
    h.secs(1.0);
    h.open(CallName::Inbox);
    assert_ne!(h.status().phase, LinkPhase::Revoked);
    h.answer_polls();
    h.secs(10.0);
    h.answer_polls();
    assert!(
        !matches!(h.status().phase, LinkPhase::Refused { .. }),
        "a call that succeeds lifts the band"
    );
    let (_, body) = h.call(CallName::Input);
    assert_eq!(body["text"], "held");
    assert!(h.e.outbox().get(id).is_some());
}

#[test]
fn stopping_backs_off_from_one_second() {
    let (mut h, c) = Harness::connected();
    h.opens();
    h.e.take_events();
    h.frame(c.inbox, f(&["streams", "ends", "stopping"]));
    for id in c.timelines.values() {
        h.frame(*id, f(&["streams", "ends", "stopping"]));
    }
    assert!(h.e.take_events().contains(&LinkEvent::StreamEnded {
        key: StreamKey::Inbox,
        reason: EndReason::Stopping
    }));
    assert_eq!(h.status().phase, LinkPhase::ServerStopped);
    assert_eq!(h.e.server().open_decisions(), Some(3), "the ◆ count stays");
    h.secs(0.95);
    assert!(h.opens().is_empty());
    h.secs(0.05);
    assert_eq!(h.opens().len(), 4);
}

// ---------------------------------------------------------------- the start page

const START: &str = "Design a `--since <entry>` flag for `tau export --events`, so an export can resume where the last one stopped.";

fn start() -> Write {
    Write::Start {
        project: Project::from("render-core"),
        with: StartWith::Workflow("w1".into()),
        text: START.into(),
    }
}

#[test]
fn start_creates_then_inputs() {
    let (mut h, _) = Harness::connected();
    let create = h.e.write(start(), h.now).unwrap();
    assert!(h.e.outbox().starting());
    let (call, body) = h.call(CallName::Create);
    assert_eq!(
        body,
        f(&["calls", "create w1 render-core", "request"]).clone()
    );
    h.no_calls(CallName::Input);
    h.done(call, f(&["calls", "create w1 render-core", "result"]));
    assert!(!h.e.outbox().starting());
    let conv = ConvRef::new("render-core", 9);
    let events = h.e.take_events();
    let input = events
        .iter()
        .find_map(|e| match e {
            LinkEvent::Started {
                conv: c,
                with,
                create: cr,
                input,
            } if *c == conv && *cr == create => {
                assert_eq!(with, &StartWith::Workflow("w1".into()));
                Some(*input)
            }
            _ => None,
        })
        .expect("started");
    let (_, body) = h.call(CallName::Input);
    let want = f(&["calls", "input start render-core #9", "request"]);
    assert_eq!(
        (&body["project"], &body["conversation"], &body["text"]),
        (&want["project"], &want["conversation"], &want["text"])
    );
    assert!(body.get("whenBusy").is_none());
    assert_eq!(h.e.outbox().get(input).unwrap().call, WriteCall::Input);
    assert_eq!(h.e.outbox().unsent_inputs(&conv).len(), 1);
}

#[test]
fn one_press_one_create() {
    let (mut h, _) = Harness::connected();
    h.e.write(start(), h.now).unwrap();
    assert_eq!(h.e.write(start(), h.now), Err(NotSent::Starting));
    assert_eq!(h.calls_named(CallName::Create).len(), 1);
}

#[test]
fn a_lost_create_is_never_resent() {
    let (mut h, _) = Harness::connected();
    let id = h.e.write(start(), h.now).unwrap();
    h.call(CallName::Create);
    h.idle(20.0);
    assert!(!h.e.outbox().starting());
    assert_eq!(h.e.outbox().unknown_count(), 1);
    h.idle(600.0);
    h.no_calls(CallName::Create);
    h.no_calls(CallName::Input);
    assert!(!h.e.outbox().get(id).unwrap().can_retry());
    assert_eq!(h.e.retry(id, h.now), Err(NotSent::NoRetry));
}

#[test]
fn a_good_start_forgets_the_lost_create() {
    let (mut h, _) = Harness::connected();
    let lost = h.e.write(start(), h.now).unwrap();
    h.call(CallName::Create);
    h.idle(20.0);
    h.e.write(start(), h.now).unwrap();
    let (call, _) = h.call(CallName::Create);
    h.done(call, f(&["calls", "create w1 render-core", "result"]));
    assert!(h.e.outbox().get(lost).is_none());
    assert_eq!(h.e.outbox().unknown_count(), 0);
}

#[test]
fn a_refused_create_sends_no_input() {
    let (mut h, _) = Harness::connected();
    let id = h.e.write(start(), h.now).unwrap();
    let (call, _) = h.call(CallName::Create);
    h.reply(
        call,
        outcome(f(&[
            "refusals",
            "options_invalid (create w1 in render-core, a model the server no longer has)",
            "reply",
        ])),
    );
    h.no_calls(CallName::Input);
    let WriteState::Refused(r) = &h.e.outbox().get(id).unwrap().state else {
        panic!()
    };
    assert_eq!(r.code, RefusalCode::OptionsInvalid);
    assert_eq!(
        r.errors()[0].path,
        "/reviewer",
        "the Project view opens at reviewer"
    );
    assert!(
        h.e.take_events()
            .iter()
            .any(|e| matches!(e, LinkEvent::Write { id: i, .. } if *i == id))
    );

    h.e.write(start(), h.now).unwrap();
    let (call, _) = h.call(CallName::Create);
    h.reply(
        call,
        outcome(f(&[
            "refusals",
            "extension_not_loaded (create w1 while the server lacks w1)",
            "reply",
        ])),
    );
    h.call(CallName::Extensions);
    h.call(CallName::Projects);
}

#[test]
fn start_checks_the_text_first() {
    let (mut h, _) = Harness::connected();
    for text in ["", "  \n\t"] {
        let w = Write::Start {
            project: Project::from("render-core"),
            with: StartWith::Workflow("w1".into()),
            text: text.into(),
        };
        assert_eq!(h.e.write(w, h.now), Err(NotSent::Empty));
    }
    let big = "x".repeat(1024 * 1024);
    let w = Write::Start {
        project: Project::from("notes-cli"),
        with: StartWith::Model("anthropic/claude-sonnet-4-5".into()),
        text: big,
    };
    assert_eq!(h.e.write(w, h.now), Err(NotSent::TooLarge));
    h.no_calls(CallName::Create);
    assert!(h.e.outbox().entries().is_empty());
}

#[test]
fn a_bare_start_names_its_model() {
    let (mut h, _) = Harness::connected();
    let w = Write::Start {
        project: Project::from("notes-cli"),
        with: StartWith::Model("anthropic/claude-sonnet-4-5".into()),
        text: "hi".into(),
    };
    h.e.write(w, h.now).unwrap();
    let (_, body) = h.call(CallName::Create);
    assert_eq!(
        body,
        f(&["calls", "create bare notes-cli", "request"]).clone()
    );
}

// ---------------------------------------------------------------- answers

fn lost_answer() -> (Harness, Connected, WriteId, DecisionRef) {
    let (mut h, c) = Harness::connected();
    let decision = DecisionRef::new("forecasts", "w2:2:pi-durable-2#1");
    let req = f(&[
        "calls",
        "answer forecasts/w2:2:pi-durable-2#1 (resent after no reply)",
        "request",
    ]);
    let w = Write::Answer {
        decision: decision.clone(),
        basis: EntryId(req["basis"].as_u64().unwrap()),
        choice: "no".into(),
        annotations: vec![],
    };
    let id = h.e.write(w, h.now).unwrap();
    let (_, body) = h.call(CallName::Answer);
    assert_eq!(&body, req);
    h.idle(20.0);
    assert_eq!(h.e.outbox().unknown_count(), 1, "? 1");
    assert!(
        h.e.outbox().answer_unsettled(&decision).is_some(),
        "␣ad is off"
    );
    h.idle(120.0);
    h.no_calls(CallName::Answer);
    (h, c, id, decision)
}

#[test]
fn a_lost_answer_settles_as_landed() {
    let (mut h, c, id, decision) = lost_answer();
    h.e.take_events();
    h.frame(
        c.inbox,
        &json!({"ops": [["d", ["forecasts", "w2:2:pi-durable-2"]]]}),
    );
    let (call, body) = h.call(CallName::Decision);
    assert_eq!(
        body,
        json!({"project": "forecasts", "decision": "w2:2:pi-durable-2#1"})
    );
    let record = f(&[
        "calls",
        "answer forecasts/w2:2:pi-durable-2#1 (resent after no reply)",
        "result",
        "decision",
    ]);
    let text = f(&[
        "calls",
        "decision",
        "forecasts/w2:2:pi-durable-2#1",
        "result",
        "text",
    ]);
    h.done(call, &json!({"decision": record, "text": text}));
    let entry = h.e.outbox().get(id).unwrap();
    assert!(
        matches!(entry.state, WriteState::Settled(Settlement::Landed(_))),
        "{:?}",
        entry.state
    );
    assert_eq!(h.e.outbox().unknown_count(), 0, "? gone");
    assert!(!entry.can_retry() && !entry.can_send_again(), "no ␣rt");
    let events = h.e.take_events();
    assert!(events.contains(&LinkEvent::DecisionLeft {
        decision: decision.clone(),
        ours: true
    }));
    assert!(events.iter().any(|e| matches!(e, LinkEvent::Write { id: i, state: WriteState::Settled(Settlement::Landed(_)), .. } if *i == id)));
    assert!(h.e.outbox().answer_unsettled(&decision).is_none());
}

#[test]
fn a_lost_answer_answered_elsewhere() {
    let (mut h, c, id, _) = lost_answer();
    h.frame(
        c.inbox,
        &json!({"ops": [["d", ["forecasts", "w2:2:pi-durable-2"]]]}),
    );
    let (call, _) = h.call(CallName::Decision);
    let mut record = f(&[
        "calls",
        "answer forecasts/w2:2:pi-durable-2#1 (resent after no reply)",
        "result",
        "decision",
    ])
    .clone();
    record["by"] = json!("phone");
    record["choice"] = json!("yes");
    h.done(call, &json!({"decision": record, "text": ""}));
    assert!(matches!(
        h.e.outbox().get(id).unwrap().state,
        WriteState::Settled(Settlement::AnsweredElsewhere(_))
    ));
}

#[test]
fn a_lost_answer_not_landed() {
    let (mut h, c, id, decision) = lost_answer();
    h.frame(c.inbox, &fixture()["streams"]["inbox"]["later"][1]["frame"]);
    assert!(
        h.e.outbox().get(id).unwrap().is_unknown(),
        "a later frame of the same stream proves nothing"
    );
    h.opens();
    h.cut(c.inbox);
    h.secs(1.0);
    let (inbox, _) = h.open(CallName::Inbox);
    h.frame(inbox, f(&["streams", "inbox", "first"]));
    assert_eq!(
        h.e.outbox().get(id).unwrap().state,
        WriteState::Settled(Settlement::NotLanded)
    );
    assert_eq!(h.e.outbox().unknown_count(), 0);
    assert!(
        h.e.outbox().answer_unsettled(&decision).is_none(),
        "␣ad is live again"
    );
}

#[test]
fn one_answer_at_a_time() {
    let (mut h, _) = Harness::connected();
    let d = DecisionRef::new("forecasts", "w2:2:pi-durable-2#1");
    let answer = |d: &DecisionRef| Write::Answer {
        decision: d.clone(),
        basis: EntryId(14),
        choice: "no".into(),
        annotations: vec![],
    };
    h.e.write(answer(&d), h.now).unwrap();
    assert_eq!(h.e.write(answer(&d), h.now), Err(NotSent::AnswerUnsettled));
    h.idle(20.0);
    assert_eq!(h.e.write(answer(&d), h.now), Err(NotSent::AnswerUnsettled));
    h.e.write(
        answer(&DecisionRef::new("forecasts", "w2:2:rune-release#2")),
        h.now,
    )
    .unwrap();
}

#[test]
fn an_answer_refetches_its_decision() {
    let (mut h, _) = Harness::connected();
    let d = DecisionRef::new("render-core", "w1:3:design#2");
    h.e.watch_decision(Some(d.clone()), h.now);
    let (call, _) = h.call(CallName::Decision);
    h.done(
        call,
        f(&["calls", "decision", "render-core/w1:3:design#2", "result"]),
    );
    let req = f(&["calls", "answer render-core/w1:3:design#2", "request"]);
    let annotations = serde_json::from_value(req["annotations"].clone()).unwrap();
    let w = Write::Answer {
        decision: d.clone(),
        basis: EntryId(req["basis"].as_u64().unwrap()),
        choice: "revise".into(),
        annotations,
    };
    h.e.write(w, h.now).unwrap();
    let (call, body) = h.call(CallName::Answer);
    assert_eq!(&body, req);
    h.done(
        call,
        f(&["calls", "answer render-core/w1:3:design#2", "result"]),
    );
    let view = h.e.server().decisions[&d].value().unwrap();
    assert_eq!(
        view.decision.choice.as_deref(),
        Some("revise"),
        "the reply's record at once"
    );
    h.secs(0.9);
    h.no_calls(CallName::Decision);
    h.secs(0.1);
    h.call(CallName::Decision);
}

// ---------------------------------------------------------------- the outbox across launches

fn local_outbox_entry() -> OutboxEntry {
    let local = &fixture()["local"]["outbox"][1];
    serde_json::from_value(json!({
        "id": 1, "call": local["call"], "body": local["body"], "created": local["sentAt"], "sent_at": local["sentAt"],
        "tries": 0, "restored": false, "from_start": false,
        "state": {"Unknown": {"reason": "Timeout", "next": {"At": 0}}}
    }))
    .unwrap()
}

#[test]
fn restored_entries_wait_for_retry() {
    let mut h = Harness::new(Setup {
        outbox: vec![local_outbox_entry()],
        ..Harness::setup()
    });
    h.connect();
    h.idle(300.0);
    h.no_calls(CallName::Input);
    let entry = h.e.outbox().entries()[0].clone();
    assert!(
        entry.restored && entry.is_unknown() && entry.can_retry(),
        "not sent automatically · ␣rt retries"
    );
    h.e.retry(entry.id, h.now).unwrap();
    let (_, body) = h.call(CallName::Input);
    assert_eq!(body, fixture()["local"]["outbox"][1]["body"]);
}

#[test]
fn an_unknown_input_is_resent_at_the_next_connection() {
    let (mut h, c) = Harness::connected();
    h.opens();
    let body = fixture()["local"]["outbox"][1]["body"].clone();
    let w = Write::Input {
        conv: ConvRef::new("notes-cli", 4),
        text: body["text"].as_str().unwrap().into(),
        mode: InputMode::FollowUp,
    };
    let id = h.e.write(w, h.now).unwrap();
    let (call, sent) = h.call(CallName::Input);
    h.cut(c.inbox);
    for t in c.timelines.values() {
        h.cut(*t);
    }
    h.unknown(call);
    assert_eq!(
        h.e.outbox()
            .unsent_inputs(&ConvRef::new("notes-cli", 4))
            .len(),
        1,
        "the unsent row"
    );
    h.secs(1.0);
    h.no_calls(CallName::Input);
    let (inbox, _) = h.open(CallName::Inbox);
    h.frame(inbox, f(&["streams", "inbox", "first"]));
    let (call, again) = h.call(CallName::Input);
    assert_eq!(again["requestId"], sent["requestId"]);
    h.done(call, &json!({"submission": {"id": 33, "conversationId": 4, "requestId": "client:laptop:x", "type": "input", "status": "queued"}}));
    assert!(matches!(
        h.e.outbox().get(id).unwrap().state,
        WriteState::Done { .. }
    ));
    h.idle(60.0);
    h.no_calls(CallName::Input);
}

#[test]
fn a_timed_out_answer_waits_without_a_reply() {
    let (h, _, id, _) = lost_answer();
    assert!(matches!(
        h.e.outbox().get(id).unwrap().state,
        WriteState::Unknown {
            reason: Unknown::Timeout,
            ..
        }
    ));
}
