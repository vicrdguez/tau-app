//! The reader over one decision: the cursor's motions, Visual, the `c` operator and the notes it
//! makes, `]n [n`, `i`, `dd`, `u` (`spec.md` §5.6 "Keys", `grammar.md` §4, §6).

use std::ops::Range;

use tau_client::DecisionRef;
use tau_client::wire::DecisionView;

use super::doc::{Doc, byte_of, utf16_of};
use super::notes::Draft;
use super::state::{Anchor, Cursor, Editing, Op};
use crate::app::{App, Stage, decision_ref};
use crate::keys::{Code, Key};

/// The decision the stage shows.
pub fn shown(app: &App) -> Option<DecisionRef> {
    match &app.stage {
        Stage::Item(item) => decision_ref(item),
        _ => None,
    }
}

pub fn record<'a>(app: &'a App, d: &DecisionRef) -> Option<&'a DecisionView> {
    app.link.server().decisions.get(d).and_then(|k| k.value())
}

/// The basis as the page draws it: rendered, or as source lines after `␣ts`.
pub fn doc(app: &App, d: &DecisionRef) -> Option<Doc> {
    let text = &record(app, d)?.text;
    let source = app.screens.decision.page_of(d).is_some_and(|p| p.source);
    Some(if source {
        Doc::source(text)
    } else {
        Doc::rendered(text)
    })
}

/// The cursor as drawn: where this device left it, else the first line of the content.
pub fn cursor(app: &App, d: &DecisionRef) -> Option<Cursor> {
    let doc = doc(app, d)?;
    let choices = record(app, d)?.decision.choices.len();
    match app.screens.decision.page_of(d).and_then(|p| p.cursor) {
        Some(Cursor::Choice(i)) if i < choices => Some(Cursor::Choice(i)),
        Some(Cursor::Text(at)) => doc.snap(at).map(Cursor::Text),
        _ => doc
            .first(0)
            .map(Cursor::Text)
            .or((choices > 0).then_some(Cursor::Choice(0))),
    }
}

fn set_cursor(app: &mut App, d: &DecisionRef, c: Cursor) {
    app.screens.decision.page(d).cursor = Some(c);
    app.screens.decision.reveal = true;
}

/// A motion's target, and whether it moves by whole blocks (for an operator).
fn target(
    doc: &Doc,
    choices: usize,
    from: Cursor,
    keys: &[Key],
    count: u32,
) -> Option<(Cursor, bool)> {
    let n = count.max(1) as i64;
    let last_text = doc.blocks.len().checked_sub(1).and_then(|b| {
        (0..=b)
            .rev()
            .find_map(|i| doc.first(i).filter(|&p| doc.block_of(p) == Some(i)))
    });
    let lines = |by: i64| -> Option<Cursor> {
        match from {
            Cursor::Text(at) => match doc.block_step(at, by) {
                Some(p) => Some(Cursor::Text(p)),
                None if by > 0 && choices > 0 => {
                    let left = by - blocks_after(doc, at);
                    Some(Cursor::Choice(
                        ((left - 1).max(0) as usize).min(choices - 1),
                    ))
                }
                None if by > 0 => last_text.map(Cursor::Text),
                None => doc.first(0).map(Cursor::Text),
            },
            Cursor::Choice(i) => {
                let to = i as i64 + by;
                if to < 0 {
                    let up = -to;
                    last_text
                        .and_then(|l| doc.block_step(l, -(up - 1)).or(Some(l)))
                        .or_else(|| doc.first(0))
                        .map(Cursor::Text)
                } else {
                    Some(Cursor::Choice((to as usize).min(choices.saturating_sub(1))))
                }
            }
        }
    };
    let chars = |f: &dyn Fn(usize) -> usize| -> Option<Cursor> {
        match from {
            Cursor::Text(mut at) => {
                for _ in 0..n {
                    at = f(at);
                }
                Some(Cursor::Text(at))
            }
            c @ Cursor::Choice(_) => Some(c),
        }
    };
    let ch = |c: char| keys == [Key::char(c)];
    Some(if ch('j') || ch('}') || keys == [Key::plain(Code::Down)] {
        (lines(n)?, true)
    } else if ch('k') || ch('{') || keys == [Key::plain(Code::Up)] {
        (lines(-n)?, true)
    } else if keys == [Key::ctrl('d')] {
        (lines(8 * n)?, true)
    } else if keys == [Key::ctrl('u')] {
        (lines(-8 * n)?, true)
    } else if keys == [Key::ctrl('f')] {
        (lines(16 * n)?, true)
    } else if keys == [Key::ctrl('b')] {
        (lines(-16 * n)?, true)
    } else if keys == [Key::char('g'), Key::char('g')] {
        (Cursor::Text(doc.first(0)?), true)
    } else if ch('G') {
        let end = if choices > 0 {
            Cursor::Choice(choices - 1)
        } else {
            Cursor::Text(last_text?)
        };
        (end, true)
    } else if ch('h') || keys == [Key::plain(Code::Left)] {
        (chars(&|a| doc.char_step(a, -1))?, false)
    } else if ch('l') || keys == [Key::plain(Code::Right)] {
        (chars(&|a| doc.char_step(a, 1))?, false)
    } else if ch('w') {
        (chars(&|a| doc.word_forward(a))?, false)
    } else if ch('b') {
        (chars(&|a| doc.word_back(a))?, false)
    } else if ch('e') {
        (chars(&|a| doc.word_end(a))?, false)
    } else if ch('0') {
        (chars(&|a| doc.line_start(a))?, false)
    } else if ch('$') {
        (chars(&|a| doc.line_end(a))?, false)
    } else {
        return None;
    })
}

