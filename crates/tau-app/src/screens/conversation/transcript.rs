//! What a conversation's transcript draws, from what the server holds (`spec.md` §5.5): its items
//! top to bottom, and the stops the cursor rests on. Pure: the view draws [`Item`]s, the keys walk
//! [`stops`].
//!
//! Provenance comes from structure only (`visual-language.md` §8): an input only from `pi.user`,
//! an agent header only from `pi.assistant`, a tool row's outcome only from `pi.tool-result`'s
//! `isError` and `pi.live.tools`, never from what the output says.

use std::collections::HashMap;

use serde_json::Value;
use tau_client::model::Transcript as Held;
use tau_client::shapes::{
    AssistantMessage, Block as Content, Content as UserContent, ConversationState, EntryBody, Live,
};
use tau_client::{EntryId, TaskId};

/// Where the cursor can rest. Each names the object it stands on, so it survives new frames
/// (`grammar.md` §7).
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Stop {
    /// A message's or a record's header line.
    Head(EntryId),
    /// One top-level Markdown block of a message body: the entry, its content block, the block.
    Text(EntryId, usize, usize),
    Thinking(EntryId, usize),
    Tool(String),
    /// Adjacent `ok` rows folded into one, named by the first call.
    Fold(String),
    /// `pi.system`, `pi.reset`, `pi.compaction`.
    Chrome(EntryId),
}

impl Stop {
    /// A message's start, for `]m [m`.
    pub fn is_head(&self) -> bool {
        matches!(self, Stop::Head(_))
    }

    /// A tool row or a fold, for `]t [t`.
    pub fn is_tool(&self) -> bool {
        matches!(self, Stop::Tool(_) | Stop::Fold(_))
    }

    /// The rune key its element carries, for scrolling to it.
    pub fn key(&self) -> String {
        match self {
            Stop::Head(e) => format!("conv-head-{e}"),
            Stop::Text(e, c, b) => format!("conv-text-{e}-{c}-{b}"),
            Stop::Thinking(e, c) => format!("conv-thinking-{e}-{c}"),
            Stop::Tool(c) => format!("conv-tool-{c}"),
            Stop::Fold(c) => format!("conv-fold-{c}"),
            Stop::Chrome(e) => format!("conv-chrome-{e}"),
        }
    }
}

