//! Failure modes:
//! - A long token escapes the note box in Insert, or the margin after leaving Insert.
//! - Whole and passage notes wrap differently, or only clip instead of adding lines.
//! - A code line hides its tail in a horizontal viewport instead of growing taller.
//! - Wrapped code loses note-range highlights or stops wrapping while streaming.
//! - Reading columns stay narrow, or widening the decision folds notes at 1512 px.

use super::*;
use crate::screens::decision::{notes::Draft, state::Anchor};
use rune::{ElementInfo, Env, Runtime};

fn token() -> String {
    format!("asdfs{}", "d".repeat(145))
}

fn scene(name: &str) -> App {
    let mut app = App::default();
    scenes::all().iter().find(|(n, _)| *n == name).unwrap().1(&mut app);
    app.window = [1512.0, 982.0];
    app
}

fn runtime(app: App) -> Runtime<App> {
    let fonts =
        rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = Runtime::with_text(app, view, (1512.0, 982.0), Env::new(true, 1.0), fonts);
    advance(&mut rt);
    rt
}

fn advance(rt: &mut Runtime<App>) {
    for _ in 0..3 {
        rt.frame(1.0);
    }
}

fn keyed<'a>(elements: &'a [ElementInfo], key: &str) -> &'a ElementInfo {
    elements
        .iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
}

fn text_element<'a>(elements: &'a [ElementInfo], body: &str) -> &'a ElementInfo {
    elements
        .iter()
        .find(|e| e.text.as_deref() == Some(body))
        .unwrap_or_else(|| panic!("missing text {body}"))
}

fn inside_x(inner: [f32; 4], outer: [f32; 4]) {
    assert!(
        inner[0] >= outer[0] - 1.0 && inner[0] + inner[2] <= outer[0] + outer[2] + 1.0,
        "text {inner:?} must stay inside {outer:?}"
    );
}

#[test]
fn reading_layout_note_draft_wraps_inside_its_box() {
    let mut app = scene("decision-note-insert");
    app.screens
        .decision
        .editing
        .as_mut()
        .unwrap()
        .editor
        .edit
        .replace_text(token());
    let rt = runtime(app);
    let elements = rt.elements();
    let field = keyed(&elements, "decision-note");
    let note_box = keyed(&elements, "mn-0");
    inside_x(field.rect, note_box.rect);
    inside_x(field.caret.expect("Insert caret"), note_box.rect);
    assert!(
        field.rect[3] > rt.state.theme().ty.ui.line_height * 2.0,
        "the token adds lines, not clipping: {:?}",
        field.rect
    );
}

#[test]
fn reading_layout_saved_notes_wrap_inside_the_margin() {
    for whole in [false, true] {
        let mut app = scene("decision-note-insert");
        let editing = app.screens.decision.editing.take().unwrap();
        let range = if whole {
            None
        } else {
            match editing.anchor {
                Anchor::Range { range, .. } => Some(range),
                Anchor::Whole => panic!("scene has a passage note"),
            }
        };
        app.screens.decision.notes.seed(
            &editing.decision,
            vec![Draft {
                text: token(),
                range,
                widened: false,
            }],
        );
        let rt = runtime(app);
        let elements = rt.elements();
        let body = text_element(&elements, &token());
        let margin = keyed(&elements, if whole { "decision-mtop" } else { "mn-0" });
        inside_x(body.rect, margin.rect);
        assert!(
            body.rect[3] > rt.state.theme().ty.ui.line_height * 2.0,
            "the saved token adds lines: {:?}",
            body.rect
        );
    }
}

#[test]
fn reading_layout_multiline_composers_keep_the_caret_inside() {
    for name in ["start", "conversation-steer-insert"] {
        let mut app = scene(name);
        let key = if name == "start" {
            app.screens.start.editor.edit.replace_text(token());
            "start-composer"
        } else {
            app.screens
                .conversation
                .conv_mut(&tau_client::ConvRef::new("notes-cli", 4))
                .composer
                .edit
                .replace_text(token());
            "composer"
        };
        let rt = runtime(app);
        let elements = rt.elements();
        let field = keyed(&elements, key);
        inside_x(field.caret.expect("composer caret"), field.rect);
        assert!(field.rect[3] >= 2.0 * rt.state.theme().ty.prose.line_height);
    }
}

