//! The pure core of the link. Inputs are the UI's commands, call replies, stream messages and the passing of time;
//! outputs are effects for the runtime to carry out (make a call, open or close a stream, persist, save
//! credentials). It never reads a clock and never does I/O: every input carries [`Now`].

use crate::clock::{Mono, Now, Wall};
use crate::ids::{ConvRef, DecisionId, DecisionRef, Device, Project, RequestId, WriteId};
use crate::model::Server;
use crate::outbox::{Next, Outbox, OutboxEntry, StartWith, WriteCall, WriteState};
use crate::sse::{EndReason, Frame};
use crate::streams::{EndCause, Phase, Stream, StreamId, StreamKey};
use crate::wire::{
    CallName, ConversationRow, Fault, HistoryQuery, Outcome, Refusal, RefusalCode, Request, Token,
    Unknown,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

/// A call's reply arrives later than this: its outcome is unknown, and a late reply changes nothing.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(20);
/// No stream connected for this long is offline.
pub const OFFLINE_AFTER: Duration = Duration::from_secs(60);

/// One call in flight. A reply naming a call no longer in flight is ignored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallId(pub u64);

/// What the engine starts from.
#[derive(Clone, Debug, Default)]
pub struct Setup {
    /// The server and this device's name, when paired.
    pub server: Option<(String, Device)>,
    /// The outbox of an earlier launch.
    pub outbox: Vec<OutboxEntry>,
    /// Labels found in earlier launches; entries never change, so they hold for good.
    pub labels: BTreeMap<ConvRef, String>,
    /// Seeds request IDs.
    pub seed: u64,
}

/// Something that happened outside the engine.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Reply {
        call: CallId,
        outcome: Outcome,
    },
    Stream {
        id: StreamId,
        event: StreamEvent,
    },
    /// Time passed; timers that are due fire.
    Tick,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StreamEvent {
    Frame(Frame),
    /// A JSON reply came instead of a stream: a refusal, or an outcome this client can't read.
    Refused(Outcome),
    /// A frame that is neither ops nor an end.
    BadFrame(String),
    /// The connection closed or failed, with no `end`.
    Closed(Option<String>),
}

/// What the runtime must do.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Make one call to `url`; with the device token unless `auth` is false (`pair`).
    Call {
        id: CallId,
        url: String,
        auth: bool,
        request: Request,
    },
    Open {
        id: StreamId,
        request: Request,
    },
    Close {
        id: StreamId,
    },
    SaveOutbox,
    SaveLabels,
    /// Keep the token (Keychain) and the server and device name (`client.toml`).
    SaveCredentials {
        url: String,
        device: Device,
        token: Token,
    },
    /// `:forget-server`: drop the token and this server's device-local state.
    ForgetServer {
        url: String,
    },
}

/// Which `open` stream a conversation is shown in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    Stage,
    Peek,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TranscriptKey {
    Conversation(ConvRef),
    Timeline(Project),
}

/// The link as the server line and the Server float say it.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkStatus {
    pub phase: LinkPhase,
    pub streams: Vec<(StreamKey, Stream)>,
    pub notices: Vec<Notice>,
    /// Anything has come from the server since launch.
    pub received_anything: bool,
    /// Writes other than a queued input can be sent now.
    pub writes_on: bool,
}

