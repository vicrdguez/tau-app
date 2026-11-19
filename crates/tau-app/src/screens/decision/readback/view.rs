//! The readback float's layout and display formatting.

use rune::{El, col, el, icon, row, text};
use tau_client::DecisionRef;

use super::super::reader::record;
use super::super::{doc, standing};
use super::{Kind, Phase, Readback, plural};
use crate::app::App;
use crate::assets::marks;
use crate::screens::ScreenFloat;
use crate::theme::Theme;
use crate::time;
use crate::ui::{Shell, float_hint, float_row, kv_row, named, shell};

/// The head's title: `Review the design #2 · render-core`.
fn title(t: Theme, title: &str, d: &DecisionRef) -> El {
    let n = d.id.request_number().map(|n| format!("#{n}"));
    row()
        .center_y()
        .gap(7.0)
        .shrink()
        .min_w(0.0)
        .child(
            crate::content::line(t, title, t.ty.float_title, t.c.fg.default)
                .shrink()
                .min_w(0.0),
        )
        .children(n.map(|n| text(n).font(t.ty.mono_small).color(t.c.kind.literal)))
        .child(text("·").font(t.ty.ui_small).color(t.c.fg.muted))
        .child(
            text(d.project.to_string())
                .font(t.ty.float_title)
                .color(t.c.kind.worker),
        )
}

/// The solid coral verb tag (`◆ ANSWER`), or the neutral outline `◇ NOT RECORDED`.
fn tag(t: Theme, word: &str, solid: bool) -> El {
    let style = t.ty.badge;
    let (fg, mark) = if solid {
        (t.c.surface.read, &marks::DIAMOND)
    } else {
        (t.c.fg.alt, &marks::DIAMOND_OUTLINE)
    };
    let r = row()
        .center_y()
        .gap(5.0)
        .h(20.0)
        .px(8.0)
        .radius(t.l.radius.pill)
        .child(icon(mark, 9.0).color(fg))
        .child(text(word.to_string()).font(style).color(fg));
    if solid {
        r.fill(t.c.state.attn)
    } else {
        r.border(1.0, t.c.border.hairline)
    }
}

fn value(t: Theme, s: &str) -> El {
    text(s.to_string())
        .font(t.ty.ui)
        .color(t.c.fg.default)
        .truncate()
}

/// A right-column row: a 72 px steel label, then the value, 30 px tall.
fn rb_row(t: Theme, label: &str, v: El) -> El {
    row()
        .center_y()
        .h(t.l.float.row)
        .child(
            text(label.to_string())
                .font(t.ty.ui_small)
                .color(t.c.kind.you)
                .w(72.0),
        )
        .child(v.grow(1.0).shrink().min_w(0.0))
}

/// `w1.design_answer runs in #3`: the task kind and the `#id` in orange mono.
fn kind_value(t: Theme, kind: &str, rest: &str) -> El {
    let mut r = row().center_y().gap(5.0).child(
        text(kind.to_string())
            .font(t.ty.mono_small)
            .color(t.c.kind.literal),
    );
    match rest.rsplit_once(" #") {
        Some((words, n)) if n.chars().all(|c| c.is_ascii_digit()) => {
            r = r.child(value(t, words)).child(
                text(format!("#{n}"))
                    .font(t.ty.mono_small)
                    .color(t.c.kind.literal),
            );
        }
        _ => r = r.child(value(t, rest)),
    }
    r
}

