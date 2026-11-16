use super::*;
use crate::app::{scenes::Scene, scenes::on_fixture};
use tau_client::{Fixture, Link};

pub(crate) fn base(a: &mut App) {
    a.live = false;
    a.screens.start.loaded = true;
    a.screens.start.saved.last_project = Some("render-core".into());
    a.screens
        .start
        .saved
        .last_with
        .insert("render-core".into(), StartWith::Workflow("w1".into()));
    a.screens.start.saved.last_with.insert(
        "notes-cli".into(),
        StartWith::Model("anthropic/claude-sonnet-4-5".into()),
    );
    on_fixture(a, Stage::Start);
    set_project(a, "render-core".into());
    crate::app::link::settle(a);
}
pub(crate) fn fixture_with(a: &mut App, change: impl FnOnce(&mut serde_json::Value)) {
    let mut value = Fixture::design().value().clone();
    change(&mut value);
    a.link = Link::fixture(Fixture::parse(&value.to_string()).expect("scene fixture"));
}
pub(crate) fn scratch(a: &mut App) {
    fixture_with(a, |v| {
        let p = v["calls"]["bind scratch"]["result"]["project"].clone();
        v["calls"]["projects"]["result"]["items"]
            .as_array_mut()
            .unwrap()
            .push(p);
        v["calls"]["conversations scratch"] =
            serde_json::json!({"result":{"items":[],"next":null}});
        v["calls"]["faults scratch"] = serde_json::json!({"result":{"items":[],"next":null}});
    });
    base(a);
    set_project(a, "scratch".into());
    crate::app::link::settle(a);
    typed(a);
}
fn typed(a: &mut App) {
    let f = Fixture::design();
    a.screens.start.editor.edit.replace_text(
        f.value()["local"]["startDraftExample"]["text"]
            .as_str()
            .unwrap(),
    );
}
pub(super) fn all() -> Vec<Scene> {
    vec![
        ("start", base),
        ("start-light", |a| {
            base(a);
            a.variant = crate::theme::Variant::EmberLight;
        }),
        ("start-project-picker", |a| {
            base(a);
            project_picker(a, ProjectPurpose::Start);
        }),
        ("start-workflow-picker", |a| {
            base(a);
            workflow_picker(a);
        }),
        ("start-starting", |a| {
            base(a);
            typed(a);
            crate::app::commands::run_id(a, "start.send", &mut Tx::default());
        }),
        ("start-no-reply", |a| {
            base(a);
            typed(a);
            a.screens.start.failure = Some(copy::start_unknown(
                &serde_json::json!({"project":"render-core"}),
            ));
            a.server.counts.unknown = Some(1);
        }),
        ("start-options-unset", scratch),
        ("start-options-invalid", |a| {
            fixture_with(a, |v| {
                v["calls"]["options render-core"] =
                    v["calls"]["options render-core (reviewer model gone)"].clone()
            });
            base(a);
            typed(a);
        }),
        ("start-missing", |a| {
            fixture_with(a, |v| {
                let project = v["calls"]["projects"]["result"]["items"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|p| p["project"] == "render-core")
                    .unwrap();
                project["missing"] = serde_json::json!(["w1"]);
            });
            base(a);
            typed(a);
        }),
        ("start-no-projects", |a| {
            fixture_with(a, |v| {
                v["calls"]["projects"]["result"]["items"] = serde_json::json!([])
            });
            base(a);
            a.open = Default::default();
            a.server.counts = Default::default();
        }),
        ("start-no-models", |a| {
            fixture_with(a, |v| {
                v["calls"]["models"]["result"]["items"] = serde_json::json!([])
            });
            base(a);
            set_project(a, "notes-cli".into());
            crate::app::link::settle(a);
        }),
    ]
}
