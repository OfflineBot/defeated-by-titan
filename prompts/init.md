# Auftrag: **Defeated by Titan** — ein 3D-Titanenkampfspiel in Bevy, von null

**Du liest `prompts/init.md` in `~/Documents/defeated-by-titan/`.** Es gibt noch keinen Code,
keine Assets, kein Git — das anzulegen ist dein erster Schritt. Vorhanden ist nur das Gerüst:
`init.md` im Wurzelverzeichnis (der Startknopf, der hierher zeigt), **`prompts/`** (diese Datei und
alles, was daneben liegt) und **`gameplay/`** (§2).

Diese Datei ist der **Initialprompt**: sie sagt, *was* gebaut wird, *wie* der Baum aussieht, *was
Bevy von dir braucht* und **wie du deinen Zustand dokumentierst, damit andere Agenten dir glauben
können**. Lösch sie nicht — sie wird der erste Commit und geht erst am Ende (§18).

> ## ⚠️ Zuerst: **lies ALLE Dateien in `prompts/`, bevor du eine Zeile Code schreibst**
>
> ```bash
> ls -la prompts/ && cat prompts/*.md
> ```
>
> Diese Datei ist **ein Teil des Auftrags, nicht der ganze**. Neben ihr können weitere
> Markdown-Dateien liegen (oder später dazukommen) — Nachträge, Präzisierungen, Design-Notizen,
> Korrekturen. **Erst alle zusammen ergeben das vollständige Bild.** Also:
>
> 1. **Alles in `prompts/` lesen**, dann `gameplay/` (§2), dann anfangen.
> 2. **Widerspruch?** Die **spezifischere und neuere** Datei gewinnt (Änderungsdatum vergleichen:
>    `ls -lt prompts/`). Bei echtem Konflikt: **nicht selbst entscheiden** → `docs/FRAGEN.md`, und
>    solange nach dieser Datei arbeiten.
> 3. **Die Rangfolge über alles:** `gameplay/` bestimmt den **Inhalt** des Spiels, `prompts/` das
>    **Handwerk** — Aufbau, Struktur, Regeln, Beweispflicht.
> 4. **Am Anfang jeder Sitzung neu nachsehen** (`ls -lt prompts/ gameplay/`). Der User legt
>    jederzeit etwas dazu, auch mitten in einer Sitzung. Was du gestern gelesen hast, ist nicht
>    alles.
> 5. **Schreib in `docs/README.md` eine Zeile, welche Prompt-Dateien du gelesen hast** (Name +
>    Änderungsdatum). Ein anderer Agent muss sehen können, auf welchem Auftragsstand du warst.
>
> ### Das Manifest von `prompts/`
>
> | Datei | was sie ist | Pflicht? |
> |---|---|---|
> | `init.md` | **diese Datei** — der Rahmen: Struktur, Regeln, Beweispflicht, Ziellinie | ja |
> | `DefeatedByTitan_Design-Bibel.md` | ⭐ **das WARUM**: Designpfeiler, Welt, Ton, Gegner-Philosophie, Phasenplan P0–P11, Kennzahlen, Risiken. **Sie gewinnt inhaltlich über diese Datei.** | ja |
> | **jede weitere `*.md` in `prompts/`** | Nachtrag, Präzisierung, Design-Notiz, Korrektur — vom User später hinzugefügt | **ja, alle** |
>
> **Es gibt keine optionale Datei in diesem Ordner.** Alles, was darin liegt, ist Teil des
> Auftrags und wird gelesen, *bevor* gebaut wird — auch was hier nicht namentlich steht, weil es
> erst nach dieser Zeile entstanden ist. Kommt während der Arbeit eine dazu, wird sie **sofort**
> gelesen und der Plan angepasst, nicht erst „nach der aktuellen Stufe".
>
> Und umgekehrt: **`init.md` ist nicht die Zusammenfassung der anderen.** Sie weiß nicht, was in
> ihnen steht. Wer nur sie liest, hat den Auftrag nicht gelesen.

> **Das Wichtigste in einem Satz:** am Ende von Tag 1 hängt ein Mensch an einem Haken, schwingt
> durch eine 3D-Szene und schneidet einem Titanen den Cortex auf — und in `docs/STATUS.md` steht
> für jede einzelne Sache ehrlich, auf welcher der **vier Stufen** sie steht.

---

## 1. Was das Spiel ist

Ein **3D-Lowpoly-Actionspiel über den Kampf gegen Titanen** (Attack on Titan). Du bist ein
Vanguard-Bergungsmann mit **Vector Gear** (VG, deutsch: Vektorgeschirr): zwei Greifhaken, zwei Gastanks, zwei
Klingen. Du hakst ein, schwingst, beschleunigst mit Gas und tötest einen Titanen **nur** durch
einen schnellen Schnitt in den **Cortex**. Alles andere kostet ihn ein Bein und dich Zeit.

