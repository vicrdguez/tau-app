//! How the transcript could fail (`spec.md` §5.5, `visual-language.md` §8):
//! - an input is headed anything but `input`, or loses its time, or a task's input is told apart
//!   from yours (D3);
//! - consecutive agent entries get a header each, or an agent entry after an input or a record
//!   joins the turn before it;
//! - a `pi.tool-result` is drawn on its own;
//! - a tool row's outcome comes from its output's words, or from another call's result, instead
//!   of `isError` and `pi.live.tools`;
//! - a call with neither a result nor a live slot reads anything but `no result` while no run is
//!   live;
//! - a single `ok` row folds, two adjacent `ok` rows don't, a fold swallows a failed, running or
//!   no-result row, or folds `ok` rows across text;
//! - a running row's count starts anywhere but its message's timestamp, or opens with more than
//!   the last 6 lines of its output, or starts closed;
//! - `za` on a row or a fold does nothing, or opening a fold hides its rows' own stops;
//! - a tool input loses extra fields or multiline strings, duplicates a complete summary,
//!   loses JSON field order, or treats an empty object as hidden input;
//! - a thinking block reads the wrong number of words;
//! - a record with no string `data.text` gets a body, or loses `by a task`;
//! - `pi.system` reads other than `tools: …`; `pi.reset` and `pi.compaction` vanish;
//! - the live generation is missing, or becomes a cursor stop;
//! - a run failure shows while a run is live, or an aborted run shows as failed;
//! - the gap line sits anywhere but where the missing entries are;
//! - Markdown splits so that a list item loses its number, a code fence breaks in two, or blank
//!   text makes a stop.

use std::collections::{BTreeMap, HashMap};

use serde_json::{Value, json};
use tau_client::model::{Gap, Transcript as Held};
use tau_client::shapes::{Docs, Entry};
use tau_client::{EntryId, Known, TaskId, Wall};

use super::*;

fn held(entries: Vec<Value>, docs: Value) -> Held {
    let entries: BTreeMap<EntryId, Entry> = entries
        .into_iter()
        .map(|v| {
            let e = Entry::parse(v).expect("an entry");
            (e.id, e)
        })
        .collect();
    Held {
        entries,
        docs: Known::Have {
            value: Docs::parse(&docs),
            at: Wall(0),
        },
        ..Held::default()
    }
}

fn user(id: u64, text: &str, by_task: Option<u64>) -> Value {
    let mut e = json!({"id": id, "conversationId": 1, "kind": "pi.user",
        "model": [{"role": "user", "content": text, "timestamp": 1000 * id}]});
    if let Some(t) = by_task {
        e["byTaskId"] = t.into();
    }
    e
}

fn agent(id: u64, content: Value) -> Value {
    json!({"id": id, "conversationId": 1, "kind": "pi.assistant", "model": [{"role": "assistant",
        "content": content, "provider": "anthropic", "model": "claude-sonnet-4-5",
        "stopReason": "toolUse", "timestamp": 1000 * id}]})
}

fn call(id: &str, name: &str, args: Value) -> Value {
    json!({"type": "toolCall", "id": id, "name": name, "arguments": args})
}

fn result(id: u64, call: &str, is_error: bool, text: &str) -> Value {
    json!({"id": id, "conversationId": 1, "kind": "pi.tool-result", "model": [{"role": "toolResult",
        "toolCallId": call, "toolName": "bash", "content": [{"type": "text", "text": text}],
        "isError": is_error, "timestamp": 1000 * id}]})
}

fn record(id: u64, kind: &str, data: Value, by_task: Option<u64>) -> Value {
    let mut e = json!({"id": id, "conversationId": 1, "kind": kind, "data": data});
    if let Some(t) = by_task {
        e["byTaskId"] = t.into();
    }
    e
}

fn idle() -> Value {
    json!({"pi.live": {}})
}

fn running(tools: Value) -> Value {
    json!({"pi.live": {"run": {"taskId": 8, "turns": 6}, "tools": tools}})
}

