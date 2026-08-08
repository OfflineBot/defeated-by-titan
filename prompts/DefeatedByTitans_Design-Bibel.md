# DEFEATED BY TITANS — Design-Bibel

**Version 1.0 · Pre-Production**
Begleitdokument zum Produktions-Backlog (`DefeatedByTitans_Produktions-Backlog.xlsx`)

Das Backlog beantwortet *was gebaut wird*. Dieses Dokument beantwortet *warum*, *in welcher Reihenfolge* und *woran wir merken, dass es funktioniert*.

---

## 1. Das Spiel in einem Satz

> Ein Bewegungsspiel mit hoher Meisterschaftsgrenze, in dem Kämpfen der Nebeneffekt guter Bewegung ist.

Alles Weitere ist Ausschmückung. Wenn ein Feature dieser Aussage widerspricht, wird es gestrichen — unabhängig davon, wie viel Arbeit bereits hineingeflossen ist.

---

## 2. Die fünf Designpfeiler

### P1 — Bewegung ist das Produkt
Das Vector Gear ist kein Fortbewegungsmittel zwischen Kämpfen. Es *ist* der Kampf. Ein Spieler, der elegant durch eine Stadt fliegt, ohne einen einzigen Titanen zu töten, muss Spaß haben. Wenn das nicht funktioniert, funktioniert nichts.

**Konsequenz:** Der Traversal Trial (reiner Bewegungsmodus, kein Kampf) ist kein Nebenmodus, sondern der Lackmustest des gesamten Projekts.

### P2 — Können schlägt Zahlen
Ein Spieler auf Stufe 20 mit sauberer Technik muss eine Mission schneller abschließen als ein Spieler auf Stufe 90 mit schlechter Technik. Stat-Wachstum öffnet neue Inhalte, es ersetzt keine Fähigkeit.

**Messgröße:** Varianz der Time-to-Kill zwischen Anfängern und Experten bei identischem Build. Zielwert mindestens 2,5-fach.

### P3 — Kein Fortschritt ohne Garantie
Zufall darf beschleunigen, aber niemals der einzige Weg sein. Jedes Ziel im Spiel ist auf einem deterministischen Pfad erreichbar. Alle Wahrscheinlichkeiten und Zähler sind im Spiel einsehbar.

**Konsequenz:** Es gibt keine leeren Lineages, keine 0,05-Prozent-Sackgassen und keinen Gegenstand, den ein fleißiger Spieler nie erreichen kann.

### P4 — Lesbarkeit vor Realismus
Jeder Titanenangriff hat eine Ausholphase von mindestens 0,4 Sekunden. Der Cortex ist aus 100 Metern erkennbar. Jede Trefferart hat einen eigenen Klang. Der Spieler soll nie fragen, warum er gestorben ist.

### P5 — Der Store verkauft nur Aussehen
Kosmetik, Privatserver, Saisonpass. Keine Inventarplätze, keine Drop-Raten, keine Neuwürfe, keine Loadout-Slots. Das ist eine bewusste Umsatzentscheidung zugunsten von Ruf und Langlebigkeit.

---

## 3. Welt und Ton

### 3.1 Setting

Die Menschheit lebt in **drei konzentrischen Bastionsringen** — Ashgate außen, Ironrose in der Mitte, Highspire innen. Vor über hundert Jahren wurden die Ringe gegen die Titanen errichtet; seither ist niemand zurückgekehrt, der sie verlassen hat.

**Der zentrale Unterschied zur Vorlage:** Der Krieg ist bereits verloren. Der Titel ist keine Drohung, sondern eine Feststellung. Ashgate ist längst gefallen; die Vanguard führt Bergungsmissionen in die eigenen Ruinen, nicht Eroberungsfeldzüge. Das verschiebt den Ton von heroisch zu elegisch und rechtfertigt die Missionsstruktur — man rettet Karren, hält Positionen und zieht sich zurück, statt Gebiete zu erobern.

Die **Vanguard** ist keine Armee, sondern ein Bergungskorps. Ränge sind Handwerksgrade, keine Militärränge. Das erlaubt uns, Fortschritt als Kompetenzaufbau statt als Beförderung zu erzählen — passend zu P2.

### 3.2 Was Titanen sind

