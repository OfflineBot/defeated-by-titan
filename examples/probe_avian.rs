//! **Die echte Probe.** Fuenf Fragen an avian3d 0.7.0, die ueber die Physik dieses Spiels
//! entscheiden. Jede Antwort ist eine Zahl, keine Behauptung.
//!
//! ```text
//! cargo run --release --example probe_avian
//! ```
//!
//! ## Warum das Beispiel nicht `DefaultPlugins` benutzt (ausser in F0)
//!
//! Die Messungen brauchen **exakte Tickzahlen**. `src/lib.rs:179` treibt die fensterlose
//! Fahrt mit `ScheduleRunnerPlugin`, und dann haengt die Zahl der `FixedUpdate`-Durchlaeufe
//! an der Wanduhr — 600 Ticks bei 60 Hz waeren 10 Sekunden Realzeit und die Tickzahl je
//! Messpunkt waere nicht reproduzierbar.
//!
//! `TimeUpdateStrategy::FixedTimesteps(1)` schiebt die Realzeit pro `App::update()` um
//! **genau einen** Zeitschritt vor (bevy_time-0.19.0/src/lib.rs:181-183), also laeuft je
//! `update()` genau ein `FixedUpdate` und damit genau ein Physikschritt. Das ist die
//! strengere Umgebung: kein Renderer, kein Fenster, keine Uhr.
//!
//! **F0 prueft dann zusaetzlich genau den Weg aus `src/lib.rs:146-190`** — `DefaultPlugins`
//! mit `backends: None`, ohne `WinitPlugin`, mit `ScheduleRunnerPlugin`. Wenn avian dort
//! nicht laeuft, sind die anderen vier Antworten wertlos.
//!
//! ## Die Zahlen kommen aus `assets/data/game.ron`
//!
//! Nicht aus dem Kopf. Sie stehen hier als Konstanten, weil ein Beispiel kein RON laedt;
//! wer `game.ron` aendert, muss diese Datei nachziehen — das steht auch im Kopf jeder
//! Konstanten.

use core::time::Duration;
use std::time::Instant;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::shared::SimulationSystems;

// ---------------------------------------------------------------------------------------
// Werte aus assets/data/game.ron. Alle Laengen in Metern, alle Zeiten in Sekunden.
// ---------------------------------------------------------------------------------------

/// `simulation_hz`
const HZ: f64 = 60.0;
/// `schwerkraft_m_s2`
const SCHWERKRAFT: f32 = -20.0;
/// `spieler.radius_m`
const SPIELER_RADIUS: f32 = 0.35;
/// `spieler.hoehe_m`
const SPIELER_HOEHE: f32 = 1.8;
/// Zylinderlaenge der Kapsel: `Collider::capsule(radius, length)` setzt die Endpunkte auf
/// `±length/2` (avian3d-0.7.0/src/collision/collider/parry/mod.rs:790-797), die Gesamthoehe
/// ist also `length + 2*radius`.
const SPIELER_KAPSEL_LAENGE: f32 = SPIELER_HOEHE - 2.0 * SPIELER_RADIUS;
/// `vector.tempo_max_m_s`
const TEMPO_MAX: f32 = 75.0;
/// `vector.hakenreichweite_m`
const REICHWEITE: f32 = 112.0;
/// `vector.seil_min_m`
const SEIL_MIN: f32 = 3.0;
/// `welt.wand_min_m` ist 0,5 m — die Probe faehrt bewusst gegen eine **duennere** Wand.
const WAND_DICKE: f32 = 0.3;

/// Bit 0: Hauswand, nicht hakbar. Bit 1: Dach, hakbar.
const SCHICHT_WAND: u32 = 1 << 0;
const SCHICHT_DACH: u32 = 1 << 1;

// ---------------------------------------------------------------------------------------
// Geruest
// ---------------------------------------------------------------------------------------

/// Eine Welt ohne Fenster, ohne Renderer, mit exakt einem Physikschritt je `update()`.
fn welt(schwerkraft_y: f32) -> App {
    welt_mit_substeps(schwerkraft_y, None)
}

/// Wie [`welt`], aber mit einstellbarer Zahl der Teilschritte. `None` = avians Vorgabe 6
/// (avian3d-0.7.0/src/dynamics/solver/schedule.rs:185-191).
fn welt_mit_substeps(schwerkraft_y: f32, substeps: Option<u32>) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // `PhysicsPlugins::default()` haengt den `PhysicsSchedule` an `FixedPostUpdate`
    // (avian3d-0.7.0/src/lib.rs:751-755).
    app.add_plugins(PhysicsPlugins::default());
    app.insert_resource(Gravity(Vec3::new(0.0, schwerkraft_y, 0.0)));
    app.insert_resource(Time::<Fixed>::from_hz(HZ));
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    if let Some(n) = substeps {
        app.insert_resource(SubstepCount(n));
    }
    app.finish();
    app.cleanup();
    app
}

/// Ein Spielerkoerper: Kapsel in Spielergroesse, Drehung gesperrt (er soll ein Punkt sein,
/// kein Kreisel), Schlafen verboten (ein schlafender Koerper faelscht jede Messung).
fn spieler(app: &mut App, ort: Vec3, tempo: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::capsule(SPIELER_RADIUS, SPIELER_KAPSEL_LAENGE),
            LockedAxes::ROTATION_LOCKED,
            SleepingDisabled,
            Transform::from_translation(ort),
            LinearVelocity(tempo),
        ))
        .id()
}

/// Ein Hakenanker: statischer Koerper ohne Collider.
fn anker(app: &mut App, ort: Vec3) -> Entity {
    app.world_mut()
        .spawn((RigidBody::Static, Transform::from_translation(ort)))
        .id()
}

/// Ein Seil: `DistanceJoint` mit `limits = [0, laenge]`. Untergrenze 0 heisst **es gibt
/// keine Untergrenze** — `DistanceLimit::compute_correction` korrigiert nur, wenn der
/// Abstand die Obergrenze ueberschreitet (avian3d-0.7.0/src/dynamics/joints/mod.rs:329-343).
/// Das ist die Definition von „zieht, drueckt nicht".
fn seil(app: &mut App, a: Entity, b: Entity, laenge: f32) -> Entity {
    app.world_mut()
        .spawn(
            DistanceJoint::new(a, b)
                .with_local_anchor1(Vec3::ZERO)
                .with_local_anchor2(Vec3::ZERO)
                .with_limits(0.0, laenge),
        )
        .id()
}

fn ort(app: &App, e: Entity) -> Vec3 {
    app.world().get::<Position>(e).expect("Position fehlt").0
}

fn tempo(app: &App, e: Entity) -> Vec3 {
    app.world()
        .get::<LinearVelocity>(e)
        .expect("LinearVelocity fehlt")
        .0
}

fn strich(titel: &str) {
    println!("\n{}", "=".repeat(88));
    println!("{titel}");
    println!("{}", "=".repeat(88));
}

// ---------------------------------------------------------------------------------------
// F0 — laeuft avian im Kopflos-Modus DIESES Projekts?
// ---------------------------------------------------------------------------------------

fn f0_kopflos() {
    strich("F0  KOPFLOS — avian unter dem Aufbau aus src/lib.rs:146-190 (DefaultPlugins)");

    let mut gruppe = DefaultPlugins.set(WindowPlugin {
        primary_window: None,
        exit_condition: bevy::window::ExitCondition::DontExit,
        ..default()
    });
    gruppe = gruppe.set(bevy::render::RenderPlugin {
        render_creation: bevy::render::settings::RenderCreation::Automatic(Box::new(
            bevy::render::settings::WgpuSettings {
                backends: None,
                ..default()
            },
        )),
        ..default()
    });
    gruppe = gruppe.add(bevy::app::ScheduleRunnerPlugin::run_loop(
        Duration::from_secs_f64(1.0 / 240.0),
    ));
    #[cfg(any(feature = "x11", feature = "wayland"))]
    {
        gruppe = gruppe.disable::<bevy::winit::WinitPlugin>();
    }

    let mut app = App::new();
    app.add_plugins(gruppe);
    app.add_plugins(PhysicsPlugins::default());
    app.insert_resource(Gravity(Vec3::new(0.0, SCHWERKRAFT, 0.0)));
    app.insert_resource(Time::<Fixed>::from_hz(HZ));
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.finish();
    app.cleanup();

    let e = spieler(&mut app, Vec3::new(0.0, 100.0, 0.0), Vec3::ZERO);
    for _ in 0..120 {
        app.update();
    }

    let t = 120.0 / HZ as f32;
    let erwartet_y = 100.0 + 0.5 * SCHWERKRAFT * t * t;
    let gemessen = ort(&app, e);
    let v = tempo(&app, e);
    println!("  120 Ticks freier Fall aus 100 m, g = {SCHWERKRAFT} m/s^2, t = {t:.4} s");
    println!("  y analytisch     : {erwartet_y:.4} m");
    println!("  y gemessen       : {:.4} m", gemessen.y);
    println!("  Abweichung       : {:.4} m", gemessen.y - erwartet_y);
    println!("  v_y gemessen     : {:.4} m/s  (analytisch {:.4})", v.y, SCHWERKRAFT * t);
    println!("  ERGEBNIS: avian laeuft unter DefaultPlugins ohne Fenster und ohne Adapter.");
}

// ---------------------------------------------------------------------------------------
// F1 — SEIL
// ---------------------------------------------------------------------------------------

struct Fahrt {
    tempo_start: f32,
    tempo_ende: f32,
    tempo_max: f32,
    abstand_min: f32,
    abstand_max: f32,
    energie_start: f32,
    energie_ende: f32,
    energie_min: f32,
    energie_max: f32,
    /// Tempo nach 1, 2, 5 und 10 Sekunden.
    marken: [f32; 4],
}

/// Laesst einen Koerper `ticks` Ticks am Seil haengen und misst jeden Tick.
fn pendel_fahren(schwerkraft_y: f32, laenge: f32, v0: f32, ticks: usize) -> Fahrt {
    pendel_fahren_mit(schwerkraft_y, laenge, v0, ticks, None)
}

fn pendel_fahren_mit(
    schwerkraft_y: f32,
    laenge: f32,
    v0: f32,
    ticks: usize,
    substeps: Option<u32>,
) -> Fahrt {
    let mut app = welt_mit_substeps(schwerkraft_y, substeps);
    let a = anker(&mut app, Vec3::new(0.0, laenge, 0.0));
    let s = spieler(&mut app, Vec3::ZERO, Vec3::new(v0, 0.0, 0.0));
    seil(&mut app, a, s, laenge);

    let anker_ort = Vec3::new(0.0, laenge, 0.0);
    // Spezifische Energie: 1/2 v^2 + g_betrag * h. Bei g = 0 ist das reine kinetische
    // Energie, dann misst `energie_*` genau den Tempoverlust.
    let energie = |p: Vec3, v: Vec3| 0.5 * v.length_squared() - schwerkraft_y * p.y;

    let mut f = Fahrt {
        tempo_start: v0,
        tempo_ende: v0,
        tempo_max: v0,
        abstand_min: f32::MAX,
        abstand_max: 0.0,
        energie_start: energie(Vec3::ZERO, Vec3::new(v0, 0.0, 0.0)),
        energie_ende: 0.0,
        energie_min: f32::MAX,
        energie_max: f32::MIN,
        marken: [0.0; 4],
    };

    for tick in 1..=ticks {
        app.update();
        let p = ort(&app, s);
        let v = tempo(&app, s);
        let d = (p - anker_ort).length();
        let e = energie(p, v);

        f.abstand_min = f.abstand_min.min(d);
        f.abstand_max = f.abstand_max.max(d);
        f.tempo_max = f.tempo_max.max(v.length());
        f.energie_min = f.energie_min.min(e);
        f.energie_max = f.energie_max.max(e);
        f.tempo_ende = v.length();
        f.energie_ende = e;

        for (i, sek) in [1usize, 2, 5, 10].iter().enumerate() {
            if tick == sek * HZ as usize {
                f.marken[i] = v.length();
            }
        }
    }
    f
}

fn f1_seil() {
    strich("F1  SEIL — zieht das Seil, ohne zu druecken? Und wie viel Tempo frisst es?");

    println!(
        "\n(a) OHNE Schwerkraft: reine Kreisbahn. Jeder Tempoverlust ist Loeserdaempfung.\n\
         \x20   Das ist der direkte Vergleich zur Eigenbau-Rechnung (99,2 %/s bei L=3, v=75)."
    );
    println!(
        "\n  {:>6} {:>7} | {:>9} {:>9} {:>9} {:>9} | {:>9} {:>9} | {:>10}",
        "L [m]", "v0", "v(1s)", "v(2s)", "v(5s)", "v(10s)", "d_min", "d_max", "Verlust/s"
    );
    println!("  {}", "-".repeat(86));

    let faelle = [(20.0f32, 30.0f32), (SEIL_MIN, TEMPO_MAX), (10.0, 50.0), (5.0, 60.0)];
    for (laenge, v0) in faelle {
        let f = pendel_fahren(0.0, laenge, v0, 600);
        let verlust_pro_s = if f.tempo_start > 0.0 {
            (1.0 - (f.tempo_ende / f.tempo_start).powf(1.0 / 10.0)) * 100.0
        } else {
            0.0
        };
        println!(
            "  {laenge:>6.1} {v0:>7.1} | {:>9.4} {:>9.4} {:>9.4} {:>9.4} | {:>9.5} {:>9.5} | {verlust_pro_s:>9.4} %",
            f.marken[0], f.marken[1], f.marken[2], f.marken[3], f.abstand_min, f.abstand_max
        );
    }

    println!(
        "\n(b) MIT Schwerkraft {SCHWERKRAFT} m/s^2: das echte Pendel, 600 Ticks = 10 s.\n\
         \x20   `d_min/d_max` sagt, wie weit das Seil dehnt und ob es drueckt (d_min < L\n\
         \x20   heisst: der Koerper darf naeher heran, das Seil drueckt NICHT)."
    );
    println!(
        "\n  {:>6} {:>7} | {:>9} {:>9} | {:>9} {:>9} | {:>11} {:>9}",
        "L [m]", "v0", "d_min", "d_max", "v_max", "v(10s)", "E_drift", "E_span"
    );
    println!("  {}", "-".repeat(86));

    for (laenge, v0) in faelle {
        let f = pendel_fahren(SCHWERKRAFT, laenge, v0, 600);
        let drift = (f.energie_ende - f.energie_start) / f.energie_start * 100.0;
        let span = (f.energie_max - f.energie_min) / f.energie_start * 100.0;
        println!(
            "  {laenge:>6.1} {v0:>7.1} | {:>9.5} {:>9.5} | {:>9.4} {:>9.4} | {drift:>10.4} % {span:>8.4} %",
            f.abstand_min, f.abstand_max, f.tempo_max, f.tempo_ende
        );
    }

    println!(
        "\n  Lesehilfe: `E_drift` ist der Fehler der spezifischen Energie nach 10 s in Prozent.\n\
         \x20 Ein Pendel, das einschlaeft, hat ein grosses negatives E_drift; eins, das\n\
         \x20 explodiert, ein grosses positives."
    );
}

// ---------------------------------------------------------------------------------------
// F2 — ZWEI SEILE
// ---------------------------------------------------------------------------------------

fn zwei_seile(name: &str, a1: Vec3, a2: Vec3, start: Vec3, l1: f32, l2: f32, ticks: usize) {
    let mut app = welt(SCHWERKRAFT);
    let e1 = anker(&mut app, a1);
    let e2 = anker(&mut app, a2);
    let s = spieler(&mut app, start, Vec3::ZERO);
    seil(&mut app, e1, s, l1);
    seil(&mut app, e2, s, l2);

    println!("\n  {name}");
    println!(
        "    Anker 1 {a1:?}  Anker 2 {a2:?}  Start {start:?}  L1 = {l1} m  L2 = {l2} m"
    );
    println!(
        "    {:>6} | {:>10} {:>10} | {:>10} {:>10} | {:>10}",
        "Tick", "d1 [m]", "d2 [m]", "y [m]", "Absacken", "|v| [m/s]"
    );
    println!("    {}", "-".repeat(74));

    let mut y_min = start.y;
    for tick in 1..=ticks {
        app.update();
        let p = ort(&app, s);
        let v = tempo(&app, s);
        y_min = y_min.min(p.y);
        if tick % 10 == 0 || tick == 1 {
            println!(
                "    {tick:>6} | {:>10.5} {:>10.5} | {:>10.5} {:>10.5} | {:>10.4}",
                (p - a1).length(),
                (p - a2).length(),
                p.y,
                start.y - p.y,
                v.length()
            );
        }
    }
    let p = ort(&app, s);
    let freier_fall = 0.5 * -SCHWERKRAFT * (ticks as f32 / HZ as f32).powi(2);
    println!(
        "    Absacken nach {ticks} Ticks: {:.5} m   (freier Fall waere {freier_fall:.4} m)",
        start.y - p.y
    );
    println!(
        "    Seildehnung: L1 {:+.6} m, L2 {:+.6} m",
        (p - a1).length() - l1,
        (p - a2).length() - l2
    );
}

