//! gizmo — die Striche, die aus einem Bild einen **Beleg** machen.
//!
//! `docs/ABNAHME.md` sagt „Ohne Bild kein 🟧, ohne Ausnahme". Ein Bild allein reicht dafuer
//! aber nicht: auf `docs/bilder/t006-welt-fern.png` sind Boden und Kloetze zu sehen und
//! sonst nichts — **welcher Klotz hakbar ist, steht nicht im Bild.** Wer als naechster eine
//! Bildzeile fuer `F-002`, `F-003` oder `F-004` fuehren muss, kann sie ohne diese Datei
//! nicht fuehren. Das Gegenstueck mit eingeschalteten Gizmos ist
//! `docs/bilder/f003-anker.png`; der Unterschied zwischen den beiden **ist** der Beleg.
//!
//! Drei Dinge werden gezeichnet, und jedes beantwortet eine Frage, die ein Bild sonst offen
//! laesst:
//!
//! | gezeichnet | Farbe | beantwortet |
//! |---|---|---|
//! | Umrandung jeder [`Ankerflaeche`] | **Zyan** | was ist hakbar? |
//! | Achsenkreuz im Ursprung, Bodenraster | neutral | wie gross, wie weit? |
//! | Huelle und Mast jedes Spielers | weiss | wo steht wer? |
//!
//! ## Die Farben sind nicht frei
//!
//! `docs/konventionen.md` §3 reserviert **Zyan** fuer „Gas, Vector Gear, **Ankerpunkte**",
//! **Bernstein** fuer Ziele und Schwachstellen, **Karminrot** fuer Gefahr. Fuer eine
//! Ankerflaeche ist Zyan damit **vorgeschrieben** und nicht gewaehlt. Alles andere hier ist
//! keins der drei und bleibt deshalb **neutral**.
//!
//! Das betrifft vor allem das Achsenkreuz: die uebliche Zuordnung X=rot / Y=gruen / Z=blau
//! waere ein Verstoss, weil Rot der Gefahr gehoert. Deshalb ist **X magenta** statt rot
//! (voller Blauanteil, also weit weg von Karminrot), **Y hellgrau** statt gruen, und **Z
//! bleibt blau** — Blau ist nicht Zyan, solange der Gruenanteil niedrig ist. Der Spieler
//! ist weiss; das Achsenkreuz steht ausschliesslich im Ursprung, die beiden koennen sich
//! also nicht verwechseln lassen.
//!
//! ## Der Schalter — und warum er eine Umgebungsvariable ist
//!
//! Gizmos duerfen nicht immer mitlaufen: sie kosten Rechenzeit, und auf einem Spielbild
//! stoeren sie. Der naheliegende Schalter waere ein Startflag im Stil der neun anderen
//! ([`Start`](crate::shared::Start)) — **aber `src/shared/start.rs` gehoert diesem Auftrag
//! nicht**, und ein stiller Fremdfix ist ein unsichtbarer Merge-Konflikt mit dem Agenten,
//! der gerade daneben arbeitet. Ein von Hand aus `std::env::args()` gefischtes `--gizmos`
//! scheidet ebenfalls aus: [`Start::aus`](crate::shared::Start::aus) legt jedes unbekannte
//! Argument in `unbekannt` ab, und der Start meldete dann laut ein Flag als unbekannt, das
//! sehr wohl gewirkt hat.
//!
//! Bleibt die Umgebungsvariable [`UMGEBUNGSSCHALTER`]. Sie kommt ohne fremde Dateien aus
//! und laesst sich an denselben Bildbefehl haengen:
//!
//! ```text
//! env DBT_GIZMOS=1 cargo run --features wayland,klang -- --offscreen \
//!     --script scripts/t006-bild-fern.txt --ticks 110 --bild docs/bilder/f003-anker.png
//! ```
//!
//! Mit Fenster schaltet **F4** um (F3 ist das Overlay). Der richtige Ort bleibt trotzdem
//! ein `--gizmos`-Flag in `src/shared/start.rs` — das steht als Fund im Bericht.
//!
//! ## Warum die Zahlen hier stehen und nicht in einer RON
//!
//! Regel 2 („Zahlen gehoeren in RON") meint Spielwerte: Titanentypen, Klingenstufen,
//! Gaskosten. Eine Linienbreite in Pixeln und eine Rasterweite sind Kantenlaengen eines
//! **Pruefwerkzeugs**, genau wie [`OFFSCREEN_BREITE`](super::bild::OFFSCREEN_BREITE) in
//! `src/debug/bild.rs`. Was hier aus der RON kommt, ist alles, was den Spieler beschreibt:
//! seine Hoehe und sein Radius stehen in `game.ron` und werden nicht nachgebaut.

