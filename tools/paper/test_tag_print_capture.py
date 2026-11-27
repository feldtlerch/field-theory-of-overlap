"""Stored association controls for occurrence-level paper tag capture."""
import copy
import json
import pathlib
import unittest

from tag_print_capture import (SP_TO_BP, bind, compare_atoms, compare_fixture, positions,
                               printed_atom, shipped_rows, tap_for_sites, validate_fixture)

FIXTURE = pathlib.Path(__file__).parent / 'fixtures/tag-print-modes.json'
REAL = FIXTURE.parent / 'tag-print-real.json'
SELECTION = FIXTURE.parent / 'tag-print-selection.json'
REPOSITORY = FIXTURE.parents[3]


class FakePdf:
    count = 1

    def __init__(self, rows, destination_page=1):
        self.destination_page = destination_page
        text = ''
        rectangles = []
        for index, row in enumerate(rows):
            top = 152 + index * 35
            for column, glyph in enumerate(row['atom']):
                text += glyph
                rectangles.append((100 + column * 5, top - 7,
                                   105 + column * 5, top + 3))
            text += '\n'
            rectangles.append((0, top, 0, top))
        self.page = dict(text=text, rectangles=rectangles, width=612, height=792)

    def raw_layout_pages(self):
        return [self.page]

    def destination(self, anchor):
        return self.destination_page if anchor in ('definition.4.2', 'theorem.15.2') else None


def evidence(rows):
    position_lines = []
    ledger_lines = []
    for ordinal, row in enumerate(rows, 1):
        top = 152 + (ordinal - 1) * 35
        x0 = round(100 / SP_TO_BP)
        x1 = round((100 + len(row['atom']) * 5) / SP_TO_BP)
        y = round((792 - top) / SP_TO_BP)
        for marker, x in (('S', x0), ('E', x1)):
            position_lines.append('|'.join((marker, str(ordinal), str(int(row['star'])),
                                            row['key'], row['site'], '1', str(x), str(y))))
        ledger_lines.append('|'.join(('main', '1', 'identity', row['key'],
                                      row.get('spelling', 'src:nd'), 'paper', row['site'])))
        if row['mode'] == 'located':
            ledger_lines.append('|'.join(('main', '1', 'locator', row['key'],
                                          '', 'paper', row['site'])))
    return position_lines, ledger_lines


