//! What commands do: tables from a keymap command id to its handler. A command runs the handler of
//! the screen family that has the keys (`crate::screens::holder`), else the global one: the
//! frame's and the floats' here, and each family's `globals`. A command with no handler says so in
//! the echo.

use std::collections::HashMap;
use std::sync::LazyLock;

use rune::Tx;

use super::input::keep_cursor;
use super::{App, Focus, Stage};
use crate::floats::{self, CmdLine, Float, Picker, PickerKind, Sheet};
use crate::frame::{Place, Row};
use crate::keys::{Code, Command, Key};
use crate::screens::{self, ScreenId};
use crate::settings::{self, Edit, Field};

/// A command as the keys (or the palette, or `:`) ran it.
pub struct Invocation<'k> {
    pub command: &'k Command,
    pub count: Option<u32>,
    /// Whether it applies where the keys are; a handler acts only when it does.
    pub applies: bool,
    /// The keys that ran it; empty from the palette and the `:` line.
    pub keys: Vec<Key>,
}

pub type Handler = fn(&mut App, &Invocation, &mut Tx);

/// The frame's handlers and the frame floats', which run wherever the keys are.
fn frame() -> HashMap<&'static str, Handler> {
    let mut t: HashMap<&'static str, Handler> = HashMap::new();
    // The frame.
    t.insert("sidebar.focus", |a, _, _| focus_sidebar(a));
    t.insert("sidebar.leave", |a, _, _| a.focus = Focus::Stage);
    t.insert("float.close", |a, _, _| a.focus = Focus::Stage);
    t.insert("toggle.sidebar", toggle_sidebar);
    t.insert("toggle.details", |a, inv, _| {
        if inv.applies {
            a.details_visible = !a.details_visible;
        }
    });
    t.insert("help.theme", |a, _, tx| {
        let next = Field::Theme.step(&a.settings, 1);
        settings::change(a, next, Edit::Field(Field::Theme));
        tx.instant();
    });
    t.insert("open.home", |a, _, _| a.show(Stage::Place(Place::Home)));
    t.insert("open.conversations", |a, _, _| {
        a.show(Stage::Place(Place::Conversations))
    });
    t.insert("open.timeline", |a, _, _| {
        a.show(Stage::Place(Place::Timeline))
    });
    t.insert("open.faults", |a, _, _| a.show(Stage::Place(Place::Faults)));
    t.insert("open.projects", |a, _, _| {
        a.show(Stage::Place(Place::Projects))
    });
    t.insert("item.next", item_next);
    t.insert("view.close", view_close);
    t.insert("app.quit", |a, _, tx| {
        super::link::on_close(a, tx);
        tx.quit();
    });
    t.insert("list.open", list_open);
    t.insert("group.next", group_next);
    // The floats.
    t.insert("palette", |a, _, _| a.open_float(Float::palette()));
    t.insert("cmdline.open", |a, _, _| a.open_float(Float::cmdline()));
    t.insert("open.messages", |a, _, _| a.open_float(Float::messages()));
    t.insert("help.keys", |a, _, _| {
        a.open_float(Float::Sheet(Sheet::keys(a.settings.keymap)))
    });
    t.insert("picker.keys", picker_keys);
    t.insert("confirm.keys", confirm_keys);
    t.extend(crate::nav::handlers());
    t.extend(crate::yank::handlers());
    t
}

struct Tables {
    /// Wherever the keys are: the frame's, then each family's `globals`.
    global: HashMap<&'static str, Handler>,
    /// While a family has the keys: its `commands`.
    held: HashMap<(ScreenId, &'static str), Handler>,
}

fn tables() -> Tables {
    let mut global = frame();
    let mut held = HashMap::new();
    for id in screens::ALL {
        let screen = screens::get(id);
        for (cmd, h) in screen.globals() {
            let taken = global.insert(cmd, h).is_some();
            assert!(!taken, "two global handlers for {cmd}");
        }
        for (cmd, h) in screen.commands() {
            held.insert((id, cmd), h);
        }
    }
    Tables { global, held }
}

static TABLES: LazyLock<Tables> = LazyLock::new(tables);

/// The ids that have a handler anywhere.
pub fn handled() -> impl Iterator<Item = &'static str> {
    TABLES
        .global
        .keys()
        .copied()
        .chain(TABLES.held.keys().map(|(_, id)| *id))
}