use bevy::gizmos::gizmos::GizmoBuffer;
use bevy::prelude::*;
use core::f32::consts::FRAC_PI_2;

use crate::data::GameData;
use crate::shared::{spielerhuelle, Ankerflaeche, Bauklotz, Koerper, PlayerId};

// ---------------------------------------------------------------------------
// Farben — lineares RGB, dieselbe Form wie `Bauklotz::farbe`
// ---------------------------------------------------------------------------

/// **Zyan.** `docs/konventionen.md` §3: „Zyan — Gas, Vector Gear, Ankerpunkte." Eine
/// Ankerflaeche ist genau das; hier ist die Farbe Vorschrift, nicht Geschmack.
const ZYAN: [f32; 3] = [0.0, 0.85, 1.0];

/// +X. Magenta statt des ueblichen Rot: Karminrot gehoert der Gefahr.
const ACHSE_X: [f32; 3] = [1.0, 0.05, 0.75];
/// +Y. Hellgrau statt des ueblichen Gruen — neutral, und „oben" ist ohnehin eindeutig.
const ACHSE_Y: [f32; 3] = [0.85, 0.85, 0.85];
/// +Z. Blau darf bleiben: es ist keine der drei Signalfarben, solange der Gruenanteil
/// niedrig genug ist, dass es nicht als Zyan durchgeht.
const ACHSE_Z: [f32; 3] = [0.10, 0.25, 1.0];

/// Das Bodenraster. Fast schwarz, weil der Boden hell ist — und neutral, weil ein Raster
/// keine Aussage ueber Gameplay macht.
const RASTER: [f32; 3] = [0.02, 0.02, 0.03];

/// Der Spieler. Weiss, neutral.
const SPIELER: [f32; 3] = [1.0, 1.0, 1.0];

/// Wie stark die negative Haelfte einer Achse gegenueber der positiven abgedunkelt wird.
/// Damit sieht man auf einen Blick, wo +X aufhoert und −X anfaengt, ohne eine zweite Farbe
/// zu verbrauchen.
const HALB_HELL: f32 = 0.35;

// ---------------------------------------------------------------------------
// Masse des Pruefwerkzeugs
// ---------------------------------------------------------------------------

/// Kantenlaenge einer Rasterzelle. Zehn Meter sind die Zahl, in der ein Mensch Entfernungen
/// schaetzt, und ein Vielfaches der Spielerhoehe (1,8 m) waere keine.
const RASTER_ZELLE_M: f32 = 10.0;
/// Zellen je Achse. 20 x 10 m = 200 m Kantenlaenge, zentriert im Ursprung — die halbe
/// Graubox (`maps.ron: groesse_m = 400 x 400`).
const RASTER_ZELLEN: u32 = 20;
/// Wie hoch das Raster ueber der Bodenoberkante schwebt. Die Bodenplatte endet bei y = 0
/// (`maps.ron: kloetze[0]`); ohne diesen Abstand flimmern Raster und Platte gegeneinander.
/// Zehn Zentimeter sind auf 45 m Entfernung ein Zehntel Pixel und aendern keine Ablesung.
const RASTER_HOEHE_M: f32 = 0.1;

/// Armlaenge des Achsenkreuzes — **genau eine Rasterzelle**. Damit ist das Kreuz zugleich
/// der Massstab fuer das Raster und nicht eine zweite, konkurrierende Laenge.
const ACHSE_M: f32 = RASTER_ZELLE_M;

/// Wie weit der Mast ueber den Kopf eines Spielers hinausragt. Ohne ihn ist eine 1,8 m hohe
/// Kapsel auf 80 m Entfernung ein paar Pixel gross und im Bild nicht zu finden.
const MAST_M: f32 = 3.0;

