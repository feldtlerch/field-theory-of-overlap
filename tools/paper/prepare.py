"""Materialize the closed English source pair into an empty disposable directory."""
import argparse
import hashlib
import pathlib
import re
import shutil
import tomllib
from bibroute import derive
from identities import emit, label_mints
from common import require, read, write, timed, runtime

INPUTS = ["tools/paper/prepare.py", "tools/paper/bibroute.py", "tools/paper/identities.py", "tools/paper/common.py", "documents/paper-build/sources.tsv", "documents/paper-build/compat.tex", "documents/paper-build/fontconfig.conf", "documents/paper-build/templates.toml", "documents/paper-build/runtime.toml", "generated/documents/inclusions.tsv"]
SOURCES = 'documents/paper/'


def scope_fragments(repository):
    """Read the checked inclusion artifact emitted by document generation."""
    rows = []
    for number, row in enumerate(read(repository/'generated/documents/inclusions.tsv').splitlines(), 1):
        fields = row.split('\t')
        require(len(fields) == 5 and fields[4].isdecimal(),
                'M079: Inclusion row differs: '+str(number))
        source, unit, fragment, role, line = fields
        require(source.startswith(SOURCES) and '..' not in pathlib.PurePosixPath(source).parts
                and role in ('refrain', 'paragraph', 'quotation', 'equation') and int(line) > 0,
                'M079: Inclusion row differs: '+str(number))
        rows.append((source, unit, fragment, role, int(line)))
    require(rows, 'M079: Checked inclusions absent')
    return tuple(rows)


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


def equation_tokens(body):
    """Compare TeX math without ignored layout whitespace, preserving word spaces."""
    pieces = re.split(rb'(\\text\{[^{}]*\})', body.replace(b'\\times{}', b'\\times'))
    return b''.join(piece if index % 2 else re.sub(rb'\s+', b'', piece)
                    for index, piece in enumerate(pieces))


