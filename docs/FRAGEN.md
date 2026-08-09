# FRAGEN — Entscheidungen, die mir nicht gehoeren

Stand: 2026-08-09

**Hier wird nicht unterbrochen.** Jede Frage bekommt eine `ANNAHME:`, mit der bis zur Antwort
weitergearbeitet wird — und die Arbeit laeuft daran vorbei weiter, statt zu warten
(`prompts/init.md` §2, §10). Beantwortete Fragen wandern nach unten, sie werden nicht geloescht.

---

## Q-001 — Gibt es ausserhalb von Roblox ueberhaupt einen Store?

**Kontext:** Die Bibel hat als fuenften Designpfeiler „Der Store verkauft nur Aussehen"
(Kosmetik, Privatserver, Saisonpass) und `01_Spielfunktionen` enthaelt **5 Zeilen
`Monetarisierung`** und **5 Zeilen `Live Ops`**. `prompts/init.md` §2 sagt dazu: Robux,
Saisonpass und Roblox-Store **entfallen in dieser Form**, der *Grundsatz* bleibt, und ob das
ausserhalb von Roblox ueberhaupt stattfindet, ist eine Produktfrage.

**ANNAHME:** Nichts davon wird gebaut. Die zehn Zeilen bleiben ⬜ in `docs/STATUS.md` und
verschwinden **nicht** aus `docs/features.ron` — sie sind erfasst, nicht gestrichen.

## Q-002 — Ist 1 stud = 0,28 m der richtige Umrechnungsfaktor?

**Kontext:** Backlog und Bibel rechnen in studs (Roblox-Mass), dieses Projekt in Metern
(`prompts/init.md` §3: 1 Bevy-Einheit = 1 Meter). Der Faktor bestimmt jede Distanz im Spiel.

**Beleg fuer 0,28** (der Roblox-Wert), drei unabhaengige Gegenproben:

| Backlog | × 0,28 | passt zu |
|---|---|---|
| Hakenreichweite 400 studs (`F-002`) | 112 m | `prompts/init.md` §1: „ein Haken fliegt 60–120" |
| Ashgate District 2000 × 2000 studs | 560 × 560 m | Missionsbogen 5–7 min |
| Titanwood 3000 × 3000 studs | 840 × 840 m | groesste Map |

**ANNAHME:** 0,28 m/stud. Die Umrechnung passiert **einmal**, beim Uebernehmen einer Zahl in
eine `assets/data/*.ron`; im Code gibt es keine studs. Ist der Faktor falsch, aendern sich nur
RON-Zahlen, kein Code.

## Q-003 — PvP ja oder nein?

**Kontext:** Bibel 8.1. Das Spiel ist als reines Koop spezifiziert. PvP waere eine getrennte
Balancing-Linie, serverautoritative Trefferpruefung und dauerhafte Wartung — „kein Feature,
sondern ein zweites Projekt". Die Bibel sagt selbst: **jetzt entscheiden, nicht in Monat 12.**

**ANNAHME:** reines Koop. `squad/` baut auf „kein Schaden, keine Kollision zwischen Spielern".

## Q-004 — Vessel Forms in v1.0 oder v1.5?

**Kontext:** Bibel 8.2 — teuerster Einzelposten (eigene Rigs, ~60 Animationen, eigenes
Balancing), ersetzt das Kern-Movement statt es zu erweitern. Vorschlag der Bibel: v1.5 planen,
technisch vorbereiten.

**ANNAHME:** v1.5. Die 9 `Vessel Form`-Zeilen stehen in `docs/ROADMAP.md`, nicht im Bauplan.

## Q-005 — Handel zwischen Spielern ja oder nein?

**Kontext:** Bibel 8.3. Bindung gegen Betrug, Schwarzmaerkte, Supportaufwand. Bei Pfeiler P3
(kein Fortschritt ohne Garantie) sinkt der Nutzen deutlich.

**ANNAHME:** nein. Nichts im Code setzt Handel voraus.

## Q-006 — Musik: eigener Komponist oder Lizenzbibliothek?

**Kontext:** Bibel 8.4 und Bibel 6.4 (Risiko Audio-Rechte): **ausschliesslich originale oder
lizenzierte Musik**. Betrifft Budget und Zeitplan ab P4.

**ANNAHME:** Bis zur Antwort gibt es nur **Klang-Rezepte** (`tools/sound/*.py`, reproduzierbar)
und CC0-Platzhalter unter `assets/extern/` mit Zeile in `HERKUNFT.md`. Keine fremde Musik im
Repo, auch nicht als Platzhalter.

## Q-007 — Welche Lizenz bekommt das oeffentliche Repo?

**Kontext:** `prompts/init.md` §18 Schritt 2 verlangt eine `LICENSE` vor der
Veroeffentlichung — und ausdruecklich: **keine Lizenzdatei erfinden**, wenn der User nichts
gesagt hat.

**ANNAHME:** keine `LICENSE` anlegen. Das Repo geht ohne Lizenzdatei online (= alle Rechte
vorbehalten), bis der User waehlt.

## Q-008 — Dedizierter Server oder Host, und wie viele Spieler genau?

**Kontext:** `prompts/init.md` §6 stellt die Frage ausdruecklich; die Bibel 3.6 legt die
**Zahlen** schon fest (20 pro Einsatz, 10 pro Raid, 40 im Hub) und das **Autoritaetsmodell**
(eigene Bewegung beim Client, alles andere beim Server), aber nicht die Betriebsform.

**ANNAHME:** Zahlen der Bibel gelten. `src/net/` bleibt Transport-agnostisch: `LocalOnly`
heute, und nichts im Code entscheidet, ob der spaetere Server dediziert ist oder ein Host.
Details in [`docs/multiplayer.md`](multiplayer.md).

