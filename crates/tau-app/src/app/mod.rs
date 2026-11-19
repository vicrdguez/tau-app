//! The app's state and its rune wiring: the view, `poll`, the key handler and the scenes for
//! headless renders. `App` owns the link to the server (a `tau_client::Link`); `poll` pumps it
//! and fills `App::server`, the plain data the frame reads. What the stage shows is a screen's
//! (`crate::screens`).

pub mod commands;
pub mod input;
pub mod link;
pub(crate) mod local;
pub mod scenes;
mod server;
mod state_scenes;

use std::time::Duration;

use rune::{AppSpec, Cx, El, TextEdit, Tx};

pub use commands::{Handler, Invocation};
pub use server::{Hidden, ItemFacts, ServerView, conv_ref, decision_ref};

use tau_client::{Link, LinkPhase};

use crate::floats::{self, Float};
use crate::frame::{self, Bar, Header, Parts, Place, Row, Sidebar, SidebarPlace};
use crate::keys::{Keymap, Keys, Mode};
use crate::open::{Item, OpenItems};
use crate::screens::{self, Screens};
use crate::settings::Settings;
use crate::theme::{Theme, tokens};
use crate::ui::{Badge, Scope};

/// What the stage shows: one page. `crate::screens::of` names the screen family that draws it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Stage {
    /// First run, and after `:forget-server` or `␣rp`.
    Connect,
    /// The start page (`␣cn`).
    Start,
    Place(Place),
    Item(Item),
}

/// The region that has the keys, besides a float.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Focus {
    #[default]
    Stage,
    Sidebar,
}

pub struct App {
    pub settings: Settings,
    /// The system's Reduce Motion, as rune last reported it.
    pub reduced_motion: bool,
    /// The window's size in points, as last laid out.
    pub window: [f32; 2],
    /// Seconds since launch (the virtual clock when headless).
    pub now: f64,
    pub focus: Focus,
    /// The floats, the top one holding the keys, each with the region that opened it.
    pub floats: Vec<(Float, Focus)>,
    pub keys: Keys,
    /// Whether which-key was drawn, so `poll` redraws when it comes due.
    pub which_key_shown: bool,
    reveal_float_target: bool,
    pub stage: Stage,
    /// The open item shown last, for `]b [b` from a place.
    pub last_item: Option<Item>,
    pub open: OpenItems,
    /// The stored choice: the sidebar is shown beside the stage when there is room.
    pub sidebar_visible: bool,
    pub sidebar_cursor: Row,
    pub details_visible: bool,
    pub echo: Option<String>,
    /// Every echo, for `:messages`.
    pub messages: Vec<String>,
    /// The `:` line's history, oldest first.
    pub history: Vec<String>,
    /// The link to the server. Screens may read its model (`server()`, `outbox()`); the frame
    /// and `ui` never see it.
    pub link: Link,
    /// Pump the link in `poll`. Tests and scenes freeze it at the state they set up.
    pub live: bool,
    /// What the frame shows of the server, refreshed from the link in `poll`.
    pub server: ServerView,
    /// Failures put aside on this device.
    pub hidden: Hidden,
    /// Decisions whose titles were asked for this launch.
    asked: link::Asked,
    /// Each screen family's own state.
    pub screens: Screens,
    /// The places jumps left, kept on this device (`grammar.md` §13).
    pub jumps: crate::nav::Jumps,
    /// `y` or `␣y` waiting for its object.
    pub yank: Option<crate::yank::Pending>,
    /// What `y` yanked last.
    pub register: Option<String>,
    /// Hint mode's labels while it is on.
    pub hints: Option<crate::nav::hints::Hints>,
    /// What a float's input edits when no float is open.
    scratch: TextEdit,
}

/// Home, on the design's fixture, frozen: the state tests and scenes start from.
impl Default for App {
    fn default() -> Self {
        App {
            live: false,
            ..App::new(link::demo())
        }
    }
}

impl App {
    fn new(link: Link) -> App {
        App {
            settings: Settings::default(),
            reduced_motion: false,
            window: tokens().layout.window,
            now: 0.0,
            focus: Focus::Stage,
            floats: Vec::new(),
            keys: Keys::default(),
            which_key_shown: false,
            reveal_float_target: false,
            stage: Stage::Place(Place::Home),
            last_item: None,
            open: OpenItems::default(),
            sidebar_visible: true,
            sidebar_cursor: Row::Place(Place::Home),
            details_visible: false,
            echo: None,
            messages: Vec::new(),
            history: Vec::new(),
            link,
            live: true,
            server: ServerView::default(),
            hidden: Hidden::default(),
            asked: link::Asked::default(),
            screens: Screens::default(),
            jumps: crate::nav::Jumps::default(),
            yank: None,
            register: None,
            hints: None,
            scratch: TextEdit::default(),
        }
    }

    /// The window at launch: the start page (D11), or Connect while the link isn't paired.
    pub fn launch(link: Link) -> App {
        let mut app = App::new(link);
        local::load(&mut app);
        crate::settings::load(&mut app);
        let paired = !matches!(
            app.link.status().phase,
            LinkPhase::NotPaired | LinkPhase::Pairing
        );
        app.show(if paired { Stage::Start } else { Stage::Connect });
        app
    }

