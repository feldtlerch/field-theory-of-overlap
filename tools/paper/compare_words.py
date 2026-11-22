"""Compare complete captured paper text and corroborate one native paragraph."""
import argparse
import ast
import ctypes
import ctypes.util
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import unicodedata

from identities import (CHECK_INPUTS, INPUTS as BINDING_INPUTS,
                        CapturedInputs, bindings, input_paths)
from inputs import inventory
from meson_adapter import pdf_libraries
from pdf_text import Pdf
from prepare import declared_fragments, scope_fragments

INPUTS = ['tools/paper/compare_words.py', 'tools/paper/pdf_text.py',
          'tools/paper/common.py', 'tools/paper/identities.py',
          'tools/paper/prepare.py', 'tools/paper/meson_adapter.py',
          'tools/paper/inputs.py']
LIGATURES = {'\ufb00': 'ff', '\ufb01': 'fi', '\ufb02': 'fl',
             '\ufb03': 'ffi', '\ufb04': 'ffl', '\ufb06': 'st'}
SOURCE = re.compile(r'([^:]+):([^:]+):([1-9][0-9]*)-([1-9][0-9]*)\Z')
TAG = re.compile(r'\\ftag\{([A-Za-z][A-Za-z0-9_:-]*)\}')
STAGES = {'request': 0, 'snapshot': 1, 'extraction': 2, 'profile': 3,
          'correspondence': 4, 'comparison': 5, 'final': 6}


class Fault(Exception):
    def __init__(self, code, detail, stage='request', document='-', side='-',
                 page=None, offset=None):
        super().__init__(str(detail))
        self.row = dict(code=code, detail=str(detail), stage=stage,
                        document=document, side=side, page=page, offset=offset)


class StageHalt(Exception):
    pass


def digest(data):
    return hashlib.sha256(data).hexdigest()


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode()


def git(repo, *args):
    result = subprocess.run(['git', '-C', str(repo), *args], capture_output=True)
    if result.returncode:
        raise Fault('SourceInvalid', result.stderr.decode(errors='replace').strip(), 'snapshot')
    return result.stdout


def source_blob(repo, selector):
    match = SOURCE.fullmatch(selector)
    if not match:
        raise Fault('SourceInvalid', 'invalid source selector', 'snapshot')
    revision, name, first, last = match.groups()
    path = pathlib.PurePosixPath(name)
    if revision.startswith('-') or name.startswith('-') or path.is_absolute() or '..' in path.parts:
        raise Fault('SourceInvalid', 'source selector escapes the repository', 'snapshot')
    first, last = int(first), int(last)
    if first > last:
        raise Fault('SourceInvalid', 'source line range is reversed', 'snapshot')
    commit = git(repo, 'rev-parse', '--verify', revision + '^{commit}').decode().strip()
    blob = git(repo, 'rev-parse', '--verify', commit + ':' + name).decode().strip()
    data = git(repo, 'show', commit + ':' + name)
    try:
        text = data.decode('utf-8')
    except UnicodeError as error:
        raise Fault('SourceInvalid', 'source is not UTF-8', 'snapshot') from error
    if '\r' in text.replace('\r\n', ''):
        raise Fault('SourceInvalid', 'source has an unsupported line ending', 'snapshot')
    parts = text.split('\n')
    lines = [part + '\n' for part in parts[:-1]] + ([parts[-1]] if parts[-1] else [])
    if last > len(lines) or not ''.join(lines[first - 1:last]).strip():
        raise Fault('SourceInvalid', 'source paragraph is absent', 'snapshot')
    if first > 1 and lines[first - 2].strip():
        raise Fault('SourceInvalid', 'source paragraph lacks a preceding boundary', 'snapshot')
    if last < len(lines) and lines[last].strip():
        raise Fault('SourceInvalid', 'source paragraph lacks a following boundary', 'snapshot')
    body = ''.join(lines[first - 1:last])
    if any(not line.strip() for line in lines[first - 1:last]):
        raise Fault('SourceInvalid', 'source selector spans paragraphs', 'snapshot')
    start = len(''.join(lines[:first - 1]).encode('utf-8'))
    return dict(commit=commit, blob=blob, path=name, first=first, last=last,
                bytes=[start, start + len(body.encode('utf-8'))],
                body=body, blob_bytes=data)


def forbidden(ch):
    category = unicodedata.category(ch)
    return ((category == 'Cc' and not ch.isspace()) or
            category in ('Cf', 'Co', 'Cs', 'Cn') or ch in ('\ufffd', '\u00ad', '\ufb05'))


def check_characters(text, document, side, page):
    for offset, ch in enumerate(text):
        if forbidden(ch):
            raise Fault('ExtractionAmbiguous', 'prohibited scalar U+%04X' % ord(ch),
                        'profile', document, side, page, offset)


def decode_paragraph(body, lookup):
    decoded = decode_located(body, lookup)
    return decoded['extracted'], decoded['semantic'], decoded['tags']


def decode_located(body, lookup, path='<source>', base_offset=0):
    """Decode the admitted prose grammar with source byte ranges and separators."""
    data = body.encode('utf-8')
    if len(data) > 65536:
        raise Fault('CorrespondenceLimit', 'source exceeds 65536 bytes: ' + path, 'snapshot')
    positions = [0]
    for char in body:
        positions.append(positions[-1] + len(char.encode('utf-8')))
    tokens = []
    tags = []
    separator = None
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
            if separator is None:
                separator = i
            i += 1
            continue
        tag = TAG.match(body, i)
        details = None
        if tag:
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
        separator_range = (None if separator is None else
                           [base_offset + positions[separator], base_offset + positions[start]])
        tokens.append(dict(kind=kind, semantic=semantic, extracted=extracted,
                           source_range=[base_offset + positions[start], base_offset + positions[i]],
                           source_spelling=body[start:i],
                           separator_before=separator is not None,
                           separator_range=separator_range, tag=details))
        if len(tokens) > 16384:
            raise Fault('CorrespondenceLimit', 'source exceeds 16384 tokens: ' + path, 'snapshot')
        separator = None
    return dict(tokens=tokens, tags=tags, source_bytes=data, base_offset=base_offset,
                extracted=''.join((' ' if row['separator_before'] and n else '') +
                                  row['extracted'] for n, row in enumerate(tokens)),
                semantic=''.join((' ' if row['separator_before'] and n else '') +
                                 row['semantic'] for n, row in enumerate(tokens)))


