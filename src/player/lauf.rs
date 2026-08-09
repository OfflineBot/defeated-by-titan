//! Bodenlauf und Luftsteuerung als **Beitrag**, nicht als fertige Bewegung.
//!
//! Schreibt genau ein Component ([`AntriebLauf`], in m/s²) und sonst nichts. Drei getrennte
//! Antriebs-Components statt eines mit drei Feldern: damit gilt „genau ein Schreiber je
//! Component" woertlich und per `grep` pruefbar, und drei Systeme mit disjunktem `&mut`
//! laufen in Bevy echt parallel, statt sich zu serialisieren.
//!
//! **Zuweisung, nie `+=`.** Ein Beitragender, der nichts will, schreibt `Vec3::ZERO`. Damit
//! gibt es kein Leer-System, keine Reihenfolgeabhaengigkeit innerhalb von
//! `SchrittSet::Antrieb` und keinen Zustand, der einen Tick zu lange lebt.
//!
//! Liest [`Bewegungszustand`] vom **Ende des vorigen Ticks** — ein Tick Verzug ist
//! deterministisch und billiger als eine Reihenfolgeabhaengigkeit zum Integrator.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{AntriebLauf, Bewegungszustand, Intent};

/// Schreibt [`AntriebLauf`] aus `Intent.bewegen()` und `spieler.laufen_m_s`.
// gefuellt von Auftrag V — Stufe 2
pub fn lauf(
    _daten: Res<GameData>,
    mut _spieler: Query<(&Intent, &Bewegungszustand, &mut AntriebLauf)>,
) {
}