fn f2_zwei_seile() {
    strich("F2  ZWEI SEILE — faellt der Spieler durch, wenn beide Seile straff ziehen?");

    println!(
        "\n  Der Eigenbauentwurf addierte beide Seilkraefte zu EINEM Vec3. Zwei Seile, die\n\
         \x20 in entgegengesetzte Richtungen ziehen, ergaben Summe = 0 und der Spieler fiel\n\
         \x20 mit vollem g. avian loest jeden Joint als eigenen Zwang."
    );

    // Normalfall: zwei Daecher, Anker 30 m auseinander, 20 m ueber dem Spieler.
    // Abstand = sqrt(15^2 + 20^2) = 25 m, beide Seile exakt straff.
    zwei_seile(
        "(a) NORMALFALL — zwei Daecher, Zug in entgegengesetzte Richtungen",
        Vec3::new(-15.0, 20.0, 0.0),
        Vec3::new(15.0, 20.0, 0.0),
        Vec3::ZERO,
        25.0,
        25.0,
        60,
    );

    // Entarteter Fall: Anker 48 m auseinander, beide Seile 25 m. Der Spieler auf der
    // Verbindungslinie darf nur auf einem Kreis mit Radius sqrt(25^2 - 24^2) = 7 m sitzen.
    // Start (0, 13, 0): Abstand zu (+-24, 20, 0) ist sqrt(24^2 + 7^2) = 25 m, also exakt
    // straff — und beide Zwangsnormalen zeigen fast entlang derselben Geraden.
    zwei_seile(
        "(b) ENTARTET — Anker 48 m auseinander, beide Seile 25 m, Spieler auf der Linie",
        Vec3::new(-24.0, 20.0, 0.0),
        Vec3::new(24.0, 20.0, 0.0),
        Vec3::new(0.0, 13.0, 0.0),
        25.0,
        25.0,
        60,
    );
}

// ---------------------------------------------------------------------------------------
// F3 — REEL-IN
// ---------------------------------------------------------------------------------------

fn f3_reel_in() {
    strich("F3  REEL-IN — steigt das Tempo, wenn das Seil kuerzer wird? (Drehimpuls)");

    println!(
        "\n  Seil 30 m -> 5 m in 60 Ticks (1,0 s, also 25 m/s; `vector.seilzug_m_s` ist 28).\n\
         \x20 Ohne Schwerkraft, damit nur der Einholeffekt gemessen wird.\n\
         \x20 Theorie (Drehimpulserhaltung, L*v = const): v_ende = v_start * 30/5 = 6 * v_start."
    );
    println!(
        "\n  {:>8} | {:>10} {:>10} {:>10} | {:>10} {:>10} | {:>10}",
        "v0 [m/s]", "v nach 1s", "Theorie", "Faktor", "d_ende", "Soll", "|v| max"
    );
    println!("  {}", "-".repeat(84));

    for v0 in [12.5f32, 30.0, 5.0] {
        let mut app = welt(0.0);
        let l0 = 30.0f32;
        let l1 = 5.0f32;
        let a = anker(&mut app, Vec3::ZERO);
        let s = spieler(&mut app, Vec3::new(0.0, -l0, 0.0), Vec3::new(v0, 0.0, 0.0));
        let j = seil(&mut app, a, s, l0);

        // 30 Ticks einschwingen lassen, dann 60 Ticks einholen.
        for _ in 0..30 {
            app.update();
        }
        let mut v_max = tempo(&app, s).length();
        for i in 1..=60 {
            let laenge = l0 + (l1 - l0) * (i as f32 / 60.0);
            app.world_mut()
                .get_mut::<DistanceJoint>(j)
                .expect("Seil fehlt")
                .limits
                .max = laenge;
            app.update();
            v_max = v_max.max(tempo(&app, s).length());
        }

        let v_ende = tempo(&app, s).length();
        let d_ende = ort(&app, s).length();
        println!(
            "  {v0:>8.2} | {v_ende:>10.4} {:>10.4} {:>10.4} | {d_ende:>10.5} {l1:>10.2} | {v_max:>10.4}",
            v0 * l0 / l1,
            v_ende / v0
        );
    }

    println!(
        "\n  Faktor nahe 6,0 heisst: avian erhaelt den Drehimpuls, das Einholen beschleunigt.\n\
         \x20 Faktor nahe 1,0 heisst: das Seil zieht den Koerper heran, ohne Schwung zu geben\n\
         \x20 — dann waere F-005 mechanisch wertlos und muesste von Hand nachhelfen."
    );
}

// ---------------------------------------------------------------------------------------
// F4 — RAYCAST
// ---------------------------------------------------------------------------------------

#[derive(Resource, Default)]
struct StrahlAuftrag {
    aktiv: bool,
}

#[derive(Resource, Default)]
struct StrahlErgebnis {
    zeilen: Vec<String>,
}

/// Deterministischer Zufall — kein `rand` in den Abhaengigkeiten, und eine Messung, die
/// sich nicht wiederholen laesst, ist keine.
struct Wuerfel(u64);

impl Wuerfel {
    fn naechste(&mut self) -> f32 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        ((self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32) / (1u32 << 24) as f32
    }
}

fn strahlen_messen(
    raum: SpatialQuery,
    auftrag: Res<StrahlAuftrag>,
    mut erg: ResMut<StrahlErgebnis>,
) {
    if !auftrag.aktiv {
        return;
    }

    let alle = SpatialQueryFilter::DEFAULT;
    let nichts = SpatialQueryFilter::from_mask(LayerMask(0));

    // (1) 1000 zufaellige Strahlen quer durch das Gitter.
    let mut w = Wuerfel(0x1234_5678_9ABC_DEF1);
    let strahlen: Vec<(Vec3, Dir3)> = (0..1000)
        .map(|_| {
            let y = w.naechste() * 54.0;
            let z = -60.0 + w.naechste() * 114.0;
            (Vec3::new(-70.0, y, z), Dir3::X)
        })
        .collect();

    let mut treffer = 0usize;
    let mut summe = 0.0f64;
    let start = Instant::now();
    for (o, d) in &strahlen {
        if let Some(h) = raum.cast_ray(*o, *d, REICHWEITE, true, &alle) {
            treffer += 1;
            summe += h.distance as f64;
        }
    }
    let dauer = start.elapsed();
    erg.zeilen.push(format!(
        "  1000 Strahlen, {REICHWEITE} m, Filter ALLE   : {:>9.2} us gesamt, {:>7.3} us/Strahl, {treffer} Treffer, mittlere Weite {:.2} m",
        dauer.as_secs_f64() * 1e6,
        dauer.as_secs_f64() * 1e6 / 1000.0,
        if treffer > 0 { summe / treffer as f64 } else { 0.0 }
    ));

    // (2) Derselbe Satz, aber die Maske passt auf nichts: der Baum wird voll durchlaufen,
    //     kein Treffer kann die Suche abkuerzen. Das ist der teure Fall.
    let start = Instant::now();
    let mut leer = 0usize;
    for (o, d) in &strahlen {
        if raum.cast_ray(*o, *d, REICHWEITE, true, &nichts).is_none() {
            leer += 1;
        }
    }
    let dauer = start.elapsed();
    erg.zeilen.push(format!(
        "  1000 Strahlen, {REICHWEITE} m, Maske LEER    : {:>9.2} us gesamt, {:>7.3} us/Strahl, {leer} ohne Treffer (voller Durchlauf)",
        dauer.as_secs_f64() * 1e6,
        dauer.as_secs_f64() * 1e6 / 1000.0
    ));

    // (3) Einzelzeiten fuer min/median/max.
    let mut zeiten: Vec<f64> = Vec::with_capacity(1000);
    for (o, d) in &strahlen {
        let t = Instant::now();
        let _ = raum.cast_ray(*o, *d, REICHWEITE, true, &alle);
        zeiten.push(t.elapsed().as_secs_f64() * 1e6);
    }
    zeiten.sort_by(|a, b| a.partial_cmp(b).unwrap());
    erg.zeilen.push(format!(
        "  Einzelzeiten je Strahl              : min {:.3} us, median {:.3} us, p99 {:.3} us, max {:.3} us",
        zeiten[0],
        zeiten[500],
        zeiten[990],
        zeiten[999]
    ));
}

fn schichten_messen(
    raum: SpatialQuery,
    auftrag: Res<StrahlAuftrag>,
    mut erg: ResMut<StrahlErgebnis>,
) {
    if !auftrag.aktiv {
        return;
    }
    let o = Vec3::ZERO;
    let d = Dir3::X;

    let alle = SpatialQueryFilter::DEFAULT;
    let nur_dach = SpatialQueryFilter::from_mask(LayerMask(SCHICHT_DACH));

    let a = raum.cast_ray(o, d, REICHWEITE, true, &alle);
    let b = raum.cast_ray(o, d, REICHWEITE, true, &nur_dach);
    let c = raum.cast_ray_predicate(o, d, REICHWEITE, true, &alle, &|_| true);

    erg.zeilen.push(format!(
        "  Filter ALLE      -> {:?}",
        a.map(|h| (h.distance, h.normal))
    ));
    erg.zeilen.push(format!(
        "  Filter NUR DACH  -> {:?}",
        b.map(|h| (h.distance, h.normal))
    ));
    erg.zeilen.push(format!(
        "  cast_ray_predicate(alle, |_| true) -> {:?}",
        c.map(|h| h.distance)
    ));
}

fn f4_raycast() {
    strich("F4  RAYCAST — wie teuer ist ein 112-m-Strahl gegen 4000 Quader, und filtert er?");

    // --- (a) 4000 statische Quader, Laufzeit ---
    let mut app = welt(0.0);
    app.init_resource::<StrahlAuftrag>();
    app.init_resource::<StrahlErgebnis>();
    app.add_systems(Update, strahlen_messen);

    let mut n = 0;
    for i in 0..20 {
        for j in 0..10 {
            for k in 0..20 {
                app.world_mut().spawn((
                    RigidBody::Static,
                    Collider::cuboid(2.0, 2.0, 2.0),
                    Transform::from_xyz(
                        -60.0 + i as f32 * 6.0,
                        j as f32 * 6.0,
                        -60.0 + k as f32 * 6.0,
                    ),
                ));
                n += 1;
            }
        }
    }
    println!("\n  Szene: {n} statische Quader (2 x 2 x 2 m), Gitter 20 x 10 x 20, Raster 6 m.");
    println!("  Strahl: Ursprung x = -70, Richtung +X, Weite {REICHWEITE} m.");

    // Baum aufbauen und warmlaufen lassen.
    for _ in 0..8 {
        app.update();
    }
    app.world_mut().resource_mut::<StrahlAuftrag>().aktiv = true;
    app.update();
    for zeile in &app.world().resource::<StrahlErgebnis>().zeilen {
        println!("{zeile}");
    }

    // --- (b) Schichten: Wand bei 10 m, Dach bei 20 m ---
    let mut app = welt(0.0);
    app.init_resource::<StrahlAuftrag>();
    app.init_resource::<StrahlErgebnis>();
    app.add_systems(Update, schichten_messen);

    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(WAND_DICKE, 20.0, 20.0),
        Transform::from_xyz(10.0, 0.0, 0.0),
        // Mitglied der Wandschicht, nicht der Dachschicht.
        CollisionLayers::new(LayerMask(SCHICHT_WAND), LayerMask::ALL),
    ));
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(WAND_DICKE, 20.0, 20.0),
        Transform::from_xyz(20.0, 0.0, 0.0),
        CollisionLayers::new(LayerMask(SCHICHT_DACH), LayerMask::ALL),
    ));

    for _ in 0..8 {
        app.update();
    }
    app.world_mut().resource_mut::<StrahlAuftrag>().aktiv = true;
    app.update();

    println!(
        "\n  Schichtprobe: Wand (nicht hakbar) bei x = 10, Dach (hakbar) bei x = 20,\n\
         \x20 Strahl vom Ursprung nach +X. Erwartung: mit Filter ALLE blockiert die Wand\n\
         \x20 bei 9,85 m; mit Maske NUR DACH ist die Wand unsichtbar und der Strahl\n\
         \x20 erreicht das Dach bei 19,85 m."
    );
    for zeile in &app.world().resource::<StrahlErgebnis>().zeilen {
        println!("{zeile}");
    }
}

// ---------------------------------------------------------------------------------------
// F5 — KAPSEL UND SCHRITTWEITE
// ---------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Sicherung {
    Vorgabe,
    OhneSpekulation,
    SweptLinear,
    SweptNichtLinear,
}

impl Sicherung {
    fn name(self) -> &'static str {
        match self {
            Sicherung::Vorgabe => "Vorgabe (Spekulation unbegrenzt)",
            Sicherung::OhneSpekulation => "SpeculativeMargin::ZERO",
            Sicherung::SweptLinear => "SpecMargin 0 + SweptCcd::LINEAR",
            Sicherung::SweptNichtLinear => "SpecMargin 0 + SweptCcd::NON_LINEAR",
        }
    }
}

fn wand_fahren(sicherung: Sicherung, v: f32, ticks: usize) -> (f32, bool) {
    let mut app = welt(0.0);
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(WAND_DICKE, 40.0, 40.0),
        Transform::from_xyz(10.0, 0.0, 0.0),
    ));

    let s = spieler(&mut app, Vec3::ZERO, Vec3::new(v, 0.0, 0.0));
    match sicherung {
        Sicherung::Vorgabe => {}
        Sicherung::OhneSpekulation => {
            app.world_mut().entity_mut(s).insert(SpeculativeMargin::ZERO);
        }
        Sicherung::SweptLinear => {
            app.world_mut()
                .entity_mut(s)
                .insert((SpeculativeMargin::ZERO, SweptCcd::LINEAR));
        }
        Sicherung::SweptNichtLinear => {
            app.world_mut()
                .entity_mut(s)
                .insert((SpeculativeMargin::ZERO, SweptCcd::NON_LINEAR));
        }
    }

    for _ in 0..ticks {
        app.update();
    }
    let x = ort(&app, s).x;
    // Wandmitte 10 m, halbe Dicke 0,15 m. Wer weiter als 10,15 m ist, steckt dahinter.
    (x, x > 10.0 + WAND_DICKE * 0.5)
}

fn f5_kapsel() {
    strich("F5  KAPSEL UND SCHRITTWEITE — tunnelt der Spieler durch eine 0,3-m-Wand?");

    println!(
        "\n  Kapsel r = {SPIELER_RADIUS} m, Gesamthoehe {SPIELER_HOEHE} m (game.ron).\n\
         \x20 Wand 0,3 m dick bei x = 10 (duenner als `welt.wand_min_m` = 0,5 m).\n\
         \x20 Ohne Schwerkraft, damit nur der Aufprall gemessen wird.\n\
         \x20 Bei {TEMPO_MAX} m/s und 60 Hz sind das {:.4} m Weg je Tick.",
        TEMPO_MAX / HZ as f32
    );
    println!(
        "\n  {:>36} | {:>9} | {:>11} | {:>10} | {}",
        "Sicherung", "v [m/s]", "m/Tick", "x_ende [m]", "durch?"
    );
    println!("  {}", "-".repeat(88));

    for sicherung in [
        Sicherung::Vorgabe,
        Sicherung::OhneSpekulation,
        Sicherung::SweptLinear,
        Sicherung::SweptNichtLinear,
    ] {
        for v in [TEMPO_MAX, 150.0, 400.0] {
            let (x, durch) = wand_fahren(sicherung, v, 60);
            println!(
                "  {:>36} | {v:>9.1} | {:>11.4} | {x:>10.4} | {}",
                sicherung.name(),
                v / HZ as f32,
                if durch { "JA, TUNNEL" } else { "nein" }
            );
        }
    }
}

// ---------------------------------------------------------------------------------------
// F6 — TEILSCHRITTE. F1 zeigt Tempoverlust bei kurzem Seil. Ist das ein RON-Schalter?
// ---------------------------------------------------------------------------------------

fn f6_teilschritte() {
    strich("F6  TEILSCHRITTE — laesst sich der Tempoverlust aus F1 mit `SubstepCount` kaufen?");

    println!(
        "\n  Ohne Schwerkraft, Kreisbahn, 600 Ticks = 10 s. avians Vorgabe ist 6 Teilschritte.\n\
         \x20 Wenn der Verlust mit der Teilschrittzahl faellt, ist er ein Projektionsfehler\n\
         \x20 und keine Daempfung — dann ist er kaufbar."
    );
    println!(
        "\n  {:>6} {:>7} | {:>8} {:>10} {:>10} {:>10} {:>12}",
        "L [m]", "v0", "Substeps", "v(1s)", "v(10s)", "Verlust/s", "Zeit/600 Tick"
    );
    println!("  {}", "-".repeat(76));

    for (laenge, v0) in [(SEIL_MIN, TEMPO_MAX), (5.0f32, 60.0f32), (20.0, 30.0)] {
        for n in [6u32, 12, 24, 48] {
            let start = Instant::now();
            let f = pendel_fahren_mit(0.0, laenge, v0, 600, Some(n));
            let dauer = start.elapsed();
            let verlust = (1.0 - (f.tempo_ende / f.tempo_start).powf(0.1)) * 100.0;
            println!(
                "  {laenge:>6.1} {v0:>7.1} | {n:>8} {:>10.4} {:>10.4} {verlust:>9.4} % {:>10.1} ms",
                f.marken[0],
                f.tempo_ende,
                dauer.as_secs_f64() * 1e3
            );
        }
    }
}

// ---------------------------------------------------------------------------------------
// F7 — EINPASSUNG. Laesst sich avian in den `SimulationSystems`-Ablauf dieses Projekts zwingen?
// ---------------------------------------------------------------------------------------

#[derive(Resource, Default)]
struct Protokoll {
    zeilen: Vec<String>,
    y_vor: f32,
    y_nach: f32,
}

