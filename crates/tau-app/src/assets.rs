//! What the app draws that isn't code: the two Iosevka families, Carbon icons, the client's own
//! small marks and the brand. Fonts are read at run time from `$TAU_ASSETS/fonts` (the Nix-built
//! asset directory); icons and the brand are small and built in.

use std::path::PathBuf;
use std::sync::LazyLock;

use rune::Icon;

use crate::theme::{PathMark, tokens};

/// The faces `nix/assets.nix` installs; Etoile is the sans slot, Iosevka the mono slot.
const FACES: [&str; 11] = [
    "IosevkaEtoile-Regular",
    "IosevkaEtoile-Italic",
    "IosevkaEtoile-SemiBold",
    "IosevkaEtoile-SemiBoldItalic",
    "IosevkaEtoile-Bold",
    "IosevkaEtoile-BoldItalic",
    "Iosevka-Regular",
    "Iosevka-Italic",
    "Iosevka-Medium",
    "Iosevka-SemiBold",
    "Iosevka-Bold",
];

/// Where the fonts are: `$TAU_ASSETS` when it is set at run time, else the directory the build
/// saw.
fn assets_dir() -> Option<PathBuf> {
    std::env::var_os("TAU_ASSETS")
        .map(PathBuf::from)
        .or_else(|| option_env!("TAU_ASSETS").map(PathBuf::from))
}

/// Every bundled face, read once and kept for the process's life (rune takes `&'static` bytes).
pub fn fonts() -> Result<&'static [&'static [u8]], String> {
    let dir = assets_dir()
        .ok_or("TAU_ASSETS is not set; run the app through `nix run .#run` or `nix run .#dev`")?;
    let mut faces: Vec<&'static [u8]> = Vec::with_capacity(FACES.len());
    for name in FACES {
        let path = dir.join("fonts").join(format!("{name}.ttf"));
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        faces.push(Box::leak(bytes.into_boxed_slice()));
    }
    Ok(Box::leak(faces.into_boxed_slice()))
}

