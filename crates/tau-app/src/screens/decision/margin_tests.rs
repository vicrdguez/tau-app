//! How the margin's notes could be placed wrong (`spec.md` §5.6 item 4, FA7):
//! - two notes overlap, or sit closer than 12 px;
//! - a note rises above the margin's head or above its own passage;
//! - a note pushed below its passage has no leader, or one that sits beside its passage does.

use super::margin::{Placed, place};

#[test]
fn notes_sit_beside_their_passages_and_never_overlap() {
    let placed = place(&[100.0, 110.0, 400.0], &[40.0, 30.0, 20.0], 50.0);
    assert_eq!(
        placed,
        vec![
            Placed {
                y: 100.0,
                leader: false
            },
            Placed {
                y: 152.0,
                leader: true
            },
            Placed {
                y: 400.0,
                leader: false
            },
        ]
    );
}

#[test]
fn no_note_rises_above_the_head() {
    let placed = place(&[10.0], &[20.0], 120.0);
    assert_eq!(placed[0].y, 120.0);
    assert!(placed[0].leader);
}

#[test]
fn a_small_push_draws_no_leader() {
    let placed = place(&[100.0, 145.0], &[40.0, 20.0], 0.0);
    assert_eq!(placed[1].y, 152.0);
    assert!(!placed[1].leader, "7 px is still beside its passage");
}
