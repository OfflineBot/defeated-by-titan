# umgebung — welche Maschine kann was

Stand: 2026-08-09 · Stufe: 🟧 (gemessen, nicht geschaetzt — die Befehle stehen daneben)

Das Projekt laeuft auf zwei Rechnern, und sie koennen nicht dasselbe. Wer das verwechselt,
haelt eine fehlende Grafiksitzung fuer einen Bug oder einen N100 fuer eine
Performance-Regression (`prompts/init.md` §14). **Erste Frage jeder Sitzung: `hostname`.**

## A — `debian` (gemessen 2026-08-09)

| Frage | Antwort | womit gemessen |
|---|---|---|
| Hostname | `debian` | `hostname` |
| Kernel | 6.12.85+deb13-amd64 | `uname -r` |
| Kerne | **4** | `nproc` |
| Grafiksitzung | **keine** — `WAYLAND_DISPLAY` und `DISPLAY` beide leer | `echo "$WAYLAND_DISPLAY $DISPLAY"` |
| Compositor | **kein `niri`** | `command -v niri` → leer |
| Platte `/` | 452 G, 406 G frei (6 % belegt) | `df -h /home` |
| Rust | **1.97.1** (8bab26f4f 2026-07-14), cargo 1.97.1 | `rustc --version` |
| Blender | **fehlt** | `command -v blender` → leer |
| `gh` | vorhanden, angemeldet als `OfflineBot` | `gh auth status` |
| Python | 3.13.5, **kein pip, kein ensurepip, kein openpyxl** | `python3 --version` |
| LibreOffice | **fehlt** | `command -v libreoffice` |
| Netz | erreichbar (crates.io, static.rust-lang.org) | `curl -sI` |
| passwortloses sudo | **nein** | `sudo -n true` |

### Was hier nachinstalliert wurde

- **Rust** gab es auf dieser Maschine nicht. Nachinstalliert mit `rustup` nach `~/.cargo`
  (`--no-modify-path --profile minimal`). **Das heisst: `export PATH="$HOME/.cargo/bin:$PATH"`
  gehoert vor jeden `cargo`-Aufruf**, solange die Shell das nicht selbst tut.

### Was daraus folgt — ohne Beschoenigung

| Sache | Folge |
|---|---|
| **Kein Fenster** | **Die Obergrenze auf A ist 🟨**, mit dem Vermerk *„Logik getestet, Pixel ungesehen — Maschine A"*. Kein Screenshot per Compositor, also kein 🟧 aus dem Fenster (§8, §14). |
| **Kein Blender** | Die Modellkette baut nur `tools/blend/*.py`; `.blend` und `.glb` entstehen hier nicht. Das Spiel muss ohne Blender laufen — **einmal warnen, vorhandene `.glb` benutzen, nicht abstuerzen** (§7). |
| **Kein pip** | `tools/features.py` benutzt ausschliesslich die Standardbibliothek (`zipfile` + `xml`), kein `openpyxl`. Laeuft dadurch auf jeder Maschine ohne Installation. |
| **4 Kerne** | Parallelitaet 2–3 gleichzeitig, und **der Compiler ist auch ein Verbraucher** (§17). Zwanzig Agenten auf vier Kernen sind langsamer als drei. |
| **N100-Klasse** | **Keine Performance-Aussage von hier.** Jede Zahl in `STATUS.md`/`BUGS.md` traegt `[debian]` — eine Bildzeit ohne Maschinenangabe ist keine Messung. |

### Der eine offene Beweis: Offscreen-Rendering

`prompts/init.md` §14 laesst eine Ausnahme zu: ein Bild aus einem Render-Target statt aus
einem Fenster. **Solange nicht bewiesen ist, dass das auf dieser Maschine wirklich ein PNG
liefert, ist es nichts wert** — und es ist bisher nicht bewiesen. Siehe `docs/FRAGEN.md`
(Q-009) und `docs/TODO.md`.

## B — `offlinebot` (gemessen 2026-08-09)

Die Angaben stammten bis dahin aus `prompts/init.md` §14 und waren an zwei Stellen falsch —
siehe die Folgen-Tabelle unten. Sie sind jetzt mit denselben Befehlen wie bei A nachgemessen.

