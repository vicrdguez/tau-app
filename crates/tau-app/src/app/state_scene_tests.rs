//! Scenes must carry real link truth, not only painted bands: stopped streams keep counts,
//! revoked streams disable writes, unknown input and answer remain in the outbox, and the
//! model-picker mockup names the start page's flat workflow list.
use super::*;
use crate::ui::Badge;
use tau_client::{LinkPhase, WriteCall};

fn scene(name: &str) -> App {
    let mut app = App::default();
    scenes::all()
        .iter()
        .find(|(n, _)| *n == name)
        .expect(name)
        .1(&mut app);
    app
}

#[test]
fn missing_states_have_their_link_truth_and_context() {
    let stopped = scene("server-stopped");
    assert_eq!(stopped.link.status().phase, LinkPhase::ServerStopped);
    assert!(!stopped.sidebar_visible);
    assert_eq!(stopped.server.counts.needs, Some(3));
    assert_eq!(stopped.server.counts.failed, Some(4));
    let revoked = scene("revoked");
    assert_eq!(revoked.link.status().phase, LinkPhase::Revoked);
    assert!(!revoked.link.status().writes_on);
    assert_eq!(revoked.screens.decision.notes.count(), 3);
    assert_eq!(
        revoked.server.notice.unwrap().action.unwrap().seq,
        "<leader>rp"
    );
    let unknown = scene("unknown-outcome");
    assert_eq!(unknown.badge(), Badge::Outbox);
    assert_eq!(unknown.link.outbox().unknown_count(), 2);
    for call in [WriteCall::Input, WriteCall::Answer] {
        assert!(
            unknown
                .link
                .outbox()
                .entries()
                .iter()
                .any(|e| e.call == call && e.is_unknown())
        );
    }
    let model = scene("model-picker");
    assert_eq!(model.stage, Stage::Start);
    assert_eq!(model.badge(), Badge::Command);
    assert!(!model.floats.is_empty());
}
