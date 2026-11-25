"""A content-only explainer conversion for the complete plain-conversion check."""

import argparse
import json
import pathlib
import subprocess
import tempfile
import unittest
from unittest import mock

import complete_gate
import compare_words


SOURCE = 'documents/paper/explainer/explainer.tex'
UNIT = 'sec:reading:confusions-intro'
UNIT_PATH = 'units/statements/confusions-intro.toml'
BODY = ('The table below collects the confusions this document exists to prevent. '
        'Each block becomes a section of what follows, where the same idea reappears '
        'wearing its formal name and its tag.')


def command(*args, cwd=None):
    result = subprocess.run(args, cwd=cwd, capture_output=True, text=True)
    if result.returncode:
        raise AssertionError(f'{args[0]}: {result.returncode}: {result.stderr}')
    return result.stdout


def replace_once(path, before, after):
    data = path.read_text(encoding='utf-8')
    if data.count(before) != 1:
        raise AssertionError(f'fixture source changed: {path}')
    path.write_text(data.replace(before, after), encoding='utf-8')


def candidate(repo, root, binary, support, work):
    command('git', 'clone', '--quiet', str(repo), str(root))
    parent = command('git', '-C', str(root), 'rev-parse', 'HEAD').strip()
    path = root / SOURCE
    authored = ('The table below collects the confusions this document exists to prevent. '
                'Each block\nbecomes a section of what follows, where the same idea reappears '
                'wearing its formal name\nand its tag.\n')
    original = path.read_text(encoding='utf-8')
    first = original[:original.index(authored)].count('\n') + 1
    replace_once(path, authored, '% generated: ' + UNIT + '\n')
    unit = root / UNIT_PATH
    unit.write_text('schema = 1\n\n[unit]\nid = "' + UNIT +
                    '"\ndocument = "explainer-pilot"\nmath = {}\n\n'
                    '[unit.texts]\nen = "' + BODY + '"\n', encoding='utf-8')
    replace_once(root / 'units/scopes/explainer.toml',
                 '[[scope.occurrences]]\nunit = "sec:say:direction"',
                 '[[scope.occurrences]]\nunit = "' + UNIT +
                 '"\nrole = "paragraph"\n\n[[scope.occurrences]]\n'
                 'unit = "sec:say:direction"')
    replace_once(root / 'units/skeletons/explainer-pilot.toml',
                 '  { kind = "quotation", unit = "sec:say:direction" },',
                 '  { kind = "paragraph", unit = "' + UNIT + '" },\n'
                 '  { kind = "quotation", unit = "sec:say:direction" },')
    acceptance = root / 'documents/paper-build/self-description.json'
    record = json.loads(acceptance.read_text(encoding='utf-8'))
    for passage in record['passages']:
        data = (root / passage['file']).read_bytes()
        anchor = passage['anchor'].encode('utf-8')
        if data.count(anchor) != 1:
            raise AssertionError('accepted passage anchor changed or ambiguous')
        size = passage['end'] - passage['start']
        passage['start'] = data.index(anchor)
        passage['end'] = passage['start'] + size
        if compare_words.digest(data[passage['start']:passage['end']]) != passage['sha256']:
            raise AssertionError('accepted passage bytes changed')
    acceptance.write_text(json.dumps(record, ensure_ascii=False, indent=2) + '\n',
                          encoding='utf-8')
    command(str(binary), 'documents-generate', '--repository', str(root),
            '--support', str(support))
    command('git', '-C', str(root), 'add', '-A')
    command(str(binary), 'generate', '--repository', str(root),
            '--support', str(support), '--work', str(work / 'candidate-pair'))
    command('git', '-C', str(root), 'add', '-A')
    command('git', '-C', str(root), '-c', 'user.name=Fixture',
            '-c', 'user.email=fixture@example.invalid', 'commit', '--quiet',
            '-m', 'fixture: plain conversion')
    current = command('git', '-C', str(root), 'rev-parse', 'HEAD').strip()
    assignment = dict(schema=3, parent=parent, candidate=current, source=SOURCE,
                      first=first, last=first + 2, unit=UNIT,
                      **{'source-blob': command('git', '-C', str(root), 'rev-parse',
                                               parent + ':' + SOURCE).strip(),
                         'unit-path': UNIT_PATH})
    return assignment


