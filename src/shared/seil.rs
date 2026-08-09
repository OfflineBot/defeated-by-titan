//! Der Seilzwang — die Rechnung, an der `F-004` und `F-005` haengen.
//!
//! **Reine Funktionen: kein Bevy ausser `bevy_math`, kein System, kein Schedule, kein `dt`.**
//! Der Kern der Sitzung muss ohne `App` pruefbar sein — ein Pendel, das man nur im
//! laufenden Spiel messen kann, ist ein Pendel, das niemand misst.
//!
//! `docs/architektur.md` uebersetzt `RopeConstraint` ausdruecklich zu „eigene Seilrechnung
//! gegen `Time<Fixed>`, keine Engine-Constraint". Der Rechenkern liegt hier in `shared/`,
//! weil er kein System ist und sein einziger Aufrufer in `player` sitzt — das spart eine
//! Kante in der Erlaubnisliste.
//!
//! ## Warum genau so, und nicht anders
//!
//! Zwei naheliegende Verfahren sind **durchgerechnet und verworfen**
//! (`docs/schnittstelle.md`, Abschnitt „Warum der Loeser so aussieht"):
//!
//! - **Reine Radialprojektion** (Position klemmen, auswaerts gerichteten Radialanteil
//!   streichen) verliert pro Tick den Faktor `1/sqrt(1 + (v*dt/L)^2)`. Bei `L = 3 m` und
//!   `75 m/s` sind das **99,2 % Tempo pro Sekunde** — genau im interessanten Bereich
//!   (kurzes Seil, hohes Tempo) schlaeft das Pendel ein. Waechter: [`tests`]
//!   `f004_kurzes_seil_bei_hohem_tempo_verliert_kaum_schwung`.
//! - **Feder/Penalty** (Beschleunigung aus der Abstandsverletzung) braucht fuer ±1 cm bei
//!   `L = 3 m, v = 75 m/s` eine Steifigkeit von `k/m ≈ 189 500 s^-2`; symplektisches Euler
//!   ist dort mit `omega*dt = 7,25` instabil (Verstaerkung −50,6 pro Tick) und laeuft in
//!   **0,41 s** nach NaN.
//!
//! Was hier steht: **Position auf die Kugel klemmen, Geschwindigkeit MITDREHEN.** Das
//! Mitdrehen erhaelt `|v|` exakt (eine Rotation ist laengentreu); danach wird der nach
//! aussen gerichtete Radialanteil gestrichen, denn **ein Seil zieht, es drueckt nicht**.
//! Dieser Ruck ist der physikalisch echte Moment des Straffwerdens und verschwindet, sobald
//! das Seil straff bleibt.

use bevy::math::{Quat, Vec3};

use super::mathe::richtung;

/// Zwei Haken, also zwei moegliche Seile — links und rechts.
pub const SEITEN: usize = 2;

/// Ein **straffes** Seil: wo es haengt und wie lang es sein darf.
///
/// Die Laenge ist eine Obergrenze, keine Sollgroesse: naeher am Anker darf der Koerper
/// jederzeit sein (dann ist das Seil schlaff und dieser Zwang tut nichts).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seilzwang {
    /// Ankerpunkt in **Weltkoordinaten**. Wer den Anker an einem beweglichen Koerper
    /// fuehrt, rechnet ihn vorher aus (`Koerper`-Mitte + `lokal_m`).
    pub anker_m: Vec3,
    /// Maximaler Abstand zum Anker in Metern. `<= 0` heisst „kein Zwang".
    pub laenge_m: f32,
}

/// Was ein Zwangsschritt aus Position und Geschwindigkeit gemacht hat.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zwangsergebnis {
    pub pos_m: Vec3,
    pub tempo_m_s: Vec3,
    /// Welche Seite in diesem Schritt wirklich straff wurde. Index wie in `zwaenge`:
    /// `0 = links`, `1 = rechts`. Das ist die Groesse, aus der der Aufrufer
    /// `Bewegungszustand::AmSeil` ableitet — nicht „ein Haken haelt".
    pub gespannt: [bool; SEITEN],
}

