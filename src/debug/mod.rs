//! debug — der `--script`-Fahrer, `--bild`, das F3-Overlay und die NaN-Wache.
//!
//! **Die Werkzeuge kommen vor den Features** (`prompts/init.md` §12). Ohne sie ist alles
//! gebaut und nichts gesehen, weil jedes Feature hinter Maus und Tastatur liegt und niemand
//! am Keyboard sitzt.
//!
//! Der Fahrer schreibt in **echte** `ButtonInput`-Ressourcen; `net::lokal` liest sie
//! anschliessend genauso, wie es die Tastatur eines Menschen lesen wuerde. Die Reihenfolge
//! garantiert [`EingabeSet`](crate::shared::EingabeSet) — nicht der Zufall der
//! Systemreihenfolge.

pub mod bild;
pub mod gizmo;
pub mod skript;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::text::FontSize;

use crate::shared::{
    Bewegungszustand, BlickVorgabe, EingabeSet, Gas, LocalPlayer, Markierung, PlayerId,
    SpielerWarpen, Start, Tempo, Tick, TitanId, TitanSpawnen,
};
use skript::{Anweisung, Befehl, Groesse};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        let start = app.world().get_resource::<Start>().cloned().unwrap_or_default();

        app.init_resource::<Fahrt>()
            .add_systems(FixedPreUpdate, fahren.in_set(EingabeSet::Quelle))
            .add_systems(FixedPostUpdate, nan_wache)
            // Gizmos sind Darstellung, also `Update` und nicht der feste Schritt.
            .add_systems(Update, gizmo::gizmos_zeichnen);

        if let Some(pfad) = start.script.clone() {
            let inhalt = std::fs::read_to_string(&pfad).unwrap_or_else(|e| {
                // Beim Start laut abbrechen ist hier das richtige Verhalten: eine Fahrt,
                // die ihr Skript nicht findet, wuerde sonst gruen enden, ohne etwas
                // getan zu haben (§9).
                panic!("--script {}: laesst sich nicht lesen — {e}", pfad.display())
            });
            let plan = skript::lesen(&inhalt).unwrap_or_else(|fehler| {
                let liste: Vec<String> = fehler.iter().map(|f| f.to_string()).collect();
                panic!(
                    "--script {}: {} Zeile(n) nicht verstanden:\n  {}",
                    pfad.display(),
                    fehler.len(),
                    liste.join("\n  ")
                )
            });
            info!("Fahrt {}: {} Anweisungen", pfad.display(), plan.len());
            app.insert_resource(Fahrt { plan, ..default() });
        }

        // `--bild`: ohne das Flag haengt hier gar nichts ein (`debug::bild`).
        bild::einhaengen(app, &start);
    }
}

