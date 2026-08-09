//! Der Simulationstick und die Reihenfolge, in der Eingaben entstehen.
//!
//! Beides liegt in `shared/`, weil **zwei** Domaenen es brauchen und keine der anderen
//! gehoeren darf: `net` stellt Intents zu, `debug` erzeugt sie aus einem Skript. Laege der
//! `SystemSet` bei `net`, braeuchte `debug` eine Kante dorthin — und die Domaenenregel waere
//! aufgeweicht, um eine Reihenfolge auszudruecken (`docs/architektur.md`).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Der Simulationstick. Zaehlt in `FixedPreUpdate` hoch, **bevor** ihn jemand liest.
///
/// Er ist Teil des Zustands: der geseedete Zufall rechnet aus `(seed, tick)`, und ein
/// `Intent` traegt den Tick, fuer den er gemeint war (§6 Regel 2 und 5).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tick(pub u64);

/// Die drei Stufen, in denen eine Eingabe pro Tick entsteht.
///
/// **Die Reihenfolge ist der ganze Punkt.** Ohne sie haengt es von der Laufreihenfolge der
/// Systeme ab, ob ein Tastendruck aus dem Skript noch im selben Tick ankommt — und das ist
/// kein Design, sondern ein Muenzwurf mit 60 Hz.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EingabeSet {
    /// Wer Eingaben **erzeugt**: der `--script`-Fahrer drueckt echte Tasten.
    Quelle,
    /// Wer sie **einsammelt**: Tastatur und Maus werden zu einem `Intent` im Posteingang.
    Sammeln,
    /// Wer sie **zustellt**: Tick hochzaehlen, faellige Intents an die Spieler.
    Zustellen,
}
