"""Exercise expansion, shipment, refusal and rendered-label leakage."""
import argparse
import difflib
import json
import os
import pathlib
import re
import shutil
import subprocess
from common import digest, runtime, require, timed, write
from identities import emit
from ledger import leaks
from pdf_text import Pdf
from prepare import prepare
from compare_words import compare
from inputs import inventory

LEAK_FIXTURE = 'tools/paper/fixtures/leak-control.tex'
INPUTS = ['tools/paper/test_identity_tex.py', 'tools/paper/fixtures/leak-control.tex', 'tools/paper/common.py',
          'tools/paper/identities.py', 'tools/paper/ledger.py', 'tools/paper/pdf_text.py',
          'documents/paper-build/fontconfig.conf', 'tools/paper/compare_words.py']
HISTORICAL_SLOTS = (
    ('parts/front.tex', 'sec:reading:shadow', 'paper-pilot/sec-reading-shadow.tex', 'paragraph'),
    ('parts/partA.tex', 'sec:say:direction', 'paper-pilot/sec-say-direction.tex', 'refrain'),
    ('parts/partA.tex', 'sec:say:harm-enlargement', 'paper-pilot/sec-say-harm-enlargement.tex', 'refrain'),
    ('parts/partA.tex', 'sec:say:wonder-beauty', 'paper-pilot/sec-say-wonder-beauty.tex', 'refrain'),
    ('parts/partD.tex', 'sec:say:enemy-to-none', 'paper-pilot/sec-say-enemy-to-none.tex', 'refrain'),
    ('parts/coda.tex', 'sec:coda:record-limit', 'paper-pilot/sec-coda-record-limit.tex', 'refrain'),
    ('explainer/explainer.tex', 'sec:say:direction', 'explainer-pilot/sec-say-direction.tex', 'quotation'),
    ('explainer/explainer.tex', 'sec:say:enemy-to-none', 'explainer-pilot/sec-say-enemy-to-none.tex', 'quotation'),
    ('explainer/explainer.tex', 'sec:coda:record-limit', 'explainer-pilot/sec-coda-record-limit.tex', 'quotation'),
)
PREAMBLE = r'''\documentclass{article}
\usepackage{amsmath,hyperref}
\ExplSyntaxOn
\NewDocumentCommand\DeclareFieldFormat{mm}{}
\NewDocumentCommand\zcref{s O{} m}{1}
\cs_new_protected:Npn\ftoo_tagfont:n#1{\text{\small\textsc{#1}}}
\cs_new_eq:NN\ftoo_tagfont_ref:n\ftoo_tagfont:n
\NewDocumentCommand\tagdef{m}{\ftoo_tagfont:n{#1}}
\input{reader-hooks.tex}
\ExplSyntaxOff
\begin{document}
'''
LAYOUT_SITES = r'''\newdimen\savedstretch \savedstretch=\emergencystretch
\newskip\savedright \savedright=\rightskip
\newskip\savedfill \savedfill=\parfillskip
\newcount\savedfinal \savedfinal=\finalhyphendemerits
\newcount\savedhyphen \savedhyphen=\hyphenpenalty
\newcount\savedwidow \savedwidow=\widowpenalty
\newcount\savedbroken \savedbroken=\brokenpenalty
\newdimen\savedalign \savedalign=\minalignsep
\newcommand\checkoriginal{%
  \ifdim\emergencystretch=\savedstretch\else\errmessage{stretch remained}\fi
  \ifdim\rightskip=\savedright\else\errmessage{right margin remained}\fi
  \ifdim\gluestretch\rightskip=\gluestretch\savedright\else\errmessage{right margin flexibility remained}\fi
  \ifdim\parfillskip=\savedfill\else\errmessage{last line remained}\fi
  \ifdim\gluestretch\parfillskip=\gluestretch\savedfill\else\errmessage{last line flexibility remained}\fi
  \ifnum\finalhyphendemerits=\savedfinal\else\errmessage{final hyphen remained}\fi
  \ifnum\hyphenpenalty=\savedhyphen\else\errmessage{hyphen remained}\fi}
\newcommand\checkchanged{%
  \ifdim\emergencystretch=2em\else\errmessage{stretch absent}\fi
  \ifdim\rightskip=0pt\else\errmessage{right margin absent}\fi
  \ifdim\gluestretch\rightskip=1em\else\errmessage{right margin flexibility absent}\fi
  \ifdim\parfillskip=0pt\else\errmessage{last line absent}\fi
  \ifdim\gluestretch\parfillskip=0.8\hsize\else\errmessage{last line flexibility absent}\fi
  \ifnum\finalhyphendemerits=1000000\else\errmessage{final hyphen absent}\fi
  \ifnum\hyphenpenalty=10000\else\errmessage{whole words absent}\fi}
\newenvironment{econ}{\begingroup}{\endgroup}
\newcommand\formalfoot[1]{\begingroup #1\par\endgroup}
\PaperParagraph{stretch=2em,rightskip=0pt plus 1em,lastline,wholewords}%
\AddToHookNext{para/end}{\checkchanged}Ordinary paragraph.\par\checkoriginal
\PaperParagraph{stretch=4em,rightskip=0pt plus 1em}%
\AddToHookNext{para/end}{%
  \ifdim\emergencystretch=4em\else\errmessage{appendix stretch absent}\fi
  \ifdim\rightskip=0pt\else\errmessage{appendix right margin absent}\fi
  \ifdim\gluestretch\rightskip=1em\else\errmessage{appendix right margin flexibility absent}\fi}%
Appendix paragraph.\par\checkoriginal
\formalfoot{\PaperParagraph{stretch=2em}\AddToHookNext{para/end}{%
  \ifdim\emergencystretch=2em\else\errmessage{formal note stretch absent}\fi}%
  Formal note paragraph.}\checkoriginal
\begingroup
\PaperDisplayParagraph{equation}{stretch=2em,lastline}
\begin{equation}
  \ifdim\emergencystretch=2em\else\errmessage{equation stretch absent}\fi
  \ifnum\finalhyphendemerits=1000000\else\errmessage{equation last line absent}\fi
  1=1
\end{equation}
\checkoriginal
\begin{equation}
  \checkoriginal 2=2
\end{equation}
\PaperAlignSep{3pt}\PaperDisplayParagraph{align}{stretch=2em,wholewords}
\begin{align}
  \ifdim\minalignsep=3pt\else\errmessage{alignment separation absent}\fi
  \ifdim\emergencystretch=2em\else\errmessage{align stretch absent}\fi
  \ifnum\hyphenpenalty=10000\else\errmessage{align whole words absent}\fi
  3&=3
\end{align}
\checkoriginal
\begin{align}
  \ifdim\minalignsep=\savedalign\else\errmessage{alignment separation remained}\fi
  \checkoriginal 4&=4
\end{align}
\endgroup
\begin{econ}Box ending.\PaperBoxEnding{widow}%
  \ifnum\widowpenalty=10000\else\errmessage{box ending absent}\fi\par\end{econ}
\ifnum\widowpenalty=\savedwidow\else\errmessage{widow penalty remained}\fi
\begin{econ}Broken line ending.\PaperBoxEnding{broken-line}%
  \ifnum\brokenpenalty=10000\else\errmessage{broken line ending absent}\fi\par\end{econ}
\ifnum\brokenpenalty=\savedbroken\else\errmessage{broken line penalty remained}\fi
\AddToHookNext{para/end}{\checkoriginal}Subsequent ordinary paragraph.\par
\checkoriginal
\typeout{LAYOUT-SITES-PASS}
'''


