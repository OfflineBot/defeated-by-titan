# multiplayer — der Plan, der heute noch nicht gebaut wird

Stand: 2026-08-09 · Stufe: ⬜ (nichts davon ist gebaut — gebaut ist nur die **Naht**, `src/net/`)

**Der Netzcode ist nicht Teil dieses Auftrags.** Kein Server, keine Prediction, keine
Lag-Kompensation. **Aber jede Entscheidung, die Multiplayer spaeter unmoeglich oder teuer
macht, wird heute vermieden** — ein fertiges Einzelspieler-Spiel netzfaehig zu machen heisst
normalerweise, die Simulation neu zu schreiben (`prompts/init.md` §6).

## Die acht Regeln, und wo sie im Code stehen

| # | Regel | wo sie heute schon gilt |
|---|---|---|
| 1 | **Simulation und Darstellung sind getrennt.** Simulation liest Eingaben + Zustand und schreibt Zustand; Rendering, HUD und Sound **lesen nur**. | Simulation laeuft in `FixedUpdate`, `render`/`hud`/`sound` in `Update` |
| 2 | **Eingabe ist ein Datum, kein Tastendruck.** Es gibt **ein** `Intent` (Bewegung, Blick, Tasten, Tick), und die Simulation liest nur das. | `shared::Intent`; gefuellt von `net` — aus Tastatur **oder** Skript **oder** spaeter dem Netz |
| 3 | **Es gibt keinen „den Spieler".** Nie `.single()`. Jeder Spieler ist einer von vielen. | `shared::PlayerId`; Gas/Klingen sind **Components am Spieler**, nie eine `Resource`. `LocalPlayer` ist die einzige Stelle, die weiss, wer „ich" ist |
| 4 | **Fester Simulationsschritt** (60 Hz), das Bild interpoliert dazwischen. | `Time<Fixed>` auf 60 Hz in `main.rs` |
| 5 | **Determinismus, wo er billig ist.** Zufall nur aus einem geseedeten Generator, dessen Seed Teil des Zustands ist. | `shared::Wuerfel` (`seed + tick`), nie `rand::random()` mitten in einem System |
| 6 | **Autoritaet wird benannt.** In der Doku jeder Domaene steht, wer ein geteiltes Feld schreibt. | die Autoritaetstabelle in [`docs/architektur.md`](architektur.md) — **spaeter heisst dieser Satz „der Server"** |
| 7 | **Stabile Ids statt Zeiger.** Alles, was gespeichert oder verschickt wird, benutzt eigene Ids. | `PlayerId`, `TitanId` — **nie** Bevys `Entity` (lokaler Index mit Generation; auf einem anderen Rechner etwas anderes). Rettet nebenbei den Spielstand |
| 8 | **`serde` auf allem, was Zustand ist**, und Messages so entwerfen, dass sie ueber eine Leitung passen — Daten, keine Handles, keine `Entity`. | `#[derive(Serialize, Deserialize)]` auf allen `shared/`-Typen |

## Was die Bibel schon entschieden hat — keine offenen Fragen mehr

| Vorgabe (Bibel 3.6) | Konsequenz fuer den Code |
|---|---|
| **Eigene Bewegung beim Client**, alles andere beim Server (Titanen, Ziele, Schaden, Beute) | Die Trennung aus Regel 1 ist damit **vorgegeben**, nicht gewaehlt: Bewegung darf lokal sofort reagieren, ein Cortex-Treffer nie |
| **20 Spieler pro Einsatz, 10 pro Raid, 40 im Hub** | Nichts skaliert mit „einem Spieler". Zwanzig Spieler mit je **zwei Seilen** plus sechzig Titanen sind die eigentliche Belastungsprobe — nicht die Grafik |
| **Kein Schaden, keine Kollision zwischen Spielern** (F-162a, F-163a) | Zwei Spieler muessen sich in voller Fahrt durchdringen koennen; Knockback bleibt als taktisches Element |
| **Getrennte Beute pro Spieler** (F-160a) | Beute ist nie globaler Zustand. Jeder wuerfelt fuer sich — kein Wettlauf |
| **Kampfunfaehigkeit statt Sofort-Tod** (F-159a), Wiederbeleben durch Mitspieler | „tot" ist ein **Zustand mit Timer**, kein Entfernen der Entity → gehoert zu `squad/`. Alleinspieler bekommen eine begrenzte Selbstaufrichtung |
| **Kein Ausschluss in oeffentlichen Instanzen** (F-170a) | nur Melden und lokales Stummschalten |
| **Verbindungsabbruch reserviert den Platz 120 s** (F-158a) | Die Sitzung ueberlebt den Spieler; sein Zustand haengt an einer `PlayerId`, nicht an einer Verbindung (Regel 7) |
| **T-019: jedes Bewegungsfeature wird bei 200 ms simulierter Latenz getestet** | **Der Verzoegerungs-Schalter gehoert ins Werkzeug** (`--lag 200`), nicht in ein spaeteres Ticket: „fuehlt sich lokal gut an" ist keine Abnahme |

## Die Naht: `src/net/`

`NetPlugin` tut heute genau eines — den Transport **`LocalOnly`** bereitstellen, der die Intents
des lokalen Spielers in die Simulation schiebt. Damit ist der Ort, an dem spaeter Client und
Server stehen, **vorhanden und leer**, statt spaeter mitten durch fuenf Domaenen zu schneiden.

```
Tastatur ─┐
Skript   ─┼─► net::Posteingang ─► net::intents_zustellen ─► Intent am Spieler ─► Simulation
(Netz)   ─┘      (PlayerId → Roh-Intent)     FixedPreUpdate
```

**Drei Quellen, ein Kanal.** Der Skript-Fahrer ist kein zweiter, falscher Weg zu spielen — er
schreibt in denselben Posteingang wie die Tastatur, und jedes System dahinter ist das echte.
Genau dieser Kanal ist der, den Multiplayer braucht: **ein Aufwand, zwei Probleme geloest.**

## Was noch offen ist

Dediziert oder Host, und ob es bei den Bibel-Zahlen bleibt: [`docs/FRAGEN.md`](FRAGEN.md)
Q-008. PvP: Q-003. Nichts davon blockiert die Arbeit — `net/` ist transport-agnostisch.

## Der Waechter

**`tests/mehrspieler.rs`** spawnt **zwei** Spieler-Entities und laesst die Simulation ein paar
Ticks laufen. Er faellt in der Sekunde um, in der jemand `.single()` auf eine Spieler-Query
schreibt oder Spielerzustand in eine `Resource` legt. **Ohne ihn verfaellt dieses Dokument
still** — und man merkt es erst, wenn Multiplayer dran ist, also nach Monaten Arbeit, die man
dann anfassen muss.

Verwandt: [`docs/architektur.md`](architektur.md) · [`docs/lessons/arbeitsweise.md`](lessons/arbeitsweise.md)
