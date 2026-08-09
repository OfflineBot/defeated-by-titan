//! Die Kamera dreht sich — **sonst zeigt jedes Bild etwas anderes als der Strahl misst.**
//!
//! Bis 2026-08-09 enthielt `src/` genau eine Rotation: die Sonne. Die Kamera haengt als Kind
//! am Spieler mit `Transform::from_xyz(0, augenhoehe_m, 0)`, also identitaetsgedreht, und
//! niemand schrieb `Intent.yaw/pitch` je in einen `Transform`. Sie blickte damit **immer**
//! nach −Z, waehrend der Zielstrahl nach `intent.blick()` geht. Sagte ein Skript
//! `look 30 -10`, zielte der Strahl woandershin als das Bild — und jedes Bildkriterium waere
//! wertlos gewesen, ohne dass es jemandem auffaellt.
//!
//! **Gedreht wird die KAMERA, nicht der Spieler.** Am Spieler haengt der Kollisionskasten;
//! dreht er mit, ist die achsenparallele Huelle keine achsenparallele Huelle mehr.
//!
//! Zeile in der Autoritaetstabelle: `Transform der Kamera | render`.
//!
//! **Keine Interpolation zwischen Simulationsschritten.** Sie braeuchte einen zweiten
//! Schreiber auf dem Spieler-`Transform` oder eine eigene Darstellungs-Entity — beides ist
//! ein eigener Entwurf und steht in `docs/ROADMAP.md`, nicht in diesem Auftrag.
//!
//! **Auch keine Glaettung.** `game.ron: kamera.glaetten_halbwertszeit_s` bleibt hier
//! ungelesen: geglaettet zeigt das Bild waehrend jeder Drehung eine **andere** Richtung als
//! `intent.blick()`, und genau diese Gleichheit ist das Abnahmekriterium (`tests/render.rs`).
//! Wer glaetten will, glaettet zuerst den Blickwinkel im `Intent` — dann bleiben Bild und
//! Strahl beieinander. Der Wert gehoert damit noch niemandem.
//!
//! **Beleg:** `tests/render.rs` · `docs/bilder/f002-blick.png` und
//! `docs/bilder/f002-blick-gedreht.png` aus `scripts/f002-blick.txt` bzw.
//! `scripts/f002-blick-gedreht.txt`.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Intent, LocalPlayer};

/// Legt `yaw` und `pitch` aus dem [`Intent`] des lokalen Spielers auf die Kamera.
///
/// Laeuft in `Update` und nicht im festen Schritt: Darstellung ist keine Simulation, und
/// eine Kamera, die nur 60-mal pro Sekunde nachzieht, fuehlt sich auf einem 144-Hz-Schirm
/// falsch an.
///
/// Die Drehung ist `Ry(yaw) * Rx(pitch)` und **kein Roll**. In dieser Reihenfolge gilt
/// `Transform::forward() == Intent::blick()` exakt — beides ist nachgerechnet und in
/// `tests/render.rs` festgenagelt:
///
/// - `bevy_transform-0.19.0/src/components/transform.rs:317-326` — `forward()` ist
///   `-(rotation * Vec3::Z)`, also `rotation * NEG_Z`.
/// - `glam-0.32.1/src/f32/sse2/quat.rs:170-181` — `from_rotation_x`/`from_rotation_y` sind
///   die gewoehnlichen rechtshaendigen Achsdrehungen.
/// - `Rx(pitch) * NEG_Z = (0, sin p, -cos p)`, darauf `Ry(yaw)` ergibt
///   `(-sin y · cos p, sin p, -cos y · cos p)` — Zeichen fuer Zeichen
///   `Intent::blick()` (`src/shared/intent.rs:42`).
pub fn kamera_drehen(
    daten: Res<GameData>,
    spieler: Query<&Intent, With<LocalPlayer>>,
    mut kamera: Query<&mut Transform, With<Camera3d>>,
) {
    // Es gibt keinen „den Spieler" — aber genau einen, der ICH ist (§6 Regel 3). Gibt es ihn
    // (noch) nicht, ist das kein Fehler: die Welt wird gerade erst aufgebaut.
    let Some(intent) = spieler.iter().next() else {
        return;
    };

    // Die Grenze ist eine Spielwert-Zahl und steht in `assets/data/game.ron`, nicht hier
    // (Regel 2). Geklemmt wird **auch hier**, nicht nur in `net::lokal`: dort wird nur der
    // Maus-Pfad geklemmt, und ein `Intent` kann spaeter auch aus dem Netz kommen. Eine
    // Kamera, die ueber den Scheitel kippt, steht auf dem Kopf.
    let grenze = daten.spiel.kamera.pitch_grenze_grad.to_radians();
    let pitch = intent.pitch.clamp(-grenze, grenze);

    // Ry(yaw) * Rx(pitch), in genau dieser Reihenfolge und ohne Roll. Nicht ueber
    // `Quat::from_euler`: dessen Achsenreihenfolge muesste man nachschlagen, diese hier
    // steht da.
    let drehung = Quat::from_rotation_y(intent.yaw) * Quat::from_rotation_x(pitch);

    // `With<Camera3d>` ohne weiteren Filter reicht, weil es **hoechstens eine** 3D-Kamera
    // gibt: `render::kamera_anhaengen` bricht ab, sobald eine existiert. Faellt diese
    // Zusicherung, faellt sie eine Datei weiter oben, nicht hier.
    for mut t in &mut kamera {
        // Nur schreiben, wenn sich wirklich etwas aendert — sonst meldet die
        // Aenderungserkennung jeden Frame eine Drehung, und die Transform-Weitergabe
        // arbeitet den Kamerazweig jedes Bild neu durch (Regel 6).
        if t.rotation != drehung {
            t.rotation = drehung;
        }
    }
}
