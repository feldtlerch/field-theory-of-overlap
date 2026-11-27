#!/usr/bin/env python3
"""Count literal reader text in document bodies and their generated inclusions.

Word tokens include literal headings, captions, table cells and body prose; they exclude comments,
blank lines, preambles, command names and keys, math notation, reference and citation
labels, and macro expansions. This source inventory differs from the PDF-extracted,
source-corresponded stream compared by compare_words.py. `paper import-dry-run`
counts diagnostic draft blocks, including refusals, rather than admitted inclusions.
Paragraphs are prose runs outside boxes, lists, tables, abstracts and displays; their
forms are literal math, command, typography and adjacent-structure spellings. Forms
overlap, and their eligibility screen is not an importer admission decision. Admitted
slot intervals are read from an importer measurement bound to the explainer source.

Run from the repository root: python3 -B tools/paper/native_coverage.py
"""

from collections import Counter, defaultdict
from pathlib import Path
import hashlib
import json
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]

MARKER = re.compile(r"^% generated: (\S+)\s*$")
COMMAND = re.compile(r"\\([A-Za-z@]+)(\*)?")
BEGIN = re.compile(r"\\begin\{([^}]+)\}")
END = re.compile(r"\\end\{([^}]+)\}")
WORD = re.compile(r"[^\W_]+(?:[-'’][^\W_]+)*", re.UNICODE)
MATH_ENV = {"equation", "equation*", "align", "align*", "gather", "gather*", "multline", "multline*"}
BOX_ENV = {"theorem", "proposition", "lemma", "corollary", "definition", "postulate", "insight", "remark", "statement", "econ", "epist"}
TABLE_ENV = {"table", "table*", "xltabular", "longtable"}
LIST_ENV = {"itemize", "enumerate", "description", "serieslist"}
REQUIRED_KINDS = {
    "headings", "paragraphs", "displayed equations", "inline mathematics", "tables",
    "figures", "lists", "list items", "footnotes", "bibliography citations",
    "cross-references", "tag or handle references", "formal notes", "granting notes",
    "refrains or quoted refrains", "abstracts", "contents", "tag index section",
    "bibliography section",
} | {f"result box: {kind}" for kind in BOX_ENV}
REF_COMMANDS = {"zcref", "see", "papertitlecref", "cref", "Cref", "ref", "eqref", "autoref", "psee"}
TAG_COMMANDS = {"ftag", "ptg", "ptag", "handle", "ftaglist"}
CITE_COMMANDS = {"parencite", "textcite", "cite", "autocite", "footcite"}
FORM_EMPH = {"emph"}
FORM_STRONG = {"textbf"}
FORM_OTHER_STYLE = {"textit", "textsc", "scshape", "normalfont", "itshape", "bfseries"}
FORM_PROSE_MACRO = {"gradeword", "gloss", "virtue", "papervirtue", "registername", "quoted"}
FORM_WRAPPER = {"formalfoot", "auditnote", "refrain", "paperrefrain", "PaperNativeRefrain", "PaperNativeQuotation"}
WRAPPER = re.compile(r"\\(?:formalfoot|auditnote|refrain|paperrefrain|PaperNativeRefrain|PaperNativeQuotation)\b")
FORM_LAYOUT = {"PaperParagraph", "PaperDisplayParagraph", "noindent", "PaperNeedLines"}
FORM_SPACING = {"enspace", "quad", "qquad", "hspace", "thinspace", "smallskip", "medskip", "bigskip"}
FORM_IGNORED = {"PaperNativeParagraph", "label", "secgrade", "grade", "par", "href", "url", "appblock", "gateway", "begin", "end", "vspace", "clearpage", "newpage", "nobreak", "setcounter", "addcontentsline", "hfill", "begingroup", "endgroup", "hypersetup", "tableofcontents", "appendix", "global", "explainerfirstsectiontrue", "subsection", "section", "part", "coda", "textwidth", "linewidth"}
MATH_RUN = re.compile(r"(?<!\\)\$[^$]*\$|\\\(.*?\\\)")
FORM_NAMES = {
    "math: one symbol", "math: compound", "emphasis", "strong", "other style",
    "tag/handle", "other reference", "citation", "prose macro", "note/refrain wrapper",
    "layout hook", "en-space", "other spacing", "TeX quotes/dashes", "escaped punctuation",
    "footnote", "code mention", "title typography", "external link", "heading lead-in/context",
    "structural artefact", "display adjacency", "list adjacency", "note/refrain adjacency",
    "other command", "plain prose only",
}
NO_TEXT_ARG = TAG_COMMANDS | REF_COMMANDS | CITE_COMMANDS | {
    "label", "grade", "secgrade", "gradeword", "hdr", "hdrunbacked", "eqtag",
    "addcontentsline", "setcounter", "setversion", "input", "subfile",
}


def uncomment(line):
    for i, value in enumerate(line):
        if value == "%" and (i == 0 or line[i - 1] != "\\"):
            return line[:i]
    return line


