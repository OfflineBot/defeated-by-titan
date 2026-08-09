//! world — die Maps, Ankerpunkte, Kollision, der raeumliche Index.
//!
//! **Der raeumliche Index gehoert hierher** (Gitterzellen -> Entities, gepflegt ueber `Added`
//! und `RemovedComponents`, damit er nicht veralten kann). Hakeneinschlag, Klingentreffer,
//! Kollision und Titanen-Zielsuche gehen **alle** darueber: eine Stadt hat Tausende Haeuser,
//! und nichts darf alle Entities durchlaufen, um eine Frage ueber die zehn Meter vor der Nase
//! zu beantworten (`prompts/init.md` §11).
//!
//! **Stand:** die Naht steht — `RaumIndex` als Resource, der Pfleger in `SchrittSet::Raum`,
//! der Abmelde-Beobachter, und `karte::karte_bauen` als Stub gegen `assets/data/maps.ron`.
//! Gefuellt sind sie nicht: [`welt_aufbauen`] spawnt weiterhin die vier Platzhalter-Kloetze
//! und stirbt im selben Commit, in dem `karte_bauen` die Karte wirklich baut. **Nicht
//! vorher** — ohne Kloetze und ohne gefuellten Index faellt der Spieler 600 Ticks lang, und
//! `scripts/t007-erste-fahrt.txt` faellt mit ihm.
//!
//! Gespawnt werden **Daten**, keine Meshes: `render` macht daraus Dreiecke, ohne diese
//! Domaene zu kennen (`shared::Bauklotz`).

pub mod index;
pub mod karte;

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Ankerflaeche, Bauklotz, Boden, RaumIndex, SchrittSet};

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

/// Die Grundfarben der Bibel: gedeckt, Steingrau, Ziegelrot, Olivgruen, Sandbraun.
/// **Keine der drei Signalfarben** — die sind fuer Gameplay reserviert.
const STEINGRAU: [f32; 3] = [0.42, 0.43, 0.40];
const ZIEGELROT: [f32; 3] = [0.45, 0.26, 0.20];
const OLIVGRUEN: [f32; 3] = [0.29, 0.33, 0.20];
const SANDBRAUN: [f32; 3] = [0.55, 0.47, 0.33];

fn welt_aufbauen(mut commands: Commands) {
    commands.spawn((
        Name::new("boden"),
        Boden { hoehe_m: 0.0 },
        Bauklotz { groesse: Vec3::new(400.0, 0.2, 400.0), farbe: OLIVGRUEN },
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));

    // Ein paar Kloetze zum Danebenstehen und spaeter Anhaken. Hoehen so gewaehlt, dass ein
    // 1,8-m-Mensch sie als Massstab lesen kann.
    let kloetze = [
        (Vec3::new(-12.0, 4.0, -20.0), Vec3::new(8.0, 8.0, 8.0), ZIEGELROT),
        (Vec3::new(10.0, 6.0, -28.0), Vec3::new(10.0, 12.0, 10.0), STEINGRAU),
        (Vec3::new(0.0, 2.0, -12.0), Vec3::new(4.0, 4.0, 4.0), SANDBRAUN),
        (Vec3::new(24.0, 9.0, -40.0), Vec3::new(12.0, 18.0, 12.0), STEINGRAU),
    ];
    for (i, (pos, groesse, farbe)) in kloetze.into_iter().enumerate() {
        commands.spawn((
            Name::new(format!("klotz_{i}")),
            Bauklotz { groesse, farbe },
            // Alles, was steht, ist hakbar — bis es eine echte Map mit getunter
            // Ankerdichte gibt (F-003, docs/FRAGEN.md Q-010).
            Ankerflaeche,
            Transform::from_translation(pos),
        ));
    }
}
