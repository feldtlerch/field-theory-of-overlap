"""Capture labelled audits from prepared paper in one typeset run.

An event belongs to the labelled scope current when it is emitted: a box
restores its enclosing subsection on closing. Grade, spend and grant snapshots,
diagnostics and rows follow this same rule. The selected scopes are the branch
and confluence subsections, the mod:branch, dyn:flow and src:nd boxes, and the
coda section and its four subsection audits. Row fields are the economics
writer's arguments; rendered table cells are not observed and need their own
comparison if the table is converted. An unselected box audit inside a selected
window refuses. Only the checker's l3msg warning and error forms are observed,
including shorter forms routed through their six-argument form; a checker error
yields a refused run, while package warnings, infos and notes are outside this
capture. Clean scopes cannot prove that spends, grants or the audit's judgement
remain live: rerun the spend, grant, diagnostic, open-spend
and refusal variants against each converted body. A rewritten box opening may
need a new anchor; an absent anchor refuses. The run uses the paper engine's
fixed rerun count, not the paper-and-explainer auxiliary fixed point.
"""
import argparse
import copy
import hashlib
import json
import pathlib
import re
import resource
import subprocess
import sys
import time

from common import runtime
from prepare import prepare


PART_B = pathlib.Path('documents/paper/parts/partB.tex')
CODA = pathlib.Path('documents/paper/parts/coda.tex')
SCOPES = (
    ('subsec:flow:branch', PART_B, r'\label{subsec:flow:branch}', 'section'),
    ('mod:branch', PART_B, r'\begin{statement}{mod:branch}', 'box'),
    ('subsec:flow:confluence', PART_B, r'\label{subsec:flow:confluence}', 'section'),
    ('dyn:flow', PART_B, r'\hdr{dyn:flow}', 'box'),
    ('src:nd', PART_B, r'\hdr{src:nd}', 'box'),
    ('sec:coda', CODA, r'\label{sec:coda}', 'section'),
    ('subsec:coda:tenses', CODA, r'\label{subsec:coda:tenses}', 'section'),
    ('subsec:coda:wall', CODA, r'\label{subsec:coda:wall}', 'section'),
    ('subsec:coda:words', CODA, r'\label{subsec:coda:words}', 'section'),
    ('subsec:coda:close', CODA, r'\label{subsec:coda:close}', 'section'),
)
ROW_SCOPES = {'e:dmp': 'subsec:flow:confluence',
              'e:capability': 'subsec:coda:wall'}
