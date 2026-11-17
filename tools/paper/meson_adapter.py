"""Content records and Cargo adapter for the Meson paper graph."""
import argparse
import ctypes
import ctypes.util
import hashlib
import json
import os
import pathlib
import shutil
import subprocess

from inputs import inventory


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def replace_if_changed(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and path.read_bytes() == data:
        return False
    temporary = path.with_name(path.name + '.next')
    temporary.write_bytes(data)
    temporary.replace(path)
    return True


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()


def tool(name):
    path = shutil.which(name)
    if path is None:
        raise RuntimeError('Missing host tool: ' + name)
    resolved = pathlib.Path(path).resolve()
    return {'path': str(resolved), 'sha256': digest(resolved)}


def pdf_libraries():
    for name in ('poppler-glib', 'glib-2.0', 'gobject-2.0'):
        library = ctypes.util.find_library(name)
        if library is None:
            raise RuntimeError('Missing PDF reader library: ' + name)
        ctypes.CDLL(library)
    paths = {line.rsplit(' ', 1)[-1].strip() for line in
             pathlib.Path('/proc/self/maps').read_text().splitlines()}
    return {path: digest(pathlib.Path(path)) for path in sorted(paths)
            if pathlib.Path(path).name.startswith(('libpoppler', 'libglib-2.0', 'libgobject-2.0'))}


def python_closure(repo, path):
    import ast
    found = {path}
    pending = [path]
    while pending:
        relative = pending.pop()
        if not relative.endswith('.py'):
            continue
        for node in ast.parse((repo / relative).read_bytes()).body:
            if isinstance(node, ast.Assign) and any(
                isinstance(target, ast.Name) and target.id == 'INPUTS'
                for target in node.targets
            ):
                for child in ast.literal_eval(node.value):
                    if child not in found:
                        found.add(child)
                        pending.append(child)
    return {name: digest(repo / name) for name in sorted(found)}


def paper_record(args):
    repo = pathlib.Path(args.repository)
    receipt = inventory(repo)
    closure = python_closure(repo, 'tools/paper/ledger.py')
    host = {name: tool(name) for name in ('meson', 'ninja', 'python3', 'cargo', 'rustc', 'node')}
    value = {'receipt': receipt, 'consumer_inputs': closure, 'host_tools': host,
             'pdf_libraries': pdf_libraries(), 'support': args.support,
             'adapter': digest(pathlib.Path(__file__))}
    replace_if_changed(pathlib.Path(args.output), canonical(value))


def t40_record(args):
    repo = pathlib.Path(args.repository)
    value = {'inputs': python_closure(repo, 'tools/paper/meson_t40.py')}
    replace_if_changed(pathlib.Path(args.output), canonical(value))


def depinfo(repo, target):
    file = target / 'release/paper.d'
    return sorted(dep_paths(repo, file)) if file.exists() else []


def dep_paths(repo, file):
    # Cargo's dep-info is a Make rule. Rust paths in this repository contain no spaces.
    text = file.read_text(errors='replace').replace('\\\n', ' ')
    if ':' not in text:
        return set()
    found = set()
    for token in text.split(':', 1)[1].split():
        path = pathlib.Path(token)
        absolute = path if path.is_absolute() else repo / path
        try:
            relative = absolute.resolve().relative_to(repo.resolve())
        except ValueError:
            continue
        found.add(str(relative))
    return found


def rust_record(args):
    repo = pathlib.Path(args.repository)
    target = pathlib.Path(args.cargo_target_dir)
    names = {'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'}
    names.update(str(path.relative_to(repo)) for path in repo.glob('crates/*/Cargo.toml'))
    names.update(depinfo(repo, target))
    config = [base / '.cargo' / name for base in (repo, *repo.parents)
              for name in ('config.toml', 'config')]
    config.extend(pathlib.Path(args.cargo_home) / name for name in ('config.toml', 'config'))
    value = {'inputs': {name: digest(repo / name) if (repo / name).exists() else None
                        for name in sorted(names)},
             'cargo_config': {str(path): digest(path) if path.exists() else None
                              for path in config},
             'cargo': tool('cargo'), 'rustc': tool('rustc'),
             'cargo_version': subprocess.check_output(['cargo', '-V'], cwd=repo).decode().strip(),
             'rustc_version': subprocess.check_output(['rustc', '-vV'], cwd=repo).decode().strip(),
             'environment': {key: value for key, value in sorted(os.environ.items())
                             if key in ('RUSTFLAGS', 'RUSTDOCFLAGS', 'RUSTUP_TOOLCHAIN',
                                        'CARGO_ENCODED_RUSTFLAGS', 'CARGO_INCREMENTAL')
                             or key.startswith(('RUSTC', 'CARGO_PROFILE_', 'CARGO_BUILD_'))
                             or (key.startswith('CARGO_TARGET_') and key != 'CARGO_TARGET_DIR')},
             'adapter': digest(pathlib.Path(__file__))}
    replace_if_changed(pathlib.Path(args.output), canonical(value))


def codegen(args):
    repo = pathlib.Path(args.repository)
    target = pathlib.Path(args.cargo_target_dir)
    record = digest(pathlib.Path(args.record))
    state = pathlib.Path(args.output).with_suffix('.record')
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_HOME=args.cargo_home)
    if not state.exists() or state.read_text() != record:
        for member in ('ftoo_codegen', 'ftoo_manifest', 'ftoo_audit'):
            subprocess.run(['cargo', 'clean', '--offline', '--release', '-p', member],
                           cwd=repo, env=env, check=True)
    subprocess.run(['cargo', 'build', '--locked', '--offline', '--release', '-p', 'ftoo_codegen',
                    '--bin', 'paper'], cwd=repo, env=env, check=True)
    binary = target / 'release' / 'paper'
    replace_if_changed(pathlib.Path(args.output), binary.read_bytes())
    pathlib.Path(args.output).chmod(0o755)
    replace_if_changed(state, record.encode())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('paper-record', 't40-record', 'rust-record', 'codegen'))
    parser.add_argument('--repository', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--support')
    parser.add_argument('--cargo-target-dir')
    parser.add_argument('--cargo-home')
    parser.add_argument('--record')
    args = parser.parse_args()
    {'paper-record': paper_record, 't40-record': t40_record,
     'rust-record': rust_record, 'codegen': codegen}[args.action](args)


if __name__ == '__main__':
    main()
