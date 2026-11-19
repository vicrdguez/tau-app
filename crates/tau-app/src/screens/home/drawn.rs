//! Lists as the whole app draws them, for tests that check where rows put their text.
use super::list::RowId;
use crate::app::App;
use rune::runtime::ElementInfo;
use rune::{Env, Runtime};

/// `app` drawn by the whole app at `w × h`, settled.
pub(crate) fn drawn(mut app: App, w: f32, h: f32) -> Runtime<App> {
    app.window = [w, h];
    let fonts =
        rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = Runtime::with_text(app, crate::app::view, (w, h), Env::new(true, 1.0), fonts);
    for _ in 0..3 {
        rt.frame(1.0);
    }
    rt
}

/// The named scene drawn at `w × h`.
pub(crate) fn scene(name: &str, w: f32, h: f32) -> Runtime<App> {
    let mut a = App::default();
    a.window = [w, h];
    crate::app::scenes::all()
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap()
        .1(&mut a);
    drawn(a, w, h)
}

/// The text elements drawn in the row `id`, the row itself included.
pub(crate) fn row_texts(els: &[ElementInfo], id: &RowId) -> Vec<ElementInfo> {
    let key = format!("list-{id:?}");
    let Some(row) = els.iter().position(|e| e.key.as_deref() == Some(&key)) else {
        panic!("no row {key}");
    };
    let inside = |mut i: usize| loop {
        if i == row {
            return true;
        }
        match els[i].parent {
            Some(p) => i = p,
            None => return false,
        }
    };
    (0..els.len())
        .filter(|&i| inside(i) && els[i].text.is_some())
        .map(|i| els[i].clone())
        .collect()
}

/// The element in `texts` that shows `s`.
pub(crate) fn find<'a>(texts: &'a [ElementInfo], s: &str) -> &'a ElementInfo {
    texts
        .iter()
        .find(|e| e.text.as_deref() == Some(s))
        .unwrap_or_else(|| {
            panic!(
                "no {s:?} in {:?}",
                texts.iter().map(|e| &e.text).collect::<Vec<_>>()
            )
        })
}

/// Clicks the middle of the first drawn text that shows `s`, then draws again.
pub(crate) fn click(rt: &mut Runtime<App>, s: &str) {
    let els = rt.elements();
    let Some(e) = texts(&els, s).into_iter().next() else {
        panic!("nothing shows {s:?}");
    };
    let (x, y) = (e.rect[0] + e.rect[2] / 2.0, e.rect[1] + e.rect[3] / 2.0);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..3 {
        rt.frame(1.0);
    }
}

/// Every text the drawn app shows.
pub(crate) fn shown(rt: &Runtime<App>) -> Vec<String> {
    rt.elements().into_iter().filter_map(|e| e.text).collect()
}

/// Every text element of the drawn app that shows `s`.
pub(crate) fn texts(els: &[ElementInfo], s: &str) -> Vec<ElementInfo> {
    els.iter()
        .filter(|e| e.text.as_deref() == Some(s))
        .cloned()
        .collect()
}
