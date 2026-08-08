# Auftrag: **Defeated by Titans** — ein 3D-Titanenkampfspiel in Bevy, von null

**Du liest `prompts/init.md` in `~/Documents/defeated-by-titans/`.** Es gibt noch keinen Code,
keine Assets, kein Git — das anzulegen ist dein erster Schritt. Vorhanden ist nur das Gerüst:
`init.md` im Wurzelverzeichnis (der Startknopf, der hierher zeigt), **`prompts/`** (diese Datei und
alles, was daneben liegt) und **`gameplay/`** (§2).

Diese Datei ist der **Initialprompt**: sie sagt, *was* gebaut wird, *wie* der Baum aussieht, *was
Bevy von dir braucht* und **wie du deinen Zustand dokumentierst, damit andere Agenten dir glauben
können**. Lösch sie nicht — sie wird der erste Commit und geht erst am Ende (§16).

> ## ⚠️ Zuerst: **lies ALLE Dateien in `prompts/`, bevor du eine Zeile Code schreibst**
>
> ```bash
> ls -la prompts/ && cat prompts/*.md
> ```
>
> Diese Datei ist **ein Teil des Auftrags, nicht der ganze**. Neben ihr können weitere
> Markdown-Dateien liegen (oder später dazukommen) — Nachträge, Präzisierungen, Design-Notizen,
> Korrekturen. **Erst alle zusammen ergeben das vollständige Bild.** Also:
>
> 1. **Alles in `prompts/` lesen**, dann `gameplay/` (§2), dann anfangen.
> 2. **Widerspruch?** Die **spezifischere und neuere** Datei gewinnt (Änderungsdatum vergleichen:
>    `ls -lt prompts/`). Bei echtem Konflikt: **nicht selbst entscheiden** → `docs/FRAGEN.md`, und
>    solange nach dieser Datei arbeiten.
> 3. **Die Rangfolge über alles:** `gameplay/` bestimmt den **Inhalt** des Spiels, `prompts/` das
>    **Handwerk** — Aufbau, Struktur, Regeln, Beweispflicht.
> 4. **Am Anfang jeder Sitzung neu nachsehen** (`ls -lt prompts/ gameplay/`). Der User legt
>    jederzeit etwas dazu, auch mitten in einer Sitzung. Was du gestern gelesen hast, ist nicht
>    alles.
> 5. **Schreib in `docs/README.md` eine Zeile, welche Prompt-Dateien du gelesen hast** (Name +
>    Änderungsdatum). Ein anderer Agent muss sehen können, auf welchem Auftragsstand du warst.
>
> ### Das Manifest von `prompts/`
>
> | Datei | was sie ist | Pflicht? |
> |---|---|---|
> | `init.md` | **diese Datei** — der Rahmen: Spiel, Struktur, Regeln, Beweispflicht, Ziellinie | ja |
> | **jede weitere `*.md` in `prompts/`** | Nachtrag, Präzisierung, Design-Notiz, Korrektur, Feature-Beschreibung — vom User später hinzugefügt | **ja, alle** |
>
> **Es gibt keine optionale Datei in diesem Ordner.** Alles, was darin liegt, ist Teil des
> Auftrags und wird gelesen, *bevor* gebaut wird — auch was hier nicht namentlich steht, weil es
> erst nach dieser Zeile entstanden ist. Kommt während der Arbeit eine dazu, wird sie **sofort**
> gelesen und der Plan angepasst, nicht erst „nach der aktuellen Stufe".
>
> Und umgekehrt: **`init.md` ist nicht die Zusammenfassung der anderen.** Sie weiß nicht, was in
> ihnen steht. Wer nur sie liest, hat den Auftrag nicht gelesen.

> **Das Wichtigste in einem Satz:** am Ende von Tag 1 hängt ein Mensch an einem Haken, schwingt
> durch eine 3D-Szene und schneidet einem Titanen den Nacken auf — und in `docs/STATUS.md` steht
> für jede einzelne Sache ehrlich, auf welcher der **vier Stufen** sie steht.

---

## 1. Was das Spiel ist

Ein **3D-Lowpoly-Actionspiel über den Kampf gegen Titanen** (Attack on Titan). Du bist ein
Aufklärer mit **ODM-Gear** (Omni-Directional Mobility): zwei Greifhaken, zwei Gastanks, zwei
Klingen. Du hakst ein, schwingst, beschleunigst mit Gas und tötest einen Titanen **nur** durch
einen schnellen Schnitt in den **Nacken**. Alles andere kostet ihn ein Bein und dich Zeit.