Bewusst unbeantwortet gelassen. Das Kompendium sammelt Feldbeobachtungen der Vanguard, keine Wahrheiten. Vessel Forms werden als "Ansteckung" beschrieben, nicht als Gabe — der Spieler nutzt etwas, das er nicht versteht und das ihn kostet.

**Erzählerische Regel:** Wir erklären nie mehr, als eine Figur im Feld wissen könnte.

### 3.3 Ton

Gedämpft, erwachsen, ohne Zynismus. Kein Splatter: Titanen verdampfen statt zu bluten, Wunden stoßen Dampf aus. Das ist zugleich eine Stiltscheidung *und* eine Moderationsentscheidung für die Roblox-Plattform.

### 3.4 Visueller Stil

**Low Poly mit weichen Normalen und flachen Farbflächen.** Kein kantiges Facettenmodell, kein PBR. Die gesamte Umgebung läuft über einen einzigen Farbatlas — das ergibt garantierte Farbkonsistenz, minimale Drawcalls und eine erkennbare Handschrift.

**Farbwelt:** Gedeckte Basis (Steingrau, Ziegelrot, Olivgrün, Sandbraun) mit exakt drei Signalfarben, die *ausschließlich* für Gameplay reserviert sind:

| Farbe | Bedeutung | Darf sonst nirgends vorkommen |
|---|---|---|
| Zyan | Gas, Vector Gear, Ankerpunkte | Keine zyanfarbene Umgebungsdeko |
| Bernstein | Cortex, Schwachstellen, Ziele | Keine bernsteinfarbenen Laternen |
| Karminrot | Gefahr, Schaden, kritischer Zustand | Keine roten Dächer |

Diese Regel ist unverhandelbar. Sie ist der Grund, warum ein Spieler bei voller Geschwindigkeit in einem Gefecht mit zwanzig Mitspielern noch erkennt, was für ihn relevant ist.

**Beleuchtung:** Future Lighting, ein starkes Richtungslicht, aggressiver Fernnebel für Tiefenstaffelung. Der Nebel arbeitet doppelt: Atmosphäre und Culling.

---

## 3.5 Plattform

**PC ausschließlich. Tastatur und Maus als einziges Eingabegerät.** Kein Mobile, kein Gamepad, kein Touch.

Das ist keine Einschränkung, sondern ein Designvorteil, und er zieht sich durch das ganze Backlog:

- Das Zielsystem darf auf Mauspräzision ausgelegt werden. Snap (F-024) wird damit zur Komfortoption statt zur Notwendigkeit — der Standardmodus bleibt *Assistiert*, und *Frei* ist eine realistische Wahl für ambitionierte Spieler.
- Es gibt keinen kleinsten gemeinsamen Nenner beim HUD. Wir können mehr Information gleichzeitig zeigen, weil kein Daumen die halbe Bildfläche verdeckt.
- Steuerungstiefe ist kein Problem. Q, E, B, C, F, Shift, Strg, Doppeltipps und Modifikatoren nebeneinander sind auf einer Tastatur zumutbar, auf einem Gamepad nicht.
- Zwei Qualitätsprofile statt fünf: **Mindestprofil** (Einsteiger-Laptop, integrierte Grafik) und **Vollprofil**. Beide zielen auf 60 FPS.

Die Barrierefreiheitsanforderungen bleiben vollständig bestehen — freie Tastenbelegung, Farbenblindmodi, Screenshake-Regler, Bewegungsreduktion. Eine Plattform weniger heißt nicht weniger Sorgfalt.

---

## 3.6 Mehrspieler

**Kooperativ, nicht kompetitiv.** Zwanzig Spieler pro Missionsinstanz, zehn pro Raid, vierzig im Hub.

### Die vier Grundregeln

Diese vier Entscheidungen bestimmen, ob sich Mitspieler wie ein Gewinn oder wie ein Hindernis anfühlen. Sie sind nicht verhandelbar:

1. **Kein Schaden zwischen Spielern** (F-162a). Knockback bleibt als taktisches Element, Schaden nicht.
2. **Keine Kollision zwischen Spielern** (F-163a). Bei dieser Bewegungsgeschwindigkeit ist Spielerkollision die größte Frustquelle überhaupt — zwei Spieler müssen sich in voller Fahrt durchdringen können.
3. **Getrennte Beute pro Spieler** (F-160a). Jeder würfelt für sich. Kein Wettlauf, kein Anreiz, Mitspieler zu benachteiligen.
4. **Kein Ausschluss in öffentlichen Instanzen** (F-170a). Nur Melden und lokales Stummschalten. Ausschluss per Mehrheit wird in jedem Spiel zur Missbrauchswaffe.

