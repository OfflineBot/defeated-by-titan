//! Einstiegspunkt: Flags lesen, App bauen lassen, starten.
//!
//! Mehr steht hier mit Absicht nicht — die Plugin-Liste ist die Naht des ganzen Projekts
//! und lebt in `lib.rs`, damit die Tests **dieselbe** App bauen (siehe dort).

use bevy::app::AppExit;
use defeated_by_titan::shared::start::{grafiksitzung_vorhanden, Start};

fn main() -> AppExit {
    let start = Start::aus_argv();

    // Ohne Grafiksitzung panikt ein Fensterstart tief in winit — eine Meldung, die aussieht
    // wie ein Bug im Spiel. Lieber vorher pruefen und einen Satz sagen, den man versteht
    // (docs/umgebung.md).
    if start.will_fenster() && !grafiksitzung_vorhanden() {
        eprintln!(
            "Kein WAYLAND_DISPLAY und kein DISPLAY — auf dieser Maschine gibt es kein Fenster.\n\
             Ohne Bildschirm laeuft das Spiel als Fahrt:\n\
             \n    cargo run -- --headless --script scripts/<datei>.txt\n\
             \n(docs/umgebung.md, prompts/init.md §14. Was hier gebaut wird, bleibt 🟨:\n\
             Logik getestet, Pixel ungesehen.)"
        );
        return AppExit::error();
    }

    defeated_by_titan::app(start).run()
}
