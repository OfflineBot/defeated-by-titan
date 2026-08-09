//! Der Waechter ueber die Stadt — `F-003`.
//!
//! Eine Stadt aus einer Datei hat vier Arten, falsch zu sein, und **keine davon sieht man im
//! Bild**:
//!
//! 1. Sie wird gar nicht gebaut (oder doppelt) — im Bild sieht beides nach „Haeuser" aus.
//! 2. Sie ist nicht deterministisch — faellt erst im Netz auf, also am teuersten Tag.
//! 3. Die Kollisionsform hat den Faktor 2 gegen die Renderform — der Haken faengt in der
//!    Luft, und das Bild zeigt trotzdem ein Haus.
//! 4. Alles ist hakbar — dann prueft „kein Haken auf ungetaggten Flaechen" (`F-003`) nichts.
//!
//! Deshalb misst dieser Test **gegen `assets/data/maps.ron`**, nicht gegen sich selbst.

use avian3d::prelude::Collider;
use bevy::prelude::*;
use defeated_by_titan::data::GameData;
use defeated_by_titan::shared::{Ankerflaeche, Bauklotz, Koerper, Maske, Start};
use defeated_by_titan::world::karte::{kloetze_planen, Rohbau};
use std::path::PathBuf;

/// Baut die **echte** App, headless, und laesst `Startup` einmal laufen.
///
/// Nicht eine zweite, aehnliche App: sonst beweist der Test nichts ueber das Spiel, das
/// gespielt wird (dasselbe Argument wie in `tests/mehrspieler.rs`).
fn gebaute_welt() -> App {
    let mut app = defeated_by_titan::app(Start { headless: true, ..default() });
    app.update();
    app
}

