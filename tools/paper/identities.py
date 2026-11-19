"""Validate the paper's tag lookup and emit the review lookup, never admissions."""
import argparse
import json
import pathlib
import re
import tomllib
from common import read, require, timed, write

INPUTS = ['tools/paper/identities.py', 'documents/paper-build/paper-tags.tsv',
          'documents/paper-build/reader-tags.toml', 'documents/paper-build/provisional-tags.toml',
          'documents/paper-build/reader-hooks.tex']
TAG = re.compile(r'[A-Za-z][A-Za-z0-9_-]*:[A-Za-z0-9][A-Za-z0-9_-]*\Z')
KEY = re.compile(r'[A-Za-z][A-Za-z0-9_-]*(?::[A-Za-z0-9_-]+)+\Z')


def holds(line, key):
    """A line holds a key when the key occurs there as a whole label, not as part of a longer one."""
    return re.search(r'(?<![A-Za-z0-9_:-])'+re.escape(key)+r'(?![A-Za-z0-9_:-])', line) is not None


def label_mints(repo):
    numbered = [(number, line.split('\t')) for number, line in
                enumerate(read(repo/'documents/paper-build/label-mints.tsv').splitlines(), 1)
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
    declared = set(read(repo/'documents/paper-build/sources.tsv').splitlines())
    sources = {}
    for key, file in rows:
        if file not in sources:
            path = repo/file
            require(path.is_file(), 'M063: label mint file is absent: '+file)
            require(file in declared, 'M063: label mint file is not declared: '+file)
            sources[file] = read(path).split('\n')
        lines = sources[file]
        require(any(holds(line, key) for line in lines),
                'M063: label mint file does not hold its key: '+key+' at '+file)
    return rows


def bindings(repo, release=False):
    root = repo/'documents/paper-build'
    paper_bytes = (root/'paper-tags.tsv').read_bytes()
    label_mints(repo)
    paper = {}
    for line in paper_bytes.decode('utf-8').splitlines():
        if line.startswith('#'): continue
        fields = line.split('\t')
        require(len(fields) == 3, 'M063: malformed paper tag row')
        tag, target, kind = fields
        require(TAG.fullmatch(tag) and KEY.fullmatch(target), 'M063: malformed paper tag pair')
        require(tag not in paper and kind in ('node','evidence'), 'M063: duplicate paper tag binding')
        paper[tag] = target
    # A confirmed admission removes its paper tag row and updates this inventory count.
    require(len(paper) == 329, 'M063: paper tag row count differs')
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
    collect(json.loads(read(repo/'generated/manifest.json')))
    theory_reserved = reserved.copy()
    documentary = tomllib.loads(read(repo/'registry/documentary-tags.toml'))
    handles = {event['handle'] for event in documentary.get('events', [])}
    require(len({handle.casefold() for handle in handles}) == len(handles), 'M063: case-insensitive registry collision')
    require(not {handle.casefold() for handle in handles} & theory_reserved, 'M063: documentary spelling collides with theory')
    collect(documentary)
    require(not set(paper.values()) & reserved_targets, 'M063: paper tag target collides with a permanent identity')
    table = tomllib.loads(read(root/'reader-tags.toml'))
    candidates = tomllib.loads(read(root/'provisional-tags.toml'))['candidates']
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
            result[key] = dict(row, status='paper' if status in ('paper','alias') else 'admitted' if status=='admitted' else row['status'], target=paper.get(spelling,key))
    require(set(paper) == {k for k,v in result.items() if k == v['spelling'] and v['status']=='paper'}, 'M063: paper tag coverage differs')
    return result


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
    parser.add_argument('--release',action='store_true')
    args=parser.parse_args()
    with timed('identity-bindings'):
        print('Checked bindings:',len(bindings(args.repository,args.release)))
