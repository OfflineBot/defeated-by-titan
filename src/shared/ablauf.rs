//! Der Simulationstick und die Reihenfolge, in der Eingaben entstehen.
//!
//! Beides liegt in `shared/`, weil **zwei** Domaenen es brauchen und keine der anderen
//! gehoeren darf: `net` stellt Intents zu, `debug` erzeugt sie aus einem Skript. Laege der
//! `SystemSet` bei `net`, braeuchte `debug` eine Kante dorthin — und die Domaenenregel waere
//! aufgeweicht, um eine Reihenfolge auszudruecken (`docs/architektur.md`).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Der Simulationstick. Zaehlt in `FixedPreUpdate` hoch, **bevor** ihn jemand liest.
///
/// Er ist Teil des Zustands: der geseedete Zufall rechnet aus `(seed, tick)`, und ein
/// `Intent` traegt den Tick, fuer den er gemeint war (§6 Regel 2 und 5).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tick(pub u64);

/// Die drei Stufen, in denen eine Eingabe pro Tick entsteht.
///
/// **Die Reihenfolge ist der ganze Punkt.** Ohne sie haengt es von der Laufreihenfolge der
/// Systeme ab, ob ein Tastendruck aus dem Skript noch im selben Tick ankommt — und das ist
/// kein Design, sondern ein Muenzwurf mit 60 Hz.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EingabeSet {
    /// Wer Eingaben **erzeugt**: der `--script`-Fahrer drueckt echte Tasten.
    Quelle,
    /// Wer sie **einsammelt**: Tastatur und Maus werden zu einem `Intent` im Posteingang.
    Sammeln,
    /// Wer sie **zustellt**: Tick hochzaehlen, faellige Intents an die Spieler.
    Zustellen,
}

/// Die sechs Stufen **eines Simulationsschritts** in `FixedUpdate`.
///
/// Sie sind `.chain()`-verkettet und werden an **genau einer** Stelle konfiguriert:
/// `src/lib.rs`. Nicht in einem Plugin, weil vier Domaenen Mitglieder sind — eine Domaene,
/// die die Reihenfolge einer anderen festlegt, ist eine versteckte Kante an der
/// Erlaubnisliste vorbei. Der Typ liegt aus demselben Grund hier wie [`EingabeSet`].
///
/// **Die Reihenfolge ist die Antwort auf „wer gewinnt".** Sie steht hier und nicht in
/// `.before()`/`.after()`-Zeilen ueber fuenf Dateien verteilt, weil ein Muenzwurf mit 60 Hz
/// im Netz ein Auseinanderlaufen ist, das niemand reproduziert (`docs/architektur.md`).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SchrittSet {
    /// Der raeumliche Index wird aktuell, **bevor** ihn jemand fragt: neue Koerper
    /// aufnehmen, abgemeldete austragen, `KoerperWeg` melden.
    Raum,
    /// Fragen an die Welt, bevor sich etwas bewegt: der Zielstrahl (`F-002`). Alle Systeme
    /// dieses Sets sehen denselben Index-Schnappschuss.
    Welt,
    /// `Intent` -> Zustandswechsel und Buchungen: Gas abbuchen (`F-018`), Haken abfeuern und
    /// loesen (`F-001`). Hier wird `Tempo` **nie** angefasst — genau daraus folgt `F-014`
    /// Momentum-Chaining ohne eine Zeile Extraarbeit: ein Hakenwechsel kann Geschwindigkeit
    /// gar nicht verlieren.
    Absicht,
    /// Jeder Beitragende schreibt **sein** Antriebs-Component mit Zuweisung, nie mit `+=`.
    /// Die Mengen sind disjunkt, also ist die Reihenfolge innerhalb des Sets beweisbar egal
    /// — **bewusst kein `.chain()`**.
    Antrieb,
    /// **Der einzige Schreiber von `Transform`, `Tempo`, `Bewegungszustand` und
    /// `Seillaenge` eines Spielers.** Integration, Klemme (`F-012`), Teilschritte,
    /// Seilzwang (`F-004`), Kollision (`F-013`).
    Vollzug,
    /// Folgen des Schritts, die niemand mitten in der Integration haben will: die Tasten
    /// des Ticks sichern (Flankenerkennung im naechsten Tick).
    Nachlauf,
}
