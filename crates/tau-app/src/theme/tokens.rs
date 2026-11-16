//! `tokens.json` as typed values. Every colour, style and size the app reads must be present and
//! well formed, or parsing fails with the key's name: nothing falls back to a default.

use std::collections::BTreeMap;

use rune::{Color, TextStyle};
use serde::Deserialize;
use serde::de::{Deserializer, Error as _};

use super::{Palette, Tokens, family};

#[derive(Debug)]
pub struct TokenError(String);

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "design/tokens.json: {}", self.0)
    }
}

impl std::error::Error for TokenError {}

fn hex_colour<E: serde::de::Error>(s: &str) -> Result<Color, E> {
    rune::color::hex(s).ok_or_else(|| E::custom(format!("`{s}` is not a #rrggbb colour")))
}

/// A group of colours read by name, so a dotted key (`cursor.read`) maps to a field and a
/// missing one is an error that names it.
macro_rules! colour_group {
    ($name:ident { $($field:ident = $key:literal),* $(,)? }) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $name { $(pub $field: Color),* }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let m = BTreeMap::<String, serde_json::Value>::deserialize(d)?;
                let take = |key: &str| -> Result<Color, D::Error> {
                    match m.get(key).and_then(|v| v.as_str()) {
                        Some(s) => hex_colour(s),
                        None => Err(D::Error::custom(format!("missing colour `{key}` in `{}`", stringify!($name)))),
                    }
                };
                Ok($name { $($field: take($key)?),* })
            }
        }
    };
}

colour_group!(Surface {
    read = "read", side = "side", raised = "raised", bar = "bar", cursor_read = "cursor.read",
    cursor_side = "cursor.side", current = "current", code = "code", sunken = "sunken",
    selection = "selection",
});
colour_group!(Border { hairline = "hairline", strong = "strong" });
colour_group!(Rule { focus = "focus", note_leader = "note_leader" });
colour_group!(Fg { default = "default", alt = "alt", muted = "muted" });
colour_group!(State {
    attn = "attn", blocked = "blocked", stale = "stale", unknown = "unknown", local = "local", ok = "ok",
});
colour_group!(Kind {
    you = "you", coordinator = "coordinator", worker = "worker", literal = "literal", code = "code",
    link = "link", link_active = "link_active",
});
colour_group!(Text { ok = "ok", failed = "failed" });
colour_group!(Content { h1 = "h1", h2 = "h2", strong = "strong" });
colour_group!(Tint { human = "human", row_fail = "row_fail", note_range = "note_range", note_active = "note_active" });

#[derive(Deserialize)]
struct RawPalette {
    surface: Surface,
    border: Border,
    rule: Rule,
    fg: Fg,
    state: State,
    kind: Kind,
    text: Text,
    content: Content,
    tint: Tint,
    syntax: SyntaxColors,
}

#[derive(Deserialize)]
struct Variants<T> {
    ember: T,
    #[serde(rename = "ember-soft")]
    ember_soft: T,
    #[serde(rename = "ember-light")]
    ember_light: T,
}

impl<T> Variants<T> {
    fn into_array(self) -> [T; 3] {
        [self.ember, self.ember_soft, self.ember_light]
    }
}

/// A `type` entry: `[family, size, line, weight, use]`.
type RawStyle = (String, f32, f32, f32, String);

/// The type scale: `type` and `tau.type` in one table.
#[derive(Clone, Copy, Debug)]
pub struct TypeScale {
    pub title_page: TextStyle,
    pub prose_h1: TextStyle,
    pub prose_h2: TextStyle,
    pub prose_h3: TextStyle,
    pub prose_h4: TextStyle,
    pub prose_h5: TextStyle,
    pub prose_h6: TextStyle,
    pub move_: TextStyle,
    pub group: TextStyle,
    pub prose: TextStyle,
    pub prose_small: TextStyle,
    pub prose_table: TextStyle,
    pub prose_code: TextStyle,
    pub ui: TextStyle,
    pub ui_strong: TextStyle,
    pub ui_small: TextStyle,
    pub mono: TextStyle,
    pub mono_small: TextStyle,
    pub mono_output: TextStyle,
    pub key: TextStyle,
    pub hint: TextStyle,
    pub badge: TextStyle,
    pub peek_h1: TextStyle,
    pub peek_h2: TextStyle,
    pub peek_h3: TextStyle,
    pub peek_body: TextStyle,
    pub sidebar_brand: TextStyle,
    pub sidebar_server: TextStyle,
    pub sidebar_section: TextStyle,
    pub sidebar_group: TextStyle,
    pub float_title: TextStyle,
    pub float_input: TextStyle,
}

