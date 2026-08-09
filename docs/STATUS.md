<!-- GENERIERT von tools/features.py aus docs/features.ron — NICHT von Hand aendern.
     Handarbeit hier ist beim naechsten Lauf verloren. Arbeitsstand (Stufe, Beleg)
     gehoert nach docs/features.ron, dann `python3 tools/features.py`.
     Ohne 'Stand:'-Zeile mit Absicht: der Stand ist der von docs/features.ron, und
     ein Datum, das sich bei jedem Lauf aendert, ist Diff-Rauschen. -->

# STATUS — was implementiert ist und was nicht

Stufen: ⬜ nicht implementiert · 🟨 halb (gebaut, ungetestet, ungesehen) ·
🟧 fast (Tests, die umfallen + im Spiel gesehen) · ✅ fertig (**nur der User setzt das**).

**🟧 braucht drei Belege:** Bild (Screenshot-Pfad), Zahl (gemessen, mit Maschine `[debian]`/`[cachy]`) und Code (ein Test, der rot wird, wenn es kaputtgeht). Fehlt einer, ist es 🟨 — Unsicherheit setzt die Stufe herunter, nicht hinauf (prompts/init.md §8, §9).

**Stand:** 239 ⬜ · 6 🟨 · 0 🟧 · 0 ✅ von 245 Zeilen.

## combat

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Nape-Trefferzone (Cortex) | F-030 | ⬜ | — | — |
| Geschwindigkeitsabhaengige Schadensformel | F-031 | ⬜ | — | — |
| Sekundaere Trefferzonen | F-032 | ⬜ | — | — |
| Klingenhaltbarkeit | F-033 | ⬜ | — | — |
| Hit-Stop und Impact-Frames | F-034 | ⬜ | — | — |
| Kein Friendly Fire | F-037 | ⬜ | — | — |
| Gerichteter Griff-Escape | F-040 | ⬜ | — | — |
| Lance Charges (Fernwaffe) | F-035 | ⬜ | — | — |
| Lance-Munition und Nachschub | F-036 | ⬜ | — | — |
| Verletzungssystem | F-038 | ⬜ | — | — |
| Feldbehandlung und Medic | F-039 | ⬜ | — | — |
| Combo-System | F-041 | ⬜ | — | — |
| Schadenszahlen und Trefferfeedback | F-043 | ⬜ | — | — |
| Finisher-Kamera | F-042 | ⬜ | — | — |
| Nahkampf am Boden | F-044 | ⬜ | — | — |

## data

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| ProfileStore-Integration | T-040 | ⬜ | — | — |
| Schema-Migration | T-041 | ⬜ | — | — |
| Backup-Strategie | T-042 | ⬜ | — | — |
| Transaktionsprotokoll | T-043 | ⬜ | — | — |
| MemoryStore-Schicht | T-044 | ⬜ | — | — |

## hud

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| HUD-Grundlayout | F-170 | ⬜ | — | — |
| Dynamisches Fadenkreuz | F-171 | ⬜ | — | — |
| Vollstaendige Tastenbelegung | F-172 | ⬜ | — | — |
| Menuestruktur | F-175 | ⬜ | — | — |
| Grafikeinstellungen | F-177 | ⬜ | — | — |
| Barrierefreiheit | F-176 | ⬜ | — | — |
| Ladebildschirme mit Tipps | F-178 | ⬜ | — | — |