class CompleteProofControls(unittest.TestCase):
    def test_new_register_kind_refuses(self):
        screen = {'entries': [], 'content-gaps': [
            {'document': 'explainer-pilot', 'language': 'en',
             'message': 'intervening', 'position': 1}]}
        html = {'gaps': []}
        changed = {'entries': [], 'content-gaps': [
            dict(screen['content-gaps'][0], message='new missing body')]}
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.register_kinds(screen, changed, html, html)
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('RegisterKindChanged', 'screen register adds a new row kind'))
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.register_kinds(screen, screen, html, {'gaps': [
                {'document': 'explainer-pilot', 'unit': UNIT,
                 'inline-kind': 'mathematics', 'block': 2}]})
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('RegisterKindChanged', 'HTML register changes row kinds'))

    def test_changed_independent_preparation_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)

            def capture_record(*args):
                (root / 'work/capture').mkdir(parents=True)
                (root / 'work/capture/envelope.json').write_text(
                    json.dumps({'prepared_inputs': {'main.tex': 'changed'}}),
                    encoding='utf-8')
                return {}

            with mock.patch.object(complete_gate.checking_capture, 'prepare'), \
                    mock.patch.object(complete_gate.checking_capture, 'input_digests',
                                      return_value={'main.tex': 'original'}), \
                    mock.patch.object(complete_gate.checking_capture, 'capture',
                                      return_value={}), \
                    mock.patch.object(complete_gate.checking_capture, 'write_capture',
                                      side_effect=capture_record):
                with self.assertRaises(complete_gate.GateFault) as raised:
                    complete_gate.checked_capture(root, root, root / 'work')
            self.assertEqual((raised.exception.code, str(raised.exception)),
                             ('CheckingSnapshotMismatch',
                              'capture differs from independently prepared inputs'))

    def test_one_structural_occurrence_and_unchanged_surroundings(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            parent = root / 'parent'
            current = root / 'candidate'
            unit = 'sec:reading:fixture'
            path = 'units/statements/fixture.toml'
            for repo in (parent, current):
                for folder in ('units/scopes', 'units/skeletons', 'units/statements'):
                    (repo / folder).mkdir(parents=True)
            (parent / SOURCE).parent.mkdir(parents=True)
            (parent / SOURCE).write_text('Plain words.\n', encoding='utf-8')
            old_scope = ('schema = 1\n[scope]\ndocument = "explainer-pilot"\n'
                         'source = "documents/paper/explainer/explainer.tex"\n'
                         '[[scope.occurrences]]\nunit = "existing"\nrole = "paragraph"\n')
            new_scope = old_scope + ('[[scope.occurrences]]\nunit = "' + unit +
                                     '"\nrole = "paragraph"\n')
            old_skeleton = ('schema = 1\n[skeleton]\ndocument = "explainer-pilot"\n'
                            '[[skeleton.sections]]\nheading = "first"\n'
                            'children = [{ kind = "paragraph", unit = "existing" }]\n')
            new_skeleton = old_skeleton.replace('unit = "existing" }]',
                'unit = "existing" }, { kind = "paragraph", unit = "' + unit + '" }]')
            for repo, scope, skeleton in ((parent, old_scope, old_skeleton),
                                          (current, new_scope, new_skeleton)):
                (repo / 'units/scopes/explainer.toml').write_text(scope, encoding='utf-8')
                (repo / 'units/skeletons/explainer-pilot.toml').write_text(
                    skeleton, encoding='utf-8')
            (current / path).write_text('schema = 1\n[unit]\nid = "' + unit +
                                        '"\ndocument = "explainer-pilot"\nmath = {}\n'
                                        '[unit.texts]\nen = "Plain words."\n',
                                        encoding='utf-8')
            assignment = {'unit': unit, 'unit-path': path, 'source': SOURCE,
                          'first': 1, 'last': 1}
            complete_gate.structure(parent, current, assignment)
            (current / 'units/scopes/explainer.toml').write_text(
                new_scope.replace('unit = "existing"', 'unit = "changed"'),
                encoding='utf-8')
            with self.assertRaises(complete_gate.GateFault) as raised:
                complete_gate.structure(parent, current, assignment)
            self.assertEqual((raised.exception.code, str(raised.exception)),
                             ('StructuralCorrespondenceMismatch',
                              'scope changed outside assigned occurrence'))
            (current / 'units/scopes/explainer.toml').write_text(new_scope,
                                                                 encoding='utf-8')
            (current / 'units/skeletons/explainer-pilot.toml').write_text(
                new_skeleton.replace('unit = "existing"', 'unit = "changed"'),
                encoding='utf-8')
            with self.assertRaises(complete_gate.GateFault) as raised:
                complete_gate.structure(parent, current, assignment)
            self.assertEqual((raised.exception.code, str(raised.exception)),
                             ('StructuralCorrespondenceMismatch',
                              'skeleton changed outside assigned child'))
            (current / 'units/skeletons/explainer-pilot.toml').write_text(
                new_skeleton, encoding='utf-8')
            unit_bytes = (current / path).read_bytes()
            (current / path).write_bytes(unit_bytes.replace(b'Plain words.',
                                                      b'Changed words.'))
            with self.assertRaises(complete_gate.GateFault) as raised:
                complete_gate.structure(parent, current, assignment)
            self.assertEqual((raised.exception.code, str(raised.exception)),
                             ('StructuralCorrespondenceMismatch',
                              'unit text differs from selected authored source'))
            (current / path).write_bytes(unit_bytes)
            (parent / path).write_text('already minted\n', encoding='utf-8')
            with self.assertRaises(complete_gate.GateFault) as raised:
                complete_gate.structure(parent, current, assignment)
            self.assertEqual((raised.exception.code, str(raised.exception)),
                             ('StructuralCorrespondenceMismatch',
                              'assigned primary unit already exists in parent'))


def integration(repo, binary, support, work):
    work.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=work) as directory:
        root = pathlib.Path(directory)
        candidate_root = root / 'candidate'
        assignment = candidate(repo, candidate_root, binary, support, root)
        request = root / 'assignment.json'
        request.write_text(json.dumps(assignment), encoding='utf-8')
        screen = candidate_root / 'registry/screen-destination-gaps.json'
        canonical = screen.read_bytes()
        screen.write_bytes(canonical.replace(b'"position": 1', b'"position": 99', 1))
        with unittest.TestCase().assertRaises(complete_gate.GateFault) as stale:
            complete_gate.run(candidate_root, request, support, binary,
                              root / 'stale-register')
        if stale.exception.code != 'RegisterMismatch' or 'canonical' not in str(stale.exception):
            raise AssertionError('stale register did not refuse canonically')
        if (root / 'stale-register/candidate-pair').exists():
            raise AssertionError('stale register reached pair production')
        screen.write_bytes(canonical)
        result = complete_gate.run(candidate_root, request, support, binary,
                                   root / 'complete')
        if result['verdict'] != 'complete':
            raise AssertionError(result)
        print(json.dumps(result, sort_keys=True))
        markdown = candidate_root / 'generated/documents/explainer-pilot/en.md'
        original = markdown.read_bytes()
        markdown.write_bytes(original.replace(b'This first section',
                                             b'This changed section', 1))
        try:
            with unittest.TestCase().assertRaises(complete_gate.GateFault) as raised:
                complete_gate.run(candidate_root, request, support, binary,
                                  root / 'fabricated-markdown')
            if raised.exception.code != 'ProofFailed' or 'M057' not in str(raised.exception):
                raise AssertionError('fabricated document did not reach M057')
        finally:
            markdown.write_bytes(original)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repository', type=pathlib.Path)
    parser.add_argument('--binary', type=pathlib.Path)
    parser.add_argument('--support', type=pathlib.Path)
    parser.add_argument('--work', type=pathlib.Path)
    options = parser.parse_args()
    if options.repository is None:
        unittest.main(argv=['test_complete_gate.py'])
    else:
        integration(options.repository, options.binary, options.support, options.work)
