//! One-shot echoes. Persistent state belongs in the screen or outbox instead.

use serde_json::Value;
use tau_client::{
    engine::LinkEvent,
    outbox::{Settlement, WriteCall, WriteState},
    sse::EndReason,
    streams::StreamKey,
    wire::{DecisionRecord, RefusalCode},
};

use super::{AnswerAction, refusal};

/// Request is the sent body. Device is this session's name, never the answer's
/// author. Send-again-after-fault is supplied by the readback that sent it.
pub fn event_echo(
    event: &LinkEvent,
    request: &Value,
    device: Option<&str>,
    action: AnswerAction,
) -> Option<String> {
    match event {
        LinkEvent::DecisionLeft { ours: false, .. } => Some("answered elsewhere".into()),
        LinkEvent::StreamEnded {
            reason: EndReason::Stopping,
            ..
        } => Some("The server is stopping. tau reconnects when it is back.".into()),
        LinkEvent::StreamEnded {
            key,
            reason: EndReason::Failed,
        } => Some(
            match key {
                StreamKey::Inbox => {
                    "The list of open decisions failed on the server and is reopening."
                }
                StreamKey::Open(_) | StreamKey::Timeline(_) => {
                    "A stream failed on the server and is reopening."
                }
            }
            .into(),
        ),
        LinkEvent::Write { call, state, .. } => match state {
            WriteState::Refused(r) if r.code == RefusalCode::BadOptions => Some(r.message.clone()),
            WriteState::Refused(r) => {
                refusal(super::outbox::call_name(*call), request, r, action).message
            }
            WriteState::Settled(Settlement::Landed(d)) => Some(format!(
                "Answered {} · the reply was lost, but it was recorded",
                d.choice.as_deref().unwrap_or("an answer")
            )),
            WriteState::Settled(Settlement::AnsweredElsewhere(d)) => {
                Some(super::refusals::conflict(d))
            }
            WriteState::Settled(Settlement::NoLongerOpen(_)) => {
                Some("Not recorded. This request is no longer open.".into())
            }
            WriteState::Done { result, .. } if *call == WriteCall::Answer => {
                answer_done(result, device, action)
            }
            WriteState::Waiting { .. }
            | WriteState::Sending
            | WriteState::Done { .. }
            | WriteState::Unknown { .. }
            | WriteState::Settled(Settlement::NotLanded) => None,
        },
        LinkEvent::Started { .. }
        | LinkEvent::DecisionLeft { ours: true, .. }
        | LinkEvent::Superseded { .. }
        | LinkEvent::Revoked
        | LinkEvent::Paired { .. }
        | LinkEvent::PairFailed(_)
        | LinkEvent::StreamEnded { .. } => None,
    }
}

fn answer_done(result: &Value, device: Option<&str>, action: AnswerAction) -> Option<String> {
    let d: DecisionRecord = serde_json::from_value(result.get("decision")?.clone()).ok()?;
    let repeated = result["repeated"].as_bool().unwrap_or(false);
    if action == AnswerAction::SendAgainAfterFault {
        let mut text = "Sent again".to_owned();
        if let Some(then) = &d.then {
            text.push_str(&format!(
                " · {} runs again in #{}",
                then.task, d.conversation
            ));
        }
        return Some(text);
    }
    if repeated {
        let mut text = "Already recorded".to_owned();
        if let Some(by) = &d.by
            && Some(by.as_str()) != device
        {
            text.push_str(&format!(" · by {by}"));
        }
        return Some(text);
    }
    let mut text = format!("Answered {}", d.choice.as_deref().unwrap_or("an answer"));
    if let Some(then) = &d.then {
        text.push_str(&format!(" · {} runs in #{}", then.task, d.conversation));
    }
    Some(text)
}
