//! The protocol's shapes: call names, request bodies, the three reply envelopes and the results this client reads.
//! Replies are parsed here, at the boundary; the rest of the crate trusts these types.

use crate::ids::{
    ConversationId, DecisionId, Device, EntryId, Project, RequestId, Subject, TaskId,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;

/// The protocol this client speaks; every reply carries it in `Tau-Protocol`.
pub const PROTOCOL: &str = "1";

/// The server's body limit (`http.ts:11`).
pub const BODY_LIMIT: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CallName {
    Pair,
    /// `POST /v1/`: no such call, so the server answers `unknown_call` with its list of calls (`client.md` §7).
    Handshake,
    Extensions,
    Models,
    Projects,
    Bind,
    AddExtensions,
    Options,
    SetOptions,
    Create,
    Conversations,
    Open,
    History,
    Input,
    Continue,
    Stop,
    SetModel,
    Inbox,
    Decision,
    Answer,
    Timeline,
    Faults,
}

impl CallName {
    /// The last segment of `/v1/<name>`.
    pub fn path(self) -> &'static str {
        match self {
            CallName::Pair => "pair",
            CallName::Handshake => "",
            CallName::Extensions => "extensions",
            CallName::Models => "models",
            CallName::Projects => "projects",
            CallName::Bind => "bind",
            CallName::AddExtensions => "addExtensions",
            CallName::Options => "options",
            CallName::SetOptions => "setOptions",
            CallName::Create => "create",
            CallName::Conversations => "conversations",
            CallName::Open => "open",
            CallName::History => "history",
            CallName::Input => "input",
            CallName::Continue => "continue",
            CallName::Stop => "stop",
            CallName::SetModel => "setModel",
            CallName::Inbox => "inbox",
            CallName::Decision => "decision",
            CallName::Answer => "answer",
            CallName::Timeline => "timeline",
            CallName::Faults => "faults",
        }
    }
}

/// One call to make: its name and its exact JSON body.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub call: CallName,
    pub body: Value,
}

/// How one call ended (`client.md` §1). The body decides, never the status.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Outcome {
    Done(Value),
    Refused(Refusal),
    Unknown(Unknown),
}

/// `{"refused": {code, message, …}}`: not done; the server decided.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Refusal {
    pub code: RefusalCode,
    pub message: String,
    /// Every other field of the refusal: `errors`, `decision`, `open`, `project`, `calls`.
    pub extra: Map<String, Value>,
}

/// The refusal codes of protocol 1 (`refusals.ts:4-26`), and any other a newer server sends: an unknown code is
/// still "not done".
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RefusalCode {
    BadRequest,
    BadCursor,
    Unauthorized,
    UnknownCall,
    UnknownProject,
    UnknownConversation,
    UnknownDecision,
    Conflict,
    NotOpen,
    StaleBasis,
    UnknownChoice,
    BadAnnotation,
    ExtensionNotLoaded,
    BadOptions,
    OptionsInvalid,
    NotAWorkflow,
    UnknownModel,
    Busy,
    Idle,
    EventLog,
    TooLarge,
    Other(String),
}

const CODES: [(RefusalCode, &str); 21] = [
    (RefusalCode::BadRequest, "bad_request"),
    (RefusalCode::BadCursor, "bad_cursor"),
    (RefusalCode::Unauthorized, "unauthorized"),
    (RefusalCode::UnknownCall, "unknown_call"),
    (RefusalCode::UnknownProject, "unknown_project"),
    (RefusalCode::UnknownConversation, "unknown_conversation"),
    (RefusalCode::UnknownDecision, "unknown_decision"),
    (RefusalCode::Conflict, "conflict"),
    (RefusalCode::NotOpen, "not_open"),
    (RefusalCode::StaleBasis, "stale_basis"),
    (RefusalCode::UnknownChoice, "unknown_choice"),
    (RefusalCode::BadAnnotation, "bad_annotation"),
    (RefusalCode::ExtensionNotLoaded, "extension_not_loaded"),
    (RefusalCode::BadOptions, "bad_options"),
    (RefusalCode::OptionsInvalid, "options_invalid"),
    (RefusalCode::NotAWorkflow, "not_a_workflow"),
    (RefusalCode::UnknownModel, "unknown_model"),
    (RefusalCode::Busy, "busy"),
    (RefusalCode::Idle, "idle"),
    (RefusalCode::EventLog, "event_log"),
    (RefusalCode::TooLarge, "too_large"),
];

