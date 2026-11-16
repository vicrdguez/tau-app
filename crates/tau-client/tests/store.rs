use std::path::{Path, PathBuf};
use std::sync::Arc;
use tau_client::clock::SystemClock;
use tau_client::fixture::FixtureTransport;
use tau_client::store::{ClientConfig, Secrets, Store};
use tau_client::wire::InputMode;
use tau_client::{ConvRef, Fixture, Link, LinkConfig, LinkPhase, Write};

fn settle(link: &mut Link) {
    for _ in 0..20 {
        if !link.pump() {
            return;
        }
    }
    panic!("the link never settled");
}

fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(files(&path));
        } else {
            out.push(path);
        }
    }
    out
}

/// Pairs a fresh link with the fixture's server, as Connect does, and sends one input.
fn paired_by_hand(dir: &Path, secrets: Secrets) -> Link {
    let fixture = Fixture::design();
    let config = LinkConfig {
        dir: Some(dir.to_path_buf()),
        secrets,
        clock: Arc::new(SystemClock),
    };
    let mut link = Link::with_transport(config, Arc::new(FixtureTransport::new(fixture.clone())));
    assert_eq!(link.status().phase, LinkPhase::NotPaired);
    link.pair(&fixture.url(), "K7QM-3XRT");
    settle(&mut link);
    assert_eq!(link.status().phase, LinkPhase::Live);
    link.write(Write::Input {
        conv: ConvRef::new("notes-cli", 4),
        text: "go".into(),
        mode: InputMode::Send,
    })
    .unwrap();
    link
}

#[test]
fn state_lives_under_the_root() {
    let dir = tempfile::tempdir().unwrap();
    let _link = paired_by_hand(dir.path(), Secrets::memory());
    let client: ClientConfig =
        toml::from_str(&std::fs::read_to_string(dir.path().join("client.toml")).unwrap()).unwrap();
    assert_eq!(
        client.server.as_deref(),
        Some("https://tau.tail4e2f.ts.net")
    );
    assert_eq!(client.device.as_deref(), Some("laptop"));
    let all = files(dir.path());
    assert!(all.iter().any(|p| p.ends_with("outbox.json")), "{all:?}");
    assert!(all.iter().all(|p| p.starts_with(dir.path())));
}

#[test]
fn the_token_never_touches_disk() {
    let dir = tempfile::tempdir().unwrap();
    let secrets = Secrets::memory();
    let _link = paired_by_hand(dir.path(), secrets.clone());
    let token = Fixture::design().token();
    assert_eq!(
        secrets.load("https://tau.tail4e2f.ts.net"),
        Some(token.clone())
    );
    let secret = token.expose().rsplit('.').next().unwrap().to_owned();
    for path in files(dir.path()) {
        let text = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
        assert!(
            !text.contains(&secret),
            "{} holds the token",
            path.display()
        );
    }
}

#[test]
fn forget_server_deletes_everything() {
    let dir = tempfile::tempdir().unwrap();
    let secrets = Secrets::memory();
    let mut link = paired_by_hand(dir.path(), secrets.clone());
    link.forget_server();
    settle(&mut link);
    assert_eq!(link.status().phase, LinkPhase::NotPaired);
    assert_eq!(secrets.load("https://tau.tail4e2f.ts.net"), None);
    assert!(link.outbox().entries().is_empty());
    let client = Store::at(dir.path().to_path_buf()).client();
    assert_eq!((client.server, client.device), (None, None));
    assert!(!files(dir.path()).iter().any(|p| p.ends_with("outbox.json")));
}

#[test]
fn the_ui_keeps_its_own_state_beside_the_link() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::at(dir.path().to_path_buf());
    store.use_server("https://tau.tail4e2f.ts.net");
    store.save("ui.json", &vec!["notes-cli/4"]).unwrap();
    assert_eq!(
        store.load::<Vec<String>>("ui.json"),
        Some(vec!["notes-cli/4".to_owned()])
    );
    assert_eq!(store.load::<Vec<String>>("absent.json"), None);
    let mut other = Store::at(dir.path().to_path_buf());
    other.use_server("http://127.0.0.1:6283");
    assert_eq!(
        other.load::<Vec<String>>("ui.json"),
        None,
        "each server has its own state"
    );
    let none = Store::none();
    none.save("ui.json", &1).unwrap();
    assert_eq!(none.load::<i32>("ui.json"), None);
}

#[test]
fn a_restart_restores_the_outbox_unsent() {
    let dir = tempfile::tempdir().unwrap();
    let secrets = Secrets::memory();
    let link = paired_by_hand(dir.path(), secrets.clone());
    let id = link.outbox().entries().last().unwrap().id;
    drop(link);
    let config = LinkConfig {
        dir: Some(dir.path().to_path_buf()),
        secrets,
        clock: Arc::new(SystemClock),
    };
    let mut again =
        Link::with_transport(config, Arc::new(FixtureTransport::new(Fixture::design())));
    settle(&mut again);
    let entry = again.outbox().get(id).expect("restored");
    assert!(entry.restored);
    assert_eq!(
        again.status().phase,
        LinkPhase::Live,
        "paired from client.toml and the stored token"
    );
}

#[cfg(unix)]
#[test]
fn token_files_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let secrets = Secrets::Files(dir.path().join("tokens"));
    let token = Fixture::design().token();
    secrets.save("http://127.0.0.1:6283", &token).unwrap();
    assert_eq!(secrets.load("http://127.0.0.1:6283"), Some(token));
    for path in files(dir.path()) {
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    secrets.delete("http://127.0.0.1:6283").unwrap();
    assert_eq!(secrets.load("http://127.0.0.1:6283"), None);
}
