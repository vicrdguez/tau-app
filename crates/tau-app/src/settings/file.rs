//! settings.toml as text: read into `Settings` with one sentence per problem, and written one key
//! at a time with the rest kept as the user wrote it. Pure.

use toml_edit::{DocumentMut, Item, Table, TableLike, TomlError, Value};

use super::{Edit, FILE, Field, Settings};
use crate::keys::{Key, Rebind, Refusal, parse_seq, written};
use crate::theme::{Measure, Variant, tokens};

pub(super) const HEADER: &str = "# tau settings for this device. ␣hs, :settings or ⌘, change them and save at once;\n# a hand edit here applies at the next launch.\n";

/// The settings `text` sets, every other one at its default, and a sentence for each problem.
/// Text that isn't TOML sets nothing.
pub(super) fn read(text: &str) -> (Settings, Vec<String>) {
    let mut s = Settings::default();
    let doc = match text.parse::<DocumentMut>() {
        Ok(doc) => doc,
        Err(e) => return (s, vec![not_toml(text, 0, &e)]),
    };
    let mut problems = Vec::new();
    let mut lines = Vec::new();
    for (key, item) in doc.iter() {
        match key {
            "theme" => match item.as_str().and_then(Variant::from_name) {
                Some(v) => s.theme = v,
                None => problems.push(format!(
                    "theme {} isn't ember, ember-soft or ember-light",
                    shown(item)
                )),
            },
            "reading_width" => {
                let r = &tokens().layout.reading;
                match item.as_integer() {
                    Some(px) => {
                        s.reading = Measure::new(px as f32).unwrap_or_else(|held| {
                            let (side, bound) = if held.px() > r.min {
                                ("past", r.max)
                            } else {
                                ("under", r.min)
                            };
                            problems.push(format!(
                                "reading_width {px} is {side} {bound} px, so it is {bound}"
                            ));
                            held
                        })
                    }
                    None => problems.push(format!(
                        "reading_width takes a whole number of px from {} to {}",
                        r.min, r.max
                    )),
                }
            }
            "motion" => match item.as_bool() {
                Some(on) => s.motion = on,
                None => problems.push("motion takes true or false".to_string()),
            },
            "keys" => keys(item, &mut lines, &mut problems),
            other => problems.push(format!("{other} is not a setting")),
        }
    }
    let (keymap, refused) = s.keymap.rebind_all(&lines);
    s.keymap = keymap;
    for (line, refusal) in refused {
        problems.push(match refusal {
            Refusal::NoCommand(_) | Refusal::NotItsKey { .. } => refusal.to_string(),
            _ => format!("{} {}: {refusal}", line.command, written(&line.design)),
        });
    }
    (s, problems)
}

