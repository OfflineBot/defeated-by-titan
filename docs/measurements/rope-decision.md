# rope-decision — measurement report, 2026-08-09

Updated: 2026-08-09 · Stage: 🟧 (command output, reproducible)

**Aufgabe** · Die Schiedsrichter-Messung kippen und entscheiden. Nichts im Projektbaum angefasst (`git status` zeigt nur Fremdarbeit); eigenes Wegwerf-Programm in `/tmp/claude-1000/-home-offlinebot-Documents-defeated-by-titan/3285eda4-fff6-4374-861e-3da38ba9f775/scratchpad/entscheidung/kipp.rs`, gebaut im warmen `altlast`-Crate (bitgleiche avian-Feature-Liste), Logs `kipp.log` … `kipp4.log` im selben Ordner.

**Getan** · `examples/probe_avian.rs` (3192 Z.) und `probe_ausgabe_final.txt` gelesen, `move_and_slide.rs` (1130 Z.) ganz gelesen, sechs Gegenmessungen K1–K6 gefahren (je 900 Ticks, RELEASE).

---

## Was die Messung nicht misst

**1. `+0,01000 m` ist keine Messung, sondern die Ausgabe der Entpenetrierung.** `move_and_slide` schliesst mit einem Entpenetrier-Durchlauf (`:636-640`), der `contact_point.penetration + skin_width` aufloest (`:928`). Die Probe liest `Position` danach (`schiri_nach_joint` laeuft `.after(StepSimulation)`, `gasse_fahren` misst nach `app.update()`). Der Wert **kann** nicht kleiner als `skin_width` werden — deshalb steht in allen 36 `+MS`-Zeilen exakt `0.01000`. Die Zahl ist unfalsifizierbar. **Die fehlende Zahl habe ich nachgezogen:** Tiefe VOR der Korrektur, Korrekturgroesse. Beispiel (P2, v0 50, Versatz +0,35, n=6): vor dem Schiri −0,00479 m / 0 Verstoss-Ticks, Korrektur max 0,0182 m, Mittel 0,00333 m, Eingriffe 896/900.

**2. S3 als Laufmittel bei 1 mm ist der falsche Waechter — in beide Richtungen.**
- *Zu streng:* der Joint hat unter Last 2–5 mm Eigenfehler. Dieselbe Zeile (P1, v0 50, Versatz 0, n=6): `s1mm 6,3 %` — „S3 VERFEHLT" — aber `s5mm 100,0 %`. Das Seil ist straff, nur nicht auf 1 mm.
- *Zu grob:* die zwei Zeilen, mit denen der Bericht (A) durchfallen laesst, sind **gueltig**, nicht ungueltig. Im Tick des tiefsten Eindringens ist `|r−L| = 0,00000 m` bei `|v| = 95,71 m/s` (v0 50, Versatz +0,35, Tick 15) bzw. 74,28 m/s. Das Laufmittel von 1,8 % kommt von den 880 Ticks danach auf dem Dach. **Die Begruendung „S3 verfehlt, also misst der Aufbau nichts" ist widerlegt.**

**3. Der Variantenvergleich ist unfair.** (B) ist `RigidBody::Kinematic` — unendliche Masse, avians Kontaktloeser bewegt ihn nie. „(B) geht durch die Wand" ist die Definition von kinematisch, kein Befund. Und `straff` ist bei (B) durch die harte Klemme `p = anker + d·(L/r)` (`:2244-2246`) per Konstruktion erfuellt, beim Joint ein Loeserergebnis. Die Spalten sind nicht vergleichbar.

**4. S1/S2 wurde bei 6 Teilschritten gemessen, S4 verlangt 24.** Keine Konfiguration wurde gegen alle vier Bedingungen zugleich gefahren.

## Die Kernaussage ueber MoveAndSlide — halb richtig, und die andere Haelfte ist die teure

„Es verlangt KEINEN RigidBody" stimmt fuer die **bewegte** Form. Fuer die **Welt** ist es falsch: `MoveAndSlide.colliders` ist `(With<ColliderOf>, Without<Sensor>)` (`:82`), und `ColliderOf` setzt `ColliderHierarchyPlugin` nur, wenn der Collider selbst oder ein Vorfahre einen `RigidBody` hat (`collider_hierarchy/plugin.rs:14-40`). Gemessen (K1, Kapsel 60 Ticks mit 30 m/s auf eine 0,5-m-Wand):

| Wand | Abstand min | x Ende | MS-Treffer | Strahl sieht sie? |
|---|---|---|---|---|
| `RigidBody::Static` + Collider | **+0,01052 m** | −0,611 | 42 | JA |
| nur `Collider` | **−0,60000 m** | **+20,000** | 0 | JA |

Ein Collider ohne `RigidBody` ist fuer den Schiedsrichter unsichtbar, waehrend jeder Strahl ihn trifft. Stiller, geometrieabhaengiger Ausfall.

## Die Zahl, die alles kippt