/// Haelt `pos_frei_m` in allen straffen Seilkugeln und dreht die Geschwindigkeit mit.
///
/// Der Aufrufer integriert selbst (`pos_frei_m = pos_alt_m + tempo * dt`) und uebergibt
/// **beide** Positionen: `pos_alt_m` ist der Bezugspunkt fuer die Drehung, `pos_frei_m` das
/// ungehinderte Ergebnis des Schritts.
///
/// `durchlaeufe` ist die Zahl der Gauss-Seidel-Durchlaeufe ueber beide Zwaenge; sie kommt
/// aus `assets/data/game.ron` (`vector.seil_durchlaeufe`), **nicht aus dem Code**. Mit einem
/// Durchlauf verletzt der zweite Zwang den ersten wieder.
///
/// **Reihenfolge ist fest** (links, dann rechts): dieselbe Eingabe ergibt bitgleich dasselbe
/// Ergebnis, auf jedem Rechner, in jedem Rollback.
///
/// Gutartig gegen Unsinn: nicht-endliche Eingaben, Anker exakt auf der Position, Laenge
/// `<= 0` und der entartete Fall „Koerper ist in einem Tick durch den Anker gefallen"
/// erzeugen kein NaN.
pub fn seil_schritt(
    pos_alt_m: Vec3,
    pos_frei_m: Vec3,
    tempo_m_s: Vec3,
    zwaenge: [Option<Seilzwang>; SEITEN],
    durchlaeufe: u32,
) -> Zwangsergebnis {
    let unveraendert = Zwangsergebnis {
        pos_m: pos_frei_m,
        tempo_m_s,
        gespannt: [false; SEITEN],
    };
    if !(pos_alt_m.is_finite() && pos_frei_m.is_finite() && tempo_m_s.is_finite()) {
        return unveraendert;
    }

    let mut pos = pos_frei_m;
    let mut gespannt = [false; SEITEN];

    // Mindestens ein Durchlauf, sonst waere ein `0` in der RON ein stillgelegtes Seil.
    for _ in 0..durchlaeufe.max(1) {
        for (i, zwang) in zwaenge.iter().enumerate() {
            let Some(z) = zwang else { continue };
            if !gueltig(z) {
                continue;
            }
            let d = pos - z.anker_m;
            if d.length() > z.laenge_m {
                let Some(dir) = richtung(d) else { continue };
                pos = z.anker_m + dir * z.laenge_m;
                gespannt[i] = true;
            }
        }
    }

    // Die Geschwindigkeit wird EINMAL je gespanntem Seil nachgezogen, nach dem letzten
    // Durchlauf. Innerhalb der Durchlaeufe waere es eine mehrfache Drehung um denselben
    // Winkel — sichtbar als Schleudern.
    let mut tempo = tempo_m_s;
    for (i, zwang) in zwaenge.iter().enumerate() {
        if !gespannt[i] {
            continue;
        }
        let Some(z) = zwang else { continue };
        let (Some(dir_alt), Some(dir_neu)) =
            (richtung(pos_alt_m - z.anker_m), richtung(pos - z.anker_m))
        else {
            continue;
        };

        // `Quat::from_rotation_arc` waehlt bei `dir_alt ≈ -dir_neu` eine BELIEBIGE
        // Drehachse (`from_axis_angle(from.any_orthonormal_vector(), PI)`,
        // glam-0.32.1/src/f32/sse2/quat.rs:337-340). Deterministisch, aber physikalisch
        // sinnlos: der Koerper waere in einem Tick durch den Anker gefallen. Dann wird
        // nicht gedreht, sondern nur der Radialanteil gestrichen.
        if dir_alt.dot(dir_neu) > -0.999 {
            tempo = Quat::from_rotation_arc(dir_alt, dir_neu) * tempo;
        }

        // Ein Seil zieht, es drueckt nicht.
        let auswaerts = tempo.dot(dir_neu);
        if auswaerts > 0.0 {
            tempo -= auswaerts * dir_neu;
        }
    }

    if !tempo.is_finite() || !pos.is_finite() {
        return unveraendert;
    }

    Zwangsergebnis { pos_m: pos, tempo_m_s: tempo, gespannt }
}

