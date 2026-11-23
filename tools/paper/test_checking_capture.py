"""Exercise stored provenance claims and live checking-capture bindings."""

import argparse
import copy
import json
import pathlib
import shutil
import subprocess
import sys
from contextlib import contextmanager

import checking_capture as capture


FIXTURES = pathlib.Path(__file__).parent / 'fixtures'


def load(path):
    return json.loads(path.read_text(encoding='utf-8'))


def refuses(action, message):
    try:
        action()
    except ValueError as error:
        if message not in str(error):
            raise AssertionError(f'Wrong refusal: {error}') from error
        return
    raise AssertionError(f'Accepted instead of refusing: {message}')


@contextmanager
def changed_file(path, suffix):
    original = path.read_bytes()
    path.write_bytes(original + suffix)
    try:
        yield
    finally:
        path.write_bytes(original)


def stored_controls():
    pair = load(FIXTURES / 'checking-capture-provenance.json')
    if pair.get('schema') != 1:
        raise AssertionError('Unsupported stored provenance pair schema')
    left, right = pair['base'], pair['source-note']
    for side in (left, right):
        capture.validate_envelope_record(side['envelope'], side['capture'])
        if side['envelope']['instrumentation']['variant'] != 'base':
            raise AssertionError('Source mutation changed the checker instrumentation')
    baseline = load(FIXTURES / 'checking-capture.json')
    if any(side['capture']['records'] != baseline['records'] for side in (left, right)):
        raise AssertionError('Stored pair differs from the established outcomes')
    if capture.compare(left['capture'], right['capture']):
        raise AssertionError('Source note changed a checking outcome')
    different = capture.compare_provenance(left['envelope'], right['envelope'])
    if ('prepared_inputs.parts/coda.tex' not in different or
            'instrumentation.inputs.parts/coda.tex' not in different):
        raise AssertionError('Legitimate source provenance was not distinguished')
    refuses(lambda: capture.validate_envelope_record(right['envelope'], left['capture']),
            'Stale or swapped capture envelope')
    broken = copy.deepcopy(left['envelope'])
    broken['successful_output'].pop('pdf_sha256')
    refuses(lambda: capture.validate_envelope_record(broken, left['capture']),
            'Missing successful output inventory')
    broken = copy.deepcopy(left['envelope'])
    broken['prepared_inputs'].pop('parts/coda.tex')
    refuses(lambda: capture.validate_envelope_record(broken, left['capture']),
            'Malformed prepared input inventory')
    broken = copy.deepcopy(left['capture'])
    broken['run']['engine_exit'] = 1
    refuses(lambda: capture.validate_envelope_record(left['envelope'], broken),
            'Incomplete typeset log')
    print('Stored provenance controls passed')


