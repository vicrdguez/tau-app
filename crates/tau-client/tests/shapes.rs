mod common;

use common::fixture;
use serde_json::{Value, json};
use tau_client::ids::{DecisionId, Subject};
use tau_client::shapes::*;
use tau_client::wire::{ExtensionInfo, OptionsItem};

fn view(stream: &str) -> &'static Value {
    &fixture()["streams"][stream]["first"]["ops"][0][1]
}

fn entry(stream: &str, kind: &str) -> Entry {
    let raw = view(stream)["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == kind)
        .unwrap();
    Entry::parse(raw.clone()).expect(kind)
}

fn options(call: &str, extension: &str) -> OptionsItem {
    let items = &fixture()["calls"][call]["result"]["items"];
    let item = items
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["extension"] == extension)
        .unwrap();
    serde_json::from_value(item.clone()).unwrap()
}

#[test]
fn user_content_string_or_blocks() {
    let e = entry("open notes-cli #4", "pi.user");
    assert_eq!(
        e.user_text(),
        Some(
            "Port the config loader from JSON to TOML. Keep reading `notes.json` for one release and warn when it is used."
        )
    );
    assert_eq!(e.by_task, None);
    let blocks = json!({"id": 3, "conversationId": 2, "kind": "pi.user", "model": [{"role": "user", "timestamp": 1,
        "content": [{"type": "image", "data": "AAAA", "mimeType": "image/png"}, {"type": "text", "text": "What is this?\nA screenshot."}]}]});
    let e = Entry::parse(blocks).unwrap();
    assert_eq!(e.user_text(), Some("What is this?\nA screenshot."));
    let EntryBody::User { content, .. } = &e.body else {
        panic!()
    };
    assert_eq!(
        content[0],
        Content::Image {
            mime_type: "image/png".into()
        }
    );
}

#[test]
fn labels_follow_the_rule() {
    let hostile = &fixture()["hostile"]["firstInput"];
    assert_eq!(label(hostile["2"].as_str().unwrap()), "# Needs you");
    let long = label(hostile["3"].as_str().unwrap());
    assert_eq!(long.chars().count(), 80);
    assert!(long.ends_with('…'));
    assert!(
        hostile["3"]
            .as_str()
            .unwrap()
            .starts_with(long.trim_end_matches('…'))
    );
    assert_eq!(
        label("Design the retention flag."),
        "Design the retention flag."
    );
    assert_eq!(label("first line\nsecond"), "first line");
    assert_eq!(label("\n\n  indented first\nnext"), "indented first");
    assert_eq!(label(&"é".repeat(80)), "é".repeat(80));
}

#[test]
fn assistant_blocks() {
    let e = entry("open notes-cli #4", "pi.assistant");
    let EntryBody::Assistant(m) = &e.body else {
        panic!()
    };
    assert_eq!(
        (m.provider.as_str(), m.model.as_str()),
        ("anthropic", "claude-sonnet-4-5")
    );
    assert!(m.timestamp.is_some());
    assert!(
        m.blocks
            .iter()
            .any(|b| matches!(b, Block::ToolCall { name, .. } if name == "read"))
    );
    let thinking = json!({"id": 9, "conversationId": 2, "kind": "pi.assistant", "model": [{"role": "assistant", "api": "x",
        "provider": "openai", "model": "gpt-5.1", "usage": {}, "stopReason": "stop", "timestamp": 5,
        "content": [{"type": "thinking", "thinking": "Hmm."}, {"type": "text", "text": "Done."}]}]});
    let EntryBody::Assistant(m) = Entry::parse(thinking).unwrap().body else {
        panic!()
    };
    assert_eq!(
        m.blocks,
        vec![Block::Thinking("Hmm.".into()), Block::Text("Done.".into())]
    );
}

