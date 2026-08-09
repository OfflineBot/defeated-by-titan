# joint-vs-hybrid — measurement report, 2026-08-09

Updated: 2026-08-09 · Stage: 🟧 (command output, reproducible)

**Aufgabe** · Messen, ob ein selbstgerechnetes Seil auf avians Kollision (Hybrid, B) dem `DistanceJoint` (A) ueberlegen ist — dieselben Szenarien, beide Varianten, Zahlen nebeneinander. Nichts im Projektbaum angefasst.

**Getan** · Eigenes Cargo-Projekt `hybrid` (Kopie von `kipp`, Bevy 0.19 + avian 0.7 warm), ein Programm mit beiden Varianten im selben Aufbau (gleicher Koerper, gleiche Kapsel, gleicher Ankerkoerper — nur der Seilrechner unterscheidet sich), plus zwei Reparaturvarianten (`+Wache` = Kapsel-Shapecast Richtung Anker) und einer Teilschritt-Reihe. 9 Szenarien, jede Zahl aus Programmausgabe.

**Beleg** · `/tmp/claude-1000/-home-offlinebot-Documents-defeated-by-titan/3285eda4-fff6-4374-861e-3da38ba9f775/scratchpad/hybrid/src/main.rs` · volle Ausgabe `/tmp/claude-1000/-home-offlinebot-Documents-defeated-by-titan/3285eda4-fff6-4374-861e-3da38ba9f775/scratchpad/hybrid.log`

---

### Der zweite Schreiber geht sauber — mit einer Bedingung (S0)

`hybrid_klemme` in `PhysicsSystems::Writeback`, `.before(PhysicsTransformSystems::PositionToTransform)`:
```
      t600  Position (-0.18071, 0.00000, -4.99673) · Transform (-0.18071, 0.00000, -4.99673) · Klaffen 0.000000 m
      groesste Abweichung |pos| von L = 5: 0.0000005 m  (Klemme wurde also NICHT zurueckgeschrieben)
```
Dieselbe Klemme *nach* der ganzen Physik: haelt auch, aber `Transform` haengt einen Tick hinterher — **0,068969 m Klaffen, dauerhaft**. avian nimmt die Fremdschrift nicht zurueck, weil `transform_to_position` `Position` nur uebernimmt, wenn sie seit dem letzten Physiktick unveraendert ist (`A/physics_transform/mod.rs:205-217`). `PhysicsTransformSystems` ist **nicht** im Prelude, muss aus `avian3d::physics_transform` importiert werden. Also: es geht sauber, aber nur an dieser einen Stelle.

### S1 Schwungverlust — A verliert, B nicht (und warum das nur halb zaehlt)

Kegelpendel (Seil dauerhaft straff, Schwerkraft -20, 600 Ticks), Energieverlust %/s:

| L | v0 | **A** | **B** |
|---|---|---|---|
| 5 | 20/35/50 | 3,27 / **8,23** / **12,74** | -0,68 / -0,34 / -0,19 |
| 8 | 20/35/50 | 1,65 / 4,78 / **8,33** | -0,59 / -0,32 / -0,18 |
| 11 | 20/35/50 | 0,99 / 2,98 / **5,64** | -0,52 / -0,30 / -0,18 |
| 15 | 20/35/50 | 0,60 / 1,77 / 3,56 | -0,45 / -0,28 / -0,17 |
| 20 | 20/35/50 | 0,39 / 1,05 / 2,19 | -0,38 / -0,25 / -0,16 |

**A reisst Kriterium (a) in 4 von 15 Faellen** — genau bei den kurzen Seilen, die in einer Stadt mit 4,5–11,5 m hohen Haeusern der Normalfall sind. B haelt ueberall (negativ = leichter Gewinn, siehe Offen). Ohne Schwerkraft (reine Kreisbahn, S1b) dasselbe schaerfer: A bis **17,93 %/s**, B **0,000 %/s** bei exaktem Radius (r_end = 5,00000).

Das ist die Sehne-gegen-Bogen-Verkuerzung der XPBD-Geschwindigkeitsprojektion; sie waechst mit `(v·dt/L)²`.

### S2 Einholen 20 → 5 m mit 28 m/s — beide falsch, verschieden falsch

```
  A  DistanceJoint   | max|v| 169.159 bei Tick 2 | max d|v|/Tick 149.159 | r bei t10 = 5.972 (L war 15.80!)
  B  Hybrid          | max|v|  20.000            | max d|v|/Tick   0.000 | r bei t10 = 15.800
```
A: **149 m/s Sprung in einem Tick**, und der Spieler steht bei r = 5,97 obwohl das Seil 15,80 lang ist — hineingeschleudert. B: nagelglatt, aber **kein Tempogewinn ueberhaupt** (Drall faellt 400 → 100; drallerhaltend waeren 80 m/s bei r = 5).

