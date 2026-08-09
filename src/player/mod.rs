//! player — der Koerper: laufen, springen, Schwerkraft, Boden.
//!
//! Liest [`Intent`], **nie die Tastatur**. Wer den Intent gefuellt hat — Mensch, Skript oder
//! eines Tages das Netz — ist dieser Domaene egal, und genau das ist der Punkt
//! (`prompts/init.md` §6 Regel 2).
//!
//! Schreibt den `Transform` **nur**, solange der Bewegungszustand `AmBoden` oder
//! `InDerLuft` ist. Am Seil gehoert er `vector`: zwei Schreiber auf demselben Feld sind kein
//! Design, sondern ein Muenzwurf mit 60 Hz (`docs/architektur.md`, Autoritaetstabelle).
//!
//! **Stand:** ein Spieler, WASD, Schwerkraft, Boden bei y = 0. Kein Sprung-Feintuning, keine
//! echte Kollision — das haengt am raeumlichen Index in `world/` (Stufe 2).

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    Bewegungszustand, Gas, IdZaehler, Intent, Klingen, LocalPlayer, PlayerId, SpielerWarpen,
    Start, Tempo,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, lokalen_spieler_spawnen)
            .add_systems(FixedUpdate, (warpen_ausfuehren, bewegen).chain());
    }
}

/// Spawnt **einen** Spieler und markiert ihn als den lokalen.
///
/// Bewusst getrennt: `PlayerId` hat jeder, `LocalPlayer` genau einer. Ein zweiter Spieler
/// (Test, spaeter Netz) bekommt dieselben Components ohne den Marker —
/// `tests/mehrspieler.rs` tut genau das.
pub fn spieler_spawnen(
    commands: &mut Commands,
    zaehler: &mut IdZaehler,
    daten: &GameData,
    pos: Vec3,
    lokal: bool,
) -> Entity {
    let id = zaehler.naechster_spieler();
    let mut e = commands.spawn((
        Name::new(format!("spieler_{}", id.0)),
        id,
        Intent::default(),
        Tempo::default(),
        Bewegungszustand::default(),
        Gas::voll(daten.spiel.vector.gas_tank),
        Klingen::frisch(daten.gear.klingen.paare_start),
        Transform::from_translation(pos),
    ));
    if lokal {
        e.insert(LocalPlayer);
    }
    e.id()
}

fn lokalen_spieler_spawnen(
    mut commands: Commands,
    mut zaehler: ResMut<IdZaehler>,
    daten: Res<GameData>,
    start: Res<Start>,
) {
    let e = spieler_spawnen(&mut commands, &mut zaehler, &daten, Vec3::new(0.0, 2.0, 0.0), true);
    if start.sandbox {
        // `--sandbox`: leeres Feld, unendlich Gas — zum Anschauen (§12a).
        commands.entity(e).insert(Gas {
            unbegrenzt: true,
            ..Gas::voll(daten.spiel.vector.gas_tank)
        });
    }
}

fn warpen_ausfuehren(
    mut nachrichten: MessageReader<SpielerWarpen>,
    mut spieler: Query<(&PlayerId, &mut Transform, &mut Tempo)>,
) {
    for w in nachrichten.read() {
        for (id, mut transform, mut tempo) in &mut spieler {
            if *id == w.spieler {
                transform.translation = Vec3::new(w.pos_x, w.pos_y, w.pos_z);
                // Ohne das nimmt der Spieler seine alte Geschwindigkeit mit und faellt
                // am Zielort sofort weiter — ein `warp`, der nicht anhaelt, ist als
                // Fehlersuche-Werkzeug wertlos (§12c).
                tempo.0 = Vec3::ZERO;
            }
        }
    }
}

/// Laufen und Fallen. **Alles pro Sekunde**, nichts pro Frame (§11).
fn bewegen(
    zeit: Res<Time<Fixed>>,
    daten: Res<GameData>,
    mut spieler: Query<(&Intent, &mut Transform, &mut Tempo, &mut Bewegungszustand)>,
) {
    let dt = crate::shared::mathe::dt_gezaehmt(zeit.delta_secs());
    let s = &daten.spiel.spieler;
    let boden_y = 0.0;

    for (intent, mut transform, mut tempo, mut zustand) in &mut spieler {
        if *zustand == Bewegungszustand::AmSeil {
            // Am Seil gehoert der Transform `vector`. Diese Domaene fasst ihn nicht an.
            continue;
        }

        // Bewegung ist spielerlokal: erst in Weltkoordinaten drehen, dann anwenden.
        let (sin, cos) = intent.yaw.sin_cos();
        let vorwaerts = Vec3::new(-sin, 0.0, -cos);
        let rechts = Vec3::new(cos, 0.0, -sin);
        let wunsch = (vorwaerts * intent.bewegen_y + rechts * intent.bewegen_x)
            .clamp_length_max(1.0)
            * s.laufen_m_s;

        let am_boden = transform.translation.y <= boden_y + 1e-3;
        if am_boden {
            tempo.0.x = wunsch.x;
            tempo.0.z = wunsch.z;
            if intent.haelt(crate::shared::Tasten::SPRINGEN) {
                tempo.0.y = s.sprung_m_s;
                *zustand = Bewegungszustand::InDerLuft;
            } else {
                tempo.0.y = 0.0;
                *zustand = Bewegungszustand::AmBoden;
            }
        } else {
            tempo.0.y += daten.spiel.schwerkraft_m_s2 * dt;
            *zustand = Bewegungszustand::InDerLuft;
        }

        transform.translation += tempo.0 * dt;
        if transform.translation.y < boden_y {
            transform.translation.y = boden_y;
            tempo.0.y = 0.0;
        }
    }
}
