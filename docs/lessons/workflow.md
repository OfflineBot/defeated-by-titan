# workflow — du kannst nicht klicken, also baust du dir Start-Flags, einen Skript-Fahrer und ein Overlay, bevor du ein Feature baust

Stand: 2026-08-09 · Stufe: 🟨 (aus `prompts/init.md` §12 aufgeschrieben — keins der
beschriebenen Werkzeuge ist in diesem Repo bisher gebaut, also nichts davon gelaufen)

## Der Fall

Alles ist gebaut, nichts ist gesehen. Jedes Feature liegt hinter Maus und Tastatur, und am
Keyboard sitzt niemand. Ein Hauptmenü ist für einen Agenten eine Wand ohne Tür: der Build
läuft, das Fenster steht da, und es passiert nie irgendwas.

Die Quelle nennt das den Punkt, an dem solche Projekte scheitern. Die Folgerung ist
unbequem: **die Prüfinfrastruktur ist Teil von Stufe 1**, nicht ein „wenn Zeit ist".
`prompts/init.md` §13 listet für Stufe 1 ausdrücklich `--sandbox`, `--script`, F3-Overlay,
einen Screenshot und `--headless` neben der Kamera und der Schwerkraft — im selben Kasten,
nicht dahinter.

## a) Start-Flags, die am Menü vorbeigehen

| Flag | wofür | was es kostet, wenn es fehlt |
|---|---|---|
| `--mission tutorial` | direkt in einen Einsatz, kein Menü | jeder Lauf endet im Hauptmenü |
| `--sandbox` | leeres Feld, ein Titan, unendlich Gas — zum Anschauen | kein Ort, an dem man etwas isoliert sieht |
| `--novsync` | zum Messen | unter Vsync misst du sechsmal 16,6 ms Deckel (§11) |
| `--lag 200` | 200 ms simulierte Latenz (Bibel T-019) | jedes Bewegungsfeature wird nur lokal geprüft |
| `--script <datei>` | das Spiel spielen, ohne zu tippen | siehe oben: niemand tippt |
| `--headless` | kein Fenster, fester Tick, N Ticks, Exit-Code | auf Maschine A ist gar nichts prüfbar |

```bash
cargo run -- --mission tutorial
cargo run -- --sandbox
cargo run -- --novsync
cargo run -- --lag 200
cargo run -- --script <datei>
```

`--headless` steht nicht in §12, sondern in §13/§14: `primary_window: None`, fester Tick,
läuft N Ticks und **beendet sich mit einem Exit-Code**, der sagt, ob alle `assert` gehalten
haben. Ohne dieses Flag ist `--script` auf einer Maschine ohne Grafiksitzung wertlos.

## b) `--script` — der Fahrer