**Vorbild:** [Attack on Titan Revolution](https://www.roblox.com/games/13379208636/Attack-on-Titan-Revolution)
(Roblox). Was daran zu übernehmen ist:

| Baustein | Was gemeint ist |
|---|---|
| **ODM als Kern** | Haken schießen, Seil einholen, Schwungenergie, Gas-Boost, Boost-Dash, Wandlauf. Das Spiel steht und fällt mit diesem Gefühl — nicht mit der Titanen-KI. |
| **Nacken ist die einzige Wahrheit** | Ein Nackentreffer tötet, egal wie voll der Titan ist. Alles andere ist Vorbereitung: Beine ab = er fällt, Arme ab = er kann nicht greifen, Augen = er sieht dich nicht. |
| **Schaden kommt aus Geschwindigkeit** | Ein Schnitt aus dem Stand kratzt. Derselbe Schnitt aus 30 m/s tötet. Die Formel gehört in die RON, nicht in den Code. |
| **Wirtschaft statt Cooldowns** | Gas ist endlich, Klingen werden stumpf und brechen. Nachladen an Versorgungspunkten / vom Pferd / an gefallenen Kameraden. |
| **Titanen-Typen** | Normal (stumpf, direkt), Abnormal (rennt an dir vorbei aufs Ziel), Deviant (springt, greift nach Haken), Shifter/Boss (Phasen, gepanzerter Nacken). |
| **Missionen / Raids** | Ein Einsatz hat Ziele und Phasen: Titanen räumen, einen Trupp eskortieren, ein Tor halten, ein Boss mit Phasenwechsel. |
| **Progression in Daten** | XP, Gold, Gear-Stufen, Perks, Familien-Passive. **Alles Zahlen in RON-Dateien**, kein Balancing in Rust. |

**Und es wird ein Mehrspieler-Spiel** (Koop-Einsätze, wie beim Vorbild). Der Netzcode wird nicht
heute gebaut — **die Architektur wird aber von Anfang an dafür gebaut**, sonst ist Multiplayer
später ein Umbau des ganzen Spiels statt ein Zubau. Wie genau: **§6**, und das ist keine Kür.

**Später, nicht jetzt:** Titan-Shifting (selbst ein Titan werden), Pferde, Donnerspeere.
Schreib sie in `docs/ROADMAP.md`, bau sie nicht.

⚠️ **Eigene Kunst, keine geklauten Assets.** Keine Rips aus Roblox, keine Anime-Frames, keine
Modelle aus fremden Spielen. Lowpoly selbst gebaut (Blender → glTF/`.glb`, oder erstmal aus
Bevys Primitiven zusammengesetzt). Der Titan ist ein Mensch mit falschen Proportionen — das ist
mit Kapsel, Box und Zylinder erreichbar und sieht sofort richtig aus.

---

## 2. ⭐ Die Gameplay-Quelle: der Ordner `gameplay/`

**Im Ordner `gameplay/` liegt, WAS gebaut werden soll.** Das Herzstück ist eine **Excel-Datei mit
allen Features** (`.xlsx`) — **sehr viele Zeilen**, dazu weitere gameplay-spezifische Dateien
(Mechaniken, Zahlen, Titanen, Missionen, Skizzen). Der Ordner existiert schon;
`gameplay/README.md` sagt, wie er gedacht ist.

> **Er ist die Autorität für den Inhalt. Diese Datei ist nur die Autorität für das Handwerk.**
> Wenn `gameplay/` etwas anderes sagt als §1 oder als der Code: **`gameplay/` gewinnt.** Ein
> Widerspruch *innerhalb* des Ordners ist keine Entscheidung, die dir gehört → `docs/FRAGEN.md`.

- **Erste Handlung jeder Sitzung: `ls -R gameplay/` und alles Neue lesen.** Der Ordner kann
  jederzeit wachsen, auch mitten in einer Sitzung. Was du gestern gelesen hast, ist nicht alles.
- **Nichts darin wird gelöscht, überschrieben oder abgehakt.** Das ist der Eingangskorb des Users,
  nicht dein Arbeitsblatt. Hinzufügen darfst du, fremden Text ändern nie. **Abgehakt wird in
  `docs/STATUS.md` und `docs/TODO.md`.**
- **Keine Zeile darf verschwinden.** Was dir unnötig, widersprüchlich oder unmöglich erscheint,
  kommt als Frage nach `docs/FRAGEN.md` — nicht in den Müll und nicht in ein stilles „habe ich
  weggelassen".

### ⭐ Die Feature-Liste ist eine **Excel-Datei** — sie wird ausgelesen, nicht abgetippt

**Die `.xlsx` bleibt liegen und bleibt unangetastet** — sie ist die Quelle, und der User arbeitet
darin weiter. **Aber sie ist kein Arbeitsformat:** man kann sie nicht greppen, nicht diffen, nicht
aus einem Test heraus lesen und nicht mit einem Subagenten teilen. Also gilt **beides**: die Datei
**behalten** *und* in ein Format **extrahieren**, mit dem man maximal effektiv arbeiten kann.

**Auslesen per Skript, niemals von Hand.** Bei hunderten Zeilen verliert Abtippen garantiert
Zeilen, und niemand merkt welche:

```bash
python3 -c "import openpyxl" || pip install --user openpyxl     # oder pandas
# Notausgang ohne Python-Paket: libreoffice --headless --convert-to csv --outdir /tmp <datei>.xlsx
```

```python
import openpyxl
wb = openpyxl.load_workbook("gameplay/features.xlsx", data_only=True)  # data_only: WERTE, nicht Formeln
for ws in wb.worksheets:          # ⚠️ ALLE Blätter, nicht nur das erste
    print(ws.title, ws.max_row, ws.max_column)
```

**Die Excel-Fallen, und alle sehen nach „vollständig gelesen" aus:**

| Falle | Was passiert |
|---|---|
| **Mehrere Blätter** | Das zweite Blatt ist oft die Hälfte des Spiels. `wb.worksheets` durchlaufen, nicht `wb.active`. |
| **Formeln statt Werte** | Ohne `data_only=True` steht in der Zelle `=B2*1.5` statt der Zahl. |
| **Bedeutung in der FARBE** | Ein farbcodierter Prioritäts- oder Status-Spalte ist in jedem Textexport **unsichtbar**. Fällt dir eine Spalte auf, deren Sinn in der Formatierung steckt (Füllfarbe, Durchstreichung, fett) → `cell.fill.start_color.rgb` lesen **oder** in `docs/FRAGEN.md` fragen. **Nicht ignorieren.** |
| **Verbundene Zellen** | Eine Überschrift über fünf Spalten liefert vier leere Werte. `ws.merged_cells` prüfen. |
| **Ausgeblendete Zeilen/Spalten** | Sind trotzdem Inhalt. |
| **Leerzeilen am Ende / Kommentare in Zellen** | `max_row` lügt gern; `cell.comment` enthält manchmal die eigentliche Anforderung. |

**Der Beweis, dass nichts verlorenging, ist eine Zahl:** *Zeilen in der Tabelle (ohne Kopf und
Leerzeilen) == Einträge im extrahierten Format.* Diese Zählung steht in `docs/TODO.md` und als
**Test**. Stimmt sie nicht, ist die Extraktion nicht fertig — und du weißt genau, wie viele Zeilen
fehlen, statt es zu ahnen (§9).

### Das Zielformat: **`docs/features.ron` + generierte Ansichten**

```ron
// docs/features.ron — die Feature-Liste als Daten. EINE Quelle, viele Ansichten.
features: [
    (id: "F-001", name: "Haken einschlagen", domain: "odm", stufe: Nicht,
     beschreibung: "Linke Maustaste schießt den Haken; er hält an jeder Fläche mit Normale.",
     abhaengt_von: [], quelle: "features.xlsx!ODM!Z12", prio: 1),
    (id: "F-002", name: "Seil einholen", domain: "odm", stufe: Halb,
     beschreibung: "…", abhaengt_von: ["F-001"], quelle: "features.xlsx!ODM!Z13", prio: 1),
]
```

**Warum RON und nicht Markdown:** greppbar, diffbar, aus Tests lesbar, aus einem Skript
erzeugbar — und dieselbe Disziplin wie der Rest des Projekts (§4). Bei hunderten Features ist eine
handgepflegte Markdown-Tabelle nach drei Tagen kaputt.

Daraus werden **Ansichten generiert**, nie parallel gepflegt (ein Schreiber pro Feld, §5):

- **`docs/TODO.md`** — nach Domäne gruppiert, in baubarer Reihenfolge (`abhaengt_von` respektiert).
- **`docs/STATUS.md`** — jede Zeile trägt ihre **`F-ID`** und ihre Stufe (§8).
- Erzeugt von einem kleinen Werkzeug (`src/bin/features.rs` oder `tools/features.py`), das man
  **jederzeit erneut laufen lassen kann**. Handarbeit in einer generierten Datei ist verloren —
  schreib das als Kopfzeile in jede erzeugte Datei.

**Die `F-ID` ist das Bindeglied durch das ganze Projekt:** sie steht in der Commit-Message
(`F-014: Gas-Verbrauch beim Boost`), im Testnamen (`f014_boost_verbraucht_gas`) und in der
STATUS-Zeile. Damit ist für **jede** Zeile der Excel-Tabelle in einem `grep` beantwortbar: ist das
gebaut, wo, und wie belegt?

**Und die Datei selbst:** die `.xlsx` wandert bei der Auflösung des Gerüsts (§16) nach
`docs/gameplay/features.xlsx` und wird **nie gelöscht**. Wenn der User eine neue Version einlegt:
**erneut extrahieren und die Extraktion diffen** — neue Zeilen kommen als ⬜ dazu, geänderte
Zeilen bekommen eine Notiz, **verschwundene Zeilen werden nicht stillschweigend gelöscht**, sondern
in `docs/FRAGEN.md` aufgelistet.

### Die Übersetzung ist die eigentliche Arbeit

| Was im Korb liegt | Wohin es geht |
|---|---|
| Die **Excel-Feature-Liste** | per Skript → `docs/features.ron` (jede Zeile eine `F-ID`) → generiert `docs/TODO.md` + `docs/STATUS.md`. Datei bleibt erhalten. |
| Ein **Feature / eine Mechanik** (aus Text/Excel) | `docs/gameplay/<thema>.md` (das Design: *warum so*, mit `F-ID`) **+ eine ⬜-Zeile in `docs/STATUS.md`** |
| Eine **Zahl / Balance** | in die passende `assets/data/*.ron` — **niemals in den Rust-Code** (§4) |
| Ein **Item / Titan / Perk / Einsatz** | ein Eintrag in `titans.ron` / `gear.ron` / `perks.ron` / `missions.ron` |
| Eine **Skizze / ein Bild** | bleibt in `gameplay/bilder/`, wird aus `docs/gameplay/` verlinkt |
| Etwas **Unklares** | `docs/FRAGEN.md` — nicht raten, nicht drumherum bauen und hoffen |

**`docs/TODO.md` ist die Landkarte, die daraus entsteht:** jede Zeile der TODO-Liste des Users
wird übernommen, mit **Domäne** und **Stufe** (am Anfang ⬜) — sortiert nach der Reihenfolge, in
der es baubar ist, nicht nach der Reihenfolge, in der es notiert wurde. Dazu ein Satz pro Zeile,
*warum* sie da steht, wo sie steht. Ein anderer Agent muss offene Arbeit finden können, **ohne**
den Eingangskorb zu lesen.

**Und: die TODO-Liste schlägt den Stufenplan (§11).** Der Stufenplan sagt, wie man von null zu
einem laufenden Spiel kommt; sobald die TODO-Liste da ist, sagt sie, was danach passiert. Bau nie
etwas Großes, das in keiner der beiden Listen steht — trag es zuerst ein.

---

## 3. Bevy — was es ist und was du dafür brauchst

**Bevy ist eine ECS-Engine in Rust.** Es gibt keine Klassenhierarchie und keinen Szenengraphen
im klassischen Sinn. Es gibt drei Dinge, und alles ist eines davon:

- **Entity** — eine ID. Ein Titan, eine Klinge, die Kamera, ein Haken.
- **Component** — Daten an einer Entity (`Transform`, `Gas`, `Nape`, `Hooked`).
- **System** — eine Funktion, die pro Frame läuft und über Components abfragt (`Query`).

Dazu **Resources** (globaler Zustand: die geladene RON, die Uhr), **Messages** (Nachrichten
zwischen Systemen) und **States** (Menü / Laden / Einsatz). Ein **Plugin** ist ein Bündel aus
Systemen + Resources — und in diesem Projekt ist **ein Plugin genau eine Domäne** (§5).

### Das absolute Minimum, um ein Fenster zu sehen

```toml
# Cargo.toml
[package]
name = "defeated_by_titans"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = "0.19"          # ⚠️ prüfe die neueste Version, siehe unten
ron  = "0.11"          # die Datendateien
serde = { version = "1", features = ["derive"] }

# Ohne das ist ein Debug-Build UNSPIELBAR: Bevy selbst macht das Batching, die
# Transform-Propagation und das Rendern. Der eigene Crate bleibt billig zu kompilieren,
# jede Dependency wird auf voller Stufe gebaut. Das ist der Unterschied zwischen 20 und 200 fps.
[profile.dev]
opt-level = 1
[profile.dev.package."*"]
opt-level = 3
```

```rust
// src/main.rs
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Defeated by Titans".into(),
                ..default()
            }),
            ..default()
        }))
        .run();
}
```

### Was du für **3D** zusätzlich brauchst (und was oft vergessen wird)

`DefaultPlugins` bringt 3D schon mit (`bevy_pbr`, glTF-Lader, PBR-Materialien). Du brauchst
**keine** Extra-Dependency dafür. Was in der Szene liegen muss, damit man überhaupt etwas sieht:

1. **Eine Kamera** — `Camera3d` + `Transform`. Ohne sie ist das Bild leer und wirkt wie ein Bug.
2. **Licht** — mindestens eine `DirectionalLight` (die Sonne) und ein Wert für `AmbientLight`.
   PBR ohne Licht ist schwarz. Schatten kosten viel — **erst zum Schluss einschalten und messen**.
3. **Etwas Sichtbares** — `Mesh3d(handle)` + `MeshMaterial3d(handle)`. Meshes aus
   `bevy::math::primitives` (`Cuboid`, `Sphere`, `Capsule3d`, `Cylinder`, `Plane3d`) oder aus
   `.glb` per `asset_server.load("modell.glb#Scene0")`.
4. **Ein Achsen-Vertrag, den du EINMAL festlegst und aufschreibst:** in Bevy zeigt **+Y nach
   oben**, die Kamera schaut standardmäßig nach **−Z**. Schreib in `docs/konventionen.md`, wohin
   ein Modell blickt (Vorschlag: Gesicht nach **−Z**, `yaw = 0` = Blick nach −Z) und richte jedes
   Modell **im Blender-Datei**, nicht per Offset in der Config. Ein Offset-Feld pro Modell ist der
   Anfang von dreißig Offset-Feldern.
5. **Eine Einheit.** Lege fest: **1 Bevy-Einheit = 1 Meter.** Ein Mensch ist 1,8, ein Titan 3–15,
   ein Haken fliegt 60–120. Wenn du eine Konstante anlegst, die Meter misst: **Einheit in den
   Namen oder den Kommentar an die Rechenstelle**, Zahl in die RON.

### Bevy-Fallen, die dich sonst einen halben Tag kosten

- ⚠️ **Bevys API dreht sich zwischen Minor-Versionen hart.** `cargo add bevy` und dann in der
  echten Doku der **installierten** Version nachsehen (`cargo doc --open -p bevy`), nicht aus
  dem Gedächtnis oder aus Blogposts von vor zwei Versionen schreiben. Namen, die zuletzt
  gewandert sind: `Event` → **`Message`** für gepufferte Nachrichten (`MessageWriter`/
  `MessageReader`), `SpriteBundle`/`PbrBundle` → **einzelne Komponenten** (`Mesh3d`,
  `MeshMaterial3d`), `time.delta_seconds()` → `delta_secs()`. **Prüfen, nicht annehmen.**
- **`add_plugins((..))` nimmt maximal ~15 Elemente pro Tupel**, ein System maximal ~16
  Parameter. Beides schlägt als unlesbarer Trait-Fehler zu. Lösung: verschachteln
  (`((A, B), C, …)`) bzw. Parameter in ein `SystemParam`-Struct bündeln.
- **Commands sind verzögert.** Was du diesen Frame spawnst, existiert erst am Ende des Frames.
  Ein Test/Skript, das spawnt und im selben Atemzug prüft, prüft ins Leere.
- **`cargo run`, NIE `./target/debug/<name>`.** Das nackte Binary sucht `assets/` relativ zum
  Arbeitsverzeichnis und findet nichts: leere Welt, keine Fehlermeldung, sieht exakt wie ein
  Render-Bug aus.
- **Audio:** Bevys Default-Decoder ist Vorbis allein. Wenn du `.wav` benutzt:
  `bevy = { version = "…", features = ["wav"] }` — sonst lädt jeder Klang fehlerfrei und spielt
  Stille.
- **RON + Serde:** RON kennt kein `include`. Wenn eine Datendatei zu groß wird, splittest du sie
  **in Rust** (mehrere Dateien einlesen und zusammenfügen), nicht in RON.

---

## 4. Die Datenregel: **Zahlen gehören in RON, nicht in Rust**

Ein neuer Titan-Typ, eine Klingenstufe, ein Perk, eine Missionsvorlage, eine Gas-Kostenzahl:
**Datei-Arbeit, kein Rust.** Im Code stehen nur *Einheiten* und *Mechanik*.

```
assets/data/
  game.ron       Tuning: ODM (Hakenreichweite, Seilzug, Gas, Boost), Kamera, Physik, Survival
  titans.ron     die Titanen-Typen (Größe, Tempo, Regeneration, Nacken-Größe, KI-Profil)
  gear.ron       Klingen, Tanks, Haken, Upgrade-Stufen und ihre Kosten
  perks.ron      Perks + Familien-Passive
  missions.ron   Einsatzvorlagen: Ziele, Phasen, Spawn-Wellen, Belohnung
  art.ron        Modellpfade, Farben, Lowpoly-Teilelisten
```

**Warum das nicht optional ist:** Balancing ist die Arbeit, die am häufigsten passiert. Wenn sie
ein Rebuild braucht, passiert sie nicht. Und: ein anderer Agent kann eine RON-Zeile ändern, ohne
deinen Code zu verstehen.

**Kein `serde(default)` für Spielwerte.** Ein fehlender Wert soll beim Laden **krachen**, nicht
still eine Null einsetzen — sonst suchst du den Bug im Code, während er in der Datei sitzt.

---

## 5. Die Ordnerstruktur: **eine Domäne = ein Ordner = ein Plugin = standalone**

```
src/
  main.rs                     nur: App bauen, Plugins in Abhängigkeitsreihenfolge, Flags lesen
  lib.rs                      pub mod je Domäne (damit tests/ dagegen bauen können)

  shared/    (kein Plugin)    Typen, die niemandem gehören: Health, Meter, Achsen-Helfer
  data/      DataPlugin       RON laden → GameData + Handles. Läuft VOR allem anderen.
  save/      SavePlugin       Spielstand: Profil, Gear-Stufen, Perks, Fortschritt
  world/     WorldPlugin      das Gelände: Distrikt, Mauern, Häuser, Wald der Riesenbäume;
                              Kollision und der räumliche Index (§9)
  render/    RenderPlugin     Kamera, Licht, Himmel, Meshes bauen, Modelle laden
  player/    PlayerPlugin     der Körper: laufen, springen, Boden-Kollision, Zustandsmaschine
  odm/       OdmPlugin        ⭐ DER KERN: Haken, Seil, Schwung, Gas, Boost, Wandlauf
  blades/    BladesPlugin     Klingen: Schwung, Abnutzung, Bruch, Wechsel, Nachschub
  titan/     TitanPlugin      Titanen: Rig, Gliedmaßen, Nacken, KI (suchen/greifen/beißen)
  combat/    CombatPlugin     Treffer: Raycast/Sweep, Schaden aus Geschwindigkeit, Amputation,
                              Dampf, Tod
  mission/   MissionPlugin    Einsatz: Ziele, Phasen, Spawn-Wellen, Sieg/Niederlage
  progress/  ProgressPlugin   XP, Gold, Stats, Perks, Familien, Gear-Upgrades
  squad/     SquadPlugin      NPC-Kameraden: folgen, angeboten, sterben
  hud/       HudPlugin        Gas, Klingenzustand, Ziel-Marker, Fadenkreuz
  menu/      MenuPlugin       Hauptmenü, Pause, Optionen
  sound/     SoundPlugin      Gas-Zischen, Hakeneinschlag, Klingenschnitt, Titanenschritt
  net/       NetPlugin        ⭐ die Naht für Multiplayer (§6) — heute ein Stub mit dem
                              Transport `LocalOnly`, aber die Naht existiert ab Tag 1
  debug/     DebugPlugin      F3-Overlay, Gizmos, `--script`-Fahrer (§11)
```

### Was „standalone" konkret heißt — und wie es geprüft wird

1. **Jeder Ordner exportiert genau ein `pub struct XPlugin` mit `impl Plugin`.** Ein Ordner ohne
   Plugin ist `shared/` oder ein Fehler.
2. **Eine Domäne darf nur nach `shared`, `data` und Bevy greifen.** Sie darf **keine** Funktion
   einer anderen Domäne aufrufen. Wer eine Ausnahme braucht (`render` liest `world`), schreibt sie
   in eine **Erlaubnisliste** in `docs/architektur.md` — mit Begründung.
3. **Kommunikation läuft über Components und Messages.** `combat` schickt `TitanHit { entity,
   part, speed }`; `titan` liest es und entscheidet, was das für seinen Körper heißt. `combat`
   weiß nicht, wie ein Titan gebaut ist.
4. **Ein Feld hat genau einen Schreiber.** Wenn zwei Systeme dasselbe Feld setzen, gewinnt das
   zuletzt gelaufene — das ist kein Design, das ist ein Münzwurf mit 60 Hz. Schreib in die
   Doku der Domäne, **wer** ein geteiltes Feld schreibt; alle anderen lesen nur.
5. **Die Reihenfolge in `main.rs` ist die Abhängigkeitsreihenfolge:**
   `data → save → net → world → render → player → odm → blades → titan → combat → mission →
   progress → squad → hud → sound → menu → debug`.
6. **Diese Regel verfällt still** — nichts geht kaputt, wenn jemand doch quer greift. Also
   **schreib einen Test, der sie prüft**: `tests/domaenen.rs` liest die Dateien unter `src/`,
   sammelt jedes `use crate::<domäne>` und fällt um, wenn eine Kante nicht in der Erlaubnisliste
   steht. Das ist ~40 Zeilen und der einzige Grund, dass die Struktur in vier Wochen noch stimmt.

### Und `docs/` spiegelt `src/`

Eine Quelldatei = eine Doku-Datei. Neue Quelldatei = **neue Doku-Datei und eine Zeile in
`docs/README.md`**. Die Doku beschreibt **nicht, was der Code tut** (das steht im Code), sondern
**warum er so ist und wo die Fallen liegen**.

---

## 6. ⭐ Es wird **Multiplayer** — und das entscheidet die Architektur ab Tag 1

Koop-Einsätze: mehrere Aufklärer im selben Einsatz gegen dieselben Titanen. **Der Netzcode ist
nicht Teil dieses Auftrags** — kein Server, keine Prediction, keine Lag-Kompensation heute. **Aber
jede Entscheidung, die Multiplayer später unmöglich oder teuer macht, wird heute vermieden.** Das
kostet jetzt fast nichts und später alles: ein fertiges Einzelspieler-Spiel netzfähig zu machen
heißt normalerweise, die Simulation neu zu schreiben.

**Die acht Regeln. Keine ist optional, und keine kostet heute mehr als fünf Minuten:**

1. **Simulation und Darstellung sind getrennt.** Die Simulation liest Eingaben + Zustand und
   schreibt Zustand. Rendering, HUD und Sound **lesen nur**. Ein System, das aus einem Mausklick
   direkt ein Mesh spawnt, ist der Anfang vom Ende — genau dieser Klick muss später vom Server
   bestätigt werden.
2. **Eingabe ist ein Datum, kein Tastendruck.** Es gibt **ein** Struct — `Intent` (Bewegungsvektor,
   Blickrichtung, Buttons, Tick-Nummer) — und die Simulation liest **nur** das. Wer es füllt, ist
   ihr egal: die lokale Tastatur, das `--script`-Harness (§11) oder später das Netz. **Genau dieser
   Kanal ist der, den Multiplayer braucht** — und du baust ihn in Stufe 1 sowieso, weil du nicht
   klicken kannst. Ein Aufwand, zwei Probleme gelöst.
3. **Es gibt keinen „den Spieler".** Nie `.single()` auf eine Spieler-Query. Jeder Spieler ist
   *einer von vielen*: `PlayerId`, und Gas/Klingen/Inventar sind **Components am Spieler**, nie eine
   globale `Resource`. Die Kamera hängt an einem `LocalPlayer`-Marker — das ist die **einzige**
   Stelle im Code, die weiß, welcher Spieler „ich" ist.
4. **Fester Simulationsschritt.** Zustand ändert sich in `FixedUpdate` (z. B. 60 Hz), das Bild
   interpoliert dazwischen. Dieselbe Regel wie §10 („nichts pro Frame"), nur strenger: im Netz ist
   ein frameabhängiges Ergebnis kein Komfortproblem, sondern **Desync**.
5. **Determinismus, wo er billig ist.** Zufall nur aus einem **geseedeten** Generator, dessen Seed
   Teil des Zustands ist (`seed + tick`), nie `rand::random()` mitten in einem System. Ein Titan,
   der auf zwei Rechnern anders abbiegt, ist ein Bug, den man nur im Netz sieht — also am
   teuersten Tag.
6. **Autorität wird benannt.** In der Doku jeder Domäne steht, **wer** ein geteiltes Feld schreibt
   (§5). Später heißt dieser Satz „der Server". Zwei Schreiber sind lokal ein Münzwurf mit 60 Hz —
   im Netz ein Auseinanderlaufen, das niemand reproduzieren kann.
7. **Stabile Ids statt Zeiger.** Alles, was gespeichert oder eines Tages verschickt wird, benutzt
   eigene Ids (`TitanId`, `PlayerId`), **nie** Bevys `Entity` — die ist ein lokaler Index mit
   Generation und bedeutet auf einem anderen Rechner etwas anderes. Dieselbe Regel rettet nebenbei
   den Spielstand.
8. **`serde` auf allem, was Zustand ist**, und **Messages so entwerfen, dass sie über eine Leitung
   passen** (Daten — keine Handles, keine Funktionszeiger, keine `Entity`). Kostet heute eine Zeile
   pro Typ.

**Die Naht heißt `src/net/`** und existiert ab Stufe 1: ein `NetPlugin`, das genau eines tut — den
Transport `LocalOnly` bereitstellen, der die Intents des lokalen Spielers in die Simulation schiebt.
Damit ist der Ort, an dem später Client und Server stehen, **vorhanden und leer**, statt später
mitten durch fünf Domänen zu schneiden.

**`docs/multiplayer.md`** hält den Plan, der noch nicht gebaut wird: Autoritätsmodell (dedizierter
Server vs. Host), wer die Titanen simuliert (der Server), was der Client vorhersagen darf (die
eigene ODM-Bewegung — sie muss sich sofort anfühlen) und was nie vorhergesagt wird (ein
Titanen-Tod). Die offenen Punkte kommen nach `docs/FRAGEN.md`: **wie viele Spieler, Koop oder auch
PvP, dedizierter Server oder Host** — Entscheidungen des Users, nicht deine.

**Der Wächter, der die Regel am Leben hält:** `tests/mehrspieler.rs` spawnt **zwei**
Spieler-Entities und lässt die Simulation ein paar Ticks laufen. Er fällt in der Sekunde um, in der
jemand `.single()` schreibt oder Spielerzustand in eine `Resource` legt. Ohne ihn verfällt dieser
Abschnitt still — und man merkt es erst, wenn Multiplayer dran ist, also nach Monaten Arbeit, die
man dann anfassen muss.

---

## 7. Die 3D-Modelle: **Claude baut sie, du füllst sie aus, der Export läuft von selbst**

**Alle 3D-Designs baust du selbst** — es gibt keine gekauften und keine geklauten Modelle. Und sie
müssen **billig zu ersetzen** sein, denn der User modelliert später nach: er öffnet eine
`.blend`, baut das Ding richtig, sagt in *einer* RON-Zeile „nimm dieses Modell", und das Spiel
benutzt es beim nächsten Start. Ohne Rust-Änderung, ohne Handarbeit am Export.

### Die Kette

```
tools/blend/<name>.py   ──►  assets/3d/blend/<name>.blend  ──►  assets/3d/glb/<name>.glb  ──►  art.ron
   Claude schreibt sie        DU öffnest und füllst aus          automatisch exportiert         der Schalter
```

### Der Assets-Ordner

```
assets/
  data/                 die RON-Dateien (§4)
  3d/
    blend/              ⭐ DIE QUELLE — von Hand editierbar, hierhin geht der User
      scout.blend             der Spielerkörper
      titan_normal.blend      Titan 5 m
      titan_gross.blend       Titan 15 m
      odm_gear.blend          Gurt, Tanks, Klingengriffe
      blade.blend
      haus_klein.blend        Stadtbausteine: Haus, Dach, Mauerstück, Tor
      baum_riese.blend
      kiste_nachschub.blend
    glb/                GENERIERT — **niemals von Hand anfassen**. Wird mitcommittet, damit
                        das Spiel auch ohne installiertes Blender läuft.
  textures/             Handgemachte PNGs (der Exporter fasst sie nie an)
  sound/
tools/
  blend/<name>.py       das Blender-Python-Skript, aus dem die .blend entsteht
```

**Warum ein Skript und nicht direkt eine `.blend`:** eine `.blend` ist ein Binärklumpen — im Git
sieht niemand, was sich geändert hat, und du kannst sie nicht schreiben, ohne Blender zu starten.
Ein Skript ist ein Diff, ist reproduzierbar, und ist der Ort, an dem *dein* Platzhalter lebt.

```bash
blender --background --factory-startup --python tools/blend/titan_normal.py
```

### ⚠️ Die wichtigste Regel dieses Abschnitts: **eine `.blend`, die der User angefasst hat, ist heilig**

Der Generator **überschreibt niemals** eine vorhandene `.blend`. Er prüft: Datei existiert und ist
neuer als ihr Skript → **„vom User bearbeitet, nicht angefasst"** ins Log, fertig. Neu erzeugt
wird nur, was fehlt; alles andere nur mit ausdrücklichem `--force <name>`. Wenn du diese Regel
brichst, löschst du Arbeit, die niemand wiederherstellen kann — Blender hat keine Historie.

### Der Auto-Export

Beim Spielstart (in `data/`, vor allem anderen) läuft ein Schritt, der für jede `.blend` prüft:
**fehlt die `.glb` oder ist sie älter?** Dann exportieren, sonst nichts tun.

```bash
blender --background --factory-startup <datei>.blend \
  --python-expr "import bpy; bpy.ops.export_scene.gltf(filepath='assets/3d/glb/<name>.glb', export_format='GLB', export_yup=True, export_apply=True, export_cameras=False, export_lights=False)"
```

- **Kein Blender installiert?** → **einmal warnen**, die vorhandene `.glb` benutzen, **nicht
  abstürzen**. Das Spiel muss auf einem Rechner ohne Blender laufen.
- **Flags:** `--reexport` (alles neu bauen), `--no-export` (Startzeit sparen). Und ein
  eigenständiges Werkzeug `src/bin/export_modelle.rs`, damit man es ohne Spielstart laufen lassen
  kann.
- Der Export ist **auch** die Stelle, an der die glTF-Fallen unten geradegezogen werden. Eine
  Nachbearbeitung des `.glb` (es ist JSON + Binärchunk) ist erlaubt und billiger, als sie in jedem
  Modell von Hand zu vermeiden.

### Der Schalter in der RON — die eine Zeile, die der User umlegt

```ron
models: {
    "titan_normal": (blend: "titan_normal", nutzen: true,  scale: 1.0),
    "scout":        (blend: "scout",        nutzen: false, scale: 1.0),  // noch Platzhalter
}
```

`nutzen: false` ⇒ das Spiel baut den **prozeduralen Platzhalter** aus Bevy-Primitiven
(Kapsel/Box/Zylinder, eingefärbt). `nutzen: true` ⇒ es lädt die `.glb`. **Beide Wege müssen
jederzeit funktionieren**, und beide benutzen dieselben Anker, dieselbe Hitbox und dieselbe
Skalierung — sonst ist das Umschalten kein Schalter, sondern ein Umbau.

### Die Konventionen, die das Ersetzen erst billig machen

Schreib sie nach `docs/konventionen.md` **und** als Kommentarkopf in jedes `tools/blend/*.py`:

- **1 Blender-Einheit = 1 Meter.** Maßstab wird im Modell gemacht, nicht per `scale` in der RON
  (das Feld ist eine Notbremse, kein Arbeitsmittel).
- **Origin zwischen den Füßen** (nicht im Körperzentrum) — sonst steht jedes Modell halb im Boden.
- **Blick nach −Z**, aufrecht. In Blender wird Z-oben modelliert, der Exporter dreht auf Y-oben
  (`export_yup=True`) — **nicht selbst rotieren**, sonst dreht es zweimal.
- **Anker sind Empties mit festen Namen**, und der Modellierer entscheidet damit *wo*, die RON
  *wie stark*: `nape` (die Todeszone!), `hit.min` / `hit.max` (die Hitbox), `hook.l` / `hook.r`,
  `hand.r` / `hand.l`, `eye`. **Fehlt ein Empty, ist die Zone ein Punkt** — und ein Nacken, der
  ein Punkt ist, fühlt sich wie ein kaputtes Spiel an.
- **Farbe per Vertex-Farben**, nicht per Textur. Lowpoly braucht keine UV-Map, und Vertex-Farben
  überleben jedes Nachmodellieren.
- **Ein Objekt pro sinnvollem Teil**, benannt (`kopf`, `arm.r`, …) — daran hängt später die
  Amputation und die Animation.

### Drei glTF-Fallen, die alle gleich aussehen („mein Modell ist weiß / chrom / unsichtbar")

1. **Bevy liest nur `COLOR_0`.** Hat ein Blender-Mesh **zwei** Color-Attribute, landet die gemalte
   Farbe in `COLOR_1` und das Modell kommt **weiß** an. Im Export das gewünschte Attribut nach
   vorn tauschen — oder im `.py` sicherstellen, dass es nur eines gibt.
2. **Fehlender `metallicFactor` bedeutet 1.0**, also *voll metallisch* — ein Diffuse-Material ohne
   den Wert sieht im Spiel wie Chrom aus. Der Export setzt ihn auf `0.0`, wo er fehlt.
3. **Kameras und Lichter nicht mitexportieren.** Sonst hängt in jedem Modell eine zweite Sonne,
   und die Szene wird von Modell zu Modell heller.

### Prüfen, und zwar jede Sitzung

- `tests/modelle.rs` — ein harter Test (jedes mit `nutzen: true` verdrahtete Modell hat eine
  `.glb` und **alle** geforderten Empties) plus ein `#[ignore]`-Bericht mit `--nocapture`, der
  eine Tabelle druckt: *Modell · `.blend` da? · `.glb` aktuell? · bemalt? · Anker vollständig? ·
  in RON verdrahtet?* Genau diese Tabelle ist das, was ein anderer Agent in zehn Sekunden lesen
  will.
- **`git status assets/3d/blend/` gehört an den Anfang jeder Sitzung.** Eine geänderte `.blend`
  heißt: exportieren, **im Spiel ansehen**, Screenshot, und die Zeile in `docs/STATUS.md`
  hochsetzen.
- **Die Stufen (§8) gelten für Modelle genauso.** Ein Primitiven-Platzhalter ist ⬜ oder 🟨 — nie
  mehr, egal wie gut er sich einbaut. Erst ein echtes Modell, das du **gesehen** hast, ist 🟧.

---

## 8. ⭐ Die Doku und die **vier Stufen** — das Herzstück dieses Auftrags

**An diesem Projekt arbeiten mehrere Agenten, teils parallel, teils nach dir.** Sie können deinen
Code lesen, aber sie können nicht sehen, was du *gesehen* hast. Genau dafür ist die Statusdoku da:
**sie sagt, wie weit man einer Sache trauen darf.**

Die Datei heißt **`docs/STATUS.md`** und ist eine Tabelle. Sie ist Pflicht, sie wird **in
derselben Nachricht** gepflegt wie der Code, und sie darf **nie** großzügiger sein als die
Wirklichkeit.

### Die vier Stufen — genau diese, keine Zwischentöne

| Marke | Stufe | Bedeutung | Wer setzt sie |
|---|---|---|---|
| ⬜ | **nicht implementiert** | Existiert nicht oder nur als Platzhalter/Stub. Auch: „Code da, tut aber nichts." | Claude |
| 🟨 | **halb implementiert** | Von Claude gebaut, **nicht getestet, nicht gesehen**. Es kompiliert. Mehr ist nicht behauptet. | Claude |
| 🟧 | **fast implementiert** | Gebaut **und** mit vielen Tests abgesichert, die umfallen, wenn es kaputtgeht, **und** von Claude im laufenden Spiel gesehen (**Screenshot**). | Claude |
| ✅ | **fertig** | **Der User hat draufgeschaut und es abgenommen.** | **NUR der User** |

**Die eiserne Regel: ✅ setzt Claude NIEMALS selbst.** Nicht bei grünen Tests, nicht bei einem
schönen Screenshot, nicht „weil es offensichtlich läuft". 🟧 ist die höchste Stufe, die du selbst
vergeben darfst. Wenn du glaubst, etwas sei reif für ✅, schreibst du es in
`docs/ABNAHME.md` — die Liste dessen, worauf der User bitte einmal schauen soll, mit dem
Screenshot-Pfad daneben.

### Was die Stufen belegen müssen

**🟧 braucht drei Belege, nicht einen:**

| | heißt |
|---|---|
| **Bild** | im laufenden Spiel gesehen — Screenshot-Pfad in der Tabelle. Nicht „gebaut". |
| **Zahl** | gemessen: Bildzeit, Zähler, Vorher/Nachher, eine Distanz. Nicht geschätzt. |
| **Code** | ein Test, der **umfällt**, wenn das Verhalten kaputtgeht (einmal absichtlich kaputt machen und zusehen, dass er rot wird — ein Test, der nie rot war, beweist nichts). |

**Und: Sonderfälle testen, nicht den Normalfall.** Der Normalfall funktioniert fast von allein.
Die Fehler sitzen an den Rändern: Haken auf eine Kante, Haken auf einen sterbenden Titanen,
Gas exakt null im Moment des Boosts, Schnitt mit einer gerade brechenden Klinge, zwei Haken in
zwei Richtungen.

### Das Format von `docs/STATUS.md`

```markdown
# STATUS — was implementiert ist und was nicht

Stufen: ⬜ nicht implementiert · 🟨 halb (gebaut, ungetestet) ·
🟧 fast (getestet + gesehen) · ✅ fertig (**nur der User setzt das**)

| Sache | Domäne | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Fenster + 3D-Kamera | render | 🟧 | `tests/kamera.rs`, `docs/bilder/kamera.png` | 2026-08-08 |
| Haken einschlagen | odm | 🟧 | `tests/haken.rs` (7 Fälle), `docs/bilder/haken.png`, Reichweite 78 m gemessen | 2026-08-08 |
| Seilzug / Schwung | odm | 🟨 | kompiliert, **kein Test, kein Bild** | 2026-08-08 |
| Nacken-Trefferzone | titan | ⬜ | — | — |
```

**Regeln für die Tabelle:**

- **Rückwärts ist erlaubt und erwünscht.** Wenn etwas kaputtgeht oder du merkst, dass ein Test
  gelogen hat: Stufe **runter**setzen. Eine zu hohe Stufe ist die teuerste Zeile im Projekt — sie
  schickt den Nächsten los, auf Sand zu bauen.
- **Keine Stufe überspringen.** Es gibt kein 🟨 → ✅.
- **Bau nicht auf 🟨.** Wenn dein neues Feature auf einer 🟨-Sache steht, bring die erst auf 🟧.
  Richtig schlägt neu: dass eine Funktion tut, was sie soll, ist wichtiger als jedes weitere
  Feature.
- **„Kein Bild möglich" wird hingeschrieben, nicht weggelassen.** Wenn es keine Grafiksitzung
  gibt (§11), bleibt die Sache 🟨 mit dem Vermerk *„gebaut + getestet, Pixel ungesehen"*. Nicht
  aufrunden.

### Die anderen Pflichtdateien

| Datei | Inhalt |
|---|---|
| `README.md` | Was das Spiel ist, wie man es startet, Tastenbelegung, aktueller Stand in einem Absatz |
| `CLAUDE.md` | **Dein Gedächtnis für die nächste Sitzung** — ein *Index*, kein Archiv: die Regeln, die immer gelten, und Zeiger auf den Rest. Halte sie unter ~150 Zeilen. |
| `docs/README.md` | Der Spiegel von `src/`: eine Zeile pro Doku-Datei |
| `docs/STATUS.md` | ⭐ die vier Stufen (oben) |
| `docs/architektur.md` | Domänen, Plugin-Reihenfolge, die Erlaubnisliste der Abhängigkeiten |
| `docs/konventionen.md` | Achsen, Einheiten, Blickrichtung, Namensregeln |
| `docs/modelle.md` | Die Modellkette (§7): welches Modell, welche Anker, welcher Stand — und die Anleitung „so tausche ich ein Modell aus", geschrieben **für den User** |
| `docs/lessons/*.md` | **Fallgeschichten**: was dich Zeit gekostet hat, ein Thema pro Datei. Der wertvollste Ordner im Projekt. |
| `docs/BUGS.md` | ⭐ jeder Bug mit **Reproduktion, Beleg, Ursache, Fix und Test** (§9) |
| `docs/ABNAHME.md` | Was der User anschauen soll, damit es ✅ werden kann |
| `docs/FRAGEN.md` | Entscheidungen, die dir nicht gehören. **Nicht unterbrechen — hier reinschreiben und drumherum arbeiten.** |
| `docs/FUNDE.md` | Fehler, die dir *nebenbei* auffallen, außerhalb deiner Aufgabe — mit der Messung daneben, damit ein anderer prüfen kann. **Nicht still mitfixen.** |
| `docs/ROADMAP.md` | Was bewusst später kommt (Shifting, Multiplayer, Pferde, Donnerspeere) |

---

## 9. ⭐ Ein Bug ohne Beleg ist ein Gerücht — **und Unsicherheit ist ein Mangel**

> **Wenn es nicht sicher ist, ist es nicht gut.** Kein „müsste jetzt gehen", kein „sollte
> passen", kein „wahrscheinlich behoben". Entweder du hast es **belegt**, oder du schreibst
> hin, dass du es nicht hast.

Das gilt in beide Richtungen: für Bugs, die du **findest**, und für Fixes, die du **behauptest**.

### a) Ein Bugbericht braucht vier Zeilen — sonst ist er keiner

Jeder Bug wird in **`docs/BUGS.md`** eingetragen, **bevor** er gefixt wird, mit:

| Feld | Was hinein muss |
|---|---|
| **Reproduktion** | Das exakte Kommando, das ihn zeigt: `cargo run -- --script scripts/haken_kante.txt`, plus Seed / Koordinate / Blickrichtung aus dem F3-Overlay. Wer es nicht nachstellen kann, kann es nicht prüfen. |
| **Beleg** | Screenshot in `docs/bilder/`, Logausschnitt **oder** eine Zahl (gemessen 34 m/s, erwartet ≤ 12). Nicht „sieht falsch aus". |
| **Erwartung** | Was stattdessen passieren müsste — und **woher** du das weißt (RON-Zeile, Doku-Absatz, Design-Entscheidung). |
| **Ursache** | `datei:zeile`, sobald bekannt. Solange sie fehlt: **„Ursache unbekannt"**, nicht geraten. |

**Kein Repro ⇒ kein Fix.** Ein Bug ohne Reproduktion wird als *unbelegt* eingetragen und **nicht
repariert** — ein Fix für etwas, das du nie gesehen hast, ist eine Änderung ohne Grund, und die
kannst du hinterher auch nicht widerlegen.

### b) Ein Fix ohne roten Test ist eine Vermutung

Die Reihenfolge ist **nicht verhandelbar**:

1. **Test schreiben, der den Bug zeigt** — und ihn laufen lassen, bis er **rot** ist. Ein Test,
   der nie rot war, beweist nichts; er beweist nur, dass er kompiliert.
2. **Fixen**, bis er grün ist.
3. **Danach den Fix wieder herausnehmen** und zusehen, dass der Test erneut umfällt. Erst dann
   weißt du, dass der Test *diesen* Fix prüft und nicht irgendetwas daneben.
4. **In `docs/BUGS.md` eintragen:** Ursache, Fix, Testname. Und wenn es eine Falle war, aus der
   man lernen kann: eine Zeile in `docs/lessons/`.

Bei einem Bug, den nur das Auge sieht (ODM-Gefühl, Kameraruckeln, ein Haken, der ins Nichts
zeigt), ist der Beleg ein **`--script`-Lauf mit `assert`** plus **Screenshot vorher/nachher**. Das
ist genau der Grund, warum der Fahrer in Stufe 1 gebaut wird.

### c) Fremde Fehler gehören nach `docs/FUNDE.md`, nicht in einen stillen Fix

Wer beim Arbeiten über etwas stolpert, das nicht zur eigenen Aufgabe gehört: **aufschreiben, mit
der Messung daneben**, damit ein anderer prüfen kann, ob es wirklich falsch ist. Ein nebenbei
mitgefixter Fremdfehler ist ein Fix, den niemand geprüft hat — und er versteckt sich im Diff einer
Aufgabe, in der ihn keiner sucht.

### d) Sicherheit im Code: nichts darf still schiefgehen

„Sicher" heißt auch: **das Programm lügt nicht und stürzt nicht an einer Stelle, die man vorher
prüfen konnte.**

- **Kein `unsafe`.** Wenn du glaubst, du brauchst es, gehört das in `docs/FRAGEN.md`.
- **`unwrap()` / `expect()` nur mit Begründung im Kommentar** — und **nie** auf Daten aus einer
  Datei oder aus Eingaben. Beim **Laden** der RON ist ein sofortiger, lauter Abbruch mit
  Dateiname und Zeile das *richtige* Verhalten (fail fast beim Start), mitten im Spiel ist er es nie.
- **Physik braucht Wachen.** ODM-Seilkräfte, Normalisierungen und Divisionen erzeugen NaN/∞,
  sobald ein Vektor Länge 0 hat oder ein Frame mal 0,5 s dauert. NaN im `Transform` ist der Bug,
  der aussieht wie „der Spieler ist verschwunden": Länge prüfen bevor normalisiert wird, `dt`
  clampen, und in `debug/` ein System, das **einmal warnt**, wenn eine Position nicht endlich ist.
- **Ein `panic!` im Spiel ist ein Bug**, auch wenn er „nie" auftritt. Und ein `Result`, das mit
  `let _ =` geschluckt wird, ist ein Fehler, den niemand mehr sehen kann.

### e) Wortwahl: schreib, was du weißt

| Nicht schreiben | Sondern |
|---|---|
| „behoben" (ohne roten Test davor) | „gefixt, Test `x` war rot, ist grün" |
| „sollte jetzt gehen" | „gebaut, **ungetestet** — 🟨" |
| „läuft" | „im Spiel gesehen, Screenshot `docs/bilder/…`" |
| „ist schneller" | „16,6 → 9,4 ms, `--release --novsync`, gemessen am …" |
| „funktioniert wahrscheinlich" | eine Zeile in `docs/FRAGEN.md` oder `docs/BUGS.md` |

**Und die Verbindung zu den vier Stufen (§8): Unsicherheit setzt die Stufe herunter, nicht
hinauf.** Wenn du dir bei einer Sache nicht sicher bist, ist sie **🟨** — auch wenn sie
funktioniert. Das kostet nichts. Eine zu hohe Stufe kostet den Nächsten einen halben Tag.

---

## 10. Performance: die Regel, die man von Anfang an einhalten muss

Eine Stadt hat Tausende Häuser, ein Einsatz Dutzende Titanen, jeder Titan sechs Gliedmaßen.
**Nichts darf alle Entities durchlaufen, um eine Frage über die zehn Meter vor der Nase zu
beantworten.**

- **Ein räumlicher Index gehört in `world/`** (Gitterzellen → Entities, gepflegt über Bevys
  `Added`/`RemovedComponents`, damit er nicht veralten kann). Hakeneinschlag, Klingentreffer,
  Kollision und Titanen-Zielsuche gehen **alle** darüber.
- **Nichts ändert sich pro Frame, alles pro Sekunde.** `* time.delta_secs()` allein reicht nicht:
  (a) **nie auf Ganzzahlen runden** — `(schaden * dt).ceil()` macht die Framerate zur
  Schadenszahl; trag Bruchteile mit. (b) **Exponentielles Glätten** ist pro Frame:
  `x += (ziel-x)*0.1` → benutze `1 - e^(-k*dt)`. (c) **Rauschen skaliert mit `sqrt(dt)`**, nicht
  mit `dt`. Schreib dafür **eine** Hilfsfunktion in `shared/` und benutze nur die.
- **Erst messen, dann behaupten.** Und: **Debug ist langsam.** `cargo run` ist ein Debug-Build
  (der eigene Crate auf `opt-level = 1`); für jede Perf-Aussage `cargo run --release`.
  Debug-Langsamkeit ist keine Regression.
- **Unter Vsync ist jede Bildzeit 16,6 ms** — damit misst „was kostet das?" sechsmal denselben
  Deckel. Bau früh ein `--novsync`-Flag ein und/oder benutze Bevys
  `RenderDiagnosticsPlugin` (echte GPU-Zeitstempel pro Renderpass).
- **Schatten sind der teuerste Schalter im Spiel.** Punktlichter sind fast gratis, Schatten nicht.
  Erst am Ende, mit Zahl.

---

## 11. Du kannst nicht klicken — bau dir die Werkzeuge **zuerst**

Das ist der Punkt, an dem solche Projekte scheitern: alles ist gebaut, nichts ist gesehen, weil
jedes Feature hinter Maus und Tastatur liegt und niemand am Keyboard sitzt. **Also kommt die
Prüfinfrastruktur vor den Features** — sie ist Teil von Stufe 1, nicht ein „wenn Zeit ist".

**a) Start-Flags, die am Menü vorbeigehen.** Ein Hauptmenü ist für dich eine Wand ohne Tür.
```bash
cargo run -- --mission tutorial   # direkt in einen Einsatz, kein Menü
cargo run -- --sandbox            # leeres Feld, ein Titan, unendlich Gas — zum Anschauen
cargo run -- --novsync            # zum Messen
cargo run -- --script <datei>     # das Spiel spielen, ohne zu tippen
```

**b) `--script`: der Fahrer.** Eine Textdatei, eine Anweisung pro Zeile, die in **dieselben
Eingaben schreibt, die ein Mensch auslöst** (`ButtonInput<KeyCode>`, `ButtonInput<MouseButton>`,
und ein „so-tun-als"-Blickvektor). **Kein zweiter, falscher Weg zu spielen** — jedes System
dahinter ist das echte.
```text
spawn titan normal 20 0 -40   # Typ und Ort in Metern
look 0 -10                    # Blickrichtung in Grad (yaw, pitch)
key Space 0.3                 # Taste 0,3 s halten
hook left                     # Haken raus
wait 1.2                      # Commands sind verzögert — sonst fotografierst du ein leeres Feld
mark eingehakt                # eine Zeile ins Log, an der man einen Screenshot ausrichtet
assert speed > 25             # ⭐ das Skript darf selbst urteilen: fällt es um, ist es ein Test
```
`assert` ist der Grund, warum das mehr ist als eine Demo: damit wird eine **Fahrt** zu einem
Test, und ODM-Gefühl ist genau die Sorte Sache, die kein Unit-Test greift.

**c) Ein Debug-Overlay (F3), das jede Meldung nachstellbar macht.** Position, Blickrichtung,
Geschwindigkeit, Gas, Hakenzustand, Bildzeit — **im Bild**. Dazu ein `warp x y z` + `look` im
Skript. Damit kann der User dir eine Koordinate schicken und du stehst genau dort. Das ist
mehr wert als jedes Bug-Formular.

