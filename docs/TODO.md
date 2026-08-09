<!-- GENERIERT von tools/features.py aus docs/features.ron — NICHT von Hand aendern.
     Handarbeit hier ist beim naechsten Lauf verloren. Arbeitsstand (Stufe, Beleg)
     gehoert nach docs/features.ron, dann `python3 tools/features.py`.
     Ohne 'Stand:'-Zeile mit Absicht: der Stand ist der von docs/features.ron, und
     ein Datum, das sich bei jedem Lauf aendert, ist Diff-Rauschen. -->

# TODO — offene Arbeit, in baubarer Reihenfolge

Sortiert nach Domaene; innerhalb der Domaene so, dass `abhaengt_von` erfuellt
ist, bevor eine Zeile drankommt. Prio 1 = Must, 2 = Should, 3 = Could —
`Must` vor `Should` vor `Could` ist die Reihenfolge, keine Empfehlung
(prompts/init.md §2).

## combat (15 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | F-030 | Nape-Trefferzone (Cortex) | 1 | — | Must, ohne Vorbedingung |
| ⬜ | F-044 | Nahkampf am Boden | 3 | — | Could |
| ⬜ | F-031 | Geschwindigkeitsabhaengige Schadensformel | 1 | F-030 | braucht F-030 |
| ⬜ | F-032 | Sekundaere Trefferzonen | 1 | F-030 | braucht F-030 |
| ⬜ | F-033 | Klingenhaltbarkeit | 1 | F-030 | braucht F-030 |
| ⬜ | F-034 | Hit-Stop und Impact-Frames | 1 | F-030 | braucht F-030 |
| ⬜ | F-040 | Gerichteter Griff-Escape | 1 | F-030 | braucht F-030 |
| ⬜ | F-035 | Lance Charges (Fernwaffe) | 2 | F-030 | braucht F-030 |
| ⬜ | F-037 | Kein Friendly Fire | 1 | F-035 | braucht F-035 |
| ⬜ | F-036 | Lance-Munition und Nachschub | 2 | F-035 | braucht F-035 |
| ⬜ | F-038 | Verletzungssystem | 2 | F-032 | braucht F-032 |
| ⬜ | F-041 | Combo-System | 2 | F-031 | braucht F-031 |
| ⬜ | F-043 | Schadenszahlen und Trefferfeedback | 2 | F-031 | braucht F-031 |
| ⬜ | F-042 | Finisher-Kamera | 3 | F-034 | braucht F-034 |
| ⬜ | F-039 | Feldbehandlung und Medic | 2 | F-038 | braucht F-038 |

## data (5 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | T-040 | ProfileStore-Integration | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-041 | Schema-Migration | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-042 | Backup-Strategie | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-043 | Transaktionsprotokoll | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-044 | MemoryStore-Schicht | 2 | — | Should |

## hud (7 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | F-170 | HUD-Grundlayout | 1 | — | Must, ohne Vorbedingung |
| ⬜ | F-172 | Vollstaendige Tastenbelegung | 1 | F-170 | braucht F-170 |
| ⬜ | F-175 | Menuestruktur | 1 | F-170 | braucht F-170 |
| ⬜ | F-177 | Grafikeinstellungen | 1 | F-170 | braucht F-170 |
| ⬜ | F-176 | Barrierefreiheit | 2 | F-170 | braucht F-170 |
| ⬜ | F-171 | Dynamisches Fadenkreuz | 1 | F-002 | braucht F-002 |
| ⬜ | F-178 | Ladebildschirme mit Tipps | 3 | F-175 | braucht F-175 |