    pub fn theme(&self) -> Theme {
        self.settings.theme(self.reduced_motion)
    }

    /// Where the keys are: the top float, else the sidebar, else the stage's page.
    pub fn scope(&self) -> Scope {
        match (self.floats.last(), self.focus) {
            (Some((f, _)), _) => f.scope(),
            (None, _) if self.hints.is_some() => Scope {
                mode: Mode::Hint,
                ctx: crate::keys::Ctx::Hint,
            },
            (None, Focus::Sidebar) => Scope {
                mode: Mode::Normal,
                ctx: crate::keys::Ctx::Sidebar,
            },
            (None, Focus::Stage) => {
                let page = screens::page(self);
                Scope {
                    mode: page.mode(self),
                    ctx: page.ctx(self),
                }
            }
        }
    }

    pub fn badge(&self) -> Badge {
        match (self.floats.last(), self.focus) {
            (Some((f, _)), _) => f.badge(),
            (None, _) if self.hints.is_some() => Badge::Hint,
            (None, Focus::Sidebar) => Badge::Normal,
            (None, Focus::Stage) => screens::page(self).badge(self),
        }
    }

    /// Whether the window has room for the sidebar beside the stage.
    pub fn sidebar_room(&self) -> bool {
        self.window[0] >= tokens().layout.sidebar.autohide_below_window
    }

    pub fn sidebar_place(&self) -> Option<SidebarPlace> {
        self.sidebar_place_at(self.window[0])
    }

    /// Where the sidebar is drawn in a window `width` wide: beside the stage by its stored choice
    /// when there is room, else over the stage while it (or a float it opened) has the keys.
    pub fn sidebar_place_at(&self, width: f32) -> Option<SidebarPlace> {
        let has_keys = match self.floats.last() {
            Some((_, opener)) => *opener == Focus::Sidebar,
            None => self.focus == Focus::Sidebar,
        };
        if self.sidebar_visible && width >= tokens().layout.sidebar.autohide_below_window {
            Some(SidebarPlace::Beside)
        } else if has_keys {
            Some(SidebarPlace::Over)
        } else {
            None
        }
    }

    /// The sidebar row of what the stage shows; Faults shows as Home, the start page as `+ New`.
    pub fn current_row(&self) -> Row {
        match &self.stage {
            Stage::Connect | Stage::Start => Row::New,
            Stage::Place(Place::Faults) => Row::Place(Place::Home),
            Stage::Place(p) => Row::Place(*p),
            Stage::Item(i) => Row::Item(i.clone()),
        }
    }

    /// Shows `stage` and gives the stage the keys; an item shown is opened.
    pub fn show(&mut self, stage: Stage) {
        local::remember_project(self);
        if let Stage::Item(item) = &stage {
            self.open.open(item.clone());
            self.last_item = Some(item.clone());
        }
        self.stage = stage;
        self.focus = Focus::Stage;
        // Labels and a waiting operator belong to the page they were made on.
        self.hints = None;
        self.yank = None;
        link::follow_stage(self);
        screens::page(self).shown(self);
    }

    pub fn say(&mut self, message: impl Into<String>) {
        let m = message.into();
        self.messages.push(m.clone());
        self.echo = Some(m);
    }

    pub fn open_float(&mut self, f: Float) {
        self.reveal_float_target = true;
        self.keys.reset();
        let opener = self.floats.last().map_or(self.focus, |(_, o)| *o);
        self.floats.push((f, opener));
    }

    pub fn close_float(&mut self) -> Option<Float> {
        self.floats.pop().map(|(f, _)| f)
    }

    /// The top float's input, for its field's `on_edit`.
    pub fn float_edit(&mut self) -> &mut TextEdit {
        match self.floats.last_mut().and_then(|(f, _)| f.edit_mut()) {
            Some(e) => e,
            None => &mut self.scratch,
        }
    }

    /// The sidebar's rows top to bottom, and where each of its sections starts.
    pub fn sidebar_rows(&self) -> (Vec<Row>, Vec<usize>) {
        let mut rows = vec![Row::New, Row::Find];
        let mut sections = vec![0, 2];
        rows.extend(
            [
                Place::Home,
                Place::Conversations,
                Place::Timeline,
                Place::Projects,
            ]
            .map(Row::Place),
        );
        for (_, items) in self.open.groups() {
            sections.push(rows.len());
            rows.extend(items.iter().cloned().map(Row::Item));
        }
        sections.push(rows.len());
        rows.push(Row::Server);
        (rows, sections)
    }
}

