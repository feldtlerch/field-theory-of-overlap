"""Refusal controls for finite bindings and rendered-label leakage."""
import pathlib
import hashlib
import json
import collections
import shutil
import subprocess
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
from identities import CapturedInputs, _assignment_line, assembly_json, bindings, holds, virtue_roster, result_header_requests
from ledger import leaks, occurrence_differences, ledger_binding, check_destination, screen_references, check_screen_references, check_screen_results, virtue_site, check_native_virtue_row, check_virtue_context, check_virtue_inventory, check_companion_mints, auxiliary, native_virtue_plan
from prepare import declared_fragments, versioned, paper_master, scope_fragments

FRAGMENTS = tuple(row[:4] for row in scope_fragments(pathlib.Path(__file__).resolve().parents[2]))
PARENT_REVISION = '68643f63939af28693897b84478e1e4988c40e32'


class Identities(unittest.TestCase):
    def test_result_header_compares_typesetter_number_title_type_and_destination(self):
        class Pdf:
            def __init__(self, page):
                self.page = page

            def destination(self, anchor):
                self.assert_anchor = anchor
                return self.page

        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            generated = root/'generated/documents'
            (generated/'paper-pilot').mkdir(parents=True)
            write(generated/'inclusions.tsv',
                  'documents/paper/parts/partA.tex\tresult:f:pool\t'
                  'generated/documents/paper-pilot/result-f-pool.tex\tcorpus-slot\t420\n')
            write(generated/'paper-pilot/result-f-pool.tex',
                  '\\begin{insight}[\\hdr{f:pool}]\\grade{cont-anch}\n'
                  '  Coherence pools the value; independence supplies the weight.\n'
                  '\\end{insight}\n')
            screen = ('<!-- unit result:f:pool document paper-pilot language en -->\n'
                      '*Insight 2.13 (Coherence pools, independence weighs)* · f:pool '
                      'cont-anch <a id="result-f-pool"></a>\n')
            write(generated/'paper-pilot/en.labels.md', screen)
            headers = [{'unit':'result:f:pool','key':'f:pool',
                        'title':'Coherence pools, independence weighs'}]
            target = {'default':'2.13','title':headers[0]['title'],
                      'zc@type':'remark','page':'19','anchor':'theorem.2.13'}
            check_screen_results(root, {'f:pool':target}, Pdf(19), headers)
            for field, value in [('default','2.14'),('title','Changed'),
                                 ('zc@type','definition')]:
                changed = dict(target, **{field:value})
                with self.subTest(field=field), self.assertRaisesRegex(
                        ValueError, 'result number, title or reference kind differs'):
                    check_screen_results(root, {'f:pool':changed}, Pdf(19), headers)
            with self.assertRaisesRegex(ValueError, 'auxiliary/PDF page discrepancy'):
                check_screen_results(root, {'f:pool':target}, Pdf(20), headers)
            write(generated/'paper-pilot/en.labels.md', screen.replace('2.13','2.14'))
            with self.assertRaisesRegex(ValueError, 'result number, title or reference kind differs'):
                check_screen_results(root, {'f:pool':target}, Pdf(19), headers)

    def test_result_header_requests_have_one_checked_corpus_owner(self):
        source = ('[unit]\nid = "result:f:pool"\ndocument = "paper-pilot"\n'
                  'composition = "corpus-slot"\nlanguage = "en"\n'
                  '[unit.documentary-targets]\nheader = "f:pool"\n'
                  '[[unit.blocks]]\nkind = "result"\ntarget = "header"\nkey = "f:pool"\n')
        files = {'units/corpus/pool.toml': source,
                 'units/corpus/existing.toml': '[unit]\nid = "sec:plain"\nblocks = [{kind = "paragraph"}]\n'}
        self.assertEqual(result_header_requests(files, files.__getitem__),
                         {'f:pool': ('units/corpus/pool.toml', 'result:f:pool')})
        files['units/corpus/other.toml'] = source.replace('result:f:pool', 'result:f:other')
        with self.assertRaisesRegex(ValueError, 'duplicate or malformed result header key'):
            result_header_requests(files, files.__getitem__)
        files.pop('units/corpus/other.toml')
        files['units/corpus/pool.toml'] = source.replace('target = "header"', 'target = "other"')
        with self.assertRaisesRegex(ValueError, 'result header selector differs'):
            result_header_requests(files, files.__getitem__)

    def test_virtue_ledger_inventory_refusals(self):
        position = ('documents/paper/parts/coda.tex', 8)
        native = {position: ('sec:coda:eight-words', [('virtue', 'wonder')])}
        seen = collections.Counter({(*position, 0): 1})
        first = collections.Counter({'v:wonder': 1})
        roster = {'v:wonder': {}}
        paper_auxiliary = {'v:wonder': {'anchor': 'virtue-wonder'}}
        check_virtue_inventory(native, seen, first, roster, paper_auxiliary)
        for changed_seen, changed_first, changed_auxiliary, reason in [
            (collections.Counter(), first, paper_auxiliary, 'native virtue occurrence missing or repeated'),
            (collections.Counter({(*position, 0): 2}), first, paper_auxiliary,
             'native virtue occurrence missing or repeated'),
            (seen, collections.Counter(), paper_auxiliary, 'virtue first-use count differs'),
            (seen, collections.Counter({'v:wonder': 2}), paper_auxiliary,
             'virtue first-use count differs'),
            (seen, first, {}, 'virtue auxiliary definition absent'),
        ]:
            with self.subTest(reason=reason), self.assertRaisesRegex(ValueError, reason):
                check_virtue_inventory(native, changed_seen, changed_first, roster, changed_auxiliary)
        for key, kind, job, reason in [
            ('v:unknown', 'virtue-later', 'main', 'unknown virtue roster key'),
            ('v:wonder', 'virtue-companion', 'main', 'virtue command mode/document differs'),
        ]:
            with self.subTest(reason=reason), self.assertRaisesRegex(ValueError, reason):
                check_virtue_context(key, kind, job, roster, key)
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            write(root/'main.aux', '\\zref@newlabel{v:wonder}\n\\zref@newlabel{v:wonder}\n')
            with self.assertRaisesRegex(ValueError, 'duplicate virtue auxiliary definition'):
                auxiliary(root, 'main')
            fragments = root/'generated/documents'
            fragments.mkdir(parents=True)
            write(fragments/'virtue.tex', '\\virtue{wonder}')
            row = 'documents/paper/parts/coda.tex\tsec:coda:eight-words\tgenerated/documents/virtue.tex\tparagraph\t8\n'
            write(fragments/'inclusions.tsv', row + row)
            with self.assertRaisesRegex(ValueError, 'duplicate native virtue inclusion'):
                native_virtue_plan(root)
            write(fragments/'slot.tex', 'heading\nchild\nparagraph\n')
            write(fragments/'inclusions.tsv',
                  'documents/paper/parts/coda.tex\tsec:fixture:slot\t'
                  'generated/documents/slot.tex\tcorpus-slot\t3\n' + row)
            shifted = native_virtue_plan(root)
            self.assertEqual(shifted,
                             {('documents/paper/parts/coda.tex', 10):
                              ('sec:coda:eight-words', [('virtue', 'wonder')])})
            with self.assertRaisesRegex(ValueError, 'native virtue occurrence missing'):
                check_virtue_inventory(shifted, seen, first, roster, paper_auxiliary)

    def test_virtue_site_and_native_mode_refuse_changed_occurrences(self):
        self.assertEqual(virtue_site('parts/coda.tex:86#virtue=0'),
                         ('documents/paper/parts/coda.tex', 86, 0))
        for site in ('parts/coda.tex:86#virtue=00', 'parts/coda.tex:86#virtue=-1',
                     '../coda.tex:86#virtue=0', '/parts/coda.tex:86#virtue=0',
                     'parts/coda.tex:86'):
            with self.subTest(site=site), self.assertRaisesRegex(ValueError, 'M064:'):
                virtue_site(site)
        calls = [('virtue', 'wonder'), ('virtue', 'beauty')]
        check_native_virtue_row('v:wonder', 'virtue-later', 0, 'sec:coda:eight-words', calls)
        for key, kind, index, fault in [
            ('v:beauty', 'virtue-later', 0, 'mode/word differs'),
            ('v:wonder', 'virtue-first', 0, 'mode/word differs'),
            ('v:wonder', 'virtue-companion', 0, 'mode/word differs'),
            ('v:wonder', 'virtue-later', 2, 'extra native virtue occurrence'),
        ]:
            with self.subTest(key=key, kind=kind, index=index), self.assertRaisesRegex(ValueError, fault):
                check_native_virtue_row(key, kind, index, 'sec:coda:eight-words', calls)
        check_companion_mints({}, {'v:wonder': {}})
        for key in ('v:wonder', 'FTOO-v:wonder'):
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'companion virtue minted'):
                check_companion_mints({key: {}}, {'v:wonder': {}})

    def test_source_line_end_interpretations(self):
        cases = (
            ('the dependence', 'the de-\npendence', 'remove-discretionary'),
            ('the dependence', 'the de- \npendence', 'remove-discretionary'),
            ('the non-linear', 'the non-\nlinear', 'keep-lexical'),
            ('the pre- and', 'the pre-\nand', 'keep-lexical'),
            ('a—b', 'a—\nb', 'keep-without-separator'),
            ('a — b', 'a —\nb', 'keep-with-separator'),
            ('a–b', 'a–\nb', 'keep-without-separator'),
            ('a – b', 'a –\nb', 'keep-with-separator'),
            ('a/b', 'a/\nb', 'keep-without-separator'),
            ('a / b', 'a /\nb', 'keep-with-separator'),
        )
        for source, raw, decision in cases:
            with self.subTest(source=source):
                decoded = compare_words.decode_located(source, {})
                locations = [[1, i] for i in range(len(raw))]
                stream, offsets, events = compare_words.normalize(raw, locations)
                extracted = dict(stream=stream, offsets=offsets, events=events,
                                 raw_pages=[dict(text=raw)])
                proof = compare_words.align_located(decoded, extracted, 'paper', 'before')
                self.assertEqual(proof['canonical'], decoded['extracted'])
                self.assertEqual(compare_words.canonicalize(extracted, proof)[0],
                                 decoded['extracted'])
                self.assertEqual([row['decision'] for row in proof['decisions']], [decision])
                self.assertEqual(proof['decisions'][0]['source_range'],
                                 decoded['tokens'][proof['decisions'][0]['token_ordinal']]
                                 ['source_range'])
                if ' ' in source:
                    changed = source.replace(' ', '  ', 1)
                    other = compare_words.decode_located(changed, {})
                    self.assertEqual(other['extracted'], decoded['extracted'])
                changed_source = source.replace('b', 'c') if 'b' in source else source + 'x'
                changed_raw = raw.replace('b', 'c') if 'b' in raw else raw + 'x'
                changed_decoded = compare_words.decode_located(changed_source, {})
                changed_stream, changed_offsets, changed_events = compare_words.normalize(
                    changed_raw, [[1, i] for i in range(len(changed_raw))])
                changed_extracted = dict(stream=changed_stream, offsets=changed_offsets,
                                         events=changed_events, raw_pages=[dict(text=changed_raw)])
                changed_proof = compare_words.align_located(changed_decoded,
                                                             changed_extracted,
                                                             'paper', 'after')
                self.assertNotEqual(proof['canonical'], changed_proof['canonical'])
                if source.startswith('a') and ' b' in source:
                    changed_separator = compare_words.decode_located(
                        source.replace(' b', 'b'), {})
                    changed_proof = compare_words.align_located(
                        changed_separator, extracted, 'paper', 'after')
                    self.assertNotEqual(proof['canonical'], changed_proof['canonical'])
        first, second = 'the de-', 'pendence'
        raw = first + '\n' + second
        locations = ([[1, i] for i in range(len(first) + 1)] +
                     [[2, i] for i in range(len(second))])
        stream, offsets, events = compare_words.normalize(raw, locations)
        proof = compare_words.align_located(
            compare_words.decode_located('the dependence', {}),
            dict(stream=stream, offsets=offsets, events=events,
                 raw_pages=[dict(text=first), dict(text=second)]), 'paper', 'before')
        self.assertEqual(proof['canonical'], 'the dependence')
        self.assertEqual(proof['decisions'][0]['newline_kind'], 'page')
        original_sources = compare_words.selected_sources
        def changed_sources(*args, **options):
            before, after = original_sources(*args, **options)
            after = dict(after, body=after['body'].replace(b'dependence', b'dependance', 1),
                         blob_bytes=after['blob_bytes'].replace(b'dependence', b'dependance', 1))
            return before, after
        def changed_body(path, document, side, value):
            if document == 'paper' and side == 'after':
                value['stream'] = value['stream'].replace('dependence', 'dependance', 1)
                value['offsets'] = [[1, index] for index in range(len(value['stream']))]
        with mock.patch.object(compare_words, 'selected_sources', side_effect=changed_sources):
            exit_code, result = self.conversion_run(changed_body)
        self.assertEqual((exit_code, result['verdict']), (1, 'different'))
        self.assertIn('source/body', [row['part'] for row in result['differences']])
        stream_change = next(row for row in result['differences'] if row['part'] == 'stream')
        self.assertEqual(stream_change['location']['after'],
                         dict(raw=[[1, result['documents']['paper']['after']['stream'].index(
                             'dependance') + 6]], source=None))

    def test_unique_correspondence(self):
        def extracted(raw):
            stream, offsets, events = compare_words.normalize(raw,
                                                                 [[1, i] for i in range(len(raw))])
            return dict(stream=stream, offsets=offsets, events=events,
                        raw_pages=[dict(text=raw)])
        decoded = compare_words.decode_located('the dependence', {})
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.align_located(decoded,
                                        extracted('the dependence and the dependence'),
                                        'paper', 'before')
        self.assertEqual((raised.exception.row['code'], raised.exception.row['stage']),
                         ('CorrespondenceInvalid', 'correspondence'))
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.unique_proof(
                [dict(range=[1, 3], canonical='ab', source_ranges=[[0, 2]]),
                 dict(range=[1, 3], canonical='ab', source_ranges=[[4, 6]])],
                'paper', 'after')
        self.assertEqual(raised.exception.row['code'], 'LineEndAmbiguous')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.unique_proof(
                [dict(range=[1, 3], canonical='ab', source_ranges=[[0, 2]]),
                 dict(range=[1, 3], canonical='ab', source_ranges=[[4, 6]]),
                 dict(range=[5, 7], canonical='ab', source_ranges=[[8, 10]])],
                'paper', 'after')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.align_located(decoded, extracted('the dependance'),
                                        'paper', 'before')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        search = raised.exception.row['search']
        self.assertEqual(search['matches_found'], 0)
        self.assertEqual(search['remaining_state_budget'],
                         search['limit'] - search['expanded_states'])
        for source in ('a' * 65537, 'a ' * 16385):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.decode_located(source, {})
            self.assertEqual(raised.exception.row['code'], 'CorrespondenceLimit')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.decode_located('ab\\unknown', {}, 'example.tex')
        self.assertEqual((raised.exception.row['code'], raised.exception.row['stage']),
                         ('SyntaxUnsupported', 'snapshot'))
        self.assertIn('example.tex:2:', raised.exception.row['detail'])
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.decode_located('ab\\unknown', {}, 'recorded.tex', 100)
        self.assertIn('recorded.tex:102:', raised.exception.row['detail'])
        shifted = compare_words.decode_located('an example', {}, 'recorded.tex', 100)
        self.assertEqual(shifted['tokens'][0]['source_range'], [100, 102])
        self.assertEqual(shifted['tokens'][1]['source_range'], [103, 110])

    def test_repeated_word_location(self):
        repo = pathlib.Path(__file__).resolve().parents[2]
        fragment = read(repo/'generated/documents/paper-pilot/sec-reading-shadow.tex')
        body = fragment.split('}{', 1)[1].removesuffix('}\n')
        decoded = compare_words.decode_located(body, bindings(repo))
        text = decoded['extracted']
        positions = [match.start() for match in re.finditer('dependence', text)]
        self.assertEqual(len(positions), 2)
        found = []
        for position in positions:
            raw = text[:position] + 'de-\npendence' + text[position + 10:]
            stream, offsets, events = compare_words.normalize(
                raw, [[1, index] for index in range(len(raw))])
            proof = compare_words.align_located(
                decoded, dict(stream=stream, offsets=offsets, events=events,
                              raw_pages=[dict(text=raw)]), 'paper', 'after')
            found.append(proof['decisions'][0])
        self.assertNotEqual(found[0]['token_ordinal'], found[1]['token_ordinal'])
        self.assertNotEqual(found[0]['source_range'], found[1]['source_range'])
        altered = copy.deepcopy(decoded)
        altered['tokens'][found[1]['token_ordinal']]['source_range'] = found[0]['source_range']
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.align_located(
                altered, dict(stream=stream, offsets=offsets, events=events,
                              raw_pages=[dict(text=raw)]), 'paper', 'after')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')

    def test_unchanged_event_keys(self):
        split = 'de-\npendence'
        stream, offsets, events = compare_words.normalize(
            split, [[1, index] for index in range(len(split))])
        selected_pdf = dict(stream=stream, offsets=offsets, events=events,
                            raw_pages=[dict(text=split)])
        decoded = compare_words.decode_located('dependence', {})
        selected_before = compare_words.align_located(
            decoded, selected_pdf, 'paper', 'before')
        selected_after = compare_words.align_located(
            decoded, selected_pdf, 'paper', 'after')
        altered = copy.deepcopy(selected_after)
        altered['decisions'][0]['raw_line'] = 'de- \npendence'
        self.assertFalse(compare_words.unchanged_paragraph_events(
            b'dependence', b'dependence', selected_before, altered))
        self.assertFalse(compare_words.unchanged_paragraph_events(
            b'dependence', b'de-pendence', selected_before, selected_after))
        self.assertTrue(compare_words.unchanged_paragraph_events(
            b'dependence', b'dependence', selected_before, selected_after))
        self.assertEqual(selected_before['decisions'][0]['decision'],
                         'unchanged-paragraph')
        def interpreted(raw):
            stream, offsets, events = compare_words.normalize(raw,
                                                                 [[1, i] for i in range(len(raw))])
            pdf = dict(stream=stream, offsets=offsets, events=events,
                       raw_pages=[dict(text=raw)])
            proof = compare_words.align_located(
                compare_words.decode_located('dependence', {}), pdf, 'paper', 'before')
            return pdf, proof, compare_words.residual_events(pdf, proof, 'paper', 'before')
        before, before_match, left = interpreted('dependence\npre-\nand')
        after, after_match, right = interpreted('de-\npendence\npre-\nand')
        envelope = dict(available=True)
        self.assertNotEqual(left[-1]['normalized_offset'], right[-1]['normalized_offset'])
        self.assertTrue(compare_words.residual_proof(
            before, after, left, right, envelope, before_match, after_match))
        self.assertEqual(left[-1]['decision'], 'unchanged-residual')
        self.assertEqual(left[-1]['residual_key'], right[-1]['residual_key'])
        changed_line = copy.deepcopy(right)
        changed_line[-1]['raw_line'] = 'pre- \nand'
        self.assertFalse(compare_words.residual_proof(before, after, left,
                                                       changed_line, envelope,
                                                       before_match, after_match))
        changed_key = copy.deepcopy(right)
        changed_key[-1]['residual_key'][-1] += 1
        self.assertFalse(compare_words.residual_proof(before, after, left,
                                                       changed_key, envelope,
                                                       before_match, after_match))
        self.assertFalse(compare_words.residual_proof(before, after, left, right,
                                                       dict(available=False),
                                                       before_match, after_match))
        changed_text = dict(after, stream=after['stream'].replace('pre-and', 'pro-and'))
        failure = []
        self.assertFalse(compare_words.residual_proof(
            before, changed_text, left, right, envelope, before_match, after_match, failure))
        self.assertEqual(failure, ['residual compared text differs'])
        unbound = compare_words.negative_witness('non-linear', list(range(10)))
        midline = compare_words.negative_witness('nonlinear', list(range(9)))
        self.assertEqual(unbound['stream'], midline['stream'])

    def test_unchanged_legacy_after_local_reflow(self):
        def located(raw):
            stream, offsets, events = compare_words.normalize(
                raw, [[1, index] for index in range(len(raw))])
            pdf = dict(stream=stream, offsets=offsets, events=events,
                       raw_pages=[dict(text=raw)])
            paragraph = compare_words.decode_located('dependence', {})
            proof = compare_words.align_located(paragraph, pdf, 'paper', 'before')
            return pdf, proof, compare_words.residual_events(pdf, proof, 'paper', 'before')

        before, before_match, old_events = located('dependence\npre-\nand')
        after, after_match, new_events = located('de-\npendence\npre-\nand')
        self.assertNotEqual(old_events[-1]['normalized_offset'],
                            new_events[-1]['normalized_offset'])
        self.assertEqual(old_events[-1]['residual_key'], new_events[-1]['residual_key'])
        self.assertTrue(compare_words.residual_proof(
            before, after, old_events, new_events, dict(available=True),
            before_match, after_match))
        self.assertEqual(old_events[-1]['decision'], 'unchanged-residual')
        changed_width = dict(available=False, reasons=['paper input differs: master measure'])
        self.assertFalse(compare_words.residual_proof(
            before, after, old_events, new_events, changed_width,
            before_match, after_match))
        raw = 'pre-\nand'
        stream, offsets, events = compare_words.normalize(
            raw, [[1, index] for index in range(len(raw))])
        companion = dict(stream=stream, offsets=offsets, events=events,
                         raw_pages=[dict(text=raw)])
        rows = compare_words.residual_events(companion, None, 'explainer', 'before')
        self.assertEqual(rows[0]['residual_key'][1], 0)

    def test_selector_and_envelope(self):
        repo = pathlib.Path(__file__).resolve().parents[2]
        parent_dir = self.root/'old-snapshot'
        candidate_dir = self.root/'new-snapshot'
        parent = compare_words.parent_snapshot(repo, 'HEAD', parent_dir)
        held = CapturedInputs(repo)
        inputs = compare_words.snapshot_inputs(repo, candidate_dir, held)
        args = argparse.Namespace(unit='sec:reading:shadow', before_source=None,
                                  before_revision='HEAD')
        before, after = compare_words.selected_sources(repo, args, parent,
                                                        parent_dir, candidate_dir)
        envelope = compare_words.source_envelope(parent, parent_dir, candidate_dir,
                                                  inputs, before, after, args.unit)
        self.assertTrue(envelope['available'], envelope['reasons'])
        with mock.patch.object(args, 'before_source',
                               'HEAD:documents/paper/parts/front.tex:203-207'):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.selected_sources(repo, args, parent, parent_dir, candidate_dir)
        self.assertEqual(raised.exception.row['code'], 'SourceInvalid')
        args.before_revision = None
        args.before_source = 'HEAD:documents/paper/parts/front.tex:203-207'
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.selected_sources(repo, args, parent, parent_dir, candidate_dir)
        self.assertEqual(raised.exception.row['code'], 'SourceInvalid')
        args.before_source = None
        args.before_revision = 'HEAD'
        missing = copy.deepcopy(parent)
        missing['fragments'] = [row for row in missing['fragments']
                                if row[1] != args.unit]
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.selected_sources(repo, args, missing, parent_dir, candidate_dir)
        self.assertEqual(raised.exception.row['code'], 'SourceInvalid')
        fragment = parent_dir/before['path']
        original = fragment.read_bytes()
        fragment.write_bytes(original.replace(b'PaperNativeParagraph', b'PaperOtherParagraph'))
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.selected_sources(repo, args, parent, parent_dir, candidate_dir)
        self.assertEqual(raised.exception.row['code'], 'SourceInvalid')
        fragment.write_bytes(original)
        source = parent_dir/before['source']
        original = source.read_bytes()
        source.write_bytes(original.replace(b'% generated: sec:reading:shadow', b'% other: sec:reading:shadow'))
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.selected_sources(repo, args, parent, parent_dir, candidate_dir)
        self.assertEqual(raised.exception.row['code'], 'SourceInvalid')
        source.write_bytes(original)
        legacy = copy.deepcopy(parent)
        legacy['fragments'] = [row for row in parent['fragments']
                               if row[1] != args.unit]
        body = 'One\nparagraph\nwith\nfive\nlines\n'
        source_name = after['source']
        candidate_bytes = (candidate_dir/source_name).read_bytes()
        marker_start, marker_end = after['marker']
        old_bytes = (candidate_bytes[:marker_start] + body.encode() +
                     candidate_bytes[marker_end:])
        authored = dict(commit=parent['commit'], path=source_name, first=after['line'],
                        last=after['line'] + 4,
                        bytes=[marker_start, marker_start + len(body.encode())],
                        body=body, blob_bytes=old_bytes, blob='fixture-blob')
        args.before_revision = None
        args.before_source = 'HEAD:' + source_name + ':203-207'
        with mock.patch.object(compare_words, 'source_blob', return_value=authored):
            authored_before, authored_after = compare_words.selected_sources(
                repo, args, legacy, parent_dir, candidate_dir)
        self.assertEqual(authored_before['kind'], 'authored')
        self.assertEqual(authored_after['marker'], after['marker'])
        selector_cases = (
            (dict(authored, path='documents/paper/parts/stray.tex'), legacy,
             'source selector misses inclusion place'),
            (authored, dict(legacy, receipt=dict(legacy['receipt'],
                inputs={key: value for key, value in legacy['receipt']['inputs'].items()
                        if key != source_name})), 'source selector is outside declared inputs'),
            (dict(authored, first=after['line'] + 1), legacy,
             'source selector misses inclusion place'),
            (dict(authored, bytes=[marker_start + 1,
                                   marker_start + 1 + len(body.encode())]), legacy,
             'source differs outside selected paragraph'),
            (dict(authored, blob_bytes=old_bytes + b'% unrelated edit\n'), legacy,
             'source differs outside selected paragraph'),
        )
        for changed_source, changed_parent, detail in selector_cases:
            with self.subTest(selector=detail, source=changed_source['path']):
                with mock.patch.object(compare_words, 'source_blob',
                                       return_value=changed_source):
                    with self.assertRaises(compare_words.Fault) as raised:
                        compare_words.selected_sources(repo, args, changed_parent,
                                                       parent_dir, candidate_dir)
                self.assertEqual((raised.exception.row['code'],
                                  raised.exception.row['detail']), ('SourceInvalid', detail))
        args.before_source = None
        args.before_revision = 'HEAD'
        inclusion = candidate_dir/'generated/documents/inclusions.tsv'
        original = inclusion.read_bytes()
        lines = original.splitlines(keepends=True)
        inclusion.write_bytes(b''.join([lines[1], lines[0], *lines[2:]]))
        changed = compare_words.source_envelope(parent, parent_dir, candidate_dir,
                                                 inputs, before, after, args.unit)
        self.assertIn('inclusion order differs from generator order', changed['reasons'])
        inclusion.write_bytes(original)
        inclusion.write_bytes(b''.join([lines[6], *lines[:6], *lines[7:]]))
        changed = compare_words.source_envelope(parent, parent_dir, candidate_dir,
                                                 inputs, before, after, args.unit)
        self.assertIn('inclusion order differs from generator order', changed['reasons'])
        inclusion.write_bytes(original)
        inclusion.write_bytes(original.replace(b'\trefrain\t', b'\tquotation\t', 1))
        changed = compare_words.source_envelope(parent, parent_dir, candidate_dir,
                                                 inputs, before, after, args.unit)
        self.assertFalse(changed['available'])
        self.assertIn('other inclusion identities differ', changed['reasons'])
        inclusion.write_bytes(original)
        receipt = candidate_dir/'generated/paper/inputs.json'
        original = receipt.read_bytes()
        changed_receipt = json.loads(original)
        changed_receipt['inputs']['unexpected.tex'] = '0' * 64
        receipt.write_text(json.dumps(changed_receipt))
        changed = compare_words.source_envelope(parent, parent_dir, candidate_dir,
                                                 inputs, before, after, args.unit)
        self.assertFalse(changed['available'])
        self.assertIn('paper input membership differs', changed['reasons'])
        receipt.write_bytes(original)
        changed_receipt = json.loads(original)
        changed_receipt['inputs']['documents/paper/main.tex'] = '0' * 64
        receipt.write_text(json.dumps(changed_receipt))
        with mock.patch.object(compare_words, 'inventory',
                               return_value=compare_words.inventory(repo)):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.snapshot_inputs(candidate_dir, self.root/'bad-receipt-copy',
                                              CapturedInputs(candidate_dir))
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('InputChanged', 'paper receipt differs from source inventory'))
        receipt.write_bytes(original)
        other = candidate_dir/'documents/paper-build/compat.tex'
        original_other = other.read_bytes()
        other.write_bytes(original_other + b'% changed\n')
        changed = compare_words.source_envelope(parent, parent_dir, candidate_dir,
                                                 inputs, before, after, args.unit)
        self.assertFalse(changed['available'])
        self.assertIn('paper input differs: documents/paper-build/compat.tex',
                      changed['reasons'])
        other.write_bytes(original_other)
        original_master = (candidate_dir/'documents/paper/main.tex').read_bytes()
        (candidate_dir/'documents/paper/main.tex').write_bytes(
            original_master.replace(b'textwidth=6.0in', b'textwidth=4.2in'))
        changed = compare_words.source_envelope(parent, parent_dir, candidate_dir,
                                                 inputs, before, after, args.unit)
        self.assertIn('paper input differs: documents/paper/main.tex', changed['reasons'])
        (candidate_dir/'documents/paper/main.tex').write_bytes(original_master)

        def encoded(rows):
            return ''.join('\t'.join(map(str, row)) + '\n' for row in rows).encode()
        old_rows = list(legacy['fragments'])
        new_rows = list(compare_words.scope_fragments(candidate_dir))
        direction = next(row for row in new_rows if row[1] == 'sec:say:direction' and
                         row[3] == 'refrain')
        old_direction = (source_name, direction[1], direction[2], direction[3], 250)
        new_direction = (source_name, direction[1], direction[2], direction[3], 246)
        old_rows = [old_direction if row == direction else row for row in old_rows]
        new_rows = [new_direction if row == direction else row for row in new_rows]
        source_order = (candidate_dir/'documents/paper-build/sources.tsv').read_text().splitlines()
        def row_order(row):
            return (0 if 'paper-pilot' in row[2] else 1,
                    source_order.index(row[0]), row[4])
        old_rows.sort(key=row_order)
        new_rows.sort(key=row_order)
        synthetic_parent = dict(legacy, fragments=old_rows)
        parent_source = parent_dir/source_name
        parent_source.write_bytes(old_bytes)
        parent_inclusion = parent_dir/'generated/documents/inclusions.tsv'
        parent_inclusion.write_bytes(encoded(old_rows))
        inclusion.write_bytes(encoded(new_rows))
        with mock.patch.object(compare_words, 'scope_fragments', return_value=new_rows):
            equal = compare_words.source_envelope(
                synthetic_parent, parent_dir, candidate_dir, inputs,
                authored_before, authored_after, args.unit)
        self.assertEqual(equal['inclusion_shift'], -4)
        self.assertTrue(equal['available'], equal['reasons'])
        wrong_rows = [tuple([*row[:4], 247]) if row == new_direction else row
                      for row in new_rows]
        inclusion.write_bytes(encoded(wrong_rows))
        with mock.patch.object(compare_words, 'scope_fragments', return_value=wrong_rows):
            changed = compare_words.source_envelope(
                synthetic_parent, parent_dir, candidate_dir, inputs,
                authored_before, authored_after, args.unit)
        self.assertFalse(changed['available'])
        self.assertIn('inclusion line differs: sec:say:direction', changed['reasons'])

    def test_real_conversion_adds_unit_and_scope_without_changing_residual_inputs(self):
        source = pathlib.Path(__file__).resolve().parents[2]
        repo = self.root/'real-conversion'
        subprocess.run(['git', 'clone', '--quiet', '--no-hardlinks', str(source), str(repo)],
                       check=True)
        published = compare_words.git(repo, 'rev-parse', '873125c^{commit}').decode().strip()
        parent = compare_words.git(repo, 'rev-parse', published+'^').decode().strip()
        subprocess.run(['git', '-C', str(repo), 'checkout', '--quiet', '--detach', parent],
                       check=True)
        for name in ('documents/paper/explainer/explainer.tex',
                     'units/statements/plain-intro.toml',
                     'units/scopes/explainer.toml',
                     'units/skeletons/explainer-pilot.toml',
                     'generated/documents/explainer-pilot/sec-reading-plain-intro.tex',
                     'generated/documents/inclusions.tsv',
                     'generated/paper/inputs.json'):
            path = repo/name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(compare_words.git(repo, 'show', published+':'+name))
        acceptance = repo/'documents/paper-build/self-description.json'
        record = json.loads(acceptance.read_text())
        for passage in record['passages']:
            source_bytes = (repo/passage['file']).read_bytes()
            anchor = passage['anchor'].encode()
            self.assertEqual(source_bytes.count(anchor), 1)
            start = source_bytes.index(anchor)
            width = passage['end'] - passage['start']
            self.assertEqual(compare_words.digest(source_bytes[start:start+width]),
                             passage['sha256'])
            passage['start'], passage['end'] = start, start+width
        acceptance.write_text(json.dumps(record, indent=2)+'\n')
        subprocess.run(['git', '-C', str(repo), 'add', '-A'], check=True)
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture',
                        '-c', 'user.email=fixture@example.invalid', 'commit', '--quiet',
                        '-m', 'plain paragraph conversion'], check=True)
        candidate = compare_words.git(repo, 'rev-parse', 'HEAD').decode().strip()
        unit = 'sec:reading:plain-intro'
        source_name = 'documents/paper/explainer/explainer.tex'
        old = compare_words.git(repo, 'show', parent+':'+source_name)
        first, last, _ = compare_words.derived_replacement(
            old, (repo/source_name).read_bytes(), unit)
        assignment = dict(schema=3, parent=parent, candidate=candidate,
                          source=source_name, first=first, last=last, unit=unit,
                          **{'source-blob': compare_words.git(
                              repo, 'rev-parse', parent+':'+source_name).decode().strip(),
                             'unit-path': 'units/statements/plain-intro.toml'})
        assignment_path = self.root/'real-assignment.json'
        assignment_path.write_text(json.dumps(assignment))
        before = {}
        for document in ('paper', 'explainer'):
            path = self.root/(document+'-real-before.pdf')
            path.write_bytes(compare_words.git(
                repo, 'show', parent+':generated/paper/'+document+'.pdf'))
            before[document] = path
        args = argparse.Namespace(repository=repo,
                                  before_paper=before['paper'],
                                  before_explainer=before['explainer'],
                                  after_paper=repo/'generated/paper/paper.pdf',
                                  after_explainer=repo/'generated/paper/explainer.pdf',
                                  before_source=(parent+':'+source_name+':'+
                                                 str(first)+'-'+str(last)),
                                  before_revision=None, unit=unit,
                                  assignment=assignment_path,
                                  output=self.root/'real-comparison')
        status = compare_words.compare(args)
        result = json.loads((args.output/'comparison.json').read_text())
        self.assertEqual((status, result['verdict']), (0, 'equal'), result['faults'])
        self.assertEqual(result['scope_admissibility']['verdict'], 'scope-admissible')
        changed = set(compare_words.git(repo, 'diff', '--name-only', parent, candidate,
                                        '--', 'units/statements', 'units/scopes').decode().splitlines())
        self.assertEqual(changed, {'units/statements/plain-intro.toml',
                                   'units/scopes/explainer.toml'})

    def test_native_target_unit_change_is_a_residual_input_change(self):
        source = pathlib.Path(__file__).resolve().parents[2]
        unit_path = 'units/statements/coda-flourishing-pole.toml'
        bridge = json.loads(assembly_json(source))
        expected = {unit_path}
        expected.update(path for path in (
            'units/corpus/result-pool.toml', 'units/corpus/result-modest.toml')
            if (source/path).is_file())
        self.assertEqual({row['path'] for row in bridge['inputs']
                          if row['path'].startswith('units/')}, expected)
        repo = self.root/'native-target-change'
        subprocess.run(['git', 'clone', '--quiet', '--no-hardlinks', str(source), str(repo)],
                       check=True)
        parent_revision = compare_words.git(repo, 'rev-parse', 'HEAD').decode().strip()
        unit = repo/unit_path
        original = unit.read_text()
        self.assertIn('The flourishing pole', original)
        unit.write_text(original.replace('The flourishing pole', 'A changed flourishing pole', 1))
        subprocess.run(['git', '-C', str(repo), 'add', unit_path], check=True)
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture',
                        '-c', 'user.email=fixture@example.invalid', 'commit', '--quiet',
                        '-m', 'changed native target source'], check=True)
        parent_dir = self.root/'native-target-parent'
        candidate_dir = self.root/'native-target-candidate'
        parent = compare_words.parent_snapshot(repo, parent_revision, parent_dir)
        held = CapturedInputs(repo)
        inputs = compare_words.snapshot_inputs(repo, candidate_dir, held)
        self.assertEqual({row['path'] for row in parent['inputs']
                          if row['path'].startswith('units/')}, expected)
        self.assertEqual({row['path'] for row in inputs
                          if row['path'].startswith('units/')}, expected)
        args = argparse.Namespace(unit='sec:coda:whole-field', before_source=None,
                                  before_revision=parent_revision)
        before, after = compare_words.selected_sources(repo, args, parent,
                                                        parent_dir, candidate_dir)
        envelope = compare_words.source_envelope(parent, parent_dir, candidate_dir,
                                                 inputs, before, after, args.unit)
        self.assertFalse(envelope['available'])
        self.assertEqual(envelope['reasons'], ['paper input differs: '+unit_path])

    def test_assigned_shadow_scope_refusals(self):
        source = pathlib.Path(__file__).resolve().parents[2]
        repo = self.root/'candidate'
        import subprocess
        subprocess.run(['git', 'clone', '--quiet', '--no-hardlinks', str(source), str(repo)],
                       check=True)
        revision = compare_words.git(repo, 'rev-parse', 'HEAD').decode().strip()
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture',
                        '-c', 'user.email=fixture@example.invalid', 'commit',
                        '--allow-empty', '--quiet', '-m', 'fixture candidate'], check=True)
        candidate = compare_words.git(repo, 'rev-parse', 'HEAD').decode().strip()
        parent_dir = self.root/'parent-snapshot'
        candidate_dir = self.root/'candidate-snapshot'
        parent = compare_words.parent_snapshot(repo, revision, parent_dir)
        held = CapturedInputs(repo)
        inputs = compare_words.snapshot_inputs(repo, candidate_dir, held)
        args = argparse.Namespace(unit='sec:reading:shadow', before_source=None,
                                  before_revision=revision)
        before, after = compare_words.selected_sources(repo, args, parent,
                                                        parent_dir, candidate_dir)
        envelope = compare_words.source_envelope(parent, parent_dir, candidate_dir,
                                                  inputs, before, after, args.unit)
        self.assertTrue(envelope['available'], envelope['reasons'])
        assignment = dict(schema=3, parent=revision, candidate=candidate,
                          source=before['source'], first=before['line'],
                          last=before['line'], unit=args.unit,
                          **{'source-blob': compare_words.git(
                              repo, 'rev-parse', '--verify',
                              revision + ':' + before['source']).decode().strip(),
                             'unit-path': 'units/statements/reading-shadow.toml'})
        assignment_path = self.root/'assignment.json'
        assignment_path.write_text(json.dumps(assignment))
        evidence = dict(verdict='equal', faults=[], differences=[],
                        units=[dict(unit=args.unit)])

        def check(value=assignment, result=evidence, coverage=envelope):
            return compare_words.assigned_plain_scope(repo, value, result, parent,
                                                       before, after, coverage)

        self.assertEqual(check()['verdict'], 'scope-admissible')
        self.assertEqual(compare_words.capture_assignment(repo, assignment_path)[0],
                         assignment)
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture',
                        '-c', 'user.email=fixture@example.invalid', 'commit',
                        '--allow-empty', '--quiet', '-m', 'fixture grandchild'], check=True)
        grandchild = compare_words.git(repo, 'rev-parse', 'HEAD').decode().strip()
        with self.assertRaises(compare_words.Fault) as raised:
            check(dict(assignment, candidate=grandchild))
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('AssignmentInvalid', 'comparison parent is not immediate'))
        with self.assertRaises(compare_words.Fault) as raised:
            check()
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('AssignmentInvalid', 'comparison parent is not immediate'))
        subprocess.run(['git', '-C', str(repo), 'reset', '--hard', candidate],
                       check=True, capture_output=True)
        readme = repo/'README.md'
        readme_bytes = readme.read_bytes()
        readme.write_bytes(readme_bytes + b'\nfixture edit\n')
        with self.assertRaises(compare_words.Fault) as raised:
            check()
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('InputChanged', 'candidate checkout differs from its commit'))
        subprocess.run(['git', '-C', str(repo), 'add', 'README.md'], check=True)
        readme.write_bytes(readme_bytes)
        with self.assertRaises(compare_words.Fault) as raised:
            check()
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('InputChanged', 'candidate checkout differs from its commit'))
        subprocess.run(['git', '-C', str(repo), 'reset', '--hard', candidate],
                       check=True, capture_output=True)
        register = repo/'registry/screen-destination-gaps.json'
        stored_register = register.read_bytes()
        register.write_bytes(stored_register + b' ')
        with self.assertRaises(compare_words.Fault) as raised:
            check()
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('InputChanged', 'candidate checkout differs from its commit'))
        register.write_bytes(stored_register)
        for name, altered, detail in (
                ('duplicate', assignment_path.read_text().replace(
                    '"schema": 3', '"schema": 3, "schema": 3'), 'duplicate assignment key'),
                ('unknown', json.dumps(dict(assignment, unknown=1)), 'assignment fields differ'),
                ('snake-case field', json.dumps(dict(
                    {key: value for key, value in assignment.items()
                     if key != 'source-blob'}, source_blob=assignment['source-blob'])),
                 'assignment fields differ'),
                ('old schema', json.dumps(dict(assignment, schema=2)), 'assignment schema differs')):
            with self.subTest(name=name):
                assignment_path.write_text(altered)
                with self.assertRaises(compare_words.Fault) as raised:
                    compare_words.capture_assignment(repo, assignment_path)
                self.assertIn(detail, raised.exception.row['detail'])
        for name, data in (('utf-16', json.dumps(assignment).encode('utf-16')),
                           ('byte-order mark', b'\xef\xbb\xbf' +
                            json.dumps(assignment).encode())):
            with self.subTest(name=name):
                assignment_path.write_bytes(data)
                with self.assertRaises(compare_words.Fault) as raised:
                    compare_words.capture_assignment(repo, assignment_path)
                self.assertEqual(raised.exception.row['code'], 'AssignmentInvalid')
        assignment_path.write_text(json.dumps(assignment))
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.capture_assignment(repo, repo/'candidate-assignment.json')
        self.assertEqual(raised.exception.row['detail'],
                         'assignment must be outside the candidate')
        changed = dict(assignment, parent='0' * 40)
        with self.assertRaises(compare_words.Fault) as raised:
            check(changed)
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('AssignmentInvalid', 'assignment parent differs'))
        changed = dict(assignment, **{'unit-path': 'README.md'})
        with self.assertRaises(compare_words.Fault) as raised:
            check(changed)
        self.assertEqual(raised.exception.row['detail'],
                         'assigned unit path class differs')
        changed = dict(assignment, unit='sec:reading:other')
        with self.assertRaises(compare_words.Fault) as raised:
            check(changed)
        self.assertEqual(raised.exception.row['code'], 'AssignmentInvalid')
        first_row = {key: assignment[key] for key in
                     ('source', 'source-blob', 'first', 'last', 'unit', 'unit-path')}
        first_row['previous-kind'] = 'native'
        duplicate = dict(assignment, occurrences=[first_row, first_row])
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.assigned_plain_scope(
                repo, duplicate, evidence, parent, before, after, envelope,
                [(before, after, args.unit), (before, after, args.unit)])
        self.assertEqual(raised.exception.row['detail'],
                         'assigned source intervals overlap')
        wrong_source = dict(first_row, source='documents/paper/explainer/explainer.tex')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.assigned_plain_scope(
                repo, dict(assignment, occurrences=[first_row, wrong_source]),
                evidence, parent, before, after, envelope,
                [(before, after, args.unit), (before, after, args.unit)])
        self.assertEqual(raised.exception.row['detail'], 'assigned source file differs')
        with self.assertRaises(compare_words.Fault) as raised:
            check(result=dict(evidence, verdict='different',
                              differences=[dict(code='TextDifference')]))
        self.assertEqual(raised.exception.row['code'], 'ComparisonIncomplete')
        with self.assertRaises(compare_words.Fault) as raised:
            check(coverage=dict(envelope, available=False,
                                reasons=['unaccounted source line']))
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('SourceEnvelopeChanged', 'unaccounted source line'))
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.assigned_plain_scope(
                repo, assignment, evidence, parent, before, after, envelope,
                [(before, after, args.unit), (before, after, args.unit)])
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('AssignmentInvalid', 'assigned occurrence count differs'))
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.decode_located('a \\CheckedSum{b}', {})
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('SyntaxUnsupported',
                          '<source>:2: unsupported source scalar U+005C'))
        layout_lines = [line for line in
                        (repo/'documents/paper/explainer/explainer.tex').read_text(
                            encoding='utf-8').splitlines()
                        if line.startswith('\\PaperParagraph{lastline}')]
        self.assertEqual(len(layout_lines), 1)
        layout_line = layout_lines[0]
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.decode_located(layout_line, {})
        self.assertEqual(raised.exception.row['code'], 'SyntaxUnsupported')
        old = compare_words.git(repo, 'show', revision + ':' + before['source'])
        marker = old[before['marker'][0]:before['marker'][1]]
        protected = dict(passages=[dict(id=999, file=before['source'],
                                        anchor=marker.decode().strip(),
                                        start=before['marker'][0], end=before['marker'][1],
                                        sha256=compare_words.digest(marker))])
        real_git = compare_words.git

        def protected_git(root, *parts):
            if parts == ('show', revision +
                          ':documents/paper-build/self-description.json'):
                return json.dumps(protected).encode()
            return real_git(root, *parts)

        with mock.patch.object(compare_words, 'git', side_effect=protected_git):
            with self.assertRaises(compare_words.Fault) as raised:
                check()
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('ProtectedSpan', 'assigned interval overlaps bound passage 999'))
        self.assertFalse(compare_words.unproved_paragraph_layout(
            b'\\PaperParagraph{stretch=2em}\nThe earlier paragraph.\n\n'))
        self.assertTrue(compare_words.unproved_paragraph_layout(
            b'\\PaperParagraph{stretch=2em}\n\n'))
        self.assertEqual(compare_words.derived_replacement(
            b'Before\n\nOne sentence.\n\nAfter\n',
            b'Before\n\n% generated: sec:reading:shadow\n\nAfter\n',
            'sec:reading:shadow'), (3, 3, 3))
        self.assertIsNone(compare_words.derived_replacement(
            b'Before\n\nOne sentence.\n\nAfter\n',
            b'Changed\n\n% generated: sec:reading:shadow\n\nAfter\n',
            'sec:reading:shadow'))
        historical = 'documents/paper/explainer/explainer.tex'
        self.assertEqual(compare_words.derived_replacement(
            compare_words.git(source, 'show',
                              'bbb1add6333c2b0449b0513eb2c8e92a3956120f:' + historical),
            compare_words.git(source, 'show',
                              '873125c90a54697397a2604ba2835a303357ee5d:' + historical),
            'sec:reading:plain-intro'), (47, 50, 47))
        with mock.patch.object(compare_words, 'unproved_paragraph_layout',
                               return_value=True):
            with self.assertRaises(compare_words.Fault) as raised:
                check()
        self.assertEqual(raised.exception.row['code'], 'LayoutUnsupported')
        exit_code, compared = self.conversion_run(options=dict(
            repository=repo, assignment=assignment_path, before_revision=revision))
        self.assertEqual((exit_code, compared['verdict']), (0, 'equal'))
        self.assertEqual(compared['scope_admissibility']['verdict'], 'scope-admissible')
        exit_code, compared = self.conversion_run(options=dict(
            repository=repo, assignment=assignment_path, before_revision=revision,
            unit='sec:wrong'))
        self.assertEqual((exit_code, compared['verdict']), (2, 'refused'))
        self.assertEqual((compared['faults'][0]['rule'], compared['faults'][0]['code'],
                          compared['faults'][0]['detail']),
                         ('M082', 'AssignmentInvalid',
                          'comparison selector and captured assignment differ'))

        def replaced_after_capture(path, document, side, value):
            if document == 'paper' and side == 'before':
                assignment_path.write_text(json.dumps(dict(assignment, first=0)))

        exit_code, compared = self.conversion_run(
            replaced_after_capture, dict(repository=repo, assignment=assignment_path,
                                         before_revision=revision))
        self.assertEqual((exit_code, compared['verdict']), (2, 'refused'))
        self.assertTrue(any(row['code'] == 'InputChanged' and row['rule'] == 'M082'
                            for row in compared['faults']))
        self.assertEqual(compared['scope_admissibility']['verdict'], 'refused')
        assignment_path.write_text(json.dumps(assignment))
        subprocess.run(['git', '-C', str(repo), 'reset', '--hard', revision],
                       check=True, capture_output=True)

        def committed_change(name, data=None, executable=False, symlink=None):
            target = repo/name
            if symlink is not None:
                target.unlink()
                target.symlink_to(symlink)
            elif data is None:
                target.unlink()
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
                if executable:
                    target.chmod(0o755)
            subprocess.run(['git', '-C', str(repo), 'add', '-A'], check=True)
            subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture',
                            '-c', 'user.email=fixture@example.invalid', 'commit',
                            '--quiet', '-m', 'fixture change'], check=True)
            commit = compare_words.git(repo, 'rev-parse', 'HEAD').decode().strip()
            with self.assertRaises(compare_words.Fault) as raised:
                check(dict(assignment, candidate=commit))
            subprocess.run(['git', '-C', str(repo), 'reset', '--hard', revision],
                           check=True, capture_output=True)
            subprocess.run(['git', '-C', str(repo), 'clean', '-fd'],
                           check=True, capture_output=True)
            return raised.exception.row

        for name in ('README.md', 'documents/paper/parts/partE.tex',
                     'tools/paper/compare_words.py'):
            with self.subTest(path=name):
                row = committed_change(name, (repo/name).read_bytes() + b'\nchanged\n')
                self.assertEqual((row['code'], row['detail']),
                                 ('PathIneligible',
                                  'unassigned or capability path changed: ' + name))
        row = committed_change('README.md')
        self.assertEqual((row['code'], row['detail']),
                         ('PathIneligible',
                          'unassigned or capability path changed: README.md'))
        register = 'registry/html-block-gaps.json'
        row = committed_change(register, (repo / register).read_bytes() + b' ')
        self.assertEqual((row['code'], row['detail']),
                         ('PathIneligible', 'derived register needs canonical checking'))
        imported = repo/'tools/paper/bibroute.py'
        relocated = repo/'zz-generated/bibroute.py'
        relocated.parent.mkdir()
        relocated.write_bytes(imported.read_bytes())
        imported.unlink()
        subprocess.run(['git', '-C', str(repo), 'config', 'diff.renames', 'true'],
                       check=True)
        subprocess.run(['git', '-C', str(repo), 'add', '-A'], check=True)
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture',
                        '-c', 'user.email=fixture@example.invalid', 'commit',
                        '--quiet', '-m', 'fixture rename'], check=True)
        renamed = compare_words.git(repo, 'rev-parse', 'HEAD').decode().strip()
        hidden = compare_words.git(repo, 'diff', '--find-renames', '--name-only', '-z',
                                   revision, renamed, '--').decode()
        visible = compare_words.git(repo, 'diff', '--no-renames', '--name-only', '-z',
                                    revision, renamed, '--').decode()
        self.assertNotIn('tools/paper/bibroute.py', hidden)
        self.assertIn('tools/paper/bibroute.py', visible)
        with self.assertRaises(compare_words.Fault) as raised:
            check(dict(assignment, candidate=renamed))
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('PathIneligible',
                          'unassigned or capability path changed: tools/paper/bibroute.py'))
        subprocess.run(['git', '-C', str(repo), 'reset', '--hard', revision],
                       check=True, capture_output=True)
        subprocess.run(['git', '-C', str(repo), 'clean', '-fd'],
                       check=True, capture_output=True)
        rogue = 'generated/documents/unlisted.txt'
        row = committed_change(rogue, b'unlisted generated text\n')
        self.assertEqual((row['code'], row['detail']),
                         ('PathIneligible',
                          'unassigned or capability path changed: ' + rogue))
        unit_name = assignment['unit-path']
        row = committed_change(unit_name, (repo/unit_name).read_bytes(), executable=True)
        self.assertEqual((row['code'], row['detail']),
                         ('PathIneligible',
                          'changed path is not an ordinary file: ' + unit_name))
        row = committed_change(unit_name, symlink=str(source/unit_name))
        self.assertEqual((row['code'], row['detail']),
                         ('PathIneligible',
                          'changed path is not an ordinary file: ' + unit_name))
        subprocess.run(['git', '-C', str(repo), 'config', 'core.fileMode', 'false'],
                       check=True)
        subprocess.run(['git', '-C', str(repo), 'update-index', '--chmod=+x', '--',
                        unit_name], check=True)
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture',
                        '-c', 'user.email=fixture@example.invalid', 'commit',
                        '--quiet', '-m', 'fixture index mode'], check=True)
        mode_candidate = compare_words.git(repo, 'rev-parse', 'HEAD').decode().strip()
        with self.assertRaises(compare_words.Fault) as raised:
            check(dict(assignment, candidate=mode_candidate))
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('PathIneligible',
                          'changed commit path is not an ordinary file: ' + unit_name))
        subprocess.run(['git', '-C', str(repo), 'reset', '--hard', revision],
                       check=True, capture_output=True)
        subprocess.run(['git', '-C', str(repo), 'config', 'core.fileMode', 'true'],
                       check=True)
        acceptance = repo/'documents/paper-build/self-description.json'
        altered = json.loads(acceptance.read_bytes())
        altered['passages'][0]['reading_acceptance'] = 'changed'
        row = committed_change('documents/paper-build/self-description.json',
                               json.dumps(altered).encode())
        self.assertEqual((row['code'], row['detail']),
                         ('ProtectedSpan', 'accepted passage identity differs'))

    def test_authored_substitution_and_offset_only_acceptance(self):
        source = pathlib.Path(__file__).resolve().parents[2]
        repo = self.root/'authored-candidate'
        import subprocess
        subprocess.run(['git', 'clone', '--quiet', '--no-hardlinks', str(source), str(repo)],
                       check=True)
        source_name = 'documents/paper/parts/front.tex'
        acceptance_name = 'documents/paper-build/self-description.json'
        unit = 'sec:reading:shadow'
        marker = ('% generated: ' + unit + '\n').encode()
        authored = b'A shadow paragraph.\n'
        source_file = repo/source_name
        original = source_file.read_bytes()
        self.assertEqual(original.count(marker), 1)
        source_file.write_bytes(original.replace(marker, authored, 1))

        def move_offsets():
            record_path = repo/acceptance_name
            record = json.loads(record_path.read_bytes())
            for passage in record['passages']:
                text = (repo/passage['file']).read_bytes()
                anchor = passage['anchor'].encode()
                self.assertEqual(text.count(anchor), 1)
                at = text.index(anchor)
                length = passage['end'] - passage['start']
                positions = [start for start in range(max(0, at - length), at + 1)
                             if compare_words.digest(text[start:start + length]) ==
                             passage['sha256']]
                self.assertEqual(len(positions), 1, passage['id'])
                passage['start'] = positions[0]
                passage['end'] = positions[0] + length
            record_path.write_text(json.dumps(record, indent=2) + '\n')

        def commit(message):
            subprocess.run(['git', '-C', str(repo), 'add', '-A'], check=True)
            subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture',
                            '-c', 'user.email=fixture@example.invalid', 'commit',
                            '--quiet', '-m', message], check=True)
            return compare_words.git(repo, 'rev-parse', 'HEAD').decode().strip()

        move_offsets()
        parent_id = commit('fixture authored parent')
        parent_source = source_file.read_bytes()
        first = parent_source[:parent_source.index(authored)].count(b'\n') + 1
        start = parent_source.index(authored)
        before = dict(kind='authored', source=source_name, line=first,
                      range=[start, start + len(authored)])
        after = dict(kind='native', source=source_name, line=first,
                     marker=[start, start + len(marker)])
        parent = dict(commit=parent_id)
        assignment = dict(schema=3, parent=parent_id, source=source_name,
                          first=first, last=first, unit=unit,
                          **{'source-blob': compare_words.git(
                              repo, 'rev-parse', parent_id + ':' + source_name
                          ).decode().strip(),
                             'unit-path': 'units/statements/reading-shadow.toml'})
        evidence = dict(verdict='equal', faults=[], differences=[])
        envelope = dict(available=True, reasons=[])

        def candidate(extra):
            source_file.write_bytes(parent_source.replace(authored, marker, 1) + extra)
            move_offsets()
            return commit('fixture candidate')

        candidate_id = candidate(b'')
        self.assertNotEqual(compare_words.git(repo, 'show',
                            parent_id + ':' + acceptance_name),
                            compare_words.git(repo, 'show',
                            candidate_id + ':' + acceptance_name))
        for passage in json.loads((repo/acceptance_name).read_bytes())['passages']:
            source_bytes = (repo/passage['file']).read_bytes()
            self.assertEqual(compare_words.digest(
                source_bytes[passage['start']:passage['end']]),
                passage['sha256'], passage['id'])
            self.assertEqual(source_bytes.count(passage['anchor'].encode()),
                             1, passage['id'])
        self.assertEqual(compare_words.assigned_plain_scope(
            repo, dict(assignment, candidate=candidate_id), evidence, parent,
            before, after, envelope)['verdict'], 'scope-admissible')
        subprocess.run(['git', '-C', str(repo), 'reset', '--hard', parent_id],
                       check=True, capture_output=True)
        candidate_id = candidate(b'\n% unrelated edit\n')
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.assigned_plain_scope(
                repo, dict(assignment, candidate=candidate_id), evidence, parent,
                before, after, envelope)
        self.assertEqual((raised.exception.row['code'], raised.exception.row['detail']),
                         ('AssignmentInvalid',
                          'assigned interval differs from commit source substitution'))

    def test_two_assigned_paragraphs_locate_changed_second_word(self):
        source = pathlib.Path(__file__).resolve().parents[2]
        revision = compare_words.git(source, 'rev-parse', 'HEAD').decode().strip()
        shadow = ('documents/paper/parts/front.tex', 'sec:reading:shadow',
                  'units/statements/reading-shadow.toml')
        intro = ('documents/paper/parts/coda.tex', 'sec:coda:eight-words',
                 'units/statements/coda-eight-words.toml')
        rows = {key: line for path, key, _, role, line in scope_fragments(source)
                if role == 'paragraph' for selected in (shadow, intro)
                if (path, key) == selected[:2]}
        if set(rows) != {shadow[1], intro[1]}:
            self.fail('the two declared paragraph occurrences are required')
        occurrences = []
        for path, key, unit_path in (shadow, intro):
            occurrences.append(dict(source=path, first=rows[key], last=rows[key],
                                    unit=key, **{'source-blob': compare_words.git(
                                        source, 'rev-parse', revision + ':' + path
                                    ).decode().strip(), 'unit-path': unit_path,
                                        'previous-kind': 'native'}))
        assignment = dict(schema=3, parent=revision, candidate=revision,
                          occurrences=occurrences,
                          **{key: occurrences[0][key] for key in
                             ('source', 'source-blob', 'first', 'last', 'unit', 'unit-path')})
        assigned_path = self.root/'two-paragraph-assignment.json'
        assigned_path.write_text(json.dumps(assignment))
        fragment = read(source/'generated/documents/paper-pilot/sec-coda-eight-words.tex')
        body = fragment.split('}{', 1)[1].removesuffix('}\n')
        rendered, _, _ = compare_words.decode_paragraph(
            body.split('\\virtue', 1)[0].rstrip(), bindings(source))

        def paper(path, document, side, value):
            if document != 'paper':
                return
            second = rendered if side == 'before' else rendered.replace(
                'document', 'manual', 1)
            value['stream'] += ' ' + second
            value['offsets'] = [[1, index] for index in range(len(value['stream']))]

        def unchanged(path, document, side, value):
            if document == 'paper':
                value['stream'] += ' ' + rendered
                value['offsets'] = [[1, index] for index in range(len(value['stream']))]

        original = compare_words.selected_sources
        def plain_selection(*arguments, **options):
            old, new = original(*arguments, **options)
            if arguments[1].unit == intro[1]:
                old = dict(old, body=old['body'].split(b'\\virtue', 1)[0].rstrip())
                new = dict(new, body=new['body'].split(b'\\virtue', 1)[0].rstrip())
            return old, new

        with mock.patch.object(compare_words, 'selected_sources', side_effect=plain_selection), \
                mock.patch.object(compare_words, 'assigned_plain_scope',
                               return_value={'verdict': 'scope-admissible'}):
            exit_code, result = self.conversion_run(unchanged,
                                                    dict(assignment=assigned_path))
        self.assertEqual((exit_code, result['verdict']), (0, 'equal'))
        self.assertEqual(result['coverage']['decoded_units'], [shadow[1], intro[1]])

        def changed_second(*arguments, **options):
            old, new = plain_selection(*arguments, **options)
            if arguments[1].unit == intro[1]:
                new = dict(new, body=new['body'].replace(b'document', b'manual', 1),
                           blob_bytes=new['blob_bytes'].replace(
                               b'document', b'manual', 1))
            return old, new

        with mock.patch.object(compare_words, 'selected_sources', side_effect=changed_second), \
                mock.patch.object(compare_words, 'assigned_plain_scope',
                                  return_value={'verdict': 'scope-admissible'}):
            exit_code, result = self.conversion_run(paper, dict(assignment=assigned_path))
        self.assertEqual((exit_code, result['verdict']), (1, 'different'))
        self.assertEqual(result['coverage']['decoded_units'], [shadow[1], intro[1]])
        self.assertTrue(any(row.get('unit') == intro[1] and row['part'] == 'source/body'
                            for row in result['differences']))
        stream = next(row for row in result['differences'] if
                      row.get('document') == 'paper' and row['part'] == 'stream')
        self.assertEqual(stream['location']['after']['raw'][0][1],
                         result['documents']['paper']['after']['stream'].index('manual'))
        with mock.patch.object(compare_words, 'selected_sources', side_effect=changed_second), \
                mock.patch.object(compare_words, 'assigned_plain_scope', side_effect=
                                  compare_words.Fault('ComparisonIncomplete',
                                                      'complete comparison is not equal',
                                                      'final')):
            exit_code, result = self.conversion_run(paper, dict(assignment=assigned_path))
        self.assertEqual((exit_code, result['verdict']), (1, 'different'))
        self.assertEqual(result['scope_admissibility']['faults'][0]['rule'], 'M082')
        self.assertEqual(result['scope_admissibility']['faults'][0]['code'],
                         'ComparisonIncomplete')

    def test_single_paper_paragraph_locates_changed_word(self):
        source = pathlib.Path(__file__).resolve().parents[2]
        fragment = read(source/'generated/documents/paper-pilot/sec-coda-eight-words.tex')
        body = fragment.split('}{', 1)[1].removesuffix('}\n')
        rendered, _, _ = compare_words.decode_paragraph(
            body.split('\\virtue', 1)[0].rstrip(), bindings(source))

        def changed(path, document, side, value):
            if document == 'paper':
                value['stream'] = rendered + ' Companion text'
                if side == 'after':
                    value['stream'] = value['stream'].replace('document', 'manual')
                value['offsets'] = [[1, index] for index in range(len(value['stream']))]

        def unchanged(path, document, side, value):
            if document == 'paper':
                value['stream'] = rendered + ' Companion text'
                value['offsets'] = [[1, index] for index in range(len(value['stream']))]

        original = compare_words.selected_sources

        def changed_source(*arguments, **options):
            old, new = original(*arguments, **options)
            old = dict(old, body=old['body'].split(b'\\virtue', 1)[0].rstrip())
            new = dict(new, body=new['body'].split(b'\\virtue', 1)[0].rstrip())
            new = dict(new, body=new['body'].replace(b'document', b'manual', 1),
                       blob_bytes=new['blob_bytes'].replace(
                           b'document', b'manual', 1))
            return old, new

        with mock.patch.object(compare_words, 'selected_sources',
                               side_effect=changed_source):
            exit_code, record = self.conversion_run(changed, dict(
                unit='sec:coda:eight-words', before_revision='HEAD'))
        self.assertEqual((exit_code, record['verdict']), (1, 'different'))
        self.assertTrue(any(row['part'] == 'stream' and
                            row['document'] == 'paper'
                            for row in record['differences']))
        with mock.patch.object(compare_words, 'selected_sources',
                               side_effect=changed_source):
            exit_code, record = self.conversion_run(unchanged, dict(
                unit='sec:coda:eight-words', before_revision='HEAD'))
        self.assertEqual((exit_code, record['verdict']), (2, 'refused'))
        self.assertEqual(record['faults'][0]['code'], 'CorrespondenceInvalid')

    def test_nested_explainer_paragraph_keeps_virtue_words(self):
        source = pathlib.Path(__file__).resolve().parents[2]
        fragment = read(source/'generated/documents/explainer-pilot/sec-explainer-two-feet.tex')
        line, = (line for line in fragment.splitlines() if line.startswith(
            r'\PaperNativeParagraph{sec:explainer:eight-words}{'))
        body = line.split('}{', 1)[1].removesuffix('}')
        words = ('wonder', 'beauty', 'goodness', 'kindness', 'love', 'wisdom',
                 'truth', 'health')
        virtues = {word: {'key': 'v:' + word, 'target': '#virtue-' + word}
                   for word in words}
        path = 'generated/documents/explainer-pilot/sec-explainer-two-feet.tex'
        original = compare_words.decode_located(body, bindings(source), path=path,
                                                virtues=virtues)
        self.assertEqual([row['word'] for row in original['virtues']], list(words))
        changed = body.replace(r'\papervirtue{wonder}', r'\papervirtue{wisdom}', 1)
        changed_record = compare_words.decode_located(changed, bindings(source), path=path,
                                                      virtues=virtues)
        self.assertNotEqual(original['extracted'], changed_record['extracted'])
        missing = body.replace(r'\papervirtue{wonder}', 'wonder', 1)
        missing_record = compare_words.decode_located(missing, bindings(source), path=path,
                                                      virtues=virtues)
        self.assertNotEqual(original['virtues'], missing_record['virtues'])

    def test_two_authored_paragraphs_in_one_source(self):
        source = 'documents/paper/parts/plain.tex'
        old = b'Before\nfirst alpha\nfirst end\nBetween\nsecond beta\nAfter\n'
        first_key, second_key = 'sec:reading:first', 'sec:reading:second'
        first_marker = ('% generated: ' + first_key + '\n').encode()
        second_marker = ('% generated: ' + second_key + '\n').encode()
        candidate = b'Before\n' + first_marker + b'Between\n' + second_marker + b'After\n'
        parent_dir = self.root/'old-two'
        candidate_dir = self.root/'new-two'
        for directory, body in ((parent_dir, old), (candidate_dir, candidate)):
            (directory/'documents/paper/parts').mkdir(parents=True)
            (directory/source).write_bytes(body)
            (directory/'documents/paper-build').mkdir()
            (directory/'documents/paper-build/sources.tsv').write_text(source + '\n')
            (directory/'generated/documents/paper-pilot').mkdir(parents=True)
            (directory/'generated/paper').mkdir()
        rows = []
        for key, line, text in ((first_key, 2, 'first alpha first end'),
                                (second_key, 4, 'second gamma')):
            fragment = 'generated/documents/paper-pilot/' + key.replace(':', '-') + '.tex'
            (candidate_dir/fragment).write_text(
                '\\PaperNativeParagraph{' + key + '}{' + text + '}\n')
            rows.append((source, key, fragment, 'paragraph', line))
        (parent_dir/'generated/documents/inclusions.tsv').write_bytes(b'')
        (candidate_dir/'generated/documents/inclusions.tsv').write_bytes(
            b''.join(('\t'.join(map(str, row)) + '\n').encode() for row in rows))
        (parent_dir/'generated/paper/inputs.json').write_text(json.dumps(
            {'inputs': {source: compare_words.digest(old)}}))
        (candidate_dir/'generated/paper/inputs.json').write_text(json.dumps(
            {'inputs': {source: compare_words.digest(candidate), **{
                row[2]: compare_words.digest((candidate_dir/row[2]).read_bytes())
                for row in rows}}}))
        parent = dict(commit='a' * 40, fragments=[], receipt={
            'inputs': {source: compare_words.digest(old)}},
            inputs=[dict(path=source, blob=compare_words.digest(old))])
        authored = {}
        for key, first, last in ((first_key, 2, 3), (second_key, 5, 5)):
            start = sum(len(line) for line in old.splitlines(keepends=True)[:first - 1])
            end = sum(len(line) for line in old.splitlines(keepends=True)[:last])
            authored[key] = dict(commit=parent['commit'], path=source, first=first,
                                 blob=compare_words.digest(old), blob_bytes=old,
                                 bytes=[start, end], body=old[start:end].decode())
        def source_row(repo, selector):
            return authored[first_key if selector.endswith(':2-3') else second_key]

        selections = []
        with mock.patch.object(compare_words, 'scope_fragments', return_value=rows), \
                mock.patch.object(compare_words, 'source_blob', side_effect=source_row):
            for key, first, last, shift in ((first_key, 2, 3, 0),
                                             (second_key, 5, 5, -1)):
                args = argparse.Namespace(unit=key, before_source=(
                    parent['commit'] + ':' + source + ':' + str(first) + '-' + str(last)),
                    before_revision=None)
                before, after = compare_words.selected_sources(
                    self.root, args, parent, parent_dir, candidate_dir,
                    defer_outside=True, line_shift=shift)
                selections.append((before, after, key))
            envelope = compare_words.source_envelope(
                parent, parent_dir, candidate_dir,
                [dict(path=source)], *selections[0], selections[1:])
        self.assertTrue(envelope['available'], envelope['reasons'])
        candidate_source = candidate_dir/source
        candidate_source.write_bytes(candidate.replace(b'Between', b'Changed'))
        with mock.patch.object(compare_words, 'scope_fragments', return_value=rows):
            changed_envelope = compare_words.source_envelope(
                parent, parent_dir, candidate_dir,
                [dict(path=source)], *selections[0], selections[1:])
        self.assertFalse(changed_envelope['available'])
        self.assertIn('paper input differs: ' + source, changed_envelope['reasons'])
        candidate_source.write_bytes(candidate)
        old_second, new_second, _ = selections[1]
        self.assertEqual(compare_words.shifted_marker_line(selections, source, 5), 4)
        self.assertEqual(new_second['line'], 4)
        before_text = compare_words.decode_located(old_second['body'].decode(), {})
        after_text = compare_words.decode_located(new_second['body'].decode(), {})
        self.assertEqual(before_text['semantic'], 'second beta')
        self.assertEqual(after_text['semantic'], 'second gamma')
        self.assertEqual(after_text['semantic'].index('gamma'), 7)
        before_stream = 'Before first alpha first end Between second beta After'
        after_stream = before_stream.replace('second beta', 'second gamma')
        proofs = []
        for decoded, stream, side in ((before_text, before_stream, 'before'),
                                      (after_text, after_stream, 'after')):
            offsets = [[1, index] for index in range(len(stream))]
            proof = compare_words.align_located(decoded, dict(
                stream=stream, offsets=offsets, events=[],
                raw_pages=[dict(text=stream)]), 'paper', side)
            proofs.append(proof)
        self.assertEqual(proofs[0]['range'][0], before_stream.index('second beta'))
        self.assertEqual(proofs[1]['range'][0], after_stream.index('second gamma'))

    def test_two_paragraphs_in_one_document_locate_second_difference(self):
        source = pathlib.Path(__file__).resolve().parents[2]
        revision = compare_words.git(source, 'rev-parse', 'HEAD').decode().strip()
        path = 'documents/paper/parts/front.tex'
        shadow = 'sec:reading:shadow'
        second = 'sec:reading:second'
        line = next(position for name, key, _, role, position in scope_fragments(source)
                    if name == path and key == shadow and role == 'paragraph')
        blob = compare_words.git(source, 'rev-parse', revision + ':' + path).decode().strip()
        first = dict(source=path, first=line, last=line, unit=shadow,
                     **{'source-blob': blob,
                        'unit-path': 'units/statements/reading-shadow.toml',
                        'previous-kind': 'native'})
        second_row = dict(first, first=line + 4, last=line + 4, unit=second,
                          **{'previous-kind': 'authored'})
        assignment = dict(schema=3, parent=revision, candidate=revision,
                          occurrences=[first, second_row],
                          **{key: first[key] for key in
                             ('source', 'source-blob', 'first', 'last', 'unit', 'unit-path')})
        assigned_path = self.root/'same-document-assignment.json'
        assigned_path.write_text(json.dumps(assignment))
        real_sources = compare_words.selected_sources
        fragment_path = 'generated/documents/paper-pilot/sec-reading-shadow.tex'
        before_blob = (source/path).read_bytes()
        before = dict(kind='authored', path=path, body=b'second beta',
                      blob_bytes=before_blob, range=[0, 11], source=path,
                      line=line + 4, commit=revision, blob=blob)
        prefix = ('\\PaperNativeParagraph{' + second + '}{').encode()
        wrapper = prefix + b'second gamma}\n'
        after = dict(kind='native', path=fragment_path, body=b'second gamma',
                     blob_bytes=wrapper, range=[len(prefix), len(wrapper) - 2],
                     source=path, marker=[0, 0], line=line + 4)

        def selected(*arguments, **options):
            if arguments[1].unit == second:
                return before, after
            return real_sources(*arguments, **options)

        def changed(path, document, side, value):
            if document != 'paper':
                return
            value['stream'] += ' second ' + ('beta' if side == 'before' else 'gamma')
            value['offsets'] = [[1, index] for index in range(len(value['stream']))]

        with mock.patch.object(compare_words, 'selected_sources', side_effect=selected), \
                mock.patch.object(compare_words, 'source_envelope', return_value=dict(
                    available=True, reasons=[], rows=[])), \
                mock.patch.object(compare_words, 'assigned_plain_scope', side_effect=
                                  compare_words.Fault('ComparisonIncomplete',
                                                      'complete comparison is not equal',
                                                      'final')):
            exit_code, result = self.conversion_run(changed, dict(assignment=assigned_path))
        self.assertEqual((exit_code, result['verdict']), (1, 'different'))
        self.assertTrue(any(row.get('unit') == second and row['part'] == 'source/body'
                            for row in result['differences']))
        stream = next(row for row in result['differences'] if row['part'] == 'stream'
                      and row['document'] == 'paper')
        self.assertEqual(stream['location']['after']['raw'][0][1],
                         result['documents']['paper']['after']['stream'].index('gamma'))

    def test_independent_side_sources(self):
        def proof(source, raw, side):
            decoded = compare_words.decode_located(source, {})
            stream, offsets, events = compare_words.normalize(
                raw, [[1, index] for index in range(len(raw))])
            pdf = dict(stream=stream, offsets=offsets, events=events,
                       raw_pages=[dict(text=raw)])
            match = compare_words.align_located(decoded, pdf, 'paper', side)
            return decoded, match, compare_words.canonicalize(pdf, match)[0]
        for old_source, new_source, raw in (
                ('non-linear', 'nonlinear', 'non-\nlinear'),
                ('dependence', 'de-pendence', 'de-\npendence')):
            with self.subTest(old=old_source, new=new_source):
                old, old_match, old_stream = proof(old_source, raw, 'before')
                new, new_match, new_stream = proof(new_source, raw, 'after')
                self.assertNotEqual(old['semantic'], new['semantic'])
                self.assertNotEqual(old_stream, new_stream)
                self.assertEqual(old_match['decisions'][0]['raw'],
                                 new_match['decisions'][0]['raw'])
                self.assertNotEqual(old_match['decisions'][0]['decision'],
                                    new_match['decisions'][0]['decision'])
        old, _, old_stream = proof('dependence', 'de-\npendence', 'before')
        new, _, new_stream = proof('independence', 'in-\ndependence', 'after')
        self.assertNotEqual(old['semantic'], new['semantic'])
        self.assertNotEqual(old_stream, new_stream)
        lookup = bindings(pathlib.Path(__file__).resolve().parents[2])
        changed_lookup = copy.deepcopy(lookup)
        changed_lookup['src:nd']['target'] = 'node:changed'
        before = compare_words.decode_located('A \\ftag{src:nd}', lookup)
        after = compare_words.decode_located('A \\ftag{src:nd}', changed_lookup)
        self.assertEqual(before['extracted'], after['extracted'])
        self.assertNotEqual(before['tags'], after['tags'])
        split_lookup = copy.deepcopy(lookup)
        split_lookup['src:nd']['spelling'] = 'foo-bar'
        tagged = compare_words.decode_located('A \\ftag{src:nd}', split_lookup)
        raw = 'A FOO-\nBAR'
        stream, offsets, events = compare_words.normalize(
            raw, [[1, i] for i in range(len(raw))])
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.align_located(tagged, dict(stream=stream, offsets=offsets,
                events=events, raw_pages=[dict(text=raw)]), 'paper', 'after')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')

    def test_negative_witness(self):
        def residual(path, document, side, value):
            if document != 'paper':
                return
            value['stream'] = value['stream'].replace('Closing', 'pre-and')
            value['offsets'] = [[1, index] for index in range(len(value['stream']))]
            position = value['stream'].rindex('pre-and') + 3
            value['raw_pages'] = [dict(text=value['stream'] + '\n')]
            value['events'] = [dict(operation='LineEndPunctuation',
                                   normalized_offset=position, raw=[1, position],
                                   spelling='-', intervening_space=False,
                                   raw_line='pre-\nand', newline=[1, len(value['stream'])],
                                   continuation=[1, position + 1])]
        with mock.patch.object(compare_words, 'source_envelope', return_value=dict(
                available=False, reasons=['captured width differs'], rows=[])):
            exit_code, record = self.conversion_run(residual)
        self.assertEqual((exit_code, record['verdict']), (2, 'refused'))
        self.assertEqual(record['faults'][0]['code'], 'SourceEnvelopeChanged')
        self.assertIn('captured width differs', record['faults'][0]['detail'])
        self.assertEqual(record['coverage']['unresolved'], 2)
        def changed_residual(path, document, side, value):
            residual(path, document, side, value)
            if document == 'paper' and side == 'after':
                value['stream'] = value['stream'].replace('pre-and', 'pre and')
        exit_code, record = self.conversion_run(changed_residual)
        self.assertEqual((exit_code, record['verdict']), (2, 'refused'))
        self.assertEqual(record['faults'][0]['code'], 'LineEndUnresolved')
        self.assertEqual(record['faults'][0]['detail'], 'residual compared text differs')
        def title_change(path, document, side, value):
            residual(path, document, side, value)
            if document == 'paper' and side == 'after':
                value['titles'] = ['Other title']
        with mock.patch.object(compare_words, 'source_envelope', return_value=dict(
                available=False, reasons=['captured width differs'], rows=[])):
            exit_code, record = self.conversion_run(title_change)
        self.assertEqual((exit_code, record['verdict']), (1, 'different'))
        self.assertEqual(record['differences'][0]['part'], 'titles')
        self.assertEqual(record['differences'][0]['proof'], 'direct')
        self.assertEqual(record['faults'], [])

    def test_tag_targets_in_comparison(self):
        real_bindings = compare_words.bindings
        def changed_target(root):
            rows = real_bindings(root)
            if root.name == 'snapshot':
                rows['src:nd'] = dict(rows['src:nd'], target='node:another')
            return rows
        with mock.patch.object(compare_words, 'bindings', side_effect=changed_target):
            exit_code, record = self.conversion_run()
        self.assertEqual((exit_code, record['verdict']), (1, 'different'))
        self.assertEqual([(row['part'], row['proof']) for row in record['differences']],
                         [('source/tags', 'direct')])
        self.assertEqual(record['faults'], [])

    def test_companion_virtue_token_keeps_word_and_identity(self):
        roster = {'wonder': {'key': 'v:wonder', 'target': 'sem:weld:wonder'}}
        located = compare_words.decode_located('The \\papervirtue{wonder}.', {},
                                               'documents/paper/explainer/explainer.tex', virtues=roster)
        self.assertEqual(located['virtues'], [
            {'command': 'papervirtue', 'word': 'wonder', 'key': 'v:wonder',
             'target': 'sem:weld:wonder'}])
        self.assertEqual(located['extracted'], 'The wonder.')
        self.assertNotEqual(located['virtues'], compare_words.decode_located(
            'The wonder.', {}, 'documents/paper/explainer/explainer.tex', virtues=roster)['virtues'])
        for body, path, reason in [
            ('\\papervirtue{unknown}', 'documents/paper/explainer/explainer.tex',
             'unknown virtue roster word: unknown'),
            ('\\papervirtue{wonder}', 'documents/paper/parts/coda.tex',
             'unsupported virtue command or document'),
            ('\\virtue{wonder}', 'documents/paper/parts/coda.tex',
             'unsupported source scalar U\\+005C'),
            ('\\papervirtue{\\name}', 'documents/paper/explainer/explainer.tex',
             'unsupported virtue command or document'),
        ]:
            with self.subTest(body=body, path=path), self.assertRaisesRegex(compare_words.Fault, reason):
                compare_words.decode_located(body, {}, path, virtues=roster)

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
                    source, 'show', 'HEAD:generated/paper/' + document + '.pdf'))
            before[document] = path
        args = argparse.Namespace(repository=source, before_paper=before['paper'],
                                  before_explainer=before['explainer'],
                                  after_paper=source/'generated/paper/paper.pdf',
                                  after_explainer=source/'generated/paper/explainer.pdf',
                                  before_source=None, before_revision='HEAD',
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
        occurrence = result['units'][0]['occurrences'][0]
        for side in ('before', 'after'):
            self.assertEqual(occurrence['tokens'][side][0]['source_range'][0],
                             occurrence['sources'][side]['range'][0])
        def split_word(path, document, side, value, change_word=False):
            if document != 'paper' or side != 'after':
                return
            raw = value['stream'].replace('dependence', 'de-\npendence', 1)
            if change_word:
                raw = raw.replace('Closing', 'Clasing')
            stream, offsets, events = compare_words.normalize(
                raw, [[1, index] for index in range(len(raw))])
            value.update(stream=stream, offsets=offsets, events=events,
                         raw_pages=[dict(text=raw)])
        exit_code, result = self.conversion_run(split_word)
        self.assertEqual((exit_code, result['verdict']), (0, 'equal'))
        self.assertNotEqual(result['documents']['paper']['before']['stream'],
                            result['documents']['paper']['after']['stream'])
        self.assertEqual(result['documents']['paper']['before']['canonical_stream'],
                         result['documents']['paper']['after']['canonical_stream'])
        self.assertIn('remove-discretionary',
                      [row['decision'] for row in result['line_events']])
        exit_code, result = self.conversion_run(
            lambda path, document, side, value: split_word(
                path, document, side, value, True))
        self.assertEqual((exit_code, result['verdict']), (1, 'different'))
        changed = [row for row in result['differences'] if row['part'] == 'stream']
        self.assertEqual(len(changed), 1)
        self.assertEqual(changed[0]['proof'], 'direct')
        changed_at = result['documents']['paper']['after']['raw_pages'][0]['text'].index('Clasing') + 2
        self.assertEqual(changed[0]['location']['after']['raw'], [[1, changed_at]])
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
        def located(source, raw, lookup=None):
            stream, offsets, events = compare_words.normalize(
                raw, [[1, index] for index in range(len(raw))])
            pdf = dict(stream=stream, offsets=offsets, events=events,
                       raw_pages=[dict(text=raw)])
            return compare_words.align_located(
                compare_words.decode_located(source, lookup or {}), pdf,
                'paper', 'before')

        raw = 'the de-\npendence and pre-\nand'
        normalized, offsets, events = compare_words.normalize(
            raw, [[1, i] for i in range(len(raw))])
        self.assertEqual(normalized, 'the de-pendence and pre-and')
        match = located('the dependence and pre-and', raw)
        self.assertEqual([row['decision'] for row in match['decisions']],
                         ['remove-discretionary', 'keep-lexical'])
        lexical = located('the de-pendence and pre-and', raw)
        self.assertEqual([row['decision'] for row in lexical['decisions']],
                         ['keep-lexical', 'keep-lexical'])
        page_break = 'de-\npendence'
        locations = [[1, i] for i in range(4)] + [[2, i] for i in range(8)]
        across, across_offsets, across_events = compare_words.normalize(page_break, locations)
        confirmed = compare_words.align_located(
            compare_words.decode_located('dependence', {}),
            dict(stream=across, offsets=across_offsets, events=across_events,
                 raw_pages=[dict(text='de-\n'), dict(text='pendence')]), 'paper', 'before')
        self.assertEqual(confirmed['decisions'][0]['raw'][0], 1)
        suspended = 'the pre-\nand'
        self.assertEqual(located('the pre- and', suspended)['decisions'][0]['decision'],
                         'keep-lexical')
        digits, _, _ = compare_words.normalize('rate 1\n2', [[1, i] for i in range(8)])
        self.assertEqual(digits, 'rate 1 2')
        with self.assertRaises(compare_words.Fault) as raised:
            located('the de-pendence and pre-and', raw + ' ' + raw)
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        with self.assertRaises(compare_words.Fault) as raised:
            located('the dependence', 'breathe dependence')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        with self.assertRaises(compare_words.Fault) as raised:
            located('the dependance and pre-and', raw)
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        def absent_after(path, document, side, value):
            if document == 'paper' and side == 'after':
                value['stream'] = value['stream'].replace('dependence', 'dependance', 1)
        exit_code, record = self.conversion_run(absent_after)
        self.assertEqual((exit_code, record['verdict']), (2, 'refused'))
        self.assertEqual(record['faults'][0]['code'], 'CorrespondenceInvalid')
        search = record['units'][0]['occurrences'][0]['search']
        self.assertEqual(set(search), {'before', 'after'})
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.decode_paragraph('a \\unknown{b}', {})
        self.assertEqual(raised.exception.row['code'], 'SyntaxUnsupported')

        for scalar in ('~', '#', '&', '^', '_', '$', '%', '{', '}', '\\', '`', '"'):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.decode_paragraph('a' + scalar + 'b', {})
            self.assertEqual(raised.exception.row['code'], 'SyntaxUnsupported')
            self.assertEqual(raised.exception.row['stage'], 'snapshot')
        self.assertEqual(compare_words.decode_paragraph('a--b', {})[0], 'a–b')
        for source_text in ("a''b", 'a--–b'):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.decode_paragraph(source_text, {})
            self.assertEqual(raised.exception.row['code'], 'SyntaxUnsupported')
        source = pathlib.Path(__file__).resolve().parents[2]
        lookup = bindings(source)
        original = compare_words.decode_located('The ground is \\ftag{src:nd}.', lookup)
        wrong = compare_words.decode_located('The ground is \\ftag{snk:two}.', lookup)
        self.assertNotEqual(wrong['extracted'], original['extracted'])
        with self.assertRaises(compare_words.Fault) as raised:
            located('The ground is \\ftag{snk:two}.', original['extracted'], lookup)
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        tagged = compare_words.decode_located('A \\ftag{src:nd}word', lookup)
        raw = 'A SRC:ND-\nword'
        stream, offsets, events = compare_words.normalize(
            raw, [[1, index] for index in range(len(raw))])
        with self.assertRaises(compare_words.Fault) as raised:
            compare_words.align_located(
                tagged, dict(stream=stream, offsets=offsets, events=events,
                             raw_pages=[dict(text=raw)]), 'paper', 'after')
        self.assertEqual(raised.exception.row['code'], 'CorrespondenceInvalid')
        def absent_match(path, document, side, value):
            if document == 'paper' and side == 'before':
                value['stream'] = value['stream'].replace('dependence', 'dependance')
                value['offsets'] = [[1, index] for index in range(len(value['stream']))]
        exit_code, result = self.conversion_run(absent_match)
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'CorrespondenceInvalid')
        search = result['units'][0]['occurrences'][0]['search']['before']
        self.assertEqual(search['matches_found'], 0)
        self.assertGreater(search['expanded_states'], 0)
        def uncertain(path, document, side, value):
            if document == 'paper' and side == 'after':
                value['stream'] = value['stream'].replace('Closing', 'C-losing')
                position = value['stream'].rindex('C-losing') + 1
                value['offsets'] = [[1, i] for i in range(len(value['stream']))]
                value['raw_pages'] = [dict(text=value['stream'] + '\n')]
                value['events'] = [dict(operation='LineEndPunctuation',
                                        normalized_offset=position, raw=[1, position],
                                        spelling='-', intervening_space=False,
                                        raw_line='C-\n', newline=[1, len(value['stream'])],
                                        continuation=[1, position + 1])]
        exit_code, result = self.conversion_run(uncertain)
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'LineEndUnresolved')
        self.assertEqual(result['faults'][0]['stage'], 'comparison')
        self.assertEqual(result['coverage']['unresolved'], 1)
        self.assertEqual(result['negative_witness']['paper']['before']['stream'],
                         result['negative_witness']['paper']['after']['stream'])
        def independent_difference(path, document, side, value):
            uncertain(path, document, side, value)
            if document == 'paper' and side == 'after':
                value['stream'] = value['stream'].replace('C-losing', 'C-lozing')
                value['offsets'] = [[1, i] for i in range(len(value['stream']))]
                value['raw_pages'][0]['text'] = value['stream'] + '\n'
        exit_code, result = self.conversion_run(independent_difference)
        self.assertEqual((exit_code, result['verdict']), (1, 'different'))
        self.assertEqual(result['differences'][0]['proof'], 'witness')
        self.assertEqual(result['faults'], [])

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
            source, 'show', PARENT_REVISION + ':generated/paper/paper.pdf') + b'!')
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
        original_verify = compare_words.CapturedInputs.verify
        def changed_verify(captured):
            if captured.repo == source:
                raise ValueError('captured input changed')
            return original_verify(captured)
        with mock.patch.object(compare_words.CapturedInputs, 'verify', changed_verify):
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
            source, PARENT_REVISION + ':documents/paper/parts/front.tex:203-207')
        self.assertIn('One may assume naively', historical['body'])
        self.assertNotIn('One may assume naively',
                         ''.join((source/'documents/paper/parts/front.tex').read_text().splitlines(
                             keepends=True)[202:207]))
        for selector in ('-bad:paragraph.tex:1-2', PARENT_REVISION + ':/escape:1-2',
                         PARENT_REVISION + ':documents/paper/parts/front.tex:203-206'):
            with self.assertRaises(compare_words.Fault) as raised:
                compare_words.source_blob(source, selector)
            self.assertEqual(raised.exception.row['code'], 'SourceInvalid')
        exit_code, result = self.conversion_run(options={'unit': 'sec:absent'})
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'UnitInvalid')
        exit_code, result = self.conversion_run(options={'unit': 'sec:say:direction'})
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'UnitInvalid')
        exit_code, result = self.conversion_run(options={'unit': 'sec:log:andreoni-joint'})
        self.assertEqual((exit_code, result['verdict']), (2, 'refused'))
        self.assertEqual(result['faults'][0]['code'], 'SyntaxUnsupported')
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
        with mock.patch.object(compare_words, 'align_located',
                               side_effect=RuntimeError('bad correspondence')):
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
        shutil.copytree(source/'units',self.root/'units')
        for name in ('tools/paper/identities.py', 'tools/paper/common.py',
                     'tools/paper/bibroute.py', 'documents/paper-build/fontconfig.conf'):
            target=self.root/name; target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(source/name,target)

    def tearDown(self):self.temp.cleanup()

    def replace(self,file,old,new):
        path=self.root/file; text=read(path)
        self.assertIn(old,text); write(path,text.replace(old,new,1))

    def test_assembly_json_evidence_and_refusals(self):
        value=json.loads(assembly_json(self.root))
        self.assertEqual(list(value), ['schema','inputs','bindings','virtues','departures','native_targets','result_headers','bibliography'])
        self.assertEqual(value['schema'], 2)
        self.assertEqual(value['bibliography']['source'], 'documents/paper/references.bib')
        self.assertEqual(value['bibliography']['stock_sha256'], hashlib.sha256(
            (self.root/'documents/paper/references.bib').read_bytes()).hexdigest())
        self.assertEqual([row['word'] for row in value['virtues']],
                         sorted(row['word'] for row in value['virtues']))
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

    def test_virtue_roster_refuses_changed_class_slug_and_binding(self):
        file = 'documents/paper/data/ftoo_manifest.tex'
        original = read(self.root/file)
        rows = virtue_roster(self.root, bindings(self.root), CapturedInputs(self.root))
        self.assertIn(('wonder', 'v:wonder'), [(row['word'], row['key']) for row in rows])
        for old, new, fault in [
            ('{ sem:weld:wonder } { virtue-weld }', '{ sem:weld:wonder } { other }',
             'M063: virtue class or slug differs: v:wonder'),
            ('{ sem:weld:wonder } { wonder }', '{ sem:weld:wonder } { beauty }',
             'M063: virtue class or slug differs: v:wonder'),
            ('{ sem:weld:wonder } { wonder }', '{ sem:weld:wonder } { Wonder }',
             'M063: virtue class or slug differs: v:wonder'),
        ]:
            with self.subTest(new=new):
                self.assertIn(old, original)
                write(self.root/file, original.replace(old, new, 1))
                with self.assertRaisesRegex(ValueError, fault):
                    virtue_roster(self.root, bindings(self.root), CapturedInputs(self.root))
        write(self.root/file, original)

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

    def test_econ_reference_uses_the_retained_definition_binding(self):
        unit = tomllib.loads(read(self.root/'units/statements/coda-flourishing-pole.toml'))['unit']
        self.assertEqual(unit['econ']['reference'], 'op:vmoral')
        rows = read(self.root/'documents/paper-build/label-mints.tsv').splitlines()
        self.assertEqual([row for row in rows if row.startswith('op:vmoral\t')],
                         ['op:vmoral\tdocuments/paper/parts/partC.tex'])
        definition = r'\begin{definition}[\hdr{op:vmoral}]\grade{nec}'
        self.assertEqual(read(self.root/'documents/paper/parts/partC.tex').splitlines().count(
            definition), 1)
        self.assertEqual(bindings(self.root)['op:vmoral']['status'], 'paper')

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

        missing_unit = 'units/statements/missing.toml'
        rows[index] = '\t'.join([key, missing_unit])
        write(path, '\n'.join(rows) + '\n')
        with self.assertRaises(ValueError) as caught: bindings(self.root)
        self.assertEqual(str(caught.exception), 'M063: malformed label mint row ' + str(index + 1))
        write(path, original)

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


