//! Inert agent and extension text. Callers supply theme, strings, and source byte ranges.
mod line;
mod links;
mod markdown;
mod outline;
mod render;
mod source;
mod style;

pub use line::{Run, invisible, line, runs};
pub use links::{link_at, links};
pub use markdown::{Markdown, Paint, Tint, markdown};
