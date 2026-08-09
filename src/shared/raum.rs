//! Der raeumliche Index — **nichts durchlaeuft alle Entities, um eine Frage ueber die zehn
//! Meter vor der Nase zu beantworten** (`prompts/init.md` §11, `T-036a`).
//!
//! Hakeneinschlag (`F-002`), Kollision (`F-013`), Klingentreffer und Titanen-Zielsuche gehen
//! **alle** hier durch. Gepflegt wird er von `world::index`; der Typ liegt in `shared/`,
//! damit `vector` und `player` ihn benutzen koennen, **ohne eine Kante zu `world`** — genau
//! das Muster, das `shared::bau` schon fuer `Bauklotz` festhaelt.
//!
//! Zulaessig als `Resource`, weil er **Weltzustand** ist und kein Spielerzustand: in diesem
//! Typ steht kein einziges autoritatives Feld pro Spieler (`docs/multiplayer.md` Regel 3
//! verbietet nur Letzteres).
//!
//! ## Drei Entscheidungen, die man nicht ohne Messung wieder aufmacht
//!
//! 1. **Gitter nur ueber X und Z.** Eine Stadt misst horizontal 560 m (Ashgate) und
//!    vertikal selten 40 m. Eine dritte Achse verdreifacht die Zellen und trennt fast
//!    nichts.
//! 2. **Grosskoerper kommen NICHT ins Gitter.** Der Boden ist ein Quader von 400 x 400 m;
//!    bei 8-m-Zellen laege er in 2500 Zellen und jeder waagerechte Strahl testete ihn
//!    einmal pro besuchter Zelle. Wer mehr als `grosskoerper_zellen` Zellen belegt, landet
//!    in einer linearen Liste, die jeder Strahl **genau einmal** prueft.
//! 3. **Kein `Default`.** `zelle_m = 0.0` waere eine Division durch null im DDA, und
//!    `app.init_resource::<RaumIndex>()` ist die naheliegendste Zeile der Welt. Es gibt nur
//!    [`RaumIndex::neu`].
//!
//! ## Warum das Verzeichnis
//!
//! `entfernen(id)` ohne Position muesste das ganze Gitter absuchen — bei 840-m-Karten sind
//! das ueber 70 000 Zellen **pro Despawn**, und `F-029` (Anker an Titanengliedmassen) und
//! `T-020` (Streaming) despawnen dauernd. Das `BTreeMap`-Verzeichnis kostet einen Eintrag je
//! Koerper und macht daraus einen Zugriff auf die Zellen, die der Koerper wirklich belegt.
//! `BTreeMap` und nicht `HashMap`: die Reihenfolge einer Iteration ist Teil des
//! Determinismus.

use std::collections::BTreeMap;

use bevy::math::Dir3;
use bevy::prelude::*;

use super::bau::Maske;
use super::ids::KoerperId;

/// Ein Koerper, wie ihn der Index fuehrt: achsenparallele Huelle plus Maske.
///
/// Eine Kopie, kein Verweis — der Index wird gelesen, waehrend die Welt sich bewegt, und
/// eine `Entity` gehoert in nichts, was gespeichert oder verschickt wird.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Eintrag {
    pub id: KoerperId,
    /// Mitte der Huelle in Weltkoordinaten.
    pub mitte_m: Vec3,
    /// Halbe Kantenlaenge. `Aabb3d::new` nimmt genau diese Form.
    pub halb_m: Vec3,
    pub maske: Maske,
}

impl Eintrag {
    pub fn min_m(&self) -> Vec3 {
        self.mitte_m - self.halb_m
    }

    pub fn max_m(&self) -> Vec3 {
        self.mitte_m + self.halb_m
    }
}

/// Der naechste feste Treffer eines Strahls.
///
/// **`maske` wird mitgeliefert, nicht vorgefiltert** (`F-023`: „Sichtlinienpruefung
/// verhindert Haken durch Waende"). Ein Strahl, der ungetaggte Koerper ueberspringt, haekt
/// durch Mauern; der Aufrufer entscheidet, ob der Treffer hakbar ist.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Treffer {
    pub koerper: KoerperId,
    pub punkt_m: Vec3,
    /// Flaechennormale am Trefferpunkt, Einheitslaenge.
    pub normale_m: Vec3,
    /// Entfernung vom Strahlursprung in Metern.
    pub weite_m: f32,
    pub maske: Maske,
}

/// Ergebnis einer Strahlabfrage.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Strahlergebnis {
    pub treffer: Option<Treffer>,
    /// Wie viele Huellen wirklich geprueft wurden.
    ///
    /// **Diagnose, kein Beleg.** Eine lineare Implementierung, die hier `1` meldet, wuerde
    /// jede Zaehlerschranke bestehen — die Kosten werden von aussen gemessen
    /// (`docs/schnittstelle.md`, Kriterium `T-036a`).
    pub geprueft: u32,
}

