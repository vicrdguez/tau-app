//! How the conversation view could fail at its seam, the keys and the link (`spec.md` §5.5, §4.9,
//! `grammar.md` §7, §10, failure-catalogue FC4):
//! - a running conversation lacks `␣cx stop`, a failed one `␣cc continue`, an idle one prints a
//!   segment, or a segment stays on while the view's stream is down;
//! - `␣cx` sends while no run is drawn, or stays on after sending until the drawn run is gone or
//!   a new one shows; `␣cc` sends while nothing failed;
//! - `⌘⏎` sends anything but the drawn mode (steer while running, plain while idle), `M-⏎` anything
//!   but a follow-up, or either sends empty or white-space text;
//! - bare `⏎` sends, from Insert or from the composer's Normal mode;
//! - the composer keeps the text it sent, or loses a draft when `Esc Esc` leaves it;
//! - the cursor opens anywhere but the last stop with follow on, an upward motion leaves follow
//!   on, `G` leaves it off, `]t [t ]m [m` stop anywhere but tool rows and message heads;
//! - `za` or Enter leaves a fold unchanged, moves the cursor away, or `zR` or `zM` miss a row;
//! - Enter changes a non-foldable stop, or Enter or `:open` toggles while composing;
//! - `gd` in an owned conversation opens anything but its owner;
//! - `␣cm` lists other than the server's models with the current one marked, `⏎` sets another
//!   model than the one under the cursor; `␣ct` lists other than the seven levels or drops the
//!   model;
//! - `K` on a record shows anything but its `data`;
//! - the composer's caption says other than what `⌘⏎` does in the drawn state.

use rune::{KeyEvent, Tx};
use serde_json::Value;
use tau_client::shapes::ConversationState;
use tau_client::{ConvRef, TaskId};

use super::*;
use crate::app::scenes::on_fixture;
use crate::app::{Stage, input};
use crate::frame::SegmentKind;
use crate::keys::parse_seq;
use crate::open::Item as OpenItem;
use crate::ui::Badge;

fn conv(project: &str, id: u64) -> OpenItem {
    OpenItem::Conversation {
        project: project.into(),
        id,
    }
}

/// The app showing a fixture conversation, its stream live.
fn at(project: &str, id: u64) -> App {
    let mut a = App::default();
    on_fixture(&mut a, Stage::Item(conv(project, id)));
    a
}

/// Presses keys as the window does; a key the handler passes on is typed into the focused field.
fn keys(a: &mut App, seq: &str) {
    for key in parse_seq(seq).expect("keys") {
        let mut tx = Tx::default();
        if !input::press(a, key, false, &mut tx)
            && let Some(c) = key.typed()
        {
            let edit = if a.floats.is_empty() {
                composer_edit(a)
            } else {
                a.float_edit()
            };
            edit.key(
                &KeyEvent {
                    key: rune::Key::Text(c.to_string()),
                    mods: rune::Mods::default(),
                },
                None,
            );
        }
    }
}

fn typed(a: &mut App, text: &str) {
    composer_edit(a).replace_text(text);
}

fn segments_of(a: &App) -> Vec<(&'static str, String, SegmentKind)> {
    Conversation
        .segments(a)
        .into_iter()
        .map(|s| (s.seq, s.word, s.kind))
        .collect()
}

/// The writes this device made, as their bodies.
fn writes(a: &App) -> Vec<(tau_client::WriteCall, Value)> {
    a.link
        .outbox()
        .entries()
        .iter()
        .map(|e| (e.call, e.body.clone()))
        .collect()
}

fn inputs(a: &App) -> Vec<Value> {
    writes(a)
        .into_iter()
        .filter(|(c, _)| *c == tau_client::WriteCall::Input)
        .map(|(_, b)| b)
        .collect()
}

#[test]
fn a_running_conversation_offers_stop_a_failed_one_continue_and_an_idle_one_nothing() {
    assert_eq!(
        segments_of(&at("notes-cli", 4)),
        [("<leader>cx", "stop".to_string(), SegmentKind::Quiet)]
    );
    assert_eq!(
        segments_of(&at("render-core", 5)),
        [("<leader>cc", "continue".to_string(), SegmentKind::Quiet)]
    );
    assert!(segments_of(&at("render-core", 8)).is_empty());
}

