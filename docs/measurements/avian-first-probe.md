# avian-first-probe — measurement report, 2026-08-09

Updated: 2026-08-09 · Stage: 🟧 (command output, reproducible)

**Aufgabe** — avian3d 0.7.0 in `Cargo.toml` aufnehmen, Featureauswahl am Quelltext begruenden, Bauzeiten messen, und in EINEM headless-Beispiel die fuenf Fragen mit Zahlen beantworten. Einziger Schreiber: `Cargo.toml`, `Cargo.lock`, `examples/`.

**Getan**

- `/home/offlinebot/Documents/defeated-by-titan/Cargo.toml` — avian3d 0.7.0 mit `default-features = false`, Features `3d, f32, parry-f32, parallel, xpbd_joints, debug-plugin`. Neues Projekt-Feature `determinismus = ["avian3d/enhanced-determinism"]`. Begruendung als Kommentar in der Datei, jede Zeile mit Quelltextbeleg.
- `/home/offlinebot/Documents/defeated-by-titan/Cargo.lock` — aufgeloest.
- `/home/offlinebot/Documents/defeated-by-titan/examples/probe_avian.rs` (1063 Zeilen, neu) — neun Proben F0–F8. Sonst nichts angefasst; `git status` zeigt ausser diesen dreien nur den vorgefundenen Zwischenstand.

---

## 1. Featureauswahl und Bauzeiten (gemessen, Maschine B `offlinebot`, 16 Threads)

**Weggelassen aus `default`** (avian3d-0.7.0/Cargo.toml:50–64):
- `collider-from-mesh` → zieht `bevy/bevy_mesh` + `bevy/bevy_mikktspace`. Die Stadt kommt als Quader aus `maps.ron`; Mesh-Collider sind erst mit echten Titanmodellen ein Thema.
- `bevy_scene` → zieht `bevy/bevy_world_serialization`, gehoert zu collider-from-mesh.
- `bevy_picking` → avians Picking-Backend. Das Fadenkreuz fragt ueber `SpatialQuery`.

**Drin geblieben, obwohl Bauzeit:**
- `xpbd_joints` — **ohne das gibt es keinen Joint-Loeser.** `dynamics/joints/mod.rs:242`: *„For a built-in joint solver, enable the `xpbd_joints` feature, and use the `XpbdSolverPlugin`."* `SolverPlugins` fuegt `XpbdSolverPlugin` nur unter diesem cfg hinzu (`dynamics/solver/mod.rs:78-80`). **Das Seil IST ein Joint** — ohne dieses Feature ist die ganze Entscheidung hinfaellig.
- `debug-plugin` — braucht `bevy/bevy_gizmos` + `bevy/bevy_render`, beides ist ueber `3d_bevy_render` schon an, also **null zusaetzliche Systembibliotheken**. `PhysicsDebugPlugin` steckt *nicht* in `PhysicsPlugins` (Gruppenliste `lib.rs:757-789`), muss also einzeln addiert werden — kostet zur Laufzeit nichts, solange niemand ihn addiert. Stufe 🟧 verlangt ein Bild; ohne ihn gibt es kein Bild von einem Collider.
- `parry-f32` zieht `default-collider` — **darin haengen `Collider`, `SpatialQuery`, `ShapeCaster`** (`lib.rs:770-775`). Nicht optional.

| Messung | Kommando | Zahl |
|---|---|---|
| Pakete im Bauzeitgraph vorher | `cargo tree --edges normal` | **318** |
| Pakete nachher | dito | **348** → **+30** |
| Neue Crates | `git diff Cargo.lock` ∩ avian-Subtree | `avian3d avian_derive parry3d obvhs bevy_heavy bevy_transform_interpolation glam_matrix_extras glamx simba spade rstar robust rdst safe_arch wide num-complex ena heapless itertools(2×) paste partition arbitrary-chunks rayon rayon-core crossbeam-deque crossbeam-epoch proc-macro-error3 proc-macro-error-attr3` |
| `time cargo build` (dev) | — | **real 6m10,824s** · user 58m41s · **105 Crates neu uebersetzt** |
| `time cargo build --release --example probe_avian` | — | **real 4m02,416s** · user 46m19s · 331 Crates |
| `cargo check --examples --lib --tests` | — | **1m13,869s**, 0 Fehler, 0 Warnungen |
| Erneuter Beispiel-Build nach Quelltextaenderung | — | **2,63 s** |

