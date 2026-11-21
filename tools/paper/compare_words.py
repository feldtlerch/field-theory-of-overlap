"""Compare complete captured paper text and corroborate one native paragraph."""
import argparse
import ctypes
import ctypes.util
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import unicodedata

from identities import CapturedInputs, bindings, input_paths
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
    rendered = []
    semantic = []
    tags = []
    i = 0
    while i < len(body):
        match = TAG.match(body, i)
        if match:
            key = match[1]
            row = lookup.get(key)
            if row is None or row['source_status'] not in ('paper', 'alias'):
                raise Fault('SyntaxUnsupported', 'unchecked bare paper tag: ' + key, 'snapshot')
            spelling = row['spelling']
            tags.append(dict(key=key, spelling=spelling, extracted=spelling.upper(),
                             target=row['target']))
            rendered.append(spelling.upper())
            semantic.append(spelling)
            i = match.end()
            continue
        if body.startswith('----', i):
            raise Fault('SyntaxUnsupported', 'four or more hyphens are unsupported', 'snapshot')
        if body.startswith('---', i):
            rendered.append('—'); semantic.append('—'); i += 3; continue
        if body.startswith('--', i):
            raise Fault('SyntaxUnsupported', 'double hyphen is unsupported', 'snapshot')
        ch = body[i]
        if ch == "'":
            if not (i and i + 1 < len(body) and body[i - 1].isascii() and
                    body[i - 1].isalpha() and body[i + 1].isascii() and body[i + 1].isalpha()):
                raise Fault('SyntaxUnsupported', 'apostrophe is not between letters', 'snapshot')
            ch = '’'
        elif not ((ch.isascii() and (ch.isalnum() or ch in ' \t\r\n.,;:!?()/ -')) or
                  ch in '’—'):
            raise Fault('SyntaxUnsupported', 'unsupported source scalar U+%04X' % ord(ch), 'snapshot')
        rendered.append(ch); semantic.append(ch); i += 1
    return (' '.join(''.join(rendered).split()), ' '.join(''.join(semantic).split()), tags)


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
                    events.append(dict(operation='LineEndPunctuation',
                                       raw=locations[previous], normalized_offset=offset,
                                       spelling=text[previous],
                                       intervening_space=previous != position - 1))
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


