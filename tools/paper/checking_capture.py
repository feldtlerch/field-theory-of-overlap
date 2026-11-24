"""Capture labelled audits from prepared paper in one typeset run.

An event belongs to the labelled scope current when it is emitted: a box
restores its enclosing subsection on closing. Grade, spend and grant snapshots,
diagnostics and rows follow this same rule. The selected scopes are the branch
and confluence subsections, the mod:branch, dyn:flow and src:nd boxes, and the
coda section and its four subsection audits. Row fields are the economics
writer's arguments; rendered table cells are not observed and need their own
comparison if the table is converted. An unselected box audit inside a selected
section window refuses even when it changes its own scope marker. An unselected
audit, checker diagnostic or economics row in that window refuses; only the two
outside markers at the known section boundaries close a capture window. Only the
checker's l3msg warning and error forms are observed,
including shorter forms routed through their six-argument form; a checker error
yields a refused run, while package warnings, infos and notes are outside this
capture. Clean scopes cannot prove that spends, grants or the audit's judgement
remain live: rerun spend, grant, diagnostic, open-spend and refusal variants against
each converted body. A separate envelope binds the repository revision, all
prepared paper input bytes, the locked engine and used bundle members, the
capture tap and instrumented input bytes, the captured record, and the
successful event, log, stream and PDF bytes. Each side is checked against its
own retained run before a pair reports outcome and provenance differences;
outcome comparison still ignores provenance. The closing paragraph and refrain
are checked as consumed prepared bodies, and the economics row is a nonempty
control. The envelope attests an instrumented typeset, not the driver's PDF
pair or rendered concordance cells. A rewritten box opening may
need a new anchor; an absent anchor refuses. The run uses the paper engine's
fixed rerun count, not the paper-and-explainer auxiliary fixed point.
Paper edits can stale every variant's provenance; the full check rederives only
the base fixture. The source-note control inserts a comment after preparation
at a fixed revision; it uses the same base checker instrumentation on both sides.
The envelope cannot independently prove a command-only rerun-count variant.
Pair comparison refuses changes to the checker source as capability changes.
"""
import argparse
import copy
import hashlib
import json
import pathlib
import re
import resource
import shutil
import subprocess
import sys
import time

from common import runtime
from prepare import package_names, prepare


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
\cs_new_protected:Npn \CaptureSpendControl
  { \ftoo_spend:nn {c:lam}{capture} }
\cs_new_protected:Npn \CaptureGrantControl
  { \clist_put_right:Ne \l__ftoo_grant_mask_clist {\tl_to_str:n {c:lam}}
    \ftoo_spend:nn {c:lam}{capture} }
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


