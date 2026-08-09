//! Das Vector Gear als **Zustand**: Haken, Seillaenge, Zielpunkt, Gasbuchung, Antriebe.
//!
//! Die Typen liegen in `shared/`, obwohl `vector` und `player` sie schreiben, weil `hud`,
//! `sound`, `render` und `debug` sie **lesen** muessen (`F-001`: „Zustaende sind im HUD
//! sichtbar"). Wer schreibt, steht in der Autoritaetstabelle in `docs/architektur.md` —
//! nicht im Typ.
//!
//! **Kein `Entity` und kein Zeiger in irgendeinem Feld.** Ein Haken haengt an einer
//! [`KoerperId`](super::ids::KoerperId); verschwindet der Traeger, meldet der Index das und
//! der Haken loest. Damit ueberlebt jedes Feld hier einen Schnappschuss, ein Rollback und
//! eines Tages eine Leitung (`docs/multiplayer.md` Regel 7 und 8).
//!
//! ## Die Trennung, die man beim Lesen erwartet und hier nicht findet
//!
//! Die **Seillaenge steht nicht im Haken**. `Haken` gehoert `F-001` (Zustandsmaschine), die
//! Verkuerzung gehoert `F-005`, und durchgesetzt wird der Zwang vom Integrator. Laege die
//! Laenge in `Haken`, haetten drei Auftraege dasselbe Feld geschrieben. So schreibt genau
//! der, der sie auch durchsetzt: [`Seillaenge`] gehoert `player::koerper::schritt`.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::ids::KoerperId;
use super::intent::Tasten;

/// Links oder rechts — zwei **unabhaengig** steuerbare Haken (`F-001`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Seite {
    Links,
    Rechts,
}

impl Seite {
    /// Index in [`Haken::arme`] und [`Seillaenge`]. `0 = links`, `1 = rechts` — dieselbe
    /// Reihenfolge wie in [`super::seil::seil_schritt`].
    pub fn index(self) -> usize {
        match self {
            Seite::Links => 0,
            Seite::Rechts => 1,
        }
    }

    pub const ALLE: [Seite; 2] = [Seite::Links, Seite::Rechts];
}

/// Die vier Zustaende aus `F-001` woertlich: „idle, fliegend, verankert, einziehend".
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Hakenzustand {
    #[default]
    Ruht,
    /// Die Spitze fliegt mit `vector.hakenflug_m_s` auf `ziel_m` zu.
    Fliegt {
        ziel_m: Vec3,
        /// Welchen Koerper der Zielstrahl getroffen hat. Er kann bis zum Einschlag
        /// verschwinden — dann loest der Haken mit `Loesegrund::TraegerWeg`.
        koerper: KoerperId,
    },
    /// Verankert. Der Ankerpunkt steht **im Koerpersystem**, nicht in der Welt: bewegt sich
    /// der Traeger (ab `F-029`), faehrt der Anker mit.
    Haelt {
        koerper: KoerperId,
        lokal_m: Vec3,
    },
    /// Die Spitze kommt mit `vector.haken_ruecklauf_m_s` zurueck.
    ZiehtEin,
}

impl Hakenzustand {
    pub fn haelt(&self) -> bool {
        matches!(self, Hakenzustand::Haelt { .. })
    }
}

/// Ein Hakenarm: sein Zustand und wo seine Spitze gerade ist.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Hakenarm {
    pub zustand: Hakenzustand,
    /// Weltposition der Spitze — fuer das Seil im Bild und den Einschlagklang.
    /// Bei `Ruht` bedeutungslos.
    pub spitze_m: Vec3,
}

/// Beide Haken eines Spielers.
///
/// **Ein Component mit zwei Faechern, keine Kind-Entities**: Bevy haengt denselben Component
/// nicht zweimal an eine Entity, und Kind-Entities waeren `Entity`-Verweise in etwas, das
/// gespeichert wird.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Haken {
    pub arme: [Hakenarm; 2],
}

impl Haken {
    pub fn arm(&self, seite: Seite) -> &Hakenarm {
        &self.arme[seite.index()]
    }

    /// Wie viele Arme gerade verankert sind — die Zahl hinter `assert haken` im Skript.
    pub fn verankert(&self) -> u32 {
        self.arme.iter().filter(|a| a.zustand.haelt()).count() as u32
    }
}

/// Die durchgesetzte Seillaenge je Seite. `0.0` heisst **kein Zwang**.
///
/// Gesetzt im Moment des Verankerns, danach durch `F-005` verkuerzt und — wenn die Wand
/// gewinnt — auf den tatsaechlichen Abstand **nachgezogen** (`docs/schnittstelle.md`,
/// „Schiedsrichter Seil gegen Wand"). Nie gegen eine Wand gekaempft.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Seillaenge {
    pub laengen_m: [f32; 2],
    /// Das Seil musste ueber `vector.hakenreichweite_m` hinaus nachgezogen werden.
    ///
    /// Der Integrator setzt das Kennzeichen, `vector::haken::haken_schalten` liest es im
    /// **naechsten** Tick und loest mit `Loesegrund::Ueberdehnt`. Ein Tick Verzug, dafuer
    /// genau ein Schreiber je Feld — statt eines zweiten Schreibers auf `Haken`.
    pub ueberdehnt: [bool; 2],
}

