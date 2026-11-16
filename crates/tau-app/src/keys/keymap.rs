//! `design/keymap.toml` as a table the engine resolves against: contexts and their tree, the
//! modes, every command with its bindings, the builtin key sets and the which-key labels. A
//! `Keymap` is that table with the keys this device moved.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::Deserialize;

use super::key::{Code, Key, display, parse_seq};

/// Where the keys act (`grammar.md` §3). A float that takes the keys is a root of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Ctx {
    Global,
    List,
    Sidebar,
    Reader,
    Conversation,
    Decision,
    Project,
    Editor,
    Composer,
    Note,
    Field,
    Connect,
    Start,
    Hint,
    Picker,
    Bind,
    Readback,
    Reply,
    Confirm,
    Outbox,
    Server,
    Peek,
}

impl Ctx {
    pub const ALL: [Ctx; 22] = [
        Ctx::Global,
        Ctx::List,
        Ctx::Sidebar,
        Ctx::Reader,
        Ctx::Conversation,
        Ctx::Decision,
        Ctx::Project,
        Ctx::Editor,
        Ctx::Composer,
        Ctx::Note,
        Ctx::Field,
        Ctx::Connect,
        Ctx::Start,
        Ctx::Hint,
        Ctx::Picker,
        Ctx::Bind,
        Ctx::Readback,
        Ctx::Reply,
        Ctx::Confirm,
        Ctx::Outbox,
        Ctx::Server,
        Ctx::Peek,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Ctx::Global => "global",
            Ctx::List => "list",
            Ctx::Sidebar => "sidebar",
            Ctx::Reader => "reader",
            Ctx::Conversation => "conversation",
            Ctx::Decision => "decision",
            Ctx::Project => "project",
            Ctx::Editor => "editor",
            Ctx::Composer => "composer",
            Ctx::Note => "note",
            Ctx::Field => "field",
            Ctx::Connect => "connect",
            Ctx::Start => "start",
            Ctx::Hint => "hint",
            Ctx::Picker => "picker",
            Ctx::Bind => "bind",
            Ctx::Readback => "readback",
            Ctx::Reply => "reply",
            Ctx::Confirm => "confirm",
            Ctx::Outbox => "outbox",
            Ctx::Server => "server",
            Ctx::Peek => "peek",
        }
    }

    pub fn from_name(name: &str) -> Option<Ctx> {
        Ctx::ALL.into_iter().find(|c| c.name() == name)
    }
}

/// The keymap's modes (`grammar.md` §2, §12).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Normal,
    Visual,
    Insert,
    /// Pickers and the `:` line: `c`.
    Command,
    Hint,
    /// The readback, send-again, reply and Confirm floats: `rb`.
    Readback,
    /// The peek float: `ho`.
    Peek,
    /// The outbox float: `ob`.
    Outbox,
    /// The Server float: `sv`.
    Server,
}

impl Mode {
    pub fn from_letters(s: &str) -> Option<Mode> {
        Some(match s {
            "n" => Mode::Normal,
            "v" => Mode::Visual,
            "i" => Mode::Insert,
            "c" => Mode::Command,
            "h" => Mode::Hint,
            "rb" => Mode::Readback,
            "ho" => Mode::Peek,
            "ob" => Mode::Outbox,
            "sv" => Mode::Server,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Local,
    Write,
    Decide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Command,
    Motion,
    Operator,
    TextObject,
    Modal,
}

/// One `[[command]]`.
#[derive(Debug, PartialEq)]
pub struct Command {
    pub id: String,
    /// The `:` name; `-` (or a leading `-`) for keys that have none.
    pub ex: String,
    pub desc: String,
    pub tier: Tier,
    pub kind: Kind,
    pub ctx: Vec<Ctx>,
    pub countable: bool,
    pub jump: bool,
}

impl Command {
    /// Whether `:` and the palette offer it.
    pub fn has_ex(&self) -> bool {
        !self.ex.starts_with('-')
    }

    /// Whether an OS auto-repeat may fire it: never a write or an answer, nor a readback's or
    /// Confirm's `y` (`grammar.md` §7, §11).
    pub fn repeats(&self) -> bool {
        !matches!(self.tier, Tier::Write | Tier::Decide)
            && !NEVER_REPEATED.contains(&self.id.as_str())
    }
}

/// What a binding runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// An index into `Keymap::commands`.
    Command(usize),
    /// A key of a builtin set (motions, editing keys): the region's own handler takes it.
    Builtin,
}

/// One binding as the design writes it.
#[derive(Clone, Debug)]
pub(super) struct Binding {
    pub mode: Mode,
    pub ctx: Ctx,
    pub seq: Vec<Key>,
    pub shadows: bool,
    pub target: Target,
}

/// One binding as this device types it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bound {
    pub mode: Mode,
    pub ctx: Ctx,
    pub seq: &'static [Key],
    pub target: Target,
}

/// A printed key: what this device types for a key the design writes, and whether it runs where
/// the keys are.
#[derive(Clone, Debug, PartialEq)]
pub struct Chip {
    pub printed: String,
    pub lit: bool,
}

#[derive(Debug, PartialEq)]
pub struct KeymapError(pub String);

impl std::fmt::Display for KeymapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "design/keymap.toml: {}", self.0)
    }
}