**Vorbild:** [Attack on Titan Revolution](https://www.roblox.com/games/13379208636/Attack-on-Titan-Revolution)
(Roblox). Was daran zu übernehmen ist:

| Baustein | Was gemeint ist |
|---|---|
| **Vector Gear als Kern** | Haken schießen, Seil einholen, Schwungenergie, Gas-Boost, Boost-Dash, Wandlauf. Das Spiel steht und fällt mit diesem Gefühl — nicht mit der Titanen-KI. |
| **Der Cortex ist die einzige Wahrheit** | Ein Cortex-Treffer tötet, egal wie voll der Titan ist. Alles andere ist Vorbereitung: Beine ab = er fällt, Arme ab = er kann nicht greifen, Augen = er sieht dich nicht. |
| **Schaden kommt aus Geschwindigkeit** | Ein Schnitt aus dem Stand kratzt. Derselbe Schnitt aus 30 m/s tötet. Die Formel gehört in die RON, nicht in den Code. |
| **Wirtschaft statt Cooldowns** | Gas ist endlich, Klingen werden stumpf und brechen. Nachladen an Versorgungspunkten / vom Pferd / an gefallenen Kameraden. |
| **Titanen-Typen** | acht Typen mit Namen aus Blatt 10: **Husk** (Grundlage), **Errant** (unberechenbar), **Scuttler** (schnell, Sprung), **Weaver** (Ausweichrolle mit I-Frames), **Warden** (schützt den Cortex mit der Hand), **Lurker** (Hinterhalt, Luftgriff), **Bellower** (ruft Verstärkung, reagiert auf Gas-Geräusch), **Chorus** (paarweise). Dazu vier Raid-Bosse. |
| **Missionen / Raids** | Ein Einsatz hat Ziele und Phasen: Titanen räumen, einen Trupp eskortieren, ein Tor halten, ein Boss mit Phasenwechsel. |
| **Progression in Daten** | XP, **Mark**/**Sigil**, Gear-Stufen, **Traits**, **Lineage**-Passive. **Alles Zahlen in RON-Dateien**, kein Balancing in Rust. |

**Und es wird ein Mehrspieler-Spiel** (Koop-Einsätze, wie beim Vorbild). Der Netzcode wird nicht
heute gebaut — **die Architektur wird aber von Anfang an dafür gebaut**, sonst ist Multiplayer
später ein Umbau des ganzen Spiels statt ein Zubau. Wie genau: **§6**, und das ist keine Kür.

**Später, nicht jetzt:** **Bonding**/**Vessel Forms** (selbst ein Titan werden), Pferde, **Lance Charge**.
Schreib sie in `docs/ROADMAP.md`, bau sie nicht.

**Assets in der Entwicklung: Platzhalter sind erlaubt, auch heruntergeladene.** Der User ersetzt
am Ende **alle** Modelle, Texturen und Klänge selbst — bis dahin zählt, dass der Prototyp gut wird,
nicht dass jedes Polygon von uns ist. Also: **fremde Assets dürfen als Platzhalter benutzt werden**
(§7 sagt, wo sie liegen und wie sie später in einem Zug austauschbar bleiben). Was du selbst baust,
ist trotzdem der Normalfall — ein Titan ist ein Mensch mit falschen Proportionen, und das sind drei
Primitive. **Die Stilregeln der Bibel gelten für Platzhalter genauso** (Low Poly, flache Farben,
die drei Signalfarben nur für Gameplay): ein Platzhalter, der stilistisch aus dem Rahmen fällt,
verfälscht genau das Urteil, für das der Prototyp da ist.

---

## 2. ⭐ Die Gameplay-Quelle: der Ordner `gameplay/`

**Im Ordner `gameplay/` liegt, WAS gebaut werden soll.** Es liegt schon da:
**`gameplay/features.xlsx`** — der Produktions-Backlog, **12 Blätter, ~790 Ticketzeilen**, jede
Zeile ein Ticket mit ID, Beschreibung, Akzeptanzkriterium, Priorität (MoSCoW), Aufwand und
Abhängigkeit. Dazu kommen weitere Dateien des Users (Skizzen, Notizen). `gameplay/README.md` sagt,
wie der Ordner gedacht ist.

> **Er ist die Autorität für den Inhalt. Diese Datei ist nur die Autorität für das Handwerk.**
> Wenn `gameplay/` etwas anderes sagt als §1 oder als der Code: **`gameplay/` gewinnt.** Ein
> Widerspruch *innerhalb* des Ordners ist keine Entscheidung, die dir gehört → `docs/FRAGEN.md`.

- **Erste Handlung jeder Sitzung: `ls -R gameplay/` und alles Neue lesen.** Der Ordner kann
  jederzeit wachsen, auch mitten in einer Sitzung. Was du gestern gelesen hast, ist nicht alles.
- **Nichts darin wird gelöscht, überschrieben oder abgehakt.** Das ist der Eingangskorb des Users,
  nicht dein Arbeitsblatt. Hinzufügen darfst du, fremden Text ändern nie. **Abgehakt wird in
  `docs/STATUS.md` und `docs/TODO.md`.**
- **Keine Zeile darf verschwinden.** Was dir unnötig, widersprüchlich oder unmöglich erscheint,
  kommt als Frage nach `docs/FRAGEN.md` — nicht in den Müll und nicht in ein stilles „habe ich
  weggelassen".

### Die zwei Dokumente, die es schon gibt — und wie sie zu dieser Datei stehen

| Datei | was darin steht | Rang |
|---|---|---|
| `prompts/DefeatedByTitan_Design-Bibel.md` | **das WARUM**: fünf Designpfeiler, Welt/Ton/Stil, Plattform, Mehrspieler-Grundregeln, Gegner-Philosophie, zehn Verbesserungen gegenüber der Referenz, **Phasenplan P0–P11**, Kennzahlen, Risiken, offene Entscheidungen | **inhaltlich über dieser Datei** |
| `gameplay/features.xlsx` | **das WAS**: 12 Blätter, ~790 Ticketzeilen — Spielfunktionen, 3D-Assets, Animationen, Texturen, VFX, Audio, UI-Screens, Maps, Tech-Backlog, **Namensschema**, Zusammenfassung | **die Arbeitsvorlage** |
| `prompts/init.md` (hier) | **das WIE**: Engine, Ordnerstruktur, Beweispflicht, Normung, Werkzeuge, Ziellinie | Handwerk |

**Regel:** Bibel und Backlog bestimmen *was und warum*, diese Datei *wie*. Bei inhaltlichem
Widerspruch gewinnen Bibel/Backlog; beim Handwerk (Struktur, Beleg, Normung) gewinnt diese Datei.
Ein Konflikt, der sich nicht so auflösen lässt, geht nach `docs/FRAGEN.md`.

### ⚠️ Der eine harte Widerspruch: **die Engine**

Die Design-Bibel ist an sechs Stellen für **Roblox** geschrieben (Rojo, ProfileStore, „Places",
Future Lighting, Plattform-Moderation, Store). **Dieses Projekt ist aber Bevy/Rust** — so vom User
gesetzt, und das ist der Grund, warum diese Datei existiert. **Die Engine-Entscheidung steht:
Bevy.** Alles andere in der Bibel bleibt gültig; die Roblox-Bezüge werden **übersetzt**, nicht
befolgt:

| Bibel (Roblox) | Hier (Bevy/Rust) |
|---|---|
| Rojo, Studio-Konflikte, Service-Framework (P0) | `cargo`, Git, Domänen-Plugins (§5) — der P0-Gate-Satz gilt trotzdem: ein Entwickler muss bauen, testen und mergen können |
| ProfileStore, Session-Lock, Transaktionsprotokoll | `save/` mit derselben **Anforderung** (kein Datenverlust, keine Duplikation) — die Umsetzung ist unsere |
| „Places", Instanzen, Hub als Place | Bevy-`States` + Szenen; „Instanz" heißt hier Server-Sitzung (§6) |
| Future Lighting, Farbatlas, Fernnebel | Bevy-PBR + `DirectionalLight` + Fog; **der Stil bleibt exakt** (Low Poly, weiche Normalen, flache Farbflächen) |
| Roblox-Store, Robux, Saisonpass | **offene Frage** → `docs/FRAGEN.md`. Nichts davon wird gebaut, solange nicht geklärt ist, ob es außerhalb von Roblox überhaupt gilt. |
| Plattform-Moderation (kein Splatter) | bleibt als **Stilregel** (Titanen verdampfen, Dampf statt Blut) — sie war ohnehin doppelt begründet |

**Und die Zahlen der Bibel, die technisch bindend sind, gelten unverändert:** PC-only, Tastatur und
Maus · 60 fps auf Mindest- **und** Vollprofil · 20 Spieler pro Einsatz, 10 pro Raid, 40 im Hub ·
Ausholphase jedes Angriffs ≥ 0,4 s · Missionsbogen 5–7 min · Cortex aus 100 m erkennbar · die drei
Signalfarben (Zyan/Bernstein/Karminrot) ausschließlich für Gameplay.

### Blatt `10_Namensschema` ist **verbindlich** — für Code, Assets, UI und Doku

Das Backlog übersetzt jeden Referenzbegriff in die eigene Welt, und diese Datei ist bereits darauf
umgestellt. Die wichtigsten, damit nichts zurückrutscht:

| statt | **hier** |
|---|---|
| ODM Gear / 3DMG | **Vector Gear** (VG) → Domäne `src/vector/` |
| Nape / Nacken | **Cortex** → das Empty im Modell heißt `cortex` |
| Scouts / Soldat | **The Vanguard** / Vanguard |
| Thunder Spear | **Lance Charge** |
| Titan Shifting / Shifter-Form | **Bonding** / **Vessel Form** |
| Family · Perk · Artifact · Memory · Prestige | **Lineage** · **Trait** · **Relic** · **Echo** · **Ascension** |
| Gold · Gems | **Mark** · **Sigil** |
| Pure/Abnormal/Crawler/Ducker | **Husk** · **Errant** · **Scuttler** · **Weaver** |
| (neu) | **Warden** · **Lurker** · **Bellower** · **Chorus** |
| Maps | **Ashgate District** · **Brackwall** · **The Fallow** · **Titanwood** · **Hollowkeep** · **Saltpier** · **Highspire**, Hub: **The Rookery** |

**Kein Referenzbegriff mehr im Code.** Ein `nape`-Feld oder ein `odm`-Modul ist ein Fehler, den ein
`grep` finden muss — nimm ihn in `tools/normen.py` (§10) auf.

### Die Status-Spalte des Backlogs ↔ die vier Stufen (§8)

Das Backlog hat eigene Statuswerte. Sie werden **eindeutig** abgebildet, damit nicht zwei Systeme
nebeneinander laufen:

| Backlog | Stufe | wer setzt es |
|---|---|---|
| `Offen` | ⬜ nicht implementiert | — |
| `In Arbeit` | 🟨 halb (gebaut, ungetestet/ungesehen) | Claude |
| `Review` | 🟧 fast (Tests + gesehen, Beleg vorhanden) | Claude |
| `Fertig` | ✅ fertig | **nur der User** |
| `Zurückgestellt` / `Gestrichen` | bleibt ⬜, mit Vermerk | User |

**Und die MoSCoW-Spalte ist die Reihenfolge:** `Must` vor `Should` vor `Could`. Bei Terminkonflikt
fallen zuerst alle `Could` — so steht es in der Bibel, und es ist keine Empfehlung.

### Die 12 Blätter → `docs/backlog/<blatt>.ron`

Ein RON pro Blatt (nicht alles in eine Datei — die Blätter haben verschiedene Spalten), plus
`docs/features.ron` als der Auszug, an dem gearbeitet wird. **Die Zeilenzahlen sind der Prüfwert
(§9)** — stimmt die Zahl nicht, ist die Extraktion nicht fertig:

| Blatt | Zeilen (inkl. Kopf) | wird zu |
|---|---|---|
| `01_Spielfunktionen` | 197 | `docs/backlog/funktionen.ron` → **Quelle von `docs/features.ron`** |
| `02_3D-Assets` | 103 | `docs/backlog/modelle.ron` → speist `tools/blend/*` (§7) |
| `03_Animationen` | 103 | `docs/backlog/animationen.ron` |
| `04_Texturen` | 31 | `docs/backlog/texturen.ron` |
| `05_VFX` | 41 | `docs/backlog/vfx.ron` |
| `06_Audio` | 121 | `docs/backlog/audio.ron` |
| `07_UI-Screens` | 47 | `docs/backlog/ui.ron` |
| `08_Maps` | 15 | `docs/backlog/maps.ron` (**Ankerdichte** ist die wichtigste Zahl darin) |
| `09_Tech-Backlog` | 53 | `docs/backlog/tech.ron` |
| `10_Namensschema` | 43 | `docs/konventionen.md` (Begriffe, siehe oben) |
| `11_Zusammenfassung` | 18 | **nicht** übertragen — sie ist berechnet; unsere Zahlen kommen aus `features.ron` |
| `00_Anleitung` | 28 | `docs/backlog/README.md` (Prio-/Status-Bedeutung) |

**Achtung, Formeln:** Blatt 11 besteht aus Formeln. Ohne `data_only=True` liest du `=COUNTIF(...)`
statt einer Zahl — und ohne einmaliges Öffnen in Excel/LibreOffice kann der zwischengespeicherte
Wert sogar fehlen. Dann **selbst nachzählen**, nicht schätzen.

### Die Phasen der Bibel schlagen den Stufenplan (§13)

Der Stufenplan in dieser Datei ist nur das **Aufsetzen**: bis das Spiel läuft, ein Modell drin ist
und die Werkzeuge stehen. Ab da gilt der Phasenplan der Bibel — und vor allem **seine harte Regel:**

> **Kein Meta-System vor bestandenem Vector-Gear-Gate.**
> Fähigkeitsbaum, Wirtschaft, Lineages, Raids, Kosmetik werden **nicht angefangen**, solange sich
> die Bewegung nicht überzeugend anfühlt (P1-Gate: Blindtest, mindestens gleichauf mit der
> Referenz).

| Bibel-Phase | entspricht hier |
|---|---|
| **P0 Setup** | Stufe 0a/0b + 1 + 1b (§13) — inkl. Namenskonventionen (§10) und Blender-Vorgaben (§7) |
| **P1 Vector Gear** | Stufe 3 — **und hier wird nicht weitergegangen, bevor das Gate steht** |
| **P2 Kampf-Kern** | Stufe 4 (ein Titan, voller Zyklus, Cortex, Klingenhaltbarkeit) |
| **P3 Erste Map** | Stufe 2 wird zu *Ashgate District* als Graybox mit getunter Ankerdichte |
| **P4+** | direkt aus dem Backlog, in MoSCoW-Reihenfolge |

### ⭐ Die Feature-Liste ist eine **Excel-Datei** — sie wird ausgelesen, nicht abgetippt

**Die `.xlsx` bleibt liegen und bleibt unangetastet** — sie ist die Quelle, und der User arbeitet
darin weiter. **Aber sie ist kein Arbeitsformat:** man kann sie nicht greppen, nicht diffen, nicht
aus einem Test heraus lesen und nicht mit einem Subagenten teilen. Also gilt **beides**: die Datei
**behalten** *und* in ein Format **extrahieren**, mit dem man maximal effektiv arbeiten kann.

**Auslesen per Skript, niemals von Hand.** Bei hunderten Zeilen verliert Abtippen garantiert
Zeilen, und niemand merkt welche:

```bash
python3 -c "import openpyxl" || pip install --user openpyxl     # oder pandas
# Notausgang ohne Python-Paket: libreoffice --headless --convert-to csv --outdir /tmp <datei>.xlsx
```

```python
import openpyxl
wb = openpyxl.load_workbook("gameplay/features.xlsx", data_only=True)  # data_only: WERTE, nicht Formeln
for ws in wb.worksheets:          # ⚠️ ALLE Blätter, nicht nur das erste
    print(ws.title, ws.max_row, ws.max_column)
```

**Die Excel-Fallen, und alle sehen nach „vollständig gelesen" aus:**

| Falle | Was passiert |
|---|---|
| **Mehrere Blätter** | Das zweite Blatt ist oft die Hälfte des Spiels. `wb.worksheets` durchlaufen, nicht `wb.active`. |
| **Formeln statt Werte** | Ohne `data_only=True` steht in der Zelle `=B2*1.5` statt der Zahl. |
| **Bedeutung in der FARBE** | Ein farbcodierter Prioritäts- oder Status-Spalte ist in jedem Textexport **unsichtbar**. Fällt dir eine Spalte auf, deren Sinn in der Formatierung steckt (Füllfarbe, Durchstreichung, fett) → `cell.fill.start_color.rgb` lesen **oder** in `docs/FRAGEN.md` fragen. **Nicht ignorieren.** |
| **Verbundene Zellen** | Eine Überschrift über fünf Spalten liefert vier leere Werte. `ws.merged_cells` prüfen. |
| **Ausgeblendete Zeilen/Spalten** | Sind trotzdem Inhalt. |
| **Leerzeilen am Ende / Kommentare in Zellen** | `max_row` lügt gern; `cell.comment` enthält manchmal die eigentliche Anforderung. |

**Der Beweis, dass nichts verlorenging, ist eine Zahl:** *Zeilen in der Tabelle (ohne Kopf und
Leerzeilen) == Einträge im extrahierten Format.* Diese Zählung steht in `docs/TODO.md` und als
**Test**. Stimmt sie nicht, ist die Extraktion nicht fertig — und du weißt genau, wie viele Zeilen
fehlen, statt es zu ahnen (§9).

### Das Zielformat: **`docs/features.ron` + generierte Ansichten**

```ron
// docs/features.ron — die Feature-Liste als Daten. EINE Quelle, viele Ansichten.
features: [
    (id: "F-001", name: "Haken einschlagen", domain: "vector", stufe: Nicht,
     beschreibung: "Linke Maustaste schießt den Haken; er hält an jeder Fläche mit Normale.",
     abhaengt_von: [], quelle: "features.xlsx!01_Spielfunktionen!Z12", prio: 1),
    (id: "F-002", name: "Seil einholen", domain: "vector", stufe: Halb,
     beschreibung: "…", abhaengt_von: ["F-001"], quelle: "features.xlsx!01_Spielfunktionen!Z13", prio: 1),
]
```

**Warum RON und nicht Markdown:** greppbar, diffbar, aus Tests lesbar, aus einem Skript
erzeugbar — und dieselbe Disziplin wie der Rest des Projekts (§4). Bei hunderten Features ist eine
handgepflegte Markdown-Tabelle nach drei Tagen kaputt.

Daraus werden **Ansichten generiert**, nie parallel gepflegt (ein Schreiber pro Feld, §5):

- **`docs/TODO.md`** — nach Domäne gruppiert, in baubarer Reihenfolge (`abhaengt_von` respektiert).
- **`docs/STATUS.md`** — jede Zeile trägt ihre **`F-ID`** und ihre Stufe (§8).
- Erzeugt von einem kleinen Werkzeug (`src/bin/features.rs` oder `tools/features.py`), das man
  **jederzeit erneut laufen lassen kann**. Handarbeit in einer generierten Datei ist verloren —
  schreib das als Kopfzeile in jede erzeugte Datei.

**Die `F-ID` ist das Bindeglied durch das ganze Projekt:** sie steht in der Commit-Message
(`F-014: Gas-Verbrauch beim Boost`), im Testnamen (`f014_boost_verbraucht_gas`) und in der
STATUS-Zeile. Damit ist für **jede** Zeile der Excel-Tabelle in einem `grep` beantwortbar: ist das
gebaut, wo, und wie belegt?

**Und die Datei selbst:** die `.xlsx` wandert bei der Auflösung des Gerüsts (§18) nach
`docs/gameplay/features.xlsx` und wird **nie gelöscht**. Wenn der User eine neue Version einlegt:
**erneut extrahieren und die Extraktion diffen** — neue Zeilen kommen als ⬜ dazu, geänderte
Zeilen bekommen eine Notiz, **verschwundene Zeilen werden nicht stillschweigend gelöscht**, sondern
in `docs/FRAGEN.md` aufgelistet.

### Die Übersetzung ist die eigentliche Arbeit

| Was im Korb liegt | Wohin es geht |
|---|---|
| Die **Excel-Feature-Liste** | per Skript → `docs/features.ron` (jede Zeile eine `F-ID`) → generiert `docs/TODO.md` + `docs/STATUS.md`. Datei bleibt erhalten. |
| Ein **Feature / eine Mechanik** (aus Text/Excel) | `docs/gameplay/<thema>.md` (das Design: *warum so*, mit `F-ID`) **+ eine ⬜-Zeile in `docs/STATUS.md`** |
| Eine **Zahl / Balance** | in die passende `assets/data/*.ron` — **niemals in den Rust-Code** (§4) |
| Ein **Item / Titan / Trait / Einsatz** | ein Eintrag in `titans.ron` / `gear.ron` / `traits.ron` / `missions.ron` |
| Eine **Skizze / ein Bild** | bleibt in `gameplay/bilder/`, wird aus `docs/gameplay/` verlinkt |
| Etwas **Unklares** | `docs/FRAGEN.md` — nicht raten, nicht drumherum bauen und hoffen |

**`docs/TODO.md` ist die Landkarte, die daraus entsteht:** jede Zeile der TODO-Liste des Users
wird übernommen, mit **Domäne** und **Stufe** (am Anfang ⬜) — sortiert nach der Reihenfolge, in
der es baubar ist, nicht nach der Reihenfolge, in der es notiert wurde. Dazu ein Satz pro Zeile,
*warum* sie da steht, wo sie steht. Ein anderer Agent muss offene Arbeit finden können, **ohne**
den Eingangskorb zu lesen.

**Und: die TODO-Liste schlägt den Stufenplan (§14).** Der Stufenplan sagt, wie man von null zu
einem laufenden Spiel kommt; sobald die TODO-Liste da ist, sagt sie, was danach passiert. Bau nie
etwas Großes, das in keiner der beiden Listen steht — trag es zuerst ein.

---

## 3. Bevy — was es ist und was du dafür brauchst

**Bevy ist eine ECS-Engine in Rust.** Es gibt keine Klassenhierarchie und keinen Szenengraphen
im klassischen Sinn. Es gibt drei Dinge, und alles ist eines davon:

- **Entity** — eine ID. Ein Titan, eine Klinge, die Kamera, ein Haken.
- **Component** — Daten an einer Entity (`Transform`, `Gas`, `Cortex`, `Hooked`).
- **System** — eine Funktion, die pro Frame läuft und über Components abfragt (`Query`).

Dazu **Resources** (globaler Zustand: die geladene RON, die Uhr), **Messages** (Nachrichten
zwischen Systemen) und **States** (Menü / Laden / Einsatz). Ein **Plugin** ist ein Bündel aus
Systemen + Resources — und in diesem Projekt ist **ein Plugin genau eine Domäne** (§5).

### Das absolute Minimum, um ein Fenster zu sehen

```toml
# Cargo.toml
[package]
name = "defeated_by_titan"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = "0.19"          # ⚠️ prüfe die neueste Version, siehe unten
ron  = "0.11"          # die Datendateien
serde = { version = "1", features = ["derive"] }

# Ohne das ist ein Debug-Build UNSPIELBAR: Bevy selbst macht das Batching, die
# Transform-Propagation und das Rendern. Der eigene Crate bleibt billig zu kompilieren,
# jede Dependency wird auf voller Stufe gebaut. Das ist der Unterschied zwischen 20 und 200 fps.
[profile.dev]
opt-level = 1
[profile.dev.package."*"]
opt-level = 3
```

```rust
// src/main.rs
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Defeated by Titan".into(),
                ..default()
            }),
            ..default()
        }))
        .run();
}
```

### Was du für **3D** zusätzlich brauchst (und was oft vergessen wird)

`DefaultPlugins` bringt 3D schon mit (`bevy_pbr`, glTF-Lader, PBR-Materialien). Du brauchst
**keine** Extra-Dependency dafür. Was in der Szene liegen muss, damit man überhaupt etwas sieht:

1. **Eine Kamera** — `Camera3d` + `Transform`. Ohne sie ist das Bild leer und wirkt wie ein Bug.
2. **Licht** — mindestens eine `DirectionalLight` (die Sonne) und ein Wert für `AmbientLight`.
   PBR ohne Licht ist schwarz. Schatten kosten viel — **erst zum Schluss einschalten und messen**.
3. **Etwas Sichtbares** — `Mesh3d(handle)` + `MeshMaterial3d(handle)`. Meshes aus
   `bevy::math::primitives` (`Cuboid`, `Sphere`, `Capsule3d`, `Cylinder`, `Plane3d`) oder aus
   `.glb` per `asset_server.load("modell.glb#Scene0")`.
4. **Ein Achsen-Vertrag, den du EINMAL festlegst und aufschreibst:** in Bevy zeigt **+Y nach
   oben**, die Kamera schaut standardmäßig nach **−Z**. Schreib in `docs/konventionen.md`, wohin
   ein Modell blickt (Vorschlag: Gesicht nach **−Z**, `yaw = 0` = Blick nach −Z) und richte jedes
   Modell **im Blender-Datei**, nicht per Offset in der Config. Ein Offset-Feld pro Modell ist der
   Anfang von dreißig Offset-Feldern.
5. **Eine Einheit.** Lege fest: **1 Bevy-Einheit = 1 Meter.** Ein Mensch ist 1,8, ein Titan 3–15,
   ein Haken fliegt 60–120. Wenn du eine Konstante anlegst, die Meter misst: **Einheit in den
   Namen oder den Kommentar an die Rechenstelle**, Zahl in die RON.

### Bevy-Fallen, die dich sonst einen halben Tag kosten

- ⚠️ **Bevys API dreht sich zwischen Minor-Versionen hart.** `cargo add bevy` und dann in der
  echten Doku der **installierten** Version nachsehen (`cargo doc --open -p bevy`), nicht aus
  dem Gedächtnis oder aus Blogposts von vor zwei Versionen schreiben. Namen, die zuletzt
  gewandert sind: `Event` → **`Message`** für gepufferte Nachrichten (`MessageWriter`/
  `MessageReader`), `SpriteBundle`/`PbrBundle` → **einzelne Komponenten** (`Mesh3d`,
  `MeshMaterial3d`), `time.delta_seconds()` → `delta_secs()`. **Prüfen, nicht annehmen.**
- **`add_plugins((..))` nimmt maximal ~15 Elemente pro Tupel**, ein System maximal ~16
  Parameter. Beides schlägt als unlesbarer Trait-Fehler zu. Lösung: verschachteln
  (`((A, B), C, …)`) bzw. Parameter in ein `SystemParam`-Struct bündeln.
- **Commands sind verzögert.** Was du diesen Frame spawnst, existiert erst am Ende des Frames.
  Ein Test/Skript, das spawnt und im selben Atemzug prüft, prüft ins Leere.
- **`cargo run`, NIE `./target/debug/<name>`.** Das nackte Binary sucht `assets/` relativ zum
  Arbeitsverzeichnis und findet nichts: leere Welt, keine Fehlermeldung, sieht exakt wie ein
  Render-Bug aus.
- **Audio:** Bevys Default-Decoder ist Vorbis allein. Wenn du `.wav` benutzt:
  `bevy = { version = "…", features = ["wav"] }` — sonst lädt jeder Klang fehlerfrei und spielt
  Stille.
- **RON + Serde:** RON kennt kein `include`. Wenn eine Datendatei zu groß wird, splittest du sie
  **in Rust** (mehrere Dateien einlesen und zusammenfügen), nicht in RON.

---

## 4. Die Datenregel: **Zahlen gehören in RON, nicht in Rust**

Ein neuer Titan-Typ, eine Klingenstufe, ein Trait, eine Missionsvorlage, eine Gas-Kostenzahl:
**Datei-Arbeit, kein Rust.** Im Code stehen nur *Einheiten* und *Mechanik*.

```
assets/data/
  game.ron       Tuning: Vector Gear (Hakenreichweite, Seilzug, Gas, Boost), Kamera, Physik
  titans.ron     die Titanen-Typen (Größe, Tempo, Regeneration, Cortex-Größe, KI-Profil)
  gear.ron       Klingen, Tanks, Haken, Upgrade-Stufen und ihre Kosten
  traits.ron      Traits + Lineage-Passive (Relics, Echos)
  missions.ron   Einsatzvorlagen: Ziele, Phasen, Spawn-Wellen, Belohnung
  art.ron        Modellpfade, Farben, Lowpoly-Teilelisten
```

**Warum das nicht optional ist:** Balancing ist die Arbeit, die am häufigsten passiert. Wenn sie
ein Rebuild braucht, passiert sie nicht. Und: ein anderer Agent kann eine RON-Zeile ändern, ohne
deinen Code zu verstehen.

**Kein `serde(default)` für Spielwerte.** Ein fehlender Wert soll beim Laden **krachen**, nicht
still eine Null einsetzen — sonst suchst du den Bug im Code, während er in der Datei sitzt.

---

## 5. Die Ordnerstruktur: **eine Domäne = ein Ordner = ein Plugin = standalone**

### Der ganze Baum auf einen Blick

Damit klar ist, **wo was hingehört** — und damit nichts irgendwo daneben landet (§10: keine
Zombie-Dateien):

```
defeated-by-titan/
  Cargo.toml            das Paket heißt defeated_by_titan (§13)
  CLAUDE.md             das Gedächtnis für die nächste Sitzung — INDEX, kein Archiv (§8)
  README.md             was das Spiel ist, wie man es startet, Tasten, Stand
  .gitignore            target/ saves/ *.blend1 *.log assets/extern/
  src/                  eine Domäne = ein Ordner = ein Plugin (unten)
  tests/                tests/<domäne>.rs — plus domaenen.rs, mehrspieler.rs, modelle.rs, assets.rs
  assets/               data/ 3d/ textures/ audio/ vfx/ extern/    → §7
  tools/                blend/ atlas/ sound/ + normen.py, features.py, hole_extern.sh
  scripts/              --script-Fahrten: <f-id>-<kurz>.txt        → §12
  docs/                 der Spiegel von src/ + alles, was gepflegt wird:
    README.md             Index: eine Zeile pro Doku-Datei
    STATUS.md  TODO.md    Fortschritt (vier Stufen) und offene Arbeit — GENERIERT aus features.ron
    features.ron          die Feature-Liste als Daten (F-IDs)       → §2
    backlog/*.ron         ein RON pro Excel-Blatt                    → §2
    gameplay/             das Design pro Thema (+ referenzen.md, bilder/)
    lessons/              Fallgeschichten: was Zeit gekostet hat
    architektur.md  konventionen.md  modelle.md  multiplayer.md  umgebung.md
    BUGS.md  FRAGEN.md  FUNDE.md  ABNAHME.md  ROADMAP.md
    bilder/               Screenshots als Beleg: <f-id>-<kurz>.png
  prompts/  gameplay/   ⚠️ Bootstrap-Gerüst — wird am Ende aufgelöst (§18)
```

**Die drei Trennungen, die man nicht verwischen darf:** `tools/` baut Dinge (Blender, Atlas, Klang,
Prüfer) — `scripts/` *spielt* das Spiel (`--script`-Fahrten) — `assets/` enthält das Ergebnis, und
`assets/extern/` allein enthält Fremdes.

```
src/
  main.rs                     nur: App bauen, Plugins in Abhängigkeitsreihenfolge, Flags lesen
  lib.rs                      pub mod je Domäne (damit tests/ dagegen bauen können)

  shared/    (kein Plugin)    Typen, die niemandem gehören: Health, Meter, Achsen-Helfer
  data/      DataPlugin       RON laden → GameData + Handles. Läuft VOR allem anderen.
  save/      SavePlugin       Spielstand: Profil, Gear-Budget, Traits, Lineage, Fortschritt
                              (Anforderung der Bibel: kein Verlust, keine Duplikation — §2)
  world/     WorldPlugin      die Maps (Ashgate District …), Bastionsringe, Häuser, Titanwood;
                              Ankerpunkte, Kollision, räumlicher Index (§9)
  render/    RenderPlugin     Kamera, Licht, Himmel, Meshes bauen, Modelle laden
  player/    PlayerPlugin     der Körper: laufen, springen, Boden-Kollision, Zustandsmaschine
  vector/    VectorPlugin     ⭐ DER KERN (Vector Gear): Haken, Seil, Schwung, Gas, Boost, Wandlauf
  blades/    BladesPlugin     Klingen: Schwung, Abnutzung, Bruch, Wechsel, Nachschub
  titan/     TitanPlugin      Titanen: Rig, Gliedmaßen, Cortex, KI (suchen/greifen/beißen)
  combat/    CombatPlugin     Treffer: Raycast/Sweep, Schaden aus Geschwindigkeit, Amputation,
                              Dampf, Tod
  mission/   MissionPlugin    Einsatz: Ziele, Phasen, Spawn-Wellen, Sieg/Niederlage
  progress/  ProgressPlugin   XP, Mark/Sigil, Gear-Budget, Traits, Lineage, Ascension, Relics
  squad/     SquadPlugin      Mitspieler und Eskorte: Kampfunfähigkeit, Wiederbeleben,
                              Markieren, geteilte Ziele (Bibel: kein PvP-Schaden, keine Kollision)
  hud/       HudPlugin        Gas, Klingenzustand, Ziel-Marker, Fadenkreuz
  menu/      MenuPlugin       Hauptmenü, Pause, Optionen
  sound/     SoundPlugin      Gas-Zischen, Hakeneinschlag, Klingenschnitt, Titanenschritt
  net/       NetPlugin        ⭐ die Naht für Multiplayer (§6) — heute ein Stub mit dem
                              Transport `LocalOnly`, aber die Naht existiert ab Tag 1
  debug/     DebugPlugin      F3-Overlay, Gizmos, `--script`-Fahrer (§12)
```

### Was „standalone" konkret heißt — und wie es geprüft wird

1. **Jeder Ordner exportiert genau ein `pub struct XPlugin` mit `impl Plugin`.** Ein Ordner ohne
   Plugin ist `shared/` oder ein Fehler.
2. **Eine Domäne darf nur nach `shared`, `data` und Bevy greifen.** Sie darf **keine** Funktion
   einer anderen Domäne aufrufen. Wer eine Ausnahme braucht (`render` liest `world`), schreibt sie
   in eine **Erlaubnisliste** in `docs/architektur.md` — mit Begründung.
3. **Kommunikation läuft über Components und Messages.** `combat` schickt `TitanHit { entity,
   part, speed }`; `titan` liest es und entscheidet, was das für seinen Körper heißt. `combat`
   weiß nicht, wie ein Titan gebaut ist.
4. **Ein Feld hat genau einen Schreiber.** Wenn zwei Systeme dasselbe Feld setzen, gewinnt das
   zuletzt gelaufene — das ist kein Design, das ist ein Münzwurf mit 60 Hz. Schreib in die
   Doku der Domäne, **wer** ein geteiltes Feld schreibt; alle anderen lesen nur.
5. **Die Reihenfolge in `main.rs` ist die Abhängigkeitsreihenfolge:**
   `data → save → net → world → render → player → vector → blades → titan → combat → mission →
   progress → squad → hud → sound → menu → debug`.
6. **Diese Regel verfällt still** — nichts geht kaputt, wenn jemand doch quer greift. Also
   **schreib einen Test, der sie prüft**: `tests/domaenen.rs` liest die Dateien unter `src/`,
   sammelt jedes `use crate::<domäne>` und fällt um, wenn eine Kante nicht in der Erlaubnisliste
   steht. Das ist ~40 Zeilen und der einzige Grund, dass die Struktur in vier Wochen noch stimmt.

### Und `docs/` spiegelt `src/`

Eine Quelldatei = eine Doku-Datei. Neue Quelldatei = **neue Doku-Datei und eine Zeile in
`docs/README.md`**. Die Doku beschreibt **nicht, was der Code tut** (das steht im Code), sondern
**warum er so ist und wo die Fallen liegen**.

---

## 6. ⭐ Es wird **Multiplayer** — und das entscheidet die Architektur ab Tag 1

Koop-Einsätze: mehrere Vanguard im selben Einsatz gegen dieselben Titanen. **Der Netzcode ist
nicht Teil dieses Auftrags** — kein Server, keine Prediction, keine Lag-Kompensation heute. **Aber
jede Entscheidung, die Multiplayer später unmöglich oder teuer macht, wird heute vermieden.** Das
kostet jetzt fast nichts und später alles: ein fertiges Einzelspieler-Spiel netzfähig zu machen
heißt normalerweise, die Simulation neu zu schreiben.

**Die acht Regeln. Keine ist optional, und keine kostet heute mehr als fünf Minuten:**

1. **Simulation und Darstellung sind getrennt.** Die Simulation liest Eingaben + Zustand und
   schreibt Zustand. Rendering, HUD und Sound **lesen nur**. Ein System, das aus einem Mausklick
   direkt ein Mesh spawnt, ist der Anfang vom Ende — genau dieser Klick muss später vom Server
   bestätigt werden.
2. **Eingabe ist ein Datum, kein Tastendruck.** Es gibt **ein** Struct — `Intent` (Bewegungsvektor,
   Blickrichtung, Buttons, Tick-Nummer) — und die Simulation liest **nur** das. Wer es füllt, ist
   ihr egal: die lokale Tastatur, das `--script`-Harness (§12) oder später das Netz. **Genau dieser
   Kanal ist der, den Multiplayer braucht** — und du baust ihn in Stufe 1 sowieso, weil du nicht
   klicken kannst. Ein Aufwand, zwei Probleme gelöst.
3. **Es gibt keinen „den Spieler".** Nie `.single()` auf eine Spieler-Query. Jeder Spieler ist
   *einer von vielen*: `PlayerId`, und Gas/Klingen/Inventar sind **Components am Spieler**, nie eine
   globale `Resource`. Die Kamera hängt an einem `LocalPlayer`-Marker — das ist die **einzige**
   Stelle im Code, die weiß, welcher Spieler „ich" ist.
4. **Fester Simulationsschritt.** Zustand ändert sich in `FixedUpdate` (z. B. 60 Hz), das Bild
   interpoliert dazwischen. Dieselbe Regel wie §11 („nichts pro Frame"), nur strenger: im Netz ist
   ein frameabhängiges Ergebnis kein Komfortproblem, sondern **Desync**.
5. **Determinismus, wo er billig ist.** Zufall nur aus einem **geseedeten** Generator, dessen Seed
   Teil des Zustands ist (`seed + tick`), nie `rand::random()` mitten in einem System. Ein Titan,
   der auf zwei Rechnern anders abbiegt, ist ein Bug, den man nur im Netz sieht — also am
   teuersten Tag.
6. **Autorität wird benannt.** In der Doku jeder Domäne steht, **wer** ein geteiltes Feld schreibt
   (§5). Später heißt dieser Satz „der Server". Zwei Schreiber sind lokal ein Münzwurf mit 60 Hz —
   im Netz ein Auseinanderlaufen, das niemand reproduzieren kann.
7. **Stabile Ids statt Zeiger.** Alles, was gespeichert oder eines Tages verschickt wird, benutzt
   eigene Ids (`TitanId`, `PlayerId`), **nie** Bevys `Entity` — die ist ein lokaler Index mit
   Generation und bedeutet auf einem anderen Rechner etwas anderes. Dieselbe Regel rettet nebenbei
   den Spielstand.
8. **`serde` auf allem, was Zustand ist**, und **Messages so entwerfen, dass sie über eine Leitung
   passen** (Daten — keine Handles, keine Funktionszeiger, keine `Entity`). Kostet heute eine Zeile
   pro Typ.

**Die Naht heißt `src/net/`** und existiert ab Stufe 1: ein `NetPlugin`, das genau eines tut — den
Transport `LocalOnly` bereitstellen, der die Intents des lokalen Spielers in die Simulation schiebt.
Damit ist der Ort, an dem später Client und Server stehen, **vorhanden und leer**, statt später
mitten durch fünf Domänen zu schneiden.

### Was die Bibel dazu schon entschieden hat — das ist keine offene Frage mehr

| Vorgabe | Konsequenz für den Code |
|---|---|
| **Eigene Bewegung beim Client**, alles andere beim Server (Titanen, Ziele, Schaden, Beute) | die Trennung aus Regel 1 ist damit **vorgegeben**, nicht gewählt: Bewegung darf lokal sofort reagieren, ein Cortex-Treffer nie |
| **20 Spieler pro Einsatz, 10 pro Raid, 40 im Hub** | nichts skaliert mit „einem Spieler". Zwanzig Spieler mit je **zwei Seilen** plus sechzig Titanen sind die eigentliche Belastungsprobe — nicht die Grafik |
| **Kein Schaden, keine Kollision zwischen Spielern** | zwei Spieler müssen sich in voller Fahrt durchdringen können; Knockback bleibt |
| **Getrennte Beute pro Spieler** | Beute ist nie ein globaler Zustand |
| **Kampfunfähigkeit statt Sofort-Tod**, Wiederbeleben durch Mitspieler | „tot" ist ein Zustand mit Timer, kein Entfernen der Entity → gehört zu `squad/` |
| **Verbindungsabbruch reserviert den Platz 120 s** | die Sitzung überlebt den Spieler; sein Zustand hängt an einer `PlayerId`, nicht an einer Verbindung (Regel 7) |
| **T-019: jedes Bewegungsfeature wird bei 200 ms simulierter Latenz getestet** | **ein Verzögerungs-Schalter gehört ins Werkzeug** (§12), nicht in ein späteres Ticket: „fühlt sich lokal gut an" ist keine Abnahme |

**`docs/multiplayer.md`** hält den Plan, der noch nicht gebaut wird: Autoritätsmodell (dedizierter
Server vs. Host), wer die Titanen simuliert (der Server), was der Client vorhersagen darf (die
eigene Vector-Gear-Bewegung — sie muss sich sofort anfühlen) und was nie vorhergesagt wird (ein
Titanen-Tod). Die offenen Punkte kommen nach `docs/FRAGEN.md`: **wie viele Spieler, Koop oder auch
PvP, dedizierter Server oder Host** — Entscheidungen des Users, nicht deine.

**Der Wächter, der die Regel am Leben hält:** `tests/mehrspieler.rs` spawnt **zwei**
Spieler-Entities und lässt die Simulation ein paar Ticks laufen. Er fällt in der Sekunde um, in der
jemand `.single()` schreibt oder Spielerzustand in eine `Resource` legt. Ohne ihn verfällt dieser
Abschnitt still — und man merkt es erst, wenn Multiplayer dran ist, also nach Monaten Arbeit, die
man dann anfassen muss.

---

## 7. Die **Assets**: Modelle, Texturen, Klänge, VFX — Claude baut sie, **RON entscheidet**, du tauschst sie aus

**Du entscheidest frei, wie ein Asset entsteht** — Modell aus Bevy-Primitiven oder aus Blender,
Farbe über **Vertexfarben** oder einen **Atlas**, Klang synthetisiert, gerendert **oder
heruntergeladen**. Das ist dein Handwerk, nicht deine Fessel.

**Und es ist ein Prototyp: heruntergeladene Assets sind ausdrücklich erlaubt** (Modelle, Klänge,
Musik-Platzhalter — §12e sagt, woher und wie). Der User ersetzt am Ende alles selbst. **Genau
deshalb** ist die Trennung unten keine Formalität: sie entscheidet, ob dieses Ersetzen später eine
Checkliste ist oder eine Suche.

**Aber drei Dinge sind nicht verhandelbar:**

1. **Alles liegt im vorgesehenen `assets/`-Ordner** (Struktur unten) — nichts irgendwo daneben,
   nichts im Code eingebettet.
2. **Was benutzt wird, entscheidet die RON** — `assets/data/assets.ron` ist die Registratur. Ein
   Asset austauschen heißt: **eine Zeile ändern**, nicht Rust anfassen.
3. **Jedes Asset ist einzeln austauschbar** und hat einen **Platzhalter-Weg**, der weiter
   funktioniert. Der User öffnet eine `.blend`, baut das Ding richtig, setzt `nutzen: true` — und
   das Spiel benutzt es beim nächsten Start.

### Die Kette

```
tools/blend/<name>.py   ──►  assets/3d/blend/<name>.blend  ──►  assets/3d/glb/<name>.glb  ──►  art.ron
   Claude schreibt sie        DU öffnest und füllst aus          automatisch exportiert         der Schalter
```

### Der Assets-Ordner

```
assets/
  data/                 die RON-Dateien (§4)
  3d/
    blend/              ⭐ DIE QUELLE — von Hand editierbar, hierhin geht der User
      scout.blend             der Spielerkörper
      titan_normal.blend      Titan 5 m
      titan_gross.blend       Titan 15 m
      vector_gear.blend       Gurt, Tanks, Klingengriffe
      blade.blend
      haus_klein.blend        Stadtbausteine: Haus, Dach, Mauerstück, Tor
      baum_riese.blend
      kiste_nachschub.blend
    glb/                GENERIERT — **niemals von Hand anfassen**. Wird mitcommittet, damit
                        das Spiel auch ohne installiertes Blender läuft.
  textures/             Handgemachte PNGs (der Exporter fasst sie nie an)
  sound/
tools/
  blend/<name>.py       das Blender-Python-Skript, aus dem die .blend entsteht
```

**Warum ein Skript und nicht direkt eine `.blend`:** eine `.blend` ist ein Binärklumpen — im Git
sieht niemand, was sich geändert hat, und du kannst sie nicht schreiben, ohne Blender zu starten.
Ein Skript ist ein Diff, ist reproduzierbar, und ist der Ort, an dem *dein* Platzhalter lebt.

```bash
blender --background --factory-startup --python tools/blend/titan_normal.py
```

### Die Registratur: `assets/data/assets.ron` — **eine Zeile pro Asset, eine Zeile zum Tauschen**

Jedes Asset hat einen **logischen Namen**, den der Code kennt, und eine **Quelle**, die die RON
bestimmt. Der Code fragt nie nach einer Datei, immer nach einem Namen.

```ron
assets: {
    // --- Modelle: Quelle ist eine .blend (exportiert, §7) ODER prozedurale Primitive ---
    "titan_husk":  (modell: Blend("titan_husk"), farbe: VertexFarben,        nutzen: true,  scale: 1.0),
    "vanguard":    (modell: Blend("vanguard"),   farbe: VertexFarben,        nutzen: true,  scale: 1.0),
    "haus_klein":  (modell: Blend("haus_klein"), farbe: Atlas("umwelt"),     nutzen: true,  scale: 1.0),
    "kiste":       (modell: Primitive([Box((1.0,0.8,0.6), "#6b5a3e")]),      nutzen: true),

    // --- Klänge: Datei ODER Rezept; das Rezept liegt im Repo und ist reproduzierbar ---
    "sfx_hook_hit":(klang: Datei("audio/sfx/hook_hit.ogg"), lautstaerke: 0.8, nutzen: true),
    "sfx_gas":     (klang: Rezept("gas_zischen"), lautstaerke: 0.5, schleife: true, nutzen: true),

    // --- Platzhalter aus dem Netz: IMMER mit `herkunft` markiert (siehe unten) ---
    "titan_gross": (modell: Extern("extern/3d/titan_big.glb"), nutzen: true, scale: 1.0,
                    herkunft: "https://… · 2026-08-09 · CC0 · ERSATZ NÖTIG"),

    // --- Effekte ---
    "vfx_dampf":   (vfx: Rezept("dampf"), nutzen: true),
}
```

- **`nutzen: false`** ⇒ der **Platzhalter-Weg** (Primitive, stiller Klang, kein Effekt). Beide Wege
  müssen jederzeit laufen und dieselbe Größe/dasselbe Timing haben — sonst ist das Umschalten kein
  Schalter, sondern ein Umbau.
- **Kein Dateiname im Rust-Code.** Ein `asset_server.load("titan.glb")` mitten in einem System ist
  ein Fehler; es gibt **eine** Stelle, die die Registratur liest (`data/`), und alle anderen fragen
  nach dem logischen Namen. Nimm die Regel in `tools/normen.py` (§10) auf.
- **Ein fehlendes Asset kracht beim Laden**, mit Name und Zeile (§4: kein `serde(default)` für
  Spielwerte) — nicht still ein weißer Würfel mitten im Spiel.

### Eigen und **Extern** bleiben getrennt — damit das Ersetzen später eine Checkliste ist

Der Plan des Users ist: **am Ende ersetzt er alles selbst, Stück für Stück.** Dieser Plan
funktioniert genau dann, wenn zu jedem Zeitpunkt in **einem Befehl** beantwortbar ist: *was ist noch
fremd, wo liegt es, und was soll es werden?* Also:

1. **Fremdes liegt ausschließlich unter `assets/extern/`.** Nie in `3d/blend`, `3d/glb`, `audio/sfx`
   — dort liegt Eigenes. Diese eine Trennung ist der ganze Trick.
2. **`assets/extern/HERKUNFT.md` listet jede Datei**: Dateiname · URL · Datum · Lizenz, falls
   bekannt · **welches Eigen-Asset sie später wird**. Ohne Eintrag darf keine Datei dort liegen
   (das ist die Zombie-Regel aus §10, hier mit Zähnen).
3. **In der Registratur trägt jedes fremde Asset `herkunft:`.** Damit ist die Ersetzungsliste ein
   `grep`, und ein Bericht (`cargo test --test assets -- --ignored --nocapture`) druckt: *Asset ·
   eigen/extern · Quelle · ersetzt durch*. **Diesen Bericht will der User sehen**, wenn er anfängt
   zu ersetzen.
4. **Der Ersatz ist eine Zeile.** Weil der Code nur den logischen Namen kennt, wird aus
   `Extern("extern/3d/titan_big.glb")` irgendwann `Blend("titan_gross")` — und nichts anderes ändert
   sich. Genau dafür ist die Registratur da.
5. **Das öffentliche Repo bekommt die Fremddateien nicht.** `assets/extern/` steht im `.gitignore`;
   was mitkommt, ist `HERKUNFT.md` **und ein Holskript** `tools/hole_extern.sh`, das alles
   wiederbeschafft. Damit läuft der Prototyp auf jeder Maschine, das Repo bleibt sauber, und du
   verteilst nichts weiter, was dir nicht gehört. *(Ein öffentliches Repo ist Weitergabe, nicht
   Entwicklung — das ist der einzige Punkt, an dem die Prototyp-Freiheit eine Grenze hat.)*
6. **Stilbruch ist ein Bug, auch bei einem Platzhalter.** Ein hochdetailliertes Fremdmodell neben
   Low-Poly-Eigenbau verfälscht das Urteil über Bewegung und Lesbarkeit — also entweder
   vereinfachen (Decimate im Blender-Skript) oder ein anderes nehmen.

### Der `assets/`-Ordner — die vorgesehene Struktur

```
assets/
  data/                 alle RON: game.ron, titans.ron, gear.ron, traits.ron, missions.ron,
                        assets.ron  ← die Registratur (oben)
  3d/
    blend/              ⭐ QUELLE, von Hand editierbar (§7)
    glb/                GENERIERT — nie von Hand anfassen, wird mitcommittet
  textures/
    atlas/              GENERIERT: der EINE Umgebungs-Farbatlas (Bibel: garantierte
                        Farbkonsistenz + minimale Drawcalls) — aus tools/atlas/
    hand/               handgemachte PNGs — der Generator fasst sie nie an
  audio/
    sfx/                GENERIERT oder handgemacht: .ogg/.wav
    music/              Musikschichten — ausschliesslich original oder lizenziert
  vfx/                  Effekt-Definitionen (Daten, kein Code)
  extern/               ⭐ HERUNTERGELADENE PLATZHALTER — streng getrennt vom Eigenen
    3d/  audio/  textures/
    HERKUNFT.md         eine Zeile pro Datei: URL · Datum · Lizenz (falls bekannt) · was sie ersetzt
tools/
  blend/<name>.py       Modell-Skripte (§7)
  atlas/<name>.py       baut den Farbatlas + schreibt die UV-Zuordnung als RON
  sound/<name>.py       Klang-Rezepte: Skript -> .ogg/.wav, reproduzierbar
```

**Die Regel dahinter ist immer dieselbe Kette:** `Skript (Quelle im Repo) → generiertes Asset →
RON-Schalter → Spiel`. Sie gilt für ein Modell genauso wie für einen Klang oder einen Atlas. Wer
ein neues Asset-*Art* anfängt, baut die Kette mit — nicht „diesmal von Hand".

### Farbe: **Vertexfarben oder Atlas — du entscheidest, aber schreib es hin**

Beides ist erlaubt, und die Bibel gibt die Richtung: **die Umgebung läuft über EINEN Farbatlas**
(Farbkonsistenz, wenige Drawcalls), **Figuren und Titanen dürfen Vertexfarben** benutzen (überlebt
jedes Nachmodellieren, keine UV-Arbeit). Was für welches Asset gilt, steht in der Registratur
(`farbe:`) und **nicht** im Code. Und die drei Signalfarben (Zyan/Bernstein/Karminrot) sind für
Gameplay reserviert — sie dürfen im Atlas **nicht** als Deko vorkommen (Bibel, unverhandelbar).

### Klang: selbst erzeugt, als **Rezept**, und **gemessen statt gehört**

Du hast keine Ohren in dieser Umgebung — also ist ein Klang genau dann fertig, wenn er **messbar**
ist: Länge, Grundfrequenz, Hüllkurve, Spitzenpegel, ob er schleift (Anfang == Ende). Ein Rezept
(`tools/sound/<name>.py`) ist die Quelle, die `.ogg` das Ergebnis, `assets.ron` der Schalter — genau
wie beim Modell. **Nur originale oder lizenzierte Musik** (Bibel-Risiko: fremde Soundtracks sind
nicht nutzbar); Referenzen darfst du anhören lassen und beschreiben, aber nichts übernehmen (§12e).

### ⚠️ Die wichtigste Regel dieses Abschnitts: **eine `.blend`, die der User angefasst hat, ist heilig**

Der Generator **überschreibt niemals** eine vorhandene `.blend`. Er prüft: Datei existiert und ist
neuer als ihr Skript → **„vom User bearbeitet, nicht angefasst"** ins Log, fertig. Neu erzeugt
wird nur, was fehlt; alles andere nur mit ausdrücklichem `--force <name>`. Wenn du diese Regel
brichst, löschst du Arbeit, die niemand wiederherstellen kann — Blender hat keine Historie.

### Der Auto-Export

Beim Spielstart (in `data/`, vor allem anderen) läuft ein Schritt, der für jede `.blend` prüft:
**fehlt die `.glb` oder ist sie älter?** Dann exportieren, sonst nichts tun.

```bash
blender --background --factory-startup <datei>.blend \
  --python-expr "import bpy; bpy.ops.export_scene.gltf(filepath='assets/3d/glb/<name>.glb', export_format='GLB', export_yup=True, export_apply=True, export_cameras=False, export_lights=False)"
```

- **Kein Blender installiert?** → **einmal warnen**, die vorhandene `.glb` benutzen, **nicht
  abstürzen**. Das Spiel muss auf einem Rechner ohne Blender laufen.
- **Flags:** `--reexport` (alles neu bauen), `--no-export` (Startzeit sparen). Und ein
  eigenständiges Werkzeug `src/bin/export_modelle.rs`, damit man es ohne Spielstart laufen lassen
  kann.
- Der Export ist **auch** die Stelle, an der die glTF-Fallen unten geradegezogen werden. Eine
  Nachbearbeitung des `.glb` (es ist JSON + Binärchunk) ist erlaubt und billiger, als sie in jedem
  Modell von Hand zu vermeiden.

### Der Schalter in der RON — die eine Zeile, die der User umlegt

```ron
models: {
    "titan_normal": (blend: "titan_normal", nutzen: true,  scale: 1.0),
    "scout":        (blend: "scout",        nutzen: false, scale: 1.0),  // noch Platzhalter
}
```

`nutzen: false` ⇒ das Spiel baut den **prozeduralen Platzhalter** aus Bevy-Primitiven
(Kapsel/Box/Zylinder, eingefärbt). `nutzen: true` ⇒ es lädt die `.glb`. **Beide Wege müssen
jederzeit funktionieren**, und beide benutzen dieselben Anker, dieselbe Hitbox und dieselbe
Skalierung — sonst ist das Umschalten kein Schalter, sondern ein Umbau.

### Die Konventionen, die das Ersetzen erst billig machen

Schreib sie nach `docs/konventionen.md` **und** als Kommentarkopf in jedes `tools/blend/*.py`:

- **1 Blender-Einheit = 1 Meter.** Maßstab wird im Modell gemacht, nicht per `scale` in der RON
  (das Feld ist eine Notbremse, kein Arbeitsmittel).
- **Origin zwischen den Füßen** (nicht im Körperzentrum) — sonst steht jedes Modell halb im Boden.
- **Blick nach −Z**, aufrecht. In Blender wird Z-oben modelliert, der Exporter dreht auf Y-oben
  (`export_yup=True`) — **nicht selbst rotieren**, sonst dreht es zweimal.
- **Anker sind Empties mit festen Namen**, und der Modellierer entscheidet damit *wo*, die RON
  *wie stark*: `cortex` (die Todeszone!), `hit.min` / `hit.max` (die Hitbox), `hook.l` / `hook.r`,
  `hand.r` / `hand.l`, `eye`. **Fehlt ein Empty, ist die Zone ein Punkt** — und ein Cortex, der
  ein Punkt ist, fühlt sich wie ein kaputtes Spiel an.
- **Farbe per Vertex-Farben**, nicht per Textur. Lowpoly braucht keine UV-Map, und Vertex-Farben
  überleben jedes Nachmodellieren.
- **Ein Objekt pro sinnvollem Teil**, benannt (`kopf`, `arm.r`, …) — daran hängt später die
  Amputation und die Animation.

### Drei glTF-Fallen, die alle gleich aussehen („mein Modell ist weiß / chrom / unsichtbar")

1. **Bevy liest nur `COLOR_0`.** Hat ein Blender-Mesh **zwei** Color-Attribute, landet die gemalte
   Farbe in `COLOR_1` und das Modell kommt **weiß** an. Im Export das gewünschte Attribut nach
   vorn tauschen — oder im `.py` sicherstellen, dass es nur eines gibt.
2. **Fehlender `metallicFactor` bedeutet 1.0**, also *voll metallisch* — ein Diffuse-Material ohne
   den Wert sieht im Spiel wie Chrom aus. Der Export setzt ihn auf `0.0`, wo er fehlt.
3. **Kameras und Lichter nicht mitexportieren.** Sonst hängt in jedem Modell eine zweite Sonne,
   und die Szene wird von Modell zu Modell heller.

### Prüfen, und zwar jede Sitzung

- `tests/modelle.rs` — ein harter Test (jedes mit `nutzen: true` verdrahtete Modell hat eine
  `.glb` und **alle** geforderten Empties) plus ein `#[ignore]`-Bericht mit `--nocapture`, der
  eine Tabelle druckt: *Modell · `.blend` da? · `.glb` aktuell? · bemalt? · Anker vollständig? ·
  in RON verdrahtet?* Genau diese Tabelle ist das, was ein anderer Agent in zehn Sekunden lesen
  will.
- **`git status assets/3d/blend/` gehört an den Anfang jeder Sitzung.** Eine geänderte `.blend`
  heißt: exportieren, **im Spiel ansehen**, Screenshot, und die Zeile in `docs/STATUS.md`
  hochsetzen.
- **Die Stufen (§8) gelten für Modelle genauso.** Ein Primitiven-Platzhalter ist ⬜ oder 🟨 — nie
  mehr, egal wie gut er sich einbaut. Erst ein echtes Modell, das du **gesehen** hast, ist 🟧.

---

## 8. ⭐ Die Doku und die **vier Stufen** — das Herzstück dieses Auftrags

**An diesem Projekt arbeiten mehrere Agenten, teils parallel, teils nach dir.** Sie können deinen
Code lesen, aber sie können nicht sehen, was du *gesehen* hast. Genau dafür ist die Statusdoku da:
**sie sagt, wie weit man einer Sache trauen darf.**

Die Datei heißt **`docs/STATUS.md`** und ist eine Tabelle. Sie ist Pflicht, sie wird **in
derselben Nachricht** gepflegt wie der Code, und sie darf **nie** großzügiger sein als die
Wirklichkeit.

### Die vier Stufen — genau diese, keine Zwischentöne

| Marke | Stufe | Bedeutung | Wer setzt sie |
|---|---|---|---|
| ⬜ | **nicht implementiert** | Existiert nicht oder nur als Platzhalter/Stub. Auch: „Code da, tut aber nichts." | Claude |
| 🟨 | **halb implementiert** | Von Claude gebaut, **nicht getestet, nicht gesehen**. Es kompiliert. Mehr ist nicht behauptet. | Claude |
| 🟧 | **fast implementiert** | Gebaut **und** mit vielen Tests abgesichert, die umfallen, wenn es kaputtgeht, **und** von Claude im laufenden Spiel gesehen (**Screenshot**). | Claude |
| ✅ | **fertig** | **Der User hat draufgeschaut und es abgenommen.** | **NUR der User** |

**Die eiserne Regel: ✅ setzt Claude NIEMALS selbst.** Nicht bei grünen Tests, nicht bei einem
schönen Screenshot, nicht „weil es offensichtlich läuft". 🟧 ist die höchste Stufe, die du selbst
vergeben darfst. Wenn du glaubst, etwas sei reif für ✅, schreibst du es in
`docs/ABNAHME.md` — die Liste dessen, worauf der User bitte einmal schauen soll, mit dem
Screenshot-Pfad daneben.

### Was die Stufen belegen müssen

**🟧 braucht drei Belege, nicht einen:**

| | heißt |
|---|---|
| **Bild** | im laufenden Spiel gesehen — Screenshot-Pfad in der Tabelle. Nicht „gebaut". |
| **Zahl** | gemessen: Bildzeit, Zähler, Vorher/Nachher, eine Distanz. Nicht geschätzt. |
| **Code** | ein Test, der **umfällt**, wenn das Verhalten kaputtgeht (einmal absichtlich kaputt machen und zusehen, dass er rot wird — ein Test, der nie rot war, beweist nichts). |

**Und: Sonderfälle testen, nicht den Normalfall.** Der Normalfall funktioniert fast von allein.
Die Fehler sitzen an den Rändern: Haken auf eine Kante, Haken auf einen sterbenden Titanen,
Gas exakt null im Moment des Boosts, Schnitt mit einer gerade brechenden Klinge, zwei Haken in
zwei Richtungen.

### Das Format von `docs/STATUS.md`

```markdown
# STATUS — was implementiert ist und was nicht

Stufen: ⬜ nicht implementiert · 🟨 halb (gebaut, ungetestet) ·
🟧 fast (getestet + gesehen) · ✅ fertig (**nur der User setzt das**)

| Sache | Domäne | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Fenster + 3D-Kamera | render | 🟧 | `tests/kamera.rs`, `docs/bilder/kamera.png` | 2026-08-08 |
| Haken einschlagen | vector | 🟧 | `tests/haken.rs` (7 Fälle), `docs/bilder/haken.png`, Reichweite 78 m gemessen | 2026-08-08 |
| Seilzug / Schwung | vector | 🟨 | kompiliert, **kein Test, kein Bild** | 2026-08-08 |
| Cortex-Trefferzone | titan | ⬜ | — | — |
```

**Regeln für die Tabelle:**

- **Rückwärts ist erlaubt und erwünscht.** Wenn etwas kaputtgeht oder du merkst, dass ein Test
  gelogen hat: Stufe **runter**setzen. Eine zu hohe Stufe ist die teuerste Zeile im Projekt — sie
  schickt den Nächsten los, auf Sand zu bauen.
- **Keine Stufe überspringen.** Es gibt kein 🟨 → ✅.
- **Bau nicht auf 🟨.** Wenn dein neues Feature auf einer 🟨-Sache steht, bring die erst auf 🟧.
  Richtig schlägt neu: dass eine Funktion tut, was sie soll, ist wichtiger als jedes weitere
  Feature.
- **„Kein Bild möglich" wird hingeschrieben, nicht weggelassen.** Wenn es keine Grafiksitzung
  gibt (§12), bleibt die Sache 🟨 mit dem Vermerk *„gebaut + getestet, Pixel ungesehen"*. Nicht
  aufrunden.

### Die anderen Pflichtdateien

| Datei | Inhalt |
|---|---|
| `README.md` | Was das Spiel ist, wie man es startet, Tastenbelegung, aktueller Stand in einem Absatz |
| `CLAUDE.md` | **Dein Gedächtnis für die nächste Sitzung** — ein *Index*, kein Archiv: die Regeln, die immer gelten, und Zeiger auf den Rest. Halte sie unter ~150 Zeilen. |
| `docs/README.md` | Der Spiegel von `src/`: eine Zeile pro Doku-Datei |
| `docs/STATUS.md` | ⭐ die vier Stufen (oben) |
| `docs/architektur.md` | Domänen, Plugin-Reihenfolge, die Erlaubnisliste der Abhängigkeiten |
| `docs/konventionen.md` | Achsen, Einheiten, Blickrichtung, Namensregeln |
| `docs/modelle.md` | Die Modellkette (§7): welches Modell, welche Anker, welcher Stand — und die Anleitung „so tausche ich ein Modell aus", geschrieben **für den User** |
| `docs/lessons/*.md` | **Fallgeschichten**: was dich Zeit gekostet hat, ein Thema pro Datei. Der wertvollste Ordner im Projekt. |
| `docs/BUGS.md` | ⭐ jeder Bug mit **Reproduktion, Beleg, Ursache, Fix und Test** (§9) |
| `docs/ABNAHME.md` | Was der User anschauen soll, damit es ✅ werden kann |
| `docs/FRAGEN.md` | Entscheidungen, die dir nicht gehören. **Nicht unterbrechen — hier reinschreiben und drumherum arbeiten.** |
| `docs/FUNDE.md` | Fehler, die dir *nebenbei* auffallen, außerhalb deiner Aufgabe — mit der Messung daneben, damit ein anderer prüfen kann. **Nicht still mitfixen.** |
| `docs/ROADMAP.md` | Was bewusst später kommt (Shifting, Multiplayer, Pferde, Donnerspeere) |

---

## 9. ⭐ Ein Bug ohne Beleg ist ein Gerücht — **und Unsicherheit ist ein Mangel**

> **Wenn es nicht sicher ist, ist es nicht gut.** Kein „müsste jetzt gehen", kein „sollte
> passen", kein „wahrscheinlich behoben". Entweder du hast es **belegt**, oder du schreibst
> hin, dass du es nicht hast.

Das gilt in beide Richtungen: für Bugs, die du **findest**, und für Fixes, die du **behauptest**.

### a) Ein Bugbericht braucht vier Zeilen — sonst ist er keiner

Jeder Bug wird in **`docs/BUGS.md`** eingetragen, **bevor** er gefixt wird, mit:

| Feld | Was hinein muss |
|---|---|
| **Reproduktion** | Das exakte Kommando, das ihn zeigt: `cargo run -- --script scripts/haken_kante.txt`, plus Seed / Koordinate / Blickrichtung aus dem F3-Overlay. Wer es nicht nachstellen kann, kann es nicht prüfen. |
| **Beleg** | Screenshot in `docs/bilder/`, Logausschnitt **oder** eine Zahl (gemessen 34 m/s, erwartet ≤ 12). Nicht „sieht falsch aus". |
| **Erwartung** | Was stattdessen passieren müsste — und **woher** du das weißt (RON-Zeile, Doku-Absatz, Design-Entscheidung). |
| **Ursache** | `datei:zeile`, sobald bekannt. Solange sie fehlt: **„Ursache unbekannt"**, nicht geraten. |

**Kein Repro ⇒ kein Fix.** Ein Bug ohne Reproduktion wird als *unbelegt* eingetragen und **nicht
repariert** — ein Fix für etwas, das du nie gesehen hast, ist eine Änderung ohne Grund, und die
kannst du hinterher auch nicht widerlegen.

### b) Ein Fix ohne roten Test ist eine Vermutung

Die Reihenfolge ist **nicht verhandelbar**:

1. **Test schreiben, der den Bug zeigt** — und ihn laufen lassen, bis er **rot** ist. Ein Test,
   der nie rot war, beweist nichts; er beweist nur, dass er kompiliert.
2. **Fixen**, bis er grün ist.
3. **Danach den Fix wieder herausnehmen** und zusehen, dass der Test erneut umfällt. Erst dann
   weißt du, dass der Test *diesen* Fix prüft und nicht irgendetwas daneben.
4. **In `docs/BUGS.md` eintragen:** Ursache, Fix, Testname. Und wenn es eine Falle war, aus der
   man lernen kann: eine Zeile in `docs/lessons/`.

Bei einem Bug, den nur das Auge sieht (Bewegungsgefühl, Kameraruckeln, ein Haken, der ins Nichts
zeigt), ist der Beleg ein **`--script`-Lauf mit `assert`** plus **Screenshot vorher/nachher**. Das
ist genau der Grund, warum der Fahrer in Stufe 1 gebaut wird.

### c) Fremde Fehler gehören nach `docs/FUNDE.md`, nicht in einen stillen Fix

Wer beim Arbeiten über etwas stolpert, das nicht zur eigenen Aufgabe gehört: **aufschreiben, mit
der Messung daneben**, damit ein anderer prüfen kann, ob es wirklich falsch ist. Ein nebenbei
mitgefixter Fremdfehler ist ein Fix, den niemand geprüft hat — und er versteckt sich im Diff einer
Aufgabe, in der ihn keiner sucht.

### d) Sicherheit im Code: nichts darf still schiefgehen

„Sicher" heißt auch: **das Programm lügt nicht und stürzt nicht an einer Stelle, die man vorher
prüfen konnte.**

- **Kein `unsafe`.** Wenn du glaubst, du brauchst es, gehört das in `docs/FRAGEN.md`.
- **`unwrap()` / `expect()` nur mit Begründung im Kommentar** — und **nie** auf Daten aus einer
  Datei oder aus Eingaben. Beim **Laden** der RON ist ein sofortiger, lauter Abbruch mit
  Dateiname und Zeile das *richtige* Verhalten (fail fast beim Start), mitten im Spiel ist er es nie.
- **Physik braucht Wachen.** Seilkräfte des Vector Gear, Normalisierungen und Divisionen erzeugen NaN/∞,
  sobald ein Vektor Länge 0 hat oder ein Frame mal 0,5 s dauert. NaN im `Transform` ist der Bug,
  der aussieht wie „der Spieler ist verschwunden": Länge prüfen bevor normalisiert wird, `dt`
  clampen, und in `debug/` ein System, das **einmal warnt**, wenn eine Position nicht endlich ist.
- **Ein `panic!` im Spiel ist ein Bug**, auch wenn er „nie" auftritt. Und ein `Result`, das mit
  `let _ =` geschluckt wird, ist ein Fehler, den niemand mehr sehen kann.

### e) Wortwahl: schreib, was du weißt

| Nicht schreiben | Sondern |
|---|---|
| „behoben" (ohne roten Test davor) | „gefixt, Test `x` war rot, ist grün" |
| „sollte jetzt gehen" | „gebaut, **ungetestet** — 🟨" |
| „läuft" | „im Spiel gesehen, Screenshot `docs/bilder/…`" |
| „ist schneller" | „16,6 → 9,4 ms, `--release --novsync`, gemessen am …" |
| „funktioniert wahrscheinlich" | eine Zeile in `docs/FRAGEN.md` oder `docs/BUGS.md` |

**Und die Verbindung zu den vier Stufen (§8): Unsicherheit setzt die Stufe herunter, nicht
hinauf.** Wenn du dir bei einer Sache nicht sicher bist, ist sie **🟨** — auch wenn sie
funktioniert. Das kostet nichts. Eine zu hohe Stufe kostet den Nächsten einen halben Tag.

---

## 10. ⭐ Alles, was sich **wiederholt**, ist genormt

> **Zwei Formen für dieselbe Sache heißen: keine Form.** Wenn eine Commit-Message frei erfunden
> wird, ist die Historie kein Werkzeug mehr, sondern ein Tagebuch. Genormt heißt nicht hübsch —
> es heißt **greppbar**: `git log --oneline | grep F-014` muss die Geschichte eines Features
> beantworten, und `grep -rn F-014 docs/ tests/` den Rest.

**Die Norm steht in `docs/konventionen.md`, mit genau EINEM Beispiel pro Zeile.** Sie ist die
einzige Quelle; wer eine neue wiederkehrende Sache anfängt (ein neuer Dateityp, ein neues
Protokoll), **normt sie dort, bevor er sie zum zweiten Mal benutzt**.

### Commit-Messages

```
<F-ID|bereich>: <eine Zeile, was jetzt anders ist>        ← max 72 Zeichen, Deutsch, aktiv

Stufe: 🟨 → 🟧                                            ← nur wenn sie sich ändert (§8)
Beleg: tests/gas.rs::f014_boost_verbraucht_gas · docs/bilder/f014-boost.png · 12,4 → 3,1 ms [cachy]
Grund: eine Zeile, WARUM — nur wenn es nicht offensichtlich ist
```

- **Präfix ist die `F-ID`**, wenn es zu einem Feature aus der Liste gehört (§2): `F-014 vector: …`.
  Gibt es keine, ist es einer von **fünf** Bereichen — und nur diese fünf:
  `docs:` · `test:` · `tool:` · `fix:` · `chore:`.
- **Ein Commit = eine Sache.** „Diverse Fixes" ist kein Commit, das ist ein Karton.
- **Eine Sprache**: Deutsch, durchgehend. Nicht heute `add gas drain`, morgen `Gas-Verbrauch`.
- **Kein Punkt am Ende, keine Emoji im Betreff** (die Stufen-Marken stehen im Rumpf).
- ⚠️ **Keine Werkzeug- oder Autorenspuren in der Message.** Kein `Co-Authored-By:`, keine
  Signatur, kein „generated with", kein Modellname, kein Hinweis darauf, *wer oder was* den Commit
  geschrieben hat. Eine Commit-Message beschreibt **die Änderung**, nicht ihren Urheber — der steht
  im Git-Autor-Feld und nirgends sonst. Das gilt auch für PR-Beschreibungen und Tags.
  Nimm es in `tools/normen.py` auf: eine Message mit `Co-Authored-By`, `Generated`, `Claude`,
  `AI` oder `🤖` fällt durch.

### Der Rest, was ebenfalls jedes Mal gleich aussieht

| Was | Norm | Beispiel |
|---|---|---|
| **Branch** | `<f-id>-<kurz>` bzw. `<bereich>/<kurz>` | `f014-gas-boost`, `fix/haken-kante` |
| **Test-Name** | `<f_id>_<die Aussage, die gilt>` — nicht „test_gas" | `f014_boost_verbraucht_gas` |
| **Test-Datei** | `tests/<domäne>.rs` | `tests/vector.rs` |
| **Screenshot** | `docs/bilder/<f-id>-<kurz>[-vorher|-nachher].png` | `docs/bilder/f014-boost-nachher.png` |
| **Skript** | `scripts/<f-id>-<kurz>.txt`, darin `mark <f-id>-<stichwort>` | `scripts/f014-boost.txt` |
| **STATUS-Zeile** | `\| Sache \| Domäne \| Stufe \| Beleg \| Datum [maschine] \|`, Datum **ISO** | `… \| 🟧 \| tests/vector.rs … \| 2026-08-09 [cachy] \|` |
| **Bug** | `B-007 <Titel>` + die vier Felder aus §9 | `B-007 Haken hält an einer Kante nicht` |
| **Frage** | `Q-003 <Frage>` + Kontext + `ANNAHME:` (womit du bis zur Antwort weiterarbeitest) | |
| **Fremdfund** | `FUND-005 <Symptom>` + Messung | |
| **Doku-Kopf** | `# <name> — <ein Satz>` und darunter `Stand: <ISO> · Stufe: <marke>` | |
| **RON-Schlüssel** | `snake_case`, **eine** Sprache pro Datei (welche, steht in `docs/konventionen.md`) | |
| **Rust** | `snake_case` Dateien/Funktionen, `CamelCase` Typen, Domänenordner immer Einzahl | `src/vector/hook.rs` |
| **Subagenten-Bericht** (§17) | fest: `Aufgabe · Getan · Beleg · Stufe · Offen · Funde` | ein Freitext-Bericht ist nicht integrierbar |

### Keine **Zombie-Dateien** — und Links werden **mitgezogen**

> **Jede Datei im Repo ist entweder von irgendwo verlinkt/benutzt — oder sie ist gelöscht.**
> Dazwischen gibt es nichts. Eine Datei, die niemand kennt, ist schlimmer als keine: sie sieht wie
> Wahrheit aus, wird nie gepflegt und schickt den Nächsten in die Irre.

- **Jede `docs/*.md` steht in `docs/README.md`.** Jede Quelldatei hat ihre Doku-Datei (§8). Jedes
  Asset steht in der Registratur (§7). Jedes Skript in `tools/` ist in einer Doku erwähnt. Wer eine
  Datei anlegt, verlinkt sie **im selben Commit**.
- **Wer Daten in ihre endgültige Struktur bringt, zieht alle Verweise mit.** Eine Umbenennung oder
  ein Umzug ist **erst dann fertig**, wenn `grep -rn "<alter pfad>" . --exclude-dir=target
  --exclude-dir=.git` **leer** ist. Das gilt für Pfade in Markdown, in RON, in Rust, in Skripten und
  in Commit-Vorlagen. **Ein Link, der nach dem Umzug ins Leere zeigt, ist ein Bug** — dieselbe Klasse
  wie eine falsche Doku (§8).
- **Prüfen, nicht hoffen:** `tools/normen.py` bekommt zwei Prüfungen dazu — (1) **jeder
  Markdown-Link auf eine Repo-Datei existiert**, (2) **jede Datei unter `docs/`, `assets/`, `tools/`
  ist mindestens einmal referenziert**. Was durchfällt, wird verlinkt oder gelöscht; „lass ich
  liegen" ist keine Option.
- **Nichts wird „zur Sicherheit" behalten.** Kein `*_alt.rs`, kein `backup_*.ron`, kein
  `titan_v2.blend` neben `titan.blend`. **Git ist das Backup** — was gebraucht wurde, steht in der
  Historie, und die kann man wiederholen. Zwei Varianten derselben Sache im Baum heißen: die falsche
  wird benutzt (und zwar an dem Tag, an dem niemand hinsieht).
- **Beim Auflösen des Gerüsts (§18)** ist genau das die Hauptarbeit: die Verweise auf
  `prompts/…` und `gameplay/…` zeigen danach auf `docs/…` — **alle**, in derselben Änderung, mit
  einem `grep` als Beleg.

### Die Rituale — auch ein Ablauf, der sich wiederholt, ist genormt

**Sitzungsanfang** (immer in dieser Reihenfolge, immer vollständig):

```bash
hostname                                  # welche Maschine? (§14)
ls -lt prompts/ && ls -R gameplay/        # neuer Auftrag? (Kopf dieser Datei, §2)
git status --short && git log --oneline -5 # was hat eine andere Sitzung getan?
cat docs/STATUS.md docs/TODO.md           # wo stehen wir wirklich?
cargo check 2>&1 | grep '^error'          # ist der Baum grün, BEVOR ich anfasse?
```

**Sitzungsende** (nichts davon ist optional):
`docs/STATUS.md` + `docs/TODO.md` nachziehen · Screenshots nach `docs/bilder/` mit normiertem Namen ·
neue Erkenntnis nach `docs/lessons/` · offene Frage nach `docs/FRAGEN.md` · **committen mit
normierter Message** · und ein ehrlicher Absatz, was ungesehen blieb.

### Und die Norm wird geprüft, nicht gehofft

Ein Skript `tools/normen.py` (oder ein `commit-msg`-Hook) prüft, was mechanisch prüfbar ist:
Commit-Betreff gegen die Regex, ISO-Datum in `STATUS.md`, Testnamen mit `f\d{3}_`-Präfix, wo eine
`F-ID` existiert, Screenshot-Namen. **Eine Norm ohne Prüfer verfällt still** — genau wie die
Domänenregel (§5) und die 20k-Regel (§11). Der Prüfer ist zwanzig Zeilen und spart die Diskussion.

---

## 11. Performance: die Regel, die man von Anfang an einhalten muss

Eine Stadt hat Tausende Häuser, ein Einsatz Dutzende Titanen, jeder Titan sechs Gliedmaßen.
**Nichts darf alle Entities durchlaufen, um eine Frage über die zehn Meter vor der Nase zu
beantworten.**

- **Ein räumlicher Index gehört in `world/`** (Gitterzellen → Entities, gepflegt über Bevys
  `Added`/`RemovedComponents`, damit er nicht veralten kann). Hakeneinschlag, Klingentreffer,
  Kollision und Titanen-Zielsuche gehen **alle** darüber.
- **Nichts ändert sich pro Frame, alles pro Sekunde.** `* time.delta_secs()` allein reicht nicht:
  (a) **nie auf Ganzzahlen runden** — `(schaden * dt).ceil()` macht die Framerate zur
  Schadenszahl; trag Bruchteile mit. (b) **Exponentielles Glätten** ist pro Frame:
  `x += (ziel-x)*0.1` → benutze `1 - e^(-k*dt)`. (c) **Rauschen skaliert mit `sqrt(dt)`**, nicht
  mit `dt`. Schreib dafür **eine** Hilfsfunktion in `shared/` und benutze nur die.
- **Erst messen, dann behaupten.** Und: **Debug ist langsam.** `cargo run` ist ein Debug-Build
  (der eigene Crate auf `opt-level = 1`); für jede Perf-Aussage `cargo run --release`.
  Debug-Langsamkeit ist keine Regression.
- **Unter Vsync ist jede Bildzeit 16,6 ms** — damit misst „was kostet das?" sechsmal denselben
  Deckel. Bau früh ein `--novsync`-Flag ein und/oder benutze Bevys
  `RenderDiagnosticsPlugin` (echte GPU-Zeitstempel pro Renderpass).
- **Schatten sind der teuerste Schalter im Spiel.** Punktlichter sind fast gratis, Schatten nicht.
  Erst am Ende, mit Zahl.

---

## 12. Du kannst nicht klicken — bau dir die Werkzeuge **zuerst**

Das ist der Punkt, an dem solche Projekte scheitern: alles ist gebaut, nichts ist gesehen, weil
jedes Feature hinter Maus und Tastatur liegt und niemand am Keyboard sitzt. **Also kommt die
Prüfinfrastruktur vor den Features** — sie ist Teil von Stufe 1, nicht ein „wenn Zeit ist".

**a) Start-Flags, die am Menü vorbeigehen.** Ein Hauptmenü ist für dich eine Wand ohne Tür.
```bash
cargo run -- --mission tutorial   # direkt in einen Einsatz, kein Menü
cargo run -- --sandbox            # leeres Feld, ein Titan, unendlich Gas — zum Anschauen
cargo run -- --novsync            # zum Messen
cargo run -- --lag 200            # 200 ms simulierte Latenz (Bibel T-019) — jedes
                                  # Bewegungsfeature wird AUCH so geprüft, nicht nur lokal
cargo run -- --script <datei>     # das Spiel spielen, ohne zu tippen
```

**b) `--script`: der Fahrer.** Eine Textdatei, eine Anweisung pro Zeile, die in **dieselben
Eingaben schreibt, die ein Mensch auslöst** (`ButtonInput<KeyCode>`, `ButtonInput<MouseButton>`,
und ein „so-tun-als"-Blickvektor). **Kein zweiter, falscher Weg zu spielen** — jedes System
dahinter ist das echte.
```text
spawn titan normal 20 0 -40   # Typ und Ort in Metern
look 0 -10                    # Blickrichtung in Grad (yaw, pitch)
key Space 0.3                 # Taste 0,3 s halten
hook left                     # Haken raus
wait 1.2                      # Commands sind verzögert — sonst fotografierst du ein leeres Feld
mark eingehakt                # eine Zeile ins Log, an der man einen Screenshot ausrichtet
assert speed > 25             # ⭐ das Skript darf selbst urteilen: fällt es um, ist es ein Test
```
`assert` ist der Grund, warum das mehr ist als eine Demo: damit wird eine **Fahrt** zu einem
Test, und Bewegungsgefühl ist genau die Sorte Sache, die kein Unit-Test greift.

**c) Ein Debug-Overlay (F3), das jede Meldung nachstellbar macht.** Position, Blickrichtung,
Geschwindigkeit, Gas, Hakenzustand, Bildzeit — **im Bild**. Dazu ein `warp x y z` + `look` im
Skript. Damit kann der User dir eine Koordinate schicken und du stehst genau dort. Das ist
mehr wert als jedes Bug-Formular.

