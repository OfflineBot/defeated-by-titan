//! `F-002` Freies Zielen per Strahl — **Ebene 1 des Zielsystems.**
//!
//! `F-002` woertlich: „Raycast aus Kameraposition in Blickrichtung, Reichweite = Range-Stat.
//! Trefferpunkt wird gegen eine gueltige Ankerflaeche geprueft. **Diese Ebene bleibt IMMER
//! aktiv und ist niemals durch das Snap-System ersetzbar.**"
//!
//! ## Zwei Fallen, beide belegt
//!
//! 1. **`bevy::picking::MeshRayCast` ist verboten.** Es liegt durch `features = ["picking"]`
//!    griffbereit (`Cargo.toml`, expandiert in `bevy-0.19.0/Cargo.toml:2820-2825`) und
//!    iteriert ueber **alle** sichtbaren Mesh-Entities — der Quelltext sagt woertlich
//!    „Check all entities" (`bevy_picking-0.19.0/src/mesh_picking/ray_cast/mod.rs:224`,
//!    `culling_query.par_iter()` bei `:228`). Das ist genau der §11-Bruch, der in der
//!    Graubox funktioniert und bei tausend Haeusern auffaellt. Gefragt wird
//!    `RaumIndex::strahl`.
//! 2. **Der Strahl startet auf Augenhoehe, nicht am Spielerursprung.** Der Ursprung liegt
//!    zwischen den Fuessen (`docs/konventionen.md`); `spieler.augenhoehe_m` ist dieselbe
//!    Zahl, an die `render` die Kamera haengt. So kommt `vector` an den Augenpunkt, **ohne
//!    die Kamera zu kennen** — keine Query auf `Camera3d`, keine Kante.
//!
//! Und: **erst treffen, dann hakbar pruefen.** Der Index liefert den naechsten festen
//! Treffer samt Maske; hier wird entschieden, ob er hakbar ist. Ein Strahl, der ungetaggte
//! Koerper ueberspringt, haekt durch Waende — `F-023` verbietet das woertlich.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Intent, RaumIndex, Zielpunkt};

/// Schreibt [`Zielpunkt`] fuer jeden Spieler, einmal pro festem Schritt.
// gefuellt von Auftrag Z — F-002, F-003
pub fn zielen(
    _daten: Res<GameData>,
    _index: Res<RaumIndex>,
    mut _spieler: Query<(&Intent, &Transform, &mut Zielpunkt)>,
) {
}