fn basis(source: &str) -> App {
    let mut fixture = tau_client::Fixture::design().value().clone();
    fixture["calls"]["decision"]["render-core/w1:3:design#2"]["result"]["text"] = source.into();
    let mut app = App {
        link: tau_client::Link::fixture(tau_client::Fixture::parse(&fixture.to_string()).unwrap()),
        ..App::default()
    };
    scenes::on_fixture(
        &mut app,
        Stage::Item(Item::Decision {
            project: "render-core".into(),
            id: "w1:3:design#2".into(),
        }),
    );
    app.window = [1512.0, 982.0];
    app
}

#[test]
fn reading_layout_code_wraps_inside_the_decision_block() {
    let line = format!("let value = \"{}\";", token());
    let rt = runtime(basis(&format!("```rust\n{line}\n```\n")));
    let elements = rt.elements();
    let block = elements.iter().find(|e| e.role == "code").unwrap();
    let body = text_element(&elements, &line);
    inside_x(body.rect, block.rect);
    assert!(
        body.rect[3] >= 40.0,
        "a long code line adds height: {:?}",
        body.rect
    );
}

#[test]
fn reading_layout_decision_measure_is_840_and_notes_stay_beside_it() {
    let rt = runtime(scene("decision-note-insert"));
    let elements = rt.elements();
    let column = keyed(&elements, "b0");
    let paragraph = "The timeline is the one record that every client reads the same way. An export lets the owner keep it outside the store, compare two exports, and feed it to other tools. The command reaches the running server through its socket. It never opens a store itself, because one process writes a store at a time.";
    let mut fonts =
        rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let font = rt.state.theme().ty.prose;
    let unwrapped = fonts.layout_rich(paragraph, &[], font, 1.0, None);
    let mean = unwrapped.full_width() / paragraph.chars().count() as f32;
    let wrapped = fonts.layout_rich(paragraph, &[], font, 1.0, Some(840.0));
    let lengths: Vec<_> = wrapped
        .lines()
        .map(|line| paragraph[line.text_range()].chars().count())
        .collect();
    eprintln!(
        "Etoile {} px: mean advance {mean:.4} px, {:.2} average characters at 840 px; actual line lengths {lengths:?}",
        font.size,
        840.0 / mean
    );
    assert!(
        (column.rect[2] - 840.0).abs() <= 1.0,
        "decision text column: {:?}",
        column.rect
    );
    let margin = keyed(&elements, "decision-mtop");
    assert!((margin.rect[2] - 320.0).abs() <= 1.0);
    assert!(
        margin.rect[0] >= column.rect[0] + column.rect[2] + 47.0,
        "notes stay beside the column: {:?}, {:?}",
        column.rect,
        margin.rect
    );
    assert!(margin.rect[0] + margin.rect[2] <= 1512.0);
}

#[test]
fn reading_layout_conversation_prose_measure_is_840() {
    let rt = runtime(scene("conversation-streaming"));
    let elements = rt.elements();
    let prose = elements
        .iter()
        .find(|e| {
            e.role == "text"
                && e.text
                    .as_deref()
                    .is_some_and(|s| s.len() > 120 && s.contains(' '))
        })
        .expect("conversation prose");
    assert!(
        (prose.rect[2] - 840.0).abs() <= 1.0,
        "conversation prose: {:?}, {:?}",
        prose.text,
        prose.rect
    );
}

#[test]
fn reading_layout_streaming_code_keeps_wrapped_note_highlights() {
    struct Code {
        source: String,
        streaming: bool,
    }
    fn code_view(code: &Code, _: &mut Cx<Code>) -> El {
        let t = Theme::of(crate::theme::Variant::Ember);
        crate::content::markdown(t, &code.source)
            .streaming(code.streaming)
            .tint([crate::content::Tint {
                range: 10..160,
                active: true,
            }])
            .build()
            .w(320.0)
    }
    for streaming in [false, true] {
        let line = format!("let value = \"{}\";", token());
        let source = format!("```rust\n{line}{}", if streaming { "" } else { "\n```\n" });
        let fonts =
            rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
        let mut rt = Runtime::with_text(
            Code { source, streaming },
            code_view,
            (320.0, 700.0),
            Env::new(true, 1.0),
            fonts,
        );
        rt.frame(1.0);
        let elements = rt.elements();
        let block = elements.iter().find(|e| e.role == "code").unwrap();
        let body = text_element(&elements, &line);
        inside_x(body.rect, block.rect);
        assert!(
            body.rect[3] >= 60.0,
            "code wraps while streaming={streaming}: {:?}",
            body.rect
        );
        let highlights: Vec<_> = elements
            .iter()
            .filter(|e| e.key.as_deref() == Some("note-0"))
            .collect();
        assert!(!highlights.is_empty(), "code keeps the note highlight");
        for highlight in highlights {
            inside_x(highlight.rect, block.rect);
        }
    }
}
