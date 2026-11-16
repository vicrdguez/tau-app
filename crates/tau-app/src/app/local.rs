//! Device-local app state. Load at launch, save on blur, close, or quit, never from `poll`.
//! Screens retain their existing draft files; this snapshot holds frame and item state.

use super::App;
use crate::open::Item;
use crate::screens::{self, conversation::transcript::Stop, decision::state::Cursor};
use serde::{Deserialize, Serialize};
use tau_client::{ConvRef, DecisionRef};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Saved {
    sidebar: Option<bool>,
    details: bool,
    open: Vec<Item>,
    conversations: Vec<(ConvRef, String, Option<Stop>)>,
    decisions: Vec<(DecisionRef, Option<Cursor>)>,
    projects: std::collections::BTreeMap<String, screens::project::Target>,
    editing: Option<Editing>,
}

#[derive(Serialize, Deserialize)]
struct Editing {
    decision: DecisionRef,
    anchor: screens::decision::state::Anchor,
    draft: Option<usize>,
    text: String,
}

pub fn load(app: &mut App) {
    let saved: Saved = app.link.store().load("workspace.json").unwrap_or_default();
    app.sidebar_visible = saved.sidebar.unwrap_or(true);
    app.details_visible = saved.details;
    for item in saved.open {
        app.open.open(item);
    }
    for (id, text, cursor) in saved.conversations {
        let c = app.screens.conversation.conv_mut(&id);
        c.composer.edit.replace_text(text);
        c.cursor = cursor;
    }
    for (id, cursor) in saved.decisions {
        app.screens.decision.page(&id).cursor = cursor;
    }
    app.screens.project.cursors = saved.projects;
    app.screens.decision.editing = saved
        .editing
        .map(|e| screens::decision::state::Editing::new(e.decision, e.anchor, e.draft, &e.text));
    app.screens.decision.notes.load(app.link.store());
    screens::home::load(app);
    screens::places::load(app);
}

pub fn remember_project(app: &mut App) {
    if let super::Stage::Item(Item::Project(p)) = &app.stage
        && let Some(cursor) = &app.screens.project.cursor
    {
        app.screens
            .project
            .cursors
            .insert(p.clone(), cursor.clone());
    }
}

pub fn save(app: &mut App) {
    remember_project(app);
    let saved = Saved {
        sidebar: Some(app.sidebar_visible),
        details: app.details_visible,
        open: app.open.iter().cloned().collect(),
        conversations: app.screens.conversation.saved(),
        projects: app.screens.project.cursors.clone(),
        editing: app.screens.decision.editing.as_ref().map(|e| Editing {
            decision: e.decision.clone(),
            anchor: e.anchor.clone(),
            draft: e.draft,
            text: e.editor.text().into(),
        }),
        decisions: app
            .screens
            .decision
            .pages
            .iter()
            .map(|(d, p)| (d.clone(), p.cursor))
            .collect(),
    };
    if let Err(e) = app.link.store().save("workspace.json", &saved) {
        app.say(format!("Could not keep the workspace on this device: {e}"));
    }
    screens::places::save(app);
}

pub fn forget(app: &mut App) {
    app.link.forget_server();
    app.open = Default::default();
    app.last_item = None;
    app.hidden = Default::default();
    app.server = Default::default();
    app.screens = Default::default();
    app.sidebar_visible = true;
    app.details_visible = false;
}