/// Seilverkuerzung, die **Arbeit verrichtet** (`F-005`).
///
/// Ein Seil einzuholen erhaelt den Drehimpuls: die **tangentiale** Geschwindigkeit skaliert
/// mit `laenge_alt / laenge_neu`. Ohne das gewinnt der Spieler Hoehe, aber kein Tempo — und
/// genau das Tempo ist das Gefuehl, an dem das ganze Spiel haengt (`F-005`: „Spieler kann
/// aus dem Tiefpunkt Hoehe gewinnen", Bibel P1).
///
/// Gedeckelt wird ausschliesslich durch `tempo_max_m_s` (`F-012`, aus
/// `assets/data/game.ron`) — und durch `vector.seil_min_m`, das der Aufrufer beim
/// Fortschreiben der Laenge anwendet. Im Code steht keine dieser Zahlen.
///
/// Die radiale Komponente bleibt unangetastet: das Heranziehen selbst passiert ueber die
/// kuerzere Laenge in [`seil_schritt`], nicht ueber eine zweite Geschwindigkeit.
pub fn seil_einholen(
    anker_m: Vec3,
    pos_m: Vec3,
    tempo_m_s: Vec3,
    laenge_alt_m: f32,
    laenge_neu_m: f32,
    tempo_max_m_s: f32,
) -> Vec3 {
    if !(anker_m.is_finite() && pos_m.is_finite() && tempo_m_s.is_finite()) {
        return tempo_m_s;
    }
    if !(laenge_alt_m.is_finite() && laenge_neu_m.is_finite())
        || laenge_alt_m <= 0.0
        || laenge_neu_m <= 0.0
    {
        return tempo_m_s;
    }
    let Some(dir) = richtung(pos_m - anker_m) else {
        return tempo_m_s;
    };

    let radial = tempo_m_s.dot(dir) * dir;
    let tangential = tempo_m_s - radial;
    let neu = radial + tangential * (laenge_alt_m / laenge_neu_m);

    if !neu.is_finite() {
        return tempo_m_s;
    }
    if tempo_max_m_s.is_finite() && tempo_max_m_s > 0.0 {
        neu.clamp_length_max(tempo_max_m_s)
    } else {
        neu
    }
}