## mission

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Missions-Zustandsmaschine | F-070 | ⬜ | — | — |
| Modus: Skirmish | F-071 | ⬜ | — | — |
| Modus: Breach (Verteidigung) | F-072 | ⬜ | — | — |
| Modus: Escort | F-073 | ⬜ | — | — |
| Schwierigkeitsgrade | F-080 | ⬜ | — | — |
| Spielerzahl-Skalierung | F-082 | ⬜ | — | — |
| Stufe 1: Bewegungsparcours | F-185 | ⬜ | — | — |
| Stufe 2: Erste Kills | F-186 | ⬜ | — | — |
| Stufe 3: Gefuehrte Erstmission | F-187 | ⬜ | — | — |
| Stufe 4: Trainingsgelaende | F-188 | ⬜ | — | — |
| Modus: Stall (Ueberleben) | F-074 | ⬜ | — | — |
| Modus: Protect | F-075 | ⬜ | — | — |
| Modus: Reclaim (neu) | F-076 | ⬜ | — | — |
| Modus: Traversal Trial (neu) | F-077 | ⬜ | — | — |
| Randomizer | F-078 | ⬜ | — | — |
| Sekundaerziele | F-079 | ⬜ | — | — |
| Modifikatoren (Mutatoren) | F-081 | ⬜ | — | — |
| Post-Match-Auswertung | F-085 | ⬜ | — | — |
| Raid-Framework | F-090 | ⬜ | — | — |
| Boss: The Bound One | F-091 | ⬜ | — | — |
| Boss: The Dancer | F-092 | ⬜ | — | — |
| Boss: The Bulwark | F-093 | ⬜ | — | — |
| Boss: The Ashwalker | F-094 | ⬜ | — | — |
| Umgebungswaffen | F-095 | ⬜ | — | — |
| Beitragsschwelle (Credit) | F-096 | ⬜ | — | — |
| Raid-Matchmaking | F-097 | ⬜ | — | — |
| Beutetruhen mit Schluesselstufen | F-098 | ⬜ | — | — |
| Adaptive Hinweise | F-189 | ⬜ | — | — |
| Uebungsmodus | F-190 | ⬜ | — | — |
| Wochenziele | F-235 | ⬜ | — | — |
| Killstreak-System | F-083 | ⬜ | — | — |
| Extraktionsphase | F-084 | ⬜ | — | — |
| Aufstiegs-Modus (Ascension) | F-099 | ⬜ | — | — |
| Saisonstruktur | F-236 | ⬜ | — | — |
| Event-Framework | F-237 | ⬜ | — | — |
| Codes-System | F-238 | ⬜ | — | — |
| Serverseitige Nachrichten | F-239 | ⬜ | — | — |

## net

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Latenzsimulation im Test | T-019 | 🟨 | scripts/t019-latenz.txt bei --lag 200: 3 assert gehalten, Exit 0 · tests/mehrspieler.rs::t019_... (200 ms = 12 Ticks) | 2026-08-09 [debian] — Schalter da und geprueft, noch kein Bewegungsfeature dahinter |
| Server-Autoritaet fuer Werte | F-215 | ⬜ | — | — |
| Positionsplausibilisierung | F-216 | ⬜ | — | — |
| Rate-Limiting | F-217 | ⬜ | — | — |
| Schadensvalidierung | F-218 | ⬜ | — | — |
| Remote-Bundling | T-010 | ⬜ | — | — |
| Client-Prediction | T-011 | ⬜ | — | — |
| Replikationsdrosselung | T-012 | ⬜ | — | — |
| Instanzverwaltung | T-014 | ⬜ | — | — |
| Matchmaking-Dienst | T-015 | ⬜ | — | — |
| Gruppen-Teleport | T-016 | ⬜ | — | — |
| Wiederverbindungslogik | T-017 | ⬜ | — | — |
| Interpolationspuffer | T-018 | ⬜ | — | — |
| Anomalieerkennung | F-219 | ⬜ | — | — |
| Latenzkompensation | T-013 | ⬜ | — | — |
| Meldesystem | F-220 | ⬜ | — | — |

## player

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Bonding-System | F-105 | ⬜ | — | — |
| Zeitlimit und Eject | F-106 | ⬜ | — | — |
| Eigenstaendiges Moveset | F-107 | ⬜ | — | — |
| Form: Warbound (Angriff) | F-108 | ⬜ | — | — |
| Gegner-Anpassung | F-113 | ⬜ | — | — |
| Form: Bladewright (Technik) | F-109 | ⬜ | — | — |
| Form: Bulwark (Panzer) | F-110 | ⬜ | — | — |
| Form: Colossus (Belagerung) | F-111 | ⬜ | — | — |
| Form-Progression | F-112 | ⬜ | — | — |