def normalize(text, locations, join_break=True):
    output = []
    offsets = []
    events = []
    for position, ch in enumerate(text):
        if ch.isspace():
            if output and output[-1] != ' ':
                confirmed_shape = (join_break and ch == '\n' and position >= 2 and
                                   text[position - 1] == '-' and
                                   text[position - 2].isascii() and text[position - 2].isalpha() and
                                   position + 1 < len(text) and text[position + 1].isascii() and
                                   text[position + 1].isalpha())
                if confirmed_shape:
                    events.append(dict(operation='HyphenUnconfirmed',
                                       raw=locations[position - 1],
                                       normalized_offset=len(output) - 1))
                else:
                    output.append(' '); offsets.append(locations[position])
            if ch == '\n':
                previous = position - 1
                while previous >= 0 and text[previous] in ' \t':
                    previous -= 1
                if previous >= 0 and text[previous] in '-–—/':
                    offset = len(output) - (2 if output and output[-1] == ' ' else 1)
                    line_start = text.rfind('\n', 0, previous) + 1
                    events.append(dict(operation='LineEndPunctuation',
                                       raw=locations[previous], normalized_offset=offset,
                                       spelling=text[previous],
                                       intervening_space=previous != position - 1,
                                       raw_line=text[line_start:position + 1],
                                       newline=locations[position],
                                       continuation=(locations[position + 1]
                                                     if position + 1 < len(locations) else None)))
            continue
        mapped = LIGATURES.get(ch, ch)
        if mapped != ch:
            events.append(dict(operation='ligature', raw=locations[position],
                               before=ch, after=mapped))
        for part in mapped:
            output.append(part); offsets.append(locations[position])
    if output and output[-1] == ' ':
        output.pop(); offsets.pop()
    return ''.join(output), offsets, events


def folio(page, number, document, side):
    width, height = page['width'], page['height']
    if abs(width - 210 * 72 / 25.4) > 1 or abs(height - 297 * 72 / 25.4) > 1:
        raise Fault('ProfileUnsupported', 'page is not A4', 'profile', document, side, number)
    if page['label'] != str(number):
        raise Fault('ProfileUnsupported', 'page label differs from physical page',
                    'profile', document, side, number)
    text, rectangles = page['text'], page['rectangles']
    found = []
    begin = 0
    for line in text.splitlines(keepends=True):
        end = begin + len(line)
        ink = [rectangles[offset] for offset in range(begin, end)
               if not text[offset].isspace()]
        if ink and line.strip().isdecimal() and min(rect[1] for rect in ink) > height - 72:
            center = (min(rect[0] for rect in ink) + max(rect[2] for rect in ink)) / 2
            if abs(center - width / 2) <= 18:
                union = [min(rect[0] for rect in ink), min(rect[1] for rect in ink),
                         max(rect[2] for rect in ink), max(rect[3] for rect in ink)]
                found.append((begin, end, line.strip(), union))
        begin = end
    if len(found) != 1 or found[0][2] != str(number):
        raise Fault('FurnitureInvalid', 'plain folio is missing, repeated or wrong',
                    'profile', document, side, number)
    return found[0]


def extract(path, document, side):
    try:
        pdf = Pdf(path)
        pages = pdf.raw_layout_pages()
        bookmarks = pdf.bookmarks()
    except Exception as error:
        raise Fault('ExtractionUnavailable', error, 'extraction', document, side) from error
    raw = []
    retained = []
    locations = []
    folios = []
    for number, page in enumerate(pages, 1):
        check_characters(page['text'], document, side, number)
        start, end, label, rectangle = folio(page, number, document, side)
        folios.append(dict(page=number, label=label, range=[start, end], rectangle=rectangle))
        raw.append(page)
        for offset, ch in enumerate(page['text']):
            if not start <= offset < end:
                retained.append(ch); locations.append([number, offset])
        if not retained or retained[-1] != '\n':
            retained.append('\n'); locations.append([number, len(page['text'])])
    stream, offsets, events = normalize(''.join(retained), locations)
    titles = []
    for number, row in enumerate(bookmarks, 1):
        title = row['title']
        check_characters(title, document, side, 'outline:' + str(number))
        normalized, _, changes = normalize(title, [[number, i] for i in range(len(title))], False)
        titles.append(normalized)
        events.extend(dict(event, outline=number) for event in changes)
    return dict(raw_pages=raw, raw_bookmarks=bookmarks, stream=stream, offsets=offsets,
                events=events, folios=folios, titles=titles)


def line_events(extracted, document, side):
    """Retain one event per physical line-end mark, including page boundaries."""
    result = []
    for event in extracted['events']:
        if event['operation'] != 'LineEndPunctuation' or 'outline' in event:
            continue
        page, offset = event['newline']
        raw_page = extracted['raw_pages'][page - 1]['text']
        kind = ('page' if offset == len(raw_page) or
                (event['continuation'] and event['continuation'][0] != page) else 'line')
        result.append(dict(document=document, side=side, mark=event['spelling'],
                           raw=event['raw'], normalized_offset=event['normalized_offset'],
                           raw_line=event['raw_line'], newline_kind=kind,
                           continuation=event['continuation'],
                           intervening_space=event['intervening_space'],
                           decision='unresolved'))
    return result


def unique_proof(proofs, document, side):
    """Distinct locations or canonical spellings cannot be chosen by search order."""
    distinct = {}
    for proof in proofs:
        key = (tuple(proof['range']), proof['canonical'],
               tuple(tuple(span) for span in proof.get('source_ranges', [])))
        distinct[key] = proof
    if len(distinct) > 1:
        if len({tuple(row['range']) for row in distinct.values()}) > 1:
            raise Fault('CorrespondenceInvalid', 'decoded paragraph matches more than once',
                        'correspondence', document, side)
        raise Fault('LineEndAmbiguous', 'paragraph has distinct source proofs',
                    'correspondence', document, side)
    if not distinct:
        raise Fault('CorrespondenceInvalid', 'decoded paragraph has no match',
                    'correspondence', document, side)
    return next(iter(distinct.values()))