## mission (37 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | F-070 | Missions-Zustandsmaschine | 1 | — | Must, ohne Vorbedingung |
| ⬜ | F-071 | Modus: Skirmish | 1 | F-070 | braucht F-070 |
| ⬜ | F-072 | Modus: Breach (Verteidigung) | 1 | F-070 | braucht F-070 |
| ⬜ | F-073 | Modus: Escort | 1 | F-070 | braucht F-070 |
| ⬜ | F-080 | Schwierigkeitsgrade | 1 | F-070 | braucht F-070 |
| ⬜ | F-074 | Modus: Stall (Ueberleben) | 2 | F-070 | braucht F-070 |
| ⬜ | F-075 | Modus: Protect | 2 | F-070 | braucht F-070 |
| ⬜ | F-076 | Modus: Reclaim (neu) | 2 | F-070 | braucht F-070 |
| ⬜ | F-078 | Randomizer | 2 | F-070 | braucht F-070 |
| ⬜ | F-079 | Sekundaerziele | 2 | F-070 | braucht F-070 |
| ⬜ | F-085 | Post-Match-Auswertung | 2 | F-070 | braucht F-070 |
| ⬜ | F-090 | Raid-Framework | 2 | F-070 | braucht F-070 |
| ⬜ | F-083 | Killstreak-System | 3 | F-070 | braucht F-070 |
| ⬜ | F-084 | Extraktionsphase | 3 | F-070 | braucht F-070 |
| ⬜ | F-238 | Codes-System | 3 | F-200 | braucht F-200 |
| ⬜ | F-082 | Spielerzahl-Skalierung | 1 | F-080 | braucht F-080 |
| ⬜ | F-188 | Stufe 4: Trainingsgelaende | 1 | F-156 | braucht F-156 |
| ⬜ | F-081 | Modifikatoren (Mutatoren) | 2 | F-080 | braucht F-080 |
| ⬜ | F-091 | Boss: The Bound One | 2 | F-090 | braucht F-090 |
| ⬜ | F-092 | Boss: The Dancer | 2 | F-090 | braucht F-090 |
| ⬜ | F-093 | Boss: The Bulwark | 2 | F-035 | braucht F-035 |
| ⬜ | F-094 | Boss: The Ashwalker | 2 | F-090 | braucht F-090 |
| ⬜ | F-096 | Beitragsschwelle (Credit) | 2 | F-090 | braucht F-090 |
| ⬜ | F-097 | Raid-Matchmaking | 2 | F-090 | braucht F-090 |
| ⬜ | F-235 | Wochenziele | 2 | F-148 | braucht F-148 |
| ⬜ | F-099 | Aufstiegs-Modus (Ascension) | 3 | F-090 | braucht F-090 |
| ⬜ | F-236 | Saisonstruktur | 3 | F-226 | braucht F-226 |
| ⬜ | F-237 | Event-Framework | 3 | F-206 | braucht F-206 |
| ⬜ | F-239 | Serverseitige Nachrichten | 3 | F-206 | braucht F-206 |
| ⬜ | F-185 | Stufe 1: Bewegungsparcours | 1 | F-006 | braucht F-006 |
| ⬜ | F-077 | Modus: Traversal Trial (neu) | 2 | F-014 | braucht F-014 |
| ⬜ | F-095 | Umgebungswaffen | 2 | F-094 | braucht F-094 |
| ⬜ | F-098 | Beutetruhen mit Schluesselstufen | 2 | F-096 | braucht F-096 |
| ⬜ | F-189 | Adaptive Hinweise | 2 | F-188 | braucht F-188 |
| ⬜ | F-190 | Uebungsmodus | 2 | F-188 | braucht F-188 |
| ⬜ | F-186 | Stufe 2: Erste Kills | 1 | F-185 | braucht F-185 |
| ⬜ | F-187 | Stufe 3: Gefuehrte Erstmission | 1 | F-186 | braucht F-186 |

## net (16 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | T-010 | Remote-Bundling | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-011 | Client-Prediction | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-012 | Replikationsdrosselung | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-014 | Instanzverwaltung | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-015 | Matchmaking-Dienst | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-016 | Gruppen-Teleport | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-017 | Wiederverbindungslogik | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-018 | Interpolationspuffer | 1 | — | Must, ohne Vorbedingung |
| 🟨 | T-019 | Latenzsimulation im Test | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-013 | Latenzkompensation | 2 | — | Should |
| ⬜ | F-215 | Server-Autoritaet fuer Werte | 1 | F-200 | braucht F-200 |
| ⬜ | F-217 | Rate-Limiting | 1 | F-215 | braucht F-215 |
| ⬜ | F-218 | Schadensvalidierung | 1 | F-215 | braucht F-215 |
| ⬜ | F-219 | Anomalieerkennung | 2 | F-205 | braucht F-205 |
| ⬜ | F-216 | Positionsplausibilisierung | 1 | F-012 | braucht F-012 |
| ⬜ | F-220 | Meldesystem | 3 | F-219 | braucht F-219 |

