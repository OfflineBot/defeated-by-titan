//! `--bild <pfad>` — **ein PNG aus dem laufenden Spiel, ohne Compositor und ohne Handarbeit.**
//!
//! `docs/ABNAHME.md` sagt woertlich: „Ohne Bild kein 🟧, ohne Ausnahme." Ein Screenshot, den
//! ein Mensch im richtigen Moment mit der richtigen Taste macht, ist trotzdem kein Beleg,
//! sondern eine Anekdote: er ist nicht wiederholbar, nicht scriptbar, und morgen zeigt er
//! etwas anderes. `--bild <pfad> --ticks <n>` ist beides — **derselbe Befehl liefert dasselbe
//! Bild**, weil die Simulation fest getaktet ist und der Ausloeser ein Tick ist, keine
//! Sekunde und keine Taste.
//!
//! ## Die drei Modi
//!
//! | Start | Fenster | GPU | Ziel des Screenshots |
//! |---|---|---|---|
//! | Vorgabe | ja | ja | [`Screenshot::primary_window`] |
//! | `--offscreen` | nein | ja | [`Screenshot::image`] — die Kamera rendert in ein `Image` |
//! | `--headless` | nein | **nein** (`backends: None`) | **kein Bild moeglich** |
//!
//! `--headless` steht hier nicht aus Bequemlichkeit ohne Bild da: es setzt in
//! [`crate::basis_plugins`] `backends: None`, wgpu sucht also gar keinen Adapter. Ohne
//! Adapter gibt es kein Render-Ziel und also nichts zu lesen. **`--offscreen` ist genau die
//! Antwort darauf** und der Grund, warum es diesen dritten Modus gibt (`docs/FRAGEN.md`
//! Q-009).
//!
//! ## Belegt am installierten Quelltext, nicht aus dem Gedaechtnis
//!
//! Bevy 0.19 ist neuer als jedes Gedaechtnis, das dieses Projekt anfasst. Alle vier
//! Behauptungen dieser Datei stehen mit Datei und Zeile da:
//!
//! - `bevy_render-0.19.0/src/view/window/screenshot.rs:78-80` — [`Screenshot`] ist eine
//!   **Komponente** an einer eigenen Entity, kein Aufruf und kein System.
//! - `.../screenshot.rs:134` — [`save_to_disk`] ist ein **Observer** (`impl FnMut(On<..>)`),
//!   der erst laeuft, wenn das Bild wirklich da ist. Screenshots sind asynchron.
//! - `.../screenshot.rs:47-53` — [`ScreenshotCaptured`] ist das Ereignis mit dem fertigen
//!   `Image`.
//! - `.../screenshot.rs:309-328` — ein `Image` ist ein **gueltiges Screenshot-Ziel**. Genau
//!   deshalb kann es ohne Fenster ueberhaupt ein Bild geben.
//! - `bevy_camera-0.19.0/src/camera.rs:376-384` — `RenderTarget` ist eine **erforderliche
//!   Komponente** der `Camera`. Man setzt das Ziel also, indem man die Komponente an der
//!   Kamera-Entity ersetzt, nicht indem man in ein Feld schreibt.
//! - `bevy_image-0.19.0/src/image.rs:1232-1246` — `Image::new_target_texture` setzt die drei
//!   noetigen `TextureUsages`. Ein `Image` ohne `RENDER_ATTACHMENT` ist kein Ziel.
//! - `bevy_render-0.19.0/src/lib.rs:501-506` — das Fenster beim Aufbau des Renderers ist
//!   `Option`. **Kein Fenster ist kein Fehler**, sondern `compatible_surface: None`.
//!
//! ## Warum das Ende hier liegt und nicht bei `--ticks`
//!
//! `crate::nach_ticks_beenden` wuerde bei `tick >= ticks` sofort `AppExit` schreiben — also
//! genau in dem Moment, in dem der Screenshot ausgeloest wird und **bevor** ihn irgendwer
//! von der GPU zurueckgelesen hat. Der Lauf endete gruen und ohne Datei. Deshalb uebernimmt
//! bei gesetztem `--bild` diese Datei das Ende, und sie beendet erst, wenn das PNG auf der
//! Platte liegt und nicht leer ist.

use std::path::PathBuf;

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured};

