//! The look, from `design/tokens.json`: three colour variants over one type scale and one layout.
//! Switching variant swaps the palette only, so nothing reflows.

mod tokens;

use std::sync::LazyLock;

#[cfg(test)]
use rune::Color;
use rune::{El, Family, Spring, SpringPair};

use crate::keys::Keymap;

pub use tokens::{
    Border, Content, CountTag, Fg, Float, Gaps, Heights, Kind, Layout, ListColumn, Marks,
    NoticeBand, PathMark, Peek, Radius, Rule, Sidebar, StageHeader, State, Surface, Text,
    TextField, Tint, TokenError, TypeScale,
};

/// One of the design's three palettes. `␣ht` cycles them in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    #[default]
    Ember,
    EmberSoft,
    EmberLight,
}

impl Variant {
    pub const ALL: [Variant; 3] = [Variant::Ember, Variant::EmberSoft, Variant::EmberLight];

    pub fn next(self) -> Variant {
        match self {
            Variant::Ember => Variant::EmberSoft,
            Variant::EmberSoft => Variant::EmberLight,
            Variant::EmberLight => Variant::Ember,
        }
    }

    /// The name used by `tokens.json` and by `theme` in the device's config.
    pub fn name(self) -> &'static str {
        match self {
            Variant::Ember => "ember",
            Variant::EmberSoft => "ember-soft",
            Variant::EmberLight => "ember-light",
        }
    }

    pub fn from_name(name: &str) -> Option<Variant> {
        Variant::ALL.into_iter().find(|v| v.name() == name)
    }

    pub fn is_light(self) -> bool {
        self == Variant::EmberLight
    }
}

/// The colours of one variant, grouped as `tokens.json` groups them.
#[derive(Clone, Debug)]
pub struct Palette {
    pub surface: Surface,
    pub border: Border,
    pub rule: Rule,
    pub fg: Fg,
    pub state: State,
    pub kind: Kind,
    pub text: Text,
    pub content: Content,
    pub tint: Tint,
    pub syntax: tokens::SyntaxColors,
    /// The float's upward shadow (`elevation.float`).
    pub float_shadow: rune::Shadow,
}

/// Everything `tokens.json` holds that the app reads.
#[derive(Debug)]
pub struct Tokens {
    panel_ms: f32,
    page_ms: f32,
    motion_bounce: f32,
    palettes: [Palette; 3],
    pub ty: TypeScale,
    pub layout: Layout,
    /// The client's own small drawings (`tau.icon`).
    pub marks: Marks,
}

impl Tokens {
    pub fn parse(json: &str) -> Result<Tokens, TokenError> {
        tokens::parse(json)
    }

    pub fn palette(&self, v: Variant) -> &Palette {
        &self.palettes[v as usize]
    }
}

static TOKENS: LazyLock<Tokens> = LazyLock::new(|| {
    Tokens::parse(include_str!("../../../../design/tokens.json"))
        .expect("design/tokens.json parses")
});

/// The design's tokens, parsed once.
pub fn tokens() -> &'static Tokens {
    &TOKENS
}

/// What a view draws with: the current palette over the shared type scale and layout, and the
/// keys this device types (`t.keymap`), which every printed key follows.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub variant: Variant,
    pub c: &'static Palette,
    pub ty: &'static TypeScale,
    pub l: &'static Layout,
    pub read: Reading,
    pub motion: Motion,
    pub keymap: Keymap,
}

impl Theme {
    pub fn of(variant: Variant) -> Theme {
        Self::new(variant, Measure::default(), true)
    }

    pub fn new(variant: Variant, measure: Measure, motion: bool) -> Theme {
        let t = tokens();
        Theme {
            variant,
            c: t.palette(variant),
            ty: &t.ty,
            l: &t.layout,
            read: Reading::new(measure),
            motion: Motion::new(motion),
            keymap: Keymap::design(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measure(f32);

impl Measure {
    pub fn new(px: f32) -> Result<Self, Self> {
        let r = &tokens().layout.reading;
        if px.is_nan() {
            return Err(Self::default());
        }
        let bounded = px.clamp(r.min, r.max);
        if bounded == px {
            Ok(Self(px))
        } else {
            Err(Self(bounded))
        }
    }

    pub fn step(self, by: i32) -> Self {
        Self::new(self.0 + by as f32 * tokens().layout.reading.step)
            .unwrap_or_else(|nearest| nearest)
    }

    pub fn px(self) -> f32 {
        self.0
    }
}

impl Default for Measure {
    fn default() -> Self {
        Self(tokens().layout.reading.measure)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub measure: f32,
    pub conversation: f32,
    pub decision_block: f32,
    pub fold_notes_below: f32,
}

impl Reading {
    fn new(measure: Measure) -> Self {
        let l = &tokens().layout;
        let measure = measure.px();
        let decision_block = measure + l.decision.gap + l.decision.notes_margin;
        Self {
            measure,
            conversation: measure + l.reading.conversation_room,
            decision_block,
            fold_notes_below: decision_block + 2.0 * l.decision.fold_margin,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub panel: Spring,
    pub page: Spring,
    pub on: bool,
}

impl Motion {
    pub fn new(on: bool) -> Self {
        let t = tokens();
        let spring = |ms| {
            if on && ms > 0.0 {
                Spring::new(ms / 1000.0, t.motion_bounce)
            } else {
                Spring::NONE
            }
        };
        Self {
            panel: spring(t.panel_ms),
            page: spring(t.page_ms),
            on,
        }
    }

    pub fn moves(self, el: El, spring: Spring) -> El {
        if self.on {
            el.animated_changes().token_motion(spring)
        } else {
            el
        }
    }

    pub fn pair(spring: Spring) -> SpringPair {
        SpringPair::new(spring, spring)
    }
}

/// The bundled faces: Etoile is the sans slot, Iosevka the mono slot (`AppSpec::families`).
pub const FAMILIES: (&str, &str) = ("Iosevka Etoile", "Iosevka");

fn family(name: &str) -> Option<Family> {
    match name {
        n if n == FAMILIES.0 => Some(Family::Sans),
        n if n == FAMILIES.1 => Some(Family::Mono),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
