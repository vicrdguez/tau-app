//! How a key's change could pick the wrong motion (`visual-language.md` §7):
//! - closing a float lands at once instead of lowering its edge;
//! - a key under a float lands the float's new height instead of gliding its edge there;
//! - a page change, with or without a float closing, glides;
//! - a cursor or theme change with no float animates.
use super::*;
use crate::app::Stage;
use crate::frame::{Place, SidebarPlace};

#[test]
fn moving_tracks_the_float_and_the_users_layout_changes() {
    let before = Moving::of(&App::default());
    assert!(
        !before.moved_since(&before),
        "cursor and theme are not moving state"
    );
    let shown = Moving {
        float: true,
        ..before.clone()
    };
    for after in [
        shown.clone(),
        Moving {
            sidebar: None,
            ..before.clone()
        },
        Moving {
            sidebar: Some(SidebarPlace::Over),
            ..before.clone()
        },
        Moving {
            details: !before.details,
            ..before.clone()
        },
        Moving {
            page: before.page + 1,
            ..before.clone()
        },
    ] {
        assert!(after.moved_since(&before));
    }
    assert!(
        before.moved_since(&shown),
        "closing a float lowers its edge"
    );
    assert!(
        shown.moved_since(&shown),
        "any key under a float may change its height"
    );
    let new_page = Moving {
        stage: Stage::Place(Place::Conversations),
        page: before.page + 1,
        ..before.clone()
    };
    assert!(!new_page.moved_since(&before), "a new page lands");
    assert!(
        !new_page.moved_since(&shown),
        "a float closing with a page change lands"
    );
    let changed_frame = Moving {
        details: !before.details,
        sidebar: None,
        ..new_page
    };
    assert!(
        !changed_frame.moved_since(&before),
        "a page change lands its panels too"
    );
}

#[test]
fn cursor_and_theme_changes_are_not_motion() {
    let mut app = App::default();
    let before = Moving::of(&app);
    app.sidebar_cursor = Row::New;
    app.settings.theme = app.settings.theme.next();
    assert!(!Moving::of(&app).moved_since(&before));
}