#[test]
fn stop_is_on_only_while_the_drawn_run_is_one_not_yet_stopped() {
    let running = ConversationState::Running;
    let on = |sent| header(&running, Some(TaskId(8)), true, sent)[0].kind;
    assert_eq!(on(None), SegmentKind::Quiet);
    assert_eq!(on(Some(TaskId(8))), SegmentKind::Off, "sent for this run");
    assert_eq!(on(Some(TaskId(7))), SegmentKind::Quiet, "a new run shows");
    assert_eq!(
        header(&running, Some(TaskId(8)), false, None)[0].kind,
        SegmentKind::Off,
        "the stream is down"
    );
    let failed = ConversationState::Failed {
        reason: "model_error".into(),
    };
    assert_eq!(
        header(&failed, None, false, None)[0],
        crate::frame::Segment {
            seq: "<leader>cc",
            word: "continue".into(),
            kind: SegmentKind::Off
        }
    );
}

#[test]
fn space_cx_stops_the_drawn_run_once_and_then_reads_off() {
    let mut a = at("notes-cli", 4);
    keys(&mut a, "<leader>cx");
    let stops: Vec<_> = writes(&a)
        .into_iter()
        .filter(|(c, _)| *c == tau_client::WriteCall::Stop)
        .collect();
    assert_eq!(stops.len(), 1);
    assert_eq!(stops[0].1["conversation"], 4);
    assert_eq!(segments_of(&a)[0].2, SegmentKind::Off);
    keys(&mut a, "<leader>cx");
    assert_eq!(
        writes(&a)
            .iter()
            .filter(|(c, _)| *c == tau_client::WriteCall::Stop)
            .count(),
        1,
        "a second press sends nothing"
    );
}

#[test]
fn stop_and_continue_send_nothing_where_the_drawn_state_has_no_run_or_failure() {
    let mut idle = at("render-core", 8);
    keys(&mut idle, "<leader>cx");
    keys(&mut idle, "<leader>cc");
    assert!(writes(&idle).is_empty());
    let mut running = at("notes-cli", 4);
    keys(&mut running, "<leader>cc");
    assert!(writes(&running).is_empty());
    let mut failed = at("render-core", 5);
    keys(&mut failed, "<leader>cc");
    assert_eq!(writes(&failed)[0].0, tau_client::WriteCall::Continue);
}

#[test]
fn i_writes_in_the_composer_in_insert() {
    let mut a = at("render-core", 8);
    assert_eq!(a.badge(), Badge::Normal);
    keys(&mut a, "i");
    assert_eq!(a.badge(), Badge::Insert);
    assert_eq!(a.scope().ctx, crate::keys::Ctx::Composer);
}

#[test]
fn cmd_enter_sends_the_drawn_mode_steer_while_running_and_plain_while_idle() {
    let mut running = at("notes-cli", 4);
    keys(&mut running, "i");
    typed(&mut running, "Use snapshots.");
    keys(&mut running, "<D-CR>");
    let sent = inputs(&running);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0]["text"], "Use snapshots.");
    assert_eq!(sent[0]["whenBusy"], "steer");
    assert_eq!(
        composer_text(&running),
        "",
        "the composer empties once sent"
    );

    let mut idle = at("render-core", 8);
    keys(&mut idle, "i");
    typed(&mut idle, "Again.");
    keys(&mut idle, "<D-CR>");
    let sent = inputs(&idle);
    assert_eq!(sent.len(), 1);
    assert!(sent[0].get("whenBusy").is_none(), "{:?}", sent[0]);
}

#[test]
fn option_enter_queues_a_follow_up() {
    let mut a = at("notes-cli", 4);
    keys(&mut a, "i");
    typed(&mut a, "After this.");
    keys(&mut a, "<M-CR>");
    assert_eq!(inputs(&a)[0]["whenBusy"], "followUp");
}

#[test]
fn cmd_enter_from_the_composers_normal_mode_sends_too() {
    let mut a = at("render-core", 8);
    keys(&mut a, "i");
    typed(&mut a, "From Normal.");
    keys(&mut a, "<Esc>");
    assert_eq!(a.badge(), Badge::Normal);
    keys(&mut a, "<D-CR>");
    assert_eq!(inputs(&a).len(), 1);
}

#[test]
fn bare_enter_never_sends() {
    let mut a = at("render-core", 8);
    keys(&mut a, "i");
    typed(&mut a, "line one");
    keys(&mut a, "<CR>");
    assert!(inputs(&a).is_empty());
    assert!(composer_text(&a).contains('\n'), "a new line in Insert");
    keys(&mut a, "<Esc><CR>");
    assert!(inputs(&a).is_empty(), "nothing in Normal either");
}

