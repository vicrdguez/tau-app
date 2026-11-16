//! Moving a command's key on this device, and the rules a move must keep: the runtime half of
//! `grammar.md` §12 (KL2, KL5, KL8, KL14) over the whole effective map. A pair of keys the design
//! itself binds is the design lint's; a pair where either key moved is checked here.

use super::key::{Code, Key, display, parse_seq, written};
use super::keymap::{Command, Ctx, Keymap, Mode, Target, Tier, overlap};

/// One line of settings.toml: `command`'s key `design`, as keymap.toml writes it, typed as `to`.
#[derive(Clone, Debug, PartialEq)]
pub struct Rebind<'a> {
    pub command: &'a str,
    pub design: Vec<Key>,
    pub to: Vec<Key>,
}

/// One key of one command, as the settings float lists it. Every binding the design writes with
/// `design` (any mode, any context) moves with it.
#[derive(Clone, Debug)]
pub struct KeyRow {
    pub command: &'static Command,
    pub design: Vec<Key>,
    pub now: Vec<Key>,
    pub modes: Vec<Mode>,
    pub ctxs: Vec<Ctx>,
    pub fixed: Option<Fixed>,
}

/// Keys that never move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fixed {
    /// `Esc`, everywhere: the way out of every float and mode.
    Leave,
    /// `<label>`: hint labels are the hint alphabet, not a key.
    Labels,
    /// Decide-tier commands and every key of the readback, reply and Confirm floats.
    Answer,
    /// The settings float's own keys: the float that moves keys never loses its own.
    Settings,
}

