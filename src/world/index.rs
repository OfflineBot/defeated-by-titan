//! Der raeumliche Index, gepflegt — `T-036a`.
//!
//! Der **Typ** liegt in [`shared::raum`](crate::shared::raum), damit `vector` und `player`
//! ihn fragen koennen, ohne eine Kante zu `world` zu brauchen. Hier steht nur, **wer ihn
//! aktuell haelt**.
//!
//! ## Warum die Pflege nicht ueber `RemovedComponents` laeuft
//!
//! Belegt am installierten Quelltext: die Puffer von `RemovedComponents` werden in
//! `World::clear_trackers` umgeschaltet (`bevy_ecs-0.19.0/src/world/mod.rs:1735-1738`), und
//! `clear_trackers` laeuft **einmal pro `App::update`** (`bevy_app-0.19.0/src/sub_app.rs:149`)
//! — nicht pro Tick. `FixedMain` laeuft dagegen „0, 1 or more times during a single update"
//! (`bevy_time-0.19.0/src/fixed.rs:37-39`). `src/lib.rs` treibt headless mit 240 Hz gegen
//! 60 Hz Fixed: **drei von vier Frames laufen ohne `FixedMain`**, und eine Meldung aus einem
//! dieser Frames ist weg, bevor der Pfleger sie sieht. Ein Hindernis bliebe fuer immer im
//! Gitter, ein Haken haengt an einem geloeschten Haus.
//!
//! Deshalb: ein **Beobachter** auf `Remove`, der die Id in das Postfach des Index schiebt
//! (`RaumIndex::abmelden`). Beobachter laufen sofort beim Entfernen und nicht am Frame-Ende
//! — die Information ueberlebt beliebig viele Frames.

use bevy::prelude::*;

use crate::shared::{
    Eintrag, IdZaehler, Koerper, KoerperId, KoerperWeg, Maske, RaumIndex, Tick,
};

/// Nimmt neue Koerper auf, traegt abgemeldete aus und meldet sie als [`KoerperWeg`].
///
/// Laeuft als erstes im festen Schritt (`SchrittSet::Raum`): der Index ist aktuell, **bevor**
/// ihn jemand fragt. Neue Koerper bekommen hier ihre [`KoerperId`] aus dem [`IdZaehler`] —
/// fortlaufend, nicht zufaellig, damit zwei Rechner dieselbe Reihenfolge bekommen.
///
/// Gelesen wird der `GlobalTransform`, nicht der `Transform`: fuer einen Kind-Koerper (ab
/// `F-029` eine Titanengliedmasse) ist der `Transform` **lokal**, und die Weltmitte steht
/// nur im `GlobalTransform`. Heute ist beides identisch — die Zeile ist trotzdem jetzt
/// richtig statt spaeter falsch.
// gefuellt von Auftrag R — T-036a
pub fn index_pflegen(
    mut _commands: Commands,
    mut _index: ResMut<RaumIndex>,
    mut _zaehler: ResMut<IdZaehler>,
    _tick: Res<Tick>,
    mut _weg: MessageWriter<KoerperWeg>,
    _neu: Query<(Entity, &Koerper, &GlobalTransform), Without<KoerperId>>,
    _bekannt: Query<(&KoerperId, &Koerper, &GlobalTransform), Changed<GlobalTransform>>,
) {
}

/// Beobachter: ein [`Koerper`] verschwindet.
///
/// Schiebt seine Id in das Postfach des Index. Der Pfleger holt sie im naechsten festen
/// Schritt ab und schickt [`KoerperWeg`] — daran loesen die Haken, die an ihm hingen.
///
/// `Option<ResMut<RaumIndex>>`, weil ein Beobachter auch vor dem Einfuegen der Resource
/// feuern kann (Test-Apps, `App::finish`). Ein fehlender Index ist dann kein Absturz,
/// sondern ein Koerper, den es im Index ohnehin nie gab.
// gefuellt von Auftrag R — T-036a
pub fn koerper_abmelden(
    _ereignis: On<Remove, Koerper>,
    _ids: Query<&KoerperId>,
    _index: Option<ResMut<RaumIndex>>,
) {
}

/// Aus dem Marker-Zustand einer Entity die Maske bauen.
///
/// Eine Stelle, an der aus `hakbar`/`fest` Bits werden — sonst steht die Uebersetzung in
/// `world::karte` **und** hier, und eine der beiden veraltet.
pub fn maske_aus(fest: bool, hakbar: bool) -> Maske {
    let mut m = Maske::KEINE;
    if fest {
        m = m.mit(Maske::FEST);
    }
    if hakbar {
        m = m.mit(Maske::HAKBAR);
    }
    m
}

/// Einen Eintrag aus Koerper und Weltposition bauen. Die einzige Stelle, die weiss, dass die
/// Mitte des Eintrags die Weltposition der Entity ist.
pub fn eintrag_aus(id: KoerperId, koerper: &Koerper, welt: &GlobalTransform) -> Eintrag {
    Eintrag {
        id,
        mitte_m: welt.translation(),
        halb_m: koerper.halb_m,
        maske: koerper.maske,
    }
}