/// Der Stand einer Skriptfahrt.
#[derive(Resource, Debug, Default)]
pub struct Fahrt {
    pub plan: Vec<Anweisung>,
    /// Naechste Anweisung.
    pub bei: usize,
    /// Restzeit der laufenden `wait`/`key`-Anweisung in Sekunden.
    pub warten_s: f32,
    /// Tasten, die bis zum Ablauf gehalten werden.
    gehalten: Vec<(Gehalten, f32)>,
    pub gescheitert: Vec<String>,
    pub geprueft: u32,
    pub fertig: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gehalten {
    Taste(KeyCode),
    Maus(MouseButton),
}

impl Fahrt {
    pub fn laeuft(&self) -> bool {
        !self.plan.is_empty() && !self.fertig
    }
}

/// Bundelt, was der Fahrer anfassen darf. Ein System nimmt maximal ~16 Parameter, und
/// darueber schlaegt es als unlesbarer Trait-Fehler zu (`docs/lessons/bevy.md`).
#[derive(SystemParam)]
pub struct FahrerWelt<'w, 's> {
    tasten: ResMut<'w, ButtonInput<KeyCode>>,
    maus: ResMut<'w, ButtonInput<MouseButton>>,
    blick: ResMut<'w, BlickVorgabe>,
    spawnen: MessageWriter<'w, TitanSpawnen>,
    warpen: MessageWriter<'w, SpielerWarpen>,
    marken: MessageWriter<'w, Markierung>,
    beenden: MessageWriter<'w, AppExit>,
    spieler: Query<
        'w,
        's,
        (&'static PlayerId, &'static Transform, &'static Gas, &'static Tempo),
        With<LocalPlayer>,
    >,
    titanen: Query<'w, 's, &'static TitanId>,
}

/// Fuehrt die Fahrt aus — eine Anweisung pro Tick, ausser bei `wait`.
fn fahren(mut fahrt: ResMut<Fahrt>, tick: Res<Tick>, zeit: Res<Time<Fixed>>, mut welt: FahrerWelt) {
    if !fahrt.laeuft() {
        return;
    }
    let dt = zeit.delta_secs();

    // Gehaltene Tasten ablaufen lassen, bevor neue dazukommen.
    fahrt.gehalten.retain_mut(|(was, rest)| {
        *rest -= dt;
        if *rest > 0.0 {
            return true;
        }
        match was {
            Gehalten::Taste(k) => welt.tasten.release(*k),
            Gehalten::Maus(m) => welt.maus.release(*m),
        }
        false
    });

    if fahrt.warten_s > 0.0 {
        fahrt.warten_s -= dt;
        return;
    }

    while fahrt.bei < fahrt.plan.len() {
        let anweisung = fahrt.plan[fahrt.bei].clone();
        fahrt.bei += 1;
        match anweisung.was {
            Befehl::SpawnTitan { art, pos } => {
                welt.spawnen.write(TitanSpawnen {
                    art,
                    pos_x: pos.x,
                    pos_y: pos.y,
                    pos_z: pos.z,
                });
            }
            Befehl::Warp(pos) => {
                if let Some((id, _, _, _)) = welt.spieler.iter().next() {
                    welt.warpen.write(SpielerWarpen {
                        spieler: *id,
                        pos_x: pos.x,
                        pos_y: pos.y,
                        pos_z: pos.z,
                    });
                }
            }
            Befehl::Blick { yaw_grad, pitch_grad } => {
                welt.blick.0 = Some((yaw_grad.to_radians(), pitch_grad.to_radians()));
            }
            Befehl::Taste { code, dauer_s } => {
                welt.tasten.press(code);
                fahrt.gehalten.push((Gehalten::Taste(code), dauer_s));
            }
            Befehl::Haken { rechts, dauer_s } => {
                let m = if rechts { MouseButton::Right } else { MouseButton::Left };
                welt.maus.press(m);
                fahrt.gehalten.push((Gehalten::Maus(m), dauer_s));
            }
            Befehl::Warten(s) => {
                // Commands sind verzoegert: was dieser Tick gespawnt wird, existiert erst
                // am Ende des Ticks. Ohne `wait` fotografiert man ein leeres Feld (§3).
                fahrt.warten_s = s;
                return;
            }
            Befehl::Marke(text) => {
                info!("MARKE t={} {}", tick.0, text);
                welt.marken.write(Markierung { text, tick: tick.0 });
            }
            Befehl::Pruefe { groesse, vergleich, wert } => {
                let ist = messen(groesse, &welt, tick.0);
                fahrt.geprueft += 1;
                let haelt = ist.is_some_and(|i| vergleich.haelt(i, wert));
                if !haelt {
                    let meldung = format!(
                        "Zeile {}: assert {groesse:?} {} {wert} — gemessen {}",
                        anweisung.zeile,
                        vergleich.zeichen(),
                        ist.map_or("nichts (keinen Spieler gefunden)".to_string(), |i| format!("{i:.3}")),
                    );
                    error!("{meldung}");
                    fahrt.gescheitert.push(meldung);
                }
            }
            Befehl::Ende => {
                fahrt.bei = fahrt.plan.len();
            }
        }
    }

    if fahrt.bei >= fahrt.plan.len() && fahrt.gehalten.is_empty() {
        fahrt.fertig = true;
        let n = fahrt.gescheitert.len();
        if n == 0 {
            info!(
                "Fahrt beendet: {} assert gehalten, {} Ticks",
                fahrt.geprueft, tick.0
            );
            welt.beenden.write(AppExit::Success);
        } else {
            error!("Fahrt beendet: {n} von {} assert gescheitert", fahrt.geprueft);
            for m in &fahrt.gescheitert {
                error!("  {m}");
            }
            welt.beenden.write(AppExit::error());
        }
    }
}

/// Was ein `assert` messen kann. `None` heisst „nicht messbar" und **gilt als gescheitert** —
/// eine Pruefung, die nichts vorfand, ist keine bestandene Pruefung (§9).
fn messen(groesse: Groesse, welt: &FahrerWelt, tick: u64) -> Option<f32> {
    match groesse {
        Groesse::Titanen => Some(welt.titanen.iter().count() as f32),
        Groesse::Tick => Some(tick as f32),
        _ => {
            let (_, transform, gas, tempo) = welt.spieler.iter().next()?;
            Some(match groesse {
                Groesse::Hoehe => transform.translation.y,
                Groesse::Gas => gas.jetzt,
                Groesse::Tempo => tempo.betrag_m_s(),
                Groesse::Titanen | Groesse::Tick => unreachable!("oben behandelt"),
            })
        }
    }
}

/// Warnt **einmal**, wenn eine Position nicht endlich ist.
///
/// NaN im `Transform` ist der Bug, der aussieht wie „der Spieler ist verschwunden" — und
/// ohne diese Wache sucht man ihn drei Systeme zu spaet (`prompts/init.md` §9d).
fn nan_wache(
    positionen: Query<(Entity, &Transform), Or<(With<PlayerId>, With<TitanId>)>>,
    mut gewarnt: Local<bool>,
) {
    if *gewarnt {
        return;
    }
    for (e, t) in &positionen {
        if !crate::shared::mathe::ist_endlich(t.translation) {
            error!(
                "Position von {e:?} ist nicht endlich: {:?} — irgendwo wurde durch null \
                 geteilt oder ein Nullvektor normalisiert (docs/BUGS.md §9d)",
                t.translation
            );
            *gewarnt = true;
            return;
        }
    }
}

/// Das F3-Overlay: Position, Blick, Tempo, Gas, Zustand, Tick — **im Bild**.
///
/// Damit ist jede Meldung nachstellbar: der User schickt eine Koordinate, und man steht per
/// `warp` genau dort (§12c). Ohne Fenster gibt es kein Overlay — dann tut es das Log.
#[derive(Component)]
pub struct F3Zeile;

pub fn overlay_bauen(mut commands: Commands) {
    commands.spawn((
        F3Zeile,
        Text::new("F3"),
        TextFont { font_size: FontSize::Px(14.0), ..default() },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

pub fn overlay_fuellen(
    tasten: Res<ButtonInput<KeyCode>>,
    tick: Res<Tick>,
    spieler: Query<(&Transform, &Gas, &Bewegungszustand), With<LocalPlayer>>,
    mut zeilen: Query<(&mut Text, &mut Node), With<F3Zeile>>,
    mut sichtbar: Local<bool>,
) {
    if tasten.just_pressed(KeyCode::F3) {
        *sichtbar = !*sichtbar;
    }
    for (mut text, mut node) in &mut zeilen {
        node.display = if *sichtbar { Display::Flex } else { Display::None };
        if !*sichtbar {
            continue;
        }
        let inhalt = match spieler.iter().next() {
            Some((t, gas, zustand)) => format!(
                "t={}  pos {:.1} {:.1} {:.1}  gas {:.0}/{:.0}  {:?}",
                tick.0,
                t.translation.x,
                t.translation.y,
                t.translation.z,
                gas.jetzt,
                gas.maximal,
                zustand
            ),
            None => format!("t={}  (kein lokaler Spieler)", tick.0),
        };
        **text = inhalt;
    }
}