/// One top-level Markdown block of a message body: a cursor stop.
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub stop: Stop,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Text(Text),
    /// An image block of an input, never loaded: its MIME type.
    Image(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    Failed,
    Running,
    NoResult,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolRow {
    pub call: String,
    pub name: String,
    /// The arguments as one line: a command, a query, a path, else the JSON.
    pub args: String,
    /// The full arguments when the summary loses fields or lines, or exceeds the row width.
    pub input: Option<String>,
    pub outcome: Outcome,
    /// The `timestamp` of the message that made the call, for a running row's count (D12).
    pub started: Option<i64>,
    /// The result's text, or a running slot's output so far.
    pub output: String,
    pub open: bool,
}

impl ToolRow {
    /// The output an opened row shows: a running row its last lines, a finished one all of it.
    pub fn shown_output(&self) -> Option<String> {
        if !self.open {
            return None;
        }
        let text = self.output.strip_suffix('\n').unwrap_or(&self.output);
        Some(if self.outcome == Outcome::Running {
            let lines: Vec<&str> = text.lines().collect();
            lines[lines.len().saturating_sub(RUNNING_LINES)..].join("\n")
        } else {
            text.to_string()
        })
    }
}

/// How many output lines a running row shows (`tokens.json` `layout.tool_row`).
const RUNNING_LINES: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Text(Text),
    Thinking {
        stop: Stop,
        words: usize,
        text: String,
        open: bool,
    },
    Tool(ToolRow),
    Fold {
        stop: Stop,
        rows: Vec<ToolRow>,
        open: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// Your message (`pi.user`), whoever sent it (D3).
    Input {
        head: EntryId,
        at: Option<i64>,
        body: Vec<Body>,
    },
    /// One quiet line: the tools a `pi.system` added, a context reset, a compaction; `text` is
    /// what `za` opens.
    Chrome {
        stop: Stop,
        words: String,
        text: Option<String>,
        open: bool,
    },
    /// `fetching entries missed while away…` (`client.md` §3.3).
    Gap,
    /// Consecutive agent entries under one header.
    Turn {
        head: EntryId,
        at: Option<i64>,
        model: String,
        parts: Vec<Part>,
    },
    /// An entry of any other kind; `body` is `data.text` when it is a string.
    Record {
        head: EntryId,
        kind: String,
        by_task: Option<TaskId>,
        body: Option<Vec<Text>>,
    },
    /// `pi.live.generation`: the message being written.
    Writing {
        model: String,
        text: String,
    },
    Failure {
        reason: String,
        detail: Option<String>,
    },
    Stopped,
}

/// The user's `za zo zc` on rows and folds: open or closed against each one's default.
pub type Folds = HashMap<Stop, bool>;

/// The transcript's items, top to bottom.
pub fn build(held: &Held, folds: &Folds) -> Vec<Item> {
    let live = held.docs.value().map(|d| &d.live);
    let run_live = live.is_some_and(|l| l.run.is_some());
    let mut results: HashMap<&str, (bool, &str)> = HashMap::new();
    for e in held.entries.values() {
        if let EntryBody::ToolResult {
            call_id,
            is_error,
            text,
            ..
        } = &e.body
        {
            results.insert(call_id, (*is_error, text));
        }
    }
    let slots: HashMap<&str, &str> = live
        .map(|l| {
            l.tools
                .iter()
                .map(|t| (t.call_id.as_str(), t.output.as_str()))
                .collect()
        })
        .unwrap_or_default();
    let tool = |call: &str, name: &str, args: &Value, started: Option<i64>| {
        let (outcome, output) = match (slots.get(call), results.get(call)) {
            (Some(out), _) => (Outcome::Running, out.to_string()),
            (None, Some((true, text))) => (Outcome::Failed, text.to_string()),
            (None, Some((false, text))) => (Outcome::Ok, text.to_string()),
            (None, None) if run_live => (Outcome::Running, String::new()),
            (None, None) => (Outcome::NoResult, String::new()),
        };
        let stop = Stop::Tool(call.to_string());
        let open = folds
            .get(&stop)
            .copied()
            .unwrap_or(outcome == Outcome::Running);
        let (args, input) = summary(args);
        ToolRow {
            call: call.to_string(),
            name: name.to_string(),
            args,
            input,
            outcome,
            started,
            output,
            open,
        }
    };

    let mut items = Vec::new();
    let mut in_turn = false;
    let mut gap = held.gap.clone();
    for (id, e) in &held.entries {
        if gap.as_ref().is_some_and(|g| *id > g.max) {
            items.push(Item::Gap);
            gap = None;
        }
        let chrome = |words: String, text: Option<String>| {
            let stop = Stop::Chrome(*id);
            let open = text.is_some() && folds.get(&stop).copied().unwrap_or(false);
            Item::Chrome {
                stop,
                words,
                text,
                open,
            }
        };
        match &e.body {
            EntryBody::ToolResult { .. } => continue,
            EntryBody::Assistant(m) => {
                if !in_turn {
                    items.push(Item::Turn {
                        head: *id,
                        at: m.timestamp,
                        model: m.model.clone(),
                        parts: Vec::new(),
                    });
                    in_turn = true;
                }
                if let Some(Item::Turn { parts, .. }) = items.last_mut() {
                    parts.extend(message_parts(*id, m, folds, &tool));
                }
                continue;
            }
            EntryBody::User { content, timestamp } => items.push(Item::Input {
                head: *id,
                at: *timestamp,
                body: content
                    .iter()
                    .enumerate()
                    .flat_map(|(c, part)| match part {
                        UserContent::Text(s) => {
                            texts(*id, c, s).into_iter().map(Body::Text).collect()
                        }
                        UserContent::Image { mime_type } => vec![Body::Image(mime_type.clone())],
                    })
                    .collect(),
            }),
            EntryBody::System { added, .. } => {
                items.push(chrome(format!("tools: {}", added.join(", ")), None))
            }
            EntryBody::Reset { handoff } => {
                items.push(chrome("context reset".into(), handoff.clone()))
            }
            EntryBody::Compaction { summary } => {
                items.push(chrome("earlier context summarised".into(), summary.clone()))
            }
            EntryBody::Record { text } => items.push(Item::Record {
                head: *id,
                kind: e.kind.clone(),
                by_task: e.by_task,
                body: text.as_ref().map(|t| texts(*id, 0, t)),
            }),
        }
        in_turn = false;
    }
    if gap.is_some() {
        items.push(Item::Gap);
    }
    for item in &mut items {
        if let Item::Turn { parts, .. } = item {
            fold_ok_rows(parts, folds);
        }
    }
    if let Some(live) = live {
        items.extend(tail(live));
    }
    items
}

fn message_parts(
    id: EntryId,
    m: &AssistantMessage,
    folds: &Folds,
    tool: &impl Fn(&str, &str, &Value, Option<i64>) -> ToolRow,
) -> Vec<Part> {
    let mut parts = Vec::new();
    for (c, block) in m.blocks.iter().enumerate() {
        match block {
            Content::Text(s) => parts.extend(texts(id, c, s).into_iter().map(Part::Text)),
            Content::Thinking(s) => {
                let stop = Stop::Thinking(id, c);
                parts.push(Part::Thinking {
                    open: folds.get(&stop).copied().unwrap_or(false),
                    stop,
                    words: s.split_whitespace().count(),
                    text: s.clone(),
                });
            }
            Content::ToolCall {
                id: call,
                name,
                arguments,
            } => parts.push(Part::Tool(tool(call, name, arguments, m.timestamp))),
        }
    }
    parts
}

/// Folds every run of two or more adjacent `ok` rows into one.
fn fold_ok_rows(parts: &mut Vec<Part>, folds: &Folds) {
    let is_ok = |p: &Part| matches!(p, Part::Tool(r) if r.outcome == Outcome::Ok);
    let mut out = Vec::with_capacity(parts.len());
    let mut rest = std::mem::take(parts).into_iter().peekable();
    while let Some(p) = rest.next() {
        if !is_ok(&p) || !rest.peek().is_some_and(is_ok) {
            out.push(p);
            continue;
        }
        let mut rows = vec![p];
        while rest.peek().is_some_and(is_ok) {
            rows.extend(rest.next());
        }
        let rows: Vec<ToolRow> = rows
            .into_iter()
            .filter_map(|p| match p {
                Part::Tool(r) => Some(r),
                _ => None,
            })
            .collect();
        let stop = Stop::Fold(rows[0].call.clone());
        out.push(Part::Fold {
            open: folds.get(&stop).copied().unwrap_or(false),
            stop,
            rows,
        });
    }
    *parts = out;
}

/// What follows the entries: the message being written, then a run failure or a stop.
fn tail(live: &Live) -> Vec<Item> {
    let mut out = Vec::new();
    if let Some(m) = live.generation.as_ref().and_then(|g| g.message.as_ref()) {
        let text: Vec<&str> = m
            .blocks
            .iter()
            .filter_map(|b| match b {
                Content::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        out.push(Item::Writing {
            model: m.model.clone(),
            text: text.join("\n\n"),
        });
    }
    match (live.state(), &live.run_failure) {
        (ConversationState::Failed { reason }, Some(f)) => out.push(Item::Failure {
            reason,
            detail: f.detail.clone(),
        }),
        (ConversationState::Stopped, _) => out.push(Item::Stopped),
        _ => {}
    }
    out
}

fn texts(id: EntryId, content: usize, source: &str) -> Vec<Text> {
    blocks(source)
        .into_iter()
        .enumerate()
        .map(|(b, source)| Text {
            stop: Stop::Text(id, content, b),
            source: source.to_string(),
        })
        .collect()
}

// The narrowest tool row shows about this many characters.
const SUMMARY_CHARS: usize = 48;

/// A tool call's one-line summary and the input it leaves out.
fn summary(args: &Value) -> (String, Option<String>) {
    let field = ["command", "query", "path", "text"]
        .into_iter()
        .find_map(|key| args.get(key)?.as_str().map(|value| (key, value)));
    let pretty = || serde_json::to_string_pretty(args).unwrap_or_else(|_| args.to_string());
    match field {
        Some((key, value)) => {
            let first = value.lines().next().unwrap_or_default();
            let summary = if key == "query" {
                format!("\"{first}\"")
            } else {
                first.to_string()
            };
            let input = if args.as_object().is_some_and(|o| o.len() > 1) {
                Some(pretty())
            } else if value.lines().count() > 1 || summary.chars().count() > SUMMARY_CHARS {
                Some(value.to_string())
            } else {
                None
            };
            (summary, input)
        }
        None => {
            let summary = if args.is_null() {
                String::new()
            } else {
                args.to_string()
            };
            let input = (args.as_object().is_some_and(|o| !o.is_empty())
                || summary.chars().count() > SUMMARY_CHARS)
                .then(pretty);
            (summary, input)
        }
    }
}

/// The top-level Markdown blocks of `source`, each list item on its own, as source slices:
/// the cursor's lines inside message text.
pub fn blocks(source: &str) -> Vec<&str> {
    use pulldown_cmark::{Event, Options, Parser, Tag};
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut in_list = false;
    for (event, range) in Parser::new_ext(source, Options::all()).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                let top = depth == 0 || (in_list && depth == 1);
                if depth == 0 && matches!(tag, Tag::List(_)) {
                    in_list = true;
                } else if top {
                    out.push(source[range].trim_end());
                }
                depth += 1;
            }
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    in_list = false;
                }
            }
            Event::Rule | Event::Html(_) | Event::Text(_) if depth == 0 => {
                out.push(source[range].trim_end())
            }
            _ => {}
        }
    }
    out.retain(|s| !s.trim().is_empty());
    out
}

