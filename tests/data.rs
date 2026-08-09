//! Der Waechter ueber die RON-Dateien.
//!
//! **Zahlen gehoeren in RON, nicht in Rust** (`prompts/init.md` §4) — und genau deshalb
//! braucht die RON einen Test. Eine Zahl im Code faengt der Compiler ab; eine Zahl in einer
//! Datei faengt niemand ab, ausser hier.
//!
//! Geprueft wird nicht „ist es huebsch", sondern **was die Bibel als bindend festlegt** und
//! **was in sich stimmen muss** (jeder Verweis zeigt auf etwas, das es gibt).

use defeated_by_titan::data::{GameData, Gasverbraucher};
use std::path::PathBuf;

fn daten() -> GameData {
    GameData::laden(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data"))
}

#[test]
fn t005_alle_ron_dateien_laden() {
    // Kein `serde(default)` fuer Spielwerte: ein fehlender Wert soll krachen. Dieser Test
    // ist die Stelle, an der das Krachen kein Spielabbruch ist, sondern ein roter Test.
    let d = daten();
    assert!(!d.titanen.arten.is_empty(), "titan.ron ohne eine einzige Art");
    assert!(!d.art.models.is_empty(), "art.ron ohne ein einziges Modell");
    assert!(!d.karten.karten.is_empty(), "maps.ron ohne eine einzige Karte");
    assert!(!d.karten.palette.is_empty(), "maps.ron ohne eine einzige Farbe");
}

#[test]
fn t005_jeder_titan_holt_mindestens_vier_zehntel_sekunden_aus() {
    // Bibel, Pfeiler P4 (Lesbarkeit vor Realismus): „Jeder Titanenangriff hat eine
    // Ausholphase von mindestens 0,4 Sekunden." Das ist keine Empfehlung — der Spieler soll
    // nie fragen, warum er gestorben ist.
    for (name, art) in &daten().titanen.arten {
        assert!(
            art.ausholphase_s >= 0.4,
            "{name}: ausholphase_s = {} — die Bibel verlangt mindestens 0,4 s",
            art.ausholphase_s
        );
    }
}

#[test]
fn t005_jeder_titan_verweist_auf_ein_modell_das_es_gibt() {
    // Ein Verweis ins Leere ist ein Bug derselben Klasse wie ein toter Link in der Doku
    // (§10). Er faellt sonst erst auf, wenn genau dieser Titan gespawnt wird.
    let d = daten();
    for (name, art) in &d.titanen.arten {
        assert!(
            d.art.models.contains_key(&art.modell),
            "{name} verweist auf Modell {:?}, das in art.ron nicht steht",
            art.modell
        );
    }
}

#[test]
fn t005_jede_spawnwelle_nennt_eine_titanenart_die_es_gibt() {
    let d = daten();
    for (mission, vorlage) in &d.missionen.vorlagen {
        for welle in &vorlage.wellen {
            assert!(
                d.titanen.arten.contains_key(&welle.art),
                "Mission {mission:?}: Welle bei {}s will {:?}, das steht nicht in titan.ron",
                welle.bei_s,
                welle.art
            );
            assert!(welle.anzahl > 0, "Mission {mission:?}: eine Welle mit null Titanen");
        }
    }
}

#[test]
fn t005_der_missionsbogen_dauert_fuenf_bis_sieben_minuten() {
    // Bibel 5, Aenderung 10: Ø Missionsdauer 5–7 min. Jede Mission muss ein vollstaendiger
    // Bogen mit spuerbarem Fortschritt sein.
    for (name, v) in &daten().missionen.vorlagen {
        assert!(
            (300.0..=420.0).contains(&v.dauer_ziel_s),
            "{name}: dauer_ziel_s = {} — die Bibel will 5–7 min (300–420 s)",
            v.dauer_ziel_s
        );
        assert!(
            v.wellen.iter().all(|w| w.bei_s <= v.dauer_ziel_s),
            "{name}: eine Welle spawnt nach dem Ende der Mission"
        );
    }
}

#[test]
fn t005_die_simulation_laeuft_mit_sechzig_hertz() {
    // §6 Regel 4: fester Simulationsschritt. Im Netz ist ein frameabhaengiges Ergebnis kein
    // Komfortproblem, sondern Desync — und dann ist es zu spaet, das zu aendern.
    let hz = daten().spiel.simulation_hz;
    assert!((hz - 60.0).abs() < 1e-9, "simulation_hz = {hz}, erwartet 60");
}

#[test]
fn t005_die_hakenreichweite_bleibt_im_entwurfsfenster() {
    // init.md §1 nennt 60–120 m. Faellt dieser Test, hat jemand die Reichweite getunt, ohne
    // die Herkunft nachzuziehen. Der engere Waechter steht direkt darunter.
    let r = daten().spiel.vector.hakenreichweite_m;
    assert!((60.0..=120.0).contains(&r), "hakenreichweite_m = {r} — init.md §1 nennt 60–120 m");
}

#[test]
fn t005_kein_wert_ist_null_negativ_oder_nan() {
    // Der Sonderfall, nicht der Normalfall: eine Null in einer Tankgroesse ist eine
    // Division durch null drei Systeme spaeter (§9d).
    let d = daten();
    let v = &d.spiel.vector;
    let positiv = [
        ("gas_tank", v.gas_tank),
        ("hakenflug_m_s", v.hakenflug_m_s),
        ("haken_ruecklauf_m_s", v.haken_ruecklauf_m_s),
        ("seilzug_m_s", v.seilzug_m_s),
        ("seil_min_m", v.seil_min_m),
        ("boost_m_s2", v.boost_m_s2),
        ("tempo_max_m_s", v.tempo_max_m_s),
        ("spieler.hoehe_m", d.spiel.spieler.hoehe_m),
        ("spieler.laufen_m_s", d.spiel.spieler.laufen_m_s),
        ("spieler.schritt_max_m", d.spiel.spieler.schritt_max_m),
        ("kamera.sicht_grad", d.spiel.kamera.sicht_grad),
        ("welt.zelle_m", d.spiel.welt.zelle_m),
        ("welt.halbe_ausdehnung_m", d.spiel.welt.halbe_ausdehnung_m),
        ("welt.wand_min_m", d.spiel.welt.wand_min_m),
        ("welt.kollision_haut_m", d.spiel.welt.kollision_haut_m),
    ];
    for (name, wert) in positiv {
        assert!(wert.is_finite() && wert > 0.0, "{name} = {wert} — muss endlich und > 0 sein");
    }
    assert!(
        d.spiel.schwerkraft_m_s2 < 0.0,
        "schwerkraft_m_s2 = {} — nach unten heisst negativ, +Y ist oben",
        d.spiel.schwerkraft_m_s2
    );
    for (name, art) in &d.titanen.arten {
        let hoehe = d
            .titan_hoehe_m(art)
            .unwrap_or_else(|| panic!("{name}: Groessenklasse {:?} gibt es nicht", art.groessenklasse));
        assert!(hoehe > 0.0, "{name}: hoehe_m = {hoehe}");
        assert!(
            art.cortex_radius_m > 0.0,
            "{name}: cortex_radius_m = 0 — ein Cortex, der ein Punkt ist, fuehlt sich wie \
             ein kaputtes Spiel an (docs/modelle.md)",

        );
    }
}

#[test]
fn t005_die_augenhoehe_liegt_im_koerper() {
    let s = &daten().spiel.spieler;
    assert!(
        s.augenhoehe_m > 0.0 && s.augenhoehe_m < s.hoehe_m,
        "augenhoehe_m = {} passt nicht zu hoehe_m = {}",
        s.augenhoehe_m,
        s.hoehe_m
    );
}

#[test]
fn t005_der_teilschritt_ist_kleiner_als_die_duennste_wand() {
    // Der Kern der Tunnelsicherung. Bei tempo_max_m_s = 75 legt ein 60-Hz-Tick 1,25 m
    // zurueck; ohne Teilschritte faehrt der Spieler durch jede Wand — und zwar nur
    // manchmal, was die schlechteste Sorte Fehler ist (F-012).
    let d = daten();
    let schritt = d.spiel.spieler.schritt_max_m;
    let wand = d.spiel.welt.wand_min_m;
    assert!(
        schritt < wand,
        "schritt_max_m = {schritt} >= wand_min_m = {wand} — jeder Teilschritt kann eine \
         Wand ueberspringen"
    );
    assert!(
        d.spiel.welt.kollision_haut_m < schritt,
        "kollision_haut_m = {} muss kleiner als ein Teilschritt sein",
        d.spiel.welt.kollision_haut_m
    );
    // Und die Zahl der Teilschritte bei Hoechsttempo muss endlich und klein bleiben.
    let teilschritte = (d.spiel.vector.tempo_max_m_s / 60.0 / schritt).ceil();
    assert!(
        (1.0..=32.0).contains(&teilschritte),
        "{teilschritte} Teilschritte pro Tick bei Hoechsttempo — zu viele oder gar keiner"
    );
}

#[test]
fn t005_das_gitter_deckt_karte_und_hakenreichweite_ab() {
    // Ein Anker ausserhalb des Gitters landet in der Randzelle und liegt damit falsch.
    // Das Gitter muss die halbe Karte PLUS eine volle Hakenreichweite tragen (T-036a).
    let d = daten();
    let w = &d.spiel.welt;
    assert!(w.zelle_m > 0.0, "welt.zelle_m = {} — Division durch null im DDA", w.zelle_m);
    assert!(w.grosskoerper_zellen >= 1, "grosskoerper_zellen = 0 legt jeden Koerper in die \
         lineare Liste — das ist genau die Iteration, die §11 verbietet");
    for (name, karte) in &d.karten.karten {
        let noetig = karte.groesse_m.0.max(karte.groesse_m.1) * 0.5
            + d.spiel.vector.hakenreichweite_m;
        assert!(
            w.halbe_ausdehnung_m >= noetig,
            "{name}: halbe_ausdehnung_m = {} deckt {noetig} m nicht ab",
            w.halbe_ausdehnung_m
        );
    }
    // Eine Zelle, die groesser als die Karte ist, waere ein Gitter mit einer Zelle.
    assert!(w.zelle_m < w.halbe_ausdehnung_m, "welt.zelle_m = {} ist kein Gitter", w.zelle_m);
}

#[test]
fn t005_die_gasrangfolge_nennt_jeden_verbraucher_genau_einmal() {
    // Wer bei knappem Tank zahlt, ist eine Spielwertentscheidung (docs/FRAGEN.md Q-017).
    // Fehlt ein Verbraucher, bekommt er nie Gas und niemand sucht in der RON danach.
    let r = &daten().spiel.vector.gas_rangfolge;
    for wer in [Gasverbraucher::Boost, Gasverbraucher::Einholen] {
        assert_eq!(
            r.iter().filter(|x| **x == wer).count(),
            1,
            "gas_rangfolge = {r:?} — {wer:?} muss genau einmal vorkommen"
        );
    }
    assert_eq!(r.len(), 2, "gas_rangfolge = {r:?} — genau zwei Verbraucher, nicht mehr");
}

#[test]
fn t005_das_seil_wird_mindestens_einmal_geloest() {
    // Null Durchlaeufe waeren ein stillgelegtes Seil; einer laesst bei zwei Ankern den
    // zweiten Zwang den ersten wieder verletzen.
    let n = daten().spiel.vector.seil_durchlaeufe;
    assert!((1..=16).contains(&n), "seil_durchlaeufe = {n} — erwartet 1..16");
    assert!(n >= 2, "seil_durchlaeufe = {n} — mit einem Durchlauf ist der Zwei-Haken-Fall \
         (F-004) nach einem Tick verletzt");
}

#[test]
fn t005_die_aktuelle_karte_steht_in_maps_ron() {
    let d = daten();
    assert!(
        d.aktuelle_karte().is_some(),
        "maps.ron: aktuell = {:?}, aber diese Karte steht nicht unter `karten`",
        d.karten.aktuell
    );
}

#[test]
fn t005_jeder_klotz_nennt_eine_farbe_aus_der_palette() {
    // Ein Verweis ins Leere ist ein Bug derselben Klasse wie ein toter Link in der Doku.
    // Ohne diesen Test faellt er erst auf, wenn genau dieser Klotz gebaut wird.
    let d = daten();
    for (id, karte) in &d.karten.karten {
        for (i, k) in karte.kloetze.iter().enumerate() {
            assert!(
                d.farbe(&k.farbe).is_some(),
                "{id}: Klotz {i} nennt Farbe {:?}, die nicht in der Palette steht",
                k.farbe
            );
        }
        for farbe in &karte.raster.farben {
            assert!(
                d.farbe(farbe).is_some(),
                "{id}: raster.farben nennt {farbe:?}, das nicht in der Palette steht"
            );
        }
        assert!(!karte.raster.farben.is_empty(), "{id}: raster ohne eine einzige Farbe");
    }
}

#[test]
fn t005_jede_karte_ist_baubar() {
    // Die Zahlen, aus denen `world` die Stadt erzeugt: eine Null oder ein vertauschter
    // Hoehenbereich waere eine leere oder eine unendliche Stadt.
    let d = daten();
    for (id, karte) in &d.karten.karten {
        let r = &karte.raster;
        assert!(karte.groesse_m.0 > 0.0 && karte.groesse_m.1 > 0.0, "{id}: groesse_m = {:?}",
                karte.groesse_m);
        assert!(r.block_m > 0.0 && r.gasse_m > 0.0, "{id}: block_m/gasse_m muessen > 0 sein");
        assert!(
            r.hoehe_min_m > 0.0 && r.hoehe_min_m < r.hoehe_max_m,
            "{id}: hoehe_min_m = {} / hoehe_max_m = {}", r.hoehe_min_m, r.hoehe_max_m
        );
        assert!((0.0..=1.0).contains(&r.dichte), "{id}: dichte = {} liegt nicht in 0..1", r.dichte);
        assert!(
            (0.0..=1.0).contains(&r.hakbar_anteil),
            "{id}: hakbar_anteil = {} liegt nicht in 0..1", r.hakbar_anteil
        );
        assert!(r.frei_radius_m > 0.0, "{id}: frei_radius_m = 0 baut ein Haus auf den Spieler");
        // Eine Gasse breiter als ein Block ist keine Stadt, sondern ein Feld mit Kloetzen.
        assert!(r.gasse_m < r.block_m, "{id}: gasse_m {} >= block_m {}", r.gasse_m, r.block_m);

        for (i, k) in karte.kloetze.iter().enumerate() {
            let g = k.groesse_m;
            assert!(
                g.0 > 0.0 && g.1 > 0.0 && g.2 > 0.0,
                "{id}: Klotz {i} hat groesse_m = {g:?} — ein Quader ohne Ausdehnung ist \
                 kein Hindernis, sondern eine Division durch null"
            );
            assert!(
                k.fest || k.hakbar,
                "{id}: Klotz {i} ist weder fest noch hakbar — er waere unsichtbar fuer \
                 jedes System, das den Index fragt"
            );
        }
    }
}

#[test]
fn t005_die_graubox_traegt_hakbare_und_ungetaggte_flaechen() {
    // Erst damit ist „Kein Haken auf ungetaggten Parts moeglich" (F-003) ueberhaupt
    // fahrbar: eine Karte, auf der alles hakbar ist, kann das Kriterium nicht widerlegen.
    let d = daten();
    let karte = d.aktuelle_karte().expect("aktuelle Karte");
    let hakbar = karte.kloetze.iter().filter(|k| k.hakbar).count();
    let ungetaggt = karte.kloetze.iter().filter(|k| !k.hakbar).count();
    assert!(hakbar > 0, "keine einzige hakbare Flaeche auf der Startkarte");
    assert!(
        ungetaggt > 0,
        "alles ist hakbar — dann prueft F-003 nichts (docs/FRAGEN.md Q-010)"
    );
}

#[test]
fn t005_jedes_fremde_asset_traegt_seine_herkunft() {
    // §7: ohne `herkunft` ist ein fremdes Asset ein Zombie — der User kann es spaeter nicht
    // finden, um es zu ersetzen. Solange alles Platzhalter ist, ist die Liste leer; genau
    // dann muss dieser Test trotzdem existieren, damit das erste Fremdmodell auffaellt.
    for (name, modell) in &daten().art.models {
        if let Some(h) = &modell.herkunft {
            assert!(
                h.contains("http") && h.contains("20"),
                "{name}: herkunft {h:?} — erwartet URL · Datum · Lizenz · was es ersetzt"
            );
        }
        assert!(modell.scale > 0.0, "{name}: scale = {}", modell.scale);
    }
}

// ===========================================================================
// Der Massstab — assets/data/massstab.ron ist die EINE Wahrheit ueber Groessen
// ===========================================================================
//
// Die Zahlen darunter hat der **User** vorgegeben (2026-08-09). Sie sind nicht ungetunt,
// sie sind entschieden. Ein Wert in einer Datei, den niemand prueft, driftet weg — und
// Groessen driften besonders leise, weil ein zu hohes Haus nicht abstuerzt, sondern nur
// falsch aussieht. Deshalb steht hinter jeder Groesse hier ein Test, der ROT wird.
//
// Die Vorrangregel, die diese Tests durchsetzen: **eine direkte Meterangabe des Users
// schlaegt jede Ableitung** — auch die Umrechnung aus dem Backlog (docs/FRAGEN.md Q-002).

#[test]
fn t005_die_spielerkapsel_ist_exakt_das_referenzmass_mensch() {
    // Faengt: eine Spielerkapsel, die vom Referenzmass 1,80 m abweicht — der User schreibt
    // ausdruecklich „Kapsel exakt pruefen!".
    // Ohne diesen Test: jeder Groessenvergleich im Bild waere um denselben Faktor daneben.
    // Eine 1,9-m-Kapsel laesst einen 21-m-Titanen wie 20 m aussehen, die Mauer wie 114 m —
    // und man wuerde am Titanen tunen, obwohl der Fehler beim Spieler sitzt. Genau die
    // Sorte Fehler, die man nur findet, wenn man sie einmal aufschreibt.
    let d = daten();
    assert_eq!(
        d.spiel.spieler.hoehe_m, d.massstab.referenz.mensch_hoehe_m,
        "spieler.hoehe_m = {} weicht vom Referenzmass Mensch = {} ab (massstab.ron)",
        d.spiel.spieler.hoehe_m, d.massstab.referenz.mensch_hoehe_m
    );
}

#[test]
fn t005_die_augenhoehe_ist_die_kamerahoehe_des_massstabs() {
    // Faengt: eine Augenhoehe, die nicht die vorgegebene Kamerahoehe (1,60 m) ist, und eine
    // Augenhoehe oberhalb des Scheitels.
    // Ohne diesen Test: die 1,65 m von frueher waeren einfach stehengeblieben. Sie waren aus
    // der Koerperhoehe geschaetzt, und eine Schaetzung faellt neben einer Vorgabe nicht auf —
    // sie ist ja „ungefaehr richtig". Fuenf Zentimeter Kamerahoehe sind bei 55–65 Grad
    // Sichtfeld genau der Unterschied zwischen „ich stehe davor" und „ich schwebe davor".
    let d = daten();
    let s = &d.spiel.spieler;
    assert_eq!(
        s.augenhoehe_m, d.massstab.kamera.hoehe_m,
        "spieler.augenhoehe_m = {} != massstab.ron kamera.hoehe_m = {}",
        s.augenhoehe_m, d.massstab.kamera.hoehe_m
    );
    assert!(
        s.augenhoehe_m < s.hoehe_m,
        "augenhoehe_m = {} liegt nicht unter hoehe_m = {} — die Kamera schwebt ueber dem Kopf",
        s.augenhoehe_m, s.hoehe_m
    );
}

#[test]
fn t005_die_hakenreichweite_ist_die_ankerreichweite_des_users() {
    // Faengt: jeden Rueckfall auf die abgeleiteten 112 m (400 studs × 0,28, Q-002). Der User
    // gibt 90 m direkt an, und eine direkte Meterangabe schlaegt jede Ableitung.
    // Ohne diesen Test: die 112 kaemen beim naechsten „ich rechne das nochmal aus dem
    // Backlog nach" zurueck — mit Begruendung und Quellenangabe, also besonders ueberzeugend.
    // Die Reichweite bestimmt, ob die Mauer in zwei Zuegen erreichbar ist und wie gross das
    // raeumliche Gitter sein muss; sie still um 24 % zu aendern, verschiebt beides.
    let d = daten();
    assert_eq!(
        d.spiel.vector.hakenreichweite_m, d.massstab.vector.ankerreichweite_m,
        "vector.hakenreichweite_m = {} != massstab.ron vector.ankerreichweite_m = {}",
        d.spiel.vector.hakenreichweite_m, d.massstab.vector.ankerreichweite_m
    );
}

#[test]
fn t005_das_sichtfeld_am_boden_bleibt_im_fenster_des_users() {
    // Faengt: ein Sichtfeld ausserhalb 55–65 Grad. Der User nennt es „groesster Hebel", und
    // er nennt ausdruecklich den BODENKAMPF — 60 Grad ist die Basis, nicht die Obergrenze.
    // Ohne diesen Test: die 90 Grad von frueher kaemen zurueck, sobald sich jemand „zu eng"
    // fuehlt. 90 Grad machen jeden Titanen klein und jeden Meter kurz; das ist genau die
    // Wahrnehmung, die dieses Spiel nicht haben darf. Der zweite assert haelt fest, dass das
    // Tempo-Sichtfeld (F-017) nach OBEN geht — ein kleinerer Wert waere eine Kamera, die bei
    // Hoechsttempo zoomt statt zu oeffnen, also das Gegenteil von Geschwindigkeitsgefuehl.
    let d = daten();
    let k = &d.spiel.kamera;
    let m = &d.massstab.kamera;
    assert!(
        (m.sicht_boden_min_grad..=m.sicht_boden_max_grad).contains(&k.sicht_grad),
        "kamera.sicht_grad = {} liegt nicht in {}..={} (massstab.ron, FOV Bodenkampf)",
        k.sicht_grad, m.sicht_boden_min_grad, m.sicht_boden_max_grad
    );
    assert!(
        k.sicht_tempo_grad >= k.sicht_grad,
        "sicht_tempo_grad = {} < sicht_grad = {} — F-017 oeffnet das Bild mit dem Tempo, \
         es zieht es nicht zu",
        k.sicht_tempo_grad, k.sicht_grad
    );
    assert!(
        k.sicht_tempo_grad < 180.0,
        "sicht_tempo_grad = {} — ab 180 Grad ist die Projektionsmatrix entartet",
        k.sicht_tempo_grad
    );
}

#[test]
fn t005_jede_titanart_traegt_genau_eine_groessenklasse() {
    // Faengt: eine Art, die auf eine Groessenklasse zeigt, die es in massstab.ron nicht gibt
    // (Tippfehler, geloeschte Klasse, erfundene Klasse) — F-064.
    // Ohne diesen Test: `titan_hoehe_m()` liefert `None`, und der erste Aufrufer, der
    // `unwrap_or(0.0)` schreibt, spawnt einen Titanen mit Hoehe null. Der steht dann im
    // Boden, sein Cortex sitzt bei 0 m, und man sucht den Fehler in der Kollision statt in
    // einem Buchstaben in einer Datei.
    let d = daten();
    for (name, art) in &d.titanen.arten {
        let hoehe = d.titan_hoehe_m(art).unwrap_or_else(|| {
            panic!(
                "{name}: groessenklasse = {:?} steht nicht in massstab.ron titan.klassen — \
                 bekannt sind {:?}",
                art.groessenklasse,
                d.massstab.titan.klassen.keys().collect::<Vec<_>>()
            )
        });
        // Und die Klasse muss wirklich eine Groesse sein: groesser als ein Mensch, kleiner
        // als der Ashwalker. Alles andere ist kein Titan, sondern ein Tippfehler mit Komma.
        assert!(
            hoehe > d.massstab.referenz.mensch_hoehe_m
                && hoehe <= d.massstab.titan.ashwalker_hoehe_m,
            "{name}: Klasse {:?} ist {hoehe} m — das ist kein Titan",
            art.groessenklasse
        );
    }
}

#[test]
fn t005_der_cortex_sitzt_bei_neunundachtzig_prozent_der_hoehe() {
    // Faengt: eine Cortexhoehe, die nicht mehr bei ~89 % sitzt (F-030). Der User gibt fuenf
    // Paare an, die alle 0,881–0,893 ergeben; „~89 %" ist seine Rundung, 88–90 % das Fenster.
    //
    // ⚠️ Dieser Test prueft seit 2026-08-09 ein **Verhaeltnis zwischen zwei Meterangaben des
    // Users** statt einer Zahl gegen ihre eigene Formel. Vorher war `cortex_m` aus
    // `hoehe_m * cortex_anteil` gerechnet — dann kann dieser assert nie rot werden, weil er
    // genau die Rechnung nachrechnet, die den Wert erzeugt hat. Ein Waechter ueber einer
    // Ableitung ist Dekoration. Jetzt stehen beide Zahlen in der RON, und der Anteil ist
    // das, was er beim User ist: die Regel, an der man merkt, dass eine der beiden driftet.
    //
    // Geprueft werden ALLE Klassen, nicht nur die belegten: `boss` (28 m) hat heute keinen
    // Vertreter in titan.ron, und genau deshalb wuerde ein Fehler dort niemandem auffallen.
    let d = daten();
    let m = &d.massstab.titan;
    for (name, k) in &m.klassen {
        let anteil = k.cortex_m / k.hoehe_m;
        assert!(
            (0.88..=0.90).contains(&anteil),
            "Klasse {name}: Cortex {} m von {} m = {anteil} — der User gibt ~89 % vor \
             (3,7/4,2 · 8,9/10 · 12,5/14 · 18,7/21 · 24,9/28)",
            k.cortex_m, k.hoehe_m
        );
        // Der Cortex sitzt am Koerper, nicht darueber: sonst zeigt der Marker in die Luft.
        assert!(
            k.cortex_m < k.hoehe_m,
            "Klasse {name}: Cortex bei {} m ueber Scheitel {} m", k.cortex_m, k.hoehe_m
        );
        // Und die Zahl muss eine ANGABE sein, keine Rechnung. Der User nennt seine
        // Cortexhoehen auf einen Dezimeter genau (3,7 · 8,9 · 12,5 · 18,7 · 24,9); was aus
        // `hoehe_m * 0,89` faellt, hat drei Nachkommastellen (4,2 × 0,89 = 3,738). Ohne
        // diesen assert waere der Rueckfall auf die Ableitung unsichtbar — 3,738 liegt
        // mitten im Fenster oben, und genau deshalb kann das Fenster ihn nicht fangen.
        let zehntel = (k.cortex_m * 10.0).round() / 10.0;
        assert!(
            (k.cortex_m - zehntel).abs() < 1e-4,
            "Klasse {name}: cortex_m = {} ist keine Angabe auf einen Dezimeter — sieht nach \
             `hoehe_m * cortex_anteil` aus. Die fuenf Cortexhoehen sind Meterangaben des \
             Users, keine Ableitung (docs/modelle.md)",
            k.cortex_m
        );
    }
    // Und der Anteil selbst muss die Mitte dieses Fensters sein — sonst prueft die Regel
    // etwas anderes als das, was sie behauptet.
    assert!(
        (m.cortex_anteil - 0.89).abs() < 0.005,
        "cortex_anteil = {} — der User schreibt „Cortex bei ~89 %\"", m.cortex_anteil
    );
    // Jede Art erreicht ihre Cortexhoehe ueber ihre Klasse, ohne Umweg ueber eine Rechnung.
    for (name, art) in &d.titanen.arten {
        let hoehe = d.titan_hoehe_m(art).expect("Groessenklasse");
        let cortex = d.titan_cortex_hoehe_m(art).expect("Cortexhoehe");
        assert!(cortex > 0.0 && cortex < hoehe, "{name}: Cortex {cortex} m / Hoehe {hoehe} m");
    }
}

#[test]
fn t005_der_cortex_passt_unter_den_kopf_des_titanen() {
    // Faengt: eine Trefferzone, die groesser ist als der ganze Kopf des Titanen.
    // Ohne diesen Test: genau das war der Zustand, und niemand konnte es sehen, weil die
    // Kopfregel des Users bis 2026-08-09 nirgends als Zahl stand. `scuttler` hatte
    // cortex_radius_m 0,40 — also 0,80 m Durchmesser — an einem 4,2-m-Koerper, dessen Kopf
    // nach der Regel 1/9..1/10 nur 0,42..0,47 m misst. Der Cortex war fast doppelt so gross
    // wie der Kopf; `weaver` (0,90 m) noch mehr. Geometrisch unmoeglich, und im Bild traegt
    // der kleine Titan eine Zielscheibe statt eines Halses.
    //
    // Der Test ist absichtlich nur eine OBERGRENZE. Ob der Radius mit der Koerpergroesse
    // mitwachsen soll, ist eine offene Frage an den User (docs/FRAGEN.md Q-019) — dass er
    // den Kopf nicht ueberragen darf, ist keine.
    let d = daten();
    for (name, art) in &d.titanen.arten {
        let hoehe = d.titan_hoehe_m(art).expect("Groessenklasse");
        let kopf = d.titan_kopf_hoehe_max_m(art).expect("Kopfhoehe");
        let durchmesser = 2.0 * art.cortex_radius_m;
        assert!(
            durchmesser <= kopf,
            "{name}: cortex_radius_m = {} ⇒ {durchmesser} m Durchmesser an einem {hoehe}-m-\
             Koerper, dessen Kopf hoechstens {kopf} m hoch ist (massstab.ron \
             titan.kopf_anteil_max). Eine Trefferzone, die groesser ist als der Kopf, kann \
             kein Halsansatz sein",
            art.cortex_radius_m
        );
    }
}

#[test]
fn t005_die_gassen_bleiben_so_eng_wie_der_user_sie_will() {
    // Faengt: eine Gassenbreite ausserhalb 6–8 m. Der User schreibt „eng halten".
    // Ohne diesen Test: die 9 m von frueher kaemen zurueck, sobald jemand am Raster dreht,
    // und breiter faellt nie auf — es sieht ja aufgeraeumter aus. Enge Gassen sind aber der
    // Grund, warum sich Tempo nach Tempo anfuehlt: die Wand, die vorbeifliegt, muss nah sein.
    let d = daten();
    let r = &d.massstab.referenz;
    for (id, karte) in &d.karten.karten {
        assert!(
            (r.strasse_breite_min_m..=r.strasse_breite_max_m).contains(&karte.raster.gasse_m),
            "{id}: raster.gasse_m = {} liegt nicht in {}..={} (massstab.ron)",
            karte.raster.gasse_m, r.strasse_breite_min_m, r.strasse_breite_max_m
        );
    }
}

#[test]
fn t005_die_rasterstadt_bleibt_wohnbebauung() {
    // Faengt: erzeugte Haeuser ausserhalb der Wohnbebauung (4,5 m kleines Haus bis 11,5 m
    // grosses Haus). Frueher stand hier 8–34 m; 34 m ist die Groesse einer Kirche.
    // Ohne diesen Test: die Stadt waechst schleichend wieder hoch, weil „mehr Ankerpunkte in
    // der Hoehe" sich in jedem Einzelfall vernuenftig anhoert. **Die Stadt SOLL flach sein.**
    // Die Vertikale kommt aus Mauer (120 m), Kirche (35 m), Wachturm und Baeumen — und die
    // wirken nur, solange die Wohnbebauung sie nicht einholt. Eine Skyline waere kein
    // Balancing-Fehler, sondern ein anderes Spiel.
    let d = daten();
    let h = &d.massstab.architektur.hoehen_m;
    let unten = h["haus_klein"];
    let oben = h["haus_gross"];
    for (id, karte) in &d.karten.karten {
        let r = &karte.raster;
        assert!(
            r.hoehe_min_m >= unten,
            "{id}: hoehe_min_m = {} liegt unter dem kleinen Haus ({unten} m)", r.hoehe_min_m
        );
        assert!(
            r.hoehe_max_m <= oben,
            "{id}: hoehe_max_m = {} liegt ueber dem grossen Haus ({oben} m) — das ist \
             Sonderbau-Hoehe (Kirche {} m) und gehoert als `kloetze`-Eintrag gesetzt, \
             nicht gewuerfelt",
            r.hoehe_max_m, h["kirche"]
        );
    }
}

#[test]
fn t005_auch_die_gesetzten_kloetze_bleiben_wohnbebauung() {
    // Faengt: einen ausdruecklich gesetzten Quader ueber 11,5 m, der sich nicht als
    // Sonderbau ausweist.
    // Ohne diesen Test: die Haelfte der Stadt steht ausserhalb der Regel, die sie flach
    // halten soll. Genau das war der Zustand — der Waechter darueber las nur `raster`,
    // waehrend in `kloetze` Firsthoehen von 12,0 / 14,0 / 18,0 m standen. 18 m ist genau die
    // Hoehe, die derselbe Test dem Raster als „Sonderbau-Hoehe" verbietet, und diese drei
    // waren keine Sonderbauten, sondern graue Wuerfel.
    //
    // Nur eine OBERGRENZE, keine Untergrenze: eine Bruestung, eine Mauerscheibe und die
    // Bodenplatte duerfen flacher sein als ein kleines Haus. Nach oben entscheidet allein
    // `sonderbau`, und das ist der Punkt — die Ausnahme muss sich benennen.
    let d = daten();
    let oben = d.massstab.architektur.hoehen_m["haus_gross"];
    for (id, karte) in &d.karten.karten {
        for (i, k) in karte.kloetze.iter().enumerate() {
            if k.sonderbau {
                continue;
            }
            let first = k.mitte_m.1 + k.groesse_m.1 * 0.5;
            assert!(
                first <= oben,
                "{id}: Klotz {i} hat Firsthoehe {first} m und steht damit ueber der \
                 Wohnbebauung ({oben} m). Entweder ist er ein Bauwerk aus \
                 massstab.ron:architektur.hoehen_m — dann `sonderbau: true` — oder er wird \
                 gekuerzt. Ein grauer Wuerfel ist kein Grund, die Stadt hochzuziehen"
            );
        }
    }
}

#[test]
fn t005_fuer_jede_groessenklasse_gibt_es_ein_bauwerk_ueber_ihrem_cortex() {
    // Faengt: eine Groessenklasse, deren Cortex ueber jedem Bauwerk liegt, das der Massstab
    // kennt — also einen Titanen, den man in dieser Welt nicht von oben angehen kann.
    // Ohne diesen Test: die Groessentabelle hat drei Klassen geschaffen (14 / 21 / 28 m),
    // deren Cortex auf 12,5 / 18,7 / 24,9 m sitzt, waehrend das hoechste Wohnhaus 11,5 m
    // misst. Aus Dachhoehe + seil_min_m folgt eine **Ankerdecke**; darueber haelt kein Seil.
    // Jeder Anflug waere ballistisch — loslassen, fliegen, ein Vorbeiflug ohne Korrektur —
    // und beschuldigt wuerden der Seilloeser, der Boost und die Kamera, weil die Ursache
    // zwei Zahlen in zwei anderen Dateien sind, die niemand gegeneinander haelt.
    // Die Zahlen des Users bleiben; was sich aendert, ist die Zusammensetzung der Stadt
    // (Kirche 35 m, Wachturm 12 m, Baum 12 m). docs/FRAGEN.md Q-022.
    let d = daten();
    let m = &d.massstab;
    let (hoechstes, hoehe) = m
        .architektur
        .hoehen_m
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .expect("massstab.ron ohne ein einziges Bauwerk");
    for (name, k) in &m.titan.klassen {
        assert!(
            *hoehe >= k.cortex_m,
            "Klasse {name}: Cortex auf {} m, aber das hoechste Bauwerk ist {hoechstes} mit \
             {hoehe} m — ueber diesem Cortex gibt es keinen Anker",
            k.cortex_m
        );
    }
}

#[test]
fn t005_die_startkarte_traegt_die_vertikale_wirklich() {
    // Faengt: eine Karte, die nur Wohnbebauung setzt, waehrend die Doku behauptet, die
    // Vertikale komme aus Sonderbauten.
    // Ohne diesen Test: genau diese Luecke. `maps.ron` versprach sich selbst „Kirche,
    // Wachturm und Mauer werden als `kloetze` gesetzt" — und in keiner Karte stand eines
    // davon. Die Aussage war eine Absicht, kein Zustand, und die Rechnung dahinter
    // (hoechster Anker 11,5 m + seil_min_m 3,0 m = 14,5 m Ankerdecke) traf genau die drei
    // Groessenklassen, die die Groessentabelle gerade erst geschaffen hatte.
    //
    // Geprueft wird die Karte, die wirklich gebaut wird — eine Zahl in `massstab.ron` ist
    // kein Anker, ein Klotz in der Karte ist einer.
    let d = daten();
    let karte = d.aktuelle_karte().expect("aktuelle Karte");
    let oben = d.massstab.architektur.hoehen_m["haus_gross"];
    let hoechster_anker = karte
        .kloetze
        .iter()
        .filter(|k| k.hakbar)
        .map(|k| k.mitte_m.1 + k.groesse_m.1 * 0.5)
        .fold(0.0f32, f32::max);
    let decke = hoechster_anker + d.spiel.vector.seil_min_m;
    assert!(
        hoechster_anker > oben,
        "der hoechste HAKBARE Punkt der Startkarte liegt bei {hoechster_anker} m — nicht \
         ueber der Wohnbebauung ({oben} m). Ohne einen hakbaren Sonderbau ist die Vertikale \
         eine Behauptung"
    );
    // Und sie muss die groesste Klasse tragen, die titan.ron wirklich benutzt.
    let hoechster_cortex = d
        .titanen
        .arten
        .values()
        .filter_map(|a| d.titan_cortex_hoehe_m(a))
        .fold(0.0f32, f32::max);
    assert!(
        decke >= hoechster_cortex,
        "Ankerdecke {decke} m, hoechster Cortex einer benutzten Titanart {hoechster_cortex} m \
         — jeder Anflug auf dieses Ziel waere ballistisch (docs/FRAGEN.md Q-022)"
    );
}

#[test]
fn t005_jede_karte_traegt_ihr_eigenes_raster() {
    // Faengt: eine Karte, die kleiner ist als das Raster, das auf ihr stehen soll — und eine
    // Freiflaeche um den Ursprung, die die halbe Karte auffrisst.
    // Ohne diesen Test: die Stadt haette ein oder null vollstaendige Bloecke, das Raster
    // erzeugte fast nichts, und man suchte den Fehler im Generator statt in zwei Zahlen.
    let d = daten();
    for (id, karte) in &d.karten.karten {
        let r = &karte.raster;
        let periode = r.block_m + r.gasse_m;
        let kleinste_kante = karte.groesse_m.0.min(karte.groesse_m.1);
        assert!(
            kleinste_kante >= 4.0 * periode,
            "{id}: Karte {kleinste_kante} m traegt keine vier Rasterperioden zu {periode} m",
        );
        assert!(
            2.0 * r.frei_radius_m < kleinste_kante,
            "{id}: frei_radius_m = {} frisst die Karte ({kleinste_kante} m) auf",
            r.frei_radius_m
        );
    }
}

#[test]
fn t005_die_massstabsfaktoren_bleiben_ungleich() {
    // Faengt: den „Aufraeumer", der die drei Faktoren auf 1,0 angleicht, weil ungleiche
    // Massstaebe nach einem Fehler aussehen.
    // Ohne diesen Test: genau das passiert, und zwar mit gutem Gewissen und in einem
    // Einzeiler. Titanen um 1,4 ueberzeichnet und Mauern um 2,4 sind die BILDSPRACHE des
    // Referenzwerks: der Mensch klein, die Bedrohung unverhaeltnismaessig, die Mauer ein
    // Horizont. Ein einheitlicher Massstab waere technisch sauber und kuenstlerisch tot.
    let m = &daten().massstab;
    for (name, f) in [
        ("architektur_faktor", m.architektur_faktor),
        ("titan_faktor", m.titan_faktor),
        ("mauer_faktor", m.mauer_faktor),
    ] {
        assert!(f.is_finite() && f > 0.0, "{name} = {f} — muss endlich und > 0 sein");
    }
    assert!(
        m.titan_faktor > m.architektur_faktor,
        "titan_faktor = {} ist nicht groesser als architektur_faktor = {} — dann sind Titanen \
         nicht mehr ueberzeichnet",
        m.titan_faktor, m.architektur_faktor
    );
    assert!(
        m.mauer_faktor > m.titan_faktor,
        "mauer_faktor = {} ist nicht groesser als titan_faktor = {} — dann ist die Mauer \
         kein Horizont mehr, sondern eine grosse Wand",
        m.mauer_faktor, m.titan_faktor
    );
}

#[test]
fn t005_der_ashwalker_ragt_dreissig_meter_ueber_die_mauer() {
    // Faengt: jede Aenderung an Mauerhoehe oder Ashwalker-Hoehe, die die Probe des Users
    // bricht — „150 m, 30 m ueber der Mauer".
    // Ohne diesen Test: jemand senkt die Mauer auf 100 m, weil sie „unerreichbar" wirkt, und
    // der Auftritt des Bosses verliert genau das Bild, wegen dem er 150 m hoch ist. Die
    // Beziehung der beiden Zahlen ist die Aussage, nicht ihr Betrag.
    let m = &daten().massstab;
    let ueberstand = m.titan.ashwalker_hoehe_m - m.mauer.hoehe_m;
    assert!(
        (ueberstand - 30.0).abs() < 0.01,
        "Ashwalker {} m − Mauer {} m = {ueberstand} m, der User gibt 30 m vor",
        m.titan.ashwalker_hoehe_m, m.mauer.hoehe_m
    );
}

#[test]
fn t005_die_mauer_ist_in_zwei_zuegen_erreichbar() {
    // Faengt: eine Mauer, deren Krone (120 m) oder deren Zwischenplattform (60 m) mit der
    // Ankerreichweite (90 m) nicht mehr erreichbar ist.
    // Ohne diesen Test: die drei Zahlen wandern unabhaengig voneinander — Reichweite runter
    // beim Balancing, Mauer hoch fuer den Effekt — und irgendwann ist die Mauer von unten
    // nicht mehr besteigbar. Das faellt erst auf, wenn jemand es im Spiel versucht, und dann
    // ist die Vermutung „die Steuerung ist kaputt", nicht „drei RON-Zahlen passen nicht".
    let d = daten();
    let m = &d.massstab.mauer;
    let reichweite = d.massstab.vector.ankerreichweite_m;
    assert!(
        m.plattform_hoehe_m > 0.0 && m.plattform_hoehe_m < m.hoehe_m,
        "plattform_hoehe_m = {} liegt nicht zwischen Boden und Krone ({} m)",
        m.plattform_hoehe_m, m.hoehe_m
    );
    assert!(
        m.plattform_hoehe_m <= reichweite,
        "Boden -> Plattform sind {} m, die Ankerreichweite ist {reichweite} m",
        m.plattform_hoehe_m
    );
    assert!(
        m.hoehe_m - m.plattform_hoehe_m <= reichweite,
        "Plattform -> Krone sind {} m, die Ankerreichweite ist {reichweite} m",
        m.hoehe_m - m.plattform_hoehe_m
    );
    // Angeschraegt heisst: unten dicker als oben. Andersherum waere ein Ueberhang, an dem
    // kein Haken haelt und unter dem jeder Titan Deckung faende.
    assert!(
        m.dicke_basis_m > m.dicke_oben_m,
        "dicke_basis_m = {} <= dicke_oben_m = {} — die Mauer haengt ueber",
        m.dicke_basis_m, m.dicke_oben_m
    );
}

#[test]
fn t005_die_skalenleiter_der_mauer_bleibt_lesbar() {
    // Faengt: eine Steinreihe oder eine Baenderung, die zu grob wird, um Groesse abzulesen.
    // Ohne diesen Test: beide Zahlen sehen wie Deko aus und werden beim ersten
    // Performance-Gespraech gestrichen („weniger Geometrie an der Mauer"). Dann ist die
    // 120-m-Wand eine graue Flaeche, das Auge hat keine Leiter, und die Mauer wirkt aus der
    // Naehe wie eine 12-m-Wand. Die Steinreihe muss deutlich kleiner sein als ein Mensch —
    // sonst ist sie kein Massstab, sondern nur ein Muster.
    let m = &daten().massstab;
    assert!(
        m.mauer.steinreihe_m > 0.0 && m.mauer.steinreihe_m < m.referenz.mensch_hoehe_m * 0.5,
        "steinreihe_m = {} — als Skalenleiter muss eine Reihe deutlich unter halber \
         Menschenhoehe liegen ({} m)",
        m.mauer.steinreihe_m, m.referenz.mensch_hoehe_m * 0.5
    );
    assert!(
        m.mauer.baenderung_m > m.mauer.steinreihe_m && m.mauer.baenderung_m < m.mauer.hoehe_m,
        "baenderung_m = {} muss groeber als eine Steinreihe und feiner als die Mauer sein",
        m.mauer.baenderung_m
    );
    // Genug Baender, damit die Leiter ueberhaupt eine Leiter ist.
    let baender = (m.mauer.hoehe_m / m.mauer.baenderung_m).floor();
    assert!(baender >= 4.0, "nur {baender} Baender auf {} m Mauer", m.mauer.hoehe_m);
}

#[test]
fn t005_jede_traufe_gehoert_zu_einem_bauwerk_das_es_gibt() {
    // Faengt: eine Traufhoehe ohne Gesamthoehe (Tippfehler im Schluessel) und eine Traufe,
    // die ueber dem First liegt.
    // Ohne diesen Test: der Modellierer baut ein Haus mit 6 m Traufe und 4,5 m Gesamthoehe —
    // ein Dach, das nach unten zeigt. Ein Verweis ins Leere ist derselbe Bug wie ein toter
    // Link in der Doku, nur dass ihn niemand anklickt.
    let a = &daten().massstab.architektur;
    for (name, traufe) in &a.traufen_m {
        let hoehe = a.hoehen_m.get(name).unwrap_or_else(|| {
            panic!("traufen_m nennt {name:?}, das in hoehen_m nicht steht")
        });
        assert!(
            *traufe > 0.0 && traufe < hoehe,
            "{name}: Traufe {traufe} m passt nicht unter die Gesamthoehe {hoehe} m"
        );
    }
    for (name, hoehe) in &a.hoehen_m {
        assert!(*hoehe > 0.0, "architektur.hoehen_m[{name}] = {hoehe}");
    }
    assert!(!a.hoehen_m.is_empty(), "massstab.ron ohne ein einziges Bauwerk");
}

#[test]
fn t005_der_titankopf_bleibt_kleiner_als_der_menschenkopf() {
    // Faengt: eine Kopfgroessenregel, die den Titankopf relativ so gross macht wie den
    // Menschenkopf (1/7,5).
    // Ohne diesen Test: der erste, der ein Titanmodell „proportionaler" macht, nimmt genau
    // diese Zahl hoch — und das Modell sieht ab da wie ein zu nah stehender Mensch aus statt
    // wie ein 21 m hoher Koerper. Der relativ kleine Kopf IST der Groesseneindruck; er
    // entscheidet zusammen mit cortex_anteil, ob der Cortex auf 100 m lesbar ist (F-030).
    let m = &daten().massstab;
    let t = &m.titan;
    assert!(
        t.kopf_anteil_min > 0.0 && t.kopf_anteil_min < t.kopf_anteil_max,
        "kopf_anteil_min = {} / kopf_anteil_max = {} — min muss echt kleiner sein",
        t.kopf_anteil_min, t.kopf_anteil_max
    );
    assert!(
        t.kopf_anteil_max < m.referenz.kopf_anteil_mensch,
        "kopf_anteil_max = {} ist nicht kleiner als der Menschenanteil {} (1/7,5) — dann \
         wirkt der Titan wie ein naher Mensch, nicht wie ein grosser Koerper",
        t.kopf_anteil_max, m.referenz.kopf_anteil_mensch
    );
    // Der User schreibt „1/9 - 1/10". Das Fenster muss beide Bruchzahlen wirklich
    // ENTHALTEN: mit 0,1111 faellt ein Modell, das exakt auf 1/9 = 0,111111… gebaut ist,
    // um ein Zehntausendstel aus der eigenen Vorgabe. Das ist kein Rundungsdetail, es ist
    // eine Obergrenze, gegen die spaeter geprueft wird.
    assert!(
        t.kopf_anteil_min <= 1.0 / 10.0 && t.kopf_anteil_max >= 1.0 / 9.0,
        "kopf_anteil {}..{} schliesst 1/10 = {} und 1/9 = {} nicht ein",
        t.kopf_anteil_min, t.kopf_anteil_max, 1.0 / 10.0, 1.0 / 9.0_f32
    );
}

#[test]
fn t005_das_gitter_traegt_auch_die_hoehe_der_welt() {
    // Faengt: eine Welt, die hoeher wird als das Gitter, das sie indiziert.
    // Ohne diesen Test: `halbe_ausdehnung_m` wurde gegen die Karte in der EBENE gerechnet
    // (400/2 + 90 = 290) und nie gegen die Hoehe. In Y stehen Mauer (120 m) und Ashwalker
    // (150 m) uebereinander — 270 m, also 30 m Rand. Ob das Gitter ueberhaupt
    // dreidimensional ist, ist offen (docs/FRAGEN.md Q-014); dass ein Gitter nicht kleiner
    // sein darf als die Welt, die es haelt, ist es nicht. Ein Koerper ausserhalb landet in
    // der Randzelle und liegt damit falsch — und das sieht aus wie ein Bug im Strahl.
    let d = daten();
    let m = &d.massstab;
    let hoechster_punkt = m.mauer.hoehe_m + m.titan.ashwalker_hoehe_m;
    assert!(
        d.spiel.welt.halbe_ausdehnung_m >= hoechster_punkt,
        "welt.halbe_ausdehnung_m = {} deckt Mauer ({} m) + Ashwalker ({} m) = \
         {hoechster_punkt} m nicht ab",
        d.spiel.welt.halbe_ausdehnung_m, m.mauer.hoehe_m, m.titan.ashwalker_hoehe_m
    );
}

/// Eine Zahl so, wie sie in `docs/modelle.md` steht: Dezimalkomma, keine haengende Null.
fn deutsch(wert: f32) -> String {
    format!("{wert}").replace('.', ",")
}

#[test]
fn t005_die_groessentabelle_in_der_doku_zeigt_dieselben_zahlen() {
    // Faengt: jede Zahl in `massstab.ron`, die ihre Zeile in `docs/modelle.md` nicht mehr
    // hat — und jedes neue Bauwerk, das in der Doku fehlt.
    // Ohne diesen Test: `docs/modelle.md` ist eine zweite, vollstaendige und voellig
    // unbewachte Kopie derselben ~30 Zahlen. Die Datei sichert das heute mit dem Satz
    // „Beide werden gemeinsam geaendert oder gar nicht" ab — das ist eine Bitte, kein
    // Waechter. Und sie ist die Fassung, die der **Modellierer** liest: eine Doku, die
    // still von den Daten abweicht, ist schlimmer als keine, weil nach ihr gebaut wird.
    //
    // Geprueft wird die Zellenform „| <zahl> m", nicht die freie Zahl — sonst faende
    // `8 m` sich in `18 m` wieder und der Test waere gruen, ohne etwas zu wissen.
    let d = daten();
    let m = &d.massstab;
    let pfad = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/modelle.md");
    let text = std::fs::read_to_string(&pfad)
        .unwrap_or_else(|e| panic!("{}: {e}", pfad.display()));

    let mut soll: Vec<(String, String)> = vec![
        ("referenz.mensch_hoehe_m".into(), format!("| {} m", deutsch(m.referenz.mensch_hoehe_m))),
        ("referenz.tuer_hoehe_m".into(), format!("| {} m", deutsch(m.referenz.tuer_hoehe_m))),
        ("mauer.hoehe_m".into(), format!("| {} m", deutsch(m.mauer.hoehe_m))),
        ("mauer.dicke_oben_m".into(), format!("| {} m", deutsch(m.mauer.dicke_oben_m))),
        ("mauer.dicke_basis_m".into(), format!("| {} m", deutsch(m.mauer.dicke_basis_m))),
        ("mauer.plattform_hoehe_m".into(), format!("| {} m", deutsch(m.mauer.plattform_hoehe_m))),
        ("mauer.steinreihe_m".into(), format!("| {} m", deutsch(m.mauer.steinreihe_m))),
        ("mauer.baenderung_m".into(), format!("| {} m", deutsch(m.mauer.baenderung_m))),
        ("titan.ashwalker_hoehe_m".into(), format!("| {} m", deutsch(m.titan.ashwalker_hoehe_m))),
        ("kamera.hoehe_m".into(), format!("| {} m", deutsch(m.kamera.hoehe_m))),
        ("vector.ankerreichweite_m".into(), format!("| {} m", deutsch(m.vector.ankerreichweite_m))),
    ];
    for (name, hoehe) in &m.architektur.hoehen_m {
        soll.push((format!("architektur.hoehen_m[{name}]"), format!("| {} m", deutsch(*hoehe))));
    }
    for (name, klasse) in &m.titan.klassen {
        soll.push((format!("titan.klassen[{name}].hoehe_m"),
                   format!("| {} m", deutsch(klasse.hoehe_m))));
        soll.push((format!("titan.klassen[{name}].cortex_m"),
                   format!("Cortex {} m", deutsch(klasse.cortex_m))));
    }

    for (herkunft, gesucht) in soll {
        assert!(
            text.contains(&gesucht),
            "docs/modelle.md enthaelt {gesucht:?} nicht — die Doku weicht bei {herkunft} von \
             assets/data/massstab.ron ab. Beide werden gemeinsam geaendert oder gar nicht"
        );
    }
}
