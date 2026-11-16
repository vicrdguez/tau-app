//! `design/keymap.toml` as a table the engine resolves against: contexts and their tree, the
//! modes, every command with its bindings, the builtin key sets and the which-key labels.

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

#[derive(Clone, Debug)]
pub struct Binding {
    pub mode: Mode,
    pub ctx: Ctx,
    pub seq: Vec<Key>,
    pub shadows: bool,
    pub target: Target,
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
}

#[derive(Debug)]
pub struct Keymap {
    parents: Vec<(Ctx, Option<Ctx>)>,
    groups: Vec<(Vec<Key>, String)>,
    commands: Vec<Command>,
    bindings: Vec<Binding>,
    hint_alphabet: String,
    whichkey_delay_ms: f64,
}

impl Keymap {
    pub fn parse(toml: &str) -> Result<Keymap, KeymapError> {
        parse(toml)
    }

    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// The keys hint labels are made of, home row first (`[meta] hint_alphabet`).
    pub fn hint_alphabet(&self) -> &str {
        &self.hint_alphabet
    }

    pub fn whichkey_delay_ms(&self) -> f64 {
        self.whichkey_delay_ms
    }

    pub fn command(&self, id: &str) -> Option<&Command> {
        self.commands.iter().find(|c| c.id == id)
    }

    /// The command a `:` name runs.
    pub fn by_ex(&self, ex: &str) -> Option<&Command> {
        self.commands.iter().find(|c| c.has_ex() && c.ex == ex)
    }

    /// Commands whose `:` name starts with `prefix`, in the table's order.
    pub fn ex_completions(&self, prefix: &str) -> Vec<&Command> {
        self.commands
            .iter()
            .filter(|c| c.has_ex() && c.ex.starts_with(prefix))
            .collect()
    }

    /// The palette's commands: every one with a `:` name, in the table's order.
    pub fn palette(&self) -> impl Iterator<Item = &Command> {
        self.commands.iter().filter(|c| c.has_ex())
    }

    /// The printed forms of a command's keys in `mode`, deduplicated (`␣;`, `⌘K`).
    pub fn keys_of(&self, command: &Command, mode: Mode) -> Vec<String> {
        let Some(i) = self.commands.iter().position(|c| std::ptr::eq(c, command)) else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        for b in self
            .bindings
            .iter()
            .filter(|b| b.mode == mode && b.target == Target::Command(i))
        {
            let shown = display(&b.seq);
            if !out.contains(&shown) {
                out.push(shown);
            }
        }
        out
    }