def load_inclusions(root):
    inclusions = defaultdict(dict)
    for row in (root / "generated/documents/inclusions.tsv").read_text(encoding="utf-8").splitlines():
        source, key, fragment, _role, _line = row.split("\t")
        if key in inclusions[source]:
            raise ValueError(f"duplicate inclusion: {source}: {key}")
        inclusions[source][key] = fragment
    return inclusions


def expanded_located(source, root, inclusions):
    seen = set()
    for number, line in enumerate((root / source).read_text(encoding="utf-8").splitlines(), 1):
        match = MARKER.fullmatch(line)
        if match:
            key = match.group(1)
            if key not in inclusions[source] or key in seen:
                raise ValueError(f"unknown or repeated marker: {source}: {key}")
            seen.add(key)
            fragment = inclusions[source][key]
            for inner, part in enumerate((root / fragment).read_text(encoding="utf-8").splitlines(), 1):
                yield "native", fragment, inner, part
        else:
            yield "authored", source, number, line
    missing = set(inclusions[source]) - seen
    if missing:
        raise ValueError(f"missing inclusion marker: {source}: {', '.join(sorted(missing))}")


def expanded(source, root, inclusions):
    for origin, _file, _number, line in expanded_located(source, root, inclusions):
        yield origin, line


def visible(line, math=False):
    """Keep literal words, not keys, command names, options or math notation."""
    if math:
        return ""
    if re.match(r"\s*\\(?:documentclass|addcontentsline|tableofcontents|input|subfile|setcounter)\b", line):
        return ""
    line = re.sub(r"\$[^$]*\$|\\\([^)]*\\\)|\\\[[^]]*\\\]", " ", line)
    line = re.sub(r"\\begin\{(?:tabularx|tabular|xltabular|longtable)\}.*", " ", line)
    line = re.sub(r"\\begin\{statement\}\{[^}]*\}", " ", line)
    line = re.sub(r"\\begin\{[^}]+\}(?:\[[^]]*\])?", " ", line)
    line = END.sub(" ", line)
    line = re.sub(r"\\(?:PaperNativeParagraph|PaperNativeEquation|refrain|paperrefrain)\{[^}]*\}", " ", line)
    line = re.sub(r"\\[A-Za-z@]+\*?\{[^{}]*\}",
                  lambda m: " " if re.match(r"\\([A-Za-z@]+)", m.group()).group(1) in NO_TEXT_ARG else m.group(), line)
    line = re.sub(r"\\[A-Za-z@]+\*?", " ", line)
    line = re.sub(r"\\[^A-Za-z\s]", " ", line)
    line = re.sub(r"\b(?:stretch|wholewords|lastline|rightskip|label|cites|departs|stopsat)\s*=\s*[^,}]+", " ", line)
    return line.replace("{", " ").replace("}", " ").replace("&", " ")


def brace_delta(line):
    delta = 0
    slashes = 0
    for char in line:
        if char == "\\":
            slashes += 1
            continue
        if char in "{}" and slashes % 2 == 0:
            delta += 1 if char == "{" else -1
        slashes = 0
    return delta


def paragraph_rows(document, source, root, inclusions):
    """Use the same prose-run boundary for the object count and form inventory."""
    rows = []
    stack = []
    paragraph_origin = None
    current = None
    prior_boundary = "start"
    prior_heading = ""
    wrapper_depth = 0
    for origin, file, number, raw in expanded_located(source, root, inclusions):
        line = uncomment(raw)
        if re.match(r"\s*\\documentclass\b", line):
            continue
        if wrapper_depth:
            wrapper_depth += brace_delta(line)
            paragraph_origin = None
            current = None
            prior_boundary = "wrapper"
            continue
        if WRAPPER.search(line):
            wrapper_depth = max(0, brace_delta(line))
            paragraph_origin = None
            current = None
            prior_boundary = "wrapper"
            continue
        begin, end = BEGIN.findall(line), END.findall(line)
        inside_math = any(item in MATH_ENV for item in stack)
        words = WORD.findall(visible(line, inside_math or any(item in MATH_ENV for item in begin)))
        excluded = any(item in BOX_ENV | TABLE_ENV | LIST_ENV | MATH_ENV | {"abstract"} for item in stack)
        tabular_cell = any(item in {"tabular", "tabularx"} for item in stack)
        special = bool(begin or end or re.search(r"\\(?:part|coda|section|subsection|subsubsection|paragraph|refrain|paperrefrain|formalfoot|PaperNativeEquation)\b|(?<!\\)\\\[", line))
        native_paragraph = r"\PaperNativeParagraph{" in line
        boundary = "display" if any(item in MATH_ENV for item in begin + end) else (
            "list" if any(item in LIST_ENV for item in begin + end) else (
            "box/table" if any(item in BOX_ENV | TABLE_ENV for item in begin + end) else (
            "heading" if re.search(r"\\(?:part|coda|section|subsection|subsubsection|paragraph)\b", line) else "other")))
        if native_paragraph:
            rows.append({"document": document, "file": file, "line": number, "last": number,
                         "origin": origin, "before": prior_boundary, "heading": prior_heading,
                         "tabular_cell": tabular_cell, "text": line, "reader_words": len(words)})
        if not line.strip() or special or excluded or native_paragraph:
            paragraph_origin = None
            current = None
            if not line.strip():
                prior_boundary = "blank"
            elif special or excluded:
                prior_boundary = boundary
                if boundary == "heading":
                    heading = re.search(r"\\(part|coda|section|subsection|subsubsection|paragraph)\b", line)
                    prior_heading = heading.group(1) if heading else ""
        elif words and paragraph_origin != origin:
            current = {"document": document, "file": file,
                       "line": number, "last": number,
                       "origin": origin, "before": prior_boundary, "heading": prior_heading,
                       "tabular_cell": tabular_cell, "text": line, "reader_words": len(words)}
            rows.append(current)
            paragraph_origin = origin
            prior_heading = ""
        elif current is not None and paragraph_origin == origin:
            current["text"] += " " + line
            current["reader_words"] += len(words)
            current["last"] = number
        stack.extend(begin)
        for env in end:
            if env in stack:
                stack.remove(env)
    return rows


