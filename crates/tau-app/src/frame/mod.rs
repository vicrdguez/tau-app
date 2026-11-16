//! The window's frame (`spec.md` §4): the sidebar, the stage header, the stage with its notice
//! band, peek and details slots, the bottom float, and the bottom bar. Every part takes plain
//! data; none reads the app's state or the link to the server.

mod bar;
mod header;
mod sidebar;

use rune::{El, Spring, SpringPair, Transition, col, el, row};

pub use bar::{Bar, bar};
pub use header::{Crumb, Header, Segment, SegmentKind, header};
pub use sidebar::{OpenGroup, OpenRow, Row, Sidebar, row_key, sidebar};

use crate::theme::Theme;
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
    pub sidebar: Option<(El, SidebarPlace)>,
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
    let beside = matches!(p.sidebar, Some((_, SidebarPlace::Beside)));
    let stage_x = if beside { side_w } else { 0.0 };
    let stage_w = w - stage_x;
    let stage_h = h - l.height.bottombar;

    let mut columns = row()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .min_w(0.0)
        .child(p.page.grow(1.0).min_w(0.0).min_h(0.0));
    if let Some(peek) = p.peek {
        columns = columns.child(
            peek.w(l.peek.width)
                .h_full()
                .border_sides([0.0, 0.0, 0.0, 1.0], t.c.border.hairline),
        );
    }
    if let Some(details) = p.details {
        columns = columns.child(
            details
                .w(l.details.width)
                .h_full()
                .border_sides([0.0, 0.0, 0.0, 1.0], t.c.border.hairline),
        );
    }
    let mut body = col()
        .key("stage-body")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .clip_rounded(0.0);
    if let Some(n) = p.notice {
        body = body.child(n);
    }
    body = body.child(columns);
    let mut stage = col()
        .key("stage")
        .absolute(stage_x, 0.0)
        .size(stage_w, stage_h)
        .fill(t.c.surface.read)
        .child(
            p.header
                .w_full()
                .h(l.stage_header.height)
                .border_sides([0.0, 0.0, 1.0, 0.0], t.c.border.hairline),
        )
        .child(body);
    if let Some(float) = p.float {
        // The page stops at the float's top edge: the insets beside and under it show
        // `surface.read`, never a line of the page.
        let f = l.float_fade_ms / 1000.0;
        stage = stage.child(
            col()
                .key("bottom-float")
                .animated_changes()
                .transition(
                    Transition::opacity()
                        .springs(SpringPair::new(Spring::new(f, 0.0), Spring::new(f, 0.0))),
                )
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
    if let Some((side, place)) = p.sidebar {
        let side = side.key("sidebar").absolute(0.0, 0.0).size(side_w, stage_h);
        root = root.child(match place {
            SidebarPlace::Beside => side,
            SidebarPlace::Over => side.border_sides([0.0, 1.0, 0.0, 0.0], t.c.border.hairline),
        });
    }
    root = root.child(p.bar.absolute(0.0, stage_h).size(w, l.height.bottombar));
    if p.lights {
        root = root.child(traffic_lights(t));
    }
    if let Some(overlay) = p.overlay {
        root = root.child(overlay);
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
