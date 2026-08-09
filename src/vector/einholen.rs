//! `F-005` Reel-In — **eine Laengenaenderung, keine Zugkraft.**
//!
//! Wer Reel-In als Kraft zum Anker baut, bekommt „lineares Ziehen", das `F-004`
//! ausdruecklich ausschliesst. Als Laengenaenderung faellt die Beschleunigung als
//! Nebenwirkung des Seilzwangs an — und `shared::seil::seil_einholen` skaliert dabei die
//! **tangentiale** Geschwindigkeit mit `L_alt / L_neu`. Das ist der Unterschied zwischen
//! „der Spieler gewinnt Hoehe" und „der Spieler gewinnt Tempo", und das Tempo ist das
//! Gefuehl, an dem das ganze Spiel haengt.
//!
//! Dieses Modul schreibt nur den **Wunsch** ([`AntriebEinholen`], in m/s je Seite);
//! ausgefuehrt wird er vom Integrator, der auch die Laenge fuehrt und auf
//! `vector.seil_min_m` klemmt. Ein Schreiber je Feld.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{AntriebEinholen, Gasfreigabe, Haken, Intent};

/// Schreibt [`AntriebEinholen`] je Seite: `vector.seilzug_m_s` oder 0.
// gefuellt von Auftrag E — F-005
pub fn einholen(
    _daten: Res<GameData>,
    mut _spieler: Query<(&Intent, &Haken, &Gasfreigabe, &mut AntriebEinholen)>,
) {
}
