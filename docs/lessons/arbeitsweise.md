# arbeitsweise — ein Supervisor, der selbst nichts schreibt, breit parallel delegiert und jede Behauptung vorher an ein Kriterium bindet

Stand: 2026-08-09 · Stufe: 🟨 (aus `prompts/init.md` §17 aufgeschrieben — die Arbeitsweise ist
beschrieben, aber in diesem Repo noch kein einziges Mal über eine volle Iteration gemessen)

## Der Fall

Ein Kopf arbeitet die Auftragsdatei von oben nach unten ab. Das fühlt sich ordentlich an und
ist der teuerste Modus, den das Projekt hat: die übrigen Kerne langweilen sich, jede Behauptung wird
erklärt statt gemessen, und dieselbe falsche Annahme wird dreimal neu implementiert, weil
niemand sie je hingeschrieben hat.

Die Quelle macht dazu zwei Vorgaben, die beide **verbindlich** sind, nicht empfohlen: gearbeitet
wird **breit parallel** und mit **wissenschaftlicher Methode**. `prompts/init.md` ist der
**Auftrag**, nicht die Arbeitsreihenfolge eines einzelnen Kopfes.

## a) Der Supervisor schreibt nichts

**Ein Supervisor läuft dauerhaft im `/loop`** und triggert Workflows und Subagenten. Er
**plant, delegiert, prüft und integriert** — mehr nicht. Sobald er selbst anfängt, eine Datei
zu schreiben, ist er kein Supervisor mehr, sondern der sechste Agent, der auf sich selbst
wartet.

**Die Iteration — sechs Schritte, in dieser Reihenfolge:**

| # | Schritt | woran man erkennt, dass er übersprungen wurde |
|---|---|---|
| 1 | Ist-Zustand | niemand kann sagen, was vor dem Eingriff galt |
| 2 | Hypothese **+ Abnahmekriterien** | das Kriterium entsteht nach dem Ergebnis und passt immer |
| 3 | parallele Delegation | ein Agent arbeitet, die übrigen Kerne stehen still |
| 4 | Ergebnisse sammeln | Teilergebnisse verschwinden in Transkripten |
| 5 | **gegen die Kriterien** prüfen | „sieht gut aus" statt Testname und Messwert |
| 6 | integrieren und über die nächste Runde entscheiden | fünf grüne Zweige, kein grüner Baum |

**Abbruchbedingungen** — der Loop hört auf bei erfüllter DoD, bei erreichtem Limit, oder wenn
**zweimal dieselbe Hypothese gescheitert** ist.

Der dritte Fall ist der wichtige und der, den man am leichtesten übergeht. Zweimal dieselbe
gescheiterte Hypothese heißt **nicht**, dass die Ausführung schlampig war — es heißt, dass die
**Annahme falsch** ist. Der dritte Anlauf ist verlorene Zeit; die Frage gehört nach
`docs/FRAGEN.md`.

## b) Fachexperten, die Abweichungen benennen statt sie zu umgehen

**Für jeden aus dem Projekt abgeleiteten Fachbereich wird ein Senior-Experte angelegt.** Die
Quelle nennt beispielhaft: Vector-Gear-Physik, Titanen-Verhalten, Rendering/3D-Pipeline,
Daten/RON, Tooling & Test, Doku & Status.

Jeder Experte

- **entscheidet eigenverantwortlich in seiner Domäne**,
- hält **alle** Projektrichtlinien ein,
- und **benennt Abweichungen explizit, statt sie zu umgehen**.

Der Satz dazu ist der Merksatz des ganzen Abschnitts: **eine benannte Abweichung ist eine
Entscheidung, eine stille ist ein Bug mit Anlauf.**

**Verbindlich für alle — Supervisor wie Experten:**

> falsifizierbare Hypothese → Prüfkriterium **vorab** festlegen → reproduzierbar messen →
> auswerten.

Jede Aussage mit Beleg, Annahmen als `ANNAHME:` markiert, Unsicherheit ausgewiesen, nichts
erfunden; bei Unklarheit messen oder eskalieren, nicht raten. Aus Sicht des Delegierenden
heißt das: **ein Ergebnis ohne vorab festgelegtes Prüfkriterium ist kein Ergebnis, sondern
eine Meinung.**

## c) Parallel ist die Voreinstellung — die Frage ist „warum nicht?"