def align_located(decoded, extracted, document, side):
    """Match one source paragraph while interpreting only its own line ends."""
    source_bytes = decoded['source_bytes']
    base_offset = decoded['base_offset']
    preceding_end = 0
    for token in decoded['tokens']:
        first, last = (value - base_offset for value in token['source_range'])
        if not preceding_end <= first < last <= len(source_bytes):
            raise Fault('CorrespondenceInvalid', 'source token range is invalid',
                        'correspondence', document, side)
        separator = source_bytes[preceding_end:first]
        if (bool(separator) != token['separator_before'] or
                (separator and (token['separator_range'] !=
                                [base_offset + preceding_end, base_offset + first] or
                                not separator.isspace()))):
            raise Fault('CorrespondenceInvalid', 'source separator range differs',
                        'correspondence', document, side)
        try:
            spelling = source_bytes[first:last].decode('utf-8')
        except UnicodeError as error:
            raise Fault('CorrespondenceInvalid', 'source token range splits UTF-8',
                        'correspondence', document, side) from error
        if spelling != token['source_spelling']:
            raise Fault('CorrespondenceInvalid', 'source token range differs',
                        'correspondence', document, side)
        preceding_end = last
    if preceding_end != len(source_bytes) and not source_bytes[preceding_end:].isspace():
        raise Fault('CorrespondenceInvalid', 'source token coverage differs',
                    'correspondence', document, side)
    expected = decoded['extracted']
    stream = extracted['stream']
    events = {row['normalized_offset']: row for row in line_events(extracted, document, side)}
    source_chars = []
    for ordinal, token in enumerate(decoded['tokens']):
        if token['separator_before'] and source_chars:
            source_chars.append((ordinal, token['separator_range'], True, 0))
        source_chars.extend((ordinal, token['source_range'], False, offset)
                            for offset, _ in enumerate(token['extracted']))
    starts = [match.start() for match in re.finditer(re.escape(expected[:1]), stream)]
    expanded = 0
    proofs = []
    def search_record():
        return dict(starts=len(starts), expanded_states=expanded,
                    peak_frontier=1, matches_found=len(proofs), limit=262144,
                    remaining_state_budget=max(0, 262144 - expanded))
    for start in starts:
        i = 0
        j = start
        edits = []
        decisions = []
        while i < len(expected) and j < len(stream):
            expanded += 1
            if expanded > 262144:
                error = Fault('CorrespondenceLimit', 'alignment exceeds 262144 states',
                              'correspondence', document, side)
                error.row['search'] = search_record()
                raise error
            event = events.get(j)
            if event and decoded['tokens'][source_chars[i][0]]['kind'] == 'tag':
                break
            if event and stream[j] == '-' and expected[i] != '-':
                if (i and expected[i - 1].isascii() and expected[i - 1].isalpha() and
                    expected[i].isascii() and expected[i].isalpha() and
                    source_chars[i - 1][0] == source_chars[i][0] and
                    decoded['tokens'][source_chars[i][0]]['kind'] == 'word'):
                    source = source_chars[i]
                    event.update(decision='remove-discretionary', token_ordinal=source[0],
                                 source_range=source[1], intra_token=source[3])
                    decisions.append(event.copy())
                    edits.append((j, j + 1, '', source[1]))
                    j += 1
                    if j < len(stream) and stream[j] == ' ':
                        edits.append((j, j + 1, '', source[1]))
                        j += 1
                    continue
            if stream[j] != expected[i]:
                break
            if event:
                source = source_chars[i]
                kind = ('keep-lexical' if stream[j] == '-' else
                        'keep-with-separator' if i + 1 < len(expected) and
                        expected[i + 1] == ' ' else 'keep-without-separator')
                event.update(decision=kind, token_ordinal=source[0],
                             source_range=source[1], intra_token=source[3])
                decisions.append(event.copy())
                j += 1
                i += 1
                pdf_space = j < len(stream) and stream[j] == ' '
                source_space = i < len(expected) and expected[i] == ' '
                if pdf_space and not source_space:
                    edits.append((j, j + 1, '', source[1]))
                    j += 1
                elif source_space and not pdf_space:
                    separator = source_chars[i][1]
                    edits.append((j, j, ' ', separator))
                    i += 1
                continue
            i += 1
            j += 1
        if (i == len(expected) and (start == 0 or not stream[start - 1].isalnum()) and
                (j == len(stream) or not stream[j].isalnum())):
            canonical = stream[start:j]
            for begin, end, replacement, _ in reversed(edits):
                canonical = canonical[:begin - start] + replacement + canonical[end - start:]
            proofs.append(dict(range=[start, j], canonical=canonical,
                               source_ranges=[token['source_range']
                                              for token in decoded['tokens']],
                               decisions=decisions, edits=edits,
                               start=extracted['offsets'][start],
                               end=extracted['offsets'][j - 1]))
    try:
        result = unique_proof(proofs, document, side)
    except Fault as error:
        error.row['search'] = search_record()
        raise
    result['search'] = search_record()
    return result


def canonicalize(extracted, proof):
    """Apply only source-proved event edits to a complete normalized stream."""
    edits = {begin: (end, replacement, source) for begin, end, replacement, source
             in proof['edits']}
    text = extracted['stream']
    output = []
    offsets = []
    i = 0
    while i < len(text):
        if i in edits:
            begin = i
            end, replacement, source = edits[i]
            for char in replacement:
                output.append(char)
                offsets.append(dict(raw=[], source=source))
            i = end
            if end > begin:
                continue
            if i >= len(text):
                break
            # A zero-width source separator precedes this raw character.
        output.append(text[i])
        offsets.append(dict(raw=[extracted['offsets'][i]], source=None))
        i += 1
    if len(text) in edits:
        _, replacement, source = edits[len(text)]
        for char in replacement:
            output.append(char)
            offsets.append(dict(raw=[], source=source))
    return ''.join(output), offsets


def unchanged_paragraph_events(before_bytes, after_bytes, before, after):
    """Record unchanged selected events by source token, raw line and mark."""
    if before_bytes != after_bytes:
        return False
    def keyed(rows):
        result = {}
        for row in rows:
            key = (row['token_ordinal'], row['intra_token'], row['mark'])
            if key in result:
                return None
            result[key] = row
        return result
    old = keyed(before['decisions'])
    new = keyed(after['decisions'])
    if old is None or new is None or not old or set(old) != set(new):
        return False
    if any(old[key]['raw_line'] != new[key]['raw_line'] or
           old[key]['source_range'] != new[key]['source_range'] for key in old):
        return False
    for row in before['decisions'] + after['decisions']:
        row['source_decision'] = row['decision']
        row['decision'] = 'unchanged-paragraph'
    return True


def residual_events(extracted, proof, document, side):
    """Name event locations by residual interval and mark-retaining token order."""
    left, right = proof['range'] if proof else (0, 0)
    rows = []
    for event in line_events(extracted, document, side):
        offset = event['normalized_offset']
        if left <= offset < right and proof:
            match = next((row for row in proof['decisions']
                          if row['normalized_offset'] == offset), None)
            if match is None:
                event['reason'] = 'selected event lacks source interpretation'
            else:
                event = match
            rows.append(event)
            continue
        ordinal = 0 if proof is None or offset < left else 1
        origin = 0 if ordinal == 0 else right
        segment = extracted['stream'][origin:offset]
        token_ordinal = len(re.findall(r'[A-Za-z]+|[0-9]+|[^\sA-Za-z0-9]', segment))
        event['residual_key'] = [document, ordinal, token_ordinal]
        rows.append(event)
    return rows


def residual_proof(before, after, before_rows, after_rows, envelope,
                   before_match=None, after_match=None, failure=None):
    """Bypass only raw-identical events at stable residual token positions."""
    def intervals(extracted, match):
        if match is None:
            return [extracted['stream']]
        left, right = match['range']
        return [extracted['stream'][:left], extracted['stream'][right:]]

    old = {tuple(row['residual_key']): row for row in before_rows
           if 'residual_key' in row}
    new = {tuple(row['residual_key']): row for row in after_rows
           if 'residual_key' in row}
    if intervals(before, before_match) != intervals(after, after_match):
        if failure is not None:
            failure.append('residual compared text differs')
        return False
    if (not envelope['available'] or
            len(old) != len(new) or
            len(old) != sum('residual_key' in row for row in before_rows) or
            len(new) != sum('residual_key' in row for row in after_rows) or
            set(old) != set(new)):
        return False
    if any(old[key]['raw_line'] != new[key]['raw_line'] or
           old[key]['mark'] != new[key]['mark'] for key in old):
        return False
    for row in before_rows + after_rows:
        if 'residual_key' in row:
            row['decision'] = 'unchanged-residual'
    return True


def negative_witness(text, offsets):
    removed = '-–—/'
    kept = []
    origins = []
    discarded = []
    for char, origin in zip(text, offsets):
        if char in removed or char.isspace():
            discarded.append(origin)
        else:
            kept.append(char)
            origins.append(origin)
    return dict(deleted='U+002D U+2013 U+2014 U+002F whitespace',
                stream=''.join(kept), offsets=origins, removed=discarded)