def private_conversion_commit(repo, private, parent_pdf, companion_pdf, paragraph,
                              native=False, master=None):
    env = dict(os.environ, GIT_CONFIG_GLOBAL='/dev/null', GIT_CONFIG_NOSYSTEM='1')
    def git(*args, data=None):
        result = subprocess.run(['git', '-C', str(private), *args], input=data,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
        require(result.returncode == 0, 'Private fixture object failed: ' +
                result.stderr.decode(errors='replace'))
        return result.stdout.strip().decode()
    def blob(data):
        return git('hash-object', '-w', '--stdin', data=data)
    def tree(base, replacements):
        rows = []
        for line in subprocess.check_output(
                ['git', '-C', str(private), 'ls-tree', '-z', base], env=env).split(b'\0'):
            if not line:
                continue
            header, name = line.split(b'\t', 1)
            mode, kind, old = header.split(b' ')
            replacement = replacements.pop(name.decode(), None)
            rows.append(mode + b' ' + kind + b' ' +
                        (replacement.encode() if replacement else old) + b'\t' + name + b'\0')
        for name, (mode, kind, object_id) in replacements.items():
            rows.append(mode.encode() + b' ' + kind.encode() + b' ' +
                        object_id.encode() + b'\t' + name.encode() + b'\0')
        return git('mktree', '-z', data=b''.join(rows))
    receipt = json.loads((private/'generated/paper/inputs.json').read_bytes())
    if master is not None:
        receipt['inputs']['documents/paper/main.tex'] = digest(master)
    fragment_name = 'generated/documents/paper-pilot/sec-reading-shadow.tex'
    if native:
        receipt['inputs'][fragment_name] = digest(paragraph)
    else:
        source_name = 'documents/paper/parts/front.tex'
        marker = b'% generated: sec:reading:shadow\n'
        authored = (private/source_name).read_bytes()
        require(authored.count(marker) == 1, 'Selected marker is not unique')
        authored = authored.replace(marker, paragraph)
        inclusion_name = 'generated/documents/inclusions.tsv'
        inclusion = (private/inclusion_name).read_bytes()
        selected = [row for row in inclusion.splitlines(keepends=True)
                    if b'\tsec:reading:shadow\t' in row and b'\tparagraph\t' in row]
        require(len(selected) == 1, 'Selected inclusion is not unique')
        inclusion = inclusion.replace(selected[0], b'')
        receipt['inputs'][source_name] = digest(authored)
        receipt['inputs'][inclusion_name] = digest(inclusion)
        receipt['inputs'].pop(fragment_name)
    receipt_data = (json.dumps(receipt, sort_keys=True, indent=2) + '\n').encode()
    paper = tree('HEAD:generated/paper', {
        'paper.pdf': blob(parent_pdf), 'explainer.pdf': blob(companion_pdf),
        'inputs.json': blob(receipt_data)})
    if native:
        paper_fragments = tree('HEAD:generated/documents/paper-pilot', {
            'sec-reading-shadow.tex': blob(paragraph)})
        generated_documents = tree('HEAD:generated/documents', {
            'paper-pilot': paper_fragments})
    else:
        generated_documents = tree('HEAD:generated/documents', {
            'inclusions.tsv': blob(inclusion)})
    generated = tree('HEAD:generated', {'paper': paper, 'documents': generated_documents})
    if native:
        replacements = {'generated': generated}
        if master is not None:
            replacements['documents'] = tree('HEAD:documents', {
                'paper': tree('HEAD:documents/paper', {'main.tex': blob(master)})})
        root = tree('HEAD', replacements)
    else:
        parts = tree('HEAD:documents/paper/parts', {'front.tex': blob(authored)})
        document_paper = tree('HEAD:documents/paper', {'parts': parts})
        documents = tree('HEAD:documents', {'paper': document_paper})
        root = tree('HEAD', {'generated': generated, 'documents': documents})
    return git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
               '-c', 'commit.gpgsign=false', 'commit-tree', root, '-p', 'HEAD',
               data=b'Paper paragraph comparison fixture\n')


