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
///
/// ⚠️ **Uebergang.** Sobald `world::karte` die Karte aus `assets/data/maps.ron` baut, ist der
/// Boden ein [`Koerper`] wie jeder andere und dieser Typ faellt weg — zusammen mit
/// `boden_y = 0.0` in `src/player/mod.rs`. Beides stirbt im selben Commit wie der gefuellte
/// Index, nicht vorher: ohne Index und ohne Boden faellt der Spieler 600 Ticks lang.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Boden {
    pub hoehe_m: f32,
}

/// Wofuer ein Koerper zaehlt — ein Bitmuster wie [`Tasten`](super::intent::Tasten).
///
/// Feste Groesse, `serde` ohne Zusatzfeature, passt ueber eine Leitung. Ein `bool` je Zweck
/// waere dasselbe mit mehr Feldern und ohne die Moeglichkeit, mehrere Zwecke in einer
/// Abfrage zu pruefen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Maske(pub u32);

impl Maske {
    pub const KEINE: Maske = Maske(0);
    /// Stoppt einen Koerper — Kollision (Stufe 2, `F-013`).
    pub const FEST: Maske = Maske(1 << 0);
    /// Hakbar (`F-003`), abgeleitet aus dem Marker [`Ankerflaeche`].
    pub const HAKBAR: Maske = Maske(1 << 1);
    /// Traegt Klingentreffer (`blades`, `combat`).
    pub const SCHNEIDBAR: Maske = Maske(1 << 2);

    pub fn hat(self, andere: Maske) -> bool {
        self.0 & andere.0 == andere.0 && andere.0 != 0
    }

    pub fn mit(self, andere: Maske) -> Maske {
        Maske(self.0 | andere.0)
    }
}

/// Die **achsenparallele Huelle** eines Koerpers in der Welt — Haus, Dach, Boden, spaeter
/// eine Titanenschulter.
///
/// Mitte ist die Weltposition der Entity; `world::index` liest sie aus dem
/// `GlobalTransform`, damit ein Kind-Koerper (ab `F-029`) nicht seine lokale Position in den
/// Index schreibt. Heute ist beides identisch.
///
/// **Nach dem Spawnen unveraenderlich** — ein unveraenderliches Component hat keinen
/// Schreibkonflikt. Ein achsenparalleler Quader **ist** exakt seine AABB; eine gedrehte
/// `Cuboid` liefert dagegen die umschliessende, zu grosse Huelle
/// (`bevy_math-0.19.0/src/bounding/bounded3d/primitive_impls.rs:100-115`), und dann faengt
/// der Haken sichtbar in der Luft. **Kloetze werden nicht gedreht.**
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Koerper {
    /// Halbe Kantenlaenge in Metern — dieselbe Form, die `Aabb3d::new(center, half_size)`
    /// nimmt (`bevy_math-0.19.0/src/bounding/bounded3d/mod.rs:66`).
    pub halb_m: Vec3,
    pub maske: Maske,
}

/// Die Huelle des Spielers: **Mitte und halbe Kantenlaenge**, aus Groesse und Position.
///
/// Der Ursprung eines Modells liegt **zwischen den Fuessen** (`docs/konventionen.md`) — die
/// naheliegende Zeile `Aabb3d::new(translation, (r, hoehe/2, r))` versenkt den Spieler also
/// 0,9 m im Boden, und `scripts/t007-erste-fahrt.txt` (`assert hoehe < 0.5`) faellt. Diese
/// zwei Zeilen stehen deshalb **einmal** hier und nicht in drei Systemen.
pub fn spielerhuelle(translation_m: Vec3, hoehe_m: f32, radius_m: f32) -> (Vec3, Vec3) {
    let halbe_hoehe = hoehe_m * 0.5;
    (
        translation_m + Vec3::Y * halbe_hoehe,
        Vec3::new(radius_m, halbe_hoehe, radius_m),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f003_eine_maske_haelt_mehrere_zwecke_auseinander() {
        let m = Maske::FEST.mit(Maske::HAKBAR);
        assert!(m.hat(Maske::FEST));
        assert!(m.hat(Maske::HAKBAR));
        assert!(!m.hat(Maske::SCHNEIDBAR));
        // „nichts" haelt nie — sonst wuerde jede Abfrage nach der leeren Maske wahr.
        assert!(!m.hat(Maske::KEINE));
        assert!(!Maske::KEINE.hat(Maske::FEST));
    }

    #[test]
    fn stufe2_die_spielerhuelle_steht_auf_dem_boden_und_nicht_darin() {
        // Ursprung zwischen den Fuessen: die Unterkante der Huelle liegt exakt auf y = 0,
        // wenn der Spieler auf y = 0 steht.
        let (mitte, halb) = spielerhuelle(Vec3::new(3.0, 0.0, -4.0), 1.8, 0.35);
        assert!((mitte.y - 0.9).abs() < 1e-6, "Mitte {mitte:?}");
        assert!((halb - Vec3::new(0.35, 0.9, 0.35)).length() < 1e-6, "halb {halb:?}");
        let unterkante = mitte.y - halb.y;
        assert!(unterkante.abs() < 1e-6, "Unterkante bei {unterkante} statt 0");
        assert!((mitte.x - 3.0).abs() < 1e-6 && (mitte.z + 4.0).abs() < 1e-6);
    }
}
