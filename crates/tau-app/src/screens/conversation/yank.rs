//! What `y` takes from the transcript (`grammar.md` §4). The cursor stands on a whole block, so a
//! word or a sentence is the one its block starts with. Text comes from the entries as the server
//! holds them, so what is copied is their source bytes; `im` is a message's text, each part as
//! sent, a blank line between parts.

use std::ops::Range;

use tau_client::EntryId;
use tau_client::model::Transcript as Held;
use tau_client::shapes::{Block as Content, Content as UserContent, EntryBody};

use super::state::{current_stop, items, shown};
use super::transcript::{Item, Part, Stop, blocks, stops};
use crate::app::App;
use crate::yank::{Object, objects};

pub fn text(app: &App, obj: Object) -> Option<String> {
    let s = shown(app)?;
    let held = s.held?;
    let at = current_stop(app, &s)?;
    let items = items(app, &s);
    let item = items
        .iter()
        .find(|i| stops(std::slice::from_ref(*i)).contains(&at))?;
    match (obj, &at) {
        (Object::Message { around }, _) => message(held, item, around),
        (Object::Line, Stop::Head(_)) => message(held, item, false),
        (_, Stop::Text(e, c, b)) => {
            let source = content(held, *e, *c)?;
            let block = block(source, *b)?;
            let range = match obj {
                Object::Word { around: false } => objects::inner_word(source, block.start),
                Object::Word { around: true } => objects::a_word(source, block.start),
                Object::Sentence { around } => {
                    let r = objects::sentence(&source[block.clone()], 0, around);
                    block.start + r.start..block.start + r.end
                }
                Object::Paragraph { around: true } => objects::spread(source, block),
                Object::Paragraph { around: false } | Object::Line => block,
                Object::Selection | Object::Message { .. } => return None,
            };
            source.get(range).map(str::to_string)
        }
        (Object::Line, _) => row_text(item, &at),
        _ => None,
    }
}

/// `gx`: the first link in the message block under the cursor.
pub fn link(app: &App) -> Option<String> {
    let s = shown(app)?;
    let Stop::Text(e, c, b) = current_stop(app, &s)? else {
        return None;
    };
    let source = content(s.held?, e, c)?;
    let block = block(source, b)?;
    crate::content::link_at(source, block.start, block)
}

/// The `c`th part of entry `e`, when it is text.
fn content(held: &Held, e: EntryId, c: usize) -> Option<&str> {
    match &held.entries.get(&e)?.body {
        EntryBody::User { content, .. } => match content.get(c)? {
            UserContent::Text(s) => Some(s),
            UserContent::Image { .. } => None,
        },
        EntryBody::Assistant(m) => match m.blocks.get(c)? {
            Content::Text(s) => Some(s),
            _ => None,
        },
        EntryBody::Record { text } => text.as_deref().filter(|_| c == 0),
        _ => None,
    }
}

/// Block `b` of `source`, as a range of it.
fn block(source: &str, b: usize) -> Option<Range<usize>> {
    let slice = *blocks(source).get(b)?;
    let start = slice.as_ptr() as usize - source.as_ptr() as usize;
    Some(start..start + slice.len())
}

/// A message's text parts, each as sent, a blank line between; `am` keeps their trailing blanks.
fn message(held: &Held, item: &Item, around: bool) -> Option<String> {
    let mut parts: Vec<(EntryId, usize)> = Vec::new();
    for stop in stops(std::slice::from_ref(item)) {
        if let Stop::Text(e, c, _) = stop
            && !parts.contains(&(e, c))
        {
            parts.push((e, c));
        }
    }
    let texts: Vec<&str> = parts
        .into_iter()
        .filter_map(|(e, c)| content(held, e, c))
        .map(|t| if around { t } else { t.trim_end() })
        .collect();
    (!texts.is_empty()).then(|| texts.join("\n\n"))
}

/// `yy` on a row that isn't message text: a tool's arguments line, a thinking block's text, a
/// chrome line's text.
fn row_text(item: &Item, at: &Stop) -> Option<String> {
    match item {
        Item::Turn { parts, .. } => parts.iter().find_map(|p| match p {
            Part::Thinking { stop, text, .. } if stop == at => Some(text.clone()),
            Part::Tool(r) if Stop::Tool(r.call.clone()) == *at => Some(r.args.clone()),
            Part::Fold { rows, .. } => rows
                .iter()
                .find(|r| Stop::Tool(r.call.clone()) == *at)
                .map(|r| r.args.clone()),
            _ => None,
        }),
        Item::Chrome { stop, text, .. } if stop == at => text.clone(),
        _ => None,
    }
}