TAP = r'''
\iow_new:N \g_ftoo_capture_iow
\iow_open:Nn \g_ftoo_capture_iow {checking-capture.events}
\tl_new:N \g_ftoo_capture_scope_tl
\bool_new:N \g_ftoo_capture_section_bool
\cs_new_protected:Npn \ftoo_capture_scope:n #1
  { \tl_gset:Nn \g_ftoo_capture_scope_tl {#1}
    \iow_now:Nx \g_ftoo_capture_iow {FT-SCOPE|#1} }
\cs_new_protected:Npn \ftoo_capture_check:nn #1#2
  {
    \iow_now:Nx \g_ftoo_capture_iow
      {FT-CHECK|\tl_use:N \g_ftoo_capture_scope_tl|#1|#2|
       \seq_use:Nn \g__ftoo_spendlog_seq {,}|
       \seq_use:Nn \g__ftoo_grantlog_seq {,}}
  }
\cs_new_protected:Npn \ftoo_capture_end:
  { \iow_now:Nx \g_ftoo_capture_iow
      {FT-END|\tl_use:N \g_ftoo_capture_scope_tl} }
\cs_new_eq:NN \ftoo_capture_original_section: \ftoo_sec_audit:
\cs_set_protected:Npn \ftoo_sec_audit:
  {
    \bool_gset_true:N \g_ftoo_capture_section_bool
    \ftoo_capture_check:nn {section}{\tl_use:N \g__ftoo_secclaim_tl}
    \ftoo_capture_original_section:
    \ftoo_capture_end:
    \bool_gset_false:N \g_ftoo_capture_section_bool
  }
\cs_new_eq:NN \ftoo_capture_original_audit:n \ftoo_grade_audit:n
\cs_set_protected:Npn \ftoo_grade_audit:n #1
  {
    \bool_if:NF \g_ftoo_capture_section_bool
      {\ftoo_capture_check:nn {box}{#1}}
    \ftoo_capture_original_audit:n {#1}
    \bool_if:NF \g_ftoo_capture_section_bool {\ftoo_capture_end:}
  }
\cs_new_protected:Npn \ftoo_capture_row:nnnnn #1#2#3#4#5
  { \iow_now:Nx \g_ftoo_capture_iow
      {FT-ROW|\tl_use:N \g_ftoo_capture_scope_tl|\tl_to_str:n {#1}|
       \tl_to_str:n {#2}|\tl_to_str:n {#3}|
       \tl_to_str:n {#4}|\tl_to_str:n {#5}} }
\AddToHook{file/econ_membrane.tex/after}
  {
    \ExplSyntaxOn
    \cs_new_eq:NN \ftoo_capture_original_row:nnnnn \ftoo_write_row:nnnnn
    \cs_set_protected:Npn \ftoo_write_row:nnnnn #1#2#3#4#5
      {
        \ftoo_capture_row:nnnnn {#1}{#2}{#3}{#4}{#5}
        \ftoo_capture_original_row:nnnnn {#1}{#2}{#3}{#4}{#5}
      }
    \ExplSyntaxOff
  }
\cs_new_eq:NN \ftoo_capture_original_warning:nnnnnn \msg_warning:nnnnnn
\cs_set_protected:Npn \msg_warning:nnnnnn #1#2#3#4#5#6
  {
    \iow_now:Nx \g_ftoo_capture_iow
      {FT-DIAG|\tl_use:N \g_ftoo_capture_scope_tl|warning|#1|#2}
    \iow_term:x {FT-DIAG|\tl_use:N \g_ftoo_capture_scope_tl|warning|#1|#2}
    \ftoo_capture_original_warning:nnnnnn {#1}{#2}{#3}{#4}{#5}{#6}
  }
\cs_new_eq:NN \ftoo_capture_original_error:nnnnnn \msg_error:nnnnnn
\cs_set_protected:Npn \msg_error:nnnnnn #1#2#3#4#5#6
  {
    \iow_now:Nx \g_ftoo_capture_iow
      {FT-DIAG|\tl_use:N \g_ftoo_capture_scope_tl|error|#1|#2}
    \iow_term:x {FT-DIAG|\tl_use:N \g_ftoo_capture_scope_tl|error|#1|#2}
    \ftoo_capture_original_error:nnnnnn {#1}{#2}{#3}{#4}{#5}{#6}
  }
\AddToHook{enddocument/end}
  {\iow_now:Nn \g_ftoo_capture_iow {FT-COMPLETE}}
'''


def unique_replace(text, old, new):
    if text.count(old) != 1:
        raise ValueError(f'Expected one source anchor: {old}')
    return text.replace(old, new, 1)


def source_locations(sources):
    result = {}
    for name, path, anchor, kind in SCOPES:
        if sources[path].count(anchor) != 1:
            raise ValueError(f'Expected one source anchor: {anchor}')
        result[name] = {'file': str(path), 'label_token': anchor, 'kind': kind}
    return result


def mark_before_line(source, anchor, name):
    if source.count(anchor) != 1:
        raise ValueError(f'Expected one label token: {anchor}')
    position = source.rfind('\n', 0, source.index(anchor)) + 1
    marker = (r'\ExplSyntaxOn\ftoo_capture_scope:n {' + name +
              r'}\ExplSyntaxOff' + '\n')
    return source[:position] + marker + source[position:]


def mark_after(source, anchor, name):
    return unique_replace(source, anchor, anchor + '\n' +
                          r'\ExplSyntaxOn\ftoo_capture_scope:n {' + name +
                          r'}\ExplSyntaxOff')


