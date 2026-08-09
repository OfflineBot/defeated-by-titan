//! Mathe-Helfer — **nichts aendert sich pro Frame, alles pro Sekunde.**
//!
//! `* dt` allein reicht nicht. Die drei Fallen kosten je einen halben Tag, sehen alle wie
//! „das Spiel fuehlt sich auf meinem Rechner anders an" aus, und **im Netz sind sie
//! Desync** (`prompts/init.md` §11, §6 Regel 4):
//!
//! 1. **Nie auf Ganzzahlen runden.** `(schaden * dt).ceil()` macht die Framerate zur
//!    Schadenszahl. Bruchteile mittragen.
//! 2. **Exponentielles Glaetten ist pro Frame.** `x += (ziel - x) * 0.1` haengt an der
//!    Bildrate — [`glaetten`] benutzen.
//! 3. **Rauschen skaliert mit `sqrt(dt)`**, nicht mit `dt` — [`rausch_faktor`].
//!
//! Es gibt **eine** Hilfsfunktion pro Fall, und nur die wird benutzt. Zwei Formen fuer
//! dieselbe Sache heissen: keine Form.

use bevy::prelude::*;

/// Obergrenze fuer einen Zeitschritt.
///
/// Ein Frame kann 0,5 s dauern (Nachladen, Fenster verschoben, Blender im Hintergrund).
/// Ungeklemmt schiebt genau dieser Frame den Spieler durch die Wand und erzeugt NaN in den
/// Seilkraeften — der Bug, der aussieht wie „der Spieler ist verschwunden" (§9d).
pub const DT_MAX_S: f32 = 0.1;

pub fn dt_gezaehmt(dt_s: f32) -> f32 {
    if dt_s.is_finite() { dt_s.clamp(0.0, DT_MAX_S) } else { 0.0 }
}

/// Exponentielles Glaetten ueber eine **Halbwertszeit**, unabhaengig von der Bildrate.
///
/// `halbwertszeit_s` ist die Zeit, nach der die halbe Differenz abgebaut ist — eine Zahl,
/// die man in eine RON schreiben und im Kopf pruefen kann. `0` heisst sofort.
pub fn glaetten(aktuell: f32, ziel: f32, halbwertszeit_s: f32, dt_s: f32) -> f32 {
    if halbwertszeit_s <= 0.0 {
        return ziel;
    }
    let anteil = 1.0 - (-core::f32::consts::LN_2 * dt_gezaehmt(dt_s) / halbwertszeit_s).exp();
    aktuell + (ziel - aktuell) * anteil
}

pub fn glaetten_vec3(aktuell: Vec3, ziel: Vec3, halbwertszeit_s: f32, dt_s: f32) -> Vec3 {
    Vec3::new(
        glaetten(aktuell.x, ziel.x, halbwertszeit_s, dt_s),
        glaetten(aktuell.y, ziel.y, halbwertszeit_s, dt_s),
        glaetten(aktuell.z, ziel.z, halbwertszeit_s, dt_s),
    )
}

/// Skalierung fuer **Rauschen** (Kamerawackeln, Streuung): `sqrt(dt)`, nicht `dt`.
///
/// Rauschen ist ein Zufallsweg; seine Standardabweichung waechst mit der Wurzel der Zeit.
/// Mit `dt` skaliert wird es bei hoher Bildrate unsichtbar und bei niedriger zum Erdbeben.
pub fn rausch_faktor(dt_s: f32) -> f32 {
    dt_gezaehmt(dt_s).sqrt()
}

/// Sichere Normalisierung. `None` heisst „diese Richtung gibt es nicht" — und der Aufrufer
/// muss sich entscheiden, statt ein NaN weiterzureichen.
///
/// `Vec3::normalize` auf einem Nullvektor liefert NaN, und ein NaN im `Transform` ist der
/// Bug, den man erst drei Systeme spaeter sieht (§9d).
pub fn richtung(v: Vec3) -> Option<Vec3> {
    let l = v.length();
    if l.is_finite() && l > 1e-6 { Some(v / l) } else { None }
}