#[test]
fn tool_results() {
    let e = entry("open notes-cli #4", "pi.tool-result");
    let EntryBody::ToolResult {
        call_id,
        tool,
        is_error,
        text,
        ..
    } = &e.body
    else {
        panic!()
    };
    assert_eq!(
        (call_id.as_str(), tool.as_str(), *is_error),
        ("toolu_01", "read", false)
    );
    assert!(!text.is_empty());
    assert!(e.by_task.is_some());
    let hostile = &fixture()["hostile"]["open hostile #2"]["first"]["ops"][0][1]["entries"];
    let raw = hostile
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "pi.tool-result")
        .unwrap();
    let EntryBody::ToolResult { is_error, text, .. } = Entry::parse(raw.clone()).unwrap().body
    else {
        panic!()
    };
    assert!(is_error, "isError decides, not the text");
    assert!(text.starts_with("✓ exit 0"));
}

#[test]
fn built_in_kinds() {
    let e = entry("open notes-cli #4", "pi.system");
    let EntryBody::System { added, .. } = &e.body else {
        panic!()
    };
    assert_eq!(added, &["read", "write", "edit", "bash"]);
    let reset = json!({"id": 4, "conversationId": 2, "kind": "pi.reset"});
    assert_eq!(
        Entry::parse(reset).unwrap().body,
        EntryBody::Reset { handoff: None }
    );
    let handoff = json!({"id": 5, "conversationId": 2, "kind": "pi.reset", "model": [{"role": "user", "content": "Carry on from the plan.", "timestamp": 1}]});
    assert_eq!(
        Entry::parse(handoff).unwrap().body,
        EntryBody::Reset {
            handoff: Some("Carry on from the plan.".into())
        }
    );
    let compaction = json!({"id": 6, "conversationId": 2, "kind": "pi.compaction", "head": 3,
        "model": [{"role": "user", "content": [{"type": "text", "text": "Summary so far."}], "timestamp": 1}]});
    assert_eq!(
        Entry::parse(compaction).unwrap().body,
        EntryBody::Compaction {
            summary: Some("Summary so far.".into())
        }
    );
}