def instrument_part_b(source, variant):
    variant = variant.removesuffix('-inert')
    if variant == 'grade':
        source = unique_replace(source, SCOPES[2][2] + '\n' +
                                r'\secgrade{cont-anch}%', SCOPES[2][2] + '\n' +
                                r'\secgrade{cont}%')
    if variant in ('spend', 'grant'):
        old = '\\end{econ}\n\n\\clearpage\n\\section{The readout}'
        if variant == 'spend':
            call = r'\ExplSyntaxOn\ftoo_spend:nn {c:Gstar}{capture}\ExplSyntaxOff'
        else:
            call = (r'\ExplSyntaxOn\clist_put_right:Ne '
                    r'\l__ftoo_grant_mask_clist {\tl_to_str:n {c:Gstar}}'
                    r'\ftoo_spend:nn {c:Gstar}{capture}\ExplSyntaxOff')
        source = unique_replace(source, old, call + '\n' + old)
    source_opening = r'\begin{definition}[\hdr{src:nd}]\grade{nec}'
    if variant == 'source-grade':
        source = unique_replace(source, source_opening,
                                r'\begin{definition}[\hdr{src:nd}]\grade{feas}')
    if variant == 'source-dropped':
        source = unique_replace(source, source_opening,
                                r'\begin{definition}[\hdr{src:nd}]')
    if variant == 'open-spend':
        source = unique_replace(source, source_opening,
                                r'\begin{definition}[\hdr{src:nd}]\grade{feas}' +
                                '\n' +
                                r'\ExplSyntaxOn\prop_gput:Nnn '
                                r'\g__ftoo_const_status_prop {c:lam}{open}' +
                                r'\ftoo_spend:nn {c:lam}{capture}' +
                                r'\ExplSyntaxOff')
    if variant == 'added-row':
        closing = '\\end{econ}\n\n\\clearpage\n\\section{The readout}'
        extra = (r'\ExplSyntaxOn\ftoo_write_row:nnnnn '
                 r'{annex}{e:bistab}{dyn:flow}{a:dmp}{control}\ExplSyntaxOff')
        source = unique_replace(source, closing,
                                r'\end{econ}' + '\n' + extra + '\n\n' +
                                r'\clearpage' + '\n' + r'\section{The readout}')
    if variant == 'unselected-box':
        opening = r'\begin{definition}[\hdr{src:nd}]\grade{nec}'
        extra = (r'\begin{definition}\grade{nec}' + '\n' +
                 '  Capture box.' + '\n' + r'\end{definition}' + '\n')
        source = unique_replace(source, opening, extra + opening)
    source = mark_after(source, SCOPES[0][2], SCOPES[0][0])
    source = mark_before_line(source, SCOPES[1][2], SCOPES[1][0])
    if variant == 'diagnostic':
        start = source.index(SCOPES[1][2])
        line_end = source.index('\n', start)
        source = (source[:line_end + 1] +
                  r'\ExplSyntaxOn\bool_set_true:N \g__ftoo_strict_bool\ExplSyntaxOff' +
                  '\n' + source[line_end + 1:])
    start = source.index(SCOPES[1][2])
    end = source.index(r'\end{statement}', start)
    source = (source[:end] + source[end:].replace(r'\end{statement}',
              r'\end{statement}' + '\n' +
              r'\ExplSyntaxOn\ftoo_capture_scope:n {' + SCOPES[0][0] +
              r'}\ExplSyntaxOff', 1))
    source = mark_after(source, SCOPES[2][2], SCOPES[2][0])
    source = mark_before_line(source, SCOPES[3][2], SCOPES[3][0])
    start = source.index(SCOPES[3][2])
    end = source.index(r'\end{definition}', start)
    if variant.startswith('refusal'):
        source = source[:end] + r'\ExplSyntaxOn\ftoo_spend:nn {c:lam}{capture}\ExplSyntaxOff' + (
            '\n' + source[end:])
        end = source.index(r'\end{definition}', start)
    source = source[:end] + source[end:].replace(r'\end{definition}',
        r'\end{definition}' + '\n' +
        r'\ExplSyntaxOn\ftoo_capture_scope:n {' + SCOPES[2][0] + r'}\ExplSyntaxOff', 1)
    source = mark_before_line(source, SCOPES[4][2], SCOPES[4][0])
    start = source.index(SCOPES[4][2])
    end = source.index(r'\end{definition}', start)
    restoration = ''
    if variant == 'open-spend':
        restoration = (r'\ExplSyntaxOn\prop_gput:Nnn '
                       r'\g__ftoo_const_status_prop {c:lam}{held}' +
                       r'\ExplSyntaxOff' + '\n')
    source = source[:end] + source[end:].replace(
        r'\end{definition}', r'\end{definition}' + '\n' +
        restoration + r'\ExplSyntaxOn\ftoo_capture_scope:n {' +
        SCOPES[2][0] + r'}\ExplSyntaxOff', 1)
    source = unique_replace(source, r'\section{The readout}', r'\section{The readout}' + '\n' +
                            r'\ExplSyntaxOn\ftoo_capture_scope:n {outside}\ExplSyntaxOff')
    return source


def instrument_coda(source):
    source = mark_after(source, r'\section{Truth and flourishing}', 'sec:coda')
    for name, _, anchor, _ in SCOPES[6:]:
        source = mark_after(source, anchor, name)
    return source


