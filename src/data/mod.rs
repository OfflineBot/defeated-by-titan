//! data — die RON-Dateien laden. **Laeuft vor allem anderen.**
//!
//! > **Zahlen gehoeren in RON, nicht in Rust.** Ein neuer Titan-Typ, eine Klingenstufe, eine
//! > Gas-Kostenzahl: Datei-Arbeit, kein Rust. Im Code stehen nur *Einheiten* und *Mechanik*
//! > (`prompts/init.md` §4).
//!
//! Warum das nicht optional ist: **Balancing ist die Arbeit, die am haeufigsten passiert.**
//! Wenn sie einen Rebuild braucht, passiert sie nicht. Und ein anderer Agent kann eine
//! RON-Zeile aendern, ohne diesen Code zu verstehen.
//!
//! **Kein `serde(default)` fuer Spielwerte.** Ein fehlender Wert soll beim Laden krachen,
//! nicht still eine Null einsetzen — sonst sucht man den Bug im Code, waehrend er in der
//! Datei sitzt. Deshalb wird hier auch **synchron beim Aufbau** geladen und nicht ueber den
//! `AssetServer`: ein Fehler soll **beim Start** laut sein, mit Dateiname und Zeile, und
//! nicht drei Systeme spaeter als leerer Bildschirm (§9d).
//!
//! **Und `#[serde(deny_unknown_fields)]` auf jedem Typ hier.** Das Weglassen krachte schon
//! immer; das *Hinzufuegen* nicht — serde ueberliest ein unbekanntes Feld stillschweigend.
//! Gemessen am 2026-08-09: ein `erfunden_m: 42.0` in `massstab.ron` und ein
//! `gewicht_kg: 70.0` unter `game.ron: spieler` luden beide ohne ein Wort. Das ist genau die
//! Falle fuer die Datei, in die von jetzt an Zahlen des Users nachgetragen werden: ein
//! Nachtrag auf der falschen Verschachtelungsstufe verschwindet lautlos, und die Zahl, die
//! man eingetragen hat, ist im Spiel nicht da.
//!
//! **Dies ist die einzige Stelle, die Dateinamen kennt.** Alle anderen fragen nach dem
//! logischen Namen; `tools/normen.py` faellt um, wenn irgendwo sonst ein Pfad im Code steht.

use bevy::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameData::laden(&wurzel()));
    }
}

/// Wo `assets/` liegt.
///
/// **`cargo run`, nie das nackte Binary**: das Binary sucht `assets/` relativ zum
/// Arbeitsverzeichnis und findet nichts — leere Welt, keine Fehlermeldung, sieht exakt wie
/// ein Render-Bug aus (`prompts/init.md` §3). Damit ein `cargo test` aus `tests/` trotzdem
/// findet, wird zusaetzlich das Crate-Verzeichnis geprueft.
fn wurzel() -> PathBuf {
    let hier = PathBuf::from("assets/data");
    if hier.is_dir() {
        return hier;
    }
    let beim_crate = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/data");
    if beim_crate.is_dir() {
        return beim_crate;
    }
    panic!(
        "assets/data/ nicht gefunden — weder unter {:?} noch unter {:?}.\n\
         Starte mit `cargo run`, nicht mit dem nackten Binary aus target/debug/ \
         (prompts/init.md §3).",
        hier.canonicalize().unwrap_or(hier.clone()),
        beim_crate
    );
}

/// Alles, was aus `assets/data/` kommt. Eine Resource, viele Leser, **kein Schreiber**.
#[derive(Resource, Debug, Clone)]
pub struct GameData {
    pub spiel: Spiel,
    pub gear: Gear,
    pub titanen: Titanen,
    pub art: Art,
    pub missionen: Missionen,
    pub traits: Traits,
    pub karten: Karten,
    /// Die **eine Wahrheit ueber Groessen**, vom User vorgegeben. Alle anderen Dateien
    /// spiegeln nur; `tests/data.rs` faellt um, sobald eine von hier abweicht.
    pub massstab: Massstab,
}