Eine Textdatei, **eine Anweisung pro Zeile**. Sie schreibt in **dieselben Eingaben, die ein
Mensch auslöst** (`ButtonInput<KeyCode>`, `ButtonInput<MouseButton>`, dazu ein
„so-tun-als"-Blickvektor). **Kein zweiter, falscher Weg zu spielen** — jedes System dahinter
ist das echte. Wer einen Nebeneingang baut, testet den Nebeneingang.

```text
spawn titan normal 20 0 -40   # Typ und Ort in Metern
look 0 -10                    # Blickrichtung in Grad (yaw, pitch)
key Space 0.3                 # Taste 0,3 s halten
hook left                     # Haken raus
wait 1.2                      # Commands sind verzoegert — sonst fotografierst du ein leeres Feld
mark eingehakt                # eine Zeile ins Log, an der man einen Screenshot ausrichtet
assert speed > 25             # das Skript darf selbst urteilen
```

| Anweisung | Einheit / Bedeutung | die Falle dahinter |
|---|---|---|
| `spawn` | Ort in **Metern** | — |
| `look` | yaw, pitch in **Grad** | — |
| `key` | Haltedauer in **Sekunden** (`0.3`) | — |
| `wait` | Sekunden | **Commands sind verzögert** — ohne `wait` fotografierst du ein leeres Feld |
| `mark` | Logzeile als Anker | ohne Anker weiss niemand, wann der Screenshot fiel |
| `assert` | Bedingung, die halten muss | ohne `assert` ist es eine Demo, kein Test |
| `warp` | Koordinate anspringen (§12c) | — |

**Warum `assert` der ganze Trick ist:** damit wird aus einer Fahrt ein Test. Bewegungsgefühl
ist genau die Sorte Sache, die kein Unit-Test greift — „hakt ein und ist danach schneller
als 25" lässt sich nicht als reine Funktion prüfen, als Skriptlauf schon. Fällt der `assert`
um, fällt der Exit-Code um, und das gilt auf jeder Maschine und eines Tages in einem CI.

## c) F3-Overlay — jede Meldung nachstellbar

Ins Bild gehören laut Quelle: Position, Blickrichtung, Geschwindigkeit, Gas, Hakenzustand,
Bildzeit. Dazu `warp x y z` + `look` im Skript. Damit schickt der User eine Koordinate und
du stehst genau dort. Die Quelle bewertet das als **mehr wert als jedes Bug-Formular**.

## d) Screenshots — **nur Maschine B**, hier nicht

⚠️ **Auf dieser Maschine (`debian`, Maschine A) gibt es keine Grafiksitzung und kein
`niri`.** Gemessen in [`docs/umgebung.md`](../umgebung.md): `WAYLAND_DISPLAY` und `DISPLAY`
beide leer, `command -v niri` leer. Der folgende Abschnitt ist **nicht** ausführbar hier; er
gilt für `offlinebot` (Maschine B, niri/Wayland).

```bash
setsid nohup cargo run -- --sandbox > /tmp/dbt.log 2>&1 < /dev/null & disown
sleep 20   # der erste Build dauert
ID=$(niri msg --json windows | python3 -c "import sys,json;print([w['id'] for w in json.load(sys.stdin) if (w.get('title') or '')=='Defeated by Titan'][0])")
niri msg action focus-window --id $ID   # SONST drosselt der Compositor auf ~5 fps
sleep 2
niri msg action screenshot-window --id $ID
```

Die Bilder landen in `~/Pictures/Screenshots/`. **Kopieren nach `docs/bilder/` und in
`STATUS.md` verlinken** — ein Screenshot, den niemand mehr findet, ist kein Beleg.

| Falle | woran man sie erkennt | was man stattdessen tut |
|---|---|---|
| unfokussiertes Fenster | ~5 fps, sieht **exakt** wie eine Regression aus | vor **jeder** fps-Messung `focus-window`, dann messen |
| mehrere Instanzen | — | prüfen, dass nur **eine** Instanz läuft — sonst fotografierst du alten Code |
| zu früh geschossen | leeres Feld, obwohl `spawn` im Skript steht | `sleep 20` nach dem Start, `wait` im Skript, `mark` als Anker |

## Der Fall „gar keine Grafiksitzung"

Kein `WAYLAND_DISPLAY`, kein `DISPLAY` → `cargo run` **panikt sofort**. Dann gibt es **kein
Bild**. Dann:

- Die Sache bleibt **🟨**, mit dem Vermerk *„Logik getestet, Pixel ungesehen — Maschine A"* (§14).
- Du bittest den User draufzuschauen.
- **Nicht aufrunden.** Kein „sieht sicher richtig aus". 🟧 setzt nur, wer etwas gesehen hat;
  ✅ setzt nur der User.

Das ist keine Blockade: `cargo test`, `--headless`-Skriptläufe mit `assert`,
`blender --background` und Doku laufen hier vollständig (§14). Nur der Beweis per Pixel
fehlt — und der wird dann eben ausgewiesen und nicht behauptet.

**Lücke:** §14 lässt Offscreen-Rendering in eine PNG als Ausnahme zu, aber **erst wenn
bewiesen ist**, dass es auf dieser Maschine wirklich ein Bild liefert. Bisher nicht bewiesen
(siehe [`docs/FRAGEN.md`](../FRAGEN.md) Q-009).

## e) Recherche und Assets — erlaubt, mit drei Bedingungen

Ausdrücklich erlaubt und erwünscht: YouTube (Bewegungs- und Level-Design ansehen —
Ankerdichte, Dachhöhen, Gassenbreiten), Google/Bilder-Suche für Referenzen, Fachartikel zu
Seilphysik/Netcode/Audio-Synthese, und die **Doku der installierten Bevy-Version** (laut §3
die wichtigste Quelle von allen). Skripte sind zulässig: `yt-dlp` für Untertitel und
Beschreibungen, `curl`, ein kleines Parse-Skript.

**Assets herunterladen ist erlaubt — es ist ein Prototyp.** Modelle, Klänge,
Musik-Platzhalter. Der User ersetzt später ohnehin alles selbst (§7); bis dahin ist ein
guter Prototyp mehr wert als eigene Polygone. Als Startpunkte nennt die Quelle (nicht als
Vorschrift): Kenney, Poly Pizza, OpenGameArt, Quaternius, Sketchfab mit CC-Filter, Freesound
(CC0), Pixabay.

| Regel | Warum, und was sonst passiert |
|---|---|
| Alles Fremde nach `assets/extern/` + Zeile in `HERKUNFT.md` + `herkunft:` in der Registratur (§7) | ohne diese drei ist es ein **Zombie** (§10) — der User findet es später nicht, um es zu ersetzen |
| `assets/extern/` **nicht** ins öffentliche Repo | es ist ignoriert; `tools/hole_extern.sh` beschafft es wieder (§7) |
| Zahlen und Erkenntnisse mit Quelle nach `docs/gameplay/referenzen.md` | *„Gassen sind 8–12 m breit, damit ein Haken beide Seiten erreicht"* — **eine Zahl ohne Herkunft ist eine Behauptung** (§9) |
| Referenzbilder nach `gameplay/bilder/` bzw. `docs/gameplay/referenzen/`, mit URL und Datum | sonst ist das Bild in einer Woche ein anonymes JPG |
| Bei Widerspruch gewinnt die Wirklichkeit | ein Blogpost über Bevy-Versionen ist keine Quelle, die installierte Doku ist eine |

Das Wertvollste an einer Recherche ist selten die Datei, sondern die **Zahl** — und die ist
nur so viel wert wie die Herkunftszeile daneben.

## Was das für die Reihenfolge heisst

| zuerst | dann |
|---|---|
| `--sandbox`, `--script`, `--headless`, F3-Overlay | das erste Feature, das man damit anschauen will |
| `mark` + `assert` in jedem Skript | ein Screenshot, der zu einer Logzeile passt |
| `hostname` (§14) | die Entscheidung, ob heute überhaupt ein Bild möglich ist |

**Lücke:** In diesem Repo ist bisher **keines** dieser Werkzeuge gebaut — keine Flags, kein
Skriptformat, kein Overlay. `src/` gibt es, aber `src/main.rs` ist ein Platzhalter
(`MinimalPlugins`). Diese Datei ist die Vorgabe, nicht der Befund. Sobald das erste
Skript wirklich mit Exit-Code 0 durchläuft, gehört die Zahl hierher.

Verwandt: [`docs/umgebung.md`](../umgebung.md) · [`docs/lessons/umgebung.md`](umgebung.md) ·
[`docs/lessons/arbeitsweise.md`](arbeitsweise.md) ·
[`docs/lessons/performance.md`](performance.md) · [`docs/STATUS.md`](../STATUS.md) ·
[`docs/TODO.md`](../TODO.md) · [`docs/BUGS.md`](../BUGS.md) ·
[`docs/FRAGEN.md`](../FRAGEN.md) · [`docs/ROADMAP.md`](../ROADMAP.md) ·
[`docs/konventionen.md`](../konventionen.md) — Quelle: `prompts/init.md` §12 (Z. 1176–1264),
`--headless` aus §13/§14.
