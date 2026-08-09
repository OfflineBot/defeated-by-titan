//! Der Waechter ueber den Achsen-Vertrag der Kamera.
//!
//! **Bild und Zielstrahl muessen in dieselbe Richtung zeigen.** Bis 2026-08-09 taten sie das
//! nicht: `render::kamera::kamera_drehen` war ein leerer Rumpf, die Kamera blickte immer nach
//! −Z, und `intent.blick()` ging dorthin, wohin der Spieler zielt. Ein Fehler dieser Sorte
//! macht **jedes Bildkriterium des Projekts wertlos**, ohne dass er auffaellt: das Bild sieht
//! plausibel aus, es zeigt nur etwas anderes als das, was gemessen wird.
//!
//! Diese Datei nagelt die Gleichheit fest. Sie faellt um, wenn jemand ein Vorzeichen dreht,
//! die Drehreihenfolge tauscht, die Pitch-Klemmung entfernt — oder `kamera_drehen` wieder
//! leert.
//!
//! **Warum die Tests nur `Update` fahren und nicht `app.update()`:** `app.update()` laeuft
//! ueber `First`, wo `Time<Virtual>` aus der **Echtzeit** gefuellt wird. Je nach Laune der
//! Maschine kaeme dabei ein fester Schritt zustande, und `net::lokal::tastatur_lesen` wuerde
//! das gerade gesetzte `Intent` mit dem Blick der (nicht vorhandenen) Maus ueberschreiben.
//! Ein Test, dessen Ergebnis von der Tageslaune der Maschine abhaengt, misst die Maschine.

use bevy::prelude::*;
use defeated_by_titan::data::GameData;
use defeated_by_titan::shared::{Intent, LocalPlayer, Start};

/// Baut die **echte** App, headless — nicht eine zweite, aehnliche.
///
/// Zwei Durchlaeufe: `Commands` wirken erst am Ende ihres Laufs, der Spieler entsteht in
/// `Startup` und `render::kamera_anhaengen` haengt die Kamera erst danach an ihn.
fn app() -> App {
    let mut app = defeated_by_titan::app(Start { headless: true, ..default() });
    app.update();
    app.update();
    app
}

/// Setzt den Blickwunsch am lokalen Spieler (in **Grad**, wie in Skript und RON) und gibt
/// zurueck, wohin die Kamera danach schaut.
fn blicken(app: &mut App, yaw_grad: f32, pitch_grad: f32) -> Vec3 {
    let spieler = lokaler_spieler(app);
    {
        let mut intent = app
            .world_mut()
            .get_mut::<Intent>(spieler)
            .expect("der lokale Spieler hat ein Intent");
        intent.yaw = yaw_grad.to_radians();
        intent.pitch = pitch_grad.to_radians();
    }
    app.world_mut().run_schedule(Update);
    kamera_vorwaerts(app)
}

fn lokaler_spieler(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<LocalPlayer>>();
    q.iter(app.world())
        .next()
        .expect("es muss einen lokalen Spieler geben")
}

/// Der Vorwaertsvektor der Kamera.
///
/// Der `Transform` der Kamera ist **lokal** — sie haengt als Kind am Spieler. Dass das
/// zugleich der Vektor in der Welt ist, haengt daran, dass der Spieler sich nie dreht; genau
/// das prueft [`f002_gedreht_wird_die_kamera_und_nicht_der_spieler`]. Auf `GlobalTransform`
/// auszuweichen ginge nicht, ohne `PostUpdate` mitzufahren — und damit die halbe
/// Render-Vorbereitung.
fn kamera_vorwaerts(app: &mut App) -> Vec3 {
    let mut q = app.world_mut().query_filtered::<&Transform, With<Camera3d>>();
    let t = q
        .iter(app.world())
        .next()
        .expect("es muss eine 3D-Kamera geben");
    // `Dir3::as_vec3` — bevy_math-0.19.0/src/direction.rs:614.
    t.forward().as_vec3()
}

fn pitch_grenze_grad(app: &App) -> f32 {
    // Die Zahl steht in assets/data/game.ron, nicht im Test (Regel 2). Ein Test, der sie
    // abschreibt, ist am Tag der ersten Aenderung eine Luege.
    app.world().resource::<GameData>().spiel.kamera.pitch_grenze_grad
}

#[test]
fn f002_blick_null_zeigt_die_kamera_nach_minus_z() {
    // Der Achsen-Vertrag aus docs/konventionen.md: `yaw = 0, pitch = 0` ist −Z. Faellt er,
    // steht jedes Modell falsch herum und niemand weiss, warum.
    let mut app = app();
    let vorwaerts = blicken(&mut app, 0.0, 0.0);
    assert!(
        (vorwaerts - Vec3::NEG_Z).length() < 1e-5,
        "yaw = 0, pitch = 0 muss −Z sein, war aber {vorwaerts:?}"
    );
}

