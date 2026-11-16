//! The tau desktop app: one native window onto one tau server. `--demo` runs it on the design's
//! fixture instead of a server.

use tau_app::app::{self, App};

fn main() {
    let mut demo = false;
    let mut live = false;
    for arg in rune::extra_args() {
        match arg.as_str() {
            "--demo" => demo = true,
            "--live" => live = true,
            other => {
                eprintln!("tau: unknown option {other}; use --demo or --live");
                std::process::exit(2);
            }
        }
    }
    if demo && live {
        eprintln!("tau: --demo and --live cannot be combined");
        std::process::exit(2);
    }
    rune::prewarm_gpu();
    let fonts = match tau_app::assets::fonts() {
        Ok(fonts) => fonts,
        Err(e) => {
            eprintln!("tau: the fonts can't be read: {e}");
            std::process::exit(1);
        }
    };
    let [x, y] = tau_app::theme::tokens().layout.sidebar.traffic_lights;
    rune::App::new(app::spec(fonts))
        .init(move || {
            App::launch(if demo || (rune::is_headless() && !live) {
                app::link::demo()
            } else {
                app::link::standard()
            })
        })
        .on_focus(app::link::on_focus)
        .on_close(app::link::on_close)
        .on_quit(app::link::on_close)
        // What the prototype draws: `-webkit-font-smoothing: antialiased`.
        .font_smoothing(rune::FontSmoothing::Off)
        .chrome(rune::Chrome {
            full_size_content: true,
            traffic_lights: Some((x, y)),
            drag_regions: true,
        })
        .run();
}