def instrument_coda(source, variant):
    variant = variant.removesuffix('-inert')
    body_effect = variant.endswith('-body')
    variant = variant.removesuffix('-body')
    if variant in ('coda-feas', 'coda-spend', 'coda-open-spend'):
        source = unique_replace(
            source, r'\label{subsec:coda:wall}' + '\n' + r'\secgrade{nec}%',
            r'\label{subsec:coda:wall}' + '\n' + r'\secgrade{feas}%')
    if variant in ('coda-spend', 'coda-grant', 'coda-open-spend', 'coda-refusal'):
        if variant == 'coda-grant':
            effect = (r'\ExplSyntaxOn\clist_put_right:Ne '
                      r'\l__ftoo_grant_mask_clist {\tl_to_str:n {c:lam}}'
                      r'\ftoo_spend:nn {c:lam}{capture}\ExplSyntaxOff')
        elif variant == 'coda-open-spend':
            effect = (r'\ExplSyntaxOn\prop_gput:Nnn '
                      r'\g__ftoo_const_status_prop {c:lam}{open}'
                      r'\ftoo_spend:nn {c:lam}{capture}\ExplSyntaxOff')
        else:
            effect = r'\ExplSyntaxOn\ftoo_spend:nn {c:lam}{capture}\ExplSyntaxOff'
        if body_effect:
            source = unique_replace(source, r'\end{econ}', effect + '\n' + r'\end{econ}')
        else:
            source = unique_replace(source, r'\end{econ}', r'\end{econ}' + '\n' + effect)
    if variant == 'coda-open-spend':
        restoration = (r'\ExplSyntaxOn\prop_gput:Nnn '
                       r'\g__ftoo_const_status_prop {c:lam}{held}\ExplSyntaxOff')
        source = unique_replace(source, r'\subsection{The eight words}',
                                r'\subsection{The eight words}' + '\n' + restoration)
    if variant == 'coda-row':
        source = unique_replace(source,
                                'the information welfarism discards}]',
                                'the information welfarism discards revised}]')
    if variant == 'coda-added-box':
        added = (r'\ExplSyntaxOn\ftoo_capture_scope:n {e:added}\ExplSyntaxOff' + '\n' +
                 r'\begin{definition}\grade{nec}' + '\n' +
                 'Capture box.' + '\n' +
                 r'\end{definition}' + '\n' +
                 r'\ExplSyntaxOn\ftoo_capture_scope:n {subsec:coda:wall}\ExplSyntaxOff')
        source = unique_replace(source, r'\end{econ}', r'\end{econ}' + '\n' + added)
    if variant.startswith('close-') and variant != 'close-provenance':
        _, body, effect_kind = variant.split('-')
        if body not in ('paragraph', 'refrain') or effect_kind not in (
                'spend', 'grant', 'refusal'):
            raise ValueError(f'Unknown closing control: {variant}')
        if effect_kind == 'refusal':
            source = unique_replace(source, r'\label{subsec:coda:close}',
                                    r'\label{subsec:coda:close}' + '\n' +
                                    r'\secgrade{nec}%')
        if effect_kind == 'grant':
            effect = r'\CaptureGrantControl'
        else:
            effect = r'\CaptureSpendControl'
        ending = ('by other hands.}' if body == 'paragraph' else 'paid in.}')
        source = unique_replace(source, ending,
                                ending[:-1] + effect + '}')
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
    enclosing = None
    outside_count = 0
    for line in lines[:-1]:
        parts = [part.strip() for part in line.split('|')]
        if parts[0] == 'FT-SCOPE':
            if len(parts) != 2:
                raise ValueError('Malformed scope event')
            if parts[1] == 'outside':
                expected = ('subsec:flow:confluence', 'subsec:coda:close')
                if (active is not None or outside_count >= len(expected) or
                        enclosing != expected[outside_count]):
                    raise ValueError('Unexpected outside scope marker')
                outside_count += 1
                enclosing = None
            elif parts[1] in locations and locations[parts[1]]['kind'] == 'section':
                enclosing = parts[1]
        elif parts[0] == 'FT-CHECK':
            if len(parts) != 6 or active is not None:
                raise ValueError('Malformed check event')
            name, kind, grade, spends, grants = parts[1:]
            if name not in locations and enclosing is not None:
                raise ValueError(f'Unselected audit inside {enclosing}: {name}')
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
                if enclosing is not None:
                    raise ValueError(f'Unselected diagnostic inside {enclosing}: {parts[1]}')
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
            if scope not in locations and enclosing is not None:
                raise ValueError(f'Unselected economics row inside {enclosing}: {scope}')
            if scope in locations:
                if tag in seen_known_rows:
                    raise ValueError(f'Duplicate economics row: {tag}')
                if tag in ROW_SCOPES:
                    seen_known_rows.add(tag)
                rows[scope].append({'table': 'tbl:log:concordance',
                                    'departure': departure, 'tag': tag,
                                    'onref': onref, 'cites': cites, 'stopsat': stopsat})
        else:
            raise ValueError(f'Unknown checking event: {parts[0]}')
    if active is not None:
        raise ValueError('Unclosed check event')
    if outside_count != 2:
        raise ValueError('Missing outside scope marker')
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


