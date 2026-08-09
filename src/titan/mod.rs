//! titan — Titanen: Rig, Gliedmassen, Cortex, KI
//!
//! **Mindestens die Haelfte aller Gegnertypen hat eine Anti-Autopilot-Eigenschaft**
//! (Bibel 4) — sonst verkommt der Kampf zu Mausklicken auf Zielscheiben. Husk, Errant,
//! Scuttler, Weaver, Warden, Lurker, Bellower, Chorus, dazu vier Raid-Bosse.
//!
//! **Jeder Angriff hat eine Ausholphase von mindestens 0,4 s** und der Cortex ist aus 100 m
//! erkennbar (Bibel 2, Pfeiler P4: Lesbarkeit vor Realismus). Der Spieler soll nie fragen,
//! warum er gestorben ist.
//!
//! Liest [`TitanGetroffen`](crate::shared::TitanGetroffen) und entscheidet selbst, was ein
//! Treffer fuer seinen Koerper heisst — `combat` weiss nicht, wie ein Titan gebaut ist.
//!
//! **Noch leer.** Das Plugin steht im Baum, damit die Reihenfolge in `lib.rs` von Anfang an
//! stimmt und ein Fan-out auf Domaenen moeglich ist, ohne dass fuenf Agenten denselben
//! Ordner anlegen (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct TitanPlugin;

impl Plugin for TitanPlugin {
    fn build(&self, _app: &mut App) {}
}
