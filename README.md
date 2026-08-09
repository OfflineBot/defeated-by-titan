# Defeated by Titan

Ein 3D-Lowpoly-Actionspiel ueber den Kampf gegen Titanen, gebaut in **Bevy (Rust)**.

Du bist ein Vanguard-Bergungsmann mit **Vector Gear**: zwei Greifhaken, zwei Gastanks, zwei
Klingen. Du hakst ein, schwingst, beschleunigst mit Gas — und toetest einen Titanen **nur**
durch einen schnellen Schnitt in den **Cortex**. Alles andere kostet ihn ein Bein und dich
Zeit.

> **Das Spiel in einem Satz:** ein Bewegungsspiel mit hoher Meisterschaftsgrenze, in dem
> Kaempfen der Nebeneffekt guter Bewegung ist.

Der Krieg ist bereits verloren. Ashgate ist gefallen; die Vanguard fuehrt Bergungsmissionen
in die eigenen Ruinen. Der Ton ist gedaempft und erwachsen, nie zynisch — Titanen verdampfen,
statt zu bluten.

## Stand

**Aufsetzen.** Es gibt noch kein spielbares Spiel: Projektbaum, Werkzeuge, Datenextraktion und
Doku stehen, der Stufenplan ist bei Stufe 0/1.

Der ehrliche Stand jeder einzelnen Sache steht in **[`docs/STATUS.md`](docs/STATUS.md)**, mit
einer von vier Stufen pro Zeile:

| | Stufe | heisst |
|---|---|---|
| ⬜ | nicht implementiert | existiert nicht oder nur als Stub. Auch: „Code da, tut aber nichts" |
| 🟨 | halb | gebaut, **nicht getestet, nicht gesehen**. Es kompiliert. Mehr ist nicht behauptet |
| 🟧 | fast | gebaut **und** mit Tests abgesichert, die umfallen **und** im laufenden Spiel gesehen (Screenshot) |
| ✅ | fertig | **der User hat draufgeschaut und abgenommen** — das setzt nur er |

## Starten

Rust 1.85+ (edition 2024). Auf Maschine A liegt die Toolchain in `~/.cargo`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

**Welches Fenstersystem gelinkt wird, entscheidet die Maschine** — Bevys Sammelfeatures
ziehen `wayland` fest nach, und das laesst sich nicht ueberall bauen:

```bash
cargo run                     # x11 (Vorgabe) — baut ueberall, auch ohne Wayland-Bibliotheken
cargo spiel                   # = cargo run --features wayland   (Wayland-Compositor, z. B. niri)
```

### Start-Flags

Ein Hauptmenue ist fuer ein Werkzeug eine Wand ohne Tuer. Deshalb gibt es Wege daran vorbei:

| Flag | wofuer |
|---|---|
| `--sandbox` | leeres Feld, ein Titan, unendlich Gas — zum Anschauen |
| `--mission <name>` | direkt in einen Einsatz, kein Menue |
| `--headless` | **kein Fenster**, fester Tick, laeuft N Ticks und beendet sich mit einem Exit-Code. Der einzige Weg auf einer Maschine ohne Grafiksitzung |
| `--script <datei>` | das Spiel spielen, ohne zu tippen — und mit `assert` wird die Fahrt zu einem Test |
| `--novsync` | zum Messen. Unter Vsync ist jede Bildzeit 16,6 ms, damit misst man sechsmal denselben Deckel |
| `--lag <ms>` | simulierte Latenz. **Jedes Bewegungsfeature wird auch bei 200 ms geprueft**, nicht nur lokal |

## Tastenbelegung

**Steht noch nicht fest, weil noch nichts davon gebaut ist.** Fest steht aus der Design-Bibel:
**PC ausschliesslich, Tastatur und Maus als einziges Eingabegeraet** — kein Gamepad, kein
Touch. Und: freie Tastenbelegung ist eine Anforderung, keine Kuer.

Sobald Stufe 1 steht, kommt die Tabelle hierher. Bis dahin waere sie eine Behauptung.

## Der Aufbau

```
src/        eine Domaene = ein Ordner = ein Plugin (vector, titan, combat, world, net …)
assets/     data/ (die RON-Zahlen)  3d/  textures/  audio/  vfx/  extern/
tools/      baut Dinge: features.py, normen.py, blend/, atlas/, sound/
scripts/    spielt das Spiel: --script-Fahrten
docs/       der Spiegel: STATUS, TODO, Architektur, Konventionen, Fallgeschichten
tests/      tests/<domaene>.rs — plus domaenen.rs, mehrspieler.rs, modelle.rs
```

**Balancing ist Datei-Arbeit, kein Rust.** Ein neuer Titan-Typ, eine Klingenstufe, eine
Gas-Kostenzahl: alles in `assets/data/*.ron`. Im Code stehen nur Einheiten und Mechanik —
sonst braucht die haeufigste Arbeit im Projekt einen Rebuild und passiert deshalb nicht.

## Mitarbeiten

- **Was gebaut werden soll**, steht in [`docs/TODO.md`](docs/TODO.md) (erzeugt, in baubarer
  Reihenfolge) — die Quelle ist `gameplay/features.xlsx` mit 687 Ticketzeilen.
- **Wie gearbeitet wird**, steht in [`CLAUDE.md`](CLAUDE.md) und
  [`docs/README.md`](docs/README.md).
- **Wuensche und Design** gehen bis auf Weiteres in den Eingangskorb `gameplay/`.
  *(Nach der Aufloesung des Bootstrap-Geruests ist es `docs/gameplay/` plus
  `docs/TODO.md` — `prompts/init.md` §18.)*

**Vorbild:** [Attack on Titan Revolution](https://www.roblox.com/games/13379208636/Attack-on-Titan-Revolution)
(Roblox). Uebernommen wird das *Gefuehl* des Gear, nicht die Plattform: dies hier ist ein
eigenstaendiges Spiel in Bevy/Rust.