/// `[keys."<command>"] "<design key>" = "<new key>"`, each line that parses into `lines`.
fn keys<'a>(item: &'a Item, lines: &mut Vec<Rebind<'a>>, problems: &mut Vec<String>) {
    let Some(commands) = item.as_table_like() else {
        problems.push("keys takes one table per command".to_string());
        return;
    };
    for (command, entries) in commands.iter() {
        let Some(entries) = entries.as_table_like() else {
            problems.push(format!(
                r#"{command} takes lines of "<its key>" = "<your key>""#
            ));
            continue;
        };
        let mut seen = std::collections::HashSet::new();
        for (spelling, to) in entries.iter() {
            let Ok(design) = parse_seq(spelling) else {
                problems.push(format!(r#"{command}: "{spelling}" isn't a key"#));
                continue;
            };
            let at = format!("{command} {}", written(&design));
            if !seen.insert(design.clone()) {
                problems.push(format!(
                    "{at} is already set in this table; ignoring \"{spelling}\""
                ));
                continue;
            }
            match to.as_str().map(|to| (to, parse_seq(to))) {
                Some((_, Ok(to))) => lines.push(Rebind {
                    command,
                    design,
                    to,
                }),
                Some((to, Err(_))) => problems.push(format!(r#"{at}: "{to}" isn't a key"#)),
                None => problems.push(format!("{at} takes a key in quotes")),
            }
        }
    }
}

/// `text` with `edit` set from `new`; everything else stays as written. The comments that open
/// the file stay at its top (the header, in a new file), above any setting a save adds.
/// `Err` when `text` isn't TOML or `keys` isn't a table, so it is never written over.
pub(super) fn write(text: &str, new: &Settings, edit: &Edit) -> Result<String, String> {
    let whole = if text.trim().is_empty() { HEADER } else { text };
    let (lead, rest) = whole.split_at(lead(whole));
    let mut doc = rest
        .parse::<DocumentMut>()
        .map_err(|e| not_toml(whole, lead.len(), &e))?;
    let fresh = doc.is_empty();
    match edit {
        Edit::Field(field) => set(doc.as_table_mut(), field.key(), value(*field, new)),
        Edit::Key { command, design } => {
            let now = new
                .keymap
                .key_rows()
                .into_iter()
                .find(|r| r.command.id == *command && r.design == *design)
                .map_or_else(|| design.clone(), |r| r.now);
            set_key(&mut doc, command, design, &now)?;
        }
    }
    let out = doc.to_string();
    Ok(match (fresh, doc.is_empty()) {
        (true, true) => text.to_string(),
        (true, false) => format!("{}\n\n{out}", lead.trim_end()),
        (false, _) => format!("{lead}{out}"),
    })
}

/// `[keys."<command>"]` gives `design` as `now`; the line goes when `now` is `design`, and the
/// command's table goes when that leaves it empty.
fn set_key(
    doc: &mut DocumentMut,
    command: &str,
    design: &[Key],
    now: &[Key],
) -> Result<(), String> {
    let not_a_table = |what: &str| format!("{FILE}: {what} isn't a table, so the key isn't saved");
    if now == design {
        let Some(commands) = doc.get_mut("keys") else {
            return Ok(());
        };
        let commands = commands
            .as_table_like_mut()
            .ok_or_else(|| not_a_table("keys"))?;
        let Some(entries) = commands.get_mut(command).and_then(Item::as_table_like_mut) else {
            return Ok(());
        };
        while let Some(at) = spelled(entries, design) {
            entries.remove(&at);
        }
        if entries.is_empty() {
            commands.remove(command);
        }
        return Ok(());
    }
    let commands = doc.entry("keys").or_insert_with(|| {
        let mut t = Table::new();
        t.set_implicit(true);
        Item::Table(t)
    });
    let inline = commands.is_inline_table();
    let commands = commands
        .as_table_like_mut()
        .ok_or_else(|| not_a_table("keys"))?;
    let entries = commands
        .entry(command)
        .or_insert_with(|| {
            if inline {
                Item::Value(Value::InlineTable(toml_edit::InlineTable::new()))
            } else {
                Item::Table(Table::new())
            }
        })
        .as_table_like_mut()
        .ok_or_else(|| not_a_table(&format!("keys.\"{command}\"")))?;
    let at = spelled(entries, design).unwrap_or_else(|| written(design));
    set(entries, &at, written(now).into());
    Ok(())
}

/// How far the comments that open `text` reach, through their last comment line.
fn lead(text: &str) -> usize {
    let (mut at, mut end) = (0, 0);
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if !t.is_empty() && !t.starts_with('#') {
            break;
        }
        at += line.len();
        if t.starts_with('#') {
            end = at;
        }
    }
    end
}

/// The key `entries` writes `design` as, however the user spelled it.
fn spelled(entries: &dyn TableLike, design: &[Key]) -> Option<String> {
    entries
        .iter()
        .map(|(k, _)| k)
        .find(|k| parse_seq(k).is_ok_and(|k| k == design))
        .map(str::to_string)
}

/// `key = value`, keeping the comment and spacing around a value already there.
fn set(table: &mut dyn TableLike, key: &str, value: Value) {
    match table.get_mut(key).and_then(Item::as_value_mut) {
        Some(old) => {
            let decor = old.decor().clone();
            *old = value;
            *old.decor_mut() = decor;
        }
        None => {
            table.insert(key, Item::Value(value));
        }
    }
}

fn value(field: Field, s: &Settings) -> Value {
    match field {
        Field::Theme => s.theme.name().into(),
        Field::Reading => (s.reading.px() as i64).into(),
        Field::Motion => s.motion.into(),
    }
}

fn shown(item: &Item) -> String {
    item.as_value().map_or_else(
        || "a table".to_string(),
        |v| v.to_string().trim().to_string(),
    )
}

/// `e` from parsing `text[from..]`, as one sentence naming its line in `text`.
fn not_toml(text: &str, from: usize, e: &TomlError) -> String {
    let at = e
        .span()
        .map_or(from, |span| (from + span.start).min(text.len()));
    let line = text[..at].matches('\n').count() + 1;
    let why = e.message().lines().next().unwrap_or("not TOML");
    format!("{FILE} line {line}: {why}")
}
