//! net — **die Naht fuer Multiplayer.** Heute ein Transport, spaeter Client und Server.
//!
//! Der Netzcode ist nicht Teil dieses Auftrags. **Aber der Ort, an dem er einmal steht, ist
//! ab Tag 1 vorhanden und leer** — statt spaeter mitten durch fuenf Domaenen zu schneiden
//! (`prompts/init.md` §6, `docs/multiplayer.md`).
//!
//! ```text
//! Tastatur ─┐
//! Skript   ─┼─► Posteingang ─► intents_zustellen ─► Intent am Spieler ─► Simulation
//! (Netz)   ─┘   (PlayerId → Intent)   FixedPreUpdate
//! ```
//!
//! **Drei Quellen, ein Kanal.** Der Skript-Fahrer ist kein zweiter, falscher Weg zu
//! spielen — jedes System dahinter ist das echte. Und weil in dieser Umgebung niemand
//! klicken kann, wird dieser Kanal ohnehin gebaut: **ein Aufwand, zwei Probleme geloest.**
//!
//! Hier sitzt auch der **Verzoegerungs-Schalter** (`--lag 200`). Er gehoert ins Werkzeug und
//! nicht in ein spaeteres Ticket: „fuehlt sich lokal gut an" ist keine Abnahme
//! (Bibel T-019).

pub mod lokal;

use bevy::prelude::*;
use std::collections::{BTreeMap, VecDeque};

use crate::shared::{EingabeSet, Intent, PlayerId, Start, Tick};

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        let start = app
            .world()
            .get_resource::<Start>()
            .cloned()
            .unwrap_or_default();

        // 60 Hz feste Simulation -> ein Tick sind 16,67 ms. Aufgerundet, damit `--lag 200`
        // nie WENIGER als 200 ms simuliert: eine zu kleine Latenz beim Testen ist die
        // gefaehrlichere Richtung.
        let lag_ticks = (start.lag_ms as f64 / 1000.0 * 60.0).ceil() as u64;

        app.insert_resource(Transport::LocalOnly)
            .insert_resource(Posteingang::mit_lag(lag_ticks))
            .init_resource::<crate::shared::BlickVorgabe>()
            .configure_sets(
                FixedPreUpdate,
                (EingabeSet::Quelle, EingabeSet::Sammeln, EingabeSet::Zustellen).chain(),
            )
            .add_systems(FixedPreUpdate, lokal::tastatur_lesen.in_set(EingabeSet::Sammeln))
            .add_systems(
                FixedPreUpdate,
                (tick_zaehlen, intents_zustellen)
                    .chain()
                    .in_set(EingabeSet::Zustellen),
            );
    }
}

/// Woher die Intents kommen.
///
/// Heute gibt es genau eine Variante. Sie steht trotzdem als Enum da, weil der Tag, an dem
/// die zweite dazukommt, sonst der Tag ist, an dem jemand `net` umbaut statt erweitert.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Transport {
    #[default]
    LocalOnly,
}

/// Der eine Kanal. **Niemand schreibt `Intent` direkt an einen Spieler** — alle werfen hier
/// ein, und [`intents_zustellen`] stellt zu.
#[derive(Resource, Debug, Default)]
pub struct Posteingang {
    /// Wie viele Ticks eine Nachricht liegen bleibt (`--lag`).
    pub lag_ticks: u64,
    warteschlange: VecDeque<(u64, PlayerId, Intent)>,
    /// Der zuletzt zugestellte Intent je Spieler. Wird gehalten, damit ein Spieler ohne
    /// neue Nachricht nicht schlagartig stehen bleibt — im Netz waere genau das das
    /// Ruckeln, das man als „Lag" sieht.
    letzter: BTreeMap<PlayerId, Intent>,
}

impl Posteingang {
    /// Ein Posteingang mit fester Latenz. Der einzige Weg, `lag_ticks` von aussen zu
    /// setzen — die Warteschlange gehoert dieser Domaene und niemandem sonst.
    pub fn mit_lag(lag_ticks: u64) -> Self {
        Posteingang { lag_ticks, ..default() }
    }

    /// Einwerfen. `faellig_ab` ist der Tick, ab dem zugestellt werden darf.
    pub fn einwerfen(&mut self, spieler: PlayerId, intent: Intent, jetzt: u64) {
        self.warteschlange
            .push_back((jetzt + self.lag_ticks, spieler, intent));
    }

