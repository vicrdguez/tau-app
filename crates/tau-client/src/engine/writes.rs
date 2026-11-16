//! Writes: the outbox's entries, when they are sent and sent again (`client.md` §2), the start page's `create`
//! then `input` (§2.4), and settling an answer whose reply was lost (§2.3).

use super::*;
use crate::ids::ConversationId;
use crate::model::Known;
use crate::outbox::{AUTO_RESEND_WINDOW, DONE_LINGERS, NotSent, RESEND_DELAYS, Settlement, Write};
use crate::wire::{
    AnswerResult, BODY_LIMIT, BindResult, DecisionRecord, DecisionStatus, DecisionView, InputMode,
    input_body,
};
use serde_json::json;

impl Engine {
    /// Puts a write in the outbox with its exact body, and sends it when the link allows.
    pub fn write(&mut self, write: Write, now: Now) -> Result<WriteId, NotSent> {
        match &self.conn {
            Conn::NotPaired | Conn::Pairing => return Err(NotSent::NotPaired),
            Conn::OtherProtocol(_) => return Err(NotSent::OtherProtocol),
            Conn::Revoked => return Err(NotSent::Revoked),
            Conn::Handshaking { .. } | Conn::Ready => {}
        }
        let (call, body) = match write {
            Write::Input { conv, text, mode } => {
                let request = self.request_id(now);
                let body = input_body(&conv.project, conv.id, &text, &request, mode);
                let id = self.add(WriteCall::Input, body, false, now);
                if self.writes_on() {
                    self.send(id, now);
                } else {
                    self.set_state(
                        id,
                        WriteState::Waiting {
                            next: Next::AtConnection,
                        },
                    );
                }
                return Ok(id);
            }
            Write::Start {
                project,
                with,
                text,
            } => {
                if text.trim().is_empty() {
                    return Err(NotSent::Empty);
                }
                let probe = input_body(
                    &project,
                    ConversationId(u64::MAX),
                    &text,
                    &self.request_id(now),
                    InputMode::Send,
                );
                if probe.to_string().len() > BODY_LIMIT {
                    return Err(NotSent::TooLarge);
                }
                if self.outbox.starting() {
                    return Err(NotSent::Starting);
                }
                let body = match &with {
                    StartWith::Workflow(w) => json!({ "project": project, "workflow": w }),
                    StartWith::Model(m) => json!({ "project": project, "model": m }),
                };
                self.check_writes()?;
                let id = self.add(WriteCall::Create, body, true, now);
                self.starts.insert(id, text);
                self.send(id, now);
                return Ok(id);
            }
            Write::Answer {
                decision,
                basis,
                choice,
                annotations,
            } => {
                if self.outbox.answer_unsettled(&decision).is_some() {
                    return Err(NotSent::AnswerUnsettled);
                }
                let body = json!({
                    "project": decision.project, "decision": decision.id, "basis": basis, "choice": choice,
                    "annotations": annotations,
                });
                (WriteCall::Answer, body)
            }
            Write::Continue { conv } => {
                let request = self.request_id(now);
                (
                    WriteCall::Continue,
                    json!({ "project": conv.project, "conversation": conv.id, "requestId": request }),
                )
            }
            Write::Stop { conv } => (
                WriteCall::Stop,
                json!({ "project": conv.project, "conversation": conv.id }),
            ),
            Write::SetModel {
                conv,
                model,
                thinking,
            } => {
                let mut body =
                    json!({ "project": conv.project, "conversation": conv.id, "model": model });
                if let Some(level) = thinking {
                    body["thinkingLevel"] = serde_json::to_value(level).unwrap_or_default();
                }
                (WriteCall::SetModel, body)
            }
            Write::SetOptions {
                project,
                extension,
                patch,
            } => (
                WriteCall::SetOptions,
                json!({ "project": project, "extension": extension, "patch": patch }),
            ),
            Write::Bind {
                project,
                workdir,
                extensions,
            } => (
                WriteCall::Bind,
                json!({ "project": project, "workdir": workdir, "extensions": extensions }),
            ),
            Write::AddExtensions {
                project,
                extensions,
            } => (
                WriteCall::AddExtensions,
                json!({ "project": project, "extensions": extensions }),
            ),
        };
        self.check_writes()?;
        let id = self.add(call, body, false, now);
        self.send(id, now);
        Ok(id)
    }

    /// Writes other than a queued input need the link up and this device accepted.
    fn check_writes(&self) -> Result<(), NotSent> {
        if self.band.is_some() {
            Err(NotSent::Refused)
        } else if !self.connected() {
            Err(NotSent::Offline)
        } else {
            Ok(())
        }
    }