## player (9 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | F-105 | Bonding-System | 2 | F-030 | braucht F-030 |
| ⬜ | F-106 | Zeitlimit und Eject | 2 | F-105 | braucht F-105 |
| ⬜ | F-107 | Eigenstaendiges Moveset | 2 | F-105 | braucht F-105 |
| ⬜ | F-108 | Form: Warbound (Angriff) | 2 | F-107 | braucht F-107 |
| ⬜ | F-113 | Gegner-Anpassung | 2 | F-107 | braucht F-107 |
| ⬜ | F-109 | Form: Bladewright (Technik) | 3 | F-107 | braucht F-107 |
| ⬜ | F-110 | Form: Bulwark (Panzer) | 3 | F-107 | braucht F-107 |
| ⬜ | F-111 | Form: Colossus (Belagerung) | 3 | F-107 | braucht F-107 |
| ⬜ | F-112 | Form-Progression | 3 | F-107 | braucht F-107 |

## progress (30 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | F-120 | Level und Erfahrungskurve | 1 | — | Must, ohne Vorbedingung |
| ⬜ | F-140 | Zwei Waehrungen | 1 | — | Must, ohne Vorbedingung |
| ⬜ | F-133 | Erfolge und Meilensteine | 2 | — | Should |
| ⬜ | F-225 | Kosmetik-System | 2 | — | Should |
| ⬜ | F-228 | Verzicht auf Progressionsverkauf | 2 | — | Should |
| ⬜ | F-122 | Ausruestungs-Upgrades mit Budget | 1 | F-120 | braucht F-120 |
| ⬜ | F-123 | Faehigkeitenbaum | 1 | F-120 | braucht F-120 |
| ⬜ | F-141 | Serverseitige Beutetabellen | 1 | F-140 | braucht F-140 |
| ⬜ | F-145 | Automatischer Verkauf und Filter | 1 | F-140 | braucht F-140 |
| ⬜ | F-146 | Waehrungssenken | 2 | F-140 | braucht F-140 |
| ⬜ | F-148 | Wochenkontingente statt AFK | 2 | F-140 | braucht F-140 |
| ⬜ | F-226 | Saisonpass | 2 | F-225 | braucht F-225 |
| ⬜ | F-229 | Transparenter Shop | 2 | F-225 | braucht F-225 |
| ⬜ | F-121 | Ausruestungsrang | 1 | F-122 | braucht F-122 |
| ⬜ | F-124 | Kostenloser Respec | 1 | F-123 | braucht F-123 |
| ⬜ | F-125 | Loadout-Speicher | 1 | F-123 | braucht F-123 |
| ⬜ | F-126 | Perk-System | 1 | F-123 | braucht F-123 |
| ⬜ | F-142 | Pity-Zaehler fuer alle Seltenheiten | 1 | F-141 | braucht F-141 |
| ⬜ | F-129 | Deterministischer Ichor-Pfad | 2 | F-105 | braucht F-105 |
| ⬜ | F-131 | Ascension (Prestige-Ersatz) | 2 | F-122 | braucht F-122 |
| ⬜ | F-147 | Handelssystem | 3 | F-141 | braucht F-141 |
| ⬜ | F-127 | Lineage-System (Familien) | 2 | F-126 | braucht F-126 |
| ⬜ | F-134 | Kompendium | 2 | F-126 | braucht F-126 |
| ⬜ | F-130 | Artefakte mit Substats | 3 | F-126 | braucht F-126 |
| ⬜ | F-132 | Memories (Ascension-Talente) | 3 | F-131 | braucht F-131 |
| ⬜ | F-128 | Pity-System fuer Lineages | 1 | F-127 | braucht F-127 |
| ⬜ | F-135 | Build-Simulator im Hub | 2 | F-134 | braucht F-134 |
| ⬜ | F-143 | Transparente Drop-Raten | 2 | F-134 | braucht F-134 |
| ⬜ | F-227 | Privatserver | 3 | F-204 | braucht F-204 |
| ⬜ | F-144 | Glueckswert (Luck) | 2 | F-143 | braucht F-143 |

