//! How a bottom float's motion can fail (`visual-language.md` §7, `spec.md` §4 and §6):
//! - which-key draws a table it wasn't showing (the root table, for one frame after `Esc` or a
//!   finished sequence) because whether it shows outlives the pending keys;
//! - which-key leaves between two keys of one sequence and comes back after a new wait;
//! - a float appears or leaves in one frame, or fades, instead of its top edge gliding;
//! - the edge jumps when a key retargets it mid-glide, a frame lacks a float that is due, or
//!   shows two, or shows the last float's content where a new one is due;
//! - the page blanks above a rising edge;
//! - what the float covers moves before or after the edge, or by more than it is covered;
//! - what it never reaches moves at all;
//! - a key after the closing key waits for the float to leave;
//! - motion off, or a close that changes the page, still glides.
use super::*;
use rune::{Env, KeyEvent, Mods, Runtime};

const FRAME: f64 = 1.0 / 60.0;

fn window(scene: &str, size: (f32, f32)) -> Runtime<App> {
    let mut a = App::default();
    scenes::all().iter().find(|(n, _)| *n == scene).unwrap().1(&mut a);
    a.window = [size.0, size.1];
    let text = rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = Runtime::with_text(a, view, size, Env::new(true, 1.0), text);
    rt.on_key = Some(input::on_key);
    rt.poll = Some(poll);
    frames(&mut rt, 30);
    rt
}

fn frames(rt: &mut Runtime<App>, n: usize) {
    for _ in 0..n {
        rt.frame(FRAME);
    }
}

/// One key as the window sends it, and the frame that handles it.
fn key(rt: &mut Runtime<App>, key: rune::Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
}

