"""Check final shipped occurrences against lookup, auxiliary anchors and PDF destinations."""
import argparse
import collections
import hashlib
import html
import json
import pathlib
import posixpath
import re
import tomllib
from common import read, require, timed, write
from identities import CapturedInputs, assembly_json, bindings, virtue_roster
from pdf_text import Pdf

INPUTS = ['tools/paper/ledger.py', 'tools/paper/identities.py', 'tools/paper/pdf_text.py',
          'generated/manifest.json', 'registry/documentary-tags.toml',
          'documents/paper-build/label-mints.tsv', 'documents/paper/data/ftoo_manifest.tex',
          'documents/paper-build/paper-tags.tsv', 'generated/documents/inclusions.tsv',
          'generated/documents/paper-pilot/en.labels.md',
          'generated/documents/explainer-pilot/en.labels.md']
LEAK = re.compile(r'\b[A-Za-z][A-Za-z0-9_-]*:[A-Za-z0-9_-]+(?::[A-Za-z0-9_-]+)+', re.I)


def property_value(line, name):
    marker='\\'+name+'{'
    if marker not in line:return ''
    begin=line.index(marker)+len(marker); depth=1; end=begin
    while end<len(line) and depth:
        if line[end]=='{' and (end==0 or line[end-1]!='\\'):depth+=1
        elif line[end]=='}' and (end==0 or line[end-1]!='\\'):depth-=1
        end+=1
    require(depth==0,'Unbalanced auxiliary property: '+name)
    return line[begin:end-1]


def leaks(text):
    # Column separation comes from glyph geometry, never from a tag allowlist.
    return [m.group().casefold() for m in LEAK.finditer(text)]


def occurrence_differences(observed, emitted):
    return [dict(document=k[0],page=k[1],spelling=k[2],observed=observed[k],emitted=emitted[k])
            for k in sorted(observed.keys() | emitted.keys()) if observed[k] != emitted[k]]


def auxiliary(path, job, unique_keys=()):
    result = {}
    counts = collections.Counter()
    pending=[path/(job+'.aux')]; seen=set()
    while pending:
        file=pending.pop()
        if file in seen:continue
        seen.add(file)
        for line in read(file).splitlines():
            m=re.match(r'\\zref@newlabel\{([^}]+)\}',line)
            if m:
                counts[m[1]] += 1
                require(not (m[1].startswith('v:') and counts[m[1]] > 1),
                        'M064: duplicate virtue auxiliary definition: '+m[1])
                result[m[1]] = {k:property_value(line,k) for k in ('default','page','anchor','title','zc@type')}
            include=re.match(r'\\@input\{([^}]+)\}',line)
            if include:
                part=path/include[1]
                require(part.resolve().is_relative_to(path.resolve()),'Auxiliary input escapes work area')
                pending.append(part)
    for key in unique_keys:
        require(counts[key] == 1, 'M055: Auxiliary definition count differs: '+key)
    return result


def ledger_binding(lookup, kind, key, spelling, status):
    source_key=key.removeprefix('FTOO-')
    if kind not in ('locator','title-locator','virtue-first','virtue-later','virtue-companion'):
        require(source_key in lookup, 'M064: unbound ledger key: '+key)
        row=lookup[source_key]
        require((spelling,status)==(row['spelling'],row['status']), 'M064: ledger binding differs: '+key)
    return source_key


def native_virtue_plan(repo):
    """Join checked inclusions to the prepared source's command positions."""
    native = {}
    shifts = collections.Counter()
    rows = [row.split('\t') for row in read(repo/'generated/documents/inclusions.tsv').splitlines()]
    for source, unit, fragment, role, line in sorted(rows, key=lambda row: (row[0], int(row[4]))):
        body = read(repo/fragment)
        if role == 'corpus-slot':
            require(body.endswith('\n'), 'M055: Corpus slot body differs: '+fragment)
            shifts[source] += body.count('\n') - 1
            continue
        if role != 'paragraph':
            continue
        calls = re.findall(r'\\(papervirtue|virtue)\{([a-z]+)\}', body)
        if calls:
            key = (pathlib.PurePosixPath(source).as_posix(), int(line) + shifts[source])
            require(key not in native, 'M064: duplicate native virtue inclusion: '+unit)
            native[key] = (unit, calls)
    return native


