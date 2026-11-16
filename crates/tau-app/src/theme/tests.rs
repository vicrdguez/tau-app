//! How the theme could fail its users, one test each:
//! - the embedded tokens don't parse, so the app panics at launch;
//! - a variant reads another variant's colours, or a token reads the wrong key;
//! - a colour missing from one variant, or a malformed one, parses as a default (black)
//!   instead of failing loudly;
//! - switching variant changes type or layout, so the page reflows;
//! - a type style loses its family, size, line height, weight or tracking;
//! - a family other than the two bundled ones slips through;
//! - `␣ht` cycles in the wrong order, or a config name maps to the wrong variant;
//! - the float's upward shadow loses its direction or its colour;
//! - the frame's sizes drift from the tokens.

use super::*;

const EMBEDDED: &str = include_str!("../../../../design/tokens.json");

fn hex(s: &str) -> Color {
    rune::color::hex(s).expect("a test colour")
}

fn without(path: &[&str]) -> String {
    let mut v: serde_json::Value = serde_json::from_str(EMBEDDED).expect("tokens.json is JSON");
    let (last, parents) = path.split_last().expect("a path");
    let mut at = &mut v;
    for p in parents {
        at = at.get_mut(*p).expect("the path exists");
    }
    at.as_object_mut().expect("an object").remove(*last);
    v.to_string()
}

fn replaced(path: &[&str], value: serde_json::Value) -> String {
    let mut v: serde_json::Value = serde_json::from_str(EMBEDDED).expect("tokens.json is JSON");
    let mut at = &mut v;
    for p in path {
        at = at.get_mut(*p).expect("the path exists");
    }
    *at = value;
    v.to_string()
}

#[test]
fn the_embedded_tokens_parse() {
    Tokens::parse(EMBEDDED).expect("design/tokens.json parses");
}

#[test]
fn each_variant_reads_its_own_colours() {
    let t = tokens();
    assert_eq!(t.palette(Variant::Ember).surface.read, hex("#151412"));
    assert_eq!(t.palette(Variant::EmberSoft).surface.read, hex("#1c1b19"));
    assert_eq!(t.palette(Variant::EmberLight).surface.read, hex("#f0e8d8"));
    assert_eq!(t.palette(Variant::EmberSoft).fg.muted, hex("#a19c92"));
    assert_eq!(t.palette(Variant::EmberLight).state.attn, hex("#af482e"));
    let e = t.palette(Variant::Ember);
    assert_eq!(e.surface.cursor_side, hex("#2e2d2a"));
    assert_eq!(e.surface.current, hex("#2e2d2a"));
    assert_eq!(e.border.hairline, hex("#2c2b28"));
    assert_eq!(e.fg.default, hex("#d8d0c0"));
    assert_eq!(e.kind.literal, hex("#c09058"));
    assert_eq!(e.kind.worker, hex("#80a090"));
    assert_eq!(e.kind.you, hex("#8097a6"));
    assert_eq!(e.text.failed, hex("#b98787"));
    assert_eq!(e.content.h1, hex("#b98787"));
    assert_eq!(e.tint.row_fail, hex("#241918"));
    assert_eq!(e.rule.note_leader, hex("#6b675f"));
}

#[test]
fn a_colour_missing_from_one_variant_is_an_error_naming_it() {
    let err = Tokens::parse(&without(&["variants", "ember-light", "surface", "raised"]))
        .expect_err("an error");
    assert!(err.to_string().contains("raised"), "{err}");
}

#[test]
fn a_missing_variant_is_an_error() {
    assert!(Tokens::parse(&without(&["variants", "ember-soft"])).is_err());
}

#[test]
fn a_malformed_colour_is_an_error() {
    let json = replaced(
        &["variants", "ember", "fg", "alt"],
        serde_json::json!("#b0a89"),
    );
    let err = Tokens::parse(&json).expect_err("an error");
    assert!(err.to_string().contains("#b0a89"), "{err}");
}

#[test]
fn switching_variant_keeps_type_and_layout() {
    let (a, b) = (Theme::of(Variant::Ember), Theme::of(Variant::EmberLight));
    assert!(std::ptr::eq(a.ty, b.ty) && std::ptr::eq(a.l, b.l));
    assert_ne!(a.c.surface.read, b.c.surface.read);
}

