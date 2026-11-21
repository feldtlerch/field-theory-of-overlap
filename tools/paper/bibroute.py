"""Derive the published bibliography inputs for the pinned embedded engine."""
import argparse
import difflib
import json
import pathlib
import re
import subprocess
import os
import shutil
from common import require, digest, read, write, timed, runtime

INPUTS = ["tools/paper/bibroute.py", "tools/paper/common.py", "documents/paper/references.bib", "documents/paper-build/fontconfig.conf"]

STOCK_BIB = '280fcfc82c0b4c49c152542b18d1cfd430b1bdbadda6207ffb3ee37f3021f003'
DERIVED_BIB = '8e4f8f413998e962b4d78c38e4c639d3b22e8abdd5236c9a678ef8b01be38cfa'


def derive(bib_path, output):
    # The style is the bundle's stock biblatex.bst, checked against the support lock
    # by preparation. The retirement command's integer probe checks that the pinned
    # engine still accepts the entry-local integer assignment during ITERATE that this
    # style relies on; derivation does not run it.
    stock_bib = bib_path.read_bytes()
    require(digest(stock_bib) == STOCK_BIB, 'Published bibliography hash differs')
    # Tectonic 0.17.0, embedded BibTeX 0.99d, crates/engine_bibtex/src/exec.rs:
    # text.length$ counts this nested accent group as 12 characters where
    # BibTeX 0.99d counts a special character as one (8 in total), with no error
    # diagnostic. The outer grouping is typographically redundant. Retire only
    # when the accent probe terminates and prints 8 unchanged; its runner has a timeout.
    bib = stock_bib.decode('utf-8')
    require(bib.count(r'Fran{\c{c}}ois') == 1, 'Accent occurrence count differs')
    bib = bib.replace(r'Fran{\c{c}}ois', r'Fran\c{c}ois')
    # Literal short authors supply the disambiguating initials while leaving
    # full author fields and bibliography name formatting untouched.
    names = {'a:sencapability':'A. Sen', 'a:senidentity':'A. Sen',
             'a:senrational':'A. K. Sen', 'a:sensocialchoice':'A. K. Sen',
             'a:fisher':'I. Fisher', 'a:vmf':'R. A. Fisher'}
    for key, name in names.items():
        pattern = r'(@[A-Za-z]+\{' + re.escape(key) + r',\n)'
        bib, count = re.subn(pattern, lambda m: m[1]+'  shortauthor = {{'+name+'}},\n', bib)
        require(count == 1, 'Short-author entry differs: '+key)
    # This compound needs a discretionary break in its second component.
    require(bib.count('Multi-Dimensioned') == 1, 'Title compound occurrence differs')
    bib = bib.replace('Multi-Dimensioned', r'Multi-Dimen\-sioned')
    require(digest(bib.encode('utf-8')) == DERIVED_BIB, 'Derived bibliography hash differs')
    output.mkdir(parents=True, exist_ok=True)
    (output/'references.bib').write_bytes(bib.encode('utf-8'))
    before = stock_bib.decode('utf-8')
    write(output/'bibliography.diff', ''.join(difflib.unified_diff(before.splitlines(True),bib.splitlines(True),fromfile='stock/bibliography',tofile='derived/bibliography')))
    record = dict(stock_bibliography=STOCK_BIB, derived_bibliography=DERIVED_BIB)
    write(output/'derivation.json', json.dumps(record,indent=2)+'\n')
    return record


def retirement(repository, support, runtime_lock, output, timeout):
    settings, lock, engine, bundle = runtime(repository, support, runtime_lock)
    require(not output.exists(), 'Probe output must not exist')
    tests = {
        'entry-integer': ('ENTRY {title} {flag} {} FUNCTION {misc} {}\nFUNCTION {test} { #1 \'flag := flag int.to.str$ write$ newline$ } READ ITERATE {test}\n', '1'),
        'accent-length': ('ENTRY {title} {} {} FUNCTION {misc} {}\nFUNCTION {test} { "Fran{\\c{c}}ois" text.length$ int.to.str$ write$ newline$ } READ ITERATE {test}\n', '8'),
    }
    env = dict(settings['environment'], SOURCE_DATE_EPOCH=str(settings['source-date-epoch']), FORCE_SOURCE_DATE=settings['force-source-date'])
    results = []
    for name, (style, expected) in tests.items():
        work = output/name
        work.mkdir(parents=True)
        for key,value in settings['work-environment'].items():
            path=work/value;path.mkdir(parents=True,exist_ok=True);env[key]=str(path)
        shutil.copyfile(repository/'documents/paper-build/fontconfig.conf', work/'paper/fontconfig.conf')
        write(work/(name+'.bst'), style)
        write(work/'one.bib', '@misc{one,title={One}}\n')
        write(work/(name+'.tex'), '\\documentclass{article}\n\\begin{document}Probe.\\nocite{*}\\bibliographystyle{'+name+'}\\bibliography{one}\\end{document}\n')
        argv = [str(engine), '--bundle', str(bundle), '--only-cached', '--untrusted', '--keep-logs', '--keep-intermediates', '--reruns', '0', name+'.tex']
        with timed(name):
            try:
                result = subprocess.run(argv, cwd=work, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
                output_bytes = result.stdout
                bbl = read(work/(name+'.bbl')).strip() if (work/(name+'.bbl')).exists() else ''
                blg = read(work/(name+'.blg')) if (work/(name+'.blg')).exists() else 'error: missing log'
                if result.returncode or re.search(r'error|warning', blg, re.I): status = 'engine error'
                elif bbl != expected: status = 'wrong output'
                else: status = 'fixed'
            except subprocess.TimeoutExpired as error:
                output_bytes = error.stdout or b''
                status = 'timeout'
        (work/'engine.txt').write_bytes(output_bytes)
        results.append(dict(test=name, expected=expected, status=status, timeout=timeout, command=argv))
    write(output/'retirement.json', json.dumps(results, indent=2)+'\n')
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    derive_parser = sub.add_parser('derive')
    for name in ['bibliography', 'output']: derive_parser.add_argument(name, type=pathlib.Path)
    retire = sub.add_parser('retirement')
    retire.add_argument('--repository', type=pathlib.Path, default=pathlib.Path(__file__).resolve().parents[2])
    retire.add_argument('--support', type=pathlib.Path, default=os.environ.get('FTOO_ASSEMBLY_SUPPORT'))
    retire.add_argument('--runtime-lock', default='documents/paper-build/runtime.toml')
    retire.add_argument('--output', type=pathlib.Path, default=pathlib.Path('paper-retirement'))
    retire.add_argument('--timeout', type=int, default=30)
    args = parser.parse_args()
    with timed(args.command):
        if args.command == 'derive': print(json.dumps(derive(args.bibliography, args.output), indent=2))
        else:
            require(args.support is not None, 'Set FTOO_ASSEMBLY_SUPPORT or --support')
            retirement(args.repository.resolve(), args.support.resolve(), args.runtime_lock, args.output.resolve(), args.timeout)
