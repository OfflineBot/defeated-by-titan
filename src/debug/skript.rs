//! Der `--script`-Fahrer: das Spiel spielen, ohne zu tippen.
//!
//! **Das ist der Punkt, an dem solche Projekte scheitern:** alles ist gebaut, nichts ist
//! gesehen, weil jedes Feature hinter Maus und Tastatur liegt und niemand am Keyboard sitzt.
//! Also kommt die Pruefinfrastruktur **vor** den Features (`prompts/init.md` §12).
//!
//! Eine Textdatei, eine Anweisung pro Zeile:
//!
//! ```text
//! spawn titan husk 20 0 -40   # Typ und Ort in Metern
//! look 0 -10                  # Blickrichtung in Grad (yaw, pitch)
//! key Space 0.3               # Taste 0,3 s halten
//! hook left                   # Haken raus
//! wait 1.2                    # Commands sind verzoegert — sonst fotografiert man ein leeres Feld
//! mark eingehakt              # eine Zeile ins Log, an der man einen Screenshot ausrichtet
//! assert speed > 25           # ⭐ das Skript darf selbst urteilen: faellt es um, ist es ein Test
//! ```
//!
//! `assert` ist der Grund, warum das mehr ist als eine Demo: damit wird eine **Fahrt** zu
//! einem Test — und Bewegungsgefuehl ist genau die Sorte Sache, die kein Unit-Test greift.
//!
//! Der Fahrer schreibt in **dieselben Eingaben, die ein Mensch ausloest**
//! (`ButtonInput<KeyCode>`, `ButtonInput<MouseButton>`) — **kein zweiter, falscher Weg zu
//! spielen.** Einzige Ausnahme ist der Blick: dafuer gibt es einen „so-tun-als"-Vektor,
//! weil eine Maus keinen absoluten Winkel kennt.

use bevy::prelude::*;

/// Eine Anweisung aus der Skriptdatei, samt Zeilennummer fuer die Fehlermeldung.
#[derive(Clone, Debug, PartialEq)]
pub struct Anweisung {
    pub zeile: usize,
    pub was: Befehl,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Befehl {
    /// `spawn titan <art> <x> <y> <z>`
    SpawnTitan { art: String, pos: Vec3 },
    /// `warp <x> <y> <z>` — der Spieler steht danach genau dort (§12c)
    Warp(Vec3),
    /// `look <yaw_grad> <pitch_grad>`
    Blick { yaw_grad: f32, pitch_grad: f32 },
    /// `key <Name> <sekunden>` — eine echte Taste halten
    Taste { code: KeyCode, dauer_s: f32 },
    /// `hook left|right <sekunden>` — eine echte Maustaste halten
    Haken { rechts: bool, dauer_s: f32 },
    /// `wait <sekunden>`
    Warten(f32),
    /// `mark <text>`
    Marke(String),
    /// `assert <groesse> <vergleich> <wert>`
    Pruefe { groesse: Groesse, vergleich: Vergleich, wert: f32 },
    /// `end` — vorzeitig beenden
    Ende,
}

/// Was ein `assert` messen kann. Bewusst wenige und **alle messbar** — eine Groesse, die
/// niemand nachrechnen kann, ist kein Pruefkriterium (§17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Groesse {
    /// Tempo des lokalen Spielers in m/s
    Tempo,
    /// Hoehe des lokalen Spielers in Metern
    Hoehe,
    /// Gas des lokalen Spielers, absolut
    Gas,
    /// Anzahl lebender Titanen
    Titanen,
    /// Der Simulationstick
    Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vergleich {
    Groesser,
    GroesserGleich,
    Kleiner,
    KleinerGleich,
    Gleich,
}

impl Vergleich {
    pub fn haelt(self, ist: f32, soll: f32) -> bool {
        match self {
            Vergleich::Groesser => ist > soll,
            Vergleich::GroesserGleich => ist >= soll,
            Vergleich::Kleiner => ist < soll,
            Vergleich::KleinerGleich => ist <= soll,
            // Gleitkomma vergleicht man nicht auf Gleichheit; eine Toleranz von 1e-3 ist
            // fuer Meter, m/s und Gas die Groessenordnung „egal".
            Vergleich::Gleich => (ist - soll).abs() <= 1e-3,
        }
    }

    pub fn zeichen(self) -> &'static str {
        match self {
            Vergleich::Groesser => ">",
            Vergleich::GroesserGleich => ">=",
            Vergleich::Kleiner => "<",
            Vergleich::KleinerGleich => "<=",
            Vergleich::Gleich => "==",
        }
    }
}

