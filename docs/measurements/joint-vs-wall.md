# joint-vs-wall — measurement report, 2026-08-09

Updated: 2026-08-09 · Stage: 🟧 (command output, reproducible)

## Aufgabe

F9-F12 messen: dringt das Seil (`DistanceJoint`) beim Einholen durch eine `welt.wand_min_m`-Wand, welche der vier Reparaturen wirkt, welcher `SubstepCount` erfuellt Bedingung (a) bei den echten Seillaengen, und loescht Wandreibung das Momentum. Besitz: **nur** `/home/offlinebot/Documents/defeated-by-titan/examples/probe_avian.rs`.

## Getan

Datei von 1063 auf 1929 Zeilen erweitert (F0-F8 unveraendert, laufen durch). `cargo run --release --example probe_avian` laeuft komplett, Exit 0. Volle Ausgabe: `/tmp/claude-1000/-home-offlinebot-Documents-defeated-by-titan/3285eda4-fff6-4374-861e-3da38ba9f775/scratchpad/lauf4.txt`.

**Gegen welchen RON-Stand gemessen** (gelesen 2026-08-09, unmittelbar vorher): `game.ron`: `seilzug_m_s 28.0` · `seil_min_m 3.0` · `wand_min_m 0.5` · `spieler.radius_m 0.35` · `hoehe_m 1.8` · `schwerkraft_m_s2 -20.0` · `simulation_hz 60.0` · `tempo_max_m_s 75.0` · `hakenreichweite_m 90.0` · `schritt_max_m 0.25`. `maps.ron`: `gasse_m 7.0` · `hoehe_min_m 4.5` · `hoehe_max_m 11.5` · Kirche 35 m · Wachturm/Baum 12 m.

---

## DIE DREI BEDINGUNGEN

**(a) Schwungverlust < 5 %/s bei L = 5..20 m — JA, aber erst ab SubstepCount = 24.**
Mit avians Vorgabe **6 faellt sie durch**: 8,97 %/s (L=5, v0=50), 6,35 % (L=5, v0=35), 5,60 % (L=8, v0=50). Mit 12: 6,42 % (L=5, v0=50) — knapp durchgefallen. Mit 24: schlechtester Wert **4,26 %/s**, alle 15 Faelle unter 5 %.

**(b) Einholen zieht NICHT durch eine 0,5-m-Wand — NEIN. Klar widerlegt.**
Mit `limits.max` bei 28 m/s ist der Spieler nach **23 Ticks (0,38 s) komplett hinter der Wand**, Eindringtiefe = voll (Deckel 1,20 m). Er kommt **nicht** wieder heraus. Bei 8 m/s Tick 77, bei 2 m/s Tick 302 — die Rate verzoegert nur, sie verhindert nichts. **JA wird es nur mit Reparatur (4a)**: Eindringtiefe **0,00000 m**, Wandebene nie ueberschritten.

**(c) Physikzeit < 4 ms mit Stadt und vier Spielern — NICHT GEMESSEN.** Diese Probe hat keine Stadt und keine vier Spieler; die `ms`-Spalten sind Wanduhrzeit ganzer 600/700-Tick-Fahrten mit 1-2 Koerpern, kein Tick-Budget. Gehoert in eine eigene Runde.

---

## F9 — Seil gegen Wand

Wand 0,5 m bei x=0, Spieler bei x=-10, Anker **dahinter** bei x=+10, Seil 20 m → `seil_min_m`.

```
                    Rate |  Tiefe max      x max | Tick>0 Tick durch | drin? |  |v| max
             28 m/s (RON)|    1.20000    16.9333 |     22        23  |  nein | 336.000
                    8 m/s|    1.20000    16.9333 |     76        77  |  nein |  96.000
                    2 m/s|    1.20000    17.5331 |    299       302  |  nein |  24.000
```

Die Spur zeigt den Mechanismus:

```
 Tick |   L [m] |    x [m] | |v| [m/s]
    1 | 19.5333 | -10.0000 |    0.0000
    2 | 19.0667 |  -4.3999 |  336.0003   <- 5,6 m in EINEM Tick
    3 | 18.6000 |  -0.6000 |    0.0000   <- Wand haelt, solange das Seil schlaff ist
 ...20| 10.6667 |  -0.6000 |    0.0000
```