Der Ausfall ist **tempogetrieben, nicht teilschrittgetrieben**. Mehr Teilschritte machen ihn stellenweise schlimmer, weil das Einholen mehr Drall liefert: (A), P2, v0 50, Versatz −0,25 → n=6: −0,00224 m; **n=24: −0,14399 m, 1 Verstoss, `|r−L| = 0,00000` im Tick, `straff(1mm) = 100,0 %`, `|v| = 127,45 m/s`.** Eine bei jedem Massstab gueltige Zeile.

Die Ursache ist bekannt und unbehandelt: der Joint erhaelt den Drehimpuls, das Einholen erreicht 116–130 m/s bei `tempo_max_m_s = 75`. Mit `MaxLinearSpeed(75)` (`integrator/mod.rs:466-482`, je Teilschritt) und 24 Teilschritten, **ohne jeden Schiedsrichter** (K5, alle 9 P2-Zeilen):

−0,00051 · −0,00068 · −0,00126 · −0,00226 · −0,00246 · −0,00260 · −0,00289 · **−0,00355** · **−0,00431** m — **0 Verstoss-Ticks in allen neun**, `|v|max 75,44`, `s5mm 96–100 %`. Dazu P1 bei n=24: schlimmster Wert −0,00059 m.
Unabhaengige Bestaetigung aus der **eigenen** Ausgabe des Berichts, uebersehen: F17, (A) allein, 24 Teilschritte: `−0,00278` (4 Sp.) und `−0,00279` (20 Sp.), `straff 100,0 %` — S1 gehalten. Zitiert wurde nur die 6-Teilschritt-Zeile darueber.

Keine der beiden Massnahmen reicht allein: 24 Teilschritte ohne Klemme → −0,14399; Klemme bei 6 Teilschritten → −0,01553.

---

# EMPFEHLUNG

```
SEIL:           DistanceJoint            — 58,23 m/s gegen exakt 20,000 m/s
SCHIEDSRICHTER: keiner                   — schlimmster Wandabstand -0,00431 m von -0,01 erlaubt
TEILSCHRITTE:   24                       — 0,7206 ms/Tick bei 20 Spielern, Budget 4 ms
EINHOLEN:       je Teilschritt + MaxLinearSpeed(tempo_max_m_s) — 75,44 statt 677,66 m/s
```

**SEIL — DistanceJoint.** *Traegt:* Einholen aus v0 20 gibt mit dem Joint **58,231 m/s** (= 20·8,7/3, Drehimpuls erhalten), mit der eigenen Klemme **exakt 20,000** — die Klemme ist kein billigerer Joint, sie frisst das Einholen, und das Einholen ist der Kern des Vector Gear. Dazu zwei straffe Seile 0,23 mm gegen 24,9 mm. *Falsch ⇒* teuerste Ruecknahme der vier: das Spielgefuehl wird um den Drallgewinn herum eingestellt: spaeter tauschen heisst alles neu einstellen.

**SCHIEDSRICHTER — keiner, aber eine Bedingung ab heute.** *Traegt:* 18 Zeilen (P1 und P2, drei v0, drei Versaetze) mit Joint + Tempoklemme + 24 Teilschritten, schlimmster Wandabstand **−0,00431 m** — 43 % der erlaubten Kollisionshaut; plus F17 mit −0,00279 bei 20 Spielern. *Falsch ⇒* **billigste Ruecknahme von allen.** `MoveAndSlide` ist ein SystemParam, kein Plugin, kein Component, und schreibt nichts (`:257-276`); nachruesten ist **ein** System in `FixedPostUpdate` zwischen `StepSimulation` und `Writeback`. **Die eine Sache, die trotzdem ab Tag 1 gelten muss:** jeder Weltcollider bekommt einen `RigidBody` (`Static` genuegt) — sonst ist der Schiedsrichter spaeter blind fuer ihn (K1) und man merkt es nur an einzelnen Haeusern. Das kostet jetzt nichts und ist spaeter Nacharbeit an jeder Kartenzeile.

**TEILSCHRITTE — 24.** *Traegt:* der einzige gemessene Wert, der S4 (4,2563 %/s gegen 8,9663 %/s bei 6) **und** S1/S2 (−0,00431 gegen −0,01553) zugleich haelt. Kostet 0,7206 ms/Tick bei 20 Spielern in einer 401-Koerper-Stadt, Budget 4 ms → **5,5-fache Reserve**; gegen 6 Teilschritte das 1,45-fache (0,4940 gegen 0,2697 bei 4 Spielern). *Falsch ⇒* frei zurueckzunehmen, es ist eine RON-Zahl (`teilschritte:` neben `simulation_hz`, `SubstepCount` daraus). Zwei Warnungen: die S4-Reserve bei 24 ist nur 15 % (4,26 von 5,0), und **12 ist nicht gemessen** — moeglicherweise reicht es und halbiert die Kosten. Das blockiert nichts, 24 passt.

