"""Refusal controls for finite bindings and rendered-label leakage."""
import pathlib
import collections
import shutil
import tempfile
import unittest
import re
import tomllib
from common import read, write, timed
from identities import bindings, holds
from ledger import leaks, occurrence_differences, ledger_binding, check_destination
from prepare import prepare, check_layout_targets, declared_fragments, versioned, paper_master, FRAGMENTS


class Identities(unittest.TestCase):
    def test_committed_fragments_match_native_text(self):
        source=pathlib.Path(__file__).resolve().parents[2]
        texts={}
        for path in sorted((source/'units/statements').glob('*.toml')):
            unit=tomllib.loads(read(path))['unit']
            texts[unit['id']]=unit['texts']['en']
        for *_, fragment, unit in FRAGMENTS:
            self.assertIn('{'+texts[unit]+'}',read(source/fragment))

    def test_declared_fragment_slots_refuse_label_marker_and_authored_copy(self):
        source=pathlib.Path(__file__).resolve().parents[2]
        for name in ['generated/documents/paper-pilot/sec-say-direction.tex',
                     'generated/documents/explainer-pilot/sec-say-direction.tex']:
            target=self.root/name; target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(source/name,target)
        rows=read(source/'documents/paper-build/sources.tsv').splitlines()
        self.assertEqual(len(declared_fragments(self.root,rows)),2)
        fragment='generated/documents/explainer-pilot/sec-say-direction.tex'
        original=read(self.root/fragment)
        write(self.root/fragment,original.replace('sec:say:direction','sec:say:other'))
        with self.assertRaisesRegex(ValueError,'Fragment label/declaration differs'):
            declared_fragments(self.root,rows)
        write(self.root/fragment,original)
        marker='documents/paper/explainer/explainer.tex'
        self.replace(marker,'% generated: sec:say:direction','% generated: sec:say:other')
        with self.assertRaisesRegex(ValueError,'Fragment marker/declaration differs'):
            declared_fragments(self.root,rows)
        self.replace(marker,'% generated: sec:say:other','% generated: sec:say:direction')
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

    def test_fragment_padding_body_mint_and_source_declaration_refuse(self):
        source=pathlib.Path(__file__).resolve().parents[2]
        for name in ['generated/documents/paper-pilot/sec-say-direction.tex',
                     'generated/documents/explainer-pilot/sec-say-direction.tex']:
            target=self.root/name; target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(source/name,target)
        rows=read(source/'documents/paper-build/sources.tsv').splitlines()
        marker='documents/paper/explainer/explainer.tex'
        self.replace(marker,'% generated: sec:say:direction\n%\n',
                     '% generated: sec:say:direction\n% padding\n')
        with self.assertRaisesRegex(ValueError,'Fragment padding differs'):
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

    def layout_fixture(self):
        source=pathlib.Path(__file__).resolve().parents[2]
        for name in ['documents/paper-build/layout-pins.toml', 'documents/paper-build/sources.tsv',
                     'documents/paper-build/reader-hooks.tex']:
            target=self.root/name
            target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(source/name,target)

    def test_shifted_layout_target_refuses_before_preparation(self):
        self.layout_fixture()
        check_layout_targets(self.root)
        target=self.root/'documents/paper/parts/front.tex'
        target.write_bytes(b'\n'+target.read_bytes())
        with self.assertRaises(ValueError) as caught:
            prepare(self.root, self.root/'support', self.root/'work')
        self.assertEqual(str(caught.exception),
            'Layout hook target differs: paragraph-front-138-144 (documents/paper/parts/front.tex)')
        self.assertFalse((self.root/'work').exists())

    def test_layout_pins_cover_exactly_the_addressed_hooks(self):
        self.layout_fixture()
        pins=self.root/'documents/paper-build/layout-pins.toml'
        original=read(pins)
        write(pins,'')
        with self.assertRaises(ValueError) as caught:check_layout_targets(self.root)
        self.assertEqual(str(caught.exception),'Layout pins are missing')
        write(pins,original.replace('[[pin]]','[[pins]]'))
        with self.assertRaises(ValueError) as caught:check_layout_targets(self.root)
        self.assertEqual(str(caught.exception),'Layout pins are missing')
        second=original.index('[[pin]]',original.index('[[pin]]')+1)
        write(pins,original[:second]+original[original.index('[[pin]]',second+1):])
        with self.assertRaises(ValueError) as caught:check_layout_targets(self.root)
        self.assertEqual(str(caught.exception),
            "Layout hook addresses differ from pins: unpinned [('front.tex', 161, 180)], unused []")
        write(pins,original)
        self.replace('documents/paper-build/reader-hooks.tex','\\paper_rebreak:nn{81}{84}','\\paper_rebreak:nn{81}{85}')
        with self.assertRaises(ValueError) as caught:check_layout_targets(self.root)
        self.assertEqual(str(caught.exception),
            "Layout hook addresses differ from pins: unpinned [('partG.tex', 81, 85)], unused [('partG.tex', 81, 84)]")

    def test_unrecognised_hook_address_form_refuses(self):
        self.layout_fixture()
        check_layout_targets(self.root)
        hooks='documents/paper-build/reader-hooks.tex'
        original=read(self.root/hooks)
        for form in ['\\inputlineno = 591', '\\inputlineno<591', '\\inputlineno}={591']:
            with self.subTest(form=form):
                self.replace(hooks,'\\inputlineno=591',form)
                with self.assertRaises(ValueError) as caught:check_layout_targets(self.root)
                self.assertEqual(str(caught.exception),'Layout hook address form is not recognised')
                write(self.root/hooks,original)
        check_layout_targets(self.root)

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
                     'documents/paper-build/provisional-tags.toml','registry/documentary-tags.toml','generated/manifest.json',
                     'documents/paper-build/label-mints.tsv']:
            target=self.root/name; target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(source/name,target)
        shutil.copytree(source/'documents/paper',self.root/'documents/paper')

    def tearDown(self):self.temp.cleanup()

    def replace(self,file,old,new):
        path=self.root/file; text=read(path)
        self.assertIn(old,text); write(path,text.replace(old,new,1))

    def test_release_refuses_explicit_candidates(self):
        rows=bindings(self.root,True)
        self.assertEqual(sum(v['status']=='paper' for v in rows.values()),332)
        self.assertEqual(sum(v['status']=='admitted' for v in rows.values()),87)
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

    def test_table_row_counts_and_grammar_refuse(self):
        for name, count_fault, grammar_fault in [
            ('paper-tags.tsv', 'M063: paper tag row count differs', 'M063: malformed paper tag pair'),
            ('label-mints.tsv', 'M063: label mint row count differs', 'M063: malformed label mint row'),
        ]:
            path=self.root/'documents/paper-build'/name
            original=read(path)
            rows=[line for line in original.splitlines() if not line.startswith('#')]
            malformed=rows.copy(); malformed[0]='!'+malformed[0]
            for changed, expected in [(rows[:-1],count_fault),(malformed,grammar_fault)]:
                with self.subTest(table=name,fault=expected):
                    write(path,'\n'.join(changed)+'\n')
                    with self.assertRaises(ValueError) as caught:bindings(self.root)
                    self.assertEqual(str(caught.exception),expected)
                    write(path,original)

    def test_label_mint_line_must_hold_its_key(self):
        bindings(self.root)
        path=self.root/'documents/paper-build/label-mints.tsv'
        rows=read(path).splitlines()
        index=next(i for i,row in enumerate(rows) if row.startswith('c:q\t'))
        key,file,line=rows[index].split('\t')
        for wrong in [str(int(line)+1), '999999']:
            with self.subTest(line=wrong):
                rows[index]='\t'.join([key,file,wrong]); write(path,'\n'.join(rows)+'\n')
                with self.assertRaises(ValueError) as caught:bindings(self.root)
                self.assertEqual(str(caught.exception),'M063: label mint line does not hold its key: c:q at '+file+':'+wrong)
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
        self.assertEqual(len(decisions),90)
        rows=bindings(self.root,True)
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
