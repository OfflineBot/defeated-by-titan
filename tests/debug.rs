//! Der Waechter ueber den Gizmos — **den Strichen, ohne die ein Bild kein Beleg ist.**
//!
//! `docs/ABNAHME.md` verlangt fuer 🟧 ein Bild, auf dem man etwas **erkennt**. Auf
//! `docs/bilder/t006-welt-fern.png` sieht man Kloetze, aber nicht, welcher davon hakbar ist.
//! `src/debug/gizmo.rs` zeichnet genau das — und diese Datei sorgt dafuer, dass die Regel
//! nicht still verfaellt:
//!
//! - **Sie sind registriert und laufen.** Nimmt jemand die `add_systems`-Zeile aus
//!   `src/debug/mod.rs`, faellt hier etwas um und nicht erst dem naechsten Auftrag sein Bild.
//! - **Sie zeichnen nur, was markiert ist.** Ein Gizmo an einem Klotz ohne
//!   [`Ankerflaeche`](defeated_by_titan::shared::Ankerflaeche) wuerde eine Aussage
//!   behaupten, die das Spiel nicht macht — und `F-003` („kein Haken auf ungetaggten
//!   Flaechen") waere im Bild nicht mehr pruefbar.
//! - **Sie zeichnen gar nichts, wenn der Schalter aus ist.**
//!
//! Was sich **ohne** App pruefen laesst — Farben, Kantenzahl, Masse, der Schalter —, steht
//! als Einheitentest in `src/debug/gizmo.rs`. Hier steht nur, was eine echte App braucht.

use bevy::prelude::*;
use defeated_by_titan::data::GameData;
use defeated_by_titan::debug::gizmo::{GizmoSchalter, GizmoZaehler, GizmoZeichnen};
use defeated_by_titan::player::spieler_spawnen;
use defeated_by_titan::shared::{Ankerflaeche, Bauklotz, IdZaehler, Start};

/// Baut die **echte** App, headless — nicht eine zweite, aehnliche.
///
/// Der Schalter wird ausdruecklich gesetzt und nicht aus der Umgebung gelesen: ein Test,
/// der `DBT_GIZMOS` umsetzt, prueft den Prozess statt der Regel und stoert jeden parallel
/// laufenden Test im selben Prozess.
fn app(gizmos_an: bool) -> App {
    let mut app = defeated_by_titan::app(Start { headless: true, ..default() });
    app.insert_resource(GizmoSchalter { an: gizmos_an });
    app
}

fn zaehler(app: &App) -> GizmoZaehler {
    *app.world().resource::<GizmoZaehler>()
}

/// Ein Quader mit derselben Form wie ein Haus, damit der Test nicht an einer Sonderform
/// haengt. Ob er hakbar ist, ist das Einzige, was sich zwischen den Faellen unterscheidet.
fn klotz() -> Bauklotz {
    Bauklotz { groesse: Vec3::new(6.0, 9.0, 6.0), farbe: [0.42, 0.43, 0.40] }
}

#[test]
fn die_gizmo_systeme_stehen_im_update_schedule() {
    // Der wortwoertliche Teil: die drei Systeme sind eingetragen. Der Test daneben prueft,
    // dass sie auch etwas tun — beides zusammen faellt um, egal ob jemand die Registrierung
    // entfernt oder sie stehen laesst und den Rumpf leert.
    //
    // Geprueft wird ueber das SET und nicht ueber Systemnamen: ohne `bevy_utils/debug`
    // heisst hier jedes System woertlich "<Enable the debug feature to see the name>"
    // (gemessen, siehe `src/debug/gizmo.rs::GizmoZeichnen`) — ein Namenstest waere gruen,
    // ohne irgendetwas zu wissen.
    let mut app = app(false);
    app.update(); // ohne einen Durchlauf ist der Schedule nicht initialisiert

    let schedule = app.get_schedule(Update).expect("Update-Schedule");
    let systeme = schedule
        .graph()
        .systems_in_set(GizmoZeichnen.intern())
        .expect("das Set GizmoZeichnen steht nicht im Update-Schedule");

    assert_eq!(
        systeme.len(),
        3,
        "es sollten drei Zeichensysteme im Set stehen (Anker, Massstab, Spieler) — ohne sie \
         hat der naechste Auftrag kein Bild, auf dem man etwas erkennt (docs/ABNAHME.md)"
    );
}

