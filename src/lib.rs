//! **Defeated by Titan** — ein 3D-Titanenkampfspiel in Bevy.
//!
//! Eine Domaene = ein Ordner = ein Plugin = standalone. Was eine Domaene darf und was nicht,
//! steht in `docs/architektur.md`; `tests/domaenen.rs` faellt um, wenn sich jemand nicht
//! daran haelt.
//!
//! ## Warum die Plugin-Liste hier steht und nicht in `main.rs`
//!
//! `prompts/init.md` §5 sieht sie in `main.rs`. Sie steht hier, weil `tests/mehrspieler.rs`
//! und `tests/domaenen.rs` **dieselbe** App bauen muessen, die auch gespielt wird — sonst
//! pruefen sie eine zweite, aehnliche App und beweisen nichts ueber die echte. `main.rs`
//! bleibt, was §5 will: Flags lesen, App bauen lassen, starten. **Benannte Abweichung**, weil
//! sie dem Zweck der Regel dient (eine Naht, ein Schreiber) statt ihm zu widersprechen.

pub mod blades;
pub mod combat;
pub mod data;
pub mod debug;
pub mod hud;
pub mod menu;
pub mod mission;
pub mod net;
pub mod player;
pub mod progress;
pub mod render;
pub mod save;
pub mod shared;
pub mod sound;
pub mod squad;
pub mod titan;
pub mod vector;
pub mod world;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::window::{ExitCondition, PresentMode};

use shared::{
    Aufprall, HakenGeloest, HakenGesetzt, IdZaehler, KoerperWeg, Markierung, SchrittSet,
    SpielerWarpen, Start, Tick, TitanGetroffen, TitanSpawnen, Wuerfel,
};

/// Der Fenstertitel. Steht an **genau einer** Stelle — `docs/konventionen.md` nennt die drei
/// Schreibweisen des Projektnamens und wo jede lebt.
pub const FENSTERTITEL: &str = "Defeated by Titan";

/// Baut die App, die gespielt **und** getestet wird.
pub fn app(start: Start) -> App {
    let mut app = App::new();

    if !start.unbekannt.is_empty() {
        // Laut, nicht still: ein vertipptes Flag, das ignoriert wird, kostet eine Stunde
        // Fehlersuche am falschen Ende.
        eprintln!(
            "Unbekannte Startargumente: {}\nBekannt sind: --headless --sandbox --novsync \
             --reexport --no-export --mission <name> --script <datei> --lag <ms> --ticks <n>",
            start.unbekannt.join(", ")
        );
    }

    app.insert_resource(start.clone());
    app.add_plugins(basis_plugins(&start));

    // data laeuft VOR allem anderen: es laedt die RON und kracht beim Start, wenn ein Wert
    // fehlt — nicht still mit einer Null mitten im Spiel (§4).
    app.add_plugins(data::DataPlugin);

    let hz = app.world().resource::<data::GameData>().spiel.simulation_hz;
    app.insert_resource(Time::<Fixed>::from_hz(hz));

    app.init_resource::<Tick>()
        .init_resource::<IdZaehler>()
        .init_resource::<Wuerfel>()
        .add_message::<TitanGetroffen>()
        .add_message::<TitanSpawnen>()
        .add_message::<SpielerWarpen>()
        .add_message::<Markierung>()
        // Ein `MessageWriter<T>` ohne `add_message::<T>()` ist ein LAUFZEITfehler beim
        // System-Init, kein Compilefehler — er faellt erst auf, wenn jemand das System
        // schreibt, und wirft dann jeden Test der Runde um. Deshalb stehen alle vier hier,
        // bevor der erste Sender existiert (docs/schnittstelle.md, „Die Naht zuerst").
        .add_message::<HakenGesetzt>()
        .add_message::<HakenGeloest>()
        .add_message::<Aufprall>()
        .add_message::<KoerperWeg>();

    // Die sechs Stufen eines Simulationsschritts, an GENAU EINER Stelle konfiguriert.
    //
    // Nicht in einem Plugin: `world`, `vector` und `player` sind alle drei Mitglieder, und
    // eine Domaene, die die Reihenfolge einer anderen festlegt, ist eine versteckte Kante an
    // der Erlaubnisliste vorbei. `src/lib.rs` ist die bereits benannte Naht.
    //
    // Die Reihenfolge ist die Antwort auf „wer gewinnt": der Index ist aktuell, bevor
    // jemand ihn fragt; gefragt wird, bevor sich etwas bewegt; gewollt wird, bevor Kraefte
    // entstehen; und bewegt wird zuletzt, von genau einem System.
    app.configure_sets(
        FixedUpdate,
        (
            SchrittSet::Raum,
            SchrittSet::Welt,
            SchrittSet::Absicht,
            SchrittSet::Antrieb,
            SchrittSet::Vollzug,
            SchrittSet::Nachlauf,
        )
            .chain(),
    );

    // Die Reihenfolge IST die Abhaengigkeitsreihenfolge (docs/architektur.md).
    // Verschachtelt, weil `add_plugins` maximal ~15 Elemente pro Tupel nimmt und darueber
    // als unlesbarer Trait-Fehler zuschlaegt (docs/lessons/bevy.md).
    app.add_plugins((
        (
            save::SavePlugin,
            net::NetPlugin,
            world::WorldPlugin,
            render::RenderPlugin,
            player::PlayerPlugin,
            vector::VectorPlugin,
        ),
        (
            blades::BladesPlugin,
            titan::TitanPlugin,
            combat::CombatPlugin,
            mission::MissionPlugin,
            progress::ProgressPlugin,
            squad::SquadPlugin,
        ),
        (
            hud::HudPlugin,
            sound::SoundPlugin,
            menu::MenuPlugin,
            debug::DebugPlugin,
        ),
    ));

    if start.ticks > 0 {
        app.add_systems(Last, nach_ticks_beenden);
    }

    app
}

