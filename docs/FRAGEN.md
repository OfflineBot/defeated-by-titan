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

### Nachtrag 2026-08-09 — die erste Gegenprobe ist weggefallen

Der User hat eine Groessentabelle geliefert und darin die Ankerreichweite **direkt mit 90 m**
angegeben (`assets/data/massstab.ron: vector.ankerreichweite_m`). Damit gilt die Vorrangregel:
**eine direkte Meterangabe des Users schlaegt jede Ableitung.** `game.ron` steht jetzt auf
90 m, nicht mehr auf 112 m.

Das trifft die Begruendung des Faktors an der Wurzel: die 112 m **waren** die erste der drei
Gegenproben oben. Rechnet man rueckwaerts, ergaeben 90 m / 400 studs einen Faktor von
**0,225 m/stud** — 20 % unter 0,28. Daraus folgt genau eine von zwei Moeglichkeiten, und
**welche, entscheidet der User, nicht ich**:

1. **0,28 gilt weiter fuer alles andere**, und die Hakenreichweite ist schlicht eine
   Spielwertentscheidung, die mit der Backlog-Zahl nichts zu tun hat. Dann bleiben Ashgate
   (560 × 560 m) und Titanwood (840 × 840 m) so, wie sie sind.
2. **Der Faktor ist insgesamt zu hoch.** Dann schrumpfen alle bisher umgerechneten Zahlen um
   20 %: Ashgate auf 450 × 450 m, Titanwood auf 675 × 675 m — und jede Karte muesste neu
   gerechnet werden.

**ANNAHME (bis zur Antwort):** Moeglichkeit 1. Der Faktor 0,28 bleibt fuer alles, wozu der
User **nichts** gesagt hat; wo er eine Meterangabe macht, gilt seine Angabe und die
Umrechnung wird gar nicht erst bemueht. Die zwei verbliebenen Gegenproben (Ashgate,
Titanwood) tragen den Faktor allein — **das ist duenner als vorher**, und die Stufe von
`docs/konventionen.md` §1 ist entsprechend zu lesen.

**Zurueckzunehmen waere:** die Zeile `1 stud = 0,28 m` in `docs/konventionen.md` §1 und jede
`groesse_m` in `assets/data/maps.ron`, die aus einer stud-Zahl entstanden ist. Nicht
betroffen: alles aus `assets/data/massstab.ron` — das kommt direkt vom User und wurde nie
umgerechnet.

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
`vector.hakenreichweite_m` (**90 m** — die Reichweite des *Hakens*, nicht die des *Seils*).
Ohne eine Obergrenze ist F-004 nicht vollstaendig spezifiziert: es entscheidet, ob man an einem
Turm haengend noch 200 m weit pendeln kann.

**ANNAHME:** Die Seillaenge ist der **Abstand im Moment des Verankerns**, gedeckelt auf
`vector.hakenreichweite_m` (90 m). Danach wird sie nur noch **verkuerzt** (F-005), nie
verlaengert — ausser die Kollision draengt den Spieler heraus, dann wird sie nachgezogen und
der Haken loest bei Ueberdehnung.

**Zurueckzunehmen waere:** ein einziger neuer RON-Wert (`vector.seil_max_m`) und die Stelle,
die beim Verankern die Laenge setzt. Kein Strukturbruch.

*Nachtrag 2026-08-09:* hier stand zweimal **112 m**. Die Zahl ist seit der Groessentabelle des
Users 90 m (Q-002); Q-018 und Q-019 wurden nachgezogen, diese Frage nicht. Korrigiert.
Praktisch heisst das: der Deckel ist 20 % niedriger, und in der flachen Stadt spielt er ohnehin
keine Rolle — brauchbar sind dort Seile von 3 bis 7 m (Q-022).

## Q-014 — Wie gross ist eine Gitterzelle, und ist das Gitter dreidimensional?

**Kontext:** `assets/data/game.ron: welt.zelle_m` steht auf 8,0 m und ist **ungemessen** —
`docs/lessons/performance.md` sagt woertlich: „§11 sagt ‚Gitterzellen', nicht wie gross. Muss
gemessen werden." Bis 2026-08-09 verwies `game.ron` fuer diese Zahl auf **Q-013**, und Q-013 ist
die Frage nach der maximalen Seillaenge. Die Frage hatte also einen Verweis, aber keine Frage.