def math_form(expression):
    body = expression[1:-1] if expression.startswith("$") else expression[2:-2]
    if re.fullmatch(r"[A-Za-z0-9]+|\\[A-Za-z@]+(?:\{[A-Za-z0-9,: -]*\})*", body.strip()):
        return "math: one symbol"
    return "math: compound"


def nearby_forms(row, root, marker_roles):
    if row["origin"] == "native":
        return set()
    lines = (root / row["file"]).read_text(encoding="utf-8").splitlines()
    forms = set()
    for index, step in ((row["line"] - 2, -1), (row["last"], 1)):
        while 0 <= index < len(lines) and (
            lines[index].lstrip().startswith("%") and (row["file"], index + 1) not in marker_roles
        ):
            index += step
        if 0 <= index < len(lines) and lines[index].strip():
            role = marker_roles.get((row["file"], index + 1))
            if role == "equation":
                forms.add("display adjacency")
            if role in {"quotation", "refrain"}:
                forms.add("note/refrain adjacency")
            name = re.search(r"\\(?:begin|end)\{([^}]+)\}", lines[index])
            if name and name.group(1) in MATH_ENV:
                forms.add("display adjacency")
            elif name and name.group(1) in LIST_ENV:
                forms.add("list adjacency")
    return forms


def classify_paragraph(row, root, marker_roles, admitted_forms):
    """Classify literal source forms; eligibility is a form screen, not admission."""
    source = row["text"]
    forms = {math_form(item) for item in MATH_RUN.findall(source)}
    outside = MATH_RUN.sub(" MATH ", source)
    outside = re.sub(r"\\verb([^A-Za-z\s]).*?\1", " CODE ", outside)
    commands = {match.group(1) for match in COMMAND.finditer(outside)}
    for names, form in ((FORM_EMPH, "emphasis"), (FORM_STRONG, "strong"),
                        (FORM_OTHER_STYLE, "other style"), (TAG_COMMANDS, "tag/handle"),
                        (REF_COMMANDS, "other reference"), (CITE_COMMANDS, "citation"),
                        (FORM_PROSE_MACRO, "prose macro"), (FORM_WRAPPER, "note/refrain wrapper"),
                        (FORM_LAYOUT, "layout hook")):
        if commands & names:
            forms.add(form)
    if "enspace" in commands:
        forms.add("en-space")
    if commands & (FORM_SPACING - {"enspace"}) or re.search(r"\\(?:[,;!: ]|hspace)", outside):
        forms.add("other spacing")
    if re.search(r"(?<!-)--(?!-)|---|``|''", outside):
        forms.add("TeX quotes/dashes")
    if re.search(r"\\(?:%|#|&|_)", outside):
        forms.add("escaped punctuation")
    if re.search(r"\\(?:footnote|footnotetext)\b", outside):
        forms.add("footnote")
    if "verb" in commands or "texttt" in commands or " CODE " in outside:
        forms.add("code mention")
    if commands & {"LARGE", "docdraft"}:
        forms.add("title typography")
    if commands & {"href", "url"}:
        forms.add("external link")
    if commands & {"appblock", "gateway", "subsection"} or row["heading"] in {"subsection", "subsubsection", "paragraph"}:
        forms.add("heading lead-in/context")
    if commands & {"tableofcontents", "hypersetup"} or row["tabular_cell"]:
        forms.add("structural artefact")
    forms.update(nearby_forms(row, root, marker_roles))
    known = (FORM_EMPH | FORM_STRONG | FORM_OTHER_STYLE | REF_COMMANDS | TAG_COMMANDS |
             CITE_COMMANDS | FORM_PROSE_MACRO | FORM_WRAPPER | FORM_LAYOUT | FORM_SPACING |
             FORM_IGNORED | {"verb", "texttt", "footnote", "footnotetext", "LARGE", "docdraft"})
    if commands - known:
        forms.add("other command")
    if not forms:
        forms.add("plain prose only")
    row["forms"] = frozenset(forms)
    row["blockers"] = frozenset(forms - admitted_forms)
    return row