**Kontakte funktionieren.** Ohne Einholen — Pendel schwingt mit 20/35/50 m/s in dieselbe Wand — ist die Eindringtiefe bei allen drei **0,00000 m**, x_max bleibt bei -0,6000. Der Kontakt haelt auch die 336 m/s aus, **solange das Seil schlaff ist** (Ticks 3-20). Es scheitert genau in dem Tick, in dem der Seilzwang wieder greift.

## F10 — die vier Reparaturen (alle bei 28 m/s)

```
                   Reparatur |  Tiefe max    x max | Tick>0 durch | drin? |  |v| max
  Vorgabe (nichts geaendert) |    1.20000  16.9333 |    22    23  |  nein | 336.000
        (1) max_overlap =  20|    1.20000  16.9333 |    22    23  |  nein | 336.000
        (1) max_overlap = 100|    1.20000  16.9333 |    22    23  |  nein | 336.000
        (1) max_overlap =1000|    1.20000  16.9333 |    22    23  |  nein | 336.000
       (2) compliance = 1e-5 |    1.20000  15.1246 |    22    23  |  nein | 188.617
       (2) compliance = 1e-4 |    1.20000  13.1916 |    22    22  |  nein |  92.653
       (2) compliance = 1e-3 |    1.20000  16.6319 |    28    29  |  nein | 146.071
       (2) compliance = 1e-2 |    0.13319  -0.4668 |     -     -  |   JA  |  55.780
   (3) je Teilschritt, n = 6 |    1.20000  13.0000 |    23    24  |  nein |  31.992
   (3) je Teilschritt, n = 24|    1.20000  13.0000 |    23    24  |  nein |  30.674
     (Vergleich je Tick,n=24)|    1.20000  28.6000 |     2     2  |  nein |1344.002
 (4a) Tempo statt Laenge,n=6 |    0.00000  -0.6000 |     -     -  |   JA  |  28.000
 (4a) Tempo statt Laenge,n=24|    0.00005  -0.6000 |     -     -  |   JA  |  28.000
```

- **(1) `max_overlap_solve_speed` wirkt UEBERHAUPT NICHT.** 4 → 1000 aendert kein einziges Digit. Nebenwirkung ebenfalls null: liegender Wuerfel 0,0000 mm Streuung, 0,00000 m/s bei allen vier Werten; 0,2 m Ueberlappung ist bei allen ab Tick 10 aufgeloest, Auswurf 0,148 / 0,140 / 0,140 / 0,140 m/s. **Der Schalter ist fuer dieses Problem tot** — und auch nicht gefaehrlich.
- **(2) `compliance` wirkt erst bei 1e-2**, und das kostet zu viel: Dehnung **0,596 m bei L=11** und **1,498 m bei L=5** (30 % der Seillaenge) im normalen Pendeln. 1e-3 haelt schon nicht mehr (Dehnung 0,059/0,432 m). Kein brauchbares Fenster.
- **(3) Verkuerzung auf die Teilschritte verteilen: GEHT** (am Quelltext belegt, s.u.) und toetet die Tempoexplosion vollstaendig — 336 → **31,99 m/s**, also genau `rate` + Bahntempo. Kostet nichts (182 vs 189 ms). **Haelt die Wand aber trotzdem nicht**: voll durch in Tick 24.
- **(4) Motor: nicht baubar.** `DistanceJoint` hat in 0.7.0 **kein Motorfeld** (`body1, body2, anchor1, anchor2, limits, compliance`, `joints/distance.rs:26-39`). `LinearMotor` ist Feld von `PrismaticJoint` (`prismatic.rs:55,:319`), `AngularMotor` von `RevoluteJoint` (`revolute.rs:74,:349`). Ein `PrismaticJoint` waere kein Seil (zwingt zusaetzlich auf `slider_axis`, `prismatic.rs:42-45`).
- **(4a) Einholen ueber die GESCHWINDIGKEIT statt ueber `limits.max`** (Radialkomponente auf 28 m/s setzen, `limits.max` nimmt nur Schlaff auf): **Eindringtiefe 0,00000 m, |v| max exakt 28,000 m/s**, endet an der Wand stehend bei x = -0,6000. **Das ist die einzige Reparatur, die haelt** — und sie kostet nichts (192 ms).

