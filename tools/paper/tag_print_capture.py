"""Capture shipped paper tag positions and their complete PDF text atoms.

The disposable typeset uses the paper's prepared inputs and a position-only
wrapper around ftag. Selection and source-to-prepared locations belong to the
consumer; this module binds each selected occurrence to its own shipped rows.
The selection file is an ordered list of source, site, command, star and key;
--write-fixture stores a fresh capture, and --check compares one with that form.
"""
import argparse
import collections
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import time

from common import read, require, runtime
from identities import bindings
from ledger import auxiliary, check_destination
from pdf_text import Pdf
from prepare import prepare

HOOK = pathlib.Path(__file__).parent / 'fixtures/tag-print-hook.tex'
SP_TO_BP = 72 / (72.27 * 65536)
FIELDS = ('document', 'page', 'kind', 'key', 'spelling', 'status', 'site')


def positions(text):
    """Pair the delayed start/end records by invocation, not by key or line."""
    pairs = {}
    for line in text.splitlines():
        fields = line.split('|')
        require(len(fields) == 8 and fields[0] in ('S', 'E') and
                fields[1].isdecimal() and fields[2] in ('0', '1') and
                fields[5].isdecimal() and all(x.lstrip('-').isdecimal() for x in fields[6:]),
                'Tag position row malformed: ' + line)
        marker, ordinal, star, key, site, page, x, y = fields
        require(key and site and int(page) > 0, 'Tag position row incomplete: ' + line)
        pair = pairs.setdefault(int(ordinal), {})
        require(marker not in pair, 'Tag position row duplicated: ' + line)
        pair[marker] = dict(ordinal=int(ordinal), star=bool(int(star)), key=key,
                            site=site, page=int(page), x=int(x), y=int(y))
    require(pairs, 'Tag position output missing')
    result = []
    for ordinal, pair in sorted(pairs.items()):
        require(set(pair) == {'S', 'E'}, 'Tag position end missing: ' + str(ordinal))
        start, end = pair['S'], pair['E']
        require(all(start[name] == end[name] for name in
                    ('ordinal', 'star', 'key', 'site', 'page')),
                'Tag position pair differs: ' + str(ordinal))
        result.append(dict(start, end_x=end['x'], end_y=end['y']))
    return result


def shipped_rows(text):
    rows = []
    for line in text.splitlines():
        fields = [value.strip() for value in line.split('|')]
        require(len(fields) == len(FIELDS), 'Tag ledger row malformed: ' + line)
        rows.append(dict(zip(FIELDS, fields)))
    return rows


def printed_atom(page, start, end, spelling):
    """Recover the complete text bounded by the two shipped positions.

    The identity spelling selects the first line. Shipped positions bound the
    atom on one line or its handle and locator on two adjacent lines. An
    ambiguous extraction refuses.
    """
    x0, x1 = start['x'] * SP_TO_BP, end['x'] * SP_TO_BP

    def lines(left, right, saved_y):
        top = page['height'] - saved_y * SP_TO_BP
        selected = []
        for index, (glyph, rect) in enumerate(zip(page['text'], page['rectangles'])):
            if glyph == '\n' or rect[0] < left - 0.7 or rect[2] > right + 0.7:
                continue
            if top - 20 < rect[1] < top + 7:
                selected.append((index, glyph, rect))
        groups = []
        for item in sorted(selected, key=lambda entry: entry[2][1]):
            if not groups or item[2][1] - groups[-1][0][2][1] >= 6:
                groups.append([])
            groups[-1].append(item)
        result = []
        for chars in groups:
            chars.sort()
            atom = ''
            for previous, (_, glyph, rect) in zip([None] + chars[:-1], chars):
                if previous is not None and rect[0] - previous[2][2] > 2.0:
                    atom += ' '
                atom += glyph
            if chars:
                result.append((abs(top - chars[0][2][1]), atom, chars))
        return sorted(result)

    if start['y'] == start['end_y']:
        require(x0 < x1, 'Tag same-line position order differs: ' + start['site'])
        candidates = [line for line in lines(x0, x1, start['end_y'])
                      if spelling.casefold() in line[1].casefold()]
        require(candidates, 'Tag printed atom absent: ' + start['site'] + ':' + start['key'])
        require(len(candidates) == 1 or candidates[1][0] - candidates[0][0] > 6,
                'Tag printed atom ambiguous: ' + start['site'] + ':' + start['key'])
        return candidates[0][1]

    first = [line for line in lines(x0, page['width'], start['y'])
             if spelling.casefold() in line[1].casefold()]
    last = [line for line in lines(0, x1, start['end_y'])
            if abs(line[2][-1][2][2] - x1) < 1.5]
    require(len(first) == 1 and len(last) == 1 and first[0][2][0][0] < last[0][2][0][0],
            'Tag printed atom ambiguous across lines: ' + start['site'] + ':' + start['key'])
    return first[0][1].strip() + ' ' + last[0][1].strip()