use crate::shared::{Start, Tick};

/// Die Aufloesung des Offscreen-Ziels.
///
/// Keine RON-Zahl: das ist kein Spielwert, sondern die Kantenlaenge eines Pruefwerkzeugs
/// (Regel 2 in `CLAUDE.md` meint Titanenwerte, Klingenstufen, Gaskosten). 1280x720 ist
/// gross genug, um einen Massstab zu erkennen, und klein genug, dass die Datei in ein
/// Repository passt.
pub const OFFSCREEN_BREITE: u32 = 1280;
pub const OFFSCREEN_HOEHE: u32 = 720;

/// Wann ausgeloest wird, wenn `--bild` ohne `--ticks` kommt.
///
/// Bei 60 Hz sind das zwei Sekunden. Der erste Frame taugt nicht: Kamera, Licht und Welt
/// entstehen ueber `Commands` und existieren erst am Ende ihres Ticks — ein Bild bei Tick 0
/// ist zuverlaessig schwarz, und ein schwarzes PNG geht als „Bild vorhanden" durch, obwohl
/// nichts zu sehen ist.
pub const BILD_TICK_VORGABE: u64 = 120;

/// Wie viele Frames auf das PNG gewartet wird, bevor der Lauf als **gescheitert** gilt.
///
/// Ein Lauf, der auf ein Bild wartet, das nie kommt, ist schlimmer als einer, der scheitert:
/// er blockiert eine Sitzung, ohne etwas zu melden.
pub const BILD_GEDULD_FRAMES: u32 = 900;

/// Der eine Bildauftrag dieses Laufs.
#[derive(Resource, Debug)]
pub struct BildAuftrag {
    /// Wohin das PNG geschrieben wird.
    pub pfad: PathBuf,
    /// Ab welchem Simulationstick ausgeloest wird.
    pub bei_tick: u64,
    /// Ohne Fenster: die Kamera rendert in ein `Image`, und das ist das Screenshot-Ziel.
    pub offscreen: bool,
    /// Das Offscreen-Ziel, sobald es steht.
    ziel: Option<Handle<Image>>,
    /// Ob die Screenshot-Entity schon gespawnt wurde.
    ausgeloest: bool,
    /// Ob [`ScreenshotCaptured`] angekommen ist — also ob die GPU wirklich geliefert hat.
    aufgenommen: bool,
    /// Frames seit der Ausloesung, gegen [`BILD_GEDULD_FRAMES`].
    frames: u32,
}

impl BildAuftrag {
    fn neu(start: &Start, pfad: PathBuf) -> Self {
        let bei_tick = if start.ticks > 0 { start.ticks } else { BILD_TICK_VORGABE };
        Self {
            pfad,
            bei_tick,
            offscreen: start.offscreen,
            ziel: None,
            ausgeloest: false,
            aufgenommen: false,
            frames: 0,
        }
    }
}

/// Sitzt an der Kamera, deren Ziel schon umgehaengt wurde — damit es genau einmal passiert.
#[derive(Component)]
pub struct BildZiel;

/// Haengt die Bildsysteme in die App, wenn `--bild` gesetzt ist.
///
/// Wird von [`crate::debug::DebugPlugin`] aufgerufen. Ohne `--bild` passiert hier gar
/// nichts: ein Pruefwerkzeug, das im Normalbetrieb mitlaeuft, ist Rechenzeit fuer etwas,
/// das niemand angefordert hat.
pub fn einhaengen(app: &mut App, start: &Start) {
    let Some(pfad) = start.bild.clone() else {
        return;
    };

    if !start.hat_gpu() {
        // Laut, mit dem Grund UND mit einem Exit-Code ungleich null: „--headless --bild"
        // sieht aus wie eine vernuenftige Zeile und kann grundsaetzlich nicht
        // funktionieren. Ein Lauf, der die angeforderte Datei nicht macht und trotzdem
        // gruen endet, ist genau die Sorte stiller Fehlschlag, die eine Stunde am
        // falschen Ende kostet (§9).
        error!(
            "--bild {} zusammen mit --headless: --headless schaltet den wgpu-Adapter ab \
             (backends: None), es gibt also gar kein Bild zu lesen. Gemeint ist --offscreen \
             (docs/FRAGEN.md Q-009).",
            pfad.display()
        );
        app.add_systems(Last, bild_unmoeglich);
        return;
    }

    let auftrag = BildAuftrag::neu(start, pfad);
    info!(
        "Bildauftrag: {} bei Tick {} ({})",
        auftrag.pfad.display(),
        auftrag.bei_tick,
        if auftrag.offscreen { "offscreen" } else { "Fenster" }
    );
    if start.ticks == 0 {
        info!("--bild ohne --ticks: es wird bei Tick {BILD_TICK_VORGABE} ausgeloest");
    }

    app.insert_resource(auftrag);
    if start.offscreen {
        app.add_systems(Update, offscreen_ziel_haengen);
    }
    app.add_systems(Update, bild_ausloesen).add_systems(Last, bild_beenden);
}