**d) Screenshots (niri/Wayland) — nur auf Maschine B, siehe §14:**
```bash
setsid nohup cargo run -- --sandbox > /tmp/dbt.log 2>&1 < /dev/null & disown
sleep 20   # der erste Build dauert
ID=$(niri msg --json windows | python3 -c "import sys,json;print([w['id'] for w in json.load(sys.stdin) if (w.get('title') or '')=='Defeated by Titan'][0])")
niri msg action focus-window --id $ID   # SONST drosselt der Compositor auf ~5 fps
sleep 2
niri msg action screenshot-window --id $ID
```
Landet in `~/Pictures/Screenshots/`. **Kopier die Bilder nach `docs/bilder/`** und verlinke sie in
`STATUS.md` — ein Screenshot, den niemand mehr findet, ist kein Beleg.

- **Ein unfokussiertes Fenster wird auf ~5 fps gedrosselt.** Das ist keine Regression, auch wenn
  es exakt so aussieht. Vor jeder fps-Messung fokussieren.
- **Prüfe, dass nur EINE Instanz läuft**, sonst screenshottest du alten Code.
- **Manchmal gibt es gar keine Grafiksitzung** (kein `WAYLAND_DISPLAY`/`DISPLAY`) → `cargo run`
  panikt sofort. Dann gibt es **kein Bild**, dann bleibt die Sache **🟨** und du bittest den User
  draufzuschauen. Nicht aufrunden.

