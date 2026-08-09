# modelle — die Modellkette, und wie DU ein Modell austauschst

Stand: 2026-08-09 · Stufe: ⬜ (die Kette ist beschrieben, aber auf Maschine A fehlt Blender —
siehe unten und [`docs/umgebung.md`](umgebung.md))

## Die Kette

```
tools/blend/<name>.py  ──►  assets/3d/blend/<name>.blend  ──►  assets/3d/glb/<name>.glb  ──►  assets/data/art.ron
  Claude schreibt sie        DU oeffnest und fuellst aus         automatisch exportiert          der Schalter
```

**Warum ein Skript und nicht direkt eine `.blend`:** eine `.blend` ist ein Binaerklumpen — im
Git sieht niemand, was sich geaendert hat, und man kann sie nicht schreiben, ohne Blender zu
starten. Ein Skript ist ein Diff, ist reproduzierbar, und ist der Ort, an dem *Claudes*
Platzhalter lebt.

```bash
blender --background --factory-startup --python tools/blend/vanguard.py
```

---

## Fuer den User: so tausche ich ein Modell aus

1. `assets/3d/blend/<name>.blend` in Blender oeffnen.
2. **Geometrie austauschen.** Die Anker (die Empties, siehe unten) liegen schon an ihrem
   Platz — lass sie liegen oder schieb sie dorthin, wo sie an *deinem* Modell hingehoeren.
3. Speichern. **Mehr nicht.** Beim naechsten Spielstart sieht das Spiel, dass die `.blend`
   neuer ist als die `.glb`, exportiert sie neu und benutzt sie.
4. Wenn das Modell noch auf `nutzen: false` steht: in `assets/data/art.ron` **eine Zeile**
   auf `nutzen: true` setzen.

> ⚠️ **Eine `.blend`, die du angefasst hast, ist heilig.** Der Generator ueberschreibt sie
> **niemals**. Er prueft: Datei existiert und ist neuer als ihr Skript → *„vom User
> bearbeitet, nicht angefasst"* ins Log, fertig. Neu erzeugt wird nur, was fehlt; alles
> andere nur mit ausdruecklichem `--force <name>`. **Blender hat keine Historie** — wer
> diese Regel bricht, loescht Arbeit, die niemand wiederherstellen kann.

---

## Die Konventionen, die das Ersetzen erst billig machen

Sie stehen **hier** und als Kommentarkopf in **jedem** `tools/blend/*.py`:

| Regel | warum |
|---|---|
| **1 Blender-Einheit = 1 Meter** | Massstab wird im Modell gemacht, nicht per `scale` in der RON — das Feld ist eine Notbremse, kein Arbeitsmittel |
| **Origin zwischen den Fuessen** | sonst steht jedes Modell halb im Boden |
| **Blick nach −Z, aufrecht** | In Blender Z-oben modellieren, der Exporter dreht mit `export_yup=True`. **Nicht selbst rotieren**, sonst dreht es zweimal |
| **Farbe per Vertex-Farben, nicht per Textur** | Lowpoly braucht keine UV-Map, und Vertex-Farben ueberleben jedes Nachmodellieren |
| **Ein Objekt pro sinnvollem Teil**, benannt (`kopf`, `arm.r`, …) | daran haengen spaeter Amputation und Animation |

### Die Anker sind Empties mit festen Namen

Der Modellierer entscheidet damit **wo**, die RON **wie stark**:

| Empty | wofuer |
|---|---|
| `cortex` | **die Todeszone.** Ein Cortex-Treffer toetet, egal wie voll der Titan ist |
| `hit.min` / `hit.max` | die Hitbox als Quader |
| `hook.l` / `hook.r` | wo die Haken des Vector Gear ansetzen |
| `hand.l` / `hand.r` | Greifen, Werfen |
| `eye` | Blickrichtung, Blendung |

> **Fehlt ein Empty, ist die Zone ein Punkt** — und ein Cortex, der ein Punkt ist, fuehlt sich
> wie ein kaputtes Spiel an. `tests/modelle.rs` faellt deshalb um, wenn einem Modell mit
> `nutzen: true` ein geforderter Anker fehlt.

## Drei glTF-Fallen, die alle gleich aussehen

