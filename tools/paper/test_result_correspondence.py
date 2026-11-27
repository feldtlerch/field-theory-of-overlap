"""Focused controls for checked result correspondence in a bounded PDF region."""
import unittest
from unittest import mock

from compare_words import Fault, _result_rotation, canonicalize, decode_located, select_result_pdf


HEADER = 'Insight 2.13 (Coherence pools, independence weighs) · F:POOL CONT-ANCH'
BODY = 'Coherence pools the value; independence supplies the weight.'
FIRST = 'Coherence pools the'
SECOND = 'value; independence supplies the weight.'
RECOVERED = ('Insight 2.13 Coherence pools, independence weighs f:pool cont-anch '
             + BODY)
METADATA = dict(kind='insight', key='f:pool', grade='cont-anch', grade_gap=False)


def extracted(extra='', ambiguous=False, trailing=''):
    raw = HEADER + ' ' + SECOND + extra + ' ' + FIRST + trailing
    first_at = raw.index(FIRST, len(HEADER))
    second_at = len(HEADER) + 1
    rectangles = []
    for offset, _ in enumerate(raw):
        if offset < len(HEADER):
            x, y = 10 + offset, 10
        elif offset >= first_at:
            x, y = 20 + len(HEADER) + offset - first_at, 20 if ambiguous else 10
        else:
            x, y = 10 + offset - second_at, 20
        rectangles.append([x, y, x + 1, y + 1])
    return dict(stream=raw, offsets=[[1, offset] for offset in range(len(raw))],
                raw_pages=[dict(text=raw, rectangles=rectangles)], events=[])


class ResultCorrespondence(unittest.TestCase):
    def test_visual_order_retains_the_complete_extractor_stream(self):
        pdf = extracted()
        original = pdf['stream']
        child = decode_located(BODY, {}, path='probe')
        proof = select_result_pdf(pdf, {}, child, METADATA, RECOVERED, 'paper', 'before')
        self.assertEqual(proof['text'], RECOVERED)
        self.assertEqual(pdf['stream'], original)
        self.assertEqual(proof['range'], [0, len(original)])

    def test_unaccounted_character_refuses(self):
        child = decode_located(BODY, {}, path='probe')
        with self.assertRaises(Fault) as caught:
            select_result_pdf(extracted('!'), {}, child, METADATA, RECOVERED,
                              'paper', 'before')
        self.assertEqual(caught.exception.row['code'], 'CorrespondenceInvalid')
        self.assertEqual(caught.exception.row['detail'],
                         'result visual line order or bounded character accounting is ambiguous: 0 candidates')

    def test_unaccounted_trailing_glyph_refuses(self):
        child = decode_located(BODY, {}, path='probe')
        for suffix in ('!', ' !'):
            with self.subTest(suffix=suffix), self.assertRaises(Fault) as caught:
                select_result_pdf(extracted(trailing=suffix), {}, child, METADATA,
                                  RECOVERED, 'paper', 'before')
            self.assertEqual(caught.exception.row['code'], 'CorrespondenceInvalid')
            self.assertEqual(caught.exception.row['detail'],
                             'result bounded region has an unaccounted glyph')

    def test_ambiguous_visual_line_order_refuses(self):
        child = decode_located(BODY, {}, path='probe')
        with self.assertRaises(Fault) as caught:
            select_result_pdf(extracted(ambiguous=True), {}, child, METADATA,
                              RECOVERED, 'paper', 'before')
        self.assertEqual(caught.exception.row['code'], 'CorrespondenceInvalid')
        self.assertEqual(caught.exception.row['detail'],
                         'result visual line order or bounded character accounting is ambiguous: 0 candidates')

    def test_two_rotation_candidates_refuse(self):
        raw = 'H a a a'
        rectangles = [[0, 10, 1, 11], None, [1, 20, 2, 21], None,
                      None, None, [3, 10, 4, 11]]
        pdf = dict(stream=raw, offsets=[[1, index] for index in range(len(raw))],
                   raw_pages=[dict(text=raw, rectangles=rectangles)], events=[])
        with self.assertRaises(Fault) as caught:
            _result_rotation(pdf, 2, 'a a a', 0, 1, 'paper', 'before')
        self.assertEqual(caught.exception.row['code'], 'CorrespondenceInvalid')
        self.assertEqual(caught.exception.row['detail'],
                         'result visual line order or bounded character accounting is ambiguous: 2 candidates')

    def test_in_order_child_keeps_its_source_decision_edits(self):
        raw = HEADER + ' ' + BODY
        pdf = dict(stream=raw, offsets=[[1, index] for index in range(len(raw))],
                   raw_pages=[dict(text=raw, rectangles=[None] * len(raw))], events=[])
        child = decode_located(BODY, {}, path='probe')
        start = len(HEADER) + 1
        edit = (start, start + 1, '', [0, 1])
        proof = dict(range=[len(HEADER) + 1, len(raw)], decisions=[], edits=[edit])
        with mock.patch('compare_words.align_located', return_value=proof):
            selected = select_result_pdf(pdf, {}, child, METADATA, RECOVERED,
                                         'paper', 'before')
        self.assertEqual(selected['aggregate']['edits'], [edit])
        self.assertEqual(canonicalize(pdf, selected['aggregate'])[0], raw[:start] + raw[start + 1:])


if __name__ == '__main__':
    unittest.main()