### Warum Kampfunfähigkeit statt Tod

F-159a ersetzt den Sofort-Tod durch einen niedergestreckten Zustand mit ablaufendem Timer. Das erzeugt den wertvollsten Moment im gesamten Koop-Design: Ein Mitspieler muss entscheiden, ob er mitten im Titanenfeuer landet, um jemanden aufzurichten. Wiederbelebung dauert bewusst lange genug, um selbst riskant zu sein. Alleinspieler erhalten eine begrenzte Selbstaufrichtung, damit Solo spielbar bleibt.

### Was das Netzwerkmodell erzwingt

Die Trennung aus F-178a ist die Grundlage jeder anderen Mehrspielerfunktion: **Die eigene Bewegung liegt beim Client** (mit serverseitiger Plausibilisierung), **alles andere liegt beim Server** — Titanen, Ziele, Schaden, Beute.

Anders geht es nicht. Bei den Latenzen, die in diesem Genre üblich sind, wäre serverautoritative Bewegung unspielbar; clientautoritative Beute wäre in der ersten Woche ausgenutzt.

Daraus folgt eine Arbeitsregel für das gesamte Projekt: **Jedes Bewegungsfeature wird bei 200 Millisekunden simulierter Latenz getestet, nicht nur lokal** (T-019). Ein Vector Gear, das nur auf dem Entwicklerrechner gut ist, ist wertlos.

### Sitzungsstruktur

| Ebene | Ort | Lebensdauer |
|---|---|---|
| Hub | Persistente Instanz, 40 Spieler | Dauerhaft |
| Gruppe | Reist geschlossen zwischen Places | Bis zur Auflösung |
| Einsatz | Reservierte Instanz, 10–20 Spieler | Bis Abschluss, dann Abbau |

Beitritt läuft über **Schnellsuche** (F-152, ein Klick) oder **Instanz-Browser** (F-153, manuelle Wahl). Beitritt während laufender Missionen ist erlaubt, bei Raids nach Phasenbeginn gesperrt. Verbindungsabbrüche reservieren den Platz 120 Sekunden lang (F-158a) — jemand mit instabilem Internet verliert nicht seinen halben Abend.

---

## 4. Die Gegner-Philosophie

Der wichtigste Befund aus der Analyse der Referenz: Von deren Gegnertypen fordert genau einer echtes Timing — der Typ, der bei Annäherung an den Nacken zurückrollt und danach unverwundbar ist. Alles andere ist bewegliches Futter.

**Unsere Regel: Mindestens die Hälfte aller Gegnertypen besitzt eine Anti-Autopilot-Eigenschaft.**

| Gegner | Autopilot-Bruch | Was der Spieler lernen muss |
|---|---|---|
| Husk | — | Grundlagen des Anflugwinkels |
| Errant | Unvorhersehbare Richtungswechsel | Führungsschüsse, Antizipation |
| Scuttler | Sehr hohe Geschwindigkeit, Sprungangriff | Vertikales Ausweichen |
| Weaver | Ausweichrolle mit I-Frames nach dem Startup | Timing statt Spam |
| Warden | Schützt den Cortex aktiv mit der Hand | Zweistufiger Angriff: erst Arme, dann Nacken |
| Lurker | Regloser Hinterhalt, Griff aus der Luft | Höhenwechsel, Aufmerksamkeit |
| Bellower | Reagiert auf Gas-Geräusch, ruft Verstärkung | Ressourcendisziplin, leises Spiel |
| Chorus | Paare decken sich gegenseitig | Zielpriorisierung, Trennung |

Bellower und Lurker fügen zusammen eine **Stealth-Ebene** hinzu, die es in der Referenz nicht gibt: Gas verbrauchen wird laut. Das koppelt die Ressource an das Risiko, statt sie zu einem reinen Timer zu machen.

---

## 5. Was wir gegenüber der Referenz besser machen

Zehn konkrete Änderungen, jede mit Begründung und Messgröße. Das ist der Wettbewerbsvorteil — nicht mehr Inhalt, sondern besser durchdachter Inhalt.