Was sich mit der Groessentabelle geaendert hat:

| Groesse | Wert | bei `zelle_m` 8,0 |
|---|---|---|
| Gassenbreite | 7,0 m | **keine** Zelle enthaelt nur Gasse |
| `haus_gross` | 11,5 m | belegt 32 Zellen — unter `grosskoerper_zellen` (64), geht also ins Gitter ✓ |
| Mauer | 120 × 45 m | 4500 Zellen ⇒ Grosskoerperliste ✓ |
| Ashwalker | 150 m | 475 Zellen ⇒ Grosskoerperliste ✓ |
| 90-m-Strahl | | kreuzt 11,2 Zellen waagerecht, 15 senkrecht an der Mauer |

Zweite, bisher ungestellte Haelfte der Frage: **`welt.halbe_ausdehnung_m` (300 m) wurde nur
gegen die Ebene gerechnet** (400/2 + 90 = 290). In der Hoehe stehen Mauer (120 m) und Ashwalker
(150 m) uebereinander — 270 m, also 30 m Rand. Ob das Gitter ueberhaupt eine Y-Achse hat, ist
offen: `src/world/index.rs` ist ein leerer Rumpf.

**ANNAHME:** `zelle_m` bleibt bei 8,0 m und `halbe_ausdehnung_m` bei 300 m, bis `world/index`
steht und gemessen werden kann. `tests/data.rs::t005_das_gitter_traegt_auch_die_hoehe_der_welt`
haelt fest, dass die Ausdehnung die 270 m deckt — unabhaengig davon, ob das Gitter sie heute
nutzt. Ein Gitter, das kleiner ist als seine Welt, ist in beiden Faellen falsch.

**Zurueckzunehmen waere:** zwei Zahlen in `assets/data/game.ron: welt`. Kein Code.

## Q-015 — Wie duenn darf eine Wand sein?

**Kontext:** `assets/data/game.ron: welt.wand_min_m` (0,5 m) kalibriert `spieler.schritt_max_m`
und damit die gesamte Tunnelsicherung (F-012): der Integrator darf pro Teilschritt nie weiter
als die duennste Wand. Die Zahl steht in **keiner Quelle** — `docs/backlog/modelle.ron` A-080
nennt ein 4er-Raster, aber das ist die Modulbreite eines Bauteils, nicht seine Dicke.
`game.ron:109` verweist seit dem Eintrag der Zahl auf diese Frage; **geschrieben war sie nie.**

Nachgerechnet mit den heutigen Werten: `tempo_max_m_s` 75 ⇒ 1,25 m je 60-Hz-Tick ⇒ bei
`schritt_max_m` 0,25 fuenf Teilschritte à 0,25 m, und `wand_min_m` ist genau das Doppelte
davon. Sauber kalibriert — aber auf einer erfundenen Zahl.

**ANNAHME:** 0,5 m. Jede Wand duenner als das ist ein Levelfehler und kein Physikfall; der
Waechter `t005_der_teilschritt_ist_kleiner_als_die_duennste_wand` haelt die Beziehung fest,
egal welche Zahl der User nennt.

**Zurueckzunehmen waere:** `welt.wand_min_m` und, falls die Wand duenner wird,
`spieler.schritt_max_m` im selben Verhaeltnis. Kein Code — das ist der Zweck der Kalibrierung.

## Q-016 — Wie schnell laeuft ein Fehlschuss zurueck?

**Kontext:** `F-001` nennt „einziehend" als vierten Hakenzustand; `assets/data/game.ron` kannte
bis dahin nur den Hinflug. `vector.haken_ruecklauf_m_s` steht auf 120,0 m/s — schneller als der
Hinflug (90,0 m/s), damit ein Fehlschuss nicht doppelt bestraft wird. **Reiner Startwert, keine
Quelle**; `game.ron:51` verweist seit dem Eintrag auf diese Frage, geschrieben war sie nie.

