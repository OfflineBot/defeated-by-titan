//! shared — Typen, die niemandem gehoeren.
//!
//! **Dies ist die einzige Domaene ohne Plugin.** Hier liegt, was mehrere Domaenen brauchen,
//! ohne dass eine davon die andere kennen muss: Ids, der Eingabekanal, die Messages, die
//! Mathe-Helfer, die Startflags.
//!
//! Warum die Messages hier liegen und nicht beim Sender: `combat` schickt
//! [`TitanGetroffen`], `titan` liest es. Laege der Typ in `combat`, braeuchte `titan` eine
//! Kante zu `combat` — und die Domaenenregel waere nach einer Woche leer
//! (`docs/architektur.md`).
//!
//! Warum hier auch [`Gas`] und [`Klingen`] liegen, obwohl `vector` und `blades` sie
//! schreiben: weil `hud` und `sound` sie **lesen** muessen. Wer schreibt, steht in der
//! Autoritaetstabelle in `docs/architektur.md` — nicht im Typ.

pub mod ablauf;
pub mod bau;
pub mod ids;
pub mod intent;
pub mod mathe;
pub mod nachricht;
pub mod start;
pub mod zufall;
pub mod zustand;

pub use ablauf::{EingabeSet, Tick};
pub use bau::{Ankerflaeche, Bauklotz, Boden};
pub use ids::{IdZaehler, LocalPlayer, PlayerId, TitanId};
pub use intent::{BlickVorgabe, Intent, Tasten};
pub use nachricht::{Koerperteil, Markierung, SpielerWarpen, TitanGetroffen, TitanSpawnen};
pub use start::Start;
pub use zufall::Wuerfel;
pub use zustand::{Bewegungszustand, Gas, Klingen, Tempo};