**d) Screenshots (niri/Wayland):**
```bash
setsid nohup cargo run -- --sandbox > /tmp/dbt.log 2>&1 < /dev/null & disown
sleep 20   # der erste Build dauert
ID=$(niri msg --json windows | python3 -c "import sys,json;print([w['id'] for w in json.load(sys.stdin) if (w.get('title') or '')=='Defeated by Titans'][0])")
niri msg action focus-window --id $ID   # SONST drosselt der Compositor auf ~5 fps
sleep 2
niri msg action screenshot-window --id $ID
```
Landet in `~/Pictures/Screenshots/`. **Kopier die Bilder nach `docs/bilder/`** und verlinke sie in
`STATUS.md` — ein Screenshot, den niemand mehr findet, ist kein Beleg.

- **Ein unfokussiertes Fenster wird auf ~5 fps gedrosselt.** Das ist keine Regression, auch wenn
  es exakt so aussieht. Vor jeder fps-Messung fokussieren.
- **Prüfe, dass nur EINE Instanz läuft**, sonst screenshottest du alten Code.
- **Manchmal gibt es gar keine Grafiksitzung** (kein `WAYLAND_DISPLAY`/`DISPLAY`) → `cargo run`
  panikt sofort. Dann gibt es **kein Bild**, dann bleibt die Sache **🟨** und du bittest den User
  draufzuschauen. Nicht aufrunden.

