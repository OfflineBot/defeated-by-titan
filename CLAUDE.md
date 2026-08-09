# CLAUDE.md — wie dieses Projekt tickt

**Defeated by Titan** — ein 3D-Lowpoly-Actionspiel ueber den Kampf gegen Titanen, in **Bevy
(Rust)**. Kern ist das **Vector Gear**: zwei Haken, zwei Gastanks, zwei Klingen. Ein Titan
stirbt **nur** durch einen schnellen Schnitt in den **Cortex**.

> 🔒 **Die Engine ist Bevy (Rust). NICHT Roblox.** Vom User bestaetigt am 2026-08-09. Die
> Design-Bibel ist an sechs Stellen fuer Roblox geschrieben — diese Stellen werden
> **uebersetzt, nicht befolgt**. Die Uebersetzungstabelle steht in [`docs/architektur.md`](docs/architektur.md)
> und **waechst**: wer auf eine neue Roblox-Anweisung stoesst, traegt die Zeile dort nach.

**Dieses Projekt wird noch aus `prompts/` und `gameplay/` aufgebaut.** Das ist
Bootstrap-Geruest und verschwindet, wenn es uebertragen ist (`prompts/init.md` §18).
Bis dahin gilt: `gameplay/` bestimmt den **Inhalt**, `prompts/` das **Handwerk**.

---

## Sitzungsanfang — immer, vollstaendig, in dieser Reihenfolge

```bash
hostname                                    # 'debian' = kein Fenster, das ist ok
ls -lt prompts/ && ls -R gameplay/          # neuer Auftrag? Der User legt jederzeit nach
git status --short && git log --oneline -5  # was hat eine andere Sitzung getan?
cat docs/STATUS.md docs/TODO.md             # wo stehen wir wirklich?
export PATH="$HOME/.cargo/bin:$PATH"        # Rust liegt in ~/.cargo (Maschine A)
cargo check 2>&1 | grep '^error'            # ist der Baum gruen, BEVOR ich anfasse?
```

**Sitzungsende:** `docs/STATUS.md` + `docs/TODO.md` nachziehen · Screenshots nach
`docs/bilder/` mit normiertem Namen · neue Erkenntnis nach `docs/lessons/` · offene Frage nach
`docs/FRAGEN.md` · committen mit normierter Message · **und ein ehrlicher Absatz, was
ungesehen blieb.**

## Die sechs Regeln, die immer gelten

1. **Die vier Stufen sind die einzige Wahrheit ueber den Fortschritt.**
   ⬜ nicht implementiert · 🟨 gebaut, **ungetestet und ungesehen** · 🟧 getestet **und** im
   Spiel gesehen (Bild + Zahl + ein Test, der rot wird) · ✅ **setzt NUR der User**.
   **Rueckwaerts ist erlaubt und erwuenscht. Bau nie auf 🟨. Unsicherheit setzt die Stufe
   herunter, nicht hinauf.** → [`docs/STATUS.md`](docs/STATUS.md)
2. **Zahlen gehoeren in RON, nicht in Rust.** Ein Titan-Typ, eine Klingenstufe, eine
   Gas-Kostenzahl: Datei-Arbeit. Im Code stehen nur Einheiten und Mechanik. **Kein
   `serde(default)` fuer Spielwerte** — ein fehlender Wert soll beim Laden krachen.
3. **Eine Domaene = ein Ordner = ein Plugin = standalone.** Nur `shared`, `data` und Bevy
   sind frei; jede andere Kante braucht eine Zeile mit Begruendung in der Erlaubnisliste von
   [`docs/architektur.md`](docs/architektur.md), und `tests/domaenen.rs` faellt sonst um.
   Kommunikation laeuft ueber Components und Messages. **Ein Feld hat genau einen Schreiber.**
4. **Multiplayer entscheidet die Architektur ab Tag 1** — der Netzcode wird heute nicht
   gebaut, aber nichts wird gebaut, das ihn spaeter teuer macht. Kein `.single()` auf Spieler,
   Spielerzustand nie als `Resource`, Eingabe ist ein `Intent`, Simulation in `FixedUpdate`,
   stabile Ids statt `Entity`. → [`docs/multiplayer.md`](docs/multiplayer.md)