    /// `␣rt`, and `␣ar` for an answer: sends the entry's stored body once, now.
    pub fn retry(&mut self, id: WriteId, now: Now) -> Result<(), NotSent> {
        let entry = self.outbox.get(id).ok_or(NotSent::NoSuchEntry)?;
        if !entry.can_retry() && !entry.can_send_again() {
            return Err(NotSent::NoRetry);
        }
        match &self.conn {
            Conn::Revoked => return Err(NotSent::Revoked),
            Conn::OtherProtocol(_) => return Err(NotSent::OtherProtocol),
            _ => {}
        }
        self.check_writes()?;
        self.send(id, now);
        Ok(())
    }

    /// `x`: forgets the entry on this device. Nothing is sent.
    pub fn forget(&mut self, id: WriteId, _now: Now) {
        self.outbox.entries.retain(|e| e.id != id);
        self.calls
            .retain(|_, c| !matches!(c.purpose, Purpose::Write(w) if w == id));
        self.linger.remove(&id);
        self.starts.remove(&id);
        self.effects.push(Effect::SaveOutbox);
    }

    fn add(&mut self, call: WriteCall, body: Value, from_start: bool, now: Now) -> WriteId {
        let id = WriteId(self.outbox.next_id);
        self.outbox.next_id += 1;
        self.outbox.entries.push(OutboxEntry {
            id,
            call,
            body,
            state: WriteState::Waiting {
                next: Next::AtConnection,
            },
            created: now.wall,
            sent_at: None,
            tries: 0,
            restored: false,
            from_start,
            created_mono: Some(now.mono),
        });
        self.effects.push(Effect::SaveOutbox);
        id
    }

    fn entry_mut(&mut self, id: WriteId) -> Option<&mut OutboxEntry> {
        self.outbox.entries.iter_mut().find(|e| e.id == id)
    }

    /// Sends the entry's stored body, unchanged.
    fn send(&mut self, id: WriteId, now: Now) {
        let Some(entry) = self.entry_mut(id) else {
            return;
        };
        entry.state = WriteState::Sending;
        entry.sent_at = Some(now.wall);
        let request = Request {
            call: write_call_name(entry.call),
            body: entry.body.clone(),
        };
        self.calls
            .retain(|_, c| !matches!(c.purpose, Purpose::Write(w) if w == id));
        self.call(Purpose::Write(id), request, now);
        self.effects.push(Effect::SaveOutbox);
    }

    /// Changes an entry's state; an outcome the UI echoes becomes an event.
    fn set_state(&mut self, id: WriteId, state: WriteState) {
        let Some(entry) = self.entry_mut(id) else {
            return;
        };
        entry.state = state.clone();
        let call = entry.call;
        if matches!(
            state,
            WriteState::Done { .. }
                | WriteState::Refused(_)
                | WriteState::Unknown { .. }
                | WriteState::Settled(_)
        ) {
            self.events.push(LinkEvent::Write { id, call, state });
        }
        self.effects.push(Effect::SaveOutbox);
    }

    pub(super) fn on_write_reply(&mut self, id: WriteId, outcome: Outcome, now: Now) {
        let Some(entry) = self.outbox.get(id).cloned() else {
            return;
        };
        match outcome {
            Outcome::Done(result) => {
                self.set_state(
                    id,
                    WriteState::Done {
                        at: now.wall,
                        result: result.clone(),
                    },
                );
                self.linger.insert(id, now.mono.after(DONE_LINGERS));
                self.after_done(&entry, result, now);
            }
            Outcome::Refused(r)
                if r.code == RefusalCode::Unauthorized && entry.call.resends().automatic =>
            {
                self.set_state(
                    id,
                    WriteState::Waiting {
                        next: Next::AtConnection,
                    },
                );
            }
            Outcome::Refused(r) => {
                self.starts.remove(&id);
                self.after_refused(&entry, &r, now);
                self.set_state(id, WriteState::Refused(r));
            }
            Outcome::Unknown(reason) => {
                self.starts.remove(&id);
                let seq = self.next_seq();
                self.unknown_at.insert(id, seq);
                let next = self.schedule(&entry, now);
                self.set_state(id, WriteState::Unknown { reason, next });
            }
        }
    }

    /// When an entry whose outcome is unknown goes again by itself (`client.md` §2.2).
    fn schedule(&self, entry: &OutboxEntry, now: Now) -> Next {
        let young = entry
            .created_mono
            .is_some_and(|at| now.mono.since(at) <= AUTO_RESEND_WINDOW);
        let tries = entry.tries as usize;
        if !entry.call.resends().automatic
            || entry.restored
            || !young
            || tries >= RESEND_DELAYS.len()
        {
            Next::Manual
        } else if self.writes_on() {
            Next::At(now.mono.after(RESEND_DELAYS[tries]))
        } else {
            Next::AtConnection
        }
    }