def declared_fragments(repository, rows):
    """Validate unique source markers and return their replacement lines."""
    replacements = {}
    marker_at = {}
    bodies = set()
    paragraph_bodies = []
    equation_bodies = []
    fragments = scope_fragments(repository)
    declared = {(source, unit) for source, unit, _, _, _ in fragments}
    require(len(declared) == len(fragments), 'Duplicate fragment slot')
    for source, unit, fragment, role, line in fragments:
        require(source in rows, 'Fragment source declaration differs: '+source)
        lines = (repository/source).read_bytes().splitlines(keepends=True)
        marker = ('% generated: '+unit+'\n').encode()
        positions = [number for number, content in enumerate(lines, 1) if content == marker]
        require(positions == [line], 'M055: Fragment marker/declaration differs: '+source+':'+unit)
        data = (repository/fragment).read_bytes()
        expected_macro = {'quotation': b'paperrefrain', 'refrain': b'refrain',
                          'paragraph': b'PaperNativeParagraph',
                          'equation': b'PaperNativeEquation'}[role]
        prefix = b'\\'+expected_macro+b'{'+unit.encode()+b'}{'
        require(data.startswith(prefix),
                ('M055: Fragment label/declaration differs: ' if role == 'paragraph'
                 else 'Fragment label/declaration differs: ')+fragment)
        require(data.endswith(b'}\n') and data.count(b'\n') == 1,
                ('M055: Fragment body differs: ' if role == 'paragraph'
                 else 'Fragment label/declaration differs: ')+fragment)
        body = data[len(prefix):-2]
        if role == 'paragraph':
            checked_math = rb'\\\(\\(?:comovfield|willvirtue|gensym)\{\}\\\)'
            allowed = (rb'(?:[^{}\\]|\\ftag\{[A-Za-z][A-Za-z0-9_:-]*\}|'
                       rb'\\see\{[A-Za-z][A-Za-z0-9_:-]*\}|\\emph\{[^{}\\]+\}|\\(?:paren|text)cite\{a:[A-Za-z0-9-]+(?:,a:[A-Za-z0-9-]+)*\}|'
                       + checked_math + rb')+')
            require(body and re.fullmatch(allowed, body),
                    'M055: Fragment body differs: '+fragment)
            from identities import bindings
            available = bindings(repository)
            for key in re.findall(rb'\\ftag\{([^{}]+)\}', body):
                row = available.get(key.decode('ascii'))
                require(row is not None and row['source_status'] in ('paper', 'alias'),
                        'M055: Fragment tag binding differs: '+fragment)
            for key in re.findall(rb'\\see\{([^{}]+)\}', body):
                require(key.decode('ascii') in {label for label, _ in label_mints(repository)},
                        'M055: Fragment reference binding differs: '+fragment)
            paragraph_bodies.append(b' '.join(body.split()))
        elif role == 'equation':
            require(body.startswith(b'\\[') and body.endswith(b'.\\]')
                    and body.count(b'\\[') == 1 and body.count(b'\\]') == 1,
                    'M055: Fragment equation envelope differs: '+fragment)
            equation_bodies.append(equation_tokens(body[2:-3]))
        else:
            require(body and b'{' not in body and b'}' not in body,
                    'Fragment body differs: '+fragment)
            bodies.add(b' '.join(body.split()))
        replacements[(source, positions[0])] = data
        marker_at[(source, positions[0])] = marker.rstrip(b'\n')
    for source in rows:
        data = (repository/source).read_bytes()
        for match in re.finditer(rb'\\(?:paper)?refrain\{([^}]*)\}\{([^}]*)\}', data, re.S):
            label, body = match.groups()
            require(label not in {unit.encode() for _, unit, _, _, _ in fragments}
                    and b' '.join(body.split()) not in bodies,
                    'Authored refrain duplicates a declared fragment: '+source)
        for _, unit, _, _, _ in fragments:
            require(not re.search(rb'\\label\{'+re.escape(unit.encode())+rb'\}', data),
                    'Authored mint duplicates a declared fragment: '+source)
        normalized = b' '.join(data.split())
        for paragraph_body in paragraph_bodies:
            require(paragraph_body not in normalized,
                    'M055: Authored paragraph duplicates a declared fragment: '+source)
        for equation_body in equation_bodies:
            require(equation_body not in equation_tokens(data),
                    'M055: Authored equation duplicates a declared fragment: '+source)
    for source in rows:
        for number, line in enumerate((repository/source).read_bytes().splitlines(), 1):
            marker = re.match(rb'\s*%\s*generated\s*:', line)
            require(not marker or marker_at.get((source, number)) == line,
                    'Undeclared fragment marker: '+source+':'+str(number))
    return replacements


def package_names(repository, rows):
    """Return package members requested by the declared paper sources."""
    sources = b'\n'.join((repository / path).read_bytes() for path in rows)
    packages = set()
    for group in re.findall(rb'\\(?:usepackage|RequirePackage)(?:\[[^]]*\])?\{([^}]+)\}',
                            sources):
        packages.update(part.decode('utf-8').strip() + '.sty'
                        for part in group.split(b','))
    return packages


def prepare(repository, support, work):
    root = repository/'documents/paper-build'
    require(not work.exists(), 'Work directory must not exist')
    rows = read(root/'sources.tsv').splitlines()
    require(len(rows) == 26 and len(set(rows)) == 26, 'Closed list must have 26 distinct paths')
    require(all(p.startswith(SOURCES) and '..' not in pathlib.PurePosixPath(p).parts for p in rows), 'Closed list path escapes paper root')
    fragments = declared_fragments(repository, rows)
    settings, lock, engine, bundle = runtime(repository, support)
    locked = {f['member']:f['sha256'] for f in lock['bundle']['files']}
    packages = package_names(repository, rows)
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
    shutil.copyfile(root/'fontconfig.conf',work/'fontconfig.conf')
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
