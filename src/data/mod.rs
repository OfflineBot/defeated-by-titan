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
        }
    }

    /// Ein Titan-Typ ueber seinen logischen Namen. `None` heisst: steht nicht in der RON —
    /// und der Aufrufer meldet das laut, statt einen Ersatztitanen zu erfinden.
    pub fn titan(&self, art: &str) -> Option<&TitanArt> {
        self.titanen.arten.get(art)
    }

    pub fn modell(&self, name: &str) -> Option<&Modell> {
        self.art.models.get(name)
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
pub struct Spiel {
    pub simulation_hz: f64,
    pub schwerkraft_m_s2: f32,
    pub spieler: SpielerWerte,
    pub vector: VectorWerte,
    pub kamera: KameraWerte,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpielerWerte {
    pub hoehe_m: f32,
    pub radius_m: f32,
    pub laufen_m_s: f32,
    pub sprung_m_s: f32,
    pub augenhoehe_m: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VectorWerte {
    pub hakenreichweite_m: f32,
    pub hakenflug_m_s: f32,
    pub seilzug_m_s: f32,
    pub seil_min_m: f32,
    pub gas_tank: f32,
    pub gas_boost_pro_s: f32,
    pub gas_einholen_pro_s: f32,
    pub boost_m_s2: f32,
    pub tempo_max_m_s: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KameraWerte {
    pub sicht_grad: f32,
    pub maus_grad_pro_punkt: f32,
    pub pitch_grenze_grad: f32,
    pub glaetten_halbwertszeit_s: f32,
}

// ---------------------------------------------------------------------------
// gear.ron
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct Gear {
    pub klingen: KlingenWerte,
    pub nachschub: NachschubWerte,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KlingenWerte {
    pub paare_start: u8,
    pub abnutzung_pro_treffer: f32,
    pub schaden_pro_m_s: f32,
    pub mindesttempo_m_s: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NachschubWerte {
    pub gas_pro_s: f32,
    pub reichweite_m: f32,
}

// ---------------------------------------------------------------------------
// titan.ron
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct Titanen {
    pub arten: BTreeMap<String, TitanArt>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TitanArt {
    pub hoehe_m: f32,
    pub tempo_m_s: f32,
    pub cortex_radius_m: f32,
    pub regeneration_pro_s: f32,
    /// Ausholphase jedes Angriffs. **Mindestens 0,4 s** — Bibel, Pfeiler P4
    /// (Lesbarkeit vor Realismus). `tests/data.rs` faellt um, wenn eine Art darunter liegt.
    pub ausholphase_s: f32,
    pub modell: String,
}

// ---------------------------------------------------------------------------
// art.ron — die Registratur
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct Art {
    pub models: BTreeMap<String, Modell>,
}

#[derive(Debug, Clone, Deserialize)]
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
pub struct Missionen {
    pub vorlagen: BTreeMap<String, Missionsvorlage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Missionsvorlage {
    pub name: String,
    pub map: String,
    /// Der Missionsbogen dauert 5–7 min (Bibel 5, Aenderung 10).
    pub dauer_ziel_s: f32,
    pub wellen: Vec<Welle>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Welle {
    pub bei_s: f32,
    pub art: String,
    pub anzahl: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Traits {
    pub eintraege: BTreeMap<String, TraitWert>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TraitWert {
    pub name: String,
    pub kosten: u32,
    pub beschreibung: String,
}
