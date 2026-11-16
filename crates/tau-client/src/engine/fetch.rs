//! What the client asks for: the catalogue, the polls of `client.md` §5, options, decisions, labels and older
//! entries.

use super::link::{HISTORY_PAGE, keep_entries};
use super::*;
use crate::model::{Known, Older};
use crate::shapes::{Entry, EntryBody, label};
use crate::wire::{
    DecisionStatus, DecisionView, ExtensionInfo, ModelInfo, OptionsItem, Order, Page, ProjectView,
};

/// `conversations` per project, every 10 s while focused.
pub const CONVERSATIONS_EVERY: Duration = Duration::from_secs(10);
/// `faults` per project, every 5 minutes while focused.
pub const FAULTS_EVERY: Duration = Duration::from_secs(300);
/// A poll this long after this device's own `input`, `continue`, `stop` or `create`.
pub const AFTER_OWN_WRITE: Duration = Duration::from_secs(1);
/// A label not found is looked for again at most this often.
pub const LABEL_RETRY: Duration = Duration::from_secs(60);
/// Pages read looking for a conversation's first input.
pub const LABEL_PAGES: u32 = 5;
/// A shown decision whose `then` task isn't terminal is fetched again this often.
pub const DECISION_EVERY: Duration = Duration::from_secs(30);

const CONVERSATIONS_LIMIT: u32 = 500;

impl Engine {
    /// `projects`, `extensions` and `models`.
    pub fn refresh_catalogue(&mut self, now: Now) {
        for call in [CallName::Projects, CallName::Extensions, CallName::Models] {
            self.catalogue(call, now);
        }
    }

    fn catalogue(&mut self, call: CallName, now: Now) {
        if self.conn != Conn::Ready
            || self.in_flight(|p| matches!(p, Purpose::Catalogue(c) if *c == call))
        {
            return;
        }
        self.call(
            Purpose::Catalogue(call),
            Request {
                call,
                body: serde_json::json!({}),
            },
            now,
        );
    }

    pub fn fetch_options(&mut self, project: Project, now: Now) {
        if self.conn != Conn::Ready
            || self.in_flight(|p| matches!(p, Purpose::Options(q) if *q == project))
        {
            return;
        }
        let body = serde_json::json!({ "project": project });
        self.call(
            Purpose::Options(project),
            Request {
                call: CallName::Options,
                body,
            },
            now,
        );
    }

    pub fn fetch_decision(&mut self, decision: DecisionRef, now: Now) {
        if self.conn != Conn::Ready
            || self.in_flight(|p| matches!(p, Purpose::Decision(d) if *d == decision))
        {
            return;
        }
        let body = serde_json::json!({ "project": decision.project, "decision": decision.id });
        self.call(
            Purpose::Decision(decision),
            Request {
                call: CallName::Decision,
                body,
            },
            now,
        );
    }

    /// The decision the stage shows, refreshed by the rules of `spec.md` §5.6.
    pub fn watch_decision(&mut self, decision: Option<DecisionRef>, now: Now) {
        if let Some(old) = self.watched.take() {
            self.refetch.remove(&old);
        }
        if let Some(d) = decision {
            self.watched = Some(d.clone());
            self.fetch_decision(d, now);
        }
    }

    pub fn set_focused(&mut self, focused: bool, now: Now) {
        let was = std::mem::replace(&mut self.focused, focused);
        if focused && !was {
            self.poll_all_now(now);
        }
        if !focused {
            for polls in self.polls.values_mut() {
                polls.conversations.due = None;
                polls.faults.due = None;
            }
        }
    }

    /// `faults` of every project, now (Home or Faults opened).
    pub fn refresh_faults(&mut self, now: Now) {
        for polls in self.polls.values_mut() {
            polls.faults.due = Some(now.mono);
        }
        self.tick_polls(now);
    }

    pub(super) fn poll_all_now(&mut self, now: Now) {
        for polls in self.polls.values_mut() {
            polls.conversations.due = Some(now.mono);
            polls.faults.due = Some(now.mono);
        }
        self.tick_polls(now);
    }

    /// The inbox came back after a cut: everything polled or fetched once is asked again (`client.md` §5,
    /// `core-reconciliation.md` "the extension catalogue is live").
    pub(super) fn reconnected(&mut self, now: Now) {
        self.refresh_catalogue(now);
        if self.focused {
            self.poll_all_now(now);
        }
    }

    /// Polls `conversations` of a project soon after this device's own write there.
    pub(super) fn poll_soon(&mut self, project: &Project, now: Now) {
        if !self.focused {
            return;
        }
        let soon = now.mono.after(AFTER_OWN_WRITE);
        let poll = &mut self.polls.entry(project.clone()).or_default().conversations;
        poll.due = Some(poll.due.map_or(soon, |d| d.min(soon)));
    }