---

**e) Recherche und Inspiration — du darfst ins Internet, und du sollst es nutzen:**

Ausdrücklich erlaubt: **YouTube** (Bewegungs- und Level-Design ansehen: wie eine Stadt für
Schwingen gebaut wird, Ankerdichte, Dachhöhen, Gassenbreiten), **Google/Bilder-Suche** für
Referenzbilder, Fachartikel zu Seilphysik/Netcode/Audio-Synthese, und die **Doku der installierten
Bevy-Version** (die wichtigste Quelle von allen, §3). Nimm Skripte, wenn es schneller geht
(`yt-dlp` für Untertitel/Beschreibungen, `curl`, ein kleines Parse-Skript).

**Assets herunterladen ist erlaubt — es ist ein Prototyp.** Modelle, Klänge, Musik-Platzhalter:
hol sie, wenn es das Spiel schneller spielbar macht. `yt-dlp` für Ton aus einem Video, `curl` für
ein Modellarchiv, ein Skript für den Rest — alles zulässig. Der User ersetzt später **alles** selbst
(§7), und bis dahin ist ein guter Prototyp wichtiger als eigene Polygone.

**Wo es sich lohnt, zuerst zu suchen** (freie Lizenzen, kein Nachdenken, gute Low-Poly-Passung):
Kenney, Poly Pizza, OpenGameArt, Quaternius, Sketchfab (CC-Filter), Freesound (CC0), Pixabay.
Nicht als Vorschrift — nur weil es dort schneller geht als beim Freistellen einer Tonspur.