def conversion_fixtures(repo, support, work, engine, bundle, env):
    template = work/'conversion-template'
    prepare(repo, support, template)
    master = (template/'main.tex').read_text()
    require(master.count('\\begin{document}') == 1, 'Paper fixture preamble differs')
    preamble = master.split('\\begin{document}', 1)[0] + '\\begin{document}\n'
    historical = subprocess.check_output(
        ['git', '-C', str(repo), 'show', '68643f6:documents/paper/parts/front.tex']).decode()
    paragraph = ''.join(historical.splitlines(keepends=True)[202:207])
    fragment = (repo/'generated/documents/paper-pilot/sec-reading-shadow.tex').read_text()
    filler = ('The neutral record stays in its place, and its words are the same in every '
              'rendering of this comparison. ' * 12 + '\n\n') * 20
    companion_area = work/'conversion-companion'
    shutil.copytree(template, companion_area)
    (companion_area/'conversion-companion.tex').write_text(
        preamble + 'Companion text.\n\\end{document}\n')
    companion_result = subprocess.run(
        [str(engine), '--bundle', str(bundle), '--only-cached', '--untrusted',
         '--keep-logs', '--reruns', '0', 'conversion-companion.tex'],
        cwd=companion_area, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=90)
    (companion_area/'engine.txt').write_bytes(companion_result.stdout)
    require(companion_result.returncode == 0 and b'Fontconfig' not in companion_result.stdout,
            'Conversion companion did not typeset cleanly')
    for name in ('conversion-parent', 'conversion-parent-hyphen', 'conversion-native',
                 'conversion-changed', 'conversion-reflow', 'conversion-reflow-hyphen'):
        area = work/name
        shutil.copytree(template, area)
        source = paragraph if name.startswith('conversion-parent') else fragment
        if name == 'conversion-changed':
            source = source.replace('poorest', 'richest')
        top = preamble
        if name in ('conversion-reflow', 'conversion-reflow-hyphen'):
            require('textwidth=6.0in' in top, 'Paper fixture measure differs')
            top = top.replace('textwidth=6.0in', 'textwidth=4.2in', 1)
        penalty = '' if name in ('conversion-parent-hyphen', 'conversion-reflow-hyphen') else '\\hyphenpenalty=10000\n'
        body = (penalty +
                '\\ExplSyntaxOn\\prop_gput:Nnn \\g_ftoo_seen_prop {src:nd}{1}\\ExplSyntaxOff\n'
                '\\handle*{src:nd}\n\n' + source + '\n\n\\hyphenpenalty=10000\n' +
                filler + '\\end{document}\n')
        (area/(name+'.tex')).write_text(top + body)
        result = subprocess.run(
            [str(engine), '--bundle', str(bundle), '--only-cached', '--untrusted',
             '--keep-logs', '--reruns', '0', name+'.tex'],
            cwd=area, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=90)
        (area/'engine.txt').write_bytes(result.stdout)
        require(b'Fontconfig' not in result.stdout,
                'Fontconfig message in conversion fixture: ' + name)
        require(result.returncode == 0, 'Conversion fixture failed: ' + name + ' ' +
                result.stdout.decode(errors='replace'))
    private = work/'conversion-private'
    result = subprocess.run(['git', 'clone', '--no-local', str(repo), str(private)],
                            capture_output=True, timeout=60)
    require(result.returncode == 0, 'Private fixture repository failed: ' +
            result.stderr.decode(errors='replace'))
    delivered = {}
    for document in ('paper', 'explainer'):
        path = work / ('delivered-before-' + document + '.pdf')
        path.write_bytes(subprocess.check_output(
            ['git', '-C', str(repo), 'show',
             '68643f6:generated/paper/' + document + '.pdf']))
        delivered[document] = path
    parent = work/'conversion-parent/conversion-parent.pdf'
    companion = companion_area/'conversion-companion.pdf'
    original = paragraph.encode()
    commit = private_conversion_commit(repo, private, parent.read_bytes(),
                                       companion.read_bytes(), original)
    source_selector = commit + ':documents/paper/parts/front.tex:203-207'
    hyphen_parent = work/'conversion-parent-hyphen/conversion-parent-hyphen.pdf'
    hyphen_commit = private_conversion_commit(repo, private, hyphen_parent.read_bytes(),
                                              companion.read_bytes(), original)
    hyphen_selector = hyphen_commit + ':documents/paper/parts/front.tex:203-207'
    baseline_pages = Pdf(parent).raw_pages()
    reflow_pages = Pdf(work/'conversion-reflow/conversion-reflow.pdf').raw_pages()
    require(len(baseline_pages) != len(reflow_pages),
            'Conversion reflow fixture page counts did not differ')
    for name, expected in (('conversion-native', 'equal'),
                           ('conversion-changed', 'different'),
                           ('conversion-reflow', 'equal'),
                           ('conversion-reflow-hyphen', 'equal')):
        fragment_path = private/'generated/documents/paper-pilot/sec-reading-shadow.tex'
        if name == 'conversion-changed':
            fragment_path.write_text(fragment.replace('poorest', 'richest'))
        else:
            fragment_path.write_text(fragment)
        receipt_path = private/'generated/paper/inputs.json'
        receipt = json.loads(receipt_path.read_text())
        receipt['inputs'][fragment_path.relative_to(private).as_posix()] = digest(fragment_path.read_bytes())
        receipt_path.write_text(json.dumps(receipt, sort_keys=True, indent=2) + '\n')
        after = work/name/(name+'.pdf')
        (private/'generated/paper/paper.pdf').write_bytes(after.read_bytes())
        (private/'generated/paper/explainer.pdf').write_bytes(companion.read_bytes())
        output = (work/name/'comparison').resolve()
        before_paper = hyphen_parent if name == 'conversion-reflow-hyphen' else parent
        before_selector = hyphen_selector if name == 'conversion-reflow-hyphen' else source_selector
        args = argparse.Namespace(
            repository=private, before_paper=before_paper, before_explainer=companion,
            after_paper=after, after_explainer=companion,
            before_source=before_selector, unit='sec:reading:shadow', output=output)
        verdict = compare(args)
        record = json.loads((output/'comparison.json').read_text())
        require(record['verdict'] == expected and verdict == {'equal': 0, 'different': 1,
                                                               'refused': 2}[expected],
                'Conversion comparison differs: ' + name + ' ' + str(record['faults']))
        if name == 'conversion-changed':
            require({row['part'] for row in record['differences']} >= {'source/body', 'stream'},
                    'Changed fixture did not exercise both comparisons')
        if name == 'conversion-reflow':
            require(parent.read_bytes() != after.read_bytes() and
                    baseline_pages != reflow_pages, 'Reflow fixture did not change raw pages')
        if name == 'conversion-reflow-hyphen':
            require(hyphen_parent.read_bytes() != after.read_bytes() and
                    Pdf(hyphen_parent).raw_pages() != Pdf(after).raw_pages(),
                    'Hyphenated reflow fixture did not change raw pages')
            require(all(any(row['decision'] == 'remove-discretionary' for row in
                            record['correspondence'][side]['decisions'])
                        for side in ('before', 'after')),
                    'Hyphenated reflow lacks a confirmed break on one side')
            require(record['documents']['paper']['before']['canonical_stream'] ==
                    record['documents']['paper']['after']['canonical_stream'],
                    'Hyphenated reflow changed canonical text')
    lexical = fragment.encode().replace(b'dependence', b'de-pendence', 1)
    require(lexical != fragment.encode(), 'Lexical source control did not change the paragraph')
    delivered_paper = delivered['paper']
    delivered_companion = delivered['explainer']
    lexical_commit = private_conversion_commit(repo, private, delivered_paper.read_bytes(),
                                                delivered_companion.read_bytes(), lexical,
                                                native=True)
    (private/'generated/paper/paper.pdf').write_bytes(delivered_paper.read_bytes())
    (private/'generated/paper/explainer.pdf').write_bytes(delivered_companion.read_bytes())
    lexical_output = (work/'conversion-parent/lexical-source-comparison').resolve()
    lexical_args = argparse.Namespace(
        repository=private, before_paper=delivered_paper, before_explainer=delivered_companion,
        after_paper=delivered_paper,
        after_explainer=delivered_companion,
        before_source=None, before_revision=lexical_commit,
        unit='sec:reading:shadow', output=lexical_output)
    lexical_verdict = compare(lexical_args)
    lexical_record = json.loads((lexical_output/'comparison.json').read_text())
    require(lexical_verdict == 1 and lexical_record['verdict'] == 'different' and
            {row['part'] for row in lexical_record['differences']} >=
            {'source/body', 'stream'} and
            lexical_record['correspondence']['before']['decisions'][0]['decision'] ==
            'keep-lexical' and
            any(row['decision'] == 'remove-discretionary' for row in
                lexical_record['correspondence']['after']['decisions']),
            'Lexical source spelling did not differ from the native paragraph')
    line_end_marks(repo, work, template, preamble, private, engine, bundle, env,
                   companion.read_bytes())


