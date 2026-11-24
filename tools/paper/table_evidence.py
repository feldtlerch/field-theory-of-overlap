"""Located source-to-PDF evidence for the explainer's nine-kind table.

This profile covers one delivered table. Its coordinate cuts lie in measured
empty gutters; a changed layout needs a new profile and reviewed evidence.
"""

import argparse
from collections import Counter
import json
from pathlib import Path
import re

from common import digest
from compare_words import Fault as DecodeFault, decode_located
from pdf_text import Pdf


SOURCE = Path('documents/paper/explainer/explainer.tex')
PDF = Path('generated/paper/explainer.pdf')
CAPTION = 'Nine kinds of act'
LABEL = r'\label{xtbl:L}'
X_EDGES = (70.0, 150.0, 272.0, 394.0, 520.0)
Y_EDGES = (196.0, 230.0, 265.0, 316.0, 351.0)


class EvidenceFault(ValueError):
    """A named refusal with its table position."""

    def __init__(self, code, detail):
        self.code = code
        self.detail = detail
        super().__init__(f'{code}: {detail}')


def need(condition, code, detail):
    if not condition:
        raise EvidenceFault(code, detail)


def source_lines(data):
    try:
        return data.decode('utf-8').splitlines()
    except UnicodeDecodeError as error:
        raise EvidenceFault('SourceInvalid', 'source is not UTF-8') from error


def source_bounds(lines):
    labels = [number for number, line in enumerate(lines, 1) if line.strip() == LABEL]
    need(len(labels) == 1, 'SourceInvalid', 'table label must occur once')
    label_line = labels[0]
    stack = []
    found = []
    for number, line in enumerate(lines, 1):
        stripped = line.strip()
        if re.fullmatch(r'\\begin\{table\}(?:\[[^]]+\])?', stripped):
            stack.append(number)
        elif stripped == r'\end{table}':
            need(stack, 'SourceInvalid', 'table end has no beginning')
            first = stack.pop()
            if first < label_line < number:
                found.append((first, number))
    need(not stack and len(found) == 1, 'SourceInvalid',
         'label has no unique enclosing table')
    return found[0]


def source_table(data):
    lines = source_lines(data)
    first, last = source_bounds(lines)
    region = lines[first - 1:last]
    need(rf'\caption{{{CAPTION}}}' in [line.strip() for line in region],
         'SourceInvalid', 'caption or table identity changed')
    body = '\n'.join(region)
    need(body.count(r'\toprule') == 1 and body.count(r'\bottomrule') == 1,
         'SourceInvalid', 'table rules changed')
    body = body.split(r'\toprule', 1)[1].split(r'\bottomrule', 1)[0]
    body = re.sub(r'(?m)^\s*\\(?:midrule|addlinespace)\s*$', '', body)
    pieces = body.split(r'\\')
    need(len(pieces) == 5 and not pieces[-1].strip(),
         'SourceInvalid', 'expected four complete source rows')
    rows = []
    for row_number, piece in enumerate(pieces):
        if row_number == 4:
            break
        cells = piece.split('&')
        need(len(cells) == 4, 'SourceInvalid', f'row {row_number} needs four cells')
        decoded = []
        for column, cell in enumerate(cells):
            cell = re.sub(r'\\textbf\{([^{}]+)\}', r'\1', cell)
            need('\\' not in cell and '{' not in cell and '}' not in cell,
                 'SourceInvalid', f'unsupported source form at {row_number},{column}')
            text = ' '.join(cell.split())
            try:
                decoded.append(decode_located(text, {})['semantic'])
            except DecodeFault as error:
                raise EvidenceFault('SourceInvalid',
                                    f'unsupported source form at {row_number},{column}: '
                                    f'{error.row["detail"]}') from error
        rows.append(decoded)
    need(rows[0][0] == '' and all(rows[r][c] for r in range(4)
                                    for c in range(4) if (r, c) != (0, 0)),
         'SourceInvalid', 'expected one empty corner and fifteen filled cells')
    return rows