class ScreenReferenceChecks(unittest.TestCase):
    def test_regular_heading_counter_and_destination_are_independent(self):
        class PaperPdf:
            page = 12
            def destination(self, anchor):
                return self.page if anchor == 'subsection.22.1' else None

        pdf = PaperPdf()
        target = {'default': '22.1', 'page': '12', 'anchor': 'subsection.22.1'}
        screen = '<a href="../paper-pilot/en.md#the-three-tenses">→ §22.1</a><!-- see subsec:coda:tenses -->'
        with tempfile.TemporaryDirectory() as directory:
            repo = pathlib.Path(directory)
            for owner in ('paper-pilot', 'explainer-pilot'):
                file = repo/'generated/documents'/owner/'en.labels.md'
                file.parent.mkdir(parents=True)
                write(file, screen)
            check_screen_references(repo, {'subsec:coda:tenses': target}, pdf)
            changed = dict(target, default='22.2')
            with self.assertRaisesRegex(ValueError, 'M055: screen/PDF reference number differs'):
                check_screen_references(repo, {'subsec:coda:tenses': changed}, pdf)
            pdf.page = None
            with self.assertRaisesRegex(ValueError, 'M064: missing PDF destination'):
                check_screen_references(repo, {'subsec:coda:tenses': target}, pdf)
            pdf.page = 13
            with self.assertRaisesRegex(ValueError, 'M064: auxiliary/PDF page discrepancy'):
                check_screen_references(repo, {'subsec:coda:tenses': target}, pdf)

    def test_both_reader_documents_bind_to_the_paper_counter(self):
        class PaperPdf:
            def destination(self, anchor):
                return {'paper.12': 12}[anchor]

        target = {'default': 'G', 'page': '12', 'anchor': 'paper.12'}
        paper = '<a href="../paper-pilot/en.md#method-self-limit">→ appendix G</a><!-- see app:self-limit -->'
        explainer = '<a href="../paper-pilot/en.md#method-self-limit">→ appendix G</a><!-- see app:self-limit -->'
        with tempfile.TemporaryDirectory() as directory:
            repo = pathlib.Path(directory)
            for owner, screen in [('paper-pilot', paper), ('explainer-pilot', explainer)]:
                file = repo/'generated/documents'/owner/'en.labels.md'
                file.parent.mkdir(parents=True)
                write(file, screen)
            check_screen_references(repo, {'app:self-limit': target}, PaperPdf())
            for owner in ('paper-pilot', 'explainer-pilot'):
                file = repo/'generated/documents'/owner/'en.labels.md'
                write(file, file.read_text().replace('appendix G', 'appendix H'))
                with self.subTest(owner=owner):
                    with self.assertRaisesRegex(ValueError, 'screen/PDF reference number differs'):
                        check_screen_references(repo, {'app:self-limit': target}, PaperPdf())
                write(file, file.read_text().replace('appendix H', 'appendix G'))
            self.assertEqual(screen_references(paper, {'app:self-limit': target}),
                             [('app:self-limit', target)])


if __name__=='__main__':
    with timed('identity-controls'):unittest.main()
