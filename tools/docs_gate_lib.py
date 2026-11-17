"""Prove a clean candidate commit preserves a base's document inputs and bytes."""
import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import resource
import stat
import subprocess
import sys
import time

RECEIPT = 'generated/paper/inputs.json'
LOCK = 'tools/assembly/native-lock.toml'
PREPARE = 'tools/assembly/prepare-native.py'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def git(root, *args):
    start = time.monotonic()
    try:
        return subprocess.check_output(
            ['git', '--no-replace-objects', '-c', 'core.fsmonitor=false', '-C', str(root), *args],
            env=dict(os.environ, GIT_OPTIONAL_LOCKS='0'))
    finally:
        print(f'git {args[0]} wall={time.monotonic() - start:.6f}s', file=sys.stderr)


def base_tree(root, base):
    commit = git(root, 'rev-parse', '--verify', '--end-of-options', base + '^{commit}').decode().strip()
    entries = git(root, 'ls-tree', '-rz', commit).split(b'\0')
    tree = {}
    for entry in filter(None, entries):
        header, path = entry.split(b'\t', 1)
        mode, kind, oid = header.split()
        if kind != b'blob':
            raise ValueError(f'{path!r}: submodules require the full gate')
        tree[path.decode()] = (mode.decode(), oid.decode())
    # One batch preserves the exact object bytes without checking out the base.
    start = time.monotonic()
    data = subprocess.check_output(
        ['git', '--no-replace-objects', '-c', 'core.fsmonitor=false',
         '-C', str(root), 'cat-file', '--batch'],
        env=dict(os.environ, GIT_OPTIONAL_LOCKS='0'),
        input=''.join(oid + '\n' for _, oid in tree.values()).encode())
    print(f'git cat-file wall={time.monotonic() - start:.6f}s', file=sys.stderr)
    at = 0
    result = {}
    for path, (mode, _) in tree.items():
        end = data.index(b'\n', at)
        size = int(data[at:end].split()[2])
        result[path] = (mode, data[end + 1:end + 1 + size])
        at = end + size + 2
    return commit, result


def working_tree(root, base):
    names = set(base)
    # Ignoring an input does not remove it from a generator's filesystem view.
    names.update(filter(None, git(root, 'ls-files', '-z', '--cached', '--others')
                        .decode().split('\0')))
    result = {}
    for name in sorted(names):
        path = root / name
        # Do not follow a replaced parent directory into another tree.
        if any(p.is_symlink() for p in path.parents if p != root):
            raise ValueError(f'{name}: linked parent requires the full gate')
        try:
            mode = path.lstat().st_mode
        except FileNotFoundError:
            continue
        if not stat.S_ISREG(mode):
            raise ValueError(f'{name}: nonregular entry requires the full gate')
        result[name] = ('100755' if mode & 0o111 else '100644', path.read_bytes())
    return result


def record_path(path):
    """Record locations are permissions, never the definition of an input."""
    p = Path(path)
    return (path in ('LICENSE-CODE', 'LICENSE-DOCS')
            or (p.parent == Path('.') and p.suffix == '.md')
            or (path.startswith('docs/') and p.suffix == '.md')
            or (path.startswith('licenses/') and p.suffix in ('.txt', '.md'))
            or (path.startswith('tools/') and p.name == 'README.md'))


def clean_head(root):
    if git(root, 'status', '--porcelain=v1', '-z', '--untracked-files=all', '--ignored=matching'):
        raise ValueError(f'{root}: checkout differs from HEAD, including untracked or ignored files')
    return git(root, 'rev-parse', '--verify', 'HEAD^{commit}').decode().strip()


def require_base_copy(root, base, tree):
    gate_root = Path(__file__).resolve().parents[1]
    if gate_root.is_relative_to(root):
        raise ValueError('load the separate base checkout\'s gate; a tree never judges itself')
    if clean_head(gate_root) != base:
        raise ValueError('the running gate must come from a clean checkout of BASE')
    for name in ('docs_gate_lib.py', 'docs_gate.py'):
        if Path(__file__).with_name(name).read_bytes() != tree['tools/' + name][1]:
            raise ValueError(f'tools/{name}: running gate bytes differ from BASE')


def candidate_tree(root, base):
    head = clean_head(root)
    git(root, 'merge-base', '--is-ancestor', base, head)
    _, tree = base_tree(root, head)
    if working_tree(root, tree) != tree:
        raise ValueError('candidate bytes or modes differ from HEAD')
    return head, tree


def classify(base, current):
    """Pure classification; never execute code from the candidate tree."""
    refused = {}
    accepted = {}
    receipt = json.loads(base[RECEIPT][1])
    inputs = receipt['inputs']
    for path, expected in inputs.items():
        if path not in base or digest(base[path][1]) != expected:
            refused[path] = 'base receipt does not match base bytes'
    source_list = 'documents/paper-build/sources.tsv'
    inputs = set(inputs) | set(base[source_list][1].decode().splitlines()) | {LOCK, PREPARE}
    changed = sorted(p for p in base.keys() | current.keys()
                     if base.get(p) != current.get(p))
    for path in changed:
        old, new = base.get(path), current.get(path)
        if old and new and old[0] != new[0]:
            refused[path] = 'file mode changed'
        elif path in inputs:
            refused[path] = 'recorded paper input, closed source-list member or protected support input'
        elif path.startswith(('crates/', 'documents/', 'units/', 'registry/', 'generated/')):
            refused[path] = 'emitter, authority, source, notation, template or generated output'
        elif record_path(path):
            accepted[path] = 'record outside document input inventory'
        else:
            refused[path] = 'no input-equivalence proof; use the full gate'
    # Every generated file, including the receipt, must remain byte-identical.
    proofs = {}
    for path in sorted(base.keys() | current.keys()):
        if not path.startswith('generated/'):
            continue
        old, new = base.get(path), current.get(path)
        if old != new:
            refused[path] = 'generated bytes or inventory differ from base'
        elif old:
            proofs[path] = digest(old[1])
    return dict(accepted=accepted, refused=refused, output_sha256=proofs)


