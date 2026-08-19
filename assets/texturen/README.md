# texturen — Atlas-Texturen als zweiter, gleichwertiger Weg neben den Vertexfarben

Stand: 2026-08-09 · Stufe: 🟨 (gebaut, im Kontaktbogen und im Kachelverbund angesehen und
nachgebessert, `tools/atlas.py --pruefen` gruen — aber **noch auf keinem Modell im Spiel
gesehen**. Ohne Bild aus dem laufenden Spiel bleibt das 🟨.)

## Was hier liegt

```
assets/texturen/
  README.md                    diese Datei
  index.html                   Kontaktbogen: jedes PNG gross, Legende, Kacheln im Verbund
  quelle/<TEX-ID>.svg          die handgepflegte Quelle — hier wird geaendert
  <TEX-ID>.png                 gerastert; das laedt das Spiel
  <TEX-ID>.emissiv.png         nur wo etwas leuchtet: schwarz ausser den Signalfeldern
  <TEX-ID>.felder.ron          Feldname -> UV-Rechteck, Farbe, Art; erzeugt
```

**Das PNG wird nie von Hand angefasst.** Es ist Ausgabe. Wer im Bildeditor am PNG malt,
verliert die Aenderung beim naechsten Lauf und hat nichts im Diff stehen.

### Die beiden grossen Atlanten dieser Runde

| Atlas | Quelle | Ausgabe | traegt |
|---|---|---|---|
| **`TEX-ENV-ATLAS`** 512 | `quelle/TEX-ENV-ATLAS.svg` | `TEX-ENV-ATLAS.png`, `TEX-ENV-ATLAS.felder.ron` | alles Gebaute und Gewachsene, A-080…A-122 und A-145…A-153 — **ein Material fuer die ganze Welt** |
| **`TEX-PROP-01`** 512 | `quelle/TEX-PROP-01.svg` | `TEX-PROP-01.png`, `TEX-PROP-01.felder.ron`, `TEX-PROP-01.emissiv.png` | Requisiten, Karren, Truhen, Banner, Nachschubstation |

`TEX-ENV-ATLAS` fuehrt 65 Feldnamen auf 64 Zellen: `fensterglas` und `stein_tief` teilen sich
eine Zelle (siehe *Farbe*). `TEX-PROP-01` traegt als einziger der beiden eine Emissivkarte —
`station_licht_bernstein` muss laut `A-130` aus 56 m identifizierbar sein, und eine Farbe
allein leistet das im Abendlicht nicht.

## Warum das der *alternative* Weg ist

[`docs/modelle.md`](../../docs/modelle.md) legt **Vertexfarben** fest: „Lowpoly braucht keine
UV-Map, und Vertex-Farben ueberleben jedes Nachmodellieren." Das gilt weiter. Dieser Ordner
baut den **zweiten Weg** daneben — einen Palettenatlas —, und **beide bleiben gueltig**:

| | Vertexfarben | Atlas-Textur |
|---|---|---|
| **stark bei** | Koerpern, Ausruestung, allem, was oft nachmodelliert wird | Waenden, Daechern, Boden, Rinde — allem mit **Struktur** |
| **kostet** | eine Farbe je Ecke, kein UV | eine UV-Map je Modell |
| **kann nicht** | Fugen, Reihen, Maserung zeigen ohne Geometrie | eine Form aendern, ohne die UV neu zu legen |

Der Grund, warum es den Atlas ueberhaupt braucht: eine 8-m-Hauswand aus zwei Dreiecken hat
vier Ecken. Mit Vertexfarben ist sie eine Flaeche. Mit `kachel_mauerwerk` hat sie eine
**Steinreihe von 0,60 m**, und daran misst der Spieler die 120 m Mauer ab.

**Welcher Weg fuer welches Asset gilt, entscheidet die Registratur** (`assets/data/art.ron`),
nicht dieser Ordner und nicht der, der gerade ein Modell baut. Ein Modellteil traegt entweder
eine Palettenfarbe oder ein `feld:` aus dem Atlas seines Modells — nie beides.

## Der Atlasaufbau

Jeder Atlas ist ein **Palettenatlas**: grosse flache Farbfelder plus einige Kacheln mit
Struktur. Kein PBR, kein Fotomaterial, kein Rauschen, kein Alpha.

```
512 x 512 = 16 x 16 Zellen a 32 px

 Zeile  0..3    y   0..127    64 Farbfelder a 32 x 32   flach, in der Mitte sampeln
 Zeile  4..15   y 128..511    12 Kacheln    a 128 x 128 nahtlos, REPEAT
```

- **Ein Farbfeld wird in der Mitte gesampelt.** Die inneren 16 x 16 px sind sicher; so blutet
  keine Mipmap-Stufe ins Nachbarfeld.
- **Eine Kachel liegt immer auf einer 128-px-Grenze** und ist nahtlos: was rechts hinauslaeuft,
  kommt links wieder herein. Das ist in der Quelle **konstruiert**, nicht gehofft.
- **Ein Feldname taucht an genau einer Stelle auf und hat genau eine Farbe.** Die einzige
  Ausnahme steht als `ALIAS` in `tools/atlas.py`, mit Begruendung in derselben Zeile.