*(„mein Modell ist weiss / chrom / unsichtbar")*

1. **Bevy liest nur `COLOR_0`.** Hat ein Blender-Mesh **zwei** Color-Attribute, landet die
   gemalte Farbe in `COLOR_1` und das Modell kommt **weiss** an. Im `.py` sicherstellen, dass
   es nur eines gibt.
2. **Fehlender `metallicFactor` bedeutet 1.0**, also *voll metallisch* — ein Diffuse-Material
   ohne den Wert sieht im Spiel wie Chrom aus. Der Export setzt ihn auf `0.0`, wo er fehlt.
3. **Kameras und Lichter nicht mitexportieren** (`export_cameras=False`,
   `export_lights=False`). Sonst haengt in jedem Modell eine zweite Sonne, und die Szene wird
   von Modell zu Modell heller.

## Der Auto-Export

Beim Spielstart (in `data/`, vor allem anderen) prueft ein Schritt fuer jede `.blend`: **fehlt
die `.glb` oder ist sie aelter?** Dann exportieren, sonst nichts tun.

```bash
blender --background --factory-startup <datei>.blend \
  --python-expr "import bpy; bpy.ops.export_scene.gltf(filepath='assets/3d/glb/<name>.glb', \
     export_format='GLB', export_yup=True, export_apply=True, \
     export_cameras=False, export_lights=False)"
```

- **Kein Blender installiert?** → **einmal warnen**, die vorhandene `.glb` benutzen, **nicht
  abstuerzen**. Das Spiel muss auf einem Rechner ohne Blender laufen. **Genau das ist auf
  Maschine A der Fall** ([`docs/umgebung.md`](umgebung.md)).
- Flags: `--reexport` (alles neu bauen), `--no-export` (Startzeit sparen). Dazu ein
  eigenstaendiges Werkzeug `src/bin/export_modelle.rs`, damit es ohne Spielstart laeuft.
- **`assets/3d/glb/` wird mitcommittet** und steht **nicht** im `.gitignore` — sonst laeuft das
  Spiel auf keinem Rechner ohne Blender.

## Der Schalter: `assets/data/art.ron`

```ron
models: {
    "vanguard":     (blend: "vanguard",     nutzen: true,  scale: 1.0),
    "titan_husk":   (blend: "titan_husk",   nutzen: false, scale: 1.0),  // noch Platzhalter
}
```

`nutzen: false` ⇒ das Spiel baut den **prozeduralen Platzhalter** aus Bevy-Primitiven
(Kapsel/Box/Zylinder, eingefaerbt). `nutzen: true` ⇒ es laedt die `.glb`. **Beide Wege muessen
jederzeit funktionieren**, und beide benutzen dieselben Anker, dieselbe Hitbox und dieselbe
Skalierung — sonst ist das Umschalten kein Schalter, sondern ein Umbau.

**Kein Dateiname im Rust-Code.** Ein `asset_server.load("titan.glb")` mitten in einem System
ist ein Fehler; es gibt **eine** Stelle, die die Registratur liest (`data/`), alle anderen
fragen nach dem logischen Namen. `tools/normen.py` prueft das.

## Eigen und Extern bleiben getrennt

Der Plan ist: **der User ersetzt am Ende alles selbst, Stueck fuer Stueck.** Das funktioniert
genau dann, wenn jederzeit in **einem Befehl** beantwortbar ist: *was ist noch fremd, wo liegt
es, was soll es werden?*

1. **Fremdes liegt ausschliesslich unter `assets/extern/`.** Nie in `3d/blend`, `3d/glb`,
   `audio/sfx` — dort liegt Eigenes. Diese eine Trennung ist der ganze Trick.
2. **`assets/extern/HERKUNFT.md` listet jede Datei**: Dateiname · URL · Datum · Lizenz ·
   **welches Eigen-Asset sie spaeter wird**. Ohne Eintrag darf dort keine Datei liegen.
3. **In der Registratur traegt jedes fremde Asset `herkunft:`.** Damit ist die Ersetzungsliste
   ein `grep`, und `cargo test --test assets -- --ignored --nocapture` druckt den Bericht:
   *Asset · eigen/extern · Quelle · ersetzt durch*.
4. **Das oeffentliche Repo bekommt die Fremddateien nicht.** `assets/extern/` steht im
   `.gitignore`; mit kommt `HERKUNFT.md` und `tools/hole_extern.sh`, das alles wiederbeschafft.
5. **Stilbruch ist ein Bug, auch bei einem Platzhalter.** Ein hochdetailliertes Fremdmodell
   neben Low-Poly-Eigenbau verfaelscht das Urteil ueber Bewegung und Lesbarkeit.

## Der Stand

`cargo test --test modelle -- --ignored --nocapture` druckt die Tabelle: *Modell · `.blend`
da? · `.glb` aktuell? · bemalt? · Anker vollstaendig? · in RON verdrahtet?* — genau das, was
ein anderer Agent in zehn Sekunden lesen will.

**Die vier Stufen gelten fuer Modelle genauso** (§8): ein Primitiven-Platzhalter ist ⬜ oder
🟨 — nie mehr, egal wie gut er sich einbaut. Erst ein echtes Modell, das jemand **gesehen**
hat, ist 🟧.

Verwandt: [`docs/konventionen.md`](konventionen.md) · [`docs/umgebung.md`](umgebung.md) ·
[`docs/STATUS.md`](STATUS.md)
