//! Lenient typed views over what the UI renders: pi-durable entries and conversation documents, and the names
//! `spec.md` §3.2 derives from core contracts. A field this client doesn't know is ignored; a missing optional part
//! reads as absent, never as a failure.

use crate::ids::{ConversationId, EntryId, TaskId};
use crate::wire::{ExtensionInfo, OptionsItem};
use serde_json::Value;

/// One entry, parsed once: entries never change.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: EntryId,
    pub conversation: ConversationId,
    pub kind: String,
    /// Set when a task wrote it: a workflow's input or record, never the user's own.
    pub by_task: Option<TaskId>,
    pub body: EntryBody,
    /// The entry as sent, for `K` (its `data` as JSON).
    pub raw: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EntryBody {
    /// `pi.user`: an input. `content` is a string or text and image blocks.
    User {
        content: Vec<Content>,
        timestamp: Option<i64>,
    },
    /// `pi.assistant`.
    Assistant(AssistantMessage),
    /// `pi.tool-result`: sets the outcome of the tool row with the same call ID.
    ToolResult {
        call_id: String,
        tool: String,
        is_error: bool,
        text: String,
        duration_ms: Option<u64>,
    },
    /// `pi.system`: the tools added and removed.
    System {
        added: Vec<String>,
        removed: Vec<String>,
    },
    /// `pi.reset`: a context reset, with the handoff text when it holds one.
    Reset { handoff: Option<String> },
    /// `pi.compaction`: earlier context summarised.
    Compaction { summary: Option<String> },
    /// Any other kind (`w1.design`, `w2.asked`): a record, with `data.text` when it is a string.
    Record { text: Option<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    Text(String),
    Image { mime_type: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssistantMessage {
    pub blocks: Vec<Block>,
    pub provider: String,
    pub model: String,
    pub timestamp: Option<i64>,
    pub stop_reason: String,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Text(String),
    Thinking(String),
    ToolCall {
        id: String,
        name: String,
        arguments: Value,
    },
}

/// A conversation's documents, read after every frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Docs {
    pub live: Live,
    /// `pi.inbox.items`: inputs queued for the run.
    pub queue: Vec<QueueItem>,
    pub agent: Agent,
}

/// `pi.live`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Live {
    pub run: Option<Run>,
    pub run_failure: Option<RunFailure>,
    pub generation: Option<Generation>,
    pub tools: Vec<ToolSlot>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub task: TaskId,
    pub turns: u64,
    pub first: Option<EntryId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunFailure {
    pub task: Option<TaskId>,
    pub reason: String,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Generation {
    pub attempt: u64,
    /// The message being written; its last block is the streaming tail.
    pub message: Option<AssistantMessage>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolSlot {
    pub call_id: String,
    pub name: String,
    pub status: String,
    pub output: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueItem {
    pub id: u64,
    pub mode: QueueMode,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueueMode {
    Steer,
    FollowUp,
    Write,
    Other(String),
}

/// `pi.agent`: the conversation's stored settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Agent {
    /// `provider/modelId`.
    pub model: Option<String>,
    pub thinking_level: Option<String>,
    pub extensions: Selection,
    pub cwd: Option<String>,
}

/// `pi.agent.extensions`: absent is the project default; a list replaces it; `{add, remove}` changes it.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Selection {
    #[default]
    ProjectDefault,
    Only(Vec<String>),
    Change {
        add: Vec<String>,
        remove: Vec<String>,
    },
}

/// A conversation's state as the client shows it (`spec.md` §3.2, §5.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversationState {
    Running,
    Failed {
        reason: String,
    },
    /// A `runFailure` whose reason names an abort: not a failure, not counted by `×`.
    Stopped,
    Idle,
}

/// Why a workflow can't start (`spec.md` §3.2): the server refuses `create` while its options item has any error.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OptionsProblems {
    /// Required keys neither set nor defaulted, in the schema's order: `needs options: designer, …`.
    pub missing: Vec<String>,
    /// Every other error, by the key its path names: `options don't fit: reviewer`.
    pub misfits: Vec<String>,
}

impl Entry {
    /// `None` when the value has no numeric `id`, `conversationId` and string `kind`.
    pub fn parse(raw: Value) -> Option<Entry> {
        let id = EntryId(raw.get("id")?.as_u64()?);
        let conversation = ConversationId(raw.get("conversationId")?.as_u64()?);
        let kind = raw.get("kind")?.as_str()?.to_owned();
        let by_task = raw.get("byTaskId").and_then(Value::as_u64).map(TaskId);
        let message = raw
            .get("model")
            .and_then(|m| m.get(0))
            .unwrap_or(&Value::Null);
        let body = match kind.as_str() {
            "pi.user" => EntryBody::User {
                content: contents(message),
                timestamp: message["timestamp"].as_i64(),
            },
            "pi.assistant" => EntryBody::Assistant(assistant(message)),
            "pi.tool-result" => EntryBody::ToolResult {
                call_id: text_of(message, "toolCallId"),
                tool: text_of(message, "toolName"),
                is_error: message["isError"].as_bool().unwrap_or(false),
                text: contents(message)
                    .into_iter()
                    .filter_map(|c| {
                        if let Content::Text(t) = c {
                            Some(t)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                duration_ms: message["durationMs"].as_u64(),
            },
            "pi.system" => EntryBody::System {
                added: names(&message["toolsAdded"]),
                removed: names(&message["toolsRemoved"]),
            },
            "pi.reset" => EntryBody::Reset {
                handoff: first_text(&contents(message)).map(str::to_owned),
            },
            "pi.compaction" => EntryBody::Compaction {
                summary: first_text(&contents(message)).map(str::to_owned),
            },
            _ => EntryBody::Record {
                text: raw
                    .get("data")
                    .and_then(|d| d.get("text"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            },
        };
        Some(Entry {
            id,
            conversation,
            kind,
            by_task,
            body,
            raw,
        })
    }

    /// The text of an input or a record, the first text block's for array content.
    pub fn user_text(&self) -> Option<&str> {
        match &self.body {
            EntryBody::User { content, .. } => first_text(content),
            EntryBody::Record { text } => text.as_deref(),
            _ => None,
        }
    }
}

fn text_of(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().to_owned()
}

fn first_text(content: &[Content]) -> Option<&str> {
    content.iter().find_map(|c| {
        if let Content::Text(t) = c {
            Some(t.as_str())
        } else {
            None
        }
    })
}

/// `content` of a message: a string, or text and image blocks.
fn contents(message: &Value) -> Vec<Content> {
    match &message["content"] {
        Value::String(text) => vec![Content::Text(text.clone())],
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|b| match b["type"].as_str()? {
                "text" => Some(Content::Text(text_of(b, "text"))),
                "image" => Some(Content::Image {
                    mime_type: text_of(b, "mimeType"),
                }),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn assistant(message: &Value) -> AssistantMessage {
    let blocks = message["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|b| match b["type"].as_str()? {
                    "text" => Some(Block::Text(text_of(b, "text"))),
                    "thinking" => Some(Block::Thinking(text_of(b, "thinking"))),
                    "toolCall" => Some(Block::ToolCall {
                        id: b["id"].as_str()?.to_owned(),
                        name: text_of(b, "name"),
                        arguments: b.get("arguments").cloned().unwrap_or(Value::Null),
                    }),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    AssistantMessage {
        blocks,
        provider: text_of(message, "provider"),
        model: text_of(message, "model"),
        timestamp: message["timestamp"].as_i64(),
        stop_reason: text_of(message, "stopReason"),
        error_message: message["errorMessage"].as_str().map(str::to_owned),
    }
}

fn names(tools: &Value) -> Vec<String> {
    tools
        .as_array()
        .map(|t| {
            t.iter()
                .filter_map(|t| t["name"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

impl Docs {
    /// Reads `docs` of a `ConversationView`.
    pub fn parse(docs: &Value) -> Docs {
        let live = &docs["pi.live"];
        let agent = &docs["pi.agent"];
        Docs {
            live: Live {
                run: live.get("run").map(|r| Run {
                    task: TaskId(r["taskId"].as_u64().unwrap_or_default()),
                    turns: r["turns"].as_u64().unwrap_or_default(),
                    first: r["first"].as_u64().map(EntryId),
                }),
                run_failure: live.get("runFailure").map(|f| RunFailure {
                    task: f["taskId"].as_u64().map(TaskId),
                    reason: text_of(f, "reason"),
                    detail: f["detail"].as_str().map(str::to_owned),
                }),
                generation: live.get("generation").map(|g| Generation {
                    attempt: g["attempt"].as_u64().unwrap_or_default(),
                    message: g.get("message").map(assistant),
                }),
                tools: live["tools"]
                    .as_array()
                    .map(|tools| {
                        tools
                            .iter()
                            .map(|t| ToolSlot {
                                call_id: text_of(t, "callId"),
                                name: text_of(t, "name"),
                                status: text_of(t, "status"),
                                output: text_of(t, "output"),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            },
            queue: docs["pi.inbox"]["items"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|i| QueueItem {
                            id: i["id"].as_u64().unwrap_or_default(),
                            mode: match i["mode"].as_str().unwrap_or_default() {
                                "steer" => QueueMode::Steer,
                                "followUp" => QueueMode::FollowUp,
                                "write" => QueueMode::Write,
                                other => QueueMode::Other(other.to_owned()),
                            },
                            text: first_text(&contents(i)).unwrap_or_default().to_owned(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            agent: Agent {
                model: agent.get("model").and_then(|m| {
                    Some(format!(
                        "{}/{}",
                        m["provider"].as_str()?,
                        m["modelId"].as_str()?
                    ))
                }),
                thinking_level: agent["thinkingLevel"].as_str().map(str::to_owned),
                extensions: match agent.get("extensions") {
                    Some(Value::Array(_)) => Selection::Only(strings(&agent["extensions"])),
                    Some(Value::Object(change)) => Selection::Change {
                        add: change.get("add").map(strings).unwrap_or_default(),
                        remove: change.get("remove").map(strings).unwrap_or_default(),
                    },
                    _ => Selection::ProjectDefault,
                },
                cwd: agent["cwd"].as_str().map(str::to_owned),
            },
        }
    }
}

impl Live {
    /// `run` present is running, else `runFailure` is failed (stopped when its reason is `aborted`), else idle.
    pub fn state(&self) -> ConversationState {
        match (&self.run, &self.run_failure) {
            (Some(_), _) => ConversationState::Running,
            (None, Some(f)) if f.reason == "aborted" => ConversationState::Stopped,
            (None, Some(f)) => ConversationState::Failed {
                reason: f.reason.clone(),
            },
            (None, None) => ConversationState::Idle,
        }
    }
}

/// The longest label, in characters, `…` included.
pub const LABEL_MAX: usize = 80;

/// A conversation's label: the first line of its first input, at most 80 characters, cut with `…`.
pub fn label(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or_default();
    if line.chars().count() <= LABEL_MAX {
        return line.to_owned();
    }
    let mut cut: String = line.chars().take(LABEL_MAX - 1).collect();
    cut.push('…');
    cut
}

/// The extension an event's kind names: `w1.event` → `w1`.
pub fn event_extension(kind: &str) -> Option<&str> {
    kind.split_once('.')
        .map(|(ext, _)| ext)
        .filter(|ext| !ext.is_empty())
}

/// The workflow a conversation's selection names, if the catalogue says that name is a workflow.
pub fn selection_workflow<'a>(
    selection: &'a Selection,
    catalogue: &[ExtensionInfo],
) -> Option<&'a str> {
    let named = match selection {
        Selection::ProjectDefault => return None,
        Selection::Only(names) => names,
        Selection::Change { add, .. } => add,
    };
    named
        .iter()
        .map(String::as_str)
        .find(|n| catalogue.iter().any(|e| e.workflow && e.name == *n))
}

/// Splits an options item's errors into missing keys and values that don't fit.
pub fn options_problems(item: &OptionsItem) -> OptionsProblems {
    if item.errors.is_empty() {
        return OptionsProblems::default();
    }
    let properties = &item.schema["properties"];
    let missing: Vec<String> = strings(&item.schema["required"])
        .into_iter()
        .filter(|key| !item.value.contains_key(key) && properties[key].get("default").is_none())
        .collect();
    // A `required` error is named by `missing`, which comes from the schema; should the schema name none, the
    // error still shows, by its path, so an item with errors always names something.
    let mut misfits: Vec<String> = Vec::new();
    for error in &item.errors {
        let name = pointer_name(&error.path);
        let counted = error.keyword == "required" && !missing.is_empty();
        if !counted && !misfits.contains(&name) {
            misfits.push(name);
        }
    }
    OptionsProblems { missing, misfits }
}

/// A JSON pointer as the key it names: `/reviewer` → `reviewer`.
fn pointer_name(path: &str) -> String {
    path.strip_prefix('/')
        .unwrap_or(path)
        .replace("~1", "/")
        .replace("~0", "~")
}
