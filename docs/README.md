# docs — der Index: eine Zeile pro Datei

Stand: 2026-08-09 · Stufe: 🟨

**Jede Datei unter `docs/` steht hier.** Was hier nicht steht, ist eine Zombie-Datei und wird
verlinkt oder geloescht — dazwischen gibt es nichts (`prompts/init.md` §10).
`tools/normen.py` prueft das.

## Wo man anfaengt

| Frage | Datei |
|---|---|
| *Wo stehen wir wirklich?* | [`STATUS.md`](STATUS.md) — die vier Stufen, generiert |
| *Was ist als Naechstes zu tun?* | [`TODO.md`](TODO.md) — offene Arbeit in baubarer Reihenfolge, generiert |
| *Wie tickt das Projekt?* | [`../CLAUDE.md`](../CLAUDE.md) — der Index der Regeln, unter 150 Zeilen |
| *Auf welcher Maschine sitze ich?* | [`umgebung.md`](umgebung.md) — **erste Frage jeder Sitzung** |

## Die gepflegten Dateien

| Datei | Inhalt |
|---|---|
| [`architektur.md`](architektur.md) | Domaenen, Plugin-Reihenfolge, die **Erlaubnisliste** der Abhaengigkeiten, die Autoritaetstabelle (wer schreibt welches Feld), die Roblox→Bevy-Uebersetzungen |
| [`konventionen.md`](konventionen.md) | Achsen, Einheiten, Blickrichtung, 1 stud = 0,28 m, die verbindlichen Begriffe, die drei Signalfarben, alle Namensnormen |
| [`modelle.md`](modelle.md) | die Modellkette, die Anker-Empties, die drei glTF-Fallen — und **die Anleitung fuer den User**, wie er ein Modell austauscht |
| [`multiplayer.md`](multiplayer.md) | der Plan, der noch nicht gebaut wird: acht Regeln, Autoritaetsmodell, die Naht `src/net/` |
| [`umgebung.md`](umgebung.md) | die zwei Maschinen, gemessen: was geht, was nicht, und was daraus folgt |
| [`BUGS.md`](BUGS.md) | jeder Bug mit Reproduktion, Beleg, Erwartung, Ursache — und die Fix-Doktrin (roter Test zuerst) |
| [`FRAGEN.md`](FRAGEN.md) | Entscheidungen, die Claude nicht gehoeren. Elf offene, jede mit `ANNAHME:`, mit der weitergearbeitet wird |
| [`FUNDE.md`](FUNDE.md) | Fehler, die *nebenbei* auffielen — mit Messung, **nicht still mitgefixt** |
| [`ABNAHME.md`](ABNAHME.md) | worauf der User schauen soll, damit etwas ✅ werden kann |
| [`ROADMAP.md`](ROADMAP.md) | was bewusst spaeter kommt, und warum — die Gate-Regel zuerst |

## Die erzeugten Dateien — **nie von Hand aendern**

Handarbeit darin ist beim naechsten Lauf verloren. Jede traegt ihren Erzeuger im Kopf.

| Datei | erzeugt von | aus |
|---|---|---|
| [`features.ron`](features.ron) | `tools/features.py` | `gameplay/features.xlsx` — 245 Zeilen (194 `F-` + 51 `T-`). **Hier** stehen Stufe, Beleg und Notiz; sie werden beim Neulauf uebernommen |
| [`STATUS.md`](STATUS.md) | `tools/features.py` | `features.ron` |
| [`TODO.md`](TODO.md) | `tools/features.py` | `features.ron` |
| [`backlog/README.md`](backlog/README.md) | `tools/features.py` | Blatt `00_Anleitung` + die Zeilenzahlen aller Blaetter |
| [`backlog/funktionen.ron`](backlog/funktionen.ron) | `tools/features.py` | Blatt `01_Spielfunktionen` (194) |
| [`backlog/modelle.ron`](backlog/modelle.ron) | " | Blatt `02_3D-Assets` (100) |
| [`backlog/animationen.ron`](backlog/animationen.ron) | " | Blatt `03_Animationen` (100) |
| [`backlog/texturen.ron`](backlog/texturen.ron) | " | Blatt `04_Texturen` (28) |
| [`backlog/vfx.ron`](backlog/vfx.ron) | " | Blatt `05_VFX` (39) |
| [`backlog/audio.ron`](backlog/audio.ron) | " | Blatt `06_Audio` (118) |
| [`backlog/ui.ron`](backlog/ui.ron) | " | Blatt `07_UI-Screens` (45) |
| [`backlog/maps.ron`](backlog/maps.ron) | " | Blatt `08_Maps` (12) |
| [`backlog/tech.ron`](backlog/tech.ron) | " | Blatt `09_Tech-Backlog` (51) |
| [`backlog/namensschema.ron`](backlog/namensschema.ron) | " | Blatt `10_Namensschema` (40) — **verbindlich** |

