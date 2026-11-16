//! What the frame shows of the server, as plain data, mapped from the link: the server line, the
//! count tags (each `None` until received), the open items' labels and marks, and the link's
//! notice band (`spec.md` §4.1, §4.4, §4.8; `client.md` §3.6, §3.7, §6.1, §7). The superseded
//! band is the decision screen's.

use std::collections::{HashMap, HashSet};

use tau_client::model::{Inbox, Liveness};
use tau_client::streams::StreamKey;
use tau_client::wire::{DecisionStatus, RunState};
use tau_client::{
    ConvRef, DecisionRef, Known, Label, Link, LinkPhase, LinkStatus, Notice as LinkNotice, Outbox,
    Project, Server, TaskId, Wall, WriteCall,
};

use crate::frame::{Notice, OpenGroup, OpenRow, ServerPhase, TroubleMark};
use crate::open::{Item, OpenItems};
use crate::time;
use crate::ui::{Counts, Mark, NoticeAction};

/// What the server has said about one open item.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemFacts {
    pub label: Option<String>,
    pub mark: Option<Mark>,
    pub gone: bool,
}

/// Failures put aside on this device with `x` (`spec.md` §5.3 "Hiding a failure").
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hidden {
    pub faults: HashSet<(Project, TaskId)>,
    pub runs: HashSet<ConvRef>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerView {
    pub phase: ServerPhase,
    pub counts: Counts,
    pub items: HashMap<Item, ItemFacts>,
    /// The link's notice band: offline, refused, revoked, another protocol, a stream that keeps
    /// failing.
    pub notice: Option<Notice>,
}

impl Default for ServerView {
    fn default() -> Self {
        ServerView {
            phase: ServerPhase::connecting(),
            counts: Counts::default(),
            items: HashMap::new(),
            notice: None,
        }
    }
}

impl ServerView {
    /// Reads the link as of `now`. An open item keeps the label it had when the server no longer
    /// names it (gone, or not received since launch).
    pub fn update(
        &mut self,
        link: &Link,
        open: &OpenItems,
        hidden: &Hidden,
        now: Wall,
        notes: usize,
    ) {
        let server = link.server();
        let status = link.status();
        self.phase = phase(&status.phase, server, now);
        self.counts = Counts {
            needs: server.open_decisions().map(|n| n as u32),
            failed: failed(server, hidden),
            unknown: Some(link.outbox().unknown_count() as u32),
        };
        self.items = open
            .iter()
            .map(|item| {
                let mut facts = facts(item, server, hidden);
                if facts.label.is_none() {
                    facts.label = self.label(item);
                }
                (item.clone(), facts)
            })
            .collect();
        self.notice = notice(status, link.outbox(), notes);
    }

    pub fn label(&self, item: &Item) -> Option<String> {
        self.items.get(item).and_then(|f| f.label.clone())
    }

    /// The sidebar's `OPEN` section: the open items by project, with what is known of each.
    pub fn open_groups(&self, open: &OpenItems) -> Vec<OpenGroup> {
        open.groups()
            .iter()
            .map(|(project, items)| OpenGroup {
                project: project.clone(),
                items: items
                    .iter()
                    .map(|item| {
                        let facts = self.items.get(item).cloned().unwrap_or_default();
                        OpenRow {
                            item: item.clone(),
                            label: facts.label,
                            mark: facts.mark,
                            gone: facts.gone,
                        }
                    })
                    .collect(),
            })
            .collect()
    }
}

/// The client's name for an open conversation.
pub fn conv_ref(item: &Item) -> Option<ConvRef> {
    match item {
        Item::Conversation { project, id } => Some(ConvRef::new(project.clone(), *id)),
        _ => None,
    }
}

/// The client's name for an open decision.
pub fn decision_ref(item: &Item) -> Option<DecisionRef> {
    match item {
        Item::Decision { project, id } => Some(DecisionRef::new(project.clone(), id.clone())),
        _ => None,
    }
}

fn phase(phase: &LinkPhase, server: &Server, now: Wall) -> ServerPhase {
    let trouble = |mark, words: String| ServerPhase::Trouble { mark, words };
    match phase {
        LinkPhase::NotPaired | LinkPhase::Pairing => ServerPhase::NotConnected,
        LinkPhase::OtherProtocol(n) => {
            trouble(TroubleMark::Failed, format!("server speaks protocol {n}"))
        }
        LinkPhase::Revoked => trouble(TroubleMark::Failed, "revoked".into()),
        LinkPhase::Refused { .. } => trouble(TroubleMark::Failed, "refused by the server".into()),
        LinkPhase::Offline { since, .. } => trouble(
            TroubleMark::Ring,
            format!("offline {}", time::span(time::between(*since, now))),
        ),
        LinkPhase::Connecting => ServerPhase::connecting(),
        LinkPhase::ServerStopped => trouble(TroubleMark::Time, "server stopped · retrying".into()),
        LinkPhase::Reconnecting { in_secs } => {
            trouble(TroubleMark::Ring, format!("reconnecting in {in_secs} s"))
        }
        LinkPhase::Live => ServerPhase::Calm {
            device: server
                .device
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            host: host(server.url.as_deref().unwrap_or_default()),
        },
    }
}

/// `https://tau.tail4e2f.ts.net/` → `tau.tail4e2f.ts.net`.
fn host(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    rest.split('/').next().unwrap_or_default().to_string()
}

/// Home's Failed head: failed runs and faulted tasks of every project, minus those hidden here;
/// `None` until `projects` and every project's `conversations` and `faults` have answered.
fn failed(server: &Server, hidden: &Hidden) -> Option<u32> {
    let mut n = 0;
    for p in server.projects.value()? {
        let project = &p.project;
        match server.conversations.get(project)? {
            Known::NotYet => return None,
            Known::Gone { .. } => {}
            Known::Have { value, .. } => {
                n += value
                    .iter()
                    .filter(|r| r.state == RunState::Failed)
                    .filter(|r| {
                        !hidden
                            .runs
                            .contains(&ConvRef::new(project.clone(), r.conversation.0))
                    })
                    .count();
            }
        }
        match server.faults.get(project)? {
            Known::NotYet => return None,
            Known::Gone { .. } => {}
            Known::Have { value, .. } => {
                n += value
                    .iter()
                    .filter(|f| !hidden.faults.contains(&(project.clone(), f.task)))
                    .count();
            }
        }
    }
    Some(n as u32)
}

fn facts(item: &Item, server: &Server, hidden: &Hidden) -> ItemFacts {
    let project = Project::new(item.project());
    let project_gone = matches!(server.conversations.get(&project), Some(Known::Gone { .. }));
    match item {
        Item::Conversation { .. } => {
            let conv = conv_ref(item).expect("a conversation");
            let label = match server.label(&conv) {
                Label::Found(text) => Some(text),
                Label::Empty | Label::NotYet => None,
            };
            let state = server
                .conversations
                .get(&project)
                .and_then(Known::value)
                .and_then(|rows| rows.iter().find(|r| r.conversation == conv.id))
                .map(|r| r.state.clone());
            let mark = match state {
                Some(RunState::Running) => Some(Mark::Running),
                Some(RunState::Failed) => Some(Mark::Failed),
                _ => None,
            };
            let gone = project_gone
                || matches!(
                    server.transcripts.get(&conv).map(|t| &t.docs),
                    Some(Known::Gone { .. })
                );
            ItemFacts { label, mark, gone }
        }
        Item::Decision { .. } => {
            let d = decision_ref(item).expect("a decision");
            let inbox = server.inbox.value();
            let fetched = server.decisions.get(&d);
            let label = inbox
                .and_then(|i| i.projects.get(&project))
                .and_then(|s| s.get(&d.id.subject()))
                .filter(|o| o.id == d.id)
                .map(|o| o.title.clone())
                .or_else(|| {
                    fetched
                        .and_then(Known::value)
                        .map(|v| v.decision.title.clone())
                });
            // An answer's reply (a `conflict`) can prove it answered before the inbox drops it.
            let answered = fetched
                .and_then(Known::value)
                .is_some_and(|v| v.decision.status == DecisionStatus::Answered);
            let mark =
                if !answered && inbox.is_some_and(|i: &Inbox| i.liveness(&d) == Liveness::Open) {
                    Some(Mark::Decision)
                } else if then_faulted(server, &d, hidden) {
                    Some(Mark::Failed)
                } else {
                    None
                };
            let gone = project_gone || matches!(fetched, Some(Known::Gone { .. }));
            ItemFacts { label, mark, gone }
        }
        Item::Project(_) => ItemFacts {
            label: None,
            mark: None,
            gone: project_gone,
        },
    }
}

/// The decision's `then` task faulted, and the fault isn't hidden here (`spec.md` §4.1).
fn then_faulted(server: &Server, d: &DecisionRef, hidden: &Hidden) -> bool {
    server
        .faults
        .get(&d.project)
        .and_then(Known::value)
        .is_some_and(|faults| {
            faults.iter().any(|f| {
                f.decision.as_ref() == Some(&d.id)
                    && !hidden.faults.contains(&(d.project.clone(), f.task))
            })
        })
}

const REVOKED: &str = "This device was revoked on the server. Nothing more is sent. To use it \
                       again, run tau device pair <new name> on the server (a revoked name can't \
                       be reused).";

/// The band the link's state needs, most urgent first; none while calm.
fn notice(status: &LinkStatus, outbox: &Outbox, notes: usize) -> Option<Notice> {
    let band = |mark, sentence: String, action| {
        Some(Notice {
            mark,
            sentence,
            action,
        })
    };
    let failed = Some(TroubleMark::Failed);
    match &status.phase {
        LinkPhase::OtherProtocol(n) => {
            return band(
                failed,
                format!("This server speaks protocol {n}; this client speaks 1."),
                None,
            );
        }
        LinkPhase::Revoked => {
            return band(
                failed,
                REVOKED.into(),
                Some(NoticeAction {
                    seq: "<leader>rp",
                    word: "pair again",
                }),
            );
        }
        LinkPhase::Refused { message } => {
            return band(failed, crate::copy::device_refused(message), None);
        }
        LinkPhase::Offline {
            since,
            received_anything,
        } => {
            let mut parts = vec![format!("Offline since {}", time::clock(*since))];
            if *received_anything {
                parts.push("showing the state as of then".into());
            }
            let unsent = outbox
                .entries()
                .iter()
                .filter(|e| e.call == WriteCall::Input && e.is_unsettled())
                .count();
            let mut safe = Vec::new();
            if notes > 0 {
                safe.push(format!("{notes} note{}", if notes == 1 { "" } else { "s" }));
            }
            if unsent > 0 {
                safe.push(format!(
                    "{unsent} unsent input{}",
                    if unsent == 1 { "" } else { "s" }
                ));
            }
            if !safe.is_empty() {
                parts.push(format!(
                    "{} {} safe",
                    safe.join(" and "),
                    if notes + unsent == 1 { "is" } else { "are" }
                ));
            }
            parts.push("writes resume when connected".into());
            return band(
                Some(TroubleMark::Ring),
                parts.join(" · "),
                Some(NoticeAction {
                    seq: "<leader>rc",
                    word: "reconnect",
                }),
            );
        }
        _ => {}
    }
    let failing = |key: &StreamKey| {
        status
            .notices
            .iter()
            .any(|LinkNotice::KeepsFailing(k)| k == key)
    };
    if failing(&StreamKey::Inbox) {
        return band(
            None,
            "Open decisions may be out of date for every project: the server keeps failing to \
             send them."
                .into(),
            None,
        );
    }
    status
        .notices
        .iter()
        .find_map(|LinkNotice::KeepsFailing(k)| {
            let project = match k {
                StreamKey::Inbox => return None,
                StreamKey::Timeline(p) => p,
                StreamKey::Open(c) => &c.project,
            };
            band(
                None,
                format!("The server keeps failing to stream {project}. Its last state is shown."),
                None,
            )
        })
}