| Frage | Antwort | womit gemessen |
|---|---|---|
| Hostname | `offlinebot` | `hostname` |
| Kernel | 7.1.6-1-cachyos | `uname -r` |
| Kerne / Threads | 8 Kerne, **16 Threads** (Ryzen 7 5800X) | `nproc` · Bevys `SystemInfo` meldet `core_count: 8` |
| Grafiksitzung | **ja** — `WAYLAND_DISPLAY=wayland-1`, `DISPLAY=:0` | `echo "$WAYLAND_DISPLAY $DISPLAY"` |
| Compositor | **niri 26.04** (8ed0da4) | `niri --version` |
| GPU | NVIDIA GeForce RTX 3080 | `nvidia-smi --query-gpu=name --format=csv,noheader` |
| RAM | 31 GB, davon 24 GB frei | `free -g` |
| Platte `/home` | 928 G, **128 G frei (87 % belegt)** | `df -h /home` |
| Rust | **1.95.0** (59807616e 2026-04-14), cargo 1.95.0 | `rustc --version` |
| Wo liegt cargo | **`/usr/bin/cargo`** — *nicht* in `~/.cargo` | `which cargo` |
| Blender | **5.2.0 LTS** vorhanden | `blender --version` |
| `gh` | vorhanden, angemeldet als `OfflineBot` | `gh auth status` |
| Python | 3.14.6, `pip3` vorhanden, **kein openpyxl** | `python3 --version` |
| LibreOffice | **fehlt** | `command -v libreoffice` |
| passwortloses sudo | **nein** | `sudo -n true` |

### Was daraus folgt — die drei Unterschiede zu A, die wehtun

| Sache | Folge |
|---|---|
| **Fenster vorhanden** | **Auf B ist 🟧 erreichbar.** Screenshot per `niri msg action screenshot-window`. Auf A ist bei 🟨 Schluss (§8, §14) — was dort gebaut wird, muss hier *gesehen* werden, sonst bleibt es 🟨. |
| **Rust ist hier ÄLTER als auf A** | B: **1.95.0**, A: 1.97.1. Die Richtung ist die unerwartete: was auf A uebersetzt, uebersetzt nicht zwingend hier. Ein Sprachfeature aus 1.96/1.97 faellt erst auf B auf. **Die Untergrenze des Projekts ist 1.95.0.** |
| **`cargo` liegt in `/usr/bin`** | Das `export PATH="$HOME/.cargo/bin:$PATH"` aus `CLAUDE.md` ist eine **Maschine-A-Zeile**. Hier ist sie wirkungslos, aber harmlos. |
| **Blender ist da** | Die Modellkette laesst sich hier wirklich fahren — auf A nicht. Modellarbeit gehoert auf B. |
| **16 Threads, aber `cargo` sperrt `target/`** | Die Parallelitaet kommt **nicht** von `nproc`: bauende Agenten warten auf denselben Lock. Brauchbar sind **vier gleichzeitig**, nicht sechzehn (`docs/lessons/arbeitsweise.md`). |

⚠️ **Vor jedem groesseren Build `df -h /home`.** `ld: signal 7 / Bus error` beim Linken heisst
volle Platte, nicht kaputter Code (§15). Und die alte Zahl in dieser Datei war deutlich zu
niedrig: **`target/` dieses Projekts liegt gemessen bei 17 G** (`du -sh target`), nicht bei
1 GB. Bei 128 G frei ist das noch Luft, aber `cargo clean` kostet danach einen vollen
Bevy-Neubau — nicht aus Gewohnheit aufrufen.

## Gemessene Bauzeiten

| Was | Maschine | Zeit | Kommando |
|---|---|---|---|
| erster `cargo build` (Bevy 0.19.0 + ~460 Crates, `opt-level = 3` fuer Dependencies) | `[debian]` | siehe `docs/lessons/umgebung.md` | `cargo build` |
| `cargo check` ueber den ganzen Baum, Bevy bereits gebaut | `[cachy]` | **1 min 22 s** | `cargo check` |
| `cargo test` (95 Tests, warmer Baum) | `[cachy]` | unter 10 s | `cargo test` |

Verwandt: [`docs/lessons/umgebung.md`](lessons/umgebung.md) (die Fallen), `prompts/init.md` §14/§15.