def bind(expected, position_rows, identity_rows, lookup, targets, pdf):
    """Require an exact one-to-one association within every selected site."""
    by_site = collections.defaultdict(list)
    for row in position_rows:
        by_site[row['site']].append(row)
    want = collections.defaultdict(list)
    for row in expected:
        want[row['site']].append(row)
    require(set(by_site) == set(want), 'Tag position site inventory differs')
    pages = pdf.raw_layout_pages()
    output = []
    for site, requests in want.items():
        positions_here = by_site[site]
        require(len(positions_here) == len(requests), 'Tag position inventory differs: ' + site)
        evidence = [row for row in identity_rows if row['document'] == 'main' and
                    row['site'] == site and row['kind'] in ('identity', 'locator')]
        require(sum(row['kind'] == 'identity' for row in evidence) == len(requests),
                'Tag identity inventory differs: ' + site)
        cursor = 0
        for request, position in zip(requests, positions_here):
            key = request['key']
            require(request['command'] == 'ftag',
                    'Tag command family differs: ' + site + ':' + key)
            require((position['key'], position['star']) == (key, request['star']),
                    'Tag source/position association differs: ' + site)
            require(key in lookup and key in targets, 'Tag resolution absent: ' + key)
            require(cursor < len(evidence) and evidence[cursor]['kind'] == 'identity' and
                    evidence[cursor]['key'] == key and
                    evidence[cursor]['spelling'] == lookup[key]['spelling'] and
                    evidence[cursor]['status'] == lookup[key]['status'] and
                    evidence[cursor]['page'] == str(position['page']),
                    'Tag identity association differs: ' + site + ':' + key +
                    ':occurrence ' + str(position['ordinal']))
            cursor += 1
            located = cursor < len(evidence) and evidence[cursor]['kind'] == 'locator'
            if located:
                require(evidence[cursor]['key'] == key and
                        evidence[cursor]['status'] == lookup[key]['status'] and
                        evidence[cursor]['page'] == str(position['page']),
                        'Tag locator association differs: ' + site + ':' + key +
                        ':occurrence ' + str(position['ordinal']))
                cursor += 1
            target = targets[key]
            require(target['page'] and target['anchor'], 'Tag target incomplete: ' + key)
            check_destination('main', key, target['anchor'], int(target['page']),
                              pdf.destination(target['anchor']))
            atom = printed_atom(pages[position['page'] - 1], position,
                                dict(x=position['end_x']), lookup[key]['spelling'])
            require(atom[:len(lookup[key]['spelling'])].casefold() ==
                    lookup[key]['spelling'].casefold(),
                    'Tag printed atom prefix differs: ' + site + ':' + key +
                    ':occurrence ' + str(position['ordinal']))
            tail = atom[len(lookup[key]['spelling']):]
            require((re.fullmatch(r'\s+\([^()]+\)', tail) is not None) if located
                    else not tail,
                    'Tag printed mode/atom differs: ' + site + ':' + key +
                    ':occurrence ' + str(position['ordinal']))
            output.append(dict(site=site, ordinal=position['ordinal'], command='ftag',
                               star=position['star'], key=key, page=position['page'],
                               spelling=lookup[key]['spelling'],
                               mode='located' if located else 'bare',
                               atom=atom, atom_lines=2 if position['y'] != position['end_y'] else 1,
                               target_anchor=target['anchor'],
                               target_page=int(target['page'])))
        require(cursor == len(evidence), 'Tag ledger inventory differs: ' + site)
    require([row['ordinal'] for row in output] ==
            sorted(row['ordinal'] for row in output), 'Tag expected occurrence order differs')
    return output


def compare_atoms(before, after):
    """Compare selected semantic and printed outcomes in occurrence order."""
    require(len(before) == len(after), 'Tag occurrence inventory differs')
    for index, (left, right) in enumerate(zip(before, after)):
        for name in ('command', 'star', 'key', 'spelling', 'mode', 'atom',
                     'target_anchor'):
            require(left[name] == right[name],
                    'Tag ' + name + ' differs at occurrence ' + str(index))