#[derive(Component)]
struct Beobachtet;

fn merken_absicht(mut p: ResMut<Protokoll>, q: Query<&Position, With<Beobachtet>>) {
    if p.zeilen.len() < 8 {
        p.zeilen.push("Absicht".into());
    }
    if let Ok(pos) = q.single() {
        p.y_vor = pos.0.y;
    }
}

fn merken_nachlauf(mut p: ResMut<Protokoll>, q: Query<&Position, With<Beobachtet>>) {
    if p.zeilen.len() < 8 {
        p.zeilen.push("Nachlauf".into());
    }
    if let Ok(pos) = q.single() {
        p.y_nach = pos.0.y;
    }
}

fn f7_einpassung() {
    strich("F7  EINPASSUNG — avian in FixedUpdate, INNERHALB von SimulationSystems::Integrate");

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // `PhysicsPlugins::new(schedule)` waehlt den Zeitplan (avian3d-0.7.0/src/lib.rs:690-695).
    app.add_plugins(PhysicsPlugins::new(FixedUpdate));
    app.insert_resource(Gravity(Vec3::new(0.0, SCHWERKRAFT, 0.0)));
    app.insert_resource(Time::<Fixed>::from_hz(HZ));
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.init_resource::<Protokoll>();

    // Die sechs Stufen aus src/lib.rs:98-109 — die ECHTEN, aus dem Crate importiert.
    app.configure_sets(
        FixedUpdate,
        (
            SimulationSystems::Spatial,
            SimulationSystems::World,
            SimulationSystems::Intent,
            SimulationSystems::Drive,
            SimulationSystems::Integrate,
            SimulationSystems::PostStep,
        )
            .chain(),
    );
    // Und avians fuenf Stufen komplett IN `Vollzug`. Nur alle fuenf gemeinsam: sie sind in
    // `PhysicsSchedulePlugin` bereits `.chain()`-verkettet (avian3d-0.7.0/src/schedule/mod.rs:74-85),
    // und wer nur eine davon hineinsteckt, baut sich einen Zyklus.
    app.configure_sets(
        FixedUpdate,
        (
            PhysicsSystems::First,
            PhysicsSystems::Prepare,
            PhysicsSystems::StepSimulation,
            PhysicsSystems::Writeback,
            PhysicsSystems::Last,
        )
            .in_set(SimulationSystems::Integrate),
    );
    app.add_systems(FixedUpdate, merken_absicht.in_set(SimulationSystems::Intent));
    app.add_systems(FixedUpdate, merken_nachlauf.in_set(SimulationSystems::PostStep));
    app.finish();
    app.cleanup();

    let e = spieler(&mut app, Vec3::new(0.0, 100.0, 0.0), Vec3::ZERO);
    app.world_mut().entity_mut(e).insert(Beobachtet);

    for _ in 0..60 {
        app.update();
    }

    let p = app.world().resource::<Protokoll>();
    let t = 60.0 / HZ as f32;
    println!("\n  Reihenfolge der ersten Ticks : {:?}", p.zeilen);
    println!(
        "  y in `Absicht`  (vor Physik) : {:.6} m\n  \
           y in `Nachlauf` (nach Physik): {:.6} m\n  \
           Differenz in EINEM Tick      : {:.6} m",
        p.y_vor,
        p.y_nach,
        p.y_nach - p.y_vor
    );
    println!(
        "  y nach 60 Ticks              : {:.4} m  (freier Fall aus 100 m: {:.4} m)",
        ort(&app, e).y,
        100.0 + 0.5 * SCHWERKRAFT * t * t
    );
    println!(
        "  ERGEBNIS: kein Zyklus, kein Absturz — die Physik liegt zwischen `Absicht` und\n\
         \x20 `Nachlauf`, also genau dort, wo `SimulationSystems::Integrate` sie haben will."
    );
}

// ---------------------------------------------------------------------------------------
// F8 — KLEMME (F-012), BOOST (F-007), WIEDERHOLBARKEIT
// ---------------------------------------------------------------------------------------

#[derive(Component)]
struct Boostet;

/// F-007: Boost als Beschleunigung ueber die `Forces`-QueryData
/// (avian3d-0.7.0/src/dynamics/rigid_body/forces/mod.rs:83-95).
fn boost_geben(mut q: Query<Forces, With<Boostet>>) {
    for mut kraefte in &mut q {
        kraefte.apply_linear_acceleration(Vec3::new(BOOST_M_S2, 0.0, 0.0));
    }
}

/// `vector.boost_m_s2`
const BOOST_M_S2: f32 = 34.0;

fn f8_klemme_boost_wiederholbar() {
    strich("F8  KLEMME (F-012), BOOST (F-007) und WIEDERHOLBARKEIT");

    // --- Klemme ---
    println!(
        "\n  (a) `MaxLinearSpeed` ist avians eingebaute Klemme; sie laeuft im SubstepSchedule\n\
         \x20     nach `integrate_velocities` (avian3d-0.7.0/src/dynamics/integrator/mod.rs:81-83,467).\n\
         \x20     Boost {BOOST_M_S2} m/s^2 ueber 10 s, ohne Schwerkraft. Ohne Klemme waeren das\n\
         \x20     {} m/s.",
        BOOST_M_S2 * 10.0
    );
    println!(
        "\n  {:>28} | {:>12} {:>12} {:>12}",
        "Aufbau", "v nach 2 s", "v nach 5 s", "v nach 10 s"
    );
    println!("  {}", "-".repeat(70));

    for mit_klemme in [false, true] {
        let mut app = welt(0.0);
        app.add_systems(FixedUpdate, boost_geben);
        let s = spieler(&mut app, Vec3::ZERO, Vec3::ZERO);
        app.world_mut().entity_mut(s).insert(Boostet);
        if mit_klemme {
            app.world_mut().entity_mut(s).insert(MaxLinearSpeed(TEMPO_MAX));
        }
        let mut marken = [0.0f32; 3];
        for tick in 1..=600 {
            app.update();
            match tick {
                120 => marken[0] = tempo(&app, s).length(),
                300 => marken[1] = tempo(&app, s).length(),
                600 => marken[2] = tempo(&app, s).length(),
                _ => {}
            }
        }
        println!(
            "  {:>28} | {:>12.4} {:>12.4} {:>12.4}",
            if mit_klemme {
                format!("MaxLinearSpeed({TEMPO_MAX})")
            } else {
                "ohne Klemme".into()
            },
            marken[0],
            marken[1],
            marken[2]
        );
    }

    // --- Wiederholbarkeit ---
    println!(
        "\n  (b) Zwei identische Fahrten im SELBEN Prozess, bitweise verglichen.\n\
         \x20     Das prueft Wiederholbarkeit auf DIESER Maschine, NICHT Determinismus\n\
         \x20     ueber Architekturen hinweg — dafuer waere `enhanced-determinism` da,\n\
         \x20     und avian testet das in 0.7.0 nur fuer 2D (src/tests/mod.rs:14)."
    );

    let ergebnis = |_i: usize| -> (u32, u32, u32, u32, u32, u32) {
        let mut app = welt(SCHWERKRAFT);
        let a = anker(&mut app, Vec3::new(0.0, 20.0, 0.0));
        let s = spieler(&mut app, Vec3::ZERO, Vec3::new(30.0, 0.0, 7.0));
        seil(&mut app, a, s, 20.0);
        // Ein zweiter Koerper, damit es Kontakte und damit Loeser-Inseln gibt.
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(200.0, 1.0, 200.0),
            Transform::from_xyz(0.0, -21.0, 0.0),
        ));
        for _ in 0..600 {
            app.update();
        }
        let p = ort(&app, s);
        let v = tempo(&app, s);
        (
            p.x.to_bits(),
            p.y.to_bits(),
            p.z.to_bits(),
            v.x.to_bits(),
            v.y.to_bits(),
            v.z.to_bits(),
        )
    };

    let a = ergebnis(0);
    let b = ergebnis(1);
    println!(
        "\n     Fahrt 1 (Rohbits) : {:08x} {:08x} {:08x} | {:08x} {:08x} {:08x}",
        a.0, a.1, a.2, a.3, a.4, a.5
    );
    println!(
        "     Fahrt 2 (Rohbits) : {:08x} {:08x} {:08x} | {:08x} {:08x} {:08x}",
        b.0, b.1, b.2, b.3, b.4, b.5
    );
    println!(
        "     GLEICH: {}",
        if a == b { "JA, bitweise" } else { "NEIN" }
    );
}

// ---------------------------------------------------------------------------------------
// F9 bis F12 — der Fund der Gegenprobe wird GEMESSEN, nicht gerechnet.
//
// Werte aus assets/data/game.ron und assets/data/maps.ron, gelesen am 2026-08-09
// unmittelbar vor dieser Messung:
//   game.ron  vector.seilzug_m_s   = 28.0     vector.seil_min_m  = 3.0
//             welt.wand_min_m      = 0.5      spieler.radius_m   = 0.35
//             spieler.hoehe_m      = 1.8      schwerkraft_m_s2   = -20.0
//             simulation_hz        = 60.0     vector.tempo_max_m_s = 75.0
//             vector.hakenreichweite_m = 90.0 (frueher 112 — `REICHWEITE` oben ist
//             ALT und wird von F4 benutzt; F9-F12 brauchen sie nicht)
//   maps.ron  raster.gasse_m       = 7.0      raster.hoehe_min_m = 4.5
//             raster.hoehe_max_m   = 11.5     Kirche 35 m, Baum/Wachturm 12 m
// ---------------------------------------------------------------------------------------

/// `vector.seilzug_m_s` — die Rate, mit der F-005 das Seil verkuerzt.
const SEILZUG: f32 = 28.0;
/// `welt.wand_min_m` — die duennste Wand, die es im Spiel geben darf.
const WAND_MIN: f32 = 0.5;
/// `maps.ron: karten.graubox.raster.gasse_m`
const GASSE: f32 = 7.0;
/// `maps.ron: karten.graubox.raster.hoehe_max_m` — Firsthoehe des groessten Wohnhauses.
const HAUS_MAX: f32 = 11.5;

/// Der Spieler, damit die Messsysteme ihn von Anker und Wand unterscheiden koennen.
/// **Kein `.single()`** (docs/multiplayer.md) — die Systeme laufen ueber eine Query.
#[derive(Component)]
struct Held;

/// Wie die Wandprobe aufgebaut ist. Die Wand liegt bei x = 0 und ist `WAND_MIN` dick,
/// der Spieler startet bei x = -10, der Anker steht bei x = +10 DAHINTER.
const WAND_X: f32 = 0.0;
const START_X: f32 = -10.0;
const ANKER_X: f32 = 10.0;
/// Vordere Wandflaeche (die dem Spieler zugewandte).
const WAND_VORN: f32 = WAND_X - WAND_MIN * 0.5;
/// Hintere Wandflaeche.
const WAND_HINTEN: f32 = WAND_X + WAND_MIN * 0.5;

/// Wie eingeholt wird.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Einholart {
    /// F-005 wie geplant: `limits.max` EINMAL je Tick verkuerzen.
    ProTick,
    /// Reparatur (3): `limits.max` in JEDEM Teilschritt um `rate/(hz*substeps)` verkuerzen.
    ProTeilschritt,
    /// Reparatur (4a): nicht die Laenge steuern, sondern die GESCHWINDIGKEIT.
    UeberTempo,
}

#[derive(Resource, Clone, Copy)]
struct Einholplan {
    rate_m_s: f32,
    min_m: f32,
    teilschritte: u32,
}

/// Reparatur (3). Dass das ueberhaupt geht, steht im Quelltext:
/// `solve_xpbd_joint::<DistanceJoint>` laeuft im `SubstepSchedule` und liest die Komponente
/// `&mut DistanceJoint` in JEDEM Teilschritt neu
/// (avian3d-0.7.0/src/dynamics/solver/xpbd/plugin.rs:160-203), und
/// `.../xpbd/joints/distance.rs:80` benutzt dabei `self.limits` direkt. Eine Aenderung im
/// `SubstepSchedule` wirkt also sofort im naechsten Teilschritt.
fn seil_kuerzen_im_teilschritt(plan: Res<Einholplan>, mut seile: Query<&mut DistanceJoint>) {
    let schritt = plan.rate_m_s / (HZ as f32 * plan.teilschritte as f32);
    for mut j in &mut seile {
        j.limits.max = (j.limits.max - schritt).max(plan.min_m);
    }
}

#[derive(Resource, Clone, Copy)]
struct Tempoplan {
    anker: Vec3,
    rate_m_s: f32,
    min_m: f32,
}

/// Reparatur (4a). `DistanceJoint` hat in 0.7.0 KEINEN Motor (die Struktur hat genau die
/// Felder `body1, body2, anchor1, anchor2, limits, compliance` —
/// avian3d-0.7.0/src/dynamics/joints/distance.rs:26-39). `LinearMotor` gibt es nur als Feld
/// von `PrismaticJoint` (.../joints/prismatic.rs:55), `AngularMotor` nur an `RevoluteJoint`
/// (.../joints/revolute.rs:74). Also wird das Einholen hier ueber die Geschwindigkeit
/// gefahren: die Radialkomponente wird auf `rate_m_s` gesetzt, und das Seil nimmt nur
/// Schlaff auf (`limits.max` folgt dem Abstand nach unten, zieht aber nie selbst).
fn einholen_ueber_tempo(
    plan: Res<Tempoplan>,
    mut held: Query<(&Position, &mut LinearVelocity), With<Held>>,
    mut seile: Query<&mut DistanceJoint>,
) {
    for (p, mut v) in &mut held {
        let nach_anker = plan.anker - p.0;
        let d = nach_anker.length();
        if d > plan.min_m {
            let richtung = nach_anker / d;
            let radial = v.0.dot(richtung);
            if radial < plan.rate_m_s {
                v.0 += richtung * (plan.rate_m_s - radial);
            }
        }
        for mut j in &mut seile {
            j.limits.max = j.limits.max.min(d.max(plan.min_m));
        }
    }
}

#[derive(Clone, Copy)]
struct Wandaufbau {
    rate_m_s: f32,
    art: Einholart,
    teilschritte: u32,
    max_overlap: f32,
    compliance: f32,
    ticks: usize,
}

impl Default for Wandaufbau {
    fn default() -> Self {
        Self {
            rate_m_s: SEILZUG,
            art: Einholart::ProTick,
            teilschritte: 6,
            max_overlap: 4.0,
            compliance: 0.0,
            ticks: 700,
        }
    }
}

struct Wandprobe {
    /// Wie weit die Vorderseite der Kapsel in die Wand eingedrungen ist, gedeckelt auf
    /// „ganz hindurch" = `WAND_MIN + 2*radius`.
    tiefe_max: f32,
    x_max: f32,
    x_ende: f32,
    /// Erster Tick, in dem der Mittelpunkt hinter der Wandmitte liegt.
    tick_mitte: Option<usize>,
    /// Erster Tick, in dem die GANZE Kapsel hinter der Wand liegt.
    tick_durch: Option<usize>,
    /// Steckt er am Ende noch in der Wand?
    drin_ende: bool,
    v_max: f32,
    l_ende: f32,
    ms: f64,
}

fn seil_gegen_wand(a: Wandaufbau) -> Wandprobe {
    seil_gegen_wand_mit_spur(a, 0)
}

fn seil_gegen_wand_mit_spur(a: Wandaufbau, spur: usize) -> Wandprobe {
    let uhr = Instant::now();
    let mut app = welt_mit_substeps(0.0, Some(a.teilschritte));
    app.insert_resource(avian3d::dynamics::solver::SolverConfig {
        max_overlap_solve_speed: a.max_overlap,
        ..default()
    });

    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(WAND_MIN, 40.0, 40.0),
        Transform::from_xyz(WAND_X, 0.0, 0.0),
    ));

    let anker_ort = Vec3::new(ANKER_X, 0.0, 0.0);
    let a_e = anker(&mut app, anker_ort);
    let s = spieler(&mut app, Vec3::new(START_X, 0.0, 0.0), Vec3::ZERO);
    app.world_mut().entity_mut(s).insert(Held);

    let l0 = ANKER_X - START_X;
    let j = seil(&mut app, a_e, s, l0);
    if a.compliance > 0.0 {
        app.world_mut()
            .get_mut::<DistanceJoint>(j)
            .expect("Seil fehlt")
            .compliance = a.compliance;
    }

    match a.art {
        Einholart::ProTeilschritt => {
            app.insert_resource(Einholplan {
                rate_m_s: a.rate_m_s,
                min_m: SEIL_MIN,
                teilschritte: a.teilschritte,
            });
            app.add_systems(
                SubstepSchedule,
                seil_kuerzen_im_teilschritt
                    .before(avian3d::dynamics::solver::schedule::SubstepSolverSystems::WarmStart)
                    .ambiguous_with_all(),
            );
        }
        Einholart::UeberTempo => {
            app.insert_resource(Tempoplan {
                anker: anker_ort,
                rate_m_s: a.rate_m_s,
                min_m: SEIL_MIN,
            });
            app.add_systems(FixedUpdate, einholen_ueber_tempo);
        }
        _ => {}
    }

    let mut p = Wandprobe {
        tiefe_max: 0.0,
        x_max: START_X,
        x_ende: START_X,
        tick_mitte: None,
        tick_durch: None,
        drin_ende: false,
        v_max: 0.0,
        l_ende: l0,
        ms: 0.0,
    };

    if spur > 0 {
        println!(
            "\n    {:>6} | {:>9} | {:>10} | {:>11} | {:>10} | {:>9}",
            "Tick", "L [m]", "x [m]", "Abstand", "Eindringen", "|v| [m/s]"
        );
        println!("    {}", "-".repeat(72));
    }

    for tick in 1..=a.ticks {
        if a.art == Einholart::ProTick {
            let mut seilj = app.world_mut().get_mut::<DistanceJoint>(j).expect("Seil fehlt");
            seilj.limits.max = (seilj.limits.max - a.rate_m_s / HZ as f32).max(SEIL_MIN);
        }
        app.update();

        let o = ort(&app, s);
        let v = tempo(&app, s);
        let vorn = o.x + SPIELER_RADIUS;
        let tiefe = (vorn - WAND_VORN).clamp(0.0, WAND_MIN + 2.0 * SPIELER_RADIUS);
        let laenge = app
            .world()
            .get::<DistanceJoint>(j)
            .expect("Seil fehlt")
            .limits
            .max;

        p.tiefe_max = p.tiefe_max.max(tiefe);
        p.x_max = p.x_max.max(o.x);
        p.v_max = p.v_max.max(v.length());
        p.x_ende = o.x;
        p.l_ende = laenge;
        if p.tick_mitte.is_none() && o.x > WAND_X {
            p.tick_mitte = Some(tick);
        }
        if p.tick_durch.is_none() && o.x - SPIELER_RADIUS > WAND_HINTEN {
            p.tick_durch = Some(tick);
        }

        if tick <= spur {
            println!(
                "    {tick:>6} | {laenge:>9.4} | {:>10.5} | {:>11.5} | {:>10.5} | {:>9.4}",
                o.x,
                WAND_VORN - vorn,
                tiefe,
                v.length()
            );
        }
    }

    p.drin_ende = p.x_ende + SPIELER_RADIUS > WAND_VORN && p.x_ende - SPIELER_RADIUS < WAND_HINTEN;
    p.ms = uhr.elapsed().as_secs_f64() * 1e3;
    p
}