    fn after_done(&mut self, entry: &OutboxEntry, result: Value, now: Now) {
        let project = entry.project();
        match entry.call {
            WriteCall::Create => self.started(entry, &result, now),
            WriteCall::Input | WriteCall::Continue | WriteCall::Stop => {
                if let Some(p) = &project {
                    self.poll_soon(p, now);
                }
            }
            WriteCall::Answer => {
                let (Some(d), Ok(answer)) = (
                    entry.decision(),
                    serde_json::from_value::<AnswerResult>(result),
                ) else {
                    return;
                };
                self.take_record(&d, answer.decision, now);
                self.refetch
                    .insert(d, now.mono.after(Duration::from_secs(1)));
            }
            WriteCall::SetOptions => {
                if let Some(p) = project {
                    self.fetch_options(p, now);
                }
            }
            WriteCall::Bind | WriteCall::AddExtensions => {
                if let Some(view) = serde_json::from_value::<BindResult>(result)
                    .ok()
                    .map(|b| b.project)
                    && let Known::Have { value, .. } = &mut self.server.projects
                {
                    value.retain(|p| p.project != view.project);
                    value.push(view);
                    value.sort_by(|a, b| a.project.cmp(&b.project));
                    self.projects_changed(now);
                }
                self.refresh_catalogue(now);
            }
            WriteCall::SetModel => {}
        }
    }

    fn after_refused(&mut self, entry: &OutboxEntry, r: &Refusal, now: Now) {
        match r.code {
            RefusalCode::ExtensionNotLoaded | RefusalCode::NotAWorkflow
                if entry.call != WriteCall::Answer =>
            {
                self.refresh_catalogue(now);
            }
            RefusalCode::UnknownModel => self.refresh_catalogue(now),
            RefusalCode::Conflict if entry.call == WriteCall::Answer => {
                if let (Some(d), Some(record)) = (entry.decision(), r.decision()) {
                    self.take_record(&d, record, now);
                }
            }
            RefusalCode::NotOpen if entry.call == WriteCall::Answer => {
                if let Some(d) = entry.decision() {
                    self.fetch_decision(d, now);
                }
            }
            _ => {}
        }
    }

    /// A decision record from an answer's reply: the view takes it at once, keeping the basis text it has.
    fn take_record(&mut self, decision: &DecisionRef, record: DecisionRecord, now: Now) {
        let text = self
            .server
            .decisions
            .get(decision)
            .and_then(|k| k.value())
            .map(|v| v.text.clone())
            .unwrap_or_default();
        let view = DecisionView {
            decision: record,
            text,
            then: None,
        };
        self.server.decisions.insert(
            decision.clone(),
            Known::Have {
                value: view,
                at: now.wall,
            },
        );
    }

    /// `create` gave a conversation: send the start page's text to it, and forget the start page's lost creates.
    fn started(&mut self, create: &OutboxEntry, result: &Value, now: Now) {
        let text = self.starts.remove(&create.id).unwrap_or_default();
        let (Some(project), Some(n)) = (create.project(), result["conversation"].as_u64()) else {
            return;
        };
        let conv = ConvRef {
            project: project.clone(),
            id: ConversationId(n),
        };
        let lost: Vec<WriteId> = self
            .outbox
            .entries
            .iter()
            .filter(|e| {
                e.call == WriteCall::Create && e.from_start && e.id != create.id && e.is_unknown()
            })
            .map(|e| e.id)
            .collect();
        for id in lost {
            self.forget(id, now);
        }
        let with = match (create.body.get("workflow"), create.body.get("model")) {
            (Some(w), _) => StartWith::Workflow(w.as_str().unwrap_or_default().to_owned()),
            (_, m) => StartWith::Model(m.and_then(Value::as_str).unwrap_or_default().to_owned()),
        };
        let request = self.request_id(now);
        let body = input_body(&project, conv.id, &text, &request, InputMode::Send);
        let input = self.add(WriteCall::Input, body, true, now);
        if self.writes_on() {
            self.send(input, now);
        }
        self.events.push(LinkEvent::Started {
            conv,
            with,
            create: create.id,
            input,
        });
        self.poll_soon(&project, now);
    }

    /// A connection came back, or the server accepts this device again: entries waiting for it go now, unless they
    /// are too old or from an earlier launch.
    pub(super) fn flush_waiting(&mut self, now: Now) {
        if !self.writes_on() {
            return;
        }
        let waiting: Vec<(WriteId, bool)> = self
            .outbox
            .entries
            .iter()
            .filter_map(|e| match e.state {
                WriteState::Waiting {
                    next: Next::AtConnection,
                } => Some((e.id, false)),
                WriteState::Unknown {
                    next: Next::AtConnection,
                    ..
                } => Some((e.id, true)),
                _ => None,
            })
            .collect();
        for (id, resend) in waiting {
            let entry = self.outbox.get(id).expect("listed");
            let young = entry
                .created_mono
                .is_some_and(|at| now.mono.since(at) <= AUTO_RESEND_WINDOW);
            if young && !entry.restored {
                if resend {
                    self.entry_mut(id).expect("listed").tries += 1;
                }
                self.send(id, now);
            } else if let Some(e) = self.entry_mut(id) {
                match &mut e.state {
                    WriteState::Waiting { next } | WriteState::Unknown { next, .. } => {
                        *next = Next::Manual
                    }
                    _ => {}
                }
                self.effects.push(Effect::SaveOutbox);
            }
        }
    }

