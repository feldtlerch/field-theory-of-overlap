"""Capture and read the existing T40 identity-control fixture."""
import argparse
import json
import pathlib
import shutil
import subprocess
import sys

INPUTS = ['tools/paper/meson_t40.py', 'tools/paper/test_identity_tex.py',
          'generated/manifest.json', 'registry/documentary-tags.toml',
          'documents/paper-build/label-mints.tsv',
          'documents/paper-build/self-description.json']


def write(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + '\n')


def fixture(args):
    private = pathlib.Path(args.private)
    work = private / 'identity-controls'
    if work.exists():
        previous = private / 'identity-controls.previous'
        if previous.exists():
            shutil.rmtree(previous)
        work.rename(previous)
    result = subprocess.run([sys.executable, '-B', str(pathlib.Path(args.repository) /
                            'tools/paper/test_identity_tex.py'), args.repository, args.support,
                             str(work)], capture_output=True, timeout=300)
    rows = []
    if work.exists():
        for file in sorted(work.glob('*/engine.txt')):
            rows.append({'fixture': file.parent.name, 'engine_stream': file.read_text(errors='replace')})
    write(pathlib.Path(args.output), {'status': result.returncode,
          'stdout': result.stdout.decode(errors='replace'),
          'stderr': result.stderr.decode(errors='replace'), 'fixtures': rows})


def assertion(args):
    record = json.loads(pathlib.Path(args.input).read_text())
    names = {row['fixture'] for row in record['fixtures']}
    expected = {'leak-control', 'shipped', 'copies', 'math', 'locator', 'layout-sites',
                'concordance', 'concordance-differs', 'missing'}
    passed = record['status'] == 0 and names == expected
    write(pathlib.Path(args.output), {'verdict': 'pass' if passed else 'refusal',
          'reason': '' if passed else 'T40 identity fixtures incomplete or failed'})


def evidence(args):
    record = json.loads(pathlib.Path(args.input).read_text())
    if record['verdict'] != 'pass':
        raise SystemExit(record['reason'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('fixture', 'assert', 'evidence'))
    parser.add_argument('--repository')
    parser.add_argument('--support')
    parser.add_argument('--private')
    parser.add_argument('--input')
    parser.add_argument('--output')
    args = parser.parse_args()
    {'fixture': fixture, 'assert': assertion, 'evidence': evidence}[args.action](args)


if __name__ == '__main__':
    main()
