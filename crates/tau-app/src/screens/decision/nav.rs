//! The decision's side of the jumplist and hint mode (`grammar.md` §13): its anchor is the reader's
//! cursor, a byte of the basis or a choice row.

use super::reader::{cursor, doc, record, shown};
use super::state::Cursor;
use crate::app::App;
use crate::screens::{Anchor, HintTarget};

/// The basis's blocks, then the choices.
pub fn targets(app: &App) -> Vec<HintTarget> {
    let Some(d) = shown(app) else {
        return Vec::new();
    };
    let (Some(doc), Some(v)) = (doc(app, &d), record(app, &d)) else {
        return Vec::new();
    };
    let target = |key: String, at: [f32; 2], c: Cursor| HintTarget {
        key,
        at,
        within: "decision-page",
        anchor: serde_json::to_value(c).unwrap_or_default(),
    };
    let blocks = (0..doc.blocks.len()).filter_map(|k| {
        let first = doc.first(k).filter(|&p| doc.block_of(p) == Some(k))?;
        Some(target(format!("b{k}"), [0.0, 3.0], Cursor::Text(first)))
    });
    let choices = (0..v.decision.choices.len())
        .map(|i| target(format!("choice-{i}"), [0.0, 6.0], Cursor::Choice(i)));
    blocks.chain(choices).collect()
}

pub fn anchor(app: &App) -> Option<Anchor> {
    let d = shown(app)?;
    serde_json::to_value(cursor(app, &d)?).ok()
}

/// `gx`: the link under the cursor, else the first in its block.
pub fn link(app: &App) -> Option<String> {
    let d = shown(app)?;
    let Cursor::Text(at) = cursor(app, &d)? else {
        return None;
    };
    let doc = doc(app, &d)?;
    let block = doc.blocks.get(doc.block_of(at)?)?.range.clone();
    crate::content::link_at(&record(app, &d)?.text, at, block)
}

pub fn land(app: &mut App, anchor: &Anchor) {
    let Some(d) = shown(app) else { return };
    if let Ok(c) = serde_json::from_value::<Cursor>(anchor.clone()) {
        let page = app.screens.decision.page(&d);
        page.cursor = Some(c);
        page.visual = None;
        page.op = None;
        app.screens.decision.reveal = true;
    }
}
