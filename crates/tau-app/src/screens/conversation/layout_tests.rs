//! How the conversation's text axis, its box edge and its gutter could fail (`spec.md` §5.5,
//! §4.7; `visual-language.md` §1.1, §3, §6):
//! - a line starts off the text axis X0: the title or its facts line, a header, prose, a record,
//!   a chrome line, a tool row's mark, opened output, a run-failure line, a queued or unsent row,
//!   the composer's caption or text;
//! - a box (your message, a tool row's tint or cursor band, opened output, the run-failure head,
//!   a record's band, the composer) is not 16 px wider than the text column on each side, so
//!   boxes differ in width or text inside one leaves the axis;
//! - prose, code or the last thing in a row runs past X1, or a tool row's chevron or `details ›`
//!   stops short of it;
//! - at 800 × 600 the gutter or the 16 px shrink instead of the text column.

use std::collections::HashMap;

use rune::{ElementInfo, Runtime};
use tau_client::{Fixture, Link};

use crate::app::{App, Stage, scenes, testing};
use crate::open::Item as OpenItem;

/// The speaker marks' gutter, left of the box edge.
const GUTTER: f32 = 24.0;
/// How far every box reaches past the text column on each side.
const HANG: f32 = 16.0;
/// A mark's slot (`tau.icon.mark_small`).
const MARK_SLOT: f32 = 14.0;
const SCENES: [&str; 7] = [
    "conversation-streaming",
    "conversation-records",
    "conversation-failed",
    "conversation-owned",
    "conversation-hostile",
    "conversation-steer-insert",
    "conversation-joined",
];
const SIZES: [[f32; 2]; 2] = [[1512.0, 982.0], [800.0, 600.0]];

/// A drawn state and the conversation column's lines.
struct Drawn {
    name: String,
    rt: Runtime<App>,
    els: Vec<ElementInfo>,
    /// The column's path, with its trailing `/`.
    column: String,
    /// Where every box starts, the text axis X0, where prose wraps (X1) and where boxes end.
    edge: f32,
    x0: f32,
    x1: f32,
    right: f32,
}

impl Drawn {
    fn new(name: &str, mut app: App, size: [f32; 2]) -> Drawn {
        app.window = size;
        let mut rt = testing::window_at(app, size);
        for _ in 0..3 {
            rt.frame(1.0);
        }
        let els = rt.elements();
        let column = els
            .iter()
            .find(|e| e.key.as_deref() == Some("conv-column"))
            .unwrap_or_else(|| panic!("{name}: no conversation column"));
        let (left, right) = (column.rect[0], column.rect[0] + column.rect[2]);
        Drawn {
            name: format!("{name} at {}×{}", size[0], size[1]),
            column: format!("{}/", column.path),
            edge: left + GUTTER,
            x0: left + GUTTER + HANG,
            x1: right - HANG,
            right,
            els,
            rt,
        }
    }

    fn in_column(&self) -> impl Iterator<Item = &ElementInfo> {
        self.els
            .iter()
            .filter(|e| e.path.starts_with(&self.column) && !e.exiting)
    }

    fn ancestors<'a>(&'a self, e: &'a ElementInfo) -> impl Iterator<Item = &'a ElementInfo> {
        let by_path: HashMap<&str, &ElementInfo> =
            self.els.iter().map(|e| (e.path.as_str(), e)).collect();
        let mut path = e.path.as_str();
        std::iter::from_fn(move || {
            loop {
                path = &path[..path.rfind('/')?];
                if let Some(a) = by_path.get(path) {
                    return Some(*a);
                }
            }
        })
    }

    fn within(&self, e: &ElementInfo, roles: &[&str]) -> bool {
        self.ancestors(e).any(|a| roles.contains(&a.role))
    }

    /// What draws ink: text and icons, each counted once (a chip, not the glyphs inside it).
    fn ink(&self) -> Vec<&ElementInfo> {
        self.in_column()
            .filter(|e| inky(e) && !self.ancestors(e).any(inky))
            .collect()
    }

    fn ink_in<'a>(&'a self, outer: &'a ElementInfo) -> impl Iterator<Item = &'a ElementInfo> {
        let prefix = format!("{}/", outer.path);
        self.ink()
            .into_iter()
            .filter(move |e| e.path.starts_with(&prefix))
    }

    fn in_gutter(&self, e: &ElementInfo) -> bool {
        e.rect[0] + e.rect[2] <= self.edge + 0.5
    }
}

fn inky(e: &ElementInfo) -> bool {
    match e.role {
        "text" | "heading" => e.text.as_deref().is_some_and(|t| !t.trim().is_empty()),
        "icon" | "textbox" => true,
        _ => false,
    }
}

fn right(e: &ElementInfo) -> f32 {
    e.rect[0] + e.rect[2]
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.5
}

/// Whether `f` sits on `e`'s first line, scrolled with it.
fn same_line(e: &ElementInfo, f: &ElementInfo) -> bool {
    let (top, bottom) = (e.rect[1], e.rect[1] + e.rect[3].min(20.0));
    f.path != e.path
        && f.scrolled == e.scrolled
        && f.rect[1] < bottom - 2.0
        && f.rect[1] + f.rect[3] > top + 2.0
}

fn scene(name: &str) -> App {
    let mut app = App::default();
    scenes::all().iter().find(|(n, _)| *n == name).unwrap().1(&mut app);
    app
}