fn gueltig(z: &Seilzwang) -> bool {
    z.anker_m.is_finite() && z.laenge_m.is_finite() && z.laenge_m > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn eins(anker_m: Vec3, laenge_m: f32) -> [Option<Seilzwang>; SEITEN] {
        [Some(Seilzwang { anker_m, laenge_m }), None]
    }

    /// Ein Tick: der Aufrufer integriert, der Zwang korrigiert.
    fn tick(
        pos: Vec3,
        tempo: Vec3,
        zwaenge: [Option<Seilzwang>; SEITEN],
        durchlaeufe: u32,
    ) -> Zwangsergebnis {
        seil_schritt(pos, pos + tempo * DT, tempo, zwaenge, durchlaeufe)
    }

    #[test]
    fn f004_ohne_zwang_ist_der_schritt_die_identitaet() {
        let r = seil_schritt(Vec3::ZERO, Vec3::new(1.0, 2.0, 3.0), Vec3::X, [None, None], 2);
        assert_eq!(r.pos_m, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(r.tempo_m_s, Vec3::X);
        assert_eq!(r.gespannt, [false, false]);
    }

    #[test]
    fn f004_ein_schlaffes_seil_aendert_nichts() {
        // Der Koerper bleibt innerhalb der Kugel — dann darf kein Ruck entstehen.
        let anker = Vec3::new(0.0, 20.0, 0.0);
        let r = tick(Vec3::ZERO, Vec3::new(0.0, 5.0, 0.0), eins(anker, 30.0), 2);
        assert_eq!(r.gespannt, [false, false]);
        assert_eq!(r.tempo_m_s, Vec3::new(0.0, 5.0, 0.0));
    }

    #[test]
    fn f004_der_abstand_bleibt_auf_der_kugel() {
        let anker = Vec3::ZERO;
        let l = 12.0;
        let r = tick(Vec3::new(l, 0.0, 0.0), Vec3::new(30.0, 0.0, 0.0), eins(anker, l), 2);
        assert!(r.gespannt[0], "ein Seil, das straff sein muss, war es nicht");
        let abstand = (r.pos_m - anker).length();
        assert!((abstand - l).abs() < 1e-4, "Abstand {abstand} statt {l}");
    }

    #[test]
    fn f004_ein_seil_zieht_und_drueckt_nicht() {
        // Rein radial nach aussen: nach dem Straffwerden bleibt davon nichts uebrig.
        let anker = Vec3::ZERO;
        let l = 10.0;
        let r = tick(Vec3::new(l, 0.0, 0.0), Vec3::new(40.0, 0.0, 0.0), eins(anker, l), 2);
        assert!(r.tempo_m_s.length() < 1e-3, "Restgeschwindigkeit {}", r.tempo_m_s);
    }

    #[test]
    fn f004_kurzes_seil_bei_hohem_tempo_verliert_kaum_schwung() {
        // **Der Waechter gegen die reine Radialprojektion.** Die verliert bei L = 3 m und
        // 75 m/s 99,2 % pro Sekunde; das Mitdrehen erhaelt |v| exakt. Ohne Schwerkraft,
        // damit nur der Loeser gemessen wird.
        let anker = Vec3::ZERO;
        let l = 3.0;
        let mut pos = Vec3::new(l, 0.0, 0.0);
        let mut tempo = Vec3::new(0.0, 0.0, 75.0);
        for _ in 0..60 {
            let r = tick(pos, tempo, eins(anker, l), 2);
            pos = r.pos_m;
            tempo = r.tempo_m_s;
        }
        let betrag = tempo.length();
        assert!(
            betrag > 75.0 * 0.99,
            "nach einer Sekunde noch {betrag} m/s statt ~75 — der Loeser frisst Schwung"
        );
        let abstand = (pos - anker).length();
        assert!((abstand - l).abs() < 1e-3, "Abstand {abstand} statt {l}");
    }

    #[test]
    fn f004_zwei_anker_halten_beide_kugeln() {
        // Der Fall, nach dem F-004 benannt ist. Zwei Daecher, der Koerper haengt darunter
        // und wird zwischen beide gezogen.
        let a = Vec3::new(-8.0, 14.0, 0.0);
        let b = Vec3::new(8.0, 14.0, 0.0);
        let zwaenge = [
            Some(Seilzwang { anker_m: a, laenge_m: 18.0 }),
            Some(Seilzwang { anker_m: b, laenge_m: 18.0 }),
        ];
        let r = tick(Vec3::new(0.0, -2.0, 0.0), Vec3::new(0.0, -25.0, 0.0), zwaenge, 8);
        assert_eq!(r.gespannt, [true, true]);
        assert!((r.pos_m - a).length() <= 18.0 + 1e-3, "linkes Seil ueberdehnt");
        assert!((r.pos_m - b).length() <= 18.0 + 1e-3, "rechtes Seil ueberdehnt");
        assert!(r.tempo_m_s.is_finite());
    }

    #[test]
    fn f004_der_entartete_fall_erzeugt_kein_nan() {
        // Ein `warp` kann den Koerper in einem Schritt durch den Anker setzen; dann ist
        // `dir_alt ≈ -dir_neu` und `from_rotation_arc` waehlt eine beliebige Achse.
        let anker = Vec3::ZERO;
        let l = 4.0;
        let r = seil_schritt(
            Vec3::new(l, 0.0, 0.0),
            Vec3::new(-40.0, 0.0, 0.0),
            Vec3::new(-30.0, 0.0, 0.0),
            eins(anker, l),
            2,
        );
        assert!(r.pos_m.is_finite() && r.tempo_m_s.is_finite());
        assert!(((r.pos_m - anker).length() - l).abs() < 1e-4);
    }

    #[test]
    fn f004_unsinnige_eingaben_kommen_unveraendert_zurueck() {
        let kaputt = Vec3::new(f32::NAN, 0.0, 0.0);
        let r = seil_schritt(kaputt, Vec3::ZERO, Vec3::X, eins(Vec3::ZERO, 5.0), 2);
        assert_eq!(r.tempo_m_s, Vec3::X);
        assert_eq!(r.gespannt, [false, false]);

        // Laenge 0 ist „kein Zwang", keine Division durch null.
        let r = tick(Vec3::new(5.0, 0.0, 0.0), Vec3::X * 10.0, eins(Vec3::ZERO, 0.0), 2);
        assert_eq!(r.gespannt, [false, false]);
        assert!(r.pos_m.is_finite());
    }

    #[test]
    fn f005_einholen_beschleunigt_tangential() {
        // Drehimpulserhaltung: halbe Laenge, doppeltes Tangentialtempo.
        let anker = Vec3::ZERO;
        let pos = Vec3::new(30.0, 0.0, 0.0);
        let neu = seil_einholen(anker, pos, Vec3::new(0.0, 0.0, 20.0), 30.0, 15.0, 75.0);
        assert!((neu.z - 40.0).abs() < 1e-4, "tangential {neu:?}, erwartet 40 m/s");
        assert!(neu.x.abs() < 1e-6, "radial darf sich nicht aendern");
    }

    #[test]
    fn f005_einholen_laesst_die_radiale_komponente_stehen() {
        let anker = Vec3::ZERO;
        let pos = Vec3::new(20.0, 0.0, 0.0);
        let neu = seil_einholen(anker, pos, Vec3::new(-6.0, 0.0, 10.0), 20.0, 10.0, 75.0);
        assert!((neu.x + 6.0).abs() < 1e-4, "radial {neu:?}");
        assert!((neu.z - 20.0).abs() < 1e-4, "tangential {neu:?}");
    }

    #[test]
    fn f005_einholen_deckelt_auf_tempo_max() {
        let neu = seil_einholen(
            Vec3::ZERO,
            Vec3::new(40.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 60.0),
            40.0,
            4.0,
            75.0,
        );
        assert!(neu.length() <= 75.0 + 1e-3, "Betrag {}", neu.length());
    }

    #[test]
    fn f005_ohne_laengenaenderung_aendert_einholen_nichts() {
        let t = Vec3::new(3.0, -4.0, 12.0);
        let neu = seil_einholen(Vec3::ZERO, Vec3::new(9.0, 0.0, 0.0), t, 9.0, 9.0, 75.0);
        assert!((neu - t).length() < 1e-4, "{neu:?} statt {t:?}");
    }

    #[test]
    fn f005_einholen_mit_unsinn_gibt_das_tempo_unveraendert_zurueck() {
        let t = Vec3::new(1.0, 2.0, 3.0);
        // Laenge null: keine Division durch null.
        assert_eq!(seil_einholen(Vec3::ZERO, Vec3::X * 5.0, t, 10.0, 0.0, 75.0), t);
        // Anker exakt auf der Position: keine Richtung, also keine Rechnung.
        assert_eq!(seil_einholen(Vec3::ZERO, Vec3::ZERO, t, 10.0, 5.0, 75.0), t);
        // NaN kommt nicht heraus, wenn keins hineingeht.
        assert_eq!(seil_einholen(Vec3::ZERO, Vec3::X * 5.0, t, f32::NAN, 5.0, 75.0), t);
    }
}
