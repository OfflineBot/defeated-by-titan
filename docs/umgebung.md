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

## B — `offlinebot` (noch nicht selbst gemessen)

Die Angaben stammen aus `prompts/init.md` §14, **nicht** aus einer Messung dieser Sitzung.
Wer zuerst auf B sitzt, misst mit denselben Befehlen wie oben nach und ersetzt diese Tabelle.

| Frage | laut `prompts/init.md` §14 |
|---|---|
| System | CachyOS, Kernel 7.x |
| Oberflaeche | niri (Wayland), kitty, fish |
| CPU / GPU | Ryzen 7 5800X, 16 Threads · RTX 3080 |
| RAM / Platte | 31 GB · **hier war die Platte schon einmal voll** |
| Fenster / Screenshot | ja · `niri msg action screenshot-window` |
| Parallelitaet | 8 und mehr |

⚠️ **Auf B vor dem ersten Build `df -h /home`.** `ld: signal 7 / Bus error` beim Linken heisst
volle Platte, nicht kaputter Code (§15). Ein `target/` dieses Projekts liegt nach dem ersten
Build bereits bei rund **1 GB** und waechst weiter.

## Gemessene Bauzeiten

| Was | Maschine | Zeit | Kommando |
|---|---|---|---|
| erster `cargo build` (Bevy 0.19.0 + ~460 Crates, `opt-level = 3` fuer Dependencies) | `[debian]` | siehe `docs/lessons/umgebung.md` | `cargo build` |

Verwandt: [`docs/lessons/umgebung.md`](lessons/umgebung.md) (die Fallen), `prompts/init.md` §14/§15.
