//! This device's settings: the theme, the reading width, motion and moved keys.

use crate::keys::Keymap;
use crate::theme::{Measure, Theme, Variant};

/// This device's choices, in one place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub theme: Variant,
    pub reading: Measure,
    pub motion: bool,
    pub keymap: Keymap,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Variant::Ember,
            reading: Measure::default(),
            motion: true,
            keymap: Keymap::design(),
        }
    }
}

impl Settings {
    /// `reduced_motion` is the system's Reduce Motion: it stills motion as `motion = false` does.
    pub fn theme(&self, reduced_motion: bool) -> Theme {
        Theme {
            keymap: self.keymap,
            ..Theme::new(self.theme, self.reading, self.motion && !reduced_motion)
        }
    }
}
