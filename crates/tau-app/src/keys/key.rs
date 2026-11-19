//! One key press as the keymap names it, read from `keymap.toml`'s syntax or from rune's events,
//! and printed the way the design prints keys (`␣cn`, `⌘K`, `C-h`, `⏎`).

/// What was pressed, without modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Code {
    /// A printable character, Space included, already shifted (`G`, `:`).
    Char(char),
    Enter,
    Esc,
    Tab,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    /// `<label>`: any key of the hint alphabet, in hint mode only.
    Label,
}

/// Modifiers as the keymap writes them: `C-` Control, `M-` Option, `D-` Command, `S-` Shift.
/// Shift is kept only for keys that type no character (`<S-Tab>`); a shifted character is its
/// own `Code::Char`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub cmd: bool,
    pub shift: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: Code,
    pub mods: Mods,
}

impl Key {
    pub const fn plain(code: Code) -> Key {
        Key {
            code,
            mods: Mods {
                ctrl: false,
                alt: false,
                cmd: false,
                shift: false,
            },
        }
    }

    pub const fn char(c: char) -> Key {
        Key::plain(Code::Char(c))
    }

    pub const fn ctrl(c: char) -> Key {
        Key {
            code: Code::Char(c),
            mods: Mods {
                ctrl: true,
                alt: false,
                cmd: false,
                shift: false,
            },
        }
    }

    /// The digit this key types with no modifier, for counts.
    pub fn digit(self) -> Option<u32> {
        match self.code {
            Code::Char(c) if self.mods == Mods::default() => c.to_digit(10),
            _ => None,
        }
    }

