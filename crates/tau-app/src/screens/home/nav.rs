//! A list's side of the jumplist and hint mode, for Home and the places (`grammar.md` §13): its
//! anchor is the row the cursor is on, named by its object's ID.

use super::list::{ListRow, RowId};
use super::{places, rows, select};
use crate::app::{App, Stage};
use crate::frame::Place;
use crate::screens::{Anchor, HintTarget};

/// The list's rows, each labelled after its mark.
pub fn targets(app: &App) -> Vec<HintTarget> {
    let within = if app.stage == Stage::Place(Place::Home) {
        "home-list"
    } else {
        "places-list"
    };
    shown_rows(app)
        .into_iter()
        .filter_map(|r| {
            Some(HintTarget {
                key: format!("list-{:?}", r.id),
                // Activity rows have no mark: their time starts the text.
                at: match r.id {
                    RowId::Event(..) => [12.0, 6.0],
                    _ => [36.0, 6.0],
                },
                within,
                anchor: serde_json::to_value(&r.id).ok()?,
            })
        })
        .collect()
}

fn shown_rows(app: &App) -> Vec<ListRow> {
    if app.stage == Stage::Place(Place::Home) {
        rows(app)
    } else {
        places::rows(app)
    }
}

pub fn anchor(app: &App) -> Option<Anchor> {
    let rows = shown_rows(app);
    let cursor = if app.stage == Stage::Place(Place::Home) {
        &app.screens.home.cursor
    } else {
        &places::settings(app).cursor
    };
    let row = rows.get(cursor.index(&rows))?;
    serde_json::to_value(&row.id).ok()
}

pub fn land(app: &mut App, anchor: &Anchor) {
    if let Ok(id) = serde_json::from_value::<RowId>(anchor.clone())
        && shown_rows(app).iter().any(|r| r.id == id)
    {
        select(app, id);
    }
}
