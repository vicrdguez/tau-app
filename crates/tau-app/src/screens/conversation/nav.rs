//! The conversation's side of the jumplist and hint mode (`grammar.md` §13): its anchor is the stop
//! the cursor stands on, or `null` while it follows the run.

use super::state::{following, items, shown};
use super::transcript::{Stop, stops};
use super::{resume, set_cursor, walk};
use crate::app::App;
use crate::screens::{Anchor, HintTarget};

/// Every stop: message heads, text blocks, thinking, tool rows and folds, chrome lines.
pub fn targets(app: &App) -> Vec<HintTarget> {
    let Some(s) = shown(app) else {
        return Vec::new();
    };
    stops(&items(app, &s))
        .into_iter()
        .map(|stop| HintTarget {
            key: stop.key(),
            at: [0.0, 2.0],
            within: super::view::TRANSCRIPT,
            anchor: serde_json::to_value(Some(&stop)).unwrap_or_default(),
        })
        .collect()
}

pub fn anchor(app: &App) -> Option<Anchor> {
    shown(app)?;
    let stop = if following(app) {
        None
    } else {
        walk(app).map(|w| w.stops[w.at].clone())
    };
    serde_json::to_value(stop).ok()
}

pub fn land(app: &mut App, anchor: &Anchor) {
    match serde_json::from_value::<Option<Stop>>(anchor.clone()) {
        Ok(None) => resume(app),
        Ok(Some(stop)) => {
            if let Some(w) = walk(app)
                && let Some(to) = w.stops.iter().position(|s| *s == stop)
            {
                set_cursor(app, &w, to);
            }
        }
        Err(_) => {}
    }
}