def extractor_identity():
    try:
        libraries = pdf_libraries()
        poppler = ctypes.CDLL(ctypes.util.find_library('poppler-glib'))
        version = poppler.poppler_get_version
        version.restype = ctypes.c_char_p
        value = version()
        if not value:
            raise ValueError('Poppler version unavailable')
    except Exception as error:
        raise Fault('ExtractionUnavailable', error, 'extraction') from error
    return dict(libraries=libraries, poppler=value.decode('utf-8'),
                python=sys.version, unicode=unicodedata.unidata_version)


def snapshot_inputs(repo, destination, held):
    paper_inputs = inventory(repo)['inputs']
    receipt_path = repo / 'generated/paper/inputs.json'
    try:
        receipt = json.loads(held.read_bytes(receipt_path))
    except (OSError, ValueError) as error:
        raise Fault('InputInvalid', 'paper receipt is unavailable: ' + str(error),
                    'snapshot') from error
    if not isinstance(receipt, dict) or not isinstance(receipt.get('inputs'), dict):
        raise Fault('InputInvalid', 'paper receipt has no input inventory', 'snapshot')
    if receipt['inputs'] != paper_inputs:
        raise Fault('InputChanged', 'paper receipt differs from source inventory', 'snapshot')
    paths = set(paper_inputs) | set(input_paths(repo, held))
    paths.update(row[2] for row in scope_fragments(repo))
    paths.add('generated/paper/inputs.json')
    inputs = []
    for name in sorted(paths):
        source = repo / name
        try:
            data = held.read_bytes(source)
            if name in paper_inputs and digest(data) != paper_inputs[name]:
                raise Fault('InputChanged', 'input changed during capture: ' + name, 'snapshot')
            target = destination / name
            try:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
            except OSError as error:
                raise Fault('EvidenceUnavailable', error, 'snapshot') from error
        except (OSError, ValueError) as error:
            raise Fault('InputInvalid', 'cannot capture ' + name + ': ' + str(error),
                        'snapshot') from error
        inputs.append(dict(path=name, sha256=digest(data)))
    return inputs


def parent_snapshot(repo, revision, destination):
    """Capture a committed paper input inventory without running its tools."""
    commit = git(repo, 'rev-parse', '--verify', revision + '^{commit}').decode().strip()
    receipt_name = 'generated/paper/inputs.json'
    try:
        receipt_bytes = git(repo, 'show', commit + ':' + receipt_name)
        receipt = json.loads(receipt_bytes)
    except (Fault, ValueError) as error:
        raise Fault('SourceInvalid', 'parent paper receipt is invalid', 'snapshot') from error
    if not isinstance(receipt, dict) or not isinstance(receipt.get('inputs'), dict):
        raise Fault('SourceInvalid', 'parent paper receipt has no inputs', 'snapshot')
    names = set(receipt['inputs']) | {receipt_name}
    names.update(input_paths_from_parent(repo, commit))
    if git_has(repo, commit, 'generated/documents/inclusions.tsv'):
        names.add('generated/documents/inclusions.tsv')
    blobs = {}
    for entry in git(repo, 'ls-tree', '-r', '-z', commit).split(b'\0'):
        if entry:
            metadata, path = entry.split(b'\t', 1)
            blobs[path.decode('utf-8')] = metadata.split()[2].decode('ascii')
    rows = []
    for name in sorted(names):
        path = pathlib.PurePosixPath(name)
        if path.is_absolute() or '..' in path.parts or not name:
            raise Fault('SourceInvalid', 'parent receipt has unsafe path: ' + name, 'snapshot')
        data = git(repo, 'show', commit + ':' + name)
        if name in receipt['inputs'] and digest(data) != receipt['inputs'][name]:
            raise Fault('SourceInvalid', 'parent receipt digest differs: ' + name, 'snapshot')
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        rows.append(dict(path=name, sha256=digest(data), blob=blobs[name]))
    fragments = parent_fragments(destination)
    sources = (destination / 'documents/paper-build/sources.tsv').read_text().splitlines()
    for source, unit, fragment, role, line in fragments:
        if source not in sources or source not in receipt['inputs'] or fragment not in receipt['inputs']:
            raise Fault('SourceInvalid', 'parent fragment is outside recorded sources: ' + unit,
                        'snapshot')
    if (destination / 'generated/documents/inclusions.tsv').is_file():
        try:
            declared_fragments(destination, sources)
        except (OSError, ValueError) as error:
            raise Fault('SourceInvalid', 'parent inclusion validation failed: ' + str(error),
                        'snapshot') from error
    return dict(commit=commit, receipt=receipt, inputs=rows, fragments=fragments)


def git_has(repo, commit, name):
    return subprocess.run(['git', '-C', str(repo), 'cat-file', '-e', commit + ':' + name],
                          capture_output=True).returncode == 0


def input_paths_from_parent(repo, commit):
    source_list = 'documents/paper-build/sources.tsv'
    try:
        sources = git(repo, 'show', commit + ':' + source_list).decode('utf-8').splitlines()
    except UnicodeError as error:
        raise Fault('SourceInvalid', 'parent source list is not UTF-8', 'snapshot') from error
    # These are validation reads of the current binding owner, not paper byte inputs.
    return set(sources) | set(BINDING_INPUTS + CHECK_INPUTS + [source_list])


def parent_fragments(snapshot):
    """Read current inclusions or one historical literal declaration."""
    inclusion = snapshot / 'generated/documents/inclusions.tsv'
    if inclusion.is_file():
        return scope_fragments(snapshot)
    script = snapshot / 'tools/paper/prepare.py'
    try:
        assignments = [node.value for node in ast.parse(script.read_text()).body
                       if isinstance(node, ast.Assign) and
                       any(isinstance(target, ast.Name) and target.id == 'FRAGMENTS'
                           for target in node.targets)]
        if len(assignments) != 1:
            raise ValueError('expected one FRAGMENTS assignment')
        declared = ast.literal_eval(assignments[0])
    except (OSError, SyntaxError, ValueError) as error:
        raise Fault('SourceInvalid', 'parent fragment declaration is not literal',
                    'snapshot') from error
    rows = []
    used = set()
    for entry in declared:
        if not isinstance(entry, tuple) or len(entry) not in (4, 5):
            raise Fault('SourceInvalid', 'parent fragment tuple has unsupported shape', 'snapshot')
        if len(entry) == 4:
            source, unit, fragment, role = entry
            marker = ('% generated: ' + unit).encode()
            lines = (snapshot / source).read_bytes().splitlines()
            positions = [number for number, line in enumerate(lines, 1) if line == marker]
            if len(positions) != 1:
                raise Fault('SourceInvalid', 'parent fragment marker is not unique: ' + unit,
                            'snapshot')
            line = positions[0]
        else:
            source, line, last, fragment, unit = entry
            if not isinstance(line, int) or not isinstance(last, int) or line < 1 or last < line:
                raise Fault('SourceInvalid', 'parent fragment range is invalid: ' + unit,
                            'snapshot')
            lines = (snapshot / source).read_bytes().splitlines()
            if last > len(lines) or lines[line - 1] != ('% generated: ' + unit).encode():
                raise Fault('SourceInvalid', 'parent fragment range misses marker: ' + unit,
                            'snapshot')
            if any(content != b'%' for content in lines[line:last]):
                raise Fault('SourceInvalid', 'parent fragment padding differs: ' + unit,
                            'snapshot')
            data = (snapshot / fragment).read_bytes()
            role = ('paragraph' if data.startswith(b'\\PaperNativeParagraph{') else
                    'quotation' if data.startswith(b'\\paperrefrain{') else
                    'refrain' if data.startswith(b'\\refrain{') else None)
        data = (snapshot / fragment).read_bytes()
        macro = {'paragraph': b'PaperNativeParagraph', 'quotation': b'paperrefrain',
                 'refrain': b'refrain'}.get(role)
        if macro is None or not data.startswith(b'\\' + macro + b'{' + unit.encode() + b'}{'):
            raise Fault('SourceInvalid', 'parent fragment wrapper differs: ' + unit, 'snapshot')
        if not data.endswith(b'}\n'):
            raise Fault('SourceInvalid', 'parent fragment ending differs: ' + unit, 'snapshot')
        key = (source, line)
        if key in used:
            raise Fault('SourceInvalid', 'parent fragment site repeats: ' + source,
                        'snapshot')
        used.add(key)
        rows.append((source, unit, fragment, role, line))
    return rows


