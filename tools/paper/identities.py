"""Validate the paper's tag lookup and emit the review lookup, never admissions."""
import argparse
import hashlib
import json
import pathlib
import re
import sys
import tomllib
import bibroute
from common import read, require, timed, write

INPUTS = ['tools/paper/identities.py', 'documents/paper-build/paper-tags.tsv',
          'documents/paper-build/reader-tags.toml', 'documents/paper-build/provisional-tags.toml',
          'documents/paper-build/reader-hooks.tex', 'documents/paper/data/ftoo_manifest.tex']
CHECK_INPUTS = ['tools/paper/common.py', 'documents/paper-build/label-mints.tsv',
                'documents/paper-build/sources.tsv', 'generated/manifest.json',
                'registry/documentary-tags.toml']
TAG = re.compile(r'[A-Za-z][A-Za-z0-9_-]*:[A-Za-z0-9][A-Za-z0-9_-]*\Z')
KEY = re.compile(r'[A-Za-z][A-Za-z0-9_-]*(?::[A-Za-z0-9_-]+)+\Z')
# A valid key is a whole token exactly when holds() would match it on a source line.
LABEL_TOKEN = re.compile(r'[A-Za-z0-9_:-]+')


def holds(line, key):
    """A line holds a validated key when it occurs as one whole label token."""
    return key in LABEL_TOKEN.findall(line)


class CapturedInputs:
    """Keep validation reads and their reported digests on one byte snapshot."""
    def __init__(self, repo):
        self.repo = repo
        self.bytes = {}

    def read_bytes(self, path):
        name = path.relative_to(self.repo).as_posix()
        if name not in self.bytes:
            self.bytes[name] = path.read_bytes()
        return self.bytes[name]

    def read(self, path):
        return self.read_bytes(path).decode('utf-8')

    def verify(self):
        for name, data in self.bytes.items():
            require((self.repo/name).read_bytes() == data,
                    'M063: checked input changed: '+name)


def input_paths(repo, captured=None):
    """Enumerate every declared validity read without adding PDF byte inputs."""
    read_text = captured.read if captured else read
    sources = read_text(repo/'documents/paper-build/sources.tsv').splitlines()
    native = native_target_requests(
        (path.relative_to(repo).as_posix()
         for path in (repo/'units/statements').glob('*.toml')),
        lambda file: read_text(repo/file))
    results = result_header_requests(
        (path.relative_to(repo).as_posix()
         for path in (repo/'units/corpus').glob('*.toml')),
        lambda file: read_text(repo/file))
    return sorted(set(INPUTS + CHECK_INPUTS + bibroute.INPUTS + sources) |
                  {file for file, _, _ in native.values()} |
                  {file for file, _ in results.values()})


def native_target_requests(files, read_text):
    """Select only the unit sources that request documentary native targets."""
    targets = {}
    for file in sorted(files):
        unit = tomllib.loads(read_text(file))['unit']
        if 'econ' not in unit:
            continue
        selector = unit['econ']['target']
        key = unit.get('documentary-targets', {}).get(selector)
        require(key is not None, 'M063: native target selector absent: '+selector)
        require(key not in targets, 'M063: native target has two unit owners: '+key)
        targets[key] = (file, unit['id'], selector)
    return targets


def result_header_requests(files, read_text):
    """Select the checked corpus sources that request a registered result header."""
    targets = {}
    for file in sorted(files):
        unit = tomllib.loads(read_text(file))['unit']
        results = [block for block in unit['blocks'] if block['kind'] == 'result']
        if not results:
            continue
        require(len(results) == len(unit['blocks']) == 1 and
                unit['document'] == 'paper-pilot' and unit['composition'] == 'corpus-slot' and
                unit['language'] == 'en', 'M063: result header owner differs: '+file)
        key = results[0]['key']
        selector = results[0].get('target')
        declarations = unit.get('documentary-targets', {})
        require(isinstance(selector, str) and len(declarations) == 1 and
                declarations.get(selector) == key,
                'M063: result header selector differs: '+file)
        require(KEY.fullmatch(key) and key not in targets,
                'M063: duplicate or malformed result header key: '+key)
        targets[key] = (file, unit['id'])
    return targets


