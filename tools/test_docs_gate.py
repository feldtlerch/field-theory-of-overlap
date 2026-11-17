"""Fast adversarial classification tests; no engine or mathematics runner."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess

import docs_gate_lib as gate


class Classification(unittest.TestCase):
    def setUp(self):
        self.base = {
            gate.LOCK: ('100644', b'schema = 1\n[executable]\nsha256 = "abc"\n'),
            gate.PREPARE: ('100644', b'print("prepare")\n'),
            'documents/paper-build/sources.tsv': ('100644', b'documents/paper/main.tex\n'),
            'documents/paper/main.tex': ('100644', b'paper'),
            'generated/paper/paper.pdf': ('100644', b'pdf'),
            'generated/manifest.json': ('100644', b'{}'),
            'README.md': ('100644', b'record'),
        }
        inputs = {p: gate.digest(self.base[p][1]) for p in (
            gate.LOCK, 'documents/paper/main.tex', 'documents/paper-build/sources.tsv')}
        self.base[gate.RECEIPT] = ('100644', (json.dumps({'inputs': inputs},
                                                        indent=2, sort_keys=True) + '\n').encode())
        self.current = dict(self.base)

    def change(self, path, data=b'changed', mode='100644'):
        self.current[path] = (mode, data)
        return gate.classify(self.base, self.current)

    def test_allowed_record_kinds(self):
        for path in ('README.md', 'LICENSING.md', 'CONTRIBUTING.md', 'CHANGELOG.md', 'LICENSE-CODE',
                     'LICENSE-DOCS', 'docs/adr/018-example.md', 'docs/publication-policy.md',
                     'licenses/fonts/OFL.txt', 'tools/paper/README.md'):
            with self.subTest(path=path):
                self.assertFalse(self.change(path)['refused'])

    def test_record_deletion(self):
        del self.current['README.md']
        self.assertFalse(gate.classify(self.base, self.current)['refused'])

    def test_all_refused_kinds(self):
        for path in ('crates/new.rs', 'documents/paper/main.tex',
                     'documents/paper-build/templates.toml', 'crates/ftoo_codegen/src/lib.rs',
                     'tools/notation/support-digests.json', 'units/example.toml',
                     'registry/open-questions.toml', 'Cargo.lock', '.github/workflows/ci.yml',
                     'tools/new.py', 'generated/new.json', gate.LOCK, gate.PREPARE,
                     'tools/docs_gate_lib.py', 'tools/test_docs_gate.py', 'tools/json.py'):
            with self.subTest(path=path):
                self.assertIn(path, self.change(path)['refused'])

    def test_input_precedes_record_permission(self):
        receipt = json.loads(self.base[gate.RECEIPT][1])
        receipt['inputs']['README.md'] = gate.digest(self.base['README.md'][1])
        self.base[gate.RECEIPT] = ('100644', json.dumps(receipt).encode())
        self.current[gate.RECEIPT] = self.base[gate.RECEIPT]
        self.assertIn('README.md', self.change('README.md')['refused'])

    def test_output_hashes_and_tampering(self):
        result = gate.classify(self.base, self.current)
        self.assertEqual(result['output_sha256']['generated/paper/paper.pdf'], gate.digest(b'pdf'))
        for path in ('generated/paper/paper.pdf', 'generated/manifest.json', gate.RECEIPT):
            with self.subTest(path=path):
                self.assertIn(path, self.change(path)['refused'])
                del self.current[path]
                self.assertIn(path, gate.classify(self.base, self.current)['refused'])

    def test_modes_and_links(self):
        self.assertIn('README.md', self.change('README.md', mode='100755')['refused'])


class WorkingTree(unittest.TestCase):
    def test_symlink_is_not_followed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'link').symlink_to('/dev/null')
            with patch.object(gate, 'git', return_value=b'link\0'):
                with self.assertRaisesRegex(ValueError, 'nonregular'):
                    gate.working_tree(root, {})

    def test_clean_candidate_and_untracked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gate.git(root, 'init', '-q')
            record = root / 'README.md'
            record.write_text('base')
            (root / '.gitignore').write_text('ignored\n')
            base = commit(root)
            record.write_text('candidate')
            head = commit(root)
            self.assertEqual(gate.candidate_tree(root, base)[0], head)
            record.write_text('base')
            for staged in (False, True):
                if staged:
                    gate.git(root, 'add', '.')
                with self.assertRaisesRegex(ValueError, 'differs from HEAD'):
                    gate.candidate_tree(root, base)
            record.write_text('candidate')
            gate.git(root, 'add', '.')
            for name in ('untracked', 'ignored'):
                (root / name).write_text('new')
                with self.assertRaisesRegex(ValueError, 'untracked or ignored'):
                    gate.clean_head(root)
                (root / name).unlink()
            gate.git(root, 'checkout', '--orphan', 'unrelated')
            commit(root)
            with self.assertRaises(subprocess.CalledProcessError):
                gate.candidate_tree(root, base)

    def test_base_copy_guard(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gate.git(root, 'init', '-q')
            source = root / 'tools/docs_gate_lib.py'
            source.parent.mkdir()
            source.write_text('fixture')
            source.with_name('docs_gate.py').write_text('entry fixture')
            base = commit(root)
            _, tree = gate.base_tree(root, base)
            with patch.object(gate, '__file__', str(source)):
                with self.assertRaisesRegex(ValueError, 'never judges itself'):
                    gate.require_base_copy(root, base, tree)
                gate.require_base_copy(root / 'candidate', base, tree)
                with self.assertRaisesRegex(ValueError, 'checkout of BASE'):
                    gate.require_base_copy(root / 'candidate', '0' * 40, tree)


def commit(root):
    gate.git(root, 'add', '.')
    gate.git(root, '-c', 'user.name=Gate test', '-c', 'user.email=gate@example.invalid',
             '-c', 'commit.gpgsign=false', 'commit', '-qm', 'test fixture')
    return gate.clean_head(root)


if __name__ == '__main__':
    unittest.main()
