"""Check final shipped occurrences against lookup, auxiliary anchors and PDF destinations."""
import argparse
import collections
import json
import pathlib
import re
from common import read, require, timed, write
from identities import bindings
from pdf_text import Pdf

INPUTS = ['tools/paper/ledger.py', 'tools/paper/identities.py', 'tools/paper/pdf_text.py',
          'generated/manifest.json', 'registry/documentary-tags.toml',
          'documents/paper-build/label-mints.tsv', 'documents/paper/data/ftoo_manifest.tex',
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


def auxiliary(path, job):
    result = {}
    pending=[path/(job+'.aux')]; seen=set()
    while pending:
        file=pending.pop()
        if file in seen:continue
        seen.add(file)
        for line in read(file).splitlines():
            m=re.match(r'\\zref@newlabel\{([^}]+)\}',line)
            if m: result[m[1]] = {k:property_value(line,k) for k in ('default','page','anchor','title')}
            include=re.match(r'\\@input\{([^}]+)\}',line)
            if include:
                part=path/include[1]
                require(part.resolve().is_relative_to(path.resolve()),'Auxiliary input escapes work area')
                pending.append(part)
    return result


def ledger_binding(lookup, kind, key, spelling, status):
    source_key=key.removeprefix('FTOO-')
    if kind not in ('locator','title-locator'):
        require(source_key in lookup, 'M064: unbound ledger key: '+key)
        row=lookup[source_key]
        require((spelling,status)==(row['spelling'],row['status']), 'M064: ledger binding differs: '+key)
    return source_key


def check_destination(document, key, anchor, auxiliary_page, destination_page):
    require(destination_page is not None, 'M064: missing PDF destination: '+key+' '+anchor)
    require(destination_page == auxiliary_page,
            'M064: auxiliary/PDF page discrepancy: '+str(dict(document=document, key=key, anchor=anchor,
                auxiliary_page=auxiliary_page, destination_page=destination_page)))


def screen_references(screen, paper_auxiliary):
    see = list(re.finditer(r'<a href="[^"]+">→ (appendix [A-Z])</a><!-- see ([A-Za-z][A-Za-z0-9_:-]*) -->', screen))
    require(len(see)==screen.count('<!-- see '), 'M055: malformed generated see reference')
    checked=[]
    for match in see:
        locator,label = match.groups()
        target = paper_auxiliary.get(label)
        require(target is not None and target['default'] and target['page'] and target['anchor'],
                'M055: paper reference target absent: '+label)
        require(locator == 'appendix '+target['default'],
                'M055: screen/PDF reference number differs: '+label)
        checked.append((label,target))
    return checked


def check_screen_references(repo, paper_auxiliary, paper_pdf):
    for owner in ('paper-pilot', 'explainer-pilot'):
        screen = read(repo/'generated/documents'/owner/'en.labels.md')
        for label,target in screen_references(screen,paper_auxiliary):
            check_destination('main',label,target['anchor'],int(target['page']),paper_pdf.destination(target['anchor']))


def check(repo, work):
    lookup = bindings(repo)
    paper = work/'paper'
    docs = {job:Pdf(paper/(job+'.pdf')) for job in ('main','main_explainer')}
    aux = {job:auxiliary(paper,job) for job in docs}
    check_screen_references(repo,aux['main'],docs['main'])
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
    write(work/'identity-ledger.json',json.dumps(rows,ensure_ascii=False,indent=2)+'\n')
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
