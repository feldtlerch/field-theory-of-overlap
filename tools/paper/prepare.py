"""Materialize the closed English source pair into an empty disposable directory."""
import argparse
import hashlib
import pathlib
import re
import shutil
import tomllib
from bibroute import derive
from identities import emit
from common import require, read, write, timed, runtime

INPUTS = ["tools/paper/prepare.py", "tools/paper/bibroute.py", "tools/paper/identities.py", "tools/paper/common.py", "documents/paper-build/sources.tsv", "documents/paper-build/compat.tex", "documents/paper-build/templates.toml", "documents/paper-build/runtime.toml", "generated/documents/paper-pilot/sec-say-direction.tex", "generated/documents/explainer-pilot/sec-say-direction.tex", "generated/documents/paper-pilot/sec-say-harm-enlargement.tex", "generated/documents/paper-pilot/sec-say-wonder-beauty.tex", "generated/documents/paper-pilot/sec-say-enemy-to-none.tex", "generated/documents/explainer-pilot/sec-say-enemy-to-none.tex", "generated/documents/paper-pilot/sec-coda-record-limit.tex", "generated/documents/explainer-pilot/sec-coda-record-limit.tex"]
SOURCES = 'documents/paper/'
FRAGMENTS = (
    ('documents/paper/parts/partA.tex', 184, 184, 'generated/documents/paper-pilot/sec-say-direction.tex', 'sec:say:direction'),
    ('documents/paper/parts/partA.tex', 601, 601, 'generated/documents/paper-pilot/sec-say-harm-enlargement.tex', 'sec:say:harm-enlargement'),
    ('documents/paper/parts/partA.tex', 675, 675, 'generated/documents/paper-pilot/sec-say-wonder-beauty.tex', 'sec:say:wonder-beauty'),
    ('documents/paper/explainer/explainer.tex', 319, 320, 'generated/documents/explainer-pilot/sec-say-direction.tex', 'sec:say:direction'),
    ('documents/paper/parts/partD.tex', 298, 298, 'generated/documents/paper-pilot/sec-say-enemy-to-none.tex', 'sec:say:enemy-to-none'),
    ('documents/paper/explainer/explainer.tex', 687, 687, 'generated/documents/explainer-pilot/sec-say-enemy-to-none.tex', 'sec:say:enemy-to-none'),
    ('documents/paper/parts/coda.tex', 112, 112, 'generated/documents/paper-pilot/sec-coda-record-limit.tex', 'sec:coda:record-limit'),
    ('documents/paper/explainer/explainer.tex', 972, 973, 'generated/documents/explainer-pilot/sec-coda-record-limit.tex', 'sec:coda:record-limit'),
)


def versioned(text, version, name):
    """Substitute the one version setting into a document master, where each placeholder occurs exactly once."""
    require(set(version) == {'VERSION', 'DRAFT'}, 'Version setting differs')
    for key,value in version.items():
        require(text.count('@'+key+'@') == 1, 'Version placeholder occurrence differs: '+name+' '+key)
        text = text.replace('@'+key+'@',value)
    return text


def paper_master(text, version):
    """Instantiate the paper's master; it carries no placeholder besides the version setting."""
    paper = versioned(text, version, 'paper')
    require(not re.search(r'@[A-Z_]+@',paper), 'Unexpanded paper placeholder')
    return paper


