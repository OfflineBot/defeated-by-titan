//! Was in der Welt steht — als **Daten**, nicht als Mesh.
//!
//! `world` spawnt Bauklötze, `render` macht daraus Meshes. Damit muss `render` die Domaene
//! `world` nicht kennen und `world` nichts vom Rendern verstehen: die beiden reden ueber
//! einen Component, nicht ueber einen Aufruf (`docs/architektur.md`).
//!
//! Und es ist dieselbe Trennung, die Multiplayer spaeter braucht: die Simulation kennt
//! Kloetze und Ankerflaechen, die Darstellung kennt Dreiecke (§6 Regel 1).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Ein Quader in der Welt. Groesse in **Metern**, Ursprung in der Mitte.
///
/// Low Poly, flache Farbflaechen — der Stil der Bibel gilt auch fuer Platzhalter
/// (`docs/konventionen.md`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bauklotz {
    pub groesse: Vec3,
    /// Grundfarbe als lineares RGB. **Keine der drei Signalfarben** (Zyan, Bernstein,
    /// Karminrot) — die sind ausschliesslich fuer Gameplay reserviert.
    pub farbe: [f32; 3],
}

/// Diese Flaeche ist **hakbar**.
///
/// Der uebersetzte `CollectionService`-Tag `AnchorSurface` der Referenz
/// (`docs/architektur.md`). Nur getaggte Flaechen sind hakbar (`F-003`) — das verhindert
/// Physik-Exploits, macht Leveldesign steuerbar und definiert ueber die Flaechendichte die
/// Traversal-Schwierigkeit einer Map.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ankerflaeche;

/// Der Boden. Solange es keinen raeumlichen Index gibt, ist er die einzige Kollision.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Boden {
    pub hoehe_m: f32,
}