    /// A plain printable character, as Insert types it.
    pub fn typed(self) -> Option<char> {
        match self.code {
            Code::Char(c) if !self.mods.ctrl && !self.mods.cmd && !self.mods.alt => Some(c),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct SeqError(pub String);

impl std::fmt::Display for SeqError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Reads a sequence in the keymap's syntax: `<leader>cn`, `]b`, `<C-h>`, `<D-CR>`, `<S-Tab>`.
/// `<leader>` is Space (`[meta] leader`).
pub fn parse_seq(seq: &str) -> Result<Vec<Key>, SeqError> {
    let mut keys = Vec::new();
    let mut rest = seq;
    while let Some(c) = rest.chars().next() {
        if c == '<'
            && let Some(end) = rest.find('>')
            && end > 1
        {
            keys.push(
                parse_angle(&rest[1..end])
                    .ok_or_else(|| SeqError(format!("`{seq}`: bad key `{}`", &rest[..=end])))?,
            );
            rest = &rest[end + 1..];
        } else {
            keys.push(Key::char(c));
            rest = &rest[c.len_utf8()..];
        }
    }
    if keys.is_empty() {
        return Err(SeqError("an empty key sequence".into()));
    }
    Ok(keys)
}

fn named(name: &str) -> Option<Code> {
    Some(match name {
        "CR" | "Enter" => Code::Enter,
        "Esc" => Code::Esc,
        "Tab" => Code::Tab,
        "BS" => Code::Backspace,
        "Del" => Code::Delete,
        "Space" | "leader" => Code::Char(' '),
        "lt" => Code::Char('<'),
        "gt" => Code::Char('>'),
        "Up" => Code::Up,
        "Down" => Code::Down,
        "Left" => Code::Left,
        "Right" => Code::Right,
        "Home" => Code::Home,
        "End" => Code::End,
        "PageUp" => Code::PageUp,
        "PageDown" => Code::PageDown,
        "label" => Code::Label,
        _ => {
            let mut cs = name.chars();
            let c = cs.next()?;
            if cs.next().is_some() {
                return None;
            }
            Code::Char(c)
        }
    })
}

fn parse_angle(inner: &str) -> Option<Key> {
    let mut mods = Mods::default();
    let mut rest = inner;
    while rest.len() > 2 && rest.as_bytes()[1] == b'-' {
        match rest.as_bytes()[0] {
            b'C' => mods.ctrl = true,
            b'M' => mods.alt = true,
            b'D' => mods.cmd = true,
            b'S' => mods.shift = true,
            _ => return None,
        }
        rest = &rest[2..];
    }
    let code = named(rest)?;
    if mods == Mods::default()
        && matches!(code, Code::Char(c) if c != ' ')
        && !matches!(rest, "lt" | "gt")
    {
        return None;
    }
    Some(Key { code, mods })
}

/// The inverse of `parse_seq`, for settings.toml: `<leader>cm`, `<M-m>`, `]o`, `<D-CR>`.
pub fn written(seq: &[Key]) -> String {
    let mut out = String::new();
    for k in seq {
        let name = match k.code {
            Code::Char('<') => "lt".to_string(),
            Code::Char('>') if k.mods != Mods::default() => "gt".to_string(),
            Code::Char(' ') if k.mods == Mods::default() => {
                out.push_str("<leader>");
                continue;
            }
            Code::Char(c) if k.mods == Mods::default() => {
                out.push(c);
                continue;
            }
            Code::Char(' ') => "Space".to_string(),
            Code::Char(c) => c.to_string(),
            Code::Enter => "CR".into(),
            Code::Esc => "Esc".into(),
            Code::Tab => "Tab".into(),
            Code::Backspace => "BS".into(),
            Code::Delete => "Del".into(),
            Code::Up => "Up".into(),
            Code::Down => "Down".into(),
            Code::Left => "Left".into(),
            Code::Right => "Right".into(),
            Code::Home => "Home".into(),
            Code::End => "End".into(),
            Code::PageUp => "PageUp".into(),
            Code::PageDown => "PageDown".into(),
            Code::Label => "label".into(),
        };
        out.push('<');
        for (on, m) in [
            (k.mods.ctrl, "C-"),
            (k.mods.alt, "M-"),
            (k.mods.cmd, "D-"),
            (k.mods.shift, "S-"),
        ] {
            if on {
                out.push_str(m);
            }
        }
        out.push_str(&name);
        out.push('>');
    }
    out
}

/// The design's printed form: `␣cn`, `⌘K`, `⌘⏎`, `⌥P`, `C-h`, `⌥⏎`, `S-Tab`, `Esc`. Option and
/// Command print as the Mac writes them; `written` keeps the keymap's `M-` and `D-`.
pub fn display(seq: &[Key]) -> String {
    let mut out = String::new();
    for (i, k) in seq.iter().enumerate() {
        let word = |s: &str| s.to_string();
        let base = match k.code {
            Code::Char(' ') => "␣".to_string(),
            Code::Char(c) if k.mods.cmd || k.mods.alt => c.to_uppercase().to_string(),
            Code::Char(c) => c.to_string(),
            Code::Enter => "⏎".to_string(),
            Code::Esc => word("Esc"),
            Code::Tab => word("Tab"),
            Code::Backspace => word("BS"),
            Code::Delete => word("Del"),
            Code::Up => word("Up"),
            Code::Down => word("Down"),
            Code::Left => word("Left"),
            Code::Right => word("Right"),
            Code::Home => word("Home"),
            Code::End => word("End"),
            Code::PageUp => word("PgUp"),
            Code::PageDown => word("PgDn"),
            Code::Label => word("label"),
        };
        let named = base.chars().count() > 1;
        if named && i > 0 && !out.ends_with(' ') {
            out.push(' ');
        }
        if k.mods.alt {
            out.push('⌥');
        }
        if k.mods.cmd {
            out.push('⌘');
        }
        if k.mods.ctrl {
            out.push_str("C-");
        }
        if k.mods.shift {
            out.push_str("S-");
        }
        out.push_str(&base);
        if named && i + 1 < seq.len() {
            out.push(' ');
        }
    }
    out
}

/// A rune key event as the keymap names it. `key_base` is `Tx::key_base()`: the character the key
/// types with no modifier, so Option-p (which types `π`) reads as `<M-p>`.
pub fn from_rune(ev: &rune::KeyEvent, key_base: Option<char>) -> Option<Key> {
    use rune::Key as R;
    let m = ev.mods;
    // Off macOS rune reports Control as `cmd` too; the keymap's `D-` is the Mac's Command only.
    let cmd = m.cmd && (cfg!(target_os = "macos") || !m.ctrl);
    let mods = Mods {
        ctrl: m.ctrl,
        alt: m.alt,
        cmd,
        shift: false,
    };
    let named = |code: Code| {
        Some(Key {
            code,
            mods: Mods {
                shift: m.shift,
                ..mods
            },
        })
    };
    match &ev.key {
        R::Text(s) => {
            let mut cs = s.chars();
            let c = cs.next()?;
            if cs.next().is_some() {
                return None;
            }
            let c = if mods.ctrl || mods.alt || mods.cmd {
                let base = key_base.unwrap_or(c);
                // A Control letter may arrive as its control character (Ctrl-H as U+0008).
                let base = if (base as u32) < 0x20 {
                    char::from_u32(base as u32 + 0x60).unwrap_or(base)
                } else {
                    base
                };
                base.to_ascii_lowercase()
            } else {
                c
            };
            Some(Key {
                code: Code::Char(c),
                mods,
            })
        }
        R::Command(c) => Some(Key {
            code: Code::Char(*c),
            mods: Mods { cmd, ..mods },
        }),
        R::Enter => named(Code::Enter),
        R::Escape => named(Code::Esc),
        R::Tab => named(Code::Tab),
        R::Backspace => named(Code::Backspace),
        R::Delete => named(Code::Delete),
        R::Up => named(Code::Up),
        R::Down => named(Code::Down),
        R::Left => named(Code::Left),
        R::Right => named(Code::Right),
        R::Home => named(Code::Home),
        R::End => named(Code::End),
        R::PageUp => named(Code::PageUp),
        R::PageDown => named(Code::PageDown),
        _ => None,
    }
}