---

## 12. Der Stufenplan — nach jeder Stufe **läuft** das Spiel

Wer breit anfängt (Titanen-KI, Missionen, Perks gleichzeitig), hat nach einem Tag nichts, was
startet, und keine Ahnung, welcher von zwölf Umbauten es kaputtmacht. Also schmal und tief, und
**jede Stufe wird einzeln committet** — der Commit ist dein Rückweg.

### Stufe 0a: **Preflight** — frag die Maschine, bevor du baust

Das kostet zwanzig Sekunden und entscheidet, was heute überhaupt möglich ist. Das Ergebnis kommt
als Tabelle nach `docs/umgebung.md`, damit der nächste Agent es nicht wieder herausfinden muss:

```bash
rustc --version && cargo --version     # Rust vorhanden? edition 2024 braucht 1.85+
df -h /home                            # ⚠️ ein Bevy-target/ wird zweistellig GB. Unter 20 GB frei: erst reden, dann bauen
echo "WAYLAND=$WAYLAND_DISPLAY DISPLAY=$DISPLAY"   # leer ⇒ KEIN Screenshot möglich (§11)
command -v niri && niri msg --version   # der Compositor für Screenshots
command -v blender && blender --version # fehlt ⇒ Modellkette baut nur .py, exportiert nicht (§7)
command -v gh && gh auth status          # fehlt/nicht angemeldet ⇒ Schritt 3 der Ziellinie braucht dich
nproc                                    # wie breit darf parallel gebaut werden (§15)
```

