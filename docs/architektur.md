# architektur — Domaenen, Plugin-Reihenfolge, Erlaubnisliste

Stand: 2026-08-09 · Stufe: 🟨 (Struktur steht und wird von `tests/domaenen.rs` geprueft;
die meisten Domaenen sind noch leere Plugins)

## Die Regel in einem Satz

**Eine Domaene = ein Ordner = ein Plugin = standalone.** Ein Ordner unter `src/` exportiert
genau ein `pub struct XPlugin` mit `impl Plugin`. Wer kein Plugin hat, ist `shared/` oder ein
Fehler (`prompts/init.md` §5).

## Was eine Domaene benutzen darf

| erlaubt, ohne dass es hier stehen muss | nicht erlaubt |
|---|---|
| `bevy::*` | eine Funktion einer anderen Domaene aufrufen |
| `crate::shared::*` — Typen, die niemandem gehoeren | `use crate::<andere domaene>` ohne Zeile in der Erlaubnisliste |
| `crate::data::*` — die geladenen RON-Daten | |

**Kommunikation laeuft ueber Components und Messages, nicht ueber Aufrufe.** `combat` schickt
`TitanGetroffen { titan, teil, tempo }`; `titan` liest es und entscheidet, was das fuer seinen
Koerper heisst. `combat` weiss nicht, wie ein Titan gebaut ist. **Deshalb liegen alle
Message-Typen in `shared/`** — sonst braeuchte jeder Empfaenger eine Kante zum Sender, und die
Regel waere nach einer Woche leer.

### Die Erlaubnisliste

`tests/domaenen.rs` liest **genau diesen Block** und faellt um, sobald im Code eine Kante
auftaucht, die hier nicht steht. Format: `von -> nach   # Begruendung`. Eine Kante ohne
Begruendung ist keine Entscheidung, sondern ein Versehen mit Doppelpunkt.

```erlaubnis
# (noch leer — jede Kante braucht eine Begruendung und einen Grund, warum sie
#  NICHT ueber eine Message in shared/ geht)
```

**Sie ist absichtlich leer.** `prompts/init.md` §5 nennt `render` liest `world` als Beispiel
einer moeglichen Ausnahme — gebraucht wird sie bisher nicht: `world` spawnt Entities mit
Komponenten aus `shared/`, und `render` fragt diese Komponenten ab, ohne `world` zu kennen.
Wer die erste Kante eintraegt, schreibt dazu, warum eine Message nicht reicht.

## Die Reihenfolge in `main.rs` ist die Abhaengigkeitsreihenfolge

```
data → save → net → world → render → player → vector → blades → titan →
combat → mission → progress → squad → hud → sound → menu → debug
```

`data` laeuft vor allem anderen: es laedt die RON und legt `GameData` als Resource ab. Ein
fehlender Wert kracht dort **beim Start**, laut und mit Dateiname — nicht still mit einer Null
mitten im Spiel (§4).

| Ordner | Plugin | wofuer |
|---|---|---|
| `shared/` | *(keins)* | Typen, die niemandem gehoeren: `Intent`, `PlayerId`, `TitanId`, Messages, Meter, Achsen-Helfer |
| `data/` | `DataPlugin` | RON laden → `GameData` + Handles. Vor allem anderen. |
| `save/` | `SavePlugin` | Spielstand: Profil, Gear-Budget, Traits, Lineage, Fortschritt |
| `net/` | `NetPlugin` | ⭐ die Naht fuer Multiplayer: heute Transport `LocalOnly`, der die Intents des lokalen Spielers in die Simulation schiebt (§6) |
| `world/` | `WorldPlugin` | die Maps, Bastionsringe, Haeuser; Ankerpunkte, Kollision, **raeumlicher Index** |
| `render/` | `RenderPlugin` | Kamera, Licht, Himmel, Meshes bauen, Modelle laden |
| `player/` | `PlayerPlugin` | der Koerper: laufen, springen, Boden-Kollision, Zustandsmaschine |
| `vector/` | `VectorPlugin` | ⭐ **der Kern** (Vector Gear): Haken, Seil, Schwung, Gas, Boost, Wandlauf |
| `blades/` | `BladesPlugin` | Klingen: Schwung, Abnutzung, Bruch, Wechsel, Nachschub |
| `titan/` | `TitanPlugin` | Titanen: Rig, Gliedmassen, Cortex, KI |
| `combat/` | `CombatPlugin` | Treffer, Schaden aus Geschwindigkeit, Amputation, Dampf, Tod |
| `mission/` | `MissionPlugin` | Einsatz: Ziele, Phasen, Spawn-Wellen, Sieg/Niederlage |
| `progress/` | `ProgressPlugin` | XP, Mark/Sigil, Gear-Budget, Traits, Lineage, Ascension |
| `squad/` | `SquadPlugin` | Mitspieler und Eskorte: Kampfunfaehigkeit, Wiederbeleben, Markieren |
| `hud/` | `HudPlugin` | Gas, Klingenzustand, Ziel-Marker, Fadenkreuz |
| `sound/` | `SoundPlugin` | Gas-Zischen, Hakeneinschlag, Klingenschnitt, Titanenschritt |
| `menu/` | `MenuPlugin` | Hauptmenue, Pause, Optionen |
| `debug/` | `DebugPlugin` | F3-Overlay, Gizmos, `--script`-Fahrer |

