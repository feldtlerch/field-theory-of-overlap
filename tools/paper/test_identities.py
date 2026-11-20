"""Refusal controls for finite bindings and rendered-label leakage."""
import pathlib
import json
import collections
import shutil
import tempfile
import unittest
import re
import tomllib
from common import read, write, timed
from identities import _assignment_line, assembly_json, bindings, holds
from ledger import leaks, occurrence_differences, ledger_binding, check_destination
from prepare import declared_fragments, versioned, paper_master, FRAGMENTS


class Identities(unittest.TestCase):
    def test_declared_fragment_slots_refuse_label_marker_and_authored_copy(self):
        source=pathlib.Path(__file__).resolve().parents[2]
        for path, _, fragment, _ in FRAGMENTS:
            for name in (path, fragment):
                target=self.root/name; target.parent.mkdir(parents=True,exist_ok=True)
                shutil.copyfile(source/name,target)
        rows=read(source/'documents/paper-build/sources.tsv').splitlines()
        declared_fragments(self.root,rows)
        for path, unit, _, _ in FRAGMENTS:
            self.replace(path,'% generated: '+unit+'\n','% ordinary comment\n% generated: '+unit+'\n')
            declared_fragments(self.root,rows)
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