fn palette(rt: &mut Runtime<App>) {
    rt.key(KeyEvent {
        key: ch('k'),
        mods: Mods {
            cmd: true,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
}

fn rect(rt: &Runtime<App>, key: &str) -> Option<[f32; 4]> {
    rt.elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key) && !e.exiting)
        .map(|e| e.rect)
}

fn bottom(r: [f32; 4]) -> f32 {
    r[1] + r[3]
}

/// The stage's foot, where a float rises from.
fn foot(rt: &Runtime<App>) -> f32 {
    bottom(rect(rt, "stage").unwrap())
}

/// The float's top edge on each of `n` frames, with the rect of `rider`.
fn glide(rt: &mut Runtime<App>, n: usize, rider: &str) -> Vec<(Option<Drawn>, Option<[f32; 4]>)> {
    let mut out = vec![(drawn(rt), rect(rt, rider))];
    for _ in 1..n {
        rt.frame(FRAME);
        out.push((drawn(rt), rect(rt, rider)));
    }
    out
}

/// The edges a glide drew, the stage's foot where no float was.
fn edges(rt: &Runtime<App>, g: &[(Option<Drawn>, Option<[f32; 4]>)]) -> Vec<f32> {
    let foot = foot(rt);
    g.iter()
        .map(|(d, _)| d.as_ref().map_or(foot, |d| d.rect[1]))
        .collect()
}

/// How many frames the edge was between `from` and `to`, not at either.
fn moving_frames(edges: &[f32], from: f32, to: f32) -> usize {
    let (lo, hi) = (from.min(to), from.max(to));
    edges
        .iter()
        .filter(|e| **e > lo + 0.5 && **e < hi - 0.5)
        .count()
}

/// The farthest the edge may travel in one frame: a spring's step from rest, at its fastest.
const MOST_PER_FRAME: f32 = 90.0;

fn ch(c: char) -> rune::Key {
    rune::Key::Text(c.to_string())
}

/// The float drawn this frame: its surface's rect and the words it shows.
#[derive(Clone, Debug, PartialEq)]
struct Drawn {
    rect: [f32; 4],
    opacity: f32,
    words: Vec<String>,
}

fn drawn(rt: &Runtime<App>) -> Option<Drawn> {
    let els = rt.elements();
    let surfaces: Vec<_> = els
        .iter()
        .filter(|e| e.key.as_deref() == Some("float-surface") && !e.exiting)
        .collect();
    assert!(surfaces.len() <= 1, "two floats drawn");
    let s = surfaces.first()?;
    let inside = format!("{}/", s.path);
    Some(Drawn {
        rect: s.rect,
        opacity: s.opacity,
        words: els
            .iter()
            .filter(|e| e.path.starts_with(&inside))
            .filter_map(|e| e.text.clone())
            .collect(),
    })
}

/// Waits for which-key to rise, at most a second.
fn until_drawn(rt: &mut Runtime<App>) -> Drawn {
    for _ in 0..60 {
        rt.frame(FRAME);
        if let Some(d) = drawn(rt) {
            return d;
        }
    }
    panic!("which-key never showed");
}

/// For each way a which-key sequence ends, the frames from its last key on: no float drawn is
/// taller than which-key was, and none shows another table.
#[test]
fn which_key_leaves_with_the_table_it_showed() {
    for (before, end) in [
        (vec![ch(' ')], rune::Key::Escape),
        (vec![ch(' '), ch('o')], ch('h')),
        (vec![ch(' ')], ch('z')),
    ] {
        let mut rt = window("conversation-streaming", (1512.0, 982.0));
        let mut shown = None;
        for k in before.clone() {
            key(&mut rt, k);
            shown = Some(until_drawn(&mut rt));
        }
        let shown = shown.unwrap();
        key(&mut rt, end.clone());
        for frame in 0..40 {
            if let Some(d) = drawn(&rt) {
                assert!(
                    d.rect[3] <= shown.rect[3] + 0.5,
                    "{end:?}, frame {frame}: {} tall, was {}",
                    d.rect[3],
                    shown.rect[3]
                );
                assert_eq!(d.words, shown.words, "{end:?}, frame {frame}");
            }
            rt.frame(FRAME);
        }
    }
}

/// `␣`, then `o` 600 ms later: which-key shows on every frame from when it rises, and `o`'s
/// table from the frame of the `o` key, with no new wait.
#[test]
fn which_key_stays_up_for_the_whole_sequence() {
    let mut rt = window("conversation-streaming", (1512.0, 982.0));
    key(&mut rt, ch(' '));
    let pressed = rt.state.now;
    let root = until_drawn(&mut rt);
    while rt.state.now - pressed < 0.6 {
        rt.frame(FRAME);
        assert!(drawn(&rt).is_some(), "which-key left before the next key");
    }
    key(&mut rt, ch('o'));
    let open = drawn(&rt).expect("which-key on the frame of the o key");
    assert_ne!(open.words, root.words);
    assert!(open.words.iter().any(|w| w == "open"), "{:?}", open.words);
    for frame in 0..30 {
        rt.frame(FRAME);
        assert_eq!(
            drawn(&rt).map(|d| d.words),
            Some(open.words.clone()),
            "frame {frame}"
        );
    }
}

/// Waits for which-key, or opens the palette, and returns the frames of the rise from there.
fn rise(
    rt: &mut Runtime<App>,
    which_key: bool,
    rider: &str,
) -> Vec<(Option<Drawn>, Option<[f32; 4]>)> {
    if which_key {
        key(rt, ch(' '));
        while drawn(rt).is_none() {
            rt.frame(FRAME);
        }
    } else {
        palette(rt);
    }
    glide(rt, 40, rider)
}

/// The palette and which-key rise from the stage's foot, opaque, over at least six frames, and the
/// composer rides their edge the same distance above it on every frame.
#[test]
fn a_float_rises_from_the_foot_with_the_composer_on_its_edge() {
    for which_key in [false, true] {
        let mut rt = window("conversation-streaming", (1512.0, 982.0));
        let foot = foot(&rt);
        let gap = foot - bottom(rect(&rt, "composer-container").unwrap());
        let g = rise(&mut rt, which_key, "composer-container");
        let e = edges(&rt, &g);
        let rest = *e.last().unwrap();
        assert!(rest < foot - 150.0, "{which_key}: {e:?}");
        assert!(
            e[0] > rest + 150.0,
            "{which_key}: it starts at the foot: {e:?}"
        );
        assert!(
            e.windows(2).all(|w| w[1] <= w[0] + 0.01),
            "{which_key}: {e:?}"
        );
        assert!(moving_frames(&e, foot, rest) >= 6, "{which_key}: {e:?}");
        for (i, (d, composer)) in g.iter().enumerate() {
            let d = d.as_ref().expect("a float on every frame");
            assert_eq!(d.opacity, 1.0, "{which_key}, frame {i}");
            let at = d.rect[1] - bottom(composer.unwrap());
            assert!(
                (at - gap).abs() <= 1.0,
                "{which_key}, frame {i}: {at} above, not {gap}"
            );
        }
    }
}

/// `Esc` lowers the float the way it rose: its edge glides to the foot over at least six frames,
/// still showing what it showed, with the composer on it. The float holds no keys from `Esc` on.
#[test]
fn esc_lowers_the_float_the_way_it_rose() {
    let mut rt = window("conversation-streaming", (1512.0, 982.0));
    let foot = foot(&rt);
    let composer = rect(&rt, "composer-container").unwrap();
    let gap = foot - bottom(composer);
    palette(&mut rt);
    frames(&mut rt, 40);
    let open = drawn(&rt).unwrap();
    key(&mut rt, rune::Key::Escape);
    assert!(
        rt.state.floats.is_empty(),
        "the page has the keys from Esc on"
    );
    let g = glide(&mut rt, 40, "composer-container");
    let e = edges(&rt, &g);
    assert!(e[0] < foot - 150.0, "it doesn't leave at once: {e:?}");
    assert!(e.windows(2).all(|w| w[1] >= w[0] - 0.01), "{e:?}");
    assert!(moving_frames(&e, open.rect[1], foot) >= 6, "{e:?}");
    for (i, (d, c)) in g.iter().enumerate() {
        let at = e[i] - bottom(c.unwrap());
        assert!((at - gap).abs() <= 1.0, "frame {i}: {at} above, not {gap}");
        if let Some(d) = d {
            assert_eq!(d.opacity, 1.0, "frame {i}");
            assert_eq!(d.words, open.words, "frame {i}");
        }
    }
    assert!(drawn(&rt).is_none(), "gone once down");
    assert_eq!(rect(&rt, "composer-container"), Some(composer));
}

/// A key 20 ms after `Esc` acts on the page on its own frame, while the float still leaves.
#[test]
fn the_key_after_esc_acts_on_the_page_at_once() {
    let mut rt = window("home", (1512.0, 982.0));
    palette(&mut rt);
    frames(&mut rt, 40);
    let was = rt.state.screens.home.cursor.id.clone();
    key(&mut rt, rune::Key::Escape);
    key(&mut rt, ch('j'));
    assert_ne!(rt.state.screens.home.cursor.id, was, "j moved the cursor");
    let cursor = format!(
        "list-{:?}",
        rt.state.screens.home.cursor.id.as_ref().unwrap()
    );
    assert!(rect(&rt, &cursor).is_some(), "and the frame shows it");
    assert!(drawn(&rt).is_some(), "while the float is leaving");
}

/// The rows of a list, by key, as drawn this frame.
fn rows(rt: &Runtime<App>) -> Vec<(String, [f32; 4])> {
    rt.elements()
        .into_iter()
        .filter(|e| !e.exiting && e.key.as_deref().is_some_and(|k| k.starts_with("list-")))
        .map(|e| (e.key.unwrap(), e.rect))
        .collect()
}

/// What the float would cover rides its edge once the edge reaches it, `float.inset_x` above it
/// or as far as it stood above the stage's `foot` if that is less, and only then.
fn rides(foot: f32, edge: f32, rest: [f32; 4], now: [f32; 4], label: &str) {
    let gap = crate::theme::tokens()
        .layout
        .float
        .inset_x
        .min(foot - bottom(rest));
    assert!((now[0] - rest[0]).abs() < 0.01, "{label}: moved sideways");
    if now[1] < rest[1] - 0.5 {
        let at = edge - bottom(now);
        assert!(
            (at - gap).abs() <= 1.0,
            "{label}: {at} above the edge, not {gap}"
        );
    } else {
        assert!((now[1] - rest[1]).abs() < 0.5, "{label}: moved down");
        assert!(edge - bottom(now) >= gap - 1.0, "{label}: under the edge");
    }
}

/// The start page's composer stays put through every float that doesn't reach it (1512 × 982),
/// and at 800 × 600, where the palette would cover it, rides the edge up and back.
#[test]
fn the_start_composer_moves_only_when_a_float_would_cover_it() {
    let mut rt = window("start", (1512.0, 982.0));
    key(&mut rt, rune::Key::Escape);
    let at = rect(&rt, "start-composer").unwrap();
    let steps: [fn(&mut Runtime<App>); 4] = [
        palette,
        |rt| key(rt, rune::Key::Escape),
        |rt| key(rt, ch(' ')),
        |rt| key(rt, rune::Key::Escape),
    ];
    for (i, step) in steps.into_iter().enumerate() {
        step(&mut rt);
        for frame in 0..50 {
            assert_eq!(
                rect(&rt, "start-composer"),
                Some(at),
                "step {i}, frame {frame}"
            );
            rt.frame(FRAME);
        }
    }

    let mut rt = window("start", (800.0, 600.0));
    key(&mut rt, rune::Key::Escape);
    let foot = foot(&rt);
    let rest = rect(&rt, "start-block").unwrap();
    palette(&mut rt);
    let g = glide(&mut rt, 40, "start-block");
    let e = edges(&rt, &g);
    for (i, (_, b)) in g.iter().enumerate() {
        rides(foot, e[i], rest, b.unwrap(), &format!("rising, frame {i}"));
    }
    assert!(
        g.last().unwrap().1.unwrap()[1] < rest[1] - 10.0,
        "the palette would cover it"
    );
    key(&mut rt, rune::Key::Escape);
    let g = glide(&mut rt, 40, "start-block");
    let e = edges(&rt, &g);
    let mut last = g[0].1.unwrap()[1];
    for (i, (_, b)) in g.iter().enumerate() {
        let b = b.unwrap();
        assert!(b[1] >= last - 0.01, "lowering, frame {i}: moved up");
        last = b[1];
        rides(foot, e[i], rest, b, &format!("lowering, frame {i}"));
    }
    assert_eq!(rect(&rt, "start-block"), Some(rest), "back where it was");
}

/// Home's cursor on the last Activity row the stage shows whole: the palette's edge carries it up
/// once it reaches it, and the rows above go with it, straight up, on every frame.
#[test]
fn a_covered_home_row_rides_the_edge_with_the_rows_above() {
    let mut rt = window("home", (1512.0, 982.0));
    let foot = foot(&rt);
    let last = rows(&rt)
        .into_iter()
        .rfind(|(k, r)| k.starts_with("list-Event") && bottom(*r) <= foot)
        .unwrap()
        .0;
    rt.update(|a, _| {
        let id = screens::home::rows(a)
            .into_iter()
            .find(|r| format!("list-{:?}", r.id) == last)
            .unwrap()
            .id;
        screens::home::select(a, id);
    });
    frames(&mut rt, 5);
    let before = rows(&rt);
    let cursor = before.iter().find(|(k, _)| *k == last).unwrap().1;
    palette(&mut rt);
    let mut frames_seen = Vec::new();
    for _ in 0..40 {
        frames_seen.push((drawn(&rt).unwrap().rect[1], rows(&rt)));
        rt.frame(FRAME);
    }
    let (rest_edge, _) = frames_seen.last().unwrap().clone();
    assert!(
        bottom(cursor) > rest_edge,
        "the palette would cover the row"
    );
    for (i, (edge, now)) in frames_seen.iter().enumerate() {
        let r = now.iter().find(|(k, _)| *k == last).unwrap().1;
        rides(foot, *edge, cursor, r, &format!("frame {i}"));
        let dy = r[1] - cursor[1];
        for (k, was) in before.iter().filter(|(_, was)| bottom(*was) <= rest_edge) {
            let r = now
                .iter()
                .find(|(n, _)| n == k)
                .unwrap_or_else(|| panic!("frame {i}: {k} gone"))
                .1;
            assert!(
                (r[0] - was[0]).abs() < 0.01,
                "frame {i}: {k} moved sideways"
            );
            assert!(
                (r[1] - was[1] - dy).abs() < 0.5,
                "frame {i}: {k} left the cursor row"
            );
        }
    }
}

/// With motion off, every float lands on the frame of its key, and leaves on it.
#[test]
fn with_motion_off_every_float_lands_on_its_frame() {
    let mut rt = window("home-motion-off", (1512.0, 982.0));
    palette(&mut rt);
    let open = drawn(&rt).expect("open on its frame").rect;
    frames(&mut rt, 20);
    assert_eq!(drawn(&rt).unwrap().rect, open);
    key(&mut rt, rune::Key::Escape);
    assert!(drawn(&rt).is_none(), "gone on its frame");
    key(&mut rt, ch(' '));
    let first = until_drawn(&mut rt).rect;
    frames(&mut rt, 20);
    assert_eq!(drawn(&rt).unwrap().rect, first);
    key(&mut rt, ch('o'));
    let open = drawn(&rt).unwrap().rect;
    frames(&mut rt, 20);
    assert_eq!(drawn(&rt).unwrap().rect, open);
    key(&mut rt, rune::Key::Escape);
    assert!(drawn(&rt).is_none(), "which-key gone on its frame");
}

/// A close that changes the page lands with the page: no float on the frame of its key.
#[test]
fn a_close_that_changes_the_page_lands_at_once() {
    let mut rt = window("find", (1512.0, 982.0));
    let was = rt.state.stage.clone();
    key(&mut rt, rune::Key::Enter);
    assert_ne!(rt.state.stage, was, "Enter opened the row");
    assert!(drawn(&rt).is_none(), "Find gone on its frame");
    let mut rt = window("conversation-streaming", (1512.0, 982.0));
    key(&mut rt, ch(' '));
    until_drawn(&mut rt);
    key(&mut rt, ch('o'));
    key(&mut rt, ch('h'));
    assert_eq!(rt.state.stage, Stage::Place(crate::frame::Place::Home));
    assert!(drawn(&rt).is_none(), "which-key gone on its frame");
}

/// Keys faster than the spring: the edge goes from where it is to where it must go, with a float
/// on every frame one is due, and only what is due in it.
#[test]
fn keys_faster_than_the_spring_retarget_the_edge() {
    let continuous = |e: &[f32], label: &str| {
        for (i, w) in e.windows(2).enumerate() {
            assert!(
                (w[1] - w[0]).abs() <= MOST_PER_FRAME,
                "{label}, frame {i}: {e:?}"
            );
        }
    };

    let mut rt = window("conversation-streaming", (1512.0, 982.0));
    palette(&mut rt);
    frames(&mut rt, 40);
    let open = drawn(&rt).unwrap();
    let mut e = vec![open.rect[1]];
    key(&mut rt, rune::Key::Escape);
    e.push(drawn(&rt).expect("leaving").rect[1]);
    rt.frame(FRAME);
    e.push(drawn(&rt).expect("leaving").rect[1]);
    rt.key(KeyEvent {
        key: ch(' '),
        mods: Mods::default(),
    });
    rt.key(KeyEvent {
        key: ch(';'),
        mods: Mods::default(),
    });
    rt.frame(FRAME);
    for frame in 0..40 {
        let d = drawn(&rt).unwrap_or_else(|| panic!("no float, frame {frame}"));
        assert_eq!(d.words, open.words, "the palette again, frame {frame}");
        e.push(d.rect[1]);
        rt.frame(FRAME);
    }
    continuous(&e, "Esc then ␣;");
    assert!((e.last().unwrap() - open.rect[1]).abs() < 0.5);

    let mut rt = window("conversation-streaming", (1512.0, 982.0));
    let foot = foot(&rt);
    palette(&mut rt);
    frames(&mut rt, 2);
    key(&mut rt, rune::Key::Escape);
    let g = glide(&mut rt, 40, "composer-container");
    let e = edges(&rt, &g);
    continuous(&e, "closed 50 ms after it opened");
    assert!(e[0] < foot - 1.0, "still drawn as it turns: {e:?}");
    assert_eq!(*e.last().unwrap(), foot);

    let mut rt = window("conversation-streaming", (1512.0, 982.0));
    palette(&mut rt);
    frames(&mut rt, 40);
    let mut e = vec![drawn(&rt).unwrap().rect[1]];
    for _ in 0..4 {
        rt.key(KeyEvent {
            key: ch('z'),
            mods: Mods::default(),
        });
    }
    rt.frame(FRAME);
    for frame in 0..40 {
        let d = drawn(&rt).unwrap();
        assert!(
            d.words.iter().any(|w| w == "No command matches \"zzzz\""),
            "frame {frame}: {:?}",
            d.words
        );
        e.push(d.rect[1]);
        rt.frame(FRAME);
    }
    continuous(&e, "zzzz");
    assert!(
        e.last().unwrap() > &(e[0] + 100.0),
        "the palette shrank to its one line: {e:?}"
    );
}

/// A float drawn as it leaves holds no field rune can reach: binding a project (whose Confirm
/// closes, then the bind float under it) runs to the end.
#[test]
fn a_leaving_float_holds_no_field() {
    let mut rt = window("bind-float", (1512.0, 982.0));
    key(&mut rt, rune::Key::Enter);
    key(&mut rt, ch('y'));
    frames(&mut rt, 40);
    assert!(rt.state.floats.is_empty(), "bound");
}

/// Every page with a cursor holds what it acts on above a float that would cover it (`spec.md`
/// §4), at 800 × 600, where the palette reaches furthest: the row, line or field rides the edge,
/// and one the palette doesn't reach (Projects has three rows) stays put.
#[test]
fn every_page_holds_what_you_act_on_above_the_edge() {
    type Case = (
        &'static str,
        fn(&mut Runtime<App>),
        fn(&App) -> String,
        bool,
    );
    let at_end = |rt: &mut Runtime<App>| key(rt, ch('G'));
    let cases: [Case; 4] = [
        (
            "timeline",
            at_end,
            |a| {
                format!(
                    "list-{:?}",
                    screens::places::settings(a).cursor.id.as_ref().unwrap()
                )
            },
            true,
        ),
        (
            "projects",
            at_end,
            |a| {
                format!(
                    "project-row-{}",
                    a.screens.project.list_cursor.as_ref().unwrap()
                )
            },
            false,
        ),
        (
            "project",
            at_end,
            |a| match a.screens.project.cursor.as_ref().unwrap() {
                screens::project::Target::Field(f) => format!("option-{}-{}", f.extension, f.key),
                screens::project::Target::Work(p) => format!("work-{p:?}"),
            },
            true,
        ),
        ("decision-long", at_end, |_| "decision-cursor".into(), true),
    ];
    for (scene, place, target, covered) in cases {
        let mut rt = window(scene, (800.0, 600.0));
        place(&mut rt);
        frames(&mut rt, 30);
        let key = target(&rt.state);
        let rest = rect(&rt, &key).unwrap_or_else(|| panic!("{scene}: no {key}"));
        palette(&mut rt);
        let g = glide(&mut rt, 40, &key);
        let e = edges(&rt, &g);
        assert_eq!(
            bottom(rest) > e[e.len() - 1],
            covered,
            "{scene}: whether the palette would cover {key}"
        );
        for (i, (_, r)) in g.iter().enumerate() {
            rides(
                foot(&rt),
                e[i],
                rest,
                r.unwrap(),
                &format!("{scene}, frame {i}"),
            );
        }
    }
}

/// A conversation's cursor on a tool row above the queue: once the queue, riding the float's
/// edge, reaches it, it rides with them the same distance above the edge on every frame, rather
/// than jumping above them once the float is up. It comes back down the same way.
#[test]
fn a_conversation_cursor_rides_the_queue_up_and_down() {
    let mut rt = window("conversation-streaming", (1512.0, 982.0));
    key(&mut rt, ch('k'));
    frames(&mut rt, 30);
    let stop = "conv-fold-toolu_04";
    let rest = rect(&rt, stop).expect("the cursor on the last fold");
    let check = |g: &[(Option<Drawn>, Option<[f32; 4]>)], e: &[f32], way: &str| {
        let mut held = None;
        for (i, (_, r)) in g.iter().enumerate() {
            let r = r.unwrap();
            if (r[1] - rest[1]).abs() < 0.5 {
                assert!(
                    held.is_none(),
                    "{way}, frame {i}: back at rest before the end"
                );
                continue;
            }
            assert!(r[1] < rest[1], "{way}, frame {i}: moved down");
            let at = e[i] - bottom(r);
            let held = *held.get_or_insert(at);
            assert!(
                (at - held).abs() <= 1.0,
                "{way}, frame {i}: {at} above the edge, not {held}"
            );
        }
    };
    palette(&mut rt);
    let g = glide(&mut rt, 40, stop);
    let e = edges(&rt, &g);
    assert!(
        g[g.len() - 1].1.unwrap()[1] < rest[1] - 50.0,
        "the queue would cover it"
    );
    check(&g, &e, "rising");
    key(&mut rt, rune::Key::Escape);
    let mut g = glide(&mut rt, 40, stop);
    let mut e = edges(&rt, &g);
    g.reverse();
    e.reverse();
    check(&g, &e, "lowering");
    assert_eq!(rect(&rt, stop), Some(rest), "back where it was");
}