def line_end_marks(repo, work, template, preamble, private, engine, bundle, env,
                   companion_pdf):
    """Observe mark spellings and breaks in the paper preamble, then compare both widths."""
    root = work/'line-end-marks'
    root.mkdir()
    original_receipt = (repo/'generated/paper/inputs.json').read_bytes()
    original_master = (repo/'documents/paper/main.tex').read_bytes()
    records = []
    for index, mark in enumerate((' — ', ' – ', ' / ', '—', '–', '/')):
        body = 'm'*44 + mark + 'strengths continue with plain words.'
        fragment = ('\\PaperNativeParagraph{sec:reading:shadow}{' + body + '}\n').encode()
        pages = []
        for side, width in (('before', '6.0in'), ('after', '6.4in')):
            area = root/(str(index) + '-' + side)
            shutil.copytree(template, area)
            top = preamble.replace('textwidth=6.0in', 'textwidth=' + width, 1)
            require(top != preamble if side == 'after' else True,
                    'Mark fixture width did not change')
            (area/'mark.tex').write_text(top + fragment.decode() + '\n\\end{document}\n')
            result = subprocess.run(
                [str(engine), '--bundle', str(bundle), '--only-cached', '--untrusted',
                 '--keep-logs', '--reruns', '0', 'mark.tex'], cwd=area, env=env,
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=90)
            (area/'engine.txt').write_bytes(result.stdout)
            require(result.returncode == 0 and b'Fontconfig' not in result.stdout,
                    'Mark fixture did not typeset cleanly: ' + str(index) + side)
            raw = Pdf(area/'mark.pdf').raw_pages()
            require(len(raw) == 1, 'Mark fixture page count differs')
            pages.append((area/'mark.pdf', raw[0]))
        left_pdf, left_raw = pages[0]
        right_pdf, right_raw = pages[1]
        ending = mark.strip() + '\n'
        if index < 4:
            require(ending in left_raw and ending not in right_raw,
                    'Mark fixture did not move the mark: ' + str(index))
        else:
            require(ending not in left_raw and ending not in right_raw,
                    'Unspaced mark unexpectedly ended a line: ' + str(index))
        (private/'generated/paper/inputs.json').write_bytes(original_receipt)
        (private/'documents/paper/main.tex').write_bytes(original_master)
        before_master = original_master
        after_master = original_master.replace(b'textwidth=6.0in', b'textwidth=6.4in', 1)
        require(after_master != before_master, 'Mark fixture master width did not change')
        parent = private_conversion_commit(repo, private, left_pdf.read_bytes(),
                                           companion_pdf, fragment, native=True,
                                           master=before_master)
        (private/'generated/documents/paper-pilot/sec-reading-shadow.tex').write_bytes(fragment)
        (private/'documents/paper/main.tex').write_bytes(after_master)
        (private/'generated/paper/paper.pdf').write_bytes(right_pdf.read_bytes())
        (private/'generated/paper/explainer.pdf').write_bytes(companion_pdf)
        receipt = json.loads(original_receipt)
        receipt['inputs']['documents/paper/main.tex'] = digest(after_master)
        receipt['inputs']['generated/documents/paper-pilot/sec-reading-shadow.tex'] = digest(fragment)
        (private/'generated/paper/inputs.json').write_text(
            json.dumps(receipt, sort_keys=True, indent=2) + '\n')
        output = root/(str(index) + '-comparison')
        args = argparse.Namespace(
            repository=private, before_paper=left_pdf,
            before_explainer=work/'conversion-companion/conversion-companion.pdf',
            after_paper=right_pdf,
            after_explainer=work/'conversion-companion/conversion-companion.pdf',
            before_source=None, before_revision=parent,
            unit='sec:reading:shadow', output=output)
        verdict = compare(args)
        record = json.loads((output/'comparison.json').read_text())
        require(verdict == 0 and record['verdict'] == 'equal' and not record['faults'],
                'Mark fixture comparison differs: ' + str(index) + ' ' +
                str(record['faults']))
        records.append(dict(mark=mark, before=left_raw, after=right_raw,
                            verdict=record['verdict']))
    (root/'engine.txt').write_text(json.dumps(records, ensure_ascii=False, indent=2) + '\n')