def label_mints(repo, read_text=read):
    numbered = [(number, line.split('\t')) for number, line in
                enumerate(read_text(repo/'documents/paper-build/label-mints.tsv').splitlines(), 1)
                if not line.startswith('#')]
    for number, row in numbered:
        require(len(row) == 2, 'M063: malformed label mint row '+str(number))
        require(KEY.fullmatch(row[0]), 'M063: malformed label mint key: '+row[0]+' at row '+str(number))
        require(re.fullmatch(r'documents/paper/[A-Za-z0-9_./-]+\.tex', row[1]) and
                '..' not in row[1].split('/'), 'M063: malformed label mint row '+str(number))
    rows = [row for _, row in numbered]
    seen = set()
    for key, _ in rows:
        require(key not in seen, 'M063: duplicate label mint key: '+key)
        seen.add(key)
    declared = set(read_text(repo/'documents/paper-build/sources.tsv').splitlines())
    sources = {}
    for key, file in rows:
        if file not in sources:
            path = repo/file
            require(path.is_file(), 'M063: label mint file is absent: '+file)
            require(file in declared, 'M063: label mint file is not declared: '+file)
            sources[file] = set(LABEL_TOKEN.findall(read_text(path)))
        require(key in sources[file],
                'M063: label mint file does not hold its key: '+key+' at '+file)
    return rows


def bindings(repo, release=False, captured=None):
    root = repo/'documents/paper-build'
    read_text = captured.read if captured else read
    paper_bytes = captured.read_bytes(root/'paper-tags.tsv') if captured else (root/'paper-tags.tsv').read_bytes()
    label_mints(repo, read_text)
    paper = {}
    for line in paper_bytes.decode('utf-8').splitlines():
        if line.startswith('#'): continue
        fields = line.split('\t')
        require(len(fields) == 3, 'M063: malformed paper tag row')
        tag, target, kind = fields
        require(TAG.fullmatch(tag) and KEY.fullmatch(target), 'M063: malformed paper tag pair')
        require(tag not in paper and kind in ('node','evidence'), 'M063: duplicate paper tag binding')
        paper[tag] = target
    source = read_text(repo/'documents/paper/data/ftoo_manifest.tex')
    pattern = re.compile(r'\\prop_gput:Nnn \\g_ftoo_(evidence_)?tag_to_id_prop \{ ([^{}]+) \} \{ ([^{}]+) \}')
    declared = {}
    for evidence, tag, target in pattern.findall(source):
        kind = 'evidence' if evidence else 'node'
        pair = (target, kind)
        require(tag not in declared or declared[tag] == pair,
                'M063: paper tag source has conflicting targets: '+tag)
        declared[tag] = pair
    actual = {}
    for line in paper_bytes.decode('utf-8').splitlines():
        if line.startswith('#'): continue
        tag, target, kind = line.split('\t')
        actual[tag] = (target, kind)
    missing = sorted(declared.keys() - actual.keys())
    extra = sorted(actual.keys() - declared.keys())
    changed = sorted(tag for tag in declared.keys() & actual.keys() if declared[tag] != actual[tag])
    require(not (missing or extra or changed),
            'M063: paper tag source coverage differs: missing='+','.join(missing)+
            ' extra='+','.join(extra)+' changed='+','.join(changed))
    require(len({tag.casefold() for tag in paper}) == len(paper), 'M063: duplicate paper tag spelling')
    require(len(set(paper.values()))==len(paper), 'M063: paper tag target has two spellings')
    reserved = set(); reserved_targets=set()
    def collect(value):
        if isinstance(value, dict):
            for k,v in value.items():
                if k in ('tag','handle') and isinstance(v,str): reserved.add(v.casefold())
                if k in ('identity','label','id') and isinstance(v,str):reserved_targets.add(v)
                collect(v)
        elif isinstance(value,list):
            for item in value: collect(item)
    collect(json.loads(read_text(repo/'generated/manifest.json')))
    theory_reserved = reserved.copy()
    documentary = tomllib.loads(read_text(repo/'registry/documentary-tags.toml'))
    handles = {event['handle'] for event in documentary.get('events', [])}
    require(len({handle.casefold() for handle in handles}) == len(handles), 'M063: case-insensitive registry collision')
    require(not {handle.casefold() for handle in handles} & theory_reserved, 'M063: documentary spelling collides with theory')
    collect(documentary)
    require(not set(paper.values()) & reserved_targets, 'M063: paper tag target collides with a permanent identity')
    table = tomllib.loads(read_text(root/'reader-tags.toml'))
    candidates = tomllib.loads(read_text(root/'provisional-tags.toml'))['candidates']
    result = {}; spellings = {}
    admissions={event['handle']:event for event in documentary.get('events',[]) if event['change']=='admit'}
    retired={event['handle'] for event in documentary.get('events',[]) if event['change']=='retire'}
    for status, rows in [('paper',[dict(key=tag, spelling=tag) for tag in paper]), ('alias',table['aliases']), ('admitted',table.get('admissions',[])), ('candidate',candidates)]:
        for row in rows:
            key, spelling = row['key'], row['spelling']
            require(KEY.fullmatch(key), 'M063: malformed source key: '+key)
            require(key not in result, 'M063: key in two lists: '+key)
            require(TAG.fullmatch(spelling), 'M064: not a two-part spelling: '+spelling)
            if status != 'admitted':
                require(spelling.casefold() not in reserved, 'M063: reserved registry collision: '+spelling)
                require(key not in reserved_targets, 'M063: unadmitted key already belongs to a permanent binding: '+key)
            if status == 'paper':
                require(key == spelling and spelling in paper, 'M063: paper tag lookup rebound: '+key)
            elif status == 'alias':
                require(spelling in paper, 'M063: alias lacks paper tag target: '+key)
                expected = {'dyn:exact':'dyn:stat','dyn:response':'dyn:resp','cor:tremble:curve':'dyn:tcurve'}
                require(expected.get(key) == spelling, 'M063: unevidenced alias: '+key)
            elif status == 'admitted':
                event=admissions.get(spelling,{})
                require(spelling not in retired and event.get('label')==key and row.get('confirmation')==event.get('authority-record'), 'M063: admission lacks exact confirmed binding: '+key)
            else:
                require(row['status'] in ('provisional','as-printed'), 'M063: invalid candidate status')
                require(row.get('definition') and row.get('sites') and TAG.fullmatch(row['alternative']), 'M063: incomplete proposal: '+key)
                require(not release, 'M064: release refuses candidate: '+key)
            if status != 'alias':
                require(spelling.casefold() not in spellings, 'M063: spelling collision: '+spelling)
                spellings[spelling.casefold()] = key
            result[key] = dict(row, status='paper' if status in ('paper','alias') else 'admitted' if status=='admitted' else row['status'], target=paper.get(spelling,key), source_status=status)
    require(set(paper) == {k for k,v in result.items() if k == v['spelling'] and v['status']=='paper'}, 'M063: paper tag coverage differs')
    return result


