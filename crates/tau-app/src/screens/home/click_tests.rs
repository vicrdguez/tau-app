//! Frame click failures. A tag must have its own target, not inherit Home's row press.
use super::*;
use crate::app::{commands, link};
use crate::frame::Row;

#[test]
fn each_tag_routes_to_its_object_or_float() {
    let mut a = App::default();
    link::settle(&mut a);
    commands::open_row(&mut a, &Row::Count(ui::Tag::Unknown), &mut Tx::default());
    assert_eq!(a.scope().ctx, Ctx::Outbox);
    a.close_float();
    commands::open_row(&mut a, &Row::Count(ui::Tag::Failed), &mut Tx::default());
    assert_eq!(a.stage, Stage::Place(Place::Home));
    assert_eq!(selected(&a).unwrap().group, "Failed");
    commands::open_row(&mut a, &Row::Count(ui::Tag::Needs), &mut Tx::default());
    assert_eq!(
        a.stage,
        Stage::Item(Item::Decision {
            project: "render-core".into(),
            id: "w1:3:design#2".into()
        })
    );
    commands::open_row(&mut a, &Row::Server, &mut Tx::default());
    assert_eq!(a.scope().ctx, Ctx::Server);
}