/// Why a move didn't happen. `Display` is the sentence the float, the echo and `:messages` show.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    NoCommand(String),
    NotItsKey {
        command: &'static Command,
        design: String,
        its: Vec<String>,
    },
    Fixed(Fixed),
    /// Where keys type (Insert, a picker), only one chord with Control, Option or Command.
    Typing {
        mode: Mode,
        ctx: Ctx,
        key: String,
    },
    /// A Normal or Visual key starting with 1–9 would never run: those digits start a count.
    Count,
    /// A reserved or trap key on a command that changes the server.
    Reserved {
        key: String,
    },
    /// A leader key under a prefix which-key has no label for.
    NoGroup {
        prefix: String,
    },
    Clash {
        key: String,
        other: Other,
        overlap: Overlap,
        mode: Mode,
        ctx: Ctx,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Other {
    Command(&'static Command),
    /// A key of a builtin set: the motions and the editor's keys.
    Builtin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlap {
    Same,
    /// The new key is the start of another key.
    Starts,
    /// The new key continues another key, which would run first.
    Extends,
}

/// Reserved for local commands in every mode (KL5).
const RESERVED: [&str; 6] = ["<C-c>", "<C-[>", "q", "u", "<C-r>", "."];
/// Reserved for local commands in Normal and Visual (KL5's trap keys).
const TRAPS: [&str; 12] = ["dd", "D", "x", "X", "s", "S", "r", "R", "p", "P", "@", "ZQ"];

/// A move resolved to the bindings it moves.
struct Change {
    bindings: Vec<usize>,
    to: Vec<Key>,
}

impl Keymap {
    /// `command`'s key `design` becomes `to` in every binding that writes it; `to == design` puts
    /// the design's key back. The whole map is checked, and the move is refused as a whole on
    /// any problem. The result depends only on the arguments and the other bindings' keys.
    pub fn rebind(self, command: &str, design: &[Key], to: &[Key]) -> Result<Keymap, Refusal> {
        let (km, mut refused) = self.rebind_all(&[Rebind {
            command,
            design: design.to_vec(),
            to: to.to_vec(),
        }]);
        match refused.pop() {
            Some((_, refusal)) => Err(refusal),
            None => Ok(km),
        }
    }

    /// settings.toml's lines, all at once, so a swap or a cycle loads. Each line in a problem is
    /// dropped and reported, and the rest are checked again, until none is left in one.
    pub fn rebind_all<'a>(self, lines: &[Rebind<'a>]) -> (Keymap, Vec<(Rebind<'a>, Refusal)>) {
        let mut refused = Vec::new();
        let mut kept = Vec::new();
        for line in lines {
            match self.change(line.command, &line.design, &line.to) {
                Ok(change) => kept.push((line.clone(), change)),
                Err(refusal) => refused.push((line.clone(), refusal)),
            }
        }
        loop {
            let candidate = kept
                .iter()
                .fold(self, |km, (_, c)| km.moving(&c.bindings, &c.to));
            let problems: Vec<Option<Refusal>> = kept
                .iter()
                .map(|(_, c)| c.bindings.iter().find_map(|&b| candidate.clash(b)))
                .collect();
            if problems.iter().all(Option::is_none) {
                return (candidate, refused);
            }
            let mut still = Vec::new();
            for (entry, problem) in kept.into_iter().zip(problems) {
                match problem {
                    Some(refusal) => refused.push((entry.0, refusal)),
                    None => still.push(entry),
                }
            }
            kept = still;
        }
    }

    /// This device's moves, for settings.toml.
    pub fn rebinds(&self) -> Vec<Rebind<'static>> {
        self.key_rows()
            .into_iter()
            .filter(|r| r.now != r.design)
            .map(|r| Rebind {
                command: r.command.id.as_str(),
                design: r.design,
                to: r.now,
            })
            .collect()
    }

    /// One row per key of each command, in the table's order, with why it can't move.
    pub fn key_rows(&self) -> Vec<KeyRow> {
        let mut rows: Vec<KeyRow> = Vec::new();
        for (i, b) in self.designed().iter().enumerate() {
            let Target::Command(c) = b.target else {
                continue;
            };
            let command = &self.commands()[c];
            let at = match rows
                .iter()
                .position(|r| std::ptr::eq(r.command, command) && r.design == b.seq)
            {
                Some(at) => at,
                None => {
                    rows.push(KeyRow {
                        command,
                        design: b.seq.clone(),
                        now: self.seq(i).to_vec(),
                        modes: Vec::new(),
                        ctxs: Vec::new(),
                        fixed: None,
                    });
                    rows.len() - 1
                }
            };
            let row = &mut rows[at];
            if !row.modes.contains(&b.mode) {
                row.modes.push(b.mode);
            }
            if !row.ctxs.contains(&b.ctx) {
                row.ctxs.push(b.ctx);
            }
            row.fixed = row.fixed.or(self.fixed(i));
        }
        rows
    }
}

impl Keymap {
    /// The bindings `command`'s key `design` names, checked against every rule that needs only
    /// the binding and `to`. Putting back the design's key skips those: the design lint has them.
    fn change(&self, command: &str, design: &[Key], to: &[Key]) -> Result<Change, Refusal> {
        let c = self
            .commands()
            .iter()
            .position(|c| c.id == command)
            .ok_or_else(|| Refusal::NoCommand(command.to_string()))?;
        let bindings = self.bound_as(c, design);
        if bindings.is_empty() {
            let mut its: Vec<String> = Vec::new();
            for b in self
                .designed()
                .iter()
                .filter(|b| b.target == Target::Command(c))
            {
                let shown = display(&b.seq);
                if !its.contains(&shown) {
                    its.push(shown);
                }
            }
            return Err(Refusal::NotItsKey {
                command: &self.commands()[c],
                design: written(design),
                its,
            });
        }
        if let Some(fixed) = bindings.iter().find_map(|&b| self.fixed(b)) {
            return Err(Refusal::Fixed(fixed));
        }
        if to.iter().any(|k| k.code == Code::Esc) {
            return Err(Refusal::Fixed(Fixed::Leave));
        }
        if to.iter().any(|k| k.code == Code::Label) {
            return Err(Refusal::Fixed(Fixed::Labels));
        }
        if to != design {
            let local = self.commands()[c].tier == Tier::Local;
            for &b in &bindings {
                let (mode, ctx) = (self.designed()[b].mode, self.designed()[b].ctx);
                self.allowed(mode, ctx, local, to)?;
            }
        }
        Ok(Change {
            bindings,
            to: to.to_vec(),
        })
    }

    /// The rules for `to` on one binding in `mode` and `ctx`.
    fn allowed(&self, mode: Mode, ctx: Ctx, local: bool, to: &[Key]) -> Result<(), Refusal> {
        let is = |seq: &str| parse_seq(seq).is_ok_and(|k| k == to);
        let chord = matches!(to, [k] if k.mods.ctrl || k.mods.alt || k.mods.cmd);
        let normal = matches!(mode, Mode::Normal | Mode::Visual);
        if matches!(mode, Mode::Insert | Mode::Command) && !chord {
            return Err(Refusal::Typing {
                mode,
                ctx,
                key: display(to),
            });
        }
        if normal && to.first().and_then(|k| k.digit()).is_some_and(|d| d != 0) {
            return Err(Refusal::Count);
        }
        if !local && (RESERVED.iter().any(|s| is(s)) || normal && TRAPS.iter().any(|s| is(s))) {
            return Err(Refusal::Reserved { key: display(to) });
        }
        if to.first() == Some(&Key::char(' ')) {
            for n in 1..to.len() {
                if self.group_label(&to[..n]).is_none() {
                    return Err(Refusal::NoGroup {
                        prefix: display(&to[..n]),
                    });
                }
            }
        }
        Ok(())
    }

    /// Why binding `b` can't move, if it can't.
    fn fixed(&self, b: usize) -> Option<Fixed> {
        let binding = &self.designed()[b];
        let decide =
            matches!(binding.target, Target::Command(c) if self.commands()[c].tier == Tier::Decide);
        if binding.seq.iter().any(|k| k.code == Code::Esc) {
            Some(Fixed::Leave)
        } else if binding.seq.iter().any(|k| k.code == Code::Label) {
            Some(Fixed::Labels)
        } else if decide || matches!(binding.ctx, Ctx::Readback | Ctx::Reply | Ctx::Confirm) {
            Some(Fixed::Answer)
        } else if binding.ctx.name() == "settings" {
            Some(Fixed::Settings)
        } else {
            None
        }
    }

    /// The first binding binding `i` clashes with on this map, where either of the two moved:
    /// in one mode, in contexts one of which is on the other's chain, one key equal to or the
    /// start of the other. An inner binding that shadows may overlap an outer one only where
    /// their design keys already did, so a move never makes a new shadow.
    fn clash(&self, i: usize) -> Option<Refusal> {
        let all = self.designed();
        let a = &all[i];
        let mine = self.seq(i);
        let chain = self.chain(a.ctx);
        all.iter().enumerate().find_map(|(j, b)| {
            let outer = chain.contains(&b.ctx);
            let inner = self.chain(b.ctx).contains(&a.ctx);
            let theirs = self.seq(j);
            let moved = mine != a.seq.as_slice() || theirs != b.seq.as_slice();
            let together = a.target == b.target && a.seq == b.seq;
            let shadow = a.ctx != b.ctx
                && (if outer { a.shadows } else { b.shadows })
                && overlap(&a.seq, &b.seq);
            if j == i
                || b.mode != a.mode
                || !(outer || inner)
                || !moved
                || together
                || shadow
                || !overlap(mine, theirs)
            {
                return None;
            }
            Some(Refusal::Clash {
                key: display(mine),
                other: match b.target {
                    Target::Command(c) => Other::Command(&self.commands()[c]),
                    Target::Builtin => Other::Builtin,
                },
                overlap: if mine == theirs {
                    Overlap::Same
                } else if theirs.starts_with(mine) {
                    Overlap::Starts
                } else {
                    Overlap::Extends
                },
                mode: a.mode,
                ctx: a.ctx,
            })
        })
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::NoCommand(id) => write!(f, "{id} is not a command"),
            Refusal::NotItsKey { command, its, .. } if its.is_empty() => {
                write!(f, "{} has no key to move", command.id)
            }
            Refusal::NotItsKey {
                command,
                design,
                its,
            } => write!(
                f,
                "{design} is not one of {}'s keys ({})",
                command.id,
                its.join(" ")
            ),
            Refusal::Fixed(Fixed::Leave) => {
                write!(
                    f,
                    "Esc stays Esc: it is the way out of every float and mode"
                )
            }
            Refusal::Fixed(Fixed::Labels) => write!(f, "hint labels are letters, not a key"),
            Refusal::Fixed(Fixed::Answer) => write!(f, "keys on the answer path don't move"),
            Refusal::Fixed(Fixed::Settings) => write!(f, "the settings float keeps its own keys"),
            Refusal::Typing { mode, ctx, .. } => write!(
                f,
                "{} a key needs Control, Option or Command",
                place(*mode, *ctx)
            ),
            Refusal::Count => write!(
                f,
                "a key in Normal can't start with 1 to 9: they type a count"
            ),
            Refusal::Reserved { key } => {
                write!(
                    f,
                    "{key} is kept for keys that change nothing on the server"
                )
            }
            Refusal::NoGroup { prefix } => {
                write!(
                    f,
                    "{prefix} isn't a group, so which-key can't show what follows it"
                )
            }
            Refusal::Clash {
                key,
                other,
                overlap,
                mode,
                ctx,
            } => {
                let theirs = match other {
                    Other::Command(c) => format!("{}'s key", c.desc),
                    Other::Builtin => "a built-in key".to_string(),
                };
                let at = place(*mode, *ctx);
                match overlap {
                    Overlap::Same => write!(f, "{key} is already {theirs} {at}"),
                    Overlap::Starts => write!(f, "{key} is the start of {theirs} {at}"),
                    Overlap::Extends => {
                        write!(f, "{key} begins with {theirs} {at}, which would run first")
                    }
                }
            }
        }
    }
}

