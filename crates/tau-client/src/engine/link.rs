//! Pairing, the handshake, and the streams: which are held, their supervision, and what their frames change.

use super::*;
use crate::ids::{EntryId, Subject};
use crate::model::{Gap, Inbox, Known, Older, Transcript};
use crate::ops;
use crate::shapes::{Docs, Entry};
use crate::wire::{OpenDecision, Order, Paired};
use std::collections::BTreeSet;

/// Entries asked for per `history` page when filling a gap or scrolling back.
pub(super) const HISTORY_PAGE: u32 = 100;

impl Engine {
    pub fn pair(&mut self, url: String, code: String, now: Now) {
        if !matches!(self.conn, Conn::NotPaired) {
            return;
        }
        self.conn = Conn::Pairing;
        let id = CallId(self.next_seq());
        let request = Request {
            call: CallName::Pair,
            body: serde_json::json!({ "code": code }),
        };
        self.effects.push(Effect::Call {
            id,
            url: url.clone(),
            auth: false,
            request,
        });
        self.calls.insert(
            id,
            InFlight {
                purpose: Purpose::Pair { url },
                deadline: now.mono.after(CALL_TIMEOUT),
            },
        );
    }

    pub(super) fn on_paired(&mut self, url: String, outcome: Outcome, now: Now) {
        let paired = match outcome {
            Outcome::Done(v) => {
                serde_json::from_value::<Paired>(v).map_err(|_| Unknown::Unreadable { status: 200 })
            }
            Outcome::Refused(r) => {
                self.conn = Conn::NotPaired;
                self.events
                    .push(LinkEvent::PairFailed(PairFailure::Refused(r)));
                return;
            }
            Outcome::Unknown(u) => Err(u),
        };
        match paired {
            Ok(Paired { device, token }) => {
                self.effects.push(Effect::SaveCredentials {
                    url: url.clone(),
                    device: device.clone(),
                    token,
                });
                self.server.url = Some(url);
                self.server.device = Some(device.clone());
                self.events.push(LinkEvent::Paired { device });
                self.handshake(now);
            }
            Err(u) => {
                self.conn = Conn::NotPaired;
                self.events
                    .push(LinkEvent::PairFailed(PairFailure::NoReply(u)));
            }
        }
    }

    pub fn forget_server(&mut self, now: Now) {
        let url = self.url();
        self.close_all();
        let mut effects = std::mem::take(&mut self.effects);
        let seed = self.random();
        *self = Engine::new(
            Setup {
                seed,
                ..Setup::default()
            },
            now,
        );
        effects.push(Effect::ForgetServer { url });
        self.effects = effects;
    }

    /// `POST /v1/`: the server must answer `unknown_call` (or anything else) as protocol 1 before anything else is sent.
    pub(super) fn handshake(&mut self, now: Now) {
        let failures = match self.conn {
            Conn::Handshaking { failures, .. } => failures,
            _ => 0,
        };
        self.calls
            .retain(|_, c| !matches!(c.purpose, Purpose::Handshake));
        self.conn = Conn::Handshaking {
            failures,
            due: None,
        };
        self.call(
            Purpose::Handshake,
            Request {
                call: CallName::Handshake,
                body: serde_json::json!({}),
            },
            now,
        );
    }

    pub(super) fn on_handshake(&mut self, outcome: Outcome, now: Now) {
        let Conn::Handshaking { failures, .. } = self.conn else {
            return;
        };
        match outcome {
            Outcome::Refused(r) if r.code == RefusalCode::Unauthorized => {
                self.conn = Conn::Handshaking {
                    failures: failures + 1,
                    due: Some(now.mono.after(backoff(failures + 1))),
                };
            }
            Outcome::Done(_) | Outcome::Refused(_) => {
                self.conn = Conn::Ready;
                self.refresh_catalogue(now);
                if let Some(d) = self.watched.clone() {
                    self.fetch_decision(d, now);
                }
                if self.focused {
                    self.poll_all_now(now);
                }
            }
            Outcome::Unknown(Unknown::OtherProtocol(_)) => {}
            Outcome::Unknown(_) => {
                self.conn = Conn::Handshaking {
                    failures: failures + 1,
                    due: Some(now.mono.after(backoff(failures + 1))),
                };
            }
        }
    }

