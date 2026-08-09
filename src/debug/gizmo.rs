//! Gizmos: der Zielstrahl, die Bogenbahn, die besuchten Gitterzellen.
//!
//! **Ohne sie gibt es keinen Beleg, nur eine Behauptung.** `docs/ABNAHME.md`: „Ohne Bild
//! kein 🟧, ohne Ausnahme" — und ein Bild, auf dem man den Strahl nicht sieht, belegt den
//! Strahl nicht. Jedes Bildkriterium in `docs/schnittstelle.md` nennt genau, was zu sehen
//! sein muss; hier wird es gezeichnet.
//!
//! Laeuft nur, wenn ein Fenster da ist — headless waere es Rechenzeit fuer niemanden.
//!
//! Die Signalfarben gelten auch hier: **Zyan** fuer Seil und Anker, **Bernstein** fuer Ziele
//! und Schwachstellen, **Karminrot** fuer Gefahr. Eine Gitterzelle ist keins von beidem und
//! bekommt eine neutrale Farbe.

use bevy::prelude::*;

use crate::shared::{Start, Tempo, Zielpunkt};

/// Zeichnet Zielstrahl, Trefferpunkt und die Spur der letzten Ticks.
// gefuellt von Auftrag S — Bildpflicht fuer F-002, F-004, F-012, T-036a
pub fn gizmos_zeichnen(
    _start: Res<Start>,
    _spieler: Query<(&Transform, &Tempo, &Zielpunkt)>,
    mut _gizmos: Gizmos,
) {
}