#[test]
fn empty_or_white_space_text_sends_nothing() {
    let mut a = at("render-core", 8);
    keys(&mut a, "i");
    typed(&mut a, "  \n ");
    keys(&mut a, "<D-CR><M-CR>");
    assert!(inputs(&a).is_empty());
}

#[test]
fn esc_esc_returns_to_the_conversation_keeping_the_draft() {
    let mut a = at("render-core", 8);
    keys(&mut a, "i");
    typed(&mut a, "half a thought");
    keys(&mut a, "<Esc><Esc>");
    assert_eq!(a.scope().ctx, crate::keys::Ctx::Conversation);
    assert_eq!(composer_text(&a), "half a thought");
    keys(&mut a, "a");
    assert_eq!(a.badge(), Badge::Insert);
    assert_eq!(composer_text(&a), "half a thought");
}

fn cursor(a: &App) -> Option<Stop> {
    let s = shown(a).expect("a conversation");
    current_stop(a, &s)
}

fn last_stop(a: &App) -> Stop {
    let s = shown(a).expect("a conversation");
    stops(&items(a, &s)).last().cloned().expect("a stop")
}

#[test]
fn the_cursor_opens_on_the_last_stop_with_follow_on_and_k_pauses_it() {
    let mut a = at("notes-cli", 4);
    assert!(following(&a));
    assert_eq!(cursor(&a), Some(last_stop(&a)));
    keys(&mut a, "k");
    assert!(!following(&a), "an upward motion pauses follow");
    assert_ne!(cursor(&a), Some(last_stop(&a)));
    keys(&mut a, "G");
    assert!(following(&a));
}

#[test]
fn tool_and_message_motions_stop_on_their_objects() {
    let mut a = at("notes-cli", 4);
    keys(&mut a, "[t");
    assert!(cursor(&a).is_some_and(|s| s.is_tool()), "{:?}", cursor(&a));
    let before = cursor(&a);
    keys(&mut a, "[t");
    assert!(cursor(&a).is_some_and(|s| s.is_tool()));
    assert_ne!(cursor(&a), before);
    keys(&mut a, "[m");
    assert!(cursor(&a).is_some_and(|s| s.is_head()), "{:?}", cursor(&a));
    keys(&mut a, "]t");
    assert!(cursor(&a).is_some_and(|s| s.is_tool()));
}

#[test]
fn za_toggles_the_row_under_the_cursor_and_zr_zm_every_row() {
    let mut a = at("notes-cli", 4);
    keys(&mut a, "[t");
    let fold = cursor(&a).expect("a stop");
    let open = |a: &App, stop: &Stop| {
        let s = shown(a).unwrap();
        foldables(&items(a, &s))
            .into_iter()
            .find(|(f, _)| f == stop)
            .map(|(_, o)| o)
    };
    let was = open(&a, &fold).expect("foldable");
    keys(&mut a, "za");
    assert_eq!(open(&a, &fold), Some(!was));
    keys(&mut a, "zR");
    let s = shown(&a).unwrap();
    assert!(foldables(&items(&a, &s)).iter().all(|(_, o)| *o));
    keys(&mut a, "zM");
    let s = shown(&a).unwrap();
    assert!(foldables(&items(&a, &s)).iter().all(|(_, o)| !*o));
}

#[test]
fn enter_toggles_each_foldable_and_keeps_the_cursor_on_it() {
    for (project, id) in [("notes-cli", 4), ("render-core", 3)] {
        let mut a = at(project, id);
        keys(&mut a, "zR");
        let rows = foldables(&items(&a, &shown(&a).unwrap()));
        assert!(!rows.is_empty());
        for (stop, _) in rows {
            let conv = shown_conv(&a).unwrap();
            a.screens.conversation.conv_mut(&conv).cursor = Some(stop.clone());
            for expected in [false, true] {
                keys(&mut a, "<CR>");
                assert_eq!(cursor(&a), Some(stop.clone()));
                assert_eq!(
                    foldables(&items(&a, &shown(&a).unwrap()))
                        .into_iter()
                        .find(|(s, _)| *s == stop),
                    Some((stop.clone(), expected))
                );
            }
        }
    }
}