pub fn run(app: &mut App, inv: &Invocation, tx: &mut Tx) {
    let id = inv.command.id.as_str();
    let held = screens::holder(app).and_then(|s| TABLES.held.get(&(s, id)));
    match held.or_else(|| TABLES.global.get(id)) {
        Some(h) => {
            let from = inv.command.jump.then(|| crate::nav::here(app));
            h(app, inv, tx);
            if let Some(from) = from {
                crate::nav::left(app, from);
            }
        }
        None => app.say(format!("{} isn't built yet", inv.command.desc)),
    }
}

fn focus_sidebar(a: &mut App) {
    a.focus = Focus::Sidebar;
    a.sidebar_cursor = a.current_row();
}

/// `␣tb`: flips the stored choice; under 1000 px it does what `C-h` does.
fn toggle_sidebar(a: &mut App, _: &Invocation, _: &mut Tx) {
    if !a.sidebar_room() {
        focus_sidebar(a);
        return;
    }
    a.sidebar_visible = !a.sidebar_visible;
    if !a.sidebar_visible {
        a.focus = Focus::Stage;
    }
}

/// `]b [b`: the next or previous open item, counting from the one shown last.
fn item_next(a: &mut App, inv: &Invocation, _: &mut Tx) {
    let back = inv.keys.first() == Some(&Key::char('['));
    let by = i64::from(inv.count.unwrap_or(1)) * if back { -1 } else { 1 };
    let from = match &a.stage {
        Stage::Item(i) => Some(i.clone()),
        _ => a.last_item.clone(),
    };
    if let Some(item) = a.open.step(from.as_ref(), by) {
        a.show(Stage::Item(item));
    }
}

/// `x` in the sidebar closes the item under its cursor; `:q` the one the stage shows.
fn view_close(a: &mut App, inv: &Invocation, _: &mut Tx) {
    let in_sidebar = a.focus == Focus::Sidebar && !inv.keys.is_empty();
    let item = if in_sidebar {
        match &a.sidebar_cursor {
            Row::Item(i) => i.clone(),
            _ => return,
        }
    } else {
        match &a.stage {
            Stage::Item(i) => i.clone(),
            _ => return,
        }
    };
    let (rows, _) = a.sidebar_rows();
    let was = rows
        .iter()
        .position(|r| *r == a.sidebar_cursor)
        .unwrap_or(0);
    let next = a.open.close(&item);
    if a.last_item.as_ref() == Some(&item) {
        a.last_item = None;
    }
    if a.stage == Stage::Item(item) {
        let focus = a.focus;
        a.show(next.map_or(Stage::Place(Place::Home), Stage::Item));
        a.focus = focus;
    }
    keep_cursor(a, was);
}

/// `⏎` and `l` in the sidebar: open the row and give the stage the keys.
fn list_open(a: &mut App, _: &Invocation, tx: &mut Tx) {
    if a.focus == Focus::Sidebar {
        open_row(a, &a.sidebar_cursor.clone(), tx);
    }
}

/// Opens a sidebar row, from its keys or a click: a place or an item shows in the stage, which
/// takes the keys; `+ New`, `Find` and `Server` run their commands.
pub fn open_row(a: &mut App, row: &Row, tx: &mut Tx) {
    if !a.floats.is_empty() {
        return;
    }
    tx.instant();
    a.focus = Focus::Stage;
    a.sidebar_cursor = row.clone();
    match row {
        Row::New => run_id(a, "conv.new", tx),
        Row::Find => run_id(a, "find.object", tx),
        Row::Server => run_id(a, "open.server", tx),
        Row::Count(crate::ui::Tag::Needs) => run_id(a, "next.decision", tx),
        Row::Count(crate::ui::Tag::Failed) => screens::home::open_failed(a),
        Row::Count(crate::ui::Tag::Unknown) => run_id(a, "recover.outbox", tx),
        Row::Place(p) => a.show(Stage::Place(*p)),
        Row::Item(i) => a.show(Stage::Item(i.clone())),
    }
}

/// Runs the command `id` as the palette would.
pub fn run_id(a: &mut App, id: &str, tx: &mut Tx) {
    if let Some(command) = a.settings.keymap.command(id) {
        let applies = a.settings.keymap.applies(command, a.scope().ctx);
        run(
            a,
            &Invocation {
                command,
                count: None,
                applies,
                keys: Vec::new(),
            },
            tx,
        );
    }
}

