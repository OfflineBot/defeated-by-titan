//! world — die Maps, Ankerpunkte, Kollision, der raeumliche Index.
//!
//! **Der raeumliche Index gehoert hierher** (Gitterzellen -> Entities, gepflegt ueber `Added`
//! und `RemovedComponents`, damit er nicht veralten kann). Hakeneinschlag, Klingentreffer,
//! Kollision und Titanen-Zielsuche gehen **alle** darueber: eine Stadt hat Tausende Haeuser,
//! und nichts darf alle Entities durchlaufen, um eine Frage ueber die zehn Meter vor der Nase
//! zu beantworten (`prompts/init.md` §11).
//!
//! **Stand:** [`karte::karte_bauen`] baut die Stadt seit 2026-08-09 wirklich — aus
//! `assets/data/maps.ron`, gesetzte Kloetze 1:1 und das Raster deterministisch aus dem Seed
//! der Karte. Die vier hart verdrahteten Platzhalterkloetze in [`welt_aufbauen`] sind damit
//! weg; sie stehen als die ersten Eintraege in der Datei und sind dort **verhaltensgleich**
//! nachpruefbar (`tests/world.rs`).
//!
//! ⚠️ Was in [`welt_aufbauen`] bleibt, ist **nur noch der Bodenmarker**. Die sichtbare
//! Bodenplatte kommt jetzt aus der Datei (`maps.ron: kloetze[0]`) — zweimal gespawnt waere
//! sie ein Flimmern zweier deckungsgleicher Flaechen. [`Boden`] selbst hat heute **keinen
//! Leser mehr**; er stirbt zusammen mit dem harten `boden_y = 0.0` in `src/player/mod.rs`,
//! sobald `player::koerper::schritt` gefuellt ist — nicht vorher, sonst faellt der Spieler
//! 600 Ticks lang und `scripts/t007-erste-fahrt.txt` faellt mit ihm.
//!
//! Gespawnt werden **Daten**, keine Meshes: `render` macht daraus Dreiecke, ohne diese
//! Domaene zu kennen (`shared::Bauklotz`).

pub mod index;
pub mod karte;

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Boden, RaumIndex, SchrittSet};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // Der Index braucht drei Zahlen aus der RON und **kein `Default`**: `zelle_m = 0.0`
        // waere eine Division durch null im DDA, und `init_resource` ist die naheliegendste
        // Zeile der Welt. Deshalb nur `insert_resource(RaumIndex::neu(..))`.
        let w = &app.world().resource::<GameData>().spiel.welt;
        let raum = RaumIndex::neu(w.zelle_m, w.halbe_ausdehnung_m, w.grosskoerper_zellen);
        app.insert_resource(raum);

        // Der Beobachter statt `RemovedComponents` — Begruendung in `world::index`.
        app.add_observer(index::koerper_abmelden);

        app.add_systems(Startup, (welt_aufbauen, karte::karte_bauen))
            .add_systems(FixedUpdate, index::index_pflegen.in_set(SchrittSet::Raum));
    }
}

/// Der Bodenmarker — **und sonst nichts mehr.**
///
/// Die Kloetze standen bis 2026-08-09 hier im Code; sie stehen jetzt in
/// `assets/data/maps.ron` und werden von [`karte::karte_bauen`] gebaut. Auch die sichtbare
/// Bodenplatte kommt von dort: dieser Marker traegt **keinen** [`Bauklotz`](crate::shared::Bauklotz)
/// mehr, sonst laegen zwei deckungsgleiche 400-m-Flaechen uebereinander.
fn welt_aufbauen(mut commands: Commands) {
    commands.spawn((
        Name::new("boden"),
        Boden { hoehe_m: 0.0 },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}
