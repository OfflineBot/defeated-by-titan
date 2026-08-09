//! Stabile Ids — **nie** Bevys `Entity` fuer etwas, das gespeichert oder verschickt wird.
//!
//! `Entity` ist ein lokaler Index mit Generation. Auf einem anderen Rechner bedeutet
//! dieselbe Zahl etwas anderes, und nach einem Neustart ebenfalls. Eigene Ids kosten heute
//! eine Zeile und retten spaeter den Netzcode **und** den Spielstand
//! (`prompts/init.md` §6 Regel 7).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Wer ein Spieler ist — ueber Sitzungen, Verbindungsabbrueche und Rechner hinweg.
///
/// Ein Verbindungsabbruch reserviert den Platz 120 s (Bibel 3.6): der Zustand haengt an
/// dieser Id, nicht an einer Verbindung und nicht an einer `Entity`.
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct PlayerId(pub u32);

/// Wer ein Titan ist. Gleiche Begruendung wie [`PlayerId`].
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct TitanId(pub u32);

/// **Die einzige Stelle im Code, die weiss, welcher Spieler „ich" bin.**
///
/// Daran haengt die Kamera, daran haengt das HUD — und sonst nichts. Jedes System, das
/// stattdessen `.single()` auf eine Spieler-Query schreibt, macht aus dem Spiel ein
/// Einzelspieler-Spiel, und das merkt niemand, bis Multiplayer drankommt
/// (`prompts/init.md` §6 Regel 3, geprueft von `tests/mehrspieler.rs`).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct LocalPlayer;

/// Vergibt fortlaufende Ids. Teil des Zustands, damit zwei Rechner dieselbe Reihenfolge
/// bekommen — deshalb eine Resource mit Zaehler und kein Zufall.
#[derive(Resource, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct IdZaehler {
    pub spieler: u32,
    pub titan: u32,
}

impl IdZaehler {
    pub fn naechster_spieler(&mut self) -> PlayerId {
        self.spieler += 1;
        PlayerId(self.spieler)
    }

    pub fn naechster_titan(&mut self) -> TitanId {
        self.titan += 1;
        TitanId(self.titan)
    }
}
