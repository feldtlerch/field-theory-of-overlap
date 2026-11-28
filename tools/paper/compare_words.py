"""Read PDF and screen words as an informational aid; decode bounded source prose for table evidence.

Differences have no retention or release exit status. Screen extraction is lexical and may
include presentation spelling; independent reading judges the rendered documents.
"""
import argparse
import difflib
from html.parser import HTMLParser
import json
import pathlib
import re
import sys

from meson_adapter import pdf_libraries
from pdf_text import Pdf

TAG = re.compile(r'\\ftag\{([A-Za-z][A-Za-z0-9_:-]*)\}')


class Fault(Exception):
    def __init__(self, code, detail, stage='request', document='-', side='-',
                 page=None, offset=None):
        super().__init__(str(detail))
        self.row = dict(code=code, detail=str(detail), stage=stage,
                        document=document, side=side, page=page, offset=offset)


def logical_source_gap(data, first, last, controls):
    """Apply only the decoder's named source controls within one token gap."""
    output = []
    cursor = first
    for begin, end, replacement in controls:
        if first <= begin and end <= last:
            output.extend((data[cursor:begin], replacement))
            cursor = end
    output.append(data[cursor:last])
    return b''.join(output)


def decode_located(body, lookup, path='<source>', base_offset=0, virtues=None):
    """Decode the admitted prose grammar with source byte ranges and separators."""
    data = body.encode('utf-8')
    if len(data) > 65536:
        raise Fault('CorrespondenceLimit', 'source exceeds 65536 bytes: ' + path, 'snapshot')
    positions = [0]
    for char in body:
        positions.append(positions[-1] + len(char.encode('utf-8')))
    tokens = []
    tags = []
    virtue_tokens = []
    controls = []
    preceding_end = 0
    i = 0

    def invalid(detail, offset):
        raise Fault('SyntaxUnsupported', path + ':' + str(base_offset + positions[offset]) + ': ' + detail,
                    'snapshot')

    while i < len(body):
        start = i
        char = body[i]
        if char in ' \t\n\r':
            if char == '\r' and (i + 1 == len(body) or body[i + 1] != '\n'):
                invalid('isolated carriage return', i)
            i += 1
            continue
        tag = TAG.match(body, i)
        details = None
        virtue = re.match(r'\\papervirtue\{([^{}]*)\}', body[i:]) if char == '\\' else None
        if virtue:
            word = virtue[1]
            companion_source = (path.startswith('documents/paper/explainer/') or
                                path.startswith('generated/documents/explainer-pilot/'))
            if not companion_source or not re.fullmatch(r'[a-z]+', word):
                invalid('unsupported virtue command or document', i)
            row = (virtues or {}).get(word)
            if row is None:
                invalid('unknown virtue roster word: '+word, i)
            detail = dict(command='papervirtue', word=word, key=row['key'], target=row['target'])
            virtue_tokens.append(detail)
            kind = 'virtue'
            semantic = extracted = word
            i += virtue.end()
        elif tag:
            key = tag[1]
            row = lookup.get(key)
            if row is None or row['source_status'] not in ('paper', 'alias'):
                invalid('unchecked bare paper tag: ' + key, i)
            spelling = row['spelling']
            details = dict(key=key, spelling=spelling, extracted=spelling.upper(),
                           target=row['target'])
            tags.append(details)
            kind = 'tag'
            semantic = spelling
            extracted = spelling.upper()
            i = tag.end()
        elif char.isascii() and char.isalpha():
            i += 1
            while i < len(body):
                next_char = body[i]
                if next_char.isascii() and next_char.isalpha():
                    i += 1
                elif (next_char in ("'", '’') and i + 1 < len(body) and
                      body[i + 1].isascii() and body[i + 1].isalpha()):
                    i += 1
                else:
                    break
            kind = 'word'
            semantic = body[start:i].replace("'", '’')
            extracted = semantic
        elif char.isascii() and char.isdigit():
            i += 1
            while i < len(body) and body[i].isascii() and body[i].isdigit():
                i += 1
            kind = 'number'
            semantic = extracted = body[start:i]
        elif char == '-':
            while i < len(body) and body[i] == '-':
                i += 1
            width = i - start
            if width > 3:
                invalid('four or more hyphens are unsupported', start)
            kind = 'mark'
            semantic = extracted = {1: '-', 2: '–', 3: '—'}[width]
        elif char in '.,;:!?()/–—':
            i += 1
            kind = 'mark'
            semantic = extracted = char
        else:
            invalid('unsupported source scalar U+%04X' % ord(char), i)
        if i < len(body) and body[i] in '-–—' and semantic in ('-', '–', '—'):
            invalid('adjacent mixed dash run', i)
        gap = logical_source_gap(data, positions[preceding_end], positions[start],
                                 controls)
        if gap and not gap.isspace():
            invalid('unsupported source between tokens', preceding_end)
        separator_before = bool(gap)
        separator_range = ([base_offset + positions[preceding_end],
                            base_offset + positions[start]] if gap else None)
        tokens.append(dict(kind=kind, semantic=semantic, extracted=extracted,
                           source_range=[base_offset + positions[start], base_offset + positions[i]],
                           source_spelling=body[start:i],
                           separator_before=separator_before,
                           separator_range=separator_range, tag=details))
        if len(tokens) > 16384:
            raise Fault('CorrespondenceLimit', 'source exceeds 16384 tokens: ' + path, 'snapshot')
        preceding_end = i
    tail = logical_source_gap(data, positions[preceding_end], len(data), controls)
    if tail and not tail.isspace():
        invalid('unsupported source after tokens', preceding_end)
    return dict(tokens=tokens, tags=tags, virtues=virtue_tokens,
                source_bytes=data, base_offset=base_offset, source_controls=controls,
                extracted=''.join((' ' if row['separator_before'] and n else '') +
                                  row['extracted'] for n, row in enumerate(tokens)),
                semantic=''.join((' ' if row['separator_before'] and n else '') +
                                 row['semantic'] for n, row in enumerate(tokens)))



