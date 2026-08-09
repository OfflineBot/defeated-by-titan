//! menu — Hauptmenue, Pause, Optionen
//!
//! Ein Hauptmenue ist fuer jemanden, der nicht klicken kann, eine Wand ohne Tuer — deshalb
//! gibt es `--sandbox`, `--mission` und `--script`, die daran vorbeigehen
//! (`prompts/init.md` §12a).
//!
//! Freie Tastenbelegung, Farbenblindmodi, Screenshake-Regler und Bewegungsreduktion sind
//! Anforderungen, keine Kuer (Bibel 3.5).
//!
//! **Noch leer.** Das Plugin steht im Baum, damit die Reihenfolge in `lib.rs` von Anfang an
//! stimmt und ein Fan-out auf Domaenen moeglich ist, ohne dass fuenf Agenten denselben
//! Ordner anlegen (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, _app: &mut App) {}
}