## Q-009 — Liefert Offscreen-Rendering auf Maschine A wirklich ein Bild?

**Kontext:** `prompts/init.md` §14 laesst als Ausnahme zu, dass ein Bild aus einem
Render-Target statt aus einem Fenster kommt — **aber erst, wenn bewiesen ist, dass es auf dem
N100 wirklich ein PNG liefert.** Behauptet ist es nichts wert. Ohne diesen Beweis ist die
Obergrenze auf Maschine A **🟨**, und dieses Projekt hat bisher **kein einziges Bild**.

**ANNAHME:** Es geht nicht, bis es gemessen ist. Alles, was auf A gebaut wird, bleibt 🟨 mit
dem Vermerk *„Logik getestet, Pixel ungesehen — Maschine A"*. Steht als Aufgabe in
`docs/TODO.md`.

## Q-010 — Die Ankerdichte braucht eine Zahl

**Kontext:** `prompts/init.md` §2 nennt die Ankerdichte „die wichtigste Zahl" in `08_Maps` —
im Backlog steht dort aber **`Hoch` / `Mittel` / `Niedrig` / `Sehr hoch`**, also keine Zahl.
Bibel 6.2 macht die Ankerdichte zum Gate von P3. Qualitativ laesst sich das nicht tunen und
nicht pruefen.

**ANNAHME:** Ankerdichte wird als **hakbare Flaechen pro 1000 m²** definiert, die vier Stufen
werden beim Bau von Ashgate District an gemessenen Traversal-Zeiten geeicht, und die Zahl
landet in `assets/data/maps.ron`. Der Bezug zur qualitativen Spalte bleibt in
`docs/backlog/maps.ron` erhalten.

## Q-011 — Was passiert mit den 24 `Could`-Zeilen bei Terminkonflikt?

**Kontext:** Bibel 6.4 und `prompts/init.md` §2: „Bei Terminkonflikt fallen zuerst alle
`Could`" — keine Empfehlung. Es gibt aber keinen Termin in diesem Auftrag.

**ANNAHME:** Kein Termin, also faellt nichts. Die MoSCoW-Reihenfolge bestimmt nur die
**Reihenfolge** in `docs/TODO.md`: 139 Must vor 81 Should vor 25 Could.

## Q-012 — Was heisst `avian3d` fuer spaeteres Rollback im Multiplayer?

**Kontext:** Der User hat am 2026-08-09 entschieden: **`avian3d` wird benutzt.** Vorher war
eine Eigenbau-Loesung aus achsenparallelen Kaesten geplant. Die Entscheidung ist gefallen und
steht nicht zur Debatte — offen ist ihre **Folge**: `docs/multiplayer.md` verlangt eine
Architektur, die spaeteres Rollback nicht teuer macht. Eine fremde Physik-Engine haelt aber
Zustand, den wir nicht selbst schreiben (Kontaktcaches, Warmstart-Impulse, Schlafzustaende) —
und was davon fuer einen Schnappschuss gesichert werden muss, entscheidet, ob Rollback
spaeter eine Woche oder einen Monat kostet.

`avian3d 0.7.0` verlangt exakt `bevy 0.19.0` (gepruefte Fundstelle:
`~/.cargo/registry/src/*/avian3d-0.7.0/Cargo.toml`) und bringt ein Feature
`enhanced-determinism` mit, das `libm` einschaltet.

**ANNAHME:** `avian3d 0.7.0` mit `enhanced-determinism`. Der Physikzustand wird als
**wiederherstellbar** behandelt, bis das Gegenteil gemessen ist; die Simulation laeuft
weiterhin in `FixedUpdate`, und Eingabe bleibt ein `Intent` (§6 Regel 2).

**Zurueckzunehmen waere:** die Zeile in `Cargo.toml`, die Autoritaetstabelle in
`docs/architektur.md` (avian schreibt `Transform`/`Position` und `LinearVelocity` selbst), und
jede Stelle, die einen avian-Typ statt eines eigenen benutzt. Die Domaenenstruktur, der
`Intent`-Kanal und die RON-Werte bleiben davon unberuehrt — das war der Zweck des Schnitts.

## Q-013 — Wie lang darf ein Seil hoechstens sein?

**Kontext:** Belegt in dieser Sitzung durch eine vollstaendige Suche: **keine Quelle nennt eine
maximale Seillaenge.** Nicht die Design-Bibel, nicht `docs/features.ron` (F-001, F-004, F-005),
nicht `assets/data/game.ron`. Es gibt nur `vector.seil_min_m` (3,0 m) und
`vector.hakenreichweite_m` (112 m — die Reichweite des *Hakens*, nicht die des *Seils*). Ohne
eine Obergrenze ist F-004 nicht vollstaendig spezifiziert: es entscheidet, ob man an einem
Turm haengend noch 200 m weit pendeln kann.

**ANNAHME:** Die Seillaenge ist der **Abstand im Moment des Verankerns**, gedeckelt auf
`vector.hakenreichweite_m` (112 m). Danach wird sie nur noch **verkuerzt** (F-005), nie
verlaengert — ausser die Kollision draengt den Spieler heraus, dann wird sie nachgezogen und
der Haken loest bei Ueberdehnung.

**Zurueckzunehmen waere:** ein einziger neuer RON-Wert (`vector.seil_max_m`) und die Stelle,
die beim Verankern die Laenge setzt. Kein Strukturbruch.

---

## Beantwortet

*(noch nichts — die erste Antwort des Users kommt hierhin, mit Datum)*