/// Linienbreite der Aussagen (Anker, Spieler, Achsen) in Pixeln. Bevys Vorgabe ist 2,0 —
/// auf 1280x720 ist eine Umrandung damit da, aber nicht *deutlich*.
const LINIE_PX: f32 = 3.0;
/// Linienbreite des Rasters. Duenner als die Aussagen: das Raster ist Hintergrund und darf
/// die Umrandungen nicht ueberschreien.
const RASTER_PX: f32 = 1.0;

/// Wie weit Gizmos zur Kamera hin verschoben werden (−1 bis 1, 0 = gar nicht,
/// `bevy_gizmos-0.19.0/src/config.rs:215-227`).
///
/// Eine Umrandung liegt **exakt** auf der Oberflaeche, die sie umrandet — ohne Versatz
/// flimmert jede Kante gegen ihre eigene Wand.
///
/// **Der Wert ist gemessen, nicht geschaetzt.** Die Skala ist die des Tiefenpuffers und
/// damit stark nichtlinear: `−0.05` klang klein und war es nicht — auf 45 bis 150 m
/// Entfernung zeichnete es jede Umrandung *durch* ihr eigenes Haus, alle zwoelf Kanten
/// sichtbar, und Umrandungen weiter hinten lagen ueber den Haeusern davor
/// (Vergleichsbild derselben Fahrt). `−0.001` haelt die Verdeckung intakt und flimmert
/// trotzdem nicht: `docs/bilder/f003-anker.png` ist damit aufgenommen.
const TIEFENVERSATZ: f32 = -0.001;

// ---------------------------------------------------------------------------
// Der Schalter
// ---------------------------------------------------------------------------

/// Die Umgebungsvariable, die die Gizmos beim Start einschaltet. Siehe Modulkopf, warum es
/// kein Startflag ist.
pub const UMGEBUNGSSCHALTER: &str = "DBT_GIZMOS";

/// Ob gezeichnet wird. **Kein Spielerzustand** — eine Anzeigeeinstellung dieses Prozesses,
/// deshalb darf sie eine `Resource` sein (`docs/multiplayer.md` verbietet Spieler*zustand*
/// als Resource, nicht jede Resource).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GizmoSchalter {
    pub an: bool,
}

impl GizmoSchalter {
    /// Liest [`UMGEBUNGSSCHALTER`] genau einmal, beim Bauen der App.
    pub fn aus_umgebung() -> Self {
        Self { an: schalter_aus_text(std::env::var(UMGEBUNGSSCHALTER).ok().as_deref()) }
    }
}

/// Ob ein Text den Schalter einschaltet.
///
/// Als eigene Funktion, damit die Regel **ohne Umgebungsvariablen** pruefbar ist — dasselbe
/// Muster wie `shared::start::skript_erzwingt_headless`: ein Test, der `DBT_GIZMOS` umsetzt,
/// prueft den Prozess und nicht die Regel, und stoert nebenbei jeden parallel laufenden
/// Test im selben Prozess.
pub fn schalter_aus_text(wert: Option<&str>) -> bool {
    matches!(wert.map(str::trim), Some("1" | "an" | "ja" | "true"))
}

/// Die Bedingung, unter der die drei Zeichensysteme ueberhaupt laufen.
pub fn gizmos_an(schalter: Res<GizmoSchalter>) -> bool {
    schalter.an
}

/// **F4** schaltet um — F3 gehoert dem Overlay (`super::overlay_fuellen`).
pub fn schalter_umschalten(tasten: Res<ButtonInput<KeyCode>>, mut schalter: ResMut<GizmoSchalter>) {
    if tasten.just_pressed(KeyCode::F4) {
        schalter.an = !schalter.an;
        info!("Gizmos {}", if schalter.an { "an" } else { "aus" });
    }
}

/// Was in der letzten Runde wirklich gezeichnet wurde.
///
/// Nicht Zierde: `docs/lessons/performance.md` verlangt, dass niemand ueber alle Entities
/// laeuft, um eine lokale Frage zu beantworten — diese beiden Zahlen sind die **Messung**
/// dazu und stehen im Log, statt geschaetzt zu werden. Jedes Feld hat genau einen
/// Schreiber: `anker` schreibt [`anker_zeichnen`], `spieler` schreibt [`spieler_zeichnen`].
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GizmoZaehler {
    pub anker: usize,
    pub spieler: usize,
}