impl std::error::Error for KeymapError {}

/// One entry of a which-key float.
#[derive(Clone, Debug, PartialEq)]
pub struct WhichEntry<'k> {
    pub key: Key,
    pub desc: String,
    /// A group (a prefix with its own label) rather than a single command.
    pub group: bool,
    /// Whether the command applies where the keys are; a group always does.
    pub applies: bool,
    pub command: Option<&'k Command>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WhichKey<'k> {
    /// The prefix's label (`leader`, `◇ answer`).
    pub label: String,
    pub prefix: Vec<Key>,
    /// Groups first, then single commands.
    pub entries: Vec<WhichEntry<'k>>,
}

/// What a typed sequence meets in one mode and context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lookup {
    pub exact: Option<Target>,
    pub longer: bool,
    /// The binding `exact` came from.
    pub(super) via: Option<usize>,
}

/// keymap.toml as written. The app's is parsed once; tests may leak their own.
struct Design {
    parents: Vec<(Ctx, Option<Ctx>)>,
    groups: Vec<(Vec<Key>, String)>,
    commands: Vec<Command>,
    bindings: Vec<Binding>,
    hint_alphabet: String,
    whichkey_delay_ms: f64,
}

/// The design's keymap with the keys this device moved. `Copy`, so every view finds it in the
/// theme (`t.keymap`). Commands never change, so a `&Command` from it is `'static`; only the keys
/// that reach them do.
#[derive(Clone, Copy)]
pub struct Keymap {
    design: &'static Design,
    /// The bindings this device types differently, in binding order. Each change leaks one slice.
    moved: &'static [Moved],
}

#[derive(Clone, Debug, PartialEq)]
struct Moved {
    binding: usize,
    seq: Vec<Key>,
}

impl PartialEq for Keymap {
    fn eq(&self, other: &Keymap) -> bool {
        std::ptr::eq(self.design, other.design) && self.moved == other.moved
    }
}

impl std::fmt::Debug for Keymap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.moved.iter().map(|m| {
                (
                    display(&self.design.bindings[m.binding].seq),
                    display(&m.seq),
                )
            }))
            .finish()
    }
}

static DESIGN: LazyLock<Design> = LazyLock::new(|| {
    parse(include_str!("../../../../design/keymap.toml")).expect("design/keymap.toml parses")
});

impl Keymap {
    /// The design's own keys.
    pub fn design() -> Keymap {
        Keymap {
            design: &DESIGN,
            moved: &[],
        }
    }

    #[cfg(test)]
    pub fn parse(toml: &str) -> Result<Keymap, KeymapError> {
        Ok(Keymap {
            design: Box::leak(Box::new(parse(toml)?)),
            moved: &[],
        })
    }

