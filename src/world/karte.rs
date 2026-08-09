//! Die Stadt aus `assets/data/maps.ron` — **Daten und ein Seed, keine 200 Zeilen Rust.**
//!
//! Gebaut wird aus zwei Quellen:
//! 1. `kloetze` — ausdruecklich gesetzte Quader, 1:1 aus der Datei.
//! 2. `raster` — deterministisch erzeugte Bloecke aus `seed` ueber
//!    [`Wuerfel`](crate::shared::Wuerfel). Derselbe Seed ergibt dieselbe Stadt, auf jedem
//!    Rechner und in jedem Rollback; `rand::random()` waere hier ein Desync.
//!
//! Jede Entity bekommt [`Bauklotz`] (das sieht `render`), [`Koerper`] (das sieht der
//! raeumliche Index), die avian-Bausteine [`RigidBody::Static`] und [`Collider::cuboid`] und
//! bei `hakbar` zusaetzlich [`Ankerflaeche`]. **Ein Schreiber fuer alle vier**, damit
//! Renderform, Indexhuelle und Kollisionsform nicht auseinanderlaufen koennen.
//!
//! ⚠️ Die avian-Komponenten sind heute **wirkungslos**: `PhysicsPlugins` ist in
//! `src/lib.rs` nicht registriert. Sie sind trotzdem jetzt richtig statt spaeter falsch —
//! und `tests/world.rs` misst ihre Form, nicht ihre Wirkung.
//!
//! ## Die Falle, die im Bild nicht auffaellt
//!
//! `Collider::cuboid` nimmt die **GANZE Kante**, nicht die halbe:
//! `avian3d-0.7.0/src/collision/collider/parry/mod.rs:747-749` ruft
//! `SharedShape::cuboid(x_length * 0.5, ..)` — parry haelt intern die halbe, avian nimmt
//! aussen die volle. [`Koerper::halb_m`] und `Aabb3d::new` nehmen dagegen die **halbe**
//! (`bevy_math-0.19.0/src/bounding/bounded3d/mod.rs:66`). Ein Faktor 2 an dieser Stelle
//! macht jedes Haus doppelt oder halb so gross, ohne dass es im Bild auffaellt — deshalb
//! misst `tests/world.rs::f003_die_collider_tragen_die_halbe_kante_aus_der_datei` die Form
//! gegen die Datei.
//!
//! ## Warum das Raster den Boden nicht bemerkt
//!
//! `maps.ron` sagt: „das Erzeugte laesst um jeden gesetzten Klotz Platz". Der erste gesetzte
//! Klotz ist die 400 x 400 m grosse Bodenplatte — eine Sonderregel fuer sie waere ein
//! `if boden`, das nie wieder jemand versteht. Stattdessen prueft [`ueberlappt`] **strikt**
//! (Beruehren zaehlt nicht): ein Haus steht auf y = 0 auf der Platte, deren Oberkante bei
//! y = 0 liegt, und beruehrt sie damit nur. Kein Sonderfall, sondern Geometrie.
//!
//! **Kein Klotz wird gedreht.** Ein achsenparalleler Quader ist exakt seine AABB; eine
//! gedrehte `Cuboid` liefert die umschliessende, zu grosse Huelle
//! (`bevy_math-0.19.0/src/bounding/bounded3d/primitive_impls.rs:100-115`), und der Haken
//! faengt sichtbar in der Luft. Das ist eine bewusst aufgeschobene Einschraenkung
//! (`docs/ROADMAP.md`), keine vergessene.
//!
//! Gesehen: `docs/bilder/f003-stadt.png`, gefahren mit `scripts/f003-stadt.txt`.

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;

use crate::data::{GameData, Karte};
use crate::shared::{Ankerflaeche, Bauklotz, Koerper, Wuerfel};

use super::index::maske_aus;

/// Vier Fragen an denselben Platz, vier Stroeme.
///
/// [`Wuerfel`] ist zustandslos und rechnet aus `(seed, tick, strom)`; **zwei Verwender mit
/// demselben Strom bekommen dieselbe Zahl** (`src/shared/zufall.rs`). Waere die Hoehe
/// derselbe Strom wie die Farbe, haetten alle hohen Haeuser dieselbe Farbe — ein Muster,
/// das man im Bild fuer Absicht haelt.
///
/// Das sind **keine Spielwerte**, sondern Namen: sie unterscheiden Verwender und stehen
/// deshalb im Code und nicht in der RON (§4).
const STROM_BEBAUT: u64 = 0xF003_0001;
const STROM_HOEHE: u64 = 0xF003_0002;
const STROM_FARBE: u64 = 0xF003_0003;
const STROM_HAKBAR: u64 = 0xF003_0004;

