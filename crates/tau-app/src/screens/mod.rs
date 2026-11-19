//! The screens, one family per file: what each page declares for the frame and the keys, its view,
//! its commands and its floats. A screen family edits only its own file (or folder); the app
//! reaches every family through [`Screen`] and the one table in [`of`].

pub mod connect;
pub mod conversation;
pub mod decision;
pub mod home;
pub mod peek;
pub mod places;
pub mod project;
pub mod start;

use std::any::Any;

use rune::{El, TextEdit, Tx, col, text};
use tau_client::LinkEvent;

use crate::app::scenes::Scene;
use crate::app::{App, Focus, Handler, Stage};
use crate::floats::Float;
use crate::frame::{Crumb, Place, Segment};
use crate::keys::{Ctx, Key, Mode};
use crate::open::Item;
use crate::theme::Theme;
use crate::ui::{Badge, Scope};

/// A screen family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenId {
    Connect,
    Start,
    Home,
    /// Conversations, Timeline and Faults.
    Places,
    /// The Projects place and the Project view.
    Project,
    /// The peek column of Home and the places, and the peek float.
    Peek,
    Conversation,
    Decision,
}

pub const ALL: [ScreenId; 8] = [
    ScreenId::Connect,
    ScreenId::Start,
    ScreenId::Home,
    ScreenId::Places,
    ScreenId::Project,
    ScreenId::Peek,
    ScreenId::Conversation,
    ScreenId::Decision,
];

/// The family that draws `stage`.
pub fn of(stage: &Stage) -> ScreenId {
    match stage {
        Stage::Connect => ScreenId::Connect,
        Stage::Start => ScreenId::Start,
        Stage::Place(Place::Home) => ScreenId::Home,
        Stage::Place(Place::Projects) | Stage::Item(Item::Project(_)) => ScreenId::Project,
        Stage::Place(_) => ScreenId::Places,
        Stage::Item(Item::Conversation { .. }) => ScreenId::Conversation,
        Stage::Item(Item::Decision { .. }) => ScreenId::Decision,
    }
}

pub fn get(id: ScreenId) -> &'static dyn Screen {
    match id {
        ScreenId::Connect => &connect::Connect,
        ScreenId::Start => &start::Start,
        ScreenId::Home => &home::Home,
        ScreenId::Places => &places::Places,
        ScreenId::Project => &project::Project,
        ScreenId::Peek => &peek::Peek,
        ScreenId::Conversation => &conversation::Conversation,
        ScreenId::Decision => &decision::Decision,
    }
}

/// The screen that draws the stage now.
pub fn page(app: &App) -> &'static dyn Screen {
    get(of(&app.stage))
}

/// Who has the keys among the screens: the owner of the top float, else the page while the stage
/// has the keys. `None` while the sidebar or a frame float (palette, `:` line, Confirm) has them.
pub fn holder(app: &App) -> Option<ScreenId> {
    match (app.floats.last(), app.focus) {
        (Some((Float::Screen(f), _)), _) => Some(f.owner()),
        (Some(_), _) | (None, Focus::Sidebar) => None,
        (None, Focus::Stage) => Some(of(&app.stage)),
    }
}

/// Each family's own state, in its own file.
#[derive(Default)]
pub struct Screens {
    pub connect: connect::State,
    pub start: start::State,
    pub home: home::State,
    pub places: places::State,
    pub project: project::State,
    pub peek: peek::State,
    pub conversation: conversation::State,
    pub decision: decision::State,
}

/// What a screen family declares. Every method but `ctx` and `view` has a default, so a family
/// fills in only what it has. A method that names the page (`ctx`, `crumbs`, `view`) runs while
/// `app.stage` is one of the family's stages; the peek's runs for the peek column and its float.
pub trait Screen: Sync {
    fn moving(&self, _app: &App) -> u64 {
        0
    }

    /// The keymap context while the page has the keys.
    fn ctx(&self, app: &App) -> Ctx;

