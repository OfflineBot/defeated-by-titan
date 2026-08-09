# umgebung — zwei Maschinen, vier Fallen, die hier wirklich zugeschnappt sind, und die Fehlermeldungen, die etwas anderes bedeuten als sie sagen

Stand: 2026-08-09 · Stufe: 🟨 (die vier Fallen unter „Was hier wirklich passiert ist" sind in
diesem Repo aufgetreten und mit Datei oder Kommando belegt — eine gemessene **Zeit** hat nur
Falle 2; die generischen Fallen weiter unten sind aus `prompts/init.md` §15 übernommen und in
diesem Repo noch nicht selbst ausgelöst worden)

## Der Fall

Die Umgebung kostet nicht deshalb Zeit, weil sie kaputt ist, sondern weil ihre Symptome wie
Programmfehler aussehen. Ein fehlendes Fenster liest sich wie ein Bug. Ein N100 liest sich wie
eine Performance-Regression. Eine volle Platte meldet sich als Linker-Absturz. Wer an dieser
Stelle den Code verdächtigt, sucht stundenlang an der falschen Stelle.

**Erste Frage jeder Sitzung, vor allem anderen:**

```bash
hostname          # 'debian' → headless (A) · 'offlinebot' → volle Grafik (B)
uname -r; nproc; echo "WAYLAND=$WAYLAND_DISPLAY DISPLAY=$DISPLAY"
df -h /home       # auf B vor dem ersten Build
```

Die Messwerte stehen in [`docs/umgebung.md`](../umgebung.md) — A gemessen, B bisher nur aus
`prompts/init.md` §14 übernommen. Hier steht nur, was die Unterschiede kosten.

| | **A — `debian`** | **B — `offlinebot`** |
|---|---|---|
| Oberfläche | **keine** — kein Monitor, kein Wayland/X | niri (Wayland), kitty, fish |
| CPU / GPU | Intel N100, 4 Kerne · UHD Graphics | Ryzen 7 5800X, 16 Threads · RTX 3080 |
| Ein Fenster | **geht nicht — und das ist in Ordnung** | geht |
| Screenshot | nur via Offscreen-Rendering, falls es läuft | `niri msg action screenshot-window` |
| Höchste erreichbare Stufe | **🟨** | 🟧 |

---

## Auf A wird gearbeitet, nur anders geprüft

Kein Fenster ist kein Grund, nicht weiterzubauen. Vollständig möglich:

| Was | Womit | Warum es ohne Bildschirm geht |
|---|---|---|
| Logik-Tests | `cargo test` | Vector-Gear-Mathematik, Trefferzonen, Schadenskurven, RON-Validierung, Weltgenerierung als Zahlen, Domänen-Test, `tests/mehrspieler.rs` — alles Zahlen |
| Skriptfahrten mit `assert` | `cargo fahrt <skript>` (Alias für `cargo run -- --headless --script`) | **nur wenn der `--headless`-Modus existiert**: `primary_window: None`, fester Tick, N Ticks, Exit-Code sagt, ob alle `assert` hielten |
| Modellkette | `blender --background` | `.py` → `.blend` → `.glb` und der Struktur-Test (Empties, Vertexfarben, `metallicFactor`) brauchen keine Anzeige |
| Excel-Extraktion | `python3 tools/features.py` | reine Dateiarbeit |
| Doku, Aufräumen, Refactoring | Editor | reine Dateiarbeit |

Dass die Skriptfahrt auf **jeder** Maschine prüfbar ist, ist der eigentliche Grund für den
`--headless`-Modus in Stufe 1 — nicht Bequemlichkeit. Er ist die einzige Brücke zwischen A, B
und einem CI eines Tages.

## Was auf A NICHT geht — ohne Ausrede

| Verbot | Warum | Was man stattdessen tut |
|---|---|---|
| **Kein Bild ⇒ kein 🟧** | Obergrenze ist 🟨 mit dem Vermerk *„Logik getestet, Pixel ungesehen — Maschine A"* | Nicht aufrunden, nicht „sieht sicher richtig aus". Auf B nachholen. |
| Offscreen-PNG als Beleg | Bevy kann in ein Render-Target zeichnen, Vulkan braucht dafür keine Anzeige — aber **behauptet ist es nichts wert** | Erst beweisen, dass es auf dem N100 wirklich ein Bild liefert, dann als Beleg benutzen |
| **Keine Performance-Aussage** | Ein N100 mit integrierter Grafik und ein 5800X mit einer 3080 sind keine Messreihe | **Jede Zahl in `STATUS.md`/`BUGS.md` trägt `[debian]` oder `[cachy]`.** Eine Bildzeit ohne Maschinenangabe ist keine Messung |
| Kein `niri msg` | gibt es auf A nicht | Der ganze Screenshot-Abschnitt gilt nur für B |
| Langsame Builds melden | 4 Kerne | Das ist **keine Regression**, das ist der N100 |