## render (7 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | T-020 | Streaming-Konfiguration | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-021 | Object Pooling | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-022 | LOD-Pipeline | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-023 | Materialkonsolidierung | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-024 | Profiling-Werkzeuge | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-036a | Raeumliches Gitter fuer Ankerpunkte | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-025 | PC-Qualitaetsprofile | 2 | — | Should |

## save (8 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | F-200 | Datenpersistenz mit Session-Lock | 1 | — | Must, ohne Vorbedingung |
| ⬜ | F-207 | Config-getriebenes Balancing | 1 | — | Must, ohne Vorbedingung |
| ⬜ | F-201 | Versionierte Datenschemata | 1 | F-200 | braucht F-200 |
| ⬜ | F-202 | Transaktionsprotokoll | 1 | F-200 | braucht F-200 |
| ⬜ | F-205 | Telemetrie | 2 | F-200 | braucht F-200 |
| ⬜ | F-206 | Feature-Flags | 2 | F-200 | braucht F-200 |
| ⬜ | F-204 | Place-Routing | 1 | F-155 | braucht F-155 |
| ⬜ | F-203 | Cross-Server-Zustand | 2 | F-097 | braucht F-097 |

## squad (33 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | F-150 | Sitzungsmodell | 1 | F-070 | braucht F-070 |
| ⬜ | F-156 | Hub: The Rookery | 1 | F-070 | braucht F-070 |
| ⬜ | F-164a | Namensschilder und Zustandsanzeige | 1 | F-170 | braucht F-170 |
| ⬜ | F-167a | Gemeinsame Zielanzeige | 1 | F-070 | braucht F-070 |
| ⬜ | F-174a | Sitzungspersistenz | 2 | F-200 | braucht F-200 |
| ⬜ | F-151 | Serverkapazitaet | 1 | F-150 | braucht F-150 |
| ⬜ | F-152 | Schnellsuche (Quick Play) | 1 | F-150 | braucht F-150 |
| ⬜ | F-155 | Gruppensystem | 1 | F-150 | braucht F-150 |
| ⬜ | F-158a | Wiederverbindung | 1 | F-150 | braucht F-150 |
| ⬜ | F-160a | Getrennte Beute pro Spieler | 1 | F-141 | braucht F-141 |
| ⬜ | F-163a | Kollisionsmodell zwischen Spielern | 1 | F-004 | braucht F-004 |
| ⬜ | F-178a | Netzwerk-Autoritaetsmodell | 1 | F-215 | braucht F-215 |
| ⬜ | F-154 | Beitritt waehrend des Einsatzes | 2 | F-150 | braucht F-150 |
| ⬜ | F-169a | Abstimmungen | 2 | F-150 | braucht F-150 |
| ⬜ | F-173a | Privatinstanzen | 2 | F-150 | braucht F-150 |
| ⬜ | F-175a | Latenz- und Verbindungsanzeige | 2 | F-164a | braucht F-164a |
| ⬜ | F-177a | Gemeinsame Auswertung | 2 | F-085 | braucht F-085 |
| ⬜ | F-172a | Zuschauermodus | 3 | F-090 | braucht F-090 |
| ⬜ | F-156a | Gruppenleiter-Rechte | 1 | F-155 | braucht F-155 |
| ⬜ | F-159a | Kampfunfaehigkeit und Wiederbelebung | 1 | F-038 | braucht F-038 |
| ⬜ | F-161a | Beitragswertung | 1 | F-096 | braucht F-096 |
| ⬜ | F-162a | Kein Schaden unter Spielern | 1 | F-037 | braucht F-037 |
| ⬜ | F-165a | Markierungssystem | 1 | F-155 | braucht F-155 |
| ⬜ | F-168a | Skalierung nach Spielerzahl | 1 | F-082 | braucht F-082 |
| ⬜ | F-179a | Interpolation fremder Spieler | 1 | F-178a | braucht F-178a |
| ⬜ | F-153 | Instanz-Browser | 2 | F-152 | braucht F-152 |
| ⬜ | F-157a | Bereitschaftspruefung | 2 | F-155 | braucht F-155 |
| ⬜ | F-166a | Unterstuetzungsauren | 2 | F-126 | braucht F-126 |
| ⬜ | F-170a | Ausschluss und Meldung | 2 | F-155 | braucht F-155 |
| ⬜ | F-171a | Anwesenheit und Freundesliste | 2 | F-155 | braucht F-155 |
| ⬜ | F-158 | Regiments (Gilden) | 3 | F-155 | braucht F-155 |
| ⬜ | F-176a | Grief-Schutz | 1 | F-162a | braucht F-162a |
| ⬜ | F-160 | Bestenlisten | 2 | F-077 | braucht F-077 |