Die Frage ist nie „kann man das parallel machen?". Seriell ist nur richtig, wo eine Datei
**einen einzigen Schreiber** braucht (Tabelle in Abschnitt f) oder wo Stufe N das **Ergebnis**
von N−1 wirklich braucht. Alles andere läuft gleichzeitig.

**Die vier Schnittarten, in dieser Vorzugsordnung:**

| # | Schnitt | Beispiel | warum er trägt |
|---|---|---|---|
| 1 | nach **Domäne** | `vector/`, `titan/`, `world/`, `hud/` gleichzeitig | Dateibesitz und Domäne sind dasselbe (§5) |
| 2 | nach **`F-ID`** | unabhängige Features aus §2 | `abhaengt_von` in `features.ron` sagt, was **nicht** gleichzeitig geht |
| 3 | nach **Prüf-Dimension** | Korrektheit · Ränder · Performance · „was passiert im Netz" (§6) | vier Blickwinkel finden vier Sachen; vier gleiche Prüfer finden dreimal dasselbe |
| 4 | nach **Datei** bei Massenarbeit | ein Agent pro `tools/blend/*.py`, pro Doku-Datei, pro Excel-Blatt | keine Naht, kein Merge |

Schnitt 1 setzt voraus, dass das Skelett steht — **erst seriell, dann breit**: Stufe 0–1
(Skelett, `Cargo.toml`, `main.rs`, Domänenordner mit leeren Plugins, `docs/`-Skelett) macht
**ein** Kopf allein. Ein Fan-out auf einen leeren Ordner erzeugt fünf inkompatible Entwürfe
derselben Datei.

## d) Die Breite kommt von `nproc`, nicht vom Wunsch

```bash
nproc
```

Der Deckel ist die Maschine (§14) — und **der Compiler ist auch ein Verbraucher**.

| Maschine | Kerne/Threads | gleichzeitige Agenten |
|---|---|---|
| A | 4 Kerne | **2–3** |
| B | 16 Threads | 8 und mehr |

**Zwanzig Agenten auf vier Kernen sind langsamer als drei — sie warten nur gemeinsam.**

**Pipeline statt Barriere.** Ein Feature, das fertig geprüft ist, wartet nicht darauf, dass
fünf andere fertig werden. Ein Sammelpunkt ist nur dort richtig, wo eine Stufe **alle**
Vorergebnisse zusammen braucht: deduplizieren, Gesamtzählung, „gibt es überhaupt Funde?".

**Was jede Parallelisierung VORHER braucht** — sonst produziert sie Integrationsarbeit statt
Fortschritt:

1. **Die Schnittstelle steht.** Components, Messages, Signaturen sind festgeschrieben **und
   committet**. Wer parallelisiert, bevor die Naht steht, integriert hinterher fünf Entwürfe
   derselben Datei.
2. **Der Dateibesitz ist verteilt.** Jede Datei hat genau **einen** schreibberechtigten Agenten.
3. **Das Abnahmekriterium ist notiert** — vorher, nicht nachher.

Und danach, nach **jedem** Zusammenlauf:

```bash
cargo check 2>&1 | grep '^error'
cargo test
```

**Fünf einzeln grüne Agenten sind zusammen nicht automatisch grün** — jeder hat nur seine
Hälfte gesehen.

## e) Wissenschaftlich heißt messbar, reproduzierbar, widerlegbar

Keine Attitüde, sondern Arbeitsweise — für jede Behauptung, von „der Haken hält nicht" bis
„das ist schneller":