def live_controls(repository, support, area):
    def side(name):
        work = area / name
        return load(work / 'capture.json'), load(work / 'envelope.json'), (
            repository, support, work)

    base = side('base')
    source_note = side('source-note')
    outcomes, provenance = capture.bound_pair(*base, *source_note)
    if outcomes or 'prepared_inputs.parts/coda.tex' not in provenance:
        raise AssertionError('Live equal outcomes lost their distinct source provenance')
    swapped = copy.deepcopy(source_note[1])
    refuses(lambda: capture.bound_pair(base[0], swapped, base[2], *source_note),
            'Stale or swapped capture envelope')
    output = base[2][2] / 'paper/main.pdf'
    missing = output.with_suffix('.pdf.absent')
    output.rename(missing)
    try:
        refuses(lambda: capture.bound_pair(*base, *source_note),
                'Missing or changed successful output: pdf_sha256')
    finally:
        missing.rename(output)
    consumed = source_note[2][2] / 'prepared-inputs/parts/coda.tex'
    with changed_file(consumed, b'\n% changed consumed input\n'):
        refuses(lambda: capture.bound_pair(*base, *source_note),
                'Changed consumed input: parts/coda.tex')
    for field, value, message in (
            ('revision', '0' * 40, 'Source revision differs from envelope'),
            ('support.engine_sha256', '0' * 64, 'Support identity differs from envelope'),
            ('instrumentation.tool_sha256', '0' * 64,
             'Capture instrument differs from envelope'),
            ('instrumentation.tap_sha256', '0' * 64,
             'Checking tap differs from envelope')):
        envelope = copy.deepcopy(base[1])
        owner = envelope
        names = field.split('.')
        for name in names[:-1]:
            owner = owner[name]
        owner[names[-1]] = value
        refuses(lambda: capture.validate_envelope(envelope, base[0], *base[2]),
                message)
    inert = side('base-inert')
    relabelled = copy.deepcopy(inert[1])
    relabelled['instrumentation']['variant'] = 'base'
    relabelled['instrumentation']['tap_sha256'] = base[1]['instrumentation']['tap_sha256']
    refuses(lambda: capture.validate_envelope(relabelled, inert[0], *inert[2]),
            'Instrumented input differs from variant: ftoo_typed_tools.tex')
    shortened = copy.deepcopy(relabelled)
    for name in capture.INSTRUMENTED:
        shortened['prepared_inputs'].pop(name)
        shortened['instrumentation']['inputs'].pop(name)
    refuses(lambda: capture.validate_envelope(shortened, inert[0], *inert[2]),
            'Malformed prepared input inventory')
    shortened = copy.deepcopy(base[1])
    shortened['prepared_inputs'].pop('data/ftoo_index.tex')
    shortened['instrumentation']['inputs'].pop('data/ftoo_index.tex')
    refuses(lambda: capture.validate_envelope(shortened, base[0], *base[2]),
            'Prepared input inventory differs from retained copy')
    shortened = copy.deepcopy(base[1])
    shortened['support']['bundle_files'].pop(next(iter(shortened['support']['bundle_files'])))
    refuses(lambda: capture.validate_envelope(shortened, base[0], *base[2]),
            'Support identity differs from envelope')
    refuses(lambda: capture.bound_pair(*base, *inert),
            'instrumentation.tap_sha256')
    checker_note = side('checker-provenance')
    refuses(lambda: capture.bound_pair(*base, *checker_note),
            'prepared_inputs.ftoo_typed_tools.tex')
    changed_capture = copy.deepcopy(base[0])
    changed_capture['records'][-1]['claimed_grade'] = 'feas'
    changed_envelope = copy.deepcopy(base[1])
    changed_envelope['capture_sha256'] = capture.canonical_sha256(changed_capture)
    refuses(lambda: capture.validate_envelope(changed_envelope, changed_capture, *base[2]),
            'Checking events differ from capture')
    with changed_file(base[2][2] / 'paper/parts/coda.tex', b'\n% changed engine input\n'):
        refuses(lambda: capture.validate_envelope(base[1], base[0], *base[2]),
                'Instrumented input differs: parts/coda.tex')
    with changed_file(base[2][2] / 'paper/data/ftoo_index.tex', b'\n% changed data\n'):
        refuses(lambda: capture.validate_envelope(base[1], base[0], *base[2]),
                'Changed consumed input: data/ftoo_index.tex')
    if capture.bound_pair(*base, *source_note)[0]:
        raise AssertionError('Restored live pair differs')
    print('Live pair and provenance refusals passed')