    /// Alles abholen, was faellig ist. Reihenfolge bleibt erhalten (FIFO je Spieler).
    pub fn abholen(&mut self, jetzt: u64) -> Vec<(PlayerId, Intent)> {
        let mut fertig = Vec::new();
        while let Some(&(faellig, _, _)) = self.warteschlange.front() {
            if faellig > jetzt {
                break;
            }
            let (_, spieler, intent) = self.warteschlange.pop_front().expect(
                // Begruendung: `front()` hat gerade Some geliefert, und niemand sonst
                // haelt hier eine Referenz.
                "front() war Some",
            );
            self.letzter.insert(spieler, intent);
            fertig.push((spieler, intent));
        }
        fertig
    }

    pub fn letzter(&self, spieler: PlayerId) -> Option<Intent> {
        self.letzter.get(&spieler).copied()
    }

    pub fn wartend(&self) -> usize {
        self.warteschlange.len()
    }
}

/// Der Tick zaehlt in `FixedPreUpdate` hoch — **vor** allem, was ihn liest.
fn tick_zaehlen(mut tick: ResMut<Tick>) {
    tick.0 += 1;
}

/// Stellt faellige Intents an die Spieler zu.
///
/// Ueber [`PlayerId`], nicht ueber `Entity`: eine `Entity` bedeutet auf einem anderen
/// Rechner etwas anderes (§6 Regel 7).
fn intents_zustellen(
    tick: Res<Tick>,
    mut post: ResMut<Posteingang>,
    mut spieler: Query<(&PlayerId, &mut Intent)>,
) {
    let faellig = post.abholen(tick.0);
    if faellig.is_empty() {
        return;
    }
    for (id, mut intent) in &mut spieler {
        if let Some((_, neu)) = faellig.iter().rev().find(|(w, _)| w == id) {
            *intent = *neu;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ohne_lag_wird_sofort_zugestellt() {
        let mut p = Posteingang::default();
        p.einwerfen(PlayerId(1), Intent { tick: 5, ..default() }, 5);
        let raus = p.abholen(5);
        assert_eq!(raus.len(), 1);
        assert_eq!(raus[0].0, PlayerId(1));
    }

    #[test]
    fn lag_haelt_genau_so_viele_ticks_zurueck() {
        // 200 ms bei 60 Hz sind 12 Ticks — das ist die Zahl, mit der jedes
        // Bewegungsfeature geprueft wird (Bibel T-019).
        let mut p = Posteingang::mit_lag(12);
        p.einwerfen(PlayerId(1), Intent::default(), 100);
        for jetzt in 100..112 {
            assert!(p.abholen(jetzt).is_empty(), "bei Tick {jetzt} noch nicht faellig");
        }
        assert_eq!(p.abholen(112).len(), 1);
    }

    #[test]
    fn mehrere_spieler_bekommen_ihre_eigene_post() {
        // Es gibt keinen „den Spieler" (§6 Regel 3).
        let mut p = Posteingang::default();
        p.einwerfen(PlayerId(1), Intent { yaw: 1.0, ..default() }, 0);
        p.einwerfen(PlayerId(2), Intent { yaw: 2.0, ..default() }, 0);
        let raus = p.abholen(0);
        assert_eq!(raus.len(), 2);
        assert_eq!(p.letzter(PlayerId(1)).map(|i| i.yaw), Some(1.0));
        assert_eq!(p.letzter(PlayerId(2)).map(|i| i.yaw), Some(2.0));
    }

    #[test]
    fn reihenfolge_bleibt_erhalten() {
        let mut p = Posteingang::default();
        for t in 0..5u64 {
            p.einwerfen(PlayerId(1), Intent { tick: t, ..default() }, 0);
        }
        let ticks: Vec<u64> = p.abholen(0).into_iter().map(|(_, i)| i.tick).collect();
        assert_eq!(ticks, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn nichts_geht_verloren_wenn_niemand_abholt() {
        let mut p = Posteingang::mit_lag(3);
        p.einwerfen(PlayerId(1), Intent::default(), 0);
        p.einwerfen(PlayerId(1), Intent::default(), 1);
        assert_eq!(p.wartend(), 2);
        assert_eq!(p.abholen(10).len(), 2, "spaeter abholen holt beides");
        assert_eq!(p.wartend(), 0);
    }
}
