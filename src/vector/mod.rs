//! vector — DER KERN: Haken, Seil, Schwung, Gas, Boost, Wandlauf
//!
//! **Das Spiel steht und faellt mit diesem Gefuehl — nicht mit der Titanen-KI.**
//! Ein Spieler, der elegant durch die Stadt fliegt, ohne einen einzigen Titanen zu toeten,
//! muss Spass haben. Wenn das nicht funktioniert, funktioniert nichts (Bibel 2, Pfeiler P1).
//!
//! Deshalb das harte Gate: **kein Meta-System, bevor die Bewegung ueberzeugt.** Und deshalb
//! ist hier jede Zahl in `assets/data/game.ron` und keine im Code.
//!
//! Zwei **unabhaengig** steuerbare Haken (`F-001`), Pendelphysik bei zwei gesetzten Haken
//! (`F-004`), Reel-In (`F-005`), Swerve (`F-006`). Seilkraefte brauchen Wachen: eine
//! Normalisierung auf Laenge 0 erzeugt NaN, und NaN im `Transform` sieht aus wie
//! „der Spieler ist verschwunden" (§9d).
//!
//! **Stand:** die Naht steht, die Rechnungen fehlen. Fuenf Module, fuenf Dateien, fuenf
//! Auftraege — **die Registrierung hier ist bereits vollstaendig**, damit spaeter kein
//! Agent diese Datei anfassen muss und zwei Auftraege sich nicht um sie streiten
//! (`docs/schnittstelle.md`, Dateibesitz).
//!
//! | Datei | F-ID | schreibt |
//! |---|---|---|
//! | `zielen.rs` | `F-002`, `F-003` | `Zielpunkt` |
//! | `gas.rs` | `F-018` | `Gas`, `Gasfreigabe` |
//! | `haken.rs` | `F-001` | `Haken`, `VorigeTasten` |
//! | `schub.rs` | `F-007` | `AntriebSchub` |
//! | `einholen.rs` | `F-005` | `AntriebEinholen` |
//!
//! Was hier **nicht** steht: `Tempo`, `Transform`, `Seillaenge`. Die schreibt der Integrator
//! in `player::koerper` — einer, nicht zwei.

pub mod einholen;
pub mod gas;
pub mod haken;
pub mod schub;
pub mod zielen;

use bevy::prelude::*;

use crate::shared::SchrittSet;

pub struct VectorPlugin;

impl Plugin for VectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, zielen::zielen.in_set(SchrittSet::Welt))
            // `.chain()`: das Gaskonto bucht, BEVOR der Haken schaltet — sonst haengt es an
            // der Systemreihenfolge, ob ein frisch gesetzter Haken im selben Tick schon Gas
            // kostet.
            .add_systems(
                FixedUpdate,
                (gas::gaskonto, haken::haken_schalten).chain().in_set(SchrittSet::Absicht),
            )
            // **Bewusst ohne `.chain()`**: beide schreiben ihr eigenes Component mit
            // Zuweisung, die `&mut`-Mengen sind disjunkt, also ist die Reihenfolge
            // beweisbar egal — und Bevy laesst sie echt parallel laufen.
            .add_systems(FixedUpdate, (schub::schub, einholen::einholen).in_set(SchrittSet::Antrieb))
            .add_systems(
                FixedUpdate,
                haken::vorige_tasten_sichern.in_set(SchrittSet::Nachlauf),
            );
    }
}