**Was fehlt, wird nicht simuliert und nicht beschönigt** — es wird in `docs/umgebung.md` notiert und
die betroffenen Sachen bleiben auf ihrer ehrlichen Stufe (§8). Ohne Grafiksitzung gibt es kein 🟧.

### Stufe 0b: **Das Projekt anlegen** — `cargo init`, nicht `cargo new`

Der Ordner ist **nicht leer** (`prompts/`, `gameplay/` liegen drin), also legt `cargo new` ein
Unterverzeichnis an bzw. bricht ab. Richtig ist:

```bash
cd ~/Documents/defeated-by-titans
cargo init --name defeated_by_titans      # Paket im VORHANDENEN Ordner
cargo add bevy                            # schreibt die WIRKLICH aktuelle Version in Cargo.toml
cargo add ron serde --features serde/derive
git add -A && git commit -m "Projekt aufgesetzt (Initialprompt in prompts/)"
```

Der Paketname ist **`defeated_by_titans`** (mit Unterstrichen — Rust mag keine Bindestriche im
Crate-Namen), das Fenster heißt **„Defeated by Titans"**, das GitHub-Repo `defeated-by-titans`.
Diese drei Schreibweisen bleiben so; jede steht an genau einer Stelle in der Doku.

| Stufe | Fertig, wenn |
|---|---|
| **0** | Preflight (0a) durch, `cargo init` (0b), `Cargo.toml` mit den Profilen, **leeres Fenster geht auf**. `docs/`-Skelett + `STATUS.md` + `TODO.md` + `CLAUDE.md` stehen — kurz, aber echt, damit Subagenten (§15) etwas zu lesen haben. |
| **1** | 3D-Szene: Boden, Sonne, ein Würfel. **FPS-Kamera** dreht mit der Maus, WASD läuft, Schwerkraft und Boden-Kollision — die Bewegung liest **`Intent`**, nicht die Tastatur (§6). **Plus: `--sandbox`, `--script`, F3-Overlay, ein Screenshot, `src/net/` als `LocalOnly`-Stub und `tests/mehrspieler.rs` mit zwei Spielern.** |
| **1b** | ⭐ **Die Modellkette steht mit EINEM Modell** (§7): `tools/blend/scout.py` → `scout.blend` → Auto-Export → `.glb` → `nutzen: true` in `art.ron` → im Spiel gesehen. Der Platzhalter-Weg (`nutzen: false`) läuft daneben weiter. **Vor Stufe 2** — jedes weitere Modell ist danach eine Kopie dieser Kette, und der User kann ab hier jederzeit selbst modellieren. |
| **2** | Die **Stadt** steht: Mauer, Häuser, Dächer, Bäume — hakbare Flächen (erst Platzhalter, dann `.blend`). Kollision gegen alles davon, über den räumlichen Index. |
| **3** | ⭐ **ODM: Haken raus, einhaken, Seil einholen, schwingen, Gas verbrauchen, Boost.** Ich fliege durch die Stadt und es fühlt sich gut an. ← **die Marke für Tag 1** |
| **4** | Ein **Titan** steht in der Stadt: Rig, Gliedmaßen, Nacken als eigene Trefferzone. Klinge schneidet, Nackentreffer tötet, Bein ab = er fällt. Schaden aus Geschwindigkeit. |
| **5** | Der Titan **wehrt sich**: sucht, geht, greift, packt dich, wirft dich. Dampf, Regeneration, Tod. Klingen werden stumpf und brechen, Gas geht aus, Nachschub. |
| **6** | Ein **Einsatz** mit Zielen und Phasen (`missions.ron`), Spawn-Wellen, Sieg/Niederlage, ein Trupp NPC-Kameraden. |
| **7** | **Progression**: XP, Gold, Gear-Upgrades, Perks, Familien — komplett aus RON. Hauptmenü, Speichern/Laden. |
| **8** | **Politur und Zahlen**: mehr Titanen-Typen, ein Boss mit Phasen, Sound, Performance mit vielen Titanen gemessen, `docs/` durchgesehen und wahr, `docs/ABNAHME.md` gefüllt. |

