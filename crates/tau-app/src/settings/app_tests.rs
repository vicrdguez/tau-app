//! How settings could fail at the app, on a device whose files are in a temp dir:
//! - launch ignores settings.toml, or applies only some of it;
//! - scenes and tests read the developer's own file (the fixture link has no root);
//! - one bad value takes the good ones down, or its problem never reaches `:messages`;
//! - a file that can't be read stops the launch;
//! - `␣ht` switches the theme but forgets it at the next launch, or loses the user's comments;
//! - a change writes over a file that isn't TOML, or doesn't apply when it can't be saved, or
//!   says nothing about it;
//! - a save writes back what was read at launch, losing a hand edit made since;
//! - a pending key sequence typed on the old keys survives a change of keys.

use super::*;
use crate::app::{input, link, testing};
use crate::frame::Place;
use crate::keys::parse_seq;
use crate::theme::Variant;
use rune::Tx;
use tau_client::Fixture;

fn device(settings: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    if let Some(text) = settings {
        std::fs::write(dir.path().join(FILE), text).unwrap();
    }
    dir
}

fn launch(dir: &tempfile::TempDir) -> App {
    testing::launch(dir.path(), &Fixture::design().url())
}

fn on_disk(dir: &tempfile::TempDir) -> String {
    std::fs::read_to_string(dir.path().join(FILE)).unwrap()
}

fn key(a: &mut App, seq: &str) {
    for k in parse_seq(seq).unwrap() {
        input::press(a, k, false, &mut Tx::default());
    }
}

/// `␣ht` from Home, where the leader is live.
fn next_theme(a: &mut App) {
    a.show(crate::app::Stage::Place(Place::Home));
    key(a, "<leader>ht");
}

#[test]
fn launch_applies_every_setting_in_the_file() {
    let dir = device(Some(
        "theme = \"ember-light\"\nreading_width = 1000\nmotion = false\n\n[keys.\"start.workflow\"]\n\"<M-w>\" = \"<M-m>\"\n",
    ));
    let a = launch(&dir);
    let keymap = Keymap::design()
        .rebind(
            "start.workflow",
            &parse_seq("<M-w>").unwrap(),
            &parse_seq("<M-m>").unwrap(),
        )
        .unwrap();
    assert_eq!(
        a.settings,
        Settings {
            theme: Variant::EmberLight,
            reading: Measure::new(1000.0).unwrap(),
            motion: false,
            keymap,
        }
    );
    assert_eq!(a.theme().read.measure, 1000.0);
    assert!(!a.theme().motion.on);
    assert!(
        a.messages.iter().all(|m| !m.contains(FILE)),
        "{:?}",
        a.messages
    );
}

#[test]
fn no_file_and_the_fixture_link_read_nothing() {
    let dir = device(None);
    let a = launch(&dir);
    assert_eq!(a.settings, Settings::default());
    assert!(a.messages.iter().all(|m| !m.contains(FILE)));
    assert!(link::demo().store().root().is_none());
    assert_eq!(App::default().settings, Settings::default());
    assert_eq!(App::launch(link::demo()).settings, Settings::default());
}

