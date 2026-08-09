//! Der Waechter ueber „es gibt keinen **den** Spieler".
//!
//! Er spawnt **zwei** Spieler und laesst die Simulation ein paar Ticks laufen. Er faellt in
//! der Sekunde um, in der jemand `.single()` auf eine Spieler-Query schreibt oder
//! Spielerzustand in eine `Resource` legt (`prompts/init.md` §6, `docs/multiplayer.md`).
//!
//! **Ohne ihn verfaellt der ganze Multiplayer-Abschnitt still** — und man merkt es erst,
//! wenn Multiplayer dran ist, also nach Monaten Arbeit, die man dann anfassen muss.

use bevy::app::FixedMain;
use bevy::prelude::*;
use defeated_by_titan::data::GameData;
use defeated_by_titan::player::spieler_spawnen;
use defeated_by_titan::shared::{
    Bewegungszustand, Gas, IdZaehler, Intent, PlayerId, Start, Tasten, Tempo,
};

/// Baut die **echte** App, headless. Nicht eine zweite, aehnliche — sonst beweist der Test
/// nichts ueber das Spiel, das gespielt wird.
fn app() -> App {
    defeated_by_titan::app(Start { headless: true, ..default() })
}

/// Laesst die feste Simulation **genau** `n` Ticks laufen.
///
/// Nicht ueber `app.update()`: `run_fixed_main_schedule` speist sich aus
/// `Time<Virtual>.delta()`, und die wird in `First` aus der **Echtzeit** ueberschrieben. In
/// einem Test vergehen zwischen zwei `update()` nur Mikrosekunden — es kaeme also fast nie
/// ein fester Schritt zustande, und wie viele es waeren, haenge an der Tageslaune der
/// Maschine. Ein Test, dessen Schrittzahl von der Maschine abhaengt, misst die Maschine.
///
/// Deshalb wird `Time<Fixed>` direkt vorgestellt und `FixedMain` direkt gefahren: `n`
/// Schritte sind `n` Schritte, auf jedem Rechner.
fn ticks(app: &mut App, n: u64) {
    let schritt = app.world().resource::<Time<Fixed>>().timestep();
    for _ in 0..n {
        app.world_mut().resource_mut::<Time<Fixed>>().advance_by(schritt);
        app.world_mut().run_schedule(FixedMain);
    }
}

#[test]
fn mehrspieler_zwei_spieler_simulieren_unabhaengig() {
    let mut app = app();
    app.update(); // Startup: die Welt und der lokale Spieler entstehen

    // Ein zweiter Spieler, ohne LocalPlayer-Marker — genau so, wie spaeter ein Mitspieler
    // aus dem Netz dazukommt.
    let zweiter = {
        let welt = app.world_mut();
        let daten = welt.resource::<GameData>().clone();
        let mut zaehler = welt.resource::<IdZaehler>().to_owned();
        let mut commands = welt.commands();
        let e = spieler_spawnen(
            &mut commands,
            &mut zaehler,
            &daten,
            Vec3::new(20.0, 2.0, 0.0),
            false,
        );
        *welt.resource_mut::<IdZaehler>() = zaehler;
        e
    };
    app.update();

    let mut ids: Vec<PlayerId> = app
        .world_mut()
        .query::<&PlayerId>()
        .iter(app.world())
        .copied()
        .collect();
    ids.sort_by_key(|p| p.0);
    assert_eq!(ids.len(), 2, "es muessen zwei Spieler in der Welt sein, nicht {}", ids.len());
    assert_ne!(ids[0], ids[1], "zwei Spieler mit derselben PlayerId");

    // Nur der ZWEITE bekommt einen Bewegungswunsch. Wenn danach beide gelaufen sind, teilen
    // sie sich Zustand — genau das darf nicht sein.
    {
        let mut intent = app
            .world_mut()
            .get_mut::<Intent>(zweiter)
            .expect("der zweite Spieler hat ein Intent");
        intent.bewegen_y = 1.0;
    }

    let vorher_lokal = position(&mut app, ids[0]);
    let vorher_zweiter = app.world().get::<Transform>(zweiter).unwrap().translation;

    ticks(&mut app, 30);

    let nachher_lokal = position(&mut app, ids[0]);
    let nachher_zweiter = app.world().get::<Transform>(zweiter).unwrap().translation;

    assert!(
        (nachher_zweiter - vorher_zweiter).length() > 0.5,
        "der zweite Spieler haette laufen muessen: {vorher_zweiter:?} -> {nachher_zweiter:?}"
    );
    assert!(
        (nachher_lokal.xz() - vorher_lokal.xz()).length() < 1e-3,
        "der lokale Spieler hat sich mitbewegt, obwohl nur der zweite einen Wunsch hatte \
         ({vorher_lokal:?} -> {nachher_lokal:?}) — irgendwo haengt Spielerzustand global"
    );
}

fn position(app: &mut App, wer: PlayerId) -> Vec3 {
    let mut q = app.world_mut().query::<(&PlayerId, &Transform)>();
    q.iter(app.world())
        .find(|(id, _)| **id == wer)
        .map(|(_, t)| t.translation)
        .expect("den Spieler muss es geben")
}