def classify_refusal(log, engine_stream, scope):
    marker = f'FT-DIAG|{scope}|error|ftoo|grade-too-strong'
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
    return {'scope': scope, 'kind': 'error', 'module': 'ftoo',
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


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def canonical_sha256(value):
    return sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode('utf-8'))


def digest_map(value):
    return (isinstance(value, dict) and bool(value) and
            all(isinstance(name, str) and bool(name) and
                not pathlib.PurePosixPath(name).is_absolute() and
                '..' not in pathlib.PurePosixPath(name).parts and
                isinstance(digest, str) and re.fullmatch(r'[0-9a-f]{64}', digest)
                for name, digest in value.items()))


def input_digests(directory):
    return {str(path.relative_to(directory)): sha256(path.read_bytes())
            for path in sorted(directory.rglob('*')) if path.is_file()}


def revision(repository):
    return subprocess.check_output(
        ['git', '-C', str(repository), 'rev-parse', '--verify', 'HEAD'],
        text=True, encoding='utf-8').strip()


PREPARATION_INPUTS = ('documents', 'generated', 'registry', 'units',
                      'tools/paper/prepare.py', 'tools/paper/bibroute.py',
                      'tools/paper/identities.py', 'tools/paper/common.py')


def preparation_changes(repository):
    changed = subprocess.check_output(
        ['git', '-C', str(repository), 'diff', '--name-only', 'HEAD', '--',
         *PREPARATION_INPUTS], text=True, encoding='utf-8').strip()
    return changed.splitlines()


def required_packages(repository):
    source_list = (repository / 'documents/paper-build/sources.tsv').read_text(
        encoding='utf-8').splitlines()
    return sorted(package_names(repository, source_list))


def support_identity(repository, support, packages):
    settings, _, engine, bundle = runtime(repository, support)
    names = sorted(set(packages) | set(settings['fonts']) | {'biblatex.bst'})
    return {'engine_sha256': sha256(engine.read_bytes()),
            'lock_sha256': sha256((repository / settings['support-lock']).read_bytes()),
            'runtime_sha256': sha256((repository / 'documents/paper-build/runtime.toml')
                                     .read_bytes()),
            'bundle_files': {name: sha256((bundle / name).read_bytes()) for name in names}}


INSTRUMENTED = ('ftoo_typed_tools.tex', 'parts/partB.tex', 'parts/coda.tex',
                'parts/appendices.tex')


def tap_for_variant(variant):
    tap = TAP
    if variant.endswith('-inert'):
        tap += '\n' + r'\cs_set_protected:Npn \ftoo_capture_original_audit:n #1 {}' + '\n'
        tap += (r'\cs_set_protected:Npn \ftoo_capture_original_section: '
                r'{ \seq_gclear:N \g__ftoo_spendlog_seq '
                r'\seq_gclear:N \g__ftoo_grantlog_seq '
                r'\tl_gset_eq:NN \g__ftoo_secclaim_tl \c__ftoo_nograde_tl }' + '\n')
    if variant == 'refusal-observed':
        tap = tap.replace(
            r'\ftoo_capture_original_error:nnnnnn {#1}{#2}{#3}{#4}{#5}{#6}',
            r'\ftoo_capture_original_warning:nnnnnn {#1}{#2}{#3}{#4}{#5}{#6}')
    return tap


def instrumentation_variant(variant):
    return 'base' if variant in ('close-provenance', 'checker-provenance') else variant


def instrumented_bytes(prepared, variant):
    """Build the exact four files the engine reads from retained prepared bytes."""
    sources = {path: (prepared / path.relative_to('documents/paper'))
               .read_text(encoding='utf-8') for path in (PART_B, CODA)}
    typed = (prepared / 'ftoo_typed_tools.tex').read_text(encoding='utf-8')
    appendices = (prepared / 'parts/appendices.tex').read_text(encoding='utf-8')
    return {
        'ftoo_typed_tools.tex': (typed + tap_for_variant(variant)).encode('utf-8'),
        'parts/partB.tex': instrument_part_b(sources[PART_B], variant).encode('utf-8'),
        'parts/coda.tex': instrument_coda(sources[CODA], variant).encode('utf-8'),
        'parts/appendices.tex': unique_replace(
            appendices, r'\section{Recovery}',
            r'\section{Recovery}' + '\n' +
            r'\ExplSyntaxOn\ftoo_capture_scope:n {outside}\ExplSyntaxOff'
        ).encode('utf-8'),
    }


