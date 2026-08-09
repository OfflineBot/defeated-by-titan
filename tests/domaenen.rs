//! Der Waechter ueber der Domaenenregel.
//!
//! **Diese Regel verfaellt still** — nichts geht kaputt, wenn jemand doch quer greift, und
//! man merkt es erst, wenn eine Domaene sich nicht mehr allein anfassen laesst. Deshalb
//! dieser Test: er liest die Dateien unter `src/`, sammelt jede Kante `crate::<domaene>` und
//! faellt um, wenn eine nicht in der Erlaubnisliste steht (`prompts/init.md` §5 Regel 6).
//!
//! **Die Erlaubnisliste steht in `docs/architektur.md`, nicht hier.** Eine Regel, die an zwei
//! Orten steht, ist nach vier Wochen an einem Ort falsch — also liest der Test die Doku.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn wurzel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Was jede Domaene ohne Eintrag benutzen darf.
const FREI: [&str; 2] = ["shared", "data"];

fn domaenen() -> BTreeSet<String> {
    let mut d = BTreeSet::new();
    for eintrag in std::fs::read_dir(wurzel().join("src")).expect("src/ muss lesbar sein") {
        let eintrag = eintrag.expect("Verzeichniseintrag");
        if eintrag.path().is_dir() {
            d.insert(eintrag.file_name().to_string_lossy().to_string());
        }
    }
    d
}

fn rust_dateien(ordner: &Path) -> Vec<PathBuf> {
    let mut fertig = Vec::new();
    let mut offen = vec![ordner.to_path_buf()];
    while let Some(p) = offen.pop() {
        for eintrag in std::fs::read_dir(&p).expect("Ordner lesbar") {
            let pfad = eintrag.expect("Eintrag").path();
            if pfad.is_dir() {
                offen.push(pfad);
            } else if pfad.extension().is_some_and(|e| e == "rs") {
                fertig.push(pfad);
            }
        }
    }
    fertig
}

/// Liest den ```erlaubnis-Block aus `docs/architektur.md`.
fn erlaubnisliste() -> BTreeSet<(String, String)> {
    let doku = std::fs::read_to_string(wurzel().join("docs/architektur.md"))
        .expect("docs/architektur.md muss es geben — dort steht die Erlaubnisliste");
    let mut drin = false;
    let mut kanten = BTreeSet::new();
    for zeile in doku.lines() {
        if zeile.trim_start().starts_with("```erlaubnis") {
            drin = true;
            continue;
        }
        if drin && zeile.trim_start().starts_with("```") {
            break;
        }
        if !drin {
            continue;
        }
        let ohne_kommentar = zeile.split('#').next().unwrap_or("").trim();
        if ohne_kommentar.is_empty() {
            continue;
        }
        let Some((von, nach)) = ohne_kommentar.split_once("->") else {
            panic!(
                "docs/architektur.md, Erlaubnisliste: {ohne_kommentar:?} passt nicht auf \
                 `von -> nach   # Begruendung`"
            );
        };
        kanten.insert((von.trim().to_string(), nach.trim().to_string()));
    }
    assert!(drin, "docs/architektur.md hat keinen ```erlaubnis-Block mehr");
    kanten
}

#[test]
fn t003_keine_domaene_greift_ohne_erlaubnis_quer() {
    let domaenen = domaenen();
    let erlaubt = erlaubnisliste();
    let mut verstoesse = Vec::new();

    for domaene in &domaenen {
        if domaene == "shared" {
            continue; // shared gehoert niemandem und darf niemanden kennen
        }
        for datei in rust_dateien(&wurzel().join("src").join(domaene)) {
            let text = std::fs::read_to_string(&datei).expect("Datei lesbar");
            for (nr, zeile) in text.lines().enumerate() {
                // Doc-Kommentare duerfen auf andere Domaenen VERWEISEN — ein Link ist
                // keine Abhaengigkeit. Nur echter Code zaehlt.
                let code = zeile.trim_start();
                if code.starts_with("//") {
                    continue;
                }
                for ziel in &domaenen {
                    if ziel == domaene || FREI.contains(&ziel.as_str()) {
                        continue;
                    }
                    let muster = format!("crate::{ziel}::");
                    if code.contains(&muster)
                        && !erlaubt.contains(&(domaene.clone(), ziel.clone()))
                    {
                        verstoesse.push(format!(
                            "{}:{} — {domaene} greift auf {ziel} zu. Entweder ueber eine \
                             Message in shared/ gehen, oder eine Zeile `{domaene} -> {ziel}` \
                             mit Begruendung in die Erlaubnisliste in docs/architektur.md",
                            datei.strip_prefix(wurzel()).unwrap_or(&datei).display(),
                            nr + 1,
                        ));
                    }
                }
            }
        }
    }

    assert!(
        verstoesse.is_empty(),
        "{} Kante(n) ohne Erlaubnis:\n  {}",
        verstoesse.len(),
        verstoesse.join("\n  ")
    );
}

#[test]
fn t003_jede_domaene_hat_genau_ein_plugin() {
    // Ein Ordner ohne Plugin ist `shared/` oder ein Fehler (§5 Regel 1).
    let mut ohne = Vec::new();
    for domaene in domaenen() {
        if domaene == "shared" {
            continue;
        }
        let text: String = rust_dateien(&wurzel().join("src").join(&domaene))
            .iter()
            .map(|p| std::fs::read_to_string(p).expect("lesbar"))
            .collect();
        let hat_plugin = text.contains("impl Plugin for");
        if !hat_plugin {
            ohne.push(domaene);
        }
    }
    assert!(
        ohne.is_empty(),
        "Diese Ordner unter src/ haben kein `impl Plugin`: {ohne:?} — \
         entweder sind sie shared/, oder es ist ein Fehler (init.md §5 Regel 1)"
    );
}

#[test]
fn t003_die_erlaubnisliste_nennt_nur_echte_domaenen() {
    // Eine Erlaubnis fuer einen Ordner, den es nicht (mehr) gibt, ist eine Luege, die
    // niemand bemerkt — bis jemand den Namen wiederverwendet.
    let domaenen = domaenen();
    for (von, nach) in erlaubnisliste() {
        assert!(domaenen.contains(&von), "Erlaubnisliste nennt `{von}`, das gibt es nicht");
        assert!(domaenen.contains(&nach), "Erlaubnisliste nennt `{nach}`, das gibt es nicht");
    }
}