| # | Änderung | Begründung | Messgröße |
|---|---|---|---|
| **1** | **Onboarding in vier Stufen** | Die Referenz hat gar kein Tutorial. Spieler lernen die Steuerung im laufenden Gefecht. Das ist der billigste Retention-Gewinn im gesamten Projekt. | Abschlussquote der Erstmission über 80 % |
| **2** | **Pity auf alles, keine leeren Lineages** | Ein 80-Prozent-Wurf auf "gar nichts" beim Spielstart ist ein statistisch garantierter schlechter erster Eindruck — in einem Spiel, dessen Reiz auf Können basiert. | 100 % der Spieler mit aktiver Fähigkeit nach 3 h |
| **3** | **Upgrade-Budget statt Statleitern** | Acht unabhängige Statleitern erzeugen 120 Käufe ohne eine einzige Entscheidung. Ein gemeinsames Budget mit Zielkonflikten erzeugt echte Builds. | Meistgespielter Build unter 25 % Anteil |
| **4** | **Gerichteter Griff-Escape statt Tastenhämmern** | QTE-Mashing prüft Tastaturhardware und Fingerausdauer, nicht Können. Der Haken in Gegenrichtung ist eine echte Reaktions- und Orientierungsprüfung. | Kein messbarer Vorteil durch höhere Klickfrequenz |
| **5** | **Ascension statt Prestige-Reset** | Mechanische Fähigkeiten und Ausrüstungsränge bleiben erhalten; nur Build-Entscheidungen werden zurückgesetzt, bei größerem Budget. Der Spieler wird flexibler, nicht schwächer. | Ascension-Quote über 60 % der berechtigten Spieler |
| **6** | **Kompendium im Spiel** | Die Referenz zwingt Spieler auf externe Wikis und Trello-Boards für Grundwissen. Ein integriertes Nachschlagewerk ist billig und stark differenzierend. | Kein Wert existiert, der nur extern auffindbar ist |
| **7** | **Store ohne Progressionsverkauf** | Die Referenz verkauft Inventarplätze, Loadout-Speicher und Drop-Raten — also die Lösung selbst erzeugter Probleme. Wir verkaufen Aussehen. | Store-Audit: null statwirksame Artikel |
| **8** | **Traversal Trial als eigener Modus** | Das Beste am Genre ist die Bewegung, und kein Titel des Genres hat einen Modus, der nur daraus besteht. Nutzt bestehende Maps, kostet fast nichts, schafft einen zweiten Bindungsanker. | Wöchentlich aktive Nutzer des Modus über 25 % |
| **9** | **Anti-Autopilot-Gegner** | Siehe Kapitel 4. Ohne dies verkommt der Kampf zu Mausklicken auf Zielscheiben. | TTK-Varianz Anfänger zu Experte über 2,5-fach |
| **10** | **Missionsbogen auf 5–7 Minuten** | Die Referenz hat eine Durchschnittssession von rund 21 Minuten. Das sind 2–4 Missionen. Jede muss ein vollständiger Bogen mit garantiertem, spürbarem Fortschritt sein. | Ø Missionsdauer 5–7 min, Abbruchquote unter 8 % |

---

## 6. Produktionsplan

### 6.1 Die Reihenfolge-Regel

> **Kein Meta-System vor bestandenem Vector-Gear-Gate.**

Der Friedhof der Roblox-Anime-Titel besteht aus Spielen mit ausgefeilten Fähigkeitsbäumen und einer Bewegung, die sich falsch anfühlt. Fähigkeitsbaum, Wirtschaft, Lineages, Raids und Kosmetik werden erst begonnen, wenn Phase 1 abgenommen ist.

### 6.2 Phasen