    /// `␣rc`: every stream reopens now, and a handshake still owed is made now.
    pub fn reconnect(&mut self, now: Now) {
        match self.conn {
            Conn::Handshaking { .. } => self.handshake(now),
            Conn::Ready => {
                let keys: Vec<StreamKey> = self.streams.keys().cloned().collect();
                for key in keys {
                    if let Some(id) = self.streams[&key].stream.attempt() {
                        self.effects.push(Effect::Close { id });
                    }
                    self.streams.get_mut(&key).expect("held").stream.failures = 0;
                    self.open(key, now);
                }
            }
            _ => {}
        }
        self.settle(self.connected(), now);
    }

    pub fn show(&mut self, slot: Slot, conv: Option<ConvRef>, now: Now) {
        let was_connected = self.connected();
        match conv {
            Some(conv) => {
                self.server.transcripts.entry(conv.clone()).or_default();
                self.slots.insert(slot.into(), conv);
            }
            None => {
                self.slots.remove(&slot.into());
            }
        }
        self.settle(was_connected, now);
    }

    /// The streams the client should hold now (`client.md` §3.1).
    fn wanted(&self) -> BTreeSet<StreamKey> {
        let mut wanted = BTreeSet::new();
        if self.conn != Conn::Ready {
            return wanted;
        }
        wanted.insert(StreamKey::Inbox);
        for p in self.server.projects.value().into_iter().flatten() {
            let gone = matches!(
                self.server.timelines.get(&p.project).map(|t| &t.docs),
                Some(Known::Gone { .. })
            );
            if !gone {
                wanted.insert(StreamKey::Timeline(p.project.clone()));
            }
        }
        for conv in self.slots.values() {
            if !matches!(
                self.server.transcripts.get(conv).map(|t| &t.docs),
                Some(Known::Gone { .. })
            ) {
                wanted.insert(StreamKey::Open(conv.clone()));
            }
        }
        wanted
    }

    pub(super) fn reconcile_streams(&mut self, now: Now) {
        let wanted = self.wanted();
        let unwanted: Vec<StreamKey> = self
            .streams
            .keys()
            .filter(|k| !wanted.contains(*k))
            .cloned()
            .collect();
        for key in unwanted {
            if let Some(id) = self.streams.remove(&key).and_then(|h| h.stream.attempt()) {
                self.effects.push(Effect::Close { id });
            }
        }
        for key in wanted {
            if !self.streams.contains_key(&key) {
                self.open(key, now);
            }
        }
    }

    pub(super) fn close_all(&mut self) {
        for held in std::mem::take(&mut self.streams).into_values() {
            if let Some(id) = held.stream.attempt() {
                self.effects.push(Effect::Close { id });
            }
        }
    }

    /// Opens (or reopens) a stream, asking `after` the newest entry held (`client.md` §3.4).
    fn open(&mut self, key: StreamKey, now: Now) {
        let id = StreamId(self.next_seq());
        let (call, body) = match &key {
            StreamKey::Inbox => (CallName::Inbox, serde_json::json!({})),
            StreamKey::Timeline(project) => {
                let mut body = serde_json::json!({ "project": project });
                if let Some(after) = self
                    .server
                    .timelines
                    .get(project)
                    .and_then(Transcript::newest)
                {
                    body["after"] = after.0.into();
                }
                (CallName::Timeline, body)
            }
            StreamKey::Open(conv) => {
                let mut body =
                    serde_json::json!({ "project": conv.project, "conversation": conv.id });
                if let Some(after) = self
                    .server
                    .transcripts
                    .get(conv)
                    .and_then(Transcript::newest)
                {
                    body["after"] = after.0.into();
                }
                (CallName::Open, body)
            }
        };
        self.effects.push(Effect::Open {
            id,
            request: Request { call, body },
        });
        match self.streams.get_mut(&key) {
            Some(held) => {
                held.stream.reopen(id, now.mono);
                held.value = None;
            }
            None => {
                self.streams.insert(
                    key,
                    Held {
                        stream: Stream::connecting(id, now.mono),
                        value: None,
                    },
                );
            }
        }
    }

