//! Die Kamera dreht sich — **sonst zeigt jedes Bild etwas anderes als der Strahl misst.**
//!
//! Heute enthaelt `src/` genau eine Rotation: die Sonne. Die Kamera haengt als Kind am
//! Spieler mit `Transform::from_xyz(0, augenhoehe_m, 0)`, also identitaetsgedreht, und
//! niemand schreibt `Intent.yaw/pitch` je in einen `Transform`. Sie blickt damit **immer**
//! nach −Z, waehrend der Zielstrahl nach `intent.blick()` geht. Sagt ein Skript
//! `look 30 -10`, zielt der Strahl woandershin als das Bild — und jedes Bildkriterium waere
//! wertlos, ohne dass es jemandem auffaellt.
//!
//! **Gedreht wird die KAMERA, nicht der Spieler.** Am Spieler haengt der Kollisionskasten;
//! dreht er mit, ist die achsenparallele Huelle keine achsenparallele Huelle mehr.
//!
//! Zeile in der Autoritaetstabelle: `Transform der Kamera | render`.
//!
//! **Keine Interpolation zwischen Simulationsschritten.** Sie braeuchte einen zweiten
//! Schreiber auf dem Spieler-`Transform` oder eine eigene Darstellungs-Entity — beides ist
//! ein eigener Entwurf und steht in `docs/ROADMAP.md`, nicht in diesem Auftrag.

use bevy::prelude::*;

use crate::shared::{Intent, LocalPlayer};

/// Legt `yaw` und `pitch` aus dem [`Intent`] des lokalen Spielers auf die Kamera.
///
/// Laeuft in `Update` und nicht im festen Schritt: Darstellung ist keine Simulation, und
/// eine Kamera, die nur 60-mal pro Sekunde nachzieht, fuehlt sich auf einem 144-Hz-Schirm
/// falsch an.
// gefuellt von Auftrag S — E6, Bildpflicht
pub fn kamera_drehen(
    _spieler: Query<&Intent, With<LocalPlayer>>,
    mut _kamera: Query<&mut Transform, With<Camera3d>>,
) {
}