⚠️ **Der teuerste Einzelposten ist nicht avian, sondern eine Featureunifikation.** avian haengt an `bevy_math` mit Feature `approx` (avian3d-0.7.0/Cargo.toml, `[dependencies.bevy_math] features = ["approx"]`). Dadurch aendert sich `bevy_math`s Featuremenge, und **ganz bevy wird neu uebersetzt** — im Baulog sind das 105 von 105 Crates, `bevy_render`, `bevy_pbr`, `bevy_ui_render` inklusive. Das passiert **einmal**; inkrementell danach 2,6 s.

**`enhanced-determinism` — gemessen, nicht geschaetzt.** Es schaltet ein: `dep:libm`, `bevy_math/libm` (= `glam/libm`, bevy_math-0.19.0/Cargo.toml:61-64), `bevy_heavy/libm` (bevy_heavy-0.5.0/Cargo.toml:61-63), `parry3d/enhanced-determinism`. Im avian-3D-Code selbst wird `libm` **nirgends** benutzt — die einzigen `#[cfg(feature = "enhanced-determinism")]`-Stellen sind `physics_transform/transform.rs:242,291`, und die liegen im **2D**-`Rotation`. Gemessen:

| | Bauzeit | F1/F6/F8-Zahlen |
|---|---|---|
| ohne | 4m02s | Referenz |
| `--features determinismus` | **3m19s** (Neubau ab `bevy_math`, 39 Crates) | **bitweise identisch, alle Zeilen** |

Also: auf x86-64 kostet es einen Vollneubau von `bevy_math` abwaerts und **aendert kein einziges Ergebnis**. Sein Wert ist Architekturgleichheit, und die ist auf einer Maschine **nicht messbar** → als Schalter drin, nicht als Vorgabe. Dazu ehrlich: **avian 0.7.0 testet Cross-Determinismus nur fuer 2D** (`src/tests/mod.rs:14`, `#[cfg(all(feature = "2d", feature = "enhanced-determinism"))] mod determinism_2d;`). Fuer 3D gibt es keinen solchen Test im Crate.

---

## 2. Die Probe — `cargo run --release --example probe_avian`

Volles Protokoll: `/tmp/claude-1000/-home-offlinebot-Documents-defeated-by-titan/3285eda4-fff6-4374-861e-3da38ba9f775/scratchpad/probe-final.log`

Aufbau: `MinimalPlugins` + `PhysicsPlugins::default()`, `Time::<Fixed>::from_hz(60)`, `TimeUpdateStrategy::FixedTimesteps(1)` — das schiebt die Realzeit je `App::update()` um genau einen Zeitschritt (bevy_time-0.19.0/src/lib.rs:181-183), also **ein Physikschritt je `update()`**, wanduhrunabhaengig. avians eigene Tests fahren genauso (`avian3d/src/tests/mod.rs:17-36`). **F0 prueft zusaetzlich den echten Projektweg** aus `src/lib.rs:146-190`.

### F0 — laeuft avian im Kopflos-Modus dieses Projekts?

```
  120 Ticks freier Fall aus 100 m, g = -20 m/s^2, t = 2.0000 s
  y analytisch     : 60.0000 m
  y gemessen       : 60.6089 m
  v_y gemessen     : -39.6668 m/s  (analytisch -40.0000)
```
**Ja** — `DefaultPlugins` mit `backends: None`, ohne `WinitPlugin`, mit `ScheduleRunnerPlugin`. Drei bekannte bevy-WARNs (kein RenderApp), kein Absturz.

