//! One modal editor for every place you type (`grammar.md` §10): the composer, notes, option
//! fields, Connect and the start page. It layers a vim subset over rune's `TextEdit`, which keeps
//! the text, the undo history and the input method; in Insert, plain typing goes to rune's field.
//! The host resolves the keymap first (`⌘⏎`, `⏎` as a new line, `Esc` back to the view) and hands
//! the editor what is left.

pub(crate) mod text;

use text::{
    a_word, at_column, ch, column, first_non_blank, inner_word, last_char, line_at, line_count,
    line_end, line_index, line_start, lines_span, lines_text, list_marker, next, prev, word_back,
    word_end, word_forward,
};

use std::ops::Range;

use rune::TextEdit;

use crate::keys::{Code, Key};

/// Insert's `jk`: the `k` must follow the `j` within this many seconds. The design names no value;
/// 0.3 s matches which-key's delay.
pub const JK_SECONDS: f64 = 0.3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lines {
    /// The composer, a note, the start page: `⏎` is a new line.
    Multi,
    /// An option field, Connect: one line, `⏎` commits.
    Single,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditMode {
    Normal,
    Insert,
    Visual,
    VisualLine,
}

/// What the editor did with a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    Handled,
    /// Not the editor's: in Insert the field types it.
    Pass,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Register {
    text: String,
    linewise: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Delete,
    Change,
    Yank,
}

/// A Normal-mode sequence in progress.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Pending {
    count: Option<u32>,
    op: Option<Op>,
    /// The count typed after the operator (`d3w`).
    op_count: Option<u32>,
    /// `g` waiting for its second key.
    g: bool,
    /// `i` or `a` after an operator, waiting for the object.
    object: Option<char>,
}

#[derive(Clone, Debug)]
pub struct Editor {
    /// The field's model; a view hands it to `text_input` and edits it through `on_edit`.
    pub edit: TextEdit,
    lines: Lines,
    mode: EditMode,
    /// Visual mode's other end.
    anchor: usize,
    pending: Pending,
    register: Register,
    /// The column `j` and `k` keep, in characters.
    column: Option<usize>,
    /// A `j` held back in Insert, and when it was typed: a `k` in time makes the two leave
    /// Insert, anything else types it first.
    jk: Option<f64>,
}

impl Editor {
    /// An editor in Normal mode over `text`, the cursor at its start.
    pub fn new(text: &str, lines: Lines) -> Editor {
        let mut edit = TextEdit::new(text);
        edit.set_caret(0);
        edit.clear_history();
        Editor {
            edit,
            lines,
            mode: EditMode::Normal,
            anchor: 0,
            pending: Pending::default(),
            register: Register::default(),
            column: None,
            jk: None,
        }
    }

    /// An editor in Insert mode with the caret at the end (the composer and the start page).
    pub fn inserting(text: &str, lines: Lines) -> Editor {
        let mut ed = Editor::new(text, lines);
        ed.edit.set_caret(text.len());
        ed.mode = EditMode::Insert;
        ed
    }

    pub fn text(&self) -> &str {
        &self.edit.text
    }

    pub fn mode(&self) -> EditMode {
        self.mode
    }

    pub fn lines(&self) -> Lines {
        self.lines
    }

    /// The cursor's byte offset (the caret in Insert).
    pub fn cursor(&self) -> usize {
        self.edit.focus
    }

    /// An operator, `g` or a text object waits for its next key: the host hands it the next
    /// key directly.
    pub fn is_pending(&self) -> bool {
        self.pending.op.is_some() || self.pending.g || self.pending.object.is_some()
    }