**ANNAHME:** 120,0 m/s, also Hinflug × 1,33. Begruendung ist Spielgefuehl, nicht Physik: die
Strafe fuer einen Fehlschuss ist die verlorene Zeit *bis* zum Fehlschlag, nicht danach.

**Zurueckzunehmen waere:** eine Zahl in `assets/data/game.ron: vector`. Kein Code.

## Q-017 — Wer zahlt zuerst, wenn der Gastank fuer beides nicht reicht?

**Kontext:** `assets/data/game.ron: vector.gas_rangfolge` steht auf `[Boost, Einholen]` — bei
knappem Tank bekommt also der Boost sein Gas zuerst, und das Einholen faellt aus. Das ist eine
**Spielwertentscheidung** und deshalb ein RON-Wert und kein `if` in `src/vector/gas.rs`
(`gas.rs:14` und `game.ron:64` verweisen beide hierher; geschrieben war die Frage nie).

Die Umkehrung ist ernsthaft vertretbar: wer bei 3 % Tank noch einholen kann, kommt an eine
Wand und ueberlebt; wer nur noch boosten kann, fliegt schneller ins Nichts. `[Einholen, Boost]`
waere die verzeihende Reihenfolge, `[Boost, Einholen]` die ausdrucksstaerkere.

**ANNAHME:** `[Boost, Einholen]`. Grund: der Boost ist die Handlung, die der Spieler in dem
Moment ausdruecklich ausloest, das Einholen laeuft oft nebenher — und eine Eingabe, die
stillschweigend nicht ausgefuehrt wird, ist die schlechtere Erfahrung.

**Zurueckzunehmen waere:** die Reihenfolge in einer Zeile `assets/data/game.ron`. Der Code
liest die Liste, er kennt keine Reihenfolge.

## Q-018 — „Geschwindigkeit x1,5 vs. Standard": welcher Standard?

**Kontext:** Die Groessentabelle des Users (2026-08-09) nennt unter *Kamera / Vector Gear* eine
**Geschwindigkeit x1,5 vs. Standard**. Der Faktor ist vorgegeben, aber die **Bezugsgroesse
fehlt** — und ohne sie ist ein Faktor keine Zahl. Was in `assets/data/game.ron` steht:

| Kandidat | Wert | x1,5 waere | plausibel? |
|---|---|---|---|
| `spieler.laufen_m_s` | 6,0 m/s | 9,0 m/s | nein — ein Vector Gear, das kaum schneller ist als Laufen, ist kein Vector Gear |
| `vector.tempo_max_m_s` | 75,0 m/s | 112,5 m/s | moeglich, aber 112,5 m/s ist ueber 400 km/h |
| `vector.hakenflug_m_s` | 90,0 m/s | 135,0 m/s | eher nicht — das ist die Flugzeit des Hakens, kein Spielertempo |
| ein *Referenz*-Tempo, das nirgends steht | ? | ? | am wahrscheinlichsten — dann fehlt uns die Bezugszahl |

Der Faktor ist erfasst und liegt in `assets/data/massstab.ron: vector.tempo_faktor`. Er wird
**nirgends verrechnet**.

**ANNAHME:** `vector.tempo_max_m_s` bleibt **unveraendert bei 75,0 m/s**. Der Faktor 1,5 wird
gespeichert, aber nicht angewandt — eine Zahl mit unbekanntem Bezug in eine Formel zu setzen,
erzeugt ein Tempo, das niemand begruendet hat, und das faellt erst im Blindtest auf, wenn
schon alles andere darauf getunt ist.

**Zurueckzunehmen waere:** genau eine Zeile — `vector.tempo_max_m_s` in
`assets/data/game.ron`. `massstab.ron: vector.tempo_faktor` bleibt so oder so stehen, weil er
die Angabe des Users festhaelt. Kein Code ist betroffen.

### Nachtrag 2026-08-09 — die Stadt sagt etwas anderes als die Zahl

Gerechnet gegen die Groessen der Tabelle (Seilloeser aus `src/shared/seil.rs` nachgebildet und
gegen vier seiner eigenen Assertions geprueft):