/// Die Bevy-Grundausstattung, eingestellt auf **diese** Maschine.
fn basis_plugins(start: &Start) -> bevy::app::PluginGroupBuilder {
    let fenster = start.will_fenster().then(|| Window {
        title: FENSTERTITEL.into(),
        // Unter Vsync ist jede Bildzeit 16,6 ms — damit misst „was kostet das?" sechsmal
        // denselben Deckel (§11).
        present_mode: if start.novsync { PresentMode::AutoNoVsync } else { PresentMode::AutoVsync },
        ..default()
    });

    let mut gruppe = DefaultPlugins.set(WindowPlugin {
        primary_window: fenster,
        // Ohne Fenster wuerde `OnAllClosed` sofort herunterfahren: null Fenster sind alle
        // Fenster (docs/lessons/bevy.md).
        exit_condition: if start.will_fenster() {
            ExitCondition::OnPrimaryClosed
        } else {
            ExitCondition::DontExit
        },
        ..default()
    });

    if start.headless {
        // `backends: None` heisst: wgpu sucht gar keinen Adapter. Ohne das faellt der Start
        // auf einer Maschine ohne GPU-Treiber tief in wgpu um.
        gruppe = gruppe.set(RenderPlugin {
            render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                backends: None,
                ..default()
            })),
            ..default()
        });
        // Ohne Fenster gibt es keine Ereignisschleife, die die App antreibt.
        gruppe = gruppe.add(ScheduleRunnerPlugin::run_loop(
            core::time::Duration::from_secs_f64(1.0 / 240.0),
        ));
        #[cfg(any(feature = "x11", feature = "wayland"))]
        {
            // WinitPlugin baut beim Start eine Ereignisschleife und panikt ohne Display.
            gruppe = gruppe.disable::<bevy::winit::WinitPlugin>();
        }
    }

    gruppe
}

/// `--ticks n`: nach n Simulationsschritten beenden.
///
/// Damit hat eine Fahrt ohne Fenster **immer** ein Ende — auch wenn ein Skript in einer
/// Schleife haengt oder gar keins angegeben wurde. Ein Testlauf, der nie zurueckkommt, ist
/// schlimmer als einer, der scheitert.
fn nach_ticks_beenden(tick: Res<Tick>, start: Res<Start>, mut beenden: MessageWriter<AppExit>) {
    if tick.0 >= start.ticks {
        info!("--ticks {} erreicht, Ende", start.ticks);
        beenden.write(AppExit::Success);
    }
}