    pub fn commands(&self) -> &'static [Command] {
        &self.design.commands
    }

    /// Every binding, at the key this device types for it.
    pub fn bindings(self) -> impl Iterator<Item = Bound> {
        (0..self.design.bindings.len()).map(move |i| {
            let b = &self.design.bindings[i];
            Bound {
                mode: b.mode,
                ctx: b.ctx,
                seq: self.seq(i),
                target: b.target,
            }
        })
    }

    /// The keys hint labels are made of, home row first (`[meta] hint_alphabet`).
    pub fn hint_alphabet(&self) -> &'static str {
        &self.design.hint_alphabet
    }

    pub fn whichkey_delay_ms(&self) -> f64 {
        self.design.whichkey_delay_ms
    }

    pub fn command(&self, id: &str) -> Option<&'static Command> {
        self.commands().iter().find(|c| c.id == id)
    }

    /// The command a `:` name runs.
    pub fn by_ex(&self, ex: &str) -> Option<&'static Command> {
        self.commands().iter().find(|c| c.has_ex() && c.ex == ex)
    }

    /// Commands whose `:` name starts with `prefix`, in the table's order.
    pub fn ex_completions(&self, prefix: &str) -> Vec<&'static Command> {
        self.commands()
            .iter()
            .filter(|c| c.has_ex() && c.ex.starts_with(prefix))
            .collect()
    }

    /// The palette's commands: every one with a `:` name, in the table's order.
    pub fn palette(&self) -> impl Iterator<Item = &'static Command> {
        self.commands().iter().filter(|c| c.has_ex())
    }

    /// The printed forms of a command's keys in `mode`, deduplicated (`␣;`, `⌘K`).
    pub fn keys_of(&self, command: &Command, mode: Mode) -> Vec<String> {
        let Some(c) = self
            .commands()
            .iter()
            .position(|x| std::ptr::eq(x, command))
        else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        for b in self
            .bindings()
            .filter(|b| b.mode == mode && b.target == Target::Command(c))
        {
            let shown = display(b.seq);
            if !out.contains(&shown) {
                out.push(shown);
            }
        }
        out
    }

    /// The parent of `ctx`; `None` for a root (`global` and every float that takes the keys).
    pub fn parent(&self, ctx: Ctx) -> Option<Ctx> {
        self.design
            .parents
            .iter()
            .find(|(c, _)| *c == ctx)
            .and_then(|(_, p)| *p)
    }

    /// `ctx` and its ancestors, innermost first.
    pub fn chain(&self, ctx: Ctx) -> Vec<Ctx> {
        let mut chain = vec![ctx];
        while let Some(p) = self.parent(*chain.last().unwrap_or(&ctx)) {
            chain.push(p);
        }
        chain
    }

    /// Whether `command` applies in `ctx`: one of its contexts is `ctx` or an ancestor of it.
    pub fn applies(&self, command: &Command, ctx: Ctx) -> bool {
        let chain = self.chain(ctx);
        command.ctx.iter().any(|c| chain.contains(c))
    }

    /// The bindings live in `mode` and `ctx`, by index: bound in `ctx` or an ancestor, less those
    /// an inner binding shadows.
    pub(super) fn active(&self, mode: Mode, ctx: Ctx) -> Vec<usize> {
        let chain = self.chain(ctx);
        let depth = |c: Ctx| chain.iter().position(|x| *x == c);
        let b = &self.design.bindings;
        let live: Vec<usize> = (0..b.len())
            .filter(|&i| b[i].mode == mode && depth(b[i].ctx).is_some())
            .collect();
        let shadowed = |i: usize| {
            live.iter().any(|&s| {
                b[s].shadows
                    && depth(b[s].ctx) < depth(b[i].ctx)
                    && overlap(self.seq(s), self.seq(i))
            })
        };
        live.iter().copied().filter(|&i| !shadowed(i)).collect()
    }

    pub fn lookup(&self, mode: Mode, ctx: Ctx, seq: &[Key]) -> Lookup {
        self.lookup_in(&self.active(mode, ctx), ctx, seq)
    }

    /// `seq` written as keymap.toml writes it, printed as this device types it where the keys
    /// are, and lit when that key runs a command that applies there (`spec.md` §4.6).
    pub fn chip(&self, seq: &str, mode: Mode, ctx: Ctx) -> Chip {
        let Ok(design) = parse_seq(seq) else {
            return Chip {
                printed: seq.to_string(),
                lit: false,
            };
        };
        let b = &self.design.bindings;
        let live = self.active(mode, ctx);
        let rank = self.rank(ctx);
        let here = live
            .iter()
            .copied()
            .filter(|&i| b[i].seq == design)
            .min_by_key(|&i| rank(i));
        let typed = match here {
            Some(i) => self.seq(i),
            None => {
                let mut all = (0..b.len())
                    .filter(|&i| b[i].seq == design)
                    .map(|i| self.seq(i));
                match all.next() {
                    Some(first) if all.all(|s| s == first) => first,
                    _ => &design,
                }
            }
        };
        let look = self.lookup_in(&live, ctx, typed);
        let lit = match look.exact {
            Some(Target::Command(i)) if !look.longer => self.applies(&self.commands()[i], ctx),
            _ => false,
        };
        Chip {
            printed: display(typed),
            lit,
        }
    }

    /// The which-key float for a pending prefix, or `None` when it isn't a proper prefix.
    pub fn which_key(&self, prefix: &[Key], mode: Mode, ctx: Ctx) -> Option<WhichKey<'static>> {
        let b = &self.design.bindings;
        let active = self.active(mode, ctx);
        let extends = |i: usize| {
            matches!(b[i].target, Target::Command(_))
                && self.seq(i).len() > prefix.len()
                && self.extends(self.seq(i), prefix)
        };
        if !active.iter().any(|&i| extends(i)) {
            return None;
        }
        let root = *self.chain(ctx).last()?;
        let in_tree = |i: usize| self.chain(b[i].ctx).last() == Some(&root);
        let mut groups: Vec<WhichEntry<'static>> = Vec::new();
        let mut leaves: Vec<WhichEntry<'static>> = Vec::new();
        for i in (0..b.len()).filter(|&i| b[i].mode == mode && in_tree(i) && extends(i)) {
            let seq = self.seq(i);
            let key = seq[prefix.len()];
            let Target::Command(c) = b[i].target else {
                continue;
            };
            if seq.len() > prefix.len() + 1 {
                if groups.iter().any(|g| g.key == key) {
                    continue;
                }
                let label = self
                    .group_label(&seq[..=prefix.len()])
                    .unwrap_or("…")
                    .to_string();
                groups.push(WhichEntry {
                    key,
                    desc: label,
                    group: true,
                    applies: true,
                    command: None,
                });
            } else {
                let command = &self.commands()[c];
                let applies = active.contains(&i) && self.applies(command, ctx);
                let entry = WhichEntry {
                    key,
                    desc: command.desc.clone(),
                    group: false,
                    applies,
                    command: Some(command),
                };
                match leaves.iter_mut().find(|l| l.key == key) {
                    Some(l) if !l.applies && applies => *l = entry,
                    Some(_) => {}
                    None => leaves.push(entry),
                }
            }
        }
        let order = ORDER
            .iter()
            .find(|(p, _)| parse_seq(p).is_ok_and(|p| p == prefix))
            .map_or("", |(_, o)| *o);
        let place = |e: &WhichEntry| match e.key.code {
            Code::Char(c) if e.key.mods == super::key::Mods::default() => {
                order.find(c).unwrap_or(usize::MAX)
            }
            _ => usize::MAX,
        };
        groups.sort_by_key(|e| place(e));
        leaves.sort_by_key(|e| place(e));
        groups.extend(leaves);
        Some(WhichKey {
            label: self.group_label(prefix).unwrap_or("").to_string(),
            prefix: prefix.to_vec(),
            entries: groups,
        })
    }
}

