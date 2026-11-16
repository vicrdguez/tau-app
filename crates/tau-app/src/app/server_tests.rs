//! How what the frame shows of the server could fail, fed from `design/fixture.json` through a real
//! `Link` (`client.md` §9, `spec.md` §4.1, §4.4, §4.8):
//! - at launch, before any frame, the server line claims calm, a count tag shows, or an open item
//!   draws a mark;
//! - the `◆` count shows before the inbox arrived, or the `×` count before every project's
//!   `conversations` and `faults` answered, or it counts a failure hidden on this device;
//! - after the fixture's first frames, the counts or the open items' marks and labels differ from
//!   Home's (`◆ 3`, `× 4`; `#4` running, `#5` failed, both decisions open);
//! - offline after 60 s with nothing received, the band claims `showing the state as of then`;
//!   with something received, it drops it;
//! - `end: stopping` on every stream fails to read `◷ server stopped · retrying`, or clears the
//!   counts last received;
//! - `end: revoked` on a stream fails to show the revoked state and its rose band;
//! - `unauthorized` on a call, with no `end`, shows the revoked state instead of the refused band;
//! - an open item the server no longer has loses its label, or isn't marked gone;
//! - an unpaired link reads anything but `not connected`, or the stage stays off Connect;
//! - a link event is not echoed in the copy's words, or is echoed twice.

use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tau_client::ConvRef;
use tau_client::engine::StreamEvent;
use tau_client::sse::EndReason;
use tau_client::store::Secrets;
use tau_client::wire::parse_reply;

use super::testing::{Setup, fixture, linked, paired, settle};

use super::server::{Hidden, ServerView};
use crate::frame::{ServerPhase, TroubleMark};
use crate::open::{Item, OpenItems};
use crate::ui::{Counts, Mark, NoticeAction};

fn open_items() -> OpenItems {
    let mut open = OpenItems::default();
    for item in [
        conversation("notes-cli", 4),
        decision("render-core", "w1:3:design#2"),
        conversation("render-core", 5),
        Item::Project("render-core".into()),
        decision("forecasts", "w2:2:pi-durable-2#1"),
    ] {
        open.open(item);
    }
    open
}

fn conversation(project: &str, id: u64) -> Item {
    Item::Conversation {
        project: project.into(),
        id,
    }
}

fn decision(project: &str, id: &str) -> Item {
    Item::Decision {
        project: project.into(),
        id: id.into(),
    }
}

fn view(s: &Setup, open: &OpenItems, hidden: &Hidden) -> ServerView {
    let mut v = ServerView::default();
    v.update(&s.link, open, hidden, s.clock_now(), 0);
    v
}

impl Setup {
    /// Every open item's labels asked for, every reply applied: what Home shows of the fixture.
    fn received(&mut self, open: &OpenItems) {
        self.link.set_focused(true);
        settle(&mut self.link);
        for item in open.iter() {
            if let Item::Conversation { project, id } = item {
                self.link.want_label(ConvRef::new(project.clone(), *id));
            }
        }
        settle(&mut self.link);
    }
}

fn ring(words: &str) -> ServerPhase {
    ServerPhase::Trouble {
        mark: TroubleMark::Ring,
        words: words.into(),
    }
}

#[test]
fn at_launch_nothing_is_claimed() {
    let s = paired(true);
    let open = open_items();
    let v = view(&s, &open, &Hidden::default());
    assert_eq!(v.phase, ring("connecting"));
    assert_eq!(v.counts.needs, None);
    assert_eq!(v.counts.failed, None);
    assert!(v.items.values().all(|f| f.mark.is_none()));
    assert_eq!(v.notice, None);
}

#[test]
fn the_fixture_shows_as_home_does() {
    let mut s = paired(false);
    let open = open_items();
    s.received(&open);
    let v = view(&s, &open, &Hidden::default());
    assert_eq!(
        v.phase,
        ServerPhase::Calm {
            device: "laptop".into(),
            host: "tau.tail4e2f.ts.net".into(),
        }
    );
    assert_eq!(
        v.counts,
        Counts {
            needs: Some(3),
            failed: Some(4),
            unknown: Some(0),
        }
    );
    let mark = |i: &Item| v.items[i].mark;
    assert_eq!(mark(&conversation("notes-cli", 4)), Some(Mark::Running));
    assert_eq!(mark(&conversation("render-core", 5)), Some(Mark::Failed));
    assert_eq!(
        mark(&decision("render-core", "w1:3:design#2")),
        Some(Mark::Decision)
    );
    assert_eq!(
        mark(&decision("forecasts", "w2:2:pi-durable-2#1")),
        Some(Mark::Decision)
    );
    assert_eq!(mark(&Item::Project("render-core".into())), None);
    assert_eq!(
        v.label(&decision("render-core", "w1:3:design#2"))
            .as_deref(),
        Some("Review the design")
    );
    assert!(
        v.label(&conversation("notes-cli", 4))
            .is_some_and(|l| l.starts_with("Port the config loader")),
        "{:?}",
        v.label(&conversation("notes-cli", 4))
    );
    assert_eq!(v.notice, None);
}

