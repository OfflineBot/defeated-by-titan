//! `F-001` Der Doppelhaken — die Zustandsmaschine beider Arme.
//!
//! `Ruht -> Fliegt -> Haelt -> ZiehtEin -> Ruht`, je Seite unabhaengig (`F-001` woertlich:
//! „Zwei unabhaengig steuerbare Enterhaken (links/rechts), einzeln abfeuerbar und
//! loesbar").
//!
//! **Ausgeloest wird auf der Flanke, nicht auf dem Halten**: `Tasten::frisch` gegen
//! [`VorigeTasten`] — halten ist nicht schiessen. Der Vorzustand ist ein **Component am
//! Spieler**, kein `Local<Tasten>`: ein `Local` gehoert dem System und wird von allen
//! Spielern geteilt (Spieler 2 loest aus, wenn Spieler 1 loslaesst), und im Schnappschuss
//! ist er unsichtbar, ueberlebt also keinen Rollback.
//!
//! Dieses Modul ist der **einzige Schreiber von [`Haken`]** und der einzige Sender von
//! `HakenGesetzt`/`HakenGeloest`. Zwei Gruende zum Loesen kommen von aussen und werden hier
//! nur ausgefuehrt:
//! - `KoerperWeg` (Traeger verschwunden) — im selben Tick, weil `SchrittSet::Raum` vor
//!   `SchrittSet::Absicht` laeuft.
//! - `Seillaenge::ueberdehnt` (die Wand hat gewonnen) — **einen Tick spaeter**, weil der
//!   Integrator es erst in `SchrittSet::Vollzug` setzt. Ein Tick Verzug ist der Preis dafuer,
//!   dass `Haken` genau einen Schreiber hat.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    Haken, HakenGeloest, HakenGesetzt, Intent, KoerperWeg, PlayerId, RaumIndex, Seillaenge,
    Tick, VorigeTasten, Zielpunkt,
};

/// Fuehrt beide Arme durch ihre Zustaende und meldet jeden Wechsel.
// gefuellt von Auftrag H — F-001
pub fn haken_schalten(
    _tick: Res<Tick>,
    _zeit: Res<Time<Fixed>>,
    _daten: Res<GameData>,
    _index: Res<RaumIndex>,
    mut _weg: MessageReader<KoerperWeg>,
    mut _gesetzt: MessageWriter<HakenGesetzt>,
    mut _geloest: MessageWriter<HakenGeloest>,
    mut _spieler: Query<(
        &PlayerId,
        &Intent,
        &VorigeTasten,
        &Zielpunkt,
        &Seillaenge,
        &mut Haken,
    )>,
) {
}

/// Sichert die Tasten dieses Ticks fuer die Flankenerkennung im naechsten.
///
/// Laeuft am Ende des Schritts (`SchrittSet::Nachlauf`) — vorher haben alle Leser die Flanke
/// noch gesehen. **Der einzige Schreiber von [`VorigeTasten`]**; wer sonst eine Flanke
/// braucht, liest dieses Component, statt einen zweiten Vorzustand zu fuehren.
// gefuellt von Auftrag H — F-001
pub fn vorige_tasten_sichern(mut _spieler: Query<(&Intent, &mut VorigeTasten)>) {}