**Stufe 3 ist die Marke für Tag 1.** Steht sie mittags, zieh weiter. Steht sie abends nicht, ist
das kein Scheitern — es heißt, Stufe 1 oder 2 hatte eine Überraschung, und die willst du gefunden
haben, *bevor* zwanzig Titanen daran hängen.

---

## 13. Fallen der Umgebung (teuer bezahlt, generisch, gelten hier auch)

- ⚠️ **`ld: signal 7 / Bus error` beim Linken heißt: die Platte ist voll.** `target/debug/deps`
  staut Bevy-Binaries im dreistelligen GB-Bereich. **Erst `df -h /home`**, nicht den Code
  verdächtigen. Aufräumen: `cargo clean` bzw. gezielt `rm -rf target/debug/incremental`.
- ⚠️ **`undefined hidden symbol: anon.…llvm.…`** = kaputter Inkrement-Cache nach einem
  abgewürgten Build → `rm -rf target/debug/incremental`.
- **`cargo check 2>&1 | grep '^error'`** — **nicht** auf `.rs` filtern, das findet auch Warnungen
  und lässt dich einem roten Build nachjagen, der grün ist.
- **`pkill` NIE an den Anfang einer Befehlskette.** Läuft kein Prozess, gibt es Exit 1 und der
  Rest der Kette wird verschluckt — und du glaubst, ein Rückbau sei passiert.
  `pkill -f target/debug/defeated_by_titans` liefert auch mal Exit 144: normal.