impl Keymap {
    /// Binding `i`'s keys on this device.
    pub(super) fn seq(&self, i: usize) -> &'static [Key] {
        let moved: &'static [Moved] = self.moved;
        match moved.binary_search_by_key(&i, |m| m.binding) {
            Ok(at) => &moved[at].seq,
            Err(_) => &self.design.bindings[i].seq,
        }
    }

    /// The keys a handler gets for binding `via`: the design's when this device moved it, so
    /// `[b` still reads as backwards; else the keys typed (a hint label stays the label).
    pub(super) fn handed(&self, via: usize, typed: Vec<Key>) -> Vec<Key> {
        if self.moved.iter().any(|m| m.binding == via) {
            self.design.bindings[via].seq.clone()
        } else {
            typed
        }
    }

    /// Every binding as the design writes it.
    pub(super) fn designed(&self) -> &'static [Binding] {
        &self.design.bindings
    }

    /// Command `command`'s bindings the design writes as `design`, by index.
    pub(super) fn bound_as(&self, command: usize, design: &[Key]) -> Vec<usize> {
        let b = &self.design.bindings;
        (0..b.len())
            .filter(|&i| b[i].target == Target::Command(command) && b[i].seq == design)
            .collect()
    }

    /// This map with `bindings` typed as `to`, which puts back the design's key where it is
    /// theirs. Unchecked: `rebind` validates first.
    pub(super) fn moving(self, bindings: &[usize], to: &[Key]) -> Keymap {
        let mut moved: Vec<Moved> = self
            .moved
            .iter()
            .filter(|m| !bindings.contains(&m.binding))
            .cloned()
            .collect();
        for &binding in bindings {
            if self.design.bindings[binding].seq != to {
                moved.push(Moved {
                    binding,
                    seq: to.to_vec(),
                });
            }
        }
        moved.sort_by_key(|m| m.binding);
        Keymap {
            design: self.design,
            moved: Box::leak(moved.into_boxed_slice()),
        }
    }

    fn lookup_in(&self, live: &[usize], ctx: Ctx, typed: &[Key]) -> Lookup {
        let rank = self.rank(ctx);
        let mut exact: Option<usize> = None;
        let mut longer = false;
        for &i in live {
            let seq = self.seq(i);
            if !self.extends(seq, typed) {
                continue;
            }
            if seq.len() > typed.len() {
                longer = true;
            } else if exact.is_none_or(|e| rank(i) < rank(e)) {
                exact = Some(i);
            }
        }
        Lookup {
            exact: exact.map(|i| self.design.bindings[i].target),
            longer,
            via: exact,
        }
    }

    /// Which of two bindings of one sequence runs where the keys are (`ctx`): a command over a
    /// builtin key, then the innermost.
    fn rank(&self, ctx: Ctx) -> impl Fn(usize) -> (bool, usize) + '_ {
        let chain = self.chain(ctx);
        move |i| {
            let b = &self.design.bindings[i];
            let depth = chain.iter().position(|x| *x == b.ctx);
            (b.target == Target::Builtin, depth.unwrap_or(usize::MAX))
        }
    }

    /// Whether `pattern` begins with the typed keys; `<label>` takes any key of the hint alphabet.
    fn extends(&self, pattern: &[Key], typed: &[Key]) -> bool {
        pattern.len() >= typed.len()
            && pattern
                .iter()
                .zip(typed)
                .all(|(p, t)| match (p.code, t.code) {
                    (Code::Label, Code::Char(c)) => {
                        t.mods == super::key::Mods::default()
                            && self.design.hint_alphabet.contains(c)
                    }
                    _ => p == t,
                })
    }

    pub(super) fn group_label(&self, prefix: &[Key]) -> Option<&'static str> {
        let design: &'static Design = self.design;
        design
            .groups
            .iter()
            .find(|(k, _)| k == prefix)
            .map(|(_, label)| label.as_str())
    }
}

