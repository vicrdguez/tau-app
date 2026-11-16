//! How the decision view and its readback could fail, at the screen's seam (keys in, `Write`s and
//! what the view holds out), on the fixture's server (`spec.md` §5.6, §5.7, FA1 to FA11):
//! - `␣ad2y` typed as a burst sends (FA1);
//! - `y` counts before the pick it confirms was drawn;
//! - `⏎`, an auto-repeated `y` or no pick sends (FA2);
//! - the sent body isn't the drawn pick, the record's basis and the drafts in order;
//! - a supersede while the readback is open leaves `y` on (FA3);
//! - a `conflict` closes the float, drops the drafts, or leaves the Answer on (FA4);
//! - a lost reply clears the drafts, leaves `␣ad` on or `␣ar` off (FA5);
//! - `␣ar` after a fault sends anything but the record's own answer, or keeps the fault (FA6, FA9);
//! - `␣ad` acts away from the decision's view (FA8);
//! - the Answer is solid where `␣ad` doesn't open the readback (FA11);
//! - `␣mn`, Visual and `c` put a note on another range than the reader chose, or a selection
//!   across markup stays exact without saying so (FA7);
//! - `dd` and `u` lose a draft for good; drafts don't survive a relaunch, or leak between
//!   decisions; `]n` skips a passage.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use rune::{KeyEvent, Tx};
use serde_json::Value;
use tau_client::wire::parse_reply;
use tau_client::{DecisionRef, OutboxEntry, WriteCall};

use super::readback::{Kind, Phase, Readback};
use super::state::Cursor;
use super::*;
use crate::app::testing::{Wire, paired};
use crate::app::{input, link};
use crate::frame::SegmentKind;
use crate::keys::parse_seq;
use crate::screens::top_float;

static FIXTURE: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../../../design/fixture.json")).unwrap()
});

struct T {
    app: App,
    wire: Arc<Wire>,
    clock: tau_client::clock::ManualClock,
    _setup: crate::app::testing::Setup,
}

fn review() -> DecisionRef {
    DecisionRef::new("render-core", "w1:3:design#2")
}

fn on(project: &str, id: &str) -> T {
    let mut setup = paired(false);
    let link = std::mem::replace(&mut setup.link, link::demo());
    let mut app = App::launch(link);
    app.window = [1512.0, 982.0];
    app.link.set_focused(true);
    let (wire, clock) = (setup.wire.clone(), setup.clock.clone());
    let mut t = T {
        app,
        wire,
        clock,
        _setup: setup,
    };
    t.app.show(Stage::Item(Item::Decision {
        project: project.into(),
        id: id.into(),
    }));
    t.frames();
    t
}

/// `w1:3:design#2` with the fixture's three drafts.
fn review_with_drafts() -> T {
    let mut t = on("render-core", "w1:3:design#2");
    let drafts =
        serde_json::from_value(FIXTURE["local"]["noteDrafts"]["render-core/w1:3:design#2"].clone())
            .unwrap();
    t.app
        .screens
        .decision
        .notes
        .set(&review(), drafts, t.app.link.store());
    t
}

impl T {
    /// Frames, as the window runs them: the link pumped, every screen polled.
    fn frames(&mut self) {
        for _ in 0..20 {
            let now = self.app.now;
            crate::app::poll(&mut self.app, now, &mut Tx::default());
        }
    }

    fn at(&mut self, seconds: f64) {
        self.app.now = seconds;
        crate::app::poll(&mut self.app, seconds, &mut Tx::default());
    }

    /// Presses keys as the window does; one a field would type is typed into the note.
    fn keys(&mut self, seq: &str) {
        for key in parse_seq(seq).expect("keys") {
            self.press(key, false);
        }
    }

    fn press(&mut self, key: Key, repeat: bool) {
        let a = &mut self.app;
        if !input::press(a, key, repeat, &mut Tx::default())
            && let Some(c) = key.typed()
        {
            a.screens.decision.edit_mut().key(
                &KeyEvent {
                    key: rune::Key::Text(c.to_string()),
                    mods: rune::Mods::default(),
                },
                None,
            );
        }
    }

    fn answers(&self) -> Vec<&OutboxEntry> {
        self.app
            .link
            .outbox()
            .entries()
            .iter()
            .filter(|e| e.call == WriteCall::Answer)
            .collect()
    }

    fn readback(&mut self) -> Option<&mut Readback> {
        top_float::<Readback>(&mut self.app)
    }