## F11 — Teilschritte gegen die echten Seillaengen

Verlust je Sekunde in %, ohne Schwerkraft, 600 Ticks:

```
n= 6 | L=5:  3.126 / 6.354 / 8.966   (v0 = 20 / 35 / 50)     ms/Fahrt ~167
     | L=8:  1.477 / 3.552 / 5.600
     | L=11: 0.838 / 2.203 / 3.746
     | L=15: 0.469 / 1.311 / 2.372
     | L=20: 0.270 / 0.781 / 1.477
n=12 | L=5:  1.819 / 4.200 / 6.424                            ms/Fahrt ~205
     | L=8:  0.796 / 2.106 / 3.603
     | L=11: 0.438 / 1.230 / 2.239
n=24 | L=5:  0.997 / 2.559 / 4.256                            ms/Fahrt ~280
     | L=8:  0.415 / 1.170 / 2.140
     | L=11: 0.224 / 0.655 / 1.252
     | L=20: 0.069 / 0.207 / 0.415
```

**Kleinster Teilschrittwert, der (a) ueber L=5..20 und v0=20..50 erfuellt: 24.** Wenn man L>=8 m garantiert: 12 reicht (3,60 %). Wenn L>=11 m: 6 reicht (3,75 %). Kosten 6→24: +64 % Rechenzeit (167 → 280 ms je 600 Ticks, 1 Koerper + 1 Seil).

**Einholstoss — die Behauptung der Gegenprobe ist bestaetigt und der Mechanismus ist jetzt exakt bekannt:**

```
 Teilschritte | |v| max  | groesster Sprung
            6 | 186.267  | 149.182
           12 | 338.146  | 316.594
           24 | 672.462  | 652.299
```

Die Skalierung ist **linear in `SubstepCount`** (×2 je Verdopplung, ×4 von 6 auf 24). Die Formel stimmt auf drei Stellen: `28 m/s × n` = 168 / 336 / 672.

**Das ist der eigentliche Fund dieser Runde**, und er ist am Quelltext erklaerbar: Der Seilzwang schreibt `delta_position` direkt (`xpbd/positional_constraint.rs:24,37`), und `project_linear_velocity` (`xpbd/plugin.rs:105-110`) rechnet diese Positionskorrektur je Teilschritt in **Geschwindigkeit** um. Eine Verkuerzung um Δ in einem Teilschritt erzeugt also `v = Δ / h_teilschritt`. Bei Verkuerzung **je Tick** ist Δ = rate/60 und h = 1/(60n) ⇒ **v = rate × n**. Bei Verkuerzung **je Teilschritt** ist Δ = rate/(60n) ⇒ **v = rate**. Genau das messen (3) und der Einholstoss.

⇒ **Direkter Zielkonflikt**: (a) verlangt n=24, und n=24 vervierfacht den Einholstoss auf 672 m/s = 9× `tempo_max_m_s`. Der Konflikt verschwindet erst mit (3) oder (4a).

## F12 — Reibung beim Schwingen

Vorab am Quelltext: avians Vorgabe ist `Friction { 0.5, 0.5, Average }` (`physics_material.rs:152-160`), und `Average(0.5, 0.5) = 0.5` — **nicht 0,65**. Die 0,65 der Gegenprobe entstehen nur mit `Max`/`GeometricMean` (`ebenda:13-24`). Gemessen mit auf beiden Koerpern gleich gesetzten Werten.

**(a) kontrolliert**, 35 m/s laengs an der Wand, `Andruck` = Normalbeschleunigung:

