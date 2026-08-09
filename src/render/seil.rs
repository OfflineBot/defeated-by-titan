//! Die zwei Seile im Bild — **ohne sie belegt kein Screenshot `F-001`.**
//!
//! Gezeichnet wird von der Schulter des Spielers zur Hakenspitze
//! ([`Hakenarm::spitze_m`](crate::shared::Hakenarm)), in **Zyan**: die Signalfarbe des
//! Vector Gear (`docs/konventionen.md`, Zyan = Gas/Vector Gear/Anker). Bernstein und
//! Karminrot sind hier verboten, auch als Platzhalter.
//!
//! `render` **liest nur**. Der Zustand kommt aus [`Haken`](crate::shared::Haken); dieses
//! Modul schreibt kein Feld der Simulation.

use bevy::prelude::*;

use crate::shared::Haken;

/// Zeichnet je verankertem oder fliegendem Arm eine Linie.
// gefuellt von Auftrag S — F-001, Bildpflicht
pub fn seile_zeichnen(_spieler: Query<(&Haken, &Transform)>, mut _gizmos: Gizmos) {}