def selected_sources(repo, args, parent, parent_dir, candidate_dir):
    """Bind the requested occurrence to both checked inclusion places."""
    if bool(args.before_source) == bool(getattr(args, 'before_revision', None)):
        raise Fault('SourceInvalid', 'exactly one parent selector is required', 'snapshot')
    current = [(source, fragment, line) for source, unit, fragment, role, line
               in scope_fragments(candidate_dir) if unit == args.unit and role == 'paragraph']
    if len(current) != 1:
        raise Fault('UnitInvalid', 'candidate paragraph occurrence is absent or ambiguous',
                    'snapshot')
    source_name, fragment_name, line = current[0]
    candidate_source = (candidate_dir / source_name).read_bytes()
    marker = ('% generated: ' + args.unit + '\n').encode()
    source_lines = candidate_source.splitlines(keepends=True)
    if line < 1 or line > len(source_lines) or source_lines[line - 1] != marker:
        raise Fault('SourceInvalid', 'candidate inclusion marker differs', 'snapshot')
    marker_start = sum(len(part) for part in source_lines[:line - 1])
    candidate_fragment = (candidate_dir / fragment_name).read_bytes()
    prefix = ('\\PaperNativeParagraph{' + args.unit + '}{').encode()
    if not candidate_fragment.startswith(prefix) or not candidate_fragment.endswith(b'}\n'):
        raise Fault('SourceInvalid', 'candidate paragraph wrapper differs', 'snapshot')
    candidate = dict(kind='native', path=fragment_name, body=candidate_fragment[len(prefix):-2],
                     blob_bytes=candidate_fragment,
                     range=[len(prefix), len(candidate_fragment) - 2],
                     source=source_name, marker=[marker_start, marker_start + len(marker)],
                     line=line)
    existing = [(source, fragment, position) for source, unit, fragment, role, position
                in parent['fragments'] if unit == args.unit and role == 'paragraph']
    if args.before_source:
        if existing:
            raise Fault('SourceInvalid', 'native parent needs --before-revision', 'snapshot')
        authored = source_blob(repo, args.before_source)
        if authored['commit'] != parent['commit']:
            raise Fault('SourceInvalid', 'source selector and parent differ', 'snapshot')
        if authored['path'] != source_name or authored['first'] != line:
            raise Fault('SourceInvalid', 'source selector misses inclusion place', 'snapshot')
        listed = (parent_dir / 'documents/paper-build/sources.tsv').read_text().splitlines()
        if source_name not in parent['receipt']['inputs'] or source_name not in listed:
            raise Fault('SourceInvalid', 'source selector is outside declared inputs', 'snapshot')
        old = authored['blob_bytes']
        start, end = authored['bytes']
        if old[:start] != candidate_source[:marker_start] or old[end:] != candidate_source[marker_start + len(marker):]:
            raise Fault('SourceInvalid', 'source differs outside selected paragraph', 'snapshot')
        before = dict(kind='authored', path=source_name, body=authored['body'].encode(),
                      blob_bytes=old, range=authored['bytes'],
                      source=source_name, line=authored['first'],
                      commit=parent['commit'], blob=authored['blob'])
    else:
        if len(existing) != 1:
            raise Fault('SourceInvalid', 'native parent paragraph is absent or ambiguous',
                        'snapshot')
        old_source, old_fragment, old_line = existing[0]
        if old_source != source_name:
            raise Fault('SourceInvalid', 'parent paragraph has another source', 'snapshot')
        lines = (parent_dir / old_source).read_bytes().splitlines(keepends=True)
        if not 1 <= old_line <= len(lines) or lines[old_line - 1] != marker:
            raise Fault('SourceInvalid', 'parent marker differs', 'snapshot')
        old_fragment_bytes = (parent_dir / old_fragment).read_bytes()
        if not old_fragment_bytes.startswith(prefix) or not old_fragment_bytes.endswith(b'}\n'):
            raise Fault('SourceInvalid', 'parent wrapper differs', 'snapshot')
        before = dict(kind='native', path=old_fragment,
                      body=old_fragment_bytes[len(prefix):-2], blob_bytes=old_fragment_bytes,
                      range=[len(prefix), len(old_fragment_bytes) - 2],
                      source=old_source, line=old_line, commit=parent['commit'],
                      marker=[sum(len(part) for part in lines[:old_line - 1]),
                              sum(len(part) for part in lines[:old_line])])
    return before, candidate