impl RefusalCode {
    pub fn parse(code: &str) -> RefusalCode {
        CODES
            .iter()
            .find(|(_, text)| *text == code)
            .map(|(c, _)| c.clone())
            .unwrap_or(RefusalCode::Other(code.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        match self {
            RefusalCode::Other(code) => code,
            known => CODES
                .iter()
                .find(|(c, _)| c == known)
                .map(|(_, text)| *text)
                .unwrap_or_default(),
        }
    }

    /// `unknown_project`, `unknown_conversation`, `unknown_decision`: the object is gone from the server.
    pub fn is_gone(&self) -> bool {
        matches!(
            self,
            RefusalCode::UnknownProject
                | RefusalCode::UnknownConversation
                | RefusalCode::UnknownDecision
        )
    }
}

impl fmt::Display for RefusalCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for RefusalCode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RefusalCode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(RefusalCode::parse(&String::deserialize(d)?))
    }
}

/// Why an outcome is unknown: the call may or may not have been done.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Unknown {
    /// `{"unknown": {code, message}}`, such as `storage_failed`.
    Server { code: String, message: String },
    /// No reply within 20 s.
    Timeout,
    /// The connection was cut, or failed in a way that may follow sending; the text is the system's.
    Transport(String),
    /// No connection could be made (a bad URL, no address, refused, timed out connecting): nothing
    /// was sent, so nothing was done. The text is the system's.
    Unreachable(String),
    /// A reply whose body is none of the three envelopes, such as a proxy's error page.
    Unreadable { status: u16 },
    /// The reply names another protocol in `Tau-Protocol`.
    OtherProtocol(String),
    /// The client quit while the call was in flight.
    Restart,
}

/// A device token, `tau.<device>.<secret>`. Never printed.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Token(String);

impl Token {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(…)")
    }
}

// ---------------------------------------------------------------- replies

/// Reads one reply: its status, its `Tau-Protocol` header and its body.
pub fn parse_reply(status: u16, protocol: Option<&str>, body: &[u8]) -> Outcome {
    if let Some(other) = protocol.map(str::trim).filter(|p| *p != PROTOCOL) {
        return Outcome::Unknown(Unknown::OtherProtocol(other.to_owned()));
    }
    let unreadable = Outcome::Unknown(Unknown::Unreadable { status });
    let Ok(Value::Object(mut envelope)) = serde_json::from_slice::<Value>(body) else {
        return unreadable;
    };
    let text = |map: &mut Map<String, Value>, key: &str| match map.remove(key) {
        Some(Value::String(s)) => Some(s),
        _ => None,
    };
    if let Some(result) = envelope.remove("result") {
        return Outcome::Done(result);
    }
    if let Some(Value::Object(mut refused)) = envelope.remove("refused") {
        let Some(code) = text(&mut refused, "code") else {
            return unreadable;
        };
        let message = text(&mut refused, "message").unwrap_or_default();
        return Outcome::Refused(Refusal {
            code: RefusalCode::parse(&code),
            message,
            extra: refused,
        });
    }
    if let Some(Value::Object(mut unknown)) = envelope.remove("unknown") {
        let code = text(&mut unknown, "code").unwrap_or_default();
        let message = text(&mut unknown, "message").unwrap_or_default();
        return Outcome::Unknown(Unknown::Server { code, message });
    }
    unreadable
}

impl Refusal {
    /// `errors` of `bad_request`, `bad_options` and `options_invalid`.
    pub fn errors(&self) -> Vec<FieldError> {
        self.field("errors").unwrap_or_default()
    }

    /// `decision` of an answer `conflict`: the record as answered.
    pub fn decision(&self) -> Option<DecisionRecord> {
        self.field("decision")
    }

    /// `open` of `not_open`: the subject's open decision now.
    pub fn open(&self) -> Option<DecisionRecord> {
        self.field("open")
    }

    /// `project` of a bind `conflict`: the project as bound.
    pub fn project(&self) -> Option<ProjectView> {
        self.field("project")
    }

    /// `calls` of `unknown_call`.
    pub fn calls(&self) -> Vec<String> {
        self.field("calls").unwrap_or_default()
    }

