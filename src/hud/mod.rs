//! hud — Gas, Klingenzustand, Ziel-Marker, Fadenkreuz
//!
//! **Liest nur.** Und liest den Zustand **des lokalen Spielers** ueber den
//! [`LocalPlayer`](crate::shared::LocalPlayer)-Marker — das ist die einzige Stelle im Code,
//! die weiss, wer „ich" bin.
//!
//! PC-only heisst: mehr Information gleichzeitig, weil kein Daumen die halbe Bildflaeche
//! verdeckt (Bibel 3.5).
//!
//! **Stand der Naht:** [`leiste`] ist registriert und leer. Solange sie leer ist, ist der
//! Gasstand nirgends im Bild — und `F-018` bleibt 🟨 („Logik getestet, Pixel ungesehen"),
//! egal wie gruen sein Test ist. Eine Zahl im Terminal ist kein Bild.

pub mod leiste;

use bevy::prelude::*;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, leiste::leiste_bauen)
            .add_systems(Update, leiste::leiste_fuellen);
    }
}
