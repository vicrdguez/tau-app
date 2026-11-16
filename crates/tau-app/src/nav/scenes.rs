//! Scenes for the keys' own floats: `:jumps` and the cheat sheet (`␣hk`).

use rune::Tx;

use crate::app::scenes::{Scene, home, on_fixture};
use crate::app::{App, Stage, commands, input};
use crate::keys::parse_seq;
use crate::open::Item;

fn press(a: &mut App, seq: &str) {
    for key in parse_seq(seq).expect("keys") {
        input::press(a, key, false, &mut Tx::default());
    }
}

pub fn all() -> Vec<Scene> {
    vec![
        ("keys-sheet", |a| {
            home(a);
            press(a, "<leader>hk");
        }),
        ("open-url", |a| {
            on_fixture(
                a,
                Stage::Item(Item::Decision {
                    project: "render-core".into(),
                    id: "w1:3:design#2".into(),
                }),
            );
            crate::floats::open_url::confirm(
                a,
                Some(
                    "https://github.com/badlogic/pi-mono/blob/main/packages/coding-agent/docs/\
                     extensions.md#registering-commands-and-their-keyboard-shortcuts"
                        .into(),
                ),
            );
        }),
        ("jumps", |a| {
            home(a);
            press(a, "<leader>oc<leader>ot<leader>oh");
            commands::run_id(a, "open.jumps", &mut Tx::default());
        }),
    ]
}