#[test]
fn the_failed_count_waits_for_every_poll_and_leaves_out_what_is_hidden() {
    let mut s = paired(false);
    let open = open_items();
    s.link.set_focused(true);
    s.link.pump();
    s.link.pump();
    let early = view(&s, &open, &Hidden::default());
    if early.counts.needs.is_some() {
        assert_eq!(early.counts.failed, None, "inbox before the polls");
    }
    s.received(&open);
    let hidden = Hidden {
        runs: HashSet::from([ConvRef::new("render-core", 5)]),
        ..Hidden::default()
    };
    assert_eq!(view(&s, &open, &hidden).counts.failed, Some(3));
}

#[test]
fn offline_after_a_minute_with_nothing_received_drops_the_state_as_of_then() {
    let mut s = paired(true);
    s.clock.advance(Duration::from_secs(61));
    s.link.pump();
    let v = view(&s, &open_items(), &Hidden::default());
    assert_eq!(v.phase, ring("offline 1 m"));
    let n = v.notice.expect("the offline band");
    assert!(n.sentence.starts_with("Offline since "), "{}", n.sentence);
    assert!(!n.sentence.contains("showing the state as of then"));
    assert!(n.sentence.ends_with("writes resume when connected"));
    assert_eq!(
        n.action,
        Some(NoticeAction {
            seq: "<leader>rc",
            word: "reconnect",
        })
    );
}

#[test]
fn offline_after_something_arrived_shows_the_state_as_of_then() {
    let mut s = paired(false);
    let open = open_items();
    s.received(&open);
    s.wire.silent.store(true, Ordering::SeqCst);
    s.wire.deliver_to_all(StreamEvent::Closed(None));
    s.link.pump();
    for _ in 0..13 {
        s.clock.advance(Duration::from_secs(5));
        s.link.pump();
    }
    let v = view(&s, &open, &Hidden::default());
    let n = v.notice.expect("the offline band");
    assert!(
        n.sentence.contains("showing the state as of then"),
        "{}",
        n.sentence
    );
    assert_eq!(v.counts.needs, Some(3), "the counts stay as last received");
}

// Offline counts can omit drafts in unopened decisions, count settled writes or answers as
// unsent inputs, double-count an edited note, or use the wrong singular verb.
#[test]
fn offline_band_counts_all_note_drafts_and_unsettled_inputs() {
    use crate::screens::decision::notes::Draft;
    for (notes, inputs, expected) in [
        (3, 1, "3 notes and 1 unsent input are safe"),
        (1, 0, "1 note is safe"),
        (0, 2, "2 unsent inputs are safe"),
        (0, 0, "writes resume when connected"),
    ] {
        let s = paired(false);
        let mut a = super::App::launch(s.link);
        super::link::settle(&mut a);
        for i in 0..notes {
            a.screens.decision.notes.seed(
                &tau_client::DecisionRef::new("render-core", format!("w1:{i}:design#1")),
                vec![Draft {
                    text: "keep me".into(),
                    range: None,
                    widened: false,
                }],
            );
        }
        s.wire.silent.store(true, Ordering::SeqCst);
        for _ in 0..inputs {
            a.link
                .write(tau_client::Write::Input {
                    conv: ConvRef::new("notes-cli", 4),
                    text: "unsent".into(),
                    mode: tau_client::wire::InputMode::FollowUp,
                })
                .unwrap();
        }
        s.wire.deliver_to_all(StreamEvent::Closed(None));
        a.link.pump();
        for _ in 0..13 {
            s.clock.advance(Duration::from_secs(5));
            a.link.pump();
        }
        super::link::refresh(&mut a);
        let words = &a.server.notice.as_ref().unwrap().sentence;
        assert!(words.contains(expected), "{words}");
        if notes == 0 && inputs == 0 {
            assert!(!words.contains("safe"));
        }
    }
}