/// Kopfzeile fuer die Wandtabellen.
fn wandkopf(erste: &str) {
    println!(
        "\n  {erste:>28} | {:>10} {:>10} | {:>8} {:>8} | {:>7} | {:>9} {:>8}",
        "Tiefe max", "x max", "Tick>0", "Tick durch", "drin?", "|v| max", "ms"
    );
    println!("  {}", "-".repeat(104));
}

fn wandzeile(name: &str, p: &Wandprobe) {
    println!(
        "  {name:>28} | {:>10.5} {:>10.4} | {:>8} {:>8} | {:>7} | {:>9.3} {:>8.0}",
        p.tiefe_max,
        p.x_max,
        p.tick_mitte.map(|t| t.to_string()).unwrap_or_else(|| "-".into()),
        p.tick_durch.map(|t| t.to_string()).unwrap_or_else(|| "-".into()),
        if p.drin_ende { "JA" } else { "nein" },
        p.v_max,
        p.ms
    );
}

fn f9_seil_gegen_wand() {
    strich("F9  SEIL GEGEN WAND — zieht das Seil den Spieler durch eine 0,5-m-Wand?");

    println!(
        "\n  Aufbau: statische Wand {WAND_MIN} m dick bei x = 0 (Flaechen {WAND_VORN} / {WAND_HINTEN}),\n\
         \x20 Spieler (Kapsel r = {SPIELER_RADIUS} m) bei x = {START_X}, Anker DAHINTER bei x = {ANKER_X}.\n\
         \x20 Seil `DistanceJoint` mit limits = [0, 20]. Dann wird `limits.max` mit der Rate\n\
         \x20 verkuerzt, bis `seil_min_m` = {SEIL_MIN} m erreicht ist — genau das, was F-005 taete.\n\
         \x20 Ohne Schwerkraft, damit NUR der Seilzug wirkt. 700 Ticks = 11,67 s.\n\
         \x20 `Tiefe max` ist gedeckelt auf {:.2} m = ganz hindurch.",
        WAND_MIN + 2.0 * SPIELER_RADIUS
    );

    println!("\n  (a) Spur der ersten 20 Ticks bei der RON-Rate {SEILZUG} m/s:");
    let p28 = seil_gegen_wand_mit_spur(
        Wandaufbau {
            rate_m_s: SEILZUG,
            ..default()
        },
        20,
    );

    println!("\n  (b) Drei Verkuerzungsraten:");
    wandkopf("Rate");
    wandzeile(&format!("{SEILZUG} m/s (RON)"), &p28);
    for rate in [8.0f32, 2.0] {
        let p = seil_gegen_wand(Wandaufbau {
            rate_m_s: rate,
            ..default()
        });
        wandzeile(&format!("{rate} m/s"), &p);
    }

    println!(
        "\n  (c) OHNE Einholen — reines Pendeln in eine Wand. Anker (-8, 10, 0), Seil 10 m,\n\
         \x20 Spieler startet unter dem Anker mit v0 nach +X, Schwerkraft {SCHWERKRAFT} m/s^2.\n\
         \x20 Die Kreisbahn kreuzt die Wandebene bei y = 4 m; dort ist die Bahn 37 Grad zur\n\
         \x20 Wandnormalen geneigt, der Spieler faehrt also schraeg hinein."
    );
    println!(
        "\n  {:>10} | {:>10} {:>10} | {:>8} {:>10} | {:>7} | {:>9}",
        "v0 [m/s]", "Tiefe max", "x max", "Tick>0", "Tick durch", "drin?", "|v| max"
    );
    println!("  {}", "-".repeat(84));
    for v0 in [20.0f32, 35.0, 50.0] {
        let p = pendel_in_wand(v0, 300);
        println!(
            "  {v0:>10.1} | {:>10.5} {:>10.4} | {:>8} {:>10} | {:>7} | {:>9.3}",
            p.tiefe_max,
            p.x_max,
            p.tick_mitte.map(|t| t.to_string()).unwrap_or_else(|| "-".into()),
            p.tick_durch.map(|t| t.to_string()).unwrap_or_else(|| "-".into()),
            if p.drin_ende { "JA" } else { "nein" },
            p.v_max
        );
    }
}

/// Pendel, das in eine Wand schwingt. Kein Einholen, nur Schwung.
fn pendel_in_wand(v0: f32, ticks: usize) -> Wandprobe {
    let uhr = Instant::now();
    let mut app = welt(SCHWERKRAFT);
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(WAND_MIN, 40.0, 40.0),
        Transform::from_xyz(WAND_X, 0.0, 0.0),
    ));
    let anker_ort = Vec3::new(-8.0, 10.0, 0.0);
    let a_e = anker(&mut app, anker_ort);
    let s = spieler(&mut app, Vec3::new(-8.0, 0.0, 0.0), Vec3::new(v0, 0.0, 0.0));
    app.world_mut().entity_mut(s).insert(Held);
    seil(&mut app, a_e, s, 10.0);

    let mut p = Wandprobe {
        tiefe_max: 0.0,
        x_max: -8.0,
        x_ende: -8.0,
        tick_mitte: None,
        tick_durch: None,
        drin_ende: false,
        v_max: v0,
        l_ende: 10.0,
        ms: 0.0,
    };
    for tick in 1..=ticks {
        app.update();
        let o = ort(&app, s);
        let v = tempo(&app, s);
        let vorn = o.x + SPIELER_RADIUS;
        p.tiefe_max = p
            .tiefe_max
            .max((vorn - WAND_VORN).clamp(0.0, WAND_MIN + 2.0 * SPIELER_RADIUS));
        p.x_max = p.x_max.max(o.x);
        p.v_max = p.v_max.max(v.length());
        p.x_ende = o.x;
        if p.tick_mitte.is_none() && o.x > WAND_X {
            p.tick_mitte = Some(tick);
        }
        if p.tick_durch.is_none() && o.x - SPIELER_RADIUS > WAND_HINTEN {
            p.tick_durch = Some(tick);
        }
    }
    p.drin_ende = p.x_ende + SPIELER_RADIUS > WAND_VORN && p.x_ende - SPIELER_RADIUS < WAND_HINTEN;
    p.ms = uhr.elapsed().as_secs_f64() * 1e3;
    p
}

// ---------------------------------------------------------------------------------------
// F10 — DIE VIER REPARATUREN
// ---------------------------------------------------------------------------------------

/// Nebenwirkung von `max_overlap_solve_speed`: zittert ein liegender Koerper, und wie
/// schnell wird ein ueberlappender herausgeschossen?
///
/// Rueckgabe: (Streuung in mm ueber 60 Ticks, |v| max im Ruhezustand,
///             |v_y| max beim Herausdruecken aus 0,2 m Ueberlappung,
///             Tick, ab dem die Ueberlappung aufgeloest ist)
fn zittern(max_overlap: f32) -> (f32, f32, f32, Option<usize>) {
    // --- (a) ruhender Koerper ---
    let mut app = welt_mit_substeps(SCHWERKRAFT, None);
    app.insert_resource(avian3d::dynamics::solver::SolverConfig {
        max_overlap_solve_speed: max_overlap,
        ..default()
    });
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(50.0, 1.0, 50.0),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));
    let k = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::cuboid(1.0, 1.0, 1.0),
            SleepingDisabled,
            Transform::from_xyz(0.0, 0.5, 0.0),
        ))
        .id();
    for _ in 0..180 {
        app.update();
    }
    let mut y_min = f32::MAX;
    let mut y_max = f32::MIN;
    let mut v_max = 0.0f32;
    for _ in 0..60 {
        app.update();
        let o = ort(&app, k);
        let v = tempo(&app, k);
        y_min = y_min.min(o.y);
        y_max = y_max.max(o.y);
        v_max = v_max.max(v.length());
    }

    // --- (b) 0,2 m ueberlappend eingesetzt ---
    let mut app = welt_mit_substeps(SCHWERKRAFT, None);
    app.insert_resource(avian3d::dynamics::solver::SolverConfig {
        max_overlap_solve_speed: max_overlap,
        ..default()
    });
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(50.0, 1.0, 50.0),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));
    let k2 = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::cuboid(1.0, 1.0, 1.0),
            SleepingDisabled,
            Transform::from_xyz(0.0, 0.3, 0.0),
        ))
        .id();
    let mut pop = 0.0f32;
    let mut frei = None;
    for tick in 1..=120 {
        app.update();
        pop = pop.max(tempo(&app, k2).y);
        if frei.is_none() && ort(&app, k2).y > 0.495 {
            frei = Some(tick);
        }
    }

    ((y_max - y_min) * 1000.0, v_max, pop, frei)
}

/// Nebenwirkung von `compliance`: wie stark dehnt sich das Seil im normalen Pendeln, und
/// wie viel Schwung frisst es?
///
/// Rueckgabe: (Dehnung max in m, Verlust je Sekunde in %, Energiedrift in %)
fn pendel_dehnung(compliance: f32, laenge: f32, v0: f32) -> (f32, f32, f32) {
    let mut app = welt(SCHWERKRAFT);
    let a = anker(&mut app, Vec3::new(0.0, laenge, 0.0));
    let s = spieler(&mut app, Vec3::ZERO, Vec3::new(v0, 0.0, 0.0));
    let j = seil(&mut app, a, s, laenge);
    if compliance > 0.0 {
        app.world_mut()
            .get_mut::<DistanceJoint>(j)
            .expect("Seil fehlt")
            .compliance = compliance;
    }
    let anker_ort = Vec3::new(0.0, laenge, 0.0);
    let energie = |p: Vec3, v: Vec3| 0.5 * v.length_squared() - SCHWERKRAFT * p.y;
    let e0 = energie(Vec3::ZERO, Vec3::new(v0, 0.0, 0.0));

    let mut dehnung = 0.0f32;
    let mut v_ende = v0;
    let mut e_ende = e0;
    for _ in 0..600 {
        app.update();
        let o = ort(&app, s);
        let v = tempo(&app, s);
        dehnung = dehnung.max((o - anker_ort).length() - laenge);
        v_ende = v.length();
        e_ende = energie(o, v);
    }
    let _ = v_ende;
    (
        dehnung,
        (1.0 - (e_ende / e0).max(0.0).powf(0.1)) * 100.0,
        (e_ende - e0) / e0 * 100.0,
    )
}

fn f10_reparaturen() {
    strich("F10 DIE VIER REPARATUREN — welche haelt die Wand, und was kostet sie?");

    println!(
        "\n  Derselbe Aufbau wie F9, immer mit {SEILZUG} m/s. Vergleichswert ist die Zeile\n\
         \x20 „Vorgabe“ — das ist F9 (b) mit avians Voreinstellungen."
    );

    let vorgabe = seil_gegen_wand(Wandaufbau::default());
    wandkopf("Reparatur");
    wandzeile("Vorgabe (nichts geaendert)", &vorgabe);

    // --- (1) max_overlap_solve_speed ---
    for m in [20.0f32, 100.0, 1000.0] {
        let p = seil_gegen_wand(Wandaufbau {
            max_overlap: m,
            ..default()
        });
        wandzeile(&format!("(1) max_overlap = {m}"), &p);
    }

    // --- (2) compliance ---
    for c in [1e-7f32, 1e-6, 1e-5, 1e-4, 1e-3, 1e-2] {
        let p = seil_gegen_wand(Wandaufbau {
            compliance: c,
            ..default()
        });
        wandzeile(&format!("(2) compliance = {c:e}"), &p);
    }

    // --- (3) Verkuerzung auf die Teilschritte verteilen ---
    for n in [6u32, 12, 24] {
        let p = seil_gegen_wand(Wandaufbau {
            art: Einholart::ProTeilschritt,
            teilschritte: n,
            ..default()
        });
        wandzeile(&format!("(3) je Teilschritt, n = {n}"), &p);
        // Gegenprobe: dieselbe Teilschrittzahl, aber weiterhin je TICK verkuerzt.
        let q = seil_gegen_wand(Wandaufbau {
            art: Einholart::ProTick,
            teilschritte: n,
            ..default()
        });
        wandzeile(&format!("    (Vergleich je Tick, n = {n})"), &q);
    }

    // --- (4) Motor ---
    println!(
        "\n  (4) MOTOR — nicht baubar wie beauftragt. `DistanceJoint` hat in avian3d 0.7.0\n\
         \x20     KEIN Motorfeld: die Struktur ist `body1, body2, anchor1, anchor2, limits,\n\
         \x20     compliance` (src/dynamics/joints/distance.rs:26-39). Es gibt genau zwei\n\
         \x20     Motoren, und beide haengen an einem anderen Gelenk:\n\
         \x20       `LinearMotor`  -> Feld von `PrismaticJoint` (joints/prismatic.rs:55, :319)\n\
         \x20       `AngularMotor` -> Feld von `RevoluteJoint`  (joints/revolute.rs:74, :349)\n\
         \x20     Ein `PrismaticJoint` waere kein Seil: er zwingt die Koerper zusaetzlich auf\n\
         \x20     EINE Achse (`slider_axis`, joints/prismatic.rs:42-45) und haelt die Drehung\n\
         \x20     fest — ein Schwingen um den Anker gibt es damit nicht.\n\
         \x20     Gemessen wird deshalb (4a): einholen ueber die GESCHWINDIGKEIT statt ueber\n\
         \x20     die Laenge. Das Seil nimmt nur Schlaff auf; der Zug ist eine Radialkomponente\n\
         \x20     der Geschwindigkeit — und Geschwindigkeit ist genau das, was der\n\
         \x20     Kontaktloeser sehen und wegnehmen kann."
    );
    wandkopf("Reparatur");
    for n in [6u32, 12, 24] {
        let p = seil_gegen_wand(Wandaufbau {
            art: Einholart::UeberTempo,
            teilschritte: n,
            ..default()
        });
        wandzeile(&format!("(4a) Tempo statt Laenge, n = {n}"), &p);
    }

    // --- Nebenwirkung (1) ---
    println!(
        "\n  NEBENWIRKUNG (1) — `max_overlap_solve_speed` hochsetzen.\n\
         \x20 (a) Wuerfel 1 m liegt 180 Ticks auf dem Boden, dann 60 Ticks gemessen.\n\
         \x20 (b) derselbe Wuerfel wird 0,2 m IM Boden eingesetzt; gemessen wird, wie schnell\n\
         \x20     er herausgeschossen wird."
    );
    println!(
        "\n  {:>22} | {:>16} {:>14} | {:>18} {:>16}",
        "max_overlap [m/s]", "Streuung [mm]", "|v| max [m/s]", "Auswurf v_y [m/s]", "frei ab Tick"
    );
    println!("  {}", "-".repeat(96));
    for m in [4.0f32, 20.0, 100.0, 1000.0] {
        let (streu, vmax, pop, frei) = zittern(m);
        println!(
            "  {:>22} | {streu:>16.4} {vmax:>14.5} | {pop:>18.4} {:>16}",
            if m == 4.0 {
                format!("{m} (Vorgabe)")
            } else {
                format!("{m}")
            },
            frei.map(|t| t.to_string()).unwrap_or_else(|| ">120".into())
        );
    }

    // --- Nebenwirkung (2) ---
    println!(
        "\n  NEBENWIRKUNG (2) — `compliance` im NORMALEN Pendeln (Schwerkraft {SCHWERKRAFT},\n\
         \x20 600 Ticks = 10 s). `Dehnung` ist der groesste Abstand ueber der Seillaenge."
    );
    println!(
        "\n  {:>16} | {:>14} {:>14} | {:>14} {:>14}",
        "compliance", "L=11 Dehnung", "L=11 Drift", "L=5 Dehnung", "L=5 Drift"
    );
    println!("  {}", "-".repeat(80));
    for c in [0.0f32, 1e-7, 1e-6, 1e-5, 1e-4, 1e-3, 1e-2] {
        let (d11, _, drift11) = pendel_dehnung(c, HAUS_MAX, 20.0);
        let (d5, _, drift5) = pendel_dehnung(c, 5.0, 20.0);
        println!(
            "  {:>16} | {d11:>13.5}m {drift11:>13.4}% | {d5:>13.5}m {drift5:>13.4}%",
            if c == 0.0 {
                "0 (Vorgabe)".to_string()
            } else {
                format!("{c:e}")
            }
        );
    }
}

