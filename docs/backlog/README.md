<!-- GENERIERT von tools/features.py aus gameplay/features.xlsx, Blatt 00_Anleitung. -->

# docs/backlog/ — die Excel-Blaetter als Daten

Ein RON pro Blatt, weil die Blaetter verschiedene Spalten haben. Die `.xlsx`
selbst bleibt liegen und unangetastet — sie ist die Quelle, und der User
arbeitet darin weiter (prompts/init.md §2).

## Was in der Anleitung des Backlogs steht


### Defeated by Titan — Produktions-Backlog

Vollstaendige To-do-Liste aller Spielfunktionen und Assets. Jede Zeile ist ein Ticket. Zielplattform: PC, Tastatur und Maus, kooperativer Mehrspieler.

- **BLATT** — INHALT
- **01_Spielfunktionen** — Alle Features mit ID, Beschreibung, Akzeptanzkriterium, Prioritaet, Aufwand und Abhaengigkeit.
- **02_3D-Assets** — Alle Modelle mit Beschreibung, Polygonbudget je Detailstufe, Variantenzahl und Textur-Slot.
- **03_Animationen** — Alle Animationsclips je Rig, mit Beschreibung, Dauer und Loop-Verhalten.
- **04_Texturen** — Alle Texturen und Atlanten mit Aufloesung, Typ und Verwendungszweck.
- **05_VFX** — Alle visuellen Effekte mit Technik und Ausloeser.
- **06_Audio** — Alle Sound- und Musikassets mit Beschreibung, Laenge und Loop-Verhalten.
- **07_UI-Screens** — Alle Bildschirme mit den jeweils enthaltenen Elementen.
- **08_Maps** — Alle Level mit Beschreibung, unterstuetzten Modi, Groesse und Ankerdichte.
- **09_Tech-Backlog** — Technische Aufgaben: Pipeline, Netzwerk, Performance, Daten, Betrieb, QA.
- **10_Namensschema** — Uebersetzung der Referenzbegriffe in die eigene Spielwelt.
- **11_Zusammenfassung** — Automatische Auszaehlung nach Prioritaet und Aufwand.
- **PRIORITAET** — BEDEUTUNG
- **Must** — Ohne dieses Element ist das Spiel nicht auslieferbar. Blockiert den Release.
- **Should** — Wichtig fuer Qualitaet und Wettbewerbsfaehigkeit. Kann in ein Folge-Update, wenn die Zeit knapp wird.
- **Could** — Wertsteigerung ohne Blockade. Erste Streichkandidaten bei Terminkonflikt.
- **STATUS-WERTE** — Offen / In Arbeit / Review / Fertig / Zurueckgestellt / Gestrichen
- **AUFWAND** — In Personentagen (PT). Schaetzung fuer eine erfahrene Fachkraft der genannten Disziplin, inklusive Iteration.
- **PLATTFORM** — PC ausschliesslich. Tastatur und Maus als einziges Eingabegeraet. Kein Mobile, kein Gamepad, kein Touch. Zwei Qualitaetsprofile: Mindestprofil (Einsteiger-Laptop) und Vollprofil.
- **MEHRSPIELER** — Kooperativ. 20 Spieler pro Missionsinstanz, 10 pro Raidinstanz, 40 im Hub. Kein Schaden und keine Kollision zwischen Spielern, getrennte Beute pro Spieler.
- **WICHTIGSTE REGEL** — Kein Meta-System vor bestandenem Vector-Gear-Gate. Erst wenn die Bewegung im Blindtest ueberzeugt, wird gebaut.

## Die Blaetter und ihre Dateien

| Blatt | Zeilen (inkl. Kopf) | Datensaetze | Datei |
|---|---|---|---|
| `00_Anleitung` | 28 | — | — (berechnet, nicht uebertragen) |
| `01_Spielfunktionen` | 197 | 194 | `funktionen.ron` |
| `02_3D-Assets` | 103 | 100 | `modelle.ron` |
| `03_Animationen` | 103 | 100 | `animationen.ron` |
| `04_Texturen` | 31 | 28 | `texturen.ron` |
| `05_VFX` | 41 | 39 | `vfx.ron` |
| `06_Audio` | 121 | 118 | `audio.ron` |
| `07_UI-Screens` | 47 | 45 | `ui.ron` |
| `08_Maps` | 15 | 12 | `maps.ron` |
| `09_Tech-Backlog` | 53 | 51 | `tech.ron` |
| `10_Namensschema` | 43 | 40 | `namensschema.ron` |
| `11_Zusammenfassung` | 18 | — | — (berechnet, nicht uebertragen) |

**Die Zeilenzahl ist der Pruefwert.** `python3 tools/features.py --pruefen`
faellt um, wenn eine Zahl nicht mehr stimmt — dann ist die Extraktion nicht
fertig, und man weiss genau, wie viele Zeilen fehlen (prompts/init.md §2, §9).

`docs/features.ron` ist die Arbeitsliste daraus: alle F-IDs aus
`01_Spielfunktionen` **und** alle T-IDs aus `09_Tech-Backlog`. Die T-Zeilen
sind mit drin, weil genau sie das Aufsetzen beschreiben — ohne sie haette
`docs/STATUS.md` keine Zeile fuer Fenster, Werkzeuge oder Tests. Das ist eine
benannte Abweichung von der Tabelle in `prompts/init.md` §2, wo
`features.ron` nur aus Blatt 01 gespeist wird.
