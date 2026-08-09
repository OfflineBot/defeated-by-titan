//! Zustand am Spieler — **Components, niemals `Resource`.**
//!
//! Gas, Klingen und Bewegungszustand haengen an *einem* Spieler. Als `Resource` waeren sie
//! global, und damit waere das Spiel ein Einzelspieler-Spiel, dem man das erst ansieht, wenn
//! Multiplayer drankommt (`prompts/init.md` §6 Regel 3).
//!
//! Sie liegen in `shared/`, obwohl `vector` und `blades` sie schreiben, weil `hud` und
//! `sound` sie **lesen** muessen — und eine Kante zwischen Domaenen dafuer zu oeffnen waere
//! der Anfang vom Ende der Domaenenregel. **Wer schreibt, steht in der Autoritaetstabelle
//! in `docs/architektur.md`, nicht im Typ.**

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Gas ist endlich, und **Gas verbrauchen ist laut** — der Bellower reagiert auf das
/// Geraeusch (Bibel 4). Das koppelt die Ressource an das Risiko, statt sie zu einem reinen
/// Timer zu machen.
///
/// Die Zahlen (Tankgroesse, Verbrauch pro Sekunde, Boost-Kosten) stehen in
/// `assets/data/gear.ron`, **nicht hier** (§4).
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gas {
    pub jetzt: f32,
    pub maximal: f32,
    /// `--sandbox` setzt das: unendlich Gas zum Anschauen (§12a).
    pub unbegrenzt: bool,
}

impl Gas {
    pub fn voll(maximal: f32) -> Self {
        Gas { jetzt: maximal, maximal, unbegrenzt: false }
    }

    pub fn anteil(&self) -> f32 {
        if self.maximal > 0.0 { (self.jetzt / self.maximal).clamp(0.0, 1.0) } else { 0.0 }
    }

    /// Versucht, `menge` zu verbrauchen. `false` heisst: es war nicht genug da, **und es
    /// wurde nichts abgezogen**.
    ///
    /// Kein Teilverbrauch: „Gas exakt null im Moment des Boosts" ist einer der Sonderfaelle,
    /// die getestet gehoeren (§8) — ein halber Boost waere schwerer zu erklaeren als keiner.
    pub fn verbrauchen(&mut self, menge: f32) -> bool {
        if self.unbegrenzt {
            return true;
        }
        if !menge.is_finite() || menge < 0.0 {
            return false;
        }
        if self.jetzt + 1e-6 < menge {
            return false;
        }
        self.jetzt = (self.jetzt - menge).max(0.0);
        true
    }

    pub fn nachfuellen(&mut self, menge: f32) {
        if menge.is_finite() && menge > 0.0 {
            self.jetzt = (self.jetzt + menge).min(self.maximal);
        }
    }

    pub fn leer(&self) -> bool {
        !self.unbegrenzt && self.jetzt <= 0.0
    }
}

/// Klingen werden stumpf und brechen. **Wirtschaft statt Cooldowns**
/// (`prompts/init.md` §1): nachgeladen wird an Versorgungspunkten, vom Pferd oder an
/// gefallenen Kameraden.
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Klingen {
    /// Wie viele Paare noch im Gurt stecken.
    pub paare_uebrig: u8,
    /// Zustand des eingesetzten Paares, 1.0 = frisch, 0.0 = gebrochen.
    pub schaerfe: f32,
}

impl Klingen {
    pub fn frisch(paare: u8) -> Self {
        Klingen { paare_uebrig: paare, schaerfe: 1.0 }
    }

    pub fn gebrochen(&self) -> bool {
        self.schaerfe <= 0.0
    }

    /// Ein frisches Paar einlegen. `false` heisst: keins mehr da.
    pub fn wechseln(&mut self) -> bool {
        if self.paare_uebrig == 0 {
            return false;
        }
        self.paare_uebrig -= 1;
        self.schaerfe = 1.0;
        true
    }
}

/// Geschwindigkeit in m/s.
///
/// Ein eigener Component und kein aus dem `Transform` abgeleiteter Wert: **Schaden kommt aus
/// Geschwindigkeit** (`prompts/init.md` §1), und eine Groesse, aus der Schaden entsteht,
/// darf nicht davon abhaengen, wie viel Zeit zwischen zwei Bildern lag. Ausserdem ist genau
/// das die Zahl, die `assert speed > 25` misst (§12b).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Tempo(pub Vec3);

impl Tempo {
    pub fn betrag_m_s(&self) -> f32 {
        self.0.length()
    }
}

/// Woran der Koerper des Spielers gerade haengt.
///
/// **Er entscheidet, wer den `Transform` schreiben darf.** `player` schreibt ihn am Boden
/// und im freien Fall, `vector` am Seil — nie beide gleichzeitig. Zwei Schreiber auf
/// demselben Feld sind kein Design, sondern ein Muenzwurf mit 60 Hz (§5 Regel 4).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bewegungszustand {
    #[default]
    AmBoden,
    InDerLuft,
    /// Mindestens ein Haken haelt — ab hier gehoert der `Transform` `vector`.
    AmSeil,
    AnDerWand,
    /// Kampfunfaehig statt tot: ein Zustand mit Timer, kein Entfernen der Entity.
    /// Wiederbeleben durch Mitspieler (Bibel 3.6, `squad/`).
    Niedergestreckt,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_verbraucht_nur_was_da_ist() {
        let mut g = Gas::voll(100.0);
        assert!(g.verbrauchen(30.0));
        assert!((g.jetzt - 70.0).abs() < 1e-6);
        assert!(!g.verbrauchen(80.0), "80 aus 70 darf nicht gelingen");
        assert!((g.jetzt - 70.0).abs() < 1e-6, "ein misslungener Boost kostet nichts");
    }

    #[test]
    fn gas_exakt_null_im_moment_des_boosts() {
        // Genau der Sonderfall aus prompts/init.md §8 — der Normalfall funktioniert
        // fast von allein, die Fehler sitzen an den Raendern.
        let mut g = Gas::voll(10.0);
        assert!(g.verbrauchen(10.0));
        assert!(g.leer());
        assert!(!g.verbrauchen(0.001));
        assert_eq!(g.anteil(), 0.0);
    }

    #[test]
    fn gas_lehnt_unsinnige_mengen_ab() {
        let mut g = Gas::voll(10.0);
        assert!(!g.verbrauchen(-5.0), "negativer Verbrauch waere ein Nachfuellen");
        assert!(!g.verbrauchen(f32::NAN));
        assert!((g.jetzt - 10.0).abs() < 1e-6);
    }

    #[test]
    fn sandbox_gas_geht_nie_aus() {
        let mut g = Gas { unbegrenzt: true, ..Gas::voll(1.0) };
        assert!(g.verbrauchen(1000.0));
        assert!(!g.leer());
    }

    #[test]
    fn gas_laeuft_beim_nachfuellen_nicht_ueber() {
        let mut g = Gas::voll(50.0);
        g.verbrauchen(10.0);
        g.nachfuellen(999.0);
        assert!((g.jetzt - 50.0).abs() < 1e-6);
    }

    #[test]
    fn klingen_wechseln_bis_der_gurt_leer_ist() {
        let mut k = Klingen::frisch(2);
        k.schaerfe = 0.0;
        assert!(k.gebrochen());
        assert!(k.wechseln());
        assert!(!k.gebrochen());
        assert!(k.wechseln());
        assert!(!k.wechseln(), "aus einem leeren Gurt kommt kein Paar mehr");
        assert_eq!(k.paare_uebrig, 0);
    }
}