def virtue_site(site):
    match = re.fullmatch(r'([^#]+):([1-9][0-9]*)#virtue=(0|[1-9][0-9]*)',site)
    require(match is not None, 'M064: malformed virtue occurrence site: '+site)
    file, line, index = match.groups()
    file = posixpath.normpath(file)
    require(not file.startswith('../') and file != '..' and not file.startswith('/'),
            'M064: virtue occurrence site escapes paper: '+site)
    return 'documents/paper/'+file.lstrip('/'), int(line), int(index)


def check_native_virtue_row(source_key, kind, occurrence, unit, calls):
    require(occurrence < len(calls), 'M064: extra native virtue occurrence: '+unit)
    form, word = calls[occurrence]
    require(source_key == 'v:'+word and
            (form == 'papervirtue') == (kind == 'virtue-companion') and
            kind != 'virtue-first',
            'M064: native virtue mode/word differs: '+unit)


def check_virtue_context(source_key, kind, job, virtues, key):
    require(source_key in virtues, 'M064: unknown virtue roster key: '+key)
    require((job == 'main_explainer') == (kind == 'virtue-companion'),
            'M064: virtue command mode/document differs: '+key)


def check_virtue_inventory(native, native_seen, shipped_first, virtues, paper_auxiliary):
    for (source,line_number),(unit,calls) in native.items():
        for index,_ in enumerate(calls):
            require(native_seen[(source,line_number,index)] == 1,
                    'M064: native virtue occurrence missing or repeated: '+unit+'#'+str(index))
    for key in virtues:
        require(shipped_first[key] == 1, 'M064: virtue first-use count differs: '+key)
        require(key in paper_auxiliary and paper_auxiliary[key]['anchor'],
                'M064: virtue auxiliary definition absent: '+key)


def check_companion_mints(companion_auxiliary, virtues):
    for key in virtues:
        require(key not in companion_auxiliary and 'FTOO-'+key not in companion_auxiliary,
                'M064: companion virtue minted a destination: '+key)


def check_destination(document, key, anchor, auxiliary_page, destination_page):
    require(destination_page is not None, 'M064: missing PDF destination: '+key+' '+anchor)
    require(destination_page == auxiliary_page,
            'M064: auxiliary/PDF page discrepancy: '+str(dict(document=document, key=key, anchor=anchor,
                auxiliary_page=auxiliary_page, destination_page=destination_page)))


def screen_references(screen, paper_auxiliary):
    component = r'(?:0|[1-9][0-9]*)'
    locator = rf'(?:appendix [A-Z]|§{component}(?:\.{component})?)'
    see = list(re.finditer(rf'<a href="[^"]+">→ ({locator})</a><!-- see ([A-Za-z][A-Za-z0-9_:-]*) -->', screen))
    require(len(see)==screen.count('<!-- see '), 'M055: malformed generated see reference')
    checked=[]
    for match in see:
        locator,label = match.groups()
        target = paper_auxiliary.get(label)
        require(target is not None and target['default'] and target['page'] and target['anchor'],
                'M055: paper reference target absent: '+label)
        expected = ('appendix ' if locator.startswith('appendix ') else '§') + target['default']
        require(locator == expected,
                'M055: screen/PDF reference number differs: '+label)
        checked.append((label,target))
    return checked


def check_screen_references(repo, paper_auxiliary, paper_pdf):
    for owner in ('paper-pilot', 'explainer-pilot'):
        screen = read(repo/'generated/documents'/owner/'en.labels.md')
        for label,target in screen_references(screen,paper_auxiliary):
            check_destination('main',label,target['anchor'],int(target['page']),paper_pdf.destination(target['anchor']))


def check_screen_economics(repo, paper_auxiliary, paper_pdf):
    screen = read(repo/'generated/documents/paper-pilot/en.labels.md')
    pattern = re.compile(r'<p><a id="([a-z0-9-]+)"></a><strong>Economics ([0-9]+\.[0-9]+)\. ([^<]+)</strong> \(on [^<]+<!-- label [^ ]+ -->\)</p><!-- econ ([a-z0-9:-]+) ([a-z0-9:-]+) -->')
    found = list(pattern.finditer(screen))
    require(len(found) == screen.count('<!-- econ '), 'M055: malformed generated economics header')
    targets = {}
    for match in found:
        fragment, number, title, unit, key = match.groups()
        require(key not in targets and unit and fragment, 'M055: repeated economics target: '+key)
        targets[key] = unit
        target = paper_auxiliary.get(key)
        require(target is not None and target['default'] and target['page'] and target['anchor'],
                'M055: paper economics target absent: '+key)
        require(number == target['default'] and title == target['title'].replace('~', ' '),
                'M055: screen/PDF economics number or title differs: '+key)
        check_destination('main', key, target['anchor'], int(target['page']), paper_pdf.destination(target['anchor']))
    return targets