def source_row_lines(data):
    lines = source_lines(data)
    first_table, last_table = source_bounds(lines)
    ranges = []
    first = None
    inside = False
    for number in range(first_table, last_table + 1):
        line = lines[number - 1].strip()
        if line == r'\toprule':
            inside = True
        elif line == r'\bottomrule':
            break
        elif inside and line and line not in (r'\midrule', r'\addlinespace'):
            if first is None:
                first = number
            if line.endswith(r'\\'):
                ranges.append([first, number])
                first = None
    need(first is None and len(ranges) == 4, 'SourceInvalid',
         'source row line bounds are ambiguous')
    return ranges


def band(value, edges):
    for index, (low, high) in enumerate(zip(edges, edges[1:])):
        if low < value < high:
            return index
    return None


def box(rectangles):
    return [round(min(rect[0] for rect in rectangles), 3),
            round(min(rect[1] for rect in rectangles), 3),
            round(max(rect[2] for rect in rectangles), 3),
            round(max(rect[3] for rect in rectangles), 3)]


def located_cells(pages):
    matches = [(index, page) for index, page in enumerate(pages)
               if CAPTION in page['text']]
    need(len(matches) == 1, 'CaptionAmbiguous', 'caption must occur on one page once')
    page_index, page = matches[0]
    text, rectangles = page['text'], page['rectangles']
    need(text.count(CAPTION) == 1 and len(text) == len(rectangles),
         'GeometryAmbiguous', 'caption or character layout is ambiguous')
    caption_start = text.index(CAPTION)
    line_start = text.rfind('\n', 0, caption_start) + 1
    line_end = text.find('\n', caption_start)
    need(line_end >= 0, 'CaptionAmbiguous', 'caption line is incomplete')
    caption_line = text[line_start:line_end]
    need(re.fullmatch(r'Table [0-9]+\.[0-9]+: ' + re.escape(CAPTION), caption_line),
         'CaptionAmbiguous', 'caption spelling changed')
    caption_rects = [rectangles[i] for i in range(line_start, line_end)
                     if not text[i].isspace()]
    need(caption_rects and box(caption_rects)[3] < Y_EDGES[0],
         'GeometryAmbiguous', 'caption touches the header band')
    caption_bottom = box(caption_rects)[3]
    header_tops = [rect[1] for char, rect in zip(text, rectangles)
                   if not char.isspace() and rect[1] > caption_bottom and
                   X_EDGES[0] < (rect[0] + rect[2]) / 2 < X_EDGES[-1] and
                   rect[1] < Y_EDGES[1]]
    need(header_tops and min(Y_EDGES[0] - caption_bottom,
                             min(header_tops) - Y_EDGES[0]) > 2.0,
         'GeometryAmbiguous', 'caption/header cut lacks clearance')
    slots = [[[] for _ in range(4)] for _ in range(4)]
    for index, char in enumerate(text):
        if char in '\r\n':
            continue
        rect = rectangles[index]
        need(len(rect) == 4 and all(isinstance(value, (float, int)) for value in rect),
             'GeometryAmbiguous', f'bad character rectangle {index}')
        centre_x = (rect[0] + rect[2]) / 2
        centre_y = (rect[1] + rect[3]) / 2
        row = band(centre_y, Y_EDGES)
        if row is None:
            continue
        column = band(centre_x, X_EDGES)
        need(column is not None and X_EDGES[column] < rect[0] <= rect[2] <
             X_EDGES[column + 1] and Y_EDGES[row] < rect[1] <= rect[3] <
             Y_EDGES[row + 1], 'GeometryAmbiguous', f'character {index} crosses a cell cut')
        slots[row][column].append((index, char, rect))
    found = []
    for row in range(4):
        found_row = []
        for column in range(4):
            glyphs = slots[row][column]
            by_line = {}
            for item in glyphs:
                by_line.setdefault(round(item[2][1], 1), []).append(item)
            ordered_lines = [sorted(by_line[key], key=lambda item: (item[2][0], item[0]))
                             for key in sorted(by_line)]
            lines = [''.join(item[1] for item in line) for line in ordered_lines]
            used = [item[2] for item in glyphs if not item[1].isspace()]
            found_row.append(dict(row=row, column=column, lines=lines,
                                  rectangle=box(used) if used else None,
                                  characters=[dict(index=item[0], char=item[1],
                                                   rectangle=[round(value, 3)
                                                              for value in item[2]])
                                              for line in ordered_lines for item in line]))
        found.append(found_row)
    return page_index + 1, dict(text=caption_line, rectangle=box(caption_rects)), found