    fn field<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        serde_json::from_value(self.extra.get(key)?.clone()).ok()
    }
}

// ---------------------------------------------------------------- results

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldError {
    pub path: String,
    #[serde(default)]
    pub keyword: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub name: String,
    #[serde(default)]
    pub workflow: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelInfo {
    pub model: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectView {
    pub project: Project,
    pub workdir: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub missing: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BindResult {
    pub project: ProjectView,
    #[serde(default)]
    pub repeated: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OptionsItem {
    pub extension: String,
    #[serde(default)]
    pub schema: Value,
    #[serde(default)]
    pub value: Map<String, Value>,
    #[serde(default)]
    pub errors: Vec<FieldError>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunState {
    Running,
    Failed,
    Idle,
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner {
    #[serde(rename = "conversationId")]
    pub conversation: ConversationId,
    #[serde(rename = "taskId")]
    pub task: TaskId,
}

/// A row of `conversations`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationRow {
    pub conversation: ConversationId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<Owner>,
    pub state: RunState,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fault {
    pub task: TaskId,
    pub kind: String,
    pub conversation: ConversationId,
    #[serde(default)]
    pub error: String,
    #[serde(rename = "endedAt", default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<DecisionId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    pub start: u64,
    pub end: u64,
}

/// A note sent with an answer: on a range of the basis text in UTF-16 code units, or on the whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Annotation {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<TextRange>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionStatus {
    Open,
    Answered,
    Superseded,
    Withdrawn,
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThenTask {
    pub task: String,
}

/// A decision record (`docs/primitives.md`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRecord {
    pub id: DecisionId,
    pub subject: Subject,
    pub title: String,
    pub basis: EntryId,
    pub choices: Vec<String>,
    pub conversation: ConversationId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub then: Option<ThenTask>,
    pub requested_at: i64,
    pub status: DecisionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choice: Option<String>,
    #[serde(default)]
    pub annotations: Vec<Annotation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<Device>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<TaskId>,
}

/// The result of `decision`: the record, its basis text, and the `then` task's state once answered.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionView {
    pub decision: DecisionRecord,
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub then: Option<TaskState>,
}

/// A pi-durable task state, read leniently: `status`, and `outcome` once completing or terminal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskState {
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<TaskOutcome>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskOutcome {
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorText>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorText {
    #[serde(default)]
    pub message: String,
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        self.status == "terminal"
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnswerResult {
    pub decision: DecisionRecord,
    #[serde(default)]
    pub repeated: bool,
}

/// An open decision as the inbox lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDecision {
    pub id: DecisionId,
    pub title: String,
    pub requested_at: i64,
    pub conversation: ConversationId,
}

/// The result of `input` and `continue`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Submission {
    pub id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Paired {
    pub device: Device,
    pub token: Token,
}

// ---------------------------------------------------------------- request bodies

/// `whenBusy` of `input`. The client never sends `whenIdle` (`client.md` §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputMode {
    /// No `whenBusy`: an idle or failed conversation starts a run.
    Send,
    Steer,
    FollowUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    Ascending,
    Descending,
}

/// The bounds of one `history` sequence. A cursor must be sent back with the same bounds (`calls.ts:412-427`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryQuery {
    pub project: Project,
    pub conversation: ConversationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_entry_id: Option<EntryId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_entry_id: Option<EntryId>,
    pub order: Order,
    pub limit: u32,
}

impl HistoryQuery {
    pub fn body(&self, cursor: Option<&str>) -> Value {
        let mut body = serde_json::to_value(self).unwrap_or_default();
        if let (Some(cursor), Value::Object(map)) = (cursor, &mut body) {
            map.insert("cursor".into(), Value::String(cursor.to_owned()));
        }
        body
    }
}

pub(crate) fn input_body(
    project: &Project,
    conversation: ConversationId,
    text: &str,
    request: &RequestId,
    mode: InputMode,
) -> Value {
    let mut body = serde_json::json!({
        "project": project, "conversation": conversation, "text": text, "requestId": request,
    });
    let when = match mode {
        InputMode::Send => None,
        InputMode::Steer => Some("steer"),
        InputMode::FollowUp => Some("followUp"),
    };
    if let (Some(when), Value::Object(map)) = (when, &mut body) {
        map.insert("whenBusy".into(), when.into());
    }
    body
}