def source_envelope(parent, parent_dir, candidate_dir, candidate_inputs, before, after,
                    unit):
    """Prove unchanged residual inputs after one checked paragraph substitution."""
    sentinel = ('<selected paragraph ' + unit + '>').encode()
    parent_names = set(parent['receipt']['inputs'])
    candidate_receipt = json.loads((candidate_dir / 'generated/paper/inputs.json').read_bytes())
    candidate_names = set(candidate_receipt['inputs'])
    parent_captured = {row['path'] for row in parent['inputs']}
    candidate_captured = {row['path'] for row in candidate_inputs if not row['path'].startswith('/')}
    selected_paths = {name for name in (before['path'], after['path'])
                      if name != before['source']}
    reasons = []
    old_rows = parent['fragments']
    new_rows = scope_fragments(candidate_dir)
    source_list = (candidate_dir / 'documents/paper-build/sources.tsv').read_text().splitlines()
    order = {name: position for position, name in enumerate(source_list)}
    # The generator emits paper uses before explainer uses, then sorts each
    # document's uses by the closed source list and marker line.
    rank = {'paper-pilot': 0, 'explainer-pilot': 1}

    def generator_key(row):
        document = pathlib.PurePosixPath(row[2]).parts[2]
        return rank[document], order[row[0]], row[4]

    if any(row[0] not in order or len(pathlib.PurePosixPath(row[2]).parts) < 3 or
           pathlib.PurePosixPath(row[2]).parts[2] not in rank
           for row in new_rows):
        reasons.append('inclusion source is not declared')
    elif list(new_rows) != sorted(new_rows, key=generator_key):
        reasons.append('inclusion order differs from generator order')
    if (parent_dir / 'generated/documents/inclusions.tsv').is_file():
        parent_sources = (parent_dir / 'documents/paper-build/sources.tsv').read_text().splitlines()
        parent_order = {name: position for position, name in enumerate(parent_sources)}
        if any(row[0] not in parent_order or
               len(pathlib.PurePosixPath(row[2]).parts) < 3 or
               pathlib.PurePosixPath(row[2]).parts[2] not in rank for row in old_rows):
            reasons.append('parent inclusion source is not declared')
        elif list(old_rows) != sorted(
                old_rows, key=lambda row: (rank[pathlib.PurePosixPath(row[2]).parts[2]],
                                           parent_order[row[0]], row[4])):
            reasons.append('parent inclusion order differs from generator order')
    old_other = {(src, key, role): line for src, key, _, role, line in old_rows
                 if not (key == unit and role == 'paragraph')}
    new_other = {(src, key, role): line for src, key, _, role, line in new_rows
                 if not (key == unit and role == 'paragraph')}
    if set(old_other) != set(new_other):
        reasons.append('other inclusion identities differ')
    shift = 0
    # The authored interval's physical-line count, not its byte length, sets
    # the line shift of later rows in the same source.
    if before['kind'] == 'authored':
        shift = 1 - len(before['body'].splitlines())
    for key in set(old_other) & set(new_other):
        source, _, _ = key
        expected = old_other[key] + (shift if source == before['source'] and
                                     old_other[key] > before['line'] else 0)
        if new_other[key] != expected:
            reasons.append('inclusion line differs: ' + key[1])
    excluded = selected_paths | {'generated/documents/inclusions.tsv',
                                 'generated/paper/inputs.json'}
    if parent_names - selected_paths - {'generated/documents/inclusions.tsv'} != (
            candidate_names - selected_paths - {'generated/documents/inclusions.tsv'}):
        reasons.append('paper input membership differs')
    if parent_captured - excluded != candidate_captured - excluded:
        reasons.append('validation input membership differs')
    rows = []
    for name in sorted((parent_captured | candidate_captured) - excluded):
        old = parent_dir / name
        new = candidate_dir / name
        if not old.is_file() or not new.is_file():
            reasons.append('paper input is absent: ' + name)
            continue
        left, right = old.read_bytes(), new.read_bytes()
        original_left, original_right = digest(left), digest(right)
        if name == before['source']:
            a, b = before['range'] if before['kind'] == 'authored' else before['marker']
            c, d = after['marker']
            left = left[:a] + sentinel + left[b:]
            right = right[:c] + sentinel + right[d:]
        if left != right:
            reasons.append('paper input differs: ' + name)
        rows.append(dict(path=name, before_original=original_left,
                         after_original=original_right,
                         before_substituted=digest(left), after_substituted=digest(right)))
    old_inclusion = parent_dir / 'generated/documents/inclusions.tsv'
    new_inclusion = candidate_dir / 'generated/documents/inclusions.tsv'
    if old_inclusion.is_file() and new_inclusion.is_file():
        rewritten = []
        selected_count = 0
        for row in new_inclusion.read_bytes().splitlines(keepends=True):
            fields = row.rstrip(b'\n').split(b'\t')
            if len(fields) != 5:
                reasons.append('inclusion row grammar differs')
                break
            source, key, fragment, role, line = fields
            if key == unit.encode() and role == b'paragraph':
                selected_count += 1
                continue
            if source.decode() == before['source'] and int(line) > after['line']:
                fields[4] = str(int(line) - shift).encode()
            rewritten.append(b'\t'.join(fields) + (b'\n' if row.endswith(b'\n') else b''))
        if before['kind'] == 'native':
            old_comparable = [row for row in old_inclusion.read_bytes().splitlines(keepends=True)
                              if b'\t' + unit.encode() + b'\t' not in row or
                              b'\tparagraph\t' not in row]
        else:
            old_comparable = old_inclusion.read_bytes().splitlines(keepends=True)
        if selected_count != 1 or b''.join(rewritten) != b''.join(old_comparable):
            reasons.append('inclusion bytes differ outside selected row')
        rows.append(dict(path='generated/documents/inclusions.tsv',
                         before_original=digest(old_inclusion.read_bytes()),
                         after_original=digest(new_inclusion.read_bytes()),
                         before_substituted=digest(b''.join(old_comparable)),
                         after_substituted=digest(b''.join(rewritten)), selected=True))
    else:
        reasons.append('inclusion declaration format differs')
    for name in selected_paths:
        if name != before['source'] and name != after['source']:
            if name in parent_names or name in candidate_names:
                rows.append(dict(path=name, selected=True,
                                 before_original=(digest((parent_dir / name).read_bytes())
                                                  if (parent_dir / name).is_file() else None),
                                 after_original=(digest((candidate_dir / name).read_bytes())
                                                 if (candidate_dir / name).is_file() else None)))
    parent_meta = {key: value for key, value in parent['receipt'].items() if key != 'inputs'}
    candidate_meta = {key: value for key, value in candidate_receipt.items()
                      if key != 'inputs'}
    if parent_meta != candidate_meta:
        reasons.append('paper receipt metadata differs')
    return dict(available=not reasons, reasons=sorted(set(reasons)), rows=rows,
                inclusion_shift=shift, source_substitution=dict(
                    path=before['source'],
                    before=before['marker'] if before['kind'] == 'native' else before['range'],
                    after=after['marker']))