impl GameData {
    pub fn laden(ordner: &Path) -> Self {
        GameData {
            spiel: lies(ordner, "game.ron"),
            gear: lies(ordner, "gear.ron"),
            titanen: lies(ordner, "titan.ron"),
            art: lies(ordner, "art.ron"),
            missionen: lies(ordner, "missions.ron"),
            traits: lies(ordner, "traits.ron"),
            karten: lies(ordner, "maps.ron"),
            massstab: lies(ordner, "massstab.ron"),
        }
    }

    /// Eine Karte ueber ihren logischen Namen. `None` heisst: steht nicht in `maps.ron` —
    /// und der Aufrufer meldet das laut, statt eine leere Welt zu bauen.
    pub fn karte(&self, id: &str) -> Option<&Karte> {
        self.karten.karten.get(id)
    }

    /// Die Karte, die beim Start gebaut wird (`maps.ron: aktuell`).
    /// `tests/data.rs` haelt fest, dass es sie gibt.
    pub fn aktuelle_karte(&self) -> Option<&Karte> {
        self.karte(&self.karten.aktuell)
    }

    /// Eine Farbe aus dem einen Farbatlas. `None` heisst: der Schluessel steht nicht in
    /// `palette` — kein stiller Ersatz, sonst rutscht irgendwann eine Signalfarbe hinein.
    pub fn farbe(&self, name: &str) -> Option<[f32; 3]> {
        self.karten.palette.get(name).map(|(r, g, b)| [*r, *g, *b])
    }

    /// Ein Titan-Typ ueber seinen logischen Namen. `None` heisst: steht nicht in der RON —
    /// und der Aufrufer meldet das laut, statt einen Ersatztitanen zu erfinden.
    pub fn titan(&self, art: &str) -> Option<&TitanArt> {
        self.titanen.arten.get(art)
    }

    pub fn modell(&self, name: &str) -> Option<&Modell> {
        self.art.models.get(name)
    }

    /// Eine Groessenklasse ueber ihren logischen Namen (`massstab.ron: titan.klassen`).
    /// `None` heisst: die Klasse steht nicht in der RON — `tests/data.rs` faengt das ab,
    /// bevor ein Titan mit Hoehe 0 im Boden steht.
    pub fn groessenklasse(&self, name: &str) -> Option<&Groessenklasse> {
        self.massstab.titan.klassen.get(name)
    }

    /// Die Hoehe einer Titanart. Sie steht **nicht** in `titan.ron` — dort steht nur die
    /// Klasse, und die Hoehe kommt aus `massstab.ron`. Eine Zahl, ein Ort.
    pub fn titan_hoehe_m(&self, art: &TitanArt) -> Option<f32> {
        self.groessenklasse(&art.groessenklasse).map(|k| k.hoehe_m)
    }

    /// Wo der Cortex sitzt — **die Meterangabe des Users**, nicht `hoehe_m * 0,89`.
    ///
    /// **Die einzige toedliche Trefferzone** (`F-030`). Bis 2026-08-09 wurde sie hier aus
    /// dem Anteil gerechnet; das war bequem und falsch: der User nennt fuenf Cortexhoehen in
    /// Metern, und *eine direkte Meterangabe schlaegt jede Ableitung*. Die Rechnung lag bei
    /// der kleinen Klasse 4 cm daneben. Der Anteil ist jetzt das, was er beim User ist —
    /// eine Regel, gegen die `tests/data.rs` die fuenf Zahlen prueft.
    pub fn titan_cortex_hoehe_m(&self, art: &TitanArt) -> Option<f32> {
        self.groessenklasse(&art.groessenklasse).map(|k| k.cortex_m)
    }

    /// Die groesste Kopfhoehe, die die Kopfregel des Users fuer diese Art zulaesst
    /// (`hoehe_m * massstab.titan.kopf_anteil_max`, also 1/9 der Koerperhoehe).
    ///
    /// Sie ist die **geometrische Obergrenze fuer `cortex_radius_m`**: eine Trefferzone,
    /// deren Durchmesser groesser ist als der ganze Kopf, kann kein Halsansatz sein. Vor
    /// diesem Waechter trugen `scuttler` (0,80 m Cortex an 0,47 m Kopf) und `weaver`
    /// (0,90 m) genau das.
    pub fn titan_kopf_hoehe_max_m(&self, art: &TitanArt) -> Option<f32> {
        self.titan_hoehe_m(art).map(|h| h * self.massstab.titan.kopf_anteil_max)
    }
}

