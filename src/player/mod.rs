//! player — der Koerper: laufen, springen, Schwerkraft, Boden.
//!
//! Liest [`Intent`], **nie die Tastatur**. Wer den Intent gefuellt hat — Mensch, Skript oder
//! eines Tages das Netz — ist dieser Domaene egal, und genau das ist der Punkt
//! (`prompts/init.md` §6 Regel 2).
//!
//! **Der `Transform` des Spielers hat genau einen Schreiber**, und der wird
//! [`koerper::schritt`] heissen. Die alte Teilung „`player` am Boden, `vector` am Seil,
//! getrennt ueber `Bewegungszustand`" haelt nicht: ein Gas-Boost wirkt in der Luft **und**
//! am Seil gleichzeitig, es gibt also keinen Zustand, der die beiden Schreiber trennt
//! (`docs/architektur.md`, Autoritaetstabelle).
//!
//! **Stand:** die Naht steht. [`koerper::schritt`] ist als Stub in `SchrittSet::Vollzug`
//! registriert und tut nichts; die heutige Bewegung laeuft weiter in [`bewegen`] — WASD,
//! Schwerkraft, Bodenebene bei y = 0, keine echte Kollision. **[`bewegen`] und das harte
//! `boden_y = 0.0` sterben in dem Commit, der `koerper::schritt` fuellt**, zusammen mit
//! `shared::Boden`. Vorher nicht: sonst faellt der Spieler 600 Ticks lang und
//! `scripts/t007-erste-fahrt.txt` mit ihm.

pub mod koerper;
pub mod lauf;

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    AntriebEinholen, AntriebLauf, AntriebSchub, Bewegungszustand, Gas, Gasfreigabe, Haken,
    IdZaehler, Intent, Klingen, LocalPlayer, PlayerId, SchrittSet, Seillaenge, SpielerWarpen,
    Start, Tempo, VorigeTasten, Zielpunkt,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, lokalen_spieler_spawnen)
            .add_systems(FixedUpdate, lauf::lauf.in_set(SchrittSet::Antrieb))
            .add_systems(
                FixedUpdate,
                (warpen_ausfuehren, bewegen, koerper::schritt)
                    .chain()
                    .in_set(SchrittSet::Vollzug),
            );
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
    // Verschachtelt, weil ein Tupel in `spawn` nur begrenzt viele Elemente nimmt und
    // darueber als unlesbarer Trait-Fehler zuschlaegt (`docs/lessons/bevy.md`).
    let mut e = commands.spawn((
        Name::new(format!("spieler_{}", id.0)),
        id,
        Intent::default(),
        Tempo::default(),
        Bewegungszustand::default(),
        Gas::voll(daten.spiel.vector.gas_tank),
        Klingen::frisch(daten.gear.klingen.paare_start),
        Transform::from_translation(pos),
        // Das Vector Gear haengt am Spieler, nicht an der Welt: jeder Spieler hat sein
        // eigenes (`docs/multiplayer.md` Regel 3). Alle acht sind ab Tick 1 vorhanden, damit
        // kein System auf ein fehlendes Component filtert und den Spieler still auslaesst.
        (
            Haken::default(),
            Seillaenge::default(),
            Zielpunkt::default(),
            Gasfreigabe::default(),
            AntriebLauf::default(),
            AntriebSchub::default(),
            AntriebEinholen::default(),
            VorigeTasten::default(),
        ),
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
