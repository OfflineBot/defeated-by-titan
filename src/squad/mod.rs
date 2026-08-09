//! squad — Mitspieler und Eskorte: Kampfunfaehigkeit, Wiederbeleben, Markieren
//!
//! Die vier Grundregeln der Bibel (3.6) sind **nicht verhandelbar** und stehen hier im Code:
//! kein Schaden zwischen Spielern, **keine Kollision** zwischen Spielern (bei dieser
//! Geschwindigkeit die groesste Frustquelle ueberhaupt), getrennte Beute pro Spieler, kein
//! Ausschluss in oeffentlichen Instanzen.
//!
//! **Kampfunfaehigkeit statt Tod**: „tot" ist ein Zustand mit Timer, kein Entfernen der
//! Entity. Das erzeugt den wertvollsten Moment im Koop-Design — jemand muss entscheiden, ob
//! er mitten im Titanenfeuer landet, um einen anderen aufzurichten.
//!
//! **Noch leer.** Das Plugin steht im Baum, damit die Reihenfolge in `lib.rs` von Anfang an
//! stimmt und ein Fan-out auf Domaenen moeglich ist, ohne dass fuenf Agenten denselben
//! Ordner anlegen (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct SquadPlugin;

impl Plugin for SquadPlugin {
    fn build(&self, _app: &mut App) {}
}