## Auf B gilt zusätzlich

Volle Grafik heißt volle Beweispflicht: hier werden die Screenshots gemacht, hier wird gemessen,
hier wird aus 🟨 ein 🟧. Und hier ist die Platte der Feind — **`df -h /home` vor dem ersten
Build**.

---

## Was hier wirklich passiert ist — vier Fallen auf Maschine A

### 1. Rust war gar nicht installiert

Kein `rustc`, kein `cargo`. Nachinstalliert per `rustup` nach `~/.cargo`.

**Kosten für den Nächsten:** die Shell findet `cargo` nicht, und die Meldung
(`command not found`) sieht aus wie eine kaputte Maschine.

```bash
export PATH="$HOME/.cargo/bin:$PATH"     # gehört vor jeden cargo-Aufruf auf A
```

### 2. Bevys Sammelfeatures ziehen `wayland` mit — und A kann es nicht bauen

Die Sammelfeatures `3d`, `ui` und `2d` ziehen `default_platform` nach, und darin steckt
`wayland` fest verdrahtet. Auf A fehlt `wayland-client.pc`, und ohne passwortloses sudo kommt
es auch nicht dazu.

| | |
|---|---|
| Symptom | `cargo build` bricht in `wayland-sys` ab |
| Gemessene Kosten | **9m22s** bis zum Abbruch `[debian]` — die Zeit ist weg, bevor die erste Zeile eigener Code übersetzt wird |
| Ursache | nicht der Code, nicht die Bevy-Version: eine fehlende `.pc`-Datei des Systems |
| Lösung | `default-features = false` und die Feature-Liste **von Hand** in `Cargo.toml`, plus `[features] default = ["x11"]` mit `x11 = ["bevy/bevy_winit", "bevy/x11"]` und `wayland = ["bevy/bevy_winit", "bevy/wayland"]` |

Warum `x11` die Vorgabe ist: winit benutzt dafür `x11rb`/`x11-dl`, und die laden zur Laufzeit
per `dlopen` — es braucht also **keine** Systembibliothek zur Bauzeit. `wayland` braucht
`wayland-client.pc` und ist deshalb optional.

```bash
cargo build                            # A (debian): Vorgabe, baut überall
cargo run --features wayland,klang     # B (offlinebot) — kurz: cargo spiel
```

`klang` hat dasselbe Problem eine Etage tiefer: es braucht `alsa.pc`, und das fehlt auf A
ebenso. Mit der Handliste **plus `audio`** bricht der Build nach **13m40s** in `alsa-sys` ab
(gemessen `[debian]`, steht in `Cargo.toml`). Deshalb ist auch `klang` optional und steckt nur
in `cargo spiel`.

Die Begründung steht als Kommentar in `Cargo.toml`, die Abkürzungen in `.cargo/config.toml`.
**Wer dort `default` wieder einschaltet, verschenkt neun Minuten auf A und merkt es zu spät.**

### 3. Kein pip, kein ensurepip, kein openpyxl, kein libreoffice

Die Excel-Liste muss trotzdem gelesen werden. Deshalb liest `tools/features.py` die `.xlsx`
mit der **Standardbibliothek**: eine `.xlsx` ist ein ZIP aus XML, `zipfile` + `xml` reichen.

**Nicht** „auf A halt nicht extrahieren" und **nicht** „openpyxl installieren" — der
Installationsversuch scheitert an fehlendem pip und fehlendem sudo und kostet nur Suchzeit.
Der Nebeneffekt ist der eigentliche Gewinn: die Extraktion läuft jetzt auf **jeder** Maschine
ohne Installation.

### 4. `target/` ist groß, bevor überhaupt ein Binary existiert

Gemessen `[debian]`: `du -sh target/` → **5,1 G**, davon fast alles `target/debug/deps`, und
`target/debug/defeated_by_titan` gibt es zu diesem Zeitpunkt noch gar nicht. Das ist kein Leck,
das ist die Größenordnung von Bevy mit `opt-level = 3` für Dependencies. Wer sie nicht einplant,
landet in der ersten generischen Falle unten (`ld: signal 7`).

---

## Die generischen Fallen: wenn die Meldung etwas anderes bedeutet als sie sagt

