//! The window's frame (`spec.md` §4): the sidebar, the stage header, the stage with its notice
//! band, peek and details slots, the bottom float, and the bottom bar. Every part takes plain
//! data; none reads the app's state or the link to the server.

mod bar;
mod header;
mod sidebar;

use rune::{El, Transition, col, el, row};

pub use bar::{Bar, bar};
pub use header::{Crumb, Header, Segment, SegmentKind, header};
pub use sidebar::{OpenGroup, OpenRow, Row, Sidebar, row_key, sidebar};

use crate::theme::{Motion, Theme};
use crate::ui::{NoticeAction, Scope};

/// A place: a page of every project (`spec.md` §3.1). Faults has no sidebar row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Place {
    Home,
    Conversations,
    Timeline,
    Projects,
    Faults,
}

impl Place {
    pub fn name(self) -> &'static str {
        match self {
            Place::Home => "Home",
            Place::Conversations => "Conversations",
            Place::Timeline => "Timeline",
            Place::Projects => "Projects",
            Place::Faults => "Faults",
        }
    }
}

/// The mark before the server line's words when it isn't calm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TroubleMark {
    /// `○`: connecting, reconnecting, offline.
    Ring,
    /// `◷`: the server stopped.
    Time,
    /// `×`: refused, revoked, another protocol.
    Failed,
}

/// The server line's state (`spec.md` §4.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerPhase {
    /// Before pairing.
    NotConnected,
    /// Calm: `laptop → tau.tail4e2f.ts.net ⌄`.
    Calm { device: String, host: String },
    /// Not calm: its mark and its words (`connecting`, `offline 6 m`, `revoked`).
    Trouble { mark: TroubleMark, words: String },
}

impl ServerPhase {
    /// From launch until the first `inbox` frame (`spec.md` §4.8).
    pub fn connecting() -> ServerPhase {
        ServerPhase::Trouble {
            mark: TroubleMark::Ring,
            words: "connecting".into(),
        }
    }
}

/// What a notice band says (`spec.md` §4): offline, refused, revoked, superseded, a stream that
/// keeps failing. A band whose mark is the failed `×` gets the rose rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub mark: Option<TroubleMark>,
    pub sentence: String,
    /// The key and word at its right (`␣rc reconnect`).
    pub action: Option<NoticeAction>,
}

pub fn notice(t: Theme, scope: Scope, n: &Notice) -> El {
    crate::ui::notice(
        t,
        scope,
        n.mark.map(|m| sidebar::trouble_mark(t, m)),
        &n.sentence,
        n.action,
        n.mark == Some(TroubleMark::Failed),
    )
}

