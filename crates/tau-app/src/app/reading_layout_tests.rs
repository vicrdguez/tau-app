//! Failure modes:
//! - A long token escapes the note box in Insert, or the margin after leaving Insert.
//! - Whole and passage notes wrap differently, or only clip instead of adding lines.
//! - A code line hides its tail in a horizontal viewport instead of growing taller.
//! - Wrapped code loses note-range highlights or stops wrapping while streaming.
//! - Reading columns stay narrow, or widening the decision folds notes at 1512 px.
//! - Opening Details moves a reading column that already clears the panel, moves it further than
//!   its shortfall, or narrows it before it reaches its 16 px margin; the decision's text column
//!   jumps when its margin folds; the cursor's paragraph moves on screen or out of view.

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
fn reading_layout_uses_the_supplied_measure_without_changing_the_peek() {
    fn measured_view<const WIDTH: u16>(app: &App, _: &mut Cx<App>) -> El {
        let t = Theme::new(
            crate::theme::Variant::Ember,
            crate::theme::Measure::new(f32::from(WIDTH)).unwrap(),
            true,
        );
        crate::screens::get(crate::screens::of(&app.stage)).view(t, app)
    }
    fn markdown_view(state: &(Theme, bool), _: &mut Cx<(Theme, bool)>) -> El {
        let source = "| first | second | third |\n| --- | --- | --- |\n| a long cell with many words repeated to fill the table width | another long cell with many words repeated to fill the table width | a third long cell with many words repeated to fill the table width |";
        let md = crate::content::markdown(state.0, source);
        if state.1 {
            md.peek().build()
        } else {
            md.build()
        }
    }
    for px in [600.0, 1200.0] {
        let t = Theme::new(
            crate::theme::Variant::Ember,
            crate::theme::Measure::new(px).unwrap(),
            true,
        );
        for name in ["conversation-streaming", "decision-note-insert"] {
            let mut app = scene(name);
            app.window = [2000.0, 982.0];
            let fonts = rune::text::TextSystem::new(
                crate::assets::fonts().unwrap(),
                crate::theme::FAMILIES,
            );
            let mut rt = Runtime::with_text(
                app,
                if px == 600.0 {
                    measured_view::<600>
                } else {
                    measured_view::<1200>
                },
                (2000.0, 982.0),
                Env::new(true, 1.0),
                fonts,
            );
            rt.frame(1.0);
            let elements = rt.elements();
            if name.starts_with("conversation") {
                assert!((keyed(&elements, "conv-column").rect[2] - (px + 40.0)).abs() <= 1.0);
                let prose = elements
                    .iter()
                    .find(|e| {
                        e.role == "text"
                            && e.text
                                .as_deref()
                                .is_some_and(|s| s.len() > 120 && s.contains(' '))
                    })
                    .unwrap();
                assert!((prose.rect[2] - px).abs() <= 1.0);
            } else {
                let column = keyed(&elements, "b0");
                let margin = keyed(&elements, "decision-mtop");
                assert!((column.rect[2] - px).abs() <= 1.0);
                assert!((margin.rect[0] - column.rect[0] - px - 48.0).abs() <= 1.0);
            }
        }
        for peek in [false, true] {
            let fonts = rune::text::TextSystem::new(
                crate::assets::fonts().unwrap(),
                crate::theme::FAMILIES,
            );
            let mut rt = Runtime::with_text(
                (t, peek),
                markdown_view,
                (2000.0, 982.0),
                Env::new(true, 1.0),
                fonts,
            );
            rt.frame(1.0);
            let elements = rt.elements();
            let table = elements
                .iter()
                .find(|e| e.role == "row")
                .expect("Markdown table row");
            let expected = if peek { 370.0 } else { px };
            assert!(
                (table.rect[2] - expected).abs() <= 1.0,
                "table {:?}, expected {expected}",
                table.rect
            );
        }
    }
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

fn at(name: &str, size: [f32; 2]) -> Runtime<App> {
    let mut app = scene(name);
    app.window = size;
    let fonts =
        rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = Runtime::with_text(app, view, (size[0], size[1]), Env::new(true, 1.0), fonts);
    advance(&mut rt);
    rt.update(|a, tx| {
        poll(a, a.now + 1.0, tx);
    });
    advance(&mut rt);
    rt
}

fn rect_of(rt: &Runtime<App>, key: &str) -> [f32; 4] {
    keyed(&rt.elements(), key).rect
}

/// `key`'s rect with Details closed, then open, toggled as `␣td` does.
fn details(rt: &mut Runtime<App>, key: &str) -> ([f32; 4], [f32; 4]) {
    let closed = rect_of(rt, key);
    rt.update(|a, tx| commands::run_id(a, "toggle.details", tx));
    for _ in 0..30 {
        rt.frame(0.05);
    }
    assert!(rt.state.details_visible);
    (closed, rect_of(rt, key))
}

#[test]
fn reading_layout_details_moves_the_conversation_only_by_its_shortfall() {
    let (closed, open) = details(
        &mut at("conversation-streaming", [1512.0, 982.0]),
        "conv-column",
    );
    eprintln!(
        "1512: conversation column x {} closed, {} open",
        closed[0], open[0]
    );
    assert_eq!((closed[0], closed[2]), (440.0, 880.0));
    assert_eq!(open[2], 880.0, "its lines don't break again");
    assert_eq!(
        open[0] + open[2],
        1512.0 - 240.0 - 24.0,
        "it moves only until it clears the panel by 24 px"
    );
    let (closed, open) = details(
        &mut at("conversation-streaming", [1800.0, 982.0]),
        "conv-column",
    );
    assert_eq!(open, closed, "wide: nothing moves");
    let (closed, open) = details(
        &mut at("conversation-streaming", [1100.0, 982.0]),
        "conv-column",
    );
    assert_eq!(
        (closed[0], closed[2]),
        (248.0 + 16.0, 1100.0 - 248.0 - 32.0)
    );
    assert_eq!(open[0], 248.0 + 16.0, "at its margin");
    assert_eq!(open[2], 1100.0 - 248.0 - 240.0 - 32.0, "narrowed only then");
}

#[test]
fn reading_layout_details_keeps_the_decision_text_column_where_it_was() {
    let (closed, open) = details(&mut at("decision-long", [1512.0, 982.0]), "b0");
    assert_eq!(closed[0], 248.0 + (1264.0 - 1208.0) / 2.0);
    assert_eq!(open[0], closed[0], "the margin folds, the text stays");
    assert_eq!(open[2], 840.0);
}

#[test]
fn reading_layout_details_keeps_the_decision_cursor_in_place() {
    for size in [[1512.0, 982.0], [800.0, 600.0]] {
        let mut rt = at("decision-long", size);
        let (closed, open) = details(&mut rt, "decision-cursor");
        assert!(
            (open[1] - closed[1]).abs() <= 1.0,
            "{size:?}: the cursor's line stays at y {}, not {}",
            closed[1],
            open[1]
        );
        let page = rect_of(&rt, "decision-page");
        assert!(
            open[1] >= page[1] && open[1] + open[3] <= page[1] + page[3],
            "{size:?}: in view: {open:?} in {page:?}"
        );
        rt.update(|a, tx| commands::run_id(a, "toggle.details", tx));
        for _ in 0..30 {
            rt.frame(0.05);
        }
        let back = rect_of(&rt, "decision-cursor");
        assert!(
            (back[1] - closed[1]).abs() <= 1.0,
            "{size:?}: and through the close"
        );
    }
}