def dirty_controls(repository, support, area):
    copied = area / 'dirty-repository'
    shutil.copytree(repository, copied,
                    ignore=shutil.ignore_patterns('.review-inputs', '.design', '__pycache__'))
    source = copied / 'documents/paper/parts/coda.tex'
    source.write_bytes(source.read_bytes() + b'\n% candidate source note\n')
    if capture.preparation_changes(repository) or capture.preparation_changes(copied) != [
            'documents/paper/parts/coda.tex']:
        raise AssertionError('Disposable candidate did not isolate its dirty input')
    work = area / 'dirty-check'
    command = [sys.executable, '-B', str(repository / 'tools/paper/checking_capture.py'),
               'check', str(copied), str(support), str(work)]
    result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8')
    if (result.returncode or 'Checking capture equal' not in result.stdout or
            'Provenance envelope omitted:' not in result.stderr or
            (work / 'envelope.json').exists()):
        raise AssertionError(f'Dirty candidate did not retain the outcome gate: {result}')
    captured = load(work / 'capture.json')
    refuses(lambda: capture.validate_envelope_record(None, captured),
            'Unsupported provenance envelope schema')
    pair = subprocess.run(
        [sys.executable, '-B', str(repository / 'tools/paper/checking_capture.py'),
         'pair', str(repository), str(support), str(area / 'base'),
         str(copied), str(support), str(work)],
        capture_output=True, text=True, encoding='utf-8')
    if pair.returncode == 0 or 'Missing provenance envelope:' not in pair.stderr:
        raise AssertionError(f'Pair accepted a dirty capture without an envelope: {pair}')
    print('Dirty candidate outcome and provenance controls passed')


def active_controls(area):
    base = load(area / 'base/capture.json')
    if base['records'][7]['derived_rows'][0]['tag'] != 'e:capability':
        raise AssertionError('Economics control row missing')
    clean_inert = load(area / 'base-inert/capture.json')
    if capture.compare(base, clean_inert):
        raise AssertionError('Clean inert checker changed a baseline outcome')
    for body in ('paragraph', 'refrain'):
        for effect in ('spend', 'grant'):
            changed = load(area / f'close-{body}-{effect}/capture.json')
            differences = capture.compare(base, changed)
            expected = [('subsec:coda:close', 'grant_set')] if effect == 'grant' else [
                ('subsec:coda:close', 'spend_set'),
                ('subsec:coda:close', 'diagnostics')]
            if differences != expected:
                raise AssertionError(f'{body} {effect} changed {differences}, not {expected}')
    active = load(area / 'close-paragraph-spend/capture.json')
    inert = load(area / 'close-paragraph-spend-inert/capture.json')
    if [item['identity'] for item in active['records'][-1]['diagnostics']] != [
            'ungraded-prose'] or inert['records'][-1]['diagnostics']:
        raise AssertionError('Section checker warning did not disappear under inert control')
    if capture.compare(active, inert) != [('subsec:coda:close', 'diagnostics')]:
        raise AssertionError('Inert checker retained the active warning')
    for body in ('paragraph', 'refrain'):
        refusal = load(area / f'close-{body}-refusal/capture.json')
        if (refusal['run']['status'] != 'refused' or
                refusal['run']['diagnostic']['scope'] != 'subsec:coda:close'):
            raise AssertionError(f'{body} checker refusal missing')
        refuses(lambda: capture.compare(base, refusal), 'Incomplete or refused typeset')
    wall = load(area / 'coda-spend/capture.json')
    if capture.compare(base, wall) != [('subsec:coda:wall', 'claimed_grade'),
                                      ('subsec:coda:wall', 'spend_set')]:
        raise AssertionError('Wall spend control differs')
    wall = load(area / 'coda-grant/capture.json')
    if capture.compare(base, wall) != [('subsec:coda:wall', 'grant_set')]:
        raise AssertionError('Wall grant control differs')
    wall = load(area / 'coda-refusal/capture.json')
    if wall['run']['status'] != 'refused':
        raise AssertionError('Wall checker refusal missing')
    print('Active closing and economics controls passed')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--live', nargs=3, metavar=('REPOSITORY', 'SUPPORT', 'AREA'),
                        type=pathlib.Path)
    args = parser.parse_args()
    stored_controls()
    if args.live:
        repository, support, area = args.live
        live_controls(repository, support, area)
        active_controls(area)
        dirty_controls(repository, support, area)


if __name__ == '__main__':
    main()