def parse_events(events, log, locations):
    lines = events.splitlines()
    if not lines or lines[-1] != 'FT-COMPLETE':
        raise ValueError('Truncated checking event log')
    found = {}
    rows = {name: [] for name in locations}
    seen_known_rows = set()
    active = None
    for line in lines[:-1]:
        parts = [part.strip() for part in line.split('|')]
        if parts[0] == 'FT-CHECK':
            if len(parts) != 6 or active is not None:
                raise ValueError('Malformed check event')
            name, kind, grade, spends, grants = parts[1:]
            if name in locations:
                if locations[name]['kind'] != kind:
                    raise ValueError(f'Unselected box or wrong audit kind: {name}')
                if name in found:
                    raise ValueError(f'Duplicate audit record: {name}')
                found[name] = {'scope': name, 'location': locations[name],
                               'claimed_grade': grade,
                               'spend_set': sorted(set(filter(None, spends.split(',')))),
                               'grant_set': sorted(set(filter(None, grants.split(',')))),
                               'diagnostics': [], 'derived_rows': [], 'end_marker': False}
            active = name
        elif parts[0] == 'FT-END':
            if len(parts) != 2 or active != parts[1]:
                raise ValueError('Unmatched check end event')
            if active in found:
                found[active]['end_marker'] = True
            active = None
        elif parts[0] == 'FT-DIAG':
            if len(parts) != 5:
                raise ValueError('Malformed diagnostic event')
            if parts[3] != 'ftoo':
                continue
            if parts[1] not in locations:
                continue
            if parts[1] != active:
                raise ValueError('Diagnostic outside its check')
            if active not in found:
                raise ValueError(f'Diagnostic has no captured audit: {active}')
            marker = '|'.join(parts)
            at = log.find(marker)
            if at < 0:
                raise ValueError('Diagnostic marker absent from engine log')
            diagnostic = re.match(
                r'\r?\n\r?\n(?:! )?Package ftoo (?:Warning|Error):[ \t]*(.*?)(?:\n\n|FT-END|\Z)',
                log[at + len(marker):], re.S)
            if diagnostic is None:
                raise ValueError('Diagnostic text absent from engine log')
            wording = re.sub(r'(?m)^\(ftoo\)\s*', '', diagnostic.group(1))
            found[active]['diagnostics'].append({
                'kind': parts[2], 'module': parts[3], 'identity': parts[4],
                'text': ' '.join(wording.split())})
        elif parts[0] == 'FT-ROW':
            if len(parts) != 7:
                raise ValueError('Malformed economics row')
            scope, departure, tag, onref, cites, stopsat = parts[1:]
            if tag in ROW_SCOPES and scope != ROW_SCOPES[tag]:
                raise ValueError(f'Economics row scope differs: {tag}')
            if scope in locations:
                if tag in seen_known_rows:
                    raise ValueError(f'Duplicate economics row: {tag}')
                if tag in ROW_SCOPES:
                    seen_known_rows.add(tag)
                rows[scope].append({'table': 'tbl:log:concordance',
                                    'departure': departure, 'tag': tag,
                                    'onref': onref, 'cites': cites, 'stopsat': stopsat})
        elif parts[0] != 'FT-SCOPE':
            raise ValueError(f'Unknown checking event: {parts[0]}')
    if active is not None:
        raise ValueError('Unclosed check event')
    if seen_known_rows != set(ROW_SCOPES):
        raise ValueError('Missing economics row')
    for scope, emitted in rows.items():
        if emitted and scope not in found:
            raise ValueError(f'Missing row owner audit: {scope}')
        if scope in found:
            found[scope]['derived_rows'] = emitted
    missing = [name for name, _, _, _ in SCOPES if name not in found]
    if missing:
        raise ValueError(f'Missing check record: {missing}')
    return [found[name] for name, _, _, _ in SCOPES]


def classify_refusal(log, engine_stream):
    marker = 'FT-DIAG|dyn:flow|error|ftoo|grade-too-strong'
    if log.count(marker) != 1 or marker not in engine_stream:
        raise ValueError('Failed typeset lacks the named checker refusal')
    if not re.search(r'Output written on main\.xdv \(\d+ pages, \d+ bytes\)\.\s*\Z', log):
        raise ValueError('Refusal log is truncated or differs')
    if ('Transcript written on main.log.' not in engine_stream or
            'caused by: halted on potentially-recoverable error as specified'
            not in engine_stream):
        raise ValueError('Engine did not report a completed checker refusal')
    match = re.match(r'\r?\n\r?\n! Package ftoo Error:[ \t]*(.*?)(?:\n\n|\Z)',
                     log[log.index(marker) + len(marker):], re.S)
    if match is None:
        raise ValueError('Checker refusal text absent')
    wording = re.sub(r'(?m)^\(ftoo\)\s*', '', match.group(1))
    return {'scope': 'dyn:flow', 'kind': 'error', 'module': 'ftoo',
            'identity': 'grade-too-strong', 'text': ' '.join(wording.split())}