def readings(lines):
    """Enumerate bounded line-end interpretations; source equality selects one."""
    choices = {''}
    for line_number, line in enumerate(lines):
        line = line.strip()
        if line_number == 0:
            choices = {line}
            continue
        previous = lines[line_number - 1].strip()
        updated = set()
        for text in choices:
            updated.add(text + ' ' + line)
            if previous.endswith('-') and line and line[0].isalpha():
                updated.add(text[:-1] + line)
                updated.add(text + line)
        choices = updated
        need(len(choices) <= 16, 'GeometryAmbiguous', 'too many line-end readings')
    return {' '.join(choice.split()) for choice in choices}


def compare(source_rows, cells):
    source_texts = {cell for row in source_rows for cell in row}
    observed = []
    for row in range(4):
        observed_row = []
        for column in range(4):
            lines = cells[row][column]['lines']
            expected = source_rows[row][column]
            if (row, column) == (0, 0):
                need(not lines, 'CellTextChanged', 'empty corner gained text')
                observed_row.append('')
                continue
            need(lines, 'CellMissing', f'cell {row},{column} has no text')
            alternatives = readings(lines)
            matches = alternatives & source_texts
            if expected in matches:
                observed_row.append(expected)
            elif len(matches) == 1:
                observed_row.append(next(iter(matches)))
            else:
                observed_row.append(' '.join(lines).strip())
        observed.append(observed_row)
    expected_flat = [cell for row in source_rows for cell in row]
    observed_flat = [cell for row in observed for cell in row]
    if expected_flat != observed_flat:
        if Counter(expected_flat) == Counter(observed_flat):
            code = 'CellPositionChanged'
        else:
            code = 'CellTextChanged'
        first = next((r, c) for r in range(4) for c in range(4)
                     if source_rows[r][c] != observed[r][c])
        raise EvidenceFault(code, f'cell {first[0]},{first[1]}: '
                            f'{observed[first[0]][first[1]]!r} != '
                            f'{source_rows[first[0]][first[1]]!r}')
    return observed


def capture(source_bytes, pdf_bytes, pages):
    source_rows = source_table(source_bytes)
    first, last = source_bounds(source_lines(source_bytes))
    page, caption, cells = located_cells(pages)
    compare(source_rows, cells)
    recorded = [{key: value for key, value in cell.items() if key != 'characters'}
                for row in cells for cell in row]
    return dict(source_sha256=digest(source_bytes), pdf_sha256=digest(pdf_bytes),
                source_path=SOURCE.as_posix(), source_lines=[first, last],
                source_row_lines=source_row_lines(source_bytes),
                pdf_path=PDF.as_posix(), page=page, caption=caption,
                x_edges=list(X_EDGES), y_edges=list(Y_EDGES), source_cells=source_rows,
                cells=[recorded[i:i + 4] for i in range(0, 16, 4)])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repository', type=Path, required=True)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    source = (args.repository / SOURCE).read_bytes()
    pdf = (args.repository / PDF).read_bytes()
    evidence = capture(source, pdf, Pdf(args.repository / PDF).raw_layout_pages())
    encoded = json.dumps(evidence, ensure_ascii=False, indent=2) + '\n'
    if args.output:
        args.output.write_text(encoded, encoding='utf-8')
    else:
        print(encoded, end='')


if __name__ == '__main__':
    main()