def historical_pair(repo, work):
    """Pin the delivered wording change to its complete raw page set."""
    require(not work.exists(), 'Control work area must be new')
    work.mkdir(parents=True)
    private = work/'reference'
    result = subprocess.run(['git', 'clone', '--no-local', str(repo), str(private)],
                            capture_output=True, timeout=60)
    require(result.returncode == 0, 'Historical fixture repository failed: ' +
            result.stderr.decode(errors='replace'))
    result = subprocess.run(['git', '-C', str(private), 'checkout', '--detach',
                             '02d3b360b6465a21abe650dd69748fe8b5c2a3ca'],
                            capture_output=True, timeout=60)
    require(result.returncode == 0, 'Historical fixture revision failed: ' +
            result.stderr.decode(errors='replace'))
    # The pinned PDFs stay historical; the fixture derives inclusion lines from
    # the pinned source tree so later conversions cannot alter this control.
    inclusion = pathlib.Path('generated/documents/inclusions.tsv')
    rows = []
    for source, unit, fragment, role in HISTORICAL_SLOTS:
        source = 'documents/paper/' + source
        fragment = 'generated/documents/' + fragment
        marker = ('% generated: ' + unit).encode()
        positions = [index for index, content in enumerate((private/source).read_bytes().splitlines(), 1)
                     if content == marker]
        require(len(positions) == 1 and (private/fragment).is_file(),
                'Historical fragment declaration differs: ' + unit)
        rows.append('\t'.join((source, unit, fragment, role, str(positions[0]))))
    (private/inclusion).write_text('\n'.join(rows) + '\n')
    legacy_prepare = private/'tools/paper/prepare.py'
    original_prepare = legacy_prepare.read_text()
    require(original_prepare.count('INPUTS = [') == 1,
            'Historical preparation input declaration differs')
    legacy_prepare.write_text(original_prepare.replace(
        'INPUTS = [', 'INPUTS = ["generated/documents/inclusions.tsv", ', 1))
    result = subprocess.run(['git', '-C', str(private), 'add', str(inclusion)],
                            capture_output=True, timeout=60)
    require(result.returncode == 0, 'Historical inclusion fixture failed: ' +
            result.stderr.decode(errors='replace'))
    (private/'generated/paper/inputs.json').write_text(
        json.dumps(inventory(private), sort_keys=True) + '\n')
    before = {}
    for document in ('paper', 'explainer'):
        path = work/(document+'-before.pdf')
        path.write_bytes(subprocess.check_output(
            ['git', '-C', str(private), 'show',
             '90da6b7124cc2f9bf550ca9d1a1520b490315ed1:generated/paper/'+document+'.pdf']))
        before[document] = path
    args = argparse.Namespace(
        repository=private, before_paper=before['paper'],
        before_explainer=before['explainer'],
        after_paper=private/'generated/paper/paper.pdf',
        after_explainer=private/'generated/paper/explainer.pdf',
        before_source=None,
        before_revision='90da6b7124cc2f9bf550ca9d1a1520b490315ed1',
        unit='sec:reading:shadow', output=work/'comparison')
    verdict = compare(args)
    record = json.loads((args.output/'comparison.json').read_text())
    require(verdict == 1 and record['verdict'] == 'different' and not record['faults'],
            'Historical fixture refused or compared equal: '+str(record['faults']))
    expected = {'paper': [7, 69, 109], 'explainer': [23]}
    actual = {}
    for document in expected:
        left = record['documents'][document]['before']
        right = record['documents'][document]['after']
        require(len(left['raw_pages']) == len(right['raw_pages']),
                'Historical fixture page count differs: '+document)
        def page_streams(pdf):
            result = {}
            for value, location in zip(pdf['stream'], pdf['offsets']):
                result.setdefault(location[0], []).append(value)
            return {page: ''.join(values) for page, values in result.items()}
        left_pages, right_pages = page_streams(left), page_streams(right)
        pages = set()
        for page in set(left_pages) | set(right_pages):
            before_text, after_text = left_pages.get(page, ''), right_pages.get(page, '')
            if before_text == after_text:
                continue
            before_tokens = re.findall(r'\S+|\s+', before_text)
            after_tokens = re.findall(r'\S+|\s+', after_text)
            if any(operation != 'equal' for operation, *_ in difflib.SequenceMatcher(
                    None, before_tokens, after_tokens, autojunk=False).get_opcodes()):
                pages.add(page)
        actual[document] = sorted(pages)
        require(left['titles'] == right['titles'],
                'Historical fixture bookmark titles differ: '+document)
    require(actual == expected
            and {(row['document'], row['part']) for row in record['differences']} ==
            {('paper', 'stream'), ('explainer', 'stream')},
            'Historical fixture changed stream pages differ: '+str(actual))
    print('historical pair:', actual)


