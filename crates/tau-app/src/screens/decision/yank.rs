//! What `y` takes from the basis (`grammar.md` §4): the source bytes of the word, sentence or
//! block under the cursor, or of the Visual selection; on a choice row, `yy` takes the choice.

use super::reader::{cursor, doc, record, selection, shown};
use super::state::Cursor;
use crate::app::App;
use crate::yank::{Object, objects};

pub fn text(app: &mut App, obj: Object) -> Option<String> {
    let d = shown(app)?;
    let view = record(app, &d)?;
    let at = match cursor(app, &d)? {
        Cursor::Text(at) => at,
        Cursor::Choice(i) if obj == Object::Line => return view.decision.choices.get(i).cloned(),
        Cursor::Choice(_) => return None,
    };
    let text = view.text.clone();
    let doc = doc(app, &d)?;
    let block = doc.blocks.get(doc.block_of(at)?)?.range.clone();
    let range = match obj {
        Object::Word { around } => doc.word_around(at, around),
        Object::Sentence { around } => {
            let r = objects::sentence(&text[block.clone()], at - block.start, around);
            block.start + r.start..block.start + r.end
        }
        Object::Paragraph { around: true } => objects::spread(&text, block),
        Object::Paragraph { around: false } | Object::Line => block,
        Object::Selection => {
            let sel = selection(app, &d)?;
            let page = app.screens.decision.page(&d);
            page.visual = None;
            page.cursor = Some(Cursor::Text(sel.start));
            sel
        }
        Object::Message { .. } => return None,
    };
    text.get(range).map(str::to_string)
}