Die 0,61 m sind **kein Fehler, sondern zwei erklaerte Effekte**: es wirken 119 statt 120 Ticks Schwerkraft (**ein neu gespawnter Rigid Body verliert seinen ersten Tick** — er ist erst nach einem `Prepare`-Durchlauf ein `SolverBody`), und avian integriert semi-implizit ueber 6 Teilschritte. Nachgerechnet: `100 − 20·(1/360)²·(714·715/2) = 60,6089 m`. **Exakt der Messwert.** F7 bestaetigt dieselbe Ein-Tick-Latenz unabhaengig (90,3033 gemessen, 90,3032 gerechnet).

### F1 — SEIL. Zieht es, ohne zu druecken?

`DistanceJoint` mit `limits = [0, L]`. `DistanceLimit::compute_correction` korrigiert **nur** bei `distance > max` (`dynamics/joints/mod.rs:329-343`) — das ist per Konstruktion eine Ungleichung, also ein Seil und keine Stange.

**(a) ohne Schwerkraft, reine Kreisbahn, 600 Ticks = 10 s**

| L [m] | v0 | v(1s) | v(2s) | v(5s) | v(10s) | d_min | d_max | Verlust/s |
|---|---|---|---|---|---|---|---|---|
| 20,0 | 30,0 | 29,8177 | 29,6353 | 29,1074 | 28,2871 | 20,00000 | 20,00000 | **0,5862 %** |
| 3,0 | 75,0 | 35,7139 | 26,7125 | 17,5318 | 12,5590 | 3,00000 | 3,00000 | **16,3647 %** |
| 10,0 | 50,0 | 46,9046 | 44,2766 | 38,4398 | 32,3668 | 10,00000 | 10,00000 | **4,2557 %** |
| 5,0 | 60,0 | 44,9033 | 37,3144 | 26,8712 | 20,0157 | 5,00000 | 5,00000 | **10,3971 %** |

**(b) mit Schwerkraft −20 m/s², 10 s**

| L [m] | v0 | d_min | d_max | v_max | v(10s) | E_drift | E_span |
|---|---|---|---|---|---|---|---|
| 20,0 | 30,0 | **19,61974** | 20,00000 | 30,0000 | 10,3731 | −6,82 % | 6,82 % |
| 3,0 | 75,0 | **2,88264** | 3,00000 | 75,0000 | 16,4082 | **−95,20 %** | 95,20 % |
| 10,0 | 50,0 | 10,00000 | 10,00000 | 50,0000 | 31,1442 | −44,78 % | 44,78 % |
| 5,0 | 60,0 | 5,00000 | 5,00000 | 60,0000 | 16,9936 | −83,73 % | 83,73 % |

**Abstandsabweichung: `d_max − L = 0,00000 m` in allen acht Faellen** (Ausgabe auf 5 Nachkommastellen). `d_min < L` bei L=20 und L=3 belegt direkt: **das Seil drueckt nicht.**

⚠️ **Der Tempoverlust ist NICHT weg — er ist durch 6 geteilt.** Der Eigenbauloeser verlor bei L=3, v=75 **99,2 %/s**; avian verliert in derselben ersten Sekunde 75 → 35,71 = **52,4 %**. Das ist derselbe Mechanismus (Positionsprojektion), nur mit 6 Teilschritten statt einem. Die Theorie `1−(1+θ²)^(−n)` mit `θ = v·dt_sub/L` sagt fuer die erste Sekunde 57,9 % voraus — Messung 52,4 %, und die Abweichung geht in die richtige Richtung (ω faellt, also faellt der Verlust). **F6 beweist die Ursache**, siehe unten.

### F2 — ZWEI SEILE. Faellt der Spieler durch?

Der strukturelle Fehler des Eigenbauentwurfs (beide Seilkraefte in EINEN `Vec3` addiert → Summe 0) **tritt nicht auf.** avian loest jeden Joint als eigenen Zwang im Constraint-Graph.

**(a) Normalfall** — Anker (−15,20,0) und (15,20,0), Spieler (0,0,0), L₁=L₂=25 m:

| Tick | d1 [m] | d2 [m] | y [m] | Absacken |
|---|---|---|---|---|
| 1 | 25,00000 | 25,00000 | 0,00000 | 0,00000 |
| 30 | 24,99997 | 25,00000 | 0,00002 | −0,00002 |
| 60 | 24,99998 | 25,00000 | 0,00002 | −0,00002 |