**Die Regeln, die auch für Platzhalter gelten:**

- **Alles Fremde nach `assets/extern/` + Zeile in `HERKUNFT.md` + `herkunft:` in der Registratur**
  (§7). Ohne diese drei ist es ein Zombie (§10) — und der User kann es später nicht finden, um es
  zu ersetzen.
- **Nicht ins öffentliche Repo.** `assets/extern/` ist ignoriert; `tools/hole_extern.sh` beschafft
  es wieder (§7).
- **Was du übernimmst, ist am wertvollsten als ZAHL oder ERKENNTNIS** — „Gassen sind 8–12 m breit,
  damit ein Haken beide Seiten erreicht" — mit **Quelle** nach `docs/gameplay/referenzen.md`. Eine
  Zahl ohne Herkunft ist eine Behauptung (§9).
- **Referenzbilder** nach `gameplay/bilder/` bzw. `docs/gameplay/referenzen/`, mit URL und Datum.
- **Erst messen, dann glauben:** ein Blogpost über Bevy-Versionen ist keine Quelle, die installierte
  Doku ist eine. Bei Widerspruch gewinnt die Wirklichkeit.

---

## 13. Der Stufenplan — nach jeder Stufe **läuft** das Spiel

Wer breit anfängt (Titanen-KI, Missionen, Perks gleichzeitig), hat nach einem Tag nichts, was
startet, und keine Ahnung, welcher von zwölf Umbauten es kaputtmacht. Also schmal und tief, und
**jede Stufe wird einzeln committet** — der Commit ist dein Rückweg.

