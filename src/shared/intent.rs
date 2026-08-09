//! `Intent` — **Eingabe ist ein Datum, kein Tastendruck.**
//!
//! Es gibt genau ein Struct, und die Simulation liest **nur** das. Wer es fuellt, ist ihr
//! egal: die lokale Tastatur, der `--script`-Fahrer oder spaeter das Netz. Genau dieser
//! Kanal ist der, den Multiplayer braucht — und er wird ohnehin gebaut, weil in dieser
//! Umgebung niemand klicken kann. **Ein Aufwand, zwei Probleme geloest**
//! (`prompts/init.md` §6 Regel 2, §12).
//!
//! Bewusst **keine `Vec2`/`Vec3`-Felder**: dieser Typ geht eines Tages ueber eine Leitung
//! und wird gespeichert. Nackte `f32` sind das, was `serde` ohne Zusatzfeature kann, und
//! sie sagen genau, wie viele Bytes es sind (§6 Regel 8).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Was ein Spieler in **einem** Simulationstick will.
///
/// Haengt als Component am Spieler — nicht als `Resource`, denn es gibt keinen „den
/// Spieler" (§6 Regel 3).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Intent {
    /// Bewegung in der Ebene, spielerlokal: `x` nach rechts, `y` nach vorn, je -1..1.
    pub bewegen_x: f32,
    pub bewegen_y: f32,
    /// Blickrichtung in **Radiant**. `yaw = 0` heisst Blick nach −Z
    /// (`docs/konventionen.md`).
    pub yaw: f32,
    /// Nach oben positiv, geklemmt auf ±89°.
    pub pitch: f32,
    /// Gedrueckte Tasten als Bitmuster.
    pub tasten: Tasten,
    /// Welcher Simulationstick. Der Server verwirft spaeter alles, was zu alt ist.
    pub tick: u64,
}

impl Intent {
    pub fn bewegen(&self) -> Vec2 {
        Vec2::new(self.bewegen_x, self.bewegen_y)
    }

    /// Blickrichtung als Einheitsvektor. `yaw = 0, pitch = 0` ergibt −Z.
    pub fn blick(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(-sy * cp, sp, -cy * cp)
    }

    pub fn haelt(&self, taste: Tasten) -> bool {
        self.tasten.haelt(taste)
    }
}

/// Die Tasten als Bitmuster — ein `u32` statt eines `HashSet`, damit ein `Intent` eine
/// feste Groesse hat und ueber eine Leitung passt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tasten(pub u32);

impl Tasten {
    pub const KEINE: Tasten = Tasten(0);
    pub const SPRINGEN: Tasten = Tasten(1 << 0);
    /// Haken links / rechts — zwei **unabhaengig** steuerbare Haken (`F-001`).
    pub const HAKEN_LINKS: Tasten = Tasten(1 << 1);
    pub const HAKEN_RECHTS: Tasten = Tasten(1 << 2);
    /// Seil einholen (Reel-In, `F-005`) — verbraucht Gas.
    pub const EINHOLEN: Tasten = Tasten(1 << 3);
    /// Gas-Boost. Gas verbrauchen ist laut — der Bellower reagiert darauf (Bibel 4).
    pub const BOOST: Tasten = Tasten(1 << 4);
    pub const SCHNITT_LINKS: Tasten = Tasten(1 << 5);
    pub const SCHNITT_RECHTS: Tasten = Tasten(1 << 6);
    pub const AUSWEICHEN: Tasten = Tasten(1 << 7);
    pub const MARKIEREN: Tasten = Tasten(1 << 8);

    pub fn haelt(self, andere: Tasten) -> bool {
        self.0 & andere.0 == andere.0 && andere.0 != 0
    }

    pub fn setzen(&mut self, andere: Tasten, gedrueckt: bool) {
        if gedrueckt {
            self.0 |= andere.0;
        } else {
            self.0 &= !andere.0;
        }
    }

    /// Welche Tasten in `self` gedrueckt sind, die in `vorher` noch nicht gedrueckt waren.
    /// Der Unterschied zwischen „haelt" und „hat gerade gedrueckt" ist der Unterschied
    /// zwischen Dauerfeuer und einem Schuss.
    pub fn frisch(self, vorher: Tasten) -> Tasten {
        Tasten(self.0 & !vorher.0)
    }
}

/// Ein absoluter Blickwinkel, den jemand von aussen vorgibt (`look 0 -10` im Skript).
///
/// Wird beim Lesen **entnommen**, nicht kopiert: eine Vorgabe gilt einmal und laesst danach
/// wieder die Maus ans Ruder. Ohne das koennte ein Skript den Blick versehentlich
/// festnageln, und niemand saehe, warum sich die Kamera nicht mehr bewegt.
#[derive(Resource, Debug, Default)]
pub struct BlickVorgabe(pub Option<(f32, f32)>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tasten_halten_und_loesen() {
        let mut t = Tasten::KEINE;
        assert!(!t.haelt(Tasten::BOOST));
        t.setzen(Tasten::BOOST, true);
        t.setzen(Tasten::HAKEN_LINKS, true);
        assert!(t.haelt(Tasten::BOOST));
        assert!(t.haelt(Tasten::HAKEN_LINKS));
        assert!(!t.haelt(Tasten::HAKEN_RECHTS));
        t.setzen(Tasten::BOOST, false);
        assert!(!t.haelt(Tasten::BOOST));
        assert!(t.haelt(Tasten::HAKEN_LINKS));
    }

    #[test]
    fn keine_taste_haelt_niemals() {
        // Sonst waere `haelt(KEINE)` immer wahr und jede Abfrage nach „nichts gedrueckt"
        // wuerde stillschweigend jeden Frame ausloesen.
        assert!(!Tasten::KEINE.haelt(Tasten::KEINE));
        assert!(!Tasten(0xffff_ffff).haelt(Tasten::KEINE));
    }

    #[test]
    fn frisch_meldet_nur_den_uebergang() {
        let vorher = Tasten::BOOST;
        let jetzt = Tasten(Tasten::BOOST.0 | Tasten::SPRINGEN.0);
        assert!(jetzt.frisch(vorher).haelt(Tasten::SPRINGEN));
        assert!(!jetzt.frisch(vorher).haelt(Tasten::BOOST));
    }

    #[test]
    fn blick_null_zeigt_nach_minus_z() {
        // Der Achsen-Vertrag aus docs/konventionen.md. Faellt er, steht jedes Modell
        // falsch herum und niemand weiss, warum.
        let i = Intent::default();
        let b = i.blick();
        assert!((b - Vec3::NEG_Z).length() < 1e-6, "blick war {b:?}");
    }

    #[test]
    fn blick_ist_immer_eine_einheit() {
        for yaw in [-3.0_f32, -1.0, 0.0, 0.7, 2.9] {
            for pitch in [-1.5_f32, -0.3, 0.0, 0.3, 1.5] {
                let i = Intent { yaw, pitch, ..default() };
                let l = i.blick().length();
                assert!((l - 1.0).abs() < 1e-5, "yaw {yaw} pitch {pitch} ergab Laenge {l}");
            }
        }
    }
}