**Absacken nach 1 s: −0,00002 m** (freier Fall waere 10,0 m). Seildehnung L1 −0,000025 m, L2 −0,000002 m.

**(b) Entartet** — Anker 48 m auseinander, L₁=L₂=25 m, Spieler auf der Verbindungslinie bei (0,13,0), also exakt straff und beide Zwangsnormalen fast kollinear:

| Tick | d1 [m] | d2 [m] | y [m] | Absacken |
|---|---|---|---|---|
| 1 | 25,00000 | 25,00000 | 13,00000 | 0,00000 |
| 10 | 25,00024 | 25,00000 | 12,99959 | 0,00041 |
| 60 | 25,00023 | 25,00000 | 12,99959 | **0,00041** |

**0,41 mm Absacken statt der gerechneten 9,6 m des Eigenbauentwurfs. Seildehnung 0,23 mm** — Faktor 43 innerhalb des ±1-cm-Kriteriums. Kein 30-Hz-Zittern: die Werte stehen ab Tick 10 still.

### F3 — REEL-IN. Steigt das Tempo?

Seil 30 m → 5 m in 60 Ticks (25 m/s; `vector.seilzug_m_s` ist 28), ohne Schwerkraft.

| v0 [m/s] | v nach 1 s | Theorie (L·v const) | Faktor | d_ende | \|v\| max |
|---|---|---|---|---|---|
| 12,50 | **156,3673** | 75,0000 | **12,51** | 4,40681 | 156,37 |
| 30,00 | **111,2253** | 180,0000 | **3,71** | 5,00000 | 176,98 |
| 5,00 | **52,0056** | 30,0000 | **10,40** | 2,77466 | 150,49 |

**Ja, das Tempo steigt — aber nicht drehimpulserhaltend.** Der Faktor ist 3,7 bis 12,5 statt 6,0. Das Einholen **pumpt Energie hinein**: XPBD behandelt die verkuerzte Sollaenge als Positionsfehler und schreibt die Korrektur als Geschwindigkeit zurueck. Fuer das ODM-Gefuehl ist das gut (F-005 ist mechanisch **nicht** wertlos), fuer Fling-Exploits ist es eine offene Tuer — genau wofuer die Klemme aus F8 da ist. `d_ende < L` ist erlaubt und kein Fehler: das Seil zieht nur.

### F4 — RAYCAST. 112 m gegen 4000 statische Quader (Gitter 20×10×20, Raster 6 m), **Release**

```
  1000 Strahlen, 112 m, Filter ALLE   :  304,37 us gesamt,  0,304 us/Strahl, 114 Treffer
  1000 Strahlen, 112 m, Maske LEER    :  310,17 us gesamt,  0,310 us/Strahl, 1000 ohne Treffer
  Einzelzeiten je Strahl              :  min 0,070 us, median 0,211 us, p99 1,463 us, max 3,787 us
```
(zweiter Lauf: 253,14 / 266,12 us, max 10,169 us — Streuung ist Cache, nicht Algorithmus)

**0,21 us Median je 112-m-Strahl.** Ein Tick hat bei 60 Hz 16 666 us. Der geplante Eigenbau-Gitterindex (`welt.zelle_m`, `halbe_ausdehnung_m`, `grosskoerper_zellen` in `game.ron`, T-036a) hat damit **keinen Gegner mehr**.

**Filter und Maske — die Frage nach „erst treffen, dann pruefen ob hakbar":**
```
  Filter ALLE      -> Some((9.85, Vec3(-1.0, 0.0, 0.0)))     <- Wand blockiert korrekt
  Filter NUR DACH  -> Some((19.85, Vec3(-1.0, 0.0, 0.0)))    <- durch die Wand hindurch aufs Dach
  cast_ray_predicate(alle, |_| true) -> Some(9.85)
```
**Beides geht, und man muss sich entscheiden.** `SpatialQueryFilter::from_mask(LayerMask)` macht die ungetaggte Wand **unsichtbar** — der Strahl faehrt hindurch. Fuer „erst treffen, dann pruefen" nimmt man `mask = ALL` und prueft `RayHitData.entity` danach; `RayHitData` liefert `entity`, `distance`, `normal` (`spatial_query/ray_caster.rs:395-404`). Ausserdem gibt es `cast_ray_predicate(..., &dyn Fn(Entity) -> bool)` (`system_param.rs:176`), das je getroffener Entity gefragt wird und den Strahl weiterlaufen laesst — das ist die saubere Naht fuer F-002/F-003.

