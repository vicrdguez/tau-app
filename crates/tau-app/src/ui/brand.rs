//! The brand on the start page and Connect (`visual-language.md` §11).

use rune::{El, el, icon};

use crate::assets::brand;
use crate::theme::Theme;

/// The wordmark, `height` tall, in its own colours: on dark in Ember and Ember Soft, on light in
/// Ember Light.
pub fn wordmark(t: Theme, height: f32) -> El {
    let mark = if t.variant.is_light() {
        &*brand::WORDMARK_ON_LIGHT
    } else {
        &*brand::WORDMARK_ON_DARK
    };
    let width = height * brand::WORDMARK_ASPECT;
    el().size(width, height)
        .clip_rounded(0.0)
        .child(icon(mark, width))
}
