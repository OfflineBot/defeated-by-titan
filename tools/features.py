#!/usr/bin/env python3
"""features.py — reads gameplay/features.xlsx and writes the working formats.

    python3 tools/features.py [--check]

Why a script and not typing it out: at ~800 rows, typing it out loses rows for
certain, and nobody notices which ones (prompts/init.md §2).

Why the standard library and not openpyxl: machine A (debian) has neither pip nor
passwordless sudo. An .xlsx is a ZIP of XML — that is enough. It makes the
extraction run on any machine without installing anything.

Writes:
    docs/backlog/<sheet>.ron   one RON per spreadsheet sheet (the sheets have
                               different columns, hence one file each)
    docs/backlog/README.md     from sheet 00_Anleitung
    docs/features.ron          the working list (F-IDs + T-IDs) — MERGED, see below
    docs/TODO.md               generated view, by domain, in buildable order
    docs/STATUS.md             generated view, the four stages (§8)

MERGE instead of overwrite: docs/features.ron carries `stage`, `evidence` and `note` —
that is work state, not spreadsheet content. On a re-run those fields are carried over
per `id` from the existing file. Rows that have vanished are NOT deleted silently, they
are reported (§2).

The proof that nothing was lost is a number: the row count per sheet lives in
EXPECTED_ROWS and comes from prompts/init.md §2. If it no longer matches, the script
stops — then you know how many rows are missing instead of guessing (§9).
"""

from __future__ import annotations

import re
import sys
import zipfile
import xml.etree.ElementTree as ET
from pathlib import Path

M = "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}"
R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"

ROOT = Path(__file__).resolve().parent.parent
XLSX = ROOT / "gameplay" / "features.xlsx"
BACKLOG = ROOT / "docs" / "backlog"

