# BUGS — jeder Bug mit Reproduktion, Beleg, Ursache, Fix und Test

Stand: 2026-08-09

> **Ein Bug ohne Beleg ist ein Geruecht — und Unsicherheit ist ein Mangel.**
> Kein „muesste jetzt gehen", kein „sollte passen", kein „wahrscheinlich behoben". Entweder
> du hast es **belegt**, oder du schreibst hin, dass du es nicht hast (`prompts/init.md` §9).

## Ein Bugbericht braucht vier Felder — sonst ist er keiner

| Feld | was hinein muss |
|---|---|
| **Reproduktion** | das exakte Kommando: `cargo run -- --headless --script scripts/haken-kante.txt`, plus Seed / Koordinate / Blickrichtung aus dem F3-Overlay und die **Maschine** (`[debian]`/`[cachy]`). Wer es nicht nachstellen kann, kann es nicht pruefen. |
| **Beleg** | Screenshot in `docs/bilder/`, Logausschnitt **oder** eine Zahl (gemessen 34 m/s, erwartet ≤ 12). Nicht „sieht falsch aus". |
| **Erwartung** | was stattdessen passieren muesste — und **woher** du das weisst (RON-Zeile, Doku-Absatz, Design-Entscheidung). |
| **Ursache** | `datei:zeile`, sobald bekannt. Solange sie fehlt: **„Ursache unbekannt"**, nicht geraten. |

**Kein Repro ⇒ kein Fix.** Ein Bug ohne Reproduktion wird als *unbelegt* eingetragen und
**nicht repariert** — ein Fix fuer etwas, das du nie gesehen hast, ist eine Aenderung ohne
Grund, und die kannst du hinterher auch nicht widerlegen.

## Ein Fix ohne roten Test ist eine Vermutung

Die Reihenfolge ist **nicht verhandelbar**:

1. **Test schreiben, der den Bug zeigt** — und laufen lassen, bis er **rot** ist. Ein Test,
   der nie rot war, beweist nur, dass er kompiliert.
2. **Fixen**, bis er gruen ist.
3. **Den Fix wieder herausnehmen** und zusehen, dass der Test erneut umfaellt. Erst dann
   weisst du, dass der Test *diesen* Fix prueft und nicht irgendetwas daneben.
4. **Hier eintragen:** Ursache, Fix, Testname. War es eine Falle, aus der man lernen kann:
   eine Datei in `docs/lessons/`.

Bei einem Bug, den nur das Auge sieht (Bewegungsgefuehl, Kameraruckeln, ein Haken, der ins
Nichts zeigt), ist der Beleg ein **`--script`-Lauf mit `assert`** plus Screenshot
vorher/nachher. Genau dafuer wird der Fahrer in Stufe 1 gebaut.

## Wortwahl

| nicht schreiben | sondern |
|---|---|
| „behoben" (ohne roten Test davor) | „gefixt, Test `x` war rot, ist gruen" |
| „sollte jetzt gehen" | „gebaut, **ungetestet** — 🟨" |
| „laeuft" | „im Spiel gesehen, Screenshot `docs/bilder/…`" |
| „ist schneller" | „16,6 → 9,4 ms, `--release --novsync`, Median aus 5 Laeufen [cachy]" |
| „funktioniert wahrscheinlich" | eine Zeile in `docs/FRAGEN.md` oder hier |

**Unsicherheit setzt die Stufe herunter, nicht hinauf** (§8, §9). Wenn du dir nicht sicher
bist, ist es **🟨** — auch wenn es funktioniert. Das kostet nichts. Eine zu hohe Stufe kostet
den Naechsten einen halben Tag.

## Sicherheit im Code: nichts darf still schiefgehen

- **Kein `unsafe`.** Wer glaubt, es zu brauchen, schreibt es nach `docs/FRAGEN.md`.
- **`unwrap()`/`expect()` nur mit Begruendung im Kommentar** — und **nie** auf Daten aus einer
  Datei oder aus Eingaben. Beim **Laden** der RON ist ein sofortiger, lauter Abbruch mit
  Dateiname das *richtige* Verhalten (fail fast beim Start); mitten im Spiel ist er es nie.
- **Physik braucht Wachen.** Seilkraefte, Normalisierungen und Divisionen erzeugen NaN/∞,
  sobald ein Vektor Laenge 0 hat oder ein Frame 0,5 s dauert. NaN im `Transform` ist der Bug,
  der aussieht wie „der Spieler ist verschwunden": Laenge pruefen, bevor normalisiert wird,
  `dt` clampen, und in `debug/` ein System, das **einmal warnt**, wenn eine Position nicht
  endlich ist.
- **Ein `panic!` im Spiel ist ein Bug**, auch wenn er „nie" auftritt. Ein `Result`, das mit
  `let _ =` geschluckt wird, ist ein Fehler, den niemand mehr sehen kann.

---

## Offene Bugs

*(keine — es gibt noch kein laufendes Spiel, in dem einer auftreten koennte. Der erste Eintrag
heisst `B-001`.)*

## Geschlossene Bugs

*(noch keine)*

Verwandt: [`docs/FUNDE.md`](FUNDE.md) (fremde Fehler) · [`docs/STATUS.md`](STATUS.md) ·
[`docs/lessons/`](lessons/)
