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

fn ch(c: char) -> rune::Key {
    rune::Key::Text(c.to_string())
}

/// The float drawn this frame: its surface's rect and the words it shows.
#[derive(Clone, Debug, PartialEq)]
struct Drawn {
    rect: [f32; 4],
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