#[test]
fn non_object_tool_input_is_available_when_the_summary_exceeds_the_row() {
    for (args, input) in [
        (Value::Null, None),
        (json!(true), None),
        (json!(123), None),
        (json!([1, 2]), None),
        (json!("x".repeat(46)), None),
        (
            json!("x".repeat(47)),
            Some(format!("\"{}\"", "x".repeat(47))),
        ),
        (
            json!(["x".repeat(45)]),
            Some(format!("[\n  \"{}\"\n]", "x".repeat(45))),
        ),
    ] {
        let h = held(
            vec![agent(1, json!([call("call", "custom", args.clone())]))],
            idle(),
        );
        let items = build(&h, &HashMap::from([(Stop::Tool("call".into()), true)]));
        let row = turn_parts(&items)
            .into_iter()
            .find_map(|p| match p {
                Part::Tool(row) => Some(row),
                _ => None,
            })
            .unwrap();
        assert!(row.open);
        assert_eq!(row.input, input, "arguments {args}");
    }
}

fn build_all(h: &Held) -> Vec<Item> {
    build(h, &HashMap::new())
}

fn turn_parts(items: &[Item]) -> Vec<&Part> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Turn { parts, .. } => Some(parts.iter()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn tool_outcomes(items: &[Item]) -> Vec<(String, Outcome)> {
    let mut out = Vec::new();
    for p in turn_parts(items) {
        match p {
            Part::Tool(r) => out.push((r.name.clone(), r.outcome)),
            Part::Fold { rows, .. } => {
                out.extend(rows.iter().map(|r| (format!("fold:{}", r.name), r.outcome)))
            }
            _ => {}
        }
    }
    out
}

#[test]
fn every_input_is_headed_input_with_its_time_whoever_sent_it() {
    let h = held(
        vec![user(1, "mine", None), user(2, "a workflow's", Some(7))],
        idle(),
    );
    let items = build_all(&h);
    let inputs: Vec<_> = items
        .iter()
        .filter_map(|i| match i {
            Item::Input { head, at, .. } => Some((*head, *at)),
            _ => None,
        })
        .collect();
    assert_eq!(
        inputs,
        [(EntryId(1), Some(1000)), (EntryId(2), Some(2000))],
        "both are inputs, told apart by nothing"
    );
}

#[test]
fn consecutive_agent_entries_share_one_header_and_an_input_or_record_starts_a_new_turn() {
    let h = held(
        vec![
            user(1, "go", None),
            agent(2, json!([{"type": "text", "text": "one"}])),
            agent(3, json!([{"type": "text", "text": "two"}])),
            record(4, "w1.design", json!({"text": "a design"}), Some(9)),
            agent(5, json!([{"type": "text", "text": "three"}])),
        ],
        idle(),
    );
    let heads: Vec<_> = build_all(&h)
        .iter()
        .filter_map(|i| match i {
            Item::Turn { head, .. } => Some(*head),
            _ => None,
        })
        .collect();
    assert_eq!(heads, [EntryId(2), EntryId(5)]);
}

#[test]
fn a_tool_result_is_never_drawn_alone() {
    let h = held(
        vec![
            agent(1, json!([call("c1", "bash", json!({"command": "ls"}))])),
            result(2, "c1", false, "out"),
        ],
        idle(),
    );
    let items = build_all(&h);
    assert_eq!(items.len(), 1, "{items:?}");
}

#[test]
fn a_tool_rows_outcome_comes_from_is_error_and_the_live_slots_only() {
    let h = held(
        vec![
            agent(
                1,
                json!([
                    call("c1", "bash", json!({"command": "npm test"})),
                    {"type": "text", "text": "then"},
                    call("c2", "bash", json!({"command": "npm test"})),
                    {"type": "text", "text": "and"},
                    call("c3", "bash", json!({"command": "npm run"})),
                    {"type": "text", "text": "last"},
                    call("c4", "read", json!({"path": "a"})),
                ]),
            ),
            result(2, "c1", true, "✓ exit 0"),
            result(3, "c2", false, "FAILED: everything"),
        ],
        running(json!([{"callId": "c3", "name": "bash", "status": "running", "output": ""}])),
    );
    assert_eq!(
        tool_outcomes(&build_all(&h)),
        [
            ("bash".to_string(), Outcome::Failed),
            ("bash".to_string(), Outcome::Ok),
            ("bash".to_string(), Outcome::Running),
            ("read".to_string(), Outcome::Running),
        ],
        "c4 has no result yet while the run is live"
    );
}

#[test]
fn a_call_with_no_result_and_no_live_run_reads_no_result() {
    let h = held(
        vec![agent(
            1,
            json!([call("c1", "bash", json!({"command": "x"}))]),
        )],
        idle(),
    );
    assert_eq!(
        tool_outcomes(&build_all(&h)),
        [("bash".to_string(), Outcome::NoResult)]
    );
}

#[test]
fn only_two_or_more_adjacent_ok_rows_fold() {
    let h = held(
        vec![
            agent(
                1,
                json!([
                    call("a", "read", json!({"path": "x"})),
                    call("b", "bash", json!({"command": "y"})),
                    {"type": "text", "text": "between"},
                    call("c", "read", json!({"path": "z"})),
                    {"type": "text", "text": "and"},
                    call("d", "write", json!({"path": "w"})),
                    call("e", "bash", json!({"command": "fails"})),
                    call("f", "edit", json!({"path": "v"})),
                ]),
            ),
            result(2, "a", false, ""),
            result(3, "b", false, ""),
            result(4, "c", false, ""),
            result(5, "d", false, ""),
            result(6, "e", true, ""),
            result(7, "f", false, ""),
        ],
        idle(),
    );
    assert_eq!(
        tool_outcomes(&build_all(&h)),
        [
            ("fold:read".to_string(), Outcome::Ok),
            ("fold:bash".to_string(), Outcome::Ok),
            ("read".to_string(), Outcome::Ok),
            ("write".to_string(), Outcome::Ok),
            ("bash".to_string(), Outcome::Failed),
            ("edit".to_string(), Outcome::Ok),
        ]
    );
}

#[test]
fn ok_rows_from_separate_agent_entries_of_one_turn_fold_together() {
    let h = held(
        vec![
            agent(1, json!([call("a", "read", json!({"path": "x"}))])),
            result(2, "a", false, ""),
            agent(3, json!([call("b", "bash", json!({"command": "y"}))])),
            result(4, "b", false, ""),
        ],
        idle(),
    );
    assert_eq!(
        tool_outcomes(&build_all(&h)),
        [
            ("fold:read".to_string(), Outcome::Ok),
            ("fold:bash".to_string(), Outcome::Ok)
        ]
    );
}

fn first_tool(items: &[Item]) -> ToolRow {
    turn_parts(items)
        .into_iter()
        .find_map(|p| match p {
            Part::Tool(r) => Some(r.clone()),
            _ => None,
        })
        .expect("a tool row")
}

#[test]
fn a_running_row_counts_from_its_message_and_opens_on_its_last_six_lines() {
    let output = (1..=9).map(|n| format!("line {n}\n")).collect::<String>();
    let h = held(
        vec![agent(
            4,
            json!([call("c1", "bash", json!({"command": "npm test"}))]),
        )],
        running(json!([{"callId": "c1", "name": "bash", "status": "running", "output": output}])),
    );
    let row = first_tool(&build_all(&h));
    assert_eq!(row.started, Some(4000));
    assert!(row.open, "a running row's output shows open");
    assert_eq!(
        row.shown_output(),
        Some("line 4\nline 5\nline 6\nline 7\nline 8\nline 9".to_string())
    );
    assert_eq!(row.args, "npm test");
}

#[test]
fn tool_input_preserves_only_what_the_summary_loses() {
    let cases = [
        (json!({"command": "ls"}), None),
        (json!({"query": "rune"}), None),
        (json!({"path": "src/main.rs"}), None),
        (json!({"text": "hello"}), None),
        (json!({}), None),
        (Value::Null, None),
        (json!([1, 2]), None),
        (json!({"command": "first\nsecond"}), Some("first\nsecond")),
        (json!({"query": "first\nsecond"}), Some("first\nsecond")),
        (json!({"path": "first\nsecond"}), Some("first\nsecond")),
        (json!({"text": "first\nsecond"}), Some("first\nsecond")),
        (
            json!({"command": "ls", "cwd": "/tmp"}),
            Some("{\n  \"command\": \"ls\",\n  \"cwd\": \"/tmp\"\n}"),
        ),
        (
            json!({"path": "src", "text": "new\ntext"}),
            Some("{\n  \"path\": \"src\",\n  \"text\": \"new\\ntext\"\n}"),
        ),
        (
            json!({"z": 1, "a": 2}),
            Some("{\n  \"z\": 1,\n  \"a\": 2\n}"),
        ),
        (json!({"command": 7}), Some("{\n  \"command\": 7\n}")),
    ];
    for (args, expected) in cases {
        let h = held(
            vec![agent(1, json!([call("c", "tool", args.clone())]))],
            idle(),
        );
        let row = first_tool(&build_all(&h));
        assert_eq!(row.input.as_deref(), expected, "{args}");
    }
}

#[test]
fn long_single_line_arguments_remain_readable() {
    for (command, expanded) in [
        ("x".repeat(48), false),
        ("x".repeat(49), true),
        ("é".repeat(48), false),
    ] {
        let h = held(
            vec![agent(
                1,
                json!([call("c", "bash", json!({"command": command}))]),
            )],
            idle(),
        );
        let row = first_tool(&build_all(&h));
        assert_eq!(row.input.as_deref(), expanded.then_some(command.as_str()));
    }
}

#[test]
fn za_opens_a_finished_row_and_closes_a_running_one() {
    let h = held(
        vec![
            agent(
                1,
                json!([
                    call("c1", "bash", json!({"command": "a"})),
                    {"type": "text", "text": "x"},
                    call("c2", "bash", json!({"command": "b"}))
                ]),
            ),
            result(2, "c1", true, "boom"),
        ],
        running(json!([{"callId": "c2", "name": "bash", "status": "running", "output": "o"}])),
    );
    let mut folds = HashMap::new();
    folds.insert(Stop::Tool("c1".into()), true);
    folds.insert(Stop::Tool("c2".into()), false);
    let items = build(&h, &folds);
    let rows: Vec<_> = turn_parts(&items)
        .into_iter()
        .filter_map(|p| match p {
            Part::Tool(r) => Some((r.open, r.shown_output())),
            _ => None,
        })
        .collect();
    assert_eq!(
        rows,
        [(true, Some("boom".to_string())), (false, None)],
        "opened output shows the bytes as sent"
    );
}

#[test]
fn an_opened_fold_shows_its_rows_and_their_stops() {
    let h = held(
        vec![
            agent(
                1,
                json!([
                    call("a", "read", json!({"path": "x"})),
                    call("b", "bash", json!({"command": "y"}))
                ]),
            ),
            result(2, "a", false, ""),
            result(3, "b", false, ""),
        ],
        idle(),
    );
    let closed = stops(&build_all(&h));
    assert_eq!(closed, [Stop::Head(EntryId(1)), Stop::Fold("a".into())]);
    let mut folds = HashMap::new();
    folds.insert(Stop::Fold("a".into()), true);
    assert_eq!(
        stops(&build(&h, &folds)),
        [
            Stop::Head(EntryId(1)),
            Stop::Fold("a".into()),
            Stop::Tool("a".into()),
            Stop::Tool("b".into())
        ]
    );
}

#[test]
fn a_thinking_block_folds_to_its_word_count() {
    let h = held(
        vec![agent(
            1,
            json!([{"type": "thinking", "thinking": "one two  three\nfour"}]),
        )],
        idle(),
    );
    let words = turn_parts(&build_all(&h))
        .into_iter()
        .find_map(|p| match p {
            Part::Thinking { words, open, .. } => Some((*words, *open)),
            _ => None,
        });
    assert_eq!(words, Some((4, false)));
}

#[test]
fn a_record_keeps_its_kind_and_task_and_has_no_body_without_string_text() {
    let h = held(
        vec![
            record(1, "w1.design", json!({"text": "# Title"}), Some(18)),
            record(2, "w2.asked", json!({"text": 3}), None),
        ],
        idle(),
    );
    let records: Vec<_> = build_all(&h)
        .into_iter()
        .filter_map(|i| match i {
            Item::Record {
                kind,
                by_task,
                body,
                ..
            } => Some((kind, by_task, body.map(|b| b.len()))),
            _ => None,
        })
        .collect();
    assert_eq!(
        records,
        [
            ("w1.design".to_string(), Some(TaskId(18)), Some(1)),
            ("w2.asked".to_string(), None, None)
        ]
    );
}

#[test]
fn system_reset_and_compaction_are_quiet_chrome_lines() {
    let h = held(
        vec![
            json!({"id": 1, "conversationId": 1, "kind": "pi.system", "model": [{"role": "system",
                "toolsAdded": [{"name": "read"}, {"name": "bash"}]}]}),
            json!({"id": 2, "conversationId": 1, "kind": "pi.reset", "model": [{"role": "user",
                "content": "handoff notes"}]}),
            json!({"id": 3, "conversationId": 1, "kind": "pi.compaction", "model": [{"role": "user",
                "content": [{"type": "text", "text": "summary"}]}]}),
        ],
        idle(),
    );
    let lines: Vec<_> = build_all(&h)
        .into_iter()
        .filter_map(|i| match i {
            Item::Chrome { words, text, .. } => Some((words, text)),
            _ => None,
        })
        .collect();
    assert_eq!(
        lines,
        [
            ("tools: read, bash".to_string(), None),
            (
                "context reset".to_string(),
                Some("handoff notes".to_string())
            ),
            (
                "earlier context summarised".to_string(),
                Some("summary".to_string())
            ),
        ]
    );
}

#[test]
fn the_live_generation_follows_the_entries_and_is_no_stop() {
    let docs = json!({"pi.live": {"run": {"taskId": 8, "turns": 3}, "generation": {"attempt": 1,
        "message": {"role": "assistant", "model": "claude-opus-4-5",
            "content": [{"type": "text", "text": "Two facts"}]}}}});
    let h = held(vec![user(1, "go", None)], docs);
    let items = build_all(&h);
    assert!(matches!(
        items.last(),
        Some(Item::Writing { text, model }) if text == "Two facts" && model == "claude-opus-4-5"
    ));
    assert_eq!(
        stops(&items).last(),
        Some(&Stop::Text(EntryId(1), 0, 0)),
        "the cursor never rests on the generation"
    );
}

#[test]
fn a_run_failure_shows_only_without_a_run_and_an_abort_reads_stopped() {
    let failure = |run: bool, reason: &str| {
        let mut live = json!({"runFailure": {"taskId": 3, "reason": reason, "detail": "429"}});
        if run {
            live["run"] = json!({"taskId": 4, "turns": 1});
        }
        build_all(&held(vec![user(1, "go", None)], json!({"pi.live": live})))
            .last()
            .cloned()
    };
    assert_eq!(
        failure(false, "model_error"),
        Some(Item::Failure {
            reason: "model_error".into(),
            detail: Some("429".into())
        })
    );
    assert_eq!(failure(false, "aborted"), Some(Item::Stopped));
    assert!(matches!(
        failure(true, "model_error"),
        Some(Item::Input { .. })
    ));
}

#[test]
fn the_gap_line_sits_where_the_missing_entries_are() {
    let mut h = held(
        vec![user(1, "a", None), user(2, "b", None), user(9, "c", None)],
        idle(),
    );
    h.gap = Some(Gap {
        min: EntryId(3),
        max: EntryId(8),
    });
    let order: Vec<_> = build_all(&h)
        .into_iter()
        .map(|i| match i {
            Item::Input { head, .. } => head.0.to_string(),
            Item::Gap => "gap".into(),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(order, ["1", "2", "gap", "9"]);
}

#[test]
fn markdown_splits_into_top_level_blocks_keeping_list_numbers_and_fences_whole() {
    let src = "Intro line\nsecond line\n\n1. one\n2. two\n\n```rust\nfn a() {}\n\nfn b() {}\n```\n\n- x\n  - nested\n- y\n";
    assert_eq!(
        blocks(src),
        [
            "Intro line\nsecond line",
            "1. one",
            "2. two",
            "```rust\nfn a() {}\n\nfn b() {}\n```",
            "- x\n  - nested",
            "- y",
        ]
    );
    assert!(blocks("  \n\n ").is_empty());
}

#[test]
fn every_text_block_is_a_stop_after_its_header() {
    let h = held(
        vec![
            user(1, "a\n\nb", None),
            agent(2, json!([{"type": "text", "text": "1. x\n2. y"}])),
        ],
        idle(),
    );
    assert_eq!(
        stops(&build_all(&h)),
        [
            Stop::Head(EntryId(1)),
            Stop::Text(EntryId(1), 0, 0),
            Stop::Text(EntryId(1), 0, 1),
            Stop::Head(EntryId(2)),
            Stop::Text(EntryId(2), 0, 0),
            Stop::Text(EntryId(2), 0, 1),
        ]
    );
}
