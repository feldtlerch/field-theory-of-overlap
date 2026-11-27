"""Controls for source-to-inclusion accounting in the coverage inventory."""

import hashlib
import json
import unittest
from collections import Counter
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import native_coverage


class InclusionTests(unittest.TestCase):
    def test_native_count_requires_a_route(self):
        counts = Counter({("native", "result box: insight"): 1})
        with self.assertRaisesRegex(ValueError, "native members have no route: result box: insight"):
            native_coverage.check_native_routes({"result box: insight": ("none", "")}, counts)
        native_coverage.check_native_routes({"result box: insight": ("subset", "")}, counts)

    def test_substitution_and_exactly_once(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = "source.tex"
            fragment = "fragment.tex"
            (root / fragment).write_text("Generated prose.\n", encoding="utf-8")
            inclusions = {source: {"unit:one": fragment}}

            (root / source).write_text("Before.\n% generated: unit:one\nAfter.\n", encoding="utf-8")
            self.assertEqual(
                list(native_coverage.expanded(source, root, inclusions)),
                [("authored", "Before."), ("native", "Generated prose."), ("authored", "After.")],
            )

            for text, reason in [
                ("% generated: unit:one\n% generated: unit:one\n", "repeated marker"),
                ("Before.\n", "missing inclusion marker"),
                ("% generated: unit:other\n", "unknown or repeated marker"),
            ]:
                with self.subTest(reason=reason):
                    (root / source).write_text(text, encoding="utf-8")
                    with self.assertRaisesRegex(ValueError, reason):
                        list(native_coverage.expanded(source, root, inclusions))

    def test_form_unlock_and_importer_interval(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "sample.tex").write_text(r"\textbf{Ties.} \ftag{one} $\willvirtue{}$" + "\n", encoding="utf-8")
            row = {"document": "explainer", "file": "sample.tex", "line": 1, "last": 1,
                   "origin": "authored", "heading": "", "tabular_cell": False,
                   "text": r"\textbf{Ties.} \ftag{one} $\willvirtue{}$", "reader_words": 1}
            admitted = {"strong", "plain prose only"}
            native_coverage.classify_paragraph(row, root, {}, admitted)
            self.assertEqual(row["blockers"], {"tag/handle", "math: one symbol"})
            self.assertEqual(native_coverage.capability_unlocks([row]), {})

            row["text"] = r"\textbf{Ties.} \ftag{one}"
            native_coverage.classify_paragraph(row, root, {}, admitted)
            self.assertEqual(row["blockers"], {"tag/handle"})
            self.assertEqual(native_coverage.capability_unlocks([row]), {"tag/handle": 1})
            row["blockers"] = frozenset({"structural artefact"})
            self.assertEqual(native_coverage.capability_unlocks([row]), {})
            row["blockers"] = frozenset({"math: one symbol", "math: compound"})
            self.assertEqual(native_coverage.capability_unlocks([row]), {"mathematics run": 1})
            row["blockers"] = frozenset()
            self.assertEqual(native_coverage.slot_summary([row], {"admitted_slot_intervals": [{"first": 2, "last": 3}]}), (1, 0))
            self.assertEqual(native_coverage.slot_summary([row], {"admitted_slot_intervals": [{"first": 1, "last": 1}]}), (1, 1))

    def test_paragraph_boundary_groups_continuation(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = "sample.tex"
            (root / source).write_text("\\section{One}\nLead.\nContinuation.\n\nNext.\n", encoding="utf-8")
            rows = native_coverage.paragraph_rows("paper", source, root, {source: {}})
            self.assertEqual([(row["line"], row["last"]) for row in rows], [(2, 3), (5, 5)])

    def test_wrapper_body_and_nearby_blank_are_not_paragraphs(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = "sample.tex"
            (root / source).write_text("\\refrain{One.}\n\nNext.\n\\formalfoot{Two\ncontinued.}\n\nLast.\n\\[x\\]\n\nAfter.\n", encoding="utf-8")
            rows = native_coverage.paragraph_rows("explainer", source, root, {source: {}})
            self.assertEqual([(row["line"], row["last"]) for row in rows], [(3, 3), (7, 7), (10, 10)])
            roles = {}
            self.assertNotIn("note/refrain adjacency", native_coverage.nearby_forms(rows[1], root, roles))
            self.assertNotIn("display adjacency", native_coverage.nearby_forms(rows[2], root, roles))

    def test_linebreak_is_not_display(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = "sample.tex"
            (root / source).write_text("Title\\\\[0.4ex]\n\\[x\\]\n", encoding="utf-8")
            _coverage, things = native_coverage.inventory(source, root, {source: {}})
            self.assertEqual(things[("authored", "displayed equations")], 1)

    def test_stale_importer_measurement_refuses(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = "documents/paper/explainer/explainer.tex"
            (root / source).parent.mkdir(parents=True)
            (root / source).write_text("Current source.\n", encoding="utf-8")
            snapshot = root / "tools/paper/native_coverage_slot_census.json"
            snapshot.parent.mkdir(parents=True)
            snapshot.write_text(json.dumps({"schema": 2, "source": source,
                                            "source_sha256": "stale", "admitted_slot_intervals": []}), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "does not match"):
                native_coverage.load_slot_census(root)

    def test_importer_file_inventory_cannot_shrink(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = "documents/paper/explainer/explainer.tex"
            listing = root / "documents/paper-build/sources.tsv"
            listing.parent.mkdir(parents=True)
            listing.write_text(source + "\n", encoding="utf-8")
            (root / source).parent.mkdir(parents=True)
            (root / source).write_text("Words.\n", encoding="utf-8")
            content = (root / source).read_bytes()
            snapshot = root / "tools/paper/native_coverage_slot_census.json"
            snapshot.parent.mkdir(parents=True)
            snapshot.write_text(json.dumps({"schema": 2, "source": source,
                "source_sha256": hashlib.sha256(content).hexdigest(),
                "source_list_digest": hashlib.sha256(listing.read_bytes()).hexdigest(),
                "import_context_digest": "context",
                "source_digests": {source: hashlib.sha256(content).hexdigest()},
                "files": {}, "admitted_slot_intervals": []}), encoding="utf-8")
            with patch.object(native_coverage, "import_context_digest", return_value="context"):
                with self.assertRaisesRegex(ValueError, "measured file inventory differs"):
                    native_coverage.load_slot_census(root)


if __name__ == "__main__":
    unittest.main()