/// Everything the frame lays out, already built.
pub struct Parts {
    pub window: [f32; 2],
    /// The sidebar beside the stage, or over it while it has the keys with no room beside.
    pub sidebar: Option<El>,
    pub sidebar_place: Option<SidebarPlace>,
    pub header: El,
    pub notice: Option<El>,
    pub page: El,
    /// The peek column (Home and the places, stage at least 1000 px).
    pub peek: Option<El>,
    /// The details panel (`␣td`).
    pub details: Option<El>,
    /// The bottom float, built in its shell.
    pub float: Option<El>,
    pub bar: El,
    /// Hint mode's labels, over everything.
    pub overlay: Option<El>,
    /// The traffic lights, drawn only in headless renders (the window's own are native).
    pub lights: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarPlace {
    Beside,
    Over,
}

/// Lays the frame out at the window's size.
pub fn frame(t: Theme, p: Parts) -> El {
    let l = t.l;
    let [w, h] = p.window;
    let side_w = l.sidebar.width;
    let beside = p.sidebar_place == Some(SidebarPlace::Beside);
    let stage_x = if beside { side_w } else { 0.0 };
    let stage_w = w - stage_x;
    let stage_h = h - l.height.bottombar;

    let mut columns = row().w_full().grow(1.0).min_h(0.0).min_w(0.0).child(
        t.motion
            .moves(col().key("page-box"), t.motion.panel)
            .grow(1.0)
            .min_w(0.0)
            .min_h(0.0)
            .child(
                p.page
                    .w_full()
                    .h_full()
                    .instant_changes()
                    .transition(Transition::none()),
            ),
    );
    if let Some(peek) = p.peek {
        columns = columns.child(
            peek.w(l.peek.width)
                .h_full()
                .border_sides([0.0, 0.0, 0.0, 1.0], t.c.border.hairline),
        );
    }
    if let Some(details) = p.details {
        columns = columns.child(
            t.motion
                .moves(col().key("details"), t.motion.panel)
                .transition(
                    Transition::offset(l.details.width, 0.0).springs(Motion::pair(t.motion.panel)),
                )
                .child(
                    details
                        .w_full()
                        .h_full()
                        .instant_changes()
                        .border_sides([0.0, 0.0, 0.0, 1.0], t.c.border.hairline),
                )
                .w(l.details.width)
                .h_full(),
        );
    }
    let mut body = col()
        .key("stage-body")
        .instant_changes()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .clip_rounded(0.0);
    if let Some(n) = p.notice {
        body = body.child(n);
    }
    body = body.child(columns);
    let mut stage = t
        .motion
        .moves(col().key("stage"), t.motion.panel)
        .absolute(stage_x, 0.0)
        .size(stage_w, stage_h)
        .fill(t.c.surface.read)
        .child(
            p.header
                .instant_changes()
                .w_full()
                .h(l.stage_header.height)
                .border_sides([0.0, 0.0, 1.0, 0.0], t.c.border.hairline),
        )
        .child(body);
    if let Some(float) = p.float {
        // The page stops at the float's top edge: the insets beside and under it show
        // `surface.read`, never a line of the page.
        stage = stage.child(
            col()
                .key("bottom-float")
                .instant_changes()
                .transition(Transition::none())
                .w_full()
                .fill(t.c.surface.read)
                .px(l.float.inset_x)
                .pb(l.float.inset_bottom)
                .child(float.max_h(0.4 * h)),
        );
    }
    let mut root = el()
        .key("root")
        .instant_changes()
        .w_full()
        .h_full()
        .fill(t.c.surface.read)
        .child(stage);
    if let Some(side) = p.sidebar {
        let side = if p.sidebar_place == Some(SidebarPlace::Over) {
            side.border_sides([0.0, 1.0, 0.0, 0.0], t.c.border.hairline)
        } else {
            side
        };
        // Rune always fades removed nodes, exposing page text through an exiting sidebar.
        let mut side = t
            .motion
            .moves(col().key("sidebar"), t.motion.panel)
            .transition(Transition::none())
            .offset(
                if p.sidebar_place.is_some() {
                    0.0
                } else {
                    -side_w
                },
                0.0,
            )
            .absolute(0.0, 0.0)
            .size(side_w, stage_h)
            .child(side.w_full().h_full().instant_changes());
        if p.sidebar_place.is_none() {
            side = side.inert().a11y_hidden();
        }
        root = root.child(side);
    }
    root = root.child(p.bar.absolute(0.0, stage_h).size(w, l.height.bottombar));
    if p.lights {
        root = root.child(traffic_lights(t));
    }
    if let Some(overlay) = p.overlay {
        root = root.child(overlay);
    }
    if !t.motion.on {
        root = root.token_motion(rune::Spring::NONE);
    }
    root
}

/// What macOS draws at the window's top left, for headless renders only.
fn traffic_lights(t: Theme) -> El {
    let [x, y] = t.l.sidebar.traffic_lights;
    let dot = |hex: &str| {
        el().size(12.0, 12.0)
            .radius(6.0)
            .fill(rune::color::hex(hex).unwrap_or(rune::Color::BLACK))
    };
    row()
        .absolute(x, y)
        .gap(8.0)
        .child(dot("#ff5f57"))
        .child(dot("#febc2e"))
        .child(dot("#28c840"))
}

#[cfg(test)]
mod tests;