| # | Regel | was sie kostet, wenn sie fehlt |
|---|---|---|
| 1 | **Erst den Ist-Zustand messen, dann ändern** — die Basismessung **vor** dem Eingriff ist der wichtigste Wert überhaupt | du „behebst" Dinge, die nie kaputt waren, und weißt nie, ob deine Änderung etwas getan hat |
| 2 | **Hypothese hinschreiben, bevor gemessen wird** — falsifizierbar („wenn X, dann sinkt Y unter Z"), samt Prüfkriterium | eine Erklärung, die nach der Messung entsteht, passt immer und ist deshalb wertlos |
| 3 | **Eine Variable pro Experiment** | am Ende weißt du nur, dass *irgendwas* half |
| 4 | **Reproduzierbar heißt: der Befehl steht daneben** — komplette Kommandozeile, Seed, Koordinate, Blickrichtung, **Maschine** (`[cachy]`/`[debian]`, §14) | was niemand nachstellen kann, ist keine Messung, sondern eine Anekdote |
| 5 | **Zeiten mehrfach messen**: N Läufe, **Median und Perzentil**, nicht Mittelwert; niemals über Maschinen hinweg vergleichen | ein Lauf ist Rauschen |
| 6 | **Erst widerlegen versuchen, dann glauben** — zu jeder Behauptung ein unabhängiger Versuch, sie zu kippen („finde den Fall, in dem das falsch ist") | was keinen Angriff überlebt hat, ist 🟨 (§8) |
| 7 | **Ein negatives Ergebnis ist ein Ergebnis** und wird aufgeschrieben (`docs/lessons/`, `docs/BUGS.md`) | in drei Wochen probiert es jemand genauso wieder |
| 8 | **Annahmen als `ANNAHME:` markieren, Unsicherheit ausweisen, nichts erfinden**; bei Unklarheit messen oder eskalieren (§9) | geraten wird als gewusst gelesen |

Punkt 6 ist keine Höflichkeitsschleife: **Prüfen gehört in den Workflow, nicht in die
Hoffnung.** Nach jeder Findungs- oder Baustufe kommt eine **unabhängige** Stufe mit dem
Auftrag „finde den Fall, in dem das kaputtgeht" — anderer Agent, nicht derselbe.

## f) Diese Dateien fasst NUR der Hauptkopf an

Sie werden als **ganze** Datei geschrieben; zwei Agenten mergen sie nicht (ein Schreiber pro
Datei, §5).

| Datei | warum |
|---|---|
| `Cargo.toml` | zwei Agenten, zwei Dependency-Listen, eine überlebt |
| `src/main.rs` | **die Plugin-Liste** ist die Naht des ganzen Projekts |
| `src/lib.rs` | dito für die Modulliste |
| `assets/data/*.ron` | RON wird als ganze Datei geschrieben — verlorene Zeilen sieht niemand |
| `docs/STATUS.md`, `docs/TODO.md` | **der Hauptkopf trägt ein**, Subagenten *melden* nur |

**Gut parallel geht dagegen:** ein Agent pro **Domäne** (`vector/`, `titan/`, `world/`,
`hud/` …), sobald das Skelett steht · ein Agent pro `tools/blend/*.py` · ein Agent pro
Doku-Datei · ein Agent, der in der **installierten** Bevy-Doku nachsieht, wie eine API dieser
Version wirklich heißt (§3).

## g) Was in JEDEM Subagenten-Auftrag stehen muss

Sonst liefert er Plausibles statt Richtiges — vier Punkte, keiner davon optional:

| # | Punkt | Grund |
|---|---|---|
| 1 | **Welche Dateien ihm gehören** und welche er *nur lesen* darf | ein Schreiber pro Datei |
| 2 | **Welche Abschnitte er lesen soll** — z. B. „`prompts/init.md` §5 + §8 + §9, `docs/architektur.md`" | „lies alles" heißt: ein Subagent mit 800 Zeilen Auftrag baut den halben Prompt nach |
| 3 | **Die Belegpflicht (§9)**: was er behauptet, misst er — der Rückgabewert enthält **Testnamen, Messwert und die Stufe (§8)**, nicht „habe es implementiert" | sonst ist der Bericht eine Meinung |
| 4 | **Kein Fremdgebiet**: was ihm auffällt, aber nicht gehört, geht nach `docs/FUNDE.md` | still mitfixen ist ein unsichtbarer Merge-Konflikt |

**Das feste Berichtsformat des Rückgabewerts:**

```
Aufgabe · Getan · Beleg · Stufe · Offen · Funde
```

Die sechs Felder decken Punkt 3 (**Beleg**, **Stufe**) und Punkt 4 (**Funde**) ab; **Offen**
ist die ehrliche Restliste. Das Format steht so wörtlich in der Normtabelle des Auftrags
(`prompts/init.md` §10, Zeile 1097, dort ausdrücklich mit „(§17)" gekennzeichnet) und in
`docs/konventionen.md`: „ein Freitext-Bericht ist nicht integrierbar".

**Und quer über alles:** `docs/STATUS.md` ist die einzige Wahrheit über den Fortschritt. Ein
Workflow, der etwas gebaut hat, ist **nicht fertig, solange die Zeile fehlt**.

## h) Autonomer Betrieb — der Normalfall, nicht die Ausnahme

Der User ist meistens nicht da. Das ändert nichts an der Arbeitsweise, aber es entfernt die
Instanz, die sonst widerspricht — und genau dagegen muss die Runde gebaut sein.

**Eine Runde ohne User** ist dieselbe wie mit: Ist-Zustand messen → Hypothese und
**Abnahmekriterium vorab** hinschreiben → parallel delegieren → Ergebnisse **gegen die vorher
notierten Kriterien** prüfen → integrieren → über die nächste Runde entscheiden. Der einzige
Unterschied liegt am Ende: es gibt kein „ich frag mal kurz".

| Situation | was zu tun ist | was verboten ist |
|---|---|---|
| Offene Entscheidung, die dem User gehört | `docs/FRAGEN.md` mit `ANNAHME:` **und** der Stelle, die zurückzunehmen wäre — dann weiterbauen | warten; oder still entscheiden, ohne es aufzuschreiben |
| Etwas ist kaputt und reproduzierbar | `docs/BUGS.md` mit Repro, dann Fix nach Regel 5 | „Fix auf Verdacht" ohne roten Test |
| Etwas ist kaputt, aber **kein Repro** | `docs/BUGS.md` als Gerücht markieren, an anderer Stelle weiterarbeiten | raten, bis es zufällig weggeht |
| Fremdgebiet fällt auf | `docs/FUNDE.md` | still mitfixen |
| Zweimal dieselbe Hypothese gescheitert | Zählstand in `docs/BUGS.md` beim Eintrag führen, dann nach `docs/FRAGEN.md` | ein dritter Versuch mit derselben Annahme |

**Die Gegenprobe ist im autonomen Betrieb kein Luxus, sondern der Ersatz für den User.** Wäre
er da, würde er widersprechen — jetzt muss das ein Agent tun, der das Ergebnis nicht selbst
gebaut hat. Regel 6 aus e) wird damit zur Pflicht nach *jeder* Baustufe, nicht nur nach
Findungsstufen. Diese Sitzung hat das belegt: zwei sorgfältig gebaute Schnittstellenentwürfe
wurden von zwei unabhängigen Angreifern mit 30 belegten Funden gekippt, darunter ein Pendel,
das bei kurzem Seil 99,2 % Tempo pro Sekunde verloren hätte, und ein Bildkriterium, das nicht
belegbar war, weil sich die Kamera in diesem Projekt gar nicht dreht. Beides hätte niemand
bemerkt, bis es im Spiel auffällt.

**Und die Obergrenze:** ✅ setzt nur der User. Autonom ist **🟧 das Maximum**, auch wenn alles
grün ist. Unsicherheit setzt die Stufe herunter, nicht hinauf.

## Lücken

| Lücke | was fehlt |
|---|---|
| ~~„bei erreichtem **Limit**"~~ | **geschlossen 2026-08-09:** pro `F-ID` höchstens **zwei** Bauversuche und eine Gegenprobe; danach ist nicht die Ausführung falsch, sondern die Annahme → `docs/FRAGEN.md`. Pro Runde höchstens **vier bauende** Aufträge (siehe d). |
| ~~„zweimal dieselbe Hypothese gescheitert"~~ | **geschlossen 2026-08-09:** der Zählstand steht beim Eintrag in `docs/BUGS.md`. Ohne Eintrag gibt es kein „zweites Mal", also auch keinen zweiten Versuch. |
| Fachbereiche | die Liste der Senior-Experten ist mit „…" offen — welche Domänen tatsächlich angelegt werden, ist eine Entscheidung, die noch niemand getroffen hat. |
| Querverweis | §17 verweist beim seriellen Arbeiten auf „die Liste oben"; die Tabelle der Hauptkopf-Dateien steht im Quelltext weiter **unten**. Gemeint ist dieselbe Liste. |
| Gemessen ist nichts | **teilweise geschlossen 2026-08-09:** auf Maschine B (`offlinebot`) sind es `nproc` = 16, aber die Breite hängt nicht daran — **`cargo` nimmt einen Lock auf `target/`**, also warten bauende Agenten aufeinander. Vier gleichzeitig ist die brauchbare Grenze, nicht sechzehn. Für Maschine A ist der Wert weiterhin 🟨. |

Verwandt: [workflow](workflow.md) · [performance](performance.md) · [STATUS](../STATUS.md) · [TODO](../TODO.md) · [FRAGEN](../FRAGEN.md) · [FUNDE](../FUNDE.md) · [BUGS](../BUGS.md) · [ABNAHME](../ABNAHME.md) · [ROADMAP](../ROADMAP.md) · [architektur](../architektur.md) · [konventionen](../konventionen.md) · [umgebung](../umgebung.md) · [README](../README.md)