#[test]
fn a_bad_value_is_reported_and_the_good_ones_apply() {
    let dir = device(Some(
        "theme = \"dark\"\nreading_width = 1000\nmotion = false\n",
    ));
    let a = launch(&dir);
    assert_eq!(
        (a.settings.theme, a.settings.reading.px(), a.settings.motion),
        (Variant::Ember, 1000.0, false)
    );
    assert!(
        a.messages
            .contains(&r#"theme "dark" isn't ember, ember-soft or ember-light"#.to_string()),
        "{:?}",
        a.messages
    );
    assert_eq!(
        a.echo.as_deref(),
        Some("settings.toml: 1 problem · :messages")
    );
}

#[test]
fn a_file_that_cannot_be_read_is_every_default_and_says_why() {
    let dir = device(None);
    std::fs::create_dir(dir.path().join(FILE)).unwrap();
    let mut a = launch(&dir);
    assert_eq!(a.settings, Settings::default());
    assert!(
        a.messages
            .iter()
            .any(|m| m.starts_with("settings.toml can't be read (")),
        "{:?}",
        a.messages
    );
    next_theme(&mut a);
    assert_eq!(a.settings.theme, Variant::EmberSoft, "applied all the same");
    assert!(dir.path().join(FILE).is_dir());
}

#[test]
fn the_theme_key_saves_the_theme_and_keeps_the_comments() {
    let dir = device(Some("# mine\ntheme = \"ember\"  # the dark one\n"));
    let mut a = launch(&dir);
    next_theme(&mut a);
    assert_eq!(a.settings.theme, Variant::EmberSoft);
    assert_eq!(
        on_disk(&dir),
        "# mine\ntheme = \"ember-soft\"  # the dark one\n"
    );
    assert_eq!(launch(&dir).settings.theme, Variant::EmberSoft);
}

#[cfg(unix)]
#[test]
fn a_read_only_link_target_reports_that_the_change_is_not_saved() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let dir = device(None);
    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join(FILE);
    std::fs::write(&target, "theme = \"ember\"\n").unwrap();
    symlink(&target, dir.path().join(FILE)).unwrap();
    let mut a = launch(&dir);
    std::fs::set_permissions(target_dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    next_theme(&mut a);
    std::fs::set_permissions(target_dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(a.settings.theme, Variant::EmberSoft);
    let echo = a.echo.as_deref().unwrap();
    assert!(
        echo.contains("can't be written") && echo.contains("not saved"),
        "{echo}"
    );
    assert_eq!(std::fs::read_link(dir.path().join(FILE)).unwrap(), target);
    assert_eq!(on_disk(&dir), "theme = \"ember\"\n");
}

#[test]
fn the_theme_key_starts_a_file_where_there_is_none() {
    let dir = device(None);
    let mut a = launch(&dir);
    next_theme(&mut a);
    assert!(on_disk(&dir).ends_with("theme = \"ember-soft\"\n"));
    assert_eq!(launch(&dir).settings.theme, Variant::EmberSoft);
}

#[test]
fn a_change_never_writes_over_a_file_that_is_not_toml() {
    let text = "theme = \"ember\"\nreading_width 760\n";
    let dir = device(Some(text));
    let mut a = launch(&dir);
    next_theme(&mut a);
    assert_eq!(a.settings.theme, Variant::EmberSoft, "applied all the same");
    assert_eq!(on_disk(&dir), text);
    let echo = a.echo.clone().unwrap_or_default();
    assert!(
        echo.starts_with("settings.toml line 2: ") && echo.contains("not saved"),
        "{echo}"
    );
}

#[test]
fn a_save_that_would_load_different_settings_keeps_the_file_and_reports_unsaved() {
    let dir = device(Some("theme = \"ember\"\n"));
    let mut a = launch(&dir);
    std::fs::write(
        dir.path().join(FILE),
        "theme = \"ember\"\nreading_width = 920\n",
    )
    .unwrap();
    next_theme(&mut a);
    assert_eq!(on_disk(&dir), "theme = \"ember\"\nreading_width = 920\n");
    assert_eq!(a.settings.theme, Variant::EmberSoft);
    assert_eq!(a.settings.reading.px(), 840.0);
    let echo = a.echo.as_deref().unwrap();
    assert!(
        echo.contains("not saved") && echo.contains("until you quit"),
        "{echo}"
    );
}

#[test]
fn a_change_of_keys_drops_a_sequence_typed_on_the_old_ones() {
    let dir = device(None);
    let mut a = launch(&dir);
    a.show(crate::app::Stage::Place(Place::Home));
    key(&mut a, "<leader>");
    assert!(!a.keys.is_idle());
    let design = parse_seq("<leader>ht").unwrap();
    let to = parse_seq("<leader>hy").unwrap();
    let new = Settings {
        keymap: a
            .settings
            .keymap
            .rebind("help.theme", &design, &to)
            .unwrap(),
        ..a.settings
    };
    change(
        &mut a,
        new,
        Edit::Key {
            command: "help.theme",
            design,
        },
    );
    assert!(a.keys.is_idle());
    assert!(on_disk(&dir).contains("\"<leader>ht\" = \"<leader>hy\""));
}