#[test]
fn records_take_string_text_only() {
    let design = view("open render-core #3")["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "w1.design")
        .unwrap();
    let e = Entry::parse(design.clone()).unwrap();
    assert!(matches!(&e.body, EntryBody::Record { text: Some(t) } if t.starts_with('#')));
    let odd = json!({"id": 7, "conversationId": 1, "kind": "w9.thing", "data": {"text": {"nested": true}}});
    assert_eq!(
        Entry::parse(odd).unwrap().body,
        EntryBody::Record { text: None }
    );
    let bare = json!({"id": 8, "conversationId": 1, "kind": "w9.thing"});
    assert_eq!(
        Entry::parse(bare).unwrap().body,
        EntryBody::Record { text: None }
    );
}

#[test]
fn entries_are_read_leniently() {
    let thin = json!({"id": 1, "conversationId": 2, "kind": "pi.assistant", "model": [{"role": "assistant", "content": [{"type": "toolCall"}, {"type": "mystery"}]}], "extra": 1});
    let EntryBody::Assistant(m) = Entry::parse(thin).unwrap().body else {
        panic!()
    };
    assert!(m.model.is_empty() && m.timestamp.is_none());
    assert!(
        Entry::parse(json!({"id": 1, "kind": "pi.user"})).is_none(),
        "no conversation"
    );
    assert!(
        Entry::parse(json!({"conversationId": 1, "kind": "pi.user"})).is_none(),
        "no id"
    );
    assert!(Entry::parse(json!({"id": 1, "conversationId": 1, "kind": "pi.user"})).is_some());
}

#[test]
fn live_state() {
    let docs = |s: &str| Docs::parse(&view(s)["docs"]);
    assert_eq!(
        docs("open notes-cli #4").live.state(),
        ConversationState::Running
    );
    assert_eq!(
        docs("open notes-cli #5").live.state(),
        ConversationState::Failed {
            reason: "model_error".into()
        }
    );
    assert_eq!(
        docs("open notes-cli #5")
            .live
            .run_failure
            .unwrap()
            .detail
            .as_deref(),
        Some("529 overloaded_error: Overloaded")
    );
    assert_eq!(
        docs("open render-core #3").live.state(),
        ConversationState::Idle
    );
    let nc4 = docs("open notes-cli #4");
    assert_eq!(nc4.live.run.as_ref().map(|r| r.turns), Some(6));
    assert_eq!(nc4.live.tools[0].status, "running");
    let rc7 = docs("open render-core #7");
    let message = rc7.live.generation.unwrap().message.unwrap();
    assert!(matches!(&message.blocks[0], Block::Text(t) if t.starts_with("Two facts")));
    let aborted = Docs::parse(
        &json!({"pi.live": {"runFailure": {"taskId": 3, "inputs": [2], "reason": "aborted"}}}),
    );
    assert_eq!(aborted.live.state(), ConversationState::Stopped);
}

#[test]
fn queue_items() {
    let nc4 = Docs::parse(&view("open notes-cli #4")["docs"]);
    assert_eq!(nc4.queue.len(), 1);
    assert_eq!(nc4.queue[0].mode, QueueMode::Steer);
    assert!(nc4.queue[0].text.contains("NOTES_CONFIG"));
    let blocks = Docs::parse(
        &json!({"pi.inbox": {"items": [{"id": 2, "mode": "followUp", "content": [{"type": "text", "text": "later"}]}]}}),
    );
    assert_eq!(
        (blocks.queue[0].mode.clone(), blocks.queue[0].text.as_str()),
        (QueueMode::FollowUp, "later")
    );
}

#[test]
fn agent_selection() {
    let agent = |s: &str| Docs::parse(&view(s)["docs"]).agent;
    assert_eq!(
        agent("open notes-cli #4").extensions,
        Selection::ProjectDefault
    );
    assert_eq!(
        agent("open notes-cli #4").model.as_deref(),
        Some("anthropic/claude-sonnet-4-5")
    );
    assert_eq!(
        agent("open notes-cli #4").thinking_level.as_deref(),
        Some("medium")
    );
    assert_eq!(
        agent("open render-core #3").extensions,
        Selection::Change {
            add: vec!["w1".into()],
            remove: vec![]
        }
    );
    assert_eq!(
        agent("open render-core #5").extensions,
        Selection::Only(vec!["coding-tools".into(), "w1".into()])
    );
    let catalogue: Vec<ExtensionInfo> =
        serde_json::from_value(fixture()["calls"]["extensions"]["result"]["items"].clone())
            .unwrap();
    assert_eq!(
        selection_workflow(&agent("open render-core #3").extensions, &catalogue),
        Some("w1")
    );
    assert_eq!(
        selection_workflow(&agent("open render-core #5").extensions, &catalogue),
        Some("w1")
    );
    assert_eq!(
        selection_workflow(&Selection::Only(vec!["websearch".into()]), &catalogue),
        None
    );
    assert_eq!(
        selection_workflow(&Selection::ProjectDefault, &catalogue),
        None
    );
}

#[test]
fn options_problems_name_missing_keys_from_the_schema() {
    let scratch = options_problems(&options("options scratch", "w1"));
    assert_eq!(scratch.missing, ["designer", "implementer", "reviewer"]);
    assert!(scratch.misfits.is_empty());
    let set = options_problems(&options("options render-core", "w1"));
    assert_eq!(set, OptionsProblems::default());
    let mut defaulted = options("options scratch", "w1");
    defaulted.errors.clear();
    assert_eq!(
        options_problems(&defaulted).missing,
        Vec::<String>::new(),
        "no errors, no problems"
    );
}

#[test]
fn options_problems_name_misfits_by_path() {
    let gone = options_problems(&options("options render-core (reviewer model gone)", "w1"));
    assert!(gone.missing.is_empty());
    assert_eq!(gone.misfits, ["reviewer"]);
}

#[test]
fn derived_names() {
    let id = DecisionId::from("w1:3:design#2");
    assert_eq!(id.subject(), Subject::from("w1:3:design"));
    assert_eq!(id.subject().extension(), "w1");
    assert_eq!(id.request_number(), Some(2));
    assert_eq!(
        DecisionId::from("w2:2:rune-release#12").request_number(),
        Some(12)
    );
    assert_eq!(event_extension("w1.event"), Some("w1"));
    assert_eq!(event_extension("w2.asked"), Some("w2"));
    assert_eq!(event_extension("plain"), None);
}