## titan (17 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | F-050 | Zustandsmaschine | 1 | — | Must, ohne Vorbedingung |
| ⬜ | F-052 | Pathfinding mit Groessenlogik | 1 | F-050 | braucht F-050 |
| ⬜ | F-053 | Telegraphierte Angriffe | 1 | F-050 | braucht F-050 |
| ⬜ | F-054 | KI-Level-of-Detail | 1 | F-050 | braucht F-050 |
| ⬜ | F-056 | Gegnertyp: Husk (Standard) | 1 | F-050 | braucht F-050 |
| ⬜ | F-058 | Gegnertyp: Scuttler (Crawler) | 1 | F-050 | braucht F-050 |
| ⬜ | F-059 | Gegnertyp: Weaver (Ducker) | 1 | F-050 | braucht F-050 |
| ⬜ | F-064 | Groessenklassen | 1 | F-050 | braucht F-050 |
| ⬜ | F-051 | Wahrnehmungsmodell | 2 | F-050 | braucht F-050 |
| ⬜ | F-057 | Gegnertyp: Errant (Abnormal) | 1 | F-056 | braucht F-056 |
| ⬜ | F-065 | Spawn-Budget-System | 1 | F-054 | braucht F-054 |
| ⬜ | F-055 | Gruppendynamik | 2 | F-052 | braucht F-052 |
| ⬜ | F-060 | Gegnertyp: Warden | 2 | F-032 | braucht F-032 |
| ⬜ | F-061 | Gegnertyp: Lurker | 2 | F-051 | braucht F-051 |
| ⬜ | F-062 | Gegnertyp: Bellower | 2 | F-051 | braucht F-051 |
| ⬜ | F-066 | Object Pooling | 1 | F-065 | braucht F-065 |
| ⬜ | F-063 | Gegnertyp: Chorus (Paar) | 3 | F-055 | braucht F-055 |

## tooling (29 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| 🟨 | T-001 | Rojo- und Git-Aufsetzung | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-002 | Branch- und Review-Prozess | 1 | — | Must, ohne Vorbedingung |
| 🟨 | T-003 | Service-Framework | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-004 | Luau strict mode | 1 | — | Must, ohne Vorbedingung |
| 🟨 | T-005 | Konfigurationsschicht | 1 | — | Must, ohne Vorbedingung |
| 🟨 | T-006 | Testumgebung | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-030 | Blender-Exportvorgabe | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-031 | Asset-Namenskonvention | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-033 | Ankerflaechen-Werkzeug | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-034 | Animations-Exportvorgabe | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-034a | Ankerpunkt-Generator | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-035 | Audio-Bibliothek | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-035a | Ankerpunkt-Validierer | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-052 | Debug-Konsole | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-060 | Fehlerueberwachung | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-064 | Lasttest | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-070 | Testplan Bewegung | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-071 | Testplan Kampf | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-072 | Testplan Progression | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-073 | Exploit-Testplan | 1 | — | Must, ohne Vorbedingung |
| ⬜ | T-074 | Playtest-Protokoll | 1 | — | Must, ohne Vorbedingung |
| 🟨 | T-007 | Automatisierte Smoke-Tests | 2 | — | Should |
| ⬜ | T-032 | Asset-Import-Automatisierung | 2 | — | Should |
| ⬜ | T-050 | Balancing-Dashboard | 2 | — | Should |
| ⬜ | T-051 | Level-Design-Werkzeuge | 2 | — | Should |
| ⬜ | T-061 | Telemetrie-Pipeline | 2 | — | Should |
| ⬜ | T-062 | Feature-Flag-Dienst | 2 | — | Should |
| ⬜ | T-063 | Wartungsmodus | 2 | — | Should |
| ⬜ | T-053 | Replay-System | 3 | — | Could |

