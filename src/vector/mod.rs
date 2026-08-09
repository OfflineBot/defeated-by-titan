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
//! **Noch leer.** Das Plugin steht im Baum, damit die Reihenfolge in `lib.rs` von Anfang an
//! stimmt und ein Fan-out auf Domaenen moeglich ist, ohne dass fuenf Agenten denselben
//! Ordner anlegen (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct VectorPlugin;

impl Plugin for VectorPlugin {
    fn build(&self, _app: &mut App) {}
}
