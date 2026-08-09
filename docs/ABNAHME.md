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

## Der bewiesene Weg zu einem Bild

**Seit 2026-08-09 gibt es einen.** `--bild <pfad>` macht nach `--ticks <n>`
Simulationsschritten ein PNG und beendet sich. Der Ausloeser ist ein **Tick**, keine Sekunde
und keine Taste: derselbe Befehl liefert deshalb morgen dasselbe Bild, und ein Auftrag kann
seine Bildzeile selbst erfuellen, ohne dass ein Mensch im richtigen Moment eine Taste
drueckt.

```bash
# Der empfohlene Weg — feste 1280x720, unabhaengig von Compositor und Fenstergroesse:
cargo run --features wayland,klang -- --offscreen \
    --script scripts/t006-bild-fern.txt --ticks 110 --bild docs/bilder/t006-welt-fern.png

# Mit Fenster — man sieht dabei zu. Dieselbe SZENE, aber nicht dasselbe Bild:
# das Seitenverhaeltnis kommt vom Compositor, nicht vom Befehl.
cargo run --features wayland,klang -- \
    --script scripts/t006-bild-nah.txt --ticks 110 --bild docs/bilder/t006-spieler-sicht-fenster.png
```

**Gemessen `[cachy]`, nicht behauptet:**

| Frage | Antwort | Beleg |
|---|---|---|
| Kommt ein Bild heraus? | ja, in beiden Modi, Exit 0 | die vier PNG unten |
| Ist es reproduzierbar? | **`--offscreen` ja, bitgleich** | zwei Laeufe, `sha256 = eb212dfe…` beide Male |
| Und mit Fenster? | **nein** | derselbe Befehl lieferte 1267x1390, vier Minuten spaeter 627x974 — **die Bildgroesse entscheidet der Compositor, nicht der Befehl** |
| Geht es ohne Grafiksitzung? | ja | `env -u WAYLAND_DISPLAY -u DISPLAY … --offscreen` schreibt ein volles Bild |
| Geht es mit `--headless`? | **nein, und es sagt das jetzt** | `--headless` schaltet `backends: None`; die Kombination endet mit **Exit 1** und einer Zeile, die auf `--offscreen` zeigt |

Die drei Modi und die Belegstellen im Bevy-Quelltext stehen in
[`src/debug/bild.rs`](../src/debug/bild.rs). **`--headless` bleibt bildlos** — das ist keine
Bequemlichkeit, sondern die Folge von `backends: None`.

## Bereit zum Draufschauen

| Sache | ID | Stufe jetzt | wo hinschauen | wie lange |
|---|---|---|---|---|
| Es gibt ueberhaupt ein Bild | T-006 | 🟧 | `docs/bilder/t006-welt-fern.png` und `docs/bilder/t006-spieler-sicht.png` — die ersten Pixel, die dieses Projekt je gesehen hat | 2 min |

Die drei Belege fuer dieses eine 🟧, damit es nachgeprueft und notfalls zurueckgesetzt werden
kann (`docs/STATUS.md` gehoert dem Hauptkopf, hier steht nur der Vorschlag):

- **Bild:** die vier PNG in `docs/bilder/`, angesehen und unten beschrieben.
- **Zahl:** 1280x720 · `sha256 = eb212dfe…` in zwei Laeufen gleich · 696808 / 1138720 Bytes
  im Fenstermodus bei 1267x1390.
- **Test, der rot wird:** der Lauf selbst. `--bild` endet mit **Exit 1**, wenn kein PNG
  entsteht — gegengeprueft mit `--headless --bild`: Exit 1, keine Datei. Es gibt **keinen**
  `cargo test`-Fall, der die Pixel prueft; was `cargo test` deckt, ist nur das Flag und der
  Ausloesetick (`src/shared/start.rs`, `src/debug/bild.rs`).

**Was auf den Bildern zu sehen ist** — vier Bilder, zwei Blicke mal zwei Wege:

| Datei | Groesse | was drauf ist |
|---|---|---|
| `t006-welt-fern.png` | 1280x720 | Aus 19–20 m Hoehe (der `warp` setzt 20 m, bis zum Bild faellt der Spieler rund einen Meter — die beiden `assert` klammern 18 < h < 21), 45 m hinter dem Ursprung: die olivgruene Bodenplatte bis zu ihrer Kante bei 245 m, darauf die vier Platzhalterkloetze aus `src/world/mod.rs` — ziegelroter Wuerfel (8 m), kleiner sandbrauner (4 m), zwei steingraue (12 m und 18 m). Alle vier stehen sauber auf dem Boden, keiner steckt darin |
| `t006-spieler-sicht.png` | 1280x720 | Dieselbe Szene aus **1,6 m Augenhoehe**, 4 m vor dem Ursprung. Der Massstab stimmt: der 4-m-Wuerfel in 16 m Entfernung ragt knapp ueber die Horizontlinie, die beiden grossen Kloetze fuellen den rechten Bildrand. Ein Mensch ist hier klein — genau das war die Vorgabe aus `massstab.ron` |
| `t006-welt-fern-fenster.png` | 1267x1390 | Dieselbe Szene, aber **durch das Fenster** aufgenommen — hochkant, weil niri die Kachel so gelegt hat. Senkrecht identisch (das Sichtfeld ist senkrecht definiert), waagerecht beschnitten. Der Beleg dafuer, dass der Fensterweg funktioniert **und** dass seine Bildgroesse nicht dem Befehl gehoert |
| `t006-spieler-sicht-fenster.png` | 1267x1390 | dito aus Augenhoehe |

