//! Named states for headless renders (`nix run .#snap`), on the fixture link (`design/fixture.json`
//! as the server): [`on_fixture`] opens the fixture's open items, shows a stage and lets the link
//! settle, so a scene draws what the server said; [`before_any_frame`] is the launch before the
//! first frame. Each scene is frozen there: `poll` doesn't pump its link. A scene named `…-800`
//! renders 800 px wide. The frame's scenes are here; each screen family lists its own
//! (`Screen::scenes`), and [`all`] joins them.

use std::sync::LazyLock;

use serde_json::Value;

use super::{App, Focus, ItemFacts, Stage, link};
use crate::floats::{Confirm, Float};
use crate::frame::{Notice, Place, Row, ServerPhase, TroubleMark};
use crate::open::Item;
use crate::theme::Variant;
use crate::ui::NoticeAction;

/// A named state, set up on a fresh `App`.
pub type Scene = (&'static str, fn(&mut App));

/// Every scene: the frame's, then each screen family's.
pub fn all() -> &'static [Scene] {
    static ALL: LazyLock<Vec<Scene>> = LazyLock::new(|| {
        let mut all = SCENES.to_vec();
        all.extend(crate::nav::scenes());
        all.extend_from_slice(super::state_scenes::SCENES);
        for id in crate::screens::ALL {
            all.extend(crate::screens::get(id).scenes());
        }
        all
    });
    &ALL
}

/// The frame's own scenes.
const SCENES: &[Scene] = &[
    ("home-connecting", before_any_frame),
    ("home-motion-off", |a| {
        home(a);
        a.settings.motion = false;
    }),
    ("home-connecting-800", before_any_frame),
    ("which-key-root", |a| {
        home(a);
        pending(a, "<leader>");
    }),
    ("decision", |a| {
        on_fixture(a, Stage::Item(review_the_design()));
    }),
    ("sidebar-focus", |a| {
        home(a);
        a.focus = Focus::Sidebar;
        a.sidebar_cursor = Row::Item(Item::Decision {
            project: "render-core".into(),
            id: "w1:3:design#2".into(),
        });
    }),
    ("home-light", |a| {
        home(a);
        a.settings.theme = Variant::EmberLight;
    }),
    ("sidebar-hidden", |a| {
        home(a);
        a.sidebar_visible = false;
    }),
    ("narrow-800", |a| {
        a.window[0] = 800.0;
        home(a);
    }),
    ("offline", |a| {
        home(a);
        a.screens.peek.as_of = Some(tau_client::Wall(1_794_674_460_000));
        a.server.phase = ServerPhase::Trouble {
            mark: TroubleMark::Ring,
            words: "offline 6 m".into(),
        };
        a.server.counts.unknown = Some(1);
        a.server.notice = Some(Notice {
            mark: Some(TroubleMark::Ring),
            sentence: "Offline since 16:41 · showing the state as of then · 3 notes and 1 unsent \
                       input are safe · writes resume when connected"
                .into(),
            action: Some(NoticeAction {
                seq: "<leader>rc",
                word: "reconnect",
            }),
        });
    }),
    ("narrow-sidebar-800", |a| {
        home(a);
        a.focus = Focus::Sidebar;
        a.sidebar_cursor = Row::Place(Place::Home);
    }),
    ("item", |a| {
        on_fixture(
            a,
            Stage::Item(Item::Conversation {
                project: "notes-cli".into(),
                id: 4,
            }),
        );
        a.details_visible = true;
    }),
    ("palette", |a| {
        home(a);
        a.open_float(Float::palette());
    }),
    ("settings", |a| {
        home(a);
        a.open_float(Float::settings());
    }),
    ("settings-capture", |a| {
        home(a);
        settings_capture(a, "<M-m>");
    }),
    ("settings-refused", |a| {
        home(a);
        settings_capture(a, "<M-p><CR>");
    }),
    ("cmdline", |a| {
        home(a);
        let mut f = Float::cmdline();
        if let Some(e) = f.edit_mut() {
            e.replace_text("th");
        }
        a.open_float(f);
    }),
    ("confirm", |a| {
        home(a);
        a.open_float(Float::Confirm(Confirm {
            sentence:
                "Pair over plain HTTP? Anyone on the network path can read this device's token."
                    .into(),
            yes: "pairs",
            on_yes: None,
        }));
    }),
];

/// The settings float filtered to the start page's workflow key, taking `seq` as its new key.
fn settings_capture(a: &mut App, seq: &str) {
    a.open_float(Float::settings());
    a.float_edit().replace_text("workflow");
    for key in crate::keys::parse_seq(&format!("<CR>{seq}")).expect("a key sequence") {
        super::input::press(a, key, false, &mut rune::Tx::default());
    }
}

