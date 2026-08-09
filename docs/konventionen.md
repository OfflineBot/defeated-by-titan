# konventionen — Achsen, Einheiten, Begriffe, Normen

Stand: 2026-08-09 · Stufe: 🟨 (festgelegt und aufgeschrieben; `tools/normen.py` prueft den
mechanisch pruefbaren Teil)

> **Zwei Formen fuer dieselbe Sache heissen: keine Form.** Genormt heisst nicht huebsch, es
> heisst **greppbar**: `git log --oneline | grep F-014` muss die Geschichte eines Features
> beantworten (`prompts/init.md` §10). Wer eine neue wiederkehrende Sache anfaengt, normt sie
> **hier**, bevor er sie zum zweiten Mal benutzt.

## 1. Achsen, Einheiten, Blickrichtung

| Festlegung | Wert | warum |
|---|---|---|
| **Laengeneinheit** | **1 Bevy-Einheit = 1 Meter** | Ein Mensch ist 1,8; ein Titan 3–15; ein Haken fliegt 60–120. Zahlen, die man im Kopf pruefen kann. |
| **Oben** | **+Y** | Bevy-Vorgabe, nicht verhandelbar |
| **Blickrichtung** | **−Z**, `yaw = 0` heisst Blick nach −Z | Bevy-Kameravorgabe. Ein Modell wird **in der `.blend`** gedreht, nie per Offset in der Config: ein Offset-Feld pro Modell ist der Anfang von dreissig Offset-Feldern. |
| **Blender** | Z-oben modellieren, Export mit `export_yup=True` | Der Exporter dreht. **Nicht selbst rotieren**, sonst dreht es zweimal. |
| **Origin eines Koerpermodells** | **zwischen den Fuessen** | sonst steht jedes Modell halb im Boden |
| **Winkel** | im Code **Radiant**, in RON und Skripten **Grad** | Grad ist les- und tippbar, Radiant ist rechenbar. Umrechnung genau an der Grenze. |
| **Zeit** | Sekunden. Simulation in `FixedUpdate` bei **60 Hz** | §6 Regel 4: im Netz ist ein frameabhaengiges Ergebnis kein Komfortproblem, sondern Desync. |

### 1 stud = 0,28 m

Backlog und Bibel sind in **studs** geschrieben (Roblox-Mass), dieses Projekt rechnet in
Metern. Der Umrechnungsfaktor ist der Roblox-Wert **0,28 m/stud**, und er ist an den Zahlen
des Backlogs gegengeprueft:

| Backlog-Zahl | × 0,28 | Plausibilitaet |
|---|---|---|
| Hakenreichweite 400 studs (`F-002`) | **112 m** | `prompts/init.md` §1 nennt „ein Haken fliegt 60–120" ✓ |
| Ashgate District 2000 × 2000 studs | **560 × 560 m** | eine Stadt, die man in 5–7 min durchquert ✓ |
| Titanwood 3000 × 3000 studs | **840 × 840 m** | groesste Map ✓ |

> `ANNAHME:` Der Faktor ist erschlossen, nicht vom User bestaetigt — die drei Gegenproben
> oben sind der einzige Beleg. Steht in `docs/FRAGEN.md` als **Q-002**. **Die Umrechnung
> passiert einmal, bei der Uebernahme einer Zahl in eine `assets/data/*.ron`** — im Code
> gibt es keine studs, und `tools/normen.py` faellt um, wenn „stud" in `src/` auftaucht.

## 2. Begriffe — Blatt `10_Namensschema` ist verbindlich

**Kein Referenzbegriff im Code, in Assets, in der UI oder in der Doku.** Ein `nape`-Feld oder
ein `odm`-Modul ist ein Fehler, den `tools/normen.py` findet.

Maschinenlesbar: [`docs/backlog/namensschema.ron`](backlog/namensschema.ron) (40 Zeilen,
generiert). Die wichtigsten:

| statt | **hier** | im Code |
|---|---|---|
| ODM Gear / 3DMG | **Vector Gear** (VG), dt. Vektorgeschirr | Domaene `src/vector/` |
| Nape / Nacken | **Cortex** | das Empty im Modell heisst `cortex` |
| Survey Corps / Scouts | **The Vanguard** / ein Vanguard | `src/player/` |
| Thunder Spear | **Lance Charge** | (Roadmap) |
| Titan Shifting / Shifter-Form | **Bonding** / **Vessel Form** | (Roadmap) |
| Titan Serum | **Ichor Vial** | |
| Family / Clan · Perk · Artifact · Memory · Prestige | **Lineage** · **Trait** · **Relic** · **Echo** · **Ascension** | `src/progress/` |
| Gold · Gems | **Mark** · **Sigil** | |
| Pure · Abnormal · Crawler · Ducker | **Husk** · **Errant** · **Scuttler** · **Weaver** | `assets/data/titan.ron` |
| *(neu)* | **Warden** · **Lurker** · **Bellower** · **Chorus** | |
| Attack · Female · Armored · Colossal Titan | **The Bound One** · **The Dancer** · **The Bulwark** · **The Ashwalker** | Raid-Bosse |
| Town Central | **The Rookery** (Hub) | |
| Shiganshina · Trost · Outskirts · Forest of Giant Trees · Utgard · Docks · Stohess | **Ashgate District** · **Brackwall** · **The Fallow** · **Titanwood** · **Hollowkeep** · **Saltpier** · **Highspire** | `assets/data/maps.ron` |
| Walls Maria/Rose/Sina | **Ashgate / Ironrose / Highspire Ring** | |

