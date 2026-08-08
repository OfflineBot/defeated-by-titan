# init — hier fängt es an

**Führe `prompts/init.md` aus.** Das ist der Auftrag; diese Datei ist nur der Startknopf.

```bash
ls -la prompts/ gameplay/     # was liegt da?
cat prompts/*.md              # ALLE lesen, nicht nur init.md
ls -R gameplay/               # dann der Gameplay-Korb
```

## Die Reihenfolge, in der gelesen wird

1. **`prompts/init.md`** — der Rahmen: Engine, Ordnerstruktur, Regeln, Beweispflicht, Normung,
   Werkzeuge, wie es endet. 18 Abschnitte.
2. **`prompts/DefeatedByTitans_Design-Bibel.md`** — das *Warum*: Designpfeiler, Welt und Ton,
   visueller Stil, Plattform, Mehrspieler-Grundregeln, Gegner-Philosophie, Phasenplan P0–P11,
   Kennzahlen, Risiken. **Inhaltlich gewinnt sie über `init.md`.**
3. **Jede weitere `*.md` in `prompts/`** — Nachträge und Präzisierungen des Users. **Es gibt darin
   keine optionale Datei.**
4. **`gameplay/features.xlsx`** — der Produktions-Backlog: 12 Blätter, ~790 Tickets, inklusive
   Blatt `10_Namensschema` (verbindliche Begriffe) und der MoSCoW-Priorität. **Die Arbeitsvorlage.**
5. **Der Rest von `gameplay/`** — Skizzen und Notizen. Bei Widerspruch: `gameplay/` und die Bibel
   bestimmen den **Inhalt**, `prompts/init.md` das **Handwerk**.

Danach wird **aufgesetzt und gebaut** — nicht geplant und zurückgefragt. Alles, was du wissen
musst, steht in den Dateien oben; was wirklich nicht darin steht, kommt nach `docs/FRAGEN.md`, und
du arbeitest daran vorbei weiter.

## Was am Ende passiert

`prompts/`, `gameplay/` und **diese Datei** sind Bootstrap-Gerüst, kein Teil des fertigen Projekts.
Wenn ihr Inhalt in die echte Struktur übertragen ist (`CLAUDE.md`, `README.md`, `docs/`,
`assets/data/*.ron`), wird das Gerüst abgebaut, ein öffentliches GitHub-Repo angelegt und diese
Datei zuletzt gelöscht. Der genaue Ablauf steht in `prompts/init.md` §18 — **erst übertragen,
dann löschen**, jeder Schritt ein eigener Commit.