class TagPrintCaptureTests(unittest.TestCase):
    def setUp(self):
        self.rows = json.loads(FIXTURE.read_text(encoding='utf-8'))
        self.lookup = {'src:nd': {'spelling': 'src:nd', 'status': 'paper'}}
        self.targets = {'src:nd': {'page': '1', 'anchor': 'definition.4.2'}}

    def capture(self, rows=None, position_lines=None, ledger_lines=None, pdf=None):
        rows = self.rows if rows is None else rows
        expected_positions, expected_ledger = evidence(rows)
        return bind(rows, positions('\n'.join(expected_positions if position_lines is None
                                              else position_lines)),
                    shipped_rows('\n'.join(expected_ledger if ledger_lines is None
                                           else ledger_lines)),
                    self.lookup, self.targets, FakePdf(rows) if pdf is None else pdf)

    def test_synthetic_reference_modes_and_complete_atoms(self):
        captured = self.capture()
        self.assertEqual([row['mode'] for row in captured],
                         ['located', 'bare', 'bare', 'located'])
        self.assertEqual([row['atom'] for row in captured],
                         [row['atom'] for row in self.rows])
        self.assertTrue(self.rows[3]['synthetic_association'])
        compare_atoms(captured, copy.deepcopy(captured))

    def test_extra_missing_duplicate_and_misassociated_rows_refuse(self):
        positions_lines, ledger_lines = evidence(self.rows)
        for case, changed, reason in (
            ('extra identity', ledger_lines + [ledger_lines[0]], 'identity inventory differs'),
            ('missing identity', [line for line in ledger_lines if line != ledger_lines[0]],
             'identity inventory differs'),
            ('duplicate identity', ledger_lines[:1] + [ledger_lines[0]] + ledger_lines[1:],
             'identity inventory differs'),
            ('wrong locator', ledger_lines[:1] + [ledger_lines[1].replace('src:nd', 'J:occ')] +
             ledger_lines[2:], 'locator association differs'),
        ):
            with self.subTest(case=case), self.assertRaisesRegex(ValueError, reason):
                self.capture(ledger_lines=changed)
        with self.assertRaisesRegex(ValueError, 'position row duplicated'):
            positions('\n'.join(positions_lines + [positions_lines[0]]))
        with self.assertRaisesRegex(ValueError, 'position end missing'):
            positions('\n'.join(positions_lines[:-1]))
        invented = [line.replace('|1|0|', '|5|0|', 1)
                    .replace('parts/partA.tex:60', 'parts/partB.tex:60')
                    for line in positions_lines[:2]]
        with self.assertRaisesRegex(ValueError, 'position site inventory differs'):
            self.capture(position_lines=positions_lines + invented)

    def test_changed_mode_atom_key_star_and_target_refuse(self):
        original = self.capture()
        for field, value in (('mode', 'bare'), ('atom', 'src:nd (theorem 4.2)'),
                             ('key', 'J:occ'), ('star', True), ('command', 'ptg'),
                             ('target_anchor', 'theorem.4.2')):
            changed = copy.deepcopy(original)
            changed[0][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError,
                                                                   'Tag ' + field + ' differs'):
                compare_atoms(original, changed)
        changed = copy.deepcopy(self.rows)
        changed[0]['star'] = True
        pos, ledger = evidence(self.rows)
        with self.assertRaisesRegex(ValueError, 'source/position association differs'):
            self.capture(rows=changed, position_lines=pos, ledger_lines=ledger,
                         pdf=FakePdf(self.rows))
        changed = copy.deepcopy(self.rows)
        changed[0]['command'] = 'ptg'
        with self.assertRaisesRegex(ValueError, 'command family differs'):
            self.capture(rows=changed, position_lines=pos, ledger_lines=ledger,
                         pdf=FakePdf(self.rows))
        changed = copy.deepcopy(self.rows)
        changed[0]['atom'] = 'X' + changed[0]['atom']
        with self.assertRaisesRegex(ValueError, 'printed atom prefix differs'):
            self.capture(rows=changed)
        changed = [self.rows[1], self.rows[0], *self.rows[2:]]
        with self.assertRaisesRegex(ValueError, 'expected occurrence order differs'):
            self.capture(rows=changed, position_lines=pos, ledger_lines=ledger,
                         pdf=FakePdf(self.rows))

    def test_equal_keys_cannot_borrow_a_located_row(self):
        original = self.capture()
        position_lines, ledger_lines = evidence(self.rows)
        locator = ledger_lines.pop(1)
        ledger_lines.insert(2, locator.replace(self.rows[0]['site'], self.rows[1]['site']))
        with self.assertRaisesRegex(ValueError, 'mode/atom differs'):
            self.capture(position_lines=position_lines, ledger_lines=ledger_lines)
        changed = copy.deepcopy(original)
        changed[0]['mode'], changed[1]['mode'] = changed[1]['mode'], changed[0]['mode']
        with self.assertRaisesRegex(ValueError, 'Tag mode differs'):
            compare_atoms(original, changed)

    def test_same_site_repeated_key_locator_reordering_names_occurrence(self):
        self.lookup['pr:agg'] = {'spelling': 'pr:agg', 'status': 'paper'}
        self.targets['pr:agg'] = {'page': '1', 'anchor': 'theorem.15.2'}
        rows = [dict(self.rows[0], site='parts/partD.tex:422', key='pr:agg',
                     spelling='pr:agg', atom='pr:agg (Thm. 15.2)'),
                dict(self.rows[1], site='parts/partD.tex:422', key='pr:agg',
                     spelling='pr:agg', atom='pr:agg')]
        position_lines, ledger_lines = evidence(rows)
        self.assertEqual([row['mode'] for row in self.capture(rows=rows)], ['located', 'bare'])
        moved = [ledger_lines[0], ledger_lines[2], ledger_lines[1]]
        with self.assertRaisesRegex(ValueError,
                                    r'mode/atom differs: parts/partD\.tex:422:pr:agg:occurrence 1'):
            self.capture(rows=rows, position_lines=position_lines, ledger_lines=moved)
        reordered = [ledger_lines[1], ledger_lines[0], ledger_lines[2]]
        with self.assertRaisesRegex(ValueError,
                                    r'identity association differs: .*:occurrence 1'):
            self.capture(rows=rows, position_lines=position_lines, ledger_lines=reordered)

    def test_two_line_atom_and_ambiguous_continuation(self):
        def layout(extra=False):
            text = ''
            rectangles = []
            rows = [('src:nd', 500, 150), ('(Def. 4.2)', 100, 170)]
            if extra:
                rows.append(('(Def. 4.2)', 100, 164))
            for atom, left, top in rows:
                for column, glyph in enumerate(atom):
                    text += glyph
                    rectangles.append((left + column * 5, top - 7,
                                       left + (column + 1) * 5, top + 3))
                text += '\n'
                rectangles.append((0, top, 0, top))
            return dict(text=text, rectangles=rectangles, width=612, height=792)

        start = dict(site='parts/partB.tex:241', key='dyn:response',
                     x=round(500 / SP_TO_BP), y=round((792 - 150) / SP_TO_BP),
                     end_y=round((792 - 170) / SP_TO_BP))
        end = dict(x=round((100 + len('(Def. 4.2)') * 5) / SP_TO_BP))
        self.assertEqual(printed_atom(layout(), start, end, 'src:nd'),
                         'src:nd (Def. 4.2)')
        with self.assertRaisesRegex(ValueError, 'ambiguous across lines'):
            printed_atom(layout(extra=True), start, end, 'src:nd')

    def test_locator_on_a_bare_occurrence_refuses(self):
        position_lines, ledger_lines = evidence(self.rows)
        ledger_lines.insert(3, '|'.join(('main', '1', 'locator', 'src:nd', '',
                                         'paper', self.rows[1]['site'])))
        with self.assertRaisesRegex(ValueError, 'mode/atom differs'):
            self.capture(position_lines=position_lines, ledger_lines=ledger_lines)

    def test_extra_locator_and_destination_refusals(self):
        _, ledger_lines = evidence(self.rows)
        extra = ledger_lines[:2] + [ledger_lines[1]] + ledger_lines[2:]
        with self.assertRaisesRegex(ValueError, 'ledger inventory differs: parts/partA.tex:60'):
            self.capture(ledger_lines=extra)
        with self.assertRaisesRegex(ValueError, 'missing PDF destination'):
            self.capture(pdf=FakePdf(self.rows, destination_page=None))
        with self.assertRaisesRegex(ValueError, 'auxiliary/PDF page discrepancy'):
            self.capture(pdf=FakePdf(self.rows, destination_page=2))

    def test_terminal_resolution_and_ledger_status_refuse(self):
        _, ledger_lines = evidence(self.rows)
        with self.assertRaisesRegex(ValueError, 'identity association differs'):
            self.capture(ledger_lines=[line.replace('|paper|', '|candidate|', 1)
                                       if index == 0 else line
                                       for index, line in enumerate(ledger_lines)])
        self.lookup.clear()
        with self.assertRaisesRegex(ValueError, 'resolution absent'):
            self.capture()

    def test_real_reference_fixture_has_full_atoms_and_source_provenance(self):
        fixture = json.loads(REAL.read_text(encoding='utf-8'))
        selection = json.loads(SELECTION.read_text(encoding='utf-8'))
        validate_fixture(REPOSITORY, selection, fixture)
        self.assertEqual(len(fixture['records']), 36)
        self.assertEqual(sum(row['mode'] == 'located' for row in fixture['records']), 33)
        self.assertEqual(sum(row['mode'] == 'bare' for row in fixture['records']), 3)
        self.assertEqual(fixture['records'][0]['atom'], 'EPI:FACT (Post. 1.1)')
        self.assertEqual([row['atom_lines'] for row in fixture['records']].count(2), 1)
        self.assertEqual([row['key'] for row in fixture['records'] if row['atom_lines'] == 2],
                         ['dyn:response'])
        changed = copy.deepcopy(fixture)
        changed['records'][0]['source_line_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'fixture source line differs at occurrence 0'):
            validate_fixture(REPOSITORY, selection, changed)
        changed = copy.deepcopy(fixture)
        changed['paper_pdf_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'fixture paper PDF differs'):
            validate_fixture(REPOSITORY, selection, changed)
        changed = copy.deepcopy(fixture)
        changed['records'][0]['atom'] = 'EPI:FACT (Post. 1.2)'
        with self.assertRaisesRegex(ValueError, 'fixture atom differs at occurrence 0'):
            compare_fixture(fixture, changed)
        changed = copy.deepcopy(selection)
        changed[0]['key'] = 'not:the:tag'
        with self.assertRaisesRegex(ValueError, 'source command absent'):
            from tag_print_capture import source_line
            source_line(REPOSITORY, changed[0])

    def test_position_tap_selects_only_declared_sites(self):
        tap = tap_for_sites(self.rows)
        self.assertEqual(tap.count(r'\prop_gput:Nnn \g_paper_tag_selected_prop'), 4)
        self.assertNotIn('% SELECTED_SITES', tap)
        self.assertIn('{generated/documents/paper/paragraph.tex:3}', tap_for_sites(
            [dict(site='generated/documents/paper/paragraph.tex:3')]))
        with self.assertRaisesRegex(ValueError, 'site inventory malformed'):
            tap_for_sites([dict(site=r'parts/x.tex:1}\input{other}')])
        with self.assertRaisesRegex(ValueError, 'site inventory malformed'):
            tap_for_sites([dict(site='parts/../reader-hooks.tex:1')])


if __name__ == '__main__':
    unittest.main()