/// `]g [g` in the sidebar: the next or previous section.
fn group_next(a: &mut App, inv: &Invocation, _: &mut Tx) {
    if a.focus != Focus::Sidebar {
        return;
    }
    let (rows, sections) = a.sidebar_rows();
    let at = rows
        .iter()
        .position(|r| *r == a.sidebar_cursor)
        .unwrap_or(0);
    let back = inv.keys.first() == Some(&Key::char('['));
    let mut to = at;
    for _ in 0..inv.count.unwrap_or(1) {
        to = if back {
            sections
                .iter()
                .rev()
                .copied()
                .find(|s| *s < to)
                .unwrap_or(0)
        } else {
            sections.iter().copied().find(|s| *s > to).unwrap_or(to)
        };
    }
    a.sidebar_cursor = rows[to].clone();
}

/// A picker's keys, and the `:` line's: move, choose, complete, cancel.
fn picker_keys(a: &mut App, inv: &Invocation, tx: &mut Tx) {
    let Some(&key) = inv.keys.first() else { return };
    if key == Key::plain(Code::Esc) || key == Key::ctrl('c') {
        a.close_float();
        return;
    }
    if let Some(step) = floats::cursor_step(key) {
        let len = match a.floats.last() {
            Some((Float::Picker(p), _)) if p.kind == PickerKind::Palette => {
                floats::palette_rows(a.settings.keymap, &p.query.text).len()
            }
            Some((Float::Picker(p), _)) => floats::message_rows(&a.messages, &p.query.text).len(),
            Some((Float::CmdLine(c), _)) => {
                floats::completions(a.settings.keymap, &c.line.text).len()
            }
            Some((Float::Sheet(s), _)) => s.shown().len(),
            _ => 0,
        };
        if let Some((
            Float::Picker(Picker { cursor, .. })
            | Float::CmdLine(CmdLine { cursor, .. })
            | Float::Sheet(Sheet { cursor, .. }),
            _,
        )) = a.floats.last_mut()
        {
            *cursor = (*cursor as i64 + step).clamp(0, len.saturating_sub(1) as i64) as usize;
        }
        return;
    }
    if key == Key::plain(Code::Tab) {
        if let Some((Float::CmdLine(c), _)) = a.floats.last_mut()
            && let Some(cmd) = floats::completions(a.settings.keymap, &c.line.text).get(c.cursor)
        {
            c.line.replace_text(cmd.ex.clone());
        }
        return;
    }
    if key != Key::plain(Code::Enter) {
        return;
    }
    match a.close_float() {
        Some(Float::Picker(p)) if p.kind == PickerKind::Palette => {
            if let Some(command) =
                floats::palette_rows(a.settings.keymap, &p.query.text).get(p.cursor)
            {
                let applies = a.settings.keymap.applies(command, a.scope().ctx);
                run(
                    a,
                    &Invocation {
                        command,
                        count: None,
                        applies,
                        keys: Vec::new(),
                    },
                    tx,
                );
            }
        }
        Some(Float::CmdLine(c)) => run_ex(a, c.line.text.trim(), tx),
        Some(Float::Sheet(s)) => {
            if let (Some(choose), Some(&i)) = (s.choose, s.shown().get(s.cursor)) {
                choose(a, i);
            }
        }
        _ => {}
    }
}

/// Runs a `:` line: its first word is a command's `:` name.
fn run_ex(a: &mut App, line: &str, tx: &mut Tx) {
    if line.is_empty() {
        return;
    }
    if a.history.last().map(String::as_str) != Some(line) {
        a.history.push(line.to_string());
    }
    let name = line.split_whitespace().next().unwrap_or(line);
    match a.settings.keymap.by_ex(name) {
        Some(command) => {
            let applies = a.settings.keymap.applies(command, a.scope().ctx);
            run(
                a,
                &Invocation {
                    command,
                    count: None,
                    applies,
                    keys: Vec::new(),
                },
                tx,
            );
        }
        None => a.say(format!("not a command: :{name}")),
    }
}

/// Confirm's keys: `y` runs its step, `n Esc q` cancel.
fn confirm_keys(a: &mut App, inv: &Invocation, tx: &mut Tx) {
    let yes = inv.keys.first() == Some(&Key::char('y'));
    if let Some(Float::Confirm(c)) = a.close_float()
        && yes
        && let Some(step) = c.on_yes
    {
        step(a, tx);
    }
}