def make_envelope(repository, support, work, prepared, instrumented, packages,
                  captured, variant, source_revision):
    paper = work / 'paper'
    return {'schema': 1, 'revision': source_revision,
            'prepared_inputs': prepared,
            'support': support_identity(repository, support, packages),
            'instrumentation': {'tool_sha256': sha256(pathlib.Path(__file__).read_bytes()),
                                'variant': variant,
                                'tap_sha256': sha256(tap_for_variant(variant).encode('utf-8')),
                                'inputs': instrumented},
            'successful_output': {
                'pdf_sha256': sha256((paper / 'main.pdf').read_bytes()),
                'events_sha256': sha256((paper / 'checking-capture.events').read_bytes()),
                'log_sha256': sha256((paper / 'main.log').read_bytes()),
                'engine_stream_sha256': sha256((work / 'engine.txt').read_bytes())},
            'capture_sha256': canonical_sha256(captured)}


def validate_envelope_record(envelope, captured):
    """Refuse a malformed, incomplete or unbound stored provenance claim."""
    validate(captured)
    if not isinstance(envelope, dict) or envelope.get('schema') != 1:
        raise ValueError('Unsupported provenance envelope schema')
    if set(envelope) != {'schema', 'revision', 'prepared_inputs', 'support',
                         'instrumentation', 'successful_output', 'capture_sha256'}:
        raise ValueError('Malformed provenance envelope')
    if envelope.get('capture_sha256') != canonical_sha256(captured):
        raise ValueError('Stale or swapped capture envelope')
    if not isinstance(envelope.get('revision'), str) or not re.fullmatch(
            r'[0-9a-f]{40}', envelope['revision']):
        raise ValueError('Malformed source revision')
    prepared = envelope.get('prepared_inputs')
    if not digest_map(prepared) or not set(INSTRUMENTED) <= set(prepared):
        raise ValueError('Malformed prepared input inventory')
    support = envelope.get('support')
    if (not isinstance(support, dict) or
            set(support) != {'engine_sha256', 'lock_sha256',
                             'runtime_sha256', 'bundle_files'} or
            not digest_map(support['bundle_files']) or
            any(not isinstance(support[name], str) or
                not re.fullmatch(r'[0-9a-f]{64}', support[name])
                for name in ('engine_sha256', 'lock_sha256', 'runtime_sha256'))):
        raise ValueError('Malformed support identity')
    instrumentation = envelope.get('instrumentation')
    if (not isinstance(instrumentation, dict) or
            set(instrumentation) != {'tool_sha256', 'variant',
                                     'tap_sha256', 'inputs'} or
            not isinstance(instrumentation['variant'], str) or
            not digest_map(instrumentation['inputs']) or
            set(instrumentation['inputs']) != set(prepared) or
            any(not isinstance(instrumentation[name], str) or
                not re.fullmatch(r'[0-9a-f]{64}', instrumentation[name])
                for name in ('tool_sha256', 'tap_sha256'))):
        raise ValueError('Malformed instrumentation inventory')
    output = envelope.get('successful_output')
    if (not isinstance(output, dict) or
            set(output) != {'pdf_sha256', 'events_sha256', 'log_sha256',
                            'engine_stream_sha256'} or
            any(not isinstance(digest, str) or not re.fullmatch(r'[0-9a-f]{64}', digest)
                for digest in output.values())):
        raise ValueError('Missing successful output inventory')
    if output['pdf_sha256'] != captured['run']['output_sha256']:
        raise ValueError('Successful output differs from capture')