### S6 nennt die Ursache des Katapults

```
   Substeps |  A max|v|  |  B max|v| |  A %/s Kreis | B %/s Kreis | A ms/Tick
          1 |    68.025  |   20.000  |     32.395   |    0.000    |  0.2174
          6 |   169.159  |   20.000  |     17.932   |    0.000    |  0.2775
         12 |   337.879  |   20.000  |     12.848   |    0.000    |  0.3338
         24 |   672.462  |   20.000  |      8.512   |    0.000    |  0.4574
```
`max|v| = 28 · SubstepCount` auf drei Punkten (6→169, 12→338, 24→672). **Das Kuerzen von `limits.max` injiziert Geschwindigkeit in Hoehe der Einholrate mal Teilschrittzahl** — die Positionskorrektur wird je Teilschritt mit dem *Teilschritt*-dt in Geschwindigkeit umgerechnet. Und die Zange schliesst sich: mehr Teilschritte gegen den Schwungverlust heisst proportional mehr Katapult und +64 % Rechenzeit — und selbst bei 24 Teilschritten liegt A noch bei 8,5 %/s, also **immer noch ueber Kriterium (a)**.

### S3 Wand — der Fund ist bestaetigt und schlimmer als gerechnet

```
  A  DistanceJoint           | max x 23.9333 (Tick 14) | x Ende  5.0000 | Eindringen 24.5433 | DURCH die Wand
  B  Hybrid                  | max x  5.0000           | x Ende  5.0000 | Eindringen  5.6100 | DURCH die Wand
  A+ Joint + Wache           | max x 23.9333           | x Ende  5.0003 | Eindringen 24.5433 | DURCH die Wand
  B+ Hybrid + Wache          | max x -0.6000 (Tick 24) | x Ende -0.6000 | Eindringen  0.0100 | haelt
  A+ Joint + Wache +Klemme75 | max x 19.0083           | x Ende  5.0005 | Eindringen 19.6183 | DURCH die Wand
  A+ Joint + Wache +Klemme28 | max x -0.2778           | x Ende -0.2780 | Eindringen  0.3322 | steckt in der Wand
```
Die Gegenprobe rechnete 0,40 m Eindringen je Tick. Gemessen wird der Spieler **24 m ueber die Wand hinaus katapultiert** — nicht durchgeschoben, geschleudert. Der nackte Hybrid geht ebenso durch (langsamer, ohne Katapult): **ohne Schiedsrichter taugt keine der beiden Varianten.** Kriterium (b) faellt fuer A, A+, B — und haelt nur fuer B+ (1 cm Rest = exakt die `CollisionMargin`). Bei A wirkt die Wache **nicht**, weil sie die Laenge begrenzt, der Joint aber Geschwindigkeit einspeist; erst `MaxLinearSpeed(28)` stoppt ihn, laesst ihn dann 33 cm **in** der Wand stecken und deckelt nebenbei jedes Spielertempo auf 28 m/s.

### S4 Zwei Seile — hier gewinnt A klar

```
  A  DistanceJoint   Tick 300 | y 42.99959 | Absacken   0.41 mm | Fehler1  0.233 mm
  B  Hybrid          Tick 300 | y 42.95572 | Absacken  44.28 mm | Fehler1 24.872 mm
```
Der sequenzielle Hybrid laesst **24,9 mm dauerhaften Restfehler** stehen (jede Klemme macht die vorherige kaputt); A loest beide Zwaenge gemeinsam und landet bei 0,23 mm — **rund 100x besser**.

### S5/S7 Rechenzeit — kein Unterschied, beide weit unter dem Budget

Stadt (401 statische Koerper) + N Spieler, ms je Tick: `ohne Seil` 0,324 · `A` 0,330 · `B` 0,321 · `B+` 0,327 (4 Spieler); 20 Spieler alle 0,36–0,40 ms. Rauschen derselben Messung (5x): ±0,021–0,049 ms. **Der Seilrechner ist in beiden Varianten unter der Messschwelle.** Kriterium (c) haelt mit ~12x Reserve — Vorbehalt: `dev`-Profil, `debug-assertions` an, also eher Obergrenze; kein Rendering, keine Titanen.

### Zusatzfrage: Hybrid gleichzeitig an der Wand (SW)