fn blocks_after(doc: &Doc, at: usize) -> i64 {
    let b = doc.block_of(at).unwrap_or(0);
    (b + 1..doc.blocks.len())
        .filter(|&i| doc.first(i).is_some_and(|p| doc.block_of(p) == Some(i)))
        .count() as i64
}

/// Keys no command took on the page: motions, `v V`, Visual's `Esc`, an operator's object.
pub fn key(app: &mut App, keys: &[Key], count: Option<u32>) -> bool {
    let Some(d) = shown(app) else { return true };
    let (Some(doc), Some(from)) = (doc(app, &d), cursor(app, &d)) else {
        return true;
    };
    let choices = record(app, &d).map_or(0, |v| v.decision.choices.len());
    let page = app.screens.decision.page(&d);
    if let Some(Op::Object { around }) = page.op {
        page.op = None;
        if let (Cursor::Text(at), [k]) = (from, keys) {
            let range = match k.typed() {
                Some('w') => Some(doc.word_around(at, around)),
                Some('p' | 's') => doc.block_of(at).map(|b| doc.blocks[b].range.clone()),
                _ => None,
            };
            if let Some(r) = range {
                annotate(app, &d, r);
            }
        }
        return true;
    }
    if keys == [Key::char('a')] && page.op == Some(Op::Annotate) {
        page.op = Some(Op::Object { around: true });
        return true;
    }
    if keys == [Key::plain(Code::Esc)] {
        page.visual = None;
        page.op = None;
        return true;
    }
    if let (Some(c @ ('v' | 'V')), [_]) = (keys[0].typed(), keys) {
        let linewise = c == 'V';
        page.visual = match (page.visual, from) {
            (Some((_, l)), _) if l == linewise => None,
            (Some((anchor, _)), _) => Some((anchor, linewise)),
            (None, Cursor::Text(at)) => Some((at, linewise)),
            (None, Cursor::Choice(_)) => None,
        };
        return true;
    }
    let Some((to, linewise)) = target(&doc, choices, from, keys, count.unwrap_or(1)) else {
        return true;
    };
    if page.op.take() == Some(Op::Annotate) {
        if let (Cursor::Text(a), Cursor::Text(b)) = (from, to) {
            let inclusive = keys == [Key::char('e')] || keys == [Key::char('$')];
            let (lo, hi) = (a.min(b), a.max(b));
            let range = if linewise {
                blocks_between(&doc, lo, hi)
            } else if inclusive {
                lo..doc.after(hi)
            } else {
                lo..hi
            };
            if !range.is_empty() {
                annotate(app, &d, range);
            }
        }
        return true;
    }
    set_cursor(app, &d, to);
    true
}

fn blocks_between(doc: &Doc, a: usize, b: usize) -> Range<usize> {
    let (Some(x), Some(y)) = (doc.block_of(a), doc.block_of(b)) else {
        return a..b;
    };
    doc.blocks[x].range.start..doc.blocks[y].range.end
}