    /// The mode while the page has the keys; a page with an editor reports the editor's.
    fn mode(&self, _app: &App) -> Mode {
        Mode::Normal
    }

    /// The mode badge while the page has the keys.
    fn badge(&self, app: &App) -> Badge {
        match self.mode(app) {
            Mode::Insert => Badge::Insert,
            Mode::Visual => Badge::Visual,
            _ => Badge::Normal,
        }
    }

    /// The breadcrumb (`spec.md` §4.2).
    fn crumbs(&self, _app: &App) -> Vec<Crumb> {
        Vec::new()
    }

    /// The header's action segments at its right.
    fn segments(&self, _app: &App) -> Vec<Segment> {
        Vec::new()
    }

    /// Whether the page has the peek column (Home and the places).
    fn has_peek(&self, _app: &App) -> bool {
        false
    }

    /// Whether the page has a details panel (`␣td`).
    fn has_details(&self, _app: &App) -> bool {
        false
    }

    /// False for a page drawn without the sidebar, the sidebar toggle and the breadcrumb.
    fn framed(&self) -> bool {
        true
    }

    /// The page, filling the stage below the header and the notice band.
    fn view(&self, t: Theme, app: &App) -> El;

    /// The page's option-draft indicator in the bottom bar.
    fn save(&self, _t: Theme, _app: &App) -> Option<El> {
        None
    }

    /// The details panel's content.
    fn details(&self, t: Theme, _app: &App) -> El {
        rune::el().fill(t.c.surface.read)
    }

    /// Why a command the keys reach on this page can't run now, in which-key's words
    /// (`grammar.md` §5: `not answered`, `its work did not fail`); `None` when it can.
    fn unavailable(&self, _app: &App, _id: &str) -> Option<String> {
        None
    }

    /// Handlers that run while this family has the keys (its page, or a float it opened), keyed
    /// by the command `id` in `keymap.toml`. Several families may handle one id (`list.open`).
    fn commands(&self) -> Vec<(&'static str, Handler)> {
        Vec::new()
    }

    /// Handlers that run wherever the keys are: commands whose keymap context is `global` and
    /// that this family owns (`conv.new` opens the start page). One family per id.
    fn globals(&self) -> Vec<(&'static str, Handler)> {
        Vec::new()
    }

    /// Where the page's cursor stands, in the family's own terms: what the jumplist keeps and a
    /// hint label moves to (`grammar.md` §13). Only this family reads it back. `None` for a page
    /// without a cursor.
    fn anchor(&self, _app: &App) -> Option<Anchor> {
        None
    }

    /// Puts the page's cursor on `anchor`; one the page no longer has leaves the cursor where it
    /// is. Nothing else changes: it never opens or acts.
    fn land(&self, _app: &mut App, _anchor: &Anchor) {}

    /// What hint mode labels on the page (rows, notes, choices), in reading order. Hint mode keeps
    /// the ones the window shows.
    fn targets(&self, _app: &App) -> Vec<HintTarget> {
        Vec::new()
    }

    /// The source text of `obj` at the cursor, for `y` and `␣y` (`grammar.md` §4); yanking a
    /// Visual selection leaves Visual. `None` where the page has no such object.
    fn yank(&self, _app: &mut App, _obj: crate::yank::Object) -> Option<String> {
        None
    }

    /// Object keys owned by a pending operator, before keymap resolution.
    fn object_key(&self, _app: &mut App, _key: Key) -> bool {
        false
    }

    /// Keys no command took while this family has the keys: list motions, an editor's keys.
    /// False hands the key on, so a focused field types it.
    fn keys(&self, _app: &mut App, _keys: &[Key], _count: Option<u32>) -> bool {
        true
    }

    /// `App::show` has just put one of this family's stages on the stage (it may have been there
    /// already).
    fn shown(&self, _app: &mut App) {}

    /// The page's target in the reduced viewport when a bottom float opens.
    fn reveal_target(&self, _app: &App, _tx: &mut Tx) {}

    /// Something the link reports, once; every family hears every event.
    fn on_event(&self, _app: &mut App, _event: &LinkEvent) {}