#[test]
fn stopping_reads_server_stopped_and_keeps_the_counts() {
    let mut s = paired(false);
    let open = open_items();
    s.received(&open);
    s.wire.end_every_stream(EndReason::Stopping);
    s.link.pump();
    let v = view(&s, &open, &Hidden::default());
    assert_eq!(
        v.phase,
        ServerPhase::Trouble {
            mark: TroubleMark::Time,
            words: "server stopped · retrying".into(),
        }
    );
    assert_eq!(v.counts.needs, Some(3));
    assert_eq!(v.counts.failed, Some(4));
}

#[test]
fn a_revoked_end_shows_the_revoked_state() {
    let mut s = paired(false);
    let open = open_items();
    s.received(&open);
    s.wire.end_every_stream(EndReason::Revoked);
    s.link.pump();
    let v = view(&s, &open, &Hidden::default());
    assert_eq!(
        v.phase,
        ServerPhase::Trouble {
            mark: TroubleMark::Failed,
            words: "revoked".into(),
        }
    );
    let n = v.notice.expect("the revoked band");
    assert_eq!(n.mark, Some(TroubleMark::Failed));
    assert!(
        n.sentence
            .starts_with("This device was revoked on the server. Nothing more is sent."),
        "{}",
        n.sentence
    );
}

#[test]
fn unauthorized_on_a_call_is_refused_not_revoked() {
    let mut s = paired(false);
    let open = open_items();
    s.received(&open);
    let f = fixture();
    let reply = &f.value()["refusals"]["unauthorized (revoked)"]["reply"];
    *s.wire.refuse.lock().unwrap() = Some(parse_reply(
        401,
        Some("1"),
        &serde_json::to_vec(reply).unwrap(),
    ));
    s.link.refresh_catalogue();
    settle(&mut s.link);
    let v = view(&s, &open, &Hidden::default());
    assert_eq!(
        v.phase,
        ServerPhase::Trouble {
            mark: TroubleMark::Failed,
            words: "refused by the server".into(),
        }
    );
    let n = v.notice.expect("the refused band");
    assert_eq!(
        n.sentence,
        "The server refused this device: Send a valid device token."
    );
}

#[test]
fn an_item_the_server_lacks_keeps_its_label_and_is_gone() {
    let mut s = paired(false);
    let mut open = open_items();
    let missing = decision("render-core", "w1:9:nothing#1");
    open.open(missing.clone());
    s.received(&open);
    s.link.fetch_decision(tau_client::DecisionRef::new(
        "render-core",
        "w1:9:nothing#1",
    ));
    settle(&mut s.link);
    let mut v = ServerView::default();
    v.items.insert(
        missing.clone(),
        super::ItemFacts {
            label: Some("Something we read before".into()),
            ..Default::default()
        },
    );
    v.update(&s.link, &open, &Hidden::default(), s.clock_now(), 0);
    assert!(v.items[&missing].gone);
    assert_eq!(
        v.label(&missing).as_deref(),
        Some("Something we read before")
    );
}

#[test]
fn an_unpaired_link_is_not_connected() {
    let dir = tempfile::tempdir().unwrap();
    let s = linked(dir, Secrets::memory(), false);
    let v = view(&s, &open_items(), &Hidden::default());
    assert_eq!(v.phase, ServerPhase::NotConnected);
    assert_eq!(v.notice, None);
}

#[test]
fn an_unpaired_link_shows_connect() {
    let dir = tempfile::tempdir().unwrap();
    let s = linked(dir, Secrets::memory(), false);
    let mut app = super::App::launch(s.link);
    assert_eq!(app.stage, super::Stage::Connect);
    app.show(super::Stage::Place(crate::frame::Place::Home));
    super::link::pump(&mut app, &mut rune::Tx::default());
    assert_eq!(app.stage, super::Stage::Connect);
}

#[test]
fn a_link_event_is_echoed_once_in_the_copy_s_words() {
    let mut s = paired(false);
    s.received(&open_items());
    let wire = s.wire.clone();
    let mut app = super::App::launch(s.link);
    wire.end_every_stream(EndReason::Stopping);
    super::link::pump(&mut app, &mut rune::Tx::default());
    let said = "The server is stopping. tau reconnects when it is back.";
    assert_eq!(app.echo.as_deref(), Some(said));
    assert_eq!(app.messages.iter().filter(|m| *m == said).count(), 1);
}
