//! The keymap engine, driven by `design/keymap.toml`: it turns key presses into command ids,
//! "pending" or "unbound" for the mode and context that have the keys. What a command does is a
//! separate table (`crate::app::commands`), filled by the frame and the screens. A `Keymap` is the
//! design with the keys this device moved (`rebind`); whoever reads keys is handed one.

mod engine;
mod key;
mod keymap;
mod rebind;

pub use engine::{Keys, Outcome};
pub use key::{Code, Key, Mods, SeqError, display, from_rune, parse_seq, spoken, written};
pub use keymap::{
    Bound, Chip, Command, Ctx, Keymap, KeymapError, Kind, Lookup, Mode, Target, Tier, WhichEntry,
    WhichKey,
};
pub use rebind::{Fixed, KeyRow, Other, Overlap, Rebind, Refusal};

#[cfg(test)]
mod rebind_tests;
#[cfg(test)]
mod tests;