/// Gitterzellen ueber XZ plus eine Liste fuer Grosskoerper.
#[derive(Resource, Debug)]
pub struct RaumIndex {
    zelle_m: f32,
    halbe_ausdehnung_m: f32,
    spalten: usize,
    grosskoerper_zellen: u32,
    /// `spalten * spalten` Zellen, einmal beim Start alloziert, nie pro Tick.
    zellen: Vec<Vec<Eintrag>>,
    /// Koerper, die zu viele Zellen belegen. Jeder Strahl prueft sie genau einmal.
    gross: Vec<Eintrag>,
    /// Wo jeder Koerper steht — fuer `entfernen` und fuer die Ankerauskunft.
    verzeichnis: BTreeMap<KoerperId, Eintrag>,
    /// Postfach des `on_remove`-Beobachters (siehe [`RaumIndex::abmelden`]).
    abgemeldet: Vec<KoerperId>,
}

impl RaumIndex {
    /// Der **einzige** Konstruktor. Alle drei Zahlen kommen aus `assets/data/game.ron`
    /// (`welt.zelle_m`, `welt.halbe_ausdehnung_m`, `welt.grosskoerper_zellen`).
    ///
    /// Bricht ab, wenn die Zellgroesse nicht positiv ist: eine Null waere eine Division
    /// durch null im DDA, drei Systeme spaeter und ohne Hinweis auf die Datei.
    /// `tests/data.rs` faengt denselben Fall als roten Test statt als Spielabbruch.
    pub fn neu(zelle_m: f32, halbe_ausdehnung_m: f32, grosskoerper_zellen: u32) -> Self {
        assert!(
            zelle_m.is_finite() && zelle_m > 0.0,
            "welt.zelle_m = {zelle_m} — muss endlich und > 0 sein (assets/data/game.ron)"
        );
        assert!(
            halbe_ausdehnung_m.is_finite() && halbe_ausdehnung_m > 0.0,
            "welt.halbe_ausdehnung_m = {halbe_ausdehnung_m} — muss endlich und > 0 sein"
        );
        let spalten = ((2.0 * halbe_ausdehnung_m / zelle_m).ceil() as usize).max(1);
        RaumIndex {
            zelle_m,
            halbe_ausdehnung_m,
            spalten,
            grosskoerper_zellen: grosskoerper_zellen.max(1),
            zellen: vec![Vec::new(); spalten * spalten],
            gross: Vec::new(),
            verzeichnis: BTreeMap::new(),
            abgemeldet: Vec::new(),
        }
    }

    pub fn zelle_m(&self) -> f32 {
        self.zelle_m
    }

    pub fn halbe_ausdehnung_m(&self) -> f32 {
        self.halbe_ausdehnung_m
    }

    pub fn spalten(&self) -> usize {
        self.spalten
    }

    /// Wie viele Koerper der Index kennt — Gitter und Grosskoerper zusammen.
    pub fn anzahl(&self) -> usize {
        self.verzeichnis.len()
    }

    /// Wie viele Koerper in der Grosskoerper-Liste liegen.
    pub fn anzahl_gross(&self) -> usize {
        self.gross.len()
    }

    /// Ein Koerper ueber seine Id — die Auskunft, aus der ein Haken seinen Ankerpunkt in
    /// Weltkoordinaten rechnet (`Koerper`-Mitte + `lokal_m`). `None` heisst: **Traeger
    /// weg**, der Haken loest mit `Loesegrund::TraegerWeg`.
    pub fn koerper(&self, id: KoerperId) -> Option<Eintrag> {
        self.verzeichnis.get(&id).copied()
    }

    /// Einen Koerper aufnehmen oder seine Huelle ersetzen.
    pub fn einfuegen(&mut self, eintrag: Eintrag) {
        self.entfernen(eintrag.id);
        match self.zellbereich(eintrag.mitte_m, eintrag.halb_m) {
            Some(bereich) if bereich.zellen() <= self.grosskoerper_zellen as usize => {
                for z in bereich.iz0..=bereich.iz1 {
                    for x in bereich.ix0..=bereich.ix1 {
                        self.zellen[z * self.spalten + x].push(eintrag);
                    }
                }
            }
            _ => self.gross.push(eintrag),
        }
        self.verzeichnis.insert(eintrag.id, eintrag);
    }

