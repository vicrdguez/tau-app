//! The handle the UI owns. It carries the engine's effects out on worker threads (calls, one thread per stream, a
//! timer) and queues what comes back. Server messages change the model only when the UI calls [`Link::pump`] at
//! its frame boundary, so keys always resolve against the frame the user saw (`reference/grammar.md` §7).

use crate::clock::{Clock, Mono, Now, SystemClock};
use crate::engine::{
    Effect, Engine, Input, LinkEvent, LinkStatus, Setup, Slot, StreamEvent, TranscriptKey,
};
use crate::fixture::{Fixture, FixtureTransport};
use crate::http::HttpTransport;
use crate::ids::{ConvRef, DecisionRef, Device, Project, WriteId};
use crate::model::Server;
use crate::outbox::{NotSent, Outbox, Write};
use crate::sse::Frame;
use crate::store::{ClientConfig, Secrets, Store};
use crate::streams::StreamId;
use crate::transport::{StreamHandle, Target, Transport};
use crate::wire::Token;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

/// Where the link keeps its state and reads its time.
#[derive(Clone)]
pub struct LinkConfig {
    /// `~/.config/tau`; `None` keeps nothing on disk.
    pub dir: Option<PathBuf>,
    pub secrets: Secrets,
    pub clock: Arc<dyn Clock>,
}

impl LinkConfig {
    /// `TAU_CLIENT_DIR` or `~/.config/tau`, the Keychain, and the system clock.
    /// `TAU_TOKEN_DIR` explicitly selects private token files instead of the Keychain.
    pub fn standard() -> LinkConfig {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        let dir = std::env::var_os("TAU_CLIENT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config").join("tau"));
        let secrets = std::env::var_os("TAU_TOKEN_DIR")
            .map(|path| Secrets::Files(PathBuf::from(path)))
            .unwrap_or_else(|| Secrets::standard(dir.join("tokens")));
        LinkConfig {
            secrets,
            dir: Some(dir),
            clock: Arc::new(SystemClock),
        }
    }
}

/// The UI's link to one server. See the module doc.
pub struct Link {
    engine: Engine,
    queue: Arc<Queue>,
    transport: Arc<dyn Transport>,
    streams: HashMap<StreamId, Box<dyn StreamHandle>>,
    store: Store,
    secrets: Secrets,
    token: Option<Token>,
    clock: Arc<dyn Clock>,
    timer: Timer,
    status: LinkStatus,
    events: Vec<LinkEvent>,
}

const OUTBOX: &str = "outbox.json";
const LABELS: &str = "labels.json";

impl Link {
    /// The server in `client.toml` with its token, or not paired.
    pub fn open(config: LinkConfig) -> Link {
        Link::with_transport(config, Arc::new(HttpTransport))
    }

    /// The design fixture as a server: every call and stream answered from `design/fixture.json`, nothing on disk.
    pub fn fixture(fixture: Fixture) -> Link {
        let secrets = Secrets::memory();
        let _ = secrets.save(&fixture.url(), &fixture.token());
        let config = LinkConfig {
            dir: None,
            secrets,
            clock: Arc::new(SystemClock),
        };
        let paired = Some((fixture.url(), Device::new(fixture.device())));
        Link::start(
            config,
            Store::none(),
            paired,
            Arc::new(FixtureTransport::new(fixture)),
        )
    }

    /// A link over any transport.
    pub fn with_transport(config: LinkConfig, transport: Arc<dyn Transport>) -> Link {
        let store = config.dir.clone().map_or_else(Store::none, Store::at);
        let client = store.client();
        let paired = client.server.zip(client.device.map(Device::new));
        Link::start(config, store, paired, transport)
    }

    fn start(
        config: LinkConfig,
        mut store: Store,
        paired: Option<(String, Device)>,
        transport: Arc<dyn Transport>,
    ) -> Link {
        let token = paired
            .as_ref()
            .and_then(|(url, _)| config.secrets.load(url));
        let paired = paired.filter(|_| token.is_some());
        if let Some((url, _)) = &paired {
            store.use_server(url);
        }
        let labels: Vec<(ConvRef, String)> = store.load(LABELS).unwrap_or_default();
        let setup = Setup {
            server: paired,
            outbox: store.load(OUTBOX).unwrap_or_default(),
            labels: labels.into_iter().collect(),
            seed: getrandom::u64().unwrap_or_else(|_| config.clock.now().wall.0 as u64),
        };
        let now = config.clock.now();
        let engine = Engine::new(setup, now);
        let queue = Arc::new(Queue::default());
        let timer = Timer::start(config.clock.clone(), queue.clone());
        let status = engine.status(now);
        let mut link = Link {
            engine,
            queue,
            transport,
            streams: HashMap::new(),
            store,
            secrets: config.secrets,
            token,
            clock: config.clock,
            timer,
            status,
            events: Vec::new(),
        };
        link.carry_out(now);
        link
    }

