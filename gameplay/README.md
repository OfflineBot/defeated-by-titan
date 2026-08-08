# `gameplay/` — der Eingangskorb des Users

**Hier legt der User ab, WAS gebaut werden soll.** Das Herzstück ist eine **Excel-Datei mit allen
Features** (`features.xlsx`, sehr viele Zeilen); dazu Mechaniken, Zahlen, Skizzen — alles, was
Gameplay ist. Dieser Ordner ist die **Autorität für den Inhalt** des
Spiels; die Dateien in `prompts/` (Start: `prompts/init.md`) beschreiben nur die Richtung und das Handwerk.

## Regeln für Agenten

1. **Erste Handlung jeder Sitzung: `ls -R gameplay/` und alles Neue lesen.** Der Ordner kann
   jederzeit wachsen, auch mitten in einer Sitzung.
2. **Nichts hier wird gelöscht, überschrieben oder abgehakt.** Es ist der Korb des Users, nicht
   dein Arbeitsblatt. Hinzufügen darfst du (z. B. eine Rückfrage-Datei), fremden Text ändern nie.
3. **Abgehakt wird in `docs/STATUS.md` und `docs/TODO.md`**, nicht hier.
4. **Widerspruch zu `prompts/` oder zum Code?** Dieser Ordner gewinnt. Widerspruch *innerhalb*
   des Ordners → `docs/FRAGEN.md`, nicht selbst entscheiden.
5. **Keine Zeile darf verschwinden.** Was dir unnötig erscheint, kommt als Frage nach
   `docs/FRAGEN.md` — nicht in den Müll.

## Was hier hinein gehört (Vorschlag, der User entscheidet)

```
gameplay/
  features.xlsx      ⭐ ALLE Tickets, 12 Blätter — die eigentliche Arbeitsvorlage (bleibt erhalten!)
  TODO.md            ergänzende Notizen zur Liste
  vector.md          Vector Gear: Haken, Seil, Gas, Boost, Wandlauf
  titanen.md         Husk/Errant/Scuttler/… — Verhalten, Cortex, Regeneration
  kampf.md           Klingen, Schaden aus Geschwindigkeit, Amputation
  missionen.md       Einsätze, Ziele, Phasen, Wellen
  progression.md     XP, Mark/Sigil, Gear-Budget, Traits, Lineages
  welt.md            Ashgate District, Ringe, Titanwood, Versorgung
  balance/           konkrete Zahlen und Tabellen
  bilder/            Skizzen, Referenzen (mit URL + Datum), Screenshots mit Anmerkungen
                     (heruntergeladene PLATZHALTER-Assets gehören NICHT hierher,
                      sondern nach assets/extern/ — prompts/init.md §7)
```

## Wohin es übersetzt wird

| Was im Korb liegt | Wohin es im Projekt landet |
|---|---|
| `features.xlsx` | **per Skript** ausgelesen (alle Blätter, `data_only=True`, Farben beachten) → `docs/features.ron` mit einer `F-ID` pro Zeile → daraus generiert: `docs/TODO.md` + `docs/STATUS.md`. Die Datei selbst wird **nie** verändert oder gelöscht. |
| Ein Feature / eine Mechanik | `docs/gameplay/<thema>.md` (das Design) + eine ⬜-Zeile in `docs/STATUS.md` |
| Eine Zahl / Balance | in die passende `assets/data/*.ron` — **niemals in den Rust-Code** |
| Ein Item / Titan / Trait / Einsatz | ein Eintrag in `titans.ron` / `gear.ron` / `traits.ron` / `missions.ron` |
| Eine Skizze / ein Bild | bleibt hier, wird aus `docs/gameplay/` verlinkt |
| Etwas Unklares | `docs/FRAGEN.md` — nicht raten |

## Design — die Bibel ist verbindlich

**Halte dich an ALLE Richtlinien der Design-Bibel:
[`../prompts/DefeatedByTitans_Design-Bibel.md`](../prompts/DefeatedByTitans_Design-Bibel.md).**

Sie ist die Autorität für *warum*, *in welcher Reihenfolge* und *woran man merkt, dass es
funktioniert*: die fünf Designpfeiler, Welt und Ton, der visuelle Stil (Low Poly, ein Farbatlas,
drei reservierte Signalfarben), Plattform (PC, Maus+Tastatur), die vier Mehrspieler-Grundregeln,
die Gegner-Philosophie, der Phasenplan P0–P11 mit dem harten Gate **„kein Meta-System vor
bestandenem Vector-Gear-Gate"**, die Kennzahlen und die Risiken.

Verbindlich ist außerdem Blatt **`10_Namensschema`** der `features.xlsx`: Vector Gear statt ODM,
**Cortex** statt Nacken, Vanguard statt Scouts, Trait/Lineage/Relic/Mark statt
Perk/Family/Artifact/Gold. Kein Referenzbegriff im Code.

## ⚠️ Wenn dieser Ordner aufgelöst wird

Der Inhalt wandert in die endgültige Struktur (`docs/gameplay/`, `docs/backlog/*.ron`,
`docs/TODO.md`, `assets/data/*.ron`) — siehe `prompts/init.md` §18. **Dabei werden ALLE Verweise
mitgezogen:** jeder Link, der auf `gameplay/…` oder `prompts/…` zeigte, zeigt danach auf den neuen
Ort, und `grep -rn "gameplay/" .` ist der Beleg dafür. **Es darf keine Datei übrig bleiben, die
niemand kennt** — jede ist verlinkt oder gelöscht (`prompts/init.md` §10).