    /// Einen Koerper aus dem Index nehmen. `true`, wenn er drin war.
    pub fn entfernen(&mut self, id: KoerperId) -> bool {
        let Some(alt) = self.verzeichnis.remove(&id) else {
            return false;
        };
        match self.zellbereich(alt.mitte_m, alt.halb_m) {
            Some(bereich) if bereich.zellen() <= self.grosskoerper_zellen as usize => {
                for z in bereich.iz0..=bereich.iz1 {
                    for x in bereich.ix0..=bereich.ix1 {
                        self.zellen[z * self.spalten + x].retain(|e| e.id != id);
                    }
                }
            }
            _ => self.gross.retain(|e| e.id != id),
        }
        true
    }

    /// **Das Postfach.** Der `on_remove`-Beobachter in `world::index` schiebt hier die Id
    /// eines verschwindenden Koerpers hinein; der Pfleger holt sie im naechsten festen
    /// Schritt ab.
    ///
    /// Warum nicht `RemovedComponents`: dessen Puffer werden in `World::clear_trackers`
    /// umgeschaltet, und das laeuft **einmal pro `App::update`**
    /// (`bevy_app-0.19.0/src/sub_app.rs:149`), waehrend `FixedMain` 0..n mal pro Frame
    /// laeuft (`bevy_time-0.19.0/src/fixed.rs:37-39`). Headless treibt 240 Hz gegen 60 Hz
    /// Fixed — **drei von vier Frames verlieren die Meldung**, und der Index veraltet genau
    /// dort, wo er es laut Doku nicht kann.
    pub fn abmelden(&mut self, id: KoerperId) {
        self.abgemeldet.push(id);
    }

    /// Die offenen Abmeldungen abholen und das Postfach leeren.
    pub fn abmeldungen_holen(&mut self) -> Vec<KoerperId> {
        std::mem::take(&mut self.abgemeldet)
    }

    /// Der naechste **feste** Treffer eines Strahls, samt Maske (`E14`: erst treffen, dann
    /// hakbar pruefen).
    ///
    /// 2D-DDA ueber XZ; je besuchter Zelle `RayCast3d::aabb_intersection_at`
    /// (`bevy_math-0.19.0/src/bounding/raycast3d.rs:49`), Abbruch, sobald der naechste
    /// Treffer naeher liegt als der Ausgang der aktuellen Zelle. Die Grosskoerper-Liste
    /// wird **einmal** vorab geprueft.
    ///
    /// Achtung beim Fuellen: `aabb_intersection_at` klemmt `tmin` auf 0
    /// (`raycast3d.rs:64`) — ein Ursprung **im** Kasten liefert `Some(0.0)`.
    // gefuellt von Auftrag R — T-036a
    pub fn strahl(&self, _von_m: Vec3, _richtung: Dir3, _weite_m: f32) -> Strahlergebnis {
        Strahlergebnis::default()
    }

    /// Alle Koerper, deren Huelle den Kasten beruehren koennte. `aus` wird geleert und neu
    /// gefuellt — der Aufrufer haelt den Puffer, damit kein System pro Tick alloziert.
    // gefuellt von Auftrag R — T-036a
    pub fn huelle(&self, _mitte_m: Vec3, _halb_m: Vec3, aus: &mut Vec<Eintrag>) {
        aus.clear();
    }

    /// Zellbereich, den eine Huelle beruehrt — geklemmt auf das Gitter. `None` heisst:
    /// vollstaendig ausserhalb.
    fn zellbereich(&self, mitte_m: Vec3, halb_m: Vec3) -> Option<Zellbereich> {
        if !(mitte_m.is_finite() && halb_m.is_finite()) {
            return None;
        }
        let min = mitte_m - halb_m;
        let max = mitte_m + halb_m;
        let grenze = self.halbe_ausdehnung_m;
        if max.x < -grenze || min.x > grenze || max.z < -grenze || min.z > grenze {
            return None;
        }
        Some(Zellbereich {
            ix0: self.spalte(min.x),
            ix1: self.spalte(max.x),
            iz0: self.spalte(min.z),
            iz1: self.spalte(max.z),
        })
    }

    /// Weltkoordinate -> Spaltenindex, geklemmt auf den Rand. Ein Koerper knapp ausserhalb
    /// verschwindet damit nicht, er landet in der Randzelle.
    fn spalte(&self, wert_m: f32) -> usize {
        let roh = (wert_m + self.halbe_ausdehnung_m) / self.zelle_m;
        if !roh.is_finite() || roh < 0.0 {
            return 0;
        }
        (roh as usize).min(self.spalten - 1)
    }
}

#[derive(Clone, Copy, Debug)]
struct Zellbereich {
    ix0: usize,
    ix1: usize,
    iz0: usize,
    iz1: usize,
}

