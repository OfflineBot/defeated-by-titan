//! Entry point: read flags, have the app built, start it.
//!
//! Nothing more stands here, on purpose — the plugin list is the seam of the whole project and
//! lives in `lib.rs`, so that the tests build the **same** app (the reasoning is there).

use bevy::app::AppExit;
use defeated_by_titan::shared::cli::{has_display, Cli};

fn main() -> AppExit {
    let start = Cli::from_argv();

    // Without a graphics session a windowed start panics deep inside winit — a message that
    // looks like a bug in the game. Better to check first and say one sentence a human
    // understands (docs/environment.md).
    if start.wants_window() && !has_display() {
        eprintln!(
            "no WAYLAND_DISPLAY and no DISPLAY — this machine has no window.\n\
             Without a screen the game runs as a script run:\n\
             \n    cargo run -- --headless --script scripts/<file>.txt\n\
             \n(docs/umgebung.md, prompts/init.md §14. What is built here stays 🟨:\n\
             logic tested, pixels unseen.)"
        );
        return AppExit::error();
    }

    defeated_by_titan::app(start).run()
}