/// The server line's state (`spec.md` §4.1), most urgent first.
#[derive(Clone, Debug, PartialEq)]
pub enum LinkPhase {
    NotPaired,
    Pairing,
    /// The server speaks another protocol; nothing is sent.
    OtherProtocol(String),
    /// A stream ended `revoked`.
    Revoked,
    /// A call was refused `unauthorized`; writes wait, streams keep retrying.
    Refused {
        message: String,
    },
    /// No stream connected for 60 s.
    Offline {
        since: Wall,
        received_anything: bool,
    },
    /// From launch until the first inbox frame.
    Connecting,
    /// A stream ended `stopping` (or the inbox answered `busy`) and is retrying.
    ServerStopped,
    /// A stream was cut and reopens in this many seconds.
    Reconnecting {
        in_secs: u64,
    },
    Live,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Notice {
    /// Three `end: failed` in a row. For the inbox: open decisions may be out of date for every project.
    KeepsFailing(StreamKey),
}

/// Something the UI echoes or reacts to, once.
#[derive(Clone, Debug, PartialEq)]
pub enum LinkEvent {
    /// A write reached `Done`, `Refused`, `Unknown` or a settlement.
    Write {
        id: WriteId,
        call: WriteCall,
        state: WriteState,
    },
    /// The start page's `create` made this conversation; its `input` is on its way.
    Started {
        conv: ConvRef,
        with: StartWith,
        create: WriteId,
        input: WriteId,
    },
    /// The inbox no longer lists this decision. `ours` when this device's answer settled it.
    DecisionLeft {
        decision: DecisionRef,
        ours: bool,
    },
    /// The decision's subject has another open decision now.
    Superseded {
        decision: DecisionRef,
        by: DecisionId,
    },
    /// A stream ended `stopping` or `failed`.
    StreamEnded {
        key: StreamKey,
        reason: EndReason,
    },
    Revoked,
    Paired {
        device: Device,
    },
    PairFailed(PairFailure),
    ModelsRefreshed(Result<(), ModelRefreshError>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModelRefreshError {
    NotConnected,
    Refused(Refusal),
    Unknown(Unknown),
    InvalidReply,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PairFailure {
    /// `unauthorized`: a wrong, used or expired code, or a Host not allowed.
    Refused(Refusal),
    /// The reply was lost: if the code was accepted, the name is used and the token is gone.
    NoReply(Unknown),
}

mod fetch;
mod link;
mod writes;

/// Where the link stands with the server.
#[derive(Clone, Debug, PartialEq)]
enum Conn {
    NotPaired,
    Pairing,
    /// `POST /v1/` not yet answered as protocol 1. `due` is the next try while backing off.
    Handshaking {
        failures: u32,
        due: Option<Mono>,
    },
    Ready,
    OtherProtocol(String),
    Revoked,
}

/// What a call in flight is for: decides what its reply changes.
#[derive(Clone, Debug)]
enum Purpose {
    Pair {
        url: String,
    },
    Handshake,
    Catalogue(CallName),
    RefreshModels,
    Options(Project),
    Conversations {
        project: Project,
        rows: Vec<ConversationRow>,
    },
    Faults {
        project: Project,
        rows: Vec<Fault>,
    },
    Decision(DecisionRef),
    Label {
        conv: ConvRef,
        pages: u32,
    },
    Gap {
        key: TranscriptKey,
        query: HistoryQuery,
    },
    Older {
        key: TranscriptKey,
        query: HistoryQuery,
        cursor: Option<String>,
    },
    Write(WriteId),
}

#[derive(Clone, Debug)]
struct InFlight {
    purpose: Purpose,
    deadline: Mono,
}

/// A stream the engine holds, and the value its frames build.
#[derive(Clone, Debug)]
struct Held {
    stream: Stream,
    value: Option<Value>,
}

/// One project's polls (`client.md` §5).
#[derive(Clone, Debug, Default)]
struct Polls {
    conversations: Poll,
    faults: Poll,
}

#[derive(Clone, Debug, Default)]
struct Poll {
    due: Option<Mono>,
    in_flight: bool,
}

/// The engine. See the module doc.
pub struct Engine {
    server: Server,
    outbox: Outbox,
    effects: Vec<Effect>,
    events: Vec<LinkEvent>,

    conn: Conn,
    /// The server's sentence after an `unauthorized` on a call; writes wait until a call succeeds.
    band: Option<String>,
    received_anything: bool,
    /// When the last connected stream went down, or launch; `None` while a stream is connected.
    down_since: Option<Now>,
    seq: u64,
    rng: u64,
    calls: BTreeMap<CallId, InFlight>,
    streams: BTreeMap<StreamKey, Held>,
    slots: BTreeMap<SlotKey, ConvRef>,
    focused: bool,
    polls: BTreeMap<Project, Polls>,
    watched: Option<DecisionRef>,
    /// Decisions to fetch again, and when.
    refetch: BTreeMap<DecisionRef, Mono>,
    /// When a label was last looked for, found or not: at most once a minute.
    label_tried: HashMap<ConvRef, Mono>,
    /// Gap fetches that failed, to try again.
    gap_retries: Vec<(Mono, TranscriptKey, HistoryQuery)>,
    /// When each done entry leaves the outbox.
    linger: BTreeMap<WriteId, Mono>,
    /// The sequence number when each write's outcome became unknown: a fresh inbox frame must come after it.
    unknown_at: BTreeMap<WriteId, u64>,
    /// The start page's text, kept until its `create` answers and the `input` is made.
    starts: BTreeMap<WriteId, String>,
}

/// `Slot` ordered, for a stable map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SlotKey {
    Stage,
    Peek,
}

impl From<Slot> for SlotKey {
    fn from(slot: Slot) -> Self {
        match slot {
            Slot::Stage => SlotKey::Stage,
            Slot::Peek => SlotKey::Peek,
        }
    }
}

impl Engine {
    pub fn new(setup: Setup, now: Now) -> Engine {
        let mut outbox = Outbox::restore(setup.outbox);
        outbox.next_id = outbox.next_id.max(1);
        let mut server = Server::default();
        server.labels.found = setup.labels;
        let mut engine = Engine {
            server,
            outbox,
            effects: Vec::new(),
            events: Vec::new(),
            conn: Conn::NotPaired,
            band: None,
            received_anything: false,
            down_since: Some(now),
            seq: 0,
            rng: setup.seed ^ 0x9E37_79B9_7F4A_7C15,
            calls: BTreeMap::new(),
            streams: BTreeMap::new(),
            slots: BTreeMap::new(),
            focused: false,
            polls: BTreeMap::new(),
            watched: None,
            refetch: BTreeMap::new(),
            label_tried: HashMap::new(),
            gap_retries: Vec::new(),
            linger: BTreeMap::new(),
            unknown_at: BTreeMap::new(),
            starts: BTreeMap::new(),
        };
        if let Some((url, device)) = setup.server {
            engine.server.url = Some(url);
            engine.server.device = Some(device);
            engine.handshake(now);
        }
        engine
    }

    pub fn server(&self) -> &Server {
        &self.server
    }

    pub fn outbox(&self) -> &Outbox {
        &self.outbox
    }

    pub fn take_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }

    pub fn take_events(&mut self) -> Vec<LinkEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn step(&mut self, input: Input, now: Now) {
        let was_connected = self.connected();
        match input {
            Input::Reply { call, outcome } => {
                if let Some(flight) = self.calls.remove(&call) {
                    self.on_reply(flight.purpose, outcome, now);
                }
            }
            Input::Stream { id, event } => self.on_stream(id, event, now),
            Input::Tick => {}
        }
        self.tick(now);
        self.settle(was_connected, now);
    }

    pub fn status(&self, now: Now) -> LinkStatus {
        LinkStatus {
            phase: self.phase(now),
            streams: self
                .streams
                .iter()
                .map(|(k, h)| (k.clone(), h.stream.clone()))
                .collect(),
            notices: self
                .streams
                .iter()
                .filter(|(_, h)| h.stream.keeps_failing())
                .map(|(k, _)| Notice::KeepsFailing(k.clone()))
                .collect(),
            received_anything: self.received_anything,
            writes_on: self.writes_on(),
        }
    }

    fn phase(&self, now: Now) -> LinkPhase {
        match &self.conn {
            Conn::NotPaired => return LinkPhase::NotPaired,
            Conn::Pairing => return LinkPhase::Pairing,
            Conn::OtherProtocol(n) => return LinkPhase::OtherProtocol(n.clone()),
            Conn::Revoked => return LinkPhase::Revoked,
            Conn::Handshaking { .. } | Conn::Ready => {}
        }
        if let Some(message) = &self.band {
            return LinkPhase::Refused {
                message: message.clone(),
            };
        }
        if let Some(down) = self.down_since
            && now.mono.since(down.mono) >= OFFLINE_AFTER
        {
            return LinkPhase::Offline {
                since: down.wall,
                received_anything: self.received_anything,
            };
        }
        if self.server.inbox == crate::model::Known::NotYet {
            return LinkPhase::Connecting;
        }
        let backing_off = self.streams.values().filter_map(|h| match h.stream.phase {
            Phase::Backoff { due } => Some((due, &h.stream.last_cause)),
            _ => None,
        });
        let mut soonest: Option<Mono> = None;
        for (due, cause) in backing_off {
            if stopping(cause.as_ref()) {
                return LinkPhase::ServerStopped;
            }
            soonest = Some(soonest.map_or(due, |s| s.min(due)));
        }
        match soonest {
            Some(due) => LinkPhase::Reconnecting {
                in_secs: due.since(now.mono).as_millis().div_ceil(1000) as u64,
            },
            None => LinkPhase::Live,
        }
    }

    /// The soonest moment a timer is due.
    pub fn next_deadline(&self, now: Now) -> Option<Mono> {
        let mut at: Vec<Mono> = Vec::new();
        at.extend(self.calls.values().map(|c| c.deadline));
        if let Conn::Handshaking { due: Some(due), .. } = self.conn {
            at.push(due);
        }
        for held in self.streams.values() {
            match held.stream.phase {
                Phase::Backoff { due } => at.push(due),
                _ => at.extend(held.stream.silent_at()),
            }
        }
        if self.conn == Conn::Ready {
            for polls in self.polls.values() {
                at.extend(
                    [polls.conversations.due, polls.faults.due]
                        .into_iter()
                        .flatten(),
                );
            }
        }
        for e in &self.outbox.entries {
            if let WriteState::Unknown {
                next: Next::At(due),
                ..
            } = e.state
            {
                at.push(due);
            }
        }
        at.extend(self.linger.values().copied());
        at.extend(self.refetch.values().copied());
        at.extend(self.gap_retries.iter().map(|(due, ..)| *due));
        if let Some(down) = self.down_since {
            let offline = down.mono.after(OFFLINE_AFTER);
            if offline > now.mono {
                at.push(offline);
            }
        }
        if matches!(self.phase(now), LinkPhase::Reconnecting { .. }) {
            at.push(now.mono.after(Duration::from_secs(1)));
        }
        at.into_iter().min()
    }

    /// Fires every timer that is due.
    fn tick(&mut self, now: Now) {
        let expired: Vec<CallId> = self
            .calls
            .iter()
            .filter(|(_, c)| c.deadline <= now.mono)
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            if let Some(flight) = self.calls.remove(&id) {
                self.on_reply(flight.purpose, Outcome::Unknown(Unknown::Timeout), now);
            }
        }
        if let Conn::Handshaking { due: Some(due), .. } = self.conn
            && due <= now.mono
        {
            self.handshake(now);
        }
        self.tick_streams(now);
        self.tick_polls(now);
        self.tick_outbox(now);
        let due: Vec<DecisionRef> = self
            .refetch
            .iter()
            .filter(|(_, at)| **at <= now.mono)
            .map(|(d, _)| d.clone())
            .collect();
        for d in due {
            self.refetch.remove(&d);
            self.fetch_decision(d, now);
        }
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.gap_retries)
            .into_iter()
            .partition(|(at, ..)| *at <= now.mono);
        self.gap_retries = later;
        for (_, key, query) in due {
            self.history(
                Purpose::Gap {
                    key,
                    query: query.clone(),
                },
                &query,
                None,
                now,
            );
        }
    }