**Drei Schreibweisen des Projektnamens, jede an genau einer Stelle:** Crate
`defeated_by_titan` (Rust mag keine Bindestriche) · Fenstertitel **„Defeated by Titan"** ·
GitHub-Repo `defeated-by-titan`. **Kein Plural-s.**

## 3. Die drei Signalfarben — unverhandelbar

| Farbe | Bedeutung | darf sonst nirgends vorkommen |
|---|---|---|
| **Zyan** | Gas, Vector Gear, Ankerpunkte | keine zyanfarbene Umgebungsdeko |
| **Bernstein** | Cortex, Schwachstellen, Ziele | keine bernsteinfarbenen Laternen |
| **Karminrot** | Gefahr, Schaden, kritischer Zustand | keine roten Daecher |

Basis dagegen gedeckt: Steingrau, Ziegelrot, Olivgruen, Sandbraun. Die Regel ist der Grund,
warum ein Spieler bei voller Geschwindigkeit in einem Gefecht mit zwanzig Mitspielern noch
erkennt, was fuer ihn relevant ist (Bibel 3.4). **Sie gilt fuer Platzhalter genauso.**

## 4. Namen im Repo

| Was | Norm | Beispiel |
|---|---|---|
| **Commit-Betreff** | `<F-ID\|bereich>: <eine Zeile, was jetzt anders ist>`, max 72 Zeichen, Deutsch, aktiv, kein Punkt, keine Emoji | `F-014 vector: Gas-Verbrauch beim Boost` |
| **Commit-Bereiche** | genau fuenf, wenn es keine F-ID gibt | `docs:` `test:` `tool:` `fix:` `chore:` |
| **Branch** | `<f-id>-<kurz>` bzw. `<bereich>/<kurz>` | `f014-gas-boost`, `fix/haken-kante` |
| **Test-Name** | `<f_id>_<die Aussage, die gilt>` — nicht `test_gas` | `f014_boost_verbraucht_gas` |
| **Test-Datei** | `tests/<domaene>.rs` | `tests/vector.rs` |
| **Screenshot** | `docs/bilder/<f-id>-<kurz>[-vorher\|-nachher].png` | `docs/bilder/f014-boost-nachher.png` |
| **Skript** | `scripts/<f-id>-<kurz>.txt`, darin `mark <f-id>-<stichwort>` | `scripts/f014-boost.txt` |
| **STATUS-Zeile** | `\| Sache \| ID \| Stufe \| Beleg \| Stand \|`, Datum **ISO** mit Maschine | `… \| 🟧 \| tests/vector.rs … \| 2026-08-09 [cachy] \|` |
| **Bug** | `B-007 <Titel>` + die vier Felder aus §9 | `B-007 Haken haelt an einer Kante nicht` |
| **Frage** | `Q-003 <Frage>` + Kontext + `ANNAHME:` | |
| **Fremdfund** | `FUND-005 <Symptom>` + Messung | |
| **Doku-Kopf** | `# <name> — <ein Satz>`, darunter `Stand: <ISO> · Stufe: <marke>` | |
| **RON-Schluessel** | `snake_case`, **deutsch**, eine Sprache pro Datei | `hakenreichweite_m` |
| **Rust** | `snake_case` Dateien/Funktionen, `CamelCase` Typen, **Domaenenordner immer Einzahl** | `src/vector/hook.rs` |
| **Subagenten-Bericht** | fest: `Aufgabe · Getan · Beleg · Stufe · Offen · Funde` | ein Freitext-Bericht ist nicht integrierbar |

⚠️ **Keine Werkzeug- oder Autorenspuren in Commit-Messages, PR-Beschreibungen oder Tags.**
Kein `Co-Authored-By`, keine Signatur, kein „generated with", kein Modellname. Eine
Commit-Message beschreibt **die Aenderung**, nicht ihren Urheber — der steht im Git-Autor-Feld
und nirgends sonst. `tools/normen.py` laesst eine Message mit `Co-Authored-By`, `Generated`,
`Claude`, `AI` oder `🤖` durchfallen.

**Eine Sprache: Deutsch, durchgehend.** Nicht heute `add gas drain`, morgen `Gas-Verbrauch`.
Umlaute werden in Quelltext, RON-Schluesseln und Dateinamen **umschrieben** (`ae oe ue ss`);
in Fliesstext (Markdown, UI-Text) stehen sie richtig.

## 5. Einheiten im Namen

Wer eine Konstante anlegt, die Meter misst, schreibt die **Einheit in den Namen** — oder den
Kommentar an die Rechenstelle, und die Zahl in die RON (§4): `hakenreichweite_m`,
`gas_pro_sekunde`, `ausholphase_s`. Ein Feld `range` ist eine Frage, kein Wert.

## 6. Keine Zombie-Dateien

**Jede Datei im Repo ist entweder verlinkt/benutzt — oder geloescht. Dazwischen gibt es
nichts.** Jede `docs/*.md` steht in [`docs/README.md`](README.md), jedes Asset in der
Registratur, jedes `tools/`-Skript in einer Doku. Wer eine Datei anlegt, verlinkt sie **im
selben Commit**. `tools/normen.py` prueft beides: kein toter Markdown-Link, keine
unreferenzierte Datei. **Nichts wird „zur Sicherheit" behalten** — kein `*_alt.rs`, kein
`titan_v2.blend`. Git ist das Backup.

Verwandt: [`docs/architektur.md`](architektur.md) · [`docs/modelle.md`](modelle.md) ·
[`docs/backlog/namensschema.ron`](backlog/namensschema.ron)