#[test]
fn type_styles_follow_the_tokens() {
    let ty = &tokens().ty;
    let s = ty.title_page;
    assert_eq!(
        (s.family, s.size, s.line_height, s.weight),
        (Family::Sans, 24.0, 30.0, 700.0)
    );
    let s = ty.mono_small;
    assert_eq!(
        (s.family, s.size, s.line_height, s.weight),
        (Family::Mono, 12.0, 16.0, 400.0)
    );
    let s = ty.key;
    assert_eq!(
        (s.family, s.size, s.line_height, s.weight),
        (Family::Mono, 11.5, 16.0, 500.0)
    );
    let s = ty.sidebar_server;
    assert_eq!(
        (s.family, s.size, s.line_height, s.weight),
        (Family::Sans, 11.5, 16.0, 400.0)
    );
    let s = ty.float_input;
    assert_eq!(
        (s.family, s.size, s.line_height, s.weight),
        (Family::Mono, 14.0, 20.0, 400.0)
    );
    assert_eq!(ty.ui.size, 13.0);
    assert_eq!(ty.ui_small.line_height, 16.0);
    assert_eq!(ty.group.weight, 600.0);
}

#[test]
fn tracked_styles_carry_their_tracking() {
    let ty = &tokens().ty;
    assert!((ty.badge.tracking - 11.0 * 0.08).abs() < 1e-4);
    assert!((ty.sidebar_section.tracking - 11.0 * 0.1).abs() < 1e-4);
    assert_eq!(ty.ui.tracking, 0.0);
}

#[test]
fn a_family_other_than_the_bundled_two_is_an_error() {
    let json = replaced(
        &["type", "ui"],
        serde_json::json!(["Inter", 13, 18, 400, "rows"]),
    );
    let err = Tokens::parse(&json).expect_err("an error");
    assert!(err.to_string().contains("Inter"), "{err}");
}

#[test]
fn the_theme_key_cycles_ember_soft_light() {
    assert_eq!(Variant::Ember.next(), Variant::EmberSoft);
    assert_eq!(Variant::EmberSoft.next(), Variant::EmberLight);
    assert_eq!(Variant::EmberLight.next(), Variant::Ember);
}

#[test]
fn variants_parse_from_their_config_names() {
    assert_eq!(Variant::from_name("ember"), Some(Variant::Ember));
    assert_eq!(Variant::from_name("ember-soft"), Some(Variant::EmberSoft));
    assert_eq!(Variant::from_name("ember-light"), Some(Variant::EmberLight));
    assert_eq!(Variant::from_name("dracula"), None);
}

#[test]
fn the_float_shadow_rises_in_each_variant() {
    let t = tokens();
    let dark = t.palette(Variant::Ember).float_shadow;
    assert_eq!((dark.y, dark.blur), (-14.0, 40.0));
    assert!((dark.color.a - 0.45).abs() < 1e-4);
    let light = t.palette(Variant::EmberLight).float_shadow;
    assert_eq!((light.y, light.blur), (-14.0, 40.0));
    assert!((light.color.a - 0.22).abs() < 1e-4);
    assert_ne!(light.color.r, dark.color.r);
}

#[test]
fn layout_reads_the_frame_sizes() {
    let l = &tokens().layout;
    assert_eq!(l.window, [1512.0, 982.0]);
    assert_eq!(l.min_window, [800.0, 600.0]);
    assert_eq!(l.sidebar.width, 248.0);
    assert_eq!(l.sidebar.autohide_below_window, 1000.0);
    assert_eq!(l.stage_header.height, 44.0);
    assert_eq!(l.stage_header.left_when_sidebar_hidden, 88.0);
    assert_eq!(l.height.bottombar, 26.0);
    assert_eq!((l.float.inset_x, l.float.inset_bottom), (12.0, 10.0));
    assert_eq!(l.peek.width, 440.0);
    assert_eq!(
        (
            l.count_tag.height,
            l.count_tag.title_height,
            l.count_tag.min_width
        ),
        (16.0, 18.0, 30.0)
    );
    assert_eq!((l.radius.pill, l.radius.surface), (4.0, 6.0));
    assert_eq!(l.whichkey_delay_ms, 300.0);
}