/// Where a key acts, in words: `in Insert on the start page`, `in a picker`.
fn place(mode: Mode, ctx: Ctx) -> String {
    let at = match ctx {
        Ctx::Global => "anywhere",
        Ctx::List => "in a list",
        Ctx::Sidebar => "in the sidebar",
        Ctx::Reader => "in a reader",
        Ctx::Conversation => "in a conversation",
        Ctx::Decision => "in a decision",
        Ctx::Project => "in the Project view",
        Ctx::Editor => "in an editor",
        Ctx::Composer => "in the composer",
        Ctx::Note => "in a note",
        Ctx::Field => "in an option field",
        Ctx::Connect => "on Connect",
        Ctx::Start => "on the start page",
        Ctx::Hint => "in hint mode",
        Ctx::Picker => "in a picker",
        Ctx::Bind => "in the bind float",
        Ctx::Readback => "in the readback",
        Ctx::Reply => "in the reply",
        Ctx::Confirm => "in Confirm",
        Ctx::Outbox => "in the outbox",
        Ctx::Server => "in the Server float",
        Ctx::Peek => "in the peek",
    };
    match mode {
        Mode::Insert => format!("in Insert {at}"),
        Mode::Visual => format!("in Visual {at}"),
        _ => at.to_string(),
    }
}