def inventory(source, root, inclusions):
    coverage = Counter()
    things = Counter()
    stack = []
    for origin, raw in expanded(source, root, inclusions):
        line = uncomment(raw)
        if re.match(r"\s*\\documentclass\b", line):
            continue
        begin = BEGIN.findall(line)
        end = END.findall(line)
        inside_math = any(x in MATH_ENV for x in stack)
        display_start = any(x in MATH_ENV for x in begin)
        text = visible(line, inside_math or display_start)
        words = WORD.findall(text)
        math_body = inside_math and bool(re.search(r"[A-Za-z0-9=+<>]", line)) and not end and not re.fullmatch(r"\s*\\label\{[^}]+\}\s*", line)
        content_line = bool(words or math_body or r"\PaperNativeEquation{" in line)
        if content_line:
            coverage[f"{origin}_lines"] += 1
        coverage[f"{origin}_words"] += len(words)

        for env in begin:
            if env in BOX_ENV:
                things[(origin, f"result box: {env}")] += 1
            elif env in TABLE_ENV and not any(x in TABLE_ENV for x in stack):
                things[(origin, "tables")] += 1
            elif env in LIST_ENV:
                things[(origin, "lists")] += 1
            elif env in MATH_ENV:
                things[(origin, "displayed equations")] += 1
            elif env in {"figure", "figure*"}:
                things[(origin, "figures")] += 1
            elif env == "abstract":
                things[(origin, "abstracts")] += 1
            elif env == "granting":
                things[(origin, "granting notes")] += 1
            stack.append(env)
        for env in end:
            if env in stack:
                stack.remove(env)
        for match in COMMAND.finditer(line):
            command = match.group(1)
            if command in {"part", "coda", "section", "subsection", "subsubsection", "paragraph"}:
                things[(origin, "headings")] += 1
            elif command in CITE_COMMANDS:
                things[(origin, "bibliography citations")] += 1
            elif command in REF_COMMANDS:
                things[(origin, "cross-references")] += 1
            elif command in TAG_COMMANDS:
                things[(origin, "tag or handle references")] += 1
            elif command in {"footnote", "footnotetext"}:
                things[(origin, "footnotes")] += 1
            elif command == "formalfoot":
                things[(origin, "formal notes")] += 1
            elif command in {"refrain", "paperrefrain"}:
                things[(origin, "refrains or quoted refrains")] += 1
            elif command in {"item", "ritem", "bitem", "titem"}:
                things[(origin, "list items")] += 1
            elif command == "PaperNativeEquation":
                things[(origin, "displayed equations")] += 1
            elif command == "includegraphics" and not any(x in {"figure", "figure*"} for x in stack):
                things[(origin, "figures")] += 1
            elif command == "tableofcontents":
                things[(origin, "contents")] += 1
        if re.search(r"(?<!\\)\\\[", line) and not inside_math and not display_start and r"\PaperNativeEquation{" not in line:
            things[(origin, "displayed equations")] += 1
        if not inside_math and not display_start:
            inline = re.sub(r"\\\[[\s\S]*?\\\]", "", line)
            things[(origin, "inline mathematics")] += len(re.findall(r"(?<!\\)\$(?!\$)[^$]+\$|\\\([^)]*\\\)", inline))
    document = "explainer" if source.endswith("/explainer.tex") else "paper"
    for row in paragraph_rows(document, source, root, inclusions):
        things[(row["origin"], "paragraphs")] += 1
    return coverage, things


def load_capabilities(root):
    rows = (root / "tools/paper/native_coverage_capabilities.tsv").read_text(encoding="utf-8").splitlines()
    if rows.pop(0) != "kind\tstatus\treason":
        raise ValueError("capability header changed")
    capabilities = {}
    for row in rows:
        kind, status, reason = row.split("\t")
        allowed = {"yes", "not yet"} if kind.startswith("paragraph form: ") else {"none", "subset", "all"}
        if kind in capabilities or status not in allowed:
            raise ValueError(f"invalid capability row: {row}")
        capabilities[kind] = (status, reason)
    expected = REQUIRED_KINDS | {f"paragraph form: {form}" for form in FORM_NAMES}
    if set(capabilities) != expected:
        raise ValueError(f"capability kinds differ: missing={sorted(expected - set(capabilities))}, extra={sorted(set(capabilities) - expected)}")
    return ({kind: capabilities[kind] for kind in REQUIRED_KINDS},
            {form: capabilities[f"paragraph form: {form}"] for form in FORM_NAMES})


def marker_roles(root):
    roles = {}
    for line in (root / "generated/documents/inclusions.tsv").read_text(encoding="utf-8").splitlines():
        source, _unit, _fragment, role, number = line.split("\t")
        roles[(source, int(number))] = role
    return roles