- **Mehrere Agenten arbeiten evtl. parallel in diesem Repo.** Dateien ändern sich unter dir, der
  Build ist zwischendurch rot ohne dein Zutun. **Vor jedem Edit die Datei frisch lesen.**
  **NIEMALS `git stash` / `git checkout --` / `git clean -fdx`**, während jemand anders arbeitet.
- **Eine RON-Datei wird als GANZE Datei geschrieben — zwei Sitzungen mergen nicht.** Wer zuletzt
  speichert, gewinnt alles. **Nach jedem Schreiben in eine geteilte Datei per `grep` prüfen, dass
  dein Wert drinsteht.** Und: trag in `docs/STATUS.md` ein, an welcher Domäne du gerade arbeitest,
  damit ein anderer Agent eine andere nimmt.
- **Temporäre Hacks zum Screenshotten immer mit `// TEMP` markieren** und danach
  `grep -rn TEMP src/` prüfen. Ein vergessener Test-Hack ist ein Geist, den der Nächste jagt.

---

## 14. Abnahme dieses Auftrags

Am Ende der Sitzung will ich sehen:

1. **`cargo test`** — die Ausgabe, ungekürzt zusammengefasst (wie viele grün, welche rot und warum).
2. **Mindestens zwei Screenshots** in `docs/bilder/`: einer aus der Stadt (Blick beim Schwingen),
   einer mit einem Titanen im Bild.
3. **`docs/STATUS.md`**, in dem jede Sache eine der vier Stufen trägt — und **kein einziges ✅**,
   weil das der User setzt.
4. **`docs/ABNAHME.md`** mit der Liste dessen, worauf der User schauen soll.
5. **Die Modell-Tabelle** aus `cargo test --test modelle -- --ignored --nocapture`, und mindestens
   eine **`.blend`, die ich öffnen kann** — mit den Ankern (`nape`, `hit.min`/`hit.max`, …) schon
   an ihrem Platz, sodass Nachmodellieren heißt: Geometrie austauschen, speichern, starten.
   Dazu die Liste, welches Modell noch Platzhalter ist (⬜/🟨) und welches echt (🟧).
6. Einen ehrlichen Absatz: **was gebaut, aber nicht gesehen ist.** Eine grüne Metrik ist keine
   Abnahme. Was du nicht gesehen hast, markierst du als nicht gesehen — auch wenn der Code stimmt
   und die Tests grün sind.

**Die eine Regel über allen: erst messen, dann behaupten.** Fast jeder teure Fehler in einem
Projekt wie diesem ist eine Stelle, an der etwas Vernünftiges *erklärt* wurde, statt es in einer
Minute zu *messen* — und die Erklärung war das Problem.

---

## 15. Wie du das abarbeitest, wenn du **nicht allein** bist (Supervisor, Workflows, Fachexperten)

Diese Datei ist der **Auftrag**, nicht die Arbeitsreihenfolge eines einzelnen Kopfes. Es wird mit
Workflows und Subagenten gearbeitet — dafür gilt verbindlich:

### Supervisor & Fachexperten

**Ein Supervisor läuft dauerhaft im `/loop`** und triggert Workflows und Subagenten, die parallel am
Projekt arbeiten. **Er schreibt selbst nichts** — er plant, delegiert, prüft und integriert.

**Loop je Iteration:**

> Ist-Zustand → Hypothese + Abnahmekriterien → parallele Delegation → Ergebnisse sammeln → gegen
> Kriterien prüfen → integrieren und über die nächste Runde entscheiden.

**Abbruch** bei erfüllter DoD, bei erreichtem Limit, oder wenn **zweimal dieselbe Hypothese
gescheitert** ist (dann ist nicht die Ausführung falsch, sondern die Annahme → `docs/FRAGEN.md`).

**Parallelität:** **Jede Datei hat genau einen schreibberechtigten Agenten.** **Schnittstellen
werden vor der Parallelisierung fixiert** — erst der Vertrag (Components, Messages, Signaturen),
dann das Fan-out. Wer parallelisiert, bevor die Naht steht, integriert hinterher fünf Entwürfe
derselben Datei.

**Für jeden aus dem Projekt abgeleiteten Fachbereich wird ein Senior-Experte angelegt** (ODM-Physik,
Titanen-Verhalten, Rendering/3D-Pipeline, Daten/RON, Tooling & Test, Doku & Status …): er
**entscheidet eigenverantwortlich in seiner Domäne**, hält **alle** Projektrichtlinien ein und
**benennt Abweichungen explizit, statt sie zu umgehen** (eine benannte Abweichung ist eine
Entscheidung, eine stille ist ein Bug mit Anlauf).

**Vorgehen — verbindlich für alle, Supervisor wie Experten:**

> **falsifizierbare Hypothese → Prüfkriterium vorab festlegen → reproduzierbar messen → auswerten.**

**Jede Aussage mit Beleg. Annahmen als `ANNAHME:` markieren. Unsicherheit ausweisen. Nichts
erfinden. Bei Unklarheit messen oder eskalieren — nicht raten.** Das ist dieselbe Regel wie §9, nur
aus der Sicht des Delegierenden: **ein Ergebnis ohne vorab festgelegtes Prüfkriterium ist kein
Ergebnis, sondern eine Meinung.**

### Und praktisch:

**Erst seriell, dann breit.** Stufe 0–1 macht **ein** Kopf allein: Skelett, `Cargo.toml`,
`main.rs`, die Domänenordner mit leeren Plugins, das `docs/`-Skelett. Vorher gibt es nichts, worauf
mehrere gleichzeitig arbeiten könnten — ein Fan-out auf einen leeren Ordner erzeugt fünf
inkompatible Entwürfe derselben Datei.

**Diese Dateien fasst NUR der Hauptkopf an** (ein Schreiber pro Datei, §5 — sie werden als *ganze*
Datei geschrieben, zwei Agenten mergen nicht):

| Datei | warum |
|---|---|
| `Cargo.toml` | zwei Agenten, zwei Dependency-Listen, eine überlebt |
| `src/main.rs` | **die Plugin-Liste** ist die Naht des ganzen Projekts |
| `src/lib.rs` | dito für die Modulliste |
| `assets/data/*.ron` | RON wird als ganze Datei geschrieben — verlorene Zeilen sieht niemand |
| `docs/STATUS.md`, `docs/TODO.md` | **der Hauptkopf trägt ein**, Subagenten *melden* nur |