| Quelle des Tempos | Ergebnis |
|---|---|
| reines Schwingen an einem 11,5-m-Dach | **17–21 m/s** (Energieprobe: `sqrt(2·20·11,5)` = 21,45 m/s) |
| Dach zu Dach ueber eine 7-m-Gasse, Anlauf 6 m/s | 17,09 m/s — und **12,64 m/s, wenn man einholt** |
| volles Einholen ueber eine freie 30-m-Sichtlinie | **75 m/s nach 0,97 s** fuer 5,8 von 100 Gas |
| was die Ausholphasen der Titanen lesbar lassen | **6–20 m/s** |

Was 75 m/s in dieser Stadt bedeuten: die 400-m-Graubox in **5,33 s**, ein 28-m-Block in 373 ms,
und eine 7-m-Gasse als Kurve mit **164 g** und 1228 Grad/s Drehrate — bei einer
Kamera-Halbwertszeit von 0,05 s (drei Ticks). Der Tick selbst haelt sauber: 1,25 m pro Tick,
fuenf Teilschritte, kein Tunneln.

Daraus folgt eine Aussage, die **in keiner Datei steht**: die Tabelle hat stillschweigend
**Schwingen zum Kampftempo und Einholen zum Reisetempo** gemacht. Das ist ein stimmiger
Entwurf — aber `tempo_max_m_s: 75.0` liest sich wie das Gegenteil. Zwei zusaetzliche
Beobachtungen zum Faktor: 75 / 1,5 = **50,0 m/s**, und **keine RON enthaelt eine 50**. Es gibt
im Datenbestand keinen Bezug, aus dem 75 als „x1,5" hervorgeht.

**Die ANNAHME bleibt unveraendert** — 75,0 m/s ist ein Clamp gegen Fling-Exploits (Bibel 6.4),
kein Zieltempo, und eine Zahl mit unbekanntem Bezug wird nicht angefasst. Der Nachtrag steht
hier, damit beim Eichen auf dem Tisch liegt, **wogegen** geeicht wird.

## Q-019 — Waechst der Cortex mit dem Titanen?

**Kontext:** Die Groessentabelle hat die Titanen deutlich groesser gemacht (`assets/data/titan.ron`,
2026-08-09): der Bellower ging von 10 m auf 21 m, der Warden von 8 m auf 14 m, der Husk von
5 m auf 10 m. **`cortex_radius_m` ist dabei unveraendert geblieben** — 0,40 bis 0,70 m, weil
der User dazu nichts gesagt hat und ich Spielwerte nicht erfinde.

Damit ist ein Verhaeltnis gekippt, das vorher stimmte: der Cortex des Bellowers war 0,70 m bei
10 m Koerper (7 % der Hoehe), jetzt sind es 0,70 m bei 21 m (3,3 %). Bibel und `F-030` fordern
**„Cortex aus 100 m erkennbar"** — auf 100 m ist eine 0,7-m-Kugel rund 0,4 Grad breit, also
etwa so gross wie ein Fingernagel auf Armlaenge. Die Kopfgroessenregel des Users (1/9 bis 1/10
der Hoehe) legt nahe, dass die Trefferzone **mitwaechst**: ein 21-m-Titan hat einen Kopf von
gut 2 m.

**ANNAHME:** `cortex_radius_m` bleibt vorerst **wie es ist**, absolut und je Art. Grund: es ist
ein reiner Balancing-Wert (wie leicht trifft man?), kein Massstabswert (wie gross ist das
Ding?), und er wird beim P1-Blindtest ohnehin geeicht. Es steht hier, damit die Frage beim
Eichen **auf dem Tisch liegt** statt als Ueberraschung.

**Zurueckzunehmen waere:** acht Zahlen in `assets/data/titan.ron`. Alternativ — und das waere
der konsequente Weg, wenn der User „mitwachsen" sagt — ein `cortex_anteil_radius` in
`assets/data/massstab.ron: titan`, aus dem `src/data/mod.rs` den Radius rechnet. Dann faellt
`cortex_radius_m` je Art ganz weg.

### Nachtrag 2026-08-09 — zwei Werte waren nicht zu klein, sondern unmoeglich