/// `render-core #3` with its texts printing the speaker words: your first message `agent` over
/// a code block, the agent's first text `input` over a code block, a record's body `agent` and
/// `input`.
fn speaker_words() -> App {
    let mut fixture = Fixture::design().value().clone();
    let entries = fixture["streams"]["open render-core #3"]["first"]["ops"][0][1]["entries"]
        .as_array_mut()
        .unwrap();
    for e in entries {
        match e["id"].as_u64() {
            Some(2) => e["model"][0]["content"] = "agent 09:10\n\n```\ninput\n```".into(),
            Some(4) => {
                e["model"][0]["content"][0]["text"] = "input 09:10\n\n```\nagent\n```".into()
            }
            Some(9) => e["data"]["text"] = "agent\n\ninput".into(),
            _ => {}
        }
    }
    let mut app = App::default();
    app.link = Link::fixture(Fixture::parse(&fixture.to_string()).unwrap());
    scenes::on_fixture(
        &mut app,
        Stage::Item(OpenItem::Conversation {
            project: "render-core".into(),
            id: 3,
        }),
    );
    app
}

/// Runs `check` on every conversation scene and on [`speaker_words`], at both sizes.
fn each(mut check: impl FnMut(&Drawn)) {
    for size in SIZES {
        for name in SCENES {
            check(&Drawn::new(name, scene(name), size));
        }
        check(&Drawn::new("speaker words", speaker_words(), size));
    }
}

#[test]
fn every_line_starts_on_the_text_axis() {
    each(|d| {
        let ink: Vec<_> = d.ink().into_iter().filter(|e| !d.in_gutter(e)).collect();
        let middle = (d.x0 + d.x1) / 2.0;
        for e in &ink {
            if d.within(e, &["code", "table", "list", "blockquote", "image"]) {
                continue;
            }
            let line: Vec<_> = ink.iter().filter(|f| same_line(e, f)).collect();
            if line.iter().any(|f| right(f) <= e.rect[0] + 0.5) {
                continue;
            }
            let ends = line.iter().map(|f| right(f)).fold(right(e), f32::max);
            if e.rect[0] > middle && near(ends, d.x1) {
                continue;
            }
            let x = if e.role == "icon" {
                e.rect[0] + e.rect[2] / 2.0 - MARK_SLOT / 2.0
            } else {
                e.rect[0]
            };
            assert!(
                near(x, d.x0),
                "{}: {} {:?} starts its line at {x}, not on X0 {}",
                d.name,
                e.role,
                e.text,
                d.x0
            );
        }
    });
}

fn yours(d: &Drawn, e: &ElementInfo) -> bool {
    let human = d.rt.state.theme().c.tint.human;
    d.ancestors(e).any(|a| a.fill == human)
}

#[test]
fn every_box_spans_the_box_edge() {
    each(|d| {
        let mut boxes = 0;
        for e in d.in_column() {
            if e.fill.a == 0.0
                || d.in_gutter(e)
                || matches!(e.role, "text" | "heading" | "mark" | "icon" | "code")
                || d.within(e, &["code", "table", "list", "blockquote", "image", "text"])
            {
                continue;
            }
            boxes += 1;
            let (l, r) = (e.rect[0], right(e));
            if yours(d, e) {
                assert!(
                    l >= d.edge && l <= d.edge + 1.0 && r <= d.right && r >= d.right - 1.0,
                    "{}: a band in your message spans it inside its hairline, not {l}..{r}",
                    d.name
                );
            } else {
                assert!(
                    near(l, d.edge) && near(r, d.right),
                    "{}: {} {:?} {:?} spans {l}..{r}, not the box edge {}..{}",
                    d.name,
                    e.role,
                    e.key,
                    e.fill,
                    d.edge,
                    d.right
                );
            }
        }
        assert!(boxes > 0, "{}: the composer at least", d.name);
    });
}

#[test]
fn nothing_runs_past_x1_and_each_row_ends_on_it() {
    each(|d| {
        for e in d.ink() {
            if d.in_gutter(e) {
                continue;
            }
            let r = if d.within(e, &["table"]) {
                e.visible[0] + e.visible[2]
            } else {
                right(e)
            };
            assert!(
                r <= d.x1 + 0.5,
                "{}: {} {:?} ends at {r}, past X1 {}",
                d.name,
                e.role,
                e.text,
                d.x1
            );
        }
        let mut rows = 0;
        for row in d.in_column().filter(|e| {
            e.role == "button"
                && e.key
                    .as_deref()
                    .is_some_and(|k| k.starts_with("conv-tool-") || k.starts_with("conv-fold-"))
        }) {
            rows += 1;
            let ends = d.ink_in(row).map(right).fold(0.0, f32::max);
            assert!(
                near(ends, d.x1),
                "{}: {:?}'s chevron ends at {ends}, not X1 {}",
                d.name,
                row.key,
                d.x1
            );
        }
        let details = d
            .in_column()
            .find(|e| {
                e.role == "button" && d.ink_in(e).any(|t| t.text.as_deref() == Some("details"))
            })
            .expect("details ›");
        assert!(
            near(right(details), d.x1),
            "{}: details › ends on X1",
            d.name
        );
        if d.name.starts_with("conversation-streaming") {
            assert!(rows > 0);
        }
    });
}