    /// One key in Normal or Visual mode, with the count the keymap engine read before it.
    pub fn normal(&mut self, count: Option<u32>, key: Key) -> Reply {
        if count.is_some() {
            self.pending.count = count;
        }
        if key == Key::plain(Code::Esc) {
            if self.is_pending() || self.pending.count.is_some() {
                self.pending = Pending::default();
            } else if self.mode != EditMode::Normal {
                self.leave_visual();
            }
            return Reply::Handled;
        }
        let Some(c) = key.typed().or(match key {
            k if k == Key::ctrl('r') => Some('\u{12}'),
            _ => None,
        }) else {
            self.pending = Pending::default();
            return Reply::Pass;
        };
        if let Some(around) = self.pending.object.take() {
            return self.object(around, c);
        }
        if let Some(d) = c.to_digit(10)
            && !self.pending.g
        {
            let slot = if self.pending.op.is_some() {
                &mut self.pending.op_count
            } else {
                &mut self.pending.count
            };
            if d != 0 || slot.is_some() {
                *slot = Some(slot.unwrap_or(0).saturating_mul(10).saturating_add(d));
                return Reply::Handled;
            }
        }
        if std::mem::take(&mut self.pending.g) {
            return match c {
                'g' => self.motion(Motion::FirstLine),
                _ => self.cancel(),
            };
        }
        let motion = match c {
            'h' => Some(Motion::Left),
            'l' => Some(Motion::Right),
            'j' => Some(Motion::Down),
            'k' => Some(Motion::Up),
            'w' => Some(Motion::WordForward),
            'b' => Some(Motion::WordBack),
            'e' => Some(Motion::WordEnd),
            '0' => Some(Motion::LineStart),
            '^' => Some(Motion::FirstNonBlank),
            '$' => Some(Motion::LineEnd),
            'G' => Some(Motion::LastLine),
            _ => None,
        };
        if let Some(m) = motion {
            return self.motion(m);
        }
        if c == 'g' {
            self.pending.g = true;
            return Reply::Handled;
        }
        let op = match c {
            'd' => Some(Op::Delete),
            'c' => Some(Op::Change),
            'y' => Some(Op::Yank),
            _ => None,
        };
        if let Some(op) = op {
            return self.operator(op);
        }
        if self.pending.op.is_some() {
            if matches!(c, 'i' | 'a') {
                self.pending.object = Some(c);
                return Reply::Handled;
            }
            return self.cancel();
        }
        let n = self.take_count();
        if self.mode != EditMode::Normal {
            return match c {
                'x' => self.apply_selection(Op::Delete),
                'v' | 'V' => {
                    let line = c == 'V';
                    let same = (self.mode == EditMode::VisualLine) == line;
                    if same {
                        self.leave_visual();
                    } else {
                        self.mode = if line {
                            EditMode::VisualLine
                        } else {
                            EditMode::Visual
                        };
                    }
                    Reply::Handled
                }
                _ => Reply::Pass,
            };
        }
        match c {
            'x' => {
                let s = &self.edit.text;
                let cur = self.cursor();
                let end = line_end(s, cur);
                let mut b = cur;
                for _ in 0..n {
                    if b < end {
                        b = next(s, b);
                    }
                }
                self.apply(Op::Delete, cur..b);
            }
            'D' => {
                let cur = self.cursor();
                self.apply(Op::Delete, cur..line_end(self.text(), cur));
            }
            'C' => {
                let cur = self.cursor();
                self.apply(Op::Change, cur..line_end(self.text(), cur));
            }
            'p' | 'P' => self.put(c == 'p', n),
            'u' => {
                for _ in 0..n {
                    self.edit.undo();
                }
                self.clamp();
            }
            '\u{12}' => {
                for _ in 0..n {
                    self.edit.redo();
                }
                self.clamp();
            }
            'i' => self.enter_insert(),
            'a' => {
                let cur = self.cursor();
                let at = if cur < line_end(self.text(), cur) {
                    next(self.text(), cur)
                } else {
                    cur
                };
                self.insert_at(at);
            }
            'I' => self.insert_at(first_non_blank(self.text(), self.cursor())),
            'A' => self.insert_at(line_end(self.text(), self.cursor())),
            'o' | 'O' if self.lines == Lines::Multi => {
                let cur = self.cursor();
                let at = if c == 'o' {
                    line_end(self.text(), cur)
                } else {
                    line_start(self.text(), cur)
                };
                self.change(|e| replace(e, at..at, "\n"));
                self.insert_at(if c == 'o' { at + 1 } else { at });
            }
            'v' | 'V' => {
                self.anchor = self.cursor();
                self.mode = if c == 'v' {
                    EditMode::Visual
                } else {
                    EditMode::VisualLine
                };
            }
            'o' | 'O' => {}
            _ => return Reply::Pass,
        }
        Reply::Handled
    }

    /// One key in Insert at time `now` (seconds). `Pass` means the field should type it.
    pub fn insert(&mut self, key: Key, now: f64) -> Reply {
        let held = self.jk.take();
        if self.edit.is_composing() {
            if held.is_some() {
                self.type_text("j");
            }
            return Reply::Pass;
        }
        if key.typed() == Some('k') && held.is_some_and(|at| now - at <= JK_SECONDS) {
            self.leave_insert();
            return Reply::Handled;
        }
        if held.is_some() {
            self.type_text("j");
        }
        if key.typed() == Some('j') {
            self.jk = Some(now);
            return Reply::Handled;
        }
        if key == Key::plain(Code::Esc) {
            self.leave_insert();
            return Reply::Handled;
        }
        if key == Key::ctrl('w') || key == Key::ctrl('u') {
            let caret = self.cursor();
            let s = &self.edit.text;
            let start = line_start(s, caret);
            let from = if caret == start {
                prev(s, caret)
            } else if key == Key::ctrl('w') {
                word_back(s, caret).max(start)
            } else {
                start
            };
            self.backspace_over(from..caret);
            return Reply::Handled;
        }
        Reply::Pass
    }

