//! `F-007` Gas-Boost — Impuls in Blickrichtung, solange die Taste gehalten wird.
//!
//! Schreibt **nur** [`AntriebSchub`], nie `Tempo` und nie `Transform`: die Sammelstelle
//! gehoert dem Integrator. Damit wirkt der Boost in der Luft **und** am Seil gleichzeitig,
//! ohne dass zwei Systeme um denselben `Transform` streiten — genau der Fall, an dem die
//! alte Zustandsteilung „`player` am Boden, `vector` am Seil" zerbrochen ist.
//!
//! Ruft `Gas::verbrauchen` **nicht**: das Konto gehoert `vector::gas`. Hier wird nur
//! [`Gasfreigabe`] gelesen. Ohne Freigabe steht `Vec3::ZERO` im Antrieb — kein halber Boost
//! (`F-018`: „Bei 0 kein Fliegen mehr").
//!
//! Hier docken spaeter `F-006` Swerve und `F-008` Dash an: ein System mehr, kein Typ mehr.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{AntriebSchub, Gasfreigabe, Intent};

/// Schreibt [`AntriebSchub`] = Blickrichtung * `vector.boost_m_s2`, oder `ZERO`.
// gefuellt von Auftrag B — F-007
pub fn schub(
    _daten: Res<GameData>,
    mut _spieler: Query<(&Intent, &Gasfreigabe, &mut AntriebSchub)>,
) {
}