/// The stops, top to bottom.
pub fn stops(items: &[Item]) -> Vec<Stop> {
    let mut out = Vec::new();
    for item in items {
        match item {
            Item::Input { head, body, .. } => {
                out.push(Stop::Head(*head));
                out.extend(body.iter().filter_map(|b| match b {
                    Body::Text(t) => Some(t.stop.clone()),
                    Body::Image(_) => None,
                }));
            }
            Item::Chrome { stop, .. } => out.push(stop.clone()),
            Item::Turn { head, parts, .. } => {
                out.push(Stop::Head(*head));
                for p in parts {
                    match p {
                        Part::Text(t) => out.push(t.stop.clone()),
                        Part::Thinking { stop, .. } => out.push(stop.clone()),
                        Part::Tool(r) => out.push(Stop::Tool(r.call.clone())),
                        Part::Fold { stop, rows, open } => {
                            out.push(stop.clone());
                            if *open {
                                out.extend(rows.iter().map(|r| Stop::Tool(r.call.clone())));
                            }
                        }
                    }
                }
            }
            Item::Record { head, body, .. } => {
                out.push(Stop::Head(*head));
                out.extend(body.iter().flatten().map(|t| t.stop.clone()));
            }
            Item::Gap | Item::Writing { .. } | Item::Failure { .. } | Item::Stopped => {}
        }
    }
    out
}

/// Every stop `za` can open or close, with whether it is open now: for `zR` and `zM`.
pub fn foldables(items: &[Item]) -> Vec<(Stop, bool)> {
    let mut out = Vec::new();
    for item in items {
        match item {
            Item::Chrome {
                stop,
                text: Some(_),
                open,
                ..
            } => out.push((stop.clone(), *open)),
            Item::Turn { parts, .. } => {
                for p in parts {
                    match p {
                        Part::Thinking { stop, open, .. } => out.push((stop.clone(), *open)),
                        Part::Tool(r) => out.push((Stop::Tool(r.call.clone()), r.open)),
                        Part::Fold { stop, rows, open } => {
                            out.push((stop.clone(), *open));
                            out.extend(rows.iter().map(|r| (Stop::Tool(r.call.clone()), r.open)));
                        }
                        Part::Text(_) => {}
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
#[path = "transcript_tests.rs"]
mod tests;