### F5 — KAPSEL UND SCHRITTWEITE. Tunnelt der Spieler durch 0,3 m?

Kapsel r=0,35 m, Gesamthoehe 1,8 m (`game.ron`). Wand 0,3 m dick bei x=10 (**duenner** als `welt.wand_min_m` = 0,5). 60 Ticks, ohne Schwerkraft.

| Sicherung | v [m/s] | m/Tick | x_ende [m] | durch? |
|---|---|---|---|---|
| **Vorgabe** (Spekulation unbegrenzt) | 75,0 | 1,2500 | **9,5000** | nein |
| **Vorgabe** | 150,0 | 2,5000 | **9,5000** | nein |
| **Vorgabe** | 400,0 | 6,6667 | **9,5000** | nein |
| `SpeculativeMargin::ZERO` | 75,0 | 1,2500 | 73,7500 | **JA, TUNNEL** |
| `SpeculativeMargin::ZERO` | 150,0 | 2,5000 | 147,5000 | **JA, TUNNEL** |
| `SpeculativeMargin::ZERO` | 400,0 | 6,6667 | 393,3332 | **JA, TUNNEL** |
| SpecMargin 0 + `SweptCcd::LINEAR` | 75/150/400 | — | **9,5000** | nein |
| SpecMargin 0 + `SweptCcd::NON_LINEAR` | 75/150/400 | — | **9,5000** | nein |

**Kein Tunneln, ohne dass man irgendetwas dazutut.** `NarrowPhaseConfig::default_speculative_margin = Scalar::MAX` (`collision/narrow_phase/mod.rs:222,252`) — die Spekulativkontakte sind per Vorgabe unbegrenzt. `SweptCcd` (`dynamics/ccd/mod.rs:389`, Modi `Linear`/`NonLinear`) und `SpeculativeMargin` (`:308`) existieren als Reserve; die Kontrollzeile mit `ZERO` beweist, dass die Vorgabe der wirksame Teil ist. `x_ende = 9,5000 = 10 − 0,15 − 0,35` — exakt Wandflaeche minus Kapselradius.

### F6 — TEILSCHRITTE. Ist der Verlust aus F1 ein RON-Schalter? (Zusatzprobe)

| L [m] | v0 | Substeps | v(1s) | v(10s) | Verlust/s | Zeit/600 Ticks |
|---|---|---|---|---|---|---|
| 3,0 | 75,0 | **6** (Vorgabe) | 35,7139 | 12,5590 | 16,3647 % | 133,1 ms |
| 3,0 | 75,0 | 12 | 45,5932 | 17,5169 | 13,5352 % | 166,3 ms |
| 3,0 | 75,0 | 24 | 55,0929 | 24,1239 | 10,7232 % | 242,4 ms |
| 3,0 | 75,0 | **48** | **62,7912** | **32,4767** | **8,0290 %** | 375,9 ms |
| 5,0 | 60,0 | 6 | 44,9033 | 20,0157 | 10,3971 % | 134,9 ms |
| 5,0 | 60,0 | 48 | 57,2515 | 42,4465 | **3,4018 %** | 368,8 ms |
| 20,0 | 30,0 | 6 | 29,8177 | 28,2871 | 0,5862 % | 133,7 ms |
| 20,0 | 30,0 | 48 | 29,9760 | 29,7660 | **0,0783 %** | 376,6 ms |