def _site(file, line):
    return {'file': file, 'line': line}


def _assignment_line(text, key, file):
    pattern = re.compile(r'^[ \t]*key[ \t]*=[ \t]*"'+re.escape(key)+r'"[ \t]*$', re.M)
    found = list(pattern.finditer(text))
    require(len(found) == 1, 'M063: key evidence differs: '+key+' at '+file)
    return _site(file, text.count('\n', 0, found[0].start()) + 1)


def virtue_roster(repo, rows, captured):
    """Bind source-declared virtue slugs to canonical paper tags on one captured input."""
    file = 'documents/paper/data/ftoo_manifest.tex'
    source = captured.read(repo/file)
    facts = {}
    pattern = re.compile(r'^\\prop_gput:Nnn \\g_ftoo_id_to_(class|slug)_prop \{ ([^{}]+) \} \{ ([^{}]+) \}$', re.M)
    for match in pattern.finditer(source):
        kind, target, value = match.groups()
        declared = facts.setdefault(target, {})
        require(kind not in declared, 'M063: repeated '+kind+' declaration: '+target)
        declared[kind] = (value, _site(file, source.count('\n', 0, match.start()) + 1))
    roster = []
    words = set()
    paper_tags = captured.read(repo/'documents/paper-build/paper-tags.tsv').splitlines()
    paper = {fields[0]: (fields[1], fields[2]) for line in paper_tags
             if not line.startswith('#') for fields in [line.split('\t')]}
    for key, (target, kind) in paper.items():
        if not key.startswith('v:'):
            continue
        declared = facts.get(target, {})
        require(kind == 'node' and declared.get('class', (None,))[0] == 'virtue-weld' and
                declared.get('slug', (None,))[0] == key.removeprefix('v:'),
                'M063: virtue class or slug differs: '+key)
    for target, declared in facts.items():
        if declared.get('class', (None,))[0] != 'virtue-weld':
            continue
        require('slug' in declared, 'M063: virtue lacks slug: '+target)
        word, slug_site = declared['slug']
        require(bool(re.fullmatch(r'[a-z]+', word)), 'M063: invalid virtue slug: '+target)
        require(word not in words, 'M063: duplicate virtue slug: '+word)
        words.add(word)
        key = 'v:'+word
        require(paper.get(key) == (target, 'node'), 'M063: virtue paper binding differs: '+key)
        binding = rows.get(key)
        require(binding is not None and binding['source_status'] == 'paper' and
                binding['target'] == target, 'M063: virtue lookup differs: '+key)
        roster.append({'word': word, 'key': key, 'target': target,
                       'class_site': declared['class'][1], 'slug_site': slug_site})
    require(bool(roster), 'M063: virtue roster is empty')
    return sorted(roster, key=lambda row: row['word'])


