# init — hier fängt es an

**Führe `prompts/init.md` aus.** Das ist der Auftrag; diese Datei ist nur der Startknopf.

```bash
ls -la prompts/ gameplay/     # was liegt da?
cat prompts/*.md              # ALLE lesen, nicht nur init.md
ls -R gameplay/               # dann der Gameplay-Korb
```

## Die Reihenfolge, in der gelesen wird

1. **`prompts/init.md`** — der Rahmen: was für ein Spiel, welche Ordnerstruktur, welche Regeln,
   welche Beweispflicht, wie es endet.
2. **Jede weitere `*.md` in `prompts/`** — Nachträge und Präzisierungen des Users. **Es gibt darin
   keine optionale Datei.** `init.md` weiß nicht, was in ihnen steht; wer nur sie liest, hat den
   Auftrag nicht gelesen.
3. **`gameplay/`** — was inhaltlich gebaut werden soll (TODO-Liste, Mechaniken, Zahlen, Skizzen).
   Bei Widerspruch: `gameplay/` bestimmt den **Inhalt**, `prompts/` das **Handwerk**.

Danach wird **aufgesetzt und gebaut** — nicht geplant und zurückgefragt. Alles, was du wissen
musst, steht in den Dateien oben; was wirklich nicht darin steht, kommt nach `docs/FRAGEN.md`, und
du arbeitest daran vorbei weiter.

## Was am Ende passiert

`prompts/`, `gameplay/` und **diese Datei** sind Bootstrap-Gerüst, kein Teil des fertigen Projekts.
Wenn ihr Inhalt in die echte Struktur übertragen ist (`CLAUDE.md`, `README.md`, `docs/`,
`assets/data/*.ron`), wird das Gerüst abgebaut, ein öffentliches GitHub-Repo angelegt und diese
Datei zuletzt gelöscht. Der genaue Ablauf steht in `prompts/init.md` §18 — **erst übertragen,
dann löschen**, jeder Schritt ein eigener Commit.