/// The Visual selection, in bytes of `text`.
pub fn selection(app: &App, d: &DecisionRef) -> Option<Range<usize>> {
    let page = app.screens.decision.page_of(d)?;
    let (anchor, linewise) = page.visual?;
    let Some(Cursor::Text(at)) = cursor(app, d) else {
        return None;
    };
    let doc = doc(app, d)?;
    let (lo, hi) = (anchor.min(at), anchor.max(at));
    Some(if linewise {
        blocks_between(&doc, lo, hi)
    } else {
        lo..doc.after(hi)
    })
}

/// Opens the note editor on `range` (bytes of `text`), or on the drafts' note already there.
fn annotate(app: &mut App, d: &DecisionRef, range: Range<usize>) {
    let (Some(doc), Some(view)) = (doc(app, d), record(app, d)) else {
        return;
    };
    let text = view.text.clone();
    let n = doc.note_range(&text, range);
    let units = utf16_of(&text, n.range.start) as u64..utf16_of(&text, n.range.end) as u64;
    let page = app.screens.decision.page(d);
    page.visual = None;
    page.op = None;
    let existing = app
        .screens
        .decision
        .notes
        .of(d)
        .iter()
        .position(|x| x.range.as_ref() == Some(&units));
    let body = existing.map_or(String::new(), |i| {
        app.screens.decision.notes.of(d)[i].text.clone()
    });
    app.screens.decision.editing = Some(Editing::new(
        d.clone(),
        Anchor::Range {
            range: units,
            widened: n.widened,
        },
        existing,
        &body,
    ));
}

/// Whether notes may be written: the decision is open (an answered one shows the answer's notes).
fn writable(app: &mut App, d: &DecisionRef) -> bool {
    let open = record(app, d).is_some_and(|v| {
        matches!(
            v.decision.status,
            tau_client::wire::DecisionStatus::Open | tau_client::wire::DecisionStatus::Other
        )
    });
    if !open {
        app.say("not open · notes go with an answer");
    }
    open
}

/// `c` and `gc`: in Visual, a note on the selection; in Normal, wait for a motion; doubled, the
/// block under the cursor.
pub fn operator(app: &mut App) {
    let Some(d) = shown(app) else { return };
    if !writable(app, &d) {
        return;
    }
    if let Some(sel) = selection(app, &d) {
        annotate(app, &d, sel);
        return;
    }
    let page = app.screens.decision.page(&d);
    if page.op.take() == Some(Op::Annotate) {
        here(app);
    } else {
        page.op = Some(Op::Annotate);
    }
}

/// `␣mn`: the selection, else the block under the cursor.
pub fn here(app: &mut App) {
    let Some(d) = shown(app) else { return };
    if !writable(app, &d) {
        return;
    }
    if let Some(sel) = selection(app, &d) {
        annotate(app, &d, sel);
        return;
    }
    let (Some(doc), Some(Cursor::Text(at))) = (doc(app, &d), cursor(app, &d)) else {
        return;
    };
    if let Some(b) = doc.block_of(at) {
        annotate(app, &d, doc.blocks[b].range.clone());
    }
}

/// `␣mw`: the note on the whole, new or the one there.
pub fn whole(app: &mut App) {
    let Some(d) = shown(app) else { return };
    if !writable(app, &d) {
        return;
    }
    let notes = app.screens.decision.notes.of(&d);
    let existing = notes.iter().position(|n| n.range.is_none());
    let body = existing.map_or(String::new(), |i| notes[i].text.clone());
    app.screens.decision.editing = Some(Editing::new(d, Anchor::Whole, existing, &body));
}

/// The draft whose passage holds the cursor, else one starting in the cursor's block.
fn under_cursor(app: &App, d: &DecisionRef) -> Option<usize> {
    let doc = doc(app, d)?;
    let text = &record(app, d)?.text;
    let Cursor::Text(at) = cursor(app, d)? else {
        return None;
    };
    let block = doc.block_of(at);
    let notes = app.screens.decision.notes.of(d);
    let bytes = |n: &Draft| {
        let r = n.range.clone()?;
        Some(byte_of(text, r.start as usize)?..byte_of(text, r.end as usize)?)
    };
    notes
        .iter()
        .position(|n| bytes(n).is_some_and(|r| r.contains(&at)))
        .or_else(|| {
            notes
                .iter()
                .position(|n| bytes(n).is_some_and(|r| doc.block_of(r.start) == block))
        })
}