#[test]
fn die_gizmos_laufen_und_umranden_die_ankerflaechen_der_karte() {
    let mut app = app(true);
    app.update(); // Startup baut die Karte, Update zeichnet sie

    let gezeichnet = zaehler(&app).anker;
    let vorhanden = ankerflaechen(&mut app);
    assert!(vorhanden > 0, "die Karte hat keine einzige Ankerflaeche — Test misst nichts");
    assert_eq!(
        gezeichnet, vorhanden,
        "{vorhanden} Ankerflaechen in der Welt, aber {gezeichnet} umrandet"
    );
}

#[test]
fn ein_klotz_ohne_ankerflaeche_bekommt_kein_gizmo() {
    // **Das ist die Aussage des Bildes.** Wuerde jeder Klotz umrandet, hiesse „umrandet"
    // nur noch „ist ein Klotz" — und `F-003` waere auf keinem Screenshot mehr pruefbar.
    let mut app = app(true);
    app.update();
    let vorher = zaehler(&app).anker;

    app.world_mut().spawn((Name::new("probe_ungetaggt"), klotz(), Transform::from_xyz(80.0, 4.5, 0.0)));
    app.update();
    assert_eq!(
        zaehler(&app).anker,
        vorher,
        "ein Klotz ohne Ankerflaeche wurde umrandet — das Bild wuerde etwas behaupten, \
         was das Spiel nicht tut"
    );

    app.world_mut().spawn((
        Name::new("probe_getaggt"),
        klotz(),
        Ankerflaeche,
        Transform::from_xyz(80.0, 4.5, 20.0),
    ));
    app.update();
    assert_eq!(
        zaehler(&app).anker,
        vorher + 1,
        "eine neue Ankerflaeche blieb unsichtbar — dann zeigt das Bild einen alten Stand"
    );
}

#[test]
fn ohne_schalter_wird_nichts_gezeichnet() {
    // Gizmos duerfen nicht immer mitlaufen: Rechenzeit, und auf einem Spielbild stoeren sie.
    let mut app = app(false);
    app.update();
    app.update();
    assert_eq!(
        zaehler(&app),
        GizmoZaehler::default(),
        "der Schalter ist aus und es wurde trotzdem gezeichnet"
    );
    assert!(ankerflaechen(&mut app) > 0, "es haette etwas zu zeichnen gegeben");
}

#[test]
fn die_kapsel_mit_der_kamera_darin_bleibt_leer_ein_mitspieler_nicht() {
    // Die Kamera haengt als Kind am lokalen Spieler und sitzt in seiner Kapsel. Zeichnete
    // man sie, laege die eigene Huelle als Gitter ueber das ganze Bild — 0,35 m vor einem
    // 60-Grad-Objektiv ist eine Kante bildfuellend.
    let mut app = app(true);
    for _ in 0..3 {
        app.update(); // Spieler, dann Kamera (Commands sind verzoegert), dann Propagation
    }
    assert_eq!(
        zaehler(&app).spieler,
        0,
        "die eigene Kapsel wurde gezeichnet — sie verdeckt jedes Bild aus der Ich-Sicht"
    );

    // Ein Mitspieler, genau so, wie er spaeter aus dem Netz dazukommt: ohne LocalPlayer,
    // ohne Kamera. Er MUSS markiert werden, sonst ist er auf einem Fernbild unsichtbar.
    {
        let welt = app.world_mut();
        let daten = welt.resource::<GameData>().clone();
        let mut id_zaehler = welt.resource::<IdZaehler>().to_owned();
        let mut commands = welt.commands();
        spieler_spawnen(&mut commands, &mut id_zaehler, &daten, Vec3::new(60.0, 2.0, 0.0), false);
    }
    for _ in 0..2 {
        app.update();
    }
    assert_eq!(
        zaehler(&app).spieler,
        1,
        "ein Mitspieler ohne Kamera blieb unmarkiert — genau der Fall, fuer den es die \
         Markierung gibt (docs/multiplayer.md Regel 3)"
    );
}

/// Wie viele Entities die Welt gerade als hakbar fuehrt.
fn ankerflaechen(app: &mut App) -> usize {
    let mut abfrage = app.world_mut().query_filtered::<Entity, With<Ankerflaeche>>();
    abfrage.iter(app.world()).count()
}