/// One is the other or starts it.
pub(super) fn overlap(a: &[Key], b: &[Key]) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

/// which-key's order for each leader group, as `grammar.md` §5 lists them; the keymap's own order
/// (and anything not listed) follows.
const ORDER: [(&str, &str); 9] = [
    ("<leader>", "acopmrthn.;y"),
    ("<leader>a", "dr"),
    ("<leader>c", "nxcmtf"),
    ("<leader>o", "hctfps"),
    ("<leader>p", "baof"),
    ("<leader>m", "nwsr"),
    ("<leader>r", "utcp"),
    ("<leader>t", "fshbd"),
    ("<leader>h", "kt"),
];

/// The `y` of the answer readback and of Confirm: a held key never confirms (`grammar.md` §11).
const NEVER_REPEATED: [&str; 2] = ["readback.confirm", "confirm.keys"];

/// Where each `[builtin]` set is live. The keymap names the sets; `grammar.md` §2, §6 and §10 say
/// where they act. `timeout_ok` (`jk`) is the editor's own and resolves there, not here.
const BUILTIN_SCOPES: [(&str, Mode, Ctx); 9] = [
    ("list_motions", Mode::Normal, Ctx::List),
    ("reader_motions", Mode::Normal, Ctx::Reader),
    ("reader_motions", Mode::Visual, Ctx::Reader),
    ("editor_normal", Mode::Normal, Ctx::Editor),
    ("editor_normal", Mode::Visual, Ctx::Editor),
    ("editor_insert", Mode::Insert, Ctx::Editor),
    ("start_insert", Mode::Insert, Ctx::Start),
    ("visual", Mode::Visual, Ctx::Global),
    ("hint", Mode::Hint, Ctx::Hint),
];

#[derive(Deserialize)]
struct RawMeta {
    leader: String,
    whichkey_delay_ms: f64,
    hint_alphabet: String,
}

#[derive(Deserialize)]
struct RawKey {
    mode: String,
    ctx: String,
    seq: String,
    #[serde(default)]
    shadows: Option<String>,
}