| Phase | Dauer | Inhalt | Abnahmekriterium (Gate) |
|---|---|---|---|
| **P0 — Setup** | 3 Wochen | Rojo, Git, Service-Framework, Config-Schicht, Testumgebung, Blender-Exportvorgaben, Namenskonventionen | Ein Entwickler kann einen Feature-Branch bauen, testen und mergen, ohne Studio-Konflikte |
| **P1 — Vector Gear** | 6–8 Wochen | Nur Bewegung. Weiße Boxen, keine Assets, keine Gegner. Alle Anforderungen der Gruppe F-001 bis F-020 | **Blindtest gegen die Referenz mit 10 Testern. Unsere Bewegung muss mindestens gleichauf bewertet werden. Nicht bestanden = iterieren, nicht weitergehen.** |
| **P2 — Kampf-Kern** | 5 Wochen | Ein Titan mit vollständigem Angriffs- und Todeszyklus, Cortex-Trefferzone, Klingenhaltbarkeit, Nachschub, Hit-Stop | Eine Minute Kampf gegen einen einzelnen Titanen macht ohne jede Belohnung Spaß |
| **P3 — Erste Map** | 5 Wochen | Ashgate District als Graybox mit getunter Ankerdichte, dann Art-Pass | Traversal-Zeiten zeigen messbaren Unterschied zwischen Anfänger und Experte |
| **P4 — Missionsschleife** | 6 Wochen | Skirmish und Breach, Direktor-System, Auswertungsbildschirm, Belohnungsvergabe | Ein Spieler spielt freiwillig drei Missionen hintereinander |
| **P5 — Onboarding** | 4 Wochen | Alle vier Tutorialstufen, Trainingsgelände, adaptive Hinweise | 80 % Abschlussquote der Erstmission bei Erstkontakt-Testern |
| **P6 — Gegnervielfalt** | 5 Wochen | Alle acht Gegnertypen, Größenklassen, Gruppendynamik | Testspieler können alle Typen benennen und ihre Konterstrategie erklären |
| **P7 — Progression** | 7 Wochen | Level, Ausrüstungsbudget, Fähigkeitenbaum, Traits, Lineages mit Pity, Loadouts, Kompendium | Vier verschiedene Builds liegen bei der Effektivität innerhalb von 10 % |
| **P8 — Inhaltsausbau** | 10 Wochen | Maps 2–5, alle Missionsmodi, Modifikatoren, Sekundärziele, Traversal Trial | Jede Map hat eine erkennbar eigene Traversal-Identität |
| **P9 — Raids** | 8 Wochen | Raid-Framework, zwei Bosse, Matchmaking, Beutesystem, Umgebungswaffen | Eine Gruppe scheitert beim ersten Versuch und will es sofort wieder versuchen |
| **P10 — Vessel Forms** | 8 Wochen | Bonding, zwei Formen mit eigenem Moveset, Gegner-Anpassung | Die Form fühlt sich nicht wie ein vergrößerter Spieler an |
| **P11 — Politur & Live-Vorbereitung** | 8 Wochen | Barrierefreiheit, Anti-Cheat-Härtung, Latenztests, Lasttests, Telemetrie, Saisonstruktur | Lasttest mit 20 Spielern und vollem Titanenbudget auf allen Maps ohne Framedrop |

**Gesamt: rund 18 Monate** bei einem eingespielten Team. Die Referenz hat drei Jahre gebraucht — wir sparen Zeit, weil wir nicht suchen müssen, sondern gezielt bauen.

### 6.3 Teamzuschnitt

| Rolle | Anzahl | Hauptverantwortung |
|---|---|---|
| Tech Lead | 1 | Architektur, Framework, Review, Datenintegrität |
| Gameplay-Engineer | 2 | Vector Gear, Kampf, Gegner-KI |
| Systems-Engineer | 1 | Progression, Wirtschaft, Persistenz, Backend |
| Tools/Tech-Art | 1 | Pipeline, LOD, Ankerwerkzeug, Import-Automatisierung |
| Game Designer | 1 | Systeme, Balancing, Modi, Config-Pflege |
| Level Designer | 2 | Maps, Ankerdichte, Spawnlogik, Hub |
| 3D-Artist | 2 | Umgebung, Charaktere, Titanen |
| Animator | 1–2 | Rund 200 Clips — der am häufigsten unterschätzte Posten |
| Technical Sound / Komponist | 1 (extern) | Rund 120 SFX plus zwölf Musikschichten |
| UI/UX | 1 | 37 Bildschirme plus HUD |
| QA | 1 | Testpläne, Exploit-Tests, Playtest-Protokolle |
| Produktion | 1 | Backlog, Termine, Abnahmen |

### 6.4 Die größten Risiken