// ---------------------------------------------------------------------------------------
// F11 — TEILSCHRITTE GEGEN DIE ECHTEN SEILLAENGEN
// ---------------------------------------------------------------------------------------

/// Einholstoss: wie stark springt das Tempo, wenn mit `SEILZUG` eingeholt wird?
/// Rueckgabe: (|v| max, groesster Sprung von |v| in EINEM Tick, |v| am Ende)
fn einholstoss(teilschritte: u32) -> (f32, f32, f32) {
    let mut app = welt_mit_substeps(0.0, Some(teilschritte));
    let l0 = 20.0f32;
    let a = anker(&mut app, Vec3::ZERO);
    let s = spieler(&mut app, Vec3::new(0.0, -l0, 0.0), Vec3::new(20.0, 0.0, 0.0));
    let j = seil(&mut app, a, s, l0);

    for _ in 0..30 {
        app.update();
    }
    let mut v_vor = tempo(&app, s).length();
    let mut v_max = v_vor;
    let mut sprung = 0.0f32;
    for _ in 0..160 {
        {
            let mut sj = app.world_mut().get_mut::<DistanceJoint>(j).expect("Seil fehlt");
            sj.limits.max = (sj.limits.max - SEILZUG / HZ as f32).max(SEIL_MIN);
        }
        app.update();
        let v = tempo(&app, s).length();
        v_max = v_max.max(v);
        sprung = sprung.max(v - v_vor);
        v_vor = v;
    }
    (v_max, sprung, v_vor)
}

fn f11_teilschritte_echte_laengen() {
    strich("F11 TEILSCHRITTE GEGEN DIE ECHTEN SEILLAENGEN — was kostet (a) wirklich?");

    println!(
        "\n  Die Stadt: Wohnhaeuser {} bis {HAUS_MAX} m (maps.ron raster.hoehe_min_m/-max_m),\n\
         \x20 Kirche 35 m, Mauer 120 m. Ein sauberer Bogen braucht Seil <= Ankerhoehe, also\n\
         \x20 sind 5 bis 20 m der Normalfall. Gemessen wird OHNE Schwerkraft — dann ist jeder\n\
         \x20 Tempoverlust Loeserdaempfung und nichts anderes. 600 Ticks = 10 s, der Verlust\n\
         \x20 je Sekunde ist die zehnte Wurzel aus v(10s)/v0.\n\
         \x20 Abnahmekriterium (a): unter 5 %/s.",
        4.5
    );

    for n in [6u32, 12, 24] {
        println!("\n  SubstepCount = {n}   (Verlust je Sekunde in %)");
        println!(
            "  {:>8} | {:>14} {:>14} {:>14} | {:>12}",
            "L [m]", "v0 = 20", "v0 = 35", "v0 = 50", "ms je Fahrt"
        );
        println!("  {}", "-".repeat(72));
        for laenge in [5.0f32, 8.0, 11.0, 15.0, 20.0] {
            let mut werte = [0.0f32; 3];
            let mut ms = 0.0f64;
            for (i, v0) in [20.0f32, 35.0, 50.0].iter().enumerate() {
                let uhr = Instant::now();
                let f = pendel_fahren_mit(0.0, laenge, *v0, 600, Some(n));
                ms += uhr.elapsed().as_secs_f64() * 1e3;
                werte[i] = (1.0 - (f.tempo_ende / f.tempo_start).max(0.0).powf(0.1)) * 100.0;
            }
            println!(
                "  {laenge:>8.1} | {:>13.4}% {:>13.4}% {:>13.4}% | {:>12.1}",
                werte[0],
                werte[1],
                werte[2],
                ms / 3.0
            );
        }
    }

    println!(
        "\n  EINHOLSTOSS — die Gegenprobe behauptet, mehr Teilschritte VERVIERFACHEN den Stoss.\n\
         \x20 Aufbau: Anker im Ursprung, Spieler 20 m darunter mit 20 m/s quer, ohne\n\
         \x20 Schwerkraft. 30 Ticks einschwingen, dann mit {SEILZUG} m/s auf {SEIL_MIN} m einholen."
    );
    println!(
        "\n  {:>12} | {:>14} {:>18} {:>14}",
        "Teilschritte", "|v| max [m/s]", "groesster Sprung", "|v| Ende [m/s]"
    );
    println!("  {}", "-".repeat(66));
    for n in [6u32, 12, 24] {
        let (vmax, sprung, vende) = einholstoss(n);
        println!("  {n:>12} | {vmax:>14.4} {sprung:>18.4} {vende:>14.4}");
    }
}

// ---------------------------------------------------------------------------------------
// F12 — REIBUNG BEIM SCHWINGEN
// ---------------------------------------------------------------------------------------

/// Kontrollierte Reibungsmessung: der Spieler gleitet mit `v0` an einer Wand entlang und
/// wird mit `andruck` (m/s^2) dagegen gedrueckt. Ohne Schwerkraft, damit NUR die Reibung
/// wirkt.
///
/// Rueckgabe: (v nach 0,25 s, v nach 0,5 s, v nach 1 s, Tick, an dem v unter 0,1 m/s faellt)
fn reibung_gleiten(mu: f32, andruck: f32, v0: f32) -> (f32, f32, f32, Option<usize>) {
    let mut app = welt(0.0);
    // Wand belegt x in [0, 0.5], Flaeche also bei x = 0.
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(WAND_MIN, 40.0, 400.0),
        Transform::from_xyz(WAND_MIN * 0.5, 0.0, 0.0),
        Friction::new(mu),
    ));
    let s = spieler(
        &mut app,
        Vec3::new(-SPIELER_RADIUS - 0.02, 0.0, -100.0),
        Vec3::new(0.0, 0.0, v0),
    );
    app.world_mut().entity_mut(s).insert((
        Held,
        Friction::new(mu),
        ConstantLinearAcceleration::new(andruck, 0.0, 0.0),
    ));

    let mut marken = [0.0f32; 3];
    let mut steht = None;
    for tick in 1..=60 {
        app.update();
        let v = tempo(&app, s);
        let laengs = v.z;
        match tick {
            15 => marken[0] = laengs,
            30 => marken[1] = laengs,
            60 => marken[2] = laengs,
            _ => {}
        }
        if steht.is_none() && laengs.abs() < 0.1 {
            steht = Some(tick);
        }
    }
    (marken[0], marken[1], marken[2], steht)
}

/// Die echte Gasse: Haus links, Anker auf seiner Dachkante (oder `anker_tiefe` m dahinter),
/// Spieler streift die Hauswand.
///
/// Rueckgabe: (|v| Start, |v| nach 1 s, kleinster Wandabstand, Ticks mit Wandkontakt)
fn gasse_streifen(mu: f32, anker_tiefe: f32, v0: f32, mit_wand: bool) -> (f32, f32, f32, usize) {
    let mut app = welt(SCHWERKRAFT);
    // Haus links: Flaeche bei x = -GASSE/2, 10 m tief, HAUS_MAX hoch, 200 m lang.
    let flaeche = -GASSE * 0.5;
    if mit_wand {
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(10.0, HAUS_MAX, 200.0),
            Transform::from_xyz(flaeche - 5.0, HAUS_MAX * 0.5, 0.0),
            Friction::new(mu),
        ));
    }
    // Boden
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(200.0, 1.0, 400.0),
        Transform::from_xyz(0.0, -0.5, 0.0),
        Friction::new(mu),
    ));

    // Anker und Spieler liegen BEIDE bei z = 0, die Startgeschwindigkeit zeigt nach +Z.
    // Damit steht das Seil im Start senkrecht auf der Bewegung — sonst frisst das straffe
    // Seil im ersten Tick die Radialkomponente, und gemessen waere der Ruck beim Einhaken
    // statt die Reibung. (Gegenprobe: die Zeile „keine Wand" muss dann nahe 0 liegen.)
    let anker_ort = Vec3::new(flaeche - anker_tiefe, HAUS_MAX, 0.0);
    let a = anker(&mut app, anker_ort);
    let start = Vec3::new(flaeche + SPIELER_RADIUS + 0.02, 2.8, 0.0);
    let s = spieler(&mut app, start, Vec3::new(0.0, 0.0, v0));
    app.world_mut()
        .entity_mut(s)
        .insert((Held, Friction::new(mu)));
    seil(&mut app, a, s, (start - anker_ort).length());

    // Ein Pendel tauscht Tempo gegen Hoehe. `|v|` allein misst deshalb KEINE Reibung —
    // gemessen wird die spezifische Energie 1/2 v^2 + |g| * h. Was davon fehlt, ist
    // Reibung plus Loeserdaempfung; die Zeile mit Reibung 0 ist die Nulllinie.
    let energie = |p: Vec3, v: Vec3| 0.5 * v.length_squared() - SCHWERKRAFT * p.y;
    let e0 = energie(start, Vec3::new(0.0, 0.0, v0));

    let mut e_ende = e0;
    let mut abstand_min = f32::MAX;
    let mut kontakt = 0usize;
    for _ in 0..60 {
        app.update();
        let o = ort(&app, s);
        let v = tempo(&app, s);
        let abstand = o.x - SPIELER_RADIUS - flaeche;
        abstand_min = abstand_min.min(abstand);
        if abstand < 0.02 {
            kontakt += 1;
        }
        e_ende = energie(o, v);
    }
    // Aequivalentes Tempo: das Tempo, das dieselbe spezifische Energie auf Starthoehe hat.
    let v_aequivalent = (2.0 * (e_ende + SCHWERKRAFT * start.y)).max(0.0).sqrt();
    (
        (1.0 - e_ende / e0) * 100.0,
        v_aequivalent,
        abstand_min,
        kontakt,
    )
}

fn f12_reibung() {
    strich("F12 REIBUNG BEIM SCHWINGEN — loescht eine gestreifte Wand das Momentum?");

    println!(
        "\n  avians Vorgabe ist `Friction {{ dynamic: 0.5, static: 0.5, combine: Average }}`\n\
         \x20 (src/dynamics/rigid_body/physics_material.rs:152-160), und Average von 0,5 und\n\
         \x20 0,5 ist 0,5 — NICHT 0,65. Die 0,65 der Gegenprobe entstehen nur mit einer\n\
         \x20 anderen Kombinationsregel (`Max`/`GeometricMean`, ebenda:13-24). Gemessen wird\n\
         \x20 deshalb mit ausdruecklich gesetzten, auf BEIDEN Koerpern gleichen Werten —\n\
         \x20 dann ist der kombinierte Wert genau der genannte."
    );

    println!(
        "\n  (a) KONTROLLIERT: Spieler gleitet mit 35 m/s an einer Wand entlang und wird mit\n\
         \x20     `Andruck` dagegen gedrueckt. Ohne Schwerkraft, 60 Ticks = 1 s.\n\
         \x20     Der Andruck ist nicht frei gewaehlt: die Seilspannung liefert v^2/L an\n\
         \x20     Zentripetalbeschleunigung, davon geht der Anteil sin(Winkel Seil/Wand) in\n\
         \x20     die Wand. 20 m/s^2 = so hart wie die Schwerkraft. 175 m/s^2 = v^2/L fuer\n\
         \x20     v = 35 und L = {GASSE} m (eine Gassenbreite), also der Fall „das Seil zieht\n\
         \x20     mich voll in die Wand“."
    );
    println!(
        "\n  {:>10} {:>12} | {:>12} {:>12} {:>12} | {:>16}",
        "Reibung", "Andruck", "v(0,25 s)", "v(0,5 s)", "v(1 s)", "steht ab Tick"
    );
    println!("  {}", "-".repeat(84));
    for andruck in [20.0f32, 60.0, 175.0] {
        for mu in [0.65f32, 0.3, 0.1, 0.0] {
            let (a, b, c, steht) = reibung_gleiten(mu, andruck, 35.0);
            println!(
                "  {mu:>10.2} {andruck:>12.1} | {a:>12.4} {b:>12.4} {c:>12.4} | {:>16}",
                steht.map(|t| t.to_string()).unwrap_or_else(|| "gleitet".into())
            );
        }
        println!("  {}", "-".repeat(84));
    }

    println!(
        "\n  (b) DIE ECHTE GASSE: Gasse {GASSE} m (maps.ron raster.gasse_m), Haus {HAUS_MAX} m hoch,\n\
         \x20     Anker auf der Dachkante bzw. `Tiefe` m dahinter (die andere Dachseite —\n\
         \x20     genau der Fall, in dem das Seil einen in die nahe Wand zieht).\n\
         \x20     Spieler startet an der Wand mit 35 m/s laengs, Schwerkraft {SCHWERKRAFT}. 1 s.\n\
         \x20     Ein Pendel tauscht Tempo gegen Hoehe — gemessen wird deshalb die\n\
         \x20     SPEZIFISCHE ENERGIE (1/2 v^2 + |g| h) und daraus das aequivalente Tempo\n\
         \x20     auf Starthoehe. Die Zeile mit Reibung 0,00 ist die Nulllinie: was dort\n\
         \x20     fehlt, ist Loeserdaempfung und NICHT Reibung."
    );
    println!(
        "\n  {:>10} {:>10} | {:>14} {:>16} | {:>14} {:>10}",
        "Reibung", "Tiefe [m]", "E-Verlust/s", "v-aequiv nach 1s", "Kontakt-Ticks", "Abstand"
    );
    println!("  {}", "-".repeat(86));
    for tiefe in [0.0f32, 2.0, 5.0] {
        // Kontrolle: dieselbe Fahrt OHNE Hauswand. Was hier fehlt, ist Loeserdaempfung
        // des Seils allein — der Rest der Spalte kommt vom Wandkontakt.
        let (ohne, v_ohne, _, _) = gasse_streifen(0.0, tiefe, 35.0, false);
        println!(
            "  {:>10} {tiefe:>10.1} | {ohne:>13.2}% {v_ohne:>16.4} | {:>14} {:>10}   <- Kontrolle, keine Wand",
            "-", "-", "-"
        );
        let mut null = 0.0f32;
        for mu in [0.0f32, 0.1, 0.3, 0.65] {
            let (verlust, v_aeq, abstand, kontakt) = gasse_streifen(mu, tiefe, 35.0, true);
            if mu == 0.0 {
                null = verlust;
            }
            println!(
                "  {mu:>10.2} {tiefe:>10.1} | {verlust:>13.2}% {v_aeq:>16.4} | {kontakt:>14} {abstand:>10.4}   {}",
                if mu == 0.0 {
                    format!("  Wand ohne Reibung: {:.2} %", verlust - ohne)
                } else {
                    format!("  davon Reibung: {:.2} %", verlust - null)
                }
            );
        }
        println!("  {}", "-".repeat(86));
    }
}

// =========================================================================================
// F13 bis F17 — MOVEANDSLIDE ALS SCHIEDSRICHTER ZWISCHEN SEIL UND GEOMETRIE
//
// `MoveAndSlide` ist in avian3d 0.7.0 ein **SystemParam**, kein Component und kein Plugin:
//   avian3d-0.7.0/src/character_controller/move_and_slide.rs:66-87
//     #[derive(SystemParam)]
//     pub struct MoveAndSlide<'w, 's> {
//         pub spatial_query: SpatialQuery<'w, 's>,
//         pub colliders: Query<'w,'s,(&Collider,&Position,&Rotation,Option<&CollisionLayers>),
//                              (With<ColliderOf>, Without<Sensor>)>,
//         pub length_unit: Res<'w, PhysicsLengthUnit>,
//     }
// Es kommt ueber `character_controller::prelude` in den Haupt-Prelude
// (mod.rs:6-12, lib.rs:564-565), und `pub mod character_controller` steht in lib.rs:512.
//
// Die Methode (move_and_slide.rs:485-495):
//     pub fn move_and_slide(&self, shape: &Collider, shape_position: Vector,
//         shape_rotation: RotationValue, mut velocity: Vector, delta_time: Duration,
//         config: &MoveAndSlideConfig, filter: &SpatialQueryFilter,
//         mut on_hit: impl FnMut(MoveAndSlideHitData) -> MoveAndSlideHitResponse,
//     ) -> MoveAndSlideOutput
// `RotationValue` ist in 3D `Quaternion` (physics_transform/transform.rs:141).
//
// **Es verlangt KEINEN RigidBody.** In der ganzen Datei kommt `RigidBody` nicht vor; das
// SystemParam liest ausschliesslich Collider, Position, Rotation und den Laengenmassstab.
// Es schreibt auch nichts: es GIBT `MoveAndSlideOutput { position, projected_velocity }`
// zurueck (:257-276), und der Aufrufer bleibt der Schreiber. avians eigenes Beispiel
// (examples/move_and_slide_3d.rs:60-70, :198-244) benutzt dafuer `RigidBody::Kinematic` +
// `CustomPositionIntegration` und schreibt `Transform` und `LinearVelocity` in `FixedUpdate`.
//
// Der Ablauf einer Fahrt (:509-645): Entpenetrieren vorweg, dann bis zu
// `move_and_slide_iterations` (Vorgabe 4, :173) Durchlaeufe aus Sweep, Anhalten am
// Treffer, Sammeln der Kontaktebenen und Projizieren der Restgeschwindigkeit, dann
// Entpenetrieren zum Schluss. `skin_width` (Vorgabe 0,01 m, :237) ist der Abstand, den der
// Sweep ueberall einhaelt (`pull_back`, :826-830) und den die Entpenetrierung zusaetzlich
// zur Eindringtiefe auffuellt (:928). Die Entpenetrierung selbst ist Gauss-Seidel ueber
// alle Kontaktebenen, bis zu 16 Durchlaeufe (:1019-1046, Vorgabe :233).
//
// Rueckfallebene (Frage 5): ja. `MoveAndSlide::cast_move` (:782-821) ist ein Sweep von Hand
// und ruft `SpatialQuery::cast_shape_predicate` mit `ShapeCastConfig`; `SpatialQuery` und
// `cast_shape` sind auch ohne `MoveAndSlide` da. `depenetrate` (:905-934),
// `intersections` (:1069-1119) und `project_velocity` (:1127-1129) sind ebenfalls einzeln
// benutzbar. Gebraucht wurde die Rueckfallebene nicht — MoveAndSlide laeuft.
// =========================================================================================