def declared_fragments(repository, rows):
    """Validate the bounded source slots and return their replacement first lines."""
    replacements = {}
    bodies = set()
    for source, first, last, fragment, unit in FRAGMENTS:
        require(source in rows and first > 0 and last >= first, 'Fragment source declaration differs: '+source)
        lines = (repository/source).read_bytes().splitlines(keepends=True)
        require(last <= len(lines) and lines[first-1] == ('% generated: '+unit+'\n').encode(),
                'Fragment marker/declaration differs: '+source+':'+str(first))
        require(all(line == b'%\n' for line in lines[first:last]), 'Fragment padding differs: '+source)
        data = (repository/fragment).read_bytes()
        expected_macro = b'paperrefrain' if source.startswith('documents/paper/explainer/') else b'refrain'
        prefix = b'\\'+expected_macro+b'{'+unit.encode()+b'}{'
        require(data.startswith(prefix) and data.endswith(b'}\n') and data.count(b'\n') == 1,
                'Fragment label/declaration differs: '+fragment)
        body = data[len(prefix):-2]
        require(body and b'{' not in body and b'}' not in body, 'Fragment body differs: '+fragment)
        bodies.add(b' '.join(body.split()))
        replacements[(source, first)] = data
    for source in rows:
        data = (repository/source).read_bytes()
        for match in re.finditer(rb'\\(?:paper)?refrain\{([^}]*)\}\{([^}]*)\}', data, re.S):
            label, body = match.groups()
            require(label not in {unit.encode() for *_, unit in FRAGMENTS}
                    and b' '.join(body.split()) not in bodies,
                    'Authored refrain duplicates a declared fragment: '+source)
        for *_, unit in FRAGMENTS:
            require(not re.search(rb'\\label\{'+re.escape(unit.encode())+rb'\}', data),
                    'Authored mint duplicates a declared fragment: '+source)
    declared = {(source, first) for source, first, *_ in FRAGMENTS}
    require(len(declared) == len(FRAGMENTS), 'Duplicate fragment slot')
    require(all(fragment in INPUTS for _, _, _, fragment, _ in FRAGMENTS),
            'Declared fragment absent from receipt inputs')
    for source in rows:
        for number, line in enumerate((repository/source).read_bytes().splitlines(), 1):
            require(not re.match(rb'\s*%\s*generated\s*:', line) or (source, number) in declared,
                    'Undeclared fragment marker: '+source+':'+str(number))
    return replacements


def prepare(repository, support, work):
    root = repository/'documents/paper-build'
    require(not work.exists(), 'Work directory must not exist')
    rows = read(root/'sources.tsv').splitlines()
    require(len(rows) == 26 and len(set(rows)) == 26, 'Closed list must have 26 distinct paths')
    require(all(p.startswith(SOURCES) and '..' not in pathlib.PurePosixPath(p).parts for p in rows), 'Closed list path escapes paper root')
    fragments = declared_fragments(repository, rows)
    settings, lock, engine, bundle = runtime(repository, support)
    locked = {f['member']:f['sha256'] for f in lock['bundle']['files']}
    sources = b'\n'.join((repository/d).read_bytes() for d in rows)
    packages = set()
    for group in re.findall(rb'\\(?:usepackage|RequirePackage)(?:\[[^]]*\])?\{([^}]+)\}',sources):
        packages.update(p.decode('utf-8').strip()+'.sty' for p in group.split(b','))
    for name in sorted(packages | set(settings['fonts']) | {'biblatex.bst'}):
        require(name in locked, 'Input absent from lock: '+name)
        require(hashlib.sha256((support/'bundle'/name).read_bytes()).hexdigest() == locked[name], 'Bundle input differs: '+name)
    work.mkdir(parents=True)
    for dest in rows:
        target = work/pathlib.Path(dest).relative_to(SOURCES)
        target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(repository/dest,target)
    for (source, first), data in fragments.items():
        target = work/pathlib.Path(source).relative_to(SOURCES)
        lines = target.read_bytes().splitlines(keepends=True)
        lines[first-1] = data
        target.write_bytes(b''.join(lines))
    shutil.copyfile(root/'compat.tex',work/'compat.tex')
    emit(repository, work)
    templates = tomllib.loads(read(root/'templates.toml'))
    master = versioned(read(work/'main_explainer.tex.in'), templates['version'], 'explainer')
    for key,value in templates['english'].items():
        expected = 2 if key == 'PAGEABBR' else 1
        require(master.count('@'+key+'@') == expected, 'English placeholder occurrence differs: '+key)
        master = master.replace('@'+key+'@',value)
    require(not re.search(r'@[A-Z_]+@',master), 'Unexpanded English placeholder')
    require(master.count('backend=bibtex') == 1 and 'backend=biber' not in master, 'Template backend differs')
    write(work/'main_explainer.tex', '\\input{compat.tex}\n'+master)
    write(work/'main.tex', '\\input{compat.tex}\n'+paper_master(read(work/'main.tex'), templates['version']))
    bridge = read(work/'paper_xref.tex.in')
    require(bridge.count('@PAPER_PDF@') == 1, 'Companion target occurrence differs')
    write(work/'paper_xref.tex', bridge.replace('@PAPER_PDF@',templates['companion']['target']))
    derive(repository/SOURCES/'references.bib',work)
    return sorted(packages)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository', type=pathlib.Path)
    for name in ['support','work']: parser.add_argument(name,type=pathlib.Path,nargs='?')
    args = parser.parse_args()
    with timed('prepare'):
        require(args.support is not None and args.work is not None, 'Support and work paths are required')
        print('Checked bundled packages:',', '.join(prepare(args.repository,args.support,args.work)))