/// Presses `seq` long enough ago for which-key to be due.
fn pending(a: &mut App, seq: &str) {
    let scope = a.scope();
    for key in crate::keys::parse_seq(seq).expect("a key sequence") {
        a.keys
            .press(a.settings.keymap, scope.mode, scope.ctx, key, false, -1.0);
    }
}

fn review_the_design() -> Item {
    Item::Decision {
        project: "render-core".into(),
        id: "w1:3:design#2".into(),
    }
}

static FIXTURE: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../../design/fixture.json"))
        .expect("design/fixture.json is JSON")
});

/// The fixture's open items (`local.openItems`), in their order.
fn open_items() -> Vec<Item> {
    let groups = FIXTURE["local"]["openItems"]["groups"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for g in groups {
        let project = g["project"].as_str().unwrap_or_default().to_string();
        for i in g["items"].as_array().cloned().unwrap_or_default() {
            out.push(match i["kind"].as_str() {
                Some("conversation") => Item::Conversation {
                    project: project.clone(),
                    id: i["id"].as_u64().unwrap_or(0),
                },
                Some("decision") => Item::Decision {
                    project: project.clone(),
                    id: i["id"].as_str().unwrap_or_default().to_string(),
                },
                _ => Item::Project(project.clone()),
            });
        }
    }
    out
}

/// The fixture's open items, opened in their order.
fn open(a: &mut App) {
    for item in open_items() {
        a.open.open(item);
    }
}

/// A scene on the fixture link: the fixture's open items opened, `stage` shown, and every reply
/// and first frame applied, as after the first seconds of a launch with the window focused.
pub fn on_fixture(a: &mut App, stage: Stage) {
    a.live = false;
    open(a);
    a.show(stage);
    link::settle(a);
}

/// Home on the fixture link.
pub fn home(a: &mut App) {
    on_fixture(a, Stage::Place(Place::Home));
    crate::screens::home::prepare_scene(a);
}

/// At launch, before the server has said anything (`spec.md` §4.8): the link has sent its first
/// call and applied nothing. The open items' labels are the ones this device kept: a found label
/// holds for good (`client.md` §5.1).
pub fn before_any_frame(a: &mut App) {
    a.live = false;
    open(a);
    a.show(Stage::Place(Place::Home));
    for item in open_items() {
        a.server.items.insert(
            item.clone(),
            ItemFacts {
                label: kept_label(&item),
                ..ItemFacts::default()
            },
        );
    }
    link::refresh(a);
}

/// A label this device kept from an earlier launch: a conversation's first input (`local`), a
/// decision's title.
fn kept_label(item: &Item) -> Option<String> {
    match item {
        Item::Conversation { project, id } => FIXTURE["local"]["firstInput"][project]
            [id.to_string()]
        .as_str()
        .map(tau_client::shapes::label),
        Item::Decision { project, id } => {
            FIXTURE["calls"]["decision"][format!("{project}/{id}")]["result"]["decision"]["title"]
                .as_str()
                .map(str::to_string)
        }
        Item::Project(_) => None,
    }
}

#[cfg(test)]
mod tests {
    //! How the scenes could fail: one panics while it sets up; the launch scene claims calm or a
    //! count; a fixture scene draws less than the fixture's server said.

    use super::*;
    use crate::frame::ServerPhase;

    #[test]
    fn every_scene_sets_up() {
        let mut names = std::collections::HashSet::new();
        for (name, setup) in all() {
            assert!(names.insert(*name), "two scenes are named {name}");
            let mut a = App::default();
            setup(&mut a);
            assert!(!a.live, "{name} is frozen");
        }
    }

    #[test]
    fn before_any_frame_nothing_is_received_but_labels_are_kept() {
        let mut a = App::default();
        before_any_frame(&mut a);
        assert_eq!(a.server.phase, ServerPhase::connecting());
        assert_eq!(a.server.counts.needs, None);
        assert!(a.server.items.values().all(|f| f.mark.is_none()));
        assert!(
            a.server
                .items
                .values()
                .filter(|f| f.label.is_some())
                .count()
                >= 4
        );
    }

    #[test]
    fn a_fixture_scene_shows_what_the_server_said() {
        let mut a = App::default();
        home(&mut a);
        assert!(matches!(a.server.phase, ServerPhase::Calm { .. }));
        assert_eq!(
            (a.server.counts.needs, a.server.counts.failed),
            (Some(3), Some(4))
        );
    }
}
