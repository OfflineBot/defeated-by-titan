#!/usr/bin/env python3
"""features.py — liest gameplay/features.xlsx aus und erzeugt die Arbeitsformate.

    python3 tools/features.py [--pruefen]

Warum ein Skript und kein Abtippen: bei ~800 Zeilen verliert Abtippen garantiert
Zeilen, und niemand merkt welche (prompts/init.md §2).

Warum die Standardbibliothek und nicht openpyxl: auf Maschine A (debian) gibt es
weder pip noch passwortloses sudo. Eine .xlsx ist ein ZIP aus XML — das reicht.
Damit laeuft die Extraktion auf jeder Maschine ohne Installation.

Erzeugt:
    docs/backlog/<blatt>.ron   ein RON pro Excel-Blatt (die Blaetter haben
                               verschiedene Spalten, deshalb je eine Datei)
    docs/backlog/README.md     aus Blatt 00_Anleitung
    docs/features.ron          die Arbeitsliste (F-IDs + T-IDs) — MERGE, siehe unten
    docs/TODO.md               generierte Ansicht, nach Domaene, baubare Reihenfolge
    docs/STATUS.md             generierte Ansicht, die vier Stufen (§8)

MERGE statt Ueberschreiben: docs/features.ron traegt `stufe`, `beleg` und `notiz` —
das ist Arbeitsstand, kein Excel-Inhalt. Beim erneuten Lauf werden diese Felder je
`id` aus der vorhandenen Datei uebernommen. Verschwundene Zeilen werden NICHT
still geloescht, sondern gemeldet (§2).

Der Beweis, dass nichts verlorenging, ist eine Zahl: die Zeilenzahl je Blatt steht
in ERWARTETE_ZEILEN und stammt aus prompts/init.md §2. Stimmt sie nicht, bricht das
Skript ab — dann weisst du, wie viele Zeilen fehlen, statt es zu ahnen (§9).
"""

from __future__ import annotations

import re
import sys
import zipfile
import xml.etree.ElementTree as ET
from pathlib import Path

M = "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}"
R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"

WURZEL = Path(__file__).resolve().parent.parent
XLSX = WURZEL / "gameplay" / "features.xlsx"
BACKLOG = WURZEL / "docs" / "backlog"

# Pruefwert aus prompts/init.md §2 — Zeilen inklusive Kopf, Titel und Leerzeilen.
ERWARTETE_ZEILEN = {
    "00_Anleitung": 28,
    "01_Spielfunktionen": 197,
    "02_3D-Assets": 103,
    "03_Animationen": 103,
    "04_Texturen": 31,
    "05_VFX": 41,
    "06_Audio": 121,
    "07_UI-Screens": 47,
    "08_Maps": 15,
    "09_Tech-Backlog": 53,
    "10_Namensschema": 43,
    "11_Zusammenfassung": 18,
}

# Blatt -> Zieldatei unter docs/backlog/. 11_Zusammenfassung wird bewusst NICHT
# uebertragen: sie ist berechnet, unsere Zahlen kommen aus features.ron (§2).
BLATT_ZU_RON = {
    "01_Spielfunktionen": "funktionen",
    "02_3D-Assets": "modelle",
    "03_Animationen": "animationen",
    "04_Texturen": "texturen",
    "05_VFX": "vfx",
    "06_Audio": "audio",
    "07_UI-Screens": "ui",
    "08_Maps": "maps",
    "09_Tech-Backlog": "tech",
    "10_Namensschema": "namensschema",
}

