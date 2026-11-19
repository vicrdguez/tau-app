//! Home owns its ID cursor, the away base, and device-local hidden failures. Rows are derived
//! from the client model; no polling or protocol behavior lives here.
#[cfg(test)]
mod click_tests;
#[cfg(test)]
pub(crate) mod drawn;
pub mod list;
pub mod nav;
mod rows;
#[cfg(test)]
mod slot_tests;
#[cfg(test)]
mod tests;

use rune::{El, Tx, col, row, text};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tau_client::clock::{Clock, SystemClock};
use tau_client::{ConvRef, DecisionRef, Project, Wall};

use super::{NOTHING_RECEIVED, Screen, places};
use crate::app::{App, Handler, Invocation, Stage};
use crate::frame::{Crumb, Place};
use crate::keys::{Ctx, Key};
use crate::open::Item;
use crate::theme::Theme;
use crate::ui::{self, Mark};
#[cfg(test)]
use list::Slots;
use list::{Cursor, ListRow, RowId};

#[derive(Default)]
pub struct State {
    pub cursor: Cursor,
    pub away: Away,
    read_events: HashSet<RowId>,
    pub(super) scroll_cursor: bool,
    loaded: bool,
    url: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Away {
    pub base: Option<i64>,
    pub newest: HashMap<String, u64>,
    #[serde(skip)]
    pub visible: bool,
    #[serde(skip)]
    returned: Option<i64>,
}
impl Away {
    pub fn resume(&mut self, now: Wall) {
        self.visible = self.base.is_some_and(|base| now.0 - base >= 300_000);
        self.returned = Some(now.0);
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Saved {
    cursor: Cursor,
    away: Away,
    runs: Vec<ConvRef>,
    faults: Vec<(Project, tau_client::TaskId)>,
}

pub(crate) fn load(app: &mut App) {
    let url = app.link.server().url.clone();
    if !app.screens.home.loaded || app.screens.home.url != url {
        app.screens.home.loaded = true;
        app.screens.home.url = url;
        if let Some(saved) = app.link.store().load::<Saved>("home.json") {
            app.screens.home.cursor = saved.cursor;
            app.screens.home.away = saved.away;
            app.hidden.runs = saved.runs.into_iter().collect();
            app.hidden.faults = saved.faults.into_iter().collect();
            let now = now(app);
            app.screens.home.away.resume(now);
        }
    }
}

pub fn save(app: &mut App) {
    let saved = Saved {
        cursor: app.screens.home.cursor.clone(),
        away: app.screens.home.away.clone(),
        runs: app.hidden.runs.iter().cloned().collect(),
        faults: app.hidden.faults.iter().cloned().collect(),
    };
    if let Err(error) = app.link.store().save("home.json", &saved) {
        app.say(format!("Could not save Home on this device: {error}"));
    }
}

pub fn now(app: &App) -> Wall {
    if app.live {
        SystemClock.now().wall
    } else {
        Wall(1_794_674_400_000)
    }
}

pub fn title(app: &App) -> &'static str {
    match app.link.server().open_decisions() {
        None => "Home",
        Some(0) => "Nothing needs you",
        Some(_) => "Needs you",
    }
}

pub fn rows(app: &App) -> Vec<ListRow> {
    let Some(inbox) = app.link.server().inbox.value() else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for (p, d) in inbox.oldest_first() {
        let mut r = ListRow::new(
            RowId::Decision(DecisionRef::new(p.clone(), d.id.clone())),
            "Needs you",
            p.as_str(),
            d.title.clone(),
        );
        r.at = Some(d.requested_at);
        r.mark = Some(Mark::Decision);
        r.conversation = Some(d.conversation.0);
        rows.push(r);
    }
    if app.server.counts.failed.is_some() {
        let faults = list::faults(app)
            .into_iter()
            .filter(|r| !r.hidden)
            .collect::<Vec<_>>();
        let convs = list::conversations(app);
        let mut failed = faults
            .iter()
            .filter(|r| r.after.is_some())
            .cloned()
            .collect::<Vec<_>>();
        failed.extend(
            convs
                .iter()
                .filter(|r| r.mark == Some(Mark::Failed) && !r.hidden)
                .cloned()
                .map(|mut r| {
                    r.group = "Failed".into();
                    r
                }),
        );
        failed.extend(faults.into_iter().filter(|r| r.after.is_none()));
        let extra_runs = failed
            .iter()
            .skip(8)
            .filter(|r| matches!(r.id, RowId::Conversation(_)))
            .count();
        let extra_faults = failed.len().saturating_sub(8) - extra_runs;
        rows.extend(failed.into_iter().take(8));
        for (id, n, one, many, place) in [
            (
                RowId::MoreRuns,
                extra_runs,
                "failed run",
                "failed runs",
                "Conversations",
            ),
            (
                RowId::MoreFaults,
                extra_faults,
                "faulted task",
                "faulted tasks",
                "Faults",
            ),
        ] {
            if n > 0 {
                rows.push(ListRow::new(
                    id,
                    "Failed",
                    "",
                    format!("{n} more {} · in {place}", if n == 1 { one } else { many }),
                ));
            }
        }
        rows.extend(
            convs
                .into_iter()
                .filter(|r| r.mark == Some(Mark::Running))
                .map(|mut r| {
                    r.group = "Running".into();
                    r
                }),
        );
    }
    let events = list::events(app);
    let extra = events.len().saturating_sub(12);
    rows.extend(events.into_iter().take(12));
    if extra > 0 {
        rows.push(ListRow::new(
            RowId::MoreEvents,
            "Activity",
            "",
            format!("{extra} older in the timeline"),
        ));
    }
    rows
}

pub fn selected(app: &App) -> Option<ListRow> {
    let rows = rows(app);
    rows.get(app.screens.home.cursor.index(&rows)).cloned()
}

pub fn reconcile(app: &mut App) -> bool {
    if app.link.server().inbox.value().is_none() || app.server.counts.failed.is_none() {
        return false;
    }
    let s = app.link.server();
    let before = app.hidden.runs.len();
    app.hidden.runs.retain(|c| {
        s.conversations
            .get(&c.project)
            .and_then(|k| k.value())
            .and_then(|rs| rs.iter().find(|r| r.conversation == c.id))
            .is_none_or(|r| r.state == tau_client::wire::RunState::Failed)
    });
    if before != app.hidden.runs.len() {
        crate::app::link::refresh(app);
    }
    let ids = rows(app).into_iter().map(|r| r.id).collect::<Vec<_>>();
    let old = app.screens.home.cursor.id.clone();
    let changed = app.screens.home.cursor.reconcile(&ids);
    if changed && matches!(old, Some(RowId::Decision(_))) && app.stage == Stage::Place(Place::Home)
    {
        app.say("answered elsewhere");
    }
    changed
}

pub fn select(app: &mut App, id: RowId) {
    if matches!(id, RowId::Event(..)) {
        app.screens.home.read_events.insert(id.clone());
    }
    app.screens.home.scroll_cursor = true;
    match app.stage {
        Stage::Place(Place::Home) => app.screens.home.cursor.id = Some(id),
        Stage::Place(_) => places::settings_mut(app).cursor.id = Some(id),
        _ => return,
    }
    save(app);
    places::save(app);
}

pub fn motion(app: &mut App, keys: &[Key], count: Option<u32>) -> bool {
    let home = app.stage == Stage::Place(Place::Home);
    let rows = if home { rows(app) } else { places::rows(app) };
    if rows.is_empty() {
        return true;
    }
    let cursor = if home {
        &app.screens.home.cursor
    } else {
        &places::settings(app).cursor
    };
    let at = cursor.index(&rows);
    let n = count.unwrap_or(1) as usize;
    let last = rows.len() - 1;
    let to = match keys {
        [k] if *k == Key::char('j') => (at + n).min(last),
        [k] if *k == Key::char('k') => at.saturating_sub(n),
        [k] if *k == Key::ctrl('d') => (at + 10 * n).min(last),
        [k] if *k == Key::ctrl('u') => at.saturating_sub(10 * n),
        [g, h] if *g == Key::char('g') && *h == Key::char('g') => 0,
        [k] if *k == Key::char('G') => last,
        _ => return false,
    };
    select(app, rows[to].id.clone());
    if !home && app.stage == Stage::Place(Place::Timeline) && to == last {
        places::page_older(app);
    }
    true
}

pub fn group_next(a: &mut App, inv: &Invocation, _: &mut Tx) {
    let rs = if a.stage == Stage::Place(Place::Home) {
        rows(a)
    } else {
        places::rows(a)
    };
    if rs.is_empty() {
        return;
    }
    let cursor = if a.stage == Stage::Place(Place::Home) {
        &a.screens.home.cursor
    } else {
        &places::settings(a).cursor
    };
    let mut at = cursor.index(&rs);
    let heads = rs
        .iter()
        .enumerate()
        .filter(|(i, r)| *i == 0 || rs[i - 1].group != r.group)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    for _ in 0..inv.count.unwrap_or(1) {
        at = if inv.keys.first() == Some(&Key::char('[')) {
            heads.iter().rev().find(|i| **i < at).copied().unwrap_or(0)
        } else {
            heads.iter().find(|i| **i > at).copied().unwrap_or(at)
        };
    }
    select(a, rs[at].id.clone());
}

pub fn open_failed(app: &mut App) {
    app.show(Stage::Place(Place::Home));
    if let Some(r) = rows(app).into_iter().find(|r| r.group == "Failed") {
        select(app, r.id);
    }
}

fn next_decision(a: &mut App, inv: &Invocation, _: &mut Tx) {
    let Some(inbox) = a.link.server().inbox.value() else {
        return;
    };
    let ds = inbox
        .oldest_first()
        .into_iter()
        .map(|(p, d)| RowId::Decision(DecisionRef::new(p.clone(), d.id.clone())))
        .collect::<Vec<_>>();
    if ds.is_empty() {
        a.say("Nothing needs you");
        return;
    }
    let from = match &a.stage {
        Stage::Item(Item::Decision { project, id }) => Some(RowId::Decision(DecisionRef::new(
            project.clone(),
            id.clone(),
        ))),
        _ => selected(a).map(|r| r.id),
    };
    let at = if inv.command.id == "next.decision" {
        0
    } else {
        let step = inv.count.unwrap_or(1) as i64
            * if inv.keys.first() == Some(&Key::char('[')) {
                -1
            } else {
                1
            };
        from.and_then(|id| ds.iter().position(|d| *d == id))
            .map(|at| (at as i64 + step).rem_euclid(ds.len() as i64) as usize)
            .unwrap_or(0)
    };
    list::open(a, &ds[at], false);
}

pub fn since_counts(app: &App) -> (usize, usize, usize) {
    let base = app.screens.home.away.base.unwrap_or(i64::MAX);
    let needs = app
        .link
        .server()
        .inbox
        .value()
        .map(|i| {
            i.oldest_first()
                .iter()
                .filter(|(_, d)| d.requested_at > base)
                .count()
        })
        .unwrap_or(0);
    let faults = list::faults(app)
        .iter()
        .filter(|r| r.at.is_some_and(|at| at > base))
        .count();
    let events = list::events(app)
        .iter()
        .filter(|r| r.at.is_some_and(|at| at > base))
        .count();
    (needs, faults, events)
}

pub fn is_new(app: &App, r: &ListRow) -> bool {
    app.screens.home.away.visible
        && r.at
            .zip(app.screens.home.away.base)
            .is_some_and(|(at, base)| at > base)
        && !app.screens.home.read_events.contains(&r.id)
}

pub fn since_view(t: Theme, app: &App) -> El {
    let away = &app.screens.home.away;
    if !away.visible {
        return rune::el();
    }
    let base = Wall(away.base.unwrap_or_default());
    let (needs, faults, events) = since_counts(app);
    let parts = [
        format!("Since {} on this device", crate::time::clock(base)),
        format!(
            "away {}",
            crate::time::span(crate::time::between(
                base,
                Wall(away.returned.unwrap_or(now(app).0))
            ))
        ),
        format!("{needs} new for you"),
        format!(
            "{faults} new {}",
            if faults == 1 { "fault" } else { "faults" }
        ),
        format!("{events} events"),
    ];
    row()
        .center_y()
        .gap(7.0)
        .mt(4.0)
        .h(18.0)
        .child(rune::icon(&crate::assets::carbon::TIME, 13.0).color(t.c.state.stale))
        .child(
            ui::facts(t, parts.iter().map(|s| ui::Part::text(t, s)).collect())
                .shrink()
                .min_w(0.0),
        )
}

pub struct Home;
impl Screen for Home {
    fn ctx(&self, _: &App) -> Ctx {
        Ctx::List
    }
    fn crumbs(&self, app: &App) -> Vec<Crumb> {
        let mut out = vec![Crumb::Text("Home".into())];
        if let Some(r) = selected(app) {
            let rs = rows(app)
                .into_iter()
                .filter(|row| row.group == r.group)
                .collect::<Vec<_>>();
            let at = rs.iter().position(|row| row.id == r.id).unwrap_or(0) + 1;
            out.push(Crumb::Text(r.group));
            out.push(Crumb::Text(format!("{at} of {}", rs.len())));
        }
        out
    }
    fn has_peek(&self, _: &App) -> bool {
        true
    }
    fn view(&self, t: Theme, app: &App) -> El {
        let mut title = row()
            .center_y()
            .gap(10.0)
            .child(text(title(app)).font(t.ty.title_page).color(t.c.state.attn));
        if let Some(tag) = app
            .link
            .server()
            .open_decisions()
            .and_then(|n| ui::count_tag(t, ui::Tag::Needs, n as u32, true))
        {
            title = title.child(tag);
        }
        let mut body = col()
            .w_full()
            .pt(28.0)
            .px(28.0)
            .child(title.mx(12.0))
            .child(since_view(t, app).mx(12.0));
        if app.link.server().inbox.value().is_none() {
            return body.child(
                list::line(t, NOTHING_RECEIVED, t.c.fg.alt)
                    .mt(14.0)
                    .mx(12.0),
            );
        }
        if app
            .link
            .server()
            .projects
            .value()
            .is_some_and(Vec::is_empty)
        {
            return body.child(
                row()
                    .center_y()
                    .gap(8.0)
                    .mt(14.0)
                    .mx(12.0)
                    .child(list::line(t, "No projects yet", t.c.fg.alt))
                    .child(ui::chip(t, app.scope(), "<leader>pb", false))
                    .child(list::line(t, "binds one", t.c.fg.alt)),
            );
        }
        let rs = rows(app);
        let grid = list::Grid::home(app, &rs);
        let selected = selected(app).map(|r| r.id);
        let mut group = "";
        for r in &rs {
            if r.group != group {
                group = &r.group;
                if group == "Needs you" {
                    body = body.child(rune::el().h(18.0));
                } else {
                    let count = if group == "Failed" {
                        app.server.counts.failed
                    } else if group == "Running" {
                        Some(rs.iter().filter(|r| r.group == group).count() as u32)
                    } else {
                        None
                    };
                    body = body.child(ui::group_head(t, group, count).mt(26.0).mb(4.0).mx(12.0));
                }
            }
            let last_need = r.group == "Needs you"
                && rs
                    .iter()
                    .rfind(|r| r.group == "Needs you")
                    .is_some_and(|last| last.id == r.id);
            let rendered = list::row_view(t, app, r, selected.as_ref() == Some(&r.id), &grid);
            body = body.child(if last_need {
                rendered.mb(2.0)
            } else {
                rendered
            });
        }
        if app.server.counts.failed.is_none() {
            body = body.child(
                list::line(t, NOTHING_RECEIVED, t.c.fg.alt)
                    .mt(26.0)
                    .mx(12.0),
            );
        }
        if !rs.iter().any(|r| r.group == "Activity") {
            body = body
                .child(ui::group_head(t, "Activity", None).mt(26.0).mx(12.0))
                .child(
                    list::line(
                        t,
                        "No events yet. Bare conversations write none.",
                        t.c.fg.alt,
                    )
                    .mx(12.0),
                );
        }
        body.pb(28.0)
            .scroll(rune::ScrollSpec {
                stick_bottom: false,
            })
            .h_full()
            .key("home-list")
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
            ("group.next", group_next),
            ("row.hide", |a, _, _| {
                if let Some(r) = selected(a) {
                    list::hide(a, &r.id);
                    reconcile(a);
                }
            }),
        ]
    }
    fn globals(&self) -> Vec<(&'static str, Handler)> {
        vec![
            ("next.decision", next_decision),
            ("decision.next", next_decision),
        ]
    }
    fn anchor(&self, app: &App) -> Option<super::Anchor> {
        nav::anchor(app)
    }
    fn land(&self, app: &mut App, anchor: &super::Anchor) {
        nav::land(app, anchor);
    }
    fn targets(&self, app: &App) -> Vec<super::HintTarget> {
        nav::targets(app)
    }
    fn keys(&self, app: &mut App, keys: &[Key], count: Option<u32>) -> bool {
        motion(app, keys, count)
    }
    fn reveal_target(&self, app: &App, tx: &mut Tx) {
        if let Some(id) = &app.screens.home.cursor.id {
            tx.scroll_into_view(format!("list-{id:?}"));
        }
    }
    fn shown(&self, app: &mut App) {
        app.link.refresh_faults();
        reconcile(app);
    }
    fn on_focus(&self, app: &mut App, focused: bool) {
        let now = now(app);
        if focused {
            app.screens.home.away.resume(now);
        } else {
            app.screens.home.away = Away {
                base: Some(now.0),
                newest: app
                    .link
                    .server()
                    .timelines
                    .iter()
                    .filter_map(|(p, t)| t.newest().map(|id| (p.to_string(), id.0)))
                    .collect(),
                ..Away::default()
            };
            app.screens.home.read_events.clear();
            save(app);
        }
    }
    fn poll(&self, app: &mut App, _: f64, tx: &mut Tx) -> bool {
        let changed = reconcile(app);
        if app.stage == Stage::Place(Place::Home)
            && std::mem::take(&mut app.screens.home.scroll_cursor)
            && let Some(id) = &app.screens.home.cursor.id
        {
            tx.scroll_into_view(format!("list-{id:?}"));
        }
        changed
    }
    fn scenes(&self) -> Vec<crate::app::scenes::Scene> {
        vec![
            ("home", scene),
            ("home-1300", |a| {
                a.window[0] = 1300.0;
                scene(a);
            }),
            ("home-peek-running", |a| {
                scene(a);
                select(a, RowId::Conversation(ConvRef::new("notes-cli", 4)));
                super::peek::settle_scene(a);
            }),
            ("home-calm", calm_scene),
        ]
    }
}

pub fn scene(app: &mut App) {
    crate::app::scenes::on_fixture(app, Stage::Place(Place::Home));
    prepare_scene(app);
}

pub fn prepare_scene(app: &mut App) {
    let f = tau_client::Fixture::design();
    app.screens.home.away.base = Some(now(app).0 - 15_000_000);
    let now = now(app);
    app.screens.home.away.resume(now);
    // Scenes freeze the link after setup; cache previews so scripted cursor changes can read them.
    for r in rows(app) {
        if let RowId::Decision(d) = r.id {
            app.link.fetch_decision(d);
        }
    }
    // Fixture-local labels and observed settings are cache data, not extra production calls.
    for r in list::conversations(app) {
        if let RowId::Conversation(c) = r.id {
            app.link.want_label(c.clone());
            if f.value()["streams"]
                .get(format!("open {} #{}", c.project, c.id))
                .is_some()
            {
                app.link.show(tau_client::Slot::Peek, Some(c));
                crate::app::link::settle(app);
            }
        }
    }
    app.link.show(tau_client::Slot::Peek, None);
    crate::app::link::settle(app);
    reconcile(app);
    super::peek::settle_scene(app);
}

fn calm_scene(app: &mut App) {
    let mut f = tau_client::Fixture::design().value().clone();
    f["streams"]["inbox"]["first"] =
        serde_json::json!({"ops":[["r",{"forecasts":{},"notes-cli":{},"render-core":{}}]]});
    for p in ["notes-cli", "render-core"] {
        if let Some(rows) =
            f["calls"][format!("conversations {p}")]["result"]["items"].as_array_mut()
        {
            for r in rows {
                if r["state"] == "failed" {
                    r["state"] = "idle".into();
                }
            }
        }
    }
    app.link = tau_client::Link::fixture(tau_client::Fixture::parse(&f.to_string()).unwrap());
    scene(app);
    for r in list::faults(app) {
        if let RowId::Fault(p, t) = r.id {
            app.hidden.faults.insert((p, t));
        }
    }
    app.screens.home.away.visible = false;
    crate::app::link::refresh(app);
    reconcile(app);
    super::peek::settle_scene(app);
}