def corroborate(expected, extracted, document, side):
    stream = extracted['stream']
    # A confirmed line-end hyphen may interrupt any longer opening prefix.
    prefix = expected[:1]
    starts = [match.start() for match in re.finditer(re.escape(prefix), stream)]
    matches = []
    hyphens = {row['normalized_offset']: row for row in extracted['events']
               if row['operation'] == 'HyphenUnconfirmed'}
    for start in starts:
        source_at = 0
        pdf_at = start
        joins = []
        while source_at < len(expected) and pdf_at < len(stream):
            if expected[source_at] == stream[pdf_at]:
                source_at += 1; pdf_at += 1; continue
            event = hyphens.get(pdf_at)
            if (event and stream[pdf_at] == '-' and 0 < source_at < len(expected) and
                    expected[source_at - 1].isascii() and expected[source_at - 1].isalpha() and
                    expected[source_at].isascii() and expected[source_at].isalpha()):
                source_begin = source_at - 1
                while source_begin and expected[source_begin - 1].isascii() and expected[source_begin - 1].isalpha():
                    source_begin -= 1
                source_end = source_at
                while source_end < len(expected) and expected[source_end].isascii() and expected[source_end].isalpha():
                    source_end += 1
                pdf_begin = pdf_at - 1
                while pdf_begin and stream[pdf_begin - 1].isascii() and stream[pdf_begin - 1].isalpha():
                    pdf_begin -= 1
                pdf_end = pdf_at + 1
                while pdf_end < len(stream) and stream[pdf_end].isascii() and stream[pdf_end].isalpha():
                    pdf_end += 1
                joins.append(dict(operation='HyphenConfirmed', raw=event['raw'],
                                  joined=expected[source_begin:source_end],
                                  extracted=stream[pdf_begin:pdf_end]))
                pdf_at += 1
                continue
            break
        if (source_at == len(expected) and (start == 0 or not stream[start - 1].isalnum()) and
                (pdf_at == len(stream) or not stream[pdf_at].isalnum())):
            matches.append(dict(range=[start, pdf_at], start=extracted['offsets'][start],
                                end=extracted['offsets'][pdf_at - 1], confirmed_joins=joins))
    if len(matches) != 1:
        raise Fault('CorrespondenceInvalid', 'decoded paragraph matches %d times' % len(matches),
                    'correspondence', document, side)
    return matches[0]


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
    evidence = dict(schema=1, verdict='refused', faults=[], differences=[],
                    source=None, inputs=[], extractor=None, tags=[], documents={},
                    correspondence=None, unsupported_locator=True)
    originals = {}
    copies = {}
    stage = 'snapshot'
    try:
        source = source_blob(repo, args.before_source)
        evidence['source'] = {key: value for key, value in source.items()
                              if key not in ('body', 'blob_bytes')}
        try:
            (output / 'source.blob').write_bytes(source['blob_bytes'])
        except OSError as error:
            raise Fault('EvidenceUnavailable', error, 'snapshot') from error
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
                    historical = git(repo, 'show', source['commit'] +
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
        rows = (snapshot / 'documents/paper-build/sources.tsv').read_text().splitlines()
        declared_fragments(snapshot, rows)
        selected = [(src, fragment) for src, unit, fragment, role, _ in scope_fragments(snapshot)
                    if unit == args.unit and role == 'paragraph']
        if len(selected) != 1:
            raise Fault('UnitInvalid', 'declared paragraph unit is absent or ambiguous', 'snapshot')
        src, fragment_path = selected[0]
        fragment = (snapshot / fragment_path).read_bytes()
        prefix = ('\\PaperNativeParagraph{' + args.unit + '}{').encode()
        if not fragment.startswith(prefix) or not fragment.endswith(b'}\n'):
            raise Fault('UnitInvalid', 'captured fragment differs from declaration', 'snapshot')
        lookup = bindings(snapshot)
        # The binding check already refuses case-insensitive printed-tag collisions.
        before, before_semantic, before_tags = decode_paragraph(source['body'], lookup)
        after, after_semantic, after_tags = decode_paragraph(
            fragment[len(prefix):-2].decode('utf-8'), lookup)
        tags = {(row['key'], row['spelling'], row['target']) for row in before_tags + after_tags}
        evidence['tags'] = [dict(key=key, spelling=spelling, target=target)
                            for key, spelling, target in sorted(tags)]
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
        evidence['correspondence'] = dict(
            before=corroborate(before, paper['before'], 'paper', 'before'),
            after=corroborate(after, paper['after'], 'paper', 'after'))
        if before_semantic != after_semantic:
            evidence['differences'].append(dict(code='TextDifference', part='source/body',
                                                before=before_semantic, after=after_semantic))
        stage = 'comparison'
        for document in ('paper', 'explainer'):
            before_pdf = evidence['documents'][document]['before']
            after_pdf = evidence['documents'][document]['after']
            if before_pdf['stream'] == after_pdf['stream']:
                def event_positions(pdf, operation):
                    return [(row['normalized_offset'], row.get('spelling'),
                             row.get('intervening_space')) for row in pdf['events']
                            if row['operation'] == operation and 'outline' not in row]
                for operation in ('HyphenUnconfirmed', 'LineEndPunctuation'):
                    if event_positions(before_pdf, operation) != event_positions(after_pdf, operation):
                        raise Fault('HyphenUnconfirmed' if operation == 'HyphenUnconfirmed'
                                    else 'ProfileUnsupported',
                                    ('line-end hyphen differs despite equal text'
                                     if operation == 'HyphenUnconfirmed'
                                     else 'line-end mark or following space differs despite equal text'),
                                    'comparison', document)
            for field in ('stream', 'titles'):
                if before_pdf[field] != after_pdf[field]:
                    if field == 'stream':
                        left, right = before_pdf[field], after_pdf[field]
                        offset = next((i for i, (a, b) in enumerate(zip(left, right)) if a != b),
                                      min(len(left), len(right)))
                        location = dict(before=before_pdf['offsets'][offset]
                                        if offset < len(left) else None,
                                        after=after_pdf['offsets'][offset]
                                        if offset < len(right) else None)
                        def without_unconfirmed(pdf):
                            excluded = {event['normalized_offset'] for event in pdf['events']
                                        if event['operation'] == 'HyphenUnconfirmed'}
                            return ''.join(ch for i, ch in enumerate(pdf['stream']) if i not in excluded)
                        if without_unconfirmed(before_pdf) == without_unconfirmed(after_pdf):
                            raise Fault('HyphenUnconfirmed',
                                        'equality would require an unconfirmed hyphen deletion',
                                        'comparison', document)
                    else:
                        offset = next((i for i, (a, b) in enumerate(zip(before_pdf[field],
                                                                         after_pdf[field])) if a != b),
                                      min(len(before_pdf[field]), len(after_pdf[field])))
                        location = dict(outline=offset + 1)
                    evidence['differences'].append(dict(
                        code='TextDifference', document=document, part=field,
                        location=location,
                        before=before_pdf[field], after=after_pdf[field]))
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
        if extractor_identity() != evidence['extractor']:
            raise Fault('InputChanged', 'extractor libraries changed', 'final')
        evidence['verdict'] = 'different' if evidence['differences'] else 'equal'
    except StageHalt:
        pass
    except Fault as error:
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
    comparison.add_argument('--before-source', required=True)
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
