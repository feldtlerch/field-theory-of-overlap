"""Controls for checked inclusions and source markers."""
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch
from prepare import FRAGMENTS, scope_fragments, declared_fragments
import meson_adapter


def checked_fragments(root, rows):
    with patch('identities.bindings') as bindings:
        bindings.return_value.get.return_value = {'source_status': 'paper'}
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

    def test_inclusion_rows_and_markers_refuse_changes(self):
        repository = pathlib.Path(__file__).resolve().parents[2]
        expected = len(scope_fragments(repository))
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            files = {'generated/documents/inclusions.tsv'}
            rows = (repository/'documents/paper-build/sources.tsv').read_text().splitlines()
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
            self.assertEqual(len(scope_fragments(root)), expected)


if __name__ == '__main__':
    unittest.main()
