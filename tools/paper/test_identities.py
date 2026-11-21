"""Refusal controls for finite bindings and rendered-label leakage."""
import pathlib
import json
import collections
import shutil
import tempfile
import unittest
import re
import tomllib
import copy
import contextlib
import io
from unittest import mock
import argparse
from common import read, write, timed
import compare_words
from identities import _assignment_line, assembly_json, bindings, holds
from ledger import leaks, occurrence_differences, ledger_binding, check_destination
from prepare import declared_fragments, versioned, paper_master, scope_fragments

FRAGMENTS = tuple(row[:4] for row in scope_fragments(pathlib.Path(__file__).resolve().parents[2]))


class Identities(unittest.TestCase):
    def conversion_run(self, change=None, options=None):
        source = pathlib.Path(__file__).resolve().parents[2]
        fragment = read(source/'generated/documents/paper-pilot/sec-reading-shadow.tex')
        body = fragment.split('}{', 1)[1].removesuffix('}\n')
        rendered, _, _ = compare_words.decode_paragraph(body, bindings(source))
        paper = 'SRC:ND ' + rendered + ' Closing'
        pair = {
            'paper': {'stream': paper, 'offsets': [[1, i] for i in range(len(paper))],
                      'events': [], 'titles': ['Paper'], 'raw_pages': [], 'raw_bookmarks': [],
                      'folios': []},
            'explainer': {'stream': 'Companion text', 'offsets': [[1, i] for i in range(14)],
                          'events': [], 'titles': ['Explainer'], 'raw_pages': [],
                          'raw_bookmarks': [], 'folios': []},
        }
        output = self.root / ('comparison-' + str(len(list(self.root.glob('comparison-*')))))
        before = {}
        for document in ('paper', 'explainer'):
            path = self.root / ('before-' + document + '.pdf')
            if not path.exists():
                path.write_bytes(compare_words.git(
                    source, 'show', '68643f6:generated/paper/' + document + '.pdf'))
            before[document] = path
        args = argparse.Namespace(repository=source, before_paper=before['paper'],
                                  before_explainer=before['explainer'],
                                  after_paper=source/'generated/paper/paper.pdf',
                                  after_explainer=source/'generated/paper/explainer.pdf',
                                  before_source='68643f6:documents/paper/parts/front.tex:203-207',
                                  unit='sec:reading:shadow', output=output)
        for key, value in (options or {}).items():
            setattr(args, key, value)
        def fake_extract(path, document, side):
            value = copy.deepcopy(pair[document])
            if change is not None:
                change(path, document, side, value)
            return value
        with mock.patch.object(compare_words, 'extract', fake_extract), mock.patch.object(
                compare_words, 'extractor_identity', return_value={'fixed': True}):
            exit_code = compare_words.compare(args)
        return exit_code, json.loads((output/'comparison.json').read_text())

    def test_conversion_complete_stream(self):
        exit_code, result = self.conversion_run()
        self.assertEqual((exit_code, result['verdict']), (0, 'equal'))
        for document in ('paper', 'explainer'):
            for place in ('first', 'middle', 'last'):
                for mutation in ('insert', 'delete', 'replace'):
                    def changed(path, name, side, value):
                        if name == document and side == 'after':
                            text = value['stream']
                            at = {'first': 0, 'middle': 4,
                                  'last': len(text) - 1}[place]
                            value['stream'] = {'insert': text[:at] + '!' + text[at:],
                                               'delete': text[:at] + text[at + 1:],
                                               'replace': text[:at] + '!' + text[at + 1:]}[mutation]
                            value['offsets'] = [[1, i] for i in range(len(value['stream']))]
                    exit_code, result = self.conversion_run(changed)
                    self.assertEqual((exit_code, result['verdict']), (1, 'different'))
                    self.assertEqual(result['differences'][0]['code'], 'TextDifference')
                    self.assertEqual(result['differences'][0]['document'], document)
                    self.assertEqual(result['differences'][0]['part'], 'stream')
        def bookmark(path, document, side, value):
            if document == 'paper' and side == 'after':
                value['titles'][0] = 'Changed'
        exit_code, result = self.conversion_run(bookmark)
        self.assertEqual((exit_code, result['verdict']), (1, 'different'))
        self.assertEqual(result['differences'][0]['part'], 'titles')
        def case_only(path, document, side, value):
            if document == 'paper' and side == 'after':
                value['stream'] = value['stream'].replace('SRC:ND ', 'src:nd ', 1)
        exit_code, result = self.conversion_run(case_only)
        self.assertEqual((exit_code, result['verdict']), (1, 'different'))
        self.assertEqual(result['differences'][0]['part'], 'stream')
        for old, new in (('SRC:ND', 'SRC:NE'), ('Closing', 'Closing.'),
                         ('Closing', 'Closing2')):
            def changed_value(path, document, side, value):
                if document == 'paper' and side == 'after':
                    value['stream'] = value['stream'].replace(old, new, 1)
                    value['offsets'] = [[1, i] for i in range(len(value['stream']))]
            exit_code, result = self.conversion_run(changed_value)
            self.assertEqual((exit_code, result['verdict']), (1, 'different'))
            self.assertEqual(result['differences'][0]['part'], 'stream')
        for mark in ('-', '—', '/'):
            def reflow(path, document, side, value):
                if document == 'explainer' and side == 'after':
                    value['stream'] = value['stream'].replace('Companion', 'pre' + mark + 'and')
                    value['offsets'] = [[1, i] for i in range(len(value['stream']))]
                elif document == 'explainer':
                    value['stream'] = value['stream'].replace('Companion', 'pre' + mark + ' and')
                    value['offsets'] = [[1, i] for i in range(len(value['stream']))]
            exit_code, result = self.conversion_run(reflow)
            self.assertEqual((exit_code, result['verdict']), (1, 'different'))
            self.assertEqual(result['differences'][0]['part'], 'stream')

    def test_conversion_furniture_and_reflow(self):
        page = {'width': 210*72/25.4, 'height': 297*72/25.4,
                'label': '1', 'text': 'A 1\n1', 'rectangles':
                [(90, 80, 94, 90), (94, 80, 98, 90), (98, 80, 102, 90),
                 (102, 80, 106, 90), (295, 790, 300, 802)]}
        self.assertEqual(compare_words.folio(page, 1, 'paper', 'before')[2], '1')
        for changed, code in ((dict(label='2'), 'ProfileUnsupported'),
                              (dict(text='A 1\n2'), 'FurnitureInvalid'),
                              (dict(width=500), 'ProfileUnsupported')):
            altered = dict(page, **changed)
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.folio(altered, 1, 'paper', 'before')
            self.assertEqual(raised.exception.row['code'], code)
            self.assertEqual(raised.exception.row['page'], 1)
        duplicated = dict(page, text='A 1\n1\n1', rectangles=page['rectangles'] +
                          [(295, 800, 300, 812), (295, 810, 300, 822)])
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.folio(duplicated, 1, 'paper', 'after')
        self.assertEqual(raised.exception.row['code'], 'FurnitureInvalid')
        for rectangles in (page['rectangles'][:-1] + [(295, 740, 300, 752)],
                           page['rectangles'][:-1] + [(100, 790, 105, 802)]):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.folio(dict(page, rectangles=rectangles), 1, 'paper', 'after')
            self.assertEqual(raised.exception.row['code'], 'FurnitureInvalid')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.folio(dict(page, text='A 1\n',
                                     rectangles=page['rectangles'][:-1]), 1, 'paper', 'after')
        self.assertEqual(raised.exception.row['code'], 'FurnitureInvalid')
        text, _, _ = compare_words.normalize('A 1\nB', [[1, i] for i in range(5)])
        self.assertEqual(text, 'A 1 B')
        before, _, _ = compare_words.normalize('Edge words 7\ncontinue',
                                               [[1, i] for i in range(21)])
        after, _, _ = compare_words.normalize('Edge words 7 continue',
                                              [[2, i] for i in range(21)])
        self.assertEqual(before, after)
        changed_digit, _, _ = compare_words.normalize('Edge words 8 continue',
                                                     [[2, i] for i in range(21)])
        self.assertNotEqual(before, changed_digit)

    def test_conversion_correspondence(self):
        raw = 'the de-\npendence and pre-\nand'
        normalized, offsets, events = compare_words.normalize(
            raw, [[1, i] for i in range(len(raw))])
        self.assertEqual(normalized, 'the de-pendence and pre-and')
        extracted = {'stream': normalized, 'offsets': offsets, 'events': events}
        match = compare_words.corroborate('the dependence and pre-and',
                                           extracted, 'paper', 'before')
        self.assertEqual(len(match['confirmed_joins']), 1)
        self.assertEqual(match['confirmed_joins'][0]['joined'], 'dependence')
        self.assertEqual(match['confirmed_joins'][0]['extracted'], 'de-pendence')
        lexical = compare_words.corroborate('the de-pendence and pre-and',
                                            extracted, 'paper', 'after')
        self.assertEqual(lexical['confirmed_joins'], [])
        page_break = 'de-\npendence'
        locations = [[1, i] for i in range(4)] + [[2, i] for i in range(8)]
        across, across_offsets, across_events = compare_words.normalize(page_break, locations)
        confirmed = compare_words.corroborate('dependence',
                                               {'stream': across, 'offsets': across_offsets,
                                                'events': across_events}, 'paper', 'before')
        self.assertEqual(confirmed['confirmed_joins'][0]['raw'][0], 1)
        suspended = 'the pre-\nand'
        suspended_stream, suspended_offsets, suspended_events = compare_words.normalize(
            suspended, [[1, i] for i in range(len(suspended))])
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.corroborate('the pre- and',
                                      {'stream': suspended_stream, 'offsets': suspended_offsets,
                                       'events': suspended_events}, 'paper', 'before')
        self.assertEqual((raised.exception.row['code'], raised.exception.row['stage']),
                         ('CorrespondenceInvalid', 'correspondence'))
        digits, _, _ = compare_words.normalize('rate 1\n2', [[1, i] for i in range(8)])
        self.assertEqual(digits, 'rate 1 2')
        repeated = {'stream': normalized + ' ' + normalized,
                    'offsets': [[1, i] for i in range(2 * len(normalized) + 1)],
                    'events': []}
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.corroborate('the de-pendence and pre-and',
                                      repeated, 'paper', 'before')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        embedded = {'stream': 'breathe dependence',
                    'offsets': [[1, i] for i in range(len('breathe dependence'))],
                    'events': []}
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.corroborate('the dependence', embedded, 'paper', 'before')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.corroborate('the dependance and pre-and',
                                       extracted, 'paper', 'before')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.decode_paragraph('a \\unknown{b}', {})
        self.assertEqual(raised.exception.row['code'], 'SyntaxUnsupported')

        for scalar in ('~', '#', '&', '^', '_', '$', '%', '{', '}', '\\', '`', '"'):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.decode_paragraph('a' + scalar + 'b', {})
            self.assertEqual(raised.exception.row['code'], 'SyntaxUnsupported')
            self.assertEqual(raised.exception.row['stage'], 'snapshot')
        for source_text in ('a--b', "a''b"):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.decode_paragraph(source_text, {})
            self.assertEqual(raised.exception.row['code'], 'SyntaxUnsupported')
        source = pathlib.Path(__file__).resolve().parents[2]
        lookup = bindings(source)
        wrong, _, _ = compare_words.decode_paragraph(
            'The ground is \\ftag{snk:two}.', lookup)
        original, _, _ = compare_words.decode_paragraph(
            'The ground is \\ftag{src:nd}.', lookup)
        self.assertNotEqual(wrong, original)
        value = {'stream': original, 'offsets': [[1, i] for i in range(len(original))],
                 'events': []}
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.corroborate(wrong, value, 'paper', 'after')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        def uncertain(path, document, side, value):
            if document == 'paper' and side == 'after':
                value['stream'] = value['stream'].replace('Closing', 'C-losing')
                position = value['stream'].rindex('C-losing') + 1
                value['offsets'] = [[1, i] for i in range(len(value['stream']))]
                value['events'] = [{'operation': 'HyphenUnconfirmed',
                                    'normalized_offset': position, 'raw': [1, position]}]
        exit_code, result = self.conversion_run(uncertain)
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'HyphenUnconfirmed')
        for broken_side in ('before', 'after'):
            for spelling in ('non-linear', 'pre-and'):
                def break_event(path, document, side, value):
                    if document == 'paper':
                        value['stream'] = value['stream'].replace('Closing', spelling)
                        value['offsets'] = [[1, i] for i in range(len(value['stream']))]
                        if side == broken_side:
                            position = value['stream'].index('-')
                            value['events'] = [{'operation': 'HyphenUnconfirmed',
                                                'normalized_offset': position,
                                                'raw': [1, position]}]
                exit_code, result = self.conversion_run(break_event)
                self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
                self.assertEqual(result['faults'][0]['code'], 'HyphenUnconfirmed')
        for mark in ('-', '—', '/'):
            def added_space(path, document, side, value):
                if document == 'paper':
                    value['stream'] = value['stream'].replace('Closing', 'pre' + mark + ' and')
                    value['offsets'] = [[1, i] for i in range(len(value['stream']))]
                    position = value['stream'].index(mark)
                    value['events'] = [{'operation': 'LineEndPunctuation',
                                        'normalized_offset': position,
                                        'raw': [1, position], 'spelling': mark,
                                        'intervening_space': side == 'after'}]
            exit_code, result = self.conversion_run(added_space)
            self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
            self.assertEqual(result['faults'][0]['code'], 'ProfileUnsupported')
        for mark in ('–', '—'):
            def one_sided(path, document, side, value):
                if document == 'paper':
                    value['stream'] = value['stream'].replace('Closing', 'pre ' + mark + ' and')
                    value['offsets'] = [[1, i] for i in range(len(value['stream']))]
                    if side == 'after':
                        position = value['stream'].index(mark)
                        value['events'] = [{'operation': 'LineEndPunctuation',
                                            'normalized_offset': position, 'raw': [1, position],
                                            'spelling': mark, 'intervening_space': False}]
            exit_code, result = self.conversion_run(one_sided)
            self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
            self.assertEqual(result['faults'][0]['code'], 'ProfileUnsupported')
        def changed_mark(path, document, side, value):
            if document == 'paper':
                value['stream'] = value['stream'].replace('Closing', 'pre—and')
                value['offsets'] = [[1, i] for i in range(len(value['stream']))]
                position = value['stream'].index('—')
                value['events'] = [{'operation': 'LineEndPunctuation',
                                    'normalized_offset': position, 'raw': [1, position],
                                    'spelling': '—' if side == 'before' else '/',
                                    'intervening_space': False}]
        exit_code, result = self.conversion_run(changed_mark)
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'ProfileUnsupported')

    def test_conversion_reader_nulls_and_cli(self):
        reader = object.__new__(compare_words.Pdf)
        reader.count = 0
        with self.assertRaisesRegex(ValueError, 'PDF has no pages'):
            reader.raw_layout_pages()
        reader.count = 1
        reader.doc = None
        reader.get_page = lambda doc, index: None
        with self.assertRaisesRegex(ValueError, 'PDF page unavailable'):
            reader.raw_layout_pages()
        reader.get_page = lambda doc, index: 1
        reader.get_text = lambda page: None
        reader.rectangle = compare_words.ctypes.c_int
        reader.unref = lambda page: None
        with self.assertRaisesRegex(ValueError, 'PDF text unavailable'):
            reader.raw_layout_pages()
        text = compare_words.ctypes.create_string_buffer(b'one')
        reader.get_text = lambda page: compare_words.ctypes.addressof(text)
        reader.layout = lambda page, areas, count: 0
        reader.free = lambda value: None
        with self.assertRaisesRegex(ValueError, 'PDF text layout unavailable'):
            reader.raw_layout_pages()
        empty = compare_words.ctypes.create_string_buffer(b'')
        reader.get_text = lambda page: compare_words.ctypes.addressof(empty)
        reader.layout = lambda page, areas, count: 1
        def page_size(page, width, height):
            width._obj.value = 595
            height._obj.value = 842
        reader.page_size = page_size
        reader.page_label = lambda page: None
        with self.assertRaisesRegex(ValueError, 'PDF page label unavailable'):
            reader.raw_layout_pages()
        arguments = ['compare_words.py', 'compare', '--repository', '.',
                     '--before-paper', 'a.pdf', '--before-explainer', 'b.pdf',
                     '--after-paper', 'c.pdf', '--after-explainer', 'd.pdf',
                     '--before-source', 'HEAD:a:1-1', '--unit', 'sec:reading:shadow',
                     '--output', '/tmp/absent-evidence']
        for verdict in (0, 1, 2):
            with mock.patch.object(compare_words.sys, 'argv', arguments), mock.patch.object(
                    compare_words, 'compare', return_value=verdict):
                self.assertEqual(compare_words.main(), verdict)
        dump_args = ['compare_words.py', 'dump', '--pdf', 'unreadable.pdf']
        for failure in (compare_words.Fault('ExtractionUnavailable', 'missing PDF'),
                        RuntimeError('reader failed')):
            with mock.patch.object(compare_words.sys, 'argv', dump_args), mock.patch.object(
                    compare_words, 'pdf_libraries', return_value={}), mock.patch.object(
                    compare_words, 'Pdf', side_effect=failure):
                self.assertEqual(compare_words.main(), 2)

    def test_conversion_extractor_and_characters(self):
        for value in ('\ufffd', '\u00ad', '\ufb05', '\u200b', '\ue000',
                      '\u0001', '\u0378'):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.check_characters(value, 'paper', 'before', 3)
            self.assertEqual(raised.exception.row['code'], 'ExtractionAmbiguous')
            self.assertEqual((raised.exception.row['page'], raised.exception.row['offset']), (3, 0))
        for before, after in compare_words.LIGATURES.items():
            value, _, events = compare_words.normalize(before, [[1, 0]])
            self.assertEqual(value, after)
            self.assertEqual(events[0]['operation'], 'ligature')
        self.assertEqual(compare_words.normalize('¹', [[1, 0]])[0], '¹')
        self.assertEqual(compare_words.normalize('AbC', [[1, i] for i in range(3)])[0], 'AbC')
        self.assertNotEqual(compare_words.normalize('AbC', [[1, i] for i in range(3)])[0],
                            compare_words.normalize('ABC', [[1, i] for i in range(3)])[0])
        for raw in ('pre-\n1', 'pre- \n1', 'pre/\nand', 'pre–\nand', 'pre—\nand'):
            stream, _, events = compare_words.normalize(raw, [[1, i] for i in range(len(raw))])
            self.assertTrue(any(row['operation'] == 'LineEndPunctuation' for row in events))
            self.assertFalse(any(row['operation'] == 'HyphenUnconfirmed' for row in events))
            self.assertIn('pre', stream)
        digits, _, events = compare_words.normalize('1-\n2', [[1, i] for i in range(4)])
        self.assertEqual(digits, '1- 2')
        self.assertFalse(any(row['operation'] == 'HyphenUnconfirmed' for row in events))
        raw = [{'text': 'ab', 'rectangles': [(0, 0, 4, 10), (7, 0, 11, 10)]},
               {'text': 'ab', 'rectangles': [(0, 0, 4, 10), (7.01, 0, 11.01, 10)]},
               {'text': 'ab', 'rectangles': [(0, 0, 4, 10), (4, 0, 8, 10.5)]},
               {'text': 'ab', 'rectangles': [(0, 0, 4, 10), (4, 0, 8, 10.51)]}]
        reader = object.__new__(compare_words.Pdf)
        with mock.patch.object(reader, 'raw_layout_pages', return_value=raw):
            self.assertEqual(reader.pages(), ['ab', 'a b', 'ab', 'a b'])
        for failure in (OSError('missing library'), ValueError('null page'),
                        UnicodeError('invalid UTF-8'), RuntimeError('invalid layout')):
            class BrokenPdf:
                def __init__(self, path):
                    if isinstance(failure, OSError):
                        raise failure
                def raw_layout_pages(self):
                    raise failure
            with mock.patch.object(compare_words, 'Pdf', BrokenPdf):
                with self.assertRaises(compare_words.Fault) as raised:
                    compare_words.extract(pathlib.Path('unused.pdf'), 'explainer', 'after')
            self.assertEqual(raised.exception.row['code'], 'ExtractionUnavailable')
            self.assertEqual((raised.exception.row['document'],
                              raised.exception.row['side']), ('explainer', 'after'))
        class MissingVersion:
            def __init__(self):
                self.poppler_get_version = lambda: None
        with mock.patch.object(compare_words, 'pdf_libraries', return_value={}), mock.patch.object(
                compare_words.ctypes, 'CDLL', return_value=MissingVersion()):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.extractor_identity()
        self.assertEqual(raised.exception.row['code'], 'ExtractionUnavailable')

    def test_conversion_source_boundaries_and_syntax(self):
        source = pathlib.Path(__file__).resolve().parents[2]
        def fake_git(repo, *args):
            return b'\nfirst\n \t\nsecond\n\n' if args[0] == 'show' else b'fixture\n'
        with mock.patch.object(compare_words, 'git', fake_git):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.source_blob(source, 'HEAD:paragraph.tex:2-4')
        self.assertEqual(raised.exception.row['code'], 'SourceInvalid')
        self.assertEqual(raised.exception.row['detail'], 'source selector spans paragraphs')
        def fake_cr(repo, *args):
            return b'\nfirst\ronly\n\n' if args[0] == 'show' else b'fixture\n'
        with mock.patch.object(compare_words, 'git', fake_cr):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.source_blob(source, 'HEAD:paragraph.tex:2-2')
        self.assertEqual(raised.exception.row['code'], 'SourceInvalid')
        self.assertEqual(raised.exception.row['detail'], 'source has an unsupported line ending')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.decode_paragraph('a----b', {})
        self.assertEqual(raised.exception.row['code'], 'SyntaxUnsupported')

    def test_conversion_captured_inputs(self):
        source = pathlib.Path(__file__).resolve().parents[2]
        invalid_before = self.root / 'invalid-before.pdf'
        invalid_before.write_bytes(compare_words.git(
            source, 'show', '68643f6:generated/paper/paper.pdf') + b'!')
        exit_code, result = self.conversion_run(options={'before_paper': invalid_before})
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'InputInvalid')
        invalid_after = self.root / 'invalid-after.pdf'
        invalid_after.write_bytes((source / 'generated/paper/paper.pdf').read_bytes() + b'!')
        exit_code, result = self.conversion_run(options={'after_paper': invalid_after})
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'InputInvalid')
        receipt_path = source / 'generated/paper/inputs.json'
        original_read = pathlib.Path.read_bytes
        def changed_receipt(path):
            data = original_read(path)
            if path == receipt_path:
                record = json.loads(data)
                first = next(iter(record['inputs']))
                record['inputs'][first] = '0' * 64
                return compare_words.json_bytes(record)
            return data
        with mock.patch.object(pathlib.Path, 'read_bytes', changed_receipt):
            exit_code, result = self.conversion_run()
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual((result['faults'][0]['code'], result['faults'][0]['stage'],
                          result['faults'][0]['detail']),
                         ('InputChanged', 'snapshot', 'paper receipt differs from source inventory'))
        def changed_snapshot(path, document, side, value):
            if document == 'paper' and side == 'after':
                target = path.parent / 'snapshot/generated/documents/paper-pilot/sec-reading-shadow.tex'
                target.write_bytes(target.read_bytes() + b'!')
        exit_code, result = self.conversion_run(changed_snapshot)
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'InputChanged')
        def removed_snapshot(path, document, side, value):
            if document == 'paper' and side == 'after':
                (path.parent / 'snapshot/generated/documents/paper-pilot/sec-reading-shadow.tex').unlink()
        exit_code, result = self.conversion_run(removed_snapshot)
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual((result['faults'][0]['code'], result['faults'][0]['stage']),
                         ('InputChanged', 'final'))
        with mock.patch.object(compare_words.CapturedInputs, 'verify',
                               side_effect=ValueError('captured input changed')):
            exit_code, result = self.conversion_run()
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'InputChanged')
        def changed_copy(path, document, side, value):
            if document == 'paper' and side == 'after':
                path.write_bytes(path.read_bytes() + b'!')
        exit_code, result = self.conversion_run(changed_copy)
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'InputChanged')
        self.assertEqual((result['faults'][0]['document'],
                          result['faults'][0]['side']), ('paper', 'after'))
        original = source/'generated/paper/paper.pdf'
        changed_original = self.root/'changed-original.pdf'
        shutil.copyfile(original, changed_original)
        def change_original(path, document, side, value):
            if document == 'paper' and side == 'after':
                changed_original.write_bytes(changed_original.read_bytes() + b'!')
        exit_code, result = self.conversion_run(
            change_original, {'after_paper': changed_original})
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'InputChanged')
        copied_after = self.root / 'copied-after.pdf'
        shutil.copyfile(original, copied_after)
        changed_tree = {'now': False}
        def tree_changed(path, document, side, value):
            if document == 'paper' and side == 'after':
                changed_tree['now'] = True
        def read_changed_tree(path):
            data = original_read(path)
            if changed_tree['now'] and path == original:
                return data + b'!'
            return data
        with mock.patch.object(pathlib.Path, 'read_bytes', read_changed_tree):
            exit_code, result = self.conversion_run(
                tree_changed, {'after_paper': copied_after})
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual((result['faults'][0]['code'], result['faults'][0]['stage'],
                          result['faults'][0]['detail']),
                         ('InputChanged', 'final', 'candidate PDF changed after capture'))
        historical = compare_words.source_blob(
            source, '68643f6:documents/paper/parts/front.tex:203-207')
        self.assertIn('One may assume naively', historical['body'])
        self.assertNotIn('One may assume naively',
                         ''.join((source/'documents/paper/parts/front.tex').read_text().splitlines(
                             keepends=True)[202:207]))
        for selector in ('-bad:paragraph.tex:1-2', '68643f6:/escape:1-2',
                         '68643f6:documents/paper/parts/front.tex:203-206'):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.source_blob(source, selector)
            self.assertEqual(raised.exception.row['code'], 'SourceInvalid')
        exit_code, result = self.conversion_run(options={'unit': 'sec:absent'})
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'UnitInvalid')
        exit_code, result = self.conversion_run(options={'unit': 'sec:say:direction'})
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'UnitInvalid')
        with self.assertRaises(compare_words.Fault) as raised:
            self.conversion_run(options={'output': self.root})
        self.assertEqual(raised.exception.row['code'], 'OutputExists')
        original_write = pathlib.Path.write_bytes
        def write(path, data):
            if path.name == 'comparison.json':
                raise OSError('evidence write refused')
            return original_write(path, data)
        with mock.patch.object(pathlib.Path, 'write_bytes', write):
            with self.assertRaises(compare_words.Fault) as raised:
                self.conversion_run()
        self.assertEqual(raised.exception.row['code'], 'EvidenceUnavailable')
        def refuse_pdf(path, data):
            if path.name == 'paper-after.pdf':
                raise OSError('PDF evidence write refused')
            return original_write(path, data)
        with mock.patch.object(pathlib.Path, 'write_bytes', refuse_pdf):
            exit_code, result = self.conversion_run()
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'EvidenceUnavailable')
        def refuse_source(path, data):
            if path.name == 'source.blob':
                raise OSError('source evidence write refused')
            return original_write(path, data)
        with mock.patch.object(pathlib.Path, 'write_bytes', refuse_source):
            exit_code, result = self.conversion_run()
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'EvidenceUnavailable')
        original_mkdir = pathlib.Path.mkdir
        def refuse_snapshot_directory(path, *args, **kwargs):
            if path.name == 'paper-build' and path.parent.name == 'documents' and 'snapshot' in path.parts:
                raise OSError('snapshot directory refused')
            return original_mkdir(path, *args, **kwargs)
        with mock.patch.object(pathlib.Path, 'mkdir', refuse_snapshot_directory):
            exit_code, result = self.conversion_run()
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual((result['faults'][0]['code'], result['faults'][0]['stage']),
                         ('EvidenceUnavailable', 'snapshot'))
        def remove_original(path, document, side, value):
            if document == 'paper' and side == 'after':
                missing_original.unlink()
        missing_original = self.root / 'removed-after.pdf'
        shutil.copyfile(original, missing_original)
        exit_code, result = self.conversion_run(
            remove_original, {'after_paper': missing_original})
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'InputChanged')
        diagnostic = io.StringIO()
        with contextlib.redirect_stderr(diagnostic), mock.patch.object(
                compare_words, 'inventory', side_effect=SyntaxError('bad inventory')):
            exit_code, result = self.conversion_run()
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'PreparationRefused')
        self.assertIn('M077: PreparationRefused: -/-: -:-: bad inventory',
                      diagnostic.getvalue())
        with mock.patch.object(compare_words, 'corroborate', side_effect=RuntimeError('bad correspondence')):
            exit_code, result = self.conversion_run()
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual((result['faults'][0]['code'], result['faults'][0]['stage']),
                         ('PreparationRefused', 'correspondence'))

    def test_declared_fragment_slots_refuse_label_marker_and_authored_copy(self):
        source=pathlib.Path(__file__).resolve().parents[2]
        inclusion='generated/documents/inclusions.tsv'
        target=self.root/inclusion; target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(source/inclusion,target)
        for path, _, fragment, _ in FRAGMENTS:
            for name in (path, fragment):
                target=self.root/name; target.parent.mkdir(parents=True,exist_ok=True)
                shutil.copyfile(source/name,target)
        rows=read(source/'documents/paper-build/sources.tsv').splitlines()
        declared_fragments(self.root,rows)
        for path, unit, _, _, line in scope_fragments(source):
            self.replace(path,'% generated: '+unit+'\n','% ordinary comment\n% generated: '+unit+'\n')
            with self.assertRaisesRegex(ValueError, 'M055: Fragment marker/declaration differs'):
                declared_fragments(self.root,rows)
            before = read(self.root/inclusion)
            changed_rows = []
            for row in before.splitlines():
                fields = row.split('\t')
                if fields[0] == path and int(fields[4]) >= line:
                    fields[4] = str(int(fields[4]) + 1)
                changed_rows.append('\t'.join(fields))
            write(self.root/inclusion, '\n'.join(changed_rows) + '\n')
            declared_fragments(self.root,rows)
            write(self.root/inclusion, before)
            self.replace(path,'% ordinary comment\n% generated: '+unit+'\n','% generated: '+unit+'\n')
        fragment='generated/documents/explainer-pilot/sec-say-direction.tex'
        original=read(self.root/fragment)
        write(self.root/fragment,original.replace('sec:say:direction','sec:say:other'))
        with self.assertRaisesRegex(ValueError,'Fragment label/declaration differs'):
            declared_fragments(self.root,rows)
        write(self.root/fragment,original.replace('paperrefrain','refrain'))
        with self.assertRaisesRegex(ValueError,'Fragment label/declaration differs'):
            declared_fragments(self.root,rows)
        write(self.root/fragment,original)
        marker='documents/paper/explainer/explainer.tex'
        self.replace(marker,'% generated: sec:say:direction','% generated: sec:say:other')
        with self.assertRaisesRegex(ValueError,'Fragment marker/declaration differs'):
            declared_fragments(self.root,rows)
        self.replace(marker,'% generated: sec:say:other','% generated: sec:say:direction')
        self.replace(marker,'% generated: sec:say:direction\n','')
        with self.assertRaisesRegex(ValueError,'Fragment marker/declaration differs'):
            declared_fragments(self.root,rows)
        shutil.copyfile(source/marker,self.root/marker)
        with (self.root/marker).open('a') as output:
            output.write('% generated: sec:say:direction\n')
        with self.assertRaisesRegex(ValueError,'Fragment marker/declaration differs'):
            declared_fragments(self.root,rows)
        shutil.copyfile(source/marker,self.root/marker)
        with (self.root/marker).open('a') as output:
            output.write('% generated: sec:say:other\n')
        with self.assertRaisesRegex(ValueError,'Undeclared fragment marker'):
            declared_fragments(self.root,rows)
        self.replace(marker,'% generated: sec:say:other\n','')
        with (self.root/marker).open('a') as output:
            output.write('  % generated: sec:say:other\n')
        with self.assertRaisesRegex(ValueError,'Undeclared fragment marker'):
            declared_fragments(self.root,rows)
        self.replace(marker,'  % generated: sec:say:other\n','')
        with (self.root/marker).open('a') as output:
            output.write('\\paperrefrain{}{Direction tells which way two fates lean, never how much fate there is to lean.}\n')
        with self.assertRaisesRegex(ValueError,'Authored refrain duplicates'):
            declared_fragments(self.root,rows)
        self.replace(marker,'\\paperrefrain{}{Direction tells which way two fates lean, never how much fate there is to lean.}\n','')
        paragraph='generated/documents/paper-pilot/sec-reading-shadow.tex'
        original=read(self.root/paragraph)
        for changed, fault in [
            (original.replace('\\PaperNativeParagraph{', '\\refrain{', 1),
             'M055: Fragment label/declaration differs'),
            (original.replace('\\ftag{src:nd}', '\\ftag{src:none}'),
             'M055: Fragment tag binding differs'),
            (original.replace('One may assume', '{One may assume'),
             'M055: Fragment body differs'),
            (original.replace('One may assume', '\\textbf One may assume'),
             'M055: Fragment body differs'),
            (original.replace('the fact is', 'the\nfact is'),
             'M055: Fragment body differs'),
        ]:
            write(self.root/paragraph,changed)
            with self.assertRaisesRegex(ValueError,fault):
                declared_fragments(self.root,rows)
        write(self.root/paragraph,original)
        body=read(self.root/paragraph).split('}{',1)[1].removesuffix('}\n')
        paper='documents/paper/parts/front.tex'
        self.replace(paper,'% generated: sec:reading:shadow\n',
                     '% generated: sec:reading:shadow\n'+body.replace(' the ', '\n the ', 1)+'\n')
        with self.assertRaisesRegex(ValueError,'M055: Authored paragraph duplicates a declared fragment'):
            declared_fragments(self.root,rows)

    def test_fragment_comments_body_mint_and_source_declaration_refuse(self):
        source=pathlib.Path(__file__).resolve().parents[2]
        inclusion='generated/documents/inclusions.tsv'
        target=self.root/inclusion; target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(source/inclusion,target)
        for path, _, fragment, _ in FRAGMENTS:
            for name in (path, fragment):
                target=self.root/name; target.parent.mkdir(parents=True,exist_ok=True)
                shutil.copyfile(source/name,target)
        rows=read(source/'documents/paper-build/sources.tsv').splitlines()
        marker='documents/paper/explainer/explainer.tex'
        self.replace(marker,'% generated: sec:say:direction\n%\n',
                     '% generated: sec:say:direction\n% padding\n')
        declared_fragments(self.root,rows)
        self.replace(marker,'% generated: sec:say:direction\n% padding\n',
                     '% generated: sec:say:direction\n%\n')
        fragment='generated/documents/explainer-pilot/sec-say-direction.tex'
        original=read(self.root/fragment)
        write(self.root/fragment,original.replace('Direction tells','{Direction tells'))
        with self.assertRaisesRegex(ValueError,'Fragment body differs'):
            declared_fragments(self.root,rows)
        write(self.root/fragment,original)
        with (self.root/marker).open('a') as output:
            output.write('\\label{sec:say:direction}\n')
        with self.assertRaisesRegex(ValueError,'Authored mint duplicates'):
            declared_fragments(self.root,rows)
        self.replace(marker,'\\label{sec:say:direction}\n','')
        with self.assertRaisesRegex(ValueError,'Fragment source declaration differs'):
            declared_fragments(self.root,[row for row in rows if row != marker])

    def test_version_setting_guards_refuse(self):
        source=pathlib.Path(__file__).resolve().parents[2]
        version=tomllib.loads(read(source/'documents/paper-build/templates.toml'))['version']
        paper=read(self.root/'documents/paper/main.tex')
        explainer=read(self.root/'documents/paper/main_explainer.tex.in')
        self.assertIn('v'+version['VERSION'],paper_master(paper,version))
        self.assertIn(version['DRAFT'],versioned(explainer,version,'explainer'))
        for text, setting, expected in [
            (paper, {**version,'EDITION':'first'}, 'Version setting differs'),
            (paper, {'VERSION':version['VERSION']}, 'Version setting differs'),
            (paper.replace('@VERSION@','@VERSION@ @VERSION@',1), version, 'Version placeholder occurrence differs: paper VERSION'),
            (paper.replace('@VERSION@','',1), version, 'Version placeholder occurrence differs: paper VERSION'),
            (paper.replace('@DRAFT@','',1), version, 'Version placeholder occurrence differs: paper DRAFT'),
            (paper+'@EDITION@\n', version, 'Unexpanded paper placeholder'),
        ]:
            with self.subTest(fault=expected, setting=sorted(setting)):
                with self.assertRaises(ValueError) as caught:paper_master(text,setting)
                self.assertEqual(str(caught.exception),expected)
        with self.assertRaises(ValueError) as caught:versioned(explainer.replace('@DRAFT@','@DRAFT@@DRAFT@',1),version,'explainer')
        self.assertEqual(str(caught.exception),'Version placeholder occurrence differs: explainer DRAFT')

    def test_destination_discrepancy_refuses(self):
        check_destination('main', 'eq:fixture', 'equation.1', 2, 2)
        with self.assertRaises(ValueError) as caught:
            check_destination('main', 'eq:fixture', 'equation.1', 2, 1)
        self.assertEqual(str(caught.exception),
            "M064: auxiliary/PDF page discrepancy: {'document': 'main', 'key': 'eq:fixture', "
            "'anchor': 'equation.1', 'auxiliary_page': 2, 'destination_page': 1}")

    def test_missing_and_surplus_occurrences_are_both_reported(self):
        key=('main',1,'eq:fixture')
        one=collections.Counter({key:1}); empty=collections.Counter()
        self.assertEqual(occurrence_differences(one,one),[])
        self.assertEqual(occurrence_differences(one,empty)[0]['emitted'],0)
        self.assertEqual(occurrence_differences(empty,one)[0]['observed'],0)

    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix='paper-identities-')
        self.root=pathlib.Path(self.temp.name)
        source=pathlib.Path(__file__).resolve().parents[2]
        for name in ['documents/paper-build/paper-tags.tsv','documents/paper-build/reader-tags.toml',
                     'documents/paper-build/reader-hooks.tex',
                     'documents/paper-build/provisional-tags.toml','registry/documentary-tags.toml','generated/manifest.json',
                     'documents/paper-build/label-mints.tsv','documents/paper-build/sources.tsv']:
            target=self.root/name; target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(source/name,target)
        shutil.copytree(source/'documents/paper',self.root/'documents/paper')
        for name in ('tools/paper/identities.py', 'tools/paper/common.py'):
            target=self.root/name; target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(source/name,target)

    def tearDown(self):self.temp.cleanup()

    def replace(self,file,old,new):
        path=self.root/file; text=read(path)
        self.assertIn(old,text); write(path,text.replace(old,new,1))

    def test_assembly_json_evidence_and_refusals(self):
        value=json.loads(assembly_json(self.root))
        self.assertEqual(list(value), ['schema','inputs','bindings'])
        self.assertEqual(value['schema'], 1)
        self.assertEqual([entry['path'] for entry in value['inputs']],
                         sorted({entry['path'] for entry in value['inputs']}))
        rows={row['key']:row for row in value['bindings']}
        alias=rows['dyn:exact']
        self.assertEqual(alias['status'], 'alias')
        self.assertEqual(alias['alias_site']['file'],
                         'documents/paper-build/reader-tags.toml')
        self.assertEqual(alias['lookup_site']['file'],
                         'documents/paper-build/paper-tags.tsv')
        self.assertIn({'file':'documents/paper/data/ftoo_manifest.tex','line':7832},
                      alias['declaration_sites'])
        self.assertTrue(all(row['target'] is None and row['target_kind'] is None
                            for row in rows.values() if row['status']=='candidate'))
        for lines in ('', 'key = "dyn:exact"\nkey = "dyn:exact"\n'):
            with self.assertRaisesRegex(ValueError,
                                        'M063: key evidence differs: dyn:exact'):
                _assignment_line(lines,'dyn:exact','reader-tags.toml')

    def test_release_refuses_explicit_candidates(self):
        rows=bindings(self.root,True)
        paper_rows=[line for line in read(self.root/'documents/paper-build/paper-tags.tsv').splitlines() if not line.startswith('#')]
        aliases=tomllib.loads(read(self.root/'documents/paper-build/reader-tags.toml'))['aliases']
        admissions=tomllib.loads(read(self.root/'documents/paper-build/reader-tags.toml'))['admissions']
        self.assertEqual(sum(v['status']=='paper' for v in rows.values()),len(paper_rows)+len(aliases))
        self.assertEqual(sum(v['status']=='admitted' for v in rows.values()),len(admissions))
        self.candidate()
        with self.assertRaisesRegex(ValueError,'M064: release refuses'):bindings(self.root,True)

    def candidate(self):
        write(self.root/'documents/paper-build/provisional-tags.toml', '''schema = 1
[[candidates]]
key = "eq:fixture"
spelling = "eq:fixture"
status = "provisional"
alternative = "eq:other"
definition = "Fixture equation"
sites = ["fixture"]
''')

    def test_overlap_and_reserved_names_refuse(self):
        self.candidate()
        file='documents/paper-build/provisional-tags.toml'
        self.replace(file,'key = "eq:fixture"','key = "dyn:stat"')
        with self.assertRaisesRegex(ValueError,'key in two lists'):bindings(self.root)
        self.replace(file,'key = "dyn:stat"','key = "eq:fixture"')
        self.replace(file,'spelling = "eq:fixture"','spelling = "c:1"')
        with self.assertRaisesRegex(ValueError,'reserved registry collision'):bindings(self.root)

    def test_alias_and_paper_tag_mutations_refuse(self):
        file='documents/paper-build/reader-tags.toml'
        self.replace(file,'key = "dyn:exact"','key = "dyn:invented"')
        with self.assertRaisesRegex(ValueError,'unevidenced alias'):bindings(self.root)
        self.replace(file,'key = "dyn:invented"','key = "dyn:exact"')
        self.replace('documents/paper-build/paper-tags.tsv','J:broker','J:forensics')
        with self.assertRaisesRegex(ValueError,'duplicate paper tag'):bindings(self.root)

    def test_unbound_ledger_key_refuses(self):
        with self.assertRaises(ValueError) as caught:
            ledger_binding({}, 'identity', 'absent:key', 'absent:key', 'paper')
        self.assertEqual(str(caught.exception), 'M064: unbound ledger key: absent:key')

    def test_paper_tag_row_count_and_grammar_refuse(self):
        path=self.root/'documents/paper-build/paper-tags.tsv'
        original=read(path)
        rows=[line for line in original.splitlines() if not line.startswith('#')]
        malformed=rows.copy(); malformed[0]='!'+malformed[0]
        missing=rows[-1].split('\t')[0]
        for changed, expected in [(rows[:-1],'M063: paper tag source coverage differs: missing='+missing+' extra= changed='),
                                  (malformed,'M063: malformed paper tag pair')]:
            with self.subTest(fault=expected):
                write(path,'\n'.join(changed)+'\n')
                with self.assertRaises(ValueError) as caught:bindings(self.root)
                self.assertEqual(str(caught.exception),expected)
                write(path,original)

    def test_paper_source_coverage_refuses_extra_changed_and_conflicting_targets(self):
        rows='documents/paper-build/paper-tags.tsv'
        source='documents/paper/data/ftoo_manifest.tex'
        original=read(self.root/rows)
        write(self.root/rows,original+'Q:fixture\tsec:fixture:extra\tnode\n')
        with self.assertRaises(ValueError) as caught:bindings(self.root)
        self.assertEqual(str(caught.exception),
                         'M063: paper tag source coverage differs: missing= extra=Q:fixture changed=')
        write(self.root/rows,original.replace('sem:claim:broker-suppression-thresholds',
                                              'sec:fixture:changed',1))
        with self.assertRaises(ValueError) as caught:bindings(self.root)
        self.assertEqual(str(caught.exception),
                         'M063: paper tag source coverage differs: missing= extra= changed=J:broker')
        write(self.root/rows,original)
        original_source=read(self.root/source)
        write(self.root/source,original_source+
              '\\prop_gput:Nnn \\g_ftoo_tag_to_id_prop { J:broker } { sec:fixture:other }\n')
        with self.assertRaises(ValueError) as caught:bindings(self.root)
        self.assertEqual(str(caught.exception),
                         'M063: paper tag source has conflicting targets: J:broker')

    def test_label_mint_file_evidence_survives_line_moves(self):
        baseline=bindings(self.root)
        path=self.root/'documents/paper-build/label-mints.tsv'
        original=read(path)
        rows=original.splitlines()
        index=next(i for i,row in enumerate(rows) if row.startswith('c:q\t'))
        key,file=rows[index].split('\t')
        source=self.root/file
        source_text=read(source)
        lines=source_text.splitlines(keepends=True)
        occurrence=next(i for i,line in enumerate(lines) if holds(line,key))
        lines.insert(occurrence,'\n')
        write(source,''.join(lines))
        self.assertEqual(bindings(self.root),baseline)
        write(source,source_text)

        rows[index]='\t'.join([key,'documents/paper/parts/missing.tex'])
        write(path,'\n'.join(rows)+'\n')
        with self.assertRaises(ValueError) as caught:bindings(self.root)
        self.assertEqual(str(caught.exception),'M063: label mint file is absent: documents/paper/parts/missing.tex')
        write(path,original)

        unlisted='documents/paper/parts/unlisted.tex'
        write(self.root/unlisted,'\\label{c:q}\n')
        rows[index]='\t'.join([key,unlisted])
        write(path,'\n'.join(rows)+'\n')
        with self.assertRaises(ValueError) as caught:bindings(self.root)
        self.assertEqual(str(caught.exception),'M063: label mint file is not declared: '+unlisted)
        write(path,original)
        rows[index]='\t'.join([key,file])

        changed=re.sub(r'(?<![A-Za-z0-9_:-])'+re.escape(key)+r'(?![A-Za-z0-9_:-])','c:qx',source_text)
        write(source,changed)
        with self.assertRaises(ValueError) as caught:bindings(self.root)
        self.assertEqual(str(caught.exception),'M063: label mint file does not hold its key: '+key+' at '+file)
        write(source,source_text)

        write(path,original+rows[index]+'\n')
        with self.assertRaises(ValueError) as caught:bindings(self.root)
        self.assertEqual(str(caught.exception),'M063: duplicate label mint key: '+key)
        write(path,original)
        rows[index]='\t'.join([key,file,'1'])
        write(path,'\n'.join(rows)+'\n')
        with self.assertRaises(ValueError) as caught:bindings(self.root)
        self.assertEqual(str(caught.exception),'M063: malformed label mint row '+str(index+1))
        rows[index]='\t'.join(['!'+key,file])
        write(path,'\n'.join(rows)+'\n')
        with self.assertRaises(ValueError) as caught:bindings(self.root)
        self.assertEqual(str(caught.exception),'M063: malformed label mint key: !'+key+' at row '+str(index+1))
        self.assertFalse(holds('\\ftag{c:qx}','c:q'))
        self.assertFalse(holds('\\label{sec:null-verdict}','c:nu'))
        self.assertTrue(holds('(the axis constant is \\ptg{c:q});','c:q'))

    def test_candidate_key_cannot_already_be_admitted_under_another_handle(self):
        self.candidate()
        self.replace('registry/documentary-tags.toml','label = "dec:weld:equal-address-sets"','label = "eq:fixture"')
        with self.assertRaisesRegex(ValueError,'key already belongs'):bindings(self.root)

    def test_admission_requires_exact_confirmation(self):
        self.replace('documents/paper-build/reader-tags.toml','confirmation = "dec:reader:confirmed-tags"','confirmation = "dec:reader:other"')
        with self.assertRaisesRegex(ValueError,'admission lacks exact confirmed binding'):bindings(self.root)

    def test_registry_matches_confirmation_record(self):
        source=pathlib.Path(__file__).resolve().parents[2]
        record=read(source/'registry/reader-confirmation.md')
        decisions=re.findall(r'\| `([^`]+)` \| `([^`]+)` \| (admit|alias) \|',record)
        rows=bindings(self.root,True)
        table=tomllib.loads(read(self.root/'documents/paper-build/reader-tags.toml'))
        expected={(row['key'],row['spelling'],kind) for kind,items in [('admit',table['admissions']),('alias',table['aliases'])] for row in items}
        self.assertEqual(len(decisions),len(expected))
        self.assertEqual(set(decisions),expected)
        for key,spelling,decision in decisions:
            self.assertEqual(rows[key]['spelling'],spelling)
            self.assertEqual(rows[key]['status'],'admitted' if decision=='admit' else 'paper')
        events=tomllib.loads(read(self.root/'registry/documentary-tags.toml'))['events'][1:]
        self.assertEqual({(e['label'],e['handle']) for e in events},
                         {(key,spelling) for key,spelling,decision in decisions if decision=='admit'})

    def test_case_only_permanent_collision_refuses(self):
        self.replace('registry/documentary-tags.toml','handle = "eq:tenses"','handle = "B:BELIEF"')
        with self.assertRaisesRegex(ValueError,'case-insensitive registry collision'):bindings(self.root)

    def test_leak_control_and_adjacent_cells(self):
        self.assertEqual(leaks('COR:TREMBLE:CURVE'),['cor:tremble:curve'])
        self.assertEqual(leaks('dyn:tcurve'),[])
        self.assertEqual(leaks('d:annex E:STOCK d:annex E:OSC'),[])
        self.assertEqual(leaks('a:bc:d'),['a:bc:d'])
        self.assertEqual(leaks('TBL:GEN:GRADES'),['tbl:gen:grades'])


if __name__=='__main__':
    with timed('identity-controls'):unittest.main()
