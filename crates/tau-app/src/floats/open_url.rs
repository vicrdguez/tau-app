//! `gx`: an external link in content opens only after Confirm has shown its whole URL
//! (`grammar.md` §4, `visual-language.md` §3). Characters that draw nothing or reorder text show
//! as their `U+` escapes, so the URL read is the URL opened.

use crate::app::App;
use crate::content::invisible;
use crate::floats::{Confirm, Float};

/// Opens Confirm for `url`, or says why there is nothing to open.
pub fn confirm(app: &mut App, url: Option<String>) {
    let Some(url) = url else {
        app.say("no link here");
        return;
    };
    if !external(&url) {
        app.say("not a web or mail link");
        return;
    }
    app.open_float(Float::Confirm(Confirm {
        sentence: format!("Open this link outside tau?\n{}", visible(&url)),
        yes: "opens",
        on_yes: Some(Box::new(move |a, tx| {
            if let Err(e) = tx.open_url(&url) {
                a.say(e.to_string());
            }
        })),
    }));
}

fn external(url: &str) -> bool {
    let lower = url.trim_start().to_ascii_lowercase();
    ["https://", "http://", "mailto:"]
        .iter()
        .any(|s| lower.starts_with(s))
}

fn visible(url: &str) -> String {
    url.chars()
        .map(|c| {
            if invisible(c) {
                format!("U+{:04X}", c as u32)
            } else {
                c.to_string()
            }
        })
        .collect()
}
