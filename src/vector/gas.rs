//! `F-018` Das Gaskonto — **die einzige Stelle, die `Gas` abbucht.**
//!
//! Ohne diesen Umweg riefen `F-007` (Boost) und `F-005` (Reel-In) beide
//! `Gas::verbrauchen`. Die Methode ist bewusst atomar und ohne Teilverbrauch
//! (`shared::zustand`), also entschiede bei knappem Tank die **Systemreihenfolge**, wer
//! zahlt — der Muenzwurf mit 60 Hz, den `docs/architektur.md` verbietet, und im Netz ein
//! Desync, den niemand reproduziert.
//!
//! Hier wird **einmal pro Tick** gebucht und das Ergebnis als [`Gasfreigabe`]
//! veroeffentlicht. Wer dort `false` liest, traegt null in seinen Antrieb ein.
//!
//! Die **Rangfolge** bei knappem Tank steht in `assets/data/game.ron`
//! (`vector.gas_rangfolge`), nicht als `if` hier: „was geht zuerst aus?" ist eine
//! Spielwertentscheidung (`docs/FRAGEN.md` Q-017).

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Gas, Gasfreigabe, Haken, Intent};

/// Bucht das Gas dieses Ticks ab und schreibt [`Gasfreigabe`].
// gefuellt von Auftrag G — F-018
pub fn gaskonto(
    _zeit: Res<Time<Fixed>>,
    _daten: Res<GameData>,
    mut _spieler: Query<(&Intent, &Haken, &mut Gas, &mut Gasfreigabe)>,
) {
}
