"""Run the complete plain-conversion conjunction for one commit pair.

The assignment is captured once by the comparison owner. This driver uses
the resulting checked value to select both immutable revisions. All proof
products are made during this invocation and discarded with its work area.
"""

import argparse
import json
import pathlib
import subprocess
import sys
import time
import tomllib

import checking_capture
import compare_words


class GateFault(Exception):
    """A missing, changed or failed complete-check proof."""

    def __init__(self, code, detail):
        super().__init__(detail)
        self.code = code


def run_command(*argv, cwd=None):
    result = subprocess.run(argv, cwd=cwd, capture_output=True, text=True)
    if result.returncode:
        raise GateFault('ProofFailed',
                        f'{argv[0]} exited {result.returncode}: {result.stderr}')
    return result.stdout


def structure(parent_root, repo, assignment):
    """The first form adds one plain occurrence without changing its surroundings."""
    unit_path = assignment['unit-path']
    unit_key = assignment['unit']
    if (parent_root / unit_path).exists():
        raise GateFault('StructuralCorrespondenceMismatch',
                        'assigned primary unit already exists in parent')
    unit = tomllib.loads((repo / unit_path).read_text(encoding='utf-8'))
    body = unit.get('unit', {})
    if (set(unit) != {'schema', 'unit'} or unit['schema'] != 1 or
            set(body) != {'id', 'document', 'math', 'texts'} or
            body['id'] != unit_key or body['document'] != 'explainer-pilot' or
            body['math'] != {} or set(body['texts']) != {'en'} or
            not isinstance(body['texts']['en'], str) or
            '@' in body['texts']['en']):
        raise GateFault('UnsupportedForm', 'assigned unit is not plain explainer prose')
    source_lines = (parent_root / assignment['source']).read_text(
        encoding='utf-8').splitlines()
    first, last = assignment['first'], assignment['last']
    if not 1 <= first <= last <= len(source_lines):
        raise GateFault('AssignmentInvalid', 'assigned source interval is unavailable')
    authored = ' '.join(' '.join(source_lines[first - 1:last]).split())
    if body['texts']['en'] != authored:
        raise GateFault('StructuralCorrespondenceMismatch',
                        'unit text differs from selected authored source')
    scope_path = pathlib.Path('units/scopes/explainer.toml')
    old_scope = tomllib.loads((parent_root / scope_path).read_text(encoding='utf-8'))
    new_scope = tomllib.loads((repo / scope_path).read_text(encoding='utf-8'))
    row = {'unit': unit_key, 'role': 'paragraph'}
    rows = new_scope['scope']['occurrences']
    if rows.count(row) != 1:
        raise GateFault('StructuralCorrespondenceMismatch',
                        'assigned scope occurrence is not unique')
    rows = [item for item in rows if item != row]
    new_scope['scope']['occurrences'] = rows
    if new_scope != old_scope:
        raise GateFault('StructuralCorrespondenceMismatch',
                        'scope changed outside assigned occurrence')
    skeleton_path = pathlib.Path('units/skeletons/explainer-pilot.toml')
    old = tomllib.loads((parent_root / skeleton_path).read_text(encoding='utf-8'))
    new = tomllib.loads((repo / skeleton_path).read_text(encoding='utf-8'))
    child = {'kind': 'paragraph', 'unit': unit_key}
    found = sum(section['children'].count(child)
                for section in new['skeleton']['sections'])
    if found != 1:
        raise GateFault('StructuralCorrespondenceMismatch',
                        'assigned skeleton child is not unique')
    for section in new['skeleton']['sections']:
        section['children'] = [item for item in section['children'] if item != child]
    if new != old:
        raise GateFault('StructuralCorrespondenceMismatch',
                        'skeleton changed outside assigned child')


def root_at(repo, revision, destination):
    run_command('git', 'clone', '--quiet', '--no-checkout', '--shared',
                str(repo), str(destination))
    run_command('git', '-C', str(destination), 'checkout', '--quiet', revision)
    actual = compare_words.git(destination, 'rev-parse', 'HEAD').decode().strip()
    if actual != revision:
        raise GateFault('InputChanged', 'parent checkout differs from request')


def checked_registers(binary, root, support):
    try:
        run_command(str(binary), 'documents-check-gaps', '--repository', str(root),
                    '--support', str(support))
    except GateFault as error:
        if 'differs from the canonical' in str(error):
            raise GateFault('RegisterMismatch', str(error)) from error
        raise


def fresh_pair(binary, root, support, work):
    run_command(str(binary), 'produce', '--repository', str(root),
                '--support', str(support), '--work', str(work / '.private' / 'work'),
                '--output-dir', str(work))


def checked_screens(binary, root, support, pair):
    run_command(str(binary), 'check-registered-html-gaps', '--repository', str(root),
                '--support', str(support), '--built', str(pair))


def checked_capture(root, support, work):
    work.mkdir(parents=True)
    independent = work / 'independent-preparation'
    checking_capture.prepare(root, support, independent)
    independent_inputs = checking_capture.input_digests(independent)
    capture_work = work / 'capture'
    result = checking_capture.capture(root, support, capture_work, 'base')
    stored = checking_capture.write_capture(capture_work, result, root, support)
    envelope = json.loads((capture_work / 'envelope.json').read_text(encoding='utf-8'))
    if envelope['prepared_inputs'] != independent_inputs:
        raise GateFault('CheckingSnapshotMismatch',
                        'capture differs from independently prepared inputs')
    return stored, envelope, (root, support, capture_work)


