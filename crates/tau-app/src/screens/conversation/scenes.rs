//! The conversation's scenes (`tools/proto/states.md` 15–21 and 47), each named after its mockup,
//! on the design's fixture as the server. The fixture link delivers each stream's first frame
//! only, so a state that plays later frames (`later[0]`, `later[1]`) or reads the `hostile`
//! block gets a fixture with those folded in; the wall clock is pinned to the fixture's `now`.

use serde_json::{Value, json};
use tau_client::{ConvRef, Fixture, Link, Wall};

use super::floats::{Pick, Picker};
use super::state::{Reveal, shown};
use super::transcript::{Item, Stop};
use crate::app::scenes::{Scene, on_fixture};
use crate::app::{App, Stage, link};
use crate::open::Item as OpenItem;

pub fn all() -> Vec<Scene> {
    vec![
        ("conversation-streaming", |a| streaming(a)),
        ("conversation-steer-insert", |a| {
            streaming(a);
            compose(a, "Use node:test's snapshot support for the TOML fixtures.");
        }),
        ("conversation-failed", |a| open(a, "render-core", 5)),
        ("conversation-send-refused", |a| {
            open(a, "render-core", 8);
            compose(a, "Keep this draft.");
            a.link.forget_server();
        }),
        ("conversation-joined", |a| {
            a.link = played(&[("open render-core #7", 1)]);
            open(a, "render-core", 7);
            compose(a, "");
        }),
        ("conversation-records", records),
        ("conversation-owned", |a| open(a, "render-core", 8)),
        ("conversation-hostile", hostile),
        ("conversation-model-picker", |a| {
            open(a, "render-core", 5);
            Picker::open(a, Pick::Model);
        }),
        ("conversation-thinking-picker", |a| {
            open(a, "render-core", 5);
            Picker::open(a, Pick::Thinking);
        }),
        ("conversation-streaming-800", |a| streaming(a)),
    ]
}

fn conv(project: &str, id: u64) -> OpenItem {
    OpenItem::Conversation {
        project: project.into(),
        id,
    }
}

/// The fixture's open items, then the conversation shown, on a clock at the fixture's `now`.
fn open(a: &mut App, project: &str, id: u64) {
    let now = Fixture::design().value()["now"]
        .as_i64()
        .unwrap_or_default();
    a.screens.conversation.clock = Some(Wall(now));
    on_fixture(a, Stage::Item(conv(project, id)));
}

/// `notes-cli #4` after its first frame, `later[0]` and `later[1]` (states.md 15).
fn streaming(a: &mut App) {
    a.link = played(&[("open notes-cli #4", 2)]);
    open(a, "notes-cli", 4);
}

fn compose(a: &mut App, draft: &str) {
    super::compose(a);
    super::state::composer_edit(a).replace_text(draft);
}

/// `render-core #3` with the details panel open, the cursor on its second `w1.design` record.
fn records(a: &mut App) {
    open(a, "render-core", 3);
    a.details_visible = true;
    let Some(s) = shown(a) else { return };
    let items = super::state::items(a, &s);
    let second = items
        .iter()
        .filter_map(|i| match i {
            Item::Record { head, kind, .. } if kind == "w1.design" => Some(Stop::Head(*head)),
            _ => None,
        })
        .nth(1);
    if let Some(stop) = second {
        let c = a
            .screens
            .conversation
            .conv_mut(&ConvRef::new("render-core", 3));
        c.cursor = Some(stop.clone());
        a.screens.conversation.reveal = Some(Reveal::Top(stop));
    }
}

/// The `hostile` block merged into the fixture (states.md 47), `hostile #2` shown and its group's
/// other two items open after it.
fn hostile(a: &mut App) {
    a.link = fixture_link(|v| {
        let h = v["hostile"].clone();
        if let Some(items) = v["calls"]["projects"]["result"]["items"].as_array_mut() {
            items.push(h["project"].clone());
        }
        v["calls"]["conversations hostile"] = h["conversations"].clone();
        v["calls"]["faults hostile"] = json!({"result": {"items": []}});
        v["calls"]["options hostile"] = json!({"result": {"items": []}});
        v["calls"]["decision"]["hostile/w1:4:design#1"] =
            h["decision hostile/w1:4:design#1"].clone();
        v["streams"]["open hostile #2"] = h["open hostile #2"].clone();
        let mut log = v["streams"]["timeline notes-cli"].clone();
        log["first"]["ops"][0][1]["entries"] = json!([]);
        v["streams"]["timeline hostile"] = log;
        v["streams"]["inbox"]["first"]["ops"][0][1]["hostile"] = h["inbox"]["hostile"].clone();
        v["local"]["firstInput"]["hostile"] = h["firstInput"].clone();
    });
    open(a, "hostile", 2);
    a.open.open(OpenItem::Decision {
        project: "hostile".into(),
        id: "w1:4:design#1".into(),
    });
    a.open.open(conv("hostile", 3));
    link::settle(a);
}

/// The design's fixture with each named stream's first `n` later frames played after its first.
fn played(streams: &[(&str, usize)]) -> Link {
    fixture_link(|v| {
        for (name, n) in streams {
            let later: Vec<Value> = v["streams"][*name]["later"]
                .as_array()
                .into_iter()
                .flatten()
                .take(*n)
                .flat_map(|f| f["ops"].as_array().cloned().unwrap_or_default())
                .collect();
            if let Some(ops) = v["streams"][*name]["first"]["ops"].as_array_mut() {
                ops.extend(later);
            }
        }
    })
}

fn fixture_link(edit: impl FnOnce(&mut Value)) -> Link {
    let mut v = Fixture::design().value().clone();
    edit(&mut v);
    Link::fixture(Fixture::parse(&v.to_string()).expect("the fixture stays JSON"))
}
