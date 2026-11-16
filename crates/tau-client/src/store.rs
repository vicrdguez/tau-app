//! Device-local state (`client.md` §8): `client.toml`, a directory per server URL for the outbox, the label cache
//! and whatever the UI keeps, and the token, kept in the Keychain by default on macOS.

use crate::wire::Token;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// `client.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
}

/// Files under one root (`~/.config/tau`), or nowhere at all for a demo.
#[derive(Clone, Debug)]
pub struct Store {
    root: Option<PathBuf>,
    server: Option<PathBuf>,
}

/// Where the token is kept.
#[derive(Clone)]
pub enum Secrets {
    /// The macOS Keychain, item `tau <server url>`.
    Keychain,
    /// A 0600 file per server under this directory, when explicitly selected or without a Keychain.
    Files(PathBuf),
    /// Held by this process only: tests and demos.
    Memory(Arc<Mutex<HashMap<String, Token>>>),
}

impl Store {
    pub fn at(root: PathBuf) -> Store {
        Store {
            root: Some(root),
            server: None,
        }
    }

    /// Reads nothing and writes nothing.
    pub fn none() -> Store {
        Store {
            root: None,
            server: None,
        }
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn load_device(&self, name: &str) -> io::Result<Option<String>> {
        let Some(root) = self.root() else {
            return Ok(None);
        };
        match fs::read_to_string(root.join(name)) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn save_device(&self, name: &str, text: &str) -> io::Result<()> {
        let Some(root) = self.root() else {
            return Ok(());
        };
        write_whole(&root.join(name), text.as_bytes())
    }

    pub fn client(&self) -> ClientConfig {
        let Some(root) = &self.root else {
            return ClientConfig::default();
        };
        fs::read_to_string(root.join("client.toml"))
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save_client(&self, config: &ClientConfig) -> io::Result<()> {
        let Some(root) = &self.root else {
            return Ok(());
        };
        let text = toml::to_string(config).map_err(io::Error::other)?;
        write_whole(&root.join("client.toml"), text.as_bytes())
    }

    /// Picks the directory of one server's state.
    pub fn use_server(&mut self, url: &str) {
        self.server = self
            .root
            .as_ref()
            .map(|root| root.join("servers").join(slug(url)));
    }

    /// Reads a file of the current server's state; `None` when absent or unreadable.
    pub fn load<T: serde::de::DeserializeOwned>(&self, name: &str) -> Option<T> {
        let text = fs::read(self.server.as_ref()?.join(name)).ok()?;
        serde_json::from_slice(&text).ok()
    }

    /// Writes a file of the current server's state, whole or not at all.
    pub fn save<T: Serialize>(&self, name: &str, value: &T) -> io::Result<()> {
        let Some(dir) = &self.server else {
            return Ok(());
        };
        write_whole(&dir.join(name), &serde_json::to_vec_pretty(value)?)
    }

    /// `:forget-server`: the server's state goes, and `client.toml` forgets the server and the device.
    pub fn forget_server(&mut self) -> io::Result<()> {
        if let Some(dir) = self.server.take()
            && dir.exists()
        {
            fs::remove_dir_all(dir)?;
        }
        self.save_client(&ClientConfig::default())
    }
}

/// A server URL as a directory name that no other URL maps to.
fn slug(url: &str) -> String {
    url.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'-' | b'_' => (b as char).to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

/// Writes beside the file, then renames over it, so a crash leaves the old file or the new one.
fn write_whole(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let partial = path.with_extension("partial");
    fs::write(&partial, bytes)?;
    fs::rename(partial, path)
}

impl Secrets {
    /// The Keychain on macOS; elsewhere 0600 files under `dir`.
    pub fn standard(dir: PathBuf) -> Secrets {
        if cfg!(target_os = "macos") {
            Secrets::Keychain
        } else {
            Secrets::Files(dir)
        }
    }

    pub fn memory() -> Secrets {
        Secrets::Memory(Arc::default())
    }

    pub fn load(&self, url: &str) -> Option<Token> {
        match self {
            Secrets::Keychain => keychain::load(url),
            Secrets::Files(dir) => fs::read_to_string(dir.join(slug(url)))
                .ok()
                .map(|t| Token::new(t.trim())),
            Secrets::Memory(map) => map
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(url)
                .cloned(),
        }
    }

    pub fn save(&self, url: &str, token: &Token) -> io::Result<()> {
        match self {
            Secrets::Keychain => keychain::save(url, token),
            Secrets::Files(dir) => {
                use std::io::Write;
                fs::create_dir_all(dir)?;
                let mut options = fs::OpenOptions::new();
                options.write(true).create(true).truncate(true);
                #[cfg(unix)]
                std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
                options
                    .open(dir.join(slug(url)))?
                    .write_all(token.expose().as_bytes())
            }
            Secrets::Memory(map) => {
                map.lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(url.to_owned(), token.clone());
                Ok(())
            }
        }
    }

    pub fn delete(&self, url: &str) -> io::Result<()> {
        match self {
            Secrets::Keychain => keychain::delete(url),
            Secrets::Files(dir) => match fs::remove_file(dir.join(slug(url))) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            },
            Secrets::Memory(map) => {
                map.lock().unwrap_or_else(|e| e.into_inner()).remove(url);
                Ok(())
            }
        }
    }
}

/// The macOS Keychain: one generic password per server, service `tau <server url>`.
#[cfg(target_os = "macos")]
mod keychain {
    use crate::wire::Token;
    use security_framework::passwords::{
        delete_generic_password, get_generic_password, set_generic_password,
    };
    use std::io;

    const ACCOUNT: &str = "device token";
    /// `errSecItemNotFound`.
    const NOT_FOUND: i32 = -25300;

    fn service(url: &str) -> String {
        format!("tau {url}")
    }

    pub fn load(url: &str) -> Option<Token> {
        let bytes = get_generic_password(&service(url), ACCOUNT).ok()?;
        String::from_utf8(bytes).ok().map(Token::new)
    }

    pub fn save(url: &str, token: &Token) -> io::Result<()> {
        set_generic_password(&service(url), ACCOUNT, token.expose().as_bytes())
            .map_err(io::Error::other)
    }

    pub fn delete(url: &str) -> io::Result<()> {
        match delete_generic_password(&service(url), ACCOUNT) {
            Err(e) if e.code() != NOT_FOUND => Err(io::Error::other(e)),
            _ => Ok(()),
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod keychain {
    use crate::wire::Token;
    use std::io;

    fn none() -> io::Error {
        io::Error::new(io::ErrorKind::Unsupported, "this system has no Keychain")
    }

    pub fn load(_: &str) -> Option<Token> {
        None
    }

    pub fn save(_: &str, _: &Token) -> io::Result<()> {
        Err(none())
    }

    pub fn delete(_: &str) -> io::Result<()> {
        Err(none())
    }
}