Die Kopfregel des Users (1/9 bis 1/10 der Hoehe) steht jetzt als Zahl in `massstab.ron`, und
damit ist etwas messbar geworden, das vorher niemand sehen konnte: **bei zwei Arten war der
Cortex groesser als der ganze Kopf.**

| Art | Klasse | Kopfhoehe (1/9) | Cortex-Durchmesser | Verhaeltnis |
|---|---|---|---|---|
| `scuttler` | klein, 4,2 m | 0,47 m | **0,80 m** | 171 % |
| `weaver` | klein, 4,2 m | 0,47 m | **0,90 m** | 193 % |

Das ist keine Balancing-, sondern eine Geometriefrage, und sie ist **aelter als die
Groessentabelle** (3,5-m-Koerper mit 0,80-m-Cortex). Beide Werte sind auf 0,20 bzw. 0,23 m
Radius korrigiert, `tests/data.rs::t005_der_cortex_passt_unter_den_kopf_des_titanen` haelt die
Obergrenze fest. **Die eigentliche Frage bleibt offen:** das Verhaeltnis Cortex zu Koerper hat
sich quer durch alle Arten halbiert.

Zwei Zahlen, die die Frage entschaerfen und deshalb hierhin gehoeren:

1. **Das engere Sichtfeld hat den Cortex fast verdoppelt.** Bei 1920 × 1080 und 60 Grad statt
   90 Grad ist der Husk-Cortex (1,10 m) auf 100 m **10,3 px** statt 5,9 px breit, auf 50 m
   20,6 px, auf 28 m 36,7 px. Sichtbar ja — zielbar erst ab etwa 4 px, das haelt bis 257 m.
2. **`F-030` fordert gar nicht 100 m.** Der Abnahmesatz lautet woertlich „Cortex ist aus 100
   Backlog-Einheiten Entfernung erkennbar" — das sind **28,0 m** (Faktor 0,28) oder 22,5 m
   (der von 90/400 implizierte Faktor 0,225). Bei 28 m ist die Anforderung um Faktor 3,5
   uebererfuellt. Welche der beiden Lesarten gilt, entscheidet diese Frage mit.

## Q-020 — „Abnormaler / Boss, 28 m": Groessenklasse oder der Errant?

**Kontext:** Die Groessentabelle des Users nennt als groesste Titanenzeile **„Abnormaler /
Boss | 28 m | Nacken 24,9 m"**. Im Projektvokabular ist „Abnormal" aber kein Groessenwort,
sondern ein **Typ**: `docs/konventionen.md` §2, `docs/backlog/namensschema.ron:24` und
`tools/normen.py` (VERBOTEN-Liste) uebersetzen ihn alle drei verbindlich zu **Errant**.

Die Zeile hat damit zwei Lesarten:

1. **Groessenklasse.** Der User beschreibt eine Groesse und benutzt „abnormal/Boss" als
   Beschriftung fuer „das Groesste, was vorkommt". Dann ist 28 m eine Klasse ohne Typ.
2. **Typangabe.** Der User sagt, der Errant sei 28 m hoch. Dann ist `assets/data/titan.ron`
   falsch: dort steht `errant` auf `mittel` (10 m).

Der Unterschied ist keine Formalie — er ist Faktor 2,8 bei einem Gegner, den der Spieler in der
zweiten Spielminute trifft.

**ANNAHME:** Lesart 1, die Groessenklasse. Gruende: die Zeile steht in einer **Groessen**tabelle
zwischen vier anderen Groessenzeilen; der User schreibt „Abnormaler / **Boss**", und „Boss" ist
in keiner Quelle ein Typ; und die vier anderen Zeilen („Kleiner Titan", „Mittlerer Titan") sind
ebenfalls Groessen, keine Typen. `massstab.ron` fuehrt die Klasse `boss` (28 m), sie ist von
keiner Art belegt, und `errant` bleibt bei 10 m.

**Zurueckzunehmen waere:** eine Zeile in `assets/data/titan.ron` (`errant` von `mittel` auf
`boss`) — mehr nicht, weil keine Hoehe je Art gepflegt wird. Genau dafuer sind die Klassen da.