def import_context_digest(root):
    """Bind the interval measurement to its declarations and production readers."""
    names = ["crates/ftoo_codegen/src/documents/input.rs", "crates/ftoo_codegen/src/units/corpus.rs",
             "crates/ftoo_manifest/src/units/corpus.rs", "units/documents.toml"]
    for directory in ("crates/ftoo_codegen/src/units/corpus", "crates/ftoo_manifest/src/units/corpus"):
        names.extend(str(path.relative_to(root)) for path in (root / directory).rglob("*.rs") if "tests" not in path.parts)
    names.extend(str(path.relative_to(root)) for path in (root / "units").rglob("*.toml"))
    digest = hashlib.sha256()
    for name in sorted(set(names)):
        data = (root / name).read_bytes()
        digest.update(name.encode("utf-8") + b"\0" + str(len(data)).encode("ascii") + b"\0" + data)
    return digest.hexdigest()


def load_slot_census(root):
    """Read importer-measured intervals; this function does not decide admission."""
    path = root / "tools/paper/native_coverage_slot_census.json"
    census = json.loads(path.read_text(encoding="utf-8"))
    source = "documents/paper/explainer/explainer.tex"
    digest = hashlib.sha256((root / source).read_bytes()).hexdigest()
    if census["schema"] != 2 or census["source"] != source or census["source_sha256"] != digest:
        raise ValueError("slot census does not match the explainer source")
    if census["source_list_digest"] != hashlib.sha256((root / "documents/paper-build/sources.tsv").read_bytes()).hexdigest():
        raise ValueError("slot census does not match the source list")
    if census["import_context_digest"] != import_context_digest(root):
        raise ValueError("slot census does not match importer declarations or readers")
    listed = {line for line in (root / "documents/paper-build/sources.tsv").read_text(encoding="utf-8").splitlines()
              if line.startswith("documents/paper/parts/") or line == source}
    if set(census["source_digests"]) != listed:
        raise ValueError("slot census source inventory differs")
    if set(census["files"]) != listed:
        raise ValueError("slot census measured file inventory differs")
    for name, expected in census["source_digests"].items():
        if hashlib.sha256((root / name).read_bytes()).hexdigest() != expected:
            raise ValueError(f"slot census does not match source: {name}")
    return census


def update_slot_census(root, residue_path):
    """Retain the importer's exact admitted intervals with their source identity."""
    residue = json.loads(residue_path.read_text(encoding="utf-8"))
    draft = json.loads((residue_path.parent / "draft.json").read_text(encoding="utf-8"))
    if residue["schema"] != 1 or residue["accounted_bytes"] != residue["source_bytes"]:
        raise ValueError("importer residue is incomplete")
    if draft["schema"] != 1 or draft["source_list_digest"] != residue["source_list_digest"]:
        raise ValueError("importer draft and residue differ")
    if residue["source_list_digest"] != hashlib.sha256((root / "documents/paper-build/sources.tsv").read_bytes()).hexdigest():
        raise ValueError("importer source list changed")
    for row in draft["sources"]:
        if row["digest"] != hashlib.sha256((root / row["path"]).read_bytes()).hexdigest():
            raise ValueError(f"importer source changed: {row['path']}")
    source = "documents/paper/explainer/explainer.tex"
    files = [row for row in residue["files"] if row["path"] == source]
    drafts = [row for row in draft["sources"] if row["path"] == source]
    digest = hashlib.sha256((root / source).read_bytes()).hexdigest()
    if len(files) != 1 or len(drafts) != 1 or drafts[0]["digest"] != digest:
        raise ValueError("importer explainer source is missing or changed")
    data = {"schema": 2, "source": source, "source_sha256": digest,
            "source_list_digest": residue["source_list_digest"],
            "import_context_digest": import_context_digest(root),
            "source_digests": {row["path"]: row["digest"] for row in draft["sources"]},
            "files": {row["path"]: {"kinds": {kind: {"decoded": counts["fully_decoded"], "refused": counts["refused"]}
                                                        for kind, counts in row["kinds"].items()},
                                   "refusal_kinds": row["refusal_kinds"]}
                      for row in residue["files"]},
            "admitted_slot_intervals": files[0]["admitted_slot_intervals"]}
    path = root / "tools/paper/native_coverage_slot_census.json"
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def measure(root):
    main = (root / "documents/paper/main.tex").read_text(encoding="utf-8")
    paper = [f"documents/paper/{part}.tex" for part in re.findall(r"\\subfile\{(parts/[^}]+)\}", main)]
    documents = {"paper": paper, "explainer": ["documents/paper/explainer/explainer.tex"]}
    inclusions = load_inclusions(root)
    body_sources = set(paper + documents["explainer"])
    if set(inclusions) - body_sources:
        raise ValueError(f"inclusion outside counted bodies: {sorted(set(inclusions) - body_sources)}")
    results = {source: inventory(source, root, inclusions) for files in documents.values() for source in files}
    if any(main.count(command) != 1 for command in (
        r"\input{data/ftoo_index.tex}", r"\printbibliography[heading=none]",
        r"\part*{The index of the tag namespace}", r"\part*{References}"
    )):
        raise ValueError("paper back-matter shape changed")
    composition = Counter({
        ("authored", "headings"): 2,
        ("authored", "tag index section"): 1,
        ("authored", "bibliography section"): 1,
    })
    coverage = []
    for document, files in [*documents.items(), ("all", paper + documents["explainer"])]:
        for source in ([*files, None] if document != "all" else [None]):
            counts = sum((results[file][0] for file in ([source] if source else files)), Counter())
            nl, al = counts["native_lines"], counts["native_lines"] + counts["authored_lines"]
            nw, aw = counts["native_words"], counts["native_words"] + counts["authored_words"]
            coverage.append((document, source or "TOTAL", nl, al, nw, aw))
    capabilities, form_capabilities = load_capabilities(root)
    things = {}
    for document, files in [*documents.items(), ("all", paper + documents["explainer"])]:
        counts = sum((results[file][1] for file in files), Counter())
        if document in {"paper", "all"}:
            counts.update(composition)
        things[document] = counts
    check_native_routes(capabilities, things["all"])
    rows = [row for document, files in documents.items() for source in files
            for row in paragraph_rows(document, source, root, inclusions)]
    roles = marker_roles(root)
    admitted_forms = {form for form, (status, _) in form_capabilities.items() if status == "yes"}
    rows = [classify_paragraph(row, root, roles, admitted_forms) for row in rows]
    for document, files in documents.items():
        for origin in ("native", "authored"):
            expected = things[document][(origin, "paragraphs")]
            actual = sum(row["document"] == document and row["origin"] == origin for row in rows)
            if expected != actual:
                raise ValueError(f"paragraph count differs: {document}: {origin}: {expected} != {actual}")
    census = load_slot_census(root)
    return coverage, things, capabilities, rows, form_capabilities, census


