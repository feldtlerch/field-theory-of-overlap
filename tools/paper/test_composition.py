"""Controls for checked inclusions and source markers."""
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch
from prepare import scope_fragments, declared_fragments, apply_fragment_replacements
from identities import input_paths
import meson_adapter

FRAGMENTS = tuple(row[:4] for row in scope_fragments(pathlib.Path(__file__).resolve().parents[2]))


def checked_fragments(root, rows):
    from identities import CapturedInputs, assembly_json, bindings as real_bindings, virtue_roster
    repository = pathlib.Path(__file__).resolve().parents[2]
    roster = virtue_roster(repository, real_bindings(repository), CapturedInputs(repository))
    bridge = assembly_json(repository)
    with (patch('identities.bindings') as bindings,
          patch('prepare.assembly_json', return_value=bridge),
          patch('prepare.bindings') as fragment_bindings,
          patch('prepare.virtue_roster', return_value=roster)):
        bindings.return_value.get.side_effect = lambda name: {
            'source_status': 'paper', 'spelling': name}
        fragment_bindings.return_value = real_bindings(repository)
        return declared_fragments(root, rows)


class Composition(unittest.TestCase):
    def test_adapter_import_does_not_require_inclusions(self):
        repository = pathlib.Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            scripts = root/'tools/paper'
            scripts.mkdir(parents=True)
            for source in (repository/'tools/paper').glob('*.py'):
                shutil.copyfile(source, scripts/source.name)
            artifact = root/'generated/documents/inclusions.tsv'
            for data in (None, b'<<<<<<< merge\n'):
                if data is not None:
                    artifact.parent.mkdir(parents=True, exist_ok=True)
                    artifact.write_bytes(data)
                result = subprocess.run([sys.executable, '-B', str(scripts/'meson_adapter.py'),
                                         '--help'], capture_output=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_paper_record_ignores_skeleton_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            skeleton = root/'units/skeletons/main.toml'
            skeleton.parent.mkdir(parents=True)
            skeleton.write_text('first')
            args = SimpleNamespace(repository=root, support=str(root/'support'),
                                   output=root/'paper-record.json')
            with (patch.object(meson_adapter, 'inventory', return_value={}),
                  patch.object(meson_adapter, 'python_closure', return_value={}),
                  patch.object(meson_adapter, 'tool', return_value={}),
                  patch.object(meson_adapter, 'pdf_libraries', return_value={})):
                meson_adapter.paper_record(args)
                before = args.output.read_bytes()
                skeleton.write_text('second')
                meson_adapter.paper_record(args)
                self.assertEqual(args.output.read_bytes(), before)

    def test_scope_change_invalidates_documents_record(self):
        repository = pathlib.Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            fixed = ['units/documents.toml', 'registry/documentary-tags.toml',
                     'generated/manifest.json', 'generated/notation.tex',
                     'generated/notation-status.json', 'generated/native-paper-support.tex',
                     'generated/addressing.tex', 'tools/assembly/native-lock.toml',
                     'tools/assembly/prepare-native.py', 'tools/notation/package-lock.json',
                     'tools/notation/package.json', 'tools/notation/support-digests.json',
                     'tools/notation/.node-version', 'tools/notation/check.cjs',
                     'documents/paper-build/label-mints.tsv',
                     'documents/paper-build/fontconfig.conf']
            for folder in ('units/statements', 'units/skeletons', 'units/scopes', 'units/displays'):
                fixed.extend(str(path.relative_to(repository)) for path in
                             (repository/folder).glob('*.toml'))
            for file in fixed:
                target = root/file
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(repository/file, target)
            for name in ('assembly', 'notation'):
                engine = root/name/'runtime/bin'/('tectonic' if name == 'assembly' else 'node')
                engine.parent.mkdir(parents=True)
                engine.write_bytes(b'fixture')
            args = SimpleNamespace(repository=root, support=root/'assembly',
                                   notation_support=root/'notation', output=root/'record.json')
            with (patch.object(meson_adapter.identities, 'input_paths', return_value=set()),
                  patch.object(meson_adapter, 'document_tree', return_value=set()),
                  patch.object(meson_adapter, 'markdown_sources', return_value=set())):
                meson_adapter.documents_record(args)
                before = args.output.read_bytes()
                source = root/'units/scopes/front.toml'
                source.write_text(source.read_text()+'\n')
                meson_adapter.documents_record(args)
                self.assertNotEqual(args.output.read_bytes(), before)
                source.write_text(source.read_text()[:-1])
                meson_adapter.documents_record(args)
                self.assertEqual(args.output.read_bytes(), before)
                for name in ('generated/notation-status.json',
                             'generated/native-paper-support.tex'):
                    artifact = root/name
                    original = artifact.read_bytes()
                    artifact.write_bytes(original+b'!')
                    meson_adapter.documents_record(args)
                    self.assertNotEqual(args.output.read_bytes(), before)
                    artifact.write_bytes(original)

    def test_inclusion_rows_and_markers_refuse_changes(self):
        repository = pathlib.Path(__file__).resolve().parents[2]
        expected = len(scope_fragments(repository))
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            files = {'generated/documents/inclusions.tsv',
                     'documents/paper-build/label-mints.tsv',
                     'documents/paper-build/sources.tsv'}
            rows = (repository/'documents/paper-build/sources.tsv').read_text().splitlines()
            files.update(input_paths(repository))
            files.update(rows)
            files.update(source for source, _, _, _ in FRAGMENTS)
            files.update(fragment for _, _, fragment, _ in FRAGMENTS)
            for file in files:
                target = root/file
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(repository/file, target)
            self.assertEqual(len(scope_fragments(root)), expected)
            self.assertEqual(len(checked_fragments(root, rows)), expected)
            artifact = root/'generated/documents/inclusions.tsv'
            original = artifact.read_text()
            artifact.write_text(original.replace('\tparagraph\t', '\tunknown\t', 1))
            with self.assertRaisesRegex(ValueError, 'M079: Inclusion row differs'):
                scope_fragments(root)
            artifact.write_text(original)
            entries = original.splitlines()
            fields = entries[0].split('\t')
            fields[4] = str(int(fields[4]) + 1)
            entries[0] = '\t'.join(fields)
            artifact.write_text('\n'.join(entries) + '\n')
            with self.assertRaisesRegex(ValueError, 'M055: Fragment marker/declaration differs'):
                checked_fragments(root, rows)
            artifact.write_text(original)
            source = root/FRAGMENTS[0][0]
            original = source.read_bytes()
            source.write_bytes(original.replace(b'% generated: ', b'% changed: ', 1))
            with self.assertRaisesRegex(ValueError, 'M055: Fragment marker/declaration differs'):
                checked_fragments(root, rows)
            source.write_bytes(original)
            equation_source, _, equation_fragment, _ = next(
                row for row in FRAGMENTS if row[3] == 'equation')
            equation_file = root/equation_source
            source_bytes = equation_file.read_bytes()
            parent = (repository/'registry/explainer-flow-parent.tex').read_bytes()
            self.assertIn(b'\\[\n', parent)
            self.assertIn(b'\n\\]', parent)
            equation_file.write_bytes(source_bytes + b'\n' + parent + b'\n')
            with self.assertRaisesRegex(ValueError, 'M055: Authored equation duplicates'):
                checked_fragments(root, rows)
            equation_file.write_bytes(source_bytes)
            equation_output = root/equation_fragment
            equation_bytes = equation_output.read_bytes()
            equation_output.write_bytes(equation_bytes.replace(b'\\[', b'\\(', 1))
            with self.assertRaisesRegex(ValueError, 'M055: Fragment equation envelope differs'):
                checked_fragments(root, rows)
            equation_output.write_bytes(equation_bytes)
            math_fragment = root/'generated/documents/paper-pilot/sec-coda-whole-field.tex'
            intact = math_fragment.read_bytes()
            altered = intact.replace(b'\\comovfield{}', b'\\unknown{}', 1)
            self.assertNotEqual(altered, intact)
            math_fragment.write_bytes(altered)
            with self.assertRaisesRegex(ValueError, 'M055: Fragment body differs'):
                checked_fragments(root, rows)
            math_fragment.write_bytes(intact)
            for command in (b'willvirtue', b'gensym'):
                admitted = intact.replace(b'\\comovfield{}', b'\\' + command + b'{}', 1)
                self.assertNotEqual(admitted, intact)
                math_fragment.write_bytes(admitted)
                self.assertEqual(len(checked_fragments(root, rows)), expected)
            refused = intact.replace(b'\\comovfield{}', b'\\readout{}', 1)
            self.assertNotEqual(refused, intact)
            math_fragment.write_bytes(refused)
            with self.assertRaisesRegex(ValueError, 'M055: Fragment body differs'):
                checked_fragments(root, rows)
            math_fragment.write_bytes(intact)
            virtue_fragment = root/'generated/documents/paper-pilot/sec-coda-eight-words.tex'
            virtue_intact = virtue_fragment.read_bytes()
            for changed, fault in [
                (virtue_intact.replace(b'\\virtue{wonder}', b'\\papervirtue{wonder}', 1),
                 'M055: Fragment virtue binding differs'),
                (virtue_intact.replace(b'\\virtue{wonder}', b'\\virtue{unknown}', 1),
                 'M055: Fragment virtue binding differs'),
                (virtue_intact.replace(b'\\virtue{wonder}', b'\\virtue{\\name}', 1),
                 'M055: Fragment body differs'),
            ]:
                self.assertNotEqual(changed, virtue_intact)
                virtue_fragment.write_bytes(changed)
                with self.assertRaisesRegex(ValueError, fault):
                    checked_fragments(root, rows)
            virtue_fragment.write_bytes(virtue_intact)
            self.assertEqual(len(scope_fragments(root)), expected)


class CorpusSlotPreparation(unittest.TestCase):
    def test_multiline_slot_does_not_shift_later_marker(self):
        with tempfile.TemporaryDirectory() as directory:
            work = pathlib.Path(directory)
            source = 'documents/paper/explainer/slot.tex'
            target = work / 'explainer/slot.tex'
            target.parent.mkdir(parents=True)
            target.write_bytes(b'Before\n% first\nBetween\n% second\nAfter\n')
            apply_fragment_replacements(work, {
                (source, 2): b'First line\nSecond line\n',
                (source, 4): b'Last fragment\n',
            })
            self.assertEqual(target.read_bytes(),
                             b'Before\nFirst line\nSecond line\nBetween\nLast fragment\nAfter\n')

    def test_slot_marker_child_and_authored_duplicates(self):
        source = 'documents/paper/explainer/slot.tex'
        unit = 'sec:explainer:plain'
        fragment = 'generated/documents/explainer-pilot/sec-explainer-plain.tex'
        body = (b'\\section{The Whole Idea}\n\\label{sec:explainer:plain}\n\n'
                b'\\PaperNativeParagraph{sec:reading:plain-intro}{The intro.}\n\n'
                b'\\textbf{Ties.}\\enspace First prose.\n\n'
                b'Another paragraph.\n\n'
                b'\\textbf{Work.}\\enspace More prose.\n\n'
                b'Help and cost.\n')
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            source_file = root / source
            output_file = root / fragment
            inclusion = root / 'generated/documents/inclusions.tsv'
            for path in (source_file, output_file, inclusion):
                path.parent.mkdir(parents=True, exist_ok=True)
            source_file.write_bytes(b'% generated: sec:explainer:plain\n')
            output_file.write_bytes(body)
            inclusion.write_text(f'{source}\t{unit}\t{fragment}\tcorpus-slot\t1\n')
            with patch('prepare.assembly_json', return_value=b'{"native_targets":[]}'), \
                    patch('prepare.bindings', return_value={}):
                self.assertEqual(scope_fragments(root)[0][3], 'corpus-slot')
                self.assertEqual(declared_fragments(root, [source])[(source, 1)], body)
                changed = body.replace(b'The intro.', b'\\handle{missing}', 1)
                output_file.write_bytes(changed)
                with self.assertRaisesRegex(ValueError, 'Fragment handle binding differs'):
                    declared_fragments(root, [source])
                output_file.write_bytes(body)
                source_file.write_bytes(source_file.read_bytes() +
                                        b'\\textbf{Ties.}\\enspace First prose.\n')
                with self.assertRaisesRegex(ValueError, 'Authored paragraph duplicates'):
                    declared_fragments(root, [source])
                source_file.write_bytes(b'% generated: sec:explainer:plain\n')
                output_file.write_bytes(body.replace(
                    b'Help and cost.',
                    b'\\PaperNativeParagraph{sec:reading:plain-intro}{The intro.}'))
                with self.assertRaisesRegex(ValueError, 'Corpus slot paragraph differs'):
                    declared_fragments(root, [source])
                output_file.write_bytes(body)
                source_file.write_bytes(source_file.read_bytes() +
                                        b'\\label{sec:explainer:plain}\n')
                with self.assertRaisesRegex(ValueError, 'Authored mint duplicates'):
                    declared_fragments(root, [source])
                source_file.write_bytes(b'% generated: sec:explainer:plain\n' * 2)
                with self.assertRaisesRegex(ValueError, 'Fragment marker/declaration differs'):
                    declared_fragments(root, [source])


if __name__ == '__main__':
    unittest.main()