impl Seillaenge {
    pub fn laenge_m(&self, seite: Seite) -> f32 {
        self.laengen_m[seite.index()]
    }
}

/// Wohin ein Haken **jetzt** fliegen wuerde (`F-002`, freies Zielen).
///
/// Ein Tick lang gueltig, jeden Tick neu. `F-002` woertlich: „Diese Ebene bleibt IMMER aktiv
/// und ist niemals durch das Snap-System ersetzbar."
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Zielpunkt {
    /// Erster **fester** Treffer des Blickstrahls in Weltkoordinaten. `None` heisst: nichts
    /// in Reichweite.
    pub punkt_m: Option<Vec3>,
    /// Welcher Koerper getroffen wurde.
    pub koerper: Option<KoerperId>,
    /// Ob das Getroffene hakbar ist (`F-003`).
    ///
    /// **Getrennt von `punkt_m`, nicht vorgefiltert.** Ein Strahl, der ungetaggte Koerper
    /// ueberspringt, haekt durch Waende — `F-023` verbietet das woertlich.
    pub hakbar: bool,
}

/// Ergebnis der Gasbuchung **dieses** Ticks (`F-018`).
///
/// Wer hier `false` liest, hat kein Gas bekommen und traegt null in seinen Antrieb ein.
/// Ohne diesen Umweg riefen `F-005` und `F-007` beide `Gas::verbrauchen` — zwei Schreiber
/// auf einem Feld, und bei knappem Tank entschiede die Systemreihenfolge, wer zahlt. Die
/// **Rangfolge** ist eine Spielwertentscheidung und steht in `assets/data/game.ron`
/// (`vector.gas_rangfolge`), nicht als `if` im Code.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Gasfreigabe {
    pub boost: bool,
    pub einholen: bool,
}

/// Beitrag von Bodenlauf und Luftsteuerung, in m/s².
///
/// **Jeden Tick geschrieben, auch wenn er null ist** — dann braucht niemand ihn zu leeren,
/// es gibt kein Leer-System und keinen Zustand, der einen Tick zu lange lebt.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AntriebLauf(pub Vec3);

/// Beitrag des Gas-Boosts in Blickrichtung, in m/s² (`F-007`).
/// Hier docken spaeter `F-006` Swerve und `F-008` Dash an — ein System mehr, kein Typ mehr.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AntriebSchub(pub Vec3);

/// Gewuenschte Seilverkuerzung je Seite, in m/s (`F-005`).
///
/// **Keine Kraft.** Reel-In ist eine Laengenaenderung; wer daraus eine Zugkraft zum Anker
/// macht, bekommt „lineares Ziehen", das `F-004` ausdruecklich ausschliesst. Die
/// Beschleunigung faellt als Nebenwirkung des Seilzwangs an — genau die Zentripetalbewegung,
/// die `F-004` verlangt.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AntriebEinholen {
    pub m_s: [f32; 2],
}

/// Die Tasten des **vorigen** Ticks, als Component am Spieler.
///
/// Flankenerkennung (`Tasten::frisch`) braucht einen Vorzustand. Ein `Local<Tasten>` waere
/// **falsch**: ein `Local` gehoert dem System, nicht der Entity — mit zwei Spielern teilen
/// sich beide denselben Vorher-Wert, und im Schnappschuss ist er unsichtbar, also ueberlebt
/// er keinen Rollback (`docs/multiplayer.md` Regel 3 und 5).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VorigeTasten(pub Tasten);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f001_die_beiden_seiten_liegen_auf_festen_plaetzen() {
        // Die Reihenfolge ist Teil der Schnittstelle: `seil_schritt`, `Seillaenge` und
        // `Haken` indizieren alle gleich.
        assert_eq!(Seite::Links.index(), 0);
        assert_eq!(Seite::Rechts.index(), 1);
    }

    #[test]
    fn f001_ein_frischer_haken_haelt_nichts() {
        let h = Haken::default();
        assert_eq!(h.verankert(), 0);
        assert!(!h.arm(Seite::Links).zustand.haelt());
    }

    #[test]
    fn f001_verankert_zaehlt_nur_haltende_arme() {
        let mut h = Haken::default();
        h.arme[Seite::Links.index()].zustand =
            Hakenzustand::Haelt { koerper: KoerperId(3), lokal_m: Vec3::Y };
        h.arme[Seite::Rechts.index()].zustand =
            Hakenzustand::Fliegt { ziel_m: Vec3::ZERO, koerper: KoerperId(4) };
        assert_eq!(h.verankert(), 1);
    }
}