# Spaltenueberschrift -> RON-Schluessel. snake_case, deutsch, eine Sprache pro
# Datei (§10). Was hier fehlt, wird aus der Ueberschrift abgeleitet.
SPALTE_ZU_SCHLUESSEL = {
    "ID": "id",
    "System": "system",
    "Feature": "name",
    "Beschreibung": "beschreibung",
    "Akzeptanzkriterium": "akzeptanz",
    "Prio": "prio",
    "Aufwand (PT)": "aufwand_pt",
    "Disziplin": "disziplin",
    "Abhaengig von": "abhaengt_von",
    "Status": "status",
    "Kategorie": "kategorie",
    "Asset": "name",
    "Tris LOD0": "tris_lod0",
    "Tris LOD1": "tris_lod1",
    "Tris LOD2": "tris_lod2",
    "Varianten": "varianten",
    "Textur-Slot": "textur_slot",
    "Rig": "rig",
    "Clip": "name",
    "Dauer (s)": "dauer_s",
    "Loop": "loop",
    "Name": "name",
    "Aufloesung": "aufloesung",
    "Typ": "typ",
    "Technik": "technik",
    "Ausloeser": "ausloeser",
    "Screen": "name",
    "Wichtigste Elemente": "elemente",
    "Plattform": "plattform",
    "Map": "name",
    "Unterstuetzte Modi": "modi",
    "Groesse (studs)": "groesse_studs",
    "Ankerdichte": "ankerdichte",
    "Modul": "modul",
    "Task": "name",
    "Rolle": "rolle",
    "Referenzbegriff": "referenzbegriff",
    "Defeated by Titan": "begriff",
    "Anmerkung": "anmerkung",
}

# Excel-System -> Domaene aus prompts/init.md §5. Das Feld `system` bleibt
# zusaetzlich erhalten, damit diese Zuordnung nachpruefbar und korrigierbar ist.
SYSTEM_ZU_DOMAENE = {
    "Vector Gear": "vector",
    "Ankersystem": "world",
    "Kampf": "combat",
    "Titan-KI": "titan",
    "Missionen": "mission",
    "Raids": "mission",
    "Vessel Form": "player",
    "Progression": "progress",
    "Oekonomie": "progress",
    "Mehrspieler": "squad",
    "Sozial": "squad",
    "UI": "hud",
    "Onboarding": "mission",
    "Backend": "save",
    "Sicherheit": "net",
    "Monetarisierung": "progress",
    "Live Ops": "mission",
}

MODUL_ZU_DOMAENE = {
    "Projekt": "tooling",
    "Architektur": "shared",
    "Daten": "data",
    "Physik": "vector",
    "Rendering": "render",
    "Netzwerk": "net",
    "Persistenz": "save",
    "Test": "tooling",
    "Werkzeuge": "tooling",
    "Performance": "render",
    "Sicherheit": "net",
    "Audio": "sound",
    "Build": "tooling",
    "Telemetrie": "tooling",
}

PRIO_ZU_ZAHL = {"Must": 1, "Should": 2, "Could": 3}

# Backlog-Status -> Stufe (§2). `Fertig` setzt nur der User, deshalb steht es hier
# nicht als Zielwert eines Skriptlaufs.
STATUS_ZU_STUFE = {
    "Offen": "Nicht",
    "In Arbeit": "Halb",
    "Review": "Fast",
    "Fertig": "Fertig",
    "Zurueckgestellt": "Nicht",
    "Gestrichen": "Nicht",
}

STUFE_MARKE = {"Nicht": "⬜", "Halb": "🟨", "Fast": "🟧", "Fertig": "✅"}


# --------------------------------------------------------------------------
# xlsx lesen
# --------------------------------------------------------------------------

def spalten_nummer(ref: str) -> int:
    n = 0
    for ch in re.match(r"([A-Z]+)", ref).group(1):
        n = n * 26 + (ord(ch) - 64)
    return n


def spalten_name(n: int) -> str:
    s = ""
    while n:
        n, rest = divmod(n - 1, 26)
        s = chr(65 + rest) + s
    return s


class Blatt:
    def __init__(self, name: str, zeilen: dict[int, dict[str, str]], zeilenzahl: int):
        self.name = name
        self.zeilen = zeilen          # Zeilennummer -> {Spaltenbuchstabe: Text}
        self.zeilenzahl = zeilenzahl  # Anzahl <row>-Elemente, der Pruefwert