| Meldung | Was sie **wirklich** heißt | Was man tut |
|---|---|---|
| `ld: signal 7` / `Bus error` beim Linken | **Die Platte ist voll.** `target/debug/deps` staut Bevy-Binaries im dreistelligen GB-Bereich | **Erst `df -h /home`**, nicht den Code verdächtigen. Dann `cargo clean` bzw. gezielt `rm -rf target/debug/incremental` |
| `undefined hidden symbol: anon.….llvm.…` | Kaputter Inkrement-Cache nach einem abgewürgten Build | `rm -rf target/debug/incremental` |
| Ein roter Build, der grün ist | Du hast auf `.rs` gefiltert und Warnungen mitgefangen | `cargo check 2>&1 \| grep '^error'` — **nicht** auf `.rs` filtern |
| „Der Rückbau ist doch passiert" | `pkill` stand am Anfang der Kette, fand keinen Prozess, gab Exit 1 — und schluckte den Rest | **`pkill` NIE an den Anfang einer Befehlskette.** `pkill -f target/debug/defeated_by_titan` liefert auch mal Exit 144: normal |

Die Platte ist auf **B** der Feind, nicht auf A: auf B war sie schon einmal voll (auf A sind
laut [`docs/umgebung.md`](../umgebung.md) 406 G von 452 G frei). Deshalb ist `df -h /home` dort
Pflicht, nicht Vorsicht.

---

## Mehrere Agenten im selben Repo

Dateien ändern sich unter dir, und der Build ist zwischendurch rot **ohne dein Zutun**. Das ist
keine Regression, das ist ein anderer Agent mitten in einem Edit.

| Regel | Warum |
|---|---|
| **Vor jedem Edit die Datei frisch lesen** | Dein Bild der Datei kann veraltet sein, ohne dass du etwas gemerkt hast |
| **NIEMALS `git stash`, `git checkout --`, `git clean -fdx`** | Das wirft die Arbeit von jemandem weg, der gerade daneben tippt. Es gibt keinen Fall, in dem das hier richtig ist |
| Eine RON-Datei wird als **GANZE** Datei geschrieben | Zwei Sitzungen mergen nicht. **Wer zuletzt speichert, gewinnt alles** |
| Nach jedem Schreiben in eine geteilte Datei per `grep` prüfen, dass dein Wert drinsteht | Sonst merkst du den Verlust nie |
| In [`docs/STATUS.md`](../STATUS.md) eintragen, an welcher Domäne du arbeitest | Damit ein anderer Agent eine andere nimmt — Konflikte vermeiden ist billiger als sie lösen |

## `// TEMP`

Temporäre Hacks zum Screenshotten **immer** mit `// TEMP` markieren, danach:

```bash
grep -rn TEMP src/
```

Ein vergessener Test-Hack ist ein Geist, den der Nächste jagt — und er jagt ihn im
Spielcode, nicht im Werkzeug.

---

## Lücken

- **Offscreen-Rendering auf A ist unbewiesen.** Ob der N100 über ein Render-Target wirklich
  eine PNG liefert, hat hier niemand getestet. Bis dahin bleibt die Obergrenze auf A 🟨.
- **`blender --background` ist auf A nicht gelaufen** — Blender fehlt auf dieser Maschine
  ([`docs/umgebung.md`](../umgebung.md)). Dass die Modellkette headless geht, ist die Aussage
  der Quelle, nicht eine Messung dieses Projekts.
- **Die generischen Fallen** (Bus error, `anon.llvm`, Exit 144) sind hier noch nicht selbst
  ausgelöst worden. Sie stehen als Warnung, nicht als Protokoll.
- **Der `--headless`-Modus existiert noch nicht.** `src/shared/start.rs` liest das Flag zwar
  schon, aber `src/main.rs` startet nur `MinimalPlugins` — es gibt weder Fenster noch Fahrt.
  Solange das so ist, kann A nur `cargo test`; das ist der Grund, warum der Modus in Stufe 1
  gehört und nicht später.
- **Die Tabelle „Auf A wird gearbeitet" sagt, was ginge, nicht was da ist.** `tests/` gibt es
  in diesem Repo noch nicht (also auch kein `tests/mehrspieler.rs`), `tools/blend/` ebenso
  wenig. Die Zeilen stammen aus `prompts/init.md` §14 und sind Vorgabe, kein Protokoll.

Verwandt: [docs/umgebung.md](../umgebung.md) · [STATUS.md](../STATUS.md) · [BUGS.md](../BUGS.md) · [konventionen.md](../konventionen.md) · [FUNDE.md](../FUNDE.md) · [lessons/workflow.md](workflow.md) · [lessons/performance.md](performance.md)