def compare(args):
    repo = args.repository.resolve()
    output = args.output
    if not output.is_absolute():
        raise Fault('InputInvalid', 'evidence directory must be absolute')
    if output.exists():
        raise Fault('OutputExists', 'evidence directory exists')
    if not repo.is_dir() or not (repo / '.git').exists():
        raise Fault('InputInvalid', 'repository is unavailable')
    try:
        output.mkdir(parents=True)
    except OSError as error:
        raise Fault('EvidenceUnavailable', error, 'request') from error
    evidence = dict(schema=2, verdict='refused', faults=[], differences=[],
                    source=None, inputs=[], extractor=None, tags=[], documents={},
                    correspondence=None, unsupported_locator=True, units=[],
                    line_events=[], residuals=[], negative_witness={}, coverage={})
    originals = {}
    copies = {}
    stage = 'snapshot'
    try:
        selector = getattr(args, 'before_source', None)
        revision = getattr(args, 'before_revision', None)
        if bool(selector) == bool(revision):
            raise Fault('SourceInvalid', 'exactly one parent selector is required', 'snapshot')
        if selector:
            match = SOURCE.fullmatch(selector)
            if not match:
                raise Fault('SourceInvalid', 'invalid source selector', 'snapshot')
            revision = match[1]
        parent_commit = git(repo, 'rev-parse', '--verify', revision + '^{commit}').decode().strip()
        stage = 'request'
        for document in ('paper', 'explainer'):
            for side in ('before', 'after'):
                path = getattr(args, side + '_' + document).resolve()
                if not path.is_file():
                    raise Fault('InputInvalid', 'PDF is absent: ' + str(path),
                                'request', document, side)
                try:
                    data = path.read_bytes()
                except OSError as error:
                    raise Fault('InputInvalid', error, 'request', document, side) from error
                target = output / (document + '-' + side + '.pdf')
                try:
                    target.write_bytes(data)
                except OSError as error:
                    raise Fault('EvidenceUnavailable', error, 'snapshot', document, side) from error
                originals[document, side] = path, data
                copies[document, side] = target
                evidence['inputs'].append(dict(path=str(path), sha256=digest(data)))
                if side == 'before':
                    historical = git(repo, 'show', parent_commit +
                                     ':generated/paper/' + document + '.pdf')
                    if data != historical:
                        raise Fault('InputInvalid', 'before PDF differs from source revision',
                                    'request', document, side)
                else:
                    try:
                        candidate = (repo / 'generated/paper' / (document + '.pdf')).read_bytes()
                    except OSError as error:
                        raise Fault('InputInvalid', error, 'request', document, side) from error
                    if data != candidate:
                        raise Fault('InputInvalid', 'after PDF differs from candidate tree',
                                    'request', document, side)
        stage = 'snapshot'
        held = CapturedInputs(repo)
        snapshot = output / 'snapshot'
        evidence['inputs'].extend(snapshot_inputs(repo, snapshot, held))
        parent_dir = output / 'parent-snapshot'
        parent = parent_snapshot(repo, parent_commit, parent_dir)
        evidence['parent'] = {key: value for key, value in parent.items()
                              if key != 'receipt'}
        before_source, after_source = selected_sources(repo, args, parent, parent_dir,
                                                       snapshot)
        evidence['source'] = {key: value for key, value in before_source.items()
                              if key not in ('body', 'blob_bytes')}
        try:
            (output / 'source.blob').write_bytes(before_source['blob_bytes'])
        except OSError as error:
            raise Fault('EvidenceUnavailable', error, 'snapshot') from error
        envelope = source_envelope(parent, parent_dir, snapshot, evidence['inputs'],
                                   before_source, after_source, args.unit)
        evidence['residuals'].append(envelope)
        rows = (snapshot / 'documents/paper-build/sources.tsv').read_text().splitlines()
        declared_fragments(snapshot, rows)
        # Each side resolves tags against the binding bytes captured for that side.
        before_lookup = bindings(parent_dir)
        after_lookup = bindings(snapshot)
        before_decoded = decode_located(before_source['body'].decode('utf-8'),
                                        before_lookup, before_source['path'],
                                        before_source['range'][0])
        after_decoded = decode_located(after_source['body'].decode('utf-8'),
                                       after_lookup, after_source['path'],
                                       after_source['range'][0])
        before, after = before_decoded['extracted'], after_decoded['extracted']
        before_semantic, after_semantic = before_decoded['semantic'], after_decoded['semantic']
        before_tags, after_tags = before_decoded['tags'], after_decoded['tags']
        tags = {(row['key'], row['spelling'], row['target']) for row in before_tags + after_tags}
        evidence['tags'] = [dict(key=key, spelling=spelling, target=target)
                            for key, spelling, target in sorted(tags)]
        before_source['sha256'] = digest(before_source['blob_bytes'])
        after_source['sha256'] = digest(after_source['blob_bytes'])
        before_source['blob'] = next(row['blob'] for row in parent['inputs']
                                     if row['path'] == before_source['path'])
        before_source['receipt_digest'] = parent['receipt']['inputs'].get(before_source['path'])
        after_source['receipt_digest'] = json.loads(
            (snapshot / 'generated/paper/inputs.json').read_bytes())['inputs'].get(after_source['path'])
        before_source['binding_inputs'] = [row for row in parent['inputs']
                                           if row['path'] in BINDING_INPUTS + CHECK_INPUTS]
        after_source['binding_inputs'] = [row for row in evidence['inputs']
                                          if row['path'] in BINDING_INPUTS + CHECK_INPUTS]
        evidence['units'] = [dict(unit=args.unit, occurrences=[dict(
            sources=dict(before={key: value for key, value in before_source.items()
                                 if key not in ('body', 'blob_bytes')},
                         after={key: value for key, value in after_source.items()
                                if key not in ('body', 'blob_bytes')}),
            tokens=dict(before=before_decoded['tokens'], after=after_decoded['tokens']))])]
        stage = 'extraction'
        evidence['extractor'] = extractor_identity()
        extraction_faults = []
        for document in ('paper', 'explainer'):
            evidence['documents'][document] = {}
            for side in ('before', 'after'):
                try:
                    evidence['documents'][document][side] = extract(
                        copies[document, side], document, side)
                except Fault as error:
                    extraction_faults.append(error.row)
                    evidence['documents'][document][side] = None
        if extraction_faults:
            first_stage = min(STAGES[row['stage']] for row in extraction_faults)
            evidence['faults'].extend(row for row in extraction_faults
                                      if STAGES[row['stage']] == first_stage)
            raise StageHalt
        stage = 'correspondence'
        paper = evidence['documents']['paper']
        before_proof = align_located(before_decoded, paper['before'], 'paper', 'before')
        evidence['units'][0]['occurrences'][0]['search'] = {
            'before': before_proof['search']}
        after_proof = align_located(after_decoded, paper['after'], 'paper', 'after')
        proof = dict(before=before_proof, after=after_proof)
        unchanged_paragraph_events(before_source['blob_bytes'], after_source['blob_bytes'],
                                   proof['before'], proof['after'])
        evidence['correspondence'] = proof
        evidence['units'][0]['occurrences'][0]['search']['after'] = after_proof['search']
        if before_semantic != after_semantic:
            evidence['differences'].append(dict(code='TextDifference', part='source/body',
                                                before=before_semantic, after=after_semantic,
                                                proof='direct'))
        if before_tags != after_tags:
            evidence['differences'].append(dict(code='TextDifference', part='source/tags',
                                                before=before_tags, after=after_tags,
                                                proof='direct'))
        stage = 'comparison'
        unresolved = []
        for document in ('paper', 'explainer'):
            before_pdf = evidence['documents'][document]['before']
            after_pdf = evidence['documents'][document]['after']
            mapped = document == 'paper'
            before_proof = proof['before'] if mapped else None
            after_proof = proof['after'] if mapped else None
            for side, pdf, match in (('before', before_pdf, before_proof),
                                     ('after', after_pdf, after_proof)):
                if match:
                    canonical, offsets = canonicalize(pdf, match)
                else:
                    canonical = pdf['stream']
                    offsets = [dict(raw=[raw], source=None) for raw in pdf['offsets']]
                pdf['canonical_stream'] = canonical
                pdf['canonical_offsets'] = offsets
                pdf['line_events'] = residual_events(pdf, match, document, side)
                evidence['line_events'].extend(pdf['line_events'])
                evidence['negative_witness'].setdefault(document, {})[side] = negative_witness(
                    canonical, offsets)
            residual_failure = []
            bypass = residual_proof(before_pdf, after_pdf, before_pdf['line_events'],
                                    after_pdf['line_events'], envelope,
                                    before_proof, after_proof, residual_failure)
            if residual_failure:
                for row in before_pdf['line_events'] + after_pdf['line_events']:
                    if 'residual_key' in row:
                        row['reason'] = residual_failure[0]
            interval_count = 2 if mapped else 1
            for ordinal in range(interval_count):
                bounds = {}
                for side, pdf, match in (('before', before_pdf, before_proof),
                                         ('after', after_pdf, after_proof)):
                    left = 0 if ordinal == 0 else match['range'][1]
                    right = match['range'][0] if ordinal == 0 and match else len(pdf['stream'])
                    segment = pdf['stream'][left:right]
                    bounds[side] = dict(range=[left, right],
                                        compared_tokens_sha256=digest(segment.encode()),
                                        event_keys=[row['residual_key'] for row in pdf['line_events']
                                                    if row.get('residual_key', [None, -1])[1] == ordinal])
                evidence['residuals'].append(dict(document=document, ordinal=ordinal,
                                                  before=bounds['before'], after=bounds['after'],
                                                  unchanged=bypass,
                                                  source_envelope=envelope['available']))
            outstanding = [row for row in before_pdf['line_events'] + after_pdf['line_events']
                           if row['decision'] == 'unresolved']
            unresolved.extend(outstanding)
            if before_pdf['titles'] != after_pdf['titles']:
                evidence['differences'].append(dict(
                    code='TextDifference', document=document, part='titles', proof='direct',
                    before=before_pdf['titles'], after=after_pdf['titles']))
            left = before_pdf['canonical_stream']
            right = after_pdf['canonical_stream']
            if left != right:
                offset = next((i for i, (a, b) in enumerate(zip(left, right)) if a != b),
                              min(len(left), len(right)))
                if not outstanding:
                    evidence['differences'].append(dict(
                        code='TextDifference', document=document, part='stream', proof='direct',
                        location=dict(before=before_pdf['canonical_offsets'][offset]
                                      if offset < len(left) else None,
                                      after=after_pdf['canonical_offsets'][offset]
                                      if offset < len(right) else None),
                        before=left, after=right))
                else:
                    left_q = evidence['negative_witness'][document]['before']['stream']
                    right_q = evidence['negative_witness'][document]['after']['stream']
                    if left_q != right_q:
                        witness_offset = next((i for i, (a, b) in enumerate(zip(left_q, right_q))
                                               if a != b), min(len(left_q), len(right_q)))
                        evidence['differences'].append(dict(
                            code='TextDifference', document=document, part='stream',
                            proof='witness', witness_offset=witness_offset,
                            before=left_q, after=right_q))
        evidence['coverage'] = dict(decoded_units=[args.unit],
                                    interpreted=sum(row['decision'] not in
                                                    ('unresolved', 'unchanged-residual',
                                                     'unchanged-paragraph')
                                                    for row in evidence['line_events']),
                                    bypassed=sum(row['decision'] in
                                                 ('unchanged-residual', 'unchanged-paragraph')
                                                 for row in evidence['line_events']),
                                    unresolved=len(unresolved),
                                    debts=['locators', 'floats', 'legacy source'] if unresolved else [])
        stage = 'final'
        for (document, side), (path, data) in originals.items():
            try:
                changed = path.read_bytes() != data or copies[document, side].read_bytes() != data
            except OSError as error:
                raise Fault('InputChanged', error, 'final', document, side) from error
            if changed:
                raise Fault('InputChanged', 'PDF changed after capture: ' + str(path),
                            'final', document, side)
            if side == 'after':
                try:
                    current = (repo / 'generated/paper' / (document + '.pdf')).read_bytes()
                except OSError as error:
                    raise Fault('InputChanged', error, 'final', document, side) from error
                if current != data:
                    raise Fault('InputChanged', 'candidate PDF changed after capture',
                                'final', document, side)
        try:
            held.verify()
        except (OSError, ValueError) as error:
            raise Fault('InputChanged', error, 'final') from error
        for row in evidence['inputs']:
            if not row['path'].startswith('/'):
                try:
                    changed = digest((snapshot / row['path']).read_bytes()) != row['sha256']
                except OSError as error:
                    raise Fault('InputChanged', 'captured snapshot is unavailable: ' + row['path'],
                                'final') from error
                if changed:
                    raise Fault('InputChanged', 'captured snapshot changed: ' + row['path'], 'final')
        for row in parent['inputs']:
            path = parent_dir / row['path']
            try:
                changed = digest(path.read_bytes()) != row['sha256']
            except OSError as error:
                raise Fault('InputChanged', 'parent snapshot is unavailable: ' + row['path'],
                            'final') from error
            if changed:
                raise Fault('InputChanged', 'parent snapshot changed: ' + row['path'], 'final')
        if extractor_identity() != evidence['extractor']:
            raise Fault('InputChanged', 'extractor libraries changed', 'final')
        if evidence['differences']:
            evidence['verdict'] = 'different'
        elif unresolved:
            for row in unresolved:
                location = row['raw']
                envelope_fault = 'residual_key' in row and not envelope['available']
                detail = ('source envelope differs: ' + '; '.join(envelope['reasons'])
                          if envelope_fault else
                          row.get('reason', 'line-end interpretation lacks source proof'))
                evidence['faults'].append(Fault(
                    'SourceEnvelopeChanged' if envelope_fault else 'LineEndUnresolved',
                    detail, 'comparison',
                    row['document'], row['side'], location[0], location[1]).row)
        else:
            evidence['verdict'] = 'equal'
    except StageHalt:
        pass
    except Fault as error:
        search = error.row.pop('search', None)
        if search is not None and evidence['units']:
            occurrence = evidence['units'][0]['occurrences'][0]
            occurrence.setdefault('search', {})[error.row['side']] = search
        evidence['faults'].append(error.row)
    except Exception as error:
        evidence['faults'].append(Fault('PreparationRefused', error, stage).row)
    evidence['inputs'].sort(key=lambda row: row['path'])
    evidence['faults'].sort(key=lambda row:
                            (STAGES[row['stage']], {'paper': 0, 'explainer': 1}.get(row['document'], 2),
                             {'before': 0, 'after': 1}.get(row['side'], 2),
                             row['page'] if isinstance(row['page'], int) else 1000000,
                             row['offset'] if row['offset'] is not None else -1,
                             row['code']))
    try:
        (output / 'comparison.json').write_bytes(json_bytes(evidence))
    except OSError as error:
        raise Fault('EvidenceUnavailable', error, 'final') from error
    for row in evidence['faults']:
        display = dict(row, page=row['page'] if row['page'] is not None else '-',
                       offset=row['offset'] if row['offset'] is not None else '-')
        print('M077: {code}: {document}/{side}: {page}:{offset}: {detail}'.format(**display),
              file=sys.stderr)
    return {'equal': 0, 'different': 1, 'refused': 2}[evidence['verdict']]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='action', required=True)
    dump = commands.add_parser('dump')
    dump.add_argument('--pdf', required=True, type=pathlib.Path)
    comparison = commands.add_parser('compare')
    comparison.add_argument('--repository', required=True, type=pathlib.Path)
    for side in ('before', 'after'):
        for document in ('paper', 'explainer'):
            comparison.add_argument('--' + side + '-' + document,
                                    required=True, type=pathlib.Path)
    comparison.add_argument('--before-source')
    comparison.add_argument('--before-revision')
    comparison.add_argument('--unit', required=True)
    comparison.add_argument('--output', required=True, type=pathlib.Path)
    args = parser.parse_args()
    try:
        if args.action == 'compare':
            return compare(args)
        pdf_libraries()
        pages = Pdf(args.pdf).raw_pages()
    except Fault as error:
        print('M077: ' + error.row['code'] + ': ' + str(error), file=sys.stderr)
        return 2
    except (OSError, RuntimeError, UnicodeError, ValueError, AttributeError) as error:
        print('M077: ExtractionUnavailable: ' + str(error), file=sys.stderr)
        return 2
    print(json.dumps(pages, ensure_ascii=False, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