def cgroup_snapshot(cgroup):
    def read(name):
        path = cgroup / name
        return path.read_text(encoding='utf-8') if path.is_file() else None
    stat = read('cpu.stat')
    cpu = dict(line.split() for line in stat.splitlines()) if stat else {}
    return {'cpu': {key: int(value) for key, value in cpu.items()},
            'memory_current_bytes': int(value) if (value := read('memory.current')) else None,
            'memory_peak_bytes': int(value) if (value := read('memory.peak')) else None,
            'cpu_pressure': read('cpu.pressure'),
            'memory_pressure': read('memory.pressure'), 'io_pressure': read('io.pressure')}


def pressure_totals(text):
    if text is None:
        return None
    return {name: int(match.group(1)) for name in ('some', 'full')
            if (match := re.search(rf'(?m)^{name} .*?total=(\d+)', text))}


def resources(start, cgroup_line, before, after):
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    keys = ('usage_usec', 'user_usec', 'system_usec', 'nr_throttled', 'throttled_usec')
    delta = {key: after['cpu'][key] - before['cpu'][key]
             if key in before['cpu'] and key in after['cpu'] else None for key in keys}
    pressure = {}
    for name in ('cpu', 'memory', 'io'):
        prior = pressure_totals(before[name + '_pressure'])
        later = pressure_totals(after[name + '_pressure'])
        pressure[name] = ({key: later[key] - prior[key] for key in prior.keys() & later.keys()}
                          if prior is not None and later is not None else None)
    return {'scope': cgroup_line, 'wall_seconds': round(time.monotonic() - start, 3),
            'child_user_seconds': round(usage.ru_utime, 3),
            'child_system_seconds': round(usage.ru_stime, 3),
            'child_peak_kib': usage.ru_maxrss,
            'cgroup_cpu_deltas': delta, 'cgroup_pressure_total_deltas': pressure,
            'cgroup_before': before, 'cgroup_after': after}


def capture(repository, support, work, variant):
    work = work.resolve()
    start = time.monotonic()
    if work.exists():
        raise ValueError('Capture work area must be new')
    cgroup_line = (pathlib.Path('/proc/self/cgroup').read_text(encoding='utf-8')
                   .strip().split(':')[-1])
    cgroup = pathlib.Path('/sys/fs/cgroup') / cgroup_line.lstrip('/')
    before = cgroup_snapshot(cgroup)
    paper = work / 'paper'
    prepare(repository, support, paper)
    sources = {path: (paper / path.relative_to('documents/paper')).read_text(encoding='utf-8')
               for path in (PART_B, CODA)}
    locations = source_locations(sources)
    typed = paper / 'ftoo_typed_tools.tex'
    tap = TAP
    if variant.endswith('-inert'):
        tap += '\n' + r'\cs_set_protected:Npn \ftoo_capture_original_audit:n #1 {}' + '\n'
    if variant == 'refusal-observed':
        tap = tap.replace(
            r'\ftoo_capture_original_error:nnnnnn {#1}{#2}{#3}{#4}{#5}{#6}',
            r'\ftoo_capture_original_warning:nnnnnn {#1}{#2}{#3}{#4}{#5}{#6}')
    typed.write_text(typed.read_text(encoding='utf-8') + tap, encoding='utf-8')
    (paper / 'parts/partB.tex').write_text(
        instrument_part_b(sources[PART_B], variant), encoding='utf-8')
    (paper / 'parts/coda.tex').write_text(instrument_coda(sources[CODA]), encoding='utf-8')
    appendices = paper / 'parts/appendices.tex'
    appendices.write_text(unique_replace(appendices.read_text(encoding='utf-8'),
        r'\section{Recovery}',
        r'\section{Recovery}' + '\n' +
        r'\ExplSyntaxOn\ftoo_capture_scope:n {outside}\ExplSyntaxOff'), encoding='utf-8')
    settings, _, engine, bundle = runtime(repository, support)
    environment = dict(settings['environment'])
    environment.update(SOURCE_DATE_EPOCH=str(settings['source-date-epoch']),
                       FORCE_SOURCE_DATE=settings['force-source-date'])
    for key, value in settings['work-environment'].items():
        path = work / value
        path.mkdir(parents=True, exist_ok=True)
        environment[key] = str(path)
    command = [str(engine), '--bundle', str(bundle), *settings['flags'], 'main.tex']
    if variant == 'first':
        command[command.index('--reruns') + 1] = '0'
    with (work / 'engine.txt').open('wb') as output:
        result = subprocess.run(command, cwd=paper, env=environment,
                                stdout=output, stderr=subprocess.STDOUT,
                                timeout=settings['invocation-timeout'])
    log_path = paper / 'main.log'
    output_path = paper / 'main.pdf'
    event_path = paper / 'checking-capture.events'
    if not log_path.is_file():
        raise ValueError('Missing typeset log')
    log = log_path.read_text(encoding='utf-8', errors='replace')
    engine_stream = (work / 'engine.txt').read_text(encoding='utf-8', errors='replace')
    log_complete = bool(re.search(
        r'Output written on main\.xdv \(\d+ pages, \d+ bytes\)\.\s*\Z', log))
    if result.returncode:
        refusal = classify_refusal(log, engine_stream)
        records = []
        run = {'status': 'refused', 'engine_exit': result.returncode,
               'output_sha256': None, 'log_complete': log_complete,
               'diagnostic': refusal}
    else:
        if not log_complete:
            raise ValueError('Incomplete typeset log')
        if not event_path.is_file():
            raise ValueError('Missing checking events')
        records = parse_events(event_path.read_text(encoding='utf-8'), log, locations)
        if not output_path.is_file():
            raise ValueError('Missing typeset output')
        run = {'status': 'complete', 'engine_exit': result.returncode,
               'output_sha256': hashlib.sha256(output_path.read_bytes()).hexdigest(),
               'log_complete': log_complete}
    after = cgroup_snapshot(cgroup)
    return {'schema': 3, 'prepared_sha256': {
                str(path): hashlib.sha256(source.encode()).hexdigest()
                for path, source in sources.items()},
            'run': run,
            'records': records,
            'resources': resources(start, cgroup_line, before, after),
            }


