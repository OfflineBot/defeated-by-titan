# move-and-slide — measurement report, 2026-08-09

Updated: 2026-08-09 · Stage: 🟧 (command output, reproducible)

**Aufgabe** — MoveAndSlide aus avian3d 0.7.0 am Quelltext beantworten und als Schiedsrichter zwischen Seil und Geometrie fahren (P1–P4), nur in `examples/probe_avian.rs`.

**Getan** — `/home/offlinebot/Documents/defeated-by-titan/examples/probe_avian.rs` von 1986 auf 3192 Zeilen erweitert, F0–F12 unveraendert (Physikzahlen bitgleich nachgeprueft, nur die ms-Spalten schwanken). Neu: F13 (API + drei Kontrollen), F14 = P1, F15 = P2, F16 = P3, F17 = P4. Vier Varianten: (A) Joint · (A+MS) Joint + MoveAndSlide als Nachkorrektur · (B) eigene Klemme ohne Schiri · (B+MS) eigene Klemme + MoveAndSlide. Volle Ausgabe: `/tmp/claude-1000/-home-offlinebot-Documents-defeated-by-titan/3285eda4-fff6-4374-861e-3da38ba9f775/scratchpad/probe_ausgabe_final.txt`.

**Die fuenf Quelltextfragen** (alle avian3d-0.7.0/src/character_controller/move_and_slide.rs, ganz gelesen, 1131 Zeilen):
1. **SystemParam.** `#[derive(SystemParam)] pub struct MoveAndSlide<'w,'s> { pub spatial_query: SpatialQuery<'w,'s>, pub colliders: Query<..>, pub length_unit: Res<'w, PhysicsLengthUnit> }` (:66-87). Kein Component, kein Plugin. Prelude ueber mod.rs:6-12 → lib.rs:564-565, `pub mod character_controller` lib.rs:512.
2. **Es verlangt KEINEN RigidBody.** Das Wort `RigidBody` kommt in der Datei nicht vor; gelesen werden nur Collider/Position/Rotation/CollisionLayers/PhysicsLengthUnit (:69-87). **Die Architekturfrage ist damit nicht entschieden** — und der eigentliche Konflikt liegt woanders, siehe Funde.
3. `pub fn move_and_slide(&self, shape: &Collider, shape_position: Vector, shape_rotation: RotationValue, mut velocity: Vector, delta_time: Duration, config: &MoveAndSlideConfig, filter: &SpatialQueryFilter, mut on_hit: impl FnMut(MoveAndSlideHitData) -> MoveAndSlideHitResponse) -> MoveAndSlideOutput` (:485-495). `RotationValue` = `Quaternion` (physics_transform/transform.rs:141). Es **schreibt nichts** — Rueckgabe `{ position, projected_velocity }` (:257-276), der Aufrufer bleibt Schreiber. Schedule frei; avians Beispiel: `FixedUpdate` (examples/move_and_slide_3d.rs:29).
4. `skin_width` (Vorgabe 0,01 m, :237) = Abstand, den der Sweep ueberall haelt (`pull_back` :826-830) und den die Entpenetrierung zur Eindringtiefe addiert (:928). Entpenetrierung = Gauss-Seidel, bis 16 Durchlaeufe (:1019-1046). Gleitdurchlaeufe: `move_and_slide_iterations`, Vorgabe **4** (:173).
5. Rueckfallebene ja: `cast_move` (:782-821) ueber `SpatialQuery::cast_shape_predicate`, dazu `depenetrate` (:905), `intersections` (:1069), `project_velocity` (:1127) einzeln. **Nicht gebraucht — MoveAndSlide laeuft.**

**Beleg — S1/S2/S3/S4 je Variante, Ja/Nein mit Zahl**

| | S1 (P1, kein Einholen) | S2 (P2, Zug 28 m/s bis 3 m) | S3 (straffe Ticks) | S4 (6 / 24 Teilschritte) |
|---|---|---|---|---|
| **(A) Joint** | **JA**, schlimmster −0,00324 m, 0 Verstoss-Ticks | **NEIN**, −0,07580 m (v0 50, Versatz +0,35), 7 Verstoss-Ticks; auch −0,01553 bei v0 35 | 6,1–100 % (5 von 9 Zeilen < 30 %) / P2: 1,6–99,9 % (6 von 9 < 30 %) | **NEIN** 8,9663 %/s (L 5, v0 50) / **JA** 4,2563 %/s |
| **(A+MS)** | **JA**, +0,01000 m in allen 9, 0 Verstoesse | **JA**, +0,01000 m in allen 9, 0 Verstoesse | 6,2–100 % (2 von 9 < 30 %) / P2: 0,8–99,9 % (6 von 9 < 30 %) | **NEIN** 8,9663 / **JA** 4,2563 — **auf 4 Nachkommastellen identisch zu (A), Eingriffe = 0** |
| **(B) Klemme roh** | **NEIN**, −5,34181 m, 197–454 Verstoss-Ticks | **NEIN**, −2,47182 m, 415–523 Verstoss-Ticks | 80,1–100 % / 86,3–90,2 % — alle bestehen | **NEIN** 7,6209 (ehrlich) / **JA** 3,2943 |
| **(B+MS)** | **JA**, +0,01000 m in allen 9, 0 Verstoesse | **JA**, +0,01000 m in allen 9, 0 Verstoesse | 78,6–100 % / 92,9–100 % — alle bestehen | identisch zu (B), Eingriffe = 0 |

Kosten (F17, **RELEASE**, 401 statische Koerper, 300 Ticks, nur `app.update()` gestoppt), ms je Tick:

