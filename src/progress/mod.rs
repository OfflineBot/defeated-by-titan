//! progress — XP, Mark/Sigil, Gear-Budget, Traits, Lineage, Ascension
//!
//! ⚠️ **Kommt erst nach dem Vector-Gear-Gate.** Faehigkeitsbaum, Wirtschaft und Lineages
//! werden nicht angefangen, solange sich die Bewegung nicht ueberzeugend anfuehlt
//! (Bibel 6.1) — der Friedhof des Genres besteht aus Spielen, die es andersherum gemacht
//! haben.
//!
//! **Koennen schlaegt Zahlen** (Pfeiler P2): Stat-Wachstum oeffnet neue Inhalte, es ersetzt
//! keine Faehigkeit. Und **kein Fortschritt ohne Garantie** (P3): jedes Ziel ist auf einem
//! deterministischen Pfad erreichbar, alle Wahrscheinlichkeiten sind im Spiel einsehbar.
//!
//! **Noch leer.** Das Plugin steht im Baum, damit die Reihenfolge in `lib.rs` von Anfang an
//! stimmt und ein Fan-out auf Domaenen moeglich ist, ohne dass fuenf Agenten denselben
//! Ordner anlegen (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct ProgressPlugin;

impl Plugin for ProgressPlugin {
    fn build(&self, _app: &mut App) {}
}