**Das geht gut parallel:** ein Agent pro **Domäne** (`odm/`, `titan/`, `world/`, `hud/` …), sobald
das Skelett steht · ein Agent pro `tools/blend/*.py` · ein Agent pro Doku-Datei · ein Agent, der in
der **installierten** Bevy-Doku nachsieht, wie eine API dieser Version wirklich heißt (§3).

**Was jeder Subagent im Auftrag stehen haben muss** — sonst liefert er Plausibles statt Richtiges:

1. **Welche Dateien ihm gehören** und welche er *nur lesen* darf.
2. **Welche Abschnitte er lesen soll** — z. B. „`prompts/init.md` §5 + §8 + §9, `docs/architektur.md`".
   Nicht „lies alles": ein Subagent mit 800 Zeilen Auftrag baut den halben Prompt nach.
3. **Die Belegpflicht (§9):** was er behauptet, misst er. Sein Rückgabewert enthält **Testnamen,
   Messwert und die Stufe (§8)** — nicht „habe es implementiert".
4. **Kein Fremdgebiet.** Was ihm auffällt, aber nicht gehört: nach `docs/FUNDE.md`, nicht still
   mitfixen.

**Prüfen gehört in den Workflow, nicht in die Hoffnung.** Nach jeder Findungs-/Baustufe eine
**unabhängige** Stufe, die versucht, die Behauptung zu **widerlegen** (anderer Agent, Auftrag:
„finde den Fall, in dem das kaputtgeht"). Eine Behauptung, die niemand angegriffen hat, ist 🟨.

**Nach jedem Fan-out: `cargo check 2>&1 | grep '^error'` und `cargo test`.** Fünf Agenten, die
einzeln grün waren, sind zusammen nicht automatisch grün — jeder hat nur seine Hälfte gesehen.

**Und quer über alles: `docs/STATUS.md` ist die einzige Wahrheit über den Fortschritt.** Ein
Workflow, der etwas gebaut hat, ist nicht fertig, solange die Zeile fehlt.

---

## 16. Die Ziellinie: übertragen, Repo anlegen, und **diese Datei löschen**

Wenn **alles aus dieser Datei abgearbeitet** ist — der Baum steht, die Modellkette läuft, der
Stufenplan ist durch, `docs/` stimmt — dann kommen drei Schritte, **in dieser Reihenfolge**.

### Schritt 1: Übertragen — diese Datei darf am Ende nichts Einzigartiges mehr enthalten

Alles hier Stehende hat ein dauerhaftes Zuhause. **Übertragen heißt umschreiben, nicht
hineinkopieren** — was aus dem Code oder der Doku schon hervorgeht, wird gekürzt; was ein
zukünftiger Agent braucht, wird ausformuliert.

| Aus `prompts/` | Wohin dauerhaft |
|---|---|
| Regeln, die **immer** gelten (RON, Domänen-Standalone, vier Stufen, Bug-Doktrin, „pro Sekunde") | `CLAUDE.md` — als **Index**, kurz, mit Zeigern |
| Bevy-Setup + die Engine-Fallen (§3) | `docs/lessons/bevy.md` |
| Achsen, Einheiten, Blickrichtung (§3) | `docs/konventionen.md` |
| Domänen, Plugin-Reihenfolge, Erlaubnisliste (§5) | `docs/architektur.md` |
| Die Modellkette (§7) | `docs/modelle.md` (+ Kommentarkopf in jedem `tools/blend/*.py`) |
| Die vier Stufen (§8) | Kopf von `docs/STATUS.md`, Kurzform in `CLAUDE.md` |
| Die Bug- und Sicherheitsdoktrin (§9) | Kopf von `docs/BUGS.md`, Kurzform in `CLAUDE.md` |
| Performance-Regeln (§10) | `docs/lessons/performance.md` |
| Flags, `--script`, Screenshots (§11) | `docs/lessons/workflow.md` + Tastenbelegung in `README.md` |
| Umgebungsfallen (§13) | `docs/lessons/umgebung.md` |
| Spielinhalt und Vorbild (§1) | `docs/gameplay/` (zusammen mit dem, was aus `gameplay/` kam) |
| Was bewusst später kommt (§1) | `docs/ROADMAP.md` |

**Prüfen, nicht glauben:** `grep -rn "prompts/init.md" . --exclude-dir=target` darf nur noch die eine
Zeile in `CLAUDE.md` finden, die sagt, dass der Initialprompt abgearbeitet und in der Historie
nachlesbar ist. Und lies `CLAUDE.md` einmal wie ein Fremder: *sagt sie mir in dreißig Sekunden,
wie das Projekt tickt und wo die Fallen liegen?* Wenn nein, ist der Schritt nicht fertig.

### Schritt 2: Vor dem Veröffentlichen aufräumen — **öffentlich heißt öffentlich**

Was einmal gepusht ist, ist indexierbar, auch nach dem Löschen. Der Check kommt **vor** dem Push:

- `.gitignore`: `target/`, `saves/`, `*.blend1`, `*.blend2`, temporäre Skripte, Logs.
  **`assets/3d/glb/` NICHT ignorieren** (§7 — das Spiel muss ohne Blender laufen).
- **Keine Zugangsdaten, Tokens, Keys, Pfade mit Klarnamen** — im Arbeitsbaum *und* in der
  Historie (`git log -p | grep -niE "token|secret|api[_-]?key|password"`).
- **Keine fremden Assets.** Jedes Modell, jede Textur, jeder Klang ist selbst gebaut (§1, §7).
- `cargo test` grün, `cargo build --release` grün, `grep -rn "TEMP" src/` leer.
- `README.md` liest sich für einen Fremden: was ist das, wie starte ich es, was ist fertig
  (mit der Stufen-Legende), welche Tasten. Dazu eine `LICENSE` — welche, fragst du in
  `docs/FRAGEN.md`, wenn der User nichts gesagt hat (bis dahin: keine Lizenzdatei erfinden).

### Schritt 3: Ein **öffentliches** GitHub-Repo anlegen

Der User hat das hier vorab erlaubt — es ist Teil des Auftrags, keine Nachfrage nötig. Aber erst
**nach** Schritt 1 und 2.

**Erst nachsehen, ob es das Repo schon gibt** (das Gerüst wurde beim Anlegen des Auftrags schon
einmal gepusht — dann wird nur noch gepusht, nicht neu angelegt):

```bash
gh auth status                      # nicht angemeldet? → den User bitten: `! gh auth login` tippen
git add -A && git commit -m "Aufsetzen abgeschlossen: Baum, Modellkette, Doku"

git remote -v                       # gibt es 'origin'?
git push -u origin main             # ja → nur pushen

gh repo create defeated-by-titans --public --source=. --remote=origin --push \
   --description "3D-Titanenkampfspiel in Bevy (Rust) — ODM-Gear, Nackentreffer, Koop-Einsätze"
                                    # nein → so anlegen
```

Danach **die URL im Chat nennen** und einmal `gh repo view --web` erwähnen. Wenn `gh` fehlt oder
nicht angemeldet ist: **nicht mit `git remote add` und einem geratenen URL improvisieren** — den
User um den Login bitten und den Rest fertig machen.

### Schritt 4: **Das Gerüst auflösen** — `prompts/`, `gameplay/` und `init.md` gehen weg

`prompts/` und `gameplay/` sind **Bootstrap-Gerüst**, kein Teil des fertigen Projekts. Wenn ihr
Inhalt in der echten Struktur angekommen ist (Schritt 1), wird das Gerüst abgebaut — **in dieser
Reihenfolge, jede Zeile ein eigener Commit, damit die Historie zeigt, wohin was gewandert ist:**

```bash
# 1. der Eingangskorb: Design nach docs/gameplay/, Arbeit nach docs/TODO.md, Zahlen in die RON,
#    Bilder nach docs/gameplay/bilder/ — DANN erst:
git rm -r gameplay/
git commit -m "gameplay/ aufgelöst: Design -> docs/gameplay/, Arbeit -> docs/TODO.md, Zahlen -> assets/data/"

# 2. die Prompts, einzeln, jede erst wenn abgearbeitet UND übertragen:
git rm prompts/init.md
git commit -m "Initialprompt abgearbeitet und übertragen (steht in der Historie)"
git rm -r prompts/ 2>/dev/null   # nur wenn leer
git rm init.md                    # der Starter im Wurzelverzeichnis, zuletzt
git commit -m "Bootstrap-Gerüst entfernt — ab hier fuehren README, CLAUDE.md und docs/"
git push
```

**Und dem User sagen, wo seine Wünsche ab jetzt hingehen:** der Eingangskorb ist nach der Auflösung
`docs/gameplay/` (Design, eine Datei pro Thema) plus `docs/TODO.md` (die offene Arbeit). Schreib
diese eine Zeile in die `README.md` — sonst legt er morgen eine Datei in einen Ordner, den es nicht
mehr gibt.

⚠️ **Nicht löschen, was du nicht übertragen hast**, und nichts, was der User gerade neu
hineingelegt hat (`ls -lt prompts/ gameplay/` vor dem Abbau). Im Zweifel: liegen lassen und
fragen. Die Historie ist ein Netz, aber nur, wenn der Commit auch gepusht wurde.

Und in `CLAUDE.md` bleibt **eine** Zeile stehen: *„Der Initialprompt ist abgearbeitet und
gelöscht; er ist in der Git-Historie nachlesbar (`git show <sha>:prompts/init.md`)."* Die Historie ist
das Sicherheitsnetz — deshalb wird gelöscht und nicht bloß umbenannt.

**Und die anderen Dateien in `prompts/`?** Genauso: **jede wird gelöscht, sobald sie abgearbeitet
und übertragen ist** — einzeln, mit einer Commit-Message, die sagt, wohin ihr Inhalt gewandert ist.
Was noch offen ist, bleibt liegen; dann bleibt auch der Ordner. Ist `prompts/` am Ende leer, kommt
er weg. **Nie eine Datei löschen, die du nicht vollständig abgearbeitet hast**, und nie eine, die
der User gerade neu hineingelegt hat.

⚠️ **Nicht früher löschen.** Solange irgendetwas nur hier steht, ist diese Datei das Gedächtnis
des Projekts. Erst wenn Schritt 1 belegt durch ist, ist sie überflüssig — und dann ist sie
Ballast, weil zwei Quellen für dieselbe Regel bedeuten, dass eine von ihnen bald lügt.

**Ab diesem Moment ist `gameplay/` die Arbeitsvorlage (§2), nicht mehr diese Datei.**
