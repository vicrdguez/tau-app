use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tau_client::clock::{Clock, ManualClock, Mono, Now, Wall};
use tau_client::fixture::FixtureTransport;
use tau_client::model::{Known, Label};
use tau_client::shapes::label;
use tau_client::store::{ClientConfig, Secrets, Store};
use tau_client::wire::InputMode;
use tau_client::{ConvRef, Fixture, Link, LinkConfig, LinkPhase, Project, Slot, Write, WriteState};

/// Pumps until nothing changes, as a UI does frame after frame.
fn settle(link: &mut Link) {
    for _ in 0..20 {
        if !link.pump() {
            return;
        }
    }
    panic!("the link never settled");
}

/// A link paired with the fixture's server, its files in a temp dir, on a clock that moves when told.
fn paired(dir: &std::path::Path) -> (Link, ManualClock) {
    let fixture = Fixture::design();
    let store = Store::at(dir.to_path_buf());
    let config = ClientConfig {
        server: Some(fixture.url()),
        device: Some(fixture.device()),
    };
    store.save_client(&config).unwrap();
    let secrets = Secrets::memory();
    secrets.save(&fixture.url(), &fixture.token()).unwrap();
    let clock = ManualClock::new(Now {
        mono: Mono(1_000_000),
        wall: Wall(fixture.value()["now"].as_i64().unwrap()),
    });
    let config = LinkConfig {
        dir: Some(dir.to_path_buf()),
        secrets,
        clock: Arc::new(clock.clone()),
    };
    (
        Link::with_transport(config, Arc::new(FixtureTransport::new(fixture))),
        clock,
    )
}

#[test]
fn server_messages_wait_for_pump() {
    let dir = tempfile::tempdir().unwrap();
    let (mut link, _) = paired(dir.path());
    assert_eq!(link.server().projects, Known::NotYet);
    link.pump();
    assert_eq!(
        link.server().projects,
        Known::NotYet,
        "replies that came during a pump wait for the next"
    );
    assert_eq!(link.server().inbox, Known::NotYet);
    link.pump();
    assert!(matches!(link.server().projects, Known::Have { .. }));
    settle(&mut link);
    assert_eq!(link.server().open_decisions(), Some(3));
    assert_eq!(link.status().phase, LinkPhase::Live);
}

#[test]
fn the_waker_is_called() {
    let dir = tempfile::tempdir().unwrap();
    let (mut link, _) = paired(dir.path());
    let woken = Arc::new(AtomicUsize::new(0));
    let count = woken.clone();
    link.set_waker(move || {
        count.fetch_add(1, Ordering::SeqCst);
    });
    link.pump();
    assert!(woken.load(Ordering::SeqCst) > 0);
}

#[test]
fn the_timer_wakes_at_the_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let (mut link, clock) = paired(dir.path());
    settle(&mut link);
    let due = link.next_deadline().expect("streams fall silent in 45 s");
    assert!(due <= Duration::from_secs(45));
    let (tx, woken) = mpsc::channel();
    link.set_waker(move || {
        let _ = tx.send(());
    });
    while woken.try_recv().is_ok() {}
    clock.advance(due + Duration::from_secs(1));
    woken
        .recv_timeout(Duration::from_secs(2))
        .expect("the timer wakes the UI");
    assert!(link.pump());
}

#[test]
fn the_fixture_link_shows_the_design_data() {
    let mut link = Link::fixture(Fixture::design());
    link.set_focused(true);
    settle(&mut link);
    let server = link.server();
    assert_eq!(link.status().phase, LinkPhase::Live);
    assert_eq!(server.open_decisions(), Some(3));
    assert_eq!(server.projects.value().unwrap().len(), 3);
    let rows = server.conversations[&Project::from("render-core")]
        .value()
        .unwrap();
    assert_eq!(
        rows.iter().map(|r| r.conversation.0).collect::<Vec<_>>(),
        [8, 7, 6, 5, 4, 3, 2]
    );
    assert!(matches!(
        server.faults[&Project::from("render-core")],
        Known::Have { .. }
    ));
    let nc4 = ConvRef::new("notes-cli", 4);
    link.show(Slot::Stage, Some(nc4.clone()));
    settle(&mut link);
    assert_eq!(link.server().transcripts[&nc4].entries.len(), 13);
    let rc5 = ConvRef::new("render-core", 5);
    link.want_label(rc5.clone());
    settle(&mut link);
    let first = Fixture::design().value()["local"]["firstInput"]["render-core"]["5"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(link.server().label(&rc5), Label::Found(label(&first)));
}

#[test]
fn writes_through_the_fixture() {
    let mut link = Link::fixture(Fixture::design());
    settle(&mut link);
    let id = link
        .write(Write::Input {
            conv: ConvRef::new("notes-cli", 4),
            text: "go".into(),
            mode: InputMode::Steer,
        })
        .unwrap();
    assert_eq!(link.outbox().get(id).unwrap().state, WriteState::Sending);
    settle(&mut link);
    assert!(matches!(
        link.outbox().get(id).unwrap().state,
        WriteState::Done { .. }
    ));
    let start = Write::Start {
        project: Project::from("render-core"),
        with: tau_client::StartWith::Workflow("w1".into()),
        text: "Design the retention flag.".into(),
    };
    link.write(start).unwrap();
    settle(&mut link);
    let started = link.take_events().into_iter().find_map(|e| match e {
        tau_client::LinkEvent::Started { conv, .. } => Some(conv),
        _ => None,
    });
    assert_eq!(
        started.map(|c| c.project),
        Some(Project::from("render-core"))
    );
}

#[test]
fn the_system_clock_moves_on() {
    let clock = tau_client::clock::SystemClock;
    let a = clock.now();
    std::thread::sleep(Duration::from_millis(20));
    let b = clock.now();
    assert!(b.mono.since(a.mono) >= Duration::from_millis(15));
    assert!(b.wall.0 - a.wall.0 >= 15);
}

/// A decision watched before the handshake answers (a window opened on it at launch) is fetched
/// once the link is ready, not left unfetched until something else asks.
#[test]
fn a_decision_watched_before_the_handshake_is_fetched_once_ready() {
    let dir = tempfile::tempdir().unwrap();
    let (mut link, _) = paired(dir.path());
    let d = tau_client::DecisionRef::new("render-core", "w1:3:design#2");
    link.watch_decision(Some(d.clone()));
    settle(&mut link);
    assert!(
        matches!(link.server().decisions.get(&d), Some(Known::Have { .. })),
        "the watched decision was fetched"
    );
}
