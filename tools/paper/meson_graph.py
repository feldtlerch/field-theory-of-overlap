"""G1 content cutoff and G2 publication checks on a nested Meson fixture."""
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile


FIXTURE = '''project('paper-graph-fixture', meson_version: '>=1.11.2')
python = find_program('python3')
record = custom_target('record', output: 'record.json', build_always_stale: true,
  command: [python, meson.project_source_root() / 'step.py', 'record',
    meson.project_source_root() / 'source.txt', '@OUTPUT@'])
producer = custom_target('producer', input: record, output: 'product.txt',
  command: [python, meson.project_source_root() / 'step.py', 'produce',
    '@INPUT@', '@OUTPUT@'])
consumer = custom_target('consumer', input: producer, output: 'verdict.txt',
  command: [python, meson.project_source_root() / 'step.py', 'consume',
    '@INPUT@', '@OUTPUT@'])
'''

STEP = '''import hashlib
import pathlib
import sys

action, source, output = sys.argv[1:]
source = pathlib.Path(source)
output = pathlib.Path(output)
if action == 'record':
    data = hashlib.sha256(source.read_bytes()).hexdigest().encode()
elif action == 'produce':
    data = source.read_bytes()
    with (output.parent / 'producer.calls').open('a') as log:
        log.write('produce\\n')
    if (output.parent / 'fail').exists():
        (output.parent / 'provisional').write_bytes(data)
        raise SystemExit(7)
else:
    data = source.read_bytes()
    with (output.parent / 'consumer.calls').open('a') as log:
        log.write('consume\\n')
if not output.exists() or output.read_bytes() != data:
    temporary = output.with_suffix('.next')
    temporary.write_bytes(data)
    temporary.replace(output)
'''

PUBLISH_FIXTURE = '''
published = custom_target('published', output: ['paper.pdf', 'explainer.pdf',
    'inputs.json', 'identity-ledger.json', 'pair.json'], build_always_stale: true,
  command: [python, meson.project_source_root() / 'publisher.py',
    '@PRIVATE_DIR@', '@OUTDIR@', '@0@', '@1@'])
'''

PUBLISHER = '''import pathlib
import shutil
import subprocess
import sys

private, output, binary, repo = sys.argv[1:]
private = pathlib.Path(private).resolve()
output = pathlib.Path(output).resolve()
repo = pathlib.Path(repo)
work = private / 'work'
if work.exists():
    shutil.rmtree(work)
work.mkdir(parents=True)
for name in ('paper.pdf', 'explainer.pdf', 'inputs.json'):
    if name != 'explainer.pdf' or not (private.parent / 'fail').exists():
        shutil.copyfile(repo / 'generated/paper' / name, work / name)
(work / 'identity-ledger.json').write_text('[]\\n')
(work / 'format.fmt').write_bytes(b'fixture format')
raise SystemExit(subprocess.call([binary, 'publish', '--repository', str(repo),
    '--work', str(work), '--output-dir', str(output)]))
'''


def run(*argv, cwd=None, allowed=(0,), env=None):
    result = subprocess.run(argv, cwd=cwd, capture_output=True, text=True, env=env)
    if result.returncode not in allowed:
        raise AssertionError(f'{argv!r}: exit {result.returncode}\n{result.stdout}\n{result.stderr}')
    return result


def count(path):
    return len(path.read_text().splitlines()) if path.exists() else 0


def fixture(parent, publish=None):
    source = parent / 'source'
    build = parent / 'build'
    source.mkdir()
    extra = ''
    if publish:
        binary, repo = publish
        extra = PUBLISH_FIXTURE.replace('@0@', str(binary)).replace('@1@', str(repo))
        (source / 'publisher.py').write_text(PUBLISHER)
    (source / 'meson.build').write_text(FIXTURE + extra)
    (source / 'step.py').write_text(STEP)
    (source / 'source.txt').write_text('alpha')
    run('meson', 'setup', str(build), str(source), '--backend=ninja', '--wrap-mode=nodownload')
    return source, build


def compile(build, allowed=(0,)):
    return run('ninja', '-C', str(build), '-j', '1', 'verdict.txt', allowed=allowed)


def g1(parent, adapter, cargo_home):
    source, build = fixture(parent)
    compile(build)
    first = (count(build / 'producer.calls'), count(build / 'consumer.calls'))
    compile(build)
    second = (count(build / 'producer.calls'), count(build / 'consumer.calls'))
    dry = run('ninja', '-C', str(build), '-n', '-d', 'explain', 'verdict.txt')
    ninja_file = (build / 'build.ninja').read_text()
    print('G1 restat emitted:', 'restat = 1' in ninja_file)
    print('G1 first and repeated calls:', first, second)
    print('G1 dry-run:', dry.stdout.strip(), dry.stderr.strip())
    assert first == second == (1, 1), 'Unchanged record restarted a producer'
    item = source / 'source.txt'
    old = item.stat()
    item.write_text('alpha')
    compile(build)
    assert count(build / 'producer.calls') == 1
    item.with_suffix('.next').write_text('alpha')
    item.with_suffix('.next').replace(item)
    compile(build)
    assert count(build / 'producer.calls') == 1
    replacement = source / 'replacement'
    replacement.mkdir()
    (replacement / 'source.txt').write_text('alpha')
    (replacement / 'source.txt').replace(item)
    compile(build)
    assert count(build / 'producer.calls') == 1
    item.write_text('bravo')
    os.utime(item, ns=(old.st_atime_ns, old.st_mtime_ns))
    compile(build)
    assert count(build / 'producer.calls') == 2
    print('G1 same bytes: zero producer calls; changed bytes with restored time: one call')
    cargo_freshness(parent, adapter, cargo_home)