impl Zellbereich {
    fn zellen(&self) -> usize {
        (self.ix1 - self.ix0 + 1) * (self.iz1 - self.iz0 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> RaumIndex {
        RaumIndex::neu(8.0, 320.0, 64)
    }

    fn klotz(id: u32, mitte: Vec3, halb: Vec3) -> Eintrag {
        Eintrag { id: KoerperId(id), mitte_m: mitte, halb_m: halb, maske: Maske::FEST }
    }

    #[test]
    fn t036a_das_gitter_deckt_die_ganze_karte_ab() {
        let i = index();
        assert_eq!(i.spalten(), 80, "2 * 320 m / 8 m = 80 Spalten");
        assert_eq!(i.anzahl(), 0);
    }

    #[test]
    fn t036a_ein_koerper_wird_gefunden_und_wieder_entfernt() {
        let mut i = index();
        i.einfuegen(klotz(1, Vec3::new(10.0, 6.0, -20.0), Vec3::splat(5.0)));
        assert_eq!(i.anzahl(), 1);
        assert_eq!(i.koerper(KoerperId(1)).map(|e| e.mitte_m), Some(Vec3::new(10.0, 6.0, -20.0)));
        assert!(i.entfernen(KoerperId(1)));
        assert_eq!(i.anzahl(), 0);
        assert_eq!(i.koerper(KoerperId(1)), None);
        assert!(!i.entfernen(KoerperId(1)), "zweimal entfernen ist kein Fehler, aber auch kein Treffer");
    }

    #[test]
    fn t036a_einfuegen_ersetzt_statt_zu_verdoppeln() {
        let mut i = index();
        i.einfuegen(klotz(7, Vec3::new(0.0, 0.0, 0.0), Vec3::splat(2.0)));
        i.einfuegen(klotz(7, Vec3::new(100.0, 0.0, 100.0), Vec3::splat(2.0)));
        assert_eq!(i.anzahl(), 1);
        assert_eq!(i.koerper(KoerperId(7)).map(|e| e.mitte_m), Some(Vec3::new(100.0, 0.0, 100.0)));
    }

    #[test]
    fn t036a_der_boden_landet_in_der_grosskoerperliste() {
        // 400 x 400 m bei 8-m-Zellen sind 2500 Zellen — der Boden gehoert NICHT ins Gitter.
        let mut i = index();
        i.einfuegen(klotz(1, Vec3::new(0.0, -0.1, 0.0), Vec3::new(200.0, 0.1, 200.0)));
        i.einfuegen(klotz(2, Vec3::new(12.0, 8.0, -30.0), Vec3::new(6.0, 8.0, 6.0)));
        assert_eq!(i.anzahl_gross(), 1, "genau der Boden");
        assert_eq!(i.anzahl(), 2);
        assert!(i.entfernen(KoerperId(1)));
        assert_eq!(i.anzahl_gross(), 0);
    }

    #[test]
    fn t036a_ein_koerper_ausserhalb_des_gitters_geht_nicht_verloren() {
        let mut i = index();
        // Weit ausserhalb: faellt in keinen Zellbereich und landet in der Grossliste.
        i.einfuegen(klotz(3, Vec3::new(9000.0, 0.0, 9000.0), Vec3::splat(1.0)));
        assert_eq!(i.anzahl(), 1);
        assert!(i.koerper(KoerperId(3)).is_some());
        // Knapp ausserhalb: wird in die Randzelle geklemmt und bleibt auffindbar.
        i.einfuegen(klotz(4, Vec3::new(319.0, 0.0, -319.0), Vec3::splat(1.0)));
        assert_eq!(i.anzahl(), 2);
        assert!(i.entfernen(KoerperId(4)));
    }

    #[test]
    fn t036a_das_postfach_ueberlebt_beliebig_viele_frames() {
        // Genau der Fall, an dem `RemovedComponents` scheitert: mehrere Meldungen, erst
        // spaeter abgeholt.
        let mut i = index();
        i.abmelden(KoerperId(1));
        i.abmelden(KoerperId(2));
        i.abmelden(KoerperId(1));
        let offen = i.abmeldungen_holen();
        assert_eq!(offen, vec![KoerperId(1), KoerperId(2), KoerperId(1)]);
        assert!(i.abmeldungen_holen().is_empty(), "das Postfach wird beim Abholen geleert");
    }

    #[test]
    #[should_panic(expected = "welt.zelle_m")]
    fn t036a_eine_zellgroesse_von_null_kracht_beim_bau() {
        // Ohne diesen Abbruch waere es eine Division durch null im DDA, drei Systeme
        // spaeter und ohne Hinweis auf die Datei.
        let _ = RaumIndex::neu(0.0, 320.0, 64);
    }
}