```
  A  DistanceJoint   | y Ende 29.9997 | Streuung 0.1125 mm | Seil gewinnt (durch)
  B  Hybrid          | y Ende 10.0000 | Streuung 0.0000 mm | Seil gewinnt (durch)
  B+ Hybrid + Wache  | y Ende  7.2001 | Streuung 0.0000 mm | Kontakt gewinnt, ruhig
```
**Es zittert nicht — in keiner Variante.** Der Grund ist unangenehm: der Streit findet gar nicht statt. Die Klemme laeuft *nach* der Kontaktloesung, also gewinnt sie immer und vollstaendig (B: Position exakt 10,0000, Streuung 0,0000 mm — der Kontakt kommt schlicht nicht vor). Mit Wache kehrt es sich sauber um: der Kontakt gewinnt, 0,0000 mm Streuung, 0,0000 mm mittlerer Weg je Tick. A schleudert den Spieler durch die Decke und 20 m an den Anker vorbei (y = 30 statt 10).

---

**Stufe** · 🟨. Gemessen und reproduziert (zwei volle Durchlaeufe, gleiche Zahlen), aber: Scratchpad-Programm, nicht das Spiel; kein Test im Projektbaum; nichts gesehen ausser Zahlen.

**Offen**
1. **Bs Null-Verlust ist zur Haelfte tautologisch.** Das Mitdrehen erhaelt `|v|` per Konstruktion — 0,000 %/s beweist, dass die Methode tut was sie soll, nicht dass sie physikalisch richtig ist. Was gemessen und *nicht* tautologisch ist: As Verlust ist echt und gross.
2. **B hebt den Spieler an.** Unter Schwerkraft gewinnt B 0,16–0,68 %/s Energie (Spalte `B dy`: +0,01 bis +0,31 m in 10 s). Die Positionsklemme arbeitet gegen die Schwerkraft, ohne Tempo abzuziehen. Klein, aber ein Aufwaertsdrift.
3. **B hat keinen Einholschub** (S2). Ob F-012 einen Tempogewinn *will*, ist eine Designfrage — falls ja, muss die Klemme den Drall nachfuehren, und das ist ungebaut und ungemessen.
4. **Bs Mehrseilfehler** (24,9 mm) ist mit Iteration wahrscheinlich zu druecken; ungemessen.
5. Nicht gemessen: bewegte Anker, Titanenkoerper als Anker, Determinismus ueber zwei Laeufe, Netzcode.

**Funde** (Fremdgebiet, nicht angefasst)
- `git status` im Projektbaum zeigt `?? examples/` — nicht von mir, eine parallele Sitzung.
- `SubstepCount`-Vorgabe ist 6 (`A/dynamics/solver/schedule.rs:187-190`); `default_speculative_margin` ist `Scalar::MAX` (`A/collision/narrow_phase/mod.rs:252`) — spekulative Kontakte helfen gegen schnelle *Geschwindigkeit*, gegen eine Zwangs-Teleportation helfen sie nicht.
- `PhysicsTransformSystems` fehlt im Prelude — jeder Umbauplan, der die Klemme dort einordnet, braucht den expliziten Import.

---

## URTEIL: **B besser** — aber nur mit Wache, und A ist nicht billig reparierbar.

Drei Zahlen:
1. **17,93 %/s gegen 0,000 %/s** (S1b, L=5, v0=50, reiner Loeser-Verlust): A reisst Kriterium (a) bei genau den kurzen Seilen, die diese Stadt hergibt; B nicht. Und A kommt da nicht raus — 24 Teilschritte druecken es nur auf 8,5 %/s, bei +64 % Rechenzeit und 672 m/s Katapult.
2. **Eindringen 24,54 m gegen 0,0100 m** (S3): mit Wache haelt B exakt am Kapselrand; A geht mit Wache *und* mit `MaxLinearSpeed(75)` immer noch durch, und die einzige Klemme, die ihn stoppt (28 m/s), laesst ihn in der Wand stecken und macht das Spiel kaputt.
3. **0,330 ms gegen 0,321 ms** bei 4 Spielern, Rauschen ±0,03 ms (S5/S7): der Preis ist bei beiden **null**. Die Entscheidung kostet keine Rechenzeit, sie kostet nur Code.

Der Gegenpunkt, der bleiben muss: **bei zwei Seilen ist A 100x genauer** (0,23 mm gegen 24,87 mm). Wer zwei Haken gleichzeitig als Kernmechanik will, kauft mit B einen dauerhaften Millimeterfehler ein — nach heutiger Messung 25 mm auf 25 m, also 0,1 %, aber ungeprueft bei enger Geometrie.

Und der Satz, der ueber allem steht: **die Wache ist nicht optional, sie ist die Entscheidung.** Ohne Schiedsrichter zwischen Seil und Geometrie geht *jede* der beiden Varianten durch die Wand (A katapultiert, B spaziert). Der geloeschte Schiedsrichter ist teurer als die Wahl des Seilrechners.