## Q-021 — Sind die 55–65 Grad Sichtfeld waagerecht oder senkrecht gemeint?

**Kontext:** Der User nennt „FOV Bodenkampf 55-65 Grad" und dazu **„groesster Hebel"** — es ist
die Zahl, der er selbst die groesste Wirkung zuschreibt. Was mit ihr passiert, hat aber niemand
entschieden, es folgt aus Bevy: `src/render/mod.rs:86` legt `sicht_grad` in
`PerspectiveProjection.fov`, und Bevy dokumentiert das Feld woertlich als *„The vertical field
of view (FOV) in radians"* (`bevy_camera-0.19.0/src/projection.rs:284-287`).

| 16:9 | senkrecht | waagerecht |
|---|---|---|
| alter Wert | 90 Grad | 121,3 Grad |
| **heute** | **60 Grad** | **91,5 Grad** |
| Fenster des Users | 55 / 65 | 85,6 / 97,1 |

Waere die Angabe **waagerecht** gemeint — die uebliche Lesart, wenn jemand „FOV" sagt —,
muesste in `game.ron` **32,6 bis 39,4** stehen, nicht 55–65. Die beiden Lesarten unterscheiden
sich um Faktor **1,7** in der wirksamen Bildbreite.

Was der Wechsel von 90 auf 60 Grad senkrecht bereits gebracht hat, ist gemessen und geht in die
vom User gewollte Richtung: die Brennweite steigt von 540 auf 935 px, alles bewegt sich bei
gleichem echten Tempo **73 % schneller ueber den Schirm**, ein grosser Titan auf 60 m fuellt
30 % der Bildhoehe statt 18 %, und der Cortex auf 100 m ist fast doppelt so breit (Q-019).

