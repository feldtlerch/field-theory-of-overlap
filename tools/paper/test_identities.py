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


class Identities(unittest.TestCase):
    def test_word_difference_report_has_no_acceptance_exit_status(self):
        before = self.root / 'before.txt'
        after = self.root / 'after.txt'
        before.write_text('one old word', encoding='utf-8')
        after.write_text('one new word', encoding='utf-8')
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = compare_words.main(
                ['report', '--format', 'text', '--before', str(before), '--after', str(after)])
        report = json.loads(output.getvalue())
        self.assertEqual(status, 0)
        self.assertTrue(report['old_new'])
        self.assertEqual(report['acceptance'], 'not decided')

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