| | 4 Sp. / 6 | 4 / 24 | 20 / 6 | 20 / 24 |
|---|---|---|---|---|
| (A) | 0,2697 | 0,4940 | 0,3531 | 0,7206 |
| (A+MS) | 0,2898 | 0,4742 | 0,3762 | **0,6447** |
| (B) | 0,2485 | 0,3895 | 0,2789 | 0,4741 |
| (B+MS) | 0,2597 | 0,3971 | 0,3256 | 0,5108 |

Schlimmster Fall 0,72 ms bei 4 ms Budget = **5,5x Reserve**. (A) verletzt dort selbst S1 (−0,01070 m), (A+MS)/(B+MS) liegen bei +0,01000.

**Stufe** 🟨 — gemessen und mehrfach gegengeprueft, aber ohne Bild und ohne Test in `tests/`, der rot wird. Gegenproben, die eingebaut sind: jede Joint-Zeile in F13(b) hat einen Lauf **ohne Seil** (Weg 9,83 m) daneben, sonst waere „Abstand bleibt 5 m" trivial; jede Schiri-Zeile hat die Zeile ohne Schiri daneben (die durch die Wand geht); S3 wird je Zeile mitgezaehlt plus ein 3-s-Fenster; S4 misst mit und ohne Schiri im selben Aufbau.

**Offen**
- **S3 wird in vielen Joint-Zeilen verfehlt** — nicht weil das Seil kaputt ist, sondern weil der Spieler nach ~2 s ueber die Dachkante gezogen wird und auf dem Dach liegt; dann ist das Seil dauerhaft schlaff. Die Gegenprobe „nur bis 6 m einholen" ist eingebaut und bessert es teilweise (A+MS 3 von 9 statt 6 von 9 unter 30 %). Wer S2 fuer den Joint hart belegen will, braucht einen Aufbau, in dem der Spieler an der Wand bleibt.
- Ungesehen: **kein Bild**, nichts im Fenster. Reibung, Boostkraft und mehrere Seile gleichzeitig sind mit MoveAndSlide nicht gefahren. Kein Netzcode. `MoveAndSlideConfig` wurde nur mit `default()` gefahren — `skin_width`, `max_planes`, `plane_similarity_dot_threshold` sind nicht variiert.
- (A+MS) greift in 84–99 % der Ticks ein; ob das langfristig stabil ist oder ein Dauerkampf gegen den Loeser, ist ueber 900 Ticks nicht zu entscheiden.

**Funde**
1. **`DistanceJoint` haelt auch einen KINEMATISCHEN Koerper** (F13b, mit Gegenprobe ohne Seil: 14,83 m → 5,00000 m, |v| 10 → 0,00). Die Annahme „ein Joint wirkt nur auf dynamische Koerper" ist fuer avian 0.7.0 **falsch**.
2. **Der echte Unvertraeglichkeitspunkt ist `CustomPositionIntegration`, nicht der Koerpertyp.** Wer selbst schreibt und trotzdem ein Joint dranhaengt, bekommt zwei Schreiber auf `Position`: der Joint zieht auf 5,00000 zurueck und **|v| explodiert auf 3530 m/s** (F13b, letzte zwei Zeilen). Der gangbare Weg ist deshalb (A+MS): dynamischer Koerper, Joint macht das Seil, MoveAndSlide faehrt die Tickstrecke als Sweep nach und schneidet ab — das laeuft, kostet nichts und haelt S1 und S2.
3. **Der Befund „ohne Einholen landet der Spieler 2–3 m IM Haus und bleibt dort" reproduziert sich nicht.** Mit exakt nachgerechnetem Kapsel-Quader-Abstand kommt (A) in P1 nie tiefer als **3,2 mm**. Die Endorte liegen bei x ≈ −3,8 bis −4,5, aber auf **y = 12,40 — ueber dem Dach (11,5 m)**; ein Ebenenmass „x + Radius gegen −3,5" meldet dort 0,3–1,0 m „im Haus", obwohl der Spieler auf dem Dach steht. Sehr wahrscheinlich ein Messfehler der letzten Runde, kein Physikfehler.
4. **Einholen: der Joint ist physikalisch richtig, die eigene Klemme frisst die Energie.** Joint 20 → **58,2 m/s** = exakt 20 · 8,7/3 (Drehimpulserhaltung); die Klemme bleibt bei **exakt 20,000**. (B) ist kein billigerer Joint, es ist eine andere Physik. Folge: mit Joint erreicht der Einholvorgang **116 m/s** bei `tempo_max_m_s` = 75 — `MaxLinearSpeed` ist Pflicht, nicht Kosmetik.
5. **MoveAndSlide kostet bei 24 Teilschritten negativ**: (A+MS) 0,6447 ms gegen (A) 0,7206 ms bei 20 Spielern. Wer den Spieler aus der Ueberlappung haelt, spart dem Kontaktloeser mehr, als der Sweep kostet.
6. Die alte Hybrid-Zahl „0 % Schwungverlust" ist als tautologisch bestaetigt und beziffert: mit `|v|`-Hochskalierung 0,0008 %/s, mit ehrlicher Projektion **7,6209 %/s** bei 6 Teilschritten (Joint 8,9663) und **3,2943 %/s** bei 24 (Joint 4,2563). Der Hybrid ist rund 15 % besser als der Joint, nicht 100 %.
7. `assets/data/game.ron` und `maps.ron` wurden nicht angefasst; `SEIL_MIN` = 3,0 wird in F15 als RON-Wert gefahren, die 6-m-Zeile ist ausdruecklich als Gegenprobe beschriftet.