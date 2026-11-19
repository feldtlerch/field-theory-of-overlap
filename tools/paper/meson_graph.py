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
    meson.project_source_root() / 'paper.tex', '@OUTPUT@'])
document_record = custom_target('document-record', output: 'document-record.json',
  build_always_stale: true,
  command: [python, meson.project_source_root() / 'step.py', 'record',
    meson.project_source_root() / 'generated/documents/main/en.md',
    meson.project_source_root() / 'generated/documents/explainer-pilot/en.labels.md',
    meson.project_source_root() / 'generated/documents/explainer-pilot/en.md',
    meson.project_source_root() / 'generated/documents/paper-pilot/en.labels.md',
    meson.project_source_root() / 'generated/documents/paper-pilot/en.md', '@OUTPUT@'])
document_check = custom_target('document-check', input: document_record,
  output: 'documents.checked', command: [python, meson.project_source_root() / 'step.py',
    'verify', '@INPUT@', '@OUTPUT@'])
producer = custom_target('producer', input: [record, document_check], output: 'product.txt',
  command: [python, meson.project_source_root() / 'step.py', 'produce',
    '@INPUT0@', '@OUTPUT@'])
consumer = custom_target('consumer', input: producer, output: 'verdict.txt',
  command: [python, meson.project_source_root() / 'step.py', 'consume',
    '@INPUT@', '@OUTPUT@'])
'''

STEP = '''import hashlib
import pathlib
import sys

action, *inputs = sys.argv[1:]
source = pathlib.Path(inputs[0])
output = pathlib.Path(inputs[-1])
if action == 'record':
    data = hashlib.sha256(b'\\0'.join(pathlib.Path(item).read_bytes()
        for item in inputs[:-1])).hexdigest().encode()
elif action == 'verify':
    source.read_bytes()
    data = b'documents checked\\n'
    with (output.parent / 'document-check.calls').open('a') as log:
        log.write('verify\\n')
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
    (source / 'paper.tex').write_text('alpha')
    (source / 'generated/documents/main').mkdir(parents=True)
    (source / 'generated/documents/main/en.md').write_text('first')
    (source / 'generated/documents/explainer-pilot').mkdir(parents=True)
    (source / 'generated/documents/explainer-pilot/en.labels.md').write_text('first')
    (source / 'generated/documents/explainer-pilot/en.md').write_text('first')
    (source / 'generated/documents/paper-pilot').mkdir(parents=True)
    (source / 'generated/documents/paper-pilot/en.labels.md').write_text('first')
    (source / 'generated/documents/paper-pilot/en.md').write_text('first')
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
    document = source / 'generated/documents/explainer-pilot/en.labels.md'
    document.write_text('second')
    compile(build)
    assert count(build / 'document-check.calls') == 2
    assert count(build / 'producer.calls') == 1
    print('Changed Markdown: check reran; producer stayed at one call')
    document.write_text('first')
    compile(build)
    assert count(build / 'document-check.calls') == 3
    assert count(build / 'producer.calls') == 1
    print('Restored Markdown: check reran; producer stayed at one call')
    item = source / 'paper.tex'
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
    (replacement / 'paper.tex').write_text('alpha')
    (replacement / 'paper.tex').replace(item)
    compile(build)
    assert count(build / 'producer.calls') == 1
    item.write_text('bravo')
    os.utime(item, ns=(old.st_atime_ns, old.st_mtime_ns))
    compile(build)
    assert count(build / 'producer.calls') == 2
    print('G1 same TeX bytes: zero producer calls; changed TeX bytes with restored time: one call')
    real_documents_record(parent, adapter)
    cargo_freshness(parent, adapter, cargo_home)