    /// Types a held `j` once its time is up; true when it did. The host calls it from `poll`.
    pub fn tick(&mut self, now: f64) -> bool {
        match self.jk {
            Some(at) if now - at > JK_SECONDS => {
                self.jk = None;
                self.type_text("j");
                true
            }
            _ => false,
        }
    }

    /// When a held `j` must be typed, for the host's wake-up.
    pub fn jk_deadline(&self) -> Option<f64> {
        self.jk.map(|at| at + JK_SECONDS)
    }

    /// `⏎` in a multi-line editor's Insert: a new line that continues a Markdown list item.
    pub fn newline(&mut self) {
        if self.lines == Lines::Single {
            return;
        }
        let caret = self.cursor();
        let s = &self.edit.text;
        let line = &s[line_start(s, caret)..caret];
        match list_marker(line) {
            Some((_, true)) => {
                let start = line_start(s, caret);
                self.backspace_over(start..caret);
            }
            Some((marker, false)) => self.type_text(&format!("\n{marker}")),
            None => self.type_text("\n"),
        }
    }

    /// Enter Insert at the caret (`i` from the host, or a composer taking the keys).
    pub fn enter_insert(&mut self) {
        let cur = self.cursor();
        self.insert_at(cur);
    }

    /// The byte range drawn as the block cursor in Normal mode.
    pub fn block(&self) -> Option<std::ops::Range<usize>> {
        let cur = self.cursor();
        match (self.mode, ch(self.text(), cur)) {
            (EditMode::Normal, Some(c)) if c != '\n' => Some(cur..next(self.text(), cur)),
            _ => None,
        }
    }