    pub(super) fn tick_streams(&mut self, now: Now) {
        let keys: Vec<StreamKey> = self.streams.keys().cloned().collect();
        for key in keys {
            let stream = &self.streams[&key].stream;
            match stream.phase {
                Phase::Backoff { due } if due <= now.mono => self.open(key, now),
                _ if stream.silent_at().is_some_and(|at| at <= now.mono) => {
                    if let Some(id) = stream.attempt() {
                        self.effects.push(Effect::Close { id });
                    }
                    self.streams.get_mut(&key).expect("held").stream.last_cause =
                        Some(EndCause::Silent);
                    self.open(key, now);
                }
                _ => {}
            }
        }
    }

    pub(super) fn on_stream(&mut self, id: StreamId, event: StreamEvent, now: Now) {
        let Some(key) = self
            .streams
            .iter()
            .find(|(_, h)| h.stream.attempt() == Some(id))
            .map(|(k, _)| k.clone())
        else {
            return;
        };
        match event {
            StreamEvent::Frame(Frame::Heartbeat) => self.held(&key).stream.heard(now.mono),
            StreamEvent::Frame(Frame::Ops(ops)) => self.on_ops(key, id, &ops, now),
            StreamEvent::Frame(Frame::End(EndReason::Revoked)) => {
                self.streams.remove(&key);
                self.revoked();
            }
            StreamEvent::Frame(Frame::End(reason)) => {
                if matches!(reason, EndReason::Stopping | EndReason::Failed) {
                    self.events.push(LinkEvent::StreamEnded {
                        key: key.clone(),
                        reason: reason.clone(),
                    });
                }
                self.held(&key)
                    .stream
                    .ended(EndCause::End(reason), now.mono);
            }
            StreamEvent::Refused(Outcome::Refused(r)) if r.code.is_gone() => {
                self.streams.remove(&key);
                let gone = Known::Gone { message: r.message };
                match &key {
                    StreamKey::Open(conv) => {
                        self.server
                            .transcripts
                            .entry(conv.clone())
                            .or_default()
                            .docs = gone
                    }
                    StreamKey::Timeline(p) => {
                        self.server.timelines.entry(p.clone()).or_default().docs = gone
                    }
                    StreamKey::Inbox => {}
                }
            }
            StreamEvent::Refused(Outcome::Refused(r)) => {
                if r.code == RefusalCode::Unauthorized {
                    self.band = Some(r.message.clone());
                }
                self.held(&key).stream.ended(EndCause::Refused(r), now.mono);
            }
            StreamEvent::Refused(Outcome::Unknown(Unknown::OtherProtocol(n))) => {
                self.other_protocol(n)
            }
            StreamEvent::Refused(Outcome::Unknown(u)) => {
                self.held(&key).stream.ended(EndCause::Unknown(u), now.mono)
            }
            StreamEvent::Refused(Outcome::Done(_)) => self.held(&key).stream.ended(
                EndCause::Unknown(Unknown::Unreadable { status: 200 }),
                now.mono,
            ),
            StreamEvent::BadFrame(text) => self.bad_frame(&key, id, text, now),
            StreamEvent::Closed(error) => {
                self.held(&key).stream.ended(EndCause::Cut(error), now.mono)
            }
        }
    }

    fn held(&mut self, key: &StreamKey) -> &mut Held {
        self.streams.get_mut(key).expect("held stream")
    }

    fn bad_frame(&mut self, key: &StreamKey, id: StreamId, text: String, now: Now) {
        self.effects.push(Effect::Close { id });
        let held = self.held(key);
        held.value = None;
        held.stream.ended(EndCause::BadFrame(text), now.mono);
    }

    fn revoked(&mut self) {
        self.close_all();
        self.conn = Conn::Revoked;
        self.events.push(LinkEvent::Revoked);
    }

