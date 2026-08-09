//! Die Gasleiste — **die einzige Stelle, an der `F-018` sichtbar wird.**
//!
//! Ohne sie ist der Gasstand nirgends im Bild, und `F-018` bleibt 🟨 („Logik getestet, Pixel
//! ungesehen"), egal wie gruen der Test ist. Eine Zahl im Terminal ist kein Bild.
//!
//! Blau/Zyan, weil Zyan die reservierte Farbe fuer Gas und Vector Gear ist
//! (`docs/konventionen.md`); `F-018` nennt sie woertlich „Blaue Leiste".
//!
//! Liest den Zustand **des lokalen Spielers** ueber [`LocalPlayer`](crate::shared::LocalPlayer)
//! — kein `.single()`, denn jeder Spieler ist einer von vielen.

use bevy::prelude::*;

use crate::shared::{Gas, LocalPlayer};

/// Marker an dem Knoten, dessen Breite den Gasstand zeigt.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Gasleiste;

/// Baut die Leiste einmal beim `Startup`.
// gefuellt von Auftrag S — F-018, Bildpflicht
pub fn leiste_bauen(mut _commands: Commands) {}

/// Zieht die Breite auf `Gas::anteil()` nach.
// gefuellt von Auftrag S — F-018, Bildpflicht
pub fn leiste_fuellen(
    _spieler: Query<&Gas, With<LocalPlayer>>,
    mut _leiste: Query<&mut Node, With<Gasleiste>>,
) {
}