**Ja — der Verlust ist kaufbar, und der Preis steht in der Tabelle.** Der Verlust faellt monoton mit `SubstepCount`; bei L=20 um Faktor 7,5 fuer 8× Substeps (also ≈1/n). Das identifiziert ihn als **Diskretisierungsfehler der Positionsprojektion, nicht als Daempfung** — `JointDamping` ist per Vorgabe abwesend (`dynamics/joints/mod.rs:611-616`, muss manuell addiert werden). `SubstepCount` ist eine Resource mit Vorgabe 6 (`dynamics/solver/schedule.rs:185-191`), gehoert also nach `game.ron`. Die Zeiten sind **Schedule-Overhead bei EINEM Koerper** — sie sagen nichts ueber die Kosten in einer vollen Szene.

### F7 — EINPASSUNG in `SchrittSet` (Zusatzprobe, mit dem echten `defeated_by_titan::shared::SchrittSet`)

```
  Reihenfolge der ersten Ticks : ["Absicht", "Nachlauf", "Absicht", "Nachlauf", ...]
  y in `Absicht`  (vor Physik) : 90.628723 m
  y in `Nachlauf` (nach Physik): 90.303261 m
  Differenz in EINEM Tick      : -0.325462 m
```
`PhysicsPlugins::new(FixedUpdate)` + alle fuenf `PhysicsSystems`-Stufen gemeinsam `.in_set(SchrittSet::Vollzug)`. **Kein Zyklus, kein Absturz, die Physik liegt zwischen `Absicht` und `Nachlauf`.**

### F8 — KLEMME, BOOST, WIEDERHOLBARKEIT (Zusatzprobe)

Boost 34 m/s² (`vector.boost_m_s2`) ueber 10 s, ohne Schwerkraft, ueber `Query<Forces>::apply_linear_acceleration`:

| Aufbau | v nach 2 s | v nach 5 s | v nach 10 s |
|---|---|---|---|
| ohne Klemme | 67,4332 | 169,4364 | **339,4498** |
| `MaxLinearSpeed(75)` | 67,4332 | **75,0000** | **75,0000** |

**F-012 ist eingebaut und haelt auf die Nachkommastelle.** F-007 laeuft ueber die Kraft-API.

```
     Fahrt 1 (Rohbits) : 419ab232 419c8523 409062c1 | be409a08 c10721f3 bd33be1e
     Fahrt 2 (Rohbits) : 419ab232 419c8523 409062c1 | be409a08 c10721f3 bd33be1e
     GLEICH: JA, bitweise
```
600 Ticks, Seil + Boden + Kontakte, zwei App-Instanzen im selben Prozess: **bitweise gleich.**

---

## 3. Die Quelltextfragen

**Zeitplan.** avian legt `PhysicsSchedule` per Vorgabe in `FixedPostUpdate` (`lib.rs:751-755`, `schedule/mod.rs:52-56`). `PhysicsPlugins::new(schedule)` (`lib.rs:690-695`) waehlt frei; `PhysicsSchedulePlugin::build` konfiguriert dort `(First, Prepare, StepSimulation, Writeback, Last).chain().before(TransformSystems::Propagate)` (`schedule/mod.rs:74-85`). **In `FixedUpdate` zwingen: ja** — `PhysicsPlugins::new(FixedUpdate)` und dann **alle fuenf gemeinsam** in `SchrittSet::Vollzug` (F7 belegt es). Nur eine Stufe hineinzustecken baut einen Zyklus, weil die fuenf bereits verkettet sind.

