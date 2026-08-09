//! mission — Einsatz: Ziele, Phasen, Spawn-Wellen, Sieg und Niederlage
//!
//! **Ein Missionsbogen dauert 5–7 Minuten** und ist ein vollstaendiger Bogen mit garantiertem,
//! spuerbarem Fortschritt (Bibel 5, Aenderung 10). Die Referenz hat rund 21 Minuten
//! Sessiondauer — das sind 2–4 Missionen.
//!
//! Einsatzvorlagen stehen in `assets/data/missions.ron`: Ziele, Phasen, Spawn-Wellen,
//! Belohnung. Eine neue Mission ist Datei-Arbeit, kein Rust.
//!
//! **Noch leer.** Das Plugin steht im Baum, damit die Reihenfolge in `lib.rs` von Anfang an
//! stimmt und ein Fan-out auf Domaenen moeglich ist, ohne dass fuenf Agenten denselben
//! Ordner anlegen (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct MissionPlugin;

impl Plugin for MissionPlugin {
    fn build(&self, _app: &mut App) {}
}