5. **Ein Bug ohne Beleg ist ein Geruecht, ein Fix ohne roten Test eine Vermutung.**
   Erst der Test, der umfaellt; dann der Fix; dann den Fix herausnehmen und zusehen, dass der
   Test wieder rot wird. **Kein Repro ⇒ kein Fix.** → [`docs/BUGS.md`](docs/BUGS.md)
6. **Nichts aendert sich pro Frame, alles pro Sekunde**, und nichts laeuft ueber alle
   Entities, um eine Frage ueber die zehn Meter vor der Nase zu beantworten.
   → [`docs/lessons/performance.md`](docs/lessons/performance.md)

## Wo was steht

| Frage | Datei |
|---|---|
| Was ist offen, in welcher Reihenfolge? | [`docs/TODO.md`](docs/TODO.md) *(erzeugt)* |
| Wie weit darf ich einer Sache trauen? | [`docs/STATUS.md`](docs/STATUS.md) *(erzeugt)* |
| Welche Maschine, was geht hier? | [`docs/umgebung.md`](docs/umgebung.md) |
| Domaenen, Erlaubnisliste, wer schreibt was | [`docs/architektur.md`](docs/architektur.md) |
| Achsen, Einheiten, Begriffe, Namensnormen | [`docs/konventionen.md`](docs/konventionen.md) |
| Modelle austauschen (fuer den User geschrieben) | [`docs/modelle.md`](docs/modelle.md) |
| Was mir nicht gehoert zu entscheiden | [`docs/FRAGEN.md`](docs/FRAGEN.md) |
| Was bewusst spaeter kommt | [`docs/ROADMAP.md`](docs/ROADMAP.md) |
| Alle Doku auf einen Blick | [`docs/README.md`](docs/README.md) |

## Die Werkzeuge

```bash
python3 tools/features.py            # gameplay/features.xlsx -> features.ron + TODO + STATUS
python3 tools/features.py --pruefen  # nur der Zeilenzahl-Pruefwert je Blatt
python3 tools/normen.py              # Begriffe, tote Links, Zombie-Dateien, Testnamen
python3 tools/normen.py --commit-msg .git/COMMIT_EDITMSG
```

**Das Spiel starten** — welches Fenstersystem gelinkt wird, entscheidet die Maschine:

```bash
cargo build                          # Maschine A (debian): x11, baut ueberall
cargo spiel                          # Maschine B (offlinebot/niri): --features wayland
cargo run -- --headless --script scripts/<f-id>-<kurz>.txt   # eine Fahrt ohne Fenster
```

## Was heute noch nicht gilt

Der Stufenplan (`prompts/init.md` §13) ist beim **Aufsetzen**. Ab Stufe 3 uebernimmt der
Phasenplan der Bibel — und **seine harte Regel: kein Meta-System vor bestandenem
Vector-Gear-Gate.** Faehigkeitsbaum, Wirtschaft, Lineages, Raids, Kosmetik werden **nicht
angefangen**, solange sich die Bewegung nicht ueberzeugend anfuehlt.

## Commit-Messages

```
<F-ID|T-ID|docs|test|tool|fix|chore>: <eine Zeile, was jetzt anders ist>   ← max 72, Deutsch

Stufe: 🟨 → 🟧
Beleg: tests/vector.rs::f014_boost_verbraucht_gas · docs/bilder/f014-boost.png · 12,4 → 3,1 ms [debian]
```

⚠️ **Keine Werkzeug- oder Autorenspuren.** Kein `Co-Authored-By`, keine Signatur, kein
„generated with", kein Modellname — in keiner Commit-Message, keiner PR-Beschreibung, keinem
Tag. Eine Commit-Message beschreibt **die Aenderung**, nicht ihren Urheber.
`tools/normen.py --commit-msg` prueft das.

## Zum Schluss

**Erst messen, dann behaupten.** Fast jeder teure Fehler in einem Projekt wie diesem ist eine
Stelle, an der etwas Vernuenftiges *erklaert* wurde, statt es in einer Minute zu *messen*.
Schreib, was du weisst: „gebaut, ungetestet — 🟨" ist ein guter Satz, „sollte jetzt gehen"
ist keiner.