def assembly_json(repo):
    """Serialize checked bindings and the captured validity closure for assembly."""
    captured = CapturedInputs(repo)
    rows = bindings(repo, captured=captured)
    paths = input_paths(repo, captured)
    for path in paths:
        captured.read_bytes(repo/path)
    virtues = virtue_roster(repo, rows, captured)
    paper_file = 'documents/paper-build/paper-tags.tsv'
    source_file = 'documents/paper/data/ftoo_manifest.tex'
    paper_lines = {}
    for number, line in enumerate(captured.read(repo/paper_file).splitlines(), 1):
        if not line.startswith('#'):
            paper_lines[line.split('\t')[0]] = number
    declarations = {}
    source = captured.read(repo/source_file)
    pattern = re.compile(r'\\prop_gput:Nnn \\g_ftoo_(evidence_)?tag_to_id_prop \{ ([^{}]+) \} \{ ([^{}]+) \}')
    for match in pattern.finditer(source):
        declarations.setdefault(match[2], []).append(_site(source_file,
                                    source.count('\n', 0, match.start()) + 1))
    alias_file = 'documents/paper-build/reader-tags.toml'
    candidate_file = 'documents/paper-build/provisional-tags.toml'
    wire = []
    for key, row in sorted(rows.items()):
        status = row['source_status']
        spelling = row['spelling']
        canonical = spelling if status == 'alias' else key
        if status in ('paper', 'alias'):
            target = row['target']
            kind = next(line.split('\t')[2] for line in captured.read(repo/paper_file).splitlines()
                        if not line.startswith('#') and line.split('\t')[0] == canonical)
            lookup = _site(paper_file, paper_lines[canonical])
            alias = _assignment_line(captured.read(repo/alias_file), key, alias_file) if status == 'alias' else None
            evidence = sorted(declarations[canonical], key=lambda site: (site['file'], site['line']))
        else:
            target = key if status == 'admitted' else None
            kind = 'documentary' if status == 'admitted' else None
            file = alias_file if status == 'admitted' else candidate_file
            lookup = _assignment_line(captured.read(repo/file), key, file)
            alias = None
            evidence = []
        wire.append({'key': key, 'spelling': spelling, 'status': status, 'target': target,
                     'target_kind': kind, 'lookup_site': lookup, 'alias_site': alias,
                     'declaration_sites': evidence})
    manifest = captured.read(repo/source_file)
    class_rows = re.findall(r'\\prop_gput:Nnn \\g_ftoo_id_to_class_prop \{ ([^{}]+) \} \{ ([^{}]+) \}', manifest)
    slug_rows = re.findall(r'\\prop_gput:Nnn \\g_ftoo_id_to_slug_prop \{ ([^{}]+) \} \{ ([^{}]+) \}', manifest)
    require(len(class_rows) == len({key for key, _ in class_rows}) and
            len(slug_rows) == len({key for key, _ in slug_rows}),
            'M063: duplicate departure source property')
    class_by_id = dict(class_rows)
    slug_by_id = dict(slug_rows)
    departure_rows = []
    for key, row in sorted(rows.items()):
        if not key.startswith('d:') or row['source_status'] != 'paper': continue
        target = row['target']
        require(class_by_id.get(target) == 'departure', 'M063: departure class differs: '+key)
        slug = slug_by_id.get(target)
        require(slug and key == 'd:'+slug, 'M063: departure slug differs: '+key)
        departure_rows.append({'slug': slug, 'target': target, 'handle': key,
                               'sites': sorted(declarations[key], key=lambda site: (site['file'], site['line']))})
    departure_rows.sort(key=lambda row: row['slug'])
    require(len({row['slug'] for row in departure_rows}) == len(departure_rows),
            'M063: duplicate departure slug')
    targets = []
    all_econ = native_target_requests(
        (path.relative_to(repo).as_posix()
         for path in (repo/'units/statements').glob('*.toml')),
        lambda file: captured.read(repo/file))
    for key, (file, unit_id, selector) in all_econ.items():
        require(rows.get(key, {}).get('source_status') == 'paper' and
                rows[key]['target'] not in (None, key),
                'M063: native target binding differs: '+key)
        source_lines = captured.read(repo/file).splitlines()
        sites = [number for number, line in enumerate(source_lines, 1)
                 if re.fullmatch(re.escape(selector)+r'\s*=\s*"'+re.escape(key)+r'"', line)]
        require(len(sites) == 1, 'M063: native target site differs: '+key)
        targets.append({'key': key, 'unit': unit_id, 'site': _site(file, sites[0])})
    targets.sort(key=lambda row: row['key'])
    require({row['key'] for row in targets} == set(all_econ),
            'M063: native target inventory differs')
    result_headers = []
    requests = result_header_requests(
        (path.relative_to(repo).as_posix()
         for path in (repo/'units/corpus').glob('*.toml')),
        lambda file: captured.read(repo/file))
    heading_pattern = re.compile(r'\\prop_gput:Nnn \\g_ftoo_id_to_heading_prop \{ ([^{}]+) \} \{ ([^{}]+) \}')
    reverse_pattern = re.compile(r'\\prop_gput:Nnn \\g_ftoo_id_to_tag_prop \{ ([^{}]+) \} \{ ([^{}]+) \}')
    headings = {}
    reverse = {}
    for match in heading_pattern.finditer(manifest):
        require(match[1] not in headings, 'M063: duplicate result heading: '+match[1])
        headings[match[1]] = (match[2], _site(source_file, manifest.count('\n', 0, match.start()) + 1))
    for match in reverse_pattern.finditer(manifest):
        require(match[1] not in reverse, 'M063: duplicate reverse paper tag: '+match[1])
        reverse[match[1]] = (match[2], _site(source_file, manifest.count('\n', 0, match.start()) + 1))
    for key, (file, unit_id) in sorted(requests.items()):
        binding = rows.get(key)
        require(binding is not None and binding['source_status'] == 'paper' and
                binding['spelling'] == key, 'M063: result header is not a canonical paper key: '+key)
        target = binding['target']
        require(reverse.get(target, (None,))[0] == key and target in headings and
                len(declarations.get(key, [])) == 1,
                'M063: result header binding or heading differs: '+key)
        title_source, heading_site = headings[target]
        require(title_source.strip() and not any(char in title_source for char in '\\{}$%#\n\r'),
                'M063: unsupported result heading: '+key)
        title = title_source.replace('~', ' ')
        unit_text = captured.read(repo/file)
        sites = [number for number, line in enumerate(unit_text.splitlines(), 1)
                 if re.fullmatch(r'key\s*=\s*"'+re.escape(key)+r'"', line)]
        require(len(sites) == 1, 'M063: result header source site differs: '+key)
        result_headers.append({'key': key, 'unit': unit_id, 'target': target,
                               'title': title, 'tag_site': declarations[key][0],
                               'reverse_site': reverse[target][1], 'heading_site': heading_site,
                               'unit_site': _site(file, sites[0])})
    stock = captured.read_bytes(repo/'documents/paper/references.bib')
    derived = bibroute.derive_bytes(stock)
    result = {'schema': 2,
              'inputs': [{'path': path, 'sha256': hashlib.sha256(captured.bytes[path]).hexdigest()}
                         for path in paths],
              'bindings': wire,
              'virtues': virtues,
              'departures': departure_rows,
              'native_targets': targets,
              'result_headers': result_headers,
              'bibliography': {'source': 'documents/paper/references.bib',
                               'stock_sha256': hashlib.sha256(stock).hexdigest(),
                               'derived_sha256': hashlib.sha256(derived).hexdigest(),
                               'derived_utf8': derived.decode('utf-8')}}
    captured.verify()
    return json.dumps(result, ensure_ascii=False, separators=(',', ':'))+'\n'


def emit(repo, work):
    rows = bindings(repo)
    lines = []
    for key,row in sorted(rows.items()):
        lines.append('\\PaperBinding{%s}{%s}{%s}' % (key,row['spelling'],row['status']))
    write(work/'reader-lookup.tex', '\n'.join(lines)+'\n')
    write(work/'reader-hooks.tex', read(repo/'documents/paper-build/reader-hooks.tex'))


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository',type=pathlib.Path)
    mode=parser.add_mutually_exclusive_group()
    mode.add_argument('--release',action='store_true')
    mode.add_argument('--assembly-json',action='store_true')
    args=parser.parse_args()
    try:
        if args.assembly_json:
            sys.stdout.write(assembly_json(args.repository))
        else:
            with timed('identity-bindings'):
                print('Checked bindings:',len(bindings(args.repository,args.release)))
    except (ValueError, OSError, UnicodeError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from None
