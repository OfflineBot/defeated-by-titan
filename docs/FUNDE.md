# FUNDE — Fehler, die mir nebenbei aufgefallen sind

Stand: 2026-08-09

**Wer beim Arbeiten ueber etwas stolpert, das nicht zur eigenen Aufgabe gehoert: aufschreiben,
mit der Messung daneben** — damit ein anderer pruefen kann, ob es wirklich falsch ist.

**Nicht still mitfixen.** Ein nebenbei mitgefixter Fremdfehler ist ein Fix, den niemand
geprueft hat, und er versteckt sich im Diff einer Aufgabe, in der ihn keiner sucht
(`prompts/init.md` §9c). Format: `FUND-00n <Symptom>` + Messung.

---

## FUND-001 — Die Ankerdichte im Backlog ist keine Zahl

**Symptom:** `prompts/init.md` §2 nennt die Ankerdichte „die wichtigste Zahl" in Blatt
`08_Maps`. In der Tabelle steht sie qualitativ.

**Messung:** Alle 12 Map-Zeilen tragen `Sehr hoch` (3×), `Hoch` (4×), `Mittel` (4×) oder
`Niedrig` (2×) — `grep -o 'ankerdichte: "[^"]*"' docs/backlog/maps.ron`. Kein einziger
Zahlenwert.

**Warum das zaehlt:** Bibel 6.2 macht die Ankerdichte zum Gate von P3 („Traversal-Zeiten
zeigen messbaren Unterschied zwischen Anfaenger und Experte"). Vier Woerter lassen sich nicht
tunen und nicht pruefen.

**Gehoert wem:** Leveldesign, nicht dem Aufsetzen. Als Entscheidung erfasst in
[`docs/FRAGEN.md`](FRAGEN.md) Q-010.

## FUND-002 — Der Backlog ist an mehreren Stellen fuer Roblox geschrieben

**Symptom:** Nicht nur die Bibel, auch `01_Spielfunktionen` nennt Roblox-Bausteine direkt.

**Messung:** `F-003` verlangt „Oberflaechen mit CollectionService-Tag `AnchorSurface`",
`F-004` nennt `RopeConstraint`, Blatt `05_VFX` nennt `ParticleEmitter + Beam`, Blatt `08_Maps`
misst in `studs`, `T-001` heisst „Rojo- und Git-Aufsetzung".

**Warum das zaehlt:** `prompts/init.md` §2 regelt die **Bibel**-Stellen und sagt: was beim
Arbeiten zusaetzlich auftaucht, wird **uebersetzt und in `docs/architektur.md` nachgetragen**
— nicht befolgt, nicht ignoriert, nicht zurueckgefragt. Genau das ist passiert: die
Uebersetzungstabelle in [`docs/architektur.md`](architektur.md) hat jetzt vier Zeilen mehr.

**Gehoert wem:** erledigt, kein offener Punkt. Steht hier, damit der Naechste weiss, dass die
Tabelle waechst statt fest zu sein.

## FUND-003 — Blatt 11 der Excel-Tabelle ist eine unabhaengige Gegenprobe, keine Datenquelle

**Symptom:** `prompts/init.md` §2 warnt, Blatt 11 bestehe aus Formeln und liefere ohne
`data_only=True` `=COUNTIF(...)` statt Zahlen — moeglicherweise sogar ohne
zwischengespeicherten Wert.

**Messung:** Alle 47 Formelzellen haben einen zwischengespeicherten Wert (`<v>` neben `<f>`);
`tools/features.py` warnt, wenn das je nicht mehr so ist. Die Werte stimmen mit der eigenen
Extraktion **exakt** ueberein: 194 / 100 / 100 / 28 / 39 / 118 / 45 / 12 / 51, Summe **687**,
und die Prio-Verteilung von Blatt 01 (99 Must / 71 Should / 24 Could) ebenfalls.

**Warum das zaehlt:** Damit ist die Extraktion **von der Tabelle selbst bestaetigt**, nicht
nur von der eigenen Zaehlung. Das ist mehr wert als der Zeilenzahl-Pruefwert allein — die
Zahlen kommen aus einer anderen Rechnung (`COUNTA` in Excel) als unsere.

**Gehoert wem:** erledigt.

---

*(Weitere Funde hier anhaengen. Ein Fund ohne Messung ist eine Meinung.)*

Verwandt: [`docs/BUGS.md`](BUGS.md) (eigene Bugs) · [`docs/FRAGEN.md`](FRAGEN.md)