/// Ob ein Wert ueberhaupt eine Position sein kann. `debug/` warnt **einmal**, wenn nicht.
pub fn ist_endlich(v: Vec3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glaetten_ist_bildratenunabhaengig() {
        // Der eigentliche Zweck: derselbe Zeitraum muss dasselbe Ergebnis liefern,
        // egal in wie vielen Schritten er durchlaufen wird. Genau das kann
        // `x += (ziel-x)*0.1` NICHT — und genau das ist im Netz ein Desync.
        //
        // Beide Laeufe decken **eine Sekunde** ab, und beide bleiben mit ihrer
        // Schrittweite unter DT_MAX_S — sonst misst der Test die Klemme statt der
        // Glaettung (siehe den Test darunter).
        let ziel = 10.0;
        let hz = 0.25;

        let mut grob = 0.0;
        for _ in 0..12 {
            grob = glaetten(grob, ziel, hz, 1.0 / 12.0);
        }
        let mut fein = 0.0;
        for _ in 0..60 {
            fein = glaetten(fein, ziel, hz, 1.0 / 60.0);
        }
        assert!(
            (grob - fein).abs() < 1e-3,
            "12 Schritte ergaben {grob}, 60 Schritte {fein} — dieselbe Sekunde muss \
             dasselbe Ergebnis liefern"
        );
        // Vier Halbwertszeiten in einer Sekunde: 1 - 1/16 = 0,9375 von 10.
        assert!((fein - 9.375).abs() < 1e-3, "erwartet 9,375, war {fein}");
    }

    #[test]
    fn ein_ruckler_wird_geklemmt_statt_nachgeholt() {
        // Ueber DT_MAX_S hinaus ist die Bildratenunabhaengigkeit **absichtlich** verletzt:
        // ein 0,5-s-Frame darf nicht 0,5 s Bewegung auf einmal ausfuehren, sonst schiebt
        // genau dieser Frame den Spieler durch die Wand (§9d). Die Glaettung holt danach
        // nach, sie springt nicht.
        //
        // Diese Zeile steht hier, damit der Unterschied eine ENTSCHEIDUNG ist und nicht
        // eines Tages als Bug gemeldet wird.
        let ein_ruckler = glaetten(0.0, 10.0, 0.25, 0.5);
        let geklemmt = glaetten(0.0, 10.0, 0.25, DT_MAX_S);
        assert_eq!(
            ein_ruckler, geklemmt,
            "ein halbe-Sekunde-Frame muss wie DT_MAX_S wirken, nicht wie eine halbe Sekunde"
        );
        assert!(ein_ruckler < 3.0, "und er darf nicht fast bis ans Ziel springen");
    }

    #[test]
    fn glaetten_haelt_die_halbwertszeit_ein() {
        let x = glaetten(0.0, 1.0, 0.1, 0.1);
        assert!((x - 0.5).abs() < 1e-5, "nach einer Halbwertszeit erwartet 0,5, war {x}");
    }

    #[test]
    fn dt_wird_geklemmt_und_nan_wird_null() {
        assert_eq!(dt_gezaehmt(0.5), DT_MAX_S);
        assert_eq!(dt_gezaehmt(-1.0), 0.0);
        assert_eq!(dt_gezaehmt(f32::NAN), 0.0);
        assert_eq!(dt_gezaehmt(f32::INFINITY), 0.0);
    }

    #[test]
    fn glaetten_erzeugt_nie_nan() {
        // Der Sonderfall, nicht der Normalfall: ein Frame von 0,5 s und eine
        // Halbwertszeit von 0 sind genau die Werte, die im Spiel wirklich vorkommen.
        for hz in [0.0_f32, 1e-9, 0.5, 1e9] {
            for dt in [0.0_f32, 1.0 / 60.0, 0.5, f32::NAN] {
                let x = glaetten(1.0, 2.0, hz, dt);
                assert!(x.is_finite(), "hz {hz}, dt {dt} ergab {x}");
            }
        }
    }

    #[test]
    fn richtung_lehnt_den_nullvektor_ab() {
        assert!(richtung(Vec3::ZERO).is_none());
        assert!(richtung(Vec3::splat(f32::NAN)).is_none());
        assert!(richtung(Vec3::new(1e-9, 0.0, 0.0)).is_none());
        let d = richtung(Vec3::new(0.0, 0.0, -3.0)).expect("3 m sind eine Richtung");
        assert!((d - Vec3::NEG_Z).length() < 1e-6);
    }

    #[test]
    fn rauschen_skaliert_mit_der_wurzel() {
        // Viermal so langer Schritt heisst doppelt so viel Rauschen, nicht viermal.
        let a = rausch_faktor(1.0 / 240.0);
        let b = rausch_faktor(4.0 / 240.0);
        assert!((b / a - 2.0).abs() < 1e-4, "Verhaeltnis war {}", b / a);
    }
}