## progress

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Level und Erfahrungskurve | F-120 | ⬜ | — | — |
| Ausruestungsrang | F-121 | ⬜ | — | — |
| Ausruestungs-Upgrades mit Budget | F-122 | ⬜ | — | — |
| Faehigkeitenbaum | F-123 | ⬜ | — | — |
| Kostenloser Respec | F-124 | ⬜ | — | — |
| Loadout-Speicher | F-125 | ⬜ | — | — |
| Perk-System | F-126 | ⬜ | — | — |
| Pity-System fuer Lineages | F-128 | ⬜ | — | — |
| Zwei Waehrungen | F-140 | ⬜ | — | — |
| Serverseitige Beutetabellen | F-141 | ⬜ | — | — |
| Pity-Zaehler fuer alle Seltenheiten | F-142 | ⬜ | — | — |
| Automatischer Verkauf und Filter | F-145 | ⬜ | — | — |
| Lineage-System (Familien) | F-127 | ⬜ | — | — |
| Deterministischer Ichor-Pfad | F-129 | ⬜ | — | — |
| Ascension (Prestige-Ersatz) | F-131 | ⬜ | — | — |
| Erfolge und Meilensteine | F-133 | ⬜ | — | — |
| Kompendium | F-134 | ⬜ | — | — |
| Build-Simulator im Hub | F-135 | ⬜ | — | — |
| Transparente Drop-Raten | F-143 | ⬜ | — | — |
| Glueckswert (Luck) | F-144 | ⬜ | — | — |
| Waehrungssenken | F-146 | ⬜ | — | — |
| Wochenkontingente statt AFK | F-148 | ⬜ | — | — |
| Kosmetik-System | F-225 | ⬜ | — | — |
| Saisonpass | F-226 | ⬜ | — | — |
| Verzicht auf Progressionsverkauf | F-228 | ⬜ | — | — |
| Transparenter Shop | F-229 | ⬜ | — | — |
| Artefakte mit Substats | F-130 | ⬜ | — | — |
| Memories (Ascension-Talente) | F-132 | ⬜ | — | — |
| Handelssystem | F-147 | ⬜ | — | — |
| Privatserver | F-227 | ⬜ | — | — |

## render

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Streaming-Konfiguration | T-020 | ⬜ | — | — |
| Object Pooling | T-021 | ⬜ | — | — |
| LOD-Pipeline | T-022 | ⬜ | — | — |
| Materialkonsolidierung | T-023 | ⬜ | — | — |
| Profiling-Werkzeuge | T-024 | ⬜ | — | — |
| Raeumliches Gitter fuer Ankerpunkte | T-036a | ⬜ | — | — |
| PC-Qualitaetsprofile | T-025 | ⬜ | — | — |

## save

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Datenpersistenz mit Session-Lock | F-200 | ⬜ | — | — |
| Versionierte Datenschemata | F-201 | ⬜ | — | — |
| Transaktionsprotokoll | F-202 | ⬜ | — | — |
| Place-Routing | F-204 | ⬜ | — | — |
| Config-getriebenes Balancing | F-207 | ⬜ | — | — |
| Cross-Server-Zustand | F-203 | ⬜ | — | — |
| Telemetrie | F-205 | ⬜ | — | — |
| Feature-Flags | F-206 | ⬜ | — | — |