**Und was NICHT drauf ist, obwohl man es erwartet:**

- **Kein Himmel.** Die obere Bildhaelfte ist ein gleichmaessiges Dunkelgrau — das ist Bevys
  `ClearColor`, nicht Atmosphaere. Es gibt weder Himmel noch den Fernnebel, den die Bibel 3.4
  verlangt (`docs/architektur.md`, Uebersetzungstabelle).
- **Keine Schatten.** Bewusst: `shadow_maps_enabled: false` in `src/render/mod.rs`, mit
  Begruendung. Man sieht nur Flaechenschattierung, keinen geworfenen Schatten.
- **Keine Stadt.** `assets/data/maps.ron` beschreibt eine, aber `world::karte::karte_bauen`
  ist ein leerer Stub — im Bild stehen die **vier Platzhalterkloetze** aus `welt_aufbauen`.
- **Kein Umschauen.** `render::kamera::kamera_drehen` ist ebenfalls leer; die Kamera blickt
  immer nach −Z. `look` im Skript aendert am Bild nichts, nur `warp` tut es.

## Nicht bereit, und warum

| Sache | Stufe | was fehlt zum 🟧 |
|---|---|---|
| **alles Sichtbare ausser dem Bildweg selbst** | ⬜/🟨 | Der Screenshot-Weg steht, aber ein Bild allein macht keine Sache 🟧: dazu gehoeren **Bild, Zahl und ein Test, der rot wird**. Wer jetzt etwas auf 🟧 hebt, holt sich seine Bildzeile mit dem Befehl oben — die Ausrede „auf dieser Maschine gibt es kein Bild" ist weg |
| Bild auf **Maschine A** (`debian`) | offen | `--offscreen` braucht einen wgpu-Adapter, kein Fenster. Dass es **ohne Grafiksitzung** geht, ist auf `[cachy]` gemessen; dass der N100 unter debian einen Adapter findet, ist es **nicht** (`docs/FRAGEN.md` Q-009) |

## Was der User ausserdem sehen wollte (`prompts/init.md` §16)

Diese sechs Punkte sind die Abnahme des **Auftrags**, nicht einzelner Features. Ihr Stand
gehoert in den Schlussbericht jeder Sitzung:

| # | Verlangt | Stand |
|---|---|---|
| 1 | `cargo test` — die Ausgabe, ungekuerzt zusammengefasst | **62 gruen, 0 rot** `[debian]`: 42 Einheitentests im Crate, 10 `tests/data.rs`, 7 `tests/mehrspieler.rs`, 3 `tests/domaenen.rs`. Keiner uebersprungen |
| 2 | Mindestens zwei Screenshots in `docs/bilder/` — **auf Maschine B**. Auf A stattdessen die `--headless`-Skriptlaeufe mit ihren `assert`-Ergebnissen und der Vermerk „Pixel ungesehen" | **Vier Bilder, alle auf `[cachy]` aufgenommen und angesehen** (Tabelle oben), aus `scripts/t006-bild-fern.txt` und `scripts/t006-bild-nah.txt`, beide mit gehaltenen `assert` und Exit 0. Dazu weiter die zwei alten Fahrten: `scripts/t007-erste-fahrt.txt` (6 `assert`, 180 Ticks) und `scripts/t019-latenz.txt` bei `--lag 200` (3 `assert`). Gegenprobe: ein absichtlich falscher `assert` endet mit **Exit 1** und druckt den gemessenen Wert |
| 3 | `docs/STATUS.md`, in dem jede Sache eine der vier Stufen traegt — und **kein einziges ✅** | steht, generiert aus `docs/features.ron`: 245 Zeilen, **239 ⬜ · 6 🟨 · 0 🟧 · 0 ✅** |
| 4 | **Diese Datei**, gefuellt | steht — und die Liste „bereit zum Draufschauen" hat seit 2026-08-09 ihre **erste Zeile** |
| 5 | Die Modell-Tabelle und mindestens eine `.blend` mit gesetzten Ankern | **offen.** Auf Maschine A gibt es kein Blender ([`umgebung.md`](umgebung.md)); es entstuenden nur `tools/blend/*.py` ohne `.blend` und ohne `.glb`. Die Kette ist in [`modelle.md`](modelle.md) beschrieben, aber **nicht gebaut** — ⬜, nicht 🟨 |
| 6 | Ein ehrlicher Absatz: **was gebaut, aber nicht gesehen ist** | **Fast alles — aber nicht mehr alles.** Gesehen sind jetzt: Boden, die vier Platzhalterkloetze, Licht, Kamerahoehe, der Massstab und die Farben. Ungesehen bleibt alles andere, und zwei Dinge sind auf den Bildern als **fehlend** aufgefallen statt als kaputt: die Stadt aus `maps.ron` (Stub) und die Kameradrehung (Stub). Der Fensterweg ist gesehen, aber **nicht reproduzierbar** — nur `--offscreen` ist es |

> **Die eine Regel ueber allen: erst messen, dann behaupten.** Fast jeder teure Fehler in
> einem Projekt wie diesem ist eine Stelle, an der etwas Vernuenftiges *erklaert* wurde,
> statt es in einer Minute zu *messen* — und die Erklaerung war das Problem.

Verwandt: [`docs/STATUS.md`](STATUS.md) · [`docs/umgebung.md`](umgebung.md) ·
[`docs/FRAGEN.md`](FRAGEN.md)
