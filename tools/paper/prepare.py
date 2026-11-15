"""Materialize the closed English source pair into an empty disposable directory."""
import argparse
import hashlib
import pathlib
import re
import shutil
import tomllib
from bibroute import derive
from identities import emit
from common import require, read, write, timed, runtime, digest

INPUTS = ["tools/paper/prepare.py", "tools/paper/bibroute.py", "tools/paper/identities.py", "tools/paper/common.py", "documents/paper-build/sources.tsv", "documents/paper-build/compat.tex", "documents/paper-build/templates.toml", "documents/paper-build/runtime.toml"]
SOURCES = 'documents/paper/'
BUILD = 'documents/paper-build/'


def hook_addresses(hooks):
    """Return every (file name, first line, last line) that the reader hooks address by input line."""
    addresses = set(); current = None; pending = None
    token = re.compile(r'\{([A-Za-z0-9_-]+\.tex)\}|rebreak:nn\{(\d+)\}\{(\d+)\}|\\inputlineno(>=|<=|=)(\d+)')
    for match in token.finditer(hooks):
        name, first, last, relation, number = match.groups()
        if name:
            current = name
        elif first:
            addresses.add((current, int(first), int(last)))
        elif relation == '=':
            addresses.add((current, int(number), int(number)))
        elif relation == '>=':
            pending = int(number)
        else:
            require(pending is not None, 'Layout hook upper bound lacks its lower bound')
            addresses.add((current, pending, int(number))); pending = None
    require(pending is None and all(a[0] is not None for a in addresses), 'Layout hook address lacks its file')
    # A comparison of the input line with a literal number in any other form would address a line unpinned.
    literal = re.findall(r'\\inputlineno\s*\}?\s*[<>=]+\s*\{?\s*\d', hooks)
    require(len(literal) == len(re.findall(r'\\inputlineno(?:>=|<=|=)\d', hooks)), 'Layout hook address form is not recognised')
    return addresses


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


def check_layout_targets(repository):
    record = tomllib.loads(read(repository/BUILD/'layout-pins.toml'))
    pins = record.get('pin', [])
    require(set(record) == {'pin'} and pins, 'Layout pins are missing')
    pinned = set()
    for item in pins:
        require(set(item) == {'site', 'target', 'lines', 'sha256'} and len(item['lines']) == 2,
                'Malformed layout pin: '+str(item.get('site')))
        require(item['target'].startswith(SOURCES) and '..' not in pathlib.PurePosixPath(item['target']).parts,
                'Layout pin target escapes paper root: '+item['site'])
        address = (pathlib.PurePosixPath(item['target']).name, *item['lines'])
        require(address not in pinned, 'Duplicate layout pin: '+item['site'])
        pinned.add(address)
        require(digest((repository/item['target']).read_bytes()) == item['sha256'],
                'Layout hook target differs: '+item['site']+' ('+item['target']+')')
    addresses = hook_addresses(read(repository/BUILD/'reader-hooks.tex'))
    require(pinned == addresses, 'Layout hook addresses differ from pins: unpinned '+str(sorted(addresses-pinned))+', unused '+str(sorted(pinned-addresses)))


def prepare(repository, support, work):
    root = repository/'documents/paper-build'
    require(not work.exists(), 'Work directory must not exist')
    rows = read(root/'sources.tsv').splitlines()
    require(len(rows) == 26 and len(set(rows)) == 26, 'Closed list must have 26 distinct paths')
    require(all(p.startswith(SOURCES) and '..' not in pathlib.PurePosixPath(p).parts for p in rows), 'Closed list path escapes paper root')
    check_layout_targets(repository)
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
    parser.add_argument('--check-layout', action='store_true')
    parser.add_argument('repository', type=pathlib.Path)
    for name in ['support','work']: parser.add_argument(name,type=pathlib.Path,nargs='?')
    args = parser.parse_args()
    if args.check_layout:
        try:
            check_layout_targets(args.repository)
        except ValueError as error:
            parser.exit(1, str(error)+'\n')
    else:
        with timed('prepare'):
            require(args.support is not None and args.work is not None, 'Support and work paths are required')
            print('Checked bundled packages:',', '.join(prepare(args.repository,args.support,args.work)))
