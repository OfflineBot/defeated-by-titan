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

---

## Die Groessentabelle — vom User vorgegeben, 2026-08-09

**Das hier ist die Wahrheit ueber Groessen.** Sie schlaegt jede Ableitung: wo frueher aus dem
Backlog umgerechnet wurde (0,28 m je Backlog-Einheit, [`docs/FRAGEN.md`](FRAGEN.md) Q-002) und
das Ergebnis dieser Tabelle widerspricht, gilt **diese Tabelle**. Die Umrechnung bleibt nur
fuer alles, wozu der User nichts gesagt hat.

> Die Tabelle steht **maschinenlesbar** in `assets/data/massstab.ron` — das hier ist die
> Fassung fuer den Modellierer. Seit 2026-08-09 ist das kein Versprechen mehr, sondern ein
> Waechter: `tests/data.rs::t005_die_groessentabelle_in_der_doku_zeigt_dieselben_zahlen` liest
> **diese Datei** und faellt um, sobald eine Zahl hier von der RON abweicht oder ein neues
> Bauwerk in der RON hier fehlt. Deshalb stehen die Zahlen **exakt so wie in der RON** — ohne
> haengende Null (`1,8 m`, nicht `1,80 m`); sonst prueft der Waechter nichts.

| Objekt | Hoehe | Notiz |
|---|---|---|
| **Referenz** | | |
| Mensch | 1,8 m | Kapsel **exakt** pruefen |
| Tuer | 2,1 m | |
| Strassenbreite | 6–8 m | eng halten |
| **Architektur (×1,0)** | | |
| Kleines Haus (1 Stock) | 4,5 m | Traufe 3 m |
| Stadthaus (2 Stock) | 8 m | Traufe 6 m |
| Grosses Haus (3 Stock) | 11,5 m | Obergrenze der Wohnbebauung |
| Baum | 12 m | Vordergrund-Staffelung |
| Wachturm auf Mauer | 12 m | |
| Kirche / Glockenturm | 35 m | Sonderbau, kein Rasterhaus |
| **Titanen (×1,4)** | | Cortex bei ~89 % |
| Kleiner Titan | 4,2 m | Cortex 3,7 m |
| Mittlerer Titan | 10 m | Cortex 8,9 m |
| Mittlerer Titan (gross) | 14 m | Cortex 12,5 m |
| Grosser Titan | 21 m | Cortex 18,7 m |
| Abnormaler / Boss | 28 m | Cortex 24,9 m — Zeilentitel wie beim User, [Q-020](FRAGEN.md) |
| Kopfgroesse Titan | 1/9 – 1/10 der Hoehe | Mensch = 1/7,5 |
| **Mauern (×2,4)** | | |
| Mauerhoehe | 120 m | |
| Mauerdicke oben | 28 m | |
| Mauerdicke Basis | 45 m | angeschraegt |
| Zwischenplattform | 60 m | Zwischenstopp beim Aufstieg |
| Steinreihe | 0,6 m | Skalenleiter, sichtbare Fugen |
| Horizontale Baenderung | 15 m | alle 15 m ein Band |
| **Boss** | | |
| The Ashwalker | 150 m | 30 m ueber der Mauer |
| **Kamera / Vector Gear** | | |
| Kamerahoehe | 1,6 m | |
| Sichtfeld Bodenkampf | 55–65 Grad | groesster Hebel — senkrecht oder waagerecht? [Q-021](FRAGEN.md) |
| Ankerreichweite | 90 m | |
| Geschwindigkeit | ×1,5 | vs. Standard — Bezug offen, [Q-018](FRAGEN.md) |

*Der User hat die letzten vier Zeilen mit dem Referenzbegriff des Vorbilds beschriftet; hier
stehen sie mit den Projektbegriffen aus [`docs/konventionen.md`](konventionen.md) §2.*