/// Bricht ab, wenn ein Bild angefordert wurde, das es nicht geben kann.
///
/// Der Grund steht schon als `error!` im Log; hier steht nur noch der Exit-Code, damit ein
/// Workflow den Fehlschlag **sieht** und nicht in einer Logzeile ueberliest.
fn bild_unmoeglich(mut beenden: MessageWriter<AppExit>) {
    beenden.write(AppExit::error());
}

/// Ohne Fenster braucht die Kamera ein anderes Ziel — sonst rendert sie in ein Fenster,
/// das es nicht gibt.
///
/// **Das ist eine Fremdkomponente an einer fremden Entity.** `render` besitzt die Kamera;
/// `debug` haengt hier ihr `RenderTarget` um. Erlaubt ist das, weil es genau einmal, nur
/// unter `--offscreen` und nur an einem Pruefpfad passiert — im Normalbetrieb existiert
/// dieses System nicht einmal in der App (siehe [`einhaengen`]). Es steht als Zeile in der
/// Erlaubnisliste von `docs/architektur.md`.
fn offscreen_ziel_haengen(
    mut commands: Commands,
    mut bilder: ResMut<Assets<Image>>,
    mut auftrag: ResMut<BildAuftrag>,
    kameras: Query<Entity, (With<Camera3d>, Without<BildZiel>)>,
) {
    if kameras.is_empty() {
        // Die Kamera entsteht erst, wenn es einen lokalen Spieler gibt (`render`), und
        // `Commands` sind verzoegert. Nicht warnen: das ist der Normalfall der ersten
        // Frames.
        return;
    }

    let ziel = match &auftrag.ziel {
        Some(h) => h.clone(),
        None => {
            // Speicherformat und Ansichtsformat wie im Bevy-Beispiel
            // `examples/3d/render_to_texture.rs:31-35`. Das Ansichtsformat ist der
            // entscheidende Teil: `Image::try_into_dynamic` kennt Rgba8UnormSrgb, aber
            // **nicht** Rgba8Unorm (`bevy_image-0.19.0/src/image_texture_conversion.rs:174-197`)
            // — mit dem falschen Format kommt das Bild von der GPU zurueck und laesst sich
            // dann nicht speichern.
            let h = bilder.add(Image::new_target_texture(
                OFFSCREEN_BREITE,
                OFFSCREEN_HOEHE,
                TextureFormat::Rgba8Unorm,
                Some(TextureFormat::Rgba8UnormSrgb),
            ));
            info!("Offscreen-Ziel: {OFFSCREEN_BREITE}x{OFFSCREEN_HOEHE}");
            auftrag.ziel = Some(h.clone());
            h
        }
    };

    for kamera in &kameras {
        commands
            .entity(kamera)
            .insert((RenderTarget::Image(ziel.clone().into()), BildZiel));
    }
}

