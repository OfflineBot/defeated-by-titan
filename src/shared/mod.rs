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
pub mod gear;
pub mod ids;
pub mod intent;
pub mod mathe;
pub mod nachricht;
pub mod raum;
pub mod seil;
pub mod start;
pub mod zufall;
pub mod zustand;

pub use ablauf::{EingabeSet, SchrittSet, Tick};
pub use bau::{spielerhuelle, Ankerflaeche, Bauklotz, Boden, Koerper, Maske};
pub use gear::{
    AntriebEinholen, AntriebLauf, AntriebSchub, Gasfreigabe, Haken, Hakenarm, Hakenzustand,
    Seillaenge, Seite, VorigeTasten, Zielpunkt,
};
pub use ids::{IdZaehler, KoerperId, LocalPlayer, PlayerId, TitanId};
pub use intent::{BlickVorgabe, Intent, Tasten};
pub use nachricht::{
    Aufprall, HakenGeloest, HakenGesetzt, KoerperWeg, Koerperteil, Loesegrund, Markierung,
    SpielerWarpen, TitanGetroffen, TitanSpawnen,
};
pub use raum::{Eintrag, RaumIndex, Strahlergebnis, Treffer};
pub use seil::{seil_einholen, seil_schritt, Seilzwang, Zwangsergebnis};
pub use start::Start;
pub use zufall::Wuerfel;
pub use zustand::{Bewegungszustand, Gas, Klingen, Tempo};
