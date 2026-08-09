//! Die Messages zwischen den Domaenen.
//!
//! **Kommunikation laeuft ueber Components und Messages, nicht ueber Aufrufe.** `combat`
//! schickt [`TitanGetroffen`]; `titan` liest es und entscheidet, was das fuer seinen Koerper
//! heisst. `combat` weiss nicht, wie ein Titan gebaut ist (`prompts/init.md` §5 Regel 3).
//!
//! Sie liegen **hier** und nicht beim Sender, weil sonst jeder Empfaenger eine Kante zum
//! Sender braeuchte und die Domaenenregel nach einer Woche leer waere.
//!
//! Und sie sind so entworfen, dass sie **ueber eine Leitung passen**: Daten, keine Handles,
//! keine Funktionszeiger, **keine `Entity`** (§6 Regel 7 und 8).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::gear::Seite;
use super::ids::{KoerperId, PlayerId, TitanId};

/// Welcher Teil eines Titanen getroffen wurde.
///
/// **Der Cortex ist die einzige Wahrheit**: ein Cortex-Treffer toetet, egal wie voll der
/// Titan ist. Alles andere ist Vorbereitung — Beine ab heisst, er faellt; Arme ab heisst, er
/// kann nicht greifen; Augen heisst, er sieht dich nicht.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Koerperteil {
    Cortex,
    Kopf,
    Auge,
    ArmLinks,
    ArmRechts,
    BeinLinks,
    BeinRechts,
    Rumpf,
}

/// Eine Klinge hat einen Titanen getroffen.
///
/// `tempo_m_s` ist der Grund, warum das eine Nachricht ist und kein Aufruf: **Schaden kommt
/// aus Geschwindigkeit.** Ein Schnitt aus dem Stand kratzt, derselbe Schnitt aus 30 m/s
/// toetet — und die Formel dazu steht in der RON, nicht im Code (`prompts/init.md` §1, §4).
#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TitanGetroffen {
    pub titan: TitanId,
    pub von: PlayerId,
    pub teil: Koerperteil,
    pub tempo_m_s: f32,
}

/// Bitte einen Titanen erzeugen. Kommt aus `mission` (Spawn-Wellen) oder aus dem
/// `--script`-Fahrer (`spawn titan husk 20 0 -40`).
///
/// `art` ist der **logische Name** aus `assets/data/titan.ron`, kein Dateiname und kein
/// Rust-Typ — sonst braeuchte es fuer einen neuen Titanen einen Rebuild (§4).
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
pub struct TitanSpawnen {
    pub art: String,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
}

impl TitanSpawnen {
    pub fn pos(&self) -> Vec3 {
        Vec3::new(self.pos_x, self.pos_y, self.pos_z)
    }
}

/// Einen Spieler an eine Koordinate setzen (`warp x y z` im Skript, F3-Overlay).
///
/// Damit kann der User eine Koordinate schicken und man steht genau dort — das ist mehr
/// wert als jedes Bug-Formular (§12c).
#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SpielerWarpen {
    pub spieler: PlayerId,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
}

/// Eine Zeile ins Log, an der man einen Screenshot ausrichtet (`mark eingehakt`).
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
pub struct Markierung {
    pub text: String,
    pub tick: u64,
}

/// Ein Haken hat sich verankert (`F-001`).
///
/// Nackte `f32` fuer den Punkt, wie bei [`SpielerWarpen`]: dieser Typ geht eines Tages ueber
/// eine Leitung. `koerper` statt einer Weltposition, damit der Empfaenger weiss, **woran**
/// der Haken haengt.
#[derive(Message, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HakenGesetzt {
    pub spieler: PlayerId,
    pub seite: Seite,
    pub koerper: KoerperId,
    pub punkt_x: f32,
    pub punkt_y: f32,
    pub punkt_z: f32,
    pub tick: u64,
}

impl HakenGesetzt {
    pub fn punkt(&self) -> Vec3 {
        Vec3::new(self.punkt_x, self.punkt_y, self.punkt_z)
    }
}

/// Warum ein Haken losgelassen hat.
///
/// Der Grund ist kein Protokolltext: `hud` und `sound` unterscheiden daran, ob der Spieler
/// selbst losgelassen hat oder ob ihm das Seil gerissen ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Loesegrund {
    /// Taste losgelassen.
    Losgelassen,
    /// Das Seil musste ueber `vector.hakenreichweite_m` hinaus nachgezogen werden, weil eine
    /// Wand gewonnen hat.
    Ueberdehnt,
    /// Der Strahl traf nichts Hakbares — die Spitze kommt leer zurueck.
    KeinAnker,
    /// **Der Traeger ist weg** (`F-029`: „loest sich beim Tod des Titanen mit Feedback";
    /// `T-020`: entladener Bereich).
    TraegerWeg,
}

/// Ein Haken hat losgelassen (`F-001`).
#[derive(Message, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HakenGeloest {
    pub spieler: PlayerId,
    pub seite: Seite,
    pub grund: Loesegrund,
    pub tick: u64,
}

/// Ein Spieler ist gegen Geometrie gefahren.
///
/// `tempo_vorher_m_s` ist die Zahl, aus der `F-013` den Stagger rechnet — dieselbe Logik wie
/// bei [`TitanGetroffen`]: **die Wirkung kommt aus der Geschwindigkeit**, und die Formel
/// steht in der RON.
#[derive(Message, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aufprall {
    pub spieler: PlayerId,
    pub tempo_vorher_m_s: f32,
    pub tempo_nachher_m_s: f32,
    pub normale_x: f32,
    pub normale_y: f32,
    pub normale_z: f32,
    pub tick: u64,
}

impl Aufprall {
    pub fn normale(&self) -> Vec3 {
        Vec3::new(self.normale_x, self.normale_y, self.normale_z)
    }
}

/// Ein Koerper ist aus dem raeumlichen Index verschwunden. **Wer daran haengt, muss
/// loslassen.**
#[derive(Message, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct KoerperWeg {
    pub koerper: KoerperId,
    pub tick: u64,
}