def lies_arbeitsmappe(pfad: Path) -> list[Blatt]:
    z = zipfile.ZipFile(pfad)

    shared: list[str] = []
    if "xl/sharedStrings.xml" in z.namelist():
        for si in ET.fromstring(z.read("xl/sharedStrings.xml")):
            shared.append("".join(t.text or "" for t in si.iter(M + "t")))

    wb = ET.fromstring(z.read("xl/workbook.xml"))
    rels = ET.fromstring(z.read("xl/_rels/workbook.xml.rels"))
    rid_zu_ziel = {r.get("Id"): r.get("Target") for r in rels}

    blaetter = []
    for sh in wb.find(M + "sheets"):
        ziel = rid_zu_ziel[sh.get(R + "id")]
        if not ziel.startswith("xl/"):
            ziel = "xl/" + ziel.lstrip("/")
        ws = ET.fromstring(z.read(ziel))

        # Verbundene Zellen wuerden leere Nachbarwerte liefern (§2). Diese Mappe
        # hat keine — wenn doch eine dazukommt, soll es auffallen, nicht stillschweigen.
        verbund = ws.findall(".//" + M + "mergeCell")
        if verbund:
            print(f"WARNUNG: {sh.get('name')} hat {len(verbund)} verbundene Zellen "
                  f"— Werte pruefen (init.md §2).", file=sys.stderr)

        rohzeilen = ws.findall(".//" + M + "sheetData/" + M + "row")
        zeilen: dict[int, dict[str, str]] = {}
        for r in rohzeilen:
            nr = int(r.get("r"))
            felder: dict[str, str] = {}
            for c in r.findall(M + "c"):
                ref = c.get("r")
                sp = re.match(r"([A-Z]+)", ref).group(1)
                v = c.find(M + "v")
                if v is not None:
                    # data_only: <v> ist der zwischengespeicherte WERT, auch wenn
                    # daneben ein <f> mit der Formel steht (§2).
                    txt = shared[int(v.text)] if c.get("t") == "s" else (v.text or "")
                else:
                    inline = c.find(M + "is")
                    if inline is None:
                        if c.find(M + "f") is not None:
                            print(f"WARNUNG: {sh.get('name')}!{ref} ist eine Formel "
                                  f"OHNE zwischengespeicherten Wert — in Excel oeffnen "
                                  f"und speichern, oder von Hand nachzaehlen.",
                                  file=sys.stderr)
                        continue
                    txt = "".join(t.text or "" for t in inline.iter(M + "t"))
                txt = txt.strip()
                if txt:
                    felder[sp] = txt
            if felder:
                zeilen[nr] = felder
        blaetter.append(Blatt(sh.get("name"), zeilen, len(rohzeilen)))
    return blaetter


def kopfzeile(blatt: Blatt) -> tuple[int, dict[str, str]] | None:
    """Die Kopfzeile steht nicht ueberall in derselben Zeile (mal 3, mal 4).

    `None` heisst: dieses Blatt ist Fliesstext (00_Anleitung), keine Tabelle.
    """
    for nr in sorted(blatt.zeilen):
        felder = blatt.zeilen[nr]
        if felder.get("A") in ("ID", "Referenzbegriff", "Blatt"):
            return nr, felder
    return None


def datensaetze(blatt: Blatt) -> list[tuple[int, dict[str, str]]]:
    """(Zeilennummer, {RON-Schluessel: Wert}) fuer jede Datenzeile."""
    kopf = kopfzeile(blatt)
    if kopf is None:
        return []
    kopf_nr, kopf = kopf
    schluessel = {}
    for sp, titel in kopf.items():
        schluessel[sp] = SPALTE_ZU_SCHLUESSEL.get(
            titel, re.sub(r"[^a-z0-9]+", "_", titel.lower()).strip("_"))
    saetze = []
    for nr in sorted(blatt.zeilen):
        if nr <= kopf_nr:
            continue
        felder = blatt.zeilen[nr]
        satz = {schluessel[sp]: wert for sp, wert in felder.items() if sp in schluessel}
        if satz:
            saetze.append((nr, satz))
    return saetze


# --------------------------------------------------------------------------
# RON schreiben
# --------------------------------------------------------------------------

