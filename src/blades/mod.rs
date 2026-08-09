//! blades — Klingen: Schwung, Abnutzung, Bruch, Wechsel, Nachschub
//!
//! **Wirtschaft statt Cooldowns.** Klingen werden stumpf und brechen; nachgeladen wird an
//! Versorgungspunkten, vom Pferd oder an gefallenen Kameraden.
//!
//! Schreibt [`Klingen`](crate::shared::Klingen).
//!
//! **Noch leer.** Das Plugin steht im Baum, damit die Reihenfolge in `lib.rs` von Anfang an
//! stimmt und ein Fan-out auf Domaenen moeglich ist, ohne dass fuenf Agenten denselben
//! Ordner anlegen (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct BladesPlugin;

impl Plugin for BladesPlugin {
    fn build(&self, _app: &mut App) {}
}