def compare_pair(repo, parent, assignment, captured, work, binary, support):
    for name in ('paper', 'explainer'):
        data = compare_words.git(repo, 'show', parent +
                                 ':generated/paper/' + name + '.pdf')
        (work / ('before-' + name + '.pdf')).write_bytes(data)
    args = argparse.Namespace(
        repository=repo, output=work / 'comparison',
        before_revision=None,
        before_source=(f"{parent}:{assignment['source']}:"
                       f"{assignment['first']}-{assignment['last']}"),
        unit=assignment['unit'],
        assignment=captured[1],
        before_paper=work / 'before-paper.pdf',
        before_explainer=work / 'before-explainer.pdf',
        after_paper=repo / 'generated/paper/paper.pdf',
        after_explainer=repo / 'generated/paper/explainer.pdf')
    verdict = compare_words.compare(args, captured_assignment=captured,
                                    paper_binary=binary, support=support)
    evidence = json.loads((args.output / 'comparison.json').read_text(encoding='utf-8'))
    if (verdict or evidence.get('scope_admissibility', {}).get('verdict') !=
            'scope-admissible'):
        raise GateFault('ComparisonIncomplete',
                        'complete comparison or scope admission refused: ' +
                        json.dumps(evidence.get('faults', []), sort_keys=True))
    if evidence.get('source', {}).get('kind') != 'authored':
        raise GateFault('UnsupportedForm', 'complete profile needs authored parent paragraph')
    return evidence


def run(repo, assignment_path, support, binary, work):
    repo = repo.resolve(strict=True)
    support = support.resolve(strict=True)
    binary = binary.resolve(strict=True)
    work = work.resolve()
    if work == repo or repo in work.parents:
        raise GateFault('InputInvalid', 'complete-check work must be outside the candidate')
    if work.exists():
        raise GateFault('OutputExists', 'complete-check work area exists')
    captured = compare_words.capture_assignment(repo, assignment_path)
    assignment = captured[0]
    if 'occurrences' in assignment and len(assignment['occurrences']) != 1:
        raise GateFault('UnsupportedForm', 'complete profile has one occurrence')
    if assignment['source'] != 'documents/paper/explainer/explainer.tex':
        raise GateFault('UnsupportedForm', 'complete profile is one plain explainer paragraph')
    work.mkdir(parents=True)
    parent_root = work / 'parent'
    root_at(repo, assignment['parent'], parent_root)
    tracked = {}
    for name in ('paper.pdf', 'explainer.pdf', 'inputs.json'):
        path = repo / 'generated/paper' / name
        tracked[name] = path.read_bytes()
    phases = {}
    phase_start = time.perf_counter()
    checked_registers(binary, repo, support)
    phases['register_check'] = time.perf_counter() - phase_start
    phase_start = time.perf_counter()
    compare_pair(repo, assignment['parent'], assignment, captured, work,
                 binary, support)
    checked_registers(binary, parent_root, support)
    structure(parent_root, repo, assignment)
    phases['comparisons'] = time.perf_counter() - phase_start
    phase_start = time.perf_counter()
    before = checked_capture(parent_root, support, work / 'parent-checking')
    phases['parent_capture'] = time.perf_counter() - phase_start
    phase_start = time.perf_counter()
    fresh_pair(binary, repo, support, work / 'candidate-pair')
    phases['candidate_pair'] = time.perf_counter() - phase_start
    phase_start = time.perf_counter()
    checked_screens(binary, repo, support, work / 'candidate-pair')
    phases['candidate_screens'] = time.perf_counter() - phase_start
    phase_start = time.perf_counter()
    after = checked_capture(repo, support, work / 'candidate-checking')
    phases['candidate_capture'] = time.perf_counter() - phase_start
    outcomes, provenance = checking_capture.bound_pair(*before, *after)
    if outcomes:
        raise GateFault('CheckingOutcomeChanged', str(outcomes))
    for name, original in tracked.items():
        if (repo / 'generated/paper' / name).read_bytes() != original:
            raise GateFault('InputChanged', 'submitted paper output changed: ' + name)
    if captured[1].read_bytes() != captured[2]:
        raise GateFault('InputChanged', 'assignment changed after capture')
    if compare_words.git(repo, 'rev-parse', 'HEAD').decode().strip() != assignment['candidate']:
        raise GateFault('InputChanged', 'candidate commit changed during check')
    if compare_words.git(repo, 'status', '--porcelain', '-z'):
        raise GateFault('InputChanged', 'candidate checkout changed during check')
    if (compare_words.git(parent_root, 'rev-parse', 'HEAD').decode().strip() !=
            assignment['parent'] or
            compare_words.git(parent_root, 'status', '--porcelain', '-z')):
        raise GateFault('InputChanged', 'parent checkout changed during check')
    return {'verdict': 'complete', 'parent': assignment['parent'],
            'candidate': assignment['candidate'], 'checking_provenance': provenance,
            'phase_walls': phases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('repository', 'assignment', 'support', 'binary', 'work'):
        parser.add_argument('--' + name, required=True, type=pathlib.Path)
    args = parser.parse_args()
    try:
        result = run(args.repository, args.assignment, args.support,
                     args.binary, args.work)
    except (GateFault, compare_words.Fault, OSError, ValueError) as error:
        code = getattr(error, 'code', None)
        if code is None and isinstance(error, compare_words.Fault):
            code = error.row['code']
        print('M083: ' + (code or 'ProofFailed') + ': ' + str(error), file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