/// Liest eine RON-Datei oder bricht mit einer Meldung ab, die den Fehler **in der Datei**
/// zeigt statt im Code.
fn lies<T: for<'a> Deserialize<'a>>(ordner: &Path, datei: &str) -> T {
    let pfad = ordner.join(datei);
    let text = std::fs::read_to_string(&pfad).unwrap_or_else(|e| {
        panic!("{}: laesst sich nicht lesen — {e}", pfad.display())
    });
    ron::de::from_str(&text).unwrap_or_else(|e| {
        // ron nennt Zeile und Spalte; genau die will man sehen.
        panic!("{}: kein gueltiges RON — {e}", pfad.display())
    })
}

// ---------------------------------------------------------------------------
// game.ron — Tuning: Vector Gear, Kamera, Physik
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spiel {
    pub simulation_hz: f64,
    pub schwerkraft_m_s2: f32,
    pub spieler: SpielerWerte,
    pub vector: VectorWerte,
    pub kamera: KameraWerte,
    pub welt: WeltWerte,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpielerWerte {
    pub hoehe_m: f32,
    pub radius_m: f32,
    pub laufen_m_s: f32,
    pub sprung_m_s: f32,
    pub augenhoehe_m: f32,
    /// Groesster Weg pro Teilschritt des Integrators. Muss echt kleiner sein als
    /// [`WeltWerte::wand_min_m`], sonst tunnelt der Spieler durch die duennste Wand.
    pub schritt_max_m: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorWerte {
    pub hakenreichweite_m: f32,
    pub hakenflug_m_s: f32,
    pub haken_ruecklauf_m_s: f32,
    pub seilzug_m_s: f32,
    pub seil_min_m: f32,
    /// Gauss-Seidel-Durchlaeufe ueber beide Seilzwaenge (`shared::seil::seil_schritt`).
    pub seil_durchlaeufe: u32,
    pub gas_tank: f32,
    pub gas_boost_pro_s: f32,
    pub gas_einholen_pro_s: f32,
    /// Wer zahlt zuerst, wenn der Tank fuer beides nicht reicht. **Eine
    /// Spielwertentscheidung**, deshalb hier und nicht als `if` in `vector/gas.rs`.
    pub gas_rangfolge: Vec<Gasverbraucher>,
    pub boost_m_s2: f32,
    pub tempo_max_m_s: f32,
}

/// Wer Gas verbraucht. Eigener Typ statt `String`, damit ein Tippfehler in der RON **beim
/// Laden kracht** statt still einen Verbraucher zu verlieren.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Gasverbraucher {
    /// `F-007` Gas-Boost.
    Boost,
    /// `F-005` Reel-In.
    Einholen,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KameraWerte {
    /// Sichtfeld im **Bodenkampf** — die Basis, nicht die Obergrenze. Muss im Fenster
    /// `massstab.ron: kamera.sicht_boden_min_grad ..= sicht_boden_max_grad` liegen.
    pub sicht_grad: f32,
    /// Sichtfeld bei `vector.tempo_max_m_s`. `F-017` interpoliert spaeter zwischen beiden;
    /// die **Kurve** gehoert in den Code, die beiden **Enden** in die RON.
    pub sicht_tempo_grad: f32,
    pub maus_grad_pro_punkt: f32,
    pub pitch_grenze_grad: f32,
    pub glaetten_halbwertszeit_s: f32,
}

/// Was die Welt selbst kostet: raeumlicher Index und Kollision.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeltWerte {
    /// Kantenlaenge einer Gitterzelle (`T-036a`). **Ungemessen** —
    /// `docs/lessons/performance.md`, `docs/FRAGEN.md` Q-014. (Stand hier bis 2026-08-09 als
    /// Q-013; Q-013 ist die Frage nach der maximalen Seillaenge.)
    pub zelle_m: f32,
    /// Halbe Kantenlaenge des Gitters. Muss Karte **und** Hakenreichweite abdecken.
    pub halbe_ausdehnung_m: f32,
    /// Ab wie vielen belegten Zellen ein Koerper in die lineare Grosskoerper-Liste geht.
    pub grosskoerper_zellen: u32,
    /// Die duennste zulaessige Wand. Kalibriert [`SpielerWerte::schritt_max_m`].
    pub wand_min_m: f32,
    /// Kollisionshaut gegen Zittern im Kontakt.
    pub kollision_haut_m: f32,
}

// ---------------------------------------------------------------------------
// maps.ron — die Stadt als Daten (E13, F-003, Q-010)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Karten {
    /// Logischer Name der Karte, die beim Start gebaut wird.
    pub aktuell: String,
    /// Der eine Farbatlas, lineares RGB. Kein RGB-Tripel je Klotz — sonst rutscht
    /// irgendwann eine Signalfarbe hinein (`docs/konventionen.md`).
    pub palette: BTreeMap<String, (f32, f32, f32)>,
    pub karten: BTreeMap<String, Karte>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Karte {
    pub name: String,
    /// Kantenlaenge in X und Z, in Metern.
    pub groesse_m: (f32, f32),
    /// Seed fuer `shared::Wuerfel`. Teil des Zustands, **nie** `rand::random()`.
    pub seed: u64,
    pub raster: Raster,
    /// Ausdruecklich gesetzte Quader. Sie gewinnen gegen das Raster.
    pub kloetze: Vec<Kartenklotz>,
}

/// Die Regel, aus der `world` deterministisch Gebaeude erzeugt.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Raster {
    pub block_m: f32,
    pub gasse_m: f32,
    pub hoehe_min_m: f32,
    pub hoehe_max_m: f32,
    /// Anteil bebauter Rasterplaetze, 0..1.
    pub dichte: f32,
    /// Anteil hakbarer Gebaeude, 0..1.
    pub hakbar_anteil: f32,
    /// Radius um den Ursprung, der frei bleibt.
    pub frei_radius_m: f32,
    /// Erlaubte Farbschluessel aus [`Karten::palette`].
    pub farben: Vec<String>,
}