def cgroup_sample():
    membership = Path('/proc/self/cgroup').read_text().strip()
    relative = next((line[3:] for line in membership.splitlines()
                     if line.startswith('0::')), None)
    if relative is None:
        return {'scope': membership, 'unavailable': 'cgroup v2 not mounted'}
    root = Path('/sys/fs/cgroup') / relative.lstrip('/')
    values = {}
    for name in ('cpu.stat', 'cpu.pressure', 'memory.pressure', 'io.pressure',
                 'memory.current', 'memory.peak'):
        try:
            text = (root / name).read_text()
            if name.startswith('memory.') and name != 'memory.pressure':
                values[name] = int(text)
            elif name.endswith('.pressure'):
                for line in text.splitlines():
                    kind, *fields = line.split()
                    for field in fields:
                        key, value = field.split('=')
                        values[f'{name}.{kind}.{key}'] = float(value) if '.' in value else int(value)
            else:
                for line in text.splitlines():
                    key, value = line.split()
                    values[f'{name}.{key}'] = int(value)
        except OSError as error:
            values[name] = f'unavailable: {error}'
    return {'scope': str(root), 'membership': membership, 'values': values}


def usage():
    own = resource.getrusage(resource.RUSAGE_SELF)
    children = resource.getrusage(resource.RUSAGE_CHILDREN)
    return own.ru_utime + children.ru_utime, own.ru_stime + children.ru_stime


@contextlib.contextmanager
def measured(name):
    start = time.monotonic()
    cpu = usage()
    before = cgroup_sample()
    try:
        yield
    finally:
        after = cgroup_sample()
        end_cpu = usage()
        delta = {k: v - before.get('values', {}).get(k, v)
                 for k, v in after.get('values', {}).items()
                 if isinstance(v, (int, float))
                 and isinstance(before.get('values', {}).get(k), (int, float))}
        print(json.dumps(dict(battery=name, wall=time.monotonic() - start,
                              user=end_cpu[0] - cpu[0], system=end_cpu[1] - cpu[1],
                              cgroup_start=before, cgroup_end=after,
                              cgroup_delta=delta)), flush=True)


def batteries(root):
    env = dict(os.environ, CARGO_NET_OFFLINE='true', PYTHONDONTWRITEBYTECODE='1')
    # Support absence makes the support_required naming contract fail closed.
    for key in list(env):
        if key.startswith('FTOO_'):
            del env[key]
    namespace = ['unshare', '--user', f'--map-user={os.getuid()}',
                 f'--map-group={os.getgid()}', '--net']
    commands = [
        ('format', ['cargo', 'fmt', '--all', '--', '--check']),
        ('lint', ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings']),
        ('labels-self-test', ['python3', '-I', 'tools/check_labels.py', '--self-test']),
        ('labels', ['python3', '-I', 'tools/check_labels.py']),
        ('gate-tests', ['python3', '-B', '-m', 'unittest', 'discover', '-s', 'tools', '-p', 'test_docs_gate.py']),
        ('engine-free-tests', ['cargo', 'test', '--workspace', '--locked', '--no-fail-fast', '--', '--skip', 'support_required']),
    ]
    failures = []
    for name, command in commands:
        with measured(name):
            try:
                subprocess.run(namespace + command, cwd=root, env=env, check=True)
            except subprocess.CalledProcessError as error:
                failures.append(f'{name}: exit {error.returncode}')
    if failures:
        raise ValueError('failed batteries: ' + '; '.join(failures))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('base')
    parser.add_argument('--repository', type=Path, default=Path.cwd())
    parser.add_argument('--classify-only', action='store_true')
    args = parser.parse_args()
    with measured('docs-only-gate'):
        try:
            root = args.repository.resolve(strict=True)
            with measured('classification'):
                commit, base = base_tree(root, args.base)
                require_base_copy(root, commit, base)
                head, current = candidate_tree(root, commit)
                result = classify(base, current)
                print(json.dumps(result, sort_keys=True))
                print(f'base={commit}; output proofs are SHA-256, not a fresh rendering verdict')
                print(f'candidate={head}', flush=True)
            if result['refused']:
                print('REFUSED: use the full gate')
                return 2
            if args.classify_only:
                print('ELIGIBLE: classification only; batteries not run')
                return 3
            target = os.environ.get('CARGO_TARGET_DIR')
            if not target or not Path(target).is_absolute() or Path(target).resolve().is_relative_to(root):
                raise ValueError('set CARGO_TARGET_DIR to an absolute build directory outside the repository')
            batteries(root)
            with measured('final-content-proof'):
                if clean_head(root) != head or working_tree(root, current) != current:
                    raise ValueError('tree changed during gate; retry on a stable tree')
                print(json.dumps({'output_sha256': result['output_sha256']}, sort_keys=True))
            print(f'PASS: candidate={head}; input equivalence and fast batteries; base verdict inherited')
            return 0
        except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
            print(f'REFUSED: {error}; use the full gate', file=sys.stderr)
            return 2