pub(super) fn view(t: Theme, app: &App, f: &Readback) -> El {
    let scope = f.scope();
    let hint = |seqs: &[&str], word: &str| float_hint(t, scope, seqs, word);
    let Some(v) = record(app, &f.decision) else {
        return el();
    };
    let r = &v.decision;
    let device = app
        .link
        .server()
        .device
        .as_ref()
        .map_or("this device".to_string(), |d| format!("this device, {d}"));
    let open_meta = format!(
        "open · requested {}",
        time::clock(tau_client::Wall(r.requested_at))
    );
    let shell_of = |tag_el: El, meta: Option<String>, body: El, hints: Vec<El>| {
        shell(
            t,
            Shell {
                tag: Some(tag_el),
                title: title(t, &r.title, &f.decision),
                subtitle: None,
                meta,
                body,
                input: None,
                hints,
            },
        )
        .key("decision-readback")
    };
    if let Phase::Reply(reply) = &f.phase {
        let mut body = col().w_full();
        for (label, words) in &reply.rows {
            body = body.child(kv_row(t, label, value(t, words)));
        }
        return shell_of(
            tag(t, "NOT RECORDED", false),
            reply.meta.clone().or(Some(open_meta)),
            body,
            vec![hint(&["<Esc>"], "closes · your notes stay on this device")],
        );
    }
    let sending = matches!(f.phase, Phase::Sending(_));
    // `y` reads as the instruction it is, never as a button: a click only says it again.
    let confirm_hints = |what: &'static str| {
        if sending {
            return vec![text("sending…").font(t.ty.ui_small).color(t.c.fg.muted)];
        }
        let y = app.settings.keymap.chip("y", scope.mode, scope.ctx);
        let mut spoken = y.spoken.chars();
        let name: String = spoken
            .next()
            .into_iter()
            .flat_map(char::to_uppercase)
            .chain(spoken)
            .collect();
        let echo = format!("press {} to {what}", y.printed);
        let confirm = named(hint(&["y"], what), format!("Press {name} to {what}"))
            .key("readback-confirm")
            .on_background_press(move |a: &mut App| a.say(echo.clone()));
        vec![confirm, hint(&["n", "<Esc>", "q"], "cancel")]
    };
    match &f.kind {
        Kind::Answer => {
            let notes = app.screens.decision.notes.of(&f.decision);
            let mut left = col().w(300.0).pb(4.0);
            let numbered = r.choices.len() <= 9;
            for (i, c) in r.choices.iter().enumerate() {
                let picked = f.pick == Some(i);
                let mut content = row().center_y().gap(12.0);
                if numbered {
                    content = content.child(
                        text((i + 1).to_string())
                            .font(t.ty.mono_small)
                            .color(t.c.fg.alt)
                            .w(10.0),
                    );
                }
                content = content.child(
                    crate::content::line(
                        t,
                        c,
                        if picked { t.ty.ui_strong } else { t.ty.ui },
                        t.c.fg.default,
                    )
                    .shrink()
                    .min_w(0.0),
                );
                left = left.child(float_row(t, picked, content));
            }
            let range = if numbered {
                format!("j k or 1–{}", r.choices.len())
            } else {
                "j k".to_string()
            };
            left = left.child(
                text(range)
                    .font(t.ty.ui_small)
                    .color(t.c.fg.muted)
                    .px(t.l.float.padding_x)
                    .pt(8.0),
            );
            let passages = notes.iter().filter(|n| n.range.is_some()).count();
            let whole = notes.len() - passages;
            let notes_words = if notes.is_empty() {
                "none".to_string()
            } else {
                let mut parts = vec![format!("{} sent with the answer", notes.len())];
                if passages > 0 {
                    parts.push(format!(
                        "{passages} on {}",
                        if passages == 1 {
                            "a passage"
                        } else {
                            "passages"
                        }
                    ));
                }
                if whole > 0 {
                    parts.push(format!("{whole} on the whole"));
                }
                parts.join(" · ")
            };
            let mut notes_value = row().center_y().gap(16.0).child(value(t, &notes_words));
            if !notes.is_empty() {
                notes_value = notes_value.child(
                    row()
                        .center_y()
                        .gap(6.0)
                        .child(crate::ui::chip(t, scope, "K", true))
                        .child(text("reads them").font(t.ty.ui_small).color(t.c.fg.muted)),
                );
            }
            let then = r.then.as_ref().map(|th| th.task.clone());
            let mut right = col().w_full().pl(28.0).pr(t.l.float.padding_x);
            if f.notes_open {
                for n in notes {
                    let quote = n
                        .range
                        .as_ref()
                        .and_then(|rg| {
                            let a = doc::byte_of(&v.text, rg.start as usize)?;
                            let b = doc::byte_of(&v.text, rg.end as usize)?;
                            Some(v.text[a..b].to_string())
                        })
                        .unwrap_or_else(|| "on the whole".into());
                    right = right.child(rb_row(
                        t,
                        "note",
                        value(t, &format!("{} · {quote}", n.text)),
                    ));
                }
            } else {
                right = right
                    .child(rb_row(t, "notes", notes_value))
                    .child(rb_row(
                        t,
                        "basis",
                        value(
                            t,
                            &format!(
                                "entry {} · {} characters · shown to the end",
                                r.basis,
                                thousands(v.text.chars().count())
                            ),
                        ),
                    ))
                    .children(then.map(|task| {
                        rb_row(
                            t,
                            "then",
                            kind_value(t, &task, &format!("runs in #{}", r.conversation)),
                        )
                    }))
                    .child(rb_row(t, "from", value(t, &device)));
            }
            let body = row()
                .w_full()
                .child(left)
                .child(el().w(1.0).h_full().fill(t.c.border.hairline))
                .child(right.grow(1.0).min_w(0.0));
            shell_of(
                tag(t, "ANSWER", true),
                Some(open_meta),
                body,
                confirm_hints("send this answer"),
            )
        }
        Kind::AgainAfterFault(_) | Kind::AgainUnknown(_) => {
            let rec = match &f.kind {
                Kind::AgainAfterFault(rec) => (**rec).clone(),
                _ => r.clone(),
            };
            let mut body = col().w_full();
            match &f.kind {
                Kind::AgainUnknown(_) => {
                    body = body.child(kv_row(
                        t,
                        "last send",
                        value(t, "had no reply · it may have been recorded"),
                    ));
                }
                _ => {
                    let error = v
                        .then
                        .as_ref()
                        .and_then(|s| standing::then(s).error)
                        .unwrap_or_default();
                    let task = rec
                        .then
                        .as_ref()
                        .map_or(String::new(), |th| th.task.clone());
                    body = body.child(kv_row(
                        t,
                        "its work",
                        kind_value(t, &task, &format!("faulted · {error}")),
                    ));
                }
            }
            let (choice, notes) = match &f.kind {
                Kind::AgainUnknown(id) => {
                    let b = app
                        .link
                        .outbox()
                        .get(*id)
                        .map(|e| e.body.clone())
                        .unwrap_or_default();
                    let n = b["annotations"].as_array().map_or(0, Vec::len);
                    (b["choice"].as_str().unwrap_or("").to_string(), n)
                }
                _ => (
                    rec.choice.clone().unwrap_or_default(),
                    rec.annotations.len(),
                ),
            };
            body = body.child(kv_row(
                t,
                "sends",
                value(
                    t,
                    &format!(
                        "the same answer: {choice}, {}, basis entry {}",
                        plural(notes, "note", "notes"),
                        rec.basis
                    ),
                ),
            ));
            if let Some(th) = &rec.then {
                let words = match &f.kind {
                    Kind::AgainAfterFault(_) => format!("starts in #{}", rec.conversation),
                    _ => format!("runs in #{}", rec.conversation),
                };
                let lead = match &f.kind {
                    Kind::AgainAfterFault(_) => "a new",
                    _ => "",
                };
                body = body.child(kv_row(
                    t,
                    "then",
                    row()
                        .center_y()
                        .gap(5.0)
                        .children((!lead.is_empty()).then(|| value(t, lead)))
                        .child(kind_value(t, &th.task, &words)),
                ));
            }
            body = body.child(kv_row(t, "from", value(t, &device)));
            let meta = match &f.kind {
                Kind::AgainAfterFault(rec) => {
                    let mut m = format!("answered {}", rec.choice.clone().unwrap_or_default());
                    if let Some(by) = &rec.by {
                        m.push_str(&format!(" · {by}"));
                    }
                    if let Some(at) = rec.at {
                        m.push_str(&format!(" {}", time::clock(tau_client::Wall(at))));
                    }
                    m
                }
                _ => open_meta,
            };
            shell_of(
                tag(t, "SEND AGAIN", true),
                Some(meta),
                body,
                confirm_hints("send it again"),
            )
        }
    }
}

/// `2,203`.
pub fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