| Risiko | Gegenmaßnahme |
|---|---|
| **Vector-Gear-Feel wird nicht gut genug** | Harte Gate-Regel in P1. Lieber vier Wochen länger iterieren als 18 Monate auf einem schlechten Fundament bauen. |
| **Animationsaufwand unterschätzt** | Rund 200 Clips im Backlog erfasst. Animator ab P2 einbinden, nicht ab P6. |
| **Umfangsexplosion** | MoSCoW-Disziplin. Bei Terminkonflikt fallen zuerst alle "Could"-Einträge, dann Vessel Forms komplett. |
| **Physik-Exploits (Fling)** | Velocity-Clamp und serverseitige Plausibilisierung ab Tag 1, nicht nachträglich. |
| **Datenverlust und Duplikation** | ProfileStore mit Session-Lock und Transaktionsprotokoll ab P0. Nachrüsten ist praktisch unmöglich. |
| **Netzwerklast bei 20 Spielern** | Interpolationspuffer und Replikationsdrosselung ab P1 einplanen. Zwanzig Spieler mit je zwei Seilen und sechzig Titanen sind die eigentliche Belastungsprobe, nicht die Grafik. |
| **Mehrspieler-Bugs erst spät sichtbar** | Latenzsimulation als Standardwerkzeug (T-019). Jedes Bewegungsfeature wird bei 200 ms Latenz getestet, nicht nur lokal. |
| **Audio-Rechte** | Ausschließlich originale oder lizenzierte Musik. Anime-Soundtracks sind auf der Plattform nicht hochladbar und führen zur Löschung. |
| **Start ohne Spielerbasis** | Ab P4 geschlossene Testkohorten aufbauen, ab P8 Community-Server. Ein technisch perfekter Start ohne Publikum ist kein Start. |

---

## 7. Erfolgsmessung

Diese Kennzahlen entscheiden, ob das Projekt funktioniert. Sie werden ab P4 durchgängig erhoben.

| Kennzahl | Zielwert | Warum diese Zahl |
|---|---|---|
| Abschlussquote Erstmission | > 80 % | Direkter Test des Onboardings |
| D1-Retention | > 35 % | Branchenüblich gut für das Genre |
| D7-Retention | > 15 % | Zeigt, ob die Schleife trägt |
| Ø Sessiondauer | 20–30 min | Referenzwert liegt bei rund 21 min |
| Missionen pro Session | 3–4 | Bestätigt die Missionslänge |
| TTK-Varianz Anfänger zu Experte | > 2,5× | Beweist, dass Können zählt (P2) |
| Anteil meistgespielter Build | < 25 % | Beweist echte Build-Vielfalt (D-3) |
| Spieler mit aktiver Lineage nach 3 h | 100 % | Beweist, dass Pity greift (P3) |
| Traversal-Trial-Nutzung wöchentlich | > 25 % | Beweist, dass Bewegung eigenständig trägt (P1) |
| Absturzquote | < 0,5 % | Technische Grundhygiene |
| Ø FPS (Mindest-/Vollprofil) | 60 / 60 | Die Referenz liefert stabil rund 60 — das ist die Messlatte |
| Anteil Sitzungen mit Mitspielern | > 70 % | Beweist, dass die Mehrspieler-Schleife trägt |
| Wiederverbindungsquote nach Abbruch | > 60 % | Zeigt, ob F-158a wirklich greift |

---

## 8. Was als Nächstes zu klären ist

Diese Entscheidungen blockieren P0 und sollten in der ersten Woche fallen:

1. **PvP ja oder nein?** Das Spiel ist als reines Koop-Erlebnis spezifiziert. PvP würde eine vollständig getrennte Balancing-Linie, serverautoritative Trefferprüfung und dauerhafte Wartung bedeuten — das ist kein Feature, sondern ein zweites Projekt. Wenn PvP gewollt ist, muss das jetzt entschieden werden, nicht in Monat 12.
2. **Vessel Forms in v1.0 oder v1.5?** Sie sind der teuerste Einzelposten (eigene Rigs, rund 60 Animationen, eigenes Balancing) und ersetzen das Kern-Movement, statt es zu erweitern. Mein Vorschlag: als v1.5-Inhalt planen, technisch aber vorbereiten.
3. **Handel ja oder nein?** Handel bringt Bindung, aber auch Betrug, Schwarzmärkte und einen dauerhaften Supportaufwand. Bei P3 (kein Fortschritt ohne Garantie) sinkt der Nutzen von Handel deutlich.
4. **Musik: eigener Komponist oder Lizenzbibliothek?** Betrifft Budget und Zeitplan ab P4.
5. **Wer besitzt die Config-Hoheit?** Wenn Balancing-Werte im Code liegen, ist das Projekt nach sechs Monaten nicht mehr steuerbar. Die Antwort muss vor der ersten Codezeile stehen.