# The row-count guard from prompts/init.md §2 — rows including header, title and blank
# lines. The sheet names are the user's own tabs and stay exactly as he wrote them.
EXPECTED_ROWS = {
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

# Sheet -> target file under docs/backlog/. 11_Zusammenfassung is deliberately NOT
# transferred: it is computed, and our numbers come from features.ron (§2).
SHEET_TO_RON = {
    "01_Spielfunktionen": "gameplay",
    "02_3D-Assets": "models",
    "03_Animationen": "animations",
    "04_Texturen": "textures",
    "05_VFX": "vfx",
    "06_Audio": "audio",
    "07_UI-Screens": "ui",
    "08_Maps": "maps",
    "09_Tech-Backlog": "tech",
    "10_Namensschema": "naming",
}

# Column heading -> RON key. The keys are the user's German column headings and are
# looked up verbatim; the values are the English RON keys we write. What is missing
# here is derived from the heading — and that derivation is SILENT: a key that no
# longer matches the spreadsheet exactly does not raise, it quietly writes a German
# key into the RON that nobody looks at again.
COLUMN_TO_KEY = {
    "ID": "id",
    "System": "system",
    "Feature": "name",
    "Beschreibung": "description",
    "Akzeptanzkriterium": "acceptance",
    "Prio": "prio",
    "Aufwand (PT)": "effort_pd",
    "Disziplin": "discipline",
    "Abhaengig von": "depends_on",
    "Status": "status",
    "Kategorie": "category",
    "Asset": "name",
    "Tris LOD0": "tris_lod0",
    "Tris LOD1": "tris_lod1",
    "Tris LOD2": "tris_lod2",
    "Varianten": "variants",
    "Textur-Slot": "texture_slot",
    "Rig": "rig",
    "Clip": "name",
    "Dauer (s)": "duration_s",
    "Laenge": "length",
    "Loop": "loop",
    "Name": "name",
    "Aufloesung": "resolution",
    "Typ": "kind",
    "Technik": "technique",
    "Ausloeser": "trigger",
    "Screen": "name",
    "Wichtigste Elemente": "elements",
    "Plattform": "platform",
    "Map": "name",
    "Unterstuetzte Modi": "modes",
    "Groesse (studs)": "size_studs",
    "Ankerdichte": "anchor_density",
    "Modul": "module",
    "Task": "name",
    "Rolle": "role",
    "Referenzbegriff": "reference_term",
    "Defeated by Titan": "term",
    "Anmerkung": "note",
}

# Excel system -> domain, from prompts/init.md §5. The `system` field is kept as well,
# so that this mapping stays checkable and correctable. Keys are the user's spreadsheet
# values and stay German.
SYSTEM_TO_DOMAIN = {
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

MODULE_TO_DOMAIN = {
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

PRIO_TO_RANK = {"Must": 1, "Should": 2, "Could": 3}

# Backlog status -> stage (§2). `Accepted` is set only by the user, which is why it is
# not a target value of a script run. Keys are his spreadsheet values and stay German.
STATUS_TO_STAGE = {
    "Offen": "Unbuilt",
    "In Arbeit": "Built",
    "Review": "Proven",
    "Fertig": "Accepted",
    "Zurueckgestellt": "Unbuilt",
    "Gestrichen": "Unbuilt",
}

STAGE_MARK = {"Unbuilt": "⬜", "Built": "🟨", "Proven": "🟧", "Accepted": "✅"}


# --------------------------------------------------------------------------
# reading the xlsx
# --------------------------------------------------------------------------

def column_number(ref: str) -> int:
    n = 0
    for ch in re.match(r"([A-Z]+)", ref).group(1):
        n = n * 26 + (ord(ch) - 64)
    return n


def column_name(n: int) -> str:
    s = ""
    while n:
        n, rest = divmod(n - 1, 26)
        s = chr(65 + rest) + s
    return s


class Sheet:
    def __init__(self, name: str, rows: dict[int, dict[str, str]], row_count: int):
        self.name = name
        self.rows = rows            # row number -> {column letter: text}
        self.row_count = row_count  # number of <row> elements, the guard


def read_workbook(path: Path) -> list[Sheet]:
    z = zipfile.ZipFile(path)

    shared: list[str] = []
    if "xl/sharedStrings.xml" in z.namelist():
        for si in ET.fromstring(z.read("xl/sharedStrings.xml")):
            shared.append("".join(t.text or "" for t in si.iter(M + "t")))

    wb = ET.fromstring(z.read("xl/workbook.xml"))
    rels = ET.fromstring(z.read("xl/_rels/workbook.xml.rels"))
    rid_to_target = {r.get("Id"): r.get("Target") for r in rels}

    sheets = []
    for sh in wb.find(M + "sheets"):
        target = rid_to_target[sh.get(R + "id")]
        if not target.startswith("xl/"):
            target = "xl/" + target.lstrip("/")
        ws = ET.fromstring(z.read(target))

        # Merged cells would hand back empty neighboring values (§2). This workbook
        # has none — if one ever appears it should be noticed, not passed over.
        merged = ws.findall(".//" + M + "mergeCell")
        if merged:
            print(f"WARNING: {sh.get('name')} has {len(merged)} merged cells "
                  f"— check the values (init.md §2).", file=sys.stderr)

        raw_rows = ws.findall(".//" + M + "sheetData/" + M + "row")
        rows: dict[int, dict[str, str]] = {}
        for r in raw_rows:
            no = int(r.get("r"))
            fields: dict[str, str] = {}
            for c in r.findall(M + "c"):
                ref = c.get("r")
                col = re.match(r"([A-Z]+)", ref).group(1)
                v = c.find(M + "v")
                if v is not None:
                    # data_only: <v> is the CACHED value, even when an <f> with the
                    # formula sits right next to it (§2).
                    txt = shared[int(v.text)] if c.get("t") == "s" else (v.text or "")
                else:
                    inline = c.find(M + "is")
                    if inline is None:
                        if c.find(M + "f") is not None:
                            print(f"WARNING: {sh.get('name')}!{ref} is a formula "
                                  f"with NO cached value — open it in Excel and save, "
                                  f"or count it by hand.", file=sys.stderr)
                        continue
                    txt = "".join(t.text or "" for t in inline.iter(M + "t"))
                txt = txt.strip()
                if txt:
                    fields[col] = txt
            if fields:
                rows[no] = fields
        sheets.append(Sheet(sh.get("name"), rows, len(raw_rows)))
    return sheets


def header_row(sheet: Sheet) -> tuple[int, dict[str, str]] | None:
    """The header is not in the same row everywhere (sometimes 3, sometimes 4).

    `None` means: this sheet is prose (00_Anleitung), not a table. The three markers
    are the user's own column titles and stay German.
    """
    for no in sorted(sheet.rows):
        fields = sheet.rows[no]
        if fields.get("A") in ("ID", "Referenzbegriff", "Blatt"):
            return no, fields
    return None


def records(sheet: Sheet) -> list[tuple[int, dict[str, str]]]:
    """(row number, {RON key: value}) for every data row."""
    head = header_row(sheet)
    if head is None:
        return []
    head_no, head = head
    keys = {}
    for col, title in head.items():
        keys[col] = COLUMN_TO_KEY.get(
            title, re.sub(r"[^a-z0-9]+", "_", title.lower()).strip("_"))
    out = []
    for no in sorted(sheet.rows):
        if no <= head_no:
            continue
        fields = sheet.rows[no]
        record = {keys[col]: value for col, value in fields.items() if col in keys}
        if record:
            out.append((no, record))
    return out


# --------------------------------------------------------------------------
# writing RON
# --------------------------------------------------------------------------

def ron_string(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def ron_value(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, list):
        return "[" + ", ".join(ron_value(x) for x in v) + "]"
    if isinstance(v, Raw):
        return v.text
    return ron_string(str(v))


class Raw:
    """A value that goes into the RON unchanged (enum variant, number)."""
    def __init__(self, text: str):
        self.text = text


GENERATED_RON_HEADER = (
    "// GENERATED by tools/features.py from gameplay/features.xlsx — do NOT edit by hand.\n"
    "// Hand edits here are lost on the next run. Changing the source means: change the\n"
    "// .xlsx and run the script again (prompts/init.md §2).\n"
)


def write_ron(path: Path, root_name: str, rows: list[dict], source: str) -> None:
    lines = [GENERATED_RON_HEADER, f"// Source: {source}\n", f"{root_name}: [\n"]
    for row in rows:
        fields = ", ".join(f"{k}: {ron_value(v)}" for k, v in row.items())
        lines.append(f"    ({fields}),\n")
    lines.append("]\n")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(lines), encoding="utf-8")


# --------------------------------------------------------------------------
# features.ron: generate it and merge it with the existing work state
# --------------------------------------------------------------------------

def read_work_state(path: Path) -> dict[str, dict[str, str]]:
    """Pulls stage/evidence/note per id out of an existing features.ron.

    Deliberately a regex and not a RON parser: the file is written by this script, so
    the format is known, and it is one dependency less.
    """
    if not path.exists():
        return {}
    state: dict[str, dict[str, str]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        m_id = re.search(r'id:\s*"([^"]+)"', line)
        if not m_id:
            continue
        entry = {}
        m = re.search(r"stage:\s*(\w+)", line)
        if m:
            entry["stage"] = m.group(1)
        for field in ("evidence", "note"):
            m = re.search(rf'{field}:\s*"((?:[^"\\]|\\.)*)"', line)
            if m:
                entry[field] = m.group(1)
        state[m_id.group(1)] = entry
    # A features.ron full of ids and without a single `stage:` is not an empty work
    # state, it is a file this script no longer reads. Merging it anyway resets every
    # hand-set stage, every evidence line and every note to the spreadsheet default —
    # silently, with exit 0, in a diff that looks like a rename. Say it instead.
    if state and not any("stage" in e for e in state.values()):
        print(f"ERROR: {path} holds {len(state)} ids but not one `stage:` — this file "
              f"is not in the format this script reads. Merging it would reset every "
              f"stage, every evidence line and every note to the spreadsheet default. "
              f"Migrate the file first, then run again.", file=sys.stderr)
        raise SystemExit(2)
    return state


def build_features(sheets: dict[str, Sheet]) -> list[dict]:
    entries: list[dict] = []

    for no, record in records(sheets["01_Spielfunktionen"]):
        system = record.get("system", "")
        deps = [t.strip() for t in re.split(r"[,;]", record.get("depends_on", ""))
                if t.strip()]
        entries.append({
            "id": record.get("id", ""),
            "name": record.get("name", ""),
            "domain": SYSTEM_TO_DOMAIN.get(system, "open"),
            "system": system,
            "stage": Raw(STATUS_TO_STAGE.get(record.get("status", "Offen"), "Unbuilt")),
            "description": record.get("description", ""),
            "acceptance": record.get("acceptance", ""),
            "depends_on": deps,
            "prio": PRIO_TO_RANK.get(record.get("prio", "Could"), 3),
            "effort_pd": Raw(record.get("effort_pd", "0")),
            "source": f"features.xlsx!01_Spielfunktionen!R{no}",
            "evidence": "",
            "note": "",
        })

    # The tech backlog comes into the working list too: it carries exactly the rows the
    # setup is about (T-IDs), and without them docs/STATUS.md would have no row for the
    # window, the tools or the tests. A named deviation from the table in §2 — there,
    # features.ron is fed from sheet 01 alone.
    for no, record in records(sheets["09_Tech-Backlog"]):
        module = record.get("module", "")
        entries.append({
            "id": record.get("id", ""),
            "name": record.get("name", ""),
            "domain": MODULE_TO_DOMAIN.get(module, "tooling"),
            "system": module,
            "stage": Raw(STATUS_TO_STAGE.get(record.get("status", "Offen"), "Unbuilt")),
            "description": record.get("description", ""),
            "acceptance": "",
            "depends_on": [],
            "prio": PRIO_TO_RANK.get(record.get("prio", "Could"), 3),
            "effort_pd": Raw(record.get("effort_pd", "0")),
            "source": f"features.xlsx!09_Tech-Backlog!R{no}",
            "evidence": "",
            "note": "",
        })
    return entries


def merge(new: list[dict], state: dict[str, dict[str, str]]) -> list[str]:
    """Carries the work state over. Returns the messages for docs/QUESTIONS.md."""
    messages = []
    new_ids = {e["id"] for e in new}
    for e in new:
        old = state.get(e["id"])
        if not old:
            continue
        if "stage" in old:
            e["stage"] = Raw(old["stage"])
        for field in ("evidence", "note"):
            if old.get(field):
                e[field] = old[field]
    for old_id, old in state.items():
        if old_id not in new_ids:
            messages.append(
                f"{old_id} was in docs/features.ron but is no longer in "
                f"features.xlsx (stage was {old.get('stage', '?')})")
    return messages


# --------------------------------------------------------------------------
# views
# --------------------------------------------------------------------------

GENERATED_MD_HEADER = (
    "<!-- GENERATED by tools/features.py from docs/features.ron — do NOT edit by hand.\n"
    "     Hand edits here are lost on the next run. Work state (stage, evidence)\n"
    "     belongs in docs/features.ron, then `python3 tools/features.py`.\n"
    "     No 'Updated:' line, on purpose: the state is the state of docs/features.ron,\n"
    "     and a date that changes on every run is diff noise. -->\n\n"
)


def buildable_order(entries: list[dict]) -> list[dict]:
    """Topological by depends_on, ties broken by prio, then by ID."""
    by_id = {e["id"]: e for e in entries}
    done: list[dict] = []
    placed: set[str] = set()
    open_ = sorted(entries, key=lambda e: (e["prio"], e["id"]))
    while open_:
        round_ = [e for e in open_
                  if all(a in placed or a not in by_id for a in e["depends_on"])]
        if not round_:     # a cycle — do not resolve it silently, append it
            round_ = open_[:]
        for e in round_:
            done.append(e)
            placed.add(e["id"])
        open_ = [e for e in open_ if e["id"] not in placed]
    return done


def write_todo(path: Path, entries: list[dict]) -> None:
    z = [GENERATED_MD_HEADER, "# TODO — open work, in buildable order\n\n",
         "Sorted by domain; inside a domain so that `depends_on` is satisfied before a\n"
         "row comes up. Prio 1 = Must, 2 = Should, 3 = Could — `Must` before `Should`\n"
         "before `Could` is the order, not a recommendation (prompts/init.md §2).\n\n"]
    ordered = buildable_order(entries)
    for domain in sorted({e["domain"] for e in entries}):
        rows = [e for e in ordered
                if e["domain"] == domain and e["stage"].text != "Accepted"]
        if not rows:
            continue
        z.append(f"## {domain} ({len(rows)} open)\n\n")
        z.append("| Stage | ID | Item | Prio | Depends on | Why here |\n")
        z.append("|---|---|---|---|---|---|\n")
        for e in rows:
            deps = ", ".join(e["depends_on"]) or "—"
            reason = (f"needs {deps}" if e["depends_on"]
                      else {1: "Must, no prerequisite", 2: "Should", 3: "Could"}[e["prio"]])
            z.append(f"| {STAGE_MARK[e['stage'].text]} | {e['id']} | {e['name']} "
                     f"| {e['prio']} | {deps} | {reason} |\n")
        z.append("\n")
    path.write_text("".join(z), encoding="utf-8")


def write_status(path: Path, entries: list[dict]) -> None:
    counts = {k: 0 for k in STAGE_MARK}
    for e in entries:
        counts[e["stage"].text] += 1
    z = [GENERATED_MD_HEADER, "# STATUS — what is implemented and what is not\n\n",
         "Stages: ⬜ unbuilt · 🟨 built (built, untested, unseen) ·\n"
         "🟧 proven (tests that go red + seen in the game) · ✅ accepted "
         "(**only the user sets this**).\n\n",
         "**🟧 needs three pieces of evidence:** a picture (screenshot path), a number "
         "(measured, with the machine `[debian]`/`[cachy]`) and code (a test that goes "
         "red when it breaks). If one is missing it is 🟨 — doubt moves the stage down, "
         "not up (prompts/init.md §8, §9).\n\n",
         f"**Tally:** {counts['Unbuilt']} ⬜ · {counts['Built']} 🟨 · "
         f"{counts['Proven']} 🟧 · {counts['Accepted']} ✅ "
         f"of {len(entries)} rows.\n\n"]
    for domain in sorted({e["domain"] for e in entries}):
        rows = [e for e in entries if e["domain"] == domain]
        rank = {"Accepted": 0, "Proven": 1, "Built": 2, "Unbuilt": 3}
        rows.sort(key=lambda e: (rank[e["stage"].text], e["prio"], e["id"]))
        z.append(f"## {domain}\n\n")
        z.append("| Item | ID | Stage | Evidence (test / screenshot / number) | Note |\n")
        z.append("|---|---|---|---|---|\n")
        for e in rows:
            evidence = e["evidence"] or "—"
            note = e["note"] or "—"
            z.append(f"| {e['name']} | {e['id']} | {STAGE_MARK[e['stage'].text]} "
                     f"| {evidence} | {note} |\n")
        z.append("\n")
    path.write_text("".join(z), encoding="utf-8")


def write_backlog_readme(path: Path, sheet: Sheet, sheets: dict[str, Sheet]) -> None:
    z = ["<!-- GENERATED by tools/features.py from gameplay/features.xlsx, "
         "sheet 00_Anleitung. -->\n\n",
         "# docs/backlog/ — the spreadsheet sheets as data\n\n",
         "One RON per sheet, because the sheets have different columns. The `.xlsx`\n"
         "itself stays where it is, untouched — it is the source, and the user keeps\n"
         "working in it (prompts/init.md §2).\n\n",
         "## What the backlog's own instructions say\n\n"]
    for no in sorted(sheet.rows):
        fields = sheet.rows[no]
        a, b = fields.get("A", ""), fields.get("B", "")
        if a and b:
            z.append(f"- **{a}** — {b}\n")
        elif a:
            z.append(f"\n### {a}\n\n" if len(a) < 60 else f"{a}\n\n")
    z.append("\n## The sheets and their files\n\n")
    z.append("| Sheet | Rows (incl. header) | Records | File |\n|---|---|---|---|\n")
    for name in EXPECTED_ROWS:
        s = sheets[name]
        target = SHEET_TO_RON.get(name)
        file = f"`{target}.ron`" if target else "— (computed, not transferred)"
        count = len(records(s)) if target else "—"
        z.append(f"| `{name}` | {s.row_count} | {count} | {file} |\n")
    z.append("\n**The row count is the guard.** `python3 tools/features.py --check`\n"
             "falls over when a number stops matching — then the extraction is not\n"
             "finished, and you know exactly how many rows are missing, instead of\n"
             "guessing that some are (prompts/init.md §2, §9).\n\n"
             "`docs/features.ron` is the working list built from it: every F-ID from\n"
             "`01_Spielfunktionen` **and** every T-ID from `09_Tech-Backlog`. The T rows\n"
             "are in there because they are exactly what describes the setup — without\n"
             "them `docs/STATUS.md` would have no row for the window, the tools or the\n"
             "tests. That is a named deviation from the table in `prompts/init.md` §2,\n"
             "where `features.ron` is fed from sheet 01 alone.\n")
    path.write_text("".join(z), encoding="utf-8")


# --------------------------------------------------------------------------

def main() -> int:
    check_only = "--check" in sys.argv
    if not XLSX.exists():
        print(f"ERROR: {XLSX} is missing", file=sys.stderr)
        return 2

    sheets = {s.name: s for s in read_workbook(XLSX)}

    # 1. The guard: row count per sheet
    errors = []
    for name, expected in EXPECTED_ROWS.items():
        if name not in sheets:
            errors.append(f"sheet {name} is missing from the workbook")
            continue
        actual = sheets[name].row_count
        if actual != expected:
            errors.append(f"sheet {name}: {actual} rows, expected {expected} "
                          f"(difference {actual - expected})")
    for name in sheets:
        if name not in EXPECTED_ROWS:
            errors.append(f"sheet {name} is new — add it to EXPECTED_ROWS")
    if errors:
        print("ROW-COUNT GUARD FAILED (prompts/init.md §2):", file=sys.stderr)
        for e in errors:
            print("  - " + e, file=sys.stderr)
        return 1

    records_per_sheet = {name: records(s) for name, s in sheets.items()}

    if check_only:
        print(f"{'Sheet':<22} {'Rows':>7} {'Records':>12}")
        for name in EXPECTED_ROWS:
            print(f"{name:<22} {sheets[name].row_count:>7} "
                  f"{len(records_per_sheet[name]):>12}")
        total = sum(len(records_per_sheet[n]) for n in SHEET_TO_RON)
        print(f"{'TOTAL (transferred)':<22} {'':>7} {total:>12}")
        return 0

    # 2. One RON per sheet
    for sheet_name, file in SHEET_TO_RON.items():
        rows = [dict(r) for _, r in records_per_sheet[sheet_name]]
        write_ron(BACKLOG / f"{file}.ron", "entries", rows,
                  f"gameplay/features.xlsx!{sheet_name} "
                  f"({len(rows)} records from "
                  f"{sheets[sheet_name].row_count} rows)")

    write_backlog_readme(BACKLOG / "README.md", sheets["00_Anleitung"], sheets)

    # 3. The working list, with merge
    target = ROOT / "docs" / "features.ron"
    state = read_work_state(target)
    entries = build_features(sheets)
    vanished = merge(entries, state)
    write_ron(target, "features", entries,
              "gameplay/features.xlsx!01_Spielfunktionen + !09_Tech-Backlog")

    # 4. Views
    write_todo(ROOT / "docs" / "TODO.md", entries)
    write_status(ROOT / "docs" / "STATUS.md", entries)

    # 5. Report
    print(f"{'Sheet':<22} {'Rows':>7} {'Records':>12}  File")
    for name in EXPECTED_ROWS:
        file = SHEET_TO_RON.get(name)
        print(f"{name:<22} {sheets[name].row_count:>7} "
              f"{len(records_per_sheet[name]):>12}  "
              f"{'docs/backlog/' + file + '.ron' if file else '(not transferred)'}")
    print(f"\ndocs/features.ron: {len(entries)} entries "
          f"({len(records_per_sheet['01_Spielfunktionen'])} F + "
          f"{len(records_per_sheet['09_Tech-Backlog'])} T)")
    if vanished:
        print("\nATTENTION — rows from the old state are missing in the new .xlsx.")
        print("Record them in docs/QUESTIONS.md, do NOT delete them silently "
              "(init.md §2):")
        for m in vanished:
            print("  - " + m)
    duplicates = [e["id"] for e in entries
                  if [x["id"] for x in entries].count(e["id"]) > 1]
    if duplicates:
        print(f"\nWARNING: duplicate IDs: {sorted(set(duplicates))}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