OUTCOME_FIELDS = ('claimed_grade', 'spend_set', 'grant_set', 'diagnostics', 'derived_rows')


def compare(base, changed):
    validate(base)
    validate(changed)
    before = {row['scope']: row for row in base['records']}
    after = {row['scope']: row for row in changed['records']}
    return [(scope, field) for scope in before for field in OUTCOME_FIELDS
            if before[scope][field] != after[scope][field]]


def changed_fields(before, after, scope):
    left = next(row for row in before['records'] if row['scope'] == scope)
    right = next(row for row in after['records'] if row['scope'] == scope)
    return [field for field in OUTCOME_FIELDS if left[field] != right[field]]


def equal(expected, actual):
    differences = compare(expected, actual)
    if differences:
        raise ValueError(f'Checking capture differs: {differences}')
    print('Checking capture equal')


def validate(value):
    if value.get('schema') != 3:
        raise ValueError('Unsupported checking capture schema')
    run = value.get('run')
    if not isinstance(run, dict) or run.get('status') != 'complete':
        raise ValueError('Incomplete or refused typeset')
    if run.get('engine_exit') != 0 or run.get('log_complete') is not True:
        raise ValueError('Incomplete typeset log')
    digest = run.get('output_sha256')
    if not isinstance(digest, str) or not re.fullmatch(r'[0-9a-f]{64}', digest):
        raise ValueError('Missing typeset output')
    records = value.get('records')
    if not isinstance(records, list) or len(records) != len(SCOPES):
        raise ValueError('Missing check record')
    if [row.get('scope') for row in records] != [row[0] for row in SCOPES]:
        raise ValueError('Check scope order differs')
    for record in records:
        if record.get('end_marker') is not True:
            raise ValueError('Missing check end marker')
        rows = record.get('derived_rows')
        if not isinstance(rows, list):
            raise ValueError('Malformed derived row list')
        expected_tag = {scope: tag for tag, scope in ROW_SCOPES.items()}.get(record['scope'])
        if expected_tag and sum(row.get('tag') == expected_tag for row in rows) != 1:
            raise ValueError('Missing economics row')
        for row in rows:
            if row.get('table') != 'tbl:log:concordance':
                raise ValueError('Economics table differs')
            if any(not isinstance(row.get(field), str) or not row[field]
                   for field in ('departure', 'tag', 'onref', 'cites', 'stopsat')):
                raise ValueError('Malformed economics row arguments')


def expect_refusal(action, description):
    try:
        action()
    except (ValueError, FileNotFoundError, json.JSONDecodeError) as error:
        print(f'{description}: refused ({error})')
        return
    raise AssertionError(f'{description}: accepted')