/// The inside of an `<svg>` element, its title dropped.
fn svg_body(svg: &'static str) -> &'static str {
    let open = svg
        .find("<svg")
        .and_then(|i| svg[i..].find('>').map(|j| i + j + 1))
        .unwrap_or(0);
    let close = svg.rfind("</svg>").unwrap_or(svg.len());
    let body = &svg[open..close];
    match (body.find("<title>"), body.find("</title>")) {
        (Some(a), Some(b)) if a == 0 || body[..a].trim().is_empty() => {
            body[b + "</title>".len()..].trim()
        }
        _ => body.trim(),
    }
}

fn view_box(svg: &str) -> f32 {
    svg.split("viewBox=\"")
        .nth(1)
        .and_then(|v| v.split('"').next())
        .and_then(|v| v.split_whitespace().nth(2))
        .and_then(|w| w.parse().ok())
        .unwrap_or(32.0)
}

fn svg_icon(name: &'static str, svg: &'static str) -> Icon {
    Icon {
        name,
        view: view_box(svg),
        body: svg_body(svg),
    }
}

fn path_icon(name: &'static str, mark: &PathMark, evenodd: bool) -> Icon {
    let view = mark
        .view_box
        .split_whitespace()
        .nth(2)
        .and_then(|w| w.parse().ok())
        .unwrap_or(16.0);
    let rule = if evenodd {
        " fill-rule=\"evenodd\""
    } else {
        ""
    };
    let body: &'static str =
        Box::leak(format!("<path{rule} d=\"{}\"/>", mark.path).into_boxed_str());
    Icon { name, view, body }
}

macro_rules! carbon {
    ($($name:ident = $file:literal),* $(,)?) => {
        /// Carbon icons (`@carbon/icons` 11.90.0, 32 px drawings), tinted where they're used.
        pub mod carbon {
            use super::*;
            $(pub static $name: LazyLock<Icon> =
                LazyLock::new(|| svg_icon($file, include_str!(concat!(env!("TAU_ASSETS"), "/icons/", $file, ".svg"))));)*
        }
    };
}

carbon! {
    ADD = "add",
    ARROW_DOWN = "arrow--down",
    BOT = "bot",
    CARET_RIGHT = "caret--right",
    CHAT = "chat",
    CHECKBOX = "checkbox",
    CHECKBOX_CHECKED = "checkbox--checked",
    CHECKMARK = "checkmark",
    CHEVRON_DOWN = "chevron--down",
    CHEVRON_RIGHT = "chevron--right",
    EDIT = "edit",
    FOLDER = "folder",
    HELP = "help",
    HOME = "home",
    HOURGLASS = "hourglass",
    SEARCH = "search",
    SETTINGS = "settings",
    SIDE_PANEL_CLOSE = "side-panel--close",
    SIDE_PANEL_OPEN = "side-panel--open",
    SUBTRACT = "subtract",
    TIME = "time",
    USER = "user",
    WARNING_ALT = "warning--alt",
}

/// The client's own small drawings (`tokens.json` `tau.icon`), and the two the prototype draws
/// that the tokens don't list: the outline diamond (a decision) and the ring (offline).
pub mod marks {
    use super::*;

    pub static DIAMOND: LazyLock<Icon> =
        LazyLock::new(|| path_icon("diamond", &tokens().marks.decision, false));
    pub static FAILED: LazyLock<Icon> =
        LazyLock::new(|| path_icon("failed", &tokens().marks.failed, false));
    pub static RUNNING: LazyLock<Icon> =
        LazyLock::new(|| path_icon("running", &tokens().marks.running, false));
    pub static KEY_SPACE: LazyLock<Icon> =
        LazyLock::new(|| path_icon("key-space", &tokens().marks.key_space, false));
    pub static KEY_RETURN: LazyLock<Icon> =
        LazyLock::new(|| path_icon("key-return", &tokens().marks.key_return, false));
    pub static DIAMOND_OUTLINE: LazyLock<Icon> = LazyLock::new(|| Icon {
        name: "diamond-outline",
        view: 16.0,
        body: r#"<path fill-rule="evenodd" d="M8 .7 15.3 8 8 15.3.7 8zm0 2.6L3.3 8 8 12.7 12.7 8z"/>"#,
    });
    pub static RING: LazyLock<Icon> = LazyLock::new(|| Icon {
        name: "ring",
        view: 16.0,
        body: r#"<path fill-rule="evenodd" d="M8 2.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zm0 1.5a4 4 0 1 1 0 8 4 4 0 0 1 0-8z"/>"#,
    });
}

/// The brand (`design/brand/README.md`), drawn as it is and never tinted.
pub mod brand {
    use super::*;

    /// The sidebar's avatar, 24 px, the same in every variant.
    pub static AVATAR: LazyLock<Icon> = LazyLock::new(|| {
        svg_icon(
            "tau",
            include_str!("../../../design/brand/tau-icon-small-coral.svg"),
        )
    });
    /// The start page's and Connect's wordmark, in its own colours, Ember and Ember Soft. Its
    /// drawing is wider than it is tall, so its square view box leaves the bottom empty: draw it
    /// with `crate::ui::wordmark`.
    pub static WORDMARK_ON_DARK: LazyLock<Icon> = LazyLock::new(|| {
        svg_icon(
            "tau-wordmark",
            include_str!("../../../design/brand/tau-wordmark-on-dark-coral.svg"),
        )
    });
    /// The wordmark in Ember Light.
    pub static WORDMARK_ON_LIGHT: LazyLock<Icon> = LazyLock::new(|| {
        svg_icon(
            "tau-wordmark-light",
            include_str!("../../../design/brand/tau-wordmark-on-light-coral.svg"),
        )
    });
    /// The wordmark's drawing, width over height (its view box is 207.787 × 73.9).
    pub const WORDMARK_ASPECT: f32 = 207.787 / 73.9;
}
