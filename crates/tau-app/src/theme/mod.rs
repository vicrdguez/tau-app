//! The look, from `design/tokens.json`: three colour variants over one type scale and one layout.
//! Switching variant swaps the palette only, so nothing reflows.

mod tokens;

use std::sync::LazyLock;

#[cfg(test)]
use rune::Color;
use rune::Family;

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
    /// The float's upward shadow (`elevation.float`).
    pub float_shadow: rune::Shadow,
}

/// Everything `tokens.json` holds that the app reads.
#[derive(Debug)]
pub struct Tokens {
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

/// What a view draws with: the current palette over the shared type scale and layout.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub variant: Variant,
    pub c: &'static Palette,
    pub ty: &'static TypeScale,
    pub l: &'static Layout,
}

impl Theme {
    pub fn of(variant: Variant) -> Theme {
        let t = tokens();
        Theme {
            variant,
            c: t.palette(variant),
            ty: &t.ty,
            l: &t.layout,
        }
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