## Wer schreibt was — die Autoritaetstabelle

**Ein Feld hat genau einen Schreiber.** Zwei Systeme auf demselben Feld sind kein Design,
sondern ein Muenzwurf mit 60 Hz — und im Netz ein Auseinanderlaufen, das niemand reproduzieren
kann (§5 Regel 4, §6 Regel 6). **Spaeter heisst „der Schreiber" schlicht „der Server".**

| Feld / Komponente | Schreiber | Leser |
|---|---|---|
| `Intent` am Spieler | **`net`** (Transport `LocalOnly`: Tastatur, Skript-Fahrer, spaeter das Netz) | `player`, `vector`, `blades` |
| `Transform` des Spielers | `player` (Boden, Schwerkraft) und `vector` (Seilkraefte) — **getrennt nach Zustand**, nie gleichzeitig; der Zustand steht in `shared::Bewegungszustand` | alle lesend |
| `Gas`, `Klingen` | `vector` bzw. `blades` | `hud`, `sound` |
| Titan-Koerper (Gliedmassen, Cortex) | `titan` | `combat` liest, schickt Messages |
| Missionszustand | `mission` | `hud`, `squad` |
| Spielstand | `save` | `progress` |

## Die drei Trennungen, die man nicht verwischt

`tools/` **baut** Dinge (Blender, Atlas, Klang, Pruefer) — `scripts/` **spielt** das Spiel
(`--script`-Fahrten) — `assets/` enthaelt das **Ergebnis**, und `assets/extern/` allein
enthaelt Fremdes.

## Uebersetzungen aus der Design-Bibel (Roblox → Bevy)

Die Bibel ist an sechs Stellen fuer Roblox geschrieben, weil die Referenz ein Roblox-Titel
ist. **Diese Stellen werden uebersetzt, nicht befolgt** (`prompts/init.md` §2). Was beim
Arbeiten zusaetzlich auftaucht, kommt **hier** dazu — nicht zurueckfragen, nicht ignorieren.

| Bibel / Backlog (Roblox) | hier (Bevy/Rust) | gefunden in |
|---|---|---|
| `CollectionService`-Tag `AnchorSurface` | eine Komponente `Ankerflaeche` an der Entity, gefuehrt ueber den raeumlichen Index in `world/` | `F-003` |
| `RopeConstraint` | eigene Seilrechnung in `vector/` gegen `Time<Fixed>`, keine Engine-Constraint | `F-004` |
| `ParticleEmitter` / `Beam` | Effektdefinition als Daten unter `assets/vfx/`, ausgewertet in `render/` | Blatt `05_VFX` |
| `studs` als Laengenmass | **1 stud = 0,28 m**, siehe `docs/konventionen.md` | Blatt `08_Maps`, `F-002` |
| `Places` / Instanzen | Bevy-`States` + Szenen; „Instanz" heisst hier Server-Sitzung | Bibel 3.6 |
| ProfileStore / Session-Lock | `save/` mit derselben **Anforderung** (kein Datenverlust, keine Duplikation), eigene Umsetzung | Bibel 6.4 |
| Future Lighting / Farbatlas / Fernnebel | Bevy-PBR + `DirectionalLight` + Fog; **der Stil bleibt exakt** | Bibel 3.4 |
| Roblox-Store / Robux / Saisonpass | entfaellt in dieser Form — **nichts davon wird gebaut**, siehe `docs/FRAGEN.md` Q-001 | Bibel 2 P5 |
| Plattform-Moderation (kein Splatter) | bleibt als **Stilregel**: Titanen verdampfen, Dampf statt Blut | Bibel 3.3 |

Verwandt: [`docs/konventionen.md`](konventionen.md) · [`docs/multiplayer.md`](multiplayer.md) ·
[`docs/lessons/bevy.md`](lessons/bevy.md)