    pub(super) fn tick_outbox(&mut self, now: Now) {
        let due: Vec<WriteId> = self
            .outbox
            .entries
            .iter()
            .filter(|e| matches!(e.state, WriteState::Unknown { next: Next::At(at), .. } if at <= now.mono))
            .map(|e| e.id)
            .collect();
        for id in due {
            if self.writes_on() {
                self.entry_mut(id).expect("listed").tries += 1;
                self.send(id, now);
            } else if let Some(WriteState::Unknown { next, .. }) =
                self.entry_mut(id).map(|e| &mut e.state)
            {
                *next = Next::AtConnection;
            }
        }
        let gone: Vec<WriteId> = self
            .linger
            .iter()
            .filter(|(_, at)| **at <= now.mono)
            .map(|(id, _)| *id)
            .collect();
        for id in gone {
            self.linger.remove(&id);
            self.outbox.entries.retain(|e| e.id != id);
        }
    }

    /// After an inbox frame: an answer whose outcome is unknown is settled once the server proves it (`client.md`
    /// §2.3). A fresh first frame still listing the decision proves it did not land; a decision the inbox no longer
    /// lists is fetched to see who answered it.
    pub(super) fn settle_answers(&mut self, fresh: Option<StreamId>, now: Now) {
        let Some(inbox) = self.server.inbox.value() else {
            return;
        };
        let mut not_landed = Vec::new();
        let mut fetch = Vec::new();
        for e in &self.outbox.entries {
            let (WriteCall::Answer, WriteState::Unknown { .. }, Some(d)) =
                (e.call, &e.state, e.decision())
            else {
                continue;
            };
            let since = self.unknown_at.get(&e.id).copied().unwrap_or(0);
            if inbox.contains(&d) {
                if fresh.is_some_and(|attempt| attempt.0 > since) {
                    not_landed.push(e.id);
                }
            } else {
                fetch.push(d);
            }
        }
        for id in not_landed {
            self.set_state(id, WriteState::Settled(Settlement::NotLanded));
        }
        for d in fetch {
            self.fetch_decision(d, now);
        }
    }

    /// A decision record proves what became of a lost answer to it.
    pub(super) fn settle_answer_from(
        &mut self,
        decision: &DecisionRef,
        record: &DecisionRecord,
        now: Now,
    ) {
        if self
            .server
            .inbox
            .value()
            .is_some_and(|inbox| inbox.contains(decision))
        {
            return;
        }
        let device = self.server.device.clone();
        let lost: Vec<OutboxEntry> = self
            .outbox
            .entries
            .iter()
            .filter(|e| {
                e.call == WriteCall::Answer
                    && e.is_unknown()
                    && e.decision().as_ref() == Some(decision)
            })
            .cloned()
            .collect();
        for e in lost {
            let settlement = match record.status {
                DecisionStatus::Answered => {
                    let same = record.by == device
                        && record.choice.as_deref() == e.body["choice"].as_str()
                        && serde_json::to_value(&record.annotations).ok().as_ref()
                            == Some(&e.body["annotations"]);
                    if same {
                        Settlement::Landed(record.clone())
                    } else {
                        Settlement::AnsweredElsewhere(record.clone())
                    }
                }
                DecisionStatus::Superseded | DecisionStatus::Withdrawn => {
                    Settlement::NoLongerOpen(record.clone())
                }
                DecisionStatus::Open | DecisionStatus::Other => continue,
            };
            if matches!(settlement, Settlement::Landed(_)) {
                self.linger.insert(e.id, now.mono.after(DONE_LINGERS));
            }
            self.set_state(e.id, WriteState::Settled(settlement));
        }
    }
}

fn write_call_name(call: WriteCall) -> CallName {
    match call {
        WriteCall::Input => CallName::Input,
        WriteCall::Continue => CallName::Continue,
        WriteCall::Stop => CallName::Stop,
        WriteCall::SetModel => CallName::SetModel,
        WriteCall::SetOptions => CallName::SetOptions,
        WriteCall::Bind => CallName::Bind,
        WriteCall::AddExtensions => CallName::AddExtensions,
        WriteCall::Answer => CallName::Answer,
        WriteCall::Create => CallName::Create,
    }
}