#[derive(Deserialize)]
struct RawCommand {
    id: String,
    ex: String,
    desc: String,
    tier: String,
    kind: String,
    ctx: Vec<String>,
    #[serde(default)]
    countable: bool,
    #[serde(default)]
    jump: bool,
    #[serde(default)]
    keys: Vec<RawKey>,
}

#[derive(Deserialize)]
struct RawKeymap {
    meta: RawMeta,
    contexts: BTreeMap<String, String>,
    groups: BTreeMap<String, String>,
    builtin: BTreeMap<String, Vec<String>>,
    command: Vec<RawCommand>,
}

fn err(msg: String) -> KeymapError {
    KeymapError(msg)
}

fn ctx_named(name: &str) -> Result<Ctx, KeymapError> {
    Ctx::from_name(name).ok_or_else(|| err(format!("unknown context `{name}`")))
}

fn parse(toml: &str) -> Result<Design, KeymapError> {
    let raw: RawKeymap = toml::from_str(toml).map_err(|e| err(e.to_string()))?;
    let leader = raw.meta.leader.as_str();
    let seq = |s: &str| parse_seq(&s.replace("<leader>", leader)).map_err(|e| err(e.0));

    let mut parents = Vec::new();
    for (name, parent) in &raw.contexts {
        let parent = if parent.is_empty() {
            None
        } else {
            Some(ctx_named(parent)?)
        };
        parents.push((ctx_named(name)?, parent));
    }
    if let Some(missing) = Ctx::ALL
        .into_iter()
        .find(|c| !parents.iter().any(|(p, _)| p == c))
    {
        return Err(err(format!(
            "context `{}` is missing from [contexts]",
            missing.name()
        )));
    }
    for (start, _) in &parents {
        let mut at = Some(*start);
        for _ in 0..=Ctx::ALL.len() {
            at = at.and_then(|c| {
                parents
                    .iter()
                    .find(|(p, _)| *p == c)
                    .and_then(|(_, parent)| *parent)
            });
        }
        if at.is_some() {
            return Err(err(format!("context `{}` is in a cycle", start.name())));
        }
    }

    let mut groups = Vec::new();
    for (prefix, label) in &raw.groups {
        groups.push((seq(prefix)?, label.clone()));
    }

    let mut commands = Vec::new();
    let mut bindings = Vec::new();
    for (i, c) in raw.command.into_iter().enumerate() {
        if commands.iter().any(|x: &Command| x.id == c.id) {
            return Err(err(format!("two commands have the id `{}`", c.id)));
        }
        let tier = match c.tier.as_str() {
            "local" => Tier::Local,
            "write" => Tier::Write,
            "decide" => Tier::Decide,
            t => return Err(err(format!("`{}`: unknown tier `{t}`", c.id))),
        };
        let kind = match c.kind.as_str() {
            "command" => Kind::Command,
            "motion" => Kind::Motion,
            "operator" => Kind::Operator,
            "textobject" => Kind::TextObject,
            "modal" => Kind::Modal,
            k => return Err(err(format!("`{}`: unknown kind `{k}`", c.id))),
        };
        for k in &c.keys {
            let mode = Mode::from_letters(&k.mode)
                .ok_or_else(|| err(format!("`{}`: unknown mode `{}`", c.id, k.mode)))?;
            bindings.push(Binding {
                mode,
                ctx: ctx_named(&k.ctx)?,
                seq: seq(&k.seq)?,
                shadows: k.shadows.as_deref() == Some("parent"),
                target: Target::Command(i),
            });
        }
        commands.push(Command {
            ctx: c
                .ctx
                .iter()
                .map(|n| ctx_named(n))
                .collect::<Result<_, _>>()?,
            id: c.id,
            ex: c.ex,
            desc: c.desc,
            tier,
            kind,
            countable: c.countable,
            jump: c.jump,
        });
    }

    let timeout_ok = raw.builtin.get("timeout_ok").cloned().unwrap_or_default();
    for (set, mode, ctx) in BUILTIN_SCOPES {
        let Some(keys) = raw.builtin.get(set) else {
            continue;
        };
        for k in keys.iter().filter(|k| !timeout_ok.contains(k)) {
            bindings.push(Binding {
                mode,
                ctx,
                seq: seq(k)?,
                shadows: false,
                target: Target::Builtin,
            });
        }
    }

    Ok(Design {
        parents,
        groups,
        commands,
        bindings,
        hint_alphabet: raw.meta.hint_alphabet,
        whichkey_delay_ms: raw.meta.whichkey_delay_ms,
    })
}