    pub(super) fn tick_polls(&mut self, now: Now) {
        if self.conn != Conn::Ready {
            return;
        }
        let projects: Vec<Project> = self.polls.keys().cloned().collect();
        for project in projects {
            let polls = self.polls.get_mut(&project).expect("polled");
            let conversations = take_due(&mut polls.conversations, now);
            let faults = take_due(&mut polls.faults, now);
            if conversations {
                self.page(
                    Purpose::Conversations {
                        project: project.clone(),
                        rows: vec![],
                    },
                    None,
                    now,
                );
            }
            if faults {
                self.page(
                    Purpose::Faults {
                        project: project.clone(),
                        rows: vec![],
                    },
                    None,
                    now,
                );
            }
        }
    }

    /// One page of `conversations` or `faults`.
    fn page(&mut self, purpose: Purpose, cursor: Option<String>, now: Now) {
        let (call, project) = match &purpose {
            Purpose::Conversations { project, .. } => (CallName::Conversations, project),
            Purpose::Faults { project, .. } => (CallName::Faults, project),
            _ => unreachable!("only polls page"),
        };
        let mut body = serde_json::json!({ "project": project });
        if call == CallName::Conversations {
            body["limit"] = CONVERSATIONS_LIMIT.into();
        }
        if let Some(cursor) = cursor {
            body["cursor"] = cursor.into();
        }
        self.call(purpose, Request { call, body }, now);
    }

    /// The UI draws this conversation's row: find its label if it isn't known (`client.md` §5.1).
    pub fn want_label(&mut self, conv: ConvRef, now: Now) {
        if self.server.labels.found.contains_key(&conv) || self.conn != Conn::Ready {
            return;
        }
        self.label_from_transcript(&conv);
        if self.server.labels.found.contains_key(&conv)
            || self.in_flight(|p| matches!(p, Purpose::Label { conv: c, .. } if *c == conv))
            || self
                .label_tried
                .get(&conv)
                .is_some_and(|at| now.mono.since(*at) < LABEL_RETRY)
        {
            return;
        }
        self.label_tried.insert(conv.clone(), now.mono);
        self.label_page(conv, 1, None, now);
    }

    fn label_page(&mut self, conv: ConvRef, pages: u32, cursor: Option<&str>, now: Now) {
        let query = label_query(&conv);
        let body = query.body(cursor);
        self.call(
            Purpose::Label { conv, pages },
            Request {
                call: CallName::History,
                body,
            },
            now,
        );
    }

    /// A conversation whose start is held takes its label from its first input.
    pub(super) fn label_from_transcript(&mut self, conv: &ConvRef) {
        if self.server.labels.found.contains_key(conv) {
            return;
        }
        if let Some(found) = self.server.transcripts.get(conv).and_then(|t| t.label()) {
            self.found_label(conv.clone(), found);
        }
    }

    fn found_label(&mut self, conv: ConvRef, text: String) {
        self.server.labels.asked.remove(&conv);
        self.server.labels.found.insert(conv, text);
        self.effects.push(Effect::SaveLabels);
    }

    /// One page of entries older than the oldest held.
    pub fn page_older(&mut self, key: TranscriptKey, now: Now) {
        let Some(t) = self.transcript(&key) else {
            return;
        };
        let Some(conversation) = t.conversation else {
            return;
        };
        let (max, cursor) = match &t.older {
            Older::Complete | Older::Fetching => return,
            Older::More { cursor, max } => (*max, Some(cursor.clone())),
            Older::Unknown => match t.oldest() {
                Some(oldest) if oldest.0 > 1 => (crate::ids::EntryId(oldest.0 - 1), None),
                Some(_) => {
                    self.transcript_mut(&key).older = Older::Complete;
                    return;
                }
                None => return,
            },
        };
        let query = HistoryQuery {
            project: key.project().clone(),
            conversation,
            min_entry_id: None,
            max_entry_id: Some(max),
            order: Order::Descending,
            limit: HISTORY_PAGE,
        };
        self.transcript_mut(&key).older = Older::Fetching;
        self.history(
            Purpose::Older {
                key,
                query: query.clone(),
                cursor: cursor.clone(),
            },
            &query,
            cursor.as_deref(),
            now,
        );
    }

    /// One `history` call, its cursor sent back with the same bounds.
    pub(super) fn history(
        &mut self,
        purpose: Purpose,
        query: &HistoryQuery,
        cursor: Option<&str>,
        now: Now,
    ) {
        if self.conn != Conn::Ready {
            return;
        }
        let body = query.body(cursor);
        self.call(
            purpose,
            Request {
                call: CallName::History,
                body,
            },
            now,
        );
    }