/// Ein ausdruecklich gesetzter Quader. Ursprung in der Mitte, wie `shared::Bauklotz`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kartenklotz {
    pub mitte_m: (f32, f32, f32),
    /// Volle Kantenlaenge. Der Index haelt intern die halbe.
    pub groesse_m: (f32, f32, f32),
    pub farbe: String,
    /// Bekommt `shared::Ankerflaeche` und `Maske::HAKBAR` (`F-003`).
    pub hakbar: bool,
    /// Stoppt einen Koerper — `Maske::FEST`.
    pub fest: bool,
    /// `false` = Wohnbebauung und muss unter `massstab.ron: architektur.hoehen_m`
    /// `haus_gross` (11,5 m) bleiben. `true` = Kirche, Wachturm, Baum, Mauer — **sie tragen
    /// die Vertikale** und duerfen darueber.
    ///
    /// Ohne dieses Feld galt die Flachheitsregel nur fuer `raster`, und die ausdruecklich
    /// gesetzten Quader der Graubox standen mit 12, 14 und 18 m Firsthoehe ausserhalb des
    /// Waechters, der die Stadt flach halten soll.
    pub sonderbau: bool,
}

// ---------------------------------------------------------------------------
// gear.ron
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gear {
    pub klingen: KlingenWerte,
    pub nachschub: NachschubWerte,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KlingenWerte {
    pub paare_start: u8,
    pub abnutzung_pro_treffer: f32,
    pub schaden_pro_m_s: f32,
    pub mindesttempo_m_s: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NachschubWerte {
    pub gas_pro_s: f32,
    pub reichweite_m: f32,
}

// ---------------------------------------------------------------------------
// titan.ron
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Titanen {
    pub arten: BTreeMap<String, TitanArt>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitanArt {
    /// Schluessel in `massstab.ron: titan.klassen` (`F-064`). **Keine Hoehe je Art** — Hoehe
    /// und Cortexhoehe kommen ueber [`GameData::titan_hoehe_m`] und
    /// [`GameData::titan_cortex_hoehe_m`] aus dem Massstab, damit sie nicht auseinanderlaufen.
    ///
    /// Hiess bis 2026-08-09 `groesse`; ein Feldname auf „-groesse" liest sich wie eine
    /// Laenge (`docs/konventionen.md` §5), hier steht aber ein Schluessel.
    pub groessenklasse: String,
    pub tempo_m_s: f32,
    pub cortex_radius_m: f32,
    pub regeneration_pro_s: f32,
    /// Ausholphase jedes Angriffs. **Mindestens 0,4 s** — Bibel, Pfeiler P4
    /// (Lesbarkeit vor Realismus). `tests/data.rs` faellt um, wenn eine Art darunter liegt.
    pub ausholphase_s: f32,
    pub modell: String,
}

// ---------------------------------------------------------------------------
// massstab.ron — die eine Wahrheit ueber Groessen (vom User vorgegeben, 2026-08-09)
// ---------------------------------------------------------------------------

/// Die Groessentabelle des Users als Typ.
///
/// **Diese Zahlen sind nicht ungetunt, sie sind vorgegeben.** Alles andere in `assets/data/`
/// darf jeder aendern; wer hier etwas aendert, aendert eine Entscheidung des Users. Die
/// Vorrangregel dazu: **eine direkte Meterangabe des Users schlaegt jede Ableitung** — auch
/// die Umrechnung aus dem Backlog (`docs/FRAGEN.md` Q-002).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Massstab {
    /// Die Welt ist **bewusst nicht einheitlich skaliert**: Architektur 1,0, Titanen 1,4,
    /// Mauern 2,4. Ein Titan wirkt groesser als „realistisch", eine Mauer monumental — das
    /// ist die Bildsprache, keine Ungenauigkeit. Wer die drei angleicht, zerstoert sie.
    pub architektur_faktor: f32,
    pub titan_faktor: f32,
    pub mauer_faktor: f32,
    pub referenz: Referenz,
    pub architektur: ArchitekturMasse,
    pub titan: TitanMassstab,
    pub mauer: MauerMasse,
    pub kamera: KameraMassstab,
    pub vector: VectorMassstab,
}

/// Woran das Auge alles andere misst.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Referenz {
    /// „Kapsel exakt pruefen!" — `game.ron: spieler.hoehe_m` muss **exakt** das sein.
    pub mensch_hoehe_m: f32,
    pub tuer_hoehe_m: f32,
    /// Fenster fuer `maps.ron: raster.gasse_m`. „eng halten."
    pub strasse_breite_min_m: f32,
    pub strasse_breite_max_m: f32,
    /// 1/7,5. Der **Vergleichswert** zu [`TitanMassstab::kopf_anteil_min`]: der Titankopf
    /// ist relativ kleiner, und genau daran liest das Auge „riesig" statt „nah".
    pub kopf_anteil_mensch: f32,
}

/// Bauhoehen. Die Wohnbebauung ist absichtlich flach; die Vertikale kommt aus Mauer,
/// Kirche, Wachturm und Baeumen.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitekturMasse {
    /// Logischer Name -> Gesamthoehe in Metern.
    pub hoehen_m: BTreeMap<String, f32>,
    /// Logischer Name -> Traufhoehe (Oberkante Aussenwand, wo das Dach ansetzt).
    /// **Nur dort gefuellt, wo der User eine Zahl genannt hat** — jeder Schluessel muss auch
    /// in [`Self::hoehen_m`] stehen, `tests/data.rs` prueft das.
    pub traufen_m: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitanMassstab {
    /// Der Anteil der Koerperhoehe, bei dem der Cortex sitzt (0,89) — **eine Pruefregel,
    /// keine Quelle.** Die fuenf Cortexhoehen stehen als Meterangaben in
    /// [`Groessenklasse::cortex_m`]; dieser Wert ist das, was der User daneben geschrieben
    /// hat, und `tests/data.rs` haelt die fuenf damit im Fenster 0,88..0,90.
    ///
    /// Als Quelle dient er nur, wo der User keine Cortexhoehe genannt hat — heute genau
    /// einmal: beim Ashwalker (150 m ⇒ 133,5 m).
    pub cortex_anteil: f32,
    /// Kopfhoehe als Anteil der Koerperhoehe: 1/10 bis 1/9.
    pub kopf_anteil_min: f32,
    pub kopf_anteil_max: f32,
    /// Die fuenf Groessenklassen (`F-064`). `titan.ron` verweist mit `groesse` hierher.
    pub klassen: BTreeMap<String, Groessenklasse>,
    /// Der 150-m-Boss. **Ausserhalb der Klassen**: kein skalierter Gegnertyp, sondern ein
    /// Bauwerk mit Gesicht. Probe des Users: 150 − 120 = 30 m ueber der Mauer.
    pub ashwalker_hoehe_m: f32,
}

/// Eine Groessenklasse. Eigener Typ statt nackter `f32`, damit spaetere Klassenwerte
/// (Reichweite, Trefferpunkte, Schrittlaenge) hier landen und nicht je Art dupliziert werden.
///
/// **Beide Felder sind Meterangaben des Users** — `cortex_m` wird nicht aus `hoehe_m`
/// gerechnet. Der Anteil ([`TitanMassstab::cortex_anteil`]) prueft sie, statt sie zu erzeugen.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Groessenklasse {
    pub hoehe_m: f32,
    /// Hoehe des Cortex ueber dem Boden. Die **einzige toedliche Trefferzone** (`F-030`).
    pub cortex_m: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MauerMasse {
    pub hoehe_m: f32,
    pub dicke_oben_m: f32,
    /// Groesser als [`Self::dicke_oben_m`]: die Mauer ist angeschraegt.
    pub dicke_basis_m: f32,
    /// Zwischenstopp auf halber Hoehe. Ohne ihn ist die Mauerkrone mit 90 m Ankerreichweite
    /// von unten nicht erreichbar (120 > 90).
    pub plattform_hoehe_m: f32,
    /// **Skalenleiter.** Eine Steinreihe von 0,6 m und ein Band alle 15 m sind der Grund,
    /// warum eine 120-m-Wand gross aussieht statt grau.
    pub steinreihe_m: f32,
    pub baenderung_m: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KameraMassstab {
    /// Augenhoehe. `game.ron: spieler.augenhoehe_m` muss das sein.
    pub hoehe_m: f32,
    /// Fenster fuer `game.ron: kamera.sicht_grad` — **Bodenkampf**, „groesster Hebel".
    pub sicht_boden_min_grad: f32,
    pub sicht_boden_max_grad: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorMassstab {
    /// 90 m, direkt vom User. `game.ron: vector.hakenreichweite_m` muss das sein — und
    /// **nicht** die 112 m aus der Umrechnung (Q-002).
    pub ankerreichweite_m: f32,
    /// „x1,5 vs. Standard". **Wird nicht verrechnet**, solange der Bezug fehlt
    /// (`docs/FRAGEN.md` Q-018).
    pub tempo_faktor: f32,
}

// ---------------------------------------------------------------------------
// art.ron — die Registratur
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Art {
    pub models: BTreeMap<String, Modell>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Modell {
    /// Name der `.blend` ohne Endung. Der Auto-Export macht daraus die `.glb` (§7).
    pub blend: String,
    /// `false` ⇒ der **Platzhalter-Weg** aus Bevy-Primitiven. Beide Wege muessen jederzeit
    /// laufen und dieselbe Groesse, Hitbox und Skalierung haben — sonst ist das Umschalten
    /// kein Schalter, sondern ein Umbau.
    pub nutzen: bool,
    pub scale: f32,
    /// Nur bei Fremdmaterial gesetzt: URL · Datum · Lizenz · was es ersetzen soll.
    /// Damit ist die Ersetzungsliste ein `grep` (§7).
    pub herkunft: Option<String>,
}

// ---------------------------------------------------------------------------
// missions.ron / traits.ron — noch fast leer, aber vorhanden und geladen
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Missionen {
    pub vorlagen: BTreeMap<String, Missionsvorlage>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Missionsvorlage {
    pub name: String,
    pub map: String,
    /// Der Missionsbogen dauert 5–7 min (Bibel 5, Aenderung 10).
    pub dauer_ziel_s: f32,
    pub wellen: Vec<Welle>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Welle {
    pub bei_s: f32,
    pub art: String,
    pub anzahl: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Traits {
    pub eintraege: BTreeMap<String, TraitWert>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraitWert {
    pub name: String,
    pub kosten: u32,
    pub beschreibung: String,
}
