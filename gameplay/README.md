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
  features.xlsx      ⭐ ALLE Features — die eigentliche Arbeitsvorlage (bleibt erhalten!)
  TODO.md            ergänzende Notizen zur Liste
  odm.md             Bewegung: Haken, Seil, Gas, Boost, Wandlauf
  titanen.md         Typen, Verhalten, Trefferzonen, Regeneration
  kampf.md           Klingen, Schaden aus Geschwindigkeit, Amputation
  missionen.md       Einsätze, Ziele, Phasen, Wellen
  progression.md     XP, Gold, Gear-Stufen, Perks, Familien
  welt.md            Stadt, Mauern, Wald, Versorgung
  balance/           konkrete Zahlen und Tabellen
  bilder/            Skizzen, Referenzen, Screenshots mit Anmerkungen
```

## Wohin es übersetzt wird

| Was im Korb liegt | Wohin es im Projekt landet |
|---|---|
| `features.xlsx` | **per Skript** ausgelesen (alle Blätter, `data_only=True`, Farben beachten) → `docs/features.ron` mit einer `F-ID` pro Zeile → daraus generiert: `docs/TODO.md` + `docs/STATUS.md`. Die Datei selbst wird **nie** verändert oder gelöscht. |
| Ein Feature / eine Mechanik | `docs/gameplay/<thema>.md` (das Design) + eine ⬜-Zeile in `docs/STATUS.md` |
| Eine Zahl / Balance | in die passende `assets/data/*.ron` — **niemals in den Rust-Code** |
| Ein Item / Titan / Perk / Einsatz | ein Eintrag in `titans.ron` / `gear.ron` / `perks.ron` / `missions.ron` |
| Eine Skizze / ein Bild | bleibt hier, wird aus `docs/gameplay/` verlinkt |
| Etwas Unklares | `docs/FRAGEN.md` — nicht raten |