def source_line(repository, request):
    """Read the selected source line and require its exact tag command."""
    source = request['source']
    match = re.fullmatch(r'(documents/paper/parts/[A-Za-z0-9_.-]+\.tex):([1-9][0-9]*)',
                         source)
    require(match is not None, 'Tag source location malformed: ' + source)
    lines = (repository / match.group(1)).read_text(encoding='utf-8').splitlines()
    index = int(match.group(2)) - 1
    require(index < len(lines), 'Tag source line absent: ' + source)
    line = lines[index]
    command = r'\ftag' + ('*' if request['star'] else '') + '{' + request['key'] + '}'
    require(command in line, 'Tag source command absent: ' + source + ':' + request['key'])
    return hashlib.sha256(line.encode('utf-8')).hexdigest()


def fixture_records(repository, selection, captured):
    """Attach source and product provenance to a fresh observed capture."""
    require(len(selection) == len(captured), 'Tag fixture selection inventory differs')
    records = []
    for request, observed in zip(selection, captured):
        for name in ('site', 'command', 'star', 'key'):
            require(request[name] == observed[name],
                    'Tag fixture selection ' + name + ' differs: ' + request['source'])
        records.append(dict(source=request['source'],
                            source_line_sha256=source_line(repository, request), **observed))
    pdf = repository / 'generated/paper/paper.pdf'
    return {'schema': 2, 'paper_pdf_sha256': hashlib.sha256(pdf.read_bytes()).hexdigest(),
            'records': records}


def validate_fixture(repository, selection, fixture):
    """Refuse stale source, selection, or committed PDF evidence before typesetting."""
    require(fixture.get('schema') == 2 and isinstance(fixture.get('records'), list),
            'Tag fixture schema differs')
    pdf = repository / 'generated/paper/paper.pdf'
    require(fixture.get('paper_pdf_sha256') == hashlib.sha256(pdf.read_bytes()).hexdigest(),
            'Tag fixture paper PDF differs')
    require(len(selection) == len(fixture['records']), 'Tag fixture inventory differs')
    for index, (request, record) in enumerate(zip(selection, fixture['records'])):
        require(record.get('source') == request['source'],
                'Tag fixture source differs at occurrence ' + str(index))
        for name in ('site', 'command', 'star', 'key'):
            require(record.get(name) == request[name],
                    'Tag fixture ' + name + ' differs at occurrence ' + str(index))
        require(record.get('source_line_sha256') == source_line(repository, request),
                'Tag fixture source line differs at occurrence ' + str(index))


def compare_fixture(stored, fresh):
    """Compare every retained field of each freshly captured occurrence."""
    require(stored.keys() == fresh.keys(), 'Tag fixture fields differ')
    require(stored['schema'] == fresh['schema'] and
            stored['paper_pdf_sha256'] == fresh['paper_pdf_sha256'],
            'Tag fixture provenance differs')
    require(len(stored['records']) == len(fresh['records']), 'Tag fixture inventory differs')
    for index, (left, right) in enumerate(zip(stored['records'], fresh['records'])):
        require(left.keys() == right.keys(),
                'Tag fixture record fields differ at occurrence ' + str(index))
        for name in left:
            require(left[name] == right[name],
                    'Tag fixture ' + name + ' differs at occurrence ' + str(index))


def compare_pdf_layout(plain_pdf, tapped_pdf):
    """Require the same extracted ink and glyph positions on every page."""
    plain = Pdf(plain_pdf).raw_layout_pages()
    tapped = Pdf(tapped_pdf).raw_layout_pages()
    require(len(plain) == len(tapped), 'Tag tap changed PDF page count')
    for index, (left, right) in enumerate(zip(plain, tapped), 1):
        require(left['text'] == right['text'] and left['rectangles'] == right['rectangles'],
                'Tag tap changed PDF text or geometry: page ' + str(index))


def tap_for_sites(expected):
    """Expand only checked source sites into the disposable reader hook."""
    sites = {row['site'] for row in expected}
    require(sites and all(re.fullmatch(r'[A-Za-z0-9_./-]+\.tex:[1-9][0-9]*', site) and
                          all(part not in ('', '.', '..')
                              for part in site.split(':')[0].split('/'))
                          for site in sites),
            'Tag capture site inventory malformed')
    template = HOOK.read_text(encoding='utf-8')
    require(template.count('% SELECTED_SITES') == 1, 'Tag capture tap template differs')
    selected = '\n'.join(r'\prop_gput:Nnn \g_paper_tag_selected_prop {' + site + '} {1}'
                         for site in sorted(sites))
    return template.replace('% SELECTED_SITES', selected)