```
 Reibung  Andruck | v(0,25s)  v(0,5s)   v(1s)  | steht ab Tick
    0.65     20.0 |  31.967   28.717   22.217  |  gleitet
    0.30     20.0 |  33.601   32.100   29.100  |  gleitet
    0.10     20.0 |  34.533   34.034   33.034  |  gleitet
    0.65     60.0 |  25.900   16.151    0.000  |  55
    0.30     60.0 |  30.801   26.301   17.302  |  gleitet
    0.65    175.0 |   8.459    0.000   -0.000  |  20   (= v^2/L bei L = 7 m Gasse)
    0.30    175.0 |  22.751    9.627   -0.000  |  41
    0.10    175.0 |  30.917   26.542   17.794  |  gleitet
    0.00      * * |  35.000   35.000   35.000  |  gleitet  (Nulllinie sauber)
```

Kein Stick-Slip: der Abfall ist linear (`Δv = μ·a` je Sekunde), er gleitet bis er steht und bleibt dann stehen.

**(b) echte Gasse** (7 m, Haus 11,5 m, Anker auf der Dachkante bzw. `Tiefe` m dahinter), Energieverlust je Sekunde, mit Kontrolle **ohne Wand**:

```
 Reibung Tiefe |  E-Verlust/s | Abstand |
       -   0.0 |     3.88%    |    -    | <- Kontrolle, keine Wand (nur Loeserdaempfung)
    0.65   0.0 |    10.77%    | -0.3124 |   davon Reibung: 6.87 %
       -   2.0 |     3.77%    |    -    | <- Kontrolle
    0.65   2.0 |    39.74%    | -1.9705 |   davon Reibung: 35.79 %
       -   5.0 |     3.36%    |    -    | <- Kontrolle
    0.65   5.0 |    62.35%    | -0.0075 |   davon Reibung: 58.39 %
    0.30   5.0 |    40.97%    | -0.8084 |   davon Reibung: 37.00 %
    0.10   5.0 |    19.10%    | -2.2786 |   davon Reibung: 15.13 %
```

**Die 75 %/s der Gegenprobe sind erreichbar, aber nicht der Normalfall.** Am sauberen Dachkanten-Haken kostet Reibung 0,65 nur **6,9 % Energie je Sekunde** (≈3,5 % Tempo). Erst wenn der Anker mehrere Meter **hinter** der Wandflaeche liegt (die andere Dachseite), zieht das Seil so hart in die Wand, dass 36-58 %/s daraus werden. **F-014 wird von Reibung nicht geloescht** — aber die Ankerwahl entscheidet ueber den Faktor 8.

**Nebenfund in derselben Zeile, ungefragt:** die Spalte `Abstand` ist **negativ**. Der Spieler steckt beim normalen Schwingen **0,31 / 1,97 / 3,57 m in der Hauswand** — dieselbe F9-Niederlage, **ohne jedes Einholen**, in einem voellig gewoehnlichen Bogen. Der Seilzwang schiebt ihn so weit hinein, wie der Anker hinter der Flaeche liegt. Die Reibungszahlen bei Tiefe 2 und 5 sind deshalb *im Wandinneren* gemessen.

---

## Beleg

- Datei: `/home/offlinebot/Documents/defeated-by-titan/examples/probe_avian.rs` (1929 Zeilen, F9-F12 angehaengt ab Zeile ~1036)
- Lauf: `cargo run --release --example probe_avian`, Exit 0, ~90 s, Ausgabe in `/tmp/claude-1000/-home-offlinebot-Documents-defeated-by-titan/3285eda4-fff6-4374-861e-3da38ba9f775/scratchpad/lauf4.txt`
- Quelltext (alle Pfade unter `~/.cargo/registry/src/index.crates.io-*/avian3d-0.7.0/src/`):
  - Teilschrittkette: `dynamics/solver/schedule.rs:50-67`, `dynamics/solver/xpbd/plugin.rs:30-41`
  - Positionsschreiben: `dynamics/solver/xpbd/positional_constraint.rs:24,37`
  - Kontaktdeckel: `dynamics/solver/contact/normal_part.rs:147`, Vorgabe 4,0 in `dynamics/solver/plugin.rs:250,296`, skaliert in `:542`
  - `limits` wird je Teilschritt neu gelesen: `dynamics/solver/xpbd/plugin.rs:160-203` + `xpbd/joints/distance.rs:80,105-112`
  - Korrektur → Geschwindigkeit: `dynamics/solver/xpbd/plugin.rs:105-110`; `delta_position` wird nur einmal je Physikschritt genullt: `dynamics/solver/solver_body/plugin.rs:211`
  - Kein Motor am `DistanceJoint`: `dynamics/joints/distance.rs:26-39`; Motoren: `joints/prismatic.rs:55,:319`, `joints/revolute.rs:74,:349`, `joints/motor.rs:228-238`
  - Reibungsvorgabe: `dynamics/rigid_body/physics_material.rs:13-24,137-160`
  - `SubstepSchedule`/`SubstepSolverSystems` sind oeffentlich: `dynamics/solver/schedule.rs:72,134`, re-exportiert `dynamics/mod.rs:115-117`