def check_screen_results(repo, paper_auxiliary, paper_pdf, headers):
    """Compare planned result headers with the independent typesetter evidence."""
    screen = read(repo/'generated/documents/paper-pilot/en.labels.md')
    inclusions = [line.split('\t') for line in
                  read(repo/'generated/documents/inclusions.tsv').splitlines()]
    by_unit = {unit: fragment for _,unit,fragment,role,_ in inclusions
               if role == 'corpus-slot'}
    keys = set()
    for row in headers:
        unit, key, title = row['unit'], row['key'], row['title']
        require(key not in keys and unit in by_unit,
                'M055: result header occurrence differs: '+key)
        keys.add(key)
        fragment = read(repo/by_unit[unit])
        first = fragment.splitlines()[0]
        match = re.fullmatch(r'\\begin\{(insight|remark|definition)\}\[\\hdr\{([^{}]+)\}\]'
                             r'(?:\\grade\{(nec|feas|cont|cont-anch|cont-comm)\})?'
                             r'(?:\\PaperGradeGap\{-2pt\})?',first)
        require(match is not None and match[2] == key,
                'M055: generated result header differs: '+key)
        kind, _, grade = match.groups()
        label = kind.capitalize()
        style = '**' if kind == 'definition' else '*'
        pattern = (r'<!-- unit '+re.escape(unit)+r' document paper-pilot language en -->\n'
                   + re.escape(style)+re.escape(label)+r' ([0-9]+\.[0-9]+) '
                   + r'\(([^\n]+)\)'+re.escape(style)+r' · '+re.escape(key)
                   + (r' '+re.escape(grade) if grade else '')
                   + r' <a id="result-[a-z0-9-]+"></a>')
        matches = list(re.finditer(pattern, screen))
        require(len(matches) == 1 and html.unescape(matches[0][2]) == title,
                'M055: recovered result header differs: '+key)
        target = paper_auxiliary.get(key)
        expected_type = 'definition' if kind == 'definition' else 'remark'
        require(target is not None and matches[0][1] == target['default']
                and title == target['title'].replace('~',' ')
                and expected_type == target['zc@type'] and target['page'] and target['anchor'],
                'M055: result number, title or reference kind differs: '+key)
        check_destination('main', key, target['anchor'], int(target['page']),
                          paper_pdf.destination(target['anchor']))

def check_paragraph_tag_modes(repo, rows):
    """Bind converted tags and handle placements to their authored source and shipped mode."""
    for row in (repo/'generated/documents/inclusions.tsv').read_text(encoding='utf-8').splitlines():
        source, unit, fragment, role, marker = row.split('\t')
        if role != 'paragraph':
            continue
        body = (repo/fragment).read_bytes()
        tags = [key.decode('ascii') for key in re.findall(rb'\\ftag\{([^{}]+)\}', body)]
        handles = [key.decode('ascii') for key in re.findall(rb'\\handle\{([^{}]+)\}', body)]
        if not tags and not handles:
            continue
        candidates = list((repo/'units/statements').glob('*.toml'))
        owners = [path for path in candidates if tomllib.loads(path.read_text(encoding='utf-8'))['unit']['id'] == unit]
        require(len(owners) == 1, 'M055: Paragraph tag owner differs: '+unit)
        owner = tomllib.loads(owners[0].read_text(encoding='utf-8'))['unit']
        authored = owner['texts']['en']
        targets = owner.get('documentary-targets', {})
        selectors = re.findall(r'@handle\{([^{}]+)\}', authored)
        require(all(selector in targets for selector in selectors)
                and handles == [targets[selector] for selector in selectors],
                'M055: Paragraph handle command sequence differs: '+unit)
        require(tags == re.findall(r'@tag\{([^{}]+)\}', authored),
                'M055: Paragraph tag command sequence differs: '+unit)
        site = source.removeprefix('documents/paper/')
        interval = {int(marker), int(marker)+1}
        def at_marker(item):
            prefix, sep, line = item['site'].rpartition(':')
            return sep and prefix == site and line.isdecimal() and int(line) in interval
        for index,key in enumerate(tags):
            identities = [item for item in rows if item['document'] == 'main' and item['kind'] == 'identity'
                          and item['key'] == key and at_marker(item)]
            locators = [item for item in rows if item['document'] == 'main' and item['kind'] == 'locator'
                        and item['key'] == key and at_marker(item)]
            require(len(identities) == 1 and len(locators) <= 1,
                    'M055: Paragraph tag shipped association differs: '+unit+':'+key)
            legacy = unit == 'sec:log:andreoni-joint' and key == 'J:occ' and index == 0
            if legacy:
                raw = owners[0].read_bytes()
                blob = b'blob '+str(len(raw)).encode()+b'\0'+raw
                require(hashlib.sha1(blob).hexdigest() == 'e0707ffeb20702a77341af79505cc1732997303f',
                        'M055: Located paragraph tag owner changed: '+unit)
            require(len(locators) == int(legacy),
                    'M055: Paragraph tag print mode differs: '+unit+':'+key)


