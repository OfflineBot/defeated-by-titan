//! Zufall, der auf zwei Rechnern gleich ausfaellt.
//!
//! **Determinismus, wo er billig ist** (`prompts/init.md` §6 Regel 5). Ein Titan, der auf
//! zwei Rechnern anders abbiegt, ist ein Bug, den man nur im Netz sieht — also am teuersten
//! Tag.
//!
//! Deshalb ist [`Wuerfel`] **zustandslos**: er zieht nicht aus einem fortlaufenden Strom,
//! sondern rechnet aus `(seed, tick, strom)` einen Wert aus. Das ist der entscheidende
//! Unterschied: ein fortlaufender Generator liefert andere Zahlen, sobald zwei Systeme in
//! anderer Reihenfolge laufen — und Bevys Systeme laufen parallel und nicht in fester
//! Reihenfolge. Ein Generator mit Zustand waere hier ein Muenzwurf mit 60 Hz.
//!
//! `strom` unterscheidet die Verwender: `titan.0` fuer eine Titanenentscheidung,
//! ein fester Hash fuer eine Beutewurf-Stelle. Zwei Verwender mit demselben `strom` im
//! selben Tick bekommen dieselbe Zahl — das ist kein Fehler, sondern die Regel: wer eine
//! eigene Zahl will, nimmt einen eigenen Strom.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Der Seed ist **Teil des Zustands** — er wird gespeichert und eines Tages verschickt.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wuerfel {
    pub seed: u64,
}

impl Default for Wuerfel {
    fn default() -> Self {
        // Feste Vorgabe statt Uhrzeit: eine Fahrt muss ohne Zutun reproduzierbar sein
        // (§17: der Befehl steht daneben, samt Seed).
        Wuerfel { seed: 0x0DEF_EA7E_D0B7_1743 }
    }
}

/// SplitMix64 — kurz, schnell, gut genug, und **ueberall gleich**. Kein `rand`-Crate:
/// dessen Ausgabe darf sich zwischen Versionen aendern, und genau das waere ein Desync,
/// den niemand als solchen erkennt.
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Wuerfel {
    pub fn neu(seed: u64) -> Self {
        Wuerfel { seed }
    }

    /// Ein roher Wert aus `(seed, tick, strom)`.
    pub fn roh(&self, tick: u64, strom: u64) -> u64 {
        splitmix64(
            self.seed
                ^ splitmix64(tick.wrapping_mul(0xD1B5_4A32_D192_ED03))
                ^ splitmix64(strom.wrapping_mul(0xA24B_AED4_963E_E407)),
        )
    }

    /// Gleichverteilt in `[0, 1)`.
    pub fn anteil(&self, tick: u64, strom: u64) -> f32 {
        // 24 Bit reichen fuer f32 und vermeiden, dass Rundung genau 1.0 ergibt.
        (self.roh(tick, strom) >> 40) as f32 / (1u32 << 24) as f32
    }

    /// Gleichverteilt in `[min, max)`.
    pub fn bereich(&self, tick: u64, strom: u64, min: f32, max: f32) -> f32 {
        min + self.anteil(tick, strom) * (max - min)
    }

    /// Ein Index in `0..n`. `n == 0` ergibt `0` — der Aufrufer prueft die Leere selbst.
    pub fn index(&self, tick: u64, strom: u64, n: usize) -> usize {
        if n == 0 { 0 } else { (self.roh(tick, strom) % n as u64) as usize }
    }

    /// Ob etwas mit Wahrscheinlichkeit `p` passiert.
    pub fn trifft(&self, tick: u64, strom: u64, p: f32) -> bool {
        self.anteil(tick, strom) < p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gleiche_eingabe_gleiche_zahl() {
        // Das ist der ganze Zweck: zwei Rechner, derselbe Tick, dieselbe Zahl.
        let w = Wuerfel::neu(42);
        assert_eq!(w.roh(7, 3), w.roh(7, 3));
        assert_eq!(w.anteil(7, 3), w.anteil(7, 3));
    }

    #[test]
    fn reihenfolge_spielt_keine_rolle() {
        // Bevys Systeme laufen parallel. Ein Generator mit Zustand haette hier je nach
        // Laufreihenfolge andere Werte — dieser nicht.
        let w = Wuerfel::neu(42);
        let vorwaerts: Vec<u64> = (0..5).map(|s| w.roh(9, s)).collect();
        let rueckwaerts: Vec<u64> = (0..5).rev().map(|s| w.roh(9, s)).collect();
        assert_eq!(vorwaerts, rueckwaerts.into_iter().rev().collect::<Vec<_>>());
    }

    #[test]
    fn verschiedene_seeds_verschiedene_zahlen() {
        assert_ne!(Wuerfel::neu(1).roh(0, 0), Wuerfel::neu(2).roh(0, 0));
        let w = Wuerfel::neu(1);
        assert_ne!(w.roh(0, 0), w.roh(1, 0));
        assert_ne!(w.roh(0, 0), w.roh(0, 1));
    }

    #[test]
    fn anteil_bleibt_im_halboffenen_einheitsintervall() {
        // Genau 1.0 waere der Fehler, der einen Index aus dem Array laufen laesst.
        let w = Wuerfel::neu(0xABCD);
        for tick in 0..2000u64 {
            let a = w.anteil(tick, tick % 7);
            assert!((0.0..1.0).contains(&a), "tick {tick} ergab {a}");
        }
    }

    #[test]
    fn index_bleibt_im_bereich_auch_bei_null() {
        let w = Wuerfel::neu(5);
        assert_eq!(w.index(1, 1, 0), 0);
        for n in 1..20usize {
            for tick in 0..50u64 {
                assert!(w.index(tick, 0, n) < n);
            }
        }
    }

    #[test]
    fn verteilung_ist_nicht_offensichtlich_schief() {
        // Kein Guetetest, nur eine Wache: ein kaputter Mischer faellt hier sofort auf.
        let w = Wuerfel::neu(7);
        let mut faecher = [0u32; 10];
        for tick in 0..10_000u64 {
            faecher[(w.anteil(tick, 0) * 10.0) as usize] += 1;
        }
        for (i, n) in faecher.iter().enumerate() {
            assert!((800..1200).contains(n), "Fach {i} hatte {n} von je ~1000");
        }
    }
}
