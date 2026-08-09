# ABNAHME — worauf der User bitte einmal schauen soll

Stand: 2026-08-09 · Stufe: 🟨

**✅ setzt Claude niemals selbst.** Nicht bei gruenen Tests, nicht bei einem schoenen
Screenshot, nicht „weil es offensichtlich laeuft". **🟧 ist die hoechste Stufe, die Claude
selbst vergeben darf** (`prompts/init.md` §8). Was Claude fuer reif haelt, steht hier — mit
dem Beleg daneben, damit das Draufschauen zwei Minuten dauert und nicht zwanzig.

## Wie eine Zeile hier aussieht

| Sache | ID | Stufe jetzt | wo hinschauen | wie lange |
|---|---|---|---|---|
| *(Beispiel)* Haken einschlagen | F-001 | 🟧 | `cargo run --features wayland -- --sandbox`, dann `docs/bilder/f001-haken.png` daneben halten | 2 min |

---

## Bereit zum Draufschauen

*(noch nichts — es gibt noch kein laufendes Spiel. Die erste Zeile kommt, wenn Stufe 1 des
Stufenplans steht.)*

## Nicht bereit, und warum

| Sache | Stufe | was fehlt zum 🟧 |
|---|---|---|
| **alles Sichtbare** | ⬜/🟨 | **Auf dieser Maschine gibt es kein Bild.** Maschine A (`debian`) hat keine Grafiksitzung, keinen Compositor und damit keinen Screenshot-Weg — `docs/umgebung.md`. Ohne Bild kein 🟧, ohne Ausnahme. Die einzige offene Tuer ist Offscreen-Rendering, und die ist **unbewiesen** (`docs/FRAGEN.md` Q-009) |

## Was der User ausserdem sehen wollte (`prompts/init.md` §16)

Diese sechs Punkte sind die Abnahme des **Auftrags**, nicht einzelner Features. Ihr Stand
gehoert in den Schlussbericht jeder Sitzung:

| # | Verlangt | Stand |
|---|---|---|
| 1 | `cargo test` — die Ausgabe, ungekuerzt zusammengefasst (wie viele gruen, welche rot und warum) | offen |
| 2 | Mindestens zwei Screenshots in `docs/bilder/` (Stadt beim Schwingen, Titan im Bild) — **auf Maschine B**. Auf A stattdessen die `--headless`-Skriptlaeufe mit ihren `assert`-Ergebnissen und der Vermerk „Pixel ungesehen" | offen |
| 3 | `docs/STATUS.md`, in dem jede Sache eine der vier Stufen traegt — und **kein einziges ✅** | steht, generiert aus `docs/features.ron` (245 Zeilen) |
| 4 | **Diese Datei**, gefuellt | steht, noch ohne Eintraege |
| 5 | Die Modell-Tabelle aus `cargo test --test modelle -- --ignored --nocapture` und mindestens eine `.blend`, die der User oeffnen kann — mit den Ankern schon an ihrem Platz | offen; **auf Maschine A fehlt Blender**, also entsteht hier nur das `.py` |
| 6 | Ein ehrlicher Absatz: **was gebaut, aber nicht gesehen ist** | im Schlussbericht |

> **Die eine Regel ueber allen: erst messen, dann behaupten.** Fast jeder teure Fehler in
> einem Projekt wie diesem ist eine Stelle, an der etwas Vernuenftiges *erklaert* wurde,
> statt es in einer Minute zu *messen* — und die Erklaerung war das Problem.

Verwandt: [`docs/STATUS.md`](STATUS.md) · [`docs/umgebung.md`](umgebung.md) ·
[`docs/FRAGEN.md`](FRAGEN.md)
