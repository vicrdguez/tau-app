//! The primitives every screen is built from. Each takes the theme and plain data and returns an
//! element; none reads app state.

mod badge;
mod brand;
mod keys;
mod marks;
mod rows;
mod shell;
mod surfaces;

pub use badge::{Badge, badge};
pub use brand::wordmark;
pub use keys::{Scope, chip, control, float_hint, float_hint_if, glyphs, hint, named, with_keys};
pub use marks::{Counts, Mark, Tag, count_tag, count_tags, mark, mark_in};
pub use rows::{Part, facts, group_head, label, slot_row};
pub use shell::{FLOAT_BODY, FLOAT_CURSOR, Shell, float_row, kv_list, kv_row, shell, title};
pub use surfaces::{NoticeAction, container, field_error, notice, text_field};