def check_auxiliary_destinations(keys, targets, pdf):
    """Require the destination of each generated auxiliary mint, including unreferenced ones."""
    for key in keys:
        target = targets.get(key)
        require(target is not None, 'M064: unresolved auxiliary target: '+key)
        anchor = target.get('anchor', '')
        check_destination('main', key, anchor, int(target['page']), pdf.destination(anchor))


def check(repo, work):
    lookup = bindings(repo)
    captured = CapturedInputs(repo)
    virtues = {row['key']: row for row in virtue_roster(repo, lookup, captured)}
    result_headers = json.loads(assembly_json(repo))['result_headers']
    captured.verify()
    native = native_virtue_plan(repo)
    native_seen = collections.Counter()
    shipped_first = collections.Counter()
    paper = work/'paper'
    docs = {job:Pdf(paper/(job+'.pdf')) for job in ('main','main_explainer')}
    fragments = (repo/'generated/documents/inclusions.tsv').read_text(encoding='utf-8').splitlines()
    auxiliary_keys = set()
    for row in fragments:
        fields = row.split('\t')
        require(len(fields) == 5, 'M055: Inclusion row differs')
        body = (repo/fields[2]).read_bytes()
        auxiliary_keys.update(key.decode('ascii') for key in re.findall(rb'\\handle\{([^{}]+)\}', body))
    auxiliary_keys.update(row['key'] for row in result_headers)
    aux = {job:auxiliary(paper,job, auxiliary_keys if job == 'main' else ()) for job in docs}
    check_auxiliary_destinations(auxiliary_keys, aux['main'], docs['main'])
    check_companion_mints(aux['main_explainer'], virtues)
    check_screen_references(repo,aux['main'],docs['main'])
    check_screen_economics(repo,aux['main'],docs['main'])
    check_screen_results(repo,aux['main'],docs['main'],result_headers)
    rows=[]; issues=[]; observed=collections.Counter()
    known={v['spelling'].casefold() for v in lookup.values()}
    for job,pdf in docs.items():
        for line in read(paper/(job+'.identities')).splitlines():
            fields=[s.strip() for s in line.split('|')]
            require(len(fields)==7, 'M064: malformed ledger row: '+line)
            document,page,kind,key,spelling,status,site=fields
            require(document==job and page.isdecimal() and 1<=int(page)<=pdf.count, 'M064: invalid shipped page: '+line)
            target_job='main' if key.startswith('FTOO-') or (job=='main_explainer' and not key.startswith('x')) else job
            source_key=ledger_binding(lookup, kind, key, spelling, status)
            if kind in ('virtue-first','virtue-later','virtue-companion'):
                check_virtue_context(source_key, kind, job, virtues, key)
                source, line_number, occurrence = virtue_site(site)
                expected = native.get((source,line_number))
                if expected is not None:
                    unit, calls = expected
                    check_native_virtue_row(source_key, kind, occurrence, unit, calls)
                    native_seen[(source,line_number,occurrence)] += 1
                if kind == 'virtue-first':
                    shipped_first[source_key] += 1
            target=aux[target_job].get(source_key)
            if target:
                anchor=target.get('anchor','')
                target_page=docs[target_job].destination(anchor)
                check_destination(target_job, source_key, anchor, int(target['page']), target_page)
            else:
                require(kind=='index', 'M064: unresolved ledger target: '+key)
                anchor=''; target_page=None
            rows.append(dict(document=job,page=int(page),kind=kind,key=source_key,spelling=spelling,status=status,
                             target_document=target_job,anchor=anchor,counter=target.get('default','') if target else '',
                             target_page=target_page,auxiliary_page=int(target['page']) if target else None,site=site,visibility='page'))
        pages=pdf.pages(); bookmarks=pdf.bookmarks()
        for page,text in enumerate(pages,1):
            for leak in leaks(text):issues.append([job,page,leak])
            for match in re.finditer(r'\b[A-Za-z][A-Za-z0-9_-]*:[A-Za-z0-9_-]+',text):
                spelling=match[0].casefold()
                if spelling not in known:issues.append([job,page,'unbound printed spelling: '+spelling])
                observed[job,page,spelling]+=1
        for bookmark in bookmarks:
            for leak in leaks(bookmark['title']):issues.append([job,'bookmark',leak])
            for match in re.finditer(r'\b[A-Za-z][A-Za-z0-9_-]*:[A-Za-z0-9_-]+',bookmark['title']):
                spelling=match[0]
                candidates=[(key,value) for key,value in lookup.items() if value['spelling'].casefold()==spelling.casefold()]
                require(len(candidates)==1,'M064: ambiguous or missing bookmark binding: '+spelling)
                key,value=candidates[0]; target=aux[job].get(key)
                require(target is not None and bookmark['page'] is not None,'M064: bookmark target missing: '+key)
                target_page=pdf.destination(target['anchor'])
                # A heading can mention an object defined on another page.
                require(pdf.destination(bookmark['anchor'])==bookmark['page'],
                        'M064: bookmark destination differs: '+key)
                require(target_page==int(target['page']), 'M064: bookmark identity target differs: '+key)
                rows.append(dict(document=job,page=bookmark['page'],kind='bookmark',key=key,spelling=value['spelling'],status=value['status'],
                                 target_document=job,anchor=target['anchor'],counter=target['default'],target_page=int(target['page']),auxiliary_page=int(target['page']),
                                 site='outline:'+bookmark['anchor'],visibility='outline'))
        write(work/(job+'-text.json'),json.dumps(pages,ensure_ascii=False,indent=2)+'\n')
        write(work/(job+'-bookmarks.json'),json.dumps(bookmarks,ensure_ascii=False,indent=2)+'\n')
    check_virtue_inventory(native, native_seen, shipped_first, virtues, aux['main'])
    write(work/'identity-ledger.json',json.dumps(rows,ensure_ascii=False,indent=2)+'\n')
    check_paragraph_tag_modes(repo, rows)
    emitted=collections.Counter((r['document'],r['page'],r['spelling'].casefold()) for r in rows if r['spelling'] and r['visibility']=='page')
    coverage=occurrence_differences(observed,emitted)
    write(work/'identity-coverage.json',json.dumps(coverage,indent=2)+'\n')
    for row in coverage:
        issues.append([row['document'],row['page'],'occurrence count differs: '+row['spelling']])
    write(work/'identity-leaks.json',json.dumps(issues,indent=2)+'\n')
    require(not issues, 'M064: rendered identity leak: '+str(issues[:10]))
    sheet=[]
    for key,row in lookup.items():
        if row['status'] in ('paper','admitted'):continue
        sites=[dict(document=r['document'],page=r['page'],site=r['site'],kind=r['kind']) for r in rows if r['key']==key]
        require(sites,'M064: candidate has no shipped occurrence: '+key)
        target=aux['main_explainer' if key.startswith('x') else 'main'].get(key,{})
        sheet.append(dict(row,printed_where=sites,auxiliary_title=target.get('title','')))
    write(work/'identity-proposals.json',json.dumps(sheet,ensure_ascii=False,indent=2)+'\n')
    print('Shipped identities and locators:',len(rows),'proposal rows:',len(sheet))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('repository',type=pathlib.Path); p.add_argument('work',type=pathlib.Path)
    a=p.parse_args()
    with timed('identity-ledger'):check(a.repository,a.work)
