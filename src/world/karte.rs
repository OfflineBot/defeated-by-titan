//! Die Stadt aus `assets/data/maps.ron` — **Daten und ein Seed, keine 200 Zeilen Rust.**
//!
//! Gebaut wird aus zwei Quellen (`docs/schnittstelle.md`, „Die Stadt"):
//! 1. `kloetze` — ausdruecklich gesetzte Quader, 1:1 aus der Datei.
//! 2. `raster` — deterministisch erzeugte Bloecke aus `seed` ueber
//!    [`Wuerfel`](crate::shared::Wuerfel). Derselbe Seed ergibt dieselbe Stadt, auf jedem
//!    Rechner und in jedem Rollback; `rand::random()` waere hier ein Desync.
//!
//! Jede Entity bekommt [`Bauklotz`] (das sieht `render`), [`Koerper`] (das sieht der Index)
//! und bei `hakbar` zusaetzlich [`Ankerflaeche`]. **Ein Schreiber fuer alle drei**, damit
//! Renderform und Kollisionsform nicht auseinanderlaufen koennen.
//!
//! **Kein Klotz wird gedreht.** Ein achsenparalleler Quader ist exakt seine AABB; eine
//! gedrehte `Cuboid` liefert die umschliessende, zu grosse Huelle
//! (`bevy_math-0.19.0/src/bounding/bounded3d/primitive_impls.rs:100-115`), und der Haken
//! faengt sichtbar in der Luft. Das ist eine bewusst aufgeschobene Einschraenkung
//! (`docs/ROADMAP.md`), keine vergessene.

use bevy::prelude::*;

use crate::data::GameData;

/// Baut die Karte aus `maps.ron: aktuell` beim `Startup`.
///
/// Ersetzt die bis heute in `world/mod.rs` hart verdrahteten Kloetze. Die ersten fuenf
/// Eintraege in `maps.ron` sind genau diese Kloetze — damit der Umbau **verhaltensgleich**
/// nachweisbar ist und nicht „sieht auch gut aus".
// gefuellt von Auftrag W — F-003, Stufe 2
pub fn karte_bauen(mut _commands: Commands, _daten: Res<GameData>) {}