/// Ein Fehler beim Lesen der Datei — **mit Zeilennummer**. Ein Skript, das still eine Zeile
/// ueberspringt, ist schlimmer als eines, das gar nicht laeuft: die Fahrt sieht dann gruen
/// aus und hat die Haelfte nicht getan.
#[derive(Debug, PartialEq)]
pub struct Lesefehler {
    pub zeile: usize,
    pub text: String,
    pub grund: String,
}

impl std::fmt::Display for Lesefehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Zeile {}: {} — {:?}", self.zeile, self.grund, self.text)
    }
}

pub fn lesen(inhalt: &str) -> Result<Vec<Anweisung>, Vec<Lesefehler>> {
    let mut fertig = Vec::new();
    let mut fehler = Vec::new();

    for (i, roh) in inhalt.lines().enumerate() {
        let zeile = i + 1;
        let ohne_kommentar = roh.split('#').next().unwrap_or("").trim();
        if ohne_kommentar.is_empty() {
            continue;
        }
        match zeile_lesen(ohne_kommentar) {
            Ok(was) => fertig.push(Anweisung { zeile, was }),
            Err(grund) => fehler.push(Lesefehler {
                zeile,
                text: roh.trim().to_string(),
                grund,
            }),
        }
    }

    if fehler.is_empty() { Ok(fertig) } else { Err(fehler) }
}

fn zahl(t: Option<&&str>, was: &str) -> Result<f32, String> {
    t.ok_or_else(|| format!("{was} fehlt"))?
        .parse()
        .map_err(|_| format!("{was} ist keine Zahl: {:?}", t.unwrap()))
}

fn zeile_lesen(z: &str) -> Result<Befehl, String> {
    let t: Vec<&str> = z.split_whitespace().collect();
    match t.first().copied().unwrap_or("") {
        "spawn" => {
            if t.get(1).copied() != Some("titan") {
                return Err("nur `spawn titan <art> <x> <y> <z>` ist bekannt".into());
            }
            let art = t.get(2).ok_or("Titan-Art fehlt")?.to_string();
            Ok(Befehl::SpawnTitan {
                art,
                pos: Vec3::new(
                    zahl(t.get(3), "x")?,
                    zahl(t.get(4), "y")?,
                    zahl(t.get(5), "z")?,
                ),
            })
        }
        "warp" => Ok(Befehl::Warp(Vec3::new(
            zahl(t.get(1), "x")?,
            zahl(t.get(2), "y")?,
            zahl(t.get(3), "z")?,
        ))),
        "look" => Ok(Befehl::Blick {
            yaw_grad: zahl(t.get(1), "yaw")?,
            pitch_grad: zahl(t.get(2), "pitch")?,
        }),
        "key" => Ok(Befehl::Taste {
            code: taste_lesen(t.get(1).ok_or("Tastenname fehlt")?)?,
            dauer_s: zahl(t.get(2), "Dauer")?,
        }),
        "hook" => {
            let seite = *t.get(1).ok_or("`left` oder `right` fehlt")?;
            let rechts = match seite {
                "left" | "links" => false,
                "right" | "rechts" => true,
                andere => return Err(format!("Seite {andere:?} — erlaubt: left, right")),
            };
            Ok(Befehl::Haken {
                rechts,
                // Ohne Angabe: ein Tick. Ein Haken ist ein Druck, kein Dauerfeuer.
                dauer_s: if t.len() > 2 { zahl(t.get(2), "Dauer")? } else { 0.05 },
            })
        }
        "wait" => Ok(Befehl::Warten(zahl(t.get(1), "Dauer")?)),
        "mark" => {
            let text = t[1..].join(" ");
            if text.is_empty() {
                return Err("`mark` ohne Text — eine Marke ohne Namen ist keine".into());
            }
            Ok(Befehl::Marke(text))
        }
        "assert" => {
            let groesse = match *t.get(1).ok_or("Groesse fehlt")? {
                "speed" | "tempo" => Groesse::Tempo,
                "height" | "hoehe" => Groesse::Hoehe,
                "gas" => Groesse::Gas,
                "titans" | "titanen" => Groesse::Titanen,
                "tick" => Groesse::Tick,
                andere => {
                    return Err(format!(
                        "Groesse {andere:?} ist nicht messbar — bekannt: \
                         speed, hoehe, gas, titanen, tick"
                    ));
                }
            };
            let vergleich = match *t.get(2).ok_or("Vergleich fehlt")? {
                ">" => Vergleich::Groesser,
                ">=" => Vergleich::GroesserGleich,
                "<" => Vergleich::Kleiner,
                "<=" => Vergleich::KleinerGleich,
                "==" => Vergleich::Gleich,
                andere => return Err(format!("Vergleich {andere:?} — erlaubt: > >= < <= ==")),
            };
            Ok(Befehl::Pruefe {
                groesse,
                vergleich,
                wert: zahl(t.get(3), "Vergleichswert")?,
            })
        }
        "end" => Ok(Befehl::Ende),
        andere => Err(format!("unbekannter Befehl {andere:?}")),
    }
}