    /// After any input: keeps `down_since`, sends what waited for a connection, holds the streams that are wanted.
    fn settle(&mut self, was_connected: bool, now: Now) {
        let connected = self.connected();
        if connected {
            self.down_since = None;
        } else if self.down_since.is_none() {
            self.down_since = Some(now);
        }
        if connected && !was_connected {
            self.flush_waiting(now);
        }
        self.reconcile_streams(now);
    }

    /// A stream delivers: the link is up.
    fn connected(&self) -> bool {
        self.streams.values().any(|h| h.stream.is_live())
    }

    fn writes_on(&self) -> bool {
        self.conn == Conn::Ready && self.band.is_none() && self.connected()
    }

    fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    fn url(&self) -> String {
        self.server.url.clone().unwrap_or_default()
    }

    /// Makes one call with the device token.
    fn call(&mut self, purpose: Purpose, request: Request, now: Now) -> CallId {
        let id = CallId(self.next_seq());
        let url = self.url();
        self.effects.push(Effect::Call {
            id,
            url,
            auth: true,
            request,
        });
        self.calls.insert(
            id,
            InFlight {
                purpose,
                deadline: now.mono.after(CALL_TIMEOUT),
            },
        );
        id
    }

    fn in_flight(&self, pred: impl Fn(&Purpose) -> bool) -> bool {
        self.calls.values().any(|c| pred(&c.purpose))
    }