#[test]
fn f002_bild_und_strahl_zeigen_in_dieselbe_richtung() {
    // **Das eigentliche Kriterium.** Der Zielstrahl geht nach `intent.blick()`, das Bild
    // dorthin, wohin die Kamera schaut. Sind das zwei verschiedene Richtungen, misst jedes
    // Bild etwas anderes als der Strahl — und man sieht es dem Bild nicht an.
    let mut app = app();

    // Negative Winkel und Werte jenseits von 90 Grad sind ausdruecklich dabei: ein
    // vertauschtes Vorzeichen oder ein `abs()` faellt genau dort auf und sonst nirgends.
    let paare = [
        (0.0_f32, 0.0_f32),
        (30.0, -10.0),
        (-45.0, 20.0),
        (135.0, -60.0),
        (200.0, 45.0),
        (-170.0, 89.0),
        (89.9, -89.0),
    ];

    for (yaw_grad, pitch_grad) in paare {
        let vorwaerts = blicken(&mut app, yaw_grad, pitch_grad);
        let strahl = Intent {
            yaw: yaw_grad.to_radians(),
            pitch: pitch_grad.to_radians(),
            ..default()
        }
        .blick();
        assert!(
            (vorwaerts - strahl).length() < 1e-5,
            "look {yaw_grad} {pitch_grad}: die Kamera zeigt nach {vorwaerts:?}, \
             der Zielstrahl nach {strahl:?} — Bild und Messung laufen auseinander"
        );
    }
}

#[test]
fn f002_pitch_bleibt_in_der_grenze_aus_game_ron() {
    // Ohne Klemmung kippt die Kamera ueber den Scheitel und das Bild steht auf dem Kopf.
    // `net::lokal` klemmt nur den MAUS-Pfad; ein Intent aus Skript oder Netz kommt
    // ungeklemmt an, und deshalb klemmt die Kamera selbst.
    let mut app = app();
    let grenze = pitch_grenze_grad(&app);

    for (gewuenscht, erwartet) in [
        (120.0_f32, grenze),
        (-120.0, -grenze),
        (grenze + 1.0, grenze),
        (45.0, 45.0),
        (-45.0, -45.0),
    ] {
        let vorwaerts = blicken(&mut app, 0.0, gewuenscht);
        // `blick().y` ist `sin(pitch)` — der Pitch laesst sich aus der Richtung zurueckrechnen.
        let ist = vorwaerts.y.asin().to_degrees();
        assert!(
            (ist - erwartet).abs() < 1e-3,
            "look 0 {gewuenscht} haette {erwartet} Grad ergeben muessen, ergab aber {ist} — \
             die Grenze aus assets/data/game.ron (kamera.pitch_grenze_grad = {grenze}) \
             wird nicht eingehalten"
        );
    }
}

#[test]
fn f002_gedreht_wird_die_kamera_und_nicht_der_spieler() {
    // Am Spieler haengt der Kollisionskasten. Dreht der mit, ist die achsenparallele Huelle
    // keine achsenparallele mehr — und die Kollision wird auf eine Weise falsch, die man
    // erst bemerkt, wenn jemand schraeg an einer Wand haengen bleibt.
    let mut app = app();
    for (yaw, pitch) in [(0.0_f32, 0.0_f32), (137.0, -42.0), (-91.0, 63.0)] {
        blicken(&mut app, yaw, pitch);
        let spieler = lokaler_spieler(&mut app);
        let drehung = app
            .world()
            .get::<Transform>(spieler)
            .expect("der Spieler hat einen Transform")
            .rotation;
        assert!(
            drehung.angle_between(Quat::IDENTITY) < 1e-6,
            "nach look {yaw} {pitch} ist der Spieler um {} Grad gedreht — gedreht wird die \
             KAMERA, nicht der Spieler (src/render/kamera.rs)",
            drehung.angle_between(Quat::IDENTITY).to_degrees()
        );
    }
}

#[test]
fn f002_drehen_verschiebt_die_augenhoehe_nicht() {
    // `kamera_drehen` schreibt genau ein Feld. Wer versehentlich den ganzen `Transform`
    // ersetzt, setzt die Kamera zwischen die Fuesse — und das Bild sieht nur „etwas tief" aus.
    let mut app = app();
    let augenhoehe = app.world().resource::<GameData>().spiel.spieler.augenhoehe_m;

    blicken(&mut app, 77.0, -33.0);

    let mut q = app.world_mut().query_filtered::<&Transform, With<Camera3d>>();
    let ort = q
        .iter(app.world())
        .next()
        .expect("es muss eine 3D-Kamera geben")
        .translation;
    assert!(
        (ort - Vec3::new(0.0, augenhoehe, 0.0)).length() < 1e-6,
        "die Kamera sitzt bei {ort:?} statt auf {augenhoehe} m Augenhoehe ueber dem Spieler"
    );
}