/// Nur die Tasten, die das Spiel wirklich benutzt. Eine vollstaendige Tabelle waere
/// dreihundert Zeilen, die niemand pflegt — wer eine neue Taste belegt, traegt sie hier ein.
fn taste_lesen(name: &str) -> Result<KeyCode, String> {
    Ok(match name {
        "W" | "w" => KeyCode::KeyW,
        "A" | "a" => KeyCode::KeyA,
        "S" | "s" => KeyCode::KeyS,
        "D" | "d" => KeyCode::KeyD,
        "Q" | "q" => KeyCode::KeyQ,
        "E" | "e" => KeyCode::KeyE,
        "C" | "c" => KeyCode::KeyC,
        "F" | "f" => KeyCode::KeyF,
        "Space" | "space" => KeyCode::Space,
        "Shift" | "shift" => KeyCode::ShiftLeft,
        "Ctrl" | "ctrl" | "Strg" | "strg" => KeyCode::ControlLeft,
        andere => {
            return Err(format!(
                "Taste {andere:?} unbekannt — bekannt: W A S D Q E C F Space Shift Ctrl"
            ));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn das_beispiel_aus_der_doku_laesst_sich_lesen() {
        let s = "\
spawn titan husk 20 0 -40   # Typ und Ort in Metern
look 0 -10
key Space 0.3
hook left
wait 1.2
mark eingehakt
assert speed > 25
";
        let a = lesen(s).expect("das Beispiel muss lesbar sein");
        assert_eq!(a.len(), 7);
        assert_eq!(
            a[0].was,
            Befehl::SpawnTitan { art: "husk".into(), pos: Vec3::new(20.0, 0.0, -40.0) }
        );
        assert_eq!(a[5].was, Befehl::Marke("eingehakt".into()));
        assert_eq!(
            a[6].was,
            Befehl::Pruefe { groesse: Groesse::Tempo, vergleich: Vergleich::Groesser, wert: 25.0 }
        );
    }

    #[test]
    fn kommentare_und_leerzeilen_verschwinden_ohne_fehler() {
        let a = lesen("# nur ein Kommentar\n\n   \nmark da\n").expect("gueltig");
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].zeile, 4, "die Zeilennummer muss die ECHTE sein");
    }

    #[test]
    fn ein_tippfehler_wird_gemeldet_und_nicht_uebersprungen() {
        // Der eigentliche Zweck der Fehlerliste: ein Skript, das still eine Zeile
        // ueberspringt, sieht gruen aus und hat die Haelfte nicht getan.
        let f = lesen("mark eins\nspwan titan husk 0 0 0\nmark zwei\n")
            .expect_err("muss fehlschlagen");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].zeile, 2);
        assert!(f[0].grund.contains("unbekannter Befehl"));
    }

    #[test]
    fn alle_fehler_kommen_auf_einmal() {
        let f = lesen("assert wolke > 1\nkey Umlaut 1\nhook oben\n").expect_err("drei Fehler");
        assert_eq!(f.len(), 3, "nicht nur der erste Fehler, sondern alle");
    }

    #[test]
    fn fehlende_zahlen_sind_ein_fehler_und_keine_null() {
        let f = lesen("wait\n").expect_err("Dauer fehlt");
        assert!(f[0].grund.contains("fehlt"));
        let f = lesen("look 0\n").expect_err("pitch fehlt");
        assert!(f[0].grund.contains("pitch"));
        let f = lesen("warp 1 zwei 3\n").expect_err("keine Zahl");
        assert!(f[0].grund.contains("keine Zahl"));
    }

    #[test]
    fn hook_ohne_dauer_ist_ein_druck_kein_dauerfeuer() {
        let a = lesen("hook right").expect("gueltig");
        assert_eq!(a[0].was, Befehl::Haken { rechts: true, dauer_s: 0.05 });
    }

    #[test]
    fn mark_ohne_text_ist_keine_marke() {
        assert!(lesen("mark").is_err());
        assert!(lesen("mark   # nur ein Kommentar").is_err());
    }

    #[test]
    fn vergleiche_rechnen_richtig() {
        assert!(Vergleich::Groesser.haelt(26.0, 25.0));
        assert!(!Vergleich::Groesser.haelt(25.0, 25.0));
        assert!(Vergleich::GroesserGleich.haelt(25.0, 25.0));
        assert!(Vergleich::KleinerGleich.haelt(25.0, 25.0));
        assert!(Vergleich::Gleich.haelt(25.0005, 25.0), "Gleitkomma braucht Toleranz");
        assert!(!Vergleich::Gleich.haelt(25.1, 25.0));
    }
}