> ⚠️ **Eine Zeile ist uebersetzt, nicht gelesen.** Der User schreibt „Abnormaler / Boss —
> 28 m". Im Projektvokabular ist „Abnormal" ein **Titan-Typ** und heisst **Errant**
> ([`docs/konventionen.md`](konventionen.md) §2) — die Zeile koennte also heissen „der Errant
> ist 28 m hoch" statt „es gibt eine Groessenklasse namens Boss". Hier gilt die zweite
> Lesart, `assets/data/titan.ron` laesst den Errant bei 10 m, und die Klasse `boss` hat
> keinen Vertreter. **Das ist eine Annahme, keine Uebersetzung** — sie steht als
> [Q-020](FRAGEN.md) und wird mit einem Satz des Users zurueckgenommen.

### Die drei Massstaebe — und warum niemand sie „korrigiert"

Architektur ×1,0, Titanen ×1,4, Mauern ×2,4. **Die Welt ist bewusst nicht einheitlich
skaliert.** Ein Haus ist so gross, wie ein Haus ist; ein Titan ist ueberzeichnet; eine Mauer
ist monumental. Der Mensch ist klein, die Bedrohung unverhaeltnismaessig, die Mauer ein
Horizont — das ist die Bildsprache, nicht ein Rechenfehler.

Wer die drei Faktoren spaeter angleicht, weil ihm eine Zahl unrealistisch vorkommt, macht das
Spiel technisch sauberer und kuenstlerisch tot — und merkt es erst, wenn alles beliebig
aussieht. `tests/data.rs::t005_die_massstabsfaktoren_bleiben_ungleich` wird deshalb rot, sobald
jemand es versucht.

**Die Stadt ist flach, und das ist Absicht.** Wohnbebauung geht von 4,5 m bis 11,5 m. Die
Vertikale kommt aus Mauer (120 m), Kirche (35 m), Wachturm (12 m) und Baeumen (12 m). Ein
Ziegeldach-Meer, aus dem einzelne Bauwerke herausragen — keine Skyline.

**Und genau deshalb sind die vier Sonderbauten keine Deko.** Aus 11,5 m Dachhoehe und 3,0 m
Mindestseil (`assets/data/game.ron: vector.seil_min_m`) folgt eine **Ankerdecke von 14,5 m** —
darueber haelt an einem Wohnhaus kein Seil mehr. Der Cortex sitzt beim mittleren grossen Titan
auf 12,5 m, beim grossen auf 18,7 m, beim Boss auf 24,9 m. Eine Stadt ohne Kirche und Turm
laesst also drei von fuenf Groessenklassen nur noch ballistisch angreifen: hinspringen, treffen
oder fallen. Wer ein Modell fuer Kirche, Wachturm oder Baum baut, baut **Spielmechanik**, nicht
Kulisse. Die Rechnung steht als [Q-022](FRAGEN.md), und `tests/data.rs` haelt fest, dass die
Startkarte wirklich einen hakbaren Sonderbau traegt.

### Kopf und Cortex: die zwei Regeln, an denen die Lesbarkeit haengt

- **Der Kopf ist 1/9 bis 1/10 der Koerperhoehe** — beim Menschen ist er 1/7,5, also relativ
  **groesser**. Genau daran liest das Auge „das Ding ist riesig" statt „das Ding ist nah". Ein
  zu grosser Kopf laesst jeden Titanen wie eine Puppe aussehen, egal wie viele Meter im
  Datenblatt stehen.
- **Der Cortex sitzt bei rund 89 % der Koerperhoehe.** Das ist keine Deko, das ist die
  **einzige toedliche Trefferzone** (`F-030`) — beim 21-m-Titanen also auf 18,7 m. Das
  `cortex`-Empty im Modell gehoert dorthin, nicht „ungefaehr oben".
  **Massgeblich ist die Meterangabe, nicht die Prozentzahl:** die fuenf Cortexhoehen stehen
  einzeln in `assets/data/massstab.ron` (`titan.klassen[...].cortex_m`), weil der User sie
  einzeln genannt hat. Die 89 % sind die *Regel*, an der geprueft wird, ob eine der fuenf
  wegdriftet — aus ihr gerechnet laege der kleine Titan 4 cm daneben.