/// Letter spacing the tokens give only in words (`visual-language.md` §2): em of the style's size.
const TRACKING_EM: [(&str, f32); 3] = [
    ("badge", 0.08),
    ("sidebar.section", 0.1),
    ("prose.h5", 0.04),
];

fn type_scale(raw: &BTreeMap<String, RawStyle>) -> Result<TypeScale, TokenError> {
    let style = |name: &str| -> Result<TextStyle, TokenError> {
        let (fam, size, line, weight, _) = raw
            .get(name)
            .ok_or_else(|| TokenError(format!("missing type style `{name}`")))?;
        let family = family(fam).ok_or_else(|| {
            TokenError(format!(
                "type style `{name}` names `{fam}`, not a bundled family"
            ))
        })?;
        let tracking = TRACKING_EM
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(0.0, |(_, em)| em * size);
        Ok(TextStyle {
            family,
            line_height: *line,
            tracking,
            ..TextStyle::new(*size, *weight)
        })
    };
    Ok(TypeScale {
        title_page: style("title.page")?,
        prose_h1: style("prose.h1")?,
        prose_h2: style("prose.h2")?,
        prose_h3: style("prose.h3")?,
        prose_h4: style("prose.h4")?,
        prose_h5: style("prose.h5")?,
        prose_h6: style("prose.h6")?.italic(),
        move_: style("move")?,
        group: style("group")?,
        prose: style("prose")?,
        prose_small: style("prose.small")?,
        prose_table: style("prose.table")?,
        prose_code: style("prose.code")?,
        ui: style("ui")?,
        ui_strong: style("ui.strong")?,
        ui_small: style("ui.small")?,
        mono: style("mono")?,
        mono_small: style("mono.small")?,
        mono_output: style("mono.output")?,
        key: style("key")?,
        hint: style("hint")?,
        badge: style("badge")?,
        peek_h1: style("peek.h1")?,
        peek_h2: style("peek.h2")?,
        peek_h3: style("peek.h3")?,
        peek_body: style("peek.body")?,
        sidebar_brand: style("sidebar.brand")?,
        sidebar_server: style("sidebar.server")?,
        sidebar_section: style("sidebar.section")?,
        sidebar_group: style("sidebar.group")?,
        float_title: style("float.title")?,
        float_input: style("float.input")?,
    })
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Sidebar {
    pub width: f32,
    pub padding_x: f32,
    pub row: f32,
    pub row_padding_x: f32,
    pub row_icon: f32,
    pub item_icon: f32,
    pub traffic_lights: [f32; 2],
    pub brand_top: f32,
    pub avatar: f32,
    pub section_label_above: f32,
    pub group_label: f32,
    pub autohide_below_window: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct StageHeader {
    pub height: f32,
    /// Left and right padding.
    pub padding_x: [f32; 2],
    pub left_when_sidebar_hidden: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ListColumn {
    pub left: f32,
    pub right_gap: f32,
    pub row_band_outset: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Peek {
    pub width: f32,
    pub padding: f32,
    pub from_stage_px: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Details {
    pub width: f32,
    pub padding: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct TitleBlock {
    pub top: f32,
    pub title: f32,
    pub facts_gap: f32,
    pub facts: f32,
    pub after: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Float {
    pub inset_x: f32,
    pub inset_bottom: f32,
    pub padding_top: f32,
    pub padding_bottom: f32,
    pub padding_x: f32,
    pub row: f32,
    pub kv_label: f32,
    pub keygrid_column: f32,
    pub keygrid_row: f32,
    pub foot_min_height: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct NoticeBand {
    pub one_line: f32,
    pub two_lines: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Gaps {
    pub segment: f32,
    pub hint: f32,
    pub float_hints: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct TextField {
    pub height: f32,
    pub padding_x: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Radius {
    pub pill: f32,
    pub surface: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Heights {
    pub stage_header: f32,
    pub bottombar: f32,
    pub dense_row: f32,
    pub float_row: f32,
    pub count_tag: f32,
    pub count_tag_title: f32,
    pub badge: f32,
    pub answer_segment: f32,
    pub text_field: f32,
    pub verb_tag: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct CountTag {
    pub height: f32,
    pub title_height: f32,
    pub min_width: f32,
    pub padding_x: f32,
    pub mark: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ReadingTokens {
    pub measure: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub conversation_room: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Decision {
    pub gap: f32,
    pub notes_margin: f32,
    pub fold_margin: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Markdown {
    pub paragraph_gap: f32,
    pub list_indent: f32,
    pub table_cell: [f32; 2],
    pub quote_bar: f32,
    pub code_rule: f32,
    pub code_padding: [f32; 2],
    pub image_placeholder: f32,
    pub task_box: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SyntaxColor(pub Color);

impl<'de> Deserialize<'de> for SyntaxColor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            color: String,
        }
        let raw = Raw::deserialize(d)?;
        hex_colour(&raw.color).map(SyntaxColor)
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct SyntaxColors {
    pub keyword: SyntaxColor,
    pub string: SyntaxColor,
    pub constant: SyntaxColor,
    pub r#type: SyntaxColor,
    pub function: SyntaxColor,
}

#[derive(Deserialize)]
struct RawLayout {
    reading: ReadingTokens,
    decision: Decision,
    markdown: Markdown,
    window: [f32; 2],
    min_window: [f32; 2],
    sidebar: Sidebar,
    stage_header: StageHeader,
    list_column: ListColumn,
    peek: Peek,
    details: Details,
    title_block: TitleBlock,
    float: Float,
    notice_band: NoticeBand,
    gaps: Gaps,
    text_field: TextField,
}

/// The frame's sizes and the timings the UI keeps.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub reading: ReadingTokens,
    pub decision: Decision,
    pub markdown: Markdown,
    pub window: [f32; 2],
    pub min_window: [f32; 2],
    pub sidebar: Sidebar,
    pub stage_header: StageHeader,
    pub list_column: ListColumn,
    pub peek: Peek,
    pub details: Details,
    pub title_block: TitleBlock,
    pub float: Float,
    pub notice_band: NoticeBand,
    pub gaps: Gaps,
    pub text_field: TextField,
    /// `space.1` to `space.7`; index 0 is unused.
    pub space: [f32; 8],
    pub radius: Radius,
    pub height: Heights,
    pub count_tag: CountTag,
    pub whichkey_delay_ms: f32,
    pub readback_arm_ms: f32,
}

/// One of the client's own small drawings: a single SVG path.
#[derive(Clone, Debug, Deserialize)]
pub struct PathMark {
    #[serde(rename = "viewBox")]
    pub view_box: String,
    pub path: String,
    pub size: f32,
}

#[derive(Debug, Deserialize)]
struct MarkSmall {
    slot: f32,
    decision: PathMark,
    failed: PathMark,
    running: PathMark,
}

/// The paths of `tau.icon`.
#[derive(Debug)]
pub struct Marks {
    pub slot: f32,
    pub decision: PathMark,
    pub failed: PathMark,
    pub running: PathMark,
    pub key_space: PathMark,
    pub key_return: PathMark,
}

#[derive(Deserialize)]
struct RawIcon {
    mark_small: MarkSmall,
    key_space: PathMark,
    key_return: PathMark,
    count_tag: CountTag,
}

#[derive(Deserialize)]
struct RawClientMs {
    whichkey_delay: f32,
    readback_arm: f32,
}

#[derive(Deserialize)]
struct RawMotion {
    float_fade: f32,
}

#[derive(Deserialize)]
struct RawTau {
    #[serde(rename = "type")]
    ty: BTreeMap<String, RawStyle>,
    layout: RawLayout,
    icon: RawIcon,
    client_ms: RawClientMs,
}

#[derive(Deserialize)]
struct RawElevation {
    float: Variants<String>,
}

#[derive(Deserialize)]
struct Raw {
    variants: Variants<RawPalette>,
    #[serde(rename = "type")]
    ty: BTreeMap<String, RawStyle>,
    space: BTreeMap<String, f32>,
    radius: Radius,
    height: Heights,
    elevation: RawElevation,
    motion_ms: RawMotion,
    tau: RawTau,
}

/// `0 -14px 40px rgba(0,0,0,.45)`: x, y, blur and an sRGB colour with alpha.
fn shadow(css: &str) -> Result<rune::Shadow, TokenError> {
    let bad = || TokenError(format!("`{css}` is not `x y blur rgba(r,g,b,a)`"));
    let (lengths, colour) = css.split_once("rgba(").ok_or_else(bad)?;
    let px: Vec<f32> = lengths
        .split_whitespace()
        .map(|v| v.trim_end_matches("px").parse())
        .collect::<Result<_, _>>()
        .map_err(|_| bad())?;
    let rgba: Vec<f32> = colour
        .trim_end_matches(')')
        .split(',')
        .map(|v| v.trim().parse())
        .collect::<Result<_, _>>()
        .map_err(|_| bad())?;
    let (&[x, y, blur], &[r, g, b, a]) = (&px[..], &rgba[..]) else {
        return Err(bad());
    };
    let to_hex = |v: f32| format!("{:02x}", v.round() as u8);
    let base =
        hex_colour::<serde_json::Error>(&format!("#{}{}{}", to_hex(r), to_hex(g), to_hex(b)))
            .map_err(|_| bad())?;
    Ok(rune::Shadow {
        x,
        y,
        blur,
        color: base.alpha(a),
        ..rune::Shadow::default()
    })
}

pub(super) fn parse(json: &str) -> Result<Tokens, TokenError> {
    let raw: Raw = serde_json::from_str(json).map_err(|e| TokenError(e.to_string()))?;
    let shadows = raw.elevation.float.into_array().map(|s| shadow(&s));
    let mut palettes = Vec::with_capacity(3);
    for (p, s) in raw.variants.into_array().into_iter().zip(shadows) {
        palettes.push(Palette {
            surface: p.surface,
            border: p.border,
            rule: p.rule,
            fg: p.fg,
            state: p.state,
            kind: p.kind,
            text: p.text,
            content: p.content,
            tint: p.tint,
            syntax: p.syntax,
            float_shadow: s?,
        });
    }
    let palettes: [Palette; 3] = palettes
        .try_into()
        .map_err(|_| TokenError("three variants".into()))?;
    let mut all_types = raw.ty;
    all_types.extend(raw.tau.ty);
    let mut space = [0.0; 8];
    for (i, slot) in space.iter_mut().enumerate().skip(1) {
        *slot = *raw
            .space
            .get(&i.to_string())
            .ok_or_else(|| TokenError(format!("missing `space.{i}`")))?;
    }
    let l = raw.tau.layout;
    let layout = Layout {
        reading: l.reading,
        decision: l.decision,
        markdown: l.markdown,
        window: l.window,
        min_window: l.min_window,
        sidebar: l.sidebar,
        stage_header: l.stage_header,
        list_column: l.list_column,
        peek: l.peek,
        details: l.details,
        title_block: l.title_block,
        float: l.float,
        notice_band: l.notice_band,
        gaps: l.gaps,
        text_field: l.text_field,
        space,
        radius: raw.radius,
        height: raw.height,
        count_tag: raw.tau.icon.count_tag,
        whichkey_delay_ms: raw.tau.client_ms.whichkey_delay,
        readback_arm_ms: raw.tau.client_ms.readback_arm,
    };
    let icon = raw.tau.icon;
    let marks = Marks {
        slot: icon.mark_small.slot,
        decision: icon.mark_small.decision,
        failed: icon.mark_small.failed,
        running: icon.mark_small.running,
        key_space: icon.key_space,
        key_return: icon.key_return,
    };
    Ok(Tokens {
        float_fade_ms: raw.motion_ms.float_fade,
        palettes,
        ty: type_scale(&all_types)?,
        layout,
        marks,
    })
}