/// Ein geplanter Quader, **bevor** er eine Entity ist.
///
/// Der Plan ist getrennt vom Spawnen, damit `tests/world.rs` die Stadt zweimal erzeugen und
/// Wert fuer Wert vergleichen kann, ohne zwei Apps zu bauen — Determinismus ist die
/// Eigenschaft, die man am billigsten verliert und am teuersten sucht.
#[derive(Clone, Debug, PartialEq)]
pub struct Rohbau {
    /// `klotz_<i>` fuer einen gesetzten, `haus_<platz>` fuer einen erzeugten Quader. Der
    /// Platz ist die **Nummer der Rasterzelle**, nicht die Reihenfolge des Spawnens: eine
    /// Luecke im Namen ist eine unbebaute Zelle und keine verlorene Entity.
    pub name: String,
    /// Weltmitte in Metern.
    pub mitte_m: Vec3,
    /// **Volle** Kantenlaenge in Metern, wie `maps.ron` und [`Bauklotz`] sie fuehren.
    pub groesse_m: Vec3,
    pub farbe: [f32; 3],
    pub hakbar: bool,
    pub fest: bool,
}

impl Rohbau {
    fn halb_m(&self) -> Vec3 {
        self.groesse_m * 0.5
    }

    /// Die **einzige** Stelle, an der aus einem geplanten Quader eine Entity wird.
    fn spawnen(&self, commands: &mut Commands) {
        let mut e = commands.spawn((
            Name::new(self.name.clone()),
            // Was `render` sieht: volle Kante.
            Bauklotz { groesse: self.groesse_m, farbe: self.farbe },
            // Was der raeumliche Index sieht: halbe Kante.
            Koerper { halb_m: self.halb_m(), maske: maske_aus(self.fest, self.hakbar) },
            // Was avian sieht: wieder volle Kante (siehe Modulkopf).
            RigidBody::Static,
            Collider::cuboid(self.groesse_m.x, self.groesse_m.y, self.groesse_m.z),
            Transform::from_translation(self.mitte_m),
        ));
        if self.hakbar {
            e.insert(Ankerflaeche);
        }
    }
}

/// Baut die Karte aus `maps.ron: aktuell` beim `Startup`.
///
/// Ersetzt die bis 2026-08-09 in `world/mod.rs` hart verdrahteten Kloetze. Die ersten
/// Eintraege in `maps.ron` sind genau diese Kloetze — damit der Umbau **verhaltensgleich**
/// nachweisbar ist und nicht „sieht auch gut aus".
pub fn karte_bauen(mut commands: Commands, daten: Res<GameData>) {
    let Some(karte) = daten.aktuelle_karte() else {
        // Laut, nicht still: eine leere Welt sieht exakt wie ein Render-Bug aus (§9d).
        panic!(
            "maps.ron: aktuell = {:?} steht nicht unter `karten` — es gaebe keine Welt, \
             und das saehe wie ein Renderfehler aus",
            daten.karten.aktuell
        );
    };

    let plan = kloetze_planen(&daten, karte);
    let hakbar = plan.iter().filter(|r| r.hakbar).count();
    for rohbau in &plan {
        rohbau.spawnen(&mut commands);
    }
    info!(
        "Karte {:?}: {} Kloetze gebaut ({} gesetzt, {} erzeugt), davon {hakbar} hakbar",
        karte.name,
        plan.len(),
        karte.kloetze.len(),
        plan.len() - karte.kloetze.len(),
    );
}

