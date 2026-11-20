//! This device's settings: the theme, the reading width, motion and moved keys, kept in
//! `settings.toml` at the client's root beside `client.toml`. Read once at launch; changed only
//! through [`change`], which applies at once and then rewrites the one key it changed.

mod file;

use crate::app::App;
use crate::keys::{Key, Keymap};
use crate::theme::{Measure, Theme, Variant, tokens};

pub const FILE: &str = "settings.toml";

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

/// The settings that are one value each, in the float's order. A fifth is a variant here, a field
/// on `Settings`, and the arms the compiler then asks for; reading, saving and the float follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Theme,
    Reading,
    Motion,
}

impl Field {
    pub const ALL: [Field; 3] = [Field::Theme, Field::Reading, Field::Motion];

    /// Its key in settings.toml.
    pub fn key(self) -> &'static str {
        match self {
            Field::Theme => "theme",
            Field::Reading => "reading_width",
            Field::Motion => "motion",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Field::Theme => "theme",
            Field::Reading => "reading width",
            Field::Motion => "motion",
        }
    }

    /// Its value in `s`, as the float prints it.
    pub fn shown(self, s: &Settings) -> String {
        match self {
            Field::Theme => s.theme.name().to_string(),
            Field::Reading => format!("{} px", s.reading.px()),
            Field::Motion => if s.motion { "on" } else { "off" }.to_string(),
        }
    }

    pub fn choices(self) -> String {
        match self {
            Field::Theme => "ember, ember-soft or ember-light".to_string(),
            Field::Reading => {
                let r = &tokens().layout.reading;
                format!("{} to {} px, by {}", r.min, r.max, r.step)
            }
            Field::Motion => "fades, slides and glides".to_string(),
        }
    }

    /// `s` with this value one step on (`by` 1) or back (-1). Theme and motion wrap; the width
    /// is held at its ends.
    pub fn step(self, s: &Settings, by: i32) -> Settings {
        let mut next = *s;
        match self {
            Field::Theme => {
                let at = Variant::ALL.iter().position(|v| *v == s.theme).unwrap_or(0) as i32;
                next.theme = Variant::ALL[(at + by).rem_euclid(Variant::ALL.len() as i32) as usize];
            }
            Field::Reading => next.reading = s.reading.step(by),
            Field::Motion => next.motion = !s.motion,
        }
        next
    }
}

/// The one key of settings.toml a change rewrites.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    Field(Field),
    /// `[keys."<command>"] "<design>"`: set to the key the keymap has for it now, or removed when
    /// that is the design's own.
    Key {
        command: &'static str,
        design: Vec<Key>,
    },
}

/// Launch: settings.toml at the link's root. The fixture link has none, so nothing is read. Each
/// problem goes to `:messages`, with one echo line for them all.
pub fn load(app: &mut App) {
    let (settings, problems) = match app.link.store().load_device(FILE) {
        Ok(None) => return,
        Ok(Some(text)) => file::read(&text),
        Err(e) => (
            Settings::default(),
            vec![format!("{FILE} can't be read ({})", e.kind())],
        ),
    };
    app.settings = settings;
    if !problems.is_empty() {
        let n = problems.len();
        app.messages.extend(problems);
        app.say(format!(
            "{FILE}: {n} {} · :messages",
            if n == 1 { "problem" } else { "problems" }
        ));
    }
}

/// The one write path. `new` applies at once; then `edit` goes into the file as it is on disk
/// now, so a hand edit made since launch survives. A file that can't be read or isn't TOML is
/// left alone, and the change lasts until quit.
pub fn change(app: &mut App, new: Settings, edit: Edit) {
    if new.keymap != app.settings.keymap {
        app.keys.reset();
    }
    app.settings = new;
    let store = app.link.store();
    if store.root().is_none() {
        return;
    }
    let saved = store
        .load_device(FILE)
        .map_err(|e| format!("{FILE} can't be read ({})", e.kind()))
        .and_then(|text| {
            let text = text.unwrap_or_default();
            let out = file::write(&text, &new, &edit)?;
            if file::read(&out).0 != new {
                return Err(format!("{FILE} would load different settings"));
            }
            if out == text {
                return Ok(());
            }
            store
                .save_device(FILE, &out)
                .map_err(|e| format!("{FILE} can't be written ({})", e.kind()))
        });
    if let Err(why) = saved {
        app.say(format!("{why} · not saved, so this lasts until you quit"));
    }
}

#[cfg(test)]
mod app_tests;
#[cfg(test)]
mod tests;
