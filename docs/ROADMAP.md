# ROADMAP — was bewusst spaeter kommt

Stand: 2026-08-09

**Hier steht, was erfasst, verstanden und absichtlich nicht gebaut ist.** Der Unterschied zu
`docs/TODO.md`: dort steht Arbeit, die drankommt. Hier steht Arbeit, die **nicht** drankommt,
und **warum** — damit sie niemand aus Versehen anfaengt und niemand glaubt, sie sei vergessen.

## Die Regel, die alles andere sortiert

> **Kein Meta-System vor bestandenem Vector-Gear-Gate.**
> Faehigkeitsbaum, Wirtschaft, Lineages, Raids und Kosmetik werden **nicht angefangen**,
> solange sich die Bewegung nicht ueberzeugend anfuehlt (Bibel 6.1, `prompts/init.md` §2).

Der Friedhof des Genres besteht aus Spielen mit ausgefeilten Faehigkeitsbaeumen und einer
Bewegung, die sich falsch anfuehlt. **Das P1-Gate ist ein Blindtest gegen die Referenz mit
zehn Testern; unsere Bewegung muss mindestens gleichauf bewertet werden. Nicht bestanden
heisst iterieren, nicht weitergehen.**

## Nach dem Gate, in dieser Reihenfolge (Bibel 6.2)

| Phase | Inhalt | Gate |
|---|---|---|
| **P2 Kampf-Kern** | ein Titan mit vollem Angriffs- und Todeszyklus, Cortex-Trefferzone, Klingenhaltbarkeit, Nachschub, Hit-Stop | Eine Minute Kampf gegen einen einzelnen Titanen macht **ohne jede Belohnung** Spass |
| **P3 Erste Map** | Ashgate District als Graybox mit getunter Ankerdichte, dann Art-Pass | Traversal-Zeiten zeigen messbaren Unterschied zwischen Anfaenger und Experte |
| **P4 Missionsschleife** | Skirmish und Breach, Direktor-System, Auswertung, Belohnung | Ein Spieler spielt freiwillig drei Missionen hintereinander |
| **P5 Onboarding** | vier Tutorialstufen, Trainingsgelaende, adaptive Hinweise | 80 % Abschlussquote der Erstmission |
| **P6 Gegnervielfalt** | alle acht Typen, Groessenklassen, Gruppendynamik | Testspieler koennen jeden Typ benennen und seine Konterstrategie erklaeren |
| **P7 Progression** | Level, Ausruestungsbudget, Faehigkeitenbaum, Traits, Lineages mit Pity, Kompendium | Vier verschiedene Builds liegen bei der Effektivitaet innerhalb von 10 % |
| **P8 Inhaltsausbau** | Maps 2–5, Missionsmodi, Modifikatoren, Traversal Trial | Jede Map hat eine erkennbar eigene Traversal-Identitaet |
| **P9 Raids** | Raid-Framework, zwei Bosse, Matchmaking, Beute, Umgebungswaffen | Eine Gruppe scheitert beim ersten Versuch und will sofort wieder |
| **P10 Vessel Forms** | Bonding, zwei Formen mit eigenem Moveset | Die Form fuehlt sich nicht wie ein vergroesserter Spieler an |
| **P11 Politur** | Barrierefreiheit, Latenz- und Lasttests, Telemetrie, Saisonstruktur | Lasttest mit 20 Spielern und vollem Titanenbudget ohne Framedrop |

## Ausdruecklich nicht heute

| Sache | warum spaeter | wo erfasst |
|---|---|---|
| **Bonding / Vessel Forms** (9 Zeilen) | teuerster Einzelposten: eigene Rigs, ~60 Animationen, eigenes Balancing — und sie **ersetzen** das Kern-Movement, statt es zu erweitern | `docs/FRAGEN.md` Q-004 (v1.0 oder v1.5) |
| **Lance Charge** | Fernwaffe; setzt den Kampf-Kern voraus | `docs/features.ron` |
| **Pferde** | Fortbewegung ausserhalb des Vector Gear; erst wenn das Gear steht | `prompts/init.md` §1 |
| **Der eigentliche Netzcode** | die *Architektur* wird ab Tag 1 dafuer gebaut, der *Code* nicht heute | [`docs/multiplayer.md`](multiplayer.md) |
| **Raids und die vier Raid-Bosse** (The Bound One, The Dancer, The Bulwark, The Ashwalker) | Meta-System — faellt unter die Gate-Regel | `docs/backlog/funktionen.ron` |
| **Store / Saisonpass / Monetarisierung** (10 Zeilen) | ausserhalb von Roblox eine offene Produktfrage; **nichts davon wird gebaut** | `docs/FRAGEN.md` Q-001 |
| **Handel zwischen Spielern** | Betrug, Schwarzmaerkte, Supportaufwand; Nutzen sinkt durch Pfeiler P3 | `docs/FRAGEN.md` Q-005 |
| **Schatten** | der teuerste Schalter im Spiel. **Erst am Ende, mit Zahl.** | `docs/lessons/performance.md` |

## Was aus dem Backlog schon zaehlt, obwohl es spaet kommt

- **Der Traversal Trial** ist kein Nebenmodus, sondern der **Lackmustest des Projekts**
  (Bibel 2/P1): ein Spieler, der elegant durch die Stadt fliegt, ohne einen einzigen Titanen
  zu toeten, muss Spass haben. Er nutzt bestehende Maps und kostet fast nichts — aber er
  kommt erst, wenn es eine Map gibt.
- **Mindestens die Haelfte aller Gegnertypen hat eine Anti-Autopilot-Eigenschaft** (Bibel 4).
  Das ist eine Vorgabe an P6, kein Feature, das man nachtraegt.

Verwandt: [`docs/TODO.md`](TODO.md) · [`docs/FRAGEN.md`](FRAGEN.md) · [`docs/STATUS.md`](STATUS.md)
