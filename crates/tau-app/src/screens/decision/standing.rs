//! Where a decision stands, from what the server said: its record, the inbox entry of its subject
//! and this device's answer in the outbox (`spec.md` §5.6, `client.md` §2.3). The title block's
//! facts, the `then` task's row of the table, and whether `␣ad` and `␣ar` are on all come from
//! here.

use tau_client::model::Liveness;
use tau_client::wire::{DecisionStatus, DecisionView, TaskState};
use tau_client::{OutboxEntry, Wall, WriteState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    Open {
        requested: Wall,
    },
    Answered {
        choice: String,
        by: Option<String>,
        at: Option<Wall>,
    },
    /// `by` is the open request's number (`#3`) when the inbox names it.
    Superseded {
        by: Option<String>,
    },
    Withdrawn,
    /// This device's answer was sent and no reply proved its outcome.
    Unknown {
        sent: Wall,
    },
}

/// The `then` task's mark (`spec.md` §5.6, the table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThenMark {
    /// `hourglass`, `fg.muted`: pending, waiting.
    Waiting,
    /// The steel triangle: running, completing.
    Running,
    /// `time`, steel: sleeping.
    Sleeping,
    /// `checkmark`, olive.
    Done,
    /// The rose `×`: failed, faulted.
    Failed,
    /// `subtract`, `fg.muted`.
    Aborted,
    /// `hourglass`, `fg`: blocked by a missing task.
    Blocked,
}

/// The `then` task's state as the facts line says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Then {
    pub word: String,
    pub mark: Option<ThenMark>,
    /// Terminal and faulted: the failed block shows, and `␣ar` sends the answer again.
    pub faulted: bool,
    pub error: Option<String>,
}

/// The table of `spec.md` §5.6. The shape of a task blocked by `missing_task` is not pinned
/// (`open-questions.md`); a `blocked` status, or an outcome naming `missing_task`, reads as one.
pub fn then(state: &TaskState) -> Then {
    let outcome = state.outcome.as_ref().map(|o| o.status.as_str());
    let (word, mark) = match (state.status.as_str(), outcome) {
        (_, Some("missing_task")) | ("blocked", _) => (
            "blocked · its extension is not on the server",
            Some(ThenMark::Blocked),
        ),
        ("pending", _) => ("queued", Some(ThenMark::Waiting)),
        ("running", _) => ("running", Some(ThenMark::Running)),
        ("waiting", _) => ("waiting", Some(ThenMark::Waiting)),
        ("sleeping", _) => ("sleeping", Some(ThenMark::Sleeping)),
        ("completing", _) => ("finishing", Some(ThenMark::Running)),
        ("terminal", Some("completed")) => ("completed", Some(ThenMark::Done)),
        ("terminal", Some("failed")) => ("failed", Some(ThenMark::Failed)),
        ("terminal", Some("aborted")) => ("aborted", Some(ThenMark::Aborted)),
        ("terminal", Some("faulted")) => ("faulted", Some(ThenMark::Failed)),
        (status, _) => (status, None),
    };
    Then {
        word: word.to_string(),
        mark,
        faulted: state.is_terminal() && outcome == Some("faulted"),
        error: state
            .outcome
            .as_ref()
            .and_then(|o| o.error.as_ref())
            .map(|e| e.message.clone()),
    }
}

/// This device's answer to `view`'s decision that has no reply yet, if the outbox holds one.
fn unknown(entry: Option<&OutboxEntry>) -> Option<&OutboxEntry> {
    entry.filter(|e| matches!(e.state, WriteState::Unknown { .. }))
}

pub fn standing(
    view: &DecisionView,
    live: Option<&Liveness>,
    unsettled: Option<&OutboxEntry>,
) -> Standing {
    let d = &view.decision;
    if let Some(e) = unknown(unsettled) {
        return Standing::Unknown {
            sent: e.sent_at.unwrap_or(e.created),
        };
    }
    match d.status {
        DecisionStatus::Answered => Standing::Answered {
            choice: d.choice.clone().unwrap_or_default(),
            by: d.by.as_ref().map(ToString::to_string),
            at: d.at.map(Wall),
        },
        DecisionStatus::Withdrawn => Standing::Withdrawn,
        DecisionStatus::Superseded => Standing::Superseded {
            by: match live {
                Some(Liveness::Superseded { by }) => Some(number(by.as_str())),
                _ => None,
            },
        },
        DecisionStatus::Open | DecisionStatus::Other => match live {
            Some(Liveness::Superseded { by }) => Standing::Superseded {
                by: Some(number(by.as_str())),
            },
            _ => Standing::Open {
                requested: Wall(d.requested_at),
            },
        },
    }
}

/// `#3` of `w1:3:design#3`.
pub fn number(id: &str) -> String {
    id.rsplit_once('#')
        .map_or_else(|| id.to_string(), |(_, n)| format!("#{n}"))
}

/// `␣ad` would open the readback: the record is open, the inbox lists this very request, and no
/// answer of this device is sending or without a reply. The link's own state is the caller's.
pub fn answer_on(
    view: &DecisionView,
    live: Option<&Liveness>,
    unsettled: Option<&OutboxEntry>,
) -> bool {
    matches!(
        view.decision.status,
        DecisionStatus::Open | DecisionStatus::Other
    ) && live == Some(&Liveness::Open)
        && unsettled.is_none()
}

/// `␣ar` is live: the `then` task ended faulted, or this device's answer has no reply.
pub fn send_again_on(view: &DecisionView, unsettled: Option<&OutboxEntry>) -> bool {
    unknown(unsettled).is_some_and(|e| e.can_send_again())
        || (view.decision.status == DecisionStatus::Answered
            && view.then.as_ref().is_some_and(|t| then(t).faulted))
}