#[test]
fn mehrspieler_spielerzustand_haengt_am_spieler_nicht_an_der_welt() {
    // Gas, Intent, Tempo und Bewegungszustand sind **Components**. Als `Resource` waeren sie
    // global — und damit waere das Spiel ein Einzelspieler-Spiel, dem man das erst in
    // Monat 12 ansieht (§6 Regel 3).
    //
    // Diese Zeilen lassen sich nicht als Laufzeit-Pruefung schreiben: `world.get_resource::<Gas>()`
    // **kompiliert gar nicht**, solange `Gas` kein `Resource` ist. Der Compiler ist hier der
    // schaerfere Waechter als jedes `assert` — die Pruefung, die bleibt, ist deshalb die
    // umgekehrte: haengt der Zustand wirklich an jedem einzelnen Spieler?
    let mut app = app();
    app.update();

    let mit_zustand = app
        .world_mut()
        .query::<(&PlayerId, &Gas, &Intent, &Tempo, &Bewegungszustand)>()
        .iter(app.world())
        .count();
    let spieler = app.world_mut().query::<&PlayerId>().iter(app.world()).count();
    assert_eq!(
        mit_zustand, spieler,
        "{spieler} Spieler, aber nur {mit_zustand} mit eigenem Zustand — irgendwo ist etwas \
         global geworden"
    );
}

#[test]
fn mehrspieler_kein_spielerzustand_wird_als_resource_abgelegt() {
    // Der Compiler faengt `get_resource::<Gas>()` ab — aber nicht den Tag, an dem jemand
    // `#[derive(Resource)]` an `Gas` schreibt. Genau davor schuetzt diese Pruefung, und sie
    // faellt beim Hinschreiben um statt in Monat 12.
    let wurzel = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let zustand = wurzel.join("src/shared/zustand.rs");
    let text = std::fs::read_to_string(&zustand).expect("src/shared/zustand.rs muss es geben");

    for (nr, zeile) in text.lines().enumerate() {
        if zeile.trim_start().starts_with("//") {
            continue;
        }
        assert!(
            !zeile.contains("Resource"),
            "{}:{} — in shared/zustand.rs steht {zeile:?}. Gas, Klingen, Tempo und \
             Bewegungszustand gehoeren an den SPIELER, nicht in die Welt (init.md §6 Regel 3)",
            zustand.display(),
            nr + 1
        );
    }
}

#[test]
fn mehrspieler_kein_single_auf_spieler_queries_im_quelltext() {
    // Der Test oben faellt erst um, wenn `.single()` auch WIRKLICH panikt (also bei zwei
    // Spielern). Diese Pruefung faellt schon beim Hinschreiben um — das ist billiger.
    let wurzel = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut treffer = Vec::new();
    let mut offen = vec![wurzel.join("src")];
    while let Some(p) = offen.pop() {
        for eintrag in std::fs::read_dir(&p).expect("lesbar") {
            let pfad = eintrag.expect("Eintrag").path();
            if pfad.is_dir() {
                offen.push(pfad);
                continue;
            }
            if pfad.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&pfad).expect("lesbar");
            let kennt_spieler = text.contains("PlayerId") || text.contains("LocalPlayer");
            for (nr, zeile) in text.lines().enumerate() {
                let code = zeile.trim_start();
                if code.starts_with("//") {
                    continue;
                }
                if kennt_spieler && (code.contains(".single()") || code.contains(".single_mut()"))
                {
                    treffer.push(format!(
                        "{}:{}",
                        pfad.strip_prefix(&wurzel).unwrap_or(&pfad).display(),
                        nr + 1
                    ));
                }
            }
        }
    }
    assert!(
        treffer.is_empty(),
        "`.single()` in Dateien, die Spieler kennen: {treffer:?}\n\
         Jeder Spieler ist einer von vielen — ueber die Query iterieren, und wenn wirklich \
         „ich\" gemeint ist, ueber den LocalPlayer-Marker gehen (init.md §6 Regel 3)"
    );
}

#[test]
fn t019_lag_verzoegert_die_zustellung_und_verliert_nichts() {
    // Bibel T-019: jedes Bewegungsfeature wird AUCH bei 200 ms simulierter Latenz geprueft.
    // Der Schalter muss also existieren und wirken — nicht nur im Hilfetext stehen.
    let mut app = defeated_by_titan::app(Start { headless: true, lag_ms: 200, ..default() });
    app.update();

    let post = app.world().resource::<defeated_by_titan::net::Posteingang>();
    assert_eq!(
        post.lag_ticks, 12,
        "200 ms bei 60 Hz sind 12 Ticks, nicht {}",
        post.lag_ticks
    );
}

#[test]
fn mehrspieler_tasten_ueberleben_den_weg_durch_den_posteingang() {
    use defeated_by_titan::net::Posteingang;

    let mut post = Posteingang::mit_lag(5);
    let mut tasten = Tasten::KEINE;
    tasten.setzen(Tasten::HAKEN_LINKS, true);
    tasten.setzen(Tasten::BOOST, true);
    post.einwerfen(PlayerId(7), Intent { tasten, tick: 3, ..default() }, 3);

    let raus = post.abholen(8);
    assert_eq!(raus.len(), 1);
    assert!(raus[0].1.haelt(Tasten::HAKEN_LINKS));
    assert!(raus[0].1.haelt(Tasten::BOOST));
    assert!(!raus[0].1.haelt(Tasten::SPRINGEN));
    assert_eq!(raus[0].1.tick, 3, "der Intent traegt den Tick, fuer den er gemeint war");
}

#[test]
fn mehrspieler_tempo_ist_ein_component_am_spieler() {
    let mut app = app();
    app.update();
    let n = app.world_mut().query::<(&PlayerId, &Tempo)>().iter(app.world()).count();
    assert_eq!(n, 1, "Geschwindigkeit gehoert an den Spieler, nicht in die Welt");
}