    /// `␣ad`, drawn and past its arm time.
    fn armed_answer(&mut self) {
        self.keys("<leader>ad");
        self.at(1.0);
        self.at(1.5);
    }

    fn text(&self) -> String {
        reader::record(&self.app, &review()).unwrap().text.clone()
    }

    fn cursor_at(&mut self, words: &str) {
        let at = self.text().find(words).unwrap();
        self.app.screens.decision.page(&review()).cursor = Some(Cursor::Text(at));
    }
}

#[test]
fn a_burst_of_space_a_d_2_y_never_sends() {
    let mut t = review_with_drafts();
    t.keys("<leader>ad2y");
    assert!(t.answers().is_empty());
    assert_eq!(t.app.echo.as_deref(), Some("read it first"));
    assert!(t.readback().is_some(), "the readback stays");
}

#[test]
fn y_waits_for_the_pick_it_confirms_to_be_drawn() {
    let mut t = review_with_drafts();
    t.armed_answer();
    t.keys("3y");
    assert!(t.answers().is_empty(), "3 was not drawn yet");
    assert_eq!(t.app.echo.as_deref(), Some("read it first"));
    t.at(1.6);
    t.keys("y");
    assert_eq!(t.answers().len(), 1);
    assert_eq!(t.answers()[0].body["choice"], "abort");
}

#[test]
fn enter_auto_repeat_and_no_pick_never_send() {
    let mut t = review_with_drafts();
    t.armed_answer();
    t.keys("<CR>");
    t.press(Key::char('y'), true);
    assert!(t.answers().is_empty());
    t.keys("y");
    assert!(t.answers().is_empty());
    assert_eq!(t.app.echo.as_deref(), Some("pick a choice first"));
}

#[test]
fn y_sends_the_drawn_pick_the_basis_and_the_drafts_in_order_then_clears_them() {
    let mut t = review_with_drafts();
    t.armed_answer();
    t.keys("2");
    t.at(1.7);
    t.keys("y");
    let body = &t.answers()[0].body;
    assert_eq!(body["choice"], "revise");
    assert_eq!(body["basis"], 34);
    assert_eq!(
        body["annotations"],
        FIXTURE["calls"]["answer render-core/w1:3:design#2"]["request"]["annotations"]
    );
    t.frames();
    assert!(t.readback().is_none(), "the float closes on the result");
    assert!(t.app.screens.decision.notes.of(&review()).is_empty());
    assert!(segments(&t.app).is_empty(), "answered: no Answer");
}

#[test]
fn a_supersede_while_reading_turns_y_off() {
    let mut t = review_with_drafts();
    t.armed_answer();
    t.keys("2");
    let ops = FIXTURE["streams"]["inbox"]["later"][2]["frame"]["ops"]
        .as_array()
        .unwrap()
        .clone();
    t.wire
        .deliver_to_all(tau_client::engine::StreamEvent::Frame(
            tau_client::sse::Frame::Ops(ops),
        ));
    t.at(2.0);
    t.at(2.1);
    let f = t.readback().expect("the float stays, as the reply");
    assert!(
        matches!(&f.phase, Phase::Reply(r) if r.rows[0].1.starts_with("Not recorded. This request is no longer open."))
    );
    assert_eq!(t.app.scope().ctx, Ctx::Reply);
    t.keys("y");
    assert!(t.answers().is_empty());
    assert_eq!(t.app.screens.decision.notes.of(&review()).len(), 3);
}

#[test]
fn a_conflict_becomes_the_reply_and_the_view_takes_the_record() {
    let mut t = review_with_drafts();
    let reply = &FIXTURE["refusals"]["conflict (answer w1:3:design#2 after the phone answered approve)"]
        ["reply"];
    *t.wire.refuse.lock().unwrap() = Some(parse_reply(
        200,
        Some(tau_client::wire::PROTOCOL),
        &serde_json::to_vec(reply).unwrap(),
    ));
    t.armed_answer();
    t.keys("2");
    t.at(1.7);
    t.keys("y");
    *t.wire.refuse.lock().unwrap() = None;
    t.frames();
    let f = t.readback().expect("the reply");
    let Phase::Reply(r) = &f.phase else {
        panic!("not the reply")
    };
    let rows: Vec<_> = r.rows.iter().map(|(k, v)| format!("{k} {v}")).collect();
    assert_eq!(rows[1], "their choice 1 approve");
    assert_eq!(rows[2], "their notes 0 notes");
    assert_eq!(
        rows[3],
        "yours your choice revise and your 3 notes were not sent; they stay on this device"
    );
    assert!(rows[0].starts_with("reply Not recorded. phone answered approve at "));
    assert_eq!(t.app.badge(), Badge::ReadbackPlain);
    assert!(segments(&t.app).is_empty(), "no Answer");
    let item = Item::Decision {
        project: "render-core".into(),
        id: "w1:3:design#2".into(),
    };
    assert_eq!(
        t.app.server.items.get(&item).and_then(|f| f.mark),
        None,
        "no diamond in the sidebar"
    );
    assert_eq!(t.app.screens.decision.notes.of(&review()).len(), 3);
    t.keys("<Esc>");
    assert!(t.readback().is_none());
}

#[test]
fn a_lost_reply_keeps_the_drafts_turns_answer_off_and_send_again_on() {
    let mut t = review_with_drafts();
    t.armed_answer();
    t.keys("2");
    t.at(1.7);
    t.wire.silent.store(true, Ordering::SeqCst);
    t.keys("y");
    t.clock.advance(Duration::from_secs(21));
    t.frames();
    assert!(t.answers()[0].is_unknown());
    assert!(t.readback().is_none(), "the float closes");
    assert_eq!(t.app.screens.decision.notes.of(&review()).len(), 3);
    let seg: Vec<_> = segments(&t.app).iter().map(|s| (s.seq, s.kind)).collect();
    assert_eq!(
        seg,
        vec![
            ("<leader>ad", SegmentKind::Off),
            ("<leader>ar", SegmentKind::Quiet)
        ]
    );
    t.keys("<leader>ad");
    assert!(t.readback().is_none());
    t.keys("<leader>ar");
    assert!(matches!(
        t.readback().map(|f| &f.kind),
        Some(Kind::AgainUnknown(_))
    ));
}

#[test]
fn send_again_after_a_fault_sends_the_records_own_answer_and_hides_the_fault() {
    let mut t = on("render-core", "w1:6:design#1");
    t.keys("<leader>ar");
    assert!(matches!(
        t.readback().map(|f| &f.kind),
        Some(Kind::AgainAfterFault(_))
    ));
    t.at(1.0);
    t.at(1.5);
    t.keys("y");
    assert_eq!(
        t.answers()[0].body,
        serde_json::json!({"project": "render-core", "decision": "w1:6:design#1", "basis": 17,
            "choice": "approve", "annotations": []})
    );
    t.frames();
    assert!(
        t.app
            .messages
            .iter()
            .any(|m| m == "Sent again · w1.design_answer runs again in #6"),
        "{:?}",
        t.app.messages
    );
    assert!(
        !t.app.hidden.faults.is_empty(),
        "the old fault is hidden here"
    );
}

#[test]
fn answer_commands_away_from_a_decision_say_where_they_run() {
    let mut t = on("render-core", "w1:3:design#2");
    t.app.show(Stage::Place(crate::frame::Place::Home));
    t.keys("<leader>ad");
    assert!(t.app.floats.is_empty());
    assert_eq!(t.app.echo.as_deref(), Some("open it first · ⏎"));
}

#[test]
fn the_answer_resolves_only_in_the_views_normal_mode() {
    let mut t = review_with_drafts();
    assert!(
        t.app.scope().lit("<leader>ad"),
        "Normal, the stage has the keys"
    );
    assert_eq!(segments(&t.app)[0].kind, SegmentKind::Answer);
    t.keys("<C-h>");
    assert!(!t.app.scope().lit("<leader>ad"), "the sidebar has the keys");
    t.keys("<C-l><leader>mw");
    assert_eq!(t.app.badge(), Badge::Insert);
    assert!(!t.app.scope().lit("<leader>ad"), "Insert");
}

#[test]
fn which_key_greys_send_again_with_its_reason() {
    let t = review_with_drafts();
    let page = crate::screens::page(&t.app);
    assert_eq!(page.unavailable(&t.app, "auth.answer"), None);
    assert_eq!(
        page.unavailable(&t.app, "auth.again").as_deref(),
        Some("its work did not fail")
    );
}

#[test]
fn space_m_n_notes_the_block_under_the_cursor() {
    let mut t = on("render-core", "w1:3:design#2");
    t.cursor_at("A line is at most");
    t.keys("<leader>mn");
    t.keys("Say where.<Esc><Esc>");
    let drafts = t.app.screens.decision.notes.of(&review()).to_vec();
    let text = t.text();
    let r = drafts[0].range.clone().unwrap();
    let (a, b) = (
        doc::byte_of(&text, r.start as usize).unwrap(),
        doc::byte_of(&text, r.end as usize).unwrap(),
    );
    assert!(text[a..b].starts_with("- A line is at most 1 MiB"));
    assert!(text[a..b].ends_with("not enforced."));
    assert_eq!(drafts[0].text, "Say where.");
}

#[test]
fn a_visual_selection_in_one_run_is_exact_and_across_markup_widens_and_says_so() {
    let mut t = on("render-core", "w1:3:design#2");
    t.cursor_at("two exports");
    t.keys("vee");
    t.keys("c");
    let e = t
        .app
        .screens
        .decision
        .editing
        .as_ref()
        .expect("the note editor");
    let text = t.text();
    let at = text.find("two exports").unwrap();
    assert_eq!(
        e.anchor,
        state::Anchor::Range {
            range: doc::utf16_of(&text, at) as u64
                ..doc::utf16_of(&text, at + "two exports".len()) as u64,
            widened: false
        }
    );
    t.keys("<Esc><Esc>");
    t.cursor_at("Keys keep");
    t.keys("v$");
    t.keys("<leader>mn");
    let e = t.app.screens.decision.editing.as_ref().unwrap();
    assert!(
        matches!(e.anchor, state::Anchor::Range { widened: true, .. }),
        "crosses `history`"
    );
}

#[test]
fn c_with_a_motion_notes_that_range() {
    let mut t = on("render-core", "w1:3:design#2");
    t.cursor_at("two exports");
    t.keys("ciw");
    let text = t.text();
    let at = text.find("two exports").unwrap();
    let e = t
        .app
        .screens
        .decision
        .editing
        .as_ref()
        .expect("the note editor");
    assert_eq!(
        e.anchor,
        state::Anchor::Range {
            range: doc::utf16_of(&text, at) as u64..doc::utf16_of(&text, at + 3) as u64,
            widened: false
        }
    );
}

#[test]
fn dd_discards_the_draft_and_u_brings_it_back() {
    let mut t = review_with_drafts();
    t.cursor_at("A longer event");
    t.keys("dd");
    assert_eq!(t.app.screens.decision.notes.of(&review()).len(), 2);
    t.keys("u");
    assert_eq!(t.app.screens.decision.notes.of(&review()).len(), 3);
    t.keys("<C-r>");
    assert_eq!(t.app.screens.decision.notes.of(&review()).len(), 2);
}

#[test]
fn drafts_survive_a_relaunch_and_stay_with_their_decision() {
    let t = review_with_drafts();
    let store = t.app.link.store().clone();
    let mut again = State::default();
    again.notes.load(&store);
    assert_eq!(again.notes.of(&review()).len(), 3);
    assert!(
        again
            .notes
            .of(&DecisionRef::new("forecasts", "w2:2:pi-durable-2#1"))
            .is_empty()
    );
}

#[test]
fn bracket_n_moves_to_each_passage_in_text_order() {
    let mut t = review_with_drafts();
    t.keys("gg]n");
    let text = t.text();
    let Some(Cursor::Text(at)) = reader::cursor(&t.app, &review()) else {
        panic!()
    };
    assert!(text[at..].starts_with("A longer event"));
    t.keys("]n");
    let Some(Cursor::Text(at)) = reader::cursor(&t.app, &review()) else {
        panic!()
    };
    assert!(text[at..].starts_with("on the render-core") || at > text.find("500").unwrap());
    t.keys("[n");
    let Some(Cursor::Text(back)) = reader::cursor(&t.app, &review()) else {
        panic!()
    };
    assert!(text[back..].starts_with("A longer event"));
}

#[test]
fn the_cursor_moves_by_block_into_the_choices() {
    let mut t = on("forecasts", "w2:2:pi-durable-2#1");
    let d = DecisionRef::new("forecasts", "w2:2:pi-durable-2#1");
    t.keys("G");
    assert_eq!(reader::cursor(&t.app, &d), Some(Cursor::Choice(3)));
    t.keys("kk");
    assert_eq!(reader::cursor(&t.app, &d), Some(Cursor::Choice(1)));
    t.keys("<leader>ad");
    assert_eq!(
        t.readback().unwrap().pick,
        Some(1),
        "preselected from the cursor"
    );
}