/// Builds the window from the state.
pub fn view(app: &App, cx: &mut Cx<App>) -> El {
    let reduced = cx.is_reduced_motion();
    let t = app.settings.theme(reduced);
    // The first frame lays out before `on_layout` reports the size: draw at the real one.
    let size = cx.layout_size("root").unwrap_or(app.window);
    let scope = app.scope();
    let page = screens::page(app);
    let framed = page.framed();
    let place = app.sidebar_place_at(size[0]).filter(|_| framed);
    let groups = app.server.open_groups(&app.open);
    let sidebar = framed.then(|| {
        let s = Sidebar {
            server: &app.server.phase,
            counts: app.server.counts,
            current: Some(app.current_row()),
            cursor: (app.focus == Focus::Sidebar).then(|| app.sidebar_cursor.clone()),
            open: &groups,
            scope,
        };
        frame::sidebar(t, &s, commands::open_row)
    });
    let header = if framed {
        frame::header(
            t,
            &Header {
                sidebar_shown: place == Some(SidebarPlace::Beside),
                crumbs: page.crumbs(app),
                segments: page.segments(app),
                answer_solid: app.floats.is_empty() && !app.which_key_shown,
                scope,
            },
            |a: &mut App, tx| commands::run_id(a, "toggle.sidebar", tx),
            commands::press_key,
        )
    } else {
        rune::el().fill(t.c.surface.read)
    };
    let stage_w = size[0]
        - if place == Some(SidebarPlace::Beside) {
            t.l.sidebar.width
        } else {
            0.0
        };
    let peek = (page.has_peek(app) && stage_w >= t.l.peek.from_stage_px)
        .then(|| screens::get(screens::ScreenId::Peek).view(t, app));
    let details = (page.has_details(app) && app.details_visible).then(|| page.details(t, app));
    let float_w = stage_w - 2.0 * t.l.float.inset_x;
    let float = match app.floats.last() {
        Some((Float::Screen(f), _)) => Some(f.view(t, app)),
        Some((f, opener)) => {
            let ctx = match opener {
                Focus::Sidebar => crate::keys::Ctx::Sidebar,
                Focus::Stage => page.ctx(app),
            };
            Some(floats::view(t, f, ctx, &app.messages, &app.settings))
        }
        None if app.which_key_shown => app
            .settings
            .keymap
            .which_key(app.keys.pending(), scope.mode, scope.ctx)
            .map(|wk| {
                let why = |id: &str| page.unavailable(app, id);
                floats::which_key::view(t, &wk, scope.ctx, &why, float_w)
            }),
        None => None,
    }
    .map(|f| f.on_layout_tx(|_: &mut App, _, tx| input::reveal_float_cursor(tx)));
    let bar = frame::bar(
        t,
        Bar {
            badge: app.badge(),
            pending: crate::yank::shown(app) + &app.keys.shown(),
            echo: app.echo.as_deref(),
            save: page.save(t, app),
            right: (place.is_none() && framed).then_some((app.server.counts, &app.server.phase)),
        },
        commands::open_row,
    );
    frame::frame(
        t,
        Parts {
            window: size,
            sidebar,
            sidebar_place: place,
            header,
            notice: app
                .server
                .notice
                .as_ref()
                .map(|n| frame::notice(t, scope, n)),
            page: page.view(t, app),
            peek,
            details,
            float,
            bar,
            overlay: crate::nav::hints::overlay(t, app, cx),
            lights: rune::is_headless(),
        },
    )
    .on_layout(move |s: &mut App, r| {
        s.window = [r[2], r[3]];
        s.reduced_motion = reduced;
    })
}

/// Runs at the start of every frame: keeps the clock, shows which-key when its delay is up, pumps
/// the link into `App::server`, and runs each screen's `poll`.
pub fn poll(app: &mut App, now: f64, tx: &mut Tx) -> bool {
    app.now = now;
    let due = app.floats.is_empty()
        && app
            .keys
            .which_key_due(now, app.settings.keymap.whichkey_delay_ms());
    let reveal = std::mem::take(&mut app.reveal_float_target) || (due && !app.which_key_shown);
    let mut changed = due != app.which_key_shown;
    app.which_key_shown = due;
    changed |= link::pump(app, tx);
    for id in screens::ALL {
        changed |= screens::get(id).poll(app, now, tx);
    }
    if reveal {
        screens::page(app).reveal_target(app, tx);
    }
    changed
}

/// The rune app: the window, its fonts, the view, the keys and the scenes.
pub fn spec(fonts: &'static [&'static [u8]]) -> AppSpec<App> {
    let l = tokens().layout;
    AppSpec {
        min_size: Some((l.min_window[0], l.min_window[1])),
        scenes: scenes::all(),
        fonts,
        families: crate::theme::FAMILIES,
        poll: Some(poll),
        setup: Some(link::setup),
        on_key: Some(input::on_key),
        ..AppSpec::new("tau", (l.window[0], l.window[1]), App::default, view)
    }
}

/// How long a pending prefix waits before which-key shows.
pub fn which_key_delay(km: Keymap) -> Duration {
    Duration::from_secs_f64(km.whichkey_delay_ms() / 1000.0)
}

#[cfg(test)]
mod float_tests;
#[cfg(test)]
mod local_tests;
#[cfg(test)]
mod reading_layout_tests;
#[cfg(test)]
mod server_tests;
#[cfg(test)]
mod state_scene_tests;
#[cfg(test)]
pub(crate) mod testing;
#[cfg(test)]
mod tests;