- **Der Cortex ist kleiner als der Kopf.** Klingt selbstverstaendlich, war es nicht: der
  kleine Titan trug bis 2026-08-09 eine Trefferzone von 0,80 m Durchmesser an einem Kopf von
  0,42–0,47 m. `tests/data.rs` haelt jetzt `2 × cortex_radius_m ≤ Kopfhoehe` fest.

Beide zusammen entscheiden, ob das Kriterium der Bibel haelt: **der Cortex muss auf Distanz
erkennbar sein.** Auf 100 m ist ein Kopf von 2,1 m (21-m-Titan, 1/10) rund 1,2 Grad breit —
sichtbar. Die Trefferzone selbst ist kleiner; ob ihr Radius mit der Groesse mitwachsen soll,
ist offen und steht als [Q-019](FRAGEN.md) fest.

> **Welche Distanz gilt eigentlich?** `docs/features.ron` `F-030` fordert woertlich „Cortex ist
> aus 100 **Backlog-Einheiten** Entfernung erkennbar" — das sind 28 m (Faktor 0,28), nicht
> 100 m. Der Unterschied ist Faktor 3,6 im Pixelmass und entscheidet Q-019 mit. Gemessen bei
> 1920 × 1080 und dem neuen Sichtfeld: der Cortex des Husk (1,10 m) ist auf 28 m **36,7 px**
> breit, auf 100 m **10,3 px**. Der Wechsel von 90 auf 60 Grad hat diese Zahl **fast
> verdoppelt** — das engere Bild ist fuer `F-030` die bessere Zahl, nicht die schlechtere.

### Die Skalenleiter der Mauer: 0,6 m und 15 m

Eine 120 m hohe Wand ohne Struktur ist **eine graue Flaeche**. Das Auge hat nichts, woran es
Groesse abliest, und aus der Naehe sieht dieselbe Wand aus wie eine 12 m hohe. Genau dagegen
sind die beiden Zahlen da:

- **Steinreihe 0,6 m, mit sichtbaren Fugen.** Eine Reihe ist ein Drittel eines Menschen — wer
  auf der Mauer steht, sieht neben sich drei Reihen und weiss sofort, wie hoch er ist. Die
  Fugen muessen **sichtbar** sein: eine glatte Wand mit Steintextur leistet das nicht.
- **Horizontale Baenderung alle 15 m.** Die grobe Leiter: acht Baender bis zur Krone, das
  vierte auf Hoehe der Zwischenplattform.

**Das sind keine Deko-Details, die man bei der Performance-Optimierung zuerst streicht** — sie
sind der Grund, warum die Mauer gross *aussieht*. `tests/data.rs::t005_die_skalenleiter_der_mauer_bleibt_lesbar`
haelt beide Zahlen fest.

### Wo die spielwirksamen Zahlen liegen

**In `assets/data/*.ron`, nirgends sonst** — damit niemand die Tabelle doppelt pflegt:

| Was | Datei |
|---|---|
| Die Tabelle selbst, als Daten | `assets/data/massstab.ron` |
| Spielerkapsel, Kamerahoehe, Sichtfeld, Ankerreichweite | `assets/data/game.ron` |
| Groessenklasse je Titanart (**keine Hoehe je Art**) | `assets/data/titan.ron` |
| Gassenbreite, Hoehenfenster, gesetzte Sonderbauten | `assets/data/maps.ron` |

Die letzten drei **spiegeln** nur, was in `massstab.ron` steht; `tests/data.rs` faellt um,
sobald eine von ihnen abweicht. Diese Datei hier ist die **Erklaerung** fuer den Modellierer —
wer eine Zahl aendern will, aendert sie in der RON und schreibt hier den Grund dazu. Und weil
„gemeinsam aendern" eine Bitte ist und kein Werkzeug, prueft
`t005_die_groessentabelle_in_der_doku_zeigt_dieselben_zahlen` die Tabelle oben Zelle fuer Zelle
gegen die RON.

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