    fn on_reply(&mut self, purpose: Purpose, outcome: Outcome, now: Now) {
        match &outcome {
            Outcome::Done(_) => self.lift_band(now),
            Outcome::Refused(r)
                if r.code == RefusalCode::Unauthorized
                    && !matches!(purpose, Purpose::Pair { .. }) =>
            {
                self.band = Some(r.message.clone());
            }
            Outcome::Refused(_) => self.lift_band(now),
            Outcome::Unknown(Unknown::OtherProtocol(n))
                if !matches!(purpose, Purpose::Pair { .. }) =>
            {
                self.other_protocol(n.clone());
            }
            Outcome::Unknown(_) => {}
        }
        match purpose {
            Purpose::Pair { url } => self.on_paired(url, outcome, now),
            Purpose::Handshake => self.on_handshake(outcome, now),
            Purpose::Write(id) => self.on_write_reply(id, outcome, now),
            other => self.on_fetched(other, outcome, now),
        }
    }

    /// A call succeeded: the server accepts this device again.
    fn lift_band(&mut self, now: Now) {
        if self.band.take().is_some() && self.connected() {
            self.flush_waiting(now);
        }
    }

    fn other_protocol(&mut self, n: String) {
        self.conn = Conn::OtherProtocol(n);
        self.close_all();
    }

    fn random(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A ULID: 48 bits of time, 80 random, in Crockford base 32, after `r-`.
    fn request_id(&mut self, now: Now) -> RequestId {
        const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        let time = (now.wall.0.max(0) as u128) & ((1 << 48) - 1);
        let random =
            ((self.random() as u128) << 16 | (self.random() as u128 & 0xFFFF)) & ((1 << 80) - 1);
        let value = time << 80 | random;
        let text: String = (0..26)
            .map(|i| ALPHABET[((value >> (125 - 5 * i)) & 31) as usize] as char)
            .collect();
        RequestId::new(format!("r-{text}"))
    }
}

fn stopping(cause: Option<&EndCause>) -> bool {
    match cause {
        Some(EndCause::End(EndReason::Stopping)) => true,
        Some(EndCause::Refused(r)) => r.code == RefusalCode::Busy,
        _ => false,
    }
}