    fn on_ops(&mut self, key: StreamKey, id: StreamId, frame: &[Value], now: Now) {
        let first = matches!(self.held(&key).stream.phase, Phase::Connecting { .. });
        let held = self.held(&key);
        let mut value = held.value.take().unwrap_or(Value::Null);
        if let Err(e) = ops::apply(&mut value, frame) {
            return self.bad_frame(&key, id, e.to_string(), now);
        }
        if first {
            held.stream.first_frame(now.mono, now.wall);
        } else {
            held.stream.heard(now.mono);
        }
        held.value = Some(value.clone());
        self.received_anything = true;
        self.lift_band(now);
        match key {
            StreamKey::Inbox => self.on_inbox(&value, first.then_some(id), now),
            StreamKey::Timeline(project) => {
                let t = self.server.timelines.entry(project.clone()).or_default();
                let gap = absorb(t, &value, now);
                self.after_absorb(TranscriptKey::Timeline(project), gap, now);
            }
            StreamKey::Open(conv) => {
                let t = self.server.transcripts.entry(conv.clone()).or_default();
                let gap = absorb(t, &value, now);
                self.after_absorb(TranscriptKey::Conversation(conv), gap, now);
            }
        }
    }

    fn after_absorb(&mut self, key: TranscriptKey, gap: Option<Gap>, now: Now) {
        if let TranscriptKey::Conversation(conv) = &key {
            self.label_from_transcript(conv);
        }
        let Some(gap) = gap else { return };
        let Some(t) = self.transcript(&key) else {
            return;
        };
        let Some(conversation) = t.conversation else {
            return;
        };
        let merged = match &t.gap {
            Some(open) => Gap {
                min: open.min.min(gap.min),
                max: open.max.max(gap.max),
            },
            None => gap.clone(),
        };
        self.transcript_mut(&key).gap = Some(merged);
        let query = HistoryQuery {
            project: key.project().clone(),
            conversation,
            min_entry_id: Some(gap.min),
            max_entry_id: Some(gap.max),
            order: Order::Ascending,
            limit: HISTORY_PAGE,
        };
        self.history(
            Purpose::Gap {
                key,
                query: query.clone(),
            },
            &query,
            None,
            now,
        );
    }

    pub(super) fn transcript(&self, key: &TranscriptKey) -> Option<&Transcript> {
        match key {
            TranscriptKey::Conversation(c) => self.server.transcripts.get(c),
            TranscriptKey::Timeline(p) => self.server.timelines.get(p),
        }
    }

    pub(super) fn transcript_mut(&mut self, key: &TranscriptKey) -> &mut Transcript {
        match key {
            TranscriptKey::Conversation(c) => self.server.transcripts.entry(c.clone()).or_default(),
            TranscriptKey::Timeline(p) => self.server.timelines.entry(p.clone()).or_default(),
        }
    }

    /// A gap page arrived: keep its entries; follow `next`; the gap closes when no sequence for it is left.
    pub(super) fn on_gap(
        &mut self,
        key: TranscriptKey,
        query: HistoryQuery,
        outcome: Outcome,
        now: Now,
    ) {
        match outcome {
            Outcome::Done(v) => {
                let page: crate::wire::Page<Value> =
                    serde_json::from_value(v).unwrap_or(crate::wire::Page {
                        items: vec![],
                        next: None,
                    });
                keep_entries(self.transcript_mut(&key), page.items);
                if let TranscriptKey::Conversation(conv) = &key {
                    self.label_from_transcript(conv);
                }
                match page.next {
                    Some(cursor) => self.history(
                        Purpose::Gap {
                            key,
                            query: query.clone(),
                        },
                        &query,
                        Some(&cursor),
                        now,
                    ),
                    None => self.gap_closed(&key),
                }
            }
            Outcome::Refused(r) if r.code == RefusalCode::BadCursor => {
                self.history(
                    Purpose::Gap {
                        key,
                        query: query.clone(),
                    },
                    &query,
                    None,
                    now,
                );
            }
            Outcome::Refused(r) if r.code.is_gone() => {
                self.transcript_mut(&key).docs = Known::Gone { message: r.message };
                self.gap_closed(&key);
            }
            Outcome::Refused(_) | Outcome::Unknown(_) => {
                self.gap_retries
                    .push((now.mono.after(Duration::from_secs(2)), key, query));
            }
        }
    }

    fn gap_closed(&mut self, key: &TranscriptKey) {
        let fetching = self.in_flight(|p| matches!(p, Purpose::Gap { key: k, .. } if k == key))
            || self.gap_retries.iter().any(|(_, k, _)| k == key);
        if !fetching {
            self.transcript_mut(key).gap = None;
        }
    }