def typeset(repository, support, work, expected):
    """Prepare and typeset once in a fresh area, tapping selected tag sites."""
    require(not work.exists(), 'Tag capture work area must be new')
    paper = work / 'paper'
    prepare(repository, support, paper)
    hook = paper / 'reader-hooks.tex'
    hook.write_text(hook.read_text(encoding='utf-8') + '\n' +
                    tap_for_sites(expected), encoding='utf-8')
    settings, _, engine, bundle = runtime(repository, support)
    environment = dict(settings['environment'])
    environment.update(SOURCE_DATE_EPOCH=str(settings['source-date-epoch']),
                       FORCE_SOURCE_DATE=settings['force-source-date'])
    for key, value in settings['work-environment'].items():
        path = work / value
        path.mkdir(parents=True, exist_ok=True)
        environment[key] = str(path)
    command = [str(engine), '--bundle', str(bundle), *settings['flags'], 'main.tex']
    with (work / 'engine.txt').open('wb') as output:
        result = subprocess.run(command, cwd=paper, env=environment,
                                stdout=output, stderr=subprocess.STDOUT,
                                timeout=settings['invocation-timeout'])
    require(result.returncode == 0 and (paper / 'main.pdf').is_file() and
            (paper / 'main.tag-positions').is_file(), 'Tag capture typeset failed')
    require(re.search(r'Output written on main\.xdv \(\d+ pages, \d+ bytes\)\.\s*\Z',
                      read(paper / 'main.log')), 'Tag capture typeset incomplete')
    return paper


def main():
    from checking_capture import cgroup_snapshot, resources

    start = time.monotonic()
    try:
        cgroup_line = pathlib.Path('/proc/self/cgroup').read_text(encoding='utf-8').strip()
        cgroup_line = cgroup_line.split(':')[-1]
    except OSError:
        cgroup_line = 'unavailable'
    cgroup = pathlib.Path('/sys/fs/cgroup') / cgroup_line.lstrip('/')
    before = cgroup_snapshot(cgroup)
    try:
        parser = argparse.ArgumentParser(description=__doc__)
        parser.add_argument('repository', type=pathlib.Path)
        parser.add_argument('support', type=pathlib.Path)
        parser.add_argument('work', type=pathlib.Path)
        parser.add_argument('selection', type=pathlib.Path)
        parser.add_argument('--fixture', type=pathlib.Path)
        parser.add_argument('--plain-pdf', type=pathlib.Path)
        mode = parser.add_mutually_exclusive_group()
        mode.add_argument('--write-fixture', action='store_true')
        mode.add_argument('--check', action='store_true')
        args = parser.parse_args()
        require(not (args.write_fixture or args.check) or args.fixture is not None,
                'Tag fixture path required')
        selection = json.loads(args.selection.read_text(encoding='utf-8'))
        stored = None
        if args.check:
            stored = json.loads(args.fixture.read_text(encoding='utf-8'))
            validate_fixture(args.repository, selection, stored)
        paper = typeset(args.repository, args.support, args.work, selection)
        if args.plain_pdf is not None:
            compare_pdf_layout(args.plain_pdf, paper / 'main.pdf')
        position_rows = positions(read(paper / 'main.tag-positions'))
        identity_rows = shipped_rows(read(paper / 'main.identities'))
        lookup = bindings(args.repository)
        targets = auxiliary(paper, 'main')
        result = bind(selection, position_rows, identity_rows, lookup, targets,
                      Pdf(paper / 'main.pdf'))
        if args.write_fixture or args.check:
            product = bind(selection, position_rows, identity_rows, lookup, targets,
                           Pdf(args.repository / 'generated/paper/paper.pdf'))
            compare_atoms(result, product)
            fresh = fixture_records(args.repository, selection, result)
            if args.write_fixture:
                args.fixture.write_text(json.dumps(fresh, ensure_ascii=False, indent=2) + '\n',
                                        encoding='utf-8')
            else:
                compare_fixture(stored, fresh)
                print('Tag fixture matches live capture')
        else:
            print(json.dumps(result, ensure_ascii=False, indent=2))
    finally:
        print(json.dumps({'resources': resources(start, cgroup_line, before,
                                                 cgroup_snapshot(cgroup))}, sort_keys=True),
              file=sys.stderr)


if __name__ == '__main__':
    main()
