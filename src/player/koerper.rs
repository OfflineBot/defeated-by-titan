//! **Der Integrator** — das einzige System, das `Transform`, `Tempo`, `Bewegungszustand`
//! und `Seillaenge` eines Spielers schreibt.
//!
//! Die alte Teilung „`player` am Boden, `vector` am Seil, getrennt ueber
//! `Bewegungszustand`" haelt nicht: ein Gas-Boost wirkt in der Luft **und** am Seil
//! gleichzeitig, es gibt also keinen Zustand, der die beiden Schreiber trennt. Statt zweier
//! Schreiber mit einem Schalter gibt es jetzt **einen** Schreiber und mehrere
//! Sammelstellen.
//!
//! ## Die Schrittfolge, nummeriert — sie ist die Schnittstelle
//!
//! ```text
//! (a) dt  = dt_gezaehmt(Time<Fixed>::delta_secs())
//! (b) a   = AntriebLauf + AntriebSchub + (0, schwerkraft_m_s2, 0)
//! (c) tempo += a * dt
//! (d) tempo  = clamp_length_max(vector.tempo_max_m_s)            F-012, VOR der Bewegung
//! (e) Seillaengen fortschreiben:
//!       frisch verankert  -> laenge = |pos - anker|
//!       AntriebEinholen   -> laenge -= wunsch * dt, geklemmt auf vector.seil_min_m
//!                            tempo   = seil_einholen(..)  (Drehimpuls, F-005)
//!       nicht verankert   -> laenge = 0
//! (f) N   = ceil(|tempo| * dt / spieler.schritt_max_m), mindestens 1
//! (g) je Teilschritt, in DIESER Reihenfolge:
//!       (g1) pos_frei = pos + tempo * dt/N
//!       (g2) seil_schritt(pos, pos_frei, tempo, zwaenge, vector.seil_durchlaeufe)
//!       (g3) Kollision gegen RaumIndex::huelle, Haut = welt.kollision_haut_m
//!       (g4) hat die Kollision aus der Seilkugel geschoben:
//!              laenge = |pos - anker|   (nachziehen, nie gegen die Wand kaempfen)
//!              laenge > vector.hakenreichweite_m -> ueberdehnt = true
//! (h) Bewegungszustand ableiten: AmSeil > AmBoden > InDerLuft
//! (i) Aufprall melden, wenn (g3) Tempo genommen hat
//! ```
//!
//! ## Der Schiedsrichter: **die Wand gewinnt, das Seil gibt nach**
//!
//! Ohne diese Festlegung entscheiden zwei Systeme unabhaengig ueber dieselbe Position, und
//! das Ergebnis ist ein Zittern mit 30 Hz auf dem `Transform`, an dem die Kamera als Kind
//! haengt (`src/render/mod.rs`) — das sieht nicht aus wie „zwei Systeme ohne Schiedsrichter",
//! sondern wie „die Physik ist kaputt". Deshalb ist die Reihenfolge in (g) **fest** und die
//! Seillaenge gibt nach, nicht die Position.
//!
//! ## Kein `Local` als Streupuffer
//!
//! Der Puffer fuer `RaumIndex::huelle` wird pro Tick angelegt, nicht in einem `Local<Vec<_>>`
//! gehalten. Ein Waechter, der `Local<T>` in Simulationssystemen verbietet
//! (`docs/multiplayer.md`), kann Zustand und Streupuffer nicht unterscheiden, und eine
//! Ausnahmeliste ist der Anfang vom Ende der Regel. Wird die Allokation **gemessen** teuer,
//! kommt der Puffer als Feld neben den Index — nicht als `Local`.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    AntriebEinholen, AntriebLauf, AntriebSchub, Aufprall, Bewegungszustand, Haken, PlayerId,
    RaumIndex, Seillaenge, Tempo, Tick,
};

/// Ein fester Simulationsschritt fuer jeden Spieler.
// gefuellt von Auftrag V — F-004, F-005, F-012, F-013, Stufe 2
pub fn schritt(
    _zeit: Res<Time<Fixed>>,
    _tick: Res<Tick>,
    _daten: Res<GameData>,
    _index: Res<RaumIndex>,
    mut _aufprall: MessageWriter<Aufprall>,
    mut _spieler: Query<(
        &PlayerId,
        &AntriebLauf,
        &AntriebSchub,
        &AntriebEinholen,
        &Haken,
        &mut Transform,
        &mut Tempo,
        &mut Bewegungszustand,
        &mut Seillaenge,
    )>,
) {
}