def validate_envelope(envelope, captured, repository, support, work):
    """Bind a complete capture to the files and support used by its typeset run."""
    validate_envelope_record(envelope, captured)
    if envelope.get('revision') != revision(repository):
        raise ValueError('Source revision differs from envelope')
    prepared = envelope['prepared_inputs']
    paper = work / 'paper'
    saved = work / 'prepared-inputs'
    retained = input_digests(saved)
    if set(retained) != set(prepared):
        raise ValueError('Prepared input inventory differs from retained copy')
    instrumentation = envelope.get('instrumentation')
    if (not isinstance(instrumentation, dict) or
            set(instrumentation) != {'tool_sha256', 'variant', 'tap_sha256', 'inputs'} or
            not isinstance(instrumentation['inputs'], dict) or
            set(instrumentation['inputs']) != set(prepared)):
        raise ValueError('Instrumented input inventory differs')
    for name, digest in prepared.items():
        if retained[name] != digest:
            raise ValueError(f'Changed consumed input: {name}')
        if name not in INSTRUMENTED:
            path = paper / name
            if not path.is_file() or sha256(path.read_bytes()) != digest:
                raise ValueError(f'Changed consumed input: {name}')
    recorded_support = envelope.get('support')
    if (not isinstance(recorded_support, dict) or
            set(recorded_support) != {'engine_sha256', 'lock_sha256',
                                      'runtime_sha256', 'bundle_files'} or
            not isinstance(recorded_support['bundle_files'], dict)):
        raise ValueError('Malformed support identity')
    if recorded_support != support_identity(repository, support,
                                            required_packages(repository)):
        raise ValueError('Support identity differs from envelope')
    if instrumentation['tool_sha256'] != sha256(pathlib.Path(__file__).read_bytes()):
        raise ValueError('Capture instrument differs from envelope')
    if instrumentation['tap_sha256'] != sha256(
            tap_for_variant(instrumentation['variant']).encode('utf-8')):
        raise ValueError('Checking tap differs from envelope')
    expected_inputs = instrumented_bytes(saved, instrumentation['variant'])
    for name, digest in instrumentation['inputs'].items():
        path = paper / name
        if not path.is_file() or sha256(path.read_bytes()) != digest:
            raise ValueError(f'Instrumented input differs: {name}')
    for name, expected in expected_inputs.items():
        if (paper / name).read_bytes() != expected:
            raise ValueError(f'Instrumented input differs from variant: {name}')
    output = envelope['successful_output']
    for name, path in (('pdf_sha256', paper / 'main.pdf'),
                       ('events_sha256', paper / 'checking-capture.events'),
                       ('log_sha256', paper / 'main.log'),
                       ('engine_stream_sha256', work / 'engine.txt')):
        if not path.is_file() or sha256(path.read_bytes()) != output.get(name):
            raise ValueError(f'Missing or changed successful output: {name}')
    locations = {row['scope']: row['location'] for row in captured['records']}
    events = (paper / 'checking-capture.events').read_text(encoding='utf-8')
    log = (paper / 'main.log').read_text(encoding='utf-8', errors='replace')
    if parse_events(events, log, locations) != captured['records']:
        raise ValueError('Checking events differ from capture')
    if not re.search(r'Output written on main\.xdv \(\d+ pages, \d+ bytes\)\.\s*\Z', log):
        raise ValueError('Successful typeset log incomplete')


def compare_provenance(left, right):
    """Report identity differences without treating them as outcome differences."""
    def leaves(before, after, prefix):
        if isinstance(before, dict) and isinstance(after, dict):
            for name in sorted(before.keys() | after.keys()):
                yield from leaves(before.get(name), after.get(name),
                                  f'{prefix}.{name}' if prefix else name)
        elif before != after:
            yield prefix

    return list(leaves(left, right, ''))


