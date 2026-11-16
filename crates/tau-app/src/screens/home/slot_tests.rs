//! A short Failed timestamp must end at the column's right edge, not start in the
//! 84 px slot and leave about 50 px of blank space after it.
use super::*;
use rune::{Cx, Env, Runtime};

fn view(a: &App, _: &mut Cx<App>) -> El {
    let fault = rows(a)
        .into_iter()
        .find(|r| r.group == "Failed" && r.at.is_some())
        .unwrap();
    super::rows::row_view(a.theme(), a, &fault, false, true)
}

#[test]
fn failed_timestamp_ends_at_the_right_edge_of_its_fixed_slot() {
    let mut a = App::default();
    crate::app::link::settle(&mut a);
    let at = rows(&a)
        .into_iter()
        .find(|r| r.group == "Failed" && r.at.is_some())
        .unwrap()
        .at
        .unwrap();
    let stamp = crate::time::clock(Wall(at));
    let fonts =
        rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = Runtime::with_text(a, view, (744.0, 28.0), Env::new(true, 1.0), fonts);
    rt.frame(0.0);
    let time = rt
        .elements()
        .into_iter()
        .find(|e| e.text.as_deref() == Some(stamp.as_str()))
        .unwrap();
    assert!(
        time.rect[2] < 50.0,
        "the five-character timestamp has intrinsic width: {:?}",
        time.rect
    );
    assert!(
        (time.rect[0] + time.rect[2] - 732.0).abs() <= 1.0,
        "time ends at the column edge inside its 12 px row padding: {:?}",
        time.rect
    );
}
