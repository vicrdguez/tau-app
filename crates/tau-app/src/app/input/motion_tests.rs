use super::*;
use crate::app::Stage;
use crate::frame::{Place, SidebarPlace};

#[test]
fn moving_only_tracks_the_users_layout_changes() {
    let before = Moving::of(&App::default());
    assert!(
        !before.moved_since(&before),
        "cursor and theme are not moving state"
    );
    for after in [
        Moving {
            floats: before.floats + 1,
            ..before.clone()
        },
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
    let opened = Moving {
        floats: before.floats + 1,
        ..before.clone()
    };
    assert!(!before.moved_since(&opened), "closing a float lands");
    let new_page = Moving {
        stage: Stage::Place(Place::Conversations),
        page: before.page + 1,
        ..before.clone()
    };
    assert!(!new_page.moved_since(&before), "a new page lands");
}

#[test]
fn cursor_and_theme_changes_are_not_motion() {
    let mut app = App::default();
    let before = Moving::of(&app);
    app.sidebar_cursor = Row::New;
    app.variant = app.variant.next();
    assert!(!Moving::of(&app).moved_since(&before));
}