**EINHOLEN — je Teilschritt, zusammen mit `MaxLinearSpeed` = `tempo_max_m_s`.** *Traegt:* bei 24 Teilschritten, v0 50, drei Versaetze — je Tick: **677,66 m/s** und −0,467 / −1,095 / **−2,534 m** durch die Wand; je Teilschritt: 130,22 m/s und −0,144 / −0,002 / −0,004; je Teilschritt **mit** Klemme: 75,44 m/s und −0,00051 / −0,00289 / −0,00355. Die Klemme allein rettet „je Tick" **nicht** (−0,01184, 1 Verstoss, Seil bis 4,80 m schlaff, `straff` faellt auf 0,1 %). Beide Schrauben sind noetig und wirken auf Verschiedenes: je Teilschritt haelt das Seil, die Klemme haelt das Tempo. *Falsch ⇒* billig: ein System im `SubstepSchedule` statt im `FixedUpdate`, plus ein Component.

**Anzufangen ist bei:** `MaxLinearSpeed(tempo_max_m_s)` auf den Spielerkoerper und `teilschritte: 24` in `game.ron` — das sind die zwei Zeilen, an denen alle drei anderen Entscheidungen haengen.

**Stufe** · 🟧 fuer die vier Empfehlungszeilen: jede hat Kommandoausgabe, jede Messung fuehrt Vorzeichen (Ueberdehnung gegen Schlaffheit), Straffheit bei 1/5/20 mm und den Seilzustand **im Tick des Verstosses** mit, und jede zentrale Behauptung hat eine Gegenprobe, die sie haette widerlegen koennen — die 24-Teilschritt-Zeile mit −0,14399 hat meine eigene Zwischenannahme („24 Teilschritte reichen") widerlegt und wurde nicht weggelassen. 🟨 fuer alles Ungesehene unten. **Kein Bild, kein Test in `tests/`, der rot wird.**

**Offen**
- **Nicht gefahren, mit keiner Variante:** Boost (34 m/s²) gegen die Wand **mit** Seil, zwei Seile gleichzeitig gegen Geometrie, Loslassen und freier Fall in die Stadt, Reibung. Das sind die drei Wege, auf denen die Empfehlung noch fallen kann.
- `MaxLinearSpeed` ist nur gegen die Wand gemessen, **nicht** gegen S4 — bei v0 ≤ 50 feuert die Klemme dort nie, S4 ist per Konstruktion unveraendert. Das ist Argument, nicht Messung.
- 12 Teilschritte sind unbekannt (eine Fahrt, ~2 min).
- `MoveAndSlideConfig` nur mit `default()`; `skin_width`, `max_planes`, `plane_similarity_dot_threshold` unvariiert.
- Ob die Empfehlung ueber 900 Ticks hinaus stabil ist, ist nicht entschieden.

**Funde** (fuer `docs/FUNDE.md`, gehoert mir nicht)
1. **`With<ColliderOf>` (`move_and_slide.rs:82`) ist eine Architekturbedingung, kein Detail** — Weltgeometrie ohne `RigidBody` ist fuer `MoveAndSlide` unsichtbar (−0,60000 m, fliegt durch), fuer `SpatialQuery::cast_ray` sichtbar. Gehoert in `docs/konventionen.md`.
2. **Das Abnahmekriterium selbst muss nachgezogen werden:** S3 mit 1 mm liegt unter dem Eigenfehler des Joints unter Last (dieselbe Zeile: `s1mm 6,3 %`, `s5mm 100,0 %`). Empfehlung: **5 mm**, und S3 zusaetzlich **im Tick des Verstosses** pruefen statt als Laufmittel — genau dort war `|r−L| = 0,00000`.
3. **Ein Schiedsrichter darf nie an seiner eigenen Ausgabe gemessen werden.** Jede kuenftige Messung muss die Tiefe **vor** der Korrektur und die Korrekturgroesse mitfuehren, sonst ist sie unfalsifizierbar.
4. **MoveAndSlide ist kein reines Sicherheitsnetz, es aendert die Bahn in beide Richtungen.** Es verhindert Zustaende (v0 50/+0,35/n=6: vor dem Schiri −0,0048 statt −0,0758 ohne ihn) und erzeugt welche (n=24, v0 35, +0,35: vor dem Schiri **−0,01661 mit 7 Verstoss-Ticks**, waehrend (A) allein bei −0,00424 und 0 liegt). Es greift in 84–99 % der Ticks mit im Mittel 3,3 mm ein und laesst das Seil bis **2,46 m** schlaff werden, wo (A) allein ≤ 0,36 m bleibt.
5. Als **alleiniger** Schreiber (kinematisch + `CustomPositionIntegration`) ist es sauber — dann fehlt aber die Einhol-Physik (exakt 20,000 m/s statt 58,2).
6. Die `ms/Tick`-Spalten in F14/F15 schwanken um das Dreifache (0,208 … 0,725) bei gleicher Arbeit — Maschinenrauschen durch parallele Builds. Nur F17 ist als Kostenmessung brauchbar; die Spalte gehoert aus F14/F15 heraus oder mit einer Warnung versehen.