fn daten() -> GameData {
    GameData::laden(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data"))
}

/// Der Plan, den `karte_bauen` abarbeitet — dieselbe Funktion, aber ohne App.
fn plan() -> Vec<Rohbau> {
    let d = daten();
    let karte = d.aktuelle_karte().expect("maps.ron: aktuell muss es geben");
    kloetze_planen(&d, karte)
}

/// Alle gebauten Quader als `(Name, Mitte, volle Groesse, hakbar)`, nach Namen sortiert.
fn gebaute_kloetze(app: &mut App) -> Vec<(String, Vec3, Vec3, bool)> {
    let mut q = app
        .world_mut()
        .query::<(&Name, &Bauklotz, &Transform, Option<&Ankerflaeche>)>();
    let mut alle: Vec<(String, Vec3, Vec3, bool)> = q
        .iter(app.world())
        .map(|(n, k, t, a)| (n.to_string(), t.translation, k.groesse, a.is_some()))
        .collect();
    alle.sort_by(|a, b| a.0.cmp(&b.0));
    alle
}

#[test]
fn f003_die_stadt_entsteht_aus_der_datei_und_nicht_doppelt() {
    // K1. Rot, wenn `karte_bauen` ein leerer Rumpf ist (0 statt ~90), wenn es doppelt
    // spawnt (2x), oder wenn das Raster nichts erzeugt (dann waeren es genau die gesetzten
    // Kloetze aus der Datei).
    let d = daten();
    let karte = d.aktuelle_karte().expect("aktuelle Karte");
    let plan = plan();
    let gesetzt = karte.kloetze.len();
    let erzeugt = plan.len() - gesetzt;

    assert!(
        erzeugt > 40,
        "das Raster erzeugte {erzeugt} Haeuser — eine Stadt mit weniger ist ein Stub, \
         kein Stadtteil (maps.ron: raster)"
    );

    let mut app = gebaute_welt();
    let gebaut = gebaute_kloetze(&mut app);
    assert_eq!(
        gebaut.len(),
        plan.len(),
        "{} Entities mit Bauklotz, aber {} geplante Quader ({gesetzt} gesetzt + {erzeugt} \
         erzeugt). Gleich Null heisst: nichts gebaut. Doppelt heisst: zwei Schreiber",
        gebaut.len(),
        plan.len()
    );

    // Und weiterlaufen lassen aendert daran nichts: die Stadt gehoert in `Startup`, nicht
    // in `Update` — sonst waechst sie jeden Frame um eine Stadt.
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(gebaute_kloetze(&mut app).len(), plan.len(), "die Stadt waechst pro Frame");

    // Unabhaengig von der Planfunktion: jeder gesetzte Klotz der Datei steht genau einmal
    // in der Welt, mit genau seiner Mitte und genau seiner Groesse.
    for (i, k) in karte.kloetze.iter().enumerate() {
        let mitte = Vec3::new(k.mitte_m.0, k.mitte_m.1, k.mitte_m.2);
        let groesse = Vec3::new(k.groesse_m.0, k.groesse_m.1, k.groesse_m.2);
        let treffer: Vec<_> = gebaut
            .iter()
            .filter(|(_, m, g, _)| *m == mitte && *g == groesse)
            .collect();
        assert_eq!(
            treffer.len(),
            1,
            "maps.ron: kloetze[{i}] (Mitte {mitte:?}, Groesse {groesse:?}) steht {}x in der \
             Welt statt genau einmal",
            treffer.len()
        );
        assert_eq!(
            treffer[0].3, k.hakbar,
            "maps.ron: kloetze[{i}] hakbar = {} in der Datei, {} in der Welt",
            k.hakbar, treffer[0].3
        );
    }
}

#[test]
fn f003_derselbe_seed_ergibt_exakt_dieselbe_stadt() {
    // K2. Rot in der Sekunde, in der jemand `rand::random()`, eine `HashMap`-Iteration oder
    // eine Uhrzeit einbaut. Verglichen wird jeder Wert, nicht die Anzahl: eine Stadt mit
    // gleich vielen, aber anders stehenden Haeusern ist im Netz derselbe Fehler.
    let erste = plan();
    let zweite = plan();
    assert_eq!(erste.len(), zweite.len(), "zwei Laeufe, zwei Stadtgroessen");
    for (a, b) in erste.iter().zip(zweite.iter()) {
        assert_eq!(a, b, "derselbe Seed, zwei verschiedene Quader:\n  {a:?}\n  {b:?}");
    }

    // Und dieselbe Stadt kommt auch aus der echten App heraus — nicht nur aus der
    // Planfunktion. Zwei getrennte Apps, Wert fuer Wert.
    let mut app_a = gebaute_welt();
    let mut app_b = gebaute_welt();
    assert_eq!(gebaute_kloetze(&mut app_a), gebaute_kloetze(&mut app_b));

    // Ein anderer Seed ergibt eine andere Stadt — sonst prueft der Vergleich oben nur, dass
    // die Funktion ueberhaupt zweimal dasselbe tut, und das taete sie auch ohne Wuerfel.
    let d = daten();
    let mut karte = d.aktuelle_karte().expect("aktuelle Karte").clone();
    karte.seed = karte.seed.wrapping_add(1);
    let andere = kloetze_planen(&d, &karte);
    assert_ne!(andere, erste, "ein anderer Seed ergab exakt dieselbe Stadt");
}

#[test]
fn f003_die_collider_tragen_die_halbe_kante_aus_der_datei() {
    // K3. `Collider::cuboid` nimmt die GANZE Kante und halbiert intern
    // (avian3d-0.7.0/src/collision/collider/parry/mod.rs:747-749), `Koerper::halb_m` und
    // parrys `Cuboid::half_extents` fuehren die HALBE. Ein Faktor 2 faellt im Bild nicht
    // auf — hier schon.
    //
    // Gemessen wird die FORMQUELLE (`collider.shape()`), nicht `ColliderAabb`: die Huelle
    // waechst gemessen um 0,01 m je Achse und bei Bewegung um den Sweep.
    let d = daten();
    let karte = d.aktuelle_karte().expect("aktuelle Karte");
    let mut app = gebaute_welt();

    let mut q = app.world_mut().query::<(&Transform, &Bauklotz, &Koerper, &Collider)>();
    let alle: Vec<(Vec3, Vec3, Vec3, Vec3)> = q
        .iter(app.world())
        .map(|(t, b, k, c)| {
            let form = c
                .shape()
                .as_cuboid()
                .expect("jeder Klotz ist ein Quader — nichts hier wird gedreht");
            let h = form.half_extents;
            (t.translation, b.groesse, k.halb_m, Vec3::new(h.x, h.y, h.z))
        })
        .collect();

    assert!(alle.len() > 40, "nur {} Kloetze mit Collider", alle.len());

    let mut gemessen = 0;
    for (i, k) in karte.kloetze.iter().enumerate() {
        let mitte = Vec3::new(k.mitte_m.0, k.mitte_m.1, k.mitte_m.2);
        let voll = Vec3::new(k.groesse_m.0, k.groesse_m.1, k.groesse_m.2);
        let (_, render, huelle, collider) = alle
            .iter()
            .find(|(m, _, _, _)| *m == mitte)
            .unwrap_or_else(|| panic!("maps.ron: kloetze[{i}] steht nicht bei {mitte:?}"));
        assert_eq!(*render, voll, "kloetze[{i}]: Renderform weicht von der Datei ab");
        assert_eq!(
            *collider,
            voll * 0.5,
            "kloetze[{i}]: Collider-Halbgroesse {collider:?}, erwartet {:?} — \
             Faktor {:.2} gegen die Datei",
            voll * 0.5,
            collider.x / (voll.x * 0.5)
        );
        assert_eq!(*huelle, voll * 0.5, "kloetze[{i}]: Koerper::halb_m weicht ab");
        gemessen += 1;
    }
    assert!(gemessen >= 3, "nur {gemessen} Kloetze gegen die Datei geprueft, mindestens 3");

    // Und fuer JEDEN Klotz, auch die erzeugten: Renderform und Kollisionsform sind dieselbe
    // Form. Genau das ist der Grund, warum ein Schreiber beide setzt.
    for (mitte, render, huelle, collider) in &alle {
        assert_eq!(*collider, *render * 0.5, "Klotz bei {mitte:?}: Render {render:?}");
        assert_eq!(*huelle, *render * 0.5, "Klotz bei {mitte:?}: Huelle {huelle:?}");
    }
}

#[test]
fn f003_nicht_jede_flaeche_ist_hakbar() {
    // K4. Rot, sobald jemand pauschal alles taggt — dann kann „Kein Haken auf ungetaggten
    // Flaechen" (F-003) nicht mehr widerlegt werden, und das Kriterium prueft nichts.
    let mut app = gebaute_welt();
    let gebaut = gebaute_kloetze(&mut app);
    let hakbar = gebaut.iter().filter(|(_, _, _, a)| *a).count();
    let ungetaggt = gebaut.len() - hakbar;
    assert!(hakbar > 0, "keine einzige Ankerflaeche in der gebauten Stadt");
    assert!(
        ungetaggt > 0,
        "alle {} Kloetze tragen Ankerflaeche — F-003 prueft dann nichts",
        gebaut.len()
    );

    // Der Marker und die Maske sind derselbe Zustand, an zwei Stellen geschrieben von
    // einem Schreiber. Laufen sie auseinander, haekt der Haken woanders als das Gizmo.
    let mut q = app.world_mut().query::<(&Koerper, Option<&Ankerflaeche>)>();
    for (koerper, anker) in q.iter(app.world()) {
        assert_eq!(
            koerper.maske.hat(Maske::HAKBAR),
            anker.is_some(),
            "Maske {:?} und Ankerflaeche {:?} widersprechen sich",
            koerper.maske,
            anker.is_some()
        );
    }
}

#[test]
fn f003_kein_rasterhaus_steht_in_einem_gesetzten_klotz() {
    // `maps.ron`: „Ausdruecklich Gesetztes gewinnt: das Erzeugte laesst um jeden gesetzten
    // Klotz Platz." Rot, wenn die Ueberlappungspruefung fehlt — dann waechst ein 28-m-Block
    // durch den Wachturm.
    let d = daten();
    let karte = d.aktuelle_karte().expect("aktuelle Karte");
    let plan = plan();
    let (gesetzt, erzeugt) = plan.split_at(karte.kloetze.len());

    for haus in erzeugt {
        for klotz in gesetzt {
            let abstand = (haus.mitte_m - klotz.mitte_m).abs();
            let summe = haus.groesse_m * 0.5 + klotz.groesse_m * 0.5;
            assert!(
                !(abstand.x < summe.x && abstand.y < summe.y && abstand.z < summe.z),
                "{} steckt in {}: Abstand {abstand:?}, Summe {summe:?}",
                haus.name,
                klotz.name
            );
        }
    }

    // Der Umkehrschluss, der das Ganze erst zu einer Aussage macht: die Bodenplatte deckt
    // die ganze Karte, und trotzdem steht die Stadt. Waere die Pruefung nicht strikt, waere
    // hier alles leer.
    assert!(!erzeugt.is_empty(), "die Bodenplatte hat die ganze Stadt verschluckt");
}

#[test]
fn f003_der_platz_um_den_ursprung_bleibt_frei() {
    // Dort startet der Spieler, und `scripts/t007-erste-fahrt.txt` laeuft 6 m nach -Z.
    // Rot, wenn `frei_radius_m` ignoriert wird — dann steht ein Haus auf dem Spieler.
    let d = daten();
    let karte = d.aktuelle_karte().expect("aktuelle Karte");
    let radius = karte.raster.frei_radius_m;
    let plan = plan();

    for haus in &plan[karte.kloetze.len()..] {
        let halb = haus.groesse_m * 0.5;
        let dx = (haus.mitte_m.x.abs() - halb.x).max(0.0);
        let dz = (haus.mitte_m.z.abs() - halb.z).max(0.0);
        let abstand = (dx * dx + dz * dz).sqrt();
        assert!(
            abstand >= radius,
            "{} kommt dem Ursprung auf {abstand:.2} m nahe, frei_radius_m = {radius}",
            haus.name
        );
    }
}

#[test]
fn f003_die_rasterhaeuser_bleiben_im_hoehenfenster_der_datei() {
    // `maps.ron` sagt: die Stadt ist flach, die Vertikale kommt aus den Sonderbauten.
    // Rot, wenn jemand die Wohnbebauung wieder hochzieht — im Bild sieht das nach Skyline
    // aus und nicht nach Fehler.
    let d = daten();
    let karte = d.aktuelle_karte().expect("aktuelle Karte");
    let r = &karte.raster;
    let plan = plan();
    let haeuser = &plan[karte.kloetze.len()..];

    let mut hoch = 0;
    for haus in haeuser {
        let h = haus.groesse_m.y;
        assert!(
            (r.hoehe_min_m..=r.hoehe_max_m).contains(&h),
            "{}: {h} m liegt nicht in {}..={} (maps.ron: raster)",
            haus.name,
            r.hoehe_min_m,
            r.hoehe_max_m
        );
        assert!(haus.mitte_m.y > 0.0, "{}: Mitte unter dem Boden", haus.name);
        assert!(
            (haus.mitte_m.y - h * 0.5).abs() < 1e-4,
            "{}: steht nicht auf y = 0, sondern bei {}",
            haus.name,
            haus.mitte_m.y - h * 0.5
        );
        if h > (r.hoehe_min_m + r.hoehe_max_m) * 0.5 {
            hoch += 1;
        }
    }
    // Nicht alle gleich hoch: eine Stadt aus lauter 8-m-Kloetzen waere ein Rechenfehler,
    // den kein Hoehenfenster faengt.
    assert!(
        hoch > 0 && hoch < haeuser.len(),
        "{hoch} von {} Haeusern in der oberen Haelfte des Fensters — die Hoehen streuen nicht",
        haeuser.len()
    );
}
