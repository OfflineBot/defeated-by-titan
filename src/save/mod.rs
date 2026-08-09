//! save — Spielstand: Profil, Gear-Budget, Traits, Lineage, Fortschritt
//!
//! Die Anforderung der Bibel gilt unveraendert, auch wenn ProfileStore hier nichts bedeutet:
//! **kein Datenverlust, keine Duplikation.** Nachruesten ist praktisch unmoeglich (Bibel 6.4)
//! — deshalb steht die Domaene ab Tag 1 im Baum, auch leer.
//!
//! Ein Spielstand haengt an einer [`PlayerId`](crate::shared::PlayerId), nicht an einer
//! `Entity` und nicht an einer Verbindung.
//!
//! **Noch leer.** Das Plugin steht im Baum, damit die Reihenfolge in `lib.rs` von Anfang an
//! stimmt und ein Fan-out auf Domaenen moeglich ist, ohne dass fuenf Agenten denselben
//! Ordner anlegen (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, _app: &mut App) {}
}