def cargo_freshness(parent, adapter, cargo_home):
    crate = parent / 'cargo-probe'
    (crate / 'crates/ftoo_codegen/src').mkdir(parents=True)
    (crate / 'Cargo.toml').write_text('[workspace]\nmembers=["crates/*"]\nresolver="2"\n')
    shutil.copyfile(pathlib.Path(adapter).parents[2] / 'rust-toolchain.toml',
                    crate / 'rust-toolchain.toml')
    for member in ('ftoo_codegen', 'ftoo_manifest', 'ftoo_audit'):
        member_dir = crate / 'crates' / member
        member_dir.mkdir(exist_ok=True)
        (member_dir / 'Cargo.toml').write_text('[package]\nname="' + member +
            '"\nversion="0.1.0"\nedition="2021"\n' +
            ('[[bin]]\nname="paper"\npath="src/main.rs"\n' if member == 'ftoo_codegen' else ''))
        if member != 'ftoo_codegen':
            (member_dir / 'src').mkdir(exist_ok=True)
            (member_dir / 'src/lib.rs').write_text('pub fn marker() {}\n')
    path = crate / 'crates/ftoo_codegen/src/main.rs'
    path.write_text('fn main() { println!("first"); }\n')
    run('cargo', 'generate-lockfile', '--offline', cwd=crate)
    target = parent / 'cargo-target'
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_HOME=str(cargo_home))
    record = parent / 'rust-record.json'
    output = parent / 'adapter-paper'
    def adapter_action(action, result_path=output):
        command = ['python3', str(adapter), action, '--repository', str(crate),
                   '--cargo-target-dir', str(target), '--cargo-home', str(cargo_home),
                   '--output', str(record if action == 'rust-record' else result_path)]
        if action == 'codegen':
            command += ['--record', str(record)]
        return run(*command)
    run('cargo', 'build', '--release', '--offline', cwd=crate, env=env)
    adapter_action('rust-record')
    adapter_action('codegen')
    assert run(str(output)).stdout.strip() == 'first'
    stable_record = record.read_bytes()
    adapter_action('rust-record')
    assert record.read_bytes() == stable_record
    unchanged = adapter_action('codegen')
    assert 'Compiling ' not in unchanged.stderr, 'Unchanged adapter call compiled Rust'
    stamp = path.stat()
    path.write_text('fn main() { println!("second"); }\n')
    os.utime(path, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
    plain = run('cargo', 'build', '--release', '--offline', cwd=crate, env=env)
    direct = run(str(target / 'release/paper')).stdout.strip()
    print('G1 Cargo preserved-mtime plain:', direct,
          'rustc launched:', 'Compiling ftoo_codegen' in plain.stderr)
    assert direct == 'first'
    adapter_action('rust-record')
    fresh = parent / 'new-build/paper'
    adapter_action('codegen', fresh)
    assert run(str(fresh)).stdout.strip() == 'second', 'New build kept stale release executable'
    stamp = path.stat()
    path.write_text('fn main() { println!("third"); }\n')
    os.utime(path, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
    adapter_action('rust-record')
    adapter_action('codegen', fresh)
    assert run(str(fresh)).stdout.strip() == 'third', 'Retained adapter kept stale release executable'
    print('G1 real adapter: unchanged call compiled no Rust; no-state and changed-state release builds recovered')


def g2(parent, binary, repo):
    source, build = fixture(parent, (binary, repo))
    run('ninja', '-C', str(build), '-j', '1', 'pair.json')
    pair = build
    check = (str(binary), 'check', '--repository', str(repo), '--built', str(pair))
    run(*check)
    (pair / 'paper.pdf').write_bytes(b'corrupted')
    assert 'Pair output differs' in run(*check, allowed=(1,)).stderr
    run('ninja', '-C', str(build), '-j', '1', 'pair.json')
    run(*check)
    (pair / 'inputs.json').unlink()
    run(*check, allowed=(1,))
    run('ninja', '-C', str(build), '-j', '1', 'pair.json')
    private_name = json.loads((pair / 'pair.json').read_text())['private_dir']
    work = build / private_name / 'work'
    (work / 'format.fmt').write_bytes(b'corrupted format')
    assert 'Pair work evidence differs' in run(*check, allowed=(1,)).stderr
    run('ninja', '-C', str(build), '-j', '1', 'pair.json')
    run(*check)
    (build / 'fail').touch()
    original = (pair / 'paper.pdf').read_bytes()
    run('ninja', '-C', str(build), '-j', '1', 'pair.json', allowed=(1,))
    assert (pair / 'paper.pdf').read_bytes() == original
    (build / 'fail').unlink()
    run('ninja', '-C', str(build), '-j', '1', 'pair.json')
    run(*check)
    print('G2 failed producer retained old output; restored producer succeeded; tampered and absent outputs refused')


def main():
    base = pathlib.Path(sys.argv[2])
    base.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=base) as directory:
        if sys.argv[1] == 'G1':
            g1(pathlib.Path(directory), pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4]))
        else:
            binary = pathlib.Path(sys.argv[3])
            if not binary.is_absolute():
                binary = base.parent / binary
            g2(pathlib.Path(directory), binary, pathlib.Path(sys.argv[4]))


if __name__ == '__main__':
    main()