## Texeldichte: 26,67 px/m — und warum genau die

**128 px Kachel = 4,8 m Welt.** Diese eine Zahl ist nicht gerundet worden, weil alles andere
daraus faellt:

| Bauteil | Welt | Textur |
|---|---|---|
| **Steinreihe** | **0,60 m** | **16 px** |
| Ziegel | 0,30 x 0,15 m | 8 x 4 px |
| Dachreihe, Diele, Kopfstein | 0,30 m | 8 px |
| Rindenriefe | 0,15 m | 4 px |
| Fachwerkbalken | 0,19 m | 5 px |

Die **Steinreihe 0,60 m ist die Skalenleiter** ([`docs/skala.md`](../../docs/skala.md)). Eine
120-m-Mauer ist 200 Steinreihen hoch — das ist die einzige Art, wie ein Spieler 120 m ueberhaupt
begreift, denn eine Flaeche ohne Mass ist nur gross. Waere die Reihe 15,7 px hoch, saesse die
Fuge nicht auf dem Texel, und aus der Leiter wuerde bei jeder Mipmap-Stufe Matsch.

`kachel_mauerwerk` und `kachel_mauer_bastion` halten die 16 px verbindlich.
`tools/atlas.py --pruefen` prueft das Raster, nicht die Reihe — die Reihe prueft, wer hinsieht.

## Farbe

Ausschliesslich [`tools/palette.py`](../../tools/palette.py), dazu **Stufen von 6 %** heller
oder dunkler (`stufe()` in `tools/atlas.py`). Sonst nichts. `--pruefen` faellt um, sobald eine
Fuellung weder in der Palette steht noch eine solche Stufe daraus ist.

**Die drei Signalfarben sind Gameplay-Sprache** ([`docs/konventionen.md`](../../docs/konventionen.md) §3)
und stehen je Atlas nur an den Feldern, die `VOKABULAR` dafuer vorsieht:

| Atlas | Zyan | Bernstein | Karmin |
|---|---|---|---|
| `TEX-ENV-ATLAS` | `anker_zyan` | `ziel_bernstein` | `gefahr_karmin` |
| `TEX-PROP-01` | — | `station_licht_bernstein`, `ring_glut_bernstein` | — |

`fensterglas` ist darum **dunkel** und teilt sich die Zelle mit `stein_tief`. Ein leuchtendes
Fenster wuerde gegen die Ablesbarkeit der Ankerpunkte arbeiten — und das ist kein Detail,
sondern der Grund, warum ein Spieler bei voller Geschwindigkeit noch sieht, wohin er haken kann.

## Wie man einen Atlas aendert

1. `assets/texturen/quelle/<TEX-ID>.svg` **im Editor** oeffnen. Alles darin sind `<rect>` mit
   ganzzahligen Koordinaten und Kommentaren; ein Farbwechsel ist eine Zeile.
2. Bauen:
   ```bash
   python3 tools/atlas.py --nur TEX-ENV-ATLAS   # ein Atlas
   python3 tools/atlas.py --alle                # alle, plus Kontaktbogen
   ```
3. Pruefen — Exit != 0 bei jedem Verstoss:
   ```bash
   python3 tools/atlas.py --pruefen
   ```
4. **Hinsehen.** `assets/texturen/index.html` im Browser: jedes PNG gross, die Feldnamen als
   Legende, jede Kachel 4 x 2 gekachelt (die Naht liegt dann in der Bildmitte) und eine
   Meterleiste. Eine Kachel luegt einzeln nie, im Verbund schon.

**Nie das PNG von Hand.** Nie eine Farbe erfinden — fehlt eine, geht sie nach
[`docs/FUNDE.md`](../../docs/FUNDE.md), und bis dahin gilt die naechstliegende.

## Was `--pruefen` prueft

Feldname doppelt · Feld nicht auf dem 32-px-Raster · Kachel nicht auf dem 128-px-Raster ·
Farbe weder Palette noch 6-%-Stufe · Feldname nicht im Vokabular · Pflichtfeld fehlt ·
Signalfarbe an einem Feld, das keine tragen darf · Emissivflaeche ohne Signalfeld ·
PNG fehlt, hat die falsche Groesse oder einen Alphakanal · **Kachel reisst an der Naht auf**
(Nahtsprung geteilt durch den groessten Sprung im Inneren, erlaubt bis 1,05).

Was es **nicht** prueft: ob es gut aussieht. Dafuer gibt es den Kontaktbogen und Augen.

## Ein neuer Atlas

SVG nach `quelle/` legen, dann seine Feldnamen aus `FELDER.md` als Eintrag in `VOKABULAR`
(`tools/atlas.py`) nachtragen. Ohne diesen Eintrag baut das Werkzeug den Atlas zwar, meldet
aber „kein Vokabular hinterlegt" und laesst Feldnamen, Pflichtfelder und Signalzuteilung
**ungeprueft** — dann haelt die Naht zu den Modellen nur noch aus Hoeflichkeit.

Verwandt: [`docs/skala.md`](../../docs/skala.md) ·
[`docs/modelle.md`](../../docs/modelle.md) ·
[`docs/konventionen.md`](../../docs/konventionen.md) ·
[`tools/palette.py`](../../tools/palette.py)