## squad

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Sitzungsmodell | F-150 | ⬜ | — | — |
| Serverkapazitaet | F-151 | ⬜ | — | — |
| Schnellsuche (Quick Play) | F-152 | ⬜ | — | — |
| Gruppensystem | F-155 | ⬜ | — | — |
| Hub: The Rookery | F-156 | ⬜ | — | — |
| Gruppenleiter-Rechte | F-156a | ⬜ | — | — |
| Wiederverbindung | F-158a | ⬜ | — | — |
| Kampfunfaehigkeit und Wiederbelebung | F-159a | ⬜ | — | — |
| Getrennte Beute pro Spieler | F-160a | ⬜ | — | — |
| Beitragswertung | F-161a | ⬜ | — | — |
| Kein Schaden unter Spielern | F-162a | ⬜ | — | — |
| Kollisionsmodell zwischen Spielern | F-163a | ⬜ | — | — |
| Namensschilder und Zustandsanzeige | F-164a | ⬜ | — | — |
| Markierungssystem | F-165a | ⬜ | — | — |
| Gemeinsame Zielanzeige | F-167a | ⬜ | — | — |
| Skalierung nach Spielerzahl | F-168a | ⬜ | — | — |
| Grief-Schutz | F-176a | ⬜ | — | — |
| Netzwerk-Autoritaetsmodell | F-178a | ⬜ | — | — |
| Interpolation fremder Spieler | F-179a | ⬜ | — | — |
| Instanz-Browser | F-153 | ⬜ | — | — |
| Beitritt waehrend des Einsatzes | F-154 | ⬜ | — | — |
| Bereitschaftspruefung | F-157a | ⬜ | — | — |
| Bestenlisten | F-160 | ⬜ | — | — |
| Unterstuetzungsauren | F-166a | ⬜ | — | — |
| Abstimmungen | F-169a | ⬜ | — | — |
| Ausschluss und Meldung | F-170a | ⬜ | — | — |
| Anwesenheit und Freundesliste | F-171a | ⬜ | — | — |
| Privatinstanzen | F-173a | ⬜ | — | — |
| Sitzungspersistenz | F-174a | ⬜ | — | — |
| Latenz- und Verbindungsanzeige | F-175a | ⬜ | — | — |
| Gemeinsame Auswertung | F-177a | ⬜ | — | — |
| Regiments (Gilden) | F-158 | ⬜ | — | — |
| Zuschauermodus | F-172a | ⬜ | — | — |

## titan

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Zustandsmaschine | F-050 | ⬜ | — | — |
| Pathfinding mit Groessenlogik | F-052 | ⬜ | — | — |
| Telegraphierte Angriffe | F-053 | ⬜ | — | — |
| KI-Level-of-Detail | F-054 | ⬜ | — | — |
| Gegnertyp: Husk (Standard) | F-056 | ⬜ | — | — |
| Gegnertyp: Errant (Abnormal) | F-057 | ⬜ | — | — |
| Gegnertyp: Scuttler (Crawler) | F-058 | ⬜ | — | — |
| Gegnertyp: Weaver (Ducker) | F-059 | ⬜ | — | — |
| Groessenklassen | F-064 | ⬜ | — | — |
| Spawn-Budget-System | F-065 | ⬜ | — | — |
| Object Pooling | F-066 | ⬜ | — | — |
| Wahrnehmungsmodell | F-051 | ⬜ | — | — |
| Gruppendynamik | F-055 | ⬜ | — | — |
| Gegnertyp: Warden | F-060 | ⬜ | — | — |
| Gegnertyp: Lurker | F-061 | ⬜ | — | — |
| Gegnertyp: Bellower | F-062 | ⬜ | — | — |
| Gegnertyp: Chorus (Paar) | F-063 | ⬜ | — | — |

## tooling

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Rojo- und Git-Aufsetzung | T-001 | 🟨 | cargo build gruen · git-Repo steht · 62 Tests gruen [debian] | 2026-08-09 [debian] — cargo statt Rojo (docs/architektur.md), Pixel ungesehen |
| Service-Framework | T-003 | 🟨 | tests/domaenen.rs (3 Faelle) · 18 Domaenen-Plugins in Abhaengigkeitsreihenfolge | 2026-08-09 [debian] — Service-Framework = Domaenen-Plugins, Erlaubnisliste leer |
| Konfigurationsschicht | T-005 | 🟨 | tests/data.rs (10 Faelle) · 6 RON-Dateien laden, kein serde(default) | 2026-08-09 [debian] — Zahlen sind UNGETUNT, Pixel ungesehen |
| Testumgebung | T-006 | 🟨 | cargo test: 62 gruen, 0 rot · --headless laeuft ohne Grafiksitzung | 2026-08-09 [debian] — Pixel ungesehen |
| Automatisierte Smoke-Tests | T-007 | 🟨 | scripts/t007-erste-fahrt.txt: 6 assert gehalten, 180 Ticks, Exit 0 · rot geprueft: falscher assert -> Exit 1 | 2026-08-09 [debian] — Pixel ungesehen |
| Branch- und Review-Prozess | T-002 | ⬜ | — | — |
| Luau strict mode | T-004 | ⬜ | — | entfaellt: Luau gibt es hier nicht. Entsprechung ist der Rust-Compiler plus tools/normen.py |
| Blender-Exportvorgabe | T-030 | ⬜ | — | — |
| Asset-Namenskonvention | T-031 | ⬜ | — | — |
| Ankerflaechen-Werkzeug | T-033 | ⬜ | — | — |
| Animations-Exportvorgabe | T-034 | ⬜ | — | — |
| Ankerpunkt-Generator | T-034a | ⬜ | — | — |
| Audio-Bibliothek | T-035 | ⬜ | — | — |
| Ankerpunkt-Validierer | T-035a | ⬜ | — | — |
| Debug-Konsole | T-052 | ⬜ | — | — |
| Fehlerueberwachung | T-060 | ⬜ | — | — |
| Lasttest | T-064 | ⬜ | — | — |
| Testplan Bewegung | T-070 | ⬜ | — | — |
| Testplan Kampf | T-071 | ⬜ | — | — |
| Testplan Progression | T-072 | ⬜ | — | — |
| Exploit-Testplan | T-073 | ⬜ | — | — |
| Playtest-Protokoll | T-074 | ⬜ | — | — |
| Asset-Import-Automatisierung | T-032 | ⬜ | — | — |
| Balancing-Dashboard | T-050 | ⬜ | — | — |
| Level-Design-Werkzeuge | T-051 | ⬜ | — | — |
| Telemetrie-Pipeline | T-061 | ⬜ | — | — |
| Feature-Flag-Dienst | T-062 | ⬜ | — | — |
| Wartungsmodus | T-063 | ⬜ | — | — |
| Replay-System | T-053 | ⬜ | — | — |

