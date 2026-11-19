//! Moving between places (`grammar.md` §13): the jumplist (`C-o`, `C-i`, `:jumps`). It reads and
//! sets where a page's cursor stands only through the one seam every screen family has
//! (`Screen::anchor`, `Screen::land`), so it never reaches into a screen's state.
//!
//! A jump is a command the keymap marks `jump` (`␣n`, `gd`, `]q`, `]b`, every open) or a builtin
//! `gg` or `G`: when one moves the stage off where it was, that place joins the list.

pub mod hints;
mod jumps;
mod scenes;

pub use scenes::all as scenes;

use rune::Tx;
use serde::{Deserialize, Serialize};

pub use jumps::Jumps;

use crate::app::{App, Handler, Invocation, Stage};
use crate::floats::{Float, Sheet, SheetRow};
use crate::keys::Key;
use crate::open::Item;
use crate::screens::{self, Anchor};

/// The file the list is kept in, beside the server's other device-local state.
const FILE: &str = "jumps.json";

/// A place on the stage: the object it shows and where its cursor stands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spot {
    pub stage: Stage,
    pub anchor: Option<Anchor>,
}

/// Where the stage stands now.
pub fn here(app: &App) -> Spot {
    Spot {
        stage: app.stage.clone(),
        anchor: screens::page(app).anchor(app),
    }
}

/// Shows `spot`'s object (opening it if it isn't an open item) with its cursor where it stood.
pub fn go(app: &mut App, spot: &Spot) {
    app.show(spot.stage.clone());
    if let Some(anchor) = &spot.anchor {
        screens::page(app).land(app, anchor);
    }
}

/// A jump ran from `from`: keeps it when the stage moved off it.
pub fn left(app: &mut App, from: Spot) {
    if from.stage == Stage::Connect || here(app) == from {
        return;
    }
    list(app).push(from);
    save(app);
}

/// The builtin keys that are jumps: `gg` and `G`.
pub fn is_jump(keys: &[Key]) -> bool {
    keys == [Key::char('g'), Key::char('g')] || keys == [Key::char('G')]
}

pub fn handlers() -> Vec<(&'static str, Handler)> {
    let mut all: Vec<(&'static str, Handler)> = vec![
        ("jump.back", back),
        ("jump.forward", forward),
        ("open.jumps", open_jumps),
    ];
    all.extend(hints::handlers());
    all
}

/// The list, read from this device the first time it is needed for the server the link is on.
fn list(app: &mut App) -> &mut Jumps {
    let url = app.link.server().url.clone();
    if app.jumps.of.as_ref() != Some(&url) {
        app.jumps = app.link.store().load(FILE).unwrap_or_default();
        app.jumps.of = Some(url);
    }
    &mut app.jumps
}

fn save(app: &App) {
    let _ = app.link.store().save(FILE, &app.jumps);
}

fn count(inv: &Invocation) -> usize {
    inv.count.unwrap_or(1).max(1) as usize
}

/// `C-o`.
fn back(app: &mut App, inv: &Invocation, _: &mut Tx) {
    let here = here(app);
    if let Some(to) = list(app).back(here, count(inv)) {
        save(app);
        go(app, &to);
    }
}

/// `C-i`.
fn forward(app: &mut App, inv: &Invocation, _: &mut Tx) {
    if let Some(to) = list(app).forward(count(inv)) {
        go(app, &to);
    }
}

/// `:jumps`: the list, newest last, the cursor on the entry `C-o` stands on; `⏎` goes there.
fn open_jumps(app: &mut App, _: &Invocation, _: &mut Tx) {
    let entries = list(app).entries().to_vec();
    if entries.is_empty() {
        app.say("no jumps yet");
        return;
    }
    let rows = entries.iter().map(|s| row(app, s)).collect();
    let mut sheet = Sheet::new(
        "Jumps",
        crate::floats::Noun::new("jump", "jumps"),
        "the places jumps left, newest last".into(),
        rows,
    );
    sheet.cursor = app.jumps.current().unwrap_or(entries.len() - 1);
    sheet.choose = Some(|a, i| {
        let here = here(a);
        if let Some(to) = list(a).to(here, i) {
            save(a);
            go(a, &to);
        }
    });
    app.open_float(Float::Sheet(sheet));
}

fn row(app: &App, spot: &Spot) -> SheetRow {
    let (text, meta) = match &spot.stage {
        Stage::Place(p) => (p.name().to_string(), String::new()),
        Stage::Start => ("New conversation".to_string(), String::new()),
        Stage::Connect => ("Connect".to_string(), String::new()),
        Stage::Item(item) => {
            let meta = match item {
                Item::Conversation { project, id } => format!("{project} #{id}"),
                Item::Decision { project, id } => format!("{project} {id}"),
                Item::Project(_) => "project".to_string(),
            };
            (screens::item_title(app, item), meta)
        }
    };
    SheetRow {
        keys: String::new(),
        text,
        meta,
    }
}

#[cfg(test)]
mod gx_tests;
#[cfg(test)]
mod hints_tests;
#[cfg(test)]
mod tests;