def check_native_routes(capabilities, counts):
    for kind, (status, _) in capabilities.items():
        if status == "none" and counts[("native", kind)]:
            raise ValueError(f"native members have no route: {kind}")


def percent(part, whole):
    return f"{100 * part / whole:.2f}%" if whole else "0.00%"


def form_summary(paragraphs, form_capabilities):
    summary = {}
    for form in form_capabilities:
        members = [row for row in paragraphs if form in row["forms"]]
        summary[form] = {
            "paper": sum(row["document"] == "paper" for row in members),
            "explainer": sum(row["document"] == "explainer" for row in members),
            "total": len(members),
        }
    return summary


def capability_unlocks(paragraphs):
    """Count explainer prose opened by one missing corpus-slot capability."""
    groups = {"math: one symbol": "mathematics run", "math: compound": "mathematics run"}
    summary = Counter()
    for row in paragraphs:
        if row["document"] != "explainer" or row["origin"] != "authored":
            continue
        blockers = {groups.get(form, form) for form in row["blockers"]}
        if len(blockers) == 1 and "structural artefact" not in blockers:
            summary[next(iter(blockers))] += 1
    return summary


def slot_summary(paragraphs, census):
    intervals = census["admitted_slot_intervals"]
    if any(interval["first"] > interval["last"] for interval in intervals):
        raise ValueError("invalid importer interval")
    eligible = [row for row in paragraphs if row["document"] == "explainer"
                and row["origin"] == "authored" and not row["blockers"]]
    in_whole_stretch = [row for row in eligible if any(
        interval["first"] <= row["line"] and row["last"] <= interval["last"]
        for interval in intervals)]
    return len(eligible), len(in_whole_stretch)


def render_tsv(coverage, things, capabilities, paragraphs, form_capabilities, census):
    rows = ["coverage\tdocument\tsource\tnative_words\tall_words\tword_pct"]
    for document, source, _nl, _al, nw, aw in coverage:
        rows.append(f"coverage\t{document}\t{source}\t{nw}\t{aw}\t{percent(nw, aw)}")
    for document, counts in things.items():
        rows.append(f"things-{document}\tkind\tnative\tall\tcapability\treason")
        for kind in sorted(capabilities):
            native = counts[("native", kind)]
            status, reason = capabilities[kind]
            rows.append(f"things-{document}\t{kind}\t{native}\t{native + counts[('authored', kind)]}\t{status}\t{reason}")
    summaries = form_summary(paragraphs, form_capabilities)
    rows.append("paragraph-forms\tform\tcorpus-slot run\tpaper\texplainer\ttotal")
    for form in sorted(summaries):
        item = summaries[form]
        rows.append(f"paragraph-forms\t{form}\t{form_capabilities[form][0]}\t{item['paper']}\t{item['explainer']}\t{item['total']}")
    eligible, in_stretch = slot_summary(paragraphs, census)
    rows.append(f"slot-stretch\t{eligible}\t{in_stretch}")
    return "\n".join(rows) + "\n"