    /// Called from worker threads whenever something waits for [`Link::pump`].
    pub fn set_waker(&mut self, waker: impl Fn() + Send + Sync + 'static) {
        *self.queue.waker.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(waker));
    }

    /// Applies what arrived and fires due timers. True when anything the UI shows may have changed.
    pub fn pump(&mut self) -> bool {
        let now = self.clock.now();
        let inputs =
            std::mem::take(&mut *self.queue.inputs.lock().unwrap_or_else(|e| e.into_inner()));
        let mut changed = !inputs.is_empty();
        for input in inputs {
            if let Input::Stream { id, event } = &input
                && ended(event)
            {
                self.streams.remove(id);
            }
            self.engine.step(input, now);
        }
        self.engine.step(Input::Tick, now);
        changed |= self.carry_out(now);
        changed
    }

    /// How long until a timer is due, for a UI that wakes itself; the link's own timer calls the waker then too.
    pub fn next_deadline(&self) -> Option<Duration> {
        let now = self.clock.now();
        self.engine
            .next_deadline(now)
            .map(|due| due.since(now.mono))
    }

    pub fn server(&self) -> &Server {
        self.engine.server()
    }

    pub fn outbox(&self) -> &Outbox {
        self.engine.outbox()
    }

    pub fn status(&self) -> &LinkStatus {
        &self.status
    }

    pub fn take_events(&mut self) -> Vec<LinkEvent> {
        std::mem::take(&mut self.events)
    }

    /// This server's device-local files, for the UI's own state.
    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn write(&mut self, write: Write) -> Result<WriteId, NotSent> {
        self.command(|e, now| e.write(write, now))
    }

    pub fn retry(&mut self, id: WriteId) -> Result<(), NotSent> {
        self.command(|e, now| e.retry(id, now))
    }

    pub fn forget(&mut self, id: WriteId) {
        self.command(|e, now| e.forget(id, now))
    }

    pub fn show(&mut self, slot: Slot, conv: Option<ConvRef>) {
        self.command(|e, now| e.show(slot, conv, now))
    }

    pub fn watch_decision(&mut self, decision: Option<DecisionRef>) {
        self.command(|e, now| e.watch_decision(decision, now))
    }

    pub fn fetch_decision(&mut self, decision: DecisionRef) {
        self.command(|e, now| e.fetch_decision(decision, now))
    }

    pub fn fetch_options(&mut self, project: Project) {
        self.command(|e, now| e.fetch_options(project, now))
    }

    pub fn refresh_catalogue(&mut self) {
        self.command(|e, now| e.refresh_catalogue(now))
    }

    pub fn refresh_faults(&mut self) {
        self.command(|e, now| e.refresh_faults(now))
    }

    pub fn page_older(&mut self, key: TranscriptKey) {
        self.command(|e, now| e.page_older(key, now))
    }

    pub fn want_label(&mut self, conv: ConvRef) {
        self.command(|e, now| e.want_label(conv, now))
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.command(|e, now| e.set_focused(focused, now))
    }

    pub fn reconnect(&mut self) {
        self.command(|e, now| e.reconnect(now))
    }

    pub fn pair(&mut self, url: &str, code: &str) {
        self.command(|e, now| e.pair(url.to_owned(), code.to_owned(), now))
    }

    pub fn forget_server(&mut self) {
        self.command(|e, now| e.forget_server(now))
    }

    fn command<R>(&mut self, run: impl FnOnce(&mut Engine, Now) -> R) -> R {
        let now = self.clock.now();
        let result = run(&mut self.engine, now);
        self.carry_out(now);
        result
    }

    /// Carries the engine's effects out. True when anything came out of it.
    fn carry_out(&mut self, now: Now) -> bool {
        let effects = self.engine.take_effects();
        let events = self.engine.take_events();
        let mut changed = !effects.is_empty() || !events.is_empty();
        for effect in effects {
            self.effect(effect);
        }
        self.events.extend(events);
        let status = self.engine.status(now);
        changed |= status != self.status;
        self.status = status;
        self.timer.set(self.engine.next_deadline(now));
        changed
    }

    fn effect(&mut self, effect: Effect) {
        match effect {
            Effect::Call {
                id,
                url,
                auth,
                request,
            } => {
                let target = Target {
                    url,
                    token: if auth { self.token.clone() } else { None },
                };
                let queue = self.queue.clone();
                self.transport.call(
                    &target,
                    request,
                    Box::new(move |outcome| queue.push(Input::Reply { call: id, outcome })),
                );
            }
            Effect::Open { id, request } => {
                let target = Target {
                    url: self.engine.server().url.clone().unwrap_or_default(),
                    token: self.token.clone(),
                };
                let queue = self.queue.clone();
                let handle = self.transport.open(
                    &target,
                    request,
                    Box::new(move |event| queue.push(Input::Stream { id, event })),
                );
                self.streams.insert(id, handle);
            }
            Effect::Close { id } => {
                if let Some(stream) = self.streams.remove(&id) {
                    stream.close();
                }
            }
            Effect::SaveOutbox => {
                let _ = self.store.save(OUTBOX, &self.engine.outbox().persisted());
            }
            Effect::SaveLabels => {
                let labels: Vec<(&ConvRef, &String)> =
                    self.engine.server().labels.found.iter().collect();
                let _ = self.store.save(LABELS, &labels);
            }
            Effect::SaveCredentials { url, device, token } => {
                let _ = self.secrets.save(&url, &token);
                let ClientConfig { theme, .. } = self.store.client();
                let _ = self.store.save_client(&ClientConfig {
                    server: Some(url.clone()),
                    device: Some(device.to_string()),
                    theme,
                });
                self.store.use_server(&url);
                self.token = Some(token);
            }
            Effect::ForgetServer { url } => {
                let _ = self.secrets.delete(&url);
                let _ = self.store.forget_server();
                self.token = None;
            }
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        for stream in self.streams.values() {
            stream.close();
        }
    }
}