### Stufe 0a: **Preflight** — frag die Maschine, bevor du baust

Das kostet zwanzig Sekunden und entscheidet, was heute überhaupt möglich ist. Das Ergebnis kommt
als Tabelle nach `docs/umgebung.md`, damit der nächste Agent es nicht wieder herausfinden muss:

```bash
hostname                               # ⭐ WELCHE MASCHINE? (§14 — 'debian' = kein Fenster, das ist ok)
rustc --version && cargo --version     # Rust vorhanden? edition 2024 braucht 1.85+
df -h /home                            # ⚠️ ein Bevy-target/ wird zweistellig GB. Unter 20 GB frei: erst reden, dann bauen
echo "WAYLAND=$WAYLAND_DISPLAY DISPLAY=$DISPLAY"   # leer ⇒ KEIN Screenshot möglich (§12)
command -v niri && niri msg --version   # der Compositor für Screenshots
command -v blender && blender --version # fehlt ⇒ Modellkette baut nur .py, exportiert nicht (§7)
command -v gh && gh auth status          # fehlt/nicht angemeldet ⇒ Schritt 3 der Ziellinie braucht dich
nproc                                    # wie breit darf parallel gebaut werden (§17)
```

**Was fehlt, wird nicht simuliert und nicht beschönigt** — es wird in `docs/umgebung.md` notiert und
die betroffenen Sachen bleiben auf ihrer ehrlichen Stufe (§8). Ohne Grafiksitzung gibt es kein 🟧.

### Stufe 0b: **Das Projekt anlegen** — `cargo init`, nicht `cargo new`

Der Ordner ist **nicht leer** (`prompts/`, `gameplay/` liegen drin), also legt `cargo new` ein
Unterverzeichnis an bzw. bricht ab. Richtig ist:

