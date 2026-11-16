//! The jumplist (`grammar.md` §13): one per device, the last 100 places a jump left, oldest first,
//! and how far `C-o` has walked back from its newest end.

use serde::{Deserialize, Serialize};

use super::Spot;

/// `[meta] jumplist_size` in `keymap.toml`.
const SIZE: usize = 100;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Jumps {
    list: Vec<Spot>,
    /// How many entries `C-o` stands behind the newest end; 0 is past it, on no entry.
    #[serde(skip)]
    back: usize,
    /// The server whose list this is, once read from this device.
    #[serde(skip)]
    pub(super) of: Option<Option<String>>,
}

impl Jumps {
    /// Oldest first.
    pub fn entries(&self) -> &[Spot] {
        &self.list
    }

    /// The entry `C-o` and `C-i` stand on, while they walk.
    pub fn current(&self) -> Option<usize> {
        self.list
            .len()
            .checked_sub(self.back)
            .filter(|_| self.back > 0)
    }

    /// A jump left `from`: it becomes the newest entry, once, and the walk starts again.
    pub fn push(&mut self, from: Spot) {
        add(&mut self.list, from);
        self.back = 0;
    }

    /// `n` entries back. From the newest end it keeps `here` first, so `C-i` returns to it. Past
    /// the oldest it moves nowhere.
    pub fn back(&mut self, here: Spot, n: usize) -> Option<Spot> {
        if self.back == 0 {
            let mut list = self.list.clone();
            add(&mut list, here);
            if list.len() <= n {
                return None;
            }
            self.list = list;
            self.back = 1;
        }
        if self.back + n > self.list.len() {
            return None;
        }
        self.back += n;
        self.list.get(self.list.len() - self.back).cloned()
    }

    /// `n` entries forward, toward the place `C-o` first left; past it nowhere.
    pub fn forward(&mut self, n: usize) -> Option<Spot> {
        if n >= self.back {
            return None;
        }
        self.back -= n;
        self.list.get(self.list.len() - self.back).cloned()
    }

    /// Stands on entry `i`, as `C-o` or `C-i` would have (`:jumps` then `⏎`); from the newest end
    /// it keeps `here` first, as `C-o` does.
    pub fn to(&mut self, here: Spot, i: usize) -> Option<Spot> {
        let spot = self.list.get(i).cloned()?;
        if self.back == 0 {
            add(&mut self.list, here);
        }
        let at = self.list.iter().position(|s| *s == spot)?;
        self.back = self.list.len() - at;
        Some(spot)
    }
}

fn add(list: &mut Vec<Spot>, spot: Spot) {
    list.retain(|s| *s != spot);
    list.push(spot);
    if list.len() > SIZE {
        list.drain(..list.len() - SIZE);
    }
}

#[cfg(test)]
#[path = "jumps_tests.rs"]
mod tests;
