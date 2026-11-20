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
pub use keys::{Scope, chip, float_hint, glyphs, hint};
pub use marks::{Counts, Mark, Tag, count_tag, count_tags, mark, mark_in};
pub use rows::{Part, facts, group_head, label, slot_row};
pub use shell::{Shell, float_row, kv_row, shell, title};
pub use surfaces::{NoticeAction, container, notice, text_field};
