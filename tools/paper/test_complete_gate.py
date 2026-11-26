"""A content-only explainer conversion for the complete plain-conversion check."""

import argparse
import difflib
import hashlib
import json
import pathlib
import subprocess
import tempfile
import time
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


class StreamComparisonControls(unittest.TestCase):
    @staticmethod
    def sides(old, new, markdown=None, html=None):
        return {name: (compare_words._CheckedPdfStream(text=value, coverage=True)
                       if name in ('old_pdf', 'new_pdf') else
                       dict(text=value, coverage=True, medium=name)) for name, value in (
            ('old_pdf', old), ('new_pdf', new),
            ('markdown', new if markdown is None else markdown),
            ('html', new if html is None else html))}

    @staticmethod
    def proofs(identities=None):
        return {name: dict(verdict='checked', identities=identities or {})
                for name in compare_words.STREAM_SIDES}

    @staticmethod
    def slot_export(side='before'):
        words = ('Heading', 'Intro', 'Ties', 'Questions', 'Work', 'Help')
        source_name = 'documents/paper/explainer/explainer.tex'
        intro_name = 'units/statements/plain-intro.toml'
        corpus_name = 'units/corpus/explainer-plain.toml'
        inherited = ('\n\n'.join((words[0], '% generated: sec:reading:plain-intro',
                                  *words[2:]))).encode()
        source = inherited if side == 'before' else b'% generated: sec:slot\n'
        corpus = ('\n\n'.join((words[0], *words[2:]))).encode()
        captured = {source_name: source, intro_name: b'Intro'}
        if side == 'after':
            captured[corpus_name] = corpus
        parent_blob = hashlib.sha1(b'blob ' + str(len(inherited)).encode() +
                                   b'\0' + inherited).hexdigest()
        assignment = dict(source=source_name, first=1, last=11, unit='sec:slot',
                          **{'source-blob': parent_blob})
        revision = 'a' * 40 if side == 'before' else 'b' * 40
        blocks = []
        for index, (word, kind) in enumerate(zip(words, compare_words.SLOT_KINDS)):
            path = intro_name if index == 1 else source_name if side == 'before' else corpus_name
            data = captured[path]
            start = data.index(word.encode())
            span = [start, start + len(word)]
            token_range = span if side == 'before' else [0, len(word)]
            blocks.append(dict(kind=kind, index=index,
                               origin=dict(path=path,
                                           blob=hashlib.sha1(
                                               b'blob ' + str(len(data)).encode() +
                                               b'\0' + data).hexdigest(),
                                           sha256=compare_words.digest(data), range=span,
                                           field=None if side == 'before' and index != 1 else
                                           f'unit.blocks[{index}].text'),
                               controls=[], tokens=[dict(kind='word',
                                   source_range=(span if side == 'before' else token_range),
                                   source_spelling=word,
                                   semantic=word, extracted=word,
                                   separator_before=False, separator_range=None, tag=None)]))
        export = dict(schema=1, side=side, revision=revision,
                      selection={key: assignment[key] for key in
                                 ('source', 'first', 'last', 'unit')},
                      **{'source-blob': hashlib.sha1(
                          b'blob ' + str(len(source)).encode() + b'\0' + source).hexdigest(),
                         'source-sha256': compare_words.digest(source)},
                      accounting=[dict(kind='block' if side == 'before' else 'event',
                                       range=[0, len(source)])],
                      blocks=blocks)
        return export, captured, assignment, revision

    def test_located_slot_export_binds_sources_and_order(self):
        export, captured, assignment, revision = self.slot_export()
        blocks = compare_words.located_slot_export(
            json.dumps(export).encode(), captured, assignment, 'before', revision)
        self.assertEqual([row['semantic'] for row in blocks],
                         ['Heading', 'Intro', 'Ties', 'Questions', 'Work', 'Help'])
        changed = json.loads(json.dumps(export))
        changed['blocks'][4]['tokens'][0]['source_spelling'] = 'Cost'
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.located_slot_export(
                json.dumps(changed).encode(), captured, assignment, 'before', revision)
        self.assertEqual(raised.exception.row['detail'],
                         'located slot token spelling differs')
        changed = json.loads(json.dumps(export))
        changed['blocks'][4], changed['blocks'][5] = changed['blocks'][5], changed['blocks'][4]
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.located_slot_export(
                json.dumps(changed).encode(), captured, assignment, 'before', revision)
        self.assertEqual(raised.exception.row['detail'], 'located slot child order differs')
        changed = json.loads(json.dumps(export))
        changed['blocks'].pop(3)
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.located_slot_export(
                json.dumps(changed).encode(), captured, assignment, 'before', revision)
        self.assertEqual(raised.exception.row['detail'],
                         'located slot identity or blocks differ')
        changed = json.loads(json.dumps(export))
        changed['accounting'][0]['range'][1] -= 1
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.located_slot_export(
                json.dumps(changed).encode(), captured, assignment, 'before', revision)
        self.assertEqual(raised.exception.row['detail'],
                         'located slot accounting omits source bytes')
        changed = json.loads(json.dumps(export))
        changed['source-sha256'] = '0' * 64
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.located_slot_export(
                json.dumps(changed).encode(), captured, assignment, 'before', revision)
        self.assertEqual(raised.exception.row['detail'], 'located slot source identity differs')
        changed = json.loads(json.dumps(export))
        changed['blocks'][0]['controls'] = [dict(kind='delete-word', range=[0, 7])]
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.located_slot_export(
                json.dumps(changed).encode(), captured, assignment, 'before', revision)
        self.assertEqual(raised.exception.row['detail'], 'located slot control differs')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.located_slot_export(
                json.dumps(export).encode(), captured, assignment, 'after', 'b' * 40)
        self.assertEqual(raised.exception.row['detail'],
                         'located slot identity or blocks differ')
        duplicate = json.dumps(export).replace('"schema": 1',
                                               '"schema": 1, "schema": 1', 1).encode()
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.located_slot_export(
                duplicate, captured, assignment, 'before', revision)
        self.assertEqual(raised.exception.row['detail'],
                         'duplicate correspondence field: schema')

    def test_native_token_ranges_address_decoded_runs_not_raw_toml(self):
        export, captured, assignment, revision = self.slot_export('after')
        path = 'units/corpus/explainer-plain.toml'
        field = b'[[unit.blocks]]\nruns = [{ kind = "text", decoded = "Heading" }]\n'
        raw = field * 5
        captured[path] = raw
        blob = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
        native_index = 0
        for block in export['blocks']:
            if block['origin']['path'] == path:
                block['origin']['blob'] = blob
                block['origin']['sha256'] = compare_words.digest(raw)
                start = native_index * len(field)
                block['origin']['range'] = [start, start + len(field)]
                native_index += 1
        blocks = compare_words.located_slot_export(
            json.dumps(export).encode(), captured, assignment, 'after', revision)
        self.assertEqual([block['semantic'] for block in blocks],
                         ['Heading', 'Intro', 'Ties', 'Questions', 'Work', 'Help'])
        changed = json.loads(json.dumps(export))
        changed['blocks'][2]['tokens'][0]['source_range'][1] += 1
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.located_slot_export(
                json.dumps(changed).encode(), captured, assignment, 'after', revision)
        self.assertEqual(raised.exception.row['detail'],
                         'native slot token spelling length differs')

    def test_slot_request_capture_and_cli_dispatch(self):
        export, captured, _, _ = self.slot_export()
        name = 'documents/paper/explainer/explainer.tex'
        source = captured[name]
        assignment = dict(schema=4, form='coarse-slot', parent='a' * 40,
                          candidate='b' * 40, source=name,
                          **{'source-blob': export['source-blob']},
                          first=1, last=11, unit='sec:slot',
                          blocks=[dict(kind=kind, first=1 + index * 2,
                                       last=1 + index * 2)
                                  for index, kind in enumerate(compare_words.SLOT_KINDS)])
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            repo = root / 'repo'
            repo.mkdir()
            request = root / 'request.json'
            request.write_text(json.dumps(assignment))
            with mock.patch.object(compare_words, 'git', return_value=source):
                captured_request = compare_words.capture_assignment(repo, request)
            self.assertEqual(captured_request[0], assignment)
            changed = json.loads(json.dumps(assignment))
            changed['blocks'][1]['kind'] = 'paragraph'
            request.write_text(json.dumps(changed))
            with mock.patch.object(compare_words, 'git', return_value=source):
                with self.assertRaises(compare_words.Fault) as raised:
                    compare_words.capture_assignment(repo, request)
            self.assertEqual(raised.exception.row['detail'],
                             'coarse-slot child kinds differ')
            request.write_text(json.dumps(assignment))
            argv = ['compare_words.py', 'compare', '--repository', str(repo),
                    '--before-paper', 'a.pdf', '--after-paper', 'b.pdf',
                    '--before-explainer', 'c.pdf', '--after-explainer', 'd.pdf',
                    '--before-revision', assignment['parent'], '--unit', 'sec:slot',
                    '--assignment', str(request), '--output', str(root / 'out')]
            with mock.patch('sys.argv', argv), \
                    mock.patch.object(compare_words, 'capture_assignment',
                                      return_value=captured_request), \
                    mock.patch.object(compare_words, 'compare_slot', return_value=0) as slot:
                self.assertEqual(compare_words.main(), 0)
            self.assertEqual(slot.call_count, 1)

    def test_named_strong_and_en_space_controls_are_source_bound(self):
        export, captured, assignment, revision = self.slot_export()
        path = assignment['source']
        changed = captured[path].replace(b'Ties', b'\\textbf{Ties}\\enspace', 1)
        captured[path] = changed
        blob = hashlib.sha1(b'blob ' + str(len(changed)).encode() +
                            b'\0' + changed).hexdigest()
        assignment['source-blob'] = blob
        export['source-blob'] = blob
        export['source-sha256'] = compare_words.digest(changed)
        export['accounting'][0]['range'][1] = len(changed)
        for block in export['blocks']:
            if block['origin']['path'] != path:
                continue
            word = block['tokens'][0]['source_spelling']
            first = changed.index(word.encode())
            block['origin']['blob'] = blob
            block['origin']['sha256'] = compare_words.digest(changed)
            block['origin']['range'] = [first, first + len(word)]
            block['tokens'][0]['source_range'] = [first, first + len(word)]
        block = export['blocks'][2]
        first = block['tokens'][0]['source_range'][0]
        block['origin']['range'] = [first - len(b'\\textbf{'),
                                    first + len(b'Ties}\\enspace')]
        block['controls'] = [
            dict(kind='strong-open', range=[first - len(b'\\textbf{'), first]),
            dict(kind='strong-close', range=[first + 4, first + 5]),
            dict(kind='en-space', range=[first + 5, first + len(b'Ties}\\enspace')])]
        blocks = compare_words.located_slot_export(
            json.dumps(export).encode(), captured, assignment, 'before', revision)
        self.assertEqual(blocks[2]['source_controls'],
                         [(0, 8, b''), (12, 13, b''), (13, 21, b' ')])

    def test_slot_streams_compare_all_children_and_screens(self):
        old_export, old_paths, assignment, old_revision = self.slot_export('before')
        new_export, new_paths, _, new_revision = self.slot_export('after')
        old_blocks = compare_words.located_slot_export(
            json.dumps(old_export).encode(), old_paths, assignment, 'before', old_revision)
        new_blocks = compare_words.located_slot_export(
            json.dumps(new_export).encode(), new_paths, assignment, 'after', new_revision)
        text = 'Heading Intro Ties Questions Work Help'

        def pdf(body):
            return dict(stream=body, offsets=[[1, n] for n in range(len(body))],
                        events=[], titles=['Heading'])

        pdfs = {'paper': {'before': pdf('Paper'), 'after': pdf('Paper')},
                'explainer': {'before': pdf(text), 'after': pdf(text)}}
        scope = {key: assignment[key] for key in ('source', 'first', 'last', 'unit')}
        screens = {medium: dict(text=text, coverage=True, medium=medium,
                                scope=scope, document_coverage='excerpt')
                   for medium in ('markdown', 'html')}
        equal = compare_words.compare_slot_streams(assignment, old_blocks, new_blocks,
                                                   pdfs, screens['markdown'],
                                                   screens['html'], {'available': True})
        self.assertEqual((equal['verdict'], len(equal['child_ranges']['after'])),
                         ('equal', 6))
        changed = dict(screens['html'], text=text.replace('Work', 'Cost'))
        different = compare_words.compare_slot_streams(
            assignment, old_blocks, new_blocks, pdfs, screens['markdown'],
            changed, {'available': True})
        self.assertEqual((different['verdict'], different['screens']['differences'][0]['side']),
                         ('different', 'html'))
        refused = compare_words.compare_slot_streams(
            assignment, old_blocks, new_blocks, pdfs, screens['markdown'],
            screens['html'], {'available': False})
        self.assertEqual(refused['verdict'], 'refused')

    def test_native_slot_word_change_is_not_screen_confirmed(self):
        old_export, old_paths, assignment, old_revision = self.slot_export('before')
        new_export, new_paths, _, new_revision = self.slot_export('after')
        corpus = 'units/corpus/explainer-plain.toml'
        new_paths[corpus] = new_paths[corpus].replace(b'Work', b'Cost')
        for block in new_export['blocks']:
            if block['origin']['path'] == corpus:
                data = new_paths[corpus]
                block['origin']['blob'] = hashlib.sha1(
                    b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
                block['origin']['sha256'] = compare_words.digest(data)
            if block['index'] == 4:
                block['tokens'][0].update(source_spelling='Cost', semantic='Cost',
                                          extracted='Cost')
        old_blocks = compare_words.located_slot_export(
            json.dumps(old_export).encode(), old_paths, assignment, 'before', old_revision)
        new_blocks = compare_words.located_slot_export(
            json.dumps(new_export).encode(), new_paths, assignment, 'after', new_revision)
        old_text = 'Heading Intro Ties Questions Work Help'
        new_text = 'Heading Intro Ties Questions Cost Help'

        def pdf(text):
            return dict(stream=text, offsets=[[1, n] for n in range(len(text))],
                        events=[], titles=['Heading'])

        pdfs = dict(paper=dict(before=pdf('Paper'), after=pdf('Paper')),
                    explainer=dict(before=pdf(old_text), after=pdf(new_text)))
        scope = {key: assignment[key] for key in ('source', 'first', 'last', 'unit')}
        screens = {medium: dict(text=new_text, coverage=True, medium=medium,
                                scope=scope, document_coverage='excerpt')
                   for medium in ('markdown', 'html')}
        result = compare_words.compare_slot_streams(
            assignment, old_blocks, new_blocks, pdfs, screens['markdown'],
            screens['html'], {'available': True})
        self.assertEqual((result['verdict'], result['source_differences'],
                          result['documents']['explainer']['verdict']),
                         ('different', [4], 'different'))

    def test_slot_screen_export_binds_candidate_rendering(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = pathlib.Path(directory)
            name = 'generated/documents/explainer-pilot.md'
            rendering = repo / name
            rendering.parent.mkdir(parents=True)
            rendering.write_bytes(b'Heading Intro')
            scope = dict(source='documents/paper/explainer/explainer.tex',
                         first=44, last=98, unit='sec:explainer:plain')
            row = dict(schema=1, medium='markdown', selection=scope,
                       coverage='excerpt', text='Heading Intro',
                       rendering=dict(path=name,
                                      sha256=compare_words.digest(rendering.read_bytes())))
            export = repo / 'recovery.json'
            export.write_text(json.dumps(row))
            with mock.patch.object(compare_words, 'git',
                                   return_value=rendering.read_bytes()):
                checked, _, _ = compare_words._slot_screen(
                    repo, 'a' * 40, export, 'markdown', scope)
            self.assertEqual(checked['text'], 'Heading Intro')
            rendering.write_bytes(b'Heading Changed')
            with mock.patch.object(compare_words, 'git',
                                   return_value=rendering.read_bytes()):
                with self.assertRaises(compare_words.Fault) as raised:
                    compare_words._slot_screen(repo, 'a' * 40, export, 'markdown', scope)
            self.assertEqual(raised.exception.row['detail'],
                             'recovered stream rendering digest differs')

    def test_slot_command_uses_both_exports_and_complete_pair(self):
        before_export, before_paths, assignment, parent = self.slot_export('before')
        after_export, after_paths, _, candidate = self.slot_export('after')
        assignment.update(parent=parent, candidate=candidate)
        selection = {key: assignment[key] for key in ('source', 'first', 'last', 'unit')}
        text = 'Heading Intro Ties Questions Work Help'
        with tempfile.TemporaryDirectory() as directory:
            base = pathlib.Path(directory)
            repo = base / 'repo'
            (repo / '.git').mkdir(parents=True)
            for name, data in after_paths.items():
                path = repo / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            for medium in ('markdown', 'html'):
                name = f'generated/documents/explainer-pilot.{"md" if medium == "markdown" else "html"}'
                path = repo / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(medium.encode())
                (base / (medium + '.json')).write_bytes(b'{}')
            request = base / 'assignment.json'
            request.write_text(json.dumps(assignment))
            pdf_paths = [base / (side + '-' + document + '.pdf')
                         for side in ('before', 'after')
                         for document in ('paper', 'explainer')]
            for path in pdf_paths:
                path.write_bytes(b'PDF')

            def fake_git(_, action, *parts):
                if action == 'rev-parse':
                    return candidate.encode() + b'\n'
                revision, name = parts[0].split(':', 1)
                if name.endswith('.pdf'):
                    return b'PDF'
                return (before_paths if revision == parent else after_paths)[name]

            def snapshot(_, destination, __):
                source = assignment['source']
                path = destination / source
                path.parent.mkdir(parents=True)
                path.write_bytes(after_paths[source])
                source_list = destination / 'documents/paper-build/sources.tsv'
                source_list.parent.mkdir(parents=True)
                source_list.write_text(source + '\n')
                return [dict(path=source, sha256=compare_words.digest(after_paths[source]))]

            def parent_snapshot(_, __, destination):
                source = assignment['source']
                path = destination / source
                path.parent.mkdir(parents=True)
                path.write_bytes(before_paths[source])
                return dict(inputs=[dict(path=source,
                                         sha256=compare_words.digest(before_paths[source]))])

            def export(_, __, ___, ____, _____, side, output):
                data = json.dumps(before_export if side == 'before' else after_export).encode()
                path = output / (side + '-correspondence/correspondence.json')
                path.parent.mkdir(parents=True)
                path.write_bytes(data)
                return data

            def screen(_, __, ___, medium, scope):
                name = f'generated/documents/explainer-pilot.{"md" if medium == "markdown" else "html"}'
                return (dict(text=text, coverage=True, medium=medium, scope=scope,
                             document_coverage='excerpt', rendering=dict(path=name)),
                        b'{}', medium.encode())

            def extract(path, _, __):
                body = text if 'explainer-' in path.name else 'Paper'
                return dict(stream=body, offsets=[[1, n] for n in range(len(body))],
                            events=[], titles=['Heading'])

            args = argparse.Namespace(repository=repo, output=base / 'output',
                paper_binary=base / 'paper', markdown_recovery=base / 'markdown.json',
                html_recovery=base / 'html.json', before_revision=parent,
                before_source=None, unit=assignment['unit'],
                **{side + '_' + document: base / (side + '-' + document + '.pdf')
                   for side in ('before', 'after')
                   for document in ('paper', 'explainer')})
            with mock.patch.object(compare_words, 'git', side_effect=fake_git), \
                    mock.patch.object(compare_words, 'snapshot_inputs', side_effect=snapshot), \
                    mock.patch.object(compare_words, 'parent_snapshot', side_effect=parent_snapshot), \
                    mock.patch.object(compare_words, '_slot_export', side_effect=export), \
                    mock.patch.object(compare_words, 'slot_source_envelope',
                                      return_value={'available': True}), \
                    mock.patch.object(compare_words, 'declared_fragments'), \
                    mock.patch.object(compare_words, '_slot_screen', side_effect=screen), \
                    mock.patch.object(compare_words, 'extract', side_effect=extract):
                self.assertEqual(compare_words.compare_slot(
                    args, (assignment, request, request.read_bytes())), 0)
            result = json.loads((args.output / 'comparison.json').read_text())
            self.assertEqual((result['verdict'], result['selection']), ('equal', selection))

    def test_reflow_preserves_ordered_content(self):
        result = compare_words.three_way_content(
            self.sides('The first\nline has 12 words.',
                       'The first line\nhas 12 words.'), self.proofs())
        self.assertEqual(result['verdict'], 'equal')

    def test_screen_apostrophe_must_match_printed_spelling(self):
        equal = compare_words.three_way_content(
            self.sides('The theory’s point.', 'The theory’s point.'), self.proofs())
        self.assertEqual(equal['verdict'], 'equal')
        changed = compare_words.three_way_content(
            self.sides('The theory’s point.', 'The theory’s point.',
                       "The theory's point.", 'The theory’s point.'), self.proofs())
        self.assertEqual(changed['verdict'], 'different')
        self.assertEqual(changed['differences'][0]['side'], 'markdown')
        self.assertEqual(changed['differences'][0]['observed']['value'], "theory's")

    def test_plan_and_screens_cannot_replace_pdf_word(self):
        result = compare_words.three_way_content(
            self.sides('The word is original.', 'The word is original.',
                       'The word is revised.', 'The word is revised.'), self.proofs())
        self.assertEqual(result['verdict'], 'different')
        self.assertEqual({row['side'] for row in result['differences']},
                         {'markdown', 'html'})
        self.assertEqual({row['observed']['value'] for row in result['differences']},
                         {'revised'})

    def test_content_numeral_is_not_a_locator(self):
        streams = self.sides('Section 2 has 4 facts.', 'Section 3 has 5 facts.')
        transforms = {}
        for side, item in streams.items():
            spelling = '2' if side == 'old_pdf' else '3'
            start = item['text'].index(spelling)
            transforms[side] = [dict(kind='heading-locator', identity='section:one',
                                     start=start, end=start + 1, spelling=spelling)]
        proof = {side: dict(verdict='checked', identities={
            ('heading-locator', 'section:one'): '2' if side == 'old_pdf' else '3'})
                 for side in compare_words.STREAM_SIDES}
        result = compare_words.three_way_content(streams, proof, transforms)
        self.assertEqual(result['verdict'], 'different')
        self.assertEqual(result['differences'][0]['observed']['value'], '4')
        self.assertEqual(result['differences'][0]['pdf']['value'], '5')

    def test_widened_locator_cannot_erase_content_numeral(self):
        streams = self.sides('Section 2 has 4 facts.', 'Section 3 has 5 facts.')
        spans = {}
        proof = {}
        for side, item in streams.items():
            spelling = '2 has 4' if side == 'old_pdf' else '3 has 5'
            first = item['text'].index(spelling)
            spans[side] = [dict(kind='heading-locator', identity='section:one',
                                start=first, end=first + len(spelling), spelling=spelling)]
            proof[side] = dict(verdict='checked', identities={
                ('heading-locator', 'section:one'): spelling[0]})
        result = compare_words.three_way_content(streams, proof, spans)
        self.assertEqual(result['verdict'], 'refused')
        self.assertIn('occurrence spelling or proof', result['reason'])

    def test_missing_structural_result_refuses(self):
        proof = self.proofs()
        proof['markdown'] = dict(verdict='unavailable')
        result = compare_words.three_way_content(self.sides('One.', 'One.'), proof)
        self.assertEqual(result['verdict'], 'refused')
        self.assertIn('structural proof', result['reason'])

    def test_partial_screens_refuse_whole_document(self):
        streams = self.sides('One.', 'One.')
        streams['html'] = dict(text='One.', coverage=False, medium='html',
                               residue=['five omitted intervals'])
        result = compare_words.three_way_content(streams, self.proofs())
        self.assertEqual((result['verdict'], result['side']), ('refused', 'html'))
        self.assertEqual(result['residue'], ['five omitted intervals'])

    def test_recovered_screen_requires_declared_selection(self):
        scope = dict(source='documents/paper/explainer/explainer.tex', first=20,
                     last=24, unit='sec:reading:plain-intro')
        emitted = b'The first section.'
        payload = dict(schema=1, medium='markdown', selection=scope,
                       coverage='excerpt', text='The first section.', rendering={
                           'path': 'generated/documents/explainer-pilot/en.md',
                           'sha256': compare_words.digest(emitted)})
        recovered = compare_words.recovered_stream(json.dumps(payload).encode(), emitted)
        streams = self.sides('The first section.', 'The first section.')
        streams['markdown'] = recovered
        other = dict(payload, medium='html', rendering=dict(payload['rendering'],
                                                          path='generated/documents/explainer-pilot/en.html'))
        streams['html'] = compare_words.recovered_stream(json.dumps(other).encode(), emitted)
        self.assertEqual(compare_words.three_way_content(
            streams, self.proofs())['verdict'], 'refused')
        for side in ('old_pdf', 'new_pdf'):
            streams[side]['scope'] = scope
        self.assertEqual(compare_words.three_way_content(
            streams, self.proofs(), selection=scope)['verdict'], 'equal')
        streams['html']['scope'] = dict(scope, first=21)
        self.assertEqual(compare_words.three_way_content(
            streams, self.proofs(), selection=scope)['verdict'], 'refused')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.recovered_stream(json.dumps(payload).encode(), b'changed')
        self.assertEqual(raised.exception.row['code'], 'StreamInvalid')
        streams['html']['scope'] = scope
        streams['html']['medium'] = 'markdown'
        self.assertIn('medium differs', compare_words.three_way_content(
            streams, self.proofs(), selection=scope)['reason'])

    def test_recovered_stream_duplicate_or_missing_field_refuses(self):
        payload = (b'{"schema":1,"schema":1,"medium":"markdown",'
                   b'"selection":{},"coverage":"excerpt",'
                   b'"rendering":{},"text":"word"}')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.recovered_stream(payload, b'word')
        self.assertEqual(raised.exception.row['code'], 'StreamInvalid')
        with self.assertRaises(compare_words.Fault):
            compare_words.recovered_stream(b'\xef\xbb\xbf' + payload, b'word')

    def test_coarse_slot_assignment_is_bounded_and_ordered(self):
        source = b'first\n\nsecond\n'
        blob = hashlib.sha1(b'blob ' + str(len(source)).encode() + b'\0' +
                            source).hexdigest()
        assignment = dict(schema=4, form='coarse-slot', parent='a' * 40,
                          candidate='b' * 40,
                          source='documents/paper/parts/coda.tex',
                          **{'source-blob': blob}, first=1, last=3,
                          unit='sec:reading:coda', blocks=[
                              dict(kind='paragraph', first=1, last=1),
                              dict(kind='display', first=3, last=3)])
        self.assertIs(compare_words.coarse_slot_assignment(assignment, source), assignment)
        altered = dict(assignment, blocks=[assignment['blocks'][1],
                                           assignment['blocks'][0]])
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.coarse_slot_assignment(altered, source)
        self.assertEqual(raised.exception.row['code'], 'AssignmentInvalid')
        with self.assertRaises(compare_words.Fault):
            compare_words.coarse_slot_assignment(assignment, b'first\nchanged\nsecond\n')
        changed_source = b'first\nmiddle\nsecond\n'
        changed_blob = hashlib.sha1(b'blob ' + str(len(changed_source)).encode() +
                                    b'\0' + changed_source).hexdigest()
        nonblank_gap = dict(assignment, **{'source-blob': changed_blob})
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.coarse_slot_assignment(nonblank_gap, changed_source)
        self.assertEqual(raised.exception.row['code'], 'AssignmentInvalid')

    def test_pdf_stretch_uses_source_order_and_refuses_ambiguity(self):
        text = 'Alpha beta. Gamma.'
        extracted = dict(stream=text, offsets=[[1, n] for n in range(len(text))],
                         events=[], raw_pages=[dict(text=text)])
        scope = dict(source='body.tex', first=1, last=2, unit='scope:two')
        blocks = [compare_words.decode_located(value, {})
                  for value in ('Alpha beta.', 'Gamma.')]
        selected = compare_words.select_pdf_stretch(extracted, scope, blocks,
                                                    'explainer', 'new_pdf')
        self.assertEqual((selected['text'], selected['ranges']),
                         (text, [[0, 11], [12, 18]]))
        # A repeated heading alone is resolved by the complete child sequence.
        title_only = text + ' Alpha beta.'
        repeated = dict(extracted, stream=title_only,
                        offsets=[[1, n] for n in range(len(title_only))])
        self.assertEqual(compare_words.select_pdf_stretch(
            repeated, scope, blocks, 'explainer', 'new_pdf')['range'], [0, len(text)])
        twice = text + ' ' + text
        repeated = dict(extracted, stream=twice,
                        offsets=[[1, n] for n in range(len(twice))])
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.select_pdf_stretch(repeated, scope, blocks,
                                             'explainer', 'new_pdf')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        reordered_text = 'Gamma. Alpha beta.'
        reordered = dict(extracted, stream=reordered_text,
                         offsets=[[1, n] for n in range(len(reordered_text))],
                         raw_pages=[dict(text=reordered_text)])
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.select_pdf_stretch(reordered, scope, blocks,
                                             'explainer', 'new_pdf')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')

    def test_slot_selected_discretionary_break_keeps_words(self):
        scope = dict(source='body.tex', first=1, last=2, unit='scope:two')
        blocks = [compare_words.decode_located(value, {})
                  for value in ('Alpha beta.', 'Gamma.')]

        def extracted(text):
            normalized, offsets, events = compare_words.normalize(
                text, [[1, n] for n in range(len(text))])
            return dict(stream=normalized, offsets=offsets, events=events,
                        raw_pages=[dict(text=text)])

        old = compare_words.select_pdf_stretch(
            extracted('Alpha beta. Gamma.'), scope, blocks, 'explainer', 'before')
        new = compare_words.select_pdf_stretch(
            extracted('Al-\npha beta. Gamma.'), scope, blocks, 'explainer', 'after')
        self.assertEqual((old['text'], new['text']),
                         ('Alpha beta. Gamma.', 'Alpha beta. Gamma.'))
        self.assertTrue(any(event['decision'] == 'remove-discretionary'
                            for event in new['aggregate']['decisions']))

    def test_whole_pdf_leg_keeps_numerals_and_order(self):
        def extracted(text):
            return dict(stream=text, offsets=[[1, n] for n in range(len(text))],
                        events=[], titles=['Introduction'])
        same = compare_words.compare_pdf_streams(extracted('Three 3 marks.'),
                                                 extracted('Three 3 marks.'))
        changed = compare_words.compare_pdf_streams(extracted('Three 3 marks.'),
                                                    extracted('Three 4 marks.'))
        self.assertEqual(same['verdict'], 'equal')
        self.assertEqual((changed['verdict'], changed['first_difference']),
                         ('different', 2))

    def test_unresolved_marks_allow_only_independent_difference(self):
        def extracted(text, title='Title'):
            locations = [[1, n] for n in range(len(text))]
            stream, offsets, events = compare_words.normalize(text, locations)
            return dict(stream=stream, offsets=offsets, events=events,
                        raw_pages=[dict(text=text)], titles=[title])
        old = extracted('alpha-\nbeta')
        self.assertEqual(compare_words.compare_pdf_streams(old, old)['unresolved'],
                         {'old_pdf': 1, 'new_pdf': 1})
        changed = compare_words.compare_pdf_streams(old, extracted('gamma-\nbeta'))
        self.assertEqual((changed['verdict'], changed['reason']),
                         ('different', 'symmetric deletion witness differs'))
        heading = compare_words.compare_pdf_streams(old, extracted('alpha-\nbeta', 'New'))
        self.assertEqual((heading['verdict'], heading['reason']),
                         ('different', 'ordered bookmark titles differ'))

    def test_unchecked_pdf_side_and_mark_spacing_refuse_or_differ(self):
        streams = self.sides('a—b', 'a — b')
        self.assertEqual(compare_words.three_way_content(
            streams, self.proofs())['verdict'], 'different')
        streams['old_pdf'] = dict(streams['old_pdf'])
        result = compare_words.three_way_content(streams, self.proofs())
        self.assertEqual(result['verdict'], 'refused')
        self.assertIn('PDF side lacks checked', result['reason'])

    def test_plain_decoder_refuses_slot_only_strong_and_en_space(self):
        source = pathlib.Path(__file__).resolve().parents[2]
        lines = (source/'units/fixtures/explainer-prefix-source.tex').read_text().splitlines()
        body = ' '.join(lines[48:55])
        self.assertIn('\\textbf{Ties.}\\enspace', body)
        for selected in (body, '\\textbf{Unclosed', '\\enspaceword'):
            with self.subTest(selected=selected), self.assertRaises(compare_words.Fault) as raised:
                compare_words.decode_located(selected, {})
            self.assertEqual(raised.exception.row['code'], 'SyntaxUnsupported')


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


class SlotEnvelopeControls(unittest.TestCase):
    @staticmethod
    def fixture(root):
        parent, candidate = root / 'parent', root / 'candidate'
        source = 'documents/paper/explainer/slot.tex'
        old_fragment = 'generated/documents/explainer-pilot/sec-reading-plain-intro.tex'
        new_fragment = 'generated/documents/explainer-pilot/sec-explainer-plain.tex'
        inclusion = 'generated/documents/inclusions.tsv'
        old_source = b'Heading\n% generated: sec:reading:plain-intro\nTies\nWork\nHelp\nEnd\nTail\n'
        new_source = b'% generated: sec:explainer:plain\nTail\n'
        old_body = b'\\PaperNativeParagraph{sec:reading:plain-intro}{Intro}\n'
        new_body = b'\\section{Heading}\n\\label{sec:explainer:plain}\n\n' + old_body
        old_rows = [(source, 'sec:reading:plain-intro', old_fragment, 'paragraph', 2),
                    (source, 'sec:other', 'generated/documents/explainer-pilot/sec-other.tex',
                     'paragraph', 7)]
        new_rows = [(source, 'sec:explainer:plain', new_fragment, 'corpus-slot', 1),
                    (source, 'sec:other', 'generated/documents/explainer-pilot/sec-other.tex',
                     'paragraph', 2)]

        def write(folder, files):
            for name, data in files.items():
                path = folder / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            receipt = {'schema': 1, 'inputs': {name: compare_words.digest(data)
                                               for name, data in files.items()}}
            path = folder / 'generated/paper/inputs.json'
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(compare_words.json_bytes(receipt))
            return receipt

        def encode(rows):
            return ''.join('\t'.join(map(str, row)) + '\n' for row in rows).encode()

        other = 'documents/paper/data/other.tex'
        before = {source: old_source, old_fragment: old_body, inclusion: encode(old_rows),
                  other: b'unchanged'}
        after = {source: new_source, new_fragment: new_body, inclusion: encode(new_rows),
                 other: b'unchanged'}
        shared = 'generated/documents/explainer-pilot/sec-other.tex'
        before[shared] = after[shared] = b'other fragment'
        old_receipt = write(parent, before)
        write(candidate, after)
        parent_record = dict(receipt=old_receipt, fragments=old_rows,
                             inputs=[dict(path=name) for name in before])
        candidate_inputs = [dict(path=name) for name in after]
        # A captured origin need not itself be a receipt input; compare its
        # bytes across sides without inventing a receipt row for it.
        extra = 'generated/manifest.json'
        for folder in (parent, candidate):
            path = folder / extra
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b'{"schema":1}\n')
        parent_record['inputs'].append(dict(path=extra))
        candidate_inputs.append(dict(path=extra))
        slot = dict(source=source, first=1, last=6, unit='sec:explainer:plain')
        return parent_record, parent, candidate, candidate_inputs, slot

    def test_slot_receipt_and_nested_use_substitution(self):
        with tempfile.TemporaryDirectory() as directory:
            rows = self.fixture(pathlib.Path(directory))
            result = compare_words.slot_source_envelope(*rows)
            self.assertEqual((result['available'], result['nested_use'],
                              result['inclusion_shift']),
                             (True, 'sec:reading:plain-intro', -5))
            candidate = rows[2]
            changed = candidate / 'documents/paper/data/other.tex'
            changed.write_bytes(b'changed macro')
            result = compare_words.slot_source_envelope(*rows)
            self.assertIn('slot outside input differs: documents/paper/data/other.tex',
                          result['reasons'])

    def test_slot_shift_child_and_receipt_refusals(self):
        changes = [
            ('generated/documents/inclusions.tsv', b'\tparagraph\t2\n',
             b'\tparagraph\t3\n', 'slot inclusion rows differ'),
            ('generated/documents/explainer-pilot/sec-explainer-plain.tex',
             b'{Intro}', b'{Changed}', 'slot child fragment is not unchanged exactly once'),
            ('documents/paper/explainer/slot.tex', b'Tail', b'Other',
             'slot source differs outside marker substitution'),
        ]
        for name, old, new, expected in changes:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                rows = self.fixture(pathlib.Path(directory))
                candidate = rows[2]
                path = candidate / name
                data = path.read_bytes()
                self.assertIn(old, data)
                path.write_bytes(data.replace(old, new, 1))
                receipt_path = candidate / 'generated/paper/inputs.json'
                receipt = json.loads(receipt_path.read_text())
                receipt['inputs'][name] = compare_words.digest(path.read_bytes())
                receipt_path.write_bytes(compare_words.json_bytes(receipt))
                self.assertIn(expected,
                              compare_words.slot_source_envelope(*rows)['reasons'])
        with tempfile.TemporaryDirectory() as directory:
            rows = self.fixture(pathlib.Path(directory))
            receipt_path = rows[2] / 'generated/paper/inputs.json'
            receipt = json.loads(receipt_path.read_text())
            receipt['inputs'][rows[4]['source']] = '0' * 64
            receipt_path.write_bytes(compare_words.json_bytes(receipt))
            self.assertIn('candidate receipt row differs: ' + rows[4]['source'],
                          compare_words.slot_source_envelope(*rows)['reasons'])


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

    pdf_evidence(repo, binary, work)


def slot_pdf_evidence(repo, binary, work):
    """Measure the transferred prefix through each side's located native export."""
    unit_path = 'units/corpus/explainer-plain.toml'
    additions = compare_words.git(repo, 'log', '--format=%H', '--diff-filter=A',
                                  '--', unit_path).decode().splitlines()
    if len(additions) != 1:
        raise AssertionError('expected one commit admitting the transferred slot')
    parent = compare_words.git(repo, 'rev-parse', additions[0] + '^').decode().strip()
    candidate = compare_words.git(repo, 'rev-parse', 'HEAD').decode().strip()
    source = 'documents/paper/explainer/explainer.tex'
    before_lines = compare_words.git(repo, 'show', parent + ':' + source).splitlines(
        keepends=True)
    after_lines = compare_words.git(repo, 'show', additions[0] + ':' + source).splitlines(
        keepends=True)
    marker = b'% generated: sec:explainer:plain\n'
    changes = [(i1, i2, j1, j2) for op, i1, i2, j1, j2 in
               difflib.SequenceMatcher(None, before_lines, after_lines,
                                       autojunk=False).get_opcodes() if op != 'equal']
    if len(changes) != 1:
        raise AssertionError('expected one source interval replaced by the slot marker')
    i1, i2, j1, j2 = changes[0]
    if after_lines[j1:j2] != [marker] or i2 - i1 != 55:
        raise AssertionError('transfer source interval or marker differs')
    first, last = i1 + 1, i2
    assignment = dict(schema=4, form='coarse-slot', parent=parent, candidate=candidate,
                      source=source,
                      **{'source-blob': compare_words.git(
                          repo, 'rev-parse', parent + ':' + source).decode().strip()},
                      first=first, last=last, unit='sec:explainer:plain', blocks=[
                          dict(kind=kind, first=first + start, last=first + end)
                          for kind, start, end in (
                              ('heading', 0, 2), ('existing-native-inclusion', 3, 4),
                              ('paragraph', 5, 11), ('paragraph', 12, 24),
                              ('paragraph', 25, 40), ('paragraph', 41, 54))])
    selection = {key: assignment[key] for key in ('source', 'first', 'last', 'unit')}
    with tempfile.TemporaryDirectory(dir=work) as directory:
        root = pathlib.Path(directory)
        request = root / 'slot-assignment.json'
        request.write_text(json.dumps(assignment), encoding='utf-8')
        old_pdf = root / 'old-explainer.pdf'
        old_pdf.write_bytes(compare_words.git(
            repo, 'show', parent + ':generated/paper/explainer.pdf'))
        result = {}
        for side, revision, pdf_path in (
                ('before', parent, old_pdf),
                ('after', candidate, repo / 'generated/paper/explainer.pdf')):
            side_root = root / (side + '-root')
            command('git', 'clone', '--quiet', '--shared', '--no-checkout',
                    str(repo), str(side_root))
            command('git', '-C', str(side_root), 'checkout', '--quiet', '--detach', revision)
            captured = {name: (side_root / name).read_bytes() for name in (
                source, 'units/statements/plain-intro.toml')}
            if side == 'after':
                name = 'units/corpus/explainer-plain.toml'
                captured[name] = (side_root / name).read_bytes()
            output = root / (side + '-output')
            output.mkdir()
            data = compare_words._slot_export(
                binary, repo, side_root, revision, request, side, output)
            blocks = compare_words.located_slot_export(
                data, captured, assignment, side, revision)
            pdf_side = 'old_pdf' if side == 'before' else 'new_pdf'
            extracted = compare_words.extract(pdf_path, 'explainer', pdf_side)
            selected = compare_words.select_pdf_stretch(
                extracted, selection, blocks, 'explainer', pdf_side)
            events = compare_words.line_events(extracted, 'explainer', pdf_side)
            decisions = {row['normalized_offset']
                         for row in selected['aggregate']['decisions']}
            inside = [row for row in events if selected['range'][0] <=
                      row['normalized_offset'] < selected['range'][1]]
            result[side] = dict(pdf_range=selected['range'], children=len(blocks),
                                line_end_marks=len(inside),
                                source_decided=sum(row['normalized_offset'] in decisions
                                                   for row in inside))
        return result


def pdf_evidence(repo, binary, work):
    """Measure the whole delivered PDF leg and its source-decision residue."""
    started = time.monotonic()
    envelope = compare_words.unchanged_commit_envelope(repo, 'HEAD')
    result = {}
    for document in ('paper', 'explainer'):
        pdf_path = repo / 'generated/paper' / (document + '.pdf')
        if pdf_path.read_bytes() != compare_words.git(
                repo, 'show', 'HEAD:generated/paper/' + document + '.pdf'):
            raise AssertionError('delivered PDF differs from the committed output')
        pdf = compare_words.extract(pdf_path,
                                    document, 'new_pdf')
        pair = compare_words.compare_pdf_streams(pdf, pdf, envelope=envelope,
                                                 document=document)
        result[document] = dict(pair=pair, pages=len(pdf['raw_pages']))
        if pair['verdict'] != 'equal' or any(pair['unresolved'].values()):
            raise AssertionError('committed self-pair did not resolve residuals')
        if document == 'explainer':
            result[document]['transferred_slot'] = slot_pdf_evidence(repo, binary, work)
    print(json.dumps(dict(pdf_evidence=result,
                          screen_leg='recovered stream export required',
                          wall_seconds=round(time.monotonic() - started, 3)),
                     sort_keys=True))


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