    pub(super) fn on_fetched(&mut self, purpose: Purpose, outcome: Outcome, now: Now) {
        match purpose {
            Purpose::Catalogue(call) => self.on_catalogue(call, outcome, now),
            Purpose::Options(project) => {
                if let Some(known) = known_from::<Vec<OptionsItem>>(outcome, "items", now) {
                    self.server.options.insert(project, known);
                }
            }
            Purpose::Conversations { project, rows } => {
                self.on_conversations(project, rows, outcome, now)
            }
            Purpose::Faults { project, rows } => self.on_faults(project, rows, outcome, now),
            Purpose::Decision(decision) => self.on_decision(decision, outcome, now),
            Purpose::Label { conv, pages } => self.on_label(conv, pages, outcome, now),
            Purpose::Gap { key, query } => self.on_gap(key, query, outcome, now),
            Purpose::Older { key, query, cursor } => {
                self.on_older(key, query, cursor, outcome, now)
            }
            Purpose::Pair { .. } | Purpose::Handshake | Purpose::Write(_) => {
                unreachable!("handled in on_reply")
            }
        }
    }

    fn on_catalogue(&mut self, call: CallName, outcome: Outcome, now: Now) {
        match call {
            CallName::Projects => {
                if let Some(known) = known_from::<Vec<ProjectView>>(outcome, "items", now) {
                    self.server.projects = known;
                    self.projects_changed(now);
                }
            }
            CallName::Extensions => {
                if let Some(known) = known_from::<Vec<ExtensionInfo>>(outcome, "items", now) {
                    self.server.extensions = known;
                }
            }
            CallName::Models => {
                if let Some(known) = known_from::<Vec<ModelInfo>>(outcome, "items", now) {
                    self.server.models = known;
                }
            }
            _ => {}
        }
    }

    /// Every project known gets its polls; a new one is polled at once while focused.
    pub(super) fn projects_changed(&mut self, now: Now) {
        let projects: Vec<Project> = self
            .server
            .projects
            .value()
            .into_iter()
            .flatten()
            .map(|p| p.project.clone())
            .collect();
        for project in projects {
            if !self.polls.contains_key(&project) {
                let due = self.focused.then_some(now.mono);
                let poll = || Poll {
                    due,
                    in_flight: false,
                };
                self.polls.insert(
                    project,
                    Polls {
                        conversations: poll(),
                        faults: poll(),
                    },
                );
            }
        }
        self.tick_polls(now);
    }

    fn on_conversations(
        &mut self,
        project: Project,
        mut rows: Vec<ConversationRow>,
        outcome: Outcome,
        now: Now,
    ) {
        let page = match outcome {
            Outcome::Done(v) => serde_json::from_value::<Page<ConversationRow>>(v).ok(),
            Outcome::Refused(r) if r.code == RefusalCode::BadCursor => {
                return self.page(
                    Purpose::Conversations {
                        project,
                        rows: vec![],
                    },
                    None,
                    now,
                );
            }
            Outcome::Refused(r) if r.code.is_gone() => {
                self.server
                    .conversations
                    .insert(project.clone(), Known::Gone { message: r.message });
                None
            }
            _ => None,
        };
        if let Some(page) = page {
            rows.extend(page.items);
            if let Some(next) = page.next {
                return self.page(Purpose::Conversations { project, rows }, Some(next), now);
            }
            rows.sort_by_key(|r| std::cmp::Reverse(r.conversation));
            self.server.conversations.insert(
                project.clone(),
                Known::Have {
                    value: rows,
                    at: now.wall,
                },
            );
        }
        self.poll_done(&project, |p| &mut p.conversations, CONVERSATIONS_EVERY, now);
    }

    fn on_faults(&mut self, project: Project, mut rows: Vec<Fault>, outcome: Outcome, now: Now) {
        let page = match outcome {
            Outcome::Done(v) => serde_json::from_value::<Page<Fault>>(v).ok(),
            Outcome::Refused(r) if r.code == RefusalCode::BadCursor => {
                return self.page(
                    Purpose::Faults {
                        project,
                        rows: vec![],
                    },
                    None,
                    now,
                );
            }
            Outcome::Refused(r) if r.code.is_gone() => {
                self.server
                    .faults
                    .insert(project.clone(), Known::Gone { message: r.message });
                None
            }
            _ => None,
        };
        if let Some(page) = page {
            rows.extend(page.items);
            if let Some(next) = page.next {
                return self.page(Purpose::Faults { project, rows }, Some(next), now);
            }
            let named = self.watched.clone().filter(|w| {
                w.project == project && rows.iter().any(|f| f.decision.as_ref() == Some(&w.id))
            });
            self.server.faults.insert(
                project.clone(),
                Known::Have {
                    value: rows,
                    at: now.wall,
                },
            );
            if let Some(watched) = named {
                self.fetch_decision(watched, now);
            }
        }
        self.poll_done(&project, |p| &mut p.faults, FAULTS_EVERY, now);
    }