class _VisibleHtml(HTMLParser):
    def __init__(self):
        super().__init__()
        self.hidden = 0
        self.parts = []

    def handle_starttag(self, tag, attrs):
        if tag in ('style', 'script'):
            self.hidden += 1

    def handle_endtag(self, tag):
        if tag in ('style', 'script') and self.hidden:
            self.hidden -= 1

    def handle_data(self, data):
        if not self.hidden:
            self.parts.append(data)


def _pdf_text(path):
    return '\n'.join(Pdf(path).raw_pages())


def _screen_text(path):
    source = path.read_text(encoding='utf-8')
    if path.suffix == '.html':
        parser = _VisibleHtml()
        parser.feed(source)
        return ' '.join(parser.parts)
    source = re.sub(r'<!--.*?-->', '', source, flags=re.S)
    source = re.sub(r'\[([^]]+)\]\([^)]+\)', r'\1', source)
    return source


def _tokens(text):
    return re.findall(r"\w+(?:[’']\w+)*|[^\w\s]", text)


def word_differences(before, after):
    """Return differences for reading, without an acceptance verdict."""
    left, right = _tokens(before), _tokens(after)
    return [dict(before=left[i:j], after=right[k:l])
            for kind, i, j, k, l in difflib.SequenceMatcher(
                None, left, right, autojunk=False).get_opcodes() if kind != 'equal']


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='action', required=True)
    dump = commands.add_parser('dump')
    dump.add_argument('--pdf', required=True, type=pathlib.Path)
    report = commands.add_parser('report')
    report.add_argument('--before', required=True, type=pathlib.Path)
    report.add_argument('--after', required=True, type=pathlib.Path)
    report.add_argument('--format', choices=('pdf', 'text'), default='pdf')
    report.add_argument('--screen', action='append', type=pathlib.Path, default=[])
    args = parser.parse_args(argv)
    try:
        if args.action == 'dump':
            pdf_libraries()
            print(json.dumps(Pdf(args.pdf).raw_pages(), ensure_ascii=False, indent=2))
            return 0
        if args.format == 'pdf':
            pdf_libraries()
            before, after = _pdf_text(args.before), _pdf_text(args.after)
        else:
            before = args.before.read_text(encoding='utf-8')
            after = args.after.read_text(encoding='utf-8')
        screens = {str(path): word_differences(after, _screen_text(path))
                   for path in args.screen}
    except (OSError, RuntimeError, UnicodeError, ValueError, AttributeError) as error:
        print('Extraction unavailable: ' + str(error), file=sys.stderr)
        return 2
    print(json.dumps(dict(old_new=word_differences(before, after),
                          new_screens=screens, acceptance='not decided'),
                     ensure_ascii=False, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