## vector

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Doppelhaken-Grundsystem | F-001 | ⬜ | — | — |
| Freies Zielen per Raycast (Ebene 1) | F-002 | ⬜ | — | — |
| Getaggte Ankerflaechen (Ebene 1) | F-003 | ⬜ | — | — |
| Pendelphysik bei Zwei-Haken-Zustand | F-004 | ⬜ | — | — |
| Reel-In / Seilverkuerzung | F-005 | ⬜ | — | — |
| Swerve-Steuerung | F-006 | ⬜ | — | — |
| Gas-Boost | F-007 | ⬜ | — | — |
| Boost-Dash | F-008 | ⬜ | — | — |
| Flips (seitlich) | F-009 | ⬜ | — | — |
| Slide-Dodge am Boden | F-010 | ⬜ | — | — |
| Velocity-Clamp gegen Fling | F-012 | ⬜ | — | — |
| Kollisionsdaempfung | F-013 | ⬜ | — | — |
| Momentum-Chaining | F-014 | ⬜ | — | — |
| Geschwindigkeits-Feedback | F-017 | ⬜ | — | — |
| Gas-Ressource | F-018 | ⬜ | — | — |
| Nachschub-Stationen | F-019 | ⬜ | — | — |
| Hook-Break (Notbremse) | F-011 | ⬜ | — | — |
| Ziel-Assist-Regler | F-016 | ⬜ | — | — |
| Wall-Run und Wall-Kick | F-015 | ⬜ | — | — |
| Tragbarer Notvorrat | F-020 | ⬜ | — | — |

## world

| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |
|---|---|---|---|---|
| Diskrete Ankerpunkte (Ebene 2) | F-021 | ⬜ | — | — |
| Prozedurale Ankerpunkt-Erzeugung | F-022 | ⬜ | — | — |
| Kandidatensuche mit Hemisphaeren-Aufteilung | F-023 | ⬜ | — | — |
| Snap auf Q und E | F-024 | ⬜ | — | — |
| Bewertungsfunktion fuer Kandidaten | F-025 | ⬜ | — | — |
| Highlighting der Ankerpunkte | F-026 | ⬜ | — | — |
| Marker-Dichtebegrenzung | F-027 | ⬜ | — | — |
| Fallback ohne Kandidat | F-028 | ⬜ | — | — |
| Performance der Kandidatensuche | F-030a | ⬜ | — | — |
| Ankerpunkt-Validierung im Editor | F-031a | ⬜ | — | — |
| Dynamische Ankerpunkte | F-029 | ⬜ | — | — |
| Trainingsanzeige fuer Ankerpunkte | F-032a | ⬜ | — | — |