    /// The parent of `ctx`; `None` for a root (`global` and every float that takes the keys).
    pub fn parent(&self, ctx: Ctx) -> Option<Ctx> {
        self.parents
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

    /// The bindings live in `mode` and `ctx`: bound in `ctx` or an ancestor, less those an
    /// inner binding shadows.
    pub fn active(&self, mode: Mode, ctx: Ctx) -> Vec<&Binding> {
        let chain = self.chain(ctx);
        let depth = |c: Ctx| chain.iter().position(|x| *x == c);
        let live: Vec<&Binding> = self
            .bindings
            .iter()
            .filter(|b| b.mode == mode && depth(b.ctx).is_some())
            .collect();
        let shadowed = |b: &Binding| {
            live.iter().any(|s| {
                s.shadows
                    && depth(s.ctx) < depth(b.ctx)
                    && (s.seq.starts_with(&b.seq) || b.seq.starts_with(&s.seq))
            })
        };
        live.iter().copied().filter(|b| !shadowed(b)).collect()
    }

    pub fn lookup(&self, mode: Mode, ctx: Ctx, seq: &[Key]) -> Lookup {
        let chain = self.chain(ctx);
        let depth = |c: Ctx| chain.iter().position(|x| *x == c).unwrap_or(usize::MAX);
        let mut exact: Option<&Binding> = None;
        let mut longer = false;
        for b in self.active(mode, ctx) {
            if !self.extends(&b.seq, seq) {
                continue;
            }
            if b.seq.len() > seq.len() {
                longer = true;
            } else if exact.is_none_or(|e| rank(b, depth(b.ctx)) < rank(e, depth(e.ctx))) {
                exact = Some(b);
            }
        }
        Lookup {
            exact: exact.map(|b| b.target),
            longer,
        }
    }

    /// Whether a printed chip for `seq` (keymap syntax) is lit: it runs a command that applies
    /// where the keys are (`spec.md` §4.6).
    pub fn lit(&self, seq: &str, mode: Mode, ctx: Ctx) -> bool {
        let Ok(keys) = parse_seq(seq) else {
            return false;
        };
        let look = self.lookup(mode, ctx, &keys);
        match look.exact {
            Some(Target::Command(i)) if !look.longer => self.applies(&self.commands[i], ctx),
            _ => false,
        }
    }

    /// The which-key float for a pending prefix, or `None` when it isn't a proper prefix.
    pub fn which_key(&self, prefix: &[Key], mode: Mode, ctx: Ctx) -> Option<WhichKey<'_>> {
        let active = self.active(mode, ctx);
        let extends = |b: &Binding| {
            matches!(b.target, Target::Command(_))
                && b.seq.len() > prefix.len()
                && self.extends(&b.seq, prefix)
        };
        if !active.iter().any(|b| extends(b)) {
            return None;
        }
        let root = *self.chain(ctx).last()?;
        let in_tree = |b: &Binding| self.chain(b.ctx).last() == Some(&root);
        let mut groups: Vec<WhichEntry<'_>> = Vec::new();
        let mut leaves: Vec<WhichEntry<'_>> = Vec::new();
        for b in self
            .bindings
            .iter()
            .filter(|b| b.mode == mode && in_tree(b) && extends(b))
        {
            let key = b.seq[prefix.len()];
            let Target::Command(i) = b.target else {
                continue;
            };
            if b.seq.len() > prefix.len() + 1 {
                if groups.iter().any(|g| g.key == key) {
                    continue;
                }
                let label = self
                    .group_label(&b.seq[..=prefix.len()])
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
                let command = &self.commands[i];
                let applies =
                    active.iter().any(|a| std::ptr::eq(*a, b)) && self.applies(command, ctx);
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
    /// Whether `pattern` begins with the typed keys; `<label>` takes any key of the hint alphabet.
    fn extends(&self, pattern: &[Key], typed: &[Key]) -> bool {
        pattern.len() >= typed.len()
            && pattern
                .iter()
                .zip(typed)
                .all(|(p, t)| match (p.code, t.code) {
                    (Code::Label, Code::Char(c)) => {
                        t.mods == super::key::Mods::default() && self.hint_alphabet.contains(c)
                    }
                    _ => p == t,
                })
    }

    fn group_label(&self, prefix: &[Key]) -> Option<&str> {
        self.groups
            .iter()
            .find(|(k, _)| k == prefix)
            .map(|(_, label)| label.as_str())
    }
}

/// Which of two bindings of one sequence runs: a command over a builtin key, then the innermost.
fn rank(b: &Binding, depth: usize) -> (bool, usize) {
    (b.target == Target::Builtin, depth)
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

fn parse(toml: &str) -> Result<Keymap, KeymapError> {
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

    Ok(Keymap {
        parents,
        groups,
        commands,
        bindings,
        hint_alphabet: raw.meta.hint_alphabet,
        whichkey_delay_ms: raw.meta.whichkey_delay_ms,
    })
}

static KEYMAP: LazyLock<Keymap> = LazyLock::new(|| {
    Keymap::parse(include_str!("../../../../design/keymap.toml"))
        .expect("design/keymap.toml parses")
});

/// The design's keymap, parsed once.
pub fn keymap() -> &'static Keymap {
    &KEYMAP
}