def bound_pair(left, left_envelope, left_context, right, right_envelope,
               right_context):
    """Validate each live side, then report outcomes and provenance separately."""
    validate_envelope(left_envelope, left, *left_context)
    validate_envelope(right_envelope, right, *right_context)
    provenance = compare_provenance(left_envelope, right_envelope)
    forbidden = [path for path in provenance if path in (
        'instrumentation.tool_sha256', 'instrumentation.tap_sha256',
        'instrumentation.variant', 'support.engine_sha256',
        'prepared_inputs.ftoo_typed_tools.tex',
        'instrumentation.inputs.ftoo_typed_tools.tex') or
        path.startswith('support.')]
    if forbidden:
        raise ValueError(f'Capability provenance differs: {forbidden}')
    return compare(left, right), provenance


def capture(repository, support, work, variant):
    work = work.resolve()
    start = time.monotonic()
    if work.exists():
        raise ValueError('Capture work area must be new')
    cgroup_line = (pathlib.Path('/proc/self/cgroup').read_text(encoding='utf-8')
                   .strip().split(':')[-1])
    cgroup = pathlib.Path('/sys/fs/cgroup') / cgroup_line.lstrip('/')
    before = cgroup_snapshot(cgroup)
    source_revision = revision(repository)
    dirty_before = preparation_changes(repository)
    paper = work / 'paper'
    packages = prepare(repository, support, paper)
    if variant == 'close-provenance':
        coda = paper / 'parts/coda.tex'
        coda.write_text(unique_replace(
            coda.read_text(encoding='utf-8'),
            r'\refrain{sec:coda:record-limit}',
            '% capture provenance copy\n' + r'\refrain{sec:coda:record-limit}'),
            encoding='utf-8')
    if variant == 'checker-provenance':
        typed = paper / 'ftoo_typed_tools.tex'
        typed.write_text(typed.read_text(encoding='utf-8') + '% checker provenance copy\n',
                         encoding='utf-8')
    prepared = input_digests(paper)
    saved = work / 'prepared-inputs'
    shutil.copytree(paper, saved)
    sources = {path: (paper / path.relative_to('documents/paper')).read_text(encoding='utf-8')
               for path in (PART_B, CODA)}
    locations = source_locations(sources)
    effective_variant = instrumentation_variant(variant)
    for name, data in instrumented_bytes(saved, effective_variant).items():
        (paper / name).write_bytes(data)
    instrumented = {name: sha256((paper / name).read_bytes()) for name in prepared}
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
        refusal_scope = ('subsec:coda:close' if variant in (
                            'close-paragraph-refusal', 'close-refrain-refusal')
                         else 'subsec:coda:wall' if variant == 'coda-refusal'
                         else 'dyn:flow')
        refusal = classify_refusal(log, engine_stream, refusal_scope)
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
    captured = {'schema': 3, 'prepared_sha256': {
                str(path): hashlib.sha256(source.encode()).hexdigest()
                for path, source in sources.items()},
            'run': run,
            'records': records,
            }
    if run['status'] == 'complete':
        dirty_after = preparation_changes(repository)
        if dirty_before or dirty_after or revision(repository) != source_revision:
            print('Provenance envelope omitted: tracked preparation inputs differ '
                  'from HEAD or the revision changed during capture', file=sys.stderr)
        else:
            stored = {key: value for key, value in captured.items() if key != 'resources'}
            captured['envelope'] = make_envelope(repository, support, work, prepared,
                                                 instrumented, packages, stored,
                                                 effective_variant, source_revision)
    captured['resources'] = resources(start, cgroup_line, before, cgroup_snapshot(cgroup))
    return captured


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
    coda = {name: json.loads(fixture.with_name(f'checking-capture-coda-{name}.json')
                             .read_text(encoding='utf-8'))
            for name in ('feas', 'spend', 'grant', 'open-spend', 'refusal', 'row')}
    expected_coda = (
        (base, coda['feas'], 'claimed_grade'),
        (coda['feas'], coda['spend'], 'spend_set'),
        (base, coda['grant'], 'grant_set'),
        (coda['spend'], coda['open-spend'], 'diagnostics'),
        (base, coda['row'], 'derived_rows'),
    )
    for left, right, field in expected_coda:
        if compare(left, right) != [('subsec:coda:wall', field)]:
            raise AssertionError(f'Coda {field} control changed another outcome')
    base_row = base['records'][7]['derived_rows'][0]
    changed_row = coda['row']['records'][7]['derived_rows'][0]
    if [key for key in base_row if base_row[key] != changed_row[key]] != ['stopsat']:
        raise AssertionError('Coda row control changed another argument')
    warning_record = coda['open-spend']['records'][7]
    coda_warning = warning_record['diagnostics']
    if (warning_record['claimed_grade'] != 'feas' or
            warning_record['spend_set'] != ['c:lam'] or
            len(coda_warning) != 1 or coda_warning[0]['kind'] != 'warning' or
            coda_warning[0]['module'] != 'ftoo' or
            coda_warning[0]['identity'] != 'open-debt' or not coda_warning[0]['text']):
        raise AssertionError('Coda open spend warning was not captured')
    coda_refusal = coda['refusal']['run']
    if (coda_refusal['status'] != 'refused' or
            coda_refusal['diagnostic']['scope'] != 'subsec:coda:wall' or
            coda_refusal['diagnostic']['identity'] != 'grade-too-strong' or
            not coda_refusal['diagnostic']['text'] or
            coda_refusal['engine_exit'] == 0 or
            coda_refusal['output_sha256'] is not None):
        raise AssertionError('Coda checking refusal differs')
    expected_refusal_text = refusal['run']['diagnostic']['text'].replace(
        "Grade 'cont-anch'", "Grade 'nec'", 1)
    if coda_refusal['diagnostic']['text'] != expected_refusal_text:
        raise AssertionError('Coda checker refusal text differs')
    expect_refusal(lambda: equal(base, coda['refusal']),
                   'coda checker refusal is not a complete record')
    print('coda grade, spend, grant, warning and row controls differ once')
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
        if name in ('subsec:flow:confluence', 'subsec:coda:close'):
            synthetic.append('FT-SCOPE|outside')
    synthetic.append('FT-COMPLETE')
    stream = '\n'.join(synthetic) + '\n'
    locations = {record['scope']: record['location'] for record in base['records']}
    if parse_events(stream, '', locations) != base['records']:
        raise AssertionError('Synthetic event stream differs from stored records')
    box_stream = stream.replace('FT-CHECK|subsec:flow:branch|section|nec||',
                                'FT-CHECK|subsec:flow:branch|box|nec||')
    expect_refusal(lambda: parse_events(box_stream, '', locations),
                   'unselected box under section scope')
    nested_box = ('FT-SCOPE|e:added\nFT-CHECK|e:added|box|nec||\n'
                  'FT-END|e:added\nFT-SCOPE|subsec:coda:wall\n')
    box_stream = stream.replace('FT-SCOPE|subsec:coda:words',
                                nested_box + 'FT-SCOPE|subsec:coda:words')
    expect_refusal(lambda: parse_events(box_stream, '', locations),
                   'unselected box with its own scope marker')
    diagnostic_event = 'FT-DIAG|e:added|warning|ftoo|open-debt\n'
    diagnostic_stream = stream.replace(
        'FT-SCOPE|subsec:coda:words',
        'FT-SCOPE|e:added\n' + diagnostic_event +
        'FT-SCOPE|subsec:coda:wall\nFT-SCOPE|subsec:coda:words')
    expect_refusal(lambda: parse_events(diagnostic_stream, '', locations),
                   'unselected checker diagnostic with its own scope marker')
    row_event = 'FT-ROW|e:added|annex|e:new|dyn:flow|a:dmp|control\n'
    row_stream = stream.replace(
        'FT-SCOPE|subsec:coda:words',
        'FT-SCOPE|e:added\n' + row_event +
        'FT-SCOPE|subsec:coda:wall\nFT-SCOPE|subsec:coda:words')
    expect_refusal(lambda: parse_events(row_stream, '', locations),
                   'unselected economics row with its own scope marker')
    hidden_stream = box_stream.replace('FT-SCOPE|e:added\n',
                                       'FT-SCOPE|outside\n', 1)
    expect_refusal(lambda: parse_events(hidden_stream, '', locations),
                   'outside scope marker inside captured section')
    extra_row = 'FT-ROW|subsec:flow:confluence|annex|e:capture|dyn:flow|a:dmp|control\n'
    with_row = parse_events(stream.replace('FT-COMPLETE', extra_row + 'FT-COMPLETE'),
                            '', locations)
    if compare(base, {'schema': 3, 'run': base['run'], 'records': with_row}) != [
            ('subsec:flow:confluence', 'derived_rows')]:
        raise AssertionError('Extra row event did not change its scope')
    print('extra row event attached to its scope')
    print('Checking capture fixture controls passed')