⚠️ **Der raeumliche Index wird INNERHALB der Physik aktuell, nicht davor.** `ColliderTreeSystems::UpdateAabbs` liegt in `PhysicsStepSystems::BroadPhase`, `EndOptimize` in `SolverSystems::Finalize` (`collider_tree/mod.rs:78-93`). Ein Strahl in `SchrittSet::Welt` sieht den Baum vom **vorigen** Tick. Das widerspricht `ablauf.rs:45-50` („Der raeumliche Index wird aktuell, **bevor** ihn jemand fragt") — entweder `Welt` faehrt nach `Vollzug`, oder die Regel wird zu „ein Tick alt, und das ist gewollt" umformuliert. **Das ist eine Architekturentscheidung, keine Codefrage.**

**Welche Components avian selbst schreibt** (die Autoritaetsfrage):

| Component | Schreiber | Beleg |
|---|---|---|
| `Position`, `Rotation` | **avian** (`writeback_solver_bodies`) | `dynamics/solver/solver_body/plugin.rs:276-289` |
| `LinearVelocity`, `AngularVelocity` | **avian** (dieselbe Stelle) | `solver_body/plugin.rs:286-287` |
| `Transform.translation/.rotation` | **avian** (`position_to_transform`, in `PhysicsSystems::Writeback`) | `physics_transform/mod.rs:116-124, 318-352` |
| `GlobalTransform` | **avian** propagiert selbst vor der Physik | `physics_transform/mod.rs:94-105` |
| `VelocityIntegrationData` | avian, geleert je Schritt | `integrator/mod.rs:316-327` |

Die Rueckrichtung: `transform_to_position` (`physics_transform/mod.rs:187-238`) liest `GlobalTransform` und schreibt `Position` — **aber nur, wenn `Position` seit dem letzten Physiktick nicht geaendert wurde**. Spielcode darf also entweder `Transform` **oder** `Position` schreiben; bei Gleichstand gewinnt `Position`. Beide Richtungen sind ueber `PhysicsTransformConfig` (`physics_transform/mod.rs:70+`) abschaltbar.

⚠️ **Damit stimmt die Autoritaetstabelle in `docs/architektur.md:83` nicht mehr:** „`Transform` des Spielers | `player` (Boden, Schwerkraft) und `vector` (Seilkraefte)". Neu ist der Schreiber **avian**; `player` und `vector` schreiben `Intent` → Joints/Kraefte, nie `Transform`. Und die Uebersetzungszeile „`RopeConstraint` → eigene Seilrechnung in `vector/`, **keine Engine-Constraint**" ist ueberholt.

**Kraft und Impuls (F-007).** `ExternalForce`/`ExternalImpulse` **existieren in 0.7.0 nicht** (grep ueber den ganzen Quelltext: null Treffer). Stattdessen:
- Dauerhaft: `ConstantForce`, `ConstantTorque`, `ConstantLinearAcceleration`, `ConstantLocalForce` … als **Components**, die ueber Schritte hinweg bleiben (`forces/mod.rs:23-45`, angewandt in `forces/plugin.rs:41-57`).
- Einmalig: die QueryData **`Forces`** — `Query<Forces>`, dann `apply_force` / `apply_linear_impulse` / `apply_linear_acceleration` (`forces/query_data.rs:300, 388, 482`).
- **Akkumulation:** Kraft und Beschleunigung landen in `VelocityIntegrationData.linear_increment` und werden ueber **alle Teilschritte** verteilt; Impuls schreibt `LinearVelocity` **sofort** (`query_data.rs:388-396`).
- **Leerung:** automatisch. `clear_velocity_increments` in `IntegrationSystems::ClearVelocityIncrements` nach `Velocity` (`integrator/mod.rs:58-60, 316-327`); `ForceSystems::Clear` in `SolverSystems::PostSubstep` (`forces/plugin.rs:35-36`). **Nichts von Hand zuruecksetzen.**

**Determinismus.** In-Prozess bitweise wiederholbar (F8b, mit Kontakten und Joints). Der `PhysicsSchedule` laeuft mit `SingleThreadedExecutor` (`schedule/mod.rs:90`); Parallelitaet gibt es nur in `par_iter_mut` ueber disjunkte Entities und in `par_for_each` ueber **Graphfarben, in denen jeder Koerper hoechstens einmal vorkommt** (`dynamics/solver/constraint_graph.rs:1-6`) — beides schreibreihenfolgeunabhaengig. Die Baumoptimierung laeuft asynchron (`collider_tree/optimization.rs:198`), wird aber im **selben** Schritt per `block_on_optimize_trees` eingesammelt (`:27`). Zu `enhanced-determinism` siehe oben: **auf dieser Maschine bitweise wirkungslos**, Cross-Architektur **unbekannt und in 0.7.0 nur fuer 2D getestet**.

**Geschwindigkeitsklemme (F-012).** **Eingebaut.** `MaxLinearSpeed(Scalar)` / `MaxAngularSpeed` (`dynamics/rigid_body/mod.rs:441, 471`), angewandt von `clamp_velocities` im `SubstepSchedule`, verkettet nach `integrate_velocities` (`integrator/mod.rs:81-83, 467-489`). Sie klemmt `SolverBody.linear_velocity` **je Teilschritt**. F8a: 339,45 → 75,0000 m/s.

**Stufe** — 🟨 fuer alles.
Alle Zahlen sind gemessen und reproduzierbar, aber: **kein Bild** (nichts wurde im Spiel gesehen, das Beispiel rendert nicht), **kein Test in `tests/`, der rot wird** (ich besitze `tests/` nicht), **und kein zweiter Kopf hat versucht, das zu widerlegen.** Nach `docs/STATUS.md:12` fehlen damit zwei von drei Belegen. Unsicherheit setzt die Stufe herunter.

**Offen**
1. **Der Tempoverlust bei kurzem Seil ist NICHT geloest, nur gemildert.** Bei L=3, v=75 und Vorgabe-Substeps sind 52 % in der ersten Sekunde weg. Ob `SubstepCount = 24` (10,7 %/s) reicht, damit sich das *anfuehlt* wie die Referenz, entscheidet ein Blindtest, keine Zahl.
2. **F3 pumpt Energie hinein** (Faktor 3,7–12,5 statt 6,0). Ob das Feature oder Fehler ist, ist eine Spielwertfrage → `docs/FRAGEN.md`.
3. **Die Kosten in einer vollen Szene sind ungemessen.** F6 misst einen Koerper, F4 misst 4000 statische Quader ohne Solver. Kein Messwert fuer „Stadt + n Spieler + m Titanen".
4. **Der Ein-Tick-Spawn-Verzug** wird jeden analytischen Test um genau einen Tick verschieben. Wer das nicht weiss, sucht eine Stunde.
5. **Der Zielkonflikt Index-vor-Frage vs. avians Baum-in-der-Physik** ist eine Entscheidung, die mir nicht gehoert.
6. `target/` steht gerade auf `--features determinismus`; das naechste `cargo run --release` ohne das Feature baut ~3 min neu.

**Funde** (Fremdgebiet, nicht angefasst — gehoeren nach `docs/FUNDE.md`)
- `docs/architektur.md:83` Autoritaetstabelle: `Transform` des Spielers hat mit avian einen neuen Schreiber. Muss neu geschrieben werden.
- `docs/architektur.md`, Uebersetzungstabelle: die Zeile „`RopeConstraint` → eigene Seilrechnung in `vector/` … **keine Engine-Constraint**" ist durch die Userentscheidung ueberholt.
- `src/shared/seil.rs`, `src/shared/raum.rs`, `src/player/koerper.rs` (Zwischenstand): F1/F2 ersetzt `seil.rs`, F4 ersetzt den Gitterindex in `raum.rs`, F5 ersetzt die Teilschritt-Tunnelsicherung in `koerper.rs`.
- `assets/data/game.ron`: `spieler.schritt_max_m`, `vector.seil_durchlaeufe`, `welt.zelle_m`, `welt.halbe_ausdehnung_m`, `welt.grosskoerper_zellen` haben nach dieser Probe keinen Leser mehr. Neu gebraucht wird ein `substeps`-Wert. **Ich fasse `assets/data/*.ron` nicht an** (CLAUDE.md).
- Spieler brauchen `SleepingDisabled` oder eine angepasste `SleepThreshold`: Vorgabe ist 0,15 m/s linear nach 0,5 s (`dynamics/rigid_body/sleeping.rs:103-107, 143-151`). Ein Spieler, der still am Seil haengt, schlaeft sonst ein.
- `Friction` Vorgabe 0,5 / `Restitution` Vorgabe 0,0 (`physics_material.rs:152-160, 320-328`) — beides Spielwerte, gehoert nach `game.ron`.
- `cargo test` nach dem Eingriff: **95 gruen** (67 + 18 + 3 + 7), 0 rot. Der Zwischenstand ueberlebt avian unveraendert.