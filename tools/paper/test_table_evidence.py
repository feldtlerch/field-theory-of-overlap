"""Direct controls for the located nine-kind table evidence."""

from collections import Counter
import copy
from pathlib import Path
import unittest

from pdf_text import Pdf
import table_evidence as table


ROOT = Path(__file__).resolve().parents[2]
class TableEvidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = (ROOT / table.SOURCE).read_bytes()
        cls.pdf = (ROOT / table.PDF).read_bytes()
        cls.pages = Pdf(ROOT / table.PDF).raw_layout_pages()
        cls.source_rows = table.source_table(cls.source)
        cls.page_number, cls.caption, cls.cells = table.located_cells(cls.pages)

    def test_delivered_table_has_all_located_cells_and_caption(self):
        actual = table.capture(self.source, self.pdf, self.pages)
        lines = table.source_lines(self.source)
        first, last = table.source_bounds(lines)
        self.assertEqual(actual['source_lines'], [first, last])
        self.assertTrue(lines[first - 1].strip().startswith(r'\begin{table}'))
        self.assertEqual(lines[last - 1].strip(), r'\end{table}')
        self.assertEqual(sum(line.strip() == table.LABEL for line in lines), 1)
        self.assertEqual(table.compare(self.source_rows, self.cells), self.source_rows)
        self.assertEqual(actual['source_cells'][0][0], '')
        self.assertEqual(sum(bool(cell) for row in actual['source_cells'] for cell in row), 15)
        self.assertEqual(len(self.cells), 4)
        self.assertTrue(all(len(row) == 4 for row in self.cells))
        self.assertEqual(actual['caption']['text'], 'Table 0.1: Nine kinds of act')
        self.assertEqual(actual['page'], self.page_number)

    def test_source_location_follows_its_label(self):
        shifted_source = b'% preceding source edit\n' + self.source
        original = table.capture(self.source, self.pdf, self.pages)
        shifted = table.capture(shifted_source, self.pdf, self.pages)
        self.assertEqual(shifted['source_lines'],
                         [number + 1 for number in original['source_lines']])
        self.assertEqual(shifted['source_cells'], original['source_cells'])
        self.assertNotEqual(shifted['source_sha256'], original['source_sha256'])

    def test_duplicate_table_label_refuses_single_occurrence(self):
        label = table.LABEL.encode('utf-8')
        self.assertEqual(self.source.count(label), 1)
        changed = self.source.replace(label, label + b'\n' + label, 1)
        with self.assertRaises(table.EvidenceFault) as raised:
            table.source_table(changed)
        self.assertEqual((raised.exception.code, raised.exception.detail),
                         ('SourceInvalid', 'table label must occur once'))

    def test_swapped_cells_keep_all_text_but_refuse_position(self):
        changed = copy.deepcopy(self.cells)
        left, right = changed[1][1], changed[1][2]
        left['lines'], right['lines'] = right['lines'], left['lines']
        before = Counter(tuple(cell['lines']) for row in self.cells for cell in row)
        after = Counter(tuple(cell['lines']) for row in changed for cell in row)
        self.assertEqual(before, after)
        with self.assertRaises(table.EvidenceFault) as raised:
            table.compare(self.source_rows, changed)
        self.assertEqual((raised.exception.code, raised.exception.detail.split(':', 1)[0]),
                         ('CellPositionChanged', 'cell 1,1'))

    def test_missing_body_cell_refuses_its_position(self):
        changed = copy.deepcopy(self.cells)
        changed[2][3]['lines'] = []
        with self.assertRaises(table.EvidenceFault) as raised:
            table.compare(self.source_rows, changed)
        self.assertEqual((raised.exception.code, raised.exception.detail),
                         ('CellMissing', 'cell 2,3 has no text'))

    def test_changed_word_refuses_its_position(self):
        changed = copy.deepcopy(self.cells)
        changed[1][1]['lines'][1] = 'that comes away'
        with self.assertRaises(table.EvidenceFault) as raised:
            table.compare(self.source_rows, changed)
        self.assertEqual(raised.exception.code, 'CellTextChanged')
        self.assertTrue(raised.exception.detail.startswith('cell 1,1:'))

    def test_wrap_inside_one_header_cell_stays_equal(self):
        pages = copy.deepcopy(self.pages)
        target = self.cells[0][3]['characters']
        spelling = ''.join(item['char'] for item in target)
        self.assertEqual(spelling, 'the other is harmed')
        moved = target[spelling.index('harmed'):]
        original_x = moved[0]['rectangle'][0]
        target_x = target[0]['rectangle'][0]
        for item in moved:
            index = item['index']
            rect = pages[self.page_number - 1]['rectangles'][index]
            pages[self.page_number - 1]['rectangles'][index] = (
                rect[0] - original_x + target_x, rect[1] + 14.9,
                rect[2] - original_x + target_x, rect[3] + 14.9)
        _, _, cells = table.located_cells(pages)
        self.assertEqual(cells[0][3]['lines'], ['the other is ', 'harmed'])
        self.assertEqual(table.compare(self.source_rows, cells), self.source_rows)

    def test_glyph_crossing_column_cut_refuses_geometry(self):
        pages = copy.deepcopy(self.pages)
        index = self.cells[1][1]['characters'][0]['index']
        rect = pages[self.page_number - 1]['rectangles'][index]
        cut = table.X_EDGES[2]
        pages[self.page_number - 1]['rectangles'][index] = (
            cut - 1.0, rect[1], cut + 0.5, rect[3])
        with self.assertRaises(table.EvidenceFault) as raised:
            table.located_cells(pages)
        self.assertEqual(raised.exception.code, 'GeometryAmbiguous')
        self.assertEqual(raised.exception.detail, f'character {index} crosses a cell cut')

    def test_changed_caption_refuses_geometry_route(self):
        pages = copy.deepcopy(self.pages)
        page = pages[self.page_number - 1]
        page['text'] = page['text'].replace(table.CAPTION, 'Xine kinds of act', 1)
        with self.assertRaises(table.EvidenceFault) as raised:
            table.located_cells(pages)
        self.assertEqual(raised.exception.code, 'CaptionAmbiguous')
        self.assertEqual(raised.exception.detail, 'caption must occur on one page once')


if __name__ == '__main__':
    unittest.main()
