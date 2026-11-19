use super::*;
use crate::app::scenes::Scene;
use crate::screens::start::scenes::{base, scratch};
fn project(a: &mut App) {
    base(a);
    open_options(a, "render-core".into(), false);
    crate::app::link::settle(a);
    a.screens.project.cursor = Some(Target::Field(FieldKey {
        project: "render-core".into(),
        extension: "w1".into(),
        key: "reviewer".into(),
    }));
}
pub(super) fn all() -> Vec<Scene> {
    vec![
        ("project", project),
        ("project-draft", |a| {
            project(a);
            a.screens.project.drafts.insert(
                FieldKey {
                    project: "render-core".into(),
                    extension: "w1".into(),
                    key: "base".into(),
                },
                Value::String("release".into()),
            );
        }),
        ("project-saved", |a| {
            project(a);
            a.screens.project.drafts.insert(
                FieldKey {
                    project: "render-core".into(),
                    extension: "w1".into(),
                    key: "base".into(),
                },
                Value::String("release".into()),
            );
            crate::app::commands::run_id(a, "options.save", &mut Tx::default());
            a.live = true;
            crate::app::link::pump(a, &mut Tx::default());
            a.live = false;
        }),
        ("projects", |a| {
            base(a);
            a.show(Stage::Place(Place::Projects));
        }),
        ("project-options-unset", |a| {
            scratch(a);
            open_options(a, "scratch".into(), true);
            crate::app::link::settle(a);
            actions::keep_cursor(a);
        }),
        ("options-error", |a| {
            scratch(a);
            open_options(a, "scratch".into(), true);
            crate::app::link::settle(a);
            let id = FieldKey {
                project: "scratch".into(),
                extension: "w1".into(),
                key: "designer".into(),
            };
            a.screens
                .project
                .drafts
                .insert(id.clone(), Value::String("anthropic/claude-opus-9".into()));
            a.screens.project.cursor = Some(Target::Field(id));
            let f = tau_client::Fixture::design();
            let errors =
                serde_json::from_value(
                    f.value()["refusals"]["bad_options (setOptions designer to an unknown model)"]
                        ["reply"]["refused"]["errors"]
                        .clone(),
                )
                .unwrap();
            a.screens
                .project
                .errors
                .insert(("scratch".into(), "w1".into()), errors);
            a.say("The options of w1 do not fit.");
        }),
        ("bind-float", |a| {
            base(a);
            a.open = Default::default();
            a.server.counts = Default::default();
            a.show(Stage::Place(Place::Home));
            bind(a);
            let f = super::super::top_float::<crate::floats::project_bind::Bind>(a).unwrap();
            f.name.edit.replace_text("scratch");
            f.directory.edit.replace_text("/home/vrod/src/scratch");
            f.part = crate::floats::project_bind::Part::Extensions;
            f.cursor = f
                .extensions
                .iter()
                .position(|e| e.name == "w1")
                .unwrap_or(0);
            if let Some(e) = f.extensions.get_mut(f.cursor) {
                e.checked = true;
            }
        }),
        ("add-extensions", |a| {
            project(a);
            add(a, "render-core".into());
        }),
    ]
}