    /// A poll sequence ended: the next is due after its interval, or sooner if a write asked for it meanwhile.
    fn poll_done(
        &mut self,
        project: &Project,
        poll: impl Fn(&mut Polls) -> &mut Poll,
        every: Duration,
        now: Now,
    ) {
        let focused = self.focused;
        let poll = poll(self.polls.entry(project.clone()).or_default());
        poll.in_flight = false;
        if focused {
            let next = now.mono.after(every);
            poll.due = Some(poll.due.map_or(next, |d| d.min(next)));
        }
    }

    fn on_decision(&mut self, decision: DecisionRef, outcome: Outcome, now: Now) {
        match outcome {
            Outcome::Done(v) => {
                let Ok(view) = serde_json::from_value::<DecisionView>(v) else {
                    return;
                };
                let answered = view.decision.status == DecisionStatus::Answered;
                let settled = view.then.as_ref().is_some_and(|t| t.is_terminal());
                if self.watched.as_ref() == Some(&decision) && answered && !settled {
                    self.refetch
                        .insert(decision.clone(), now.mono.after(DECISION_EVERY));
                }
                self.settle_answer_from(&decision, &view.decision, now);
                self.server.decisions.insert(
                    decision,
                    Known::Have {
                        value: view,
                        at: now.wall,
                    },
                );
            }
            Outcome::Refused(r) if r.code.is_gone() => {
                self.server
                    .decisions
                    .insert(decision, Known::Gone { message: r.message });
            }
            _ => {}
        }
    }

    fn on_label(&mut self, conv: ConvRef, pages: u32, outcome: Outcome, now: Now) {
        let Outcome::Done(v) = outcome else { return };
        let Ok(page) = serde_json::from_value::<Page<Value>>(v) else {
            return;
        };
        let first_input = page
            .items
            .into_iter()
            .filter_map(Entry::parse)
            .find(|e| matches!(e.body, EntryBody::User { .. }));
        if let Some(text) = first_input.as_ref().and_then(Entry::user_text) {
            return self.found_label(conv, label(text));
        }
        match page.next {
            Some(cursor) if pages < LABEL_PAGES => {
                self.label_page(conv, pages + 1, Some(&cursor), now)
            }
            _ => {
                self.server.labels.asked.insert(conv, now.mono);
            }
        }
    }

    fn on_older(
        &mut self,
        key: TranscriptKey,
        query: HistoryQuery,
        cursor: Option<String>,
        outcome: Outcome,
        now: Now,
    ) {
        let max = query.max_entry_id.unwrap_or_default();
        let restore = match &cursor {
            Some(c) => Older::More {
                cursor: c.clone(),
                max,
            },
            None => Older::Unknown,
        };
        match outcome {
            Outcome::Done(v) => {
                let page: Page<Value> = serde_json::from_value(v).unwrap_or(Page {
                    items: vec![],
                    next: None,
                });
                let t = self.transcript_mut(&key);
                keep_entries(t, page.items);
                t.older = match page.next {
                    Some(cursor) => Older::More { cursor, max },
                    None => Older::Complete,
                };
                if let TranscriptKey::Conversation(conv) = &key {
                    self.label_from_transcript(conv);
                }
            }
            Outcome::Refused(r) if r.code == RefusalCode::BadCursor => {
                self.history(
                    Purpose::Older {
                        key,
                        query: query.clone(),
                        cursor: None,
                    },
                    &query,
                    None,
                    now,
                );
            }
            Outcome::Refused(r) if r.code.is_gone() => {
                let t = self.transcript_mut(&key);
                t.docs = Known::Gone { message: r.message };
                t.older = restore;
            }
            _ => self.transcript_mut(&key).older = restore,
        }
    }
}

fn take_due(poll: &mut Poll, now: Now) -> bool {
    if poll.in_flight || !poll.due.is_some_and(|d| d <= now.mono) {
        return false;
    }
    poll.due = None;
    poll.in_flight = true;
    true
}

fn label_query(conv: &ConvRef) -> HistoryQuery {
    HistoryQuery {
        project: conv.project.clone(),
        conversation: conv.id,
        min_entry_id: None,
        max_entry_id: None,
        order: Order::Ascending,
        limit: 10,
    }
}

/// A read's outcome as the model keeps it: held, gone, or (on anything else) unchanged.
fn known_from<T: serde::de::DeserializeOwned>(
    outcome: Outcome,
    field: &str,
    now: Now,
) -> Option<Known<T>> {
    match outcome {
        Outcome::Done(mut v) => {
            let value = serde_json::from_value(v.get_mut(field)?.take()).ok()?;
            Some(Known::Have {
                value,
                at: now.wall,
            })
        }
        Outcome::Refused(r) if r.code.is_gone() => Some(Known::Gone { message: r.message }),
        _ => None,
    }
}
