//! Refusal headings and the details shown beside them.

use serde_json::Value;
use tau_client::{
    Wall,
    wire::{CallName, DecisionRecord, FieldError, ProjectView, Refusal, RefusalCode},
};

use crate::time;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnswerAction {
    #[default]
    Answer,
    SendAgainAfterFault,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefusalCopy {
    pub message: Option<String>,
    pub errors: Vec<FieldError>,
    pub project: Option<ProjectView>,
    /// The subject's open request, such as `#3`.
    pub open: Option<String>,
}

/// The sent body supplies model, workflow, extension, and decision names.
/// Missing context falls back to the server's message, never message parsing.
pub fn refusal(
    call: CallName,
    body: &Value,
    refused: &Refusal,
    action: AnswerAction,
) -> RefusalCopy {
    use RefusalCode::*;
    let quoted = || format!("Not recorded. The server refused it: {}", refused.message);
    let message = match &refused.code {
        BadCursor | Idle | EventLog | BadOptions => None,
        BadRequest if call == CallName::Answer => Some(quoted()),
        BadRequest => Some(format!(
            "The server did not accept the request: {}",
            refused.message
        )),
        Unauthorized if call == CallName::Pair => Some(super::connect::pair_refusal(refused)),
        Unauthorized => Some(device_refused(&refused.message)),
        UnknownCall => Some(format!(
            "This server has no call {}; it may be older than this client.",
            call.path()
        )),
        UnknownProject | UnknownConversation | UnknownDecision | NotAWorkflow => {
            Some(refused.message.clone())
        }
        TooLarge => Some("Over 1 MiB · the server refused it · nothing was done".into()),
        Conflict if call == CallName::Answer => Some(
            refused
                .decision()
                .map(|d| conflict(&d))
                .unwrap_or_else(quoted),
        ),
        Conflict if call == CallName::Bind => Some(match refused.project() {
            Some(p) => format!("{} is already bound with other fields:", p.project),
            None => refused.message.clone(),
        }),
        NotOpen => Some("Not recorded. This request is no longer open.".into()),
        StaleBasis | UnknownChoice | BadAnnotation => Some(quoted()),
        ExtensionNotLoaded => Some(match extension(call, body) {
            Some(name) if call == CallName::Answer => match action {
                AnswerAction::Answer => format!(
                    "Not recorded. The server has no code for {name} now; the decision stays open."
                ),
                AnswerAction::SendAgainAfterFault => format!(
                    "Not sent again. The server has no code for {name} now; the answer stays recorded and its work is not started."
                ),
            },
            Some(name) => format!("The server has no code for {name} now."),
            None if call == CallName::Answer => quoted(),
            None => refused.message.clone(),
        }),
        OptionsInvalid => Some(match body["workflow"].as_str() {
            Some(workflow) => {
                format!("{workflow} can't start: its options are not all set or don't fit")
            }
            None => refused.message.clone(),
        }),
        UnknownModel => Some(match body["model"].as_str() {
            Some(model) => format!("This server has no model {model}."),
            None => refused.message.clone(),
        }),
        Busy if call == CallName::Continue => {
            Some("A run is working; there is nothing to continue.".into())
        }
        Conflict | Busy | Other(_) => Some(format!("Not done: {}", refused.message)),
    };
    RefusalCopy {
        message,
        errors: refused.errors(),
        project: refused.project(),
        open: refused.open().map(|d| request_number(d.id.as_str())),
    }
}

fn extension(call: CallName, body: &Value) -> Option<&str> {
    match call {
        CallName::Answer => body["decision"]
            .as_str()?
            .split_once(':')
            .map(|(name, _)| name),
        CallName::Create => body["workflow"].as_str(),
        _ => body["extension"].as_str(),
    }
}

pub(super) const HOST_MESSAGE: &str = "This Host or Origin is not allowed.";
pub(super) const HOST_ADVICE: &str =
    "If you reach the server through a proxy, start it with tau --allow-host <name>";

/// The refused band of `client.md` §6.1: any `unauthorized` on a call but `pair`.
pub fn device_refused(message: &str) -> String {
    let mut text = format!("The server refused this device: {message}");
    if message == HOST_MESSAGE {
        text.push(' ');
        text.push_str(HOST_ADVICE);
    }
    text
}

pub(super) fn conflict(decision: &DecisionRecord) -> String {
    let by = decision
        .by
        .as_ref()
        .map(|d| d.as_str())
        .unwrap_or("another device");
    let choice = decision.choice.as_deref().unwrap_or("an answer");
    match decision.at {
        Some(at) => format!(
            "Not recorded. {by} answered {choice} at {}.",
            time::clock(Wall(at))
        ),
        None => format!("Not recorded. {by} answered {choice}."),
    }
}

fn request_number(id: &str) -> String {
    id.rsplit_once('#')
        .map(|(_, n)| format!("#{n}"))
        .unwrap_or_else(|| id.to_owned())
}