    /// The byte range Visual mode selects.
    pub fn selection(&self) -> Option<std::ops::Range<usize>> {
        let s = &self.edit.text;
        let (a, b) = (
            self.anchor.min(self.cursor()),
            self.anchor.max(self.cursor()),
        );
        match self.mode {
            EditMode::Visual => Some(a..next(s, b).max(b)),
            EditMode::VisualLine => Some(line_start(s, a)..line_end(s, b)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Motion {
    Left,
    Right,
    Down,
    Up,
    WordForward,
    WordBack,
    WordEnd,
    LineStart,
    FirstNonBlank,
    LineEnd,
    FirstLine,
    LastLine,
}

/// How a motion's span is taken by an operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Span {
    Exclusive,
    Inclusive,
    Linewise,
}

fn replace(e: &mut TextEdit, r: Range<usize>, with: &str) {
    e.set_selection(r.start, r.end);
    e.insert(with);
}

impl Editor {
    fn take_count(&mut self) -> u32 {
        self.pending.count.take().unwrap_or(1).max(1)
    }

    fn cancel(&mut self) -> Reply {
        self.pending = Pending::default();
        Reply::Handled
    }

    /// One undo step for a Normal-mode change.
    fn change(&mut self, f: impl FnOnce(&mut TextEdit)) {
        self.edit.break_history_group();
        self.edit.undo_group(f);
    }

    /// Deletes `r` as Backspace does, so the deletion joins the typing it follows in one undo
    /// step.
    /// Deletes `r` as Backspace over a selection does.
    fn backspace_over(&mut self, r: Range<usize>) {
        if r.is_empty() {
            return;
        }
        self.edit.set_selection(r.start, r.end);
        let bs = rune::KeyEvent {
            key: rune::Key::Backspace,
            mods: rune::Mods::default(),
        };
        self.edit.key(&bs, None);
    }

    fn type_text(&mut self, text: &str) {
        let ev = rune::KeyEvent {
            key: rune::Key::Text(text.to_string()),
            mods: rune::Mods::default(),
        };
        self.edit.key(&ev, None);
    }

    fn set_cursor(&mut self, at: usize) {
        self.edit.set_caret(at);
        if self.mode != EditMode::Insert {
            self.clamp();
        }
    }

    /// Normal mode's cursor sits on a character, never past a line's last one.
    fn clamp(&mut self) {
        let s = &self.edit.text;
        let cur = self.cursor().min(s.len());
        let at = if cur == line_end(s, cur) {
            last_char(s, cur)
        } else {
            cur
        };
        self.edit.set_caret(at);
    }

    fn insert_at(&mut self, at: usize) {
        self.pending = Pending::default();
        self.mode = EditMode::Insert;
        self.edit.set_caret(at);
        self.edit.break_history_group();
    }

    fn leave_insert(&mut self) {
        self.edit.break_history_group();
        self.mode = EditMode::Normal;
        let s = &self.edit.text;
        let cur = self.cursor();
        if cur > line_start(s, cur) {
            self.edit.set_caret(prev(s, cur));
        }
        self.clamp();
    }

    fn leave_visual(&mut self) {
        self.mode = EditMode::Normal;
        self.clamp();
    }

    /// Where `m` goes from the cursor, `count` times, and how an operator takes it.
    fn target(&mut self, m: Motion, count: u32, explicit: bool) -> (usize, Span) {
        let s = &self.edit.text;
        let cur = self.cursor();
        let single = self.lines == Lines::Single;
        let normal = self.pending.op.is_none();
        if !matches!(m, Motion::Down | Motion::Up) {
            self.column = None;
        }
        let mut at = cur;
        let span = match m {
            Motion::Left => {
                let start = line_start(s, cur);
                for _ in 0..count {
                    if at > start {
                        at = prev(s, at);
                    }
                }
                Span::Exclusive
            }
            Motion::Right => {
                let end = if normal {
                    last_char(s, cur)
                } else {
                    line_end(s, cur)
                };
                for _ in 0..count {
                    if at < end {
                        at = next(s, at);
                    }
                }
                Span::Exclusive
            }
            Motion::Down | Motion::Up => {
                if single {
                    return (cur, Span::Exclusive);
                }
                let col = *self.column.get_or_insert(column(s, cur));
                let line = line_index(s, cur) as i64;
                let to = if m == Motion::Down {
                    line + count as i64
                } else {
                    line - count as i64
                };
                let to = to.clamp(0, line_count(s) as i64 - 1) as usize;
                at = at_column(s, line_at(s, to), col, true);
                Span::Linewise
            }
            Motion::WordForward => {
                for _ in 0..count {
                    at = word_forward(s, at);
                }
                Span::Exclusive
            }
            Motion::WordBack => {
                for _ in 0..count {
                    at = word_back(s, at);
                }
                Span::Exclusive
            }
            Motion::WordEnd => {
                for _ in 0..count {
                    at = word_end(s, at);
                }
                Span::Inclusive
            }
            Motion::LineStart => {
                at = line_start(s, cur);
                Span::Exclusive
            }
            Motion::FirstNonBlank => {
                at = first_non_blank(s, cur);
                Span::Exclusive
            }
            Motion::LineEnd => {
                at = last_char(s, cur);
                Span::Inclusive
            }
            Motion::FirstLine | Motion::LastLine => {
                let last = line_count(s) - 1;
                let line = if explicit {
                    (count as usize).saturating_sub(1).min(last)
                } else if m == Motion::FirstLine {
                    0
                } else {
                    last
                };
                at = first_non_blank(s, line_at(s, line));
                Span::Linewise
            }
        };
        (at, span)
    }

    fn motion(&mut self, m: Motion) -> Reply {
        let explicit = self.pending.count.is_some() || self.pending.op_count.is_some();
        let count = self.pending.count.take().unwrap_or(1).max(1)
            * self.pending.op_count.take().unwrap_or(1).max(1);
        let Some(op) = self.pending.op.take() else {
            let (at, _) = self.target(m, count, explicit);
            self.set_cursor(at);
            return Reply::Handled;
        };
        let s = &self.edit.text;
        let cur = self.cursor();
        // `cw` on a word changes to its end, as `ce` does.
        let m = if op == Op::Change
            && m == Motion::WordForward
            && ch(s, cur).is_some_and(|c| !c.is_whitespace())
        {
            Motion::WordEnd
        } else {
            m
        };
        self.pending.op = Some(op);
        let (mut at, span) = self.target(m, count, explicit);
        self.pending.op = None;
        let s = &self.edit.text;
        match span {
            Span::Linewise => {
                let (a, b) = (line_index(s, cur.min(at)), line_index(s, cur.max(at)));
                self.apply_lines(op, a, b);
            }
            Span::Exclusive => {
                // `dw` on a line's last word stops at the line's end.
                if m == Motion::WordForward && line_index(s, at) > line_index(s, cur) {
                    at = line_end(s, cur);
                }
                self.apply(op, cur.min(at)..cur.max(at));
            }
            Span::Inclusive => {
                let (a, b) = (cur.min(at), cur.max(at));
                self.apply(op, a..next(s, b).min(line_end(s, b)));
            }
        }
        Reply::Handled
    }

    fn operator(&mut self, op: Op) -> Reply {
        if self.mode != EditMode::Normal {
            return self.apply_selection(op);
        }
        match self.pending.op {
            Some(pending) if pending == op => {
                self.pending.op = None;
                let n = self.pending.count.take().unwrap_or(1).max(1)
                    * self.pending.op_count.take().unwrap_or(1).max(1);
                let s = &self.edit.text;
                let first = line_index(s, self.cursor());
                let last = (first + n as usize - 1).min(line_count(s) - 1);
                self.apply_lines(op, first, last);
            }
            Some(_) => {
                self.cancel();
            }
            None => self.pending.op = Some(op),
        }
        Reply::Handled
    }

    fn object(&mut self, around: char, c: char) -> Reply {
        let Some(op) = self.pending.op.take() else {
            return self.cancel();
        };
        self.pending = Pending::default();
        let s = &self.edit.text;
        let cur = self.cursor();
        let r = match (around, c) {
            ('i', 'w') => inner_word(s, cur),
            ('a', 'w') => a_word(s, cur),
            _ => return Reply::Handled,
        };
        self.apply(op, r);
        Reply::Handled
    }

    fn apply_selection(&mut self, op: Op) -> Reply {
        let Some(r) = self.selection() else {
            return Reply::Handled;
        };
        let line = self.mode == EditMode::VisualLine;
        self.mode = EditMode::Normal;
        if line {
            let s = &self.edit.text;
            let (a, b) = (line_index(s, r.start), line_index(s, r.end));
            self.apply_lines(op, a, b);
        } else {
            self.apply(op, r);
        }
        Reply::Handled
    }

    /// An operator over lines `first..=last`.
    fn apply_lines(&mut self, op: Op, first: usize, last: usize) {
        let s = &self.edit.text;
        self.register = Register {
            text: lines_text(s, first, last),
            linewise: true,
        };
        let start = line_at(s, first);
        match op {
            Op::Yank => {}
            Op::Delete if self.lines == Lines::Single => {
                let len = s.len();
                self.change(|e| replace(e, 0..len, ""));
                self.set_cursor(0);
            }
            Op::Delete => {
                let span = lines_span(s, first, last);
                self.change(|e| replace(e, span.clone(), ""));
                let s = &self.edit.text;
                let at = span.start.min(s.len());
                let at = if at == s.len() && at > 0 && first > 0 {
                    line_start(s, prev(s, at))
                } else {
                    at
                };
                self.set_cursor(first_non_blank(self.text(), at));
            }
            Op::Change => {
                let end = line_end(s, line_at(s, last));
                self.change(|e| replace(e, start..end, ""));
                self.insert_at(start);
            }
        }
    }

    /// An operator over a span of characters.
    fn apply(&mut self, op: Op, r: Range<usize>) {
        let s = &self.edit.text;
        self.register = Register {
            text: s[r.clone()].to_string(),
            linewise: false,
        };
        match op {
            Op::Yank => self.set_cursor(r.start),
            Op::Delete => {
                self.change(|e| replace(e, r.clone(), ""));
                self.set_cursor(r.start);
            }
            Op::Change => {
                self.change(|e| replace(e, r.clone(), ""));
                self.insert_at(r.start);
            }
        }
    }

    /// `p` (after) and `P` (before), `n` times.
    fn put(&mut self, after: bool, n: u32) {
        let reg = self.register.clone();
        if reg.text.is_empty() {
            return;
        }
        let s = &self.edit.text;
        let cur = self.cursor();
        if reg.linewise && self.lines == Lines::Multi {
            let block = reg.text.repeat(n as usize);
            let end = line_end(s, cur);
            let (at, with) = if !after {
                (line_start(s, cur), block)
            } else if end < s.len() {
                (end + 1, block)
            } else {
                (end, format!("\n{}", block.trim_end_matches('\n')))
            };
            let line_at_put = if after {
                line_index(s, cur) + 1
            } else {
                line_index(s, cur)
            };
            self.change(|e| replace(e, at..at, &with));
            let s = &self.edit.text;
            self.set_cursor(first_non_blank(s, line_at(s, line_at_put)));
            return;
        }
        let piece = if reg.linewise {
            reg.text.trim_end_matches('\n').replace('\n', " ")
        } else {
            reg.text.clone()
        };
        let with = piece.repeat(n as usize);
        let at = if after && cur < line_end(s, cur) {
            next(s, cur)
        } else {
            cur
        };
        self.change(|e| replace(e, at..at, &with));
        let end = at + with.len();
        self.set_cursor(prev(self.text(), end));
    }
}

#[cfg(test)]
mod tests;
