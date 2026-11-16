//! Chord operations, applied generically to a JSON value (`docs/protocol.md` "Streams").
//!
//! `["r", value]` replace · `["s", path, value]` set · `["d", path]` delete · `["a", path, text]` append text ·
//! `["t", path, n]` drop the first n UTF-16 code units · `["p", path, index, remove, items]` splice ·
//! `["m", path, order]` reorder so `new[i] = old[order[i]]`.

use serde_json::Value;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum OpError {
    /// Not an op this client knows, or not of its documented shape.
    Shape(String),
    /// `__proto__`, `constructor` or `prototype` in a path, or a segment that is neither a key nor an index.
    UnsafePath(String),
    /// The path names nothing that can take the op.
    Path(Vec<Value>),
}

impl fmt::Display for OpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpError::Shape(what) => write!(f, "bad op: {what}"),
            OpError::UnsafePath(seg) => write!(f, "refused path segment {seg}"),
            OpError::Path(path) => write!(f, "unresolvable path {}", Value::Array(path.clone())),
        }
    }
}

/// Applies `ops` in order. On an error the value may be half-changed; the caller drops it and reopens the stream.
pub fn apply(target: &mut Value, ops: &[Value]) -> Result<(), OpError> {
    for op in ops {
        apply_one(target, op)?;
    }
    Ok(())
}

const RESERVED: [&str; 3] = ["__proto__", "constructor", "prototype"];

enum Seg<'a> {
    Key(&'a str),
    Index(usize),
}

fn shape(op: &Value) -> OpError {
    OpError::Shape(op.to_string())
}

fn apply_one(root: &mut Value, op: &Value) -> Result<(), OpError> {
    let parts = op.as_array().ok_or_else(|| shape(op))?;
    let verb = parts
        .first()
        .and_then(Value::as_str)
        .ok_or_else(|| shape(op))?;
    let arity = match verb {
        "r" | "d" => 2,
        "s" | "a" | "t" | "m" => 3,
        "p" => 5,
        _ => return Err(shape(op)),
    };
    if parts.len() != arity {
        return Err(shape(op));
    }
    if verb == "r" {
        *root = parts[1].clone();
        return Ok(());
    }
    let raw_path = parts[1].as_array().ok_or_else(|| shape(op))?;
    let path = segments(raw_path)?;
    match verb {
        "p" => {
            let index = whole(&parts[2]).ok_or_else(|| shape(op))?;
            let remove = whole(&parts[3]).ok_or_else(|| shape(op))?;
            let items = parts[4].as_array().ok_or_else(|| shape(op))?;
            let list = resolve(root, &path)
                .and_then(Value::as_array_mut)
                .ok_or_else(|| OpError::Path(raw_path.clone()))?;
            if index > list.len() {
                return Err(OpError::Path(raw_path.clone()));
            }
            let end = (index + remove).min(list.len());
            list.splice(index..end, items.iter().cloned());
            Ok(())
        }
        "m" => {
            let order = parts[2].as_array().ok_or_else(|| shape(op))?;
            let order: Vec<usize> = order
                .iter()
                .map(whole)
                .collect::<Option<_>>()
                .ok_or_else(|| shape(op))?;
            let list = resolve(root, &path)
                .and_then(Value::as_array_mut)
                .ok_or_else(|| OpError::Path(raw_path.clone()))?;
            let mut seen = vec![false; list.len()];
            let permutation = order.len() == list.len()
                && order
                    .iter()
                    .all(|&i| i < seen.len() && !std::mem::replace(&mut seen[i], true));
            if !permutation {
                return Err(OpError::Path(raw_path.clone()));
            }
            let old = std::mem::take(list);
            *list = order.iter().map(|&i| old[i].clone()).collect();
            Ok(())
        }
        _ => {
            let (last, parent_path) = path.split_last().ok_or_else(|| shape(op))?;
            let parent =
                resolve(root, parent_path).ok_or_else(|| OpError::Path(raw_path.clone()))?;
            let arg = parts.get(2).unwrap_or(&Value::Null);
            edit(parent, last, verb, arg).ok_or_else(|| OpError::Path(raw_path.clone()))
        }
    }
}

/// `s d a t` on the last segment. `None` when the parent can't take the op there.
fn edit(parent: &mut Value, seg: &Seg, verb: &str, arg: &Value) -> Option<()> {
    let slot = match (parent, seg) {
        (Value::Object(map), Seg::Key(key)) => match verb {
            "s" => {
                map.insert((*key).to_owned(), arg.clone());
                return Some(());
            }
            "d" => {
                map.shift_remove(*key);
                return Some(());
            }
            _ => map.get_mut(*key)?,
        },
        (Value::Array(list), Seg::Index(i)) => match verb {
            "s" if *i == list.len() => {
                list.push(arg.clone());
                return Some(());
            }
            "d" if *i < list.len() => {
                list.remove(*i);
                return Some(());
            }
            _ => list.get_mut(*i)?,
        },
        _ => return None,
    };
    match verb {
        "s" => *slot = arg.clone(),
        "a" => slot_text(slot)?.push_str(arg.as_str()?),
        "t" => {
            let text = slot_text(slot)?;
            let drop = byte_offset_of_utf16(text, whole(arg)?);
            text.replace_range(..drop, "");
        }
        _ => return None,
    }
    Some(())
}

fn slot_text(slot: &mut Value) -> Option<&mut String> {
    match slot {
        Value::String(text) => Some(text),
        _ => None,
    }
}

/// The byte offset after the first `units` UTF-16 code units, or the end. A cut inside a surrogate pair keeps the
/// character whole.
fn byte_offset_of_utf16(text: &str, units: usize) -> usize {
    let mut seen = 0;
    for (at, c) in text.char_indices() {
        if seen >= units {
            return at;
        }
        seen += c.len_utf16();
    }
    text.len()
}

fn whole(v: &Value) -> Option<usize> {
    v.as_u64().map(|n| n as usize)
}

fn segments(path: &[Value]) -> Result<Vec<Seg<'_>>, OpError> {
    path.iter()
        .map(|seg| match seg {
            Value::String(key) if RESERVED.contains(&key.as_str()) => {
                Err(OpError::UnsafePath(key.clone()))
            }
            Value::String(key) => Ok(Seg::Key(key)),
            other => whole(other)
                .map(Seg::Index)
                .ok_or_else(|| OpError::UnsafePath(other.to_string())),
        })
        .collect()
}

fn resolve<'v>(mut at: &'v mut Value, path: &[Seg]) -> Option<&'v mut Value> {
    for seg in path {
        at = match (at, seg) {
            (Value::Object(map), Seg::Key(key)) => map.get_mut(*key)?,
            (Value::Array(list), Seg::Index(i)) => list.get_mut(*i)?,
            _ => return None,
        };
    }
    Some(at)
}
