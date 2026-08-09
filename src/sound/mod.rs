//! sound — Gas-Zischen, Hakeneinschlag, Klingenschnitt, Titanenschritt
//!
//! **Jede Trefferart hat einen eigenen Klang** (Bibel 2, Pfeiler P4). Und Gas verbrauchen ist
//! laut: der Bellower reagiert darauf, das koppelt die Ressource an das Risiko.
//!
//! ⚠️ Bevys Audio haengt an ALSA und ist deshalb hinter dem Feature `klang` versteckt — auf
//! Maschine A gibt es kein `alsa.pc` (`docs/umgebung.md`). Ohne das Feature laedt diese
//! Domaene nichts und spielt nichts; sie meldet es beim Start **einmal**, statt still zu
//! schweigen.
//!
//! Klaenge werden **gemessen statt gehoert**: Laenge, Grundfrequenz, Huellkurve,
//! Spitzenpegel, ob er schleift. Nur originale oder lizenzierte Musik (Bibel 6.4).
//!
//! **Noch leer.** Das Plugin steht im Baum, damit die Reihenfolge in `lib.rs` von Anfang an
//! stimmt und ein Fan-out auf Domaenen moeglich ist, ohne dass fuenf Agenten denselben
//! Ordner anlegen (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, _app: &mut App) {}
}