def selftest(fixture):
    base = json.loads(fixture.read_text(encoding='utf-8'))
    validate(base)
    altered = copy.deepcopy(base)
    altered['prepared_sha256'] = {'prepared source': 'changed'}
    altered['records'][0]['location']['label_token'] = 'moved label'
    if compare(base, altered):
        raise AssertionError('Provenance affected outcome comparison')
    print('source provenance ignored in outcome comparison')
    warning = json.loads(fixture.with_name('checking-capture-warning.json')
                         .read_text(encoding='utf-8'))
    if compare(base, warning) != [('mod:branch', 'diagnostics')]:
        raise AssertionError('Warning changed more than diagnostics')
    print('warning differs only in diagnostics')
    if warning['records'][1]['diagnostics'][0]['identity'] != 'ungraded':
        raise AssertionError('Wrong warning identity')
    warning_inert = json.loads(fixture.with_name('checking-capture-warning-inert.json')
                               .read_text(encoding='utf-8'))
    if compare(base, warning_inert):
        raise AssertionError('Clean base no longer equals the inert-checker capture; revisit the clean-scope limit')
    if compare(warning_inert, warning) != [('mod:branch', 'diagnostics')]:
        raise AssertionError('Inert checker did not lose the warning')
    print('inert checker loses warning under retained audit tap')
    refusal = json.loads(fixture.with_name('checking-capture-refusal.json')
                         .read_text(encoding='utf-8'))
    if (refusal['run']['status'] != 'refused' or
            refusal['run']['diagnostic']['identity'] != 'grade-too-strong' or
            not refusal['run']['diagnostic']['text']):
        raise AssertionError('Wrong refusal identity or text')
    expect_refusal(lambda: equal(base, refusal), 'checker refusal is not a complete record')
    inert = json.loads(fixture.with_name('checking-capture-refusal-inert.json')
                       .read_text(encoding='utf-8'))
    observed = json.loads(fixture.with_name('checking-capture-refusal-observed.json')
                          .read_text(encoding='utf-8'))
    if compare(inert, observed) != [('dyn:flow', 'diagnostics')]:
        raise AssertionError('Checker error changed more than diagnostics')
    if observed['records'][3]['diagnostics'][0]['identity'] != 'grade-too-strong':
        raise AssertionError('Wrong checker error identity')
    if observed['records'][3]['diagnostics'][0]['text'] != refusal['run']['diagnostic']['text']:
        raise AssertionError('Observed error text differs from the refusal log')
    print('checker error differs only in diagnostics under retained audit tap')
    open_spend = json.loads(fixture.with_name('checking-capture-open-spend.json')
                            .read_text(encoding='utf-8'))
    if compare(base, open_spend) != [('src:nd', 'claimed_grade'),
                                    ('src:nd', 'spend_set'),
                                    ('src:nd', 'diagnostics')]:
        raise AssertionError('Open spend changed unexpected outcome fields')
    open_record = open_spend['records'][4]
    open_diag = open_record['diagnostics']
    if (open_record['claimed_grade'] != 'feas' or
            open_record['spend_set'] != ['c:lam'] or
            len(open_diag) != 1 or open_diag[0]['kind'] != 'warning' or
            open_diag[0]['module'] != 'ftoo' or
            open_diag[0]['identity'] != 'open-debt' or not open_diag[0]['text']):
        raise AssertionError('Open spend warning was not captured')
    print('open spend warning and its grade and spend changes captured')
    altered = copy.deepcopy(base)
    altered['records'][2]['derived_rows'] = []
    expect_refusal(lambda: equal(base, altered), 'lost economics row')
    altered = copy.deepcopy(base)
    altered['records'].pop(1)
    expect_refusal(lambda: equal(base, altered), 'dropped checking scope')
    altered = copy.deepcopy(base)
    row = altered['records'][2]['derived_rows'][0]
    row['cites'] += 'x'
    expect_refusal(lambda: equal(base, altered), 'changed economics row argument')
    altered = copy.deepcopy(base)
    row = altered['records'][2]['derived_rows'][0]
    row['cites'], row['stopsat'] = row['stopsat'], row['cites']
    expect_refusal(lambda: equal(base, altered), 'swapped economics row arguments')
    altered = copy.deepcopy(base)
    extra = copy.deepcopy(altered['records'][2]['derived_rows'][0])
    extra['tag'] = 'e:capture'
    altered['records'][2]['derived_rows'].append(extra)
    expect_refusal(lambda: equal(base, altered), 'added economics row')
    altered = copy.deepcopy(base)
    altered['run']['engine_exit'] = 1
    expect_refusal(lambda: equal(base, altered), 'failed typeset after final audit')
    altered = copy.deepcopy(base)
    altered['records'][-1]['end_marker'] = False
    expect_refusal(lambda: equal(base, altered), 'missing audit end marker')
    altered = copy.deepcopy(base)
    altered['run']['output_sha256'] = None
    expect_refusal(lambda: equal(base, altered), 'missing typeset output')
    expect_refusal(lambda: parse_events('FT-CHECK|x|section|nec||\n', '', {}),
                   'truncated event file')
    synthetic = []
    for record in base['records']:
        name = record['scope']
        synthetic.append(f"FT-SCOPE|{name}")
        synthetic.append('|'.join(('FT-CHECK', name, record['location']['kind'],
                                   record['claimed_grade'], ','.join(record['spend_set']),
                                   ','.join(record['grant_set']))))
        synthetic.append(f'FT-END|{name}')
        for row in record['derived_rows']:
            synthetic.append('|'.join(('FT-ROW', name, *(row[key] for key in
                                    ('departure', 'tag', 'onref', 'cites', 'stopsat')))))
    synthetic.append('FT-COMPLETE')
    stream = '\n'.join(synthetic) + '\n'
    locations = {record['scope']: record['location'] for record in base['records']}
    if parse_events(stream, '', locations) != base['records']:
        raise AssertionError('Synthetic event stream differs from stored records')
    box_stream = stream.replace('FT-CHECK|subsec:flow:branch|section|nec||',
                                'FT-CHECK|subsec:flow:branch|box|nec||')
    expect_refusal(lambda: parse_events(box_stream, '', locations),
                   'unselected box under section scope')
    extra_row = 'FT-ROW|subsec:flow:confluence|annex|e:capture|dyn:flow|a:dmp|control\n'
    with_row = parse_events(stream.replace('FT-COMPLETE', extra_row + 'FT-COMPLETE'),
                            '', locations)
    if compare(base, {'schema': 3, 'run': base['run'], 'records': with_row}) != [
            ('subsec:flow:confluence', 'derived_rows')]:
        raise AssertionError('Extra row event did not change its scope')
    print('extra row event attached to its scope')
    print('Checking capture fixture controls passed')


