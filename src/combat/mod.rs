//! combat — Treffer, Schaden aus Geschwindigkeit, Amputation, Dampf, Tod
//!
//! **Schaden kommt aus Geschwindigkeit.** Ein Schnitt aus dem Stand kratzt, derselbe Schnitt
//! aus 30 m/s toetet — und die Formel gehoert in die RON, nicht in den Code
//! (`prompts/init.md` §1, §4).
//!
//! **Der Cortex ist die einzige Wahrheit:** ein Cortex-Treffer toetet, egal wie voll der
//! Titan ist. Alles andere ist Vorbereitung.
//!
//! Kein Splatter: Titanen verdampfen, Wunden stossen Dampf aus (Bibel 3.3). Das war ohnehin
//! doppelt begruendet und bleibt als Stilregel, auch ohne Plattform-Moderation.
//!
//! **Noch leer.** Das Plugin steht im Baum, damit die Reihenfolge in `lib.rs` von Anfang an
//! stimmt und ein Fan-out auf Domaenen moeglich ist, ohne dass fuenf Agenten denselben
//! Ordner anlegen (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, _app: &mut App) {}
}
