//! hud — Gas, Klingenzustand, Ziel-Marker, Fadenkreuz
//!
//! **Liest nur.** Und liest den Zustand **des lokalen Spielers** ueber den
//! [`LocalPlayer`](crate::shared::LocalPlayer)-Marker — das ist die einzige Stelle im Code,
//! die weiss, wer „ich" bin.
//!
//! PC-only heisst: mehr Information gleichzeitig, weil kein Daumen die halbe Bildflaeche
//! verdeckt (Bibel 3.5).
//!
//! **Noch leer.** Das Plugin steht im Baum, damit die Reihenfolge in `lib.rs` von Anfang an
//! stimmt und ein Fan-out auf Domaenen moeglich ist, ohne dass fuenf Agenten denselben
//! Ordner anlegen (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, _app: &mut App) {}
}
