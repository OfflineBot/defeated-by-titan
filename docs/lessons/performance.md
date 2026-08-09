# performance — die fünf Regeln aus §11, die Zielzahlen aus der Bibel, und was in beiden Quellen fehlt

Stand: 2026-08-09 · Stufe: 🟨 (aus den Quellen aufgeschrieben, in diesem Projekt noch nichts davon gemessen)

§11 heißt „die Regel, die man von Anfang an einhalten muss" — Performance ist hier also kein
Feinschliff am Ende. Quellen: `prompts/init.md` §11 (Zeile 1151-1172) und
`prompts/DefeatedByTitan_Design-Bibel.md` 3.5 (Zeile 99) / 6.4 (Zeile 235). Was hier steht, steht
dort; wo etwas abgeleitet ist, ist es als abgeleitet markiert.

## Die Zielzahlen aus der Bibel

| Vorgabe | Zahl | Quelle |
|---|---|---|
| Bildrate | **Mindestprofil und Vollprofil zielen beide auf 60 FPS** | Bibel 3.5 |
| Mindestprofil | Einsteiger-Laptop, **integrierte Grafik** | Bibel 3.5 |
| Qualitätsprofile insgesamt | **zwei statt fünf** | Bibel 3.5 |
| Eigentliche Belastungsprobe | **20 Spieler mit je zwei Seilen + 60 Titanen** — „nicht die Grafik" | Bibel 6.4 |
| Gegenmaßnahme dafür | Interpolationspuffer und Replikationsdrosselung **ab P1 einplanen** | Bibel 6.4 |

60 FPS heißt 16,6 ms pro Bild — für **alles** zusammen: Simulation, Netz, Rendering. Eine
Aufteilung dieses Budgets nennt keine der beiden Quellen (siehe Lücken).

## Regel 1 — der räumliche Index gehört in `world/`, ab dem ersten Tag

Eine Stadt hat Tausende Häuser, ein Einsatz Dutzende Titanen, jeder Titan sechs Gliedmaßen
(§11). Die harte Formulierung der Quelle:

> **Nichts darf alle Entities durchlaufen, um eine Frage über die zehn Meter vor der Nase zu
> beantworten.**

| | |
|---|---|
| **Was** | Gitterzellen → Entities, in `world/` (siehe `docs/architektur.md`: `world/` führt den räumlichen Index) |
| **Wie gepflegt** | über Bevys `Added` / `RemovedComponents` — damit der Index **nicht veralten kann** |
| **Wer benutzt ihn** | Hakeneinschlag, Klingentreffer, Kollision, Titanen-Zielsuche — **alle vier**, keine Ausnahme |
| **Woran man den Verstoß erkennt** | eine Query ohne Ortsfilter in einem System, das eine lokale Frage stellt |

## Regel 2 — nichts ändert sich pro Frame, alles pro Sekunde

`* time.delta_secs()` allein reicht **nicht**. Drei Unterfälle, die alle so aussehen, als wären sie
schon framerate-unabhängig, und es nicht sind:

| Fall | Falsch (pro Frame) | Richtig (pro Sekunde) |
|---|---|---|
| **(a) Ganzzahlen** | `(schaden * dt).ceil()` — das „macht die Framerate zur Schadenszahl" (§11) | Bruchteile mittragen, **nie runden** |
| **(b) Exponentielles Glätten** | `x += (ziel - x) * 0.1` | Faktor `1 - e^(-k*dt)` |
| **(c) Rauschen** | `rauschen * dt` | Rauschen skaliert mit **`sqrt(dt)`** |

**Die Vorschrift dazu ist organisatorisch, nicht mathematisch:** eine einzige Hilfsfunktion in
`shared/` schreiben und **nur die** benutzen (§11).

Im Repo steht sie bereits: `src/shared/mathe.rs` — `dt_gezaehmt`, `glaetten`, `glaetten_vec3`,
`rausch_faktor`. Neue Aufrufer nehmen diese Funktionen, keine zweite Rechnung daneben.

## Regel 3 — erst messen, dann behaupten. Und: Debug ist langsam

```bash
cargo run              # Debug-Build — der eigene Crate steht auf opt-level = 1
cargo run --release    # das Einzige, worüber man eine Perf-Aussage machen darf
```