    /// The window gained or lost focus; every family hears it.
    fn on_focus(&self, _app: &mut App, _focused: bool) {}

    /// Named states for headless renders (`nix run .#snap <name>`), built with
    /// `app::scenes::on_fixture`; a scene named after a mockup compares with it.
    fn scenes(&self) -> Vec<Scene> {
        Vec::new()
    }

    /// Every frame, for every family: timers such as Insert's `jk`. True redraws.
    fn poll(&self, _app: &mut App, _now: f64, _tx: &mut Tx) -> bool {
        false
    }
}

/// A cursor position as one family writes it ([`Screen::anchor`]): JSON, so the jumplist can keep
/// it on this device and a family can read back only what it wrote.
pub type Anchor = serde_json::Value;

/// A place hint mode labels: the element drawn for it, where its text starts inside that element
/// (after its mark), and where its label moves the cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct HintTarget {
    /// The rune key of the element.
    pub key: String,
    /// From the element's top left to the label's, in points.
    pub at: [f32; 2],
    /// The rune key of the scrolling element that clips it: a label shows only where both it
    /// and the stage show the target.
    pub within: &'static str,
    pub anchor: Anchor,
}

/// A float a screen family opens and handles: the peek float, the readback, a picker. The family
/// that owns it gets its commands (`Screen::commands`) and its other keys (`Screen::keys`).
pub trait ScreenFloat: Any {
    fn owner(&self) -> ScreenId;
    fn scope(&self) -> Scope;
    fn badge(&self) -> Badge;
    fn view(&self, t: Theme, app: &App) -> El;
    /// The float's input, for its field's `on_edit`.
    fn edit_mut(&mut self) -> Option<&mut TextEdit> {
        None
    }
}

/// The top float, when it is a `T`.
pub fn top_float<T: ScreenFloat>(app: &mut App) -> Option<&mut T> {
    match app.floats.last_mut() {
        Some((Float::Screen(f), _)) => (f.as_mut() as &mut dyn Any).downcast_mut::<T>(),
        _ => None,
    }
}

/// The stage's width, as laid out last, or as the window gives it before the first layout.
pub(crate) fn stage_width(t: Theme, app: &App) -> f32 {
    rune::memo::layout_size("stage")
        .map(|s| s[0])
        .unwrap_or_else(|| {
            let side = app.sidebar_place() == Some(crate::frame::SidebarPlace::Beside);
            app.window[0] - if side { t.l.sidebar.width } else { 0.0 }
        })
}

/// Where a reading column `width` wide starts in the page while the details panel is open
/// (`spec.md` §4.7): where it starts with the panel closed (`closed`) while its right edge clears
/// the panel by the panel's padding, else moved left by the shortfall, down to its 16 px margin.
/// `room` is the page's width left of the panel.
pub(crate) fn beside_details(t: Theme, closed: f32, width: f32, room: f32) -> f32 {
    closed.min(room - t.l.details.padding - width).max(16.0)
}

/// The line that stands in for a page's groups until its data arrives (`spec.md` §4.8).
pub const NOTHING_RECEIVED: &str = "Nothing received from the server yet";

/// A page not built yet: its title over one line.
pub fn stub(t: Theme, title: &str, line: &str) -> El {
    let l = t.l;
    col()
        .w_full()
        .h_full()
        .pl(l.list_column.left)
        .pr(l.list_column.right_gap)
        .pt(l.title_block.top)
        .child(
            text(title.to_string())
                .font(t.ty.title_page)
                .color(t.c.state.attn)
                .truncate(),
        )
        .child(
            text(line.to_string())
                .font(t.ty.ui)
                .color(t.c.fg.alt)
                .mt(14.0),
        )
}

/// An open item's title as the server named it, else its project.
pub fn item_title(app: &App, item: &Item) -> String {
    app.server
        .label(item)
        .unwrap_or_else(|| item.project().to_string())
}

#[cfg(test)]
mod tests;