def write_capture(work, result):
    stored = {key: value for key, value in result.items() if key != 'resources'}
    (work / 'capture.json').write_text(
        json.dumps(stored, indent=2, sort_keys=True) + '\n', encoding='utf-8')
    (work / 'resources.json').write_text(
        json.dumps(result['resources'], indent=2, sort_keys=True) + '\n', encoding='utf-8')
    return stored


def main():
    if len(sys.argv) == 3 and sys.argv[1] == 'selftest':
        selftest(pathlib.Path(sys.argv[2]))
        return
    if len(sys.argv) == 4 and sys.argv[1] == 'equal':
        expected = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding='utf-8'))
        actual = json.loads(pathlib.Path(sys.argv[3]).read_text(encoding='utf-8'))
        equal(expected, actual)
        return
    if len(sys.argv) == 6 and sys.argv[1] == 'expect-change':
        before = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding='utf-8'))
        after = json.loads(pathlib.Path(sys.argv[3]).read_text(encoding='utf-8'))
        differences = compare(before, after)
        expected = [(sys.argv[4], sys.argv[5])]
        if differences != expected:
            raise ValueError(f'Expected {expected}, got {differences}')
        print(differences[0])
        return
    if len(sys.argv) == 5 and sys.argv[1] == 'check':
        repository, support, work = map(pathlib.Path, sys.argv[2:])
        start = time.monotonic()
        cgroup_line = (pathlib.Path('/proc/self/cgroup').read_text(encoding='utf-8')
                       .strip().split(':')[-1])
        cgroup = pathlib.Path('/sys/fs/cgroup') / cgroup_line.lstrip('/')
        before = cgroup_snapshot(cgroup)
        result = None
        try:
            result = capture(repository, support, work, 'base')
            stored = write_capture(work, result)
            expected = json.loads((repository / 'tools/paper/fixtures/checking-capture.json')
                                  .read_text(encoding='utf-8'))
            equal(expected, stored)
        finally:
            measurement = (result['resources'] if result is not None else
                           resources(start, cgroup_line, before, cgroup_snapshot(cgroup)))
            print(json.dumps(measurement, sort_keys=True), file=sys.stderr)
        return
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository', type=pathlib.Path)
    parser.add_argument('support', type=pathlib.Path)
    parser.add_argument('work', type=pathlib.Path)
    parser.add_argument('--variant', default='base',
                        choices=('base', 'grade', 'spend', 'grant', 'diagnostic',
                                 'refusal', 'refusal-inert', 'refusal-observed',
                                 'diagnostic-inert', 'base-inert', 'first',
                                 'source-grade', 'source-dropped', 'added-row',
                                 'open-spend', 'unselected-box'))
    args = parser.parse_args()
    stored = write_capture(args.work, capture(args.repository, args.support, args.work,
                                              args.variant))
    print(json.dumps(stored, indent=2, sort_keys=True))
    validate(stored)


if __name__ == '__main__':
    main()