## Stufe

🟧 fuer F9, F10, F11, F12: jede Zahl hat Kommandoausgabe, jede API-Aussage eine Datei:Zeile, und jede Behauptung eine Gegenprobe im selben Lauf (F9 (c) gegen F9 (b), F10 „Vergleich je Tick" gegen (3), F12 (b) „keine Wand" und „Reibung 0,00" gegen den Rest). Kein Bild — das ist eine Konsolenprobe.

## Offen

- **(c) fehlt vollstaendig.** Braucht Stadt aus `maps.ron` + vier Spieler + `PhysicsDiagnosticsPlugin`.
- **(4a) ist nur gegen eine gerade Wand gemessen**, nicht gegen Innenecken, Daecher, mehrere Kontakte gleichzeitig, und nicht mit zwei Seilen (F2-Fall). Bevor darauf gebaut wird, muss das nachgemessen werden.
- **Warum `max_overlap_solve_speed` gar nichts tut, ist nicht geklaert.** Ich habe die Nichtwirkung viermal gemessen, aber die Ursache nicht belegt (Verdacht: die Kontaktzwaenge werden einmal je Physikschritt vorbereitet, `SolverSystems::PrepareContactConstraints`, und der Sprung von 5,6 m passiert innerhalb desselben Schritts). Das ist eine **Hypothese, kein Befund**.
- `pendel_dehnung` bei L=5 zeigt −92,7 % Energiedrift ueber 10 s auch bei `compliance = 0`; das ist ein Pendel am Ueberschlagspunkt (v0² = 4gL) und nicht untersucht.

## Funde (Fremdgebiet — nicht angefasst, gehoert nach `docs/FUNDE.md`)

1. **`examples/probe_avian.rs:57` haelt `REICHWEITE = 112.0` fest, `game.ron` sagt seit 2026-08-09 `hakenreichweite_m: 90.0`.** F4 misst also Strahlen ueber eine Reichweite, die es nicht mehr gibt. Nicht meine Datei zu entscheiden — der Konstantenkopf sagt selbst „wer `game.ron` aendert, muss diese Datei nachziehen".
2. **Der Umbauplan muss den Schiedsrichter zwischen Seil und Geometrie behalten.** Nicht weil der Kontakt schwach waere (F9 (c): 0,00000 m bei 50 m/s), sondern weil `DistanceJoint` ihn strukturell ueberschreibt. Reparatur (4a) verlegt das Einholen von der Position in die Geschwindigkeit und stellt den Kontakt damit wieder als letzte Instanz her — mit der RON-Zahl 28 m/s als exaktem Ergebnis.
3. **`seil_durchlaeufe: 2` in `game.ron` beschreibt einen Gauss-Seidel-Loeser, den es in avian nicht gibt.** avian iteriert ueber `SubstepCount`; die gemessene Groesse ist 24, nicht 2. Die Zahl in `game.ron` ist damit ohne Verbraucher.
4. **Beim Schwingen steckt der Spieler bis zu 3,6 m in der Hauswand** (F12 b), ohne dass irgendetwas eingeholt wird. Das trifft F-014 und F-023 direkt und ist unabhaengig von F-005.