#[test]
fn enter_on_text_and_enter_or_open_while_composing_leave_folds_alone() {
    let mut a = at("render-core", 3);
    keys(&mut a, "[t[t");
    let before = items(&a, &shown(&a).unwrap());
    keys(&mut a, "i<CR><Esc><CR>:open<CR>");
    assert_eq!(items(&a, &shown(&a).unwrap()), before);
    assert!(inputs(&a).is_empty());
    keys(&mut a, "<Esc>gg");
    let stop = cursor(&a);
    let before = items(&a, &shown(&a).unwrap());
    keys(&mut a, "<CR>");
    assert_eq!(cursor(&a), stop);
    assert_eq!(items(&a, &shown(&a).unwrap()), before);
}

#[test]
fn gd_in_an_owned_conversation_opens_its_owner() {
    let mut a = at("render-core", 8);
    keys(&mut a, "gd");
    assert_eq!(a.stage, Stage::Item(conv("render-core", 3)));
}

#[test]
fn space_cm_lists_the_servers_models_with_the_current_one_and_sets_the_chosen_one() {
    let mut a = at("render-core", 8);
    keys(&mut a, "<leader>cm");
    assert_eq!(a.badge(), Badge::Command);
    let picker = picker_rows(&a);
    assert!(picker.len() >= 2, "{picker:?}");
    assert_eq!(
        picker.iter().filter(|r| r.current).count(),
        1,
        "the current model is marked"
    );
    let current = picker.iter().position(|r| r.current).unwrap();
    let other = if current == 0 { 1 } else { 0 };
    let chosen = picker[other].value.clone();
    while picker_cursor(&a) != other {
        let step = if other > picker_cursor(&a) {
            "<C-j>"
        } else {
            "<C-k>"
        };
        keys(&mut a, step);
    }
    keys(&mut a, "<CR>");
    assert!(a.floats.is_empty());
    let set: Vec<_> = writes(&a)
        .into_iter()
        .filter(|(c, _)| *c == tau_client::WriteCall::SetModel)
        .collect();
    assert_eq!(set.len(), 1);
    assert_eq!(set[0].1["model"], Value::String(chosen));
}

#[test]
fn space_ct_lists_the_seven_levels_and_keeps_the_model() {
    let mut a = at("render-core", 8);
    keys(&mut a, "<leader>ct");
    let levels: Vec<String> = picker_rows(&a).into_iter().map(|r| r.label).collect();
    assert_eq!(
        levels,
        ["off", "minimal", "low", "medium", "high", "xhigh", "max"]
    );
    keys(&mut a, "<C-j><C-j><CR>");
    let set: Vec<_> = writes(&a)
        .into_iter()
        .filter(|(c, _)| *c == tau_client::WriteCall::SetModel)
        .collect();
    assert_eq!(set[0].1["thinkingLevel"], "low");
    assert_eq!(set[0].1["model"], "anthropic/claude-sonnet-4-5");
}

#[test]
fn esc_closes_a_picker_without_a_write() {
    let mut a = at("render-core", 8);
    keys(&mut a, "<leader>cm<Esc>");
    assert!(a.floats.is_empty());
    assert!(writes(&a).is_empty());
}

#[test]
fn k_on_a_record_shows_its_data_as_json() {
    let mut a = at("render-core", 3);
    keys(&mut a, "[m");
    while !matches!(cursor(&a), Some(Stop::Head(_)))
        || !record_heads(&a).contains(cursor(&a).as_ref().unwrap())
    {
        keys(&mut a, "[m");
    }
    keys(&mut a, "K");
    let shown = record_float_text(&a).expect("the record's data");
    assert!(shown.contains("\"text\""), "{shown}");
}

fn record_heads(a: &App) -> Vec<Stop> {
    let s = shown(a).unwrap();
    items(a, &s)
        .iter()
        .filter_map(|i| match i {
            Item::Record { head, .. } => Some(Stop::Head(*head)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_caption_says_what_cmd_enter_does_in_the_drawn_state() {
    let failed = ConversationState::Failed {
        reason: "model_error".into(),
    };
    assert_eq!(
        caption(&ConversationState::Idle, "claude-sonnet-4-5"),
        ("send", "starts a run on claude-sonnet-4-5".to_string())
    );
    assert_eq!(
        caption(&ConversationState::Running, "claude-sonnet-4-5"),
        (
            "steer",
            "joins this run after the current tool round".to_string()
        )
    );
    assert_eq!(
        caption(&failed, "claude-sonnet-4-5"),
        ("send", "starts a new run · the last run failed".to_string())
    );
}

#[test]
fn space_tf_toggles_follow() {
    let mut a = at("notes-cli", 4);
    keys(&mut a, "<leader>tf");
    assert!(!following(&a));
    keys(&mut a, "<leader>tf");
    assert!(following(&a));
}

#[test]
fn the_view_names_the_conversation_it_shows() {
    let a = at("render-core", 5);
    let s = shown(&a).expect("shown");
    assert_eq!(s.conv, ConvRef::new("render-core", 5));
    assert!(s.up, "the fixture's open stream is live");
}

/// How the view could fail while its stream is down (`spec.md` §4.9): a write segment stays on,
/// the facts line claims a current state, a running count keeps counting, or `␣cx` still sends.
#[test]
fn while_the_open_stream_is_down_writes_read_off_and_the_count_stops() {
    use crate::app::testing::paired;
    use tau_client::sse::EndReason;
    let crate::app::testing::Setup {
        link, wire, clock, ..
    } = paired(false);
    let mut a = App::launch(link);
    a.screens.conversation.clock = Some(tau_client::clock::Clock::now(&clock).wall);
    on_fixture(&mut a, Stage::Item(conv("notes-cli", 4)));
    // Scenes freeze the link after setup; this test watches the stream end, so it pumps.
    a.live = true;
    assert!(shown(&a).unwrap().up);
    assert!(running_count(&a).is_some(), "a live run counts");
    wire.end_every_stream(EndReason::Failed);
    crate::app::link::pump(&mut a, &mut Tx::default());
    let s = shown(&a).unwrap();
    assert!(!s.up);
    assert!(s.at.is_some(), "the facts line says as of when");
    assert_eq!(segments_of(&a)[0].2, SegmentKind::Off);
    assert_eq!(running_count(&a), None, "the count stops");
    keys(&mut a, "<leader>cx");
    assert!(writes(&a).is_empty());
}

#[test]
fn moving_tracks_folds_and_unsent_inputs_but_not_the_cursor() {
    let mut a = at("notes-cli", 4);
    let before = Conversation.moving(&a);
    keys(&mut a, "[t");
    assert_eq!(Conversation.moving(&a), before);
    keys(&mut a, "za");
    let folded = Conversation.moving(&a);
    assert_ne!(folded, before);
    keys(&mut a, "i");
    typed(&mut a, "Use snapshots.");
    assert_eq!(Conversation.moving(&a), folded, "typing lands");
    keys(&mut a, "<D-CR>");
    assert_ne!(Conversation.moving(&a), folded);
}

#[test]
fn moving_ignores_a_server_entry() {
    use crate::app::testing::{paired, settle};
    use tau_client::engine::StreamEvent;
    use tau_client::sse::Frame;
    let setup = paired(false);
    let mut a = App::launch(setup.link);
    on_fixture(&mut a, Stage::Item(conv("notes-cli", 4)));
    let before = Conversation.moving(&a);
    let count = shown(&a).unwrap().held.unwrap().entries.len();
    let fixture = tau_client::Fixture::design();
    let ops = fixture.value()["streams"]["open notes-cli #4"]["later"][2]["ops"]
        .as_array()
        .unwrap()
        .clone();
    setup
        .wire
        .deliver_to_all(StreamEvent::Frame(Frame::Ops(ops)));
    settle(&mut a.link);
    assert!(shown(&a).unwrap().held.unwrap().entries.len() > count);
    assert_eq!(Conversation.moving(&a), before);
}

#[test]
fn a_send_refused_before_leaving_keeps_the_draft_and_does_not_move() {
    let mut a = at("render-core", 8);
    keys(&mut a, "i");
    typed(&mut a, "Keep this draft.");
    a.link.forget_server();
    let before = Conversation.moving(&a);
    keys(&mut a, "<D-CR>");
    assert_eq!(composer_text(&a), "Keep this draft.");
    assert!(inputs(&a).is_empty());
    assert_eq!(Conversation.moving(&a), before);
    assert!(a.echo.as_deref().unwrap().contains("not connected"));
}

#[test]
fn a_picker_opens_on_the_current_choice() {
    let mut a = at("render-core", 8);
    keys(&mut a, "<leader>cm");
    let rows = picker_rows(&a);
    assert!(rows[picker_cursor(&a)].current, "{rows:?}");
}