def ron_text(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def ron_wert(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, list):
        return "[" + ", ".join(ron_wert(x) for x in v) + "]"
    if isinstance(v, Roh):
        return v.text
    return ron_text(str(v))


class Roh:
    """Ein Wert, der unveraendert in die RON geht (Enum-Variante, Zahl)."""
    def __init__(self, text: str):
        self.text = text


KOPF_GENERIERT = (
    "// GENERIERT von tools/features.py aus gameplay/features.xlsx — NICHT von Hand aendern.\n"
    "// Handarbeit hier ist beim naechsten Lauf verloren. Quelle aendern heisst: die .xlsx\n"
    "// aendern und das Skript erneut laufen lassen (prompts/init.md §2).\n"
)


def schreibe_ron(pfad: Path, wurzel_name: str, saetze: list[dict], quelle: str) -> None:
    zeilen = [KOPF_GENERIERT, f"// Quelle: {quelle}\n", f"{wurzel_name}: [\n"]
    for satz in saetze:
        felder = ", ".join(f"{k}: {ron_wert(v)}" for k, v in satz.items())
        zeilen.append(f"    ({felder}),\n")
    zeilen.append("]\n")
    pfad.parent.mkdir(parents=True, exist_ok=True)
    pfad.write_text("".join(zeilen), encoding="utf-8")


# --------------------------------------------------------------------------
# features.ron: erzeugen und mit dem vorhandenen Arbeitsstand mischen
# --------------------------------------------------------------------------

def lies_arbeitsstand(pfad: Path) -> dict[str, dict[str, str]]:
    """Holt stufe/beleg/notiz je id aus einer vorhandenen features.ron.

    Bewusst eine Regex und kein RON-Parser: die Datei ist von diesem Skript
    erzeugt, das Format also bekannt, und eine Abhaengigkeit weniger.
    """
    if not pfad.exists():
        return {}
    stand: dict[str, dict[str, str]] = {}
    for zeile in pfad.read_text(encoding="utf-8").splitlines():
        m_id = re.search(r'id:\s*"([^"]+)"', zeile)
        if not m_id:
            continue
        eintrag = {}
        m = re.search(r"stufe:\s*(\w+)", zeile)
        if m:
            eintrag["stufe"] = m.group(1)
        for feld in ("beleg", "notiz"):
            m = re.search(rf'{feld}:\s*"((?:[^"\\]|\\.)*)"', zeile)
            if m:
                eintrag[feld] = m.group(1)
        stand[m_id.group(1)] = eintrag
    return stand


def baue_features(blaetter: dict[str, Blatt]) -> list[dict]:
    eintraege: list[dict] = []

    for nr, satz in datensaetze(blaetter["01_Spielfunktionen"]):
        system = satz.get("system", "")
        abh = [t.strip() for t in re.split(r"[,;]", satz.get("abhaengt_von", "")) if t.strip()]
        eintraege.append({
            "id": satz.get("id", ""),
            "name": satz.get("name", ""),
            "domain": SYSTEM_ZU_DOMAENE.get(system, "offen"),
            "system": system,
            "stufe": Roh(STATUS_ZU_STUFE.get(satz.get("status", "Offen"), "Nicht")),
            "beschreibung": satz.get("beschreibung", ""),
            "akzeptanz": satz.get("akzeptanz", ""),
            "abhaengt_von": abh,
            "prio": PRIO_ZU_ZAHL.get(satz.get("prio", "Could"), 3),
            "aufwand_pt": Roh(satz.get("aufwand_pt", "0")),
            "quelle": f"features.xlsx!01_Spielfunktionen!Z{nr}",
            "beleg": "",
            "notiz": "",
        })

    # Das Tech-Backlog kommt mit in die Arbeitsliste: es traegt genau die Zeilen,
    # die das Aufsetzen betreffen (T-IDs), und ohne sie haette docs/STATUS.md
    # keine Zeile fuer Fenster, Werkzeuge oder Tests. Benannte Abweichung von der
    # Tabelle in §2 — dort ist features.ron nur aus Blatt 01 gespeist.
    for nr, satz in datensaetze(blaetter["09_Tech-Backlog"]):
        modul = satz.get("modul", "")
        eintraege.append({
            "id": satz.get("id", ""),
            "name": satz.get("name", ""),
            "domain": MODUL_ZU_DOMAENE.get(modul, "tooling"),
            "system": modul,
            "stufe": Roh(STATUS_ZU_STUFE.get(satz.get("status", "Offen"), "Nicht")),
            "beschreibung": satz.get("beschreibung", ""),
            "akzeptanz": "",
            "abhaengt_von": [],
            "prio": PRIO_ZU_ZAHL.get(satz.get("prio", "Could"), 3),
            "aufwand_pt": Roh(satz.get("aufwand_pt", "0")),
            "quelle": f"features.xlsx!09_Tech-Backlog!Z{nr}",
            "beleg": "",
            "notiz": "",
        })
    return eintraege


def mische(neu: list[dict], stand: dict[str, dict[str, str]]) -> list[str]:
    """Traegt den Arbeitsstand nach. Gibt die Meldungen fuer docs/FRAGEN.md zurueck."""
    meldungen = []
    ids_neu = {e["id"] for e in neu}
    for e in neu:
        alt = stand.get(e["id"])
        if not alt:
            continue
        if "stufe" in alt:
            e["stufe"] = Roh(alt["stufe"])
        for feld in ("beleg", "notiz"):
            if alt.get(feld):
                e[feld] = alt[feld]
    for alte_id, alt in stand.items():
        if alte_id not in ids_neu:
            meldungen.append(
                f"{alte_id} war in docs/features.ron, steht aber nicht mehr in der "
                f"features.xlsx (Stufe war {alt.get('stufe', '?')})")
    return meldungen


# --------------------------------------------------------------------------
# Ansichten
# --------------------------------------------------------------------------

KOPF_MD = (
    "<!-- GENERIERT von tools/features.py aus docs/features.ron — NICHT von Hand aendern.\n"
    "     Handarbeit hier ist beim naechsten Lauf verloren. Arbeitsstand (Stufe, Beleg)\n"
    "     gehoert nach docs/features.ron, dann `python3 tools/features.py`.\n"
    "     Ohne 'Stand:'-Zeile mit Absicht: der Stand ist der von docs/features.ron, und\n"
    "     ein Datum, das sich bei jedem Lauf aendert, ist Diff-Rauschen. -->\n\n"
)


def baubare_reihenfolge(eintraege: list[dict]) -> list[dict]:
    """Topologisch nach abhaengt_von, bei Gleichstand nach Prio, dann nach ID."""
    nach_id = {e["id"]: e for e in eintraege}
    fertig: list[dict] = []
    gesetzt: set[str] = set()
    offen = sorted(eintraege, key=lambda e: (e["prio"], e["id"]))
    while offen:
        runde = [e for e in offen
                 if all(a in gesetzt or a not in nach_id for a in e["abhaengt_von"])]
        if not runde:      # Zyklus — nicht still auflösen, sondern anhaengen
            runde = offen[:]
        for e in runde:
            fertig.append(e)
            gesetzt.add(e["id"])
        offen = [e for e in offen if e["id"] not in gesetzt]
    return fertig


def schreibe_todo(pfad: Path, eintraege: list[dict]) -> None:
    z = [KOPF_MD, "# TODO — offene Arbeit, in baubarer Reihenfolge\n\n",
         "Sortiert nach Domaene; innerhalb der Domaene so, dass `abhaengt_von` erfuellt\n"
         "ist, bevor eine Zeile drankommt. Prio 1 = Must, 2 = Should, 3 = Could —\n"
         "`Must` vor `Should` vor `Could` ist die Reihenfolge, keine Empfehlung\n"
         "(prompts/init.md §2).\n\n"]
    geordnet = baubare_reihenfolge(eintraege)
    for domaene in sorted({e["domain"] for e in eintraege}):
        zeilen = [e for e in geordnet
                  if e["domain"] == domaene and e["stufe"].text != "Fertig"]
        if not zeilen:
            continue
        z.append(f"## {domaene} ({len(zeilen)} offen)\n\n")
        z.append("| Stufe | ID | Sache | Prio | haengt an | warum hier |\n")
        z.append("|---|---|---|---|---|---|\n")
        for e in zeilen:
            abh = ", ".join(e["abhaengt_von"]) or "—"
            grund = (f"braucht {abh}" if e["abhaengt_von"]
                     else {1: "Must, ohne Vorbedingung", 2: "Should", 3: "Could"}[e["prio"]])
            z.append(f"| {STUFE_MARKE[e['stufe'].text]} | {e['id']} | {e['name']} "
                     f"| {e['prio']} | {abh} | {grund} |\n")
        z.append("\n")
    pfad.write_text("".join(z), encoding="utf-8")


def schreibe_status(pfad: Path, eintraege: list[dict]) -> None:
    zaehler = {k: 0 for k in STUFE_MARKE}
    for e in eintraege:
        zaehler[e["stufe"].text] += 1
    z = [KOPF_MD, "# STATUS — was implementiert ist und was nicht\n\n",
         "Stufen: ⬜ nicht implementiert · 🟨 halb (gebaut, ungetestet, ungesehen) ·\n"
         "🟧 fast (Tests, die umfallen + im Spiel gesehen) · ✅ fertig "
         "(**nur der User setzt das**).\n\n",
         "**🟧 braucht drei Belege:** Bild (Screenshot-Pfad), Zahl (gemessen, mit "
         "Maschine `[debian]`/`[cachy]`) und Code (ein Test, der rot wird, wenn es "
         "kaputtgeht). Fehlt einer, ist es 🟨 — Unsicherheit setzt die Stufe herunter, "
         "nicht hinauf (prompts/init.md §8, §9).\n\n",
         f"**Stand:** {zaehler['Nicht']} ⬜ · {zaehler['Halb']} 🟨 · "
         f"{zaehler['Fast']} 🟧 · {zaehler['Fertig']} ✅ "
         f"von {len(eintraege)} Zeilen.\n\n"]
    for domaene in sorted({e["domain"] for e in eintraege}):
        zeilen = [e for e in eintraege if e["domain"] == domaene]
        rang = {"Fertig": 0, "Fast": 1, "Halb": 2, "Nicht": 3}
        zeilen.sort(key=lambda e: (rang[e["stufe"].text], e["prio"], e["id"]))
        z.append(f"## {domaene}\n\n")
        z.append("| Sache | ID | Stufe | Beleg (Test / Screenshot / Zahl) | Stand |\n")
        z.append("|---|---|---|---|---|\n")
        for e in zeilen:
            beleg = e["beleg"] or "—"
            stand = e["notiz"] or "—"
            z.append(f"| {e['name']} | {e['id']} | {STUFE_MARKE[e['stufe'].text]} "
                     f"| {beleg} | {stand} |\n")
        z.append("\n")
    pfad.write_text("".join(z), encoding="utf-8")


def schreibe_backlog_readme(pfad: Path, blatt: Blatt, blaetter: dict[str, Blatt]) -> None:
    z = ["<!-- GENERIERT von tools/features.py aus gameplay/features.xlsx, "
         "Blatt 00_Anleitung. -->\n\n",
         "# docs/backlog/ — die Excel-Blaetter als Daten\n\n",
         "Ein RON pro Blatt, weil die Blaetter verschiedene Spalten haben. Die `.xlsx`\n"
         "selbst bleibt liegen und unangetastet — sie ist die Quelle, und der User\n"
         "arbeitet darin weiter (prompts/init.md §2).\n\n",
         "## Was in der Anleitung des Backlogs steht\n\n"]
    for nr in sorted(blatt.zeilen):
        felder = blatt.zeilen[nr]
        a, b = felder.get("A", ""), felder.get("B", "")
        if a and b:
            z.append(f"- **{a}** — {b}\n")
        elif a:
            z.append(f"\n### {a}\n\n" if len(a) < 60 else f"{a}\n\n")
    z.append("\n## Die Blaetter und ihre Dateien\n\n")
    z.append("| Blatt | Zeilen (inkl. Kopf) | Datensaetze | Datei |\n|---|---|---|---|\n")
    for name in ERWARTETE_ZEILEN:
        b = blaetter[name]
        ziel = BLATT_ZU_RON.get(name)
        datei = f"`{ziel}.ron`" if ziel else "— (berechnet, nicht uebertragen)"
        anzahl = len(datensaetze(b)) if ziel else "—"
        z.append(f"| `{name}` | {b.zeilenzahl} | {anzahl} | {datei} |\n")
    z.append("\n**Die Zeilenzahl ist der Pruefwert.** `python3 tools/features.py --pruefen`\n"
             "faellt um, wenn eine Zahl nicht mehr stimmt — dann ist die Extraktion nicht\n"
             "fertig, und man weiss genau, wie viele Zeilen fehlen (prompts/init.md §2, §9).\n\n"
             "`docs/features.ron` ist die Arbeitsliste daraus: alle F-IDs aus\n"
             "`01_Spielfunktionen` **und** alle T-IDs aus `09_Tech-Backlog`. Die T-Zeilen\n"
             "sind mit drin, weil genau sie das Aufsetzen beschreiben — ohne sie haette\n"
             "`docs/STATUS.md` keine Zeile fuer Fenster, Werkzeuge oder Tests. Das ist eine\n"
             "benannte Abweichung von der Tabelle in `prompts/init.md` §2, wo\n"
             "`features.ron` nur aus Blatt 01 gespeist wird.\n")
    pfad.write_text("".join(z), encoding="utf-8")


# --------------------------------------------------------------------------

def main() -> int:
    nur_pruefen = "--pruefen" in sys.argv
    if not XLSX.exists():
        print(f"FEHLER: {XLSX} fehlt", file=sys.stderr)
        return 2

    blaetter = {b.name: b for b in lies_arbeitsmappe(XLSX)}

    # 1. Pruefwert: Zeilenzahl je Blatt
    fehler = []
    for name, erwartet in ERWARTETE_ZEILEN.items():
        if name not in blaetter:
            fehler.append(f"Blatt {name} fehlt in der Mappe")
            continue
        ist = blaetter[name].zeilenzahl
        if ist != erwartet:
            fehler.append(f"Blatt {name}: {ist} Zeilen, erwartet {erwartet} "
                          f"(Differenz {ist - erwartet})")
    for name in blaetter:
        if name not in ERWARTETE_ZEILEN:
            fehler.append(f"Blatt {name} ist neu — ERWARTETE_ZEILEN ergaenzen")
    if fehler:
        print("PRUEFWERT NICHT ERFUELLT (prompts/init.md §2):", file=sys.stderr)
        for f in fehler:
            print("  - " + f, file=sys.stderr)
        return 1

    saetze_je_blatt = {name: datensaetze(b) for name, b in blaetter.items()}

    if nur_pruefen:
        print(f"{'Blatt':<22} {'Zeilen':>7} {'Datensaetze':>12}")
        for name in ERWARTETE_ZEILEN:
            print(f"{name:<22} {blaetter[name].zeilenzahl:>7} "
                  f"{len(saetze_je_blatt[name]):>12}")
        gesamt = sum(len(saetze_je_blatt[n]) for n in BLATT_ZU_RON)
        print(f"{'SUMME (uebertragen)':<22} {'':>7} {gesamt:>12}")
        return 0

    # 2. Ein RON pro Blatt
    for blatt_name, datei in BLATT_ZU_RON.items():
        saetze = [dict(s) for _, s in saetze_je_blatt[blatt_name]]
        wurzel = "eintraege"
        schreibe_ron(BACKLOG / f"{datei}.ron", wurzel, saetze,
                     f"gameplay/features.xlsx!{blatt_name} "
                     f"({len(saetze)} Datensaetze aus "
                     f"{blaetter[blatt_name].zeilenzahl} Zeilen)")

    schreibe_backlog_readme(BACKLOG / "README.md", blaetter["00_Anleitung"], blaetter)

    # 3. Arbeitsliste mit Merge
    ziel = WURZEL / "docs" / "features.ron"
    stand = lies_arbeitsstand(ziel)
    eintraege = baue_features(blaetter)
    verschwunden = mische(eintraege, stand)
    schreibe_ron(ziel, "features", eintraege,
                 "gameplay/features.xlsx!01_Spielfunktionen + !09_Tech-Backlog")

    # 4. Ansichten
    schreibe_todo(WURZEL / "docs" / "TODO.md", eintraege)
    schreibe_status(WURZEL / "docs" / "STATUS.md", eintraege)

    # 5. Bericht
    print(f"{'Blatt':<22} {'Zeilen':>7} {'Datensaetze':>12}  Datei")
    for name in ERWARTETE_ZEILEN:
        datei = BLATT_ZU_RON.get(name)
        print(f"{name:<22} {blaetter[name].zeilenzahl:>7} "
              f"{len(saetze_je_blatt[name]):>12}  "
              f"{'docs/backlog/' + datei + '.ron' if datei else '(nicht uebertragen)'}")
    print(f"\ndocs/features.ron: {len(eintraege)} Eintraege "
          f"({len(saetze_je_blatt['01_Spielfunktionen'])} F + "
          f"{len(saetze_je_blatt['09_Tech-Backlog'])} T)")
    if verschwunden:
        print("\nACHTUNG — Zeilen aus dem alten Stand fehlen in der neuen .xlsx.")
        print("Nach docs/FRAGEN.md eintragen, NICHT still loeschen (init.md §2):")
        for m in verschwunden:
            print("  - " + m)
    doppelt = [e["id"] for e in eintraege
               if [x["id"] for x in eintraege].count(e["id"]) > 1]
    if doppelt:
        print(f"\nWARNUNG: doppelte IDs: {sorted(set(doppelt))}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