use avian3d::character_controller::move_and_slide::DepenetrationConfig;
use avian3d::math::Dir;

/// Wandflaeche des Hauses links in der Gasse (`maps.ron raster.gasse_m` = 7 m).
const HAUS_FLAECHE: f32 = -GASSE * 0.5;
/// Tiefe des Hauskoerpers hinter der Flaeche.
const HAUS_TIEFE: f32 = 10.0;
/// Starthoehe des Spielers in der Gasse (wie F12).
const START_Y: f32 = 2.8;
/// Ein Tick in Sekunden.
const DT: f32 = 1.0 / HZ as f32;
/// Abnahmekriterium S1/S2: so tief darf der Kapselrand hoechstens in die Wand.
const HAUT: f32 = -0.01;
/// Abnahmekriterium S3: so nah muss der Abstand an der Sollaenge liegen, damit der Tick
/// als „Seil straff“ zaehlt.
const STRAFF_MM: f32 = 1e-3;

/// Ein achsenparalleler Quader, um den Abstand zur Wand ohne avian nachzurechnen.
#[derive(Clone, Copy)]
struct Quader {
    min: Vec3,
    max: Vec3,
}

/// Vorzeichenbehafteter Abstand des KAPSELRANDS zur Quaderoberflaeche.
/// `> 0` heisst Luft dazwischen, `< 0` heisst so tief steckt die Kapsel drin.
///
/// Das ist exakt und nicht geschaetzt: die Kapselachse steht senkrecht
/// (`LockedAxes::ROTATION_LOCKED`), laeuft also nur in y. Damit haengen die x- und
/// z-Anteile des Abstands NICHT vom Achsenparameter ab, und der kleinste Abstand entsteht
/// bei dem y, das den y-Anteil minimiert — genau das leistet das doppelte Klemmen.
fn kapsel_abstand(q: &Quader, p: Vec3) -> f32 {
    let h = SPIELER_KAPSEL_LAENGE * 0.5;
    let y = p.y.clamp(q.min.y, q.max.y).clamp(p.y - h, p.y + h);
    let d = Vec3::new(p.x, y, p.z);
    let aussen = (q.min - d).max(d - q.max).max(Vec3::ZERO);
    if aussen.length_squared() > 0.0 {
        aussen.length() - SPIELER_RADIUS
    } else {
        -((d - q.min).min(q.max - d).min_element()) - SPIELER_RADIUS
    }
}

/// Wer den Spieler bewegt und wer zwischen Seil und Geometrie entscheidet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Schiri {
    /// (A) `DistanceJoint` am dynamischen Koerper, Kontakte loest avian. Kein Schiedsrichter.
    Joint,
    /// (A+MS) wie (A), danach MoveAndSlide als Nachkorrektur ueber die Tickstrecke.
    JointMs,
    /// (B) eigene Seilklemme am kinematischen Koerper, OHNE Schiedsrichter. Gegenprobe.
    KlemmeRoh,
    /// (B+MS) eigene Seilklemme + MoveAndSlide.
    KlemmeMs,
}

impl Schiri {
    fn name(self) -> &'static str {
        match self {
            Schiri::Joint => "(A)    Joint, kein Schiri",
            Schiri::JointMs => "(A+MS) Joint + MoveAndSlide",
            Schiri::KlemmeRoh => "(B)    Klemme, kein Schiri",
            Schiri::KlemmeMs => "(B+MS) Klemme + MoveAndSlide",
        }
    }
    fn kinematisch(self) -> bool {
        matches!(self, Schiri::KlemmeRoh | Schiri::KlemmeMs)
    }
    fn mit_ms(self) -> bool {
        matches!(self, Schiri::JointMs | Schiri::KlemmeMs)
    }
}

/// Das Seil als Component — **nicht** als Resource, weil es Spielerzustand ist
/// (docs/multiplayer.md). In F17 haengen 20 Spieler an 20 eigenen Seilen.
#[derive(Component, Clone, Copy)]
struct Seilzustand {
    anker: Vec3,
    laenge: f32,
}

/// Der Ort VOR dem Physikschritt. Nur (A+MS) braucht ihn: der Schiedsrichter muss wissen,
/// welche Strecke der Loeser in diesem Tick zurueckgelegt hat, um sie nachzufahren.
#[derive(Component, Default)]
struct VorOrt(Vec3);

/// Welteinstellung fuer die eigene Seilklemme — kein Spielerzustand.
#[derive(Resource, Clone, Copy)]
struct Klemmplan {
    zug_m_s: f32,
    min_m: f32,
    teilschritte: u32,
    mit_ms: bool,
    /// `true` = die alte Hybrid-Rechnung, die `|v|` beim Wegnehmen der Radialkomponente
    /// wieder hochskaliert. Das erhaelt den Schwung **per Konstruktion** und beweist damit
    /// nur das Verfahren, nicht die Physik. `false` = ehrliche Projektion.
    tempo_erhalten: bool,
}

/// Beleg, dass der Schiedsrichter ueberhaupt Arbeit verrichtet.
#[derive(Resource, Default)]
struct SchiriZaehler {
    aufrufe: usize,
    eingriffe: usize,
}

/// Merkt sich den Ort vor dem Physikschritt (laeuft in `FixedUpdate`, der `PhysicsSchedule`
/// haengt an `FixedPostUpdate` — avian3d-0.7.0/src/lib.rs:751-753).
fn merken_vor_ort(mut q: Query<(&Position, &mut VorOrt)>) {
    for (p, mut v) in &mut q {
        v.0 = p.0;
    }
}

/// (A+MS): MoveAndSlide als **Nachkorrektur** hinter dem Loeser.
///
/// Der Joint wirkt nur auf dynamische Koerper, also darf der Spieler nicht kinematisch sein
/// — und ein dynamischer Koerper wird von avian selbst integriert. MoveAndSlide kann hier
/// deshalb nur nachtraeglich pruefen: es faehrt die Strecke `p_nach - p_vor`, die der Loeser
/// in diesem Tick erzeugt hat, als Sweep noch einmal ab und schneidet ab, was durch
/// Geometrie geht.
///
/// `ParamSet` ist noetig, weil `MoveAndSlide` `&Position` liest und dieses System
/// `&mut Position` schreibt — bevy laesst beides nicht im selben System nebeneinander.
fn schiri_nach_joint(
    mut zugriff: ParamSet<(
        MoveAndSlide,
        Query<(Entity, &VorOrt, &Position, &Rotation, &LinearVelocity), With<Held>>,
        Query<(&mut Position, &mut LinearVelocity), With<Held>>,
    )>,
    mut kapsel: Local<Option<Collider>>,
    mut zaehler: ResMut<SchiriZaehler>,
) {
    if kapsel.is_none() {
        *kapsel = Some(Collider::capsule(SPIELER_RADIUS, SPIELER_KAPSEL_LAENGE));
    }
    let cfg = MoveAndSlideConfig::default();

    let mut auftraege: Vec<(Entity, Vec3, Vec3, Quat, Vec3)> = Vec::new();
    for (e, vor, p, r, v) in zugriff.p1().iter() {
        auftraege.push((e, vor.0, p.0, r.0, v.0));
    }

    let mut ergebnisse: Vec<(Entity, Vec3, Vec3)> = Vec::new();
    {
        let form = kapsel.as_ref().expect("Kapsel fehlt");
        let ms = zugriff.p0();
        for (e, p_vor, p_nach, rot, v) in &auftraege {
            let filter = SpatialQueryFilter::from_excluded_entities([*e]);
            let mut normalen: Vec<Dir> = Vec::new();
            let aus = ms.move_and_slide(
                form,
                *p_vor,
                *rot,
                (*p_nach - *p_vor) / DT,
                Duration::from_secs_f64(1.0 / HZ),
                &cfg,
                &filter,
                |treffer| {
                    normalen.push(*treffer.normal);
                    MoveAndSlideHitResponse::Accept
                },
            );
            if !normalen.is_empty() || (aus.position - *p_nach).length() > 1e-6 {
                let v_neu = if normalen.is_empty() {
                    *v
                } else {
                    MoveAndSlide::project_velocity(*v, &normalen)
                };
                ergebnisse.push((*e, aus.position, v_neu));
            }
        }
    }
    zaehler.aufrufe += auftraege.len();
    zaehler.eingriffe += ergebnisse.len();

    let mut schreiben = zugriff.p2();
    for (e, p, v) in ergebnisse {
        if let Ok((mut pos, mut vel)) = schreiben.get_mut(e) {
            pos.0 = p;
            vel.0 = v;
        }
    }
}

/// (B) und (B+MS): der Spieler ist **kinematisch** mit `CustomPositionIntegration`
/// (avian3d-0.7.0/src/dynamics/integrator/mod.rs:184-195, benutzt in :504). avian integriert
/// ihn dann nicht, und der Kontaktloeser bewegt ihn nicht (unendliche Masse). Alles, was ihn
/// bewegt, steht in diesem einen System — ein Feld, ein Schreiber.
///
/// Geschrieben wird `Transform`; `transform_to_position`
/// (avian3d-0.7.0/src/physics_transform/mod.rs:187-224) uebernimmt das in `Position`, solange
/// `Position` seit dem letzten Physikschritt nicht selbst geaendert wurde — genau der Weg,
/// den avians eigenes Beispiel geht (examples/move_and_slide_3d.rs:241).
fn eigene_seilklemme(
    schiri: MoveAndSlide,
    plan: Res<Klemmplan>,
    schwerkraft: Res<Gravity>,
    mut zaehler: ResMut<SchiriZaehler>,
    mut q: Query<
        (
            Entity,
            &Collider,
            &mut Transform,
            &mut LinearVelocity,
            &mut Seilzustand,
        ),
        With<Held>,
    >,
) {
    let n = plan.teilschritte.max(1);
    let h = DT / n as f32;
    let cfg = MoveAndSlideConfig::default();
    let depen: DepenetrationConfig = (&cfg).into();

    for (e, form, mut tf, mut vel, mut seil) in &mut q {
        let p0 = tf.translation;
        let mut p = p0;
        let mut v = vel.0;

        // --- Seil: Schwerkraft, Zwang und Einholen, auf die TEILSCHRITTE verteilt ---
        for _ in 0..n {
            v += schwerkraft.0 * h;
            if plan.zug_m_s > 0.0 {
                seil.laenge = (seil.laenge - plan.zug_m_s * h).max(plan.min_m);
            }
            let d = p - seil.anker;
            let r = d.length();
            if r >= seil.laenge && r > 1e-6 {
                let nrm = d / r;
                let radial = v.dot(nrm);
                if radial > 0.0 {
                    let vor = v.length();
                    v -= nrm * radial;
                    if plan.tempo_erhalten {
                        let nach = v.length();
                        if nach > 1e-6 {
                            v *= vor / nach;
                        }
                    }
                }
            }
            p += v * h;
            let d = p - seil.anker;
            let r = d.length();
            if r > seil.laenge && r > 1e-6 {
                p = seil.anker + d * (seil.laenge / r);
            }
        }

        // --- Schiedsrichter ---
        if plan.mit_ms {
            zaehler.aufrufe += 1;
            let filter = SpatialQueryFilter::from_excluded_entities([e]);
            let mut normalen: Vec<Dir> = Vec::new();
            let aus = schiri.move_and_slide(
                form,
                p0,
                tf.rotation,
                (p - p0) / DT,
                Duration::from_secs_f64(1.0 / HZ),
                &cfg,
                &filter,
                |treffer| {
                    normalen.push(*treffer.normal);
                    MoveAndSlideHitResponse::Accept
                },
            );
            if !normalen.is_empty() || (aus.position - p).length() > 1e-6 {
                zaehler.eingriffe += 1;
                p = aus.position;
                if !normalen.is_empty() {
                    v = MoveAndSlide::project_velocity(v, &normalen);
                }
                // Das Seil hat das letzte Wort ueber die Laenge — aber danach muss der
                // Schiedsrichter noch einmal entpenetrieren, sonst schiebt die Klemme den
                // Spieler in die Wand, die MoveAndSlide gerade freigeraeumt hat.
                let d = p - seil.anker;
                let r = d.length();
                if r > seil.laenge && r > 1e-6 {
                    p = seil.anker + d * (seil.laenge / r);
                    p += schiri.depenetrate(form, p, tf.rotation, &depen, &filter);
                }
            }
        }

        tf.translation = p;
        vel.0 = v;
    }
}

/// Setzt einen Spielerkoerper passend zur Variante.
fn spieler_fuer(
    app: &mut App,
    schiri: Schiri,
    ort: Vec3,
    tempo: Vec3,
    anker_ort: Vec3,
    l0: f32,
) -> Entity {
    let e = if schiri.kinematisch() {
        app.world_mut()
            .spawn((
                RigidBody::Kinematic,
                CustomPositionIntegration,
                Collider::capsule(SPIELER_RADIUS, SPIELER_KAPSEL_LAENGE),
                LockedAxes::ROTATION_LOCKED,
                SleepingDisabled,
                Transform::from_translation(ort),
                LinearVelocity(tempo),
            ))
            .id()
    } else {
        spieler(app, ort, tempo)
    };
    app.world_mut().entity_mut(e).insert((
        Held,
        VorOrt(ort),
        Seilzustand {
            anker: anker_ort,
            laenge: l0,
        },
    ));
    e
}

/// Haengt die Systeme der Variante in die App und setzt ihre Ressourcen.
fn schiri_einhaengen(
    app: &mut App,
    schiri: Schiri,
    zug: f32,
    zug_min: f32,
    teilschritte: u32,
    tempo_erhalten: bool,
) {
    app.insert_resource(SchiriZaehler::default());
    match schiri {
        Schiri::Joint => {}
        Schiri::JointMs => {
            app.add_systems(FixedUpdate, merken_vor_ort);
            app.add_systems(
                FixedPostUpdate,
                schiri_nach_joint
                    .after(PhysicsSystems::StepSimulation)
                    .before(PhysicsSystems::Writeback),
            );
        }
        Schiri::KlemmeRoh | Schiri::KlemmeMs => {
            app.insert_resource(Klemmplan {
                zug_m_s: zug,
                min_m: zug_min,
                teilschritte,
                mit_ms: schiri.mit_ms(),
                tempo_erhalten,
            });
            app.add_systems(FixedUpdate, eigene_seilklemme);
        }
    }
    // Einholen bei den Joint-Varianten: auf die Teilschritte verteilt — gemessen die
    // einzige Art, die den Katapult vermeidet (F10 (3)).
    if zug > 0.0 && !schiri.kinematisch() {
        app.insert_resource(Einholplan {
            rate_m_s: zug,
            min_m: zug_min,
            teilschritte,
        });
        app.add_systems(
            SubstepSchedule,
            seil_kuerzen_im_teilschritt
                .before(avian3d::dynamics::solver::schedule::SubstepSolverSystems::WarmStart)
                .ambiguous_with_all(),
        );
    }
}

// -----------------------------------------------------------------------------------------
// F13 — laeuft MoveAndSlide ueberhaupt, und mit welchem Koerper?
// -----------------------------------------------------------------------------------------

/// Treibt einen Koerper von Hand nach +X, so wie es ein eigener Bewegungscode taete.
/// Ohne das ruehrt sich ein Koerper mit `CustomPositionIntegration` ueberhaupt nicht — und
/// dann haelt jedes Seil trivial.
fn selbst_treiben(mut q: Query<&mut Transform, With<Held>>) {
    for mut tf in &mut q {
        tf.translation.x += 10.0 * DT;
    }
}