/// Loest den Screenshot aus — genau einmal, an einem Tick, nicht an einer Sekunde.
fn bild_ausloesen(mut commands: Commands, tick: Res<Tick>, mut auftrag: ResMut<BildAuftrag>) {
    if auftrag.ausgeloest || tick.0 < auftrag.bei_tick {
        return;
    }

    if auftrag.offscreen && auftrag.ziel.is_none() {
        // Ohne Ziel gibt es nichts zu fotografieren. Warten statt ein leeres Bild machen.
        return;
    }

    // Der Ordner muss existieren, bevor `image` hineinschreibt — sonst scheitert das
    // Speichern in einem Observer, wo es nur eine Logzeile gibt und keinen Exit-Code.
    if let Some(ordner) = auftrag.pfad.parent()
        && let Err(e) = std::fs::create_dir_all(ordner)
    {
        error!("{} laesst sich nicht anlegen — {e}", ordner.display());
    }

    let ziel = match &auftrag.ziel {
        Some(h) => Screenshot::image(h.clone()),
        None => Screenshot::primary_window(),
    };

    commands
        .spawn(ziel)
        .observe(save_to_disk(auftrag.pfad.clone()))
        .observe(bild_vermerken);

    info!("Screenshot ausgeloest bei Tick {}", tick.0);
    auftrag.ausgeloest = true;
}

/// Vermerkt, dass die GPU geliefert hat.
///
/// Zweiter Observer neben [`save_to_disk`] statt eines eigenen Speicherwegs: der
/// Speicherweg von Bevy ist der geprueftere, und zwei Observer an derselben Entity laufen
/// beide am selben Synchronisationspunkt — also lange vor `Last`, wo [`bild_beenden`]
/// nachsieht.
fn bild_vermerken(_: On<ScreenshotCaptured>, mut auftrag: ResMut<BildAuftrag>) {
    auftrag.aufgenommen = true;
}

/// Beendet den Lauf — aber erst, wenn die Datei wirklich da ist.
///
/// **Nicht „der Screenshot wurde ausgeloest".** Ein Lauf, der endet, weil er etwas
/// angefordert hat, beweist nichts ueber das Ergebnis (§9). Geprueft wird die Datei: sie
/// existiert und sie ist nicht leer. Erst dann Exit 0.
fn bild_beenden(mut auftrag: ResMut<BildAuftrag>, mut beenden: MessageWriter<AppExit>) {
    if !auftrag.ausgeloest {
        return;
    }
    auftrag.frames += 1;

    if auftrag.aufgenommen {
        match std::fs::metadata(&auftrag.pfad) {
            Ok(m) if m.len() > 0 => {
                info!("Bild geschrieben: {} ({} Bytes)", auftrag.pfad.display(), m.len());
                beenden.write(AppExit::Success);
                return;
            }
            Ok(_) => {
                error!("{} ist 0 Bytes gross", auftrag.pfad.display());
                beenden.write(AppExit::error());
                return;
            }
            // Der Observer kann in derselben Runde noch am Schreiben sein — noch ein Frame.
            Err(_) => {}
        }
    }

    if auftrag.frames >= BILD_GEDULD_FRAMES {
        error!(
            "Nach {BILD_GEDULD_FRAMES} Frames kein Bild unter {} — der Screenshot wurde \
             ausgeloest, aber nie zurueckgelesen",
            auftrag.pfad.display()
        );
        beenden.write(AppExit::error());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auftrag(ticks: u64, offscreen: bool) -> BildAuftrag {
        let start = Start { ticks, offscreen, ..Start::default() };
        BildAuftrag::neu(&start, PathBuf::from("docs/bilder/probe"))
    }

    #[test]
    fn ohne_ticks_wird_nicht_bei_null_ausgeloest() {
        // Ein Bild bei Tick 0 ist zuverlaessig schwarz: Kamera, Licht und Welt entstehen
        // ueber Commands und existieren erst am Ende ihres Ticks.
        let a = auftrag(0, false);
        assert_eq!(a.bei_tick, BILD_TICK_VORGABE);
        assert!(a.bei_tick > 0, "Tick 0 waere ein schwarzes Bild");
    }

    #[test]
    fn ticks_bestimmen_den_ausloeser() {
        assert_eq!(auftrag(300, false).bei_tick, 300);
    }

    #[test]
    fn offscreen_wird_uebernommen() {
        assert!(auftrag(60, true).offscreen);
        assert!(!auftrag(60, false).offscreen);
    }

    #[test]
    fn ein_frischer_auftrag_ist_weder_ausgeloest_noch_aufgenommen() {
        // Sonst wuerde `bild_beenden` im ersten Frame mit Erfolg enden, ohne dass je ein
        // Screenshot angefordert wurde.
        let a = auftrag(60, false);
        assert!(!a.ausgeloest);
        assert!(!a.aufgenommen);
        assert_eq!(a.frames, 0);
    }
}
