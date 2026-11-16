//! How the decision's standing could fail (`spec.md` §5.6, §5.7, `client.md` §2.3, FA5, FA9, FA10):
//! - a `then` status or outcome shows another word or mark than the table's, or an unknown one
//!   shows nothing instead of its raw word;
//! - `failed` offers `␣ar` (the core never runs it again), or `faulted` doesn't;
//! - the facts line says `open` for a superseded, withdrawn or answered decision, or forgets the
//!   `then` task after an answer;
//! - an answer with no reply leaves `␣ad` on, or `␣ar` off;
//! - `␣ad` stays on while the inbox says another request is open, or the decision left it.

use serde_json::json;
use tau_client::model::Liveness;
use tau_client::wire::{DecisionView, TaskState};
use tau_client::{OutboxEntry, Wall};

use super::standing::*;

fn task(v: serde_json::Value) -> TaskState {
    serde_json::from_value(v).unwrap()
}

#[test]
fn every_then_state_has_its_word_and_mark() {
    let cases = [
        (
            json!({"status": "pending"}),
            "queued",
            Some(ThenMark::Waiting),
        ),
        (
            json!({"status": "running"}),
            "running",
            Some(ThenMark::Running),
        ),
        (
            json!({"status": "waiting"}),
            "waiting",
            Some(ThenMark::Waiting),
        ),
        (
            json!({"status": "sleeping"}),
            "sleeping",
            Some(ThenMark::Sleeping),
        ),
        (
            json!({"status": "completing"}),
            "finishing",
            Some(ThenMark::Running),
        ),
        (
            json!({"status": "terminal", "outcome": {"status": "completed"}}),
            "completed",
            Some(ThenMark::Done),
        ),
        (
            json!({"status": "terminal", "outcome": {"status": "failed"}}),
            "failed",
            Some(ThenMark::Failed),
        ),
        (
            json!({"status": "terminal", "outcome": {"status": "aborted"}}),
            "aborted",
            Some(ThenMark::Aborted),
        ),
        (
            json!({"status": "terminal", "outcome": {"status": "faulted", "error": {"message": "boom"}}}),
            "faulted",
            Some(ThenMark::Failed),
        ),
        (
            json!({"status": "blocked"}),
            "blocked · its extension is not on the server",
            Some(ThenMark::Blocked),
        ),
        (json!({"status": "hibernating"}), "hibernating", None),
    ];
    for (state, word, mark) in cases {
        let t = then(&task(state.clone()));
        assert_eq!((t.word.as_str(), t.mark), (word, mark), "{state}");
    }
}

#[test]
fn only_faulted_is_sent_again() {
    let faulted = then(&task(
        json!({"status": "terminal", "outcome": {"status": "faulted", "error": {"message": "boom"}}}),
    ));
    assert!(faulted.faulted);
    assert_eq!(faulted.error.as_deref(), Some("boom"));
    let failed = then(&task(
        json!({"status": "terminal", "outcome": {"status": "failed"}}),
    ));
    assert!(!failed.faulted);
}

fn view(status: &str, extra: serde_json::Value) -> DecisionView {
    let mut d = json!({
        "id": "w1:3:design#2", "subject": "w1:3:design", "title": "Review the design",
        "basis": 34, "choices": ["approve", "revise", "abort"], "conversation": 3,
        "then": {"task": "w1.design_answer"}, "requestedAt": 1794664500000i64, "status": status,
    });
    for (k, v) in extra.as_object().unwrap() {
        d[k] = v.clone();
    }
    serde_json::from_value(json!({"decision": d, "text": "# x"})).unwrap()
}

fn unknown_answer() -> OutboxEntry {
    serde_json::from_value(json!({"id": 1, "call": "answer",
        "body": {"project": "render-core", "decision": "w1:3:design#2"},
        "state": {"Unknown": {"reason": "Timeout", "next": "Manual"}},
        "created": 1794674520000i64, "sent_at": 1794674520000i64, "tries": 1,
        "restored": false, "from_start": false}))
    .unwrap()
}

#[test]
fn the_standing_follows_the_record_the_inbox_and_the_outbox() {
    let open = view("open", json!({}));
    assert!(matches!(
        standing(&open, Some(&Liveness::Open), None),
        Standing::Open { .. }
    ));
    let by = Liveness::Superseded {
        by: "w1:3:design#3".into(),
    };
    assert_eq!(
        standing(&open, Some(&by), None),
        Standing::Superseded {
            by: Some("#3".into())
        }
    );
    assert_eq!(
        standing(&view("withdrawn", json!({})), None, None),
        Standing::Withdrawn
    );
    let answered = view(
        "answered",
        json!({"choice": "approve", "by": "phone", "at": 1794665400000i64}),
    );
    assert!(matches!(
        standing(&answered, Some(&Liveness::NotOpen), None),
        Standing::Answered { ref choice, .. } if choice == "approve"
    ));
    let entry = unknown_answer();
    assert_eq!(
        standing(&open, Some(&Liveness::Open), Some(&entry)),
        Standing::Unknown {
            sent: Wall(1794674520000)
        }
    );
}

#[test]
fn answer_is_on_only_for_an_open_request_with_nothing_unsettled() {
    let open = view("open", json!({}));
    let live = Liveness::Open;
    assert!(answer_on(&open, Some(&live), None));
    assert!(!answer_on(&open, Some(&Liveness::NotOpen), None));
    assert!(
        !answer_on(&open, None, None),
        "nothing received from the inbox yet"
    );
    let entry = unknown_answer();
    assert!(!answer_on(&open, Some(&live), Some(&entry)));
    assert!(send_again_on(&open, Some(&entry)));
    assert!(!send_again_on(&open, None));
    let mut faulted = view("answered", json!({"choice": "approve", "by": "phone"}));
    faulted.then = Some(task(
        json!({"status": "terminal", "outcome": {"status": "faulted"}}),
    ));
    assert!(send_again_on(&faulted, None));
    let mut failed = faulted.clone();
    failed.then = Some(task(
        json!({"status": "terminal", "outcome": {"status": "failed"}}),
    ));
    assert!(!send_again_on(&failed, None));
}