/// Haelt ein `DistanceJoint` einen Koerper — und haengt das am Koerpertyp?
/// Rueckgabe: (groesster Abstand zum Anker, |v| am Ende, zurueckgelegter Weg).
fn joint_haelt(kinematisch: bool, custom: bool, getrieben: bool, mit_seil: bool) -> (f32, f32, f32) {
    let mut app = welt(0.0);
    let a = anker(&mut app, Vec3::ZERO);
    let s = if kinematisch {
        app.world_mut()
            .spawn((
                RigidBody::Kinematic,
                Collider::capsule(SPIELER_RADIUS, SPIELER_KAPSEL_LAENGE),
                LockedAxes::ROTATION_LOCKED,
                SleepingDisabled,
                Transform::from_xyz(5.0, 0.0, 0.0),
                LinearVelocity(Vec3::new(10.0, 0.0, 0.0)),
            ))
            .id()
    } else {
        spieler(&mut app, Vec3::new(5.0, 0.0, 0.0), Vec3::new(10.0, 0.0, 0.0))
    };
    if custom {
        app.world_mut().entity_mut(s).insert(CustomPositionIntegration);
    }
    if getrieben {
        app.world_mut().entity_mut(s).insert(Held);
        app.add_systems(FixedUpdate, selbst_treiben);
    }
    if mit_seil {
        seil(&mut app, a, s, 5.0);
    } else {
        app.world_mut().entity_mut(a).despawn();
    }
    let start = ort(&app, s);
    let mut r_max = 5.0f32;
    for _ in 0..60 {
        app.update();
        r_max = r_max.max(ort(&app, s).length());
    }
    (
        r_max,
        tempo(&app, s).length(),
        (ort(&app, s) - start).length(),
    )
}

/// Kinematischer Koerper mit `CustomPositionIntegration` und ohne jedes eigene System:
/// bewegt avian ihn? Wirkt Schwerkraft auf ihn?
/// Rueckgabe: (Ort nach 60 Ticks, |v| nach 60 Ticks).
fn kinematik_kontrolle(mit_custom: bool) -> (Vec3, Vec3) {
    let mut app = welt(SCHWERKRAFT);
    let mut e = app.world_mut().spawn((
        RigidBody::Kinematic,
        Collider::capsule(SPIELER_RADIUS, SPIELER_KAPSEL_LAENGE),
        LockedAxes::ROTATION_LOCKED,
        SleepingDisabled,
        Transform::from_xyz(0.0, 10.0, 0.0),
        LinearVelocity(Vec3::new(3.0, 0.0, 0.0)),
    ));
    if mit_custom {
        e.insert(CustomPositionIntegration);
    }
    let s = e.id();
    for _ in 0..60 {
        app.update();
    }
    (ort(&app, s), tempo(&app, s))
}

/// MoveAndSlide ALLEIN gegen eine Wand: der Spieler fliegt mit `v0` waagerecht auf eine
/// 0,5-m-Wand zu, ohne Schwerkraft, ohne Seil (Anker 100 km entfernt, Seil 1000 km lang —
/// der Seilzwang kann also nichts tun).
/// Rueckgabe: (kleinster Abstand des Kapselrands zur Wand, x am Ende, Eingriffe).
fn nur_ms_gegen_wand(v0: f32, mit_ms: bool) -> (f32, f32, usize) {
    let mut app = welt(0.0);
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(WAND_MIN, 40.0, 40.0),
        Transform::from_xyz(WAND_X, 0.0, 0.0),
    ));
    let wand = Quader {
        min: Vec3::new(WAND_VORN, -20.0, -20.0),
        max: Vec3::new(WAND_HINTEN, 20.0, 20.0),
    };
    let art = if mit_ms {
        Schiri::KlemmeMs
    } else {
        Schiri::KlemmeRoh
    };
    let s = spieler_fuer(
        &mut app,
        art,
        Vec3::new(START_X, 0.0, 0.0),
        Vec3::new(v0, 0.0, 0.0),
        Vec3::new(0.0, 1.0e5, 0.0),
        1.0e6,
    );
    schiri_einhaengen(&mut app, art, 0.0, SEIL_MIN, 6, false);

    let mut abstand = f32::MAX;
    for _ in 0..200 {
        app.update();
        abstand = abstand.min(kapsel_abstand(&wand, ort(&app, s)));
    }
    (
        abstand,
        ort(&app, s).x,
        app.world().resource::<SchiriZaehler>().eingriffe,
    )
}

fn f13_moveandslide_grundlagen() {
    strich("F13 MOVEANDSLIDE — was ist es, welchen Koerper braucht es, laeuft es?");

    println!(
        "\n  Antworten am installierten Quelltext, nicht aus dem Gedaechtnis:\n\
         \x20 1. SystemParam. `#[derive(SystemParam)] pub struct MoveAndSlide<'w,'s>`\n\
         \x20    (character_controller/move_and_slide.rs:66-87). Kein Component, kein Plugin.\n\
         \x20 2. KEINEN. Das Wort `RigidBody` kommt in der Datei nicht vor; das SystemParam\n\
         \x20    liest `Collider`, `Position`, `Rotation`, `CollisionLayers` und\n\
         \x20    `PhysicsLengthUnit` (:69-87). Es funktioniert an jedem Koerper — und an\n\
         \x20    keinem, denn es braucht nur eine Form und einen Ort.\n\
         \x20 3. Aufruf `move_and_slide(shape, pos, rot, v, dt, cfg, filter, on_hit)`\n\
         \x20    (:485-495), Rueckgabe `MoveAndSlideOutput {{ position, projected_velocity }}`\n\
         \x20    (:257-276). Es schreibt NICHTS — der Aufrufer bleibt Schreiber. Schedule ist\n\
         \x20    frei; avians Beispiel setzt es in `FixedUpdate` (move_and_slide_3d.rs:29).\n\
         \x20 4. `skin_width` (Vorgabe 0,01 m, :237) ist der Abstand, den der Sweep ueberall\n\
         \x20    einhaelt (`pull_back` :826-830) und den die Entpenetrierung zusaetzlich zur\n\
         \x20    Eindringtiefe auffuellt (:928). Entpenetrierung: Gauss-Seidel, bis zu 16\n\
         \x20    Durchlaeufe (:1019-1046). Gleitdurchlaeufe: `move_and_slide_iterations`,\n\
         \x20    Vorgabe 4 (:173), je Durchlauf ein Sweep + eine Geschwindigkeitsprojektion.\n\
         \x20 5. Rueckfallebene ja: `cast_move` (:782-821) ist ein Sweep von Hand ueber\n\
         \x20    `SpatialQuery::cast_shape_predicate`; `depenetrate` (:905), `intersections`\n\
         \x20    (:1069) und `project_velocity` (:1127) sind einzeln benutzbar."
    );

    println!(
        "\n  (a) KONTROLLE: was tut avian mit einem kinematischen Koerper von allein?\n\
         \x20     Start (0, 10, 0) mit v = (3, 0, 0), Schwerkraft {SCHWERKRAFT}, 60 Ticks = 1 s."
    );
    println!(
        "\n  {:>34} | {:>28} | {:>24}",
        "Aufbau", "Ort nach 1 s", "v nach 1 s"
    );
    println!("  {}", "-".repeat(94));
    for mit in [false, true] {
        let (o, v) = kinematik_kontrolle(mit);
        println!(
            "  {:>34} | ({:>7.3},{:>7.3},{:>7.3}) | ({:>6.2},{:>6.2},{:>6.2})",
            if mit {
                "Kinematic + CustomPositionInteg."
            } else {
                "Kinematic (avian integriert)"
            },
            o.x,
            o.y,
            o.z,
            v.x,
            v.y,
            v.z
        );
    }

    println!(
        "\n  (b) DIE ARCHITEKTURFRAGE: haelt ein `DistanceJoint` einen KINEMATISCHEN Koerper?\n\
         \x20     Anker im Ursprung, Seil 5 m, Koerper startet bei x = 5 mit v = (10,0,0)\n\
         \x20     nach AUSSEN, ohne Schwerkraft, 60 Ticks. Haelt das Seil, bleibt der Abstand\n\
         \x20     bei 5 m; haelt es nicht, sind es 5 + 10 = 15 m."
    );
    println!(
        "\n      Jede Zeile hat eine GEGENPROBE ohne Seil. Bewegt sich der Koerper auch ohne\n\
         \x20     Seil nicht, beweist „Abstand bleibt 5 m“ gar nichts."
    );
    println!(
        "\n  {:>50} | {:>13} {:>12} | {:>13} {:>12} {:>12} | {:>24}",
        "Koerper", "ohne Seil: r", "Weg", "mit Seil: r", "Weg", "|v| Ende", "Urteil"
    );
    println!("  {}", "-".repeat(148));
    for (kin, cust, getrieben, name) in [
        (false, false, false, "Dynamic"),
        (true, false, false, "Kinematic"),
        (false, true, false, "Dynamic + CustomPositionIntegration"),
        (true, true, false, "Kinematic + CustomPositionIntegration"),
        (false, true, true, "Dynamic + CPI + selbst getrieben"),
        (true, true, true, "Kinematic + CPI + selbst getrieben"),
    ] {
        let (r0, _, weg0) = joint_haelt(kin, cust, getrieben, false);
        let (r1, v1, weg1) = joint_haelt(kin, cust, getrieben, true);
        // Ein Seil, das nichts zu tun hatte, hat nichts bewiesen.
        let urteil = if weg0 < 0.05 {
            "AUFBAU TAUGT NICHT"
        } else if r1 <= 5.01 {
            "JA, Seil haelt"
        } else {
            "NEIN, Seil wirkt nicht"
        };
        println!(
            "  {name:>50} | {r0:>13.4} {weg0:>12.4} | {r1:>13.5} {weg1:>12.4} {v1:>12.2} | {urteil:>24}"
        );
    }

    println!(
        "\n  (c) MOVEANDSLIDE ALLEIN gegen eine Wand: kinematischer Koerper mit\n\
         \x20     `CustomPositionIntegration`, Anker 100 km weit weg und Seil 1000 km lang —\n\
         \x20     der Seilzwang kann also NICHTS tun, gemessen wird nur der Schiedsrichter.\n\
         \x20     Wand {WAND_MIN} m dick bei x = 0, Start x = {START_X}, ohne Schwerkraft, 200 Ticks."
    );
    println!(
        "\n  {:>10} {:>14} | {:>20} {:>14} | {:>12}",
        "v0 [m/s]", "MoveAndSlide", "Abstand min [m]", "x Ende [m]", "Eingriffe"
    );
    println!("  {}", "-".repeat(80));
    for v0 in [20.0f32, 50.0, 75.0] {
        for mit in [false, true] {
            let (a, x, ein) = nur_ms_gegen_wand(v0, mit);
            println!(
                "  {v0:>10.0} {:>14} | {a:>20.6} {x:>14.4} | {ein:>12}",
                if mit { "AN" } else { "AUS" },
                );
        }
    }
}

// -----------------------------------------------------------------------------------------
// P1 / P2 — der Gassenaufbau: Anker auf der Dachkante, Spieler laengs an der Hauswand
// -----------------------------------------------------------------------------------------

struct Wandlauf {
    /// S1/S2: kleinster Abstand des Kapselrands zur HAUSWAND ueber alle Ticks.
    abstand_min: f32,
    /// Kontrolle: dasselbe fuer den Boden.
    boden_min: f32,
    ende: Vec3,
    v_max: f32,
    /// S3: Ticks, in denen der Abstand zum Anker innerhalb 1 mm an der Sollaenge liegt.
    straff: usize,
    /// Dasselbe, aber nur ueber die ersten 180 Ticks (3 s). Trennt „das Seil hat nie
    /// gearbeitet“ von „der Spieler ist am Ende auf dem Dach zur Ruhe gekommen“.
    straff_frueh: usize,
    /// Ticks, in denen der Kapselrand tiefer als die Kollisionshaut in der Wand steckte.
    verstoesse: usize,
    ticks: usize,
    eingriffe: usize,
    l_ende: f32,
    ms_tick: f64,
}

/// Die ersten `FRUEH` Ticks zaehlen fuer die fruehe Straffheit.
const FRUEH: usize = 180;

impl Wandlauf {
    fn straff_prozent(&self) -> f32 {
        self.straff as f32 / self.ticks as f32 * 100.0
    }
    fn straff_frueh_prozent(&self) -> f32 {
        self.straff_frueh as f32 / FRUEH.min(self.ticks) as f32 * 100.0
    }
}

#[derive(Clone, Copy)]
struct Gasseplan {
    schiri: Schiri,
    v0: f32,
    /// `> 0`: der Anker liegt so viele Meter HINTER der Wandflaeche (im Haus) und zieht den
    /// Spieler in die Wand. `< 0`: er ragt in die Gasse hinaus.
    versatz: f32,
    zug: f32,
    /// Bis zu welcher Laenge eingeholt wird. `SEIL_MIN` ist der RON-Wert.
    zug_min: f32,
    teilschritte: u32,
    ticks: usize,
    tempo_erhalten: bool,
}

impl Default for Gasseplan {
    fn default() -> Self {
        Self {
            schiri: Schiri::Joint,
            v0: 35.0,
            versatz: 0.0,
            zug: 0.0,
            zug_min: SEIL_MIN,
            teilschritte: 6,
            ticks: 900,
            tempo_erhalten: false,
        }
    }
}

fn gasse_fahren(g: Gasseplan) -> Wandlauf {
    let mut app = welt_mit_substeps(SCHWERKRAFT, Some(g.teilschritte));

    let haus = Quader {
        min: Vec3::new(HAUS_FLAECHE - HAUS_TIEFE, 0.0, -200.0),
        max: Vec3::new(HAUS_FLAECHE, HAUS_MAX, 200.0),
    };
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(HAUS_TIEFE, HAUS_MAX, 400.0),
        Transform::from_xyz(HAUS_FLAECHE - HAUS_TIEFE * 0.5, HAUS_MAX * 0.5, 0.0),
    ));
    let boden = Quader {
        min: Vec3::new(-100.0, -1.0, -200.0),
        max: Vec3::new(100.0, 0.0, 200.0),
    };
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(200.0, 1.0, 400.0),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));

    let start = Vec3::new(HAUS_FLAECHE + SPIELER_RADIUS + 0.02, START_Y, 0.0);
    let anker_ort = Vec3::new(HAUS_FLAECHE - g.versatz, HAUS_MAX, 0.0);
    let l0 = (start - anker_ort).length();

    let s = spieler_fuer(&mut app, g.schiri, start, Vec3::ZERO, anker_ort, l0);
    let joint = if g.schiri.kinematisch() {
        None
    } else {
        let a = anker(&mut app, anker_ort);
        Some(seil(&mut app, a, s, l0))
    };
    schiri_einhaengen(
        &mut app,
        g.schiri,
        g.zug,
        g.zug_min,
        g.teilschritte,
        g.tempo_erhalten,
    );

    // Ein Aufwaermtick OHNE Schwerkraft und OHNE Anfangstempo: er baut den Collider-Baum,
    // die Massen und die Kontaktpaare auf. Ohne ihn saehe MoveAndSlide im ersten Tick eine
    // leere Welt — und der Spieler startet 2 cm vor der Wand.
    app.insert_resource(Gravity(Vec3::ZERO));
    app.update();
    app.insert_resource(Gravity(Vec3::new(0.0, SCHWERKRAFT, 0.0)));
    {
        let w = app.world_mut();
        w.entity_mut(s).insert(Transform::from_translation(start));
        *w.get_mut::<Position>(s).expect("Position fehlt") = Position(start);
        *w.get_mut::<LinearVelocity>(s).expect("Tempo fehlt") =
            LinearVelocity(Vec3::new(0.0, 0.0, g.v0));
        w.get_mut::<Seilzustand>(s).expect("Seil fehlt").laenge = l0;
        w.resource_mut::<SchiriZaehler>().eingriffe = 0;
        w.resource_mut::<SchiriZaehler>().aufrufe = 0;
    }
    if let Some(j) = joint {
        app.world_mut()
            .get_mut::<DistanceJoint>(j)
            .expect("Seil fehlt")
            .limits
            .max = l0;
    }

    let mut w = Wandlauf {
        abstand_min: f32::MAX,
        boden_min: f32::MAX,
        ende: start,
        v_max: g.v0,
        straff: 0,
        straff_frueh: 0,
        verstoesse: 0,
        ticks: g.ticks,
        eingriffe: 0,
        l_ende: l0,
        ms_tick: 0.0,
    };
    let uhr = Instant::now();
    for tick in 1..=g.ticks {
        app.update();
        let p = ort(&app, s);
        let v = tempo(&app, s);
        let a_haus = kapsel_abstand(&haus, p);
        if a_haus < HAUT {
            w.verstoesse += 1;
        }
        w.abstand_min = w.abstand_min.min(a_haus);
        w.boden_min = w.boden_min.min(kapsel_abstand(&boden, p));
        w.v_max = w.v_max.max(v.length());
        w.ende = p;
        w.l_ende = match joint {
            Some(j) => {
                app.world()
                    .get::<DistanceJoint>(j)
                    .expect("Seil fehlt")
                    .limits
                    .max
            }
            None => app.world().get::<Seilzustand>(s).expect("Seil fehlt").laenge,
        };
        if ((p - anker_ort).length() - w.l_ende).abs() <= STRAFF_MM {
            w.straff += 1;
            if tick <= FRUEH {
                w.straff_frueh += 1;
            }
        }
    }
    w.ms_tick = uhr.elapsed().as_secs_f64() * 1e3 / g.ticks as f64;
    w.eingriffe = app.world().resource::<SchiriZaehler>().eingriffe;
    w
}

