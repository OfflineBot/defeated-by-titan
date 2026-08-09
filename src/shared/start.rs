//! Die Start-Flags — **die Tueren, die am Hauptmenue vorbeigehen.**
//!
//! Ein Hauptmenue ist fuer jemanden, der nicht klicken kann, eine Wand ohne Tuer. Deshalb
//! kommt die Pruefinfrastruktur **vor** den Features, nicht „wenn Zeit ist"
//! (`prompts/init.md` §12).
//!
//! Von Hand geparst statt mit `clap`: es sind neun Flags, und eine Dependency, die man
//! nicht braucht, ist eine Dependency, die eines Tages nicht baut — auf dieser Maschine
//! eine sehr konkrete Sorge (`docs/umgebung.md`).

use bevy::prelude::*;
use std::path::PathBuf;

/// Womit das Spiel gestartet wurde. Liegt als `Resource` an, damit jede Domaene es lesen
/// kann, ohne `main.rs` zu kennen.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct Start {
    /// Kein Fenster, fester Tick, laeuft [`Start::ticks`] Schritte und beendet sich mit
    /// einem Exit-Code, der sagt, ob alle `assert` gehalten haben. **Der einzige Weg auf
    /// einer Maschine ohne Grafiksitzung** (§14).
    pub headless: bool,
    /// Leeres Feld, ein Titan, unendlich Gas — zum Anschauen.
    pub sandbox: bool,
    /// Direkt in einen Einsatz, kein Menue.
    pub mission: Option<String>,
    /// Eine Fahrt aus einer Textdatei (§12b). Mit `assert` wird sie zu einem Test.
    pub script: Option<PathBuf>,
    /// Zum Messen. Unter Vsync ist jede Bildzeit 16,6 ms — damit misst „was kostet das?"
    /// sechsmal denselben Deckel (§11).
    pub novsync: bool,
    /// Simulierte Latenz in Millisekunden. **Jedes Bewegungsfeature wird auch bei 200 ms
    /// geprueft** (Bibel T-019) — „fuehlt sich lokal gut an" ist keine Abnahme.
    pub lag_ms: u32,
    /// Obergrenze fuer `--headless`. 0 heisst: bis zum Ende des Skripts.
    pub ticks: u64,
    /// Alle Modelle neu exportieren (§7).
    pub reexport: bool,
    /// Den Export ueberspringen, Startzeit sparen.
    pub no_export: bool,
    /// Was nicht verstanden wurde. Wird beim Start **laut** gemeldet — ein vertipptes Flag,
    /// das still ignoriert wird, kostet eine Stunde Fehlersuche am falschen Ende.
    pub unbekannt: Vec<String>,
}

impl Start {
    pub fn aus_argv() -> Self {
        Self::aus(std::env::args().skip(1))
    }

    pub fn aus(argumente: impl IntoIterator<Item = String>) -> Self {
        let mut s = Start::default();
        let mut es = argumente.into_iter().peekable();
        while let Some(a) = es.next() {
            let mut wert = |s: &mut Start, name: &str| -> Option<String> {
                match es.next() {
                    Some(v) if !v.starts_with("--") => Some(v),
                    andere => {
                        s.unbekannt.push(format!("{name} ohne Wert"));
                        if let Some(v) = andere {
                            // Nicht schlucken: das naechste Flag ist ein Flag.
                            s.unbekannt.push(v);
                        }
                        None
                    }
                }
            };
            match a.as_str() {
                "--headless" => s.headless = true,
                "--sandbox" => s.sandbox = true,
                "--novsync" => s.novsync = true,
                "--reexport" => s.reexport = true,
                "--no-export" => s.no_export = true,
                "--mission" => s.mission = wert(&mut s, "--mission"),
                "--script" => s.script = wert(&mut s, "--script").map(PathBuf::from),
                "--lag" => {
                    if let Some(v) = wert(&mut s, "--lag") {
                        match v.parse() {
                            Ok(n) => s.lag_ms = n,
                            Err(_) => s.unbekannt.push(format!("--lag {v} ist keine Zahl")),
                        }
                    }
                }
                "--ticks" => {
                    if let Some(v) = wert(&mut s, "--ticks") {
                        match v.parse() {
                            Ok(n) => s.ticks = n,
                            Err(_) => s.unbekannt.push(format!("--ticks {v} ist keine Zahl")),
                        }
                    }
                }
                andere => s.unbekannt.push(andere.to_string()),
            }
        }
        // Eine Fahrt ohne Fenster braucht kein Flag mehr als noetig: wer ein Skript faehrt
        // und keine Grafiksitzung hat, meint --headless.
        if s.script.is_some() && !grafiksitzung_vorhanden() {
            s.headless = true;
        }
        s
    }

    /// Ob ueberhaupt ein Fenster geoeffnet werden soll.
    pub fn will_fenster(&self) -> bool {
        !self.headless
    }
}

/// Ob es eine Grafiksitzung gibt.
///
/// Ohne sie panikt ein Fenster-Start sofort und tief in winit — eine Meldung, die aussieht
/// wie ein Bug im Spiel. **Lieber vorher pruefen und einen Satz sagen, den man versteht**
/// (§12d, `docs/umgebung.md`).
pub fn grafiksitzung_vorhanden() -> bool {
    let gesetzt = |k: &str| std::env::var(k).is_ok_and(|v| !v.is_empty());
    gesetzt("WAYLAND_DISPLAY") || gesetzt("DISPLAY")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start(args: &[&str]) -> Start {
        Start::aus(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn flags_werden_gelesen() {
        let s = start(&["--sandbox", "--novsync", "--lag", "200", "--ticks", "600"]);
        assert!(s.sandbox);
        assert!(s.novsync);
        assert_eq!(s.lag_ms, 200);
        assert_eq!(s.ticks, 600);
        assert!(s.unbekannt.is_empty(), "unerwartet: {:?}", s.unbekannt);
    }

    #[test]
    fn ein_vertipptes_flag_wird_gemeldet_und_nicht_geschluckt() {
        // Der Sonderfall, nicht der Normalfall: still ignorierte Flags kosten eine Stunde
        // Fehlersuche am falschen Ende.
        let s = start(&["--sandkasten"]);
        assert_eq!(s.unbekannt, vec!["--sandkasten".to_string()]);
        assert!(!s.sandbox);
    }

    #[test]
    fn ein_wert_der_fehlt_frisst_nicht_das_naechste_flag() {
        let s = start(&["--mission", "--sandbox"]);
        assert!(s.mission.is_none());
        assert!(!s.unbekannt.is_empty());
        // --sandbox darf nicht als Missionsname verschwunden sein
        assert!(s.unbekannt.iter().any(|u| u == "--sandbox"));
    }

    #[test]
    fn lag_mit_unsinn_setzt_nicht_still_null() {
        let s = start(&["--lag", "viel"]);
        assert_eq!(s.lag_ms, 0);
        assert!(s.unbekannt.iter().any(|u| u.contains("keine Zahl")));
    }

    #[test]
    fn leere_argumente_ergeben_die_vorgabe() {
        assert_eq!(start(&[]), Start::default());
    }
}