/// A stream event after which the stream delivers nothing more.
fn ended(event: &StreamEvent) -> bool {
    matches!(
        event,
        StreamEvent::Closed(_) | StreamEvent::Refused(_) | StreamEvent::Frame(Frame::End(_))
    )
}

type Waker = Arc<dyn Fn() + Send + Sync>;

/// What worker threads hand to the UI thread, and how they wake it.
#[derive(Default)]
struct Queue {
    inputs: Mutex<Vec<Input>>,
    waker: Mutex<Option<Waker>>,
}

impl Queue {
    fn push(&self, input: Input) {
        self.inputs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(input);
        self.wake();
    }

    fn wake(&self) {
        let waker = self.waker.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let Some(waker) = waker {
            waker();
        }
    }
}

/// Wakes the UI when the engine's next deadline passes, on the link's clock. It checks at least four times a
/// second, so a deadline that passed during sleep is noticed on wake.
struct Timer {
    shared: Arc<(Mutex<TimerState>, Condvar)>,
}

#[derive(Default)]
struct TimerState {
    due: Option<Mono>,
    stop: bool,
}

const TIMER_GRAIN: Duration = Duration::from_millis(250);

impl Timer {
    fn start(clock: Arc<dyn Clock>, queue: Arc<Queue>) -> Timer {
        let shared = Arc::new((Mutex::new(TimerState::default()), Condvar::new()));
        let mine = shared.clone();
        thread::spawn(move || {
            let (lock, wakeup) = &*mine;
            let mut state = lock.lock().unwrap_or_else(|e| e.into_inner());
            while !state.stop {
                match state.due {
                    None => state = wakeup.wait(state).unwrap_or_else(|e| e.into_inner()),
                    Some(due) => {
                        let now = clock.now().mono;
                        if now >= due {
                            state.due = None;
                            drop(state);
                            queue.wake();
                            state = lock.lock().unwrap_or_else(|e| e.into_inner());
                        } else {
                            let wait = due.since(now).min(TIMER_GRAIN);
                            state = wakeup
                                .wait_timeout(state, wait)
                                .unwrap_or_else(|e| e.into_inner())
                                .0;
                        }
                    }
                }
            }
        });
        Timer { shared }
    }

    fn set(&self, due: Option<Mono>) {
        let (lock, wakeup) = &*self.shared;
        lock.lock().unwrap_or_else(|e| e.into_inner()).due = due;
        wakeup.notify_one();
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let (lock, wakeup) = &*self.shared;
        lock.lock().unwrap_or_else(|e| e.into_inner()).stop = true;
        wakeup.notify_one();
    }
}
