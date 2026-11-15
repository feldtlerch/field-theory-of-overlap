"""Enumerate the paper driver's declared inputs and prepare its clean source area."""
import argparse
import ast
import json
import pathlib
import subprocess
import tomllib
from common import digest, read, require, runtime, timed
from prepare import prepare

INPUTS = ['tools/paper/inputs.py', 'tools/paper/prepare.py', 'tools/paper/common.py']


def inventory(repo):
    paths = set(INPUTS)
    queue = list(paths)
    while queue:
        path = queue.pop()
        if path.endswith('.py'):
            for node in ast.parse(read(repo/path)).body:
                if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'INPUTS' for t in node.targets):
                    for value in ast.literal_eval(node.value):
                        if value not in paths: paths.add(value); queue.append(value)
    paths.update(read(repo/'documents/paper-build/sources.tsv').splitlines())
    settings = tomllib.loads(read(repo/'documents/paper-build/runtime.toml'))
    paths.add(settings['support-lock'])
    tracked = set(subprocess.check_output(['git', 'ls-files', '-z'], cwd=repo).decode('utf-8').split('\0'))
    require(paths <= tracked, 'Untracked byte input: '+', '.join(sorted(paths-tracked)))
    return dict(settings=settings, inputs={p:digest((repo/p).read_bytes()) for p in sorted(paths)})


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository', type=pathlib.Path)
    parser.add_argument('--support', type=pathlib.Path)
    parser.add_argument('--work', type=pathlib.Path)
    args = parser.parse_args()
    with timed('paper-inputs'):
        data = inventory(args.repository)
        if args.work is not None:
            require(args.support is not None, 'Pinned support is required')
            settings, lock, engine, bundle = runtime(args.repository, args.support)
            for row in lock['bundle']['files']:
                require(digest((bundle/row['member']).read_bytes()) == row['sha256'], 'Bundle member differs: '+row['member'])
            prepare(args.repository, args.support, args.work)
            data.update(engine=str(engine), bundle=str(bundle))
        print(json.dumps(data, sort_keys=True))