**ANNAHME:** senkrecht, so wie Bevy das Feld liest. Grund: es ist die Lesart, die **ohne
Umrechnung** funktioniert, und sie erfuellt die Absicht („groesster Hebel", engeres Bild) hoerbar
besser als 32,6 Grad, die ein Fernglas waeren. `docs/konventionen.md` §1 normt Achsen, Einheiten
und Winkel — aber **keine FOV-Konvention**; sie gehoert dorthin und fehlt.

**Zurueckzunehmen waere:** zwei Zahlen in `assets/data/massstab.ron: kamera` und eine in
`assets/data/game.ron: kamera.sicht_grad`. Kein Code — `src/render/` liest nur.

## Q-022 — Die flache Stadt traegt kein Seil ueber 14,5 m. Was gilt?

**Kontext:** Zwei Vorgaben des Users treffen sich, und das Ergebnis hat niemand nachgerechnet.
Wohnbebauung 4,5–11,5 m, Ankerreichweite 90 m, dazu `vector.seil_min_m` 3,0 m. Daraus folgt
eine **Ankerdecke**: 11,5 + 3,0 = **14,50 m** ist der hoechste Punkt, den ein Seil an einem
Rasterhaus halten kann. Darueber liegt:

| Klasse | Cortex | ueber der Decke |
|---|---|---|
| `mittel_gross` (14 m) | 12,5 m | −2,0 m (passt knapp) |
| `gross` (21 m) | 18,7 m | **+4,2 m** |
| `boss` (28 m) | 24,9 m | **+10,4 m** |

**Jeder Anflug auf einen Titanen ab 14 m waere damit ballistisch:** loslassen, fliegen, ein
Vorbeiflug ohne Korrektur, und wer verfehlt, faellt mit nichts zum Haken. Sichtbar wird das als
„das Vector Gear fuehlt sich gegen grosse Titanen falsch an", und beschuldigt werden der
Seilloeser, der Boost und die Kamera — weil die Ursache eine Haushoehe und ein Seilminimum in
zwei anderen Dateien sind.

Zwei weitere Folgen derselben Geometrie, gerechnet:

- **87 % der 90-m-Hakenreichweite erzeugen an einem Rasterhaus kein Schwingen.** Ein sauberer
  Bogen braucht Seillaenge ≤ Ankerhoehe; ueber 11,5 m gibt es das nicht. Brauchbar sind Seile
  von **3 bis 7 m** — eine Gassenbreite.
- **Der Schwung ist langsamer als der Fall.** Der freie Fall aus 11,5 m dauert 1,07 s, die
  Viertelperiode eines 11,5-m-Seils 1,19–1,41 s. Man ist unten, bevor man ausgeschwungen ist.

**Die Zahlen des Users werden nicht angefasst.** Was sich aendert, ist die **Zusammensetzung
der Stadt** — und das ist ohnehin der Weg, den `assets/data/maps.ron` sich selbst
vorgeschrieben hatte („Kirche, Wachturm und Mauer werden als `kloetze` gesetzt"), ohne ihn zu
gehen: bis 2026-08-09 stand keiner dieser Bauten in einer Karte.

**ANNAHME:** Die Vertikale wird gebaut, nicht die Vorgabe gesenkt. Die Graubox traegt jetzt
Kirche (35 m), Wachturm (12 m) und einen Baum (12 m) als `kloetze` mit `sonderbau: true`; die
Ankerdecke steigt damit von 14,5 m auf **38 m** und deckt alle fuenf Groessenklassen.
`tests/data.rs` prueft beides: dass der Massstab fuer jede Klasse ein Bauwerk ueber ihrem Cortex
kennt, und dass die Startkarte wirklich einen hakbaren Sonderbau gesetzt hat.

**Offen bleibt trotzdem die Designfrage:** soll ein grosser Titan ueberhaupt aus der
Wohnbebauung heraus angreifbar sein, oder ist „such dir einen Turm" die gewollte Antwort? Das
entscheidet der User, nicht die Rechnung.

**Zurueckzunehmen waere:** die drei `sonderbau`-Kloetze in `assets/data/maps.ron` und die
beiden Waechter. Keine Zahl des Users, kein Code.

## Q-023 — Ist die Mauerflanke hakbar?

**Kontext:** `massstab.ron: mauer.plattform_hoehe_m` (60 m) wird damit begruendet, dass die
Krone (120 m) mit 90 m Reichweite nicht in einem Zug erreichbar sei. Nachgerechnet mit der
Boeschung ((45 − 28)/2 = 8,5 m Einzug, 4,05 Grad) stimmt das:

| Zug | Strecke | in 90 m? |
|---|---|---|
| Boden → Plattform | 60,15 m | ✓ |
| Plattform → Krone | 60,15 m | ✓ |
| Boden → Krone direkt | 120,30 m | ✗ |

**Aber das ist nicht der Grund fuer die Plattform.** Der hoechste Punkt der Mauerflanke, der
vom Mauerfuss aus in 90 m Reichweite liegt, ist **y = 89,78 m** — mit zwei freien Zuegen
(90 m + 30 m) waere die Krone auch ohne Plattform erreichbar. Tragend ist die Plattform erst
dann, wenn die **Flanke selbst keine Ankerflaeche ist** und nur Plattform und Krone haken
lassen. Diese Entscheidung steht nirgends.

Sie ist nicht klein: eine hakbare Flanke macht die Mauer zu einer 120 m hohen Kletterwand mit
freier Routenwahl, eine ungetaggte Flanke macht sie zu einem Bauwerk mit **zwei** Zugaengen —
und erst dann ist der Aufstieg eine Koennensfrage. Gemessen ist der Aufstieg selbst bequem:
60 m Einholen bei 28 m/s dauern 2,14 s und kosten 12,9 von 100 Gas, die ganze Mauer ueber die
Plattform 25,7 Gas.

**ANNAHME:** Die Flanke ist **nicht** hakbar, Plattform und Krone sind es. Grund: nur so ist
die Zahl 60 m, die der User genannt hat, ueberhaupt wirksam — sonst waere sie Deko, und eine
Vorgabe als Deko zu behandeln ist die schlechtere der beiden Annahmen.

**Zurueckzunehmen waere:** ein `hakbar`-Feld an den Mauerkloetzen, sobald die Mauer als
`kloetze`-Eintrag existiert. Heute steht sie in keiner Karte, also kostet die Annahme nichts.

---

## Beantwortet

*(noch nichts — die erste Antwort des Users kommt hierhin, mit Datum)*
