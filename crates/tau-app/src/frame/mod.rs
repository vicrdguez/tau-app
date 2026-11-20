//! The window's frame (`spec.md` §4): the sidebar, the stage header, the stage with its notice
//! band, peek and details slots, the bottom float, and the bottom bar. Every part takes plain
//! data; none reads the app's state or the link to the server.

mod bar;
mod header;
mod sidebar;

use rune::{El, ScrollSpec, Transition, col, el, row};

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
    /// How the page makes room for the bottom float.
    pub room: Room,
    /// The peek column (Home and the places, stage at least 1000 px).
    pub peek: Option<El>,
    /// The details panel (`␣td`).
    pub details: Option<El>,
    /// The bottom float, built in its shell.
    pub float: Option<BottomFloat>,
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

/// The bottom float: one surface at the stage's foot, whose top edge glides on the `panel` spring
/// whenever its height changes (`visual-language.md` §7).
pub enum BottomFloat {
    Shown(El),
    /// Closed on this page: drawn inert while its edge glides down to the foot.
    Leaving(El),
}

/// How a page makes room for the bottom float (`spec.md` §4).
pub enum Room {
    /// The page lays out above the float, so what stands at its foot rides the edge (a
    /// conversation's queue and composer).
    Shrink,
    /// The page keeps its layout, and the element keyed here rides the edge once the edge comes
    /// within `float.inset_x` of it (a list's cursor row, the start page's composer).
    Lift(Option<String>),
}

/// The bottom float's key: its band of `surface.read` at the stage's foot, whose top edge is the
/// float's.
pub const FLOAT_BAND: &str = "bottom-float";

/// Lays the frame out at the window's size.
pub fn frame(t: Theme, p: Parts) -> El {
    let l = t.l;
    let [w, h] = p.window;
    let side_w = l.sidebar.width;
    let beside = p.sidebar_place == Some(SidebarPlace::Beside);
    let stage_x = if beside { side_w } else { 0.0 };
    let stage_w = w - stage_x;
    let stage_h = h - l.height.bottombar;
    let band_h = rune::memo::layout_with(FLOAT_BAND, |r| r[3]).unwrap_or(0.0);
    let page_w = stage_w
        - p.peek.as_ref().map_or(0.0, |_| l.peek.width)
        - p.details.as_ref().map_or(0.0, |_| l.details.width);

    let mut columns = row()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .min_w(0.0)
        .child(page_box(
            t,
            p.page,
            p.room,
            [page_w, stage_w],
            stage_h - l.stage_header.height,
            band_h,
            p.float.is_some(),
        ));
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
    // Always there, so a float rises from the foot: its height is the float's, and its fill hides
    // the page under the float's edge and in the insets beside and under it.
    let band = t
        .motion
        .moves(col().key(FLOAT_BAND), t.motion.panel)
        .transition(Transition::none())
        .pin(Some(0.0), None, Some(0.0), Some(0.0))
        .fill(t.c.surface.read)
        .px(l.float.inset_x);
    stage = stage.child(match p.float {
        Some(BottomFloat::Shown(f)) => band.pb(l.float.inset_bottom).child(f.max_h(0.4 * h)),
        // Overflowing a band of no height, it rides the band's top edge down behind the bar.
        Some(BottomFloat::Leaving(f)) => band.h(0.0).child(f.max_h(0.4 * h).inert().a11y_hidden()),
        None => band.h(0.0),
    });
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

/// The page's column. The page stands in a port whose foot is the float's top edge, as both glide
/// on one spring. A page that shrinks lays out in the port; one that lifts keeps its whole height,
/// and `sticky_bottom` holds its target above the port's foot, worked out by rune on every frame
/// from where the foot is drawn. `first` is the page's height until it has been laid out.
///
/// The port spans the stage, not just the page. It clips, and its width springs while the page
/// lands at its new width at once, so a port as wide as the page would cut the page short while
/// the stage widens.
fn page_box(
    t: Theme,
    page: El,
    room: Room,
    [page_w, stage_w]: [f32; 2],
    first: f32,
    band_h: f32,
    float: bool,
) -> El {
    let full = rune::memo::layout_with("page-box", |r| r[3]).unwrap_or(first);
    let port = (full - band_h).max(0.0);
    let page = page
        .w_full()
        .instant_changes()
        .transition(Transition::none());
    let page = match room {
        Room::Shrink => page.h(port),
        Room::Lift(target) => {
            let page = page.h(full);
            match target.filter(|_| float).and_then(|k| lift(t, &k, full)) {
                Some(inset) => page.sticky_bottom(inset),
                None => page,
            }
        }
    };
    // Sticky lifts within its parent's box: this one starts a page above the port and ends at
    // the port's foot, so the port never has anything to scroll.
    let holder = col()
        .w(page_w)
        .mt(-full)
        .h(full + port)
        .child(el().h(full))
        .child(page);
    t.motion
        .moves(col().key("page-box"), t.motion.panel)
        .grow(1.0)
        .min_w(0.0)
        .min_h(0.0)
        .child(
            col()
                .key("page-port")
                .w(stage_w)
                .h(port)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(holder),
        )
}

/// The `sticky_bottom` inset that keeps `target` above the port's foot by `float.inset_x`, or
/// by the room it has under it now if that is less, so it doesn't move until the edge reaches it.
fn lift(t: Theme, target: &str, full: f32) -> Option<f32> {
    let top = rune::memo::layout_with("page-box", |r| r[1])?;
    let under = top + full - rune::memo::window_layout_with(target, |r| r[1] + r[3])?;
    Some(t.l.float.inset_x.min(under) - under)
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