/// Was gebaut werden soll — **ohne** Bevy, ohne `Commands`, ohne Nebenwirkung.
///
/// Reihenfolge: erst die gesetzten Kloetze in Dateireihenfolge, dann das Raster in
/// Platzreihenfolge. Beides ist geordnet und keine `HashMap` — eine Stadt, die je nach
/// Iterationsreihenfolge anders aussieht, ist im Netz ein Desync.
pub fn kloetze_planen(daten: &GameData, karte: &Karte) -> Vec<Rohbau> {
    let mut plan: Vec<Rohbau> = Vec::new();

    for (i, k) in karte.kloetze.iter().enumerate() {
        plan.push(Rohbau {
            name: format!("klotz_{i}"),
            mitte_m: Vec3::new(k.mitte_m.0, k.mitte_m.1, k.mitte_m.2),
            groesse_m: Vec3::new(k.groesse_m.0, k.groesse_m.1, k.groesse_m.2),
            farbe: farbe_aus(daten, &k.farbe),
            hakbar: k.hakbar,
            fest: k.fest,
        });
    }
    let gesetzt = plan.len();

    let r = &karte.raster;
    let wuerfel = Wuerfel::neu(karte.seed);
    let periode_m = r.block_m + r.gasse_m;
    let nx = plaetze(karte.groesse_m.0, periode_m);
    let nz = plaetze(karte.groesse_m.1, periode_m);
    // Die Bebauung wird auf ihre eigene Ausdehnung zentriert, nicht auf `nx * periode`:
    // hinter dem letzten Block folgt keine Gasse mehr, und ohne diese Korrektur laege die
    // ganze Stadt um eine halbe Gassenbreite schief.
    let start_x = -(nx as f32 * periode_m - r.gasse_m) * 0.5;
    let start_z = -(nz as f32 * periode_m - r.gasse_m) * 0.5;

    for iz in 0..nz {
        for ix in 0..nx {
            // Die Nummer des PLATZES, nicht des Hauses. Sie ist der `tick` fuer den
            // Wuerfel: wer einen Klotz in `maps.ron` nachtraegt, verschiebt damit nicht
            // die Hoehen aller folgenden Haeuser.
            let platz = (iz * nx + ix) as u64;
            let mitte_x = start_x + ix as f32 * periode_m + r.block_m * 0.5;
            let mitte_z = start_z + iz as f32 * periode_m + r.block_m * 0.5;

            if im_freien_radius(mitte_x, mitte_z, r.block_m * 0.5, r.frei_radius_m) {
                continue;
            }
            if !wuerfel.trifft(platz, STROM_BEBAUT, r.dichte) {
                continue;
            }

            let hoehe_m = wuerfel.bereich(platz, STROM_HOEHE, r.hoehe_min_m, r.hoehe_max_m);
            // Ein Haus steht AUF dem Boden: Unterkante y = 0, Mitte auf halber Hoehe.
            let mitte_m = Vec3::new(mitte_x, hoehe_m * 0.5, mitte_z);
            let groesse_m = Vec3::new(r.block_m, hoehe_m, r.block_m);

            // Ausdruecklich Gesetztes gewinnt (`maps.ron`). Geprueft wird nur gegen die
            // gesetzten Kloetze: zwei Rasterhaeuser koennen sich nie ueberlappen, dafuer
            // sorgt die Gasse.
            let stoert = plan[..gesetzt]
                .iter()
                .any(|g| ueberlappt(mitte_m, groesse_m * 0.5, g.mitte_m, g.halb_m()));
            if stoert {
                continue;
            }

            let farben = &r.farben;
            let farbe = farben
                .get(wuerfel.index(platz, STROM_FARBE, farben.len()))
                .unwrap_or_else(|| {
                    panic!("maps.ron: raster.farben ist leer — jedes Haus waere farblos")
                });

            plan.push(Rohbau {
                name: format!("haus_{platz}"),
                mitte_m,
                groesse_m,
                farbe: farbe_aus(daten, farbe),
                hakbar: wuerfel.trifft(platz, STROM_HAKBAR, r.hakbar_anteil),
                // Ein Haus haelt an. Das ist Mechanik und keine Spielwertfrage — es gibt
                // in `maps.ron` bewusst kein `fest_anteil`.
                fest: true,
            });
        }
    }

    plan
}

/// Wie viele Rasterplaetze auf eine Kante passen. `0` heisst: kein Raster.
fn plaetze(kante_m: f32, periode_m: f32) -> u32 {
    if !(kante_m.is_finite() && periode_m.is_finite()) || periode_m <= 0.0 || kante_m <= 0.0 {
        return 0;
    }
    (kante_m / periode_m).floor() as u32
}