def real_documents_record(parent, adapter):
    repo = parent / 'record-probe'
    support = parent / 'record-support'
    notation = parent / 'notation-support'
    support.joinpath('runtime/bin').mkdir(parents=True)
    (support / 'runtime/bin/tectonic').write_bytes(b'engine')
    notation.joinpath('runtime/bin').mkdir(parents=True)
    (notation / 'runtime/bin/node').write_bytes(b'notation engine')
    # These fixture inputs follow the document loader, notation sweep, native support, and generated-document readers.
    values = {
        'units/documents.toml': b'documents',
        'units/statements/one.toml': b'statement',
        'units/skeletons/one.toml': b'skeleton',
        'units/displays/one.toml': b'display',
        'registry/documentary-tags.toml': b'events',
        'documents/paper-build/label-mints.tsv': b'mints',
        'generated/manifest.json': b'manifest',
        'generated/audit-report.json': b'audit',
        'generated/documents/main/en.md': b'document',
        'generated/notation.tex': b'notation',
        'generated/addressing.tex': b'addressing',
        'tools/assembly/native-lock.toml': b'lock',
        'tools/assembly/prepare-native.py': b'prepare',
        'tools/notation/package-lock.json': b'package lock',
        'tools/notation/package.json': b'package',
        'tools/notation/support-digests.json': b'digests',
        'tools/notation/.node-version': b'node',
        'tools/notation/check.cjs': b'notation check',
        '.hidden/deeper/mint.md': b'markdown mint',
    }
    for name, data in values.items():
        path = repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    output = parent / 'documents-record.json'

    def refresh(notation_path=notation, allowed=(0,)):
        return run('python3', str(adapter), 'documents-record', '--repository', str(repo),
                   '--support', str(support), '--notation-support', str(notation_path),
                   '--output', str(output), allowed=allowed)

    refresh()
    original = output.read_bytes()
    original_time = output.stat().st_mtime_ns
    recorded = json.loads(original)
    assert set(values) == recorded['inputs'].keys()
    assert recorded['support']['engine']['sha256'] == hashlib.sha256(b'engine').hexdigest()
    assert recorded['support']['notation_engine']['sha256'] == hashlib.sha256(b'notation engine').hexdigest()
    assert recorded['adapter'] == hashlib.sha256(adapter.read_bytes()).hexdigest()
    refresh()
    assert output.read_bytes() == original and output.stat().st_mtime_ns == original_time
    for name, data in values.items():
        path = repo / name
        stamp = path.stat()
        path.write_bytes(data + b' changed')
        os.utime(path, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
        refresh()
        assert json.loads(output.read_bytes())['inputs'][name] != recorded['inputs'][name], name
        path.write_bytes(data)
        os.utime(path, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
        refresh()
        assert output.read_bytes() == original, name
    for name in ('generated/audit-report.json', 'documents/paper-build/label-mints.tsv'):
        path = repo / name
        path.unlink()
        refresh()
        assert json.loads(output.read_bytes())['inputs'][name] is None
        path.write_bytes(values[name])
        refresh()
        assert output.read_bytes() == original
    added_unit = repo / 'units/statements/added.toml'
    unit_directory_stamp = added_unit.parent.stat()
    added_unit.write_bytes(b'new statement')
    os.utime(added_unit.parent, ns=(unit_directory_stamp.st_atime_ns,
                                    unit_directory_stamp.st_mtime_ns))
    refresh()
    assert 'units/statements/added.toml' in json.loads(output.read_bytes())['inputs']
    added_unit.unlink()
    os.utime(added_unit.parent, ns=(unit_directory_stamp.st_atime_ns,
                                    unit_directory_stamp.st_mtime_ns))
    refresh()
    assert output.read_bytes() == original
    added = repo / '.hidden/deeper/added.md'
    directory_stamp = added.parent.stat()
    added.write_bytes(b'new mint')
    os.utime(added.parent, ns=(directory_stamp.st_atime_ns, directory_stamp.st_mtime_ns))
    refresh()
    assert '.hidden/deeper/added.md' in json.loads(output.read_bytes())['inputs']
    added.unlink()
    os.utime(added.parent, ns=(directory_stamp.st_atime_ns, directory_stamp.st_mtime_ns))
    refresh()
    assert output.read_bytes() == original
    invalid = repo / '.hidden/deeper/bad.md'
    invalid.mkdir()
    assert 'Invalid Markdown input entry' in refresh(allowed=(1,)).stderr
    invalid.rmdir()
    refresh()
    assert output.read_bytes() == original
    required = repo / 'generated/notation.tex'
    required.unlink()
    assert 'Missing document input entry: generated/notation.tex' in refresh(allowed=(1,)).stderr
    required.write_bytes(values['generated/notation.tex'])
    generated = repo / 'generated/documents'
    generated.rename(repo / 'generated/documents.saved')
    assert 'Missing document input directory: generated/documents' in refresh(allowed=(1,)).stderr
    (repo / 'generated/documents.saved').rename(generated)
    displays = repo / 'units/displays'
    (displays / 'one.toml').unlink()
    displays.rmdir()
    displays.symlink_to('absent-displays')
    assert 'Invalid document input directory: units/displays' in refresh(allowed=(1,)).stderr
    displays.unlink()
    displays.mkdir()
    (displays / 'one.toml').write_bytes(values['units/displays/one.toml'])
    refresh()
    assert output.read_bytes() == original
    engine = support / 'runtime/bin/tectonic'
    engine.unlink()
    assert 'Missing assembly engine' in refresh(allowed=(1,)).stderr
    engine.write_bytes(b'engine')
    engine.write_bytes(b'changed engine')
    refresh()
    assert json.loads(output.read_bytes())['support']['engine']['sha256'] != recorded['support']['engine']['sha256']
    engine.write_bytes(b'engine')
    refresh()
    assert output.read_bytes() == original
    notation_engine = notation / 'runtime/bin/node'
    notation_engine.unlink()
    assert 'Missing notation engine' in refresh(allowed=(1,)).stderr
    notation_engine.write_bytes(b'notation engine')
    notation_engine.write_bytes(b'changed notation engine')
    refresh()
    assert json.loads(output.read_bytes())['support']['notation_engine']['sha256'] != recorded['support']['notation_engine']['sha256']
    notation_engine.write_bytes(b'notation engine')
    refresh()
    assert output.read_bytes() == original
    other_notation = parent / 'other-notation-support'
    other_notation.joinpath('runtime/bin').mkdir(parents=True)
    (other_notation / 'runtime/bin/node').write_bytes(b'notation engine')
    refresh(other_notation)
    assert output.read_bytes() != original
    refresh()
    assert output.read_bytes() == original
    print('G1 real documents record: all input categories, hidden Markdown, membership, invalid entry, support context, equal cutoff')


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