fn gassenkopf() {
    println!(
        "\n  {:>28} {:>6} {:>8} | {:>13} {:>9} | {:>26} | {:>9} | {:>7} {:>8} {:>9} | {:>8}",
        "Variante",
        "v0",
        "Versatz",
        "Abst.min [m]",
        "Verstoss",
        "Endort (x, y, z)",
        "|v| max",
        "straff",
        "straff 3s",
        "Eingriffe",
        "ms/Tick"
    );
    println!("  {}", "-".repeat(158));
}

fn gassenzeile(g: &Gasseplan, w: &Wandlauf) {
    println!(
        "  {:>28} {:>6.0} {:>8.2} | {:>13.5} {:>9} | ({:>7.2},{:>7.2},{:>7.2}) | {:>9.3} | {:>6.1}% {:>7.1}% {:>9} | {:>8.3}   {}{}",
        g.schiri.name(),
        g.v0,
        g.versatz,
        w.abstand_min,
        w.verstoesse,
        w.ende.x,
        w.ende.y,
        w.ende.z,
        w.v_max,
        w.straff_prozent(),
        w.straff_frueh_prozent(),
        w.eingriffe,
        w.ms_tick,
        if w.abstand_min >= HAUT { "S1/S2 ok" } else { "DURCH DIE WAND" },
        if w.straff_prozent() >= 30.0 { "" } else { " · S3 VERFEHLT" }
    );
}

fn f14_schwingen_gegen_wand() {
    strich("F14 P1 — SCHWINGEN gegen die Hauswand, KEIN Einholen (Bedingung S1 und S3)");

    println!(
        "\n  Aufbau (der, bei dem gemessen wurde, dass der Spieler 2-3 m IM Haus landet):\n\
         \x20 Haus links, Wandflaeche bei x = {HAUS_FLAECHE} (halbe Gasse aus maps.ron), {HAUS_TIEFE} m tief,\n\
         \x20 {HAUS_MAX} m hoch, 400 m lang. Boden bei y = 0. Anker auf der DACHKANTE\n\
         \x20 (x = {HAUS_FLAECHE} - Versatz, y = {HAUS_MAX}); Versatz > 0 heisst, er liegt im Haus und zieht\n\
         \x20 den Spieler in die Wand. Spieler startet 2 cm vor der Wand auf y = {START_Y} mit v0\n\
         \x20 laengs (+Z), Schwerkraft {SCHWERKRAFT}. 900 Ticks = 15 s.\n\
         \x20 `Abst.min` ist der kleinste Abstand des KAPSELRANDS zur Hausoberflaeche, exakt\n\
         \x20 nachgerechnet (nicht avians Zahl) — S1 verlangt >= {HAUT} m.\n\
         \x20 `straff` ist S3: Anteil der Ticks, in denen |Abstand zum Anker - Sollaenge|\n\
         \x20 <= 1 mm ist. Unter 30 % misst der Aufbau nichts.\n\
         \x20 `Eingriffe` zaehlt, wie oft MoveAndSlide den Ort wirklich geaendert hat."
    );

    gassenkopf();
    for versatz in [-0.25f32, 0.0, 0.35] {
        for v0 in [20.0f32, 35.0, 50.0] {
            for schiri in [
                Schiri::Joint,
                Schiri::JointMs,
                Schiri::KlemmeRoh,
                Schiri::KlemmeMs,
            ] {
                let g = Gasseplan {
                    schiri,
                    v0,
                    versatz,
                    ..default()
                };
                gassenzeile(&g, &gasse_fahren(g));
            }
            println!("  {}", "-".repeat(158));
        }
    }
}

fn f15_einholen_gegen_wand() {
    strich("F15 P2 — EINHOLEN mit seilzug_m_s = 28 gegen dieselbe Wand (Bedingung S2 und S3)");

    println!(
        "\n  Derselbe Aufbau wie F14, aber das Seil wird mit {SEILZUG} m/s auf {SEIL_MIN} m eingeholt.\n\
         \x20 Bei den Joint-Varianten geschieht das im `SubstepSchedule` — die Verkuerzung\n\
         \x20 wird auf die TEILSCHRITTE verteilt (F10 (3)), nicht je Tick. Bei der eigenen\n\
         \x20 Klemme laeuft dieselbe Verteilung in ihrer eigenen Teilschrittschleife.\n\
         \x20 900 Ticks; nach {:.1} s ist die Mindestlaenge erreicht und danach schwingt der\n\
         \x20 Spieler am kurzen Seil weiter.",
        (((HAUS_MAX - START_Y) - SEIL_MIN) / SEILZUG).max(0.0)
    );

    for zug_min in [SEIL_MIN, 6.0f32] {
        println!("\n  {}", "=".repeat(158));
        if zug_min == SEIL_MIN {
            println!("  EINGEHOLT BIS seil_min_m = {SEIL_MIN} m (der RON-Wert)");
        } else {
            println!(
                "  EINGEHOLT NUR BIS {zug_min} m — GEGENPROBE. Bei 3 m zieht das Seil den Spieler\n\
                 \x20 binnen 2 s ueber die Dachkante; dort liegt er auf dem Dach und das Seil ist\n\
                 \x20 dauerhaft SCHLAFF (straff faellt unter 10 %). Dann misst der Aufbau keinen\n\
                 \x20 Konflikt zwischen Seil und Wand mehr. Mit {zug_min} m bleibt er an der Wand."
            );
        }
        gassenkopf();
        for versatz in [-0.25f32, 0.0, 0.35] {
            for v0 in [20.0f32, 35.0, 50.0] {
                for schiri in [
                    Schiri::Joint,
                    Schiri::JointMs,
                    Schiri::KlemmeRoh,
                    Schiri::KlemmeMs,
                ] {
                    let g = Gasseplan {
                        schiri,
                        v0,
                        versatz,
                        zug: SEILZUG,
                        zug_min,
                        ..default()
                    };
                    gassenzeile(&g, &gasse_fahren(g));
                }
                println!("  {}", "-".repeat(158));
            }
        }
    }
}

// -----------------------------------------------------------------------------------------
// P3 — aendert der Schiedsrichter den Schwungverlust?
// -----------------------------------------------------------------------------------------

/// Freies Pendel OHNE Schwerkraft und ohne Wandberuehrung: jeder Tempoverlust ist
/// Loeserdaempfung. Der Boden liegt 100 m tiefer, damit der Collider-Baum nicht leer ist
/// und MoveAndSlide echte Arbeit hat (Abfragen laufen, Treffer gibt es keine).
///
/// Rueckgabe: (Verlust je Sekunde in %, straffe Ticks in %, Eingriffe, ms je Tick).
fn schwung_fahren(
    schiri: Schiri,
    laenge: f32,
    v0: f32,
    teilschritte: u32,
    tempo_erhalten: bool,
) -> (f32, f32, usize, f64) {
    let mut app = welt_mit_substeps(0.0, Some(teilschritte));
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(400.0, 1.0, 400.0),
        Transform::from_xyz(0.0, -100.0, 0.0),
    ));

    let anker_ort = Vec3::new(0.0, laenge, 0.0);
    let s = spieler_fuer(
        &mut app,
        schiri,
        Vec3::ZERO,
        Vec3::new(v0, 0.0, 0.0),
        anker_ort,
        laenge,
    );
    if !schiri.kinematisch() {
        let a = anker(&mut app, anker_ort);
        seil(&mut app, a, s, laenge);
    }
    schiri_einhaengen(&mut app, schiri, 0.0, SEIL_MIN, teilschritte, tempo_erhalten);

    let ticks = 600usize;
    let mut straff = 0usize;
    let uhr = Instant::now();
    for _ in 0..ticks {
        app.update();
        let p = ort(&app, s);
        if ((p - anker_ort).length() - laenge).abs() <= STRAFF_MM {
            straff += 1;
        }
    }
    let ms = uhr.elapsed().as_secs_f64() * 1e3 / ticks as f64;
    let v_ende = tempo(&app, s).length();
    (
        (1.0 - (v_ende / v0).max(0.0).powf(0.1)) * 100.0,
        straff as f32 / ticks as f32 * 100.0,
        app.world().resource::<SchiriZaehler>().eingriffe,
        ms,
    )
}

fn f16_schwungverlust() {
    strich("F16 P3 — aendert der Schiedsrichter den Schwungverlust? (Bedingung S4)");

    println!(
        "\n  Freies Pendel, OHNE Schwerkraft, ohne Wandberuehrung, 600 Ticks = 10 s. Jeder\n\
         \x20 Tempoverlust ist Loeserdaempfung. Verlust je Sekunde = 1 - (v(10s)/v0)^0,1.\n\
         \x20 S4 verlangt unter 5 %/s. Der Boden liegt 100 m tiefer, damit MoveAndSlide\n\
         \x20 wirklich abfragt — `Eingriffe` muss dabei 0 bleiben, sonst misst der Aufbau\n\
         \x20 nicht „ohne Wandberuehrung“.\n\
         \x20 `straff` ist wieder S3: liegt es bei 0, haelt das Seil gar nicht und die Zahl\n\
         \x20 daneben ist wertlos."
    );

    for teil in [6u32, 24] {
        println!("\n  {}", "=".repeat(104));
        println!("  TEILSCHRITTE = {teil}");
        for (schiri, erhalten, marke) in [
            (Schiri::Joint, false, ""),
            (Schiri::JointMs, false, ""),
            (Schiri::KlemmeRoh, true, "  |v| hochskaliert (TAUTOLOGISCH)"),
            (Schiri::KlemmeMs, true, "  |v| hochskaliert (TAUTOLOGISCH)"),
            (Schiri::KlemmeRoh, false, "  ehrliche Projektion"),
            (Schiri::KlemmeMs, false, "  ehrliche Projektion"),
        ] {
            println!("\n  {}{marke}", schiri.name());
            println!(
                "  {:>8} | {:>22} {:>22} {:>22} | {:>10} {:>9}",
                "L [m]",
                "v0=20 Verlust/s",
                "v0=35 Verlust/s",
                "v0=50 Verlust/s",
                "straff",
                "Eingriffe"
            );
            println!("  {}", "-".repeat(104));
            for laenge in [5.0f32, 8.0, 11.0] {
                let mut sp = [(0.0f32, 0.0f32, 0usize); 3];
                for (i, v0) in [20.0f32, 35.0, 50.0].iter().enumerate() {
                    let (verlust, straff, ein, _) =
                        schwung_fahren(schiri, laenge, *v0, teil, erhalten);
                    sp[i] = (verlust, straff, ein);
                }
                let schlimm = sp[0].0.max(sp[1].0).max(sp[2].0);
                println!(
                    "  {laenge:>8.1} | {:>21.4}% {:>21.4}% {:>21.4}% | {:>9.1}% {:>9}   {}",
                    sp[0].0,
                    sp[1].0,
                    sp[2].0,
                    sp[1].1,
                    sp[0].2 + sp[1].2 + sp[2].2,
                    if schlimm < 5.0 { "S4 ok" } else { "S4 VERFEHLT" }
                );
            }
        }
    }
}

// -----------------------------------------------------------------------------------------
// P4 — was kostet der Schiedsrichter in der Stadt?
// -----------------------------------------------------------------------------------------

/// 400 Haeuser im Raster + 1 Boden = 401 statische Koerper, dazu `n` Spieler, die JEDER an
/// einer eigenen Hauswand schwingen. Rueckgabe:
/// (ms je Tick, kleinster Wandabstand ueber alle Spieler, straffe Ticks in %).
fn stadt_kosten(schiri: Schiri, n: usize, teilschritte: u32, ticks: usize) -> (f64, f32, f32) {
    let mut app = welt_mit_substeps(SCHWERKRAFT, Some(teilschritte));

    let raster = HAUS_TIEFE + GASSE; // 17 m Mitte zu Mitte
    let mut haeuser: Vec<Quader> = Vec::new();
    for i in 0..20usize {
        for j in 0..20usize {
            let x = i as f32 * raster;
            let z = j as f32 * raster;
            app.world_mut().spawn((
                RigidBody::Static,
                Collider::cuboid(HAUS_TIEFE, HAUS_MAX, HAUS_TIEFE),
                Transform::from_xyz(x, HAUS_MAX * 0.5, z),
            ));
            if i == j {
                haeuser.push(Quader {
                    min: Vec3::new(x - 5.0, 0.0, z - 5.0),
                    max: Vec3::new(x + 5.0, HAUS_MAX, z + 5.0),
                });
            }
        }
    }
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(800.0, 1.0, 800.0),
        Transform::from_xyz(150.0, -0.5, 150.0),
    ));

    let mut leute: Vec<(Entity, Vec3, f32, Quader)> = Vec::new();
    for k in 0..n {
        let h = haeuser[k % haeuser.len()];
        let flaeche = h.max.x;
        let start = Vec3::new(flaeche + SPIELER_RADIUS + 0.02, START_Y, h.min.z + 5.0);
        let anker_ort = Vec3::new(flaeche, HAUS_MAX, h.min.z + 5.0);
        let l0 = (start - anker_ort).length();
        let s = spieler_fuer(&mut app, schiri, start, Vec3::new(0.0, 0.0, 35.0), anker_ort, l0);
        if !schiri.kinematisch() {
            let a = anker(&mut app, anker_ort);
            seil(&mut app, a, s, l0);
        }
        leute.push((s, anker_ort, l0, h));
    }
    schiri_einhaengen(&mut app, schiri, 0.0, SEIL_MIN, teilschritte, false);

    // Aufwaermtick wie in F14.
    app.insert_resource(Gravity(Vec3::ZERO));
    app.update();
    app.insert_resource(Gravity(Vec3::new(0.0, SCHWERKRAFT, 0.0)));

    let mut summe = 0.0f64;
    let mut abstand = f32::MAX;
    let mut straff = 0usize;
    for _ in 0..ticks {
        let uhr = Instant::now();
        app.update();
        summe += uhr.elapsed().as_secs_f64();
        for (s, ank, l0, h) in &leute {
            let p = ort(&app, *s);
            abstand = abstand.min(kapsel_abstand(h, p));
            let l = match app.world().get::<Seilzustand>(*s) {
                Some(z) => z.laenge,
                None => *l0,
            };
            if ((p - *ank).length() - l).abs() <= STRAFF_MM {
                straff += 1;
            }
        }
    }
    (
        summe * 1e3 / ticks as f64,
        abstand,
        straff as f32 / (ticks * n) as f32 * 100.0,
    )
}

fn f17_kosten() {
    strich("F17 P4 — was kostet der Schiedsrichter? (401 statische Koerper, RELEASE messen)");

    println!(
        "\n  Stadt: 20 x 20 Wohnhaeuser {HAUS_TIEFE} x {HAUS_MAX} x {HAUS_TIEFE} m im {}-m-Raster + 1 Boden\n\
         \x20 = 401 statische Koerper. Jeder Spieler haengt an der Dachkante SEINES Hauses und\n\
         \x20 schwingt mit 35 m/s laengs an der Wand — echter Wandkontakt, kein Leerlauf.\n\
         \x20 Gemessen wird nur `app.update()`, 300 Ticks; die Auswertung liegt ausserhalb der\n\
         \x20 Uhr. Budget je Tick bei 60 Hz: 16,7 ms, davon 4 ms fuer Physik.\n\
         \x20 Profil: {}",
        HAUS_TIEFE + GASSE,
        if cfg!(debug_assertions) {
            "DEBUG — diese Zahlen sind wertlos"
        } else {
            "RELEASE"
        }
    );

    println!(
        "\n  {:>28} {:>9} {:>14} | {:>12} {:>14} | {:>14} {:>9}",
        "Variante", "Spieler", "Teilschritte", "ms je Tick", "ms je Spieler", "Abstand min", "straff"
    );
    println!("  {}", "-".repeat(122));
    for n in [4usize, 20] {
        for teil in [6u32, 24] {
            for schiri in [
                Schiri::Joint,
                Schiri::JointMs,
                Schiri::KlemmeRoh,
                Schiri::KlemmeMs,
            ] {
                let (ms, abstand, straff) = stadt_kosten(schiri, n, teil, 300);
                println!(
                    "  {:>28} {n:>9} {teil:>14} | {ms:>12.4} {:>14.4} | {abstand:>14.5} {straff:>8.1}%",
                    schiri.name(),
                    ms / n as f64
                );
            }
            println!("  {}", "-".repeat(122));
        }
    }
}

// ---------------------------------------------------------------------------------------

fn main() {
    println!("PROBE avian3d 0.7.0 auf bevy 0.19.0");
    println!(
        "Simulation {HZ} Hz, Substeps = Vorgabe, Werte aus assets/data/game.ron.\n\
         Profil: {}",
        if cfg!(debug_assertions) {
            "DEBUG — die Zeiten in F4 sind wertlos, bitte mit --release fahren"
        } else {
            "RELEASE"
        }
    );

    f0_kopflos();
    f1_seil();
    f2_zwei_seile();
    f3_reel_in();
    f4_raycast();
    f5_kapsel();
    f6_teilschritte();
    f7_einpassung();
    f8_klemme_boost_wiederholbar();
    f9_seil_gegen_wand();
    f10_reparaturen();
    f11_teilschritte_echte_laengen();
    f12_reibung();
    f13_moveandslide_grundlagen();
    f14_schwingen_gegen_wand();
    f15_einholen_gegen_wand();
    f16_schwungverlust();
    f17_kosten();

    println!("\n{}", "=".repeat(88));
    println!("PROBE DURCHGELAUFEN");
    println!("{}", "=".repeat(88));
}