    /// The inbox's value changed: read it, say what left or was superseded, and settle lost answers.
    fn on_inbox(&mut self, value: &Value, fresh: Option<StreamId>, now: Now) {
        let inbox = read_inbox(value);
        let before = self.server.inbox.value().cloned();
        let first_ever = before.is_none();
        if let Some(before) = &before {
            for (project, subjects) in &before.projects {
                for (subject, old) in subjects {
                    let decision = DecisionRef {
                        project: project.clone(),
                        id: old.id.clone(),
                    };
                    match inbox.projects.get(project).and_then(|s| s.get(subject)) {
                        Some(now_open) if now_open.id == old.id => continue,
                        Some(now_open) => self.events.push(LinkEvent::Superseded {
                            decision: decision.clone(),
                            by: now_open.id.clone(),
                        }),
                        None => {
                            let ours = self.outbox.entries.iter().any(|e| {
                                e.call == WriteCall::Answer
                                    && e.decision().as_ref() == Some(&decision)
                                    && !matches!(e.state, WriteState::Refused(_))
                            });
                            self.events.push(LinkEvent::DecisionLeft {
                                decision: decision.clone(),
                                ours,
                            });
                        }
                    }
                    self.subject_changed(project, subject, now);
                }
            }
        }
        self.server.inbox = Known::Have {
            value: inbox,
            at: now.wall,
        };
        self.settle_answers(fresh, now);
        if fresh.is_some() && !first_ever {
            self.reconnected(now);
        }
        if fresh.is_some() && first_ever && self.focused {
            self.poll_all_now(now);
        }
    }

    /// Refetches the decisions of a subject the inbox changed: the one shown, and any cached as open.
    fn subject_changed(&mut self, project: &Project, subject: &Subject, now: Now) {
        let stale: Vec<DecisionRef> = self
            .server
            .decisions
            .iter()
            .filter(|(d, known)| {
                &d.project == project
                    && &d.id.subject() == subject
                    && (self.watched.as_ref() == Some(*d)
                        || known.value().is_some_and(|v| {
                            v.decision.status == crate::wire::DecisionStatus::Open
                        }))
            })
            .map(|(d, _)| d.clone())
            .collect();
        for d in stale {
            self.fetch_decision(d, now);
        }
    }
}

impl TranscriptKey {
    pub fn project(&self) -> &Project {
        match self {
            TranscriptKey::Conversation(c) => &c.project,
            TranscriptKey::Timeline(p) => p,
        }
    }
}

/// The 1, 2, 4, 8, 16, 30 s steps, shared with the streams.
pub(super) fn backoff(failures: u32) -> Duration {
    crate::streams::backoff(failures)
}

fn read_inbox(value: &Value) -> Inbox {
    let mut inbox = Inbox::default();
    for (project, subjects) in value.as_object().into_iter().flatten() {
        let decisions = subjects
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(subject, d)| {
                Some((
                    Subject::from(subject.as_str()),
                    serde_json::from_value::<OpenDecision>(d.clone()).ok()?,
                ))
            })
            .collect();
        inbox
            .projects
            .insert(Project::from(project.as_str()), decisions);
    }
    inbox
}

/// Takes a `ConversationView` into the transcript. Returns the gap to fetch, by the rule of `client.md` §3.3:
/// compared with the newest entry held before this frame, and only when something was held.
fn absorb(t: &mut Transcript, view: &Value, now: Now) -> Option<Gap> {
    let held = t.newest();
    t.conversation = view["conversation"]["id"]
        .as_u64()
        .map(crate::ids::ConversationId);
    t.before = view["before"].as_u64().map(EntryId);
    keep_entries(t, view["entries"].as_array().cloned().unwrap_or_default());
    t.docs = Known::Have {
        value: Docs::parse(&view["docs"]),
        at: now.wall,
    };
    if t.before.is_none() {
        t.older = Older::Complete;
    }
    match (held, t.before) {
        (Some(held), Some(before)) if before > held => Some(Gap {
            min: EntryId(held.0 + 1),
            max: before,
        }),
        _ => None,
    }
}

pub(super) fn keep_entries(t: &mut Transcript, items: Vec<Value>) {
    for raw in items {
        if let Some(entry) = Entry::parse(raw)
            && !t.entries.contains_key(&entry.id)
        {
            t.entries.insert(entry.id, entry);
        }
    }
}