/// Ob ein Block dem Ursprung naeher kommt als `frei_radius_m`.
///
/// Gemessen wird vom Ursprung zur **Kante** des Blocks, nicht zu seiner Mitte: sonst haengt
/// der freie Platz an der Blockgroesse, und `frei_radius_m` waere keine Zusage mehr.
fn im_freien_radius(mitte_x: f32, mitte_z: f32, halb_m: f32, frei_radius_m: f32) -> bool {
    let dx = (mitte_x.abs() - halb_m).max(0.0);
    let dz = (mitte_z.abs() - halb_m).max(0.0);
    dx * dx + dz * dz < frei_radius_m * frei_radius_m
}

/// Strikte Ueberlappung zweier achsenparalleler Quader — **Beruehren zaehlt nicht.**
///
/// Genau daran laeuft die Bodenplatte vorbei: ein Haus mit Unterkante y = 0 und eine Platte
/// mit Oberkante y = 0 haben auf der Y-Achse `abstand == summe`, und `<` ist falsch. Beide
/// Seiten rechnen dieselbe Summe aus denselben Gleitkommazahlen, das Ergebnis ist also
/// exakt gleich und nicht „fast".
fn ueberlappt(a_mitte: Vec3, a_halb: Vec3, b_mitte: Vec3, b_halb: Vec3) -> bool {
    let abstand = (a_mitte - b_mitte).abs();
    let summe = a_halb + b_halb;
    abstand.x < summe.x && abstand.y < summe.y && abstand.z < summe.z
}

/// Eine Farbe aus dem einen Farbatlas — oder ein Abbruch mit dem Namen, der fehlt.
///
/// Kein stiller Ersatz: sonst rutscht irgendwann eine der drei Signalfarben in die Deko
/// (`docs/konventionen.md`).
fn farbe_aus(daten: &GameData, name: &str) -> [f32; 3] {
    daten.farbe(name).unwrap_or_else(|| {
        panic!("maps.ron: Farbe {name:?} steht nicht in `palette`")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f003_ein_haus_steht_auf_der_bodenplatte_und_nicht_in_ihr() {
        // Der Grund, warum das Raster ueberhaupt etwas baut: die Bodenplatte ist der erste
        // gesetzte Klotz und deckt die ganze Karte. Waere `ueberlappt` nicht strikt, waere
        // die Stadt leer — und zwar ohne eine einzige Fehlermeldung.
        let platte_mitte = Vec3::new(0.0, -0.1, 0.0);
        let platte_halb = Vec3::new(200.0, 0.1, 200.0);
        for hoehe_m in [4.5f32, 7.3, 11.5, 35.0] {
            let haus_mitte = Vec3::new(0.0, hoehe_m * 0.5, 0.0);
            let haus_halb = Vec3::new(14.0, hoehe_m * 0.5, 14.0);
            assert!(
                !ueberlappt(haus_mitte, haus_halb, platte_mitte, platte_halb),
                "Haus mit {hoehe_m} m Hoehe steckt angeblich in der Bodenplatte"
            );
        }
        // Und ein Keller steckt sehr wohl drin.
        assert!(ueberlappt(
            Vec3::new(0.0, -0.1, 0.0),
            Vec3::splat(2.0),
            platte_mitte,
            platte_halb
        ));
    }

    #[test]
    fn f003_der_freie_radius_misst_zur_kante_und_nicht_zur_mitte() {
        // Ein Block, dessen Mitte 30 m weg ist, dessen Kante aber 16 m: er steht im Weg.
        assert!(im_freien_radius(30.0, 0.0, 14.0, 24.0), "Kante bei 16 m, Radius 24 m");
        assert!(!im_freien_radius(40.0, 0.0, 14.0, 24.0), "Kante bei 26 m, Radius 24 m");
        // Diagonal zaehlt der echte Abstand, nicht der groessere der beiden Achsen.
        assert!(!im_freien_radius(35.0, 35.0, 14.0, 24.0), "Kante diagonal bei 29,7 m");
    }

    #[test]
    fn f003_die_platzzahl_laesst_die_letzte_gasse_weg() {
        // 400 m bei 28 + 7: elf Bloecke sind 385 m, zwoelf waeren 420 m.
        assert_eq!(plaetze(400.0, 35.0), 11);
        assert_eq!(plaetze(35.0, 35.0), 1);
        assert_eq!(plaetze(34.0, 35.0), 0);
        // Kein Sturz bei Unsinn, sondern kein Raster.
        assert_eq!(plaetze(400.0, 0.0), 0);
        assert_eq!(plaetze(f32::NAN, 35.0), 0);
    }
}
