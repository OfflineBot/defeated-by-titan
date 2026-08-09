//! Der Waechter ueber die RON-Dateien.
//!
//! **Zahlen gehoeren in RON, nicht in Rust** (`prompts/init.md` §4) — und genau deshalb
//! braucht die RON einen Test. Eine Zahl im Code faengt der Compiler ab; eine Zahl in einer
//! Datei faengt niemand ab, ausser hier.
//!
//! Geprueft wird nicht „ist es huebsch", sondern **was die Bibel als bindend festlegt** und
//! **was in sich stimmen muss** (jeder Verweis zeigt auf etwas, das es gibt).

use defeated_by_titan::data::GameData;
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
    // 400 studs × 0,28 m = 112 m (docs/FRAGEN.md Q-002), und init.md §1 nennt 60–120 m.
    // Faellt dieser Test, ist entweder die Umrechnung falsch oder jemand hat die Reichweite
    // getunt, ohne die Herkunft nachzuziehen.
    let r = daten().spiel.vector.hakenreichweite_m;
    assert!(
        (60.0..=120.0).contains(&r),
        "hakenreichweite_m = {r} — init.md §1 nennt 60–120 m (400 studs × 0,28 = 112)"
    );
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
        ("seilzug_m_s", v.seilzug_m_s),
        ("seil_min_m", v.seil_min_m),
        ("boost_m_s2", v.boost_m_s2),
        ("tempo_max_m_s", v.tempo_max_m_s),
        ("spieler.hoehe_m", d.spiel.spieler.hoehe_m),
        ("spieler.laufen_m_s", d.spiel.spieler.laufen_m_s),
        ("kamera.sicht_grad", d.spiel.kamera.sicht_grad),
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
        assert!(art.hoehe_m > 0.0, "{name}: hoehe_m = {}", art.hoehe_m);
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
