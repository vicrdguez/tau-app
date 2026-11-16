//! The keymap engine, driven by `design/keymap.toml`: it turns key presses into command ids,
//! "pending" or "unbound" for the mode and context that have the keys. What a command does is a
//! separate table (`crate::app::commands`), filled by the frame and the screens.

mod engine;
mod key;
mod keymap;

pub use engine::{Keys, Outcome};
pub use key::{Code, Key, Mods, SeqError, display, from_rune, parse_seq};
pub use keymap::{
    Binding, Command, Ctx, Keymap, KeymapError, Kind, Lookup, Mode, Target, Tier, WhichEntry,
    WhichKey, keymap,
};

#[cfg(test)]
mod tests;