```bash
cd ~/Documents/defeated-by-titan
cargo init --name defeated_by_titan      # Paket im VORHANDENEN Ordner
cargo add bevy                            # schreibt die WIRKLICH aktuelle Version in Cargo.toml
cargo add ron serde --features serde/derive
git add -A && git commit -m "Projekt aufgesetzt (Initialprompt in prompts/)"
```

Der Paketname ist **`defeated_by_titan`** (mit Unterstrichen — Rust mag keine Bindestriche im
Crate-Namen), das Fenster heißt **„Defeated by Titan"**, das GitHub-Repo `defeated-by-titan`.
Diese drei Schreibweisen bleiben so; jede steht an genau einer Stelle in der Doku.

| Stufe | Fertig, wenn |
|---|---|
| **0** | Preflight (0a) durch, `cargo init` (0b), `Cargo.toml` mit den Profilen, **leeres Fenster geht auf**. `docs/`-Skelett + `STATUS.md` + `TODO.md` + `CLAUDE.md` stehen — kurz, aber echt, damit Subagenten (§17) etwas zu lesen haben. |
| **1** | 3D-Szene: Boden, Sonne, ein Würfel. **FPS-Kamera** dreht mit der Maus, WASD läuft, Schwerkraft und Boden-Kollision — die Bewegung liest **`Intent`**, nicht die Tastatur (§6). **Plus: `--sandbox`, `--script`, F3-Overlay, ein Screenshot, `src/net/` als `LocalOnly`-Stub, `tests/mehrspieler.rs` mit zwei Spielern und `--headless` (§14), damit alles auch ohne Bildschirm prüfbar ist.** |
| **1b** | ⭐ **Die Modellkette steht mit EINEM Modell** (§7): `tools/blend/scout.py` → `scout.blend` → Auto-Export → `.glb` → `nutzen: true` in `art.ron` → im Spiel gesehen. Der Platzhalter-Weg (`nutzen: false`) läuft daneben weiter. **Vor Stufe 2** — jedes weitere Modell ist danach eine Kopie dieser Kette, und der User kann ab hier jederzeit selbst modellieren. |
| **2** | Die **Stadt** steht: Mauer, Häuser, Dächer, Bäume — hakbare Flächen (erst Platzhalter, dann `.blend`). Kollision gegen alles davon, über den räumlichen Index. |
| **3** | ⭐ **Vector Gear: Haken raus, einhaken, Seil einholen, schwingen, Gas verbrauchen, Boost.** Ich fliege durch die Stadt und es fühlt sich gut an. ← **die Marke für Tag 1** |
| **4** | Ein **Titan** steht in der Stadt: Rig, Gliedmaßen, Cortex als eigene Trefferzone. Klinge schneidet, Cortex-Treffer tötet, Bein ab = er fällt. Schaden aus Geschwindigkeit. |
| **5** | Der Titan **wehrt sich**: sucht, geht, greift, packt dich, wirft dich. Dampf, Regeneration, Tod. Klingen werden stumpf und brechen, Gas geht aus, Nachschub. |
| **6** | Ein **Einsatz** mit Zielen und Phasen (`missions.ron`), Spawn-Wellen, Sieg/Niederlage, ein Trupp NPC-Kameraden. |
| **7** | **Progression**: XP, Mark/Sigil, Gear-Budget, Traits, Lineages, Ascension — komplett aus RON. Hauptmenü, Speichern/Laden. **Achtung: erst nach dem Vector-Gear-Gate** (§2). |
| **8** | **Politur und Zahlen**: mehr Titanen-Typen, ein Boss mit Phasen, Sound, Performance mit vielen Titanen gemessen, `docs/` durchgesehen und wahr, `docs/ABNAHME.md` gefüllt. |

**Stufe 3 ist die Marke für Tag 1.** Steht sie mittags, zieh weiter. Steht sie abends nicht, ist
das kein Scheitern — es heißt, Stufe 1 oder 2 hatte eine Überraschung, und die willst du gefunden
haben, *bevor* zwanzig Titanen daran hängen.

---

## 14. ⭐ Zwei Maschinen — **frag zuerst, auf welcher du bist**

Dieses Projekt läuft auf **zwei verschiedenen Rechnern**, und sie können nicht dasselbe. Das ist
kein Problem, solange du weißt, auf welchem du sitzt — und **eine Katastrophe, wenn du es
verwechselst**: du hältst eine fehlende Grafiksitzung für einen Bug, oder einen N100 für eine
Performance-Regression.

```bash
hostname          # 'debian' → headless (A) · 'offlinebot' → volle Grafik (B)
uname -r; nproc; echo "WAYLAND=$WAYLAND_DISPLAY DISPLAY=$DISPLAY"
```

| | **A — `debian`** | **B — `offlinebot`** |
|---|---|---|
| System | Debian 13 (trixie), Kernel 6.12 | CachyOS, Kernel 7.x |
| Oberfläche | **keine** — kein Monitor, kein Wayland/X | **niri** (Wayland), kitty, fish |
| CPU / GPU | Intel N100, 4 Kerne · UHD Graphics (integriert) | Ryzen 7 5800X, 16 Threads · **RTX 3080** |
| RAM / Platte | 15 GB · 451 GB (viel frei) | 31 GB · **hier war die Platte schon voll** (§14 unten) |
| Nutzung | **selten, aber es kommt vor** | der Normalfall |
| **Ein Fenster** | **geht nicht — und das ist in Ordnung** | geht |
| **Screenshot** | nur via Offscreen-Rendering, falls es läuft | ja, `niri msg action screenshot-window` |

### Auf **A (headless)** wird trotzdem gearbeitet — nur anders geprüft

**Kein Fenster ist kein Grund, nicht weiterzubauen.** Was dort vollständig geht:

- **`cargo test`** — alles, was Logik ist: Vector-Gear-Mathematik, Trefferzonen, Schadenskurven,
  RON-Validierung, Weltgenerierung als Zahlen, der Domänen-Test (§5), `tests/mehrspieler.rs` (§6).
- **`--script` mit `assert`** (§12) — **aber nur, wenn das Spiel einen `--headless`-Modus hat.**
  Deshalb ist der **Teil von Stufe 1**: kein Fenster (`primary_window: None`), fester Tick, läuft N
  Ticks und **beendet sich mit einem Exit-Code**, der sagt, ob alle `assert` gehalten haben. Damit
  ist eine Fahrt auf **jeder** Maschine prüfbar — und in einem CI eines Tages auch.
- **Die Modellkette (§7)** — `blender --background` braucht **keinen** Bildschirm. `.py` → `.blend`
  → `.glb` und der Struktur-Test (Empties, Vertexfarben, `metallicFactor`) laufen dort einwandfrei.
- **Doku, Extraktion der Excel-Liste (§2), Aufräumen, Refactoring.**

**Und was dort NICHT geht — ohne Ausrede:**

- **Kein Bild ⇒ kein 🟧.** Die Obergrenze auf A ist **🟨**, mit dem Vermerk *„Logik getestet,
  Pixel ungesehen — Maschine A"*. Nicht aufrunden, nicht „sieht sicher richtig aus".
  *(Ausnahme, wenn du sie **belegst**: Offscreen-Rendering in eine PNG — Bevy kann in ein
  Render-Target zeichnen, Vulkan braucht dafür keine Anzeige. **Erst beweisen, dass es auf dem N100
  wirklich ein Bild liefert**, dann als Beleg benutzen. Behauptet ist es nichts wert.)*
- **Keine Performance-Aussage.** Ein N100 mit integrierter Grafik und ein 5800X mit einer 3080 sind
  keine Messreihe. **Jede Zahl in `STATUS.md`/`BUGS.md` trägt die Maschine dazu** — `[debian]` oder
  `[cachy]`. Eine Bildzeit ohne Maschinenangabe ist keine Messung.
- **Kein `niri msg`** (der ganze Screenshot-Abschnitt in §12 gilt nur für B), und Builds sind auf
  4 Kernen deutlich langsamer — das ist keine Regression, sondern der N100.

### Auf **B** gilt zusätzlich

Volle Grafik, also volle Beweispflicht: hier werden die Screenshots gemacht, hier wird gemessen,
hier wird aus 🟨 ein 🟧. Und hier ist die Platte der Feind (siehe unten) — **`df -h /home` vor dem
ersten Build.**

**Schreib das Ergebnis in `docs/umgebung.md`** (eine Tabelle pro Maschine: Hostname, Grafik ja/nein,
Blender, `gh`, Kerne, freie Platte, Datum). Der nächste Agent soll es nicht wieder herausfinden —
und beim Lesen einer alten Messung wissen, **wo** sie entstanden ist.

---

## 15. Fallen der Umgebung (teuer bezahlt, generisch, gelten hier auch)

- ⚠️ **`ld: signal 7 / Bus error` beim Linken heißt: die Platte ist voll.** `target/debug/deps`
  staut Bevy-Binaries im dreistelligen GB-Bereich. **Erst `df -h /home`**, nicht den Code
  verdächtigen. Aufräumen: `cargo clean` bzw. gezielt `rm -rf target/debug/incremental`.
- ⚠️ **`undefined hidden symbol: anon.…llvm.…`** = kaputter Inkrement-Cache nach einem
  abgewürgten Build → `rm -rf target/debug/incremental`.
- **`cargo check 2>&1 | grep '^error'`** — **nicht** auf `.rs` filtern, das findet auch Warnungen
  und lässt dich einem roten Build nachjagen, der grün ist.
- **`pkill` NIE an den Anfang einer Befehlskette.** Läuft kein Prozess, gibt es Exit 1 und der
  Rest der Kette wird verschluckt — und du glaubst, ein Rückbau sei passiert.
  `pkill -f target/debug/defeated_by_titan` liefert auch mal Exit 144: normal.
- **Mehrere Agenten arbeiten evtl. parallel in diesem Repo.** Dateien ändern sich unter dir, der
  Build ist zwischendurch rot ohne dein Zutun. **Vor jedem Edit die Datei frisch lesen.**
  **NIEMALS `git stash` / `git checkout --` / `git clean -fdx`**, während jemand anders arbeitet.
- **Eine RON-Datei wird als GANZE Datei geschrieben — zwei Sitzungen mergen nicht.** Wer zuletzt
  speichert, gewinnt alles. **Nach jedem Schreiben in eine geteilte Datei per `grep` prüfen, dass
  dein Wert drinsteht.** Und: trag in `docs/STATUS.md` ein, an welcher Domäne du gerade arbeitest,
  damit ein anderer Agent eine andere nimmt.
- **Temporäre Hacks zum Screenshotten immer mit `// TEMP` markieren** und danach
  `grep -rn TEMP src/` prüfen. Ein vergessener Test-Hack ist ein Geist, den der Nächste jagt.

---

## 16. Abnahme dieses Auftrags

Am Ende der Sitzung will ich sehen:

1. **`cargo test`** — die Ausgabe, ungekürzt zusammengefasst (wie viele grün, welche rot und warum).
2. **Mindestens zwei Screenshots** in `docs/bilder/`: einer aus der Stadt (Blick beim Schwingen),
   einer mit einem Titanen im Bild — **auf Maschine B**. Auf A (headless, §14) stattdessen die
   `--headless`-Skriptläufe mit ihren `assert`-Ergebnissen und der Vermerk „Pixel ungesehen".
3. **`docs/STATUS.md`**, in dem jede Sache eine der vier Stufen trägt — und **kein einziges ✅**,
   weil das der User setzt.
4. **`docs/ABNAHME.md`** mit der Liste dessen, worauf der User schauen soll.
5. **Die Modell-Tabelle** aus `cargo test --test modelle -- --ignored --nocapture`, und mindestens
   eine **`.blend`, die ich öffnen kann** — mit den Ankern (`cortex`, `hit.min`/`hit.max`, …) schon
   an ihrem Platz, sodass Nachmodellieren heißt: Geometrie austauschen, speichern, starten.
   Dazu die Liste, welches Modell noch Platzhalter ist (⬜/🟨) und welches echt (🟧).
6. Einen ehrlichen Absatz: **was gebaut, aber nicht gesehen ist.** Eine grüne Metrik ist keine
   Abnahme. Was du nicht gesehen hast, markierst du als nicht gesehen — auch wenn der Code stimmt
   und die Tests grün sind.

**Die eine Regel über allen: erst messen, dann behaupten.** Fast jeder teure Fehler in einem
Projekt wie diesem ist eine Stelle, an der etwas Vernünftiges *erklärt* wurde, statt es in einer
Minute zu *messen* — und die Erklärung war das Problem.

---

## 17. ⭐ Wie abgearbeitet wird: **parallel und wissenschaftlich** (Supervisor, Workflows, Experten)

Diese Datei ist der **Auftrag**, nicht die Arbeitsreihenfolge eines einzelnen Kopfes. Gearbeitet wird
mit Workflows und Subagenten — **breit parallel** und mit **wissenschaftlicher Methode**. Beides ist
verbindlich, nicht empfohlen:

### Supervisor & Fachexperten

**Ein Supervisor läuft dauerhaft im `/loop`** und triggert Workflows und Subagenten, die parallel am
Projekt arbeiten. **Er schreibt selbst nichts** — er plant, delegiert, prüft und integriert.

**Loop je Iteration:**

> Ist-Zustand → Hypothese + Abnahmekriterien → parallele Delegation → Ergebnisse sammeln → gegen
> Kriterien prüfen → integrieren und über die nächste Runde entscheiden.

**Abbruch** bei erfüllter DoD, bei erreichtem Limit, oder wenn **zweimal dieselbe Hypothese
gescheitert** ist (dann ist nicht die Ausführung falsch, sondern die Annahme → `docs/FRAGEN.md`).

**Parallelität:** **Jede Datei hat genau einen schreibberechtigten Agenten.** **Schnittstellen
werden vor der Parallelisierung fixiert** — erst der Vertrag (Components, Messages, Signaturen),
dann das Fan-out. Wer parallelisiert, bevor die Naht steht, integriert hinterher fünf Entwürfe
derselben Datei.

**Für jeden aus dem Projekt abgeleiteten Fachbereich wird ein Senior-Experte angelegt** (Vector-Gear-Physik,
Titanen-Verhalten, Rendering/3D-Pipeline, Daten/RON, Tooling & Test, Doku & Status …): er
**entscheidet eigenverantwortlich in seiner Domäne**, hält **alle** Projektrichtlinien ein und
**benennt Abweichungen explizit, statt sie zu umgehen** (eine benannte Abweichung ist eine
Entscheidung, eine stille ist ein Bug mit Anlauf).

**Vorgehen — verbindlich für alle, Supervisor wie Experten:**

> **falsifizierbare Hypothese → Prüfkriterium vorab festlegen → reproduzierbar messen → auswerten.**

**Jede Aussage mit Beleg. Annahmen als `ANNAHME:` markieren. Unsicherheit ausweisen. Nichts
erfinden. Bei Unklarheit messen oder eskalieren — nicht raten.** Das ist dieselbe Regel wie §9, nur
aus der Sicht des Delegierenden: **ein Ergebnis ohne vorab festgelegtes Prüfkriterium ist kein
Ergebnis, sondern eine Meinung.**

### Parallelisieren ist die **Voreinstellung**, nicht die Ausnahme

**Die Frage ist nie „kann man das parallel machen?", sondern „warum nicht?".** Serielles Arbeiten
ist nur dort richtig, wo eine **Datei einen einzigen Schreiber** braucht (die Liste oben) oder wo
Stufe N das **Ergebnis** von N−1 wirklich braucht. Alles andere läuft gleichzeitig.

**Wie zerschnitten wird — in dieser Vorzugsordnung:**

1. **Nach Domäne** — der natürliche Schnitt, weil Dateibesitz und Domäne dasselbe sind (§5).
   `vector/`, `titan/`, `world/`, `hud/` gleichzeitig, sobald die Schnittstellen stehen.
2. **Nach `F-ID`** — unabhängige Features aus der Liste (§2) laufen parallel; `abhaengt_von`
   sagt dir, was **nicht** gleichzeitig geht. Genau dafür steht das Feld in `features.ron`.
3. **Nach Prüf-Dimension** — dasselbe Stück Code von mehreren Seiten gleichzeitig: Korrektheit,
   Ränder, Performance, „was passiert im Netz" (§6). Vier Blickwinkel finden vier verschiedene
   Sachen; vier identische Prüfer finden dreimal dasselbe.