Blatt `11_Zusammenfassung` wird **nicht** uebertragen: es ist berechnet. Es dient als
**unabhaengige Gegenprobe** — seine `COUNTA`-Formeln bestaetigen unsere Extraktion Zeile fuer
Zeile ([`FUNDE.md`](FUNDE.md) FUND-003).

## Die Fallgeschichten — der wertvollste Ordner im Projekt

| Datei | was Zeit gekostet hat |
|---|---|
| [`lessons/bevy.md`](lessons/bevy.md) | Bevy-Aufsetzung und die Engine-Fallen: API-Drift zwischen Minor-Versionen, Tupel-Grenzen, verzoegerte Commands, `cargo run` statt nacktes Binary |
| [`lessons/performance.md`](lessons/performance.md) | der raeumliche Index, „nichts pro Frame, alles pro Sekunde", Vsync als Messdeckel, Schatten |
| [`lessons/workflow.md`](lessons/workflow.md) | die Werkzeuge zuerst: Flags, der `--script`-Fahrer, das F3-Overlay, Screenshots |
| [`lessons/umgebung.md`](lessons/umgebung.md) | zwei Maschinen, volle Platte, kaputter Inkrement-Cache, parallele Sitzungen |
| [`lessons/arbeitsweise.md`](lessons/arbeitsweise.md) | parallel und wissenschaftlich: Supervisor, Schnittarten, Belegpflicht, das Berichtsformat |

## Die Fahrten in `scripts/`

`scripts/` **spielt** das Spiel, `tools/` **baut** Dinge — die Trennung wird nicht verwischt
(`prompts/init.md` §5). Eine Fahrt mit `assert` ist ein Test, kein Demo-Video.

| Datei | was sie faehrt |
|---|---|
| [`../scripts/t007-erste-fahrt.txt`](../scripts/t007-erste-fahrt.txt) | der Rauchtest: laeuft das Spiel, faellt niemand durch den Boden, hat ein Sprung Hoehe |
| [`../scripts/t019-latenz.txt`](../scripts/t019-latenz.txt) | dieselbe Fahrt bei 200 ms simulierter Latenz (`--lag 200`, Bibel T-019) |

```bash
cargo run -- --headless --script scripts/t007-erste-fahrt.txt --ticks 600
cargo run -- --headless --lag 200 --script scripts/t019-latenz.txt --ticks 900
```

## Noch leer, aber vorgesehen

| Ordner | wofuer |
|---|---|
| `gameplay/` | das Design pro Thema (`<thema>.md`), plus `referenzen.md` und `bilder/`. Fuellt sich, wenn aus dem Eingangskorb uebersetzt wird |
| `bilder/` | Screenshots als Beleg: `<f-id>-<kurz>.png`. **Auf Maschine A entsteht hier nichts** |

## Benannte Abweichung: `docs/` spiegelt `src/` noch nicht Datei fuer Datei

`prompts/init.md` §5 verlangt eine Doku-Datei pro Quelldatei. Solange eine Domaene nur aus
einem leeren `mod.rs` besteht, waere das ein Ordner voll Dateien ohne Inhalt — also genau die
Zombies, die §10 verbietet. **Bis dahin traegt [`architektur.md`](architektur.md) die
Beschreibung jeder Domaene in einer Zeile.** Wer die erste echte Logik in eine Domaene
schreibt, legt `docs/<domaene>.md` an und traegt sie hier ein — **im selben Commit**.
