//! render — Kamera, Licht, Meshes bauen.
//!
//! **Liest nur.** Rendering ist Darstellung, nicht Simulation — ein System, das aus einem
//! Mausklick direkt ein Mesh spawnt, ist der Anfang vom Ende, denn genau dieser Klick muss
//! spaeter vom Server bestaetigt werden (`prompts/init.md` §6 Regel 1).
//!
//! Der Stil ist vorgegeben: Low Poly, weiche Normalen, flache Farbflaechen, aggressiver
//! Fernnebel (er arbeitet doppelt: Atmosphaere und Culling). Die drei Signalfarben
//! ausschliesslich fuer Gameplay (`docs/konventionen.md`).
//!
//! ⚠️ Nichts davon ist je **gesehen** worden — auf Maschine A gibt es kein Fenster
//! (`docs/umgebung.md`). Alles hier bleibt 🟨, bis jemand auf Maschine B draufschaut.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Bauklotz, LocalPlayer};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, licht_aufbauen)
            .add_systems(Update, (kamera_anhaengen, kloetze_bauen));
    }
}

/// Sonne und Grundhelligkeit.
///
/// ⚠️ Schatten sind **aus**. Sie sind der teuerste Schalter im Spiel — erst am Ende, und dann
/// mit einer Zahl daneben (`docs/lessons/performance.md`).
fn licht_aufbauen(mut commands: Commands) {
    commands.spawn((
        Name::new("sonne"),
        DirectionalLight {
            illuminance: 10_000.0,
            // Schatten sind der teuerste Schalter im Spiel — erst am Ende, mit Zahl.
            // `shadow_maps_enabled` ist der Schalter, der wirklich etwas kostet.
            // (`contact_shadows_enabled` steht hier bewusst NICHT: es wirkt allein gar
            // nicht — Kontaktschatten brauchen zusaetzlich eine `ContactShadows`-Komponente
            // an der Kamera. Ein Feld, das man fuer einen Schalter haelt, obwohl es keiner
            // ist, ist schlimmer als kein Feld.)
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(50.0, 100.0, 60.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Haengt die Kamera an den lokalen Spieler.
///
/// **Das ist die einzige Stelle, an der „ich" eine Kamera bekommt.** Jeder andere Spieler ist
/// einer von vielen und hat keine (§6 Regel 3).
///
/// `AmbientLight` haengt in Bevy 0.19 an der **Kamera**, nicht an der Welt — es ist ein
/// Component mit `#[require(Camera)]` und kein `Resource` mehr (`docs/lessons/bevy.md`).
fn kamera_anhaengen(
    mut commands: Commands,
    daten: Res<GameData>,
    neu: Query<Entity, (With<LocalPlayer>, Without<Children>)>,
    schon_da: Query<(), With<Camera3d>>,
) {
    if !schon_da.is_empty() {
        return;
    }
    let Some(spieler) = neu.iter().next() else {
        return;
    };
    let k = &daten.spiel.kamera;
    let kamera = commands
        .spawn((
            Name::new("kamera"),
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: k.sicht_grad.to_radians(),
                ..default()
            }),
            AmbientLight { brightness: 220.0, ..default() },
            // Augenhoehe ueber dem Ursprung des Spielers — der liegt zwischen den Fuessen
            // (docs/konventionen.md).
            Transform::from_xyz(0.0, daten.spiel.spieler.augenhoehe_m, 0.0),
        ))
        .id();
    commands.entity(spieler).add_child(kamera);
}

/// Macht aus [`Bauklotz`]-Daten Dreiecke — **einmal je Entity**.
///
/// `render` kennt `world` dafuer nicht: es fragt nach einem Component, nicht nach einer
/// Funktion (`docs/architektur.md`).
fn kloetze_bauen(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materialien: ResMut<Assets<StandardMaterial>>,
    ohne_mesh: Query<(Entity, &Bauklotz), Without<Mesh3d>>,
) {
    for (e, klotz) in &ohne_mesh {
        let mesh = meshes.add(Cuboid::new(klotz.groesse.x, klotz.groesse.y, klotz.groesse.z));
        let material = materialien.add(StandardMaterial {
            base_color: Color::linear_rgb(klotz.farbe[0], klotz.farbe[1], klotz.farbe[2]),
            // Fehlender metallicFactor bedeutet 1.0, also voll metallisch — ein
            // Diffuse-Material ohne den Wert sieht im Spiel wie Chrom aus
            // (docs/modelle.md, glTF-Falle 2). Hier gilt dasselbe.
            metallic: 0.0,
            perceptual_roughness: 0.95,
            ..default()
        });
        commands.entity(e).insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }
}
