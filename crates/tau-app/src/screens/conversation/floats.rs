//! The conversation's floats: the model and thinking pickers (`␣cm`, `␣ct`, `spec.md` §6) and a
//! record's data (`K`). Each reads the link here and draws through `crate::floats`.

use rune::{El, TextEdit, Tx};
use tau_client::shapes::selection_workflow;
use tau_client::wire::ThinkingLevel;
use tau_client::{ConvRef, Write};

use super::state::shown;
use crate::app::{App, Invocation};
use crate::floats::choices::{self, Choice, Choices};
use crate::floats::detail::{self, Detail};
use crate::floats::{Float, Noun, cursor_step};
use crate::keys::{Code, Ctx, Key, Mode};
use crate::screens::{NOTHING_RECEIVED, ScreenFloat, ScreenId, top_float, top_float_ref};
use crate::theme::Theme;
use crate::ui::{Badge, Scope};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Model,
    Thinking,
}

/// `␣cm` or `␣ct` over the conversation it was opened in.
pub struct Picker {
    pub conv: ConvRef,
    pub pick: Pick,
    pub query: TextEdit,
    pub selected: Option<String>,
}

/// A row and what choosing it sets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub label: String,
    /// The model ID or the level's wire word.
    pub value: String,
    pub detail: Option<String>,
    pub current: bool,
}

const LEVELS: [(ThinkingLevel, &str); 7] = [
    (ThinkingLevel::Off, "off"),
    (ThinkingLevel::Minimal, "minimal"),
    (ThinkingLevel::Low, "low"),
    (ThinkingLevel::Medium, "medium"),
    (ThinkingLevel::High, "high"),
    (ThinkingLevel::Xhigh, "xhigh"),
    (ThinkingLevel::Max, "max"),
];

impl Picker {
    pub fn open(app: &mut App, pick: Pick) {
        let Some(s) = shown(app) else { return };
        let conv = s.conv.clone();
        if pick == Pick::Model {
            app.link.refresh_catalogue();
        }
        let mut query = TextEdit::new("");
        query.focused = true;
        let mut picker = Picker {
            conv,
            pick,
            query,
            selected: None,
        };
        picker.selected = picker
            .all(app)
            .into_iter()
            .find(|r| r.current)
            .map(|r| r.value);
        app.open_float(Float::Screen(Box::new(picker)));
    }

    /// The rows the query keeps, in the server's order.
    pub fn rows(&self, app: &App) -> Vec<Row> {
        let q = self.query.text.trim().to_lowercase();
        self.all(app)
            .into_iter()
            .filter(|r| {
                q.is_empty()
                    || r.label.to_lowercase().contains(&q)
                    || r.value.to_lowercase().contains(&q)
            })
            .collect()
    }

    fn cursor(&self, rows: &[Row]) -> usize {
        rows.iter()
            .position(|row| Some(&row.value) == self.selected.as_ref())
            .unwrap_or(0)
    }

    /// Every row, before the query.
    fn all(&self, app: &App) -> Vec<Row> {
        let agent = app
            .link
            .server()
            .transcripts
            .get(&self.conv)
            .and_then(|h| h.docs.value())
            .map(|d| &d.agent);
        let model = agent.and_then(|a| a.model.clone());
        let level = agent.and_then(|a| a.thinking_level.clone());
        match self.pick {
            Pick::Model => app
                .link
                .server()
                .models
                .value()
                .into_iter()
                .flatten()
                .map(|m| Row {
                    label: m.name.clone(),
                    value: m.model.clone(),
                    detail: Some(m.model.clone()),
                    current: model.as_deref() == Some(m.model.as_str()),
                })
                .collect(),
            Pick::Thinking => LEVELS
                .iter()
                .map(|(_, word)| Row {
                    label: word.to_string(),
                    value: word.to_string(),
                    detail: None,
                    current: level.as_deref() == Some(word),
                })
                .collect(),
        }
    }
}

impl ScreenFloat for Picker {
    fn owner(&self) -> ScreenId {
        ScreenId::Conversation
    }

    fn scope(&self) -> Scope {
        Scope {
            mode: Mode::Command,
            ctx: Ctx::Picker,
        }
    }

    fn badge(&self) -> Badge {
        Badge::Command
    }

    fn offers_model_refresh(&self) -> bool {
        self.pick == Pick::Model
    }

