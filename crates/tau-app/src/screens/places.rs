//! Conversations, Timeline, and Faults. Each place keeps its own filter and object cursor.
pub mod floats;
#[cfg(test)]
mod tests;

use super::home::list::{self, Cursor, ListRow, RowId};
use super::{NOTHING_RECEIVED, Screen, home};
use crate::app::{App, Handler, Stage};
use crate::frame::{Crumb, Place, Room};
use crate::keys::{Ctx, Key};
use crate::theme::Theme;
use crate::ui::Mark;
use rune::{El, Tx, col, row, text};
use serde::{Deserialize, Serialize};
use tau_client::ConvRef;

#[derive(Default, Serialize, Deserialize)]
pub struct State {
    pub conversations: ListState,
    pub timeline: ListState,
    pub faults: ListState,
    #[serde(skip)]
    loaded_url: Option<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ListState {
    pub project: Option<String>,
    pub state: Option<String>,
    pub search: String,
    pub show_hidden: bool,
    pub cursor: Cursor,
}

pub fn place(app: &App) -> Place {
    match app.stage {
        Stage::Place(p) => p,
        _ => Place::Conversations,
    }
}
pub fn settings(app: &App) -> &ListState {
    match place(app) {
        Place::Timeline => &app.screens.places.timeline,
        Place::Faults => &app.screens.places.faults,
        _ => &app.screens.places.conversations,
    }
}
pub fn settings_mut(app: &mut App) -> &mut ListState {
    match place(app) {
        Place::Timeline => &mut app.screens.places.timeline,
        Place::Faults => &mut app.screens.places.faults,
        _ => &mut app.screens.places.conversations,
    }
}
pub fn filter_project(app: &mut App, place: Place, project: Option<String>) {
    let s = match place {
        Place::Timeline => &mut app.screens.places.timeline,
        Place::Faults => &mut app.screens.places.faults,
        _ => &mut app.screens.places.conversations,
    };
    s.project = project;
    s.cursor = Cursor::default();
    save(app);
}
pub(crate) fn load(app: &mut App) {
    if app.link.server().url != app.screens.places.loaded_url {
        let url = app.link.server().url.clone();
        if let Some(saved) = app.link.store().load::<State>("places.json") {
            app.screens.places = saved;
        }
        app.screens.places.loaded_url = url;
    }
}

pub fn save(app: &mut App) {
    if let Err(error) = app.link.store().save("places.json", &app.screens.places) {
        app.say(format!("Could not save places on this device: {error}"));
    }
}
pub fn rows(app: &App) -> Vec<ListRow> {
    matching_rows(app, &settings(app).search)
}

pub fn matching_rows(app: &App, query: &str) -> Vec<ListRow> {
    let settings = settings(app);
    filtered_rows(app)
        .into_iter()
        .filter(|r| !r.hidden || settings.show_hidden)
        .filter(|r| r.label.to_lowercase().contains(&query.to_lowercase()))
        .collect()
}

/// Every row of the place, hidden ones included.
fn all_rows(app: &App) -> Vec<ListRow> {
    match place(app) {
        Place::Timeline => list::events(app),
        Place::Faults => list::faults(app),
        _ => list::conversations(app),
    }
}

/// The rows the place's filter keeps, hidden ones included.
fn filtered_rows(app: &App) -> Vec<ListRow> {
    let settings = settings(app);
    all_rows(app)
        .into_iter()
        .filter(|r| settings.project.as_ref().is_none_or(|p| *p == r.project))
        .filter(|r| {
            place(app) != Place::Conversations
                || settings.state.as_deref().is_none_or(|s| match s {
                    "failed" => r.mark == Some(Mark::Failed),
                    "running" => r.mark == Some(Mark::Running),
                    "idle" => r.mark.is_none(),
                    _ => true,
                })
        })
        .collect()
}

/// Why a place draws no rows.
enum Empty {
    /// The server has none.
    Nothing,
    /// The filter keeps none.
    Filtered,
    /// The filter keeps this many, all hidden on this device.
    Hidden(usize),
    /// The search matches none.
    Searched,
}

fn empty(app: &App) -> Empty {
    let kept = filtered_rows(app);
    if all_rows(app).is_empty() {
        Empty::Nothing
    } else if kept.is_empty() {
        Empty::Filtered
    } else if !settings(app).show_hidden && kept.iter().all(|r| r.hidden) {
        Empty::Hidden(kept.len())
    } else {
        Empty::Searched
    }
}

/// One line where the first row would be, saying why there are none.
fn empty_view(t: Theme, app: &App) -> El {
    let p = place(app);
    let s = settings(app);
    let say = |s: &str| list::line(t, s, t.c.fg.alt);
    let quiet = |s: &str| text(s.to_string()).font(t.ty.ui_small).color(t.c.fg.muted);
    let line = row().center_y().h(28.0).gap(12.0);
    match empty(app) {
        Empty::Nothing => line.child(say(match p {
            Place::Faults => "No faulted tasks.",
            Place::Timeline => "No events yet. Bare conversations write none.",
            _ => "No conversations yet.",
        })),
        Empty::Filtered => {
            let project = s.project.as_deref().unwrap_or_default();
            let sentence = match p {
                Place::Faults => format!("No faulted tasks in {project}."),
                Place::Timeline => format!("No events in {project} yet."),
                _ => format!(
                    "No {}conversations{}.",
                    s.state.as_ref().map_or(String::new(), |s| format!("{s} ")),
                    s.project
                        .as_ref()
                        .map_or(String::new(), |p| format!(" in {p}")),
                ),
            };
            let control = if s.state.is_some() {
                "show all"
            } else {
                "show all projects"
            };
            line.child(say(&sentence))
                .child(quiet(control).on_press(|a: &mut App| {
                    if a.floats.is_empty() {
                        show_all(a);
                    }
                }))
        }
        Empty::Hidden(n) => {
            let what = match (p, n) {
                (Place::Faults, 1) => "faulted task",
                (Place::Faults, _) => "faulted tasks",
                (_, 1) => "failed run",
                _ => "failed runs",
            };
            line.gap(8.0)
                .child(say(&format!("{n} {what} hidden on this device")))
                .child(
                    row()
                        .center_y()
                        .gap(8.0)
                        .child(crate::ui::chip(t, app.scope(), "<leader>th", false))
                        .child(say("show hidden"))
                        .on_press(|a: &mut App| {
                            if a.floats.is_empty() {
                                toggle_hidden(a);
                            }
                        }),
                )
        }
        Empty::Searched => line.child(say("No matching rows.")),
    }
}

/// Resets this place's filter to every project and, in Conversations, every state.
pub fn show_all(app: &mut App) {
    let s = settings_mut(app);
    s.project = None;
    s.state = None;
    s.cursor = Cursor::default();
    save(app);
    reconcile(app);
}

fn toggle_hidden(app: &mut App) {
    let s = settings_mut(app);
    s.show_hidden = !s.show_hidden;
    reconcile(app);
    save(app);
}
pub fn selected(app: &App) -> Option<ListRow> {
    let rs = rows(app);
    rs.get(settings(app).cursor.index(&rs)).cloned()
}
pub fn reconcile(app: &mut App) -> bool {
    if !received(app) {
        return false;
    }
    let ids = rows(app).into_iter().map(|r| r.id).collect::<Vec<_>>();
    settings_mut(app).cursor.reconcile(&ids)
}
pub fn page_older(app: &mut App) {
    let filter = settings(app).project.clone();
    let ps = app
        .link
        .server()
        .timelines
        .keys()
        .filter(|p| filter.as_ref().is_none_or(|s| s == p.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    for p in ps {
        app.link
            .page_older(tau_client::engine::TranscriptKey::Timeline(p));
    }
}
fn received(app: &App) -> bool {
    let Some(projects) = app.link.server().projects.value() else {
        return false;
    };
    projects
        .iter()
        .filter(|p| {
            settings(app)
                .project
                .as_ref()
                .is_none_or(|s| s == p.project.as_str())
        })
        .all(|p| match place(app) {
            Place::Timeline => app
                .link
                .server()
                .timelines
                .get(&p.project)
                .is_some_and(|t| t.docs.value().is_some()),
            Place::Faults => app
                .link
                .server()
                .faults
                .get(&p.project)
                .is_some_and(|k| k.value().is_some()),
            _ => app
                .link
                .server()
                .conversations
                .get(&p.project)
                .is_some_and(|k| k.value().is_some()),
        })
}

pub struct Places;
impl Screen for Places {
    fn ctx(&self, _: &App) -> Ctx {
        Ctx::List
    }
    fn crumbs(&self, app: &App) -> Vec<Crumb> {
        let s = settings(app);
        let mut filter = s.project.clone().unwrap_or("all projects".into());
        if place(app) == Place::Conversations
            && let Some(state) = &s.state
        {
            filter.push_str(&format!(" · {state}"));
        }
        let mut crumbs = vec![Crumb::Text(place(app).name().into()), Crumb::Text(filter)];
        if s.project.is_some() || s.state.is_some() {
            crumbs.push(Crumb::ShowAll(show_all));
        }
        crumbs
    }
    fn has_peek(&self, _: &App) -> bool {
        true
    }
    fn view(&self, t: Theme, app: &App) -> El {
        let p = place(app);
        let rs = rows(app);
        let id = selected(app).map(|r| r.id);
        let mut body = col().w_full().pt(28.0).px(28.0).child(
            text(p.name())
                .font(t.ty.title_page)
                .color(t.c.state.attn)
                .mx(12.0),
        );
        if p == Place::Timeline {
            body = body.child(home::since_view(t, app).mx(12.0));
        }
        if p == Place::Faults {
            body=body.child(text("A faulted task never runs again by itself. If it names a decision, open it and send the answer again. Hiding a fault is local to this device.").wrap().max_w(640.0).font(t.ty.ui).color(t.c.fg.alt).mt(6.0).mx(12.0));
        }
        if !received(app) {
            return body.child(
                list::line(t, NOTHING_RECEIVED, t.c.fg.alt)
                    .mt(14.0)
                    .mx(12.0),
            );
        }
        body = body.child(rune::el().h(if p == Place::Conversations { 0.0 } else { 18.0 }));
        let grid = list::Grid::place(app, &rs);
        let mut group = "";
        for r in &rs {
            if p == Place::Conversations && r.group != group {
                group = &r.group;
                let all = list::conversations(app)
                    .into_iter()
                    .filter(|r| r.group == group)
                    .collect::<Vec<_>>();
                let mut counts = Vec::new();
                if let Some(state) = &settings(app).state {
                    let shown = rs.iter().filter(|r| r.group == group).count();
                    counts.push(format!("{shown} {state} shown"));
                } else {
                    for (mark, word) in [(Mark::Running, "running"), (Mark::Failed, "failed")] {
                        let n = all.iter().filter(|r| r.mark == Some(mark)).count();
                        if n > 0 {
                            counts.push(format!("{n} {word}"));
                        }
                    }
                }
                counts.push(format!("{} in all", all.len()));
                body = body.child(
                    row()
                        .center_y()
                        .gap(8.0)
                        .h(30.0)
                        .mt(10.0)
                        .mx(12.0)
                        .child(
                            text(group.to_string())
                                .font(t.ty.group)
                                .color(t.c.kind.worker),
                        )
                        .child(
                            text(counts.join(" · "))
                                .font(t.ty.ui_small)
                                .color(t.c.fg.muted),
                        ),
                );
            }
            body = body.child(list::row_view(t, app, r, id.as_ref() == Some(&r.id), &grid));
            if r.hidden {
                body = body.child(row().pl(list::label_inset()).child(list::line(
                    t,
                    "hidden on this device",
                    t.c.fg.muted,
                )));
            }
        }
        if rs.is_empty() {
            body = body.child(empty_view(t, app).mx(12.0));
        }
        if p == Place::Timeline {
            body = body.child(rune::el().h(1.0).on_layout(|a: &mut App, rect| {
                if a.live && rect[1] >= 44.0 && rect[1] < a.window[1] - 26.0 {
                    page_older(a);
                }
            }));
        }
        body.pb(28.0)
            .scroll(rune::ScrollSpec {
                stick_bottom: false,
            })
            .h_full()
            .key("places-list")
    }
    fn commands(&self) -> Vec<(&'static str, Handler)> {
        vec![
            ("list.open", |a, _, _| {
                if let Some(r) = selected(a) {
                    list::open(a, &r.id, false)
                }
            }),
            ("goto.linked", |a, _, _| {
                if let Some(r) = selected(a) {
                    list::open(a, &r.id, true)
                }
            }),
            ("group.next", home::group_next),
            ("row.hide", |a, _, _| {
                if a.scope().ctx == Ctx::Outbox {
                    floats::forget(a);
                } else if let Some(r) = selected(a) {
                    list::hide(a, &r.id);
                    reconcile(a);
                }
            }),
            ("toggle.hidden", |a, _, _| toggle_hidden(a)),
            ("list.filter_project", |a, _, _| floats::filter(a)),
            ("picker.keys", floats::picker_keys),
            ("outbox.keys", floats::outbox_keys),
            ("recover.retry", floats::retry),
            ("server.keys", floats::server_keys),
        ]
    }
    fn globals(&self) -> Vec<(&'static str, Handler)> {
        vec![
            ("find.object", |a, _, _| floats::find(a)),
            ("recover.outbox", |a, _, _| floats::outbox(a)),
            ("open.server", |a, _, _| floats::server(a)),
            ("recover.reconnect", |a, _, _| a.link.reconnect()),
        ]
    }
    fn anchor(&self, app: &App) -> Option<super::Anchor> {
        home::nav::anchor(app)
    }
    fn land(&self, app: &mut App, anchor: &super::Anchor) {
        home::nav::land(app, anchor);
    }
    fn targets(&self, app: &App) -> Vec<super::HintTarget> {
        home::nav::targets(app)
    }
    fn keys(&self, app: &mut App, keys: &[Key], count: Option<u32>) -> bool {
        if !app.floats.is_empty() {
            return floats::typing(app, keys);
        }
        if keys == [Key::char('/')] {
            floats::search(app);
            return true;
        }
        home::motion(app, keys, count)
    }
    fn room(&self, app: &App) -> Room {
        Room::Lift(
            settings(app)
                .cursor
                .id
                .as_ref()
                .map(|id| format!("list-{id:?}")),
        )
    }
    fn shown(&self, app: &mut App) {
        load(app);
        if place(app) == Place::Faults {
            app.link.refresh_faults();
        }
        reconcile(app);
    }
    fn poll(&self, app: &mut App, _: f64, tx: &mut Tx) -> bool {
        if !matches!(
            app.stage,
            Stage::Place(Place::Conversations | Place::Timeline | Place::Faults)
        ) {
            return false;
        }
        let changed = reconcile(app);
        if std::mem::take(&mut app.screens.home.scroll_cursor)
            && let Some(id) = &settings(app).cursor.id
        {
            tx.scroll_into_view(format!("list-{id:?}"));
        }
        changed
    }
    fn scenes(&self) -> Vec<crate::app::scenes::Scene> {
        vec![
            ("conversations", |a| {
                scene(a, Place::Conversations);
                home::select(a, RowId::Conversation(ConvRef::new("render-core", 7)));
                super::peek::settle_scene(a);
            }),
            ("timeline", |a| {
                scene(a, Place::Timeline);
                if let Some(r) = rows(a).into_iter().find(|r| r.project == "render-core") {
                    home::select(a, r.id);
                }
                super::peek::settle_scene(a);
            }),
            ("faults", |a| scene(a, Place::Faults)),
            ("faults-hidden", |a| {
                scene(a, Place::Faults);
                for r in list::faults(a) {
                    if let RowId::Fault(p, task) = r.id {
                        a.hidden.faults.insert((p, task));
                    }
                }
                crate::app::link::refresh(a);
                reconcile(a);
            }),
            ("conversations-running", |a| {
                scene(a, Place::Conversations);
                a.screens.places.conversations.state = Some("running".into());
                reconcile(a);
                super::peek::settle_scene(a);
            }),
            ("timeline-filtered", |a| {
                scene(a, Place::Timeline);
                filter_project(a, Place::Timeline, Some("notes-cli".into()));
                reconcile(a);
            }),
            ("filter", |a| {
                scene(a, Place::Conversations);
                floats::filter(a)
            }),
            ("find", |a| {
                home::scene(a);
                floats::find(a)
            }),
            ("server", |a| {
                home::scene(a);
                floats::server(a)
            }),
            ("outbox", |a| {
                home::scene(a);
                floats::outbox(a)
            }),
        ]
    }
}
fn scene(app: &mut App, p: Place) {
    crate::app::scenes::on_fixture(app, Stage::Place(p));
    home::prepare_scene(app);
    reconcile(app);
    super::peek::settle_scene(app);
}