def blob_id(path):
    return subprocess.check_output(["git", "hash-object", "--stdin"], input=path.read_bytes(), cwd=ROOT).decode("ascii").strip()


def render_markdown(root, coverage, things, capabilities, paragraphs, form_capabilities, census):
    paper_source_count = sum(source != 'TOTAL' and document == 'paper'
                             for document, source, *_ in coverage)
    body_source_count = sum(source != 'TOTAL' for _, source, *_ in coverage)
    lines = [
        "# Native document coverage", "",
        "[ADR 030](../adr/030-native-renderings-complete-graph-native-model.md) requires every paper and explainer passage to come from one native source for PDF, Markdown and HTML, and every rendered table to have a native body in each medium by release 0.6.0; a missing-content placeholder does not meet that boundary.", "",
        "The identities below name measured file bytes, independent of the commit that contains them.", "",
        f"The {paper_source_count} paper body files selected by `main.tex` and the explainer body are counted with each checked inclusion marker replaced by its generated fragment. Native means content from those inclusions; the remaining body TeX is authored. The paper's derived index and bibliography sections are counted as objects, but their entries are outside the body-word denominator.", "",
        "## Classes", "",
        "Each member is an occurrence of its class. Inline forms and objects may be nested, so class rows do not sum to a document block count. Native route names whether a checked route carries none, a subset, or all member forms; native counts measure inclusions in the body sources.", "",
        "| Class | Paper native / members | Explainer native / members | All native / members | Native route | Basis |",
        "|---|---:|---:|---:|---|---|",
    ]
    class_rows = []
    paper, explainer, all_counts = (things[name] for name in ("paper", "explainer", "all"))
    for kind, (status, reason) in capabilities.items():
        native = all_counts[("native", kind)]
        total = native + all_counts[("authored", kind)]
        class_rows.append((kind, status, reason, native, total))
    for kind, status, reason, native, total in sorted(class_rows, key=lambda item: (-(item[4] - item[3]), -item[4], item[0])):
        def cell(counts):
            own = counts[("native", kind)]
            return f"{own} / {own + counts[('authored', kind)]}"
        lines.append(f"| {kind} | {cell(paper)} | {cell(explainer)} | {native} / {total} | {status} | {reason} |")
    lines.extend(["", "## Paragraph forms", "",
        "A paragraph is a lexical prose run outside boxes, notes, refrains, lists, tables, abstracts and displays. Forms overlap within a paragraph. A lexical one-symbol mathematics form is not a claim of mathematical simplicity. Title and setup text can still appear as structural artefacts. The corpus-slot run column describes only the bounded importer route, not other native paragraph units.", "",
        "| Form | Paper members | Explainer members | All members | Corpus-slot run (`dec:documents:bounded-corpus-slot`) | Basis |",
        "|---|---:|---:|---:|---|---|"])
    summaries = form_summary(paragraphs, form_capabilities)
    for form, item in sorted(summaries.items(), key=lambda entry: (-entry[1]["total"], entry[0])):
        status, reason = form_capabilities[form]
        lines.append(f"| {form} | {item['paper']} | {item['explainer']} | {item['total']} | {status} | {reason} |")
    lines.extend(["", "## Explainer capability unlocks", "",
        "A capability unlock counts an authored explainer paragraph whose only missing detected capability is that row. The two mathematics forms share one mathematics-run capability. This lexical screen does not prove importer admission or transfer of the surrounding source.", "",
        "| Missing capability | Authored paragraphs opened |", "|---|---:|"])
    for capability, count in sorted(capability_unlocks(paragraphs).items(), key=lambda item: (-item[1], item[0])):
        lines.append(f"| {capability} | {count} |")
    lines.extend(["",
        "| Document | Paragraph members | Native | Using only admitted forms | Authored using only admitted forms |",
        "|---|---:|---:|---:|---:|"])
    for document in ("paper", "explainer", "all"):
        selected = [row for row in paragraphs if document == "all" or row["document"] == document]
        lines.append(f"| {document} | {len(selected)} | {sum(row['origin'] == 'native' for row in selected)} | {sum(not row['blockers'] for row in selected)} | {sum(row['origin'] == 'authored' and not row['blockers'] for row in selected)} |")
    eligible, in_stretch = slot_summary(paragraphs, census)
    lines.extend(["", f"Of the {eligible} authored explainer paragraphs using only admitted forms, {in_stretch} lie wholly within admissible intervals from the [importer's dry run](../adr/053-whole-corpus-import.md). This retained snapshot lists explainer intervals; the importer source report enumerates all {body_source_count} body files.", "",
        "## Importer blocks and refusals", "",
        "The importer owns block splitting and diagnostic refusal counts. Its counts below differ from the lexical paragraph and word measures above; neither measure substitutes for the other.", "",
        "| Kind | Fully decoded | Refused |", "|---|---:|---:|"])
    kinds = defaultdict(Counter)
    refusals = Counter()
    for file in census["files"].values():
        for kind, counts in file["kinds"].items():
            kinds[kind].update(counts)
        refusals.update(file["refusal_kinds"])
    for kind, counts in sorted(kinds.items()):
        lines.append(f"| {kind} | {counts['decoded']} | {counts['refused']} |")
    lines.extend(["", "| First refusal kind | Outer blocks |", "|---|---:|"])
    for kind, count in sorted(refusals.items(), key=lambda item: (-item[1], item[0])):
        name = json.dumps(kind, ensure_ascii=False)[1:-1].replace("|", "&#124;").replace("`", "\\u0060")
        lines.append(f"| `{name}` | {count} |")
    lines.extend(["", "| Explainer admissible interval | Stopping condition |", "|---|---|"])
    for interval in census["admitted_slot_intervals"]:
        lines.append(f"| {interval['first']}–{interval['last']} | `{interval['stopping_condition']}` |")
    lines.extend(["",
        "## Coverage by reader words", "",
        "| Document | Native words / all | Word share |",
        "|---|---:|---:|"])
    for document, source, _nl, _al, nw, aw in coverage:
        if source == "TOTAL":
            lines.append(f"| {document} | {nw} / {aw} | {percent(nw, aw)} |")
    lines.extend(["", "## Per-file detail", "",
        "Word tokens count literal headings, captions, table cells and prose; they exclude comments, blank lines, preambles, command names and keys, math notation, reference and citation labels, and macro expansions. Multi-line mathematics and TeX-produced prose can escape this lexical count.", "",
        "| Document | Source | Native words / all | Word share | Source blob |",
        "|---|---|---:|---:|---|"])
    for document, source, nl, al, nw, aw in coverage:
        name = f"[`{source}`](../../{source})" if source != "TOTAL" else "Total"
        identity = f"`{blob_id(root / source)}`" if source != "TOTAL" else "—"
        lines.append(f"| {document} | {name} | {nw} / {aw} | {percent(nw, aw)} | {identity} |")
    lines.extend([
        "", "## Limits and regeneration", "",
        "This source inventory cannot determine whether native text is still compared with retained wording, what a TeX macro expands into as prose, the printed words of references and citations, or whether a rendered link or object is correct. Mathematical annotations and derived index and bibliography entry text are not body words here. The form classifier reads lexical spellings and nearby boundaries; it cannot prove importer admission, source correspondence or complete rendering. It is not the PDF-extracted word comparison.", "",
        "A later inventory can take its block and form boundaries directly from the importer's residue and leave only word share to the lexical reader. That would remove the present duplicate boundary interpretation while retaining the source-word measure the importer does not provide.", "",
        "Input identities are Git blob IDs of the measured bytes. The importer source-list digest is SHA-256; its source digests in the snapshot bind the diagnostic measurement to all twelve body sources.", "",
        "| Input | Blob ID |", "|---|---|"])
    inputs = ["documents/paper/main.tex", "documents/paper-build/sources.tsv",
              "generated/documents/inclusions.tsv", "tools/paper/native_coverage_capabilities.tsv",
              "tools/paper/native_coverage_slot_census.json"]
    fragments = sorted({row.split("\t")[2] for row in (root / "generated/documents/inclusions.tsv").read_text(encoding="utf-8").splitlines()})
    for name in inputs + fragments:
        lines.append(f"| `{name}` | `{blob_id(root / name)}` |")
    lines.extend(["", f"Importer source-list digest: `{census['source_list_digest']}`.", "",
        "Regenerate this page with `python3 -B tools/paper/native_coverage.py`. The maintained capability judgments are in `tools/paper/native_coverage_capabilities.tsv`. After a body-source edit, run `paper import-dry-run --repository ROOT --output-dir DIR`, then `python3 -B tools/paper/native_coverage.py --import-residue DIR/residue.json`. The snapshot records the importer's measured sources and intervals; the page is then regenerated and checked with the other generated documents.", "",
    ])
    return "\n".join(lines)


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "--import-residue":
        update_slot_census(ROOT, Path(sys.argv[2]))
    elif sys.argv[1:] not in ([], ["--tsv"], ["--check"]):
        raise SystemExit("usage: native_coverage.py [--check | --tsv | --import-residue RESIDUE]")
    coverage, things, capabilities, paragraphs, form_capabilities, census = measure(ROOT)
    if sys.argv[1:] == ["--tsv"]:
        sys.stdout.write(render_tsv(coverage, things, capabilities, paragraphs, form_capabilities, census))
        return
    destination = ROOT / "docs/status/native-coverage.md"
    content = render_markdown(ROOT, coverage, things, capabilities, paragraphs, form_capabilities, census).encode("utf-8")
    if sys.argv[1:] == ["--check"]:
        if not destination.exists() or destination.read_bytes() != content:
            raise SystemExit("native coverage page differs from measured inputs")
        print("native coverage page matches measured inputs")
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(content)
    print(destination.relative_to(ROOT))


if __name__ == "__main__":
    main()
