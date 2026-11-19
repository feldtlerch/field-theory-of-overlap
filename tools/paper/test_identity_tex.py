"""Exercise expansion, shipment, refusal and rendered-label leakage."""
import argparse
import pathlib
import shutil
import subprocess
from common import runtime, require, timed, write
from identities import emit
from ledger import leaks
from pdf_text import Pdf

LEAK_FIXTURE = 'tools/paper/fixtures/leak-control.tex'
INPUTS = ['tools/paper/test_identity_tex.py', 'tools/paper/fixtures/leak-control.tex', 'tools/paper/common.py',
          'tools/paper/identities.py', 'tools/paper/ledger.py', 'tools/paper/pdf_text.py']
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


def check(repo,support,work):
    require(not work.exists(),'Control work area must be new')
    work.mkdir(parents=True)
    settings,_,engine,bundle=runtime(repo,support)
    env=dict(settings['environment'],SOURCE_DATE_EPOCH=str(settings['source-date-epoch']),FORCE_SOURCE_DATE=settings['force-source-date'])
    for name,value in settings['work-environment'].items():
        path=work/value;path.mkdir(parents=True,exist_ok=True);env[name]=str(path)
    # The leak control typesets a committed page that prints a three-part key; a missing fixture refuses.
    fixture=repo/LEAK_FIXTURE
    require(fixture.is_file(),'Leak control fixture is absent: '+LEAK_FIXTURE)
    area=work/'leak-control';area.mkdir();shutil.copyfile(fixture,area/'leak-control.tex')
    result=subprocess.run([str(engine),'--bundle',str(bundle),'--only-cached','--untrusted','--keep-logs','--reruns','0','leak-control.tex'],cwd=area,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=30)
    (area/'engine.txt').write_bytes(result.stdout)
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


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('repository','support','work'):p.add_argument(name,type=pathlib.Path)
    a=p.parse_args()
    with timed('identity-expansion-controls'):check(a.repository,a.support,a.work)