    fn view(&self, t: Theme, app: &App) -> El {
        let rows = self.rows(app);
        let choices: Vec<Choice> = rows
            .iter()
            .map(|r| Choice {
                label: r.label.clone(),
                detail: r.detail.clone(),
                current: r.current,
            })
            .collect();
        let (title, noun) = match self.pick {
            Pick::Model => ("Model", Noun::new("model", "models")),
            Pick::Thinking => ("Thinking", Noun::new("level", "levels")),
        };
        let received = self.pick == Pick::Thinking || app.link.server().models.value().is_some();
        choices::view(
            t,
            self.scope(),
            Choices {
                title,
                subject: Some(format!("#{}", self.conv.id)),
                meta: workflow_note(app, &self.conv),
                rows: &choices,
                cursor: self.cursor(&rows),
                query: &self.query,
                noun,
                total: self.all(app).len(),
                empty: (!received).then_some(NOTHING_RECEIVED),
                refresh: self
                    .offers_model_refresh()
                    .then(|| crate::floats::model_refresh(t, self.scope(), app)),
            },
        )
    }

    fn edit_mut(&mut self) -> Option<&mut TextEdit> {
        Some(&mut self.query)
    }
}

/// `w1 may set the model again when it next sends work here`, when the conversation's selection
/// names a workflow (`spec.md` §4.5, §6).
pub fn workflow_note(app: &App, conv: &ConvRef) -> Option<String> {
    let server = app.link.server();
    let agent = &server.transcripts.get(conv)?.docs.value()?.agent;
    let catalogue = server.extensions.value()?;
    selection_workflow(&agent.extensions, catalogue)
        .map(|w| format!("{w} may set the model again when it next sends work here"))
}

/// The picker's keys: `C-j C-k` move, `⏎` sets, `Esc C-c` close.
pub fn picker_keys(app: &mut App, inv: &Invocation, _: &mut Tx) {
    let Some(&key) = inv.keys.first() else { return };
    if key == Key::plain(Code::Esc) || key == Key::ctrl('c') {
        app.close_float();
        return;
    }
    let Some(p) = top_float_ref::<Picker>(app) else {
        return;
    };
    let rows = p.rows(app);
    let cursor = p.cursor(&rows);
    if let Some(step) = cursor_step(key) {
        let next = (cursor as i64 + step).clamp(0, rows.len().saturating_sub(1) as i64) as usize;
        if let Some(p) = top_float::<Picker>(app) {
            p.selected = rows.get(next).map(|r| r.value.clone());
        }
        return;
    }
    if key != Key::plain(Code::Enter) {
        return;
    }
    let Some(row) = rows.get(cursor) else { return };
    let (conv, pick) = (p.conv.clone(), p.pick);
    app.close_float();
    let current_model = app
        .link
        .server()
        .transcripts
        .get(&conv)
        .and_then(|h| h.docs.value())
        .and_then(|d| d.agent.model.clone());
    let write = match pick {
        Pick::Model => Write::SetModel {
            conv,
            model: row.value.clone(),
            thinking: None,
        },
        Pick::Thinking => {
            let Some(model) = current_model else {
                app.say("The model isn't known yet; set the thinking level once it is");
                return;
            };
            Write::SetModel {
                conv,
                model,
                thinking: LEVELS
                    .iter()
                    .find(|(_, w)| *w == row.value)
                    .map(|(l, _)| *l),
            }
        }
    };
    if let Err(e) = app.link.write(write) {
        app.say(super::not_sent(&e));
    }
}

/// A record's `data` as JSON (`K`).
pub struct RecordData {
    pub kind: String,
    pub subtitle: String,
    pub json: String,
}

impl ScreenFloat for RecordData {
    fn owner(&self) -> ScreenId {
        ScreenId::Conversation
    }

    fn scope(&self) -> Scope {
        Scope {
            mode: Mode::Peek,
            ctx: Ctx::Peek,
        }
    }

    fn badge(&self) -> Badge {
        Badge::Peek
    }

    fn view(&self, t: Theme, _: &App) -> El {
        detail::view(
            t,
            self.scope(),
            Detail {
                kind: &self.kind,
                subtitle: self.subtitle.clone(),
                text: &self.json,
            },
        )
    }
}

#[cfg(test)]
pub fn record_float_text(app: &App) -> Option<String> {
    top_float_ref::<RecordData>(app).map(|r| r.json.clone())
}

#[cfg(test)]
pub fn picker_rows(app: &App) -> Vec<Row> {
    top_float_ref::<Picker>(app)
        .map(|p| p.rows(app))
        .unwrap_or_default()
}

#[cfg(test)]
pub fn picker_cursor(app: &App) -> usize {
    top_float_ref::<Picker>(app).map_or(0, |p| p.cursor(&p.rows(app)))
}