/// `i`: edit the note under the cursor; after `c`, the start of an inner object.
pub fn edit(app: &mut App) {
    let Some(d) = shown(app) else { return };
    let page = app.screens.decision.page(&d);
    if page.op == Some(Op::Annotate) {
        page.op = Some(Op::Object { around: false });
        return;
    }
    let Some(i) = under_cursor(app, &d) else {
        app.say("no note here");
        return;
    };
    let draft = app.screens.decision.notes.of(&d)[i].clone();
    let anchor = match draft.range {
        Some(range) => Anchor::Range {
            range,
            widened: draft.widened,
        },
        None => Anchor::Whole,
    };
    app.screens.decision.editing = Some(Editing::new(d, anchor, Some(i), &draft.text));
}

/// `dd`: discard the draft under the cursor.
pub fn discard(app: &mut App) {
    let Some(d) = shown(app) else { return };
    let Some(i) = under_cursor(app, &d) else {
        app.say("no note here");
        return;
    };
    let mut all = app.screens.decision.notes.of(&d).to_vec();
    all.remove(i);
    let s = &mut app.screens.decision;
    s.notes.set(&d, all, app.link.store());
}

/// `u` and `C-r`.
pub fn undo(app: &mut App, back: bool) {
    let Some(d) = shown(app) else { return };
    let s = &mut app.screens.decision;
    let done = if back {
        s.notes.undo(&d, app.link.store())
    } else {
        s.notes.redo(&d, app.link.store())
    };
    if !done {
        app.say(if back {
            "Already at oldest change"
        } else {
            "Already at newest change"
        });
    }
}

/// `]n` and `[n`: the next or previous note's passage, in text order.
pub fn next_note(app: &mut App, forward: bool, count: u32) {
    let Some(d) = shown(app) else { return };
    let (Some(doc), Some(view)) = (doc(app, &d), record(app, &d)) else {
        return;
    };
    let text = view.text.clone();
    let mut starts: Vec<usize> = app
        .screens
        .decision
        .notes
        .of(&d)
        .iter()
        .filter_map(|n| byte_of(&text, n.range.as_ref()?.start as usize))
        .filter_map(|b| doc.snap(b))
        .collect();
    starts.sort_unstable();
    starts.dedup();
    let at = match cursor(app, &d) {
        Some(Cursor::Text(at)) => at,
        Some(Cursor::Choice(_)) => usize::MAX,
        None => return,
    };
    let mut to = None;
    let mut from = at;
    for _ in 0..count.max(1) {
        let next = if forward {
            starts.iter().copied().find(|&s| s > from)
        } else {
            starts.iter().rev().copied().find(|&s| s < from)
        };
        match next {
            Some(n) => {
                from = n;
                to = Some(n);
            }
            None => break,
        }
    }
    match to {
        Some(p) => set_cursor(app, &d, Cursor::Text(p)),
        None => app.say(if forward {
            "no next note"
        } else {
            "no earlier note"
        }),
    }
}

/// The note editor's keys: Insert types (false hands the key to the field), Normal edits.
pub fn editor_key(app: &mut App, keys: &[Key], count: Option<u32>) -> bool {
    let now = app.now;
    let Some(e) = app.screens.decision.editing.as_mut() else {
        return true;
    };
    if e.inserting() {
        return keys
            .iter()
            .all(|k| e.editor.insert(*k, now) == crate::editor::Reply::Handled);
    }
    for k in keys {
        e.editor.normal(count, *k);
    }
    true
}

/// Leaves the note editor for the view, keeping what was typed as the draft (none if empty).
pub fn finish(app: &mut App) {
    let Some(e) = app.screens.decision.editing.take() else {
        return;
    };
    let text = e.editor.text().trim_end().to_string();
    let range = match &e.anchor {
        Anchor::Whole => None,
        Anchor::Range { range, .. } => Some(range.clone()),
    };
    let widened = matches!(e.anchor, Anchor::Range { widened: true, .. });
    let s = &mut app.screens.decision;
    let store = app.link.store();
    let mut all = s.notes.of(&e.decision).to_vec();
    match (e.draft, text.trim().is_empty()) {
        (Some(i), true) if i < all.len() => {
            all.remove(i);
            s.notes.set(&e.decision, all, store);
        }
        (Some(i), false) if i < all.len() => {
            all[i].text = text;
            s.notes.set(&e.decision, all, store);
        }
        (_, false) => s.notes.add(
            &e.decision,
            Draft {
                text,
                range,
                widened,
            },
            store,
        ),
        _ => {}
    }
}