def check(repo,support,work):
    work=work.resolve()
    require(not work.exists(),'Control work area must be new')
    work.mkdir(parents=True)
    settings,_,engine,bundle=runtime(repo,support)
    env=dict(settings['environment'],SOURCE_DATE_EPOCH=str(settings['source-date-epoch']),FORCE_SOURCE_DATE=settings['force-source-date'])
    for name,value in settings['work-environment'].items():
        path=work/value;path.mkdir(parents=True,exist_ok=True);env[name]=str(path)
    shutil.copyfile(repo/'documents/paper-build/fontconfig.conf', work/'paper/fontconfig.conf')
    # The leak control typesets a committed page that prints a three-part key; a missing fixture refuses.
    fixture=repo/LEAK_FIXTURE
    require(fixture.is_file(),'Leak control fixture is absent: '+LEAK_FIXTURE)
    area=work/'leak-control';area.mkdir();shutil.copyfile(fixture,area/'leak-control.tex')
    result=subprocess.run([str(engine),'--bundle',str(bundle),'--only-cached','--untrusted','--keep-logs','--reruns','0','leak-control.tex'],cwd=area,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=30)
    (area/'engine.txt').write_bytes(result.stdout)
    require(b'Fontconfig' not in result.stdout,'Fontconfig message in leak control engine output')
    require(result.returncode==0,'Leak control failed to typeset: '+result.stdout.decode('utf-8'))
    pages=Pdf(area/'leak-control.pdf').pages()
    found=leaks(pages[0]) if len(pages)==1 else []
    require(len(found)==2 and set(found)=={'cor:tremble:curve'},'Leak control three-part key was not caught: '+str(found))
    print('Leak control caught',found)
    fixtures={
        'shipped':r'\def\ExpandedKey{eq:f:felt}\setbox0=\hbox{\tagdef{eq:f:cap}}\tagdef{\ExpandedKey}\par',
        'copies':r'\setbox0=\hbox{\tagdef{eq:f:felt}}\copy0\copy0\par',
        'math':r'\setbox0=\hbox{$\tagdef{eq:f:cap}$}$\tagdef{eq:f:felt}$\par',
        'locator':r'\zcref{number:only:source}\par',
        'concordance':r'\newcommand\ftooBoxFoot[1]{}\newcommand\GenerateConcordanceE{\def\prevdep{@none}{\thisstopsat\\}}\UseHook{file/econ_membrane.tex/after}\def\thisstopsat{eq:f:felt}\GenerateConcordanceE\par',
        'concordance-differs':r'\newcommand\ftooBoxFoot[1]{}\newcommand\GenerateConcordanceE{changed}\UseHook{file/econ_membrane.tex/after}',
        'missing':r'\tagdef{absent:identity}\par',
        'layout-sites':LAYOUT_SITES,
    }
    for name,body in fixtures.items():
        area=work/name;area.mkdir();emit(repo,area)
        write(area/(name+'.tex'),PREAMBLE+body+'\n\\end{document}\n')
        result=subprocess.run([str(engine),'--bundle',str(bundle),'--only-cached','--untrusted','--keep-logs','--keep-intermediates','--reruns','0',name+'.tex'],cwd=area,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=30)
        (area/'engine.txt').write_bytes(result.stdout)
        require(b'Fontconfig' not in result.stdout,'Fontconfig message in expansion control: '+name)
        refusals={'missing':b'Missing printed identity absent:identity', 'concordance-differs':b'Concordance output hook differs'}
        if name in refusals:
            require(result.returncode!=0 and refusals[name] in b' '.join(result.stdout.split()),'Expected refusal absent: '+name)
        else:
            require(result.returncode==0,'Expansion control failed: '+name+' '+result.stdout.decode('utf-8'))
            if name=='layout-sites':
                require('LAYOUT-SITES-PASS' in (area/(name+'.log')).read_text(),
                        'Layout site checks did not complete')
            else:
                lines=(area/(name+'.identities')).read_text(encoding='utf-8').splitlines()
                require(len(lines)==(2 if name=='copies' else 1),'Discarded measurement or copied output count differs: '+str(lines))
                expected='|number:only:source||' if name=='locator' else '|eq:f:felt|eq:felt|'
                require(all(expected in line.replace(' ','') for line in lines),'Expanded lookup differs: '+str(lines))
        print(name,'exit',result.returncode)
    conversion_fixtures(repo, support, work, engine, bundle, env)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('paths', nargs='+', type=pathlib.Path)
    p.add_argument('--historical-pair', action='store_true')
    a=p.parse_args()
    if a.historical_pair:
        if len(a.paths) != 2:
            p.error('historical pair requires repository and absent work directory')
        repository, work = a.paths
        with timed('historical-pair-control'):
            historical_pair(repository, work)
    else:
        if len(a.paths) != 3:
            p.error('identity controls require repository, support and absent work directory')
        repository, support, work = a.paths
        with timed('identity-expansion-controls'):check(repository, support, work)