/// Haengt Schalter, Zaehler, die Rastergruppe und die Linienbreiten in die App.
///
/// Wird von [`super::DebugPlugin`] aufgerufen — dieselbe Form wie
/// [`super::bild::einhaengen`], damit `debug/mod.rs` duenn bleibt.
pub fn einhaengen(app: &mut App) {
    app.insert_resource(GizmoSchalter::aus_umgebung())
        .init_resource::<GizmoZaehler>()
        // Eigene Gruppe nur fuer das Raster: die Linienbreite haengt an der Gruppe, nicht
        // am einzelnen Aufruf. Ohne sie waere das Raster genauso fett wie die Umrandungen
        // und wuerde sie im Bild erschlagen.
        .init_gizmo_group::<RasterGizmos>();

    let mut store = app.world_mut().resource_mut::<GizmoConfigStore>();
    let (aussage, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    aussage.line.width = LINIE_PX;
    aussage.depth_bias = TIEFENVERSATZ;
    let (raster, _) = store.config_mut::<RasterGizmos>();
    raster.line.width = RASTER_PX;
    raster.depth_bias = TIEFENVERSATZ;
}

/// Die Gizmo-Gruppe des Bodenrasters — duenner als alles, was eine Aussage macht.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct RasterGizmos;

/// Die drei Zeichensysteme als **eine benannte Menge**.
///
/// Das ist keine Kosmetik, sondern die einzige Art, ihre Registrierung zu pruefen: dieses
/// Crate schaltet `bevy_utils/debug` nicht ein, und ohne dieses Feature liefert
/// `System::name()` woertlich `"<Enable the debug feature to see the name>"`
/// (`bevy_utils-0.19.0/src/debug_info.rs:10-21`) — ein Test ueber Systemnamen ist hier also
/// grundsaetzlich blind. Ueber ein benanntes Set geht es ohne Namen:
/// `schedule.graph().systems_in_set(GizmoZeichnen.intern())` zaehlt sie
/// (`bevy_ecs-0.19.0/src/schedule/schedule.rs:964-980`, `tests/debug.rs`).
///
/// Und sie traegt die Bedingung: [`gizmos_an`] wird **einmal** je Durchlauf ausgewertet und
/// nicht dreimal.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GizmoZeichnen;

// ---------------------------------------------------------------------------
// 1. Ankerflaechen — was ist hakbar?
// ---------------------------------------------------------------------------

/// Umrandet **jede** [`Ankerflaeche`] in Zyan.
///
/// Die Query traegt `With<Ankerflaeche>`, beruehrt also nur die markierten Entities und
/// nicht die Welt: der Boden, die ungetaggte Wand aus `maps.ron` und jedes nicht hakbare
/// Rasterhaus fallen schon im ECS heraus und kosten keinen Vergleich
/// (`docs/lessons/performance.md`, Regel 1). Wie viele es sind, steht in [`GizmoZaehler`]
/// und im Log.
pub fn anker_zeichnen(
    mut gizmos: Gizmos,
    mut zaehler: ResMut<GizmoZaehler>,
    flaechen: Query<(&GlobalTransform, Option<&Koerper>, Option<&Bauklotz>), With<Ankerflaeche>>,
) {
    let mut gezeichnet = 0usize;
    for (transform, koerper, klotz) in &flaechen {
        let Some(halb_m) = ankermass(koerper, klotz) else {
            continue;
        };
        anker_umranden(&mut gizmos, transform.translation(), transform.rotation(), halb_m);
        gezeichnet += 1;
    }
    // Nur bei Aenderung schreiben: ein `ResMut`, in das jeden Frame derselbe Wert faellt,
    // loest jeden Frame Aenderungserkennung aus (§11 „nichts aendert sich pro Frame").
    if zaehler.anker != gezeichnet {
        info!("Gizmos: {gezeichnet} Ankerflaechen umrandet");
        zaehler.anker = gezeichnet;
    }
}

/// Die **halbe** Kantenlaenge, die umrandet wird — oder `None`, wenn die Entity keine Form
/// hat.
///
/// [`Koerper`] gewinnt gegen [`Bauklotz`], und das ist Absicht: `Koerper::halb_m` ist die
/// Huelle, gegen die der Haken wirklich prueft, `Bauklotz::groesse` nur die, die `render`
/// zu Dreiecken macht. Laufen die beiden je auseinander, soll das Bild die **Hakenwahrheit**
/// zeigen und nicht die huebschere Form. `Bauklotz` fuehrt die **ganze** Kante, `Koerper`
/// die halbe (`src/shared/bau.rs`) — der Faktor 2 sitzt genau hier.
pub fn ankermass(koerper: Option<&Koerper>, klotz: Option<&Bauklotz>) -> Option<Vec3> {
    koerper
        .map(|k| k.halb_m)
        .or_else(|| klotz.map(|b| b.groesse * 0.5))
}

/// Zeichnet die zwoelf Kanten einer Ankerflaeche.
///
/// Generisch ueber die Gizmo-Gruppe, damit ein Test die Linien in einen eigenen
/// [`GizmoBuffer`] zeichnen und **nachzaehlen** kann, ohne eine App zu bauen.
pub fn anker_umranden<C, K>(
    zeichner: &mut GizmoBuffer<C, K>,
    mitte_m: Vec3,
    drehung: Quat,
    halb_m: Vec3,
) where
    C: GizmoConfigGroup,
    K: 'static + Send + Sync,
{
    // `cube` skaliert einen Einheitswuerfel — also die GANZE Kante in die Skalierung.
    zeichner.cube(
        Transform { translation: mitte_m, rotation: drehung, scale: halb_m * 2.0 },
        farbe(ZYAN),
    );
}

// ---------------------------------------------------------------------------
// 2. Massstab — wie gross, wie weit?
// ---------------------------------------------------------------------------

/// Bodenraster und Achsenkreuz im Ursprung.
///
/// Beruehrt **keine einzige Entity**: beides haengt nur am Weltursprung.
pub fn massstab_zeichnen(mut raster: Gizmos<RasterGizmos>, mut kreuz: Gizmos) {
    bodenraster(&mut raster);
    achsenkreuz(&mut kreuz);
}

/// Das Raster in der XZ-Ebene.
pub fn bodenraster<C, K>(zeichner: &mut GizmoBuffer<C, K>)
where
    C: GizmoConfigGroup,
    K: 'static + Send + Sync,
{
    // `grid` legt sein Gitter in die XY-Ebene (`bevy_gizmos-0.19.0/src/grid.rs:199-201`);
    // die Vierteldrehung um X kippt es auf den Boden.
    zeichner
        .grid(
            Isometry3d::new(Vec3::Y * RASTER_HOEHE_M, Quat::from_rotation_x(-FRAC_PI_2)),
            UVec2::splat(RASTER_ZELLEN),
            Vec2::splat(RASTER_ZELLE_M),
            farbe(RASTER),
        )
        .outer_edges();
}

/// Drei Achsen durch den Ursprung: die positive Haelfte mit Spitze, die negative gedaempft.
pub fn achsenkreuz<C, K>(zeichner: &mut GizmoBuffer<C, K>)
where
    C: GizmoConfigGroup,
    K: 'static + Send + Sync,
{
    for (richtung, ton) in
        [(Vec3::X, ACHSE_X), (Vec3::Y, ACHSE_Y), (Vec3::Z, ACHSE_Z)]
    {
        let arm = richtung * ACHSE_M;
        // Die negative Haelfte zuerst und dunkler — sie sagt „hier geht es weiter", ohne
        // den Blick vom Vorzeichen abzulenken. Fuer −Z ist das die Blickrichtung der
        // Kamera (`docs/konventionen.md`: yaw = 0 heisst Blick nach −Z), also genau die
        // Linie, an der man auf einem Bild die Tiefe abliest.
        zeichner.line(-arm, Vec3::ZERO, gedaempft(ton, HALB_HELL));
        zeichner.arrow(Vec3::ZERO, arm, farbe(ton));
    }
}

// ---------------------------------------------------------------------------
// 3. Spieler — wo steht wer?
// ---------------------------------------------------------------------------

/// Markiert **jeden** Spieler: Huelle, Fusskreuz, Mast.
///
/// Kein `.single()` und kein `With<LocalPlayer>` — jeder Spieler ist einer von vielen
/// (`docs/multiplayer.md` Regel 3). Uebersprungen wird nur, in wessen Huelle die Kamera
/// steckt: siehe [`huelle_enthaelt`].
pub fn spieler_zeichnen(
    mut gizmos: Gizmos,
    mut zaehler: ResMut<GizmoZaehler>,
    daten: Res<GameData>,
    spieler: Query<&GlobalTransform, With<PlayerId>>,
    kameras: Query<&GlobalTransform, With<Camera3d>>,
) {
    let s = &daten.spiel.spieler;
    let mut gezeichnet = 0usize;
    for transform in &spieler {
        let fuesse_m = transform.translation();
        let (mitte_m, halb_m) = spielerhuelle(fuesse_m, s.hoehe_m, s.radius_m);
        if kameras.iter().any(|k| huelle_enthaelt(mitte_m, halb_m, k.translation())) {
            continue;
        }
        spieler_markieren(&mut gizmos, fuesse_m, mitte_m, halb_m);
        gezeichnet += 1;
    }
    if zaehler.spieler != gezeichnet {
        info!("Gizmos: {gezeichnet} Spieler markiert");
        zaehler.spieler = gezeichnet;
    }
}

/// Ob ein Punkt in einer achsenparallelen Huelle liegt.
///
/// Die Kamera haengt heute als Kind am lokalen Spieler und sitzt damit **in** seiner Kapsel
/// (`src/render/mod.rs::kamera_anhaengen`, Augenhoehe 1,6 m bei 1,8 m Koerperhoehe). Wuerde
/// man sie trotzdem zeichnen, legte sich die eigene Huelle als Gitter ueber das ganze Bild:
/// 0,35 m vor einem 60-Grad-Objektiv ist eine Kante bildfuellend.
///
/// Geprueft wird **die Geometrie und nicht der `LocalPlayer`-Marker**. Der Marker waere
/// heute dieselbe Antwort und morgen die falsche: sobald es eine dritte Person oder eine
/// freie Kamera gibt, gehoert die eigene Kapsel wieder ins Bild — und dann stimmt diese
/// Regel weiter, ohne dass jemand daran denken muss.
pub fn huelle_enthaelt(mitte_m: Vec3, halb_m: Vec3, punkt_m: Vec3) -> bool {
    let abstand = (punkt_m - mitte_m).abs();
    abstand.x <= halb_m.x && abstand.y <= halb_m.y && abstand.z <= halb_m.z
}

/// Huelle, Fusskreuz und Mast eines Spielers.
pub fn spieler_markieren<C, K>(
    zeichner: &mut GizmoBuffer<C, K>,
    fuesse_m: Vec3,
    mitte_m: Vec3,
    halb_m: Vec3,
) where
    C: GizmoConfigGroup,
    K: 'static + Send + Sync,
{
    let ton = farbe(SPIELER);
    zeichner.cube(
        Transform { translation: mitte_m, rotation: Quat::IDENTITY, scale: halb_m * 2.0 },
        ton,
    );
    // Das Fusskreuz sitzt im Ursprung des Modells, und der liegt zwischen den Fuessen
    // (`docs/konventionen.md`) — es markiert also den Punkt, den `warp` setzt und den ein
    // `assert hoehe` misst.
    zeichner.cross(Isometry3d::from_translation(fuesse_m), halb_m.x, ton);
    zeichner.line(fuesse_m, fuesse_m + Vec3::Y * (halb_m.y * 2.0 + MAST_M), ton);
}

// ---------------------------------------------------------------------------
// Farbhilfen
// ---------------------------------------------------------------------------

fn farbe(rgb: [f32; 3]) -> Color {
    Color::linear_rgb(rgb[0], rgb[1], rgb[2])
}

fn gedaempft(rgb: [f32; 3], anteil: f32) -> Color {
    Color::linear_rgb(rgb[0] * anteil, rgb[1] * anteil, rgb[2] * anteil)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ein Puffer, in den gezeichnet wird, ohne dass eine App laeuft — `Gizmos` ist nur ein
    /// `SystemParam` um genau diesen Typ herum (`bevy_gizmos-0.19.0/src/gizmos.rs:155-175`).
    fn puffer() -> GizmoBuffer<DefaultGizmoConfigGroup, ()> {
        GizmoBuffer::new()
    }

    /// Alle **echten** Punkte eines Puffers.
    ///
    /// Bevy schliesst jeden Linienzug mit einem `Vec3::NAN` ab und faerbt ihn mit
    /// `LinearRgba::NAN` — das ist der Trenner zwischen zwei Streifen, kein Punkt
    /// (`bevy_gizmos-0.19.0/src/gizmos.rs:939-942` und `:501-515`). Wer ihn mitzaehlt,
    /// zaehlt eine Ecke zu viel und vergleicht eine Farbe, die keine ist.
    fn punkte(b: &GizmoBuffer<DefaultGizmoConfigGroup, ()>) -> Vec<Vec3> {
        b.list_positions
            .iter()
            .chain(b.strip_positions.iter())
            .copied()
            .filter(|p| p.is_finite())
            .collect()
    }

    fn hoechster_punkt(b: &GizmoBuffer<DefaultGizmoConfigGroup, ()>) -> f32 {
        punkte(b).iter().fold(f32::MIN, |m, p| m.max(p.y))
    }

    #[test]
    fn eine_ankerumrandung_ist_zyan_und_hat_zwoelf_kanten() {
        let mut b = puffer();
        anker_umranden(&mut b, Vec3::new(10.0, 5.75, -28.0), Quat::IDENTITY, Vec3::splat(5.0));

        // Ein Quader hat zwoelf Kanten: acht als zwei Ringe zu je fuenf Punkten im
        // Streifen (plus dem NAN-Trenner), vier als drei Paare in der Liste.
        assert_eq!(b.strip_positions.len(), 11, "zwei Ringe zu fuenf Punkten plus Trenner");
        assert_eq!(b.list_positions.len(), 6, "drei Verbindungen zu je zwei Punkten");
        assert_eq!(punkte(&b).len(), 16, "sechzehn echte Ecken");

        // Die Farbe ist keine Geschmacksfrage (docs/konventionen.md §3).
        let zyan = farbe(ZYAN).to_linear();
        let farben: Vec<_> = b
            .strip_colors
            .iter()
            .chain(b.list_colors.iter())
            .filter(|c| c.red.is_finite())
            .collect();
        assert!(!farben.is_empty(), "es wurde ueberhaupt nichts gefaerbt");
        for c in farben {
            assert_eq!(*c, zyan, "eine Ankerflaeche ist zyan, sonst nichts");
        }
    }

    #[test]
    fn die_umrandung_nimmt_die_ganze_kante_und_nicht_die_halbe() {
        // Der Faktor 2 zwischen `Koerper::halb_m` und `Bauklotz::groesse` ist die Falle,
        // die im Bild nicht auffaellt (`src/world/karte.rs`): eine doppelt so grosse
        // Umrandung sieht aus wie ein grosszuegiger Rahmen und ist eine Luege.
        let mut b = puffer();
        anker_umranden(&mut b, Vec3::ZERO, Quat::IDENTITY, Vec3::new(4.0, 6.0, 4.0));
        let hoch = hoechster_punkt(&b);
        assert!((hoch - 6.0).abs() < 1e-5, "Oberkante bei {hoch} statt 6,0 m");
    }

    #[test]
    fn koerper_gewinnt_gegen_bauklotz_und_beide_gegen_nichts() {
        let koerper = Koerper { halb_m: Vec3::splat(2.0), maske: crate::shared::Maske::HAKBAR };
        let klotz = Bauklotz { groesse: Vec3::splat(10.0), farbe: [0.4, 0.4, 0.4] };
        assert_eq!(ankermass(Some(&koerper), Some(&klotz)), Some(Vec3::splat(2.0)));
        assert_eq!(ankermass(None, Some(&klotz)), Some(Vec3::splat(5.0)));
        assert_eq!(ankermass(Some(&koerper), None), Some(Vec3::splat(2.0)));
        // Eine Ankerflaeche ohne jede Form wird nicht geraten, sondern ausgelassen.
        assert_eq!(ankermass(None, None), None);
    }

    #[test]
    fn das_achsenkreuz_benutzt_keine_der_drei_signalfarben() {
        // Zyan, Bernstein und Karminrot sind ausschliesslich fuer Gameplay reserviert
        // (docs/konventionen.md §3). Der Test prueft die Eigenschaft, nicht den Zahlenwert:
        // eine Farbe ist verdaechtig, wenn sie einem der drei Toene nahe kommt.
        for ton in [ACHSE_X, ACHSE_Y, ACHSE_Z, RASTER, SPIELER] {
            let [r, g, b] = ton;
            assert!(!(r < 0.4 && g > 0.5 && b > 0.5), "{ton:?} ist Zyan");
            assert!(!(r > 0.6 && g > 0.3 && g < 0.75 && b < 0.3), "{ton:?} ist Bernstein");
            assert!(!(r > 0.5 && g < 0.25 && b < 0.25), "{ton:?} ist Karminrot");
        }
        // Und Zyan bleibt der Ankerflaeche vorbehalten.
        assert!(ZYAN[0] < 0.4 && ZYAN[1] > 0.5 && ZYAN[2] > 0.5);
    }

    #[test]
    fn das_bodenraster_liegt_flach_und_nicht_senkrecht() {
        // `grid` zeichnet in die XY-Ebene; ohne die Vierteldrehung stuende das Raster wie
        // eine Wand vor der Kamera — und zwar so, dass es auf einem Bild wie Absicht
        // aussieht.
        let mut b = puffer();
        bodenraster(&mut b);
        let punkte = punkte(&b);
        assert!(!punkte.is_empty(), "das Raster hat keine einzige Linie gezeichnet");
        let halb = RASTER_ZELLEN as f32 * RASTER_ZELLE_M * 0.5;
        for p in punkte {
            assert!((p.y - RASTER_HOEHE_M).abs() < 1e-4, "Rasterpunkt bei y = {}", p.y);
            assert!(p.x.abs() <= halb + 1e-3 && p.z.abs() <= halb + 1e-3, "{p:?} liegt draussen");
        }
    }

    #[test]
    fn der_schalter_liest_nur_klare_ja_worte() {
        for ja in ["1", "an", "ja", "true", " 1 "] {
            assert!(schalter_aus_text(Some(ja)), "{ja:?} sollte einschalten");
        }
        for nein in ["0", "", "aus", "nein", "false", "vielleicht"] {
            assert!(!schalter_aus_text(Some(nein)), "{nein:?} sollte nicht einschalten");
        }
        assert!(!schalter_aus_text(None), "ohne Variable bleibt es aus");
    }

    #[test]
    fn die_kamera_steckt_in_der_kapsel_des_spielers_der_sie_traegt() {
        // Die Zahlen sind die aus `game.ron`: Koerper 1,8 m, Radius 0,35 m, Auge 1,6 m.
        let (mitte, halb) = spielerhuelle(Vec3::new(6.0, 20.0, 45.0), 1.8, 0.35);
        let auge = Vec3::new(6.0, 20.0 + 1.6, 45.0);
        assert!(huelle_enthaelt(mitte, halb, auge), "das eigene Auge sitzt in der Kapsel");

        // Ein Mitspieler zwei Meter weiter ist nicht dieselbe Kapsel.
        let (mitte2, halb2) = spielerhuelle(Vec3::new(8.0, 20.0, 45.0), 1.8, 0.35);
        assert!(!huelle_enthaelt(mitte2, halb2, auge), "der Nachbar wird gezeichnet");
        // Und eine freie Kamera ueber dem Spieler ebenfalls nicht.
        assert!(!huelle_enthaelt(mitte, halb, Vec3::new(6.0, 30.0, 45.0)));
    }

    #[test]
    fn ein_spieler_bekommt_huelle_kreuz_und_mast() {
        let mut b = puffer();
        let fuesse = Vec3::new(0.0, 2.0, 0.0);
        let (mitte, halb) = spielerhuelle(fuesse, 1.8, 0.35);
        spieler_markieren(&mut b, fuesse, mitte, halb);
        assert!(punkte(&b).len() > 16, "Huelle, Kreuz und Mast, nicht nur ein Strich");
        // Der Mast ragt MAST_M ueber den Kopf: 2,0 + 1,8 + 3,0.
        let hoch = hoechster_punkt(&b);
        assert!((hoch - (2.0 + 1.8 + MAST_M)).abs() < 1e-4, "Mastspitze bei {hoch}");
    }
}
