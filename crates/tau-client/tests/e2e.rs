//! The link against a real tau server: `nix run .#e2e` starts one with the fake model `faux/echo` and the w2
//! extension, and passes it here as `TAU_URL`, `TAU_BIN`, `TAU_STATE` (the server's home), `TAU_PID` and
//! `TAU_FAUX_SCRIPT` (the fake model's script file, see `tools/e2e/tau-faux.mjs`). One scenario, in order:
//! the steps share the server, and the last one stops it.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use tau_client::clock::SystemClock;
use tau_client::shapes::{Block, ConversationState, EntryBody};
use tau_client::sse::EndReason;
use tau_client::store::Secrets;
use tau_client::streams::{Phase, StreamKey};
use tau_client::wire::InputMode;
use tau_client::{
    ConvRef, DecisionRef, Link, LinkConfig, LinkEvent, LinkPhase, Project, Slot, StartWith, Write,
    WriteId, WriteState,
};

fn env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} is not set: run this test with nix run .#e2e"))
}

/// `tau <args>` on the server's machine, as its owner: `device pair`, `device revoke`.
fn admin(args: &[&str]) -> String {
    let out = Command::new(env("TAU_BIN"))
        .args(args)
        .env("HOME", env("TAU_STATE"))
        .output()
        .expect("tau runs");
    assert!(
        out.status.success(),
        "tau {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("tau prints UTF-8")
}

/// What the fake model answers to its next requests, one step each: a text, or `{tool, arguments}`.
fn script(steps: Value) {
    std::fs::write(env("TAU_FAUX_SCRIPT"), steps.to_string()).expect("the script file is writable");
}

fn workdir(project: &str) -> String {
    let dir = PathBuf::from(env("TAU_STATE")).join("work").join(project);
    std::fs::create_dir_all(&dir).expect("the work dir can be made");
    dir.to_string_lossy().into_owned()
}

/// One paired device: its link, and every event the link reported.
struct Device {
    link: Link,
    events: Vec<LinkEvent>,
    woken: mpsc::Receiver<()>,
    _dir: tempfile::TempDir,
}

impl Device {
    /// Pairs `name` with a code from the server's owner, and waits until the link is live.
    fn pair(name: &str) -> Device {
        let dir = tempfile::tempdir().expect("a temp dir");
        let config = LinkConfig {
            dir: Some(dir.path().to_path_buf()),
            secrets: Secrets::memory(),
            clock: Arc::new(SystemClock),
        };
        let mut link = Link::open(config);
        assert_eq!(link.status().phase, LinkPhase::NotPaired);
        let (tx, woken) = mpsc::channel();
        link.set_waker(move || {
            let _ = tx.send(());
        });
        let printed = admin(&["device", "pair", name]);
        let code = printed
            .lines()
            .find_map(|l| l.strip_prefix("Pairing code: "))
            .unwrap_or_else(|| panic!("no pairing code in {printed}"));
        link.pair(&env("TAU_URL"), code);
        link.set_focused(true);
        let mut device = Device {
            link,
            events: Vec::new(),
            woken,
            _dir: dir,
        };
        device.until("paired", |d| {
            d.events
                .iter()
                .any(|e| matches!(e, LinkEvent::Paired { device } if device.to_string() == name))
        });
        device.until("live", |d| d.link.status().phase == LinkPhase::Live);
        device
    }

    /// Pumps as a UI does on each wake, until `done` holds.
    fn until(&mut self, what: &str, mut done: impl FnMut(&mut Device) -> bool) {
        self.until_some(what, |d| done(d).then_some(()))
    }

    fn until_some<T>(&mut self, what: &str, mut done: impl FnMut(&mut Device) -> Option<T>) -> T {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            self.link.pump();
            self.events.extend(self.link.take_events());
            if let Some(found) = done(self) {
                return found;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}; status {:?}; events {:?}",
                self.link.status(),
                self.events
            );
            let _ = self.woken.recv_timeout(Duration::from_millis(100));
        }
    }

    fn write(&mut self, write: Write) -> WriteId {
        self.link.write(write).expect("the link takes the write")
    }

    /// Waits for a write's result.
    fn done(&mut self, id: WriteId) -> Value {
        self.until_some("a write's result", |d| {
            match &d.link.outbox().get(id)?.state {
                WriteState::Done { result, .. } => Some(result.clone()),
                WriteState::Sending | WriteState::Waiting { .. } => None,
                other => panic!("write {id:?} ended {other:?}"),
            }
        })
    }

    fn started(&mut self) -> ConvRef {
        self.until_some("a started conversation", |d| {
            d.events.iter().find_map(|e| match e {
                LinkEvent::Started { conv, .. } => Some(conv.clone()),
                _ => None,
            })
        })
    }

    fn state(&self, conv: &ConvRef) -> Option<ConversationState> {
        Some(
            self.link
                .server()
                .transcripts
                .get(conv)?
                .docs
                .value()?
                .live
                .state(),
        )
    }

    /// The text of every assistant message in the conversation's transcript, oldest first.
    fn replies(&self, conv: &ConvRef) -> Vec<String> {
        let Some(t) = self.link.server().transcripts.get(conv) else {
            return Vec::new();
        };
        t.entries
            .values()
            .filter_map(|e| match &e.body {
                EntryBody::Assistant(m) => Some(
                    m.blocks
                        .iter()
                        .filter_map(|b| match b {
                            Block::Text(t) => Some(t.as_str()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .collect()
    }
}

#[test]
fn a_real_server() {
    let mut laptop = pairs_and_connects();
    binds_a_project(&mut laptop);
    let conv = starts_a_bare_conversation(&mut laptop);
    stops_and_continues(&mut laptop, &conv);
    answers_a_w2_decision(&mut laptop);
    revoke_ends_the_link(&mut laptop);
    a_stopping_server_is_retried(&mut laptop);
}

fn pairs_and_connects() -> Device {
    let mut laptop = Device::pair("laptop");
    laptop.until("the catalogue", |d| {
        let s = d.link.server();
        let model = s
            .models
            .value()
            .is_some_and(|m| m.iter().any(|m| m.model == "faux/echo"));
        let w2 = s
            .extensions
            .value()
            .is_some_and(|e| e.iter().any(|e| e.name == "w2" && e.workflow));
        model && w2 && s.projects.value().is_some()
    });
    assert_eq!(laptop.link.server().open_decisions(), Some(0));
    laptop
}

fn binds_a_project(laptop: &mut Device) {
    let bind = laptop.write(Write::Bind {
        project: "bare".into(),
        workdir: workdir("bare"),
        extensions: vec![],
    });
    assert_eq!(laptop.done(bind)["repeated"], false);
    let bare = Project::from("bare");
    laptop.until("the project's timeline", |d| {
        let listed = d
            .link
            .server()
            .projects
            .value()
            .is_some_and(|p| p.iter().any(|p| p.project == bare));
        let timeline = d.link.status().streams.iter().any(|(key, stream)| {
            *key == StreamKey::Timeline(bare.clone()) && matches!(stream.phase, Phase::Live { .. })
        });
        listed && timeline
    });
}

fn starts_a_bare_conversation(laptop: &mut Device) -> ConvRef {
    let text = "hello from the e2e test";
    laptop.write(Write::Start {
        project: "bare".into(),
        with: StartWith::Model("faux/echo".into()),
        text: text.into(),
    });
    let conv = laptop.started();
    laptop.link.show(Slot::Stage, Some(conv.clone()));
    let echo = format!("echo: {text}");
    laptop.until("the echo reply on the open stream", |d| {
        d.replies(&conv).contains(&echo)
    });
    let transcript = &laptop.link.server().transcripts[&conv];
    let inputs: Vec<_> = transcript
        .entries
        .values()
        .filter_map(|e| e.user_text())
        .collect();
    assert_eq!(inputs, [text], "one input, sent once");
    laptop.until("the conversation in the poll", |d| {
        d.link
            .server()
            .conversations
            .get(&conv.project)
            .and_then(|k| k.value())
            .is_some_and(|rows| rows.iter().any(|r| r.conversation == conv.id))
    });
    conv
}

fn stops_and_continues(laptop: &mut Device, conv: &ConvRef) {
    script(json!(["word ".repeat(2000)]));
    let before = laptop.replies(conv).len();
    let input = laptop.write(Write::Input {
        conv: conv.clone(),
        text: "keep going".into(),
        mode: InputMode::Send,
    });
    laptop.done(input);
    laptop.until("a long generation", |d| {
        let docs = d.link.server().transcripts[conv].docs.value().cloned();
        docs.and_then(|docs| docs.live.generation)
            .and_then(|g| g.message)
            .is_some()
    });
    let stop = laptop.write(Write::Stop { conv: conv.clone() });
    laptop.done(stop);
    laptop.until("the run stopped", |d| {
        d.state(conv) == Some(ConversationState::Stopped)
    });
    let resume = laptop.write(Write::Continue { conv: conv.clone() });
    laptop.done(resume);
    laptop.until("the continued run's reply", |d| {
        d.state(conv) == Some(ConversationState::Idle)
            && d.replies(conv).iter().any(|r| r == "echo: keep going")
    });
    assert!(laptop.replies(conv).len() > before);
}

fn answers_a_w2_decision(laptop: &mut Device) {
    let bind = laptop.write(Write::Bind {
        project: "flow".into(),
        workdir: workdir("flow"),
        extensions: vec!["w2".into()],
    });
    laptop.done(bind);
    let mut patch = serde_json::Map::new();
    patch.insert("model".into(), json!("faux/echo"));
    let options = laptop.write(Write::SetOptions {
        project: "flow".into(),
        extension: "w2".into(),
        patch,
    });
    laptop.done(options);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    script(json!([
        {"tool": "record_forecast", "arguments": {
            "key": "release", "text": "pi-durable ships 2.0 before March.", "p": 0.7,
            "resolveBy": now_ms, "rule": "A public 2.0 release before March 1."}},
        "Recorded the forecast."
    ]));
    laptop.events.clear();
    laptop.write(Write::Start {
        project: "flow".into(),
        with: StartWith::Workflow("w2".into()),
        text: "Forecast the 2.0 release".into(),
    });
    let conv = laptop.started();
    let flow = Project::from("flow");
    let open = laptop.until_some("the decision in the inbox", |d| {
        let inbox = d.link.server().inbox.value()?;
        inbox
            .projects
            .get(&flow)?
            .values()
            .find(|o| o.conversation == conv.id)
            .cloned()
    });
    assert_eq!(laptop.link.server().open_decisions(), Some(1));
    let decision = DecisionRef {
        project: flow.clone(),
        id: open.id.clone(),
    };
    laptop.link.fetch_decision(decision.clone());
    let record = laptop.until_some("the decision record", |d| {
        d.link
            .server()
            .decisions
            .get(&decision)?
            .value()
            .map(|v| v.decision.clone())
    });
    assert_eq!(record.title, "Resolve forecast");
    let choice = record
        .choices
        .first()
        .expect("the decision offers choices")
        .clone();
    let answer = laptop.write(Write::Answer {
        decision: decision.clone(),
        basis: record.basis,
        choice: choice.clone(),
        annotations: vec![],
    });
    assert_eq!(laptop.done(answer)["repeated"], false);
    laptop.until("the decision leaves the inbox as ours", |d| {
        d.events.iter().any(|e| matches!(e, LinkEvent::DecisionLeft { decision: left, ours: true } if *left == decision))
    });
    assert_eq!(laptop.link.server().open_decisions(), Some(0));
    let answered = laptop.link.server().decisions[&decision]
        .value()
        .expect("the record stays")
        .decision
        .clone();
    assert_eq!(answered.choice.as_deref(), Some(choice.as_str()));
}

fn revoke_ends_the_link(laptop: &mut Device) {
    let mut tablet = Device::pair("tablet");
    admin(&["device", "revoke", "tablet"]);
    tablet.until("the revoked state", |d| {
        d.link.status().phase == LinkPhase::Revoked && d.events.contains(&LinkEvent::Revoked)
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        tablet.link.pump();
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(
        tablet.link.status().phase,
        LinkPhase::Revoked,
        "no reconnect after a revoke"
    );
    assert!(
        tablet.link.status().streams.is_empty(),
        "no stream is held or reopened"
    );
    laptop.link.pump();
    assert_eq!(
        laptop.link.status().phase,
        LinkPhase::Live,
        "another device's revoke leaves this one alone"
    );
}

fn a_stopping_server_is_retried(laptop: &mut Device) {
    laptop.events.clear();
    let pid: i32 = env("TAU_PID").parse().expect("TAU_PID is a process ID");
    // SAFETY: kill(2) with a valid signal number has no memory effects.
    assert_eq!(
        unsafe { libc::kill(pid, libc::SIGTERM) },
        0,
        "the server takes SIGTERM"
    );
    laptop.until("end: stopping and retrying", |d| {
        let ended = d.events.iter().any(|e| {
            matches!(
                e,
                LinkEvent::StreamEnded {
                    key: StreamKey::Inbox,
                    reason: EndReason::Stopping
                }
            )
        });
        ended && d.link.status().phase == LinkPhase::ServerStopped
    });
}