## vector (20 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | F-001 | Doppelhaken-Grundsystem | 1 | — | Must, ohne Vorbedingung |
| ⬜ | F-010 | Slide-Dodge am Boden | 1 | — | Must, ohne Vorbedingung |
| ⬜ | F-002 | Freies Zielen per Raycast (Ebene 1) | 1 | F-001 | braucht F-001 |
| ⬜ | F-004 | Pendelphysik bei Zwei-Haken-Zustand | 1 | F-001 | braucht F-001 |
| ⬜ | F-007 | Gas-Boost | 1 | F-001 | braucht F-001 |
| ⬜ | F-009 | Flips (seitlich) | 1 | F-001 | braucht F-001 |
| ⬜ | F-011 | Hook-Break (Notbremse) | 2 | F-001 | braucht F-001 |
| ⬜ | F-003 | Getaggte Ankerflaechen (Ebene 1) | 1 | F-002 | braucht F-002 |
| ⬜ | F-005 | Reel-In / Seilverkuerzung | 1 | F-004 | braucht F-004 |
| ⬜ | F-006 | Swerve-Steuerung | 1 | F-004 | braucht F-004 |
| ⬜ | F-008 | Boost-Dash | 1 | F-007 | braucht F-007 |
| ⬜ | F-012 | Velocity-Clamp gegen Fling | 1 | F-004 | braucht F-004 |
| ⬜ | F-013 | Kollisionsdaempfung | 1 | F-004 | braucht F-004 |
| ⬜ | F-014 | Momentum-Chaining | 1 | F-004 | braucht F-004 |
| ⬜ | F-017 | Geschwindigkeits-Feedback | 1 | F-004 | braucht F-004 |
| ⬜ | F-018 | Gas-Ressource | 1 | F-007 | braucht F-007 |
| ⬜ | F-019 | Nachschub-Stationen | 1 | F-018 | braucht F-018 |
| ⬜ | F-015 | Wall-Run und Wall-Kick | 3 | F-013 | braucht F-013 |
| ⬜ | F-020 | Tragbarer Notvorrat | 3 | F-019 | braucht F-019 |
| ⬜ | F-016 | Ziel-Assist-Regler | 2 | F-024 | braucht F-024 |

## world (12 offen)

| Stufe | ID | Sache | Prio | haengt an | warum hier |
|---|---|---|---|---|---|
| ⬜ | F-021 | Diskrete Ankerpunkte (Ebene 2) | 1 | F-003 | braucht F-003 |
| ⬜ | F-022 | Prozedurale Ankerpunkt-Erzeugung | 1 | F-021 | braucht F-021 |
| ⬜ | F-023 | Kandidatensuche mit Hemisphaeren-Aufteilung | 1 | F-021 | braucht F-021 |
| ⬜ | F-029 | Dynamische Ankerpunkte | 2 | F-021 | braucht F-021 |
| ⬜ | F-024 | Snap auf Q und E | 1 | F-023 | braucht F-023 |
| ⬜ | F-025 | Bewertungsfunktion fuer Kandidaten | 1 | F-023 | braucht F-023 |
| ⬜ | F-026 | Highlighting der Ankerpunkte | 1 | F-023 | braucht F-023 |
| ⬜ | F-030a | Performance der Kandidatensuche | 1 | F-023 | braucht F-023 |
| ⬜ | F-031a | Ankerpunkt-Validierung im Editor | 1 | F-022 | braucht F-022 |
| ⬜ | F-027 | Marker-Dichtebegrenzung | 1 | F-026 | braucht F-026 |
| ⬜ | F-028 | Fallback ohne Kandidat | 1 | F-024 | braucht F-024 |
| ⬜ | F-032a | Trainingsanzeige fuer Ankerpunkte | 2 | F-025 | braucht F-025 |