4. **Nach Datei bei Massenarbeit** — ein Agent pro `tools/blend/*.py`, pro Doku-Datei, pro
   Excel-Blatt.

**Die Breite kommt von der Maschine, nicht vom Wunsch** (§14): `nproc` ist der Deckel, und der
**Compiler ist auch ein Verbraucher**. Auf Maschine A (4 Kerne) heißt das 2–3 gleichzeitig; auf B
(16 Threads) 8 und mehr. Zwanzig Agenten auf vier Kernen sind langsamer als drei — sie warten nur
gemeinsam.

**Pipeline statt Barriere.** Ein Feature, das fertig geprüft ist, wartet nicht darauf, dass fünf
andere fertig werden. Nur wo eine Stufe **alle** Vorergebnisse zusammen braucht (dedupliieren,
Gesamtzählung, „gibt es überhaupt Funde?"), ist ein Sammelpunkt richtig.

**Was jede Parallelisierung vorher braucht** — sonst produziert sie Integrationsarbeit statt
Fortschritt: **die Schnittstelle steht** (Components, Messages, Signaturen sind festgeschrieben und
committet), **der Dateibesitz ist verteilt**, **das Abnahmekriterium ist notiert**. Danach:
`cargo check` + `cargo test` nach jedem Zusammenlauf — fünf einzeln grüne Agenten sind zusammen
nicht automatisch grün.

### Wissenschaftlich heißt: **messbar, reproduzierbar, widerlegbar**

Nicht als Attitüde, sondern als Arbeitsweise — für jede Behauptung, von „der Haken hält nicht" bis
„das ist schneller":

1. **Erst den Ist-Zustand messen, dann ändern.** Die Basismessung **vor** dem Eingriff ist der
   wichtigste Wert überhaupt: ohne sie „behebst" du Dinge, die nie kaputt waren, und weißt nie, ob
   deine Änderung überhaupt etwas getan hat.
2. **Hypothese hinschreiben, bevor gemessen wird** — falsifizierbar formuliert („wenn X, dann
   sinkt Y unter Z"), samt Prüfkriterium. Eine Erklärung, die nach der Messung entsteht, passt
   immer; sie ist deshalb wertlos.
3. **Eine Variable pro Experiment.** Zwei Änderungen gleichzeitig, und du weißt am Ende nur, dass
   *irgendwas* half.
4. **Reproduzierbar heißt: der Befehl steht daneben** — komplette Kommandozeile, Seed, Koordinate,
   Blickrichtung, **Maschine** (`[cachy]`/`[debian]`, §14). Was niemand nachstellen kann, ist keine
   Messung, sondern eine Anekdote.
5. **Zeiten mehrfach messen.** Ein Lauf ist Rauschen: N Läufe, **Median und Perzentil**, nicht
   Mittelwert. Und niemals über Maschinen hinweg vergleichen.
6. **Erst widerlegen versuchen, dann glauben.** Zu jeder Behauptung ein unabhängiger Versuch, sie
   zu **kippen** („finde den Fall, in dem das falsch ist"). Was keinen Angriff überlebt hat, ist
   🟨 (§8).
7. **Ein negatives Ergebnis ist ein Ergebnis** und wird aufgeschrieben (`docs/lessons/`,
   `docs/BUGS.md`) — sonst probiert es in drei Wochen jemand genauso wieder.
8. **Annahmen als `ANNAHME:` markieren, Unsicherheit ausweisen, nichts erfinden.** Bei Unklarheit
   **messen oder eskalieren** — nicht raten (§9).

### Und praktisch:

**Erst seriell, dann breit.** Stufe 0–1 macht **ein** Kopf allein: Skelett, `Cargo.toml`,
`main.rs`, die Domänenordner mit leeren Plugins, das `docs/`-Skelett. Vorher gibt es nichts, worauf
mehrere gleichzeitig arbeiten könnten — ein Fan-out auf einen leeren Ordner erzeugt fünf
inkompatible Entwürfe derselben Datei.

**Diese Dateien fasst NUR der Hauptkopf an** (ein Schreiber pro Datei, §5 — sie werden als *ganze*
Datei geschrieben, zwei Agenten mergen nicht):

| Datei | warum |
|---|---|
| `Cargo.toml` | zwei Agenten, zwei Dependency-Listen, eine überlebt |
| `src/main.rs` | **die Plugin-Liste** ist die Naht des ganzen Projekts |
| `src/lib.rs` | dito für die Modulliste |
| `assets/data/*.ron` | RON wird als ganze Datei geschrieben — verlorene Zeilen sieht niemand |
| `docs/STATUS.md`, `docs/TODO.md` | **der Hauptkopf trägt ein**, Subagenten *melden* nur |

**Das geht gut parallel:** ein Agent pro **Domäne** (`vector/`, `titan/`, `world/`, `hud/` …), sobald
das Skelett steht · ein Agent pro `tools/blend/*.py` · ein Agent pro Doku-Datei · ein Agent, der in
der **installierten** Bevy-Doku nachsieht, wie eine API dieser Version wirklich heißt (§3).

**Was jeder Subagent im Auftrag stehen haben muss** — sonst liefert er Plausibles statt Richtiges:

1. **Welche Dateien ihm gehören** und welche er *nur lesen* darf.
2. **Welche Abschnitte er lesen soll** — z. B. „`prompts/init.md` §5 + §8 + §9, `docs/architektur.md`".
   Nicht „lies alles": ein Subagent mit 800 Zeilen Auftrag baut den halben Prompt nach.
3. **Die Belegpflicht (§9):** was er behauptet, misst er. Sein Rückgabewert enthält **Testnamen,
   Messwert und die Stufe (§8)** — nicht „habe es implementiert".
4. **Kein Fremdgebiet.** Was ihm auffällt, aber nicht gehört: nach `docs/FUNDE.md`, nicht still
   mitfixen.

**Prüfen gehört in den Workflow, nicht in die Hoffnung.** Nach jeder Findungs-/Baustufe eine
**unabhängige** Stufe, die versucht, die Behauptung zu **widerlegen** (anderer Agent, Auftrag:
„finde den Fall, in dem das kaputtgeht"). Eine Behauptung, die niemand angegriffen hat, ist 🟨.

**Nach jedem Fan-out: `cargo check 2>&1 | grep '^error'` und `cargo test`.** Fünf Agenten, die
einzeln grün waren, sind zusammen nicht automatisch grün — jeder hat nur seine Hälfte gesehen.

**Und quer über alles: `docs/STATUS.md` ist die einzige Wahrheit über den Fortschritt.** Ein
Workflow, der etwas gebaut hat, ist nicht fertig, solange die Zeile fehlt.

---

## 18. Die Ziellinie: übertragen, Repo anlegen, und **diese Datei löschen**

Wenn **alles aus dieser Datei abgearbeitet** ist — der Baum steht, die Modellkette läuft, der
Stufenplan ist durch, `docs/` stimmt — dann kommen drei Schritte, **in dieser Reihenfolge**.

### Schritt 1: Übertragen — diese Datei darf am Ende nichts Einzigartiges mehr enthalten

Alles hier Stehende hat ein dauerhaftes Zuhause. **Übertragen heißt umschreiben, nicht
hineinkopieren** — was aus dem Code oder der Doku schon hervorgeht, wird gekürzt; was ein
zukünftiger Agent braucht, wird ausformuliert.

| Aus `prompts/` | Wohin dauerhaft |
|---|---|
| Regeln, die **immer** gelten (RON, Domänen-Standalone, vier Stufen, Bug-Doktrin, „pro Sekunde") | `CLAUDE.md` — als **Index**, kurz, mit Zeigern |
| Bevy-Setup + die Engine-Fallen (§3) | `docs/lessons/bevy.md` |
| Achsen, Einheiten, Blickrichtung (§3) | `docs/konventionen.md` |
| Domänen, Plugin-Reihenfolge, Erlaubnisliste (§5) | `docs/architektur.md` |
| Die Modellkette (§7) | `docs/modelle.md` (+ Kommentarkopf in jedem `tools/blend/*.py`) |
| Die vier Stufen (§8) | Kopf von `docs/STATUS.md`, Kurzform in `CLAUDE.md` |
| Die Bug- und Sicherheitsdoktrin (§9) | Kopf von `docs/BUGS.md`, Kurzform in `CLAUDE.md` |
| Performance-Regeln (§11) | `docs/lessons/performance.md` |
| Flags, `--script`, Screenshots (§12) | `docs/lessons/workflow.md` + Tastenbelegung in `README.md` |
| Zwei Maschinen + Umgebungsfallen (§14 + §15) | `docs/lessons/umgebung.md` |
| Arbeitsweise: Parallelität + Methode (§17) | `docs/lessons/arbeitsweise.md` |
| Spielinhalt und Vorbild (§1) | `docs/gameplay/` (zusammen mit dem, was aus `gameplay/` kam) |
| Was bewusst später kommt (§1) | `docs/ROADMAP.md` |

**Prüfen, nicht glauben:** `grep -rn "prompts/init.md" . --exclude-dir=target` darf nur noch die eine
Zeile in `CLAUDE.md` finden, die sagt, dass der Initialprompt abgearbeitet und in der Historie
nachlesbar ist. Und lies `CLAUDE.md` einmal wie ein Fremder: *sagt sie mir in dreißig Sekunden,
wie das Projekt tickt und wo die Fallen liegen?* Wenn nein, ist der Schritt nicht fertig.

### Schritt 2: Vor dem Veröffentlichen aufräumen — **öffentlich heißt öffentlich**

Was einmal gepusht ist, ist indexierbar, auch nach dem Löschen. Der Check kommt **vor** dem Push:

- `.gitignore`: `target/`, `saves/`, `*.blend1`, `*.blend2`, temporäre Skripte, Logs, **`assets/extern/`**.
  **`assets/3d/glb/` NICHT ignorieren** (§7 — das Spiel muss ohne Blender laufen).
- **Keine Zugangsdaten, Tokens, Keys, Pfade mit Klarnamen** — im Arbeitsbaum *und* in der
  Historie (`git log -p | grep -niE "token|secret|api[_-]?key|password"`).
- **`assets/extern/` ist im `.gitignore`**, `HERKUNFT.md` ist vollständig (jede Datei eine Zeile) und
  `tools/hole_extern.sh` beschafft alles wieder (§7). Der Ersetzungsbericht liegt dem User vor.
- `cargo test` grün, `cargo build --release` grün, `grep -rn "TEMP" src/` leer.
- `README.md` liest sich für einen Fremden: was ist das, wie starte ich es, was ist fertig
  (mit der Stufen-Legende), welche Tasten. Dazu eine `LICENSE` — welche, fragst du in
  `docs/FRAGEN.md`, wenn der User nichts gesagt hat (bis dahin: keine Lizenzdatei erfinden).

### Schritt 3: Ein **öffentliches** GitHub-Repo anlegen

Der User hat das hier vorab erlaubt — es ist Teil des Auftrags, keine Nachfrage nötig. Aber erst
**nach** Schritt 1 und 2.

**Erst nachsehen, ob es das Repo schon gibt** (das Gerüst wurde beim Anlegen des Auftrags schon
einmal gepusht — dann wird nur noch gepusht, nicht neu angelegt):

```bash
gh auth status                      # nicht angemeldet? → den User bitten: `! gh auth login` tippen
git add -A && git commit -m "Aufsetzen abgeschlossen: Baum, Modellkette, Doku"

git remote -v                       # gibt es 'origin'?
git push -u origin main             # ja → nur pushen

gh repo create defeated-by-titan --public --source=. --remote=origin --push \
   --description "3D-Titanenkampfspiel in Bevy (Rust) — Vector Gear, Cortex-Treffer, Koop-Einsätze"
                                    # nein → so anlegen
```

Danach **die URL im Chat nennen** und einmal `gh repo view --web` erwähnen. Wenn `gh` fehlt oder
nicht angemeldet ist: **nicht mit `git remote add` und einem geratenen URL improvisieren** — den
User um den Login bitten und den Rest fertig machen.

### Schritt 4: **Das Gerüst auflösen** — `prompts/`, `gameplay/` und `init.md` gehen weg

`prompts/` und `gameplay/` sind **Bootstrap-Gerüst**, kein Teil des fertigen Projekts. Wenn ihr
Inhalt in der echten Struktur angekommen ist (Schritt 1), wird das Gerüst abgebaut — **in dieser
Reihenfolge, jede Zeile ein eigener Commit, damit die Historie zeigt, wohin was gewandert ist:**

```bash
# 1. der Eingangskorb: Design nach docs/gameplay/, Arbeit nach docs/TODO.md, Zahlen in die RON,
#    Bilder nach docs/gameplay/bilder/ — DANN erst:
git rm -r gameplay/
git commit -m "gameplay/ aufgelöst: Design -> docs/gameplay/, Arbeit -> docs/TODO.md, Zahlen -> assets/data/"

# 2. die Prompts, einzeln, jede erst wenn abgearbeitet UND übertragen:
git rm prompts/init.md
git commit -m "Initialprompt abgearbeitet und übertragen (steht in der Historie)"
git rm -r prompts/ 2>/dev/null   # nur wenn leer
git rm init.md                    # der Starter im Wurzelverzeichnis, zuletzt
git commit -m "Bootstrap-Gerüst entfernt — ab hier fuehren README, CLAUDE.md und docs/"
git push
```

**Vor jedem `git rm`: die Verweise umschreiben, nicht danach.** Ein Umzug ist erst fertig, wenn

```bash
grep -rn -e 'prompts/' -e 'gameplay/' . --exclude-dir=target --exclude-dir=.git
```

nur noch Treffer zeigt, die **absichtlich** von der Historie sprechen. Jeder Link in `README.md`,
`CLAUDE.md`, `docs/**`, in den RON-Dateien und in den `tools/`-Skripten zeigt danach auf den neuen
Ort. Zusätzlich läuft `tools/normen.py` (§10): **kein toter Markdown-Link, keine unreferenzierte
Datei.** Beides ist Teil dieses Schritts, nicht „Aufräumen später".

**Und dem User sagen, wo seine Wünsche ab jetzt hingehen:** der Eingangskorb ist nach der Auflösung
`docs/gameplay/` (Design, eine Datei pro Thema) plus `docs/TODO.md` (die offene Arbeit). Schreib
diese eine Zeile in die `README.md` — sonst legt er morgen eine Datei in einen Ordner, den es nicht
mehr gibt.

⚠️ **Nicht löschen, was du nicht übertragen hast**, und nichts, was der User gerade neu
hineingelegt hat (`ls -lt prompts/ gameplay/` vor dem Abbau). Im Zweifel: liegen lassen und
fragen. Die Historie ist ein Netz, aber nur, wenn der Commit auch gepusht wurde.

Und in `CLAUDE.md` bleibt **eine** Zeile stehen: *„Der Initialprompt ist abgearbeitet und
gelöscht; er ist in der Git-Historie nachlesbar (`git show <sha>:prompts/init.md`)."* Die Historie ist
das Sicherheitsnetz — deshalb wird gelöscht und nicht bloß umbenannt.

**Und die anderen Dateien in `prompts/`?** Genauso: **jede wird gelöscht, sobald sie abgearbeitet
und übertragen ist** — einzeln, mit einer Commit-Message, die sagt, wohin ihr Inhalt gewandert ist.
Was noch offen ist, bleibt liegen; dann bleibt auch der Ordner. Ist `prompts/` am Ende leer, kommt
er weg. **Nie eine Datei löschen, die du nicht vollständig abgearbeitet hast**, und nie eine, die
der User gerade neu hineingelegt hat.

⚠️ **Nicht früher löschen.** Solange irgendetwas nur hier steht, ist diese Datei das Gedächtnis
des Projekts. Erst wenn Schritt 1 belegt durch ist, ist sie überflüssig — und dann ist sie
Ballast, weil zwei Quellen für dieselbe Regel bedeuten, dass eine von ihnen bald lügt.

**Ab diesem Moment ist `gameplay/` die Arbeitsvorlage (§2), nicht mehr diese Datei.**