def write_capture(work, result, repository, support):
    stored = {key: value for key, value in result.items()
              if key not in ('resources', 'envelope')}
    (work / 'capture.json').write_text(
        json.dumps(stored, indent=2, sort_keys=True) + '\n', encoding='utf-8')
    (work / 'resources.json').write_text(
        json.dumps(result['resources'], indent=2, sort_keys=True) + '\n', encoding='utf-8')
    if 'envelope' in result:
        (work / 'envelope.json').write_text(
            json.dumps(result['envelope'], indent=2, sort_keys=True) + '\n',
            encoding='utf-8')
        validate_envelope(result['envelope'], stored, repository, support, work)
    return stored


def main():
    if len(sys.argv) == 8 and sys.argv[1] == 'pair':
        start = time.monotonic()
        try:
            left_repository, left_support, left_work, right_repository, right_support, right_work = (
                map(pathlib.Path, sys.argv[2:]))

            def side(repository, support, work):
                captured = json.loads((work / 'capture.json').read_text(encoding='utf-8'))
                envelope_path = work / 'envelope.json'
                if not envelope_path.is_file():
                    raise ValueError(f'Missing provenance envelope: {work}')
                envelope = json.loads(envelope_path.read_text(encoding='utf-8'))
                return captured, envelope, (repository, support, work)

            outcomes, provenance = bound_pair(
                *side(left_repository, left_support, left_work),
                *side(right_repository, right_support, right_work))
            print(json.dumps({'outcome_differences': outcomes,
                              'provenance_differences': provenance}, sort_keys=True))
            if outcomes:
                raise ValueError(f'Checking outcomes differ: {outcomes}')
        finally:
            print(json.dumps({'pair_wall_seconds': round(time.monotonic() - start, 3)}),
                  file=sys.stderr)
        return
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
        try:
            stored = write_capture(work, capture(repository, support, work, 'base'),
                                   repository, support)
            expected = json.loads((repository / 'tools/paper/fixtures/checking-capture.json')
                                  .read_text(encoding='utf-8'))
            equal(expected, stored)
        finally:
            measurement = resources(start, cgroup_line, before, cgroup_snapshot(cgroup))
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
                                 'open-spend', 'unselected-box', 'coda-added-box',
                                 'coda-feas', 'coda-spend', 'coda-grant',
                                 'coda-open-spend', 'coda-refusal', 'coda-row',
                                 'coda-spend-body', 'coda-grant-body',
                                 'coda-open-spend-body', 'close-provenance',
                                 'checker-provenance',
                                 'close-paragraph-spend', 'close-paragraph-spend-inert',
                                 'close-paragraph-grant',
                                 'close-paragraph-refusal', 'close-refrain-spend',
                                 'close-refrain-grant',
                                 'close-refrain-refusal'))
    args = parser.parse_args()
    stored = write_capture(args.work, capture(args.repository, args.support, args.work,
                                              args.variant), args.repository, args.support)
    print(json.dumps(stored, indent=2, sort_keys=True))
    validate(stored)


if __name__ == '__main__':
    main()