| Beobachtung | Richtige Reaktion |
|---|---|
| „Es ruckelt seit heute" — gemessen mit `cargo run` | Nichts tun. **Debug-Langsamkeit ist keine Regression** (§11). Erst `--release` gegenmessen |
| „Das System ist zu teuer" ohne Zahl | Keine Optimierung beginnen. Erst messen, dann behaupten |

## Regel 4 — unter Vsync ist jede Bildzeit 16,6 ms

Mit Vsync misst die Frage „was kostet das?" **sechsmal denselben Deckel** (§11).

Beides zusammengelesen (§11 + Bibel 3.5, so in keiner Quelle formuliert): das Ziel ist 60 FPS, und
der Vsync-Deckel liegt bei genau diesen 16,6 ms. „Läuft mit 60" ist damit **kein** Beleg, dass Luft
im Budget ist. Erst ohne Deckel gemessen sagt die Zahl etwas.

| Werkzeug | Status | Was es liefert |
|---|---|---|
| `--novsync`-Startflag | **muss gebaut werden**, „früh" (§11) — existiert noch nicht | Deckel weg, echte Frametime |
| Bevys `RenderDiagnosticsPlugin` | von §11 als Alternative oder Ergänzung genannt, hier noch nicht eingebaut | **echte GPU-Zeitstempel pro Renderpass** |

## Regel 5 — Schatten sind der teuerste Schalter im Spiel

Punktlichter sind fast gratis, Schatten nicht (§11). Deshalb: **erst am Ende, mit Zahl.**

## Die eigentliche Belastungsprobe ist nicht die Grafik

Die Bibel führt das unter **Risiken** (6.4), nicht unter Grafik:

> Zwanzig Spieler mit je zwei Seilen und sechzig Titanen sind die eigentliche Belastungsprobe, nicht
> die Grafik.

| Was | Zahl | Woher |
|---|---|---|
| Spieler pro Instanz | 20 | Bibel 6.4 |
| Seile | 2 pro Spieler → **40** gleichzeitig | 2 pro Spieler steht in Bibel 6.4; die 40 sind gerechnet |
| Titanen | 60 | Bibel 6.4 |
| Gliedmaßen | 6 pro Titan → **360** (§11 × Bibel 6.4, in keiner Quelle so genannt) | abgeleitet |

Praktische Folge für die Reihenfolge: eine Szene, die 60 Titanen und 40 Seile gleichzeitig
simuliert, ist der Maßstab. Und die Gegenmaßnahme, die die Bibel dazu nennt, ist **Netz**, nicht
Rendering: Interpolationspuffer und Replikationsdrosselung, ab P1 eingeplant (siehe
`docs/multiplayer.md`).

## Lücken — was in den Quellen NICHT steht

| Lücke | Warum sie weh tut |
|---|---|
| Keine Aufteilung der 16,6 ms auf Simulation / Netz / Rendering | „Zu teuer" ist ohne Teilbudget nicht entscheidbar |
| Keine Hardware-Definition des Mindestprofils außer „Einsteiger-Laptop, integrierte Grafik" | Ohne konkretes Gerät ist „60 FPS auf Mindestprofil" nicht prüfbar |
| Keine definierte Messszene | 60 FPS worin? Die 20/40/60-Szene aus 6.4 ist die naheliegende Kandidatin, aber nirgends als Prüfszene festgeschrieben |
| Keine Zellgröße für den räumlichen Index | §11 sagt „Gitterzellen", nicht wie groß. Muss gemessen werden |
| Kein Zahlenwert für die Schattenkosten | „Erst am Ende, mit Zahl" — die Zahl gibt es noch nicht |
| Kein Wert für `k` in `1 - e^(-k*dt)` | §11 gibt die Formel vor, nicht die Konstante; `mathe.rs` nimmt sie als Halbwertszeit-Parameter entgegen, pro Anwendungsfall zu bestimmen |

Diese sechs Punkte gehören als Fragen in `docs/FRAGEN.md`, nicht als Annahmen in den Code.

Verwandt: [architektur.md](../architektur.md) · [multiplayer.md](../multiplayer.md) · [konventionen.md](../konventionen.md) · [umgebung.md](../umgebung.md) · [FRAGEN.md](../FRAGEN.md) · [ROADMAP.md](../ROADMAP.md) · [umgebung.md (lessons)](umgebung.md) · [workflow.md](workflow.md) · [arbeitsweise.md](arbeitsweise.md)
