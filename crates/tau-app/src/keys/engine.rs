//! The key state of one window: a pending sequence and a count, resolved prefix-free against the
//! keymap for the mode and context that have the keys. It decides; it never runs a command.

use super::key::{Code, Key, display};
use super::keymap::{Command, Ctx, Keymap, Mode, Target};

/// What one key press came to.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome<'k> {
    /// A proper prefix: wait for more (which-key shows after its delay).
    Pending,
    /// A command's binding completed. `count` is kept only for countable commands. `applies` is
    /// false when the key is bound here but the command has nothing to act on in this context
    /// (`␣td` on Home): dispatch says so instead of acting.
    Command {
        command: &'k Command,
        count: Option<u32>,
        applies: bool,
        /// The keys that ran it (`]b` or `[b`), without the count; the design's keys when this
        /// device moved the binding.
        keys: Vec<Key>,
    },
    /// A key of a builtin set completed (`gg`, `j`, `<BS>`): the region's own handler takes it.
    Builtin { keys: Vec<Key>, count: Option<u32> },
    /// Nothing is bound: the keys go to the region's own handler (an editor types them).
    Unbound { keys: Vec<Key>, count: Option<u32> },
    /// `Esc` dropped a pending prefix or count.
    Cancelled,
    /// An auto-repeat that may not fire what it reached.
    Ignored,
}

#[derive(Clone, Debug, Default)]
pub struct Keys {
    pending: Vec<Key>,
    count: Option<u32>,
    /// When the last key arrived, in seconds.
    last_at: f64,
    /// The longest wait, in seconds, with a prefix of this sequence pending: once it reaches the
    /// delay, which-key has shown, and it stays for the sequence.
    held: f64,
}

impl Keys {
    /// Resolves one key in `mode` and `ctx`. `repeat` is an OS auto-repeat; `now` is in seconds.
    pub fn press(
        &mut self,
        km: Keymap,
        mode: Mode,
        ctx: Ctx,
        key: Key,
        repeat: bool,
        now: f64,
    ) -> Outcome<'static> {
        if key == Key::plain(Code::Esc) && !self.is_idle() {
            self.reset();
            return Outcome::Cancelled;
        }
        self.held = if self.pending.is_empty() {
            0.0
        } else {
            self.held.max(now - self.last_at)
        };
        self.last_at = now;
        if matches!(mode, Mode::Normal | Mode::Visual)
            && self.pending.is_empty()
            && let Some(d) = key.digit()
        {
            let unbound = || {
                let look = km.lookup(mode, ctx, &[key]);
                look.exact.is_none() && !look.longer
            };
            if self.count.is_some() || (d != 0 && unbound()) {
                self.count = Some(self.count.unwrap_or(0).saturating_mul(10).saturating_add(d));
                return Outcome::Pending;
            }
        }
        self.pending.push(key);
        let look = km.lookup(mode, ctx, &self.pending);
        if look.longer {
            return Outcome::Pending;
        }
        let keys = std::mem::take(&mut self.pending);
        let count = self.count.take();
        match (look.exact, look.via) {
            (Some(Target::Command(i)), Some(via)) => {
                let command = &km.commands()[i];
                if repeat && !command.repeats() {
                    return Outcome::Ignored;
                }
                let applies = km.applies(command, ctx);
                Outcome::Command {
                    command,
                    count: count.filter(|_| command.countable),
                    applies,
                    keys: km.handed(via, keys),
                }
            }
            (Some(Target::Builtin), _) => Outcome::Builtin { keys, count },
            _ => Outcome::Unbound { keys, count },
        }
    }

    pub fn pending(&self) -> &[Key] {
        &self.pending
    }

    pub fn count(&self) -> Option<u32> {
        self.count
    }

    pub fn is_idle(&self) -> bool {
        self.pending.is_empty() && self.count.is_none()
    }

    /// The bottom bar's pending keys: the count, then the prefix (`2␣a`).
    pub fn shown(&self) -> String {
        let count = self.count.map(|c| c.to_string()).unwrap_or_default();
        count + &display(&self.pending)
    }

    /// Whether which-key shows: a prefix has waited `delay_ms` since its last key, now or earlier
    /// in the sequence, so it stays until the sequence ends.
    pub fn which_key_due(&self, now: f64, delay_ms: f64) -> bool {
        !self.pending.is_empty() && self.held.max(now - self.last_at) * 1000.0 >= delay_ms - 1e-6
    }

    pub fn reset(&mut self) {
        *self = Keys::default();
    }
}
