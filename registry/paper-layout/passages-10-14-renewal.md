# Renewal evidence for two self-description passages

The renewed bytes of passages 10 and 14 differ from the previously accepted bytes only by one layout instruction each. A reviewer other than the change's author accepted both renewals through independent review of `registry/paper-layout/passages-10-14-renewal.md`; the `reading_acceptance` fields record that verdict and this evidence path.

The exact source passages below are delimited by fenced blocks; the insertion is identified outside each block so the block preserves the source bytes. Each instruction prints nothing, sets line-breaking values for its paragraph, and restores the previous values afterward. Removing the marked insertion from each new passage yields its old passage byte for byte.

| Passage | Source and anchor | Old span and SHA-256 | New span and SHA-256 |
| --- | --- | --- | --- |
| 10 | `documents/paper/parts/appendices.tex`; `What each mechanism certifies, and what it does not. This is an implementation` | 44771–47190, 2419 bytes; `c317c9efcc6041144c80a32030b0037172f9be56f3b5bd921a0485af18074f86` | 44771–47241, 2470 bytes; `deb99ef883f68b75080f2e3af1a8da73f86c1ee3ece6a6994fe4b3bd96014db2` |
| 14 | `documents/paper/explainer/explainer.tex`; `\section{The Paper and Its Checks}` | 53475–56168, 2693 bytes; `002068089d36e3135d0e0926bb745dcb8708930db2ef87de3ce24c476d2ba453` | 53475–56196, 2721 bytes; `0204ef8616c7c00a63f28a2b2deae49923cd68a7df472a1d1839c3e4a7d88099` |

The recorded old bounds for passage 14 were 53402–56095; the unchanged anchor is at 53475 in the parent source. The old span hash and length still match.

## Passage 10

Anchor: `What each mechanism certifies, and what it does not. This is an implementation` in `documents/paper/parts/appendices.tex`.

Insertion at offset 2041 within the passage: `\PaperParagraph{stretch=4em,rightskip=0pt plus 1em}`.

The only source change is this local layout instruction. Every word, formula, reference spelling, and claim is identical after removing it; the new block minus the marked insertion has SHA-256 `c317c9efcc6041144c80a32030b0037172f9be56f3b5bd921a0485af18074f86`.

Old exact bytes:

```tex
What each mechanism certifies, and what it does not. This is an implementation
boundary, not a theorem and not an interpretive result, so it carries a structural
label and no semantic key: nothing in the theory turns on it, and a reader auditing
the document needs it stated in one place. No silent caps --- what is certified and
what is not are stated in the same breath.

\begin{table}[htbp]
  \centering
  \captiontag{The verification contract: what is checked, and what is
    not}{tbl:verify}
  \begin{tabularx}{\textwidth}{@{} l L L @{}}
    \toprule
    \textbf{mechanism} & \textbf{certifies} & \textbf{does not certify} \\
    \midrule
    reproduction & this paper and its explainer regenerate byte for byte from a
      fixed, listed set of inputs & truth, empirical adequacy, or completeness \\
    \addlinespace
    addressing & every printed tag, reference and bibliography entry resolves to
      its target & that a tag names a certified object \\
    \addlinespace
    typed layer & ledger and registers render from one set of declarations; in
      typed results, a grade above the recorded spends, an undeclared constant, or a
      sum or comparison across dimensions stops the build & prose and displays set in
      the legacy layer; ordinary unregistered algebra \\
    \addlinespace
    human review & names, scope, interpretation, presentation & mechanical
      reproducibility by itself \\
    \bottomrule
  \end{tabularx}
\end{table}

Two scoping facts belong with the row for the typed layer, because they say what
that row means. A DEFINITION deposits nothing: naming a metric-dependent object is
free, the spend deferred into the name and cashed at every later assertion, so a
definition environment audits at the ceiling while the asserting prose after it pays.
And a subsection grade grades the subsection's OWN running prose: a result environment
audits at its own grain and its spends are consumed there, so a \texttt{nec} subsection
may contain results graded lower without the grade avalanching upward.

The legacy symbol layer deposits nothing: prose set in it audits vacuously, and
\verb|\RegisterVariance| supplies variance heads only, so dimensions stay loudly
unregistered there. The flagship dynamics displays (\zcref{eq:dyn:flow} and the
threshold family) are set in that layer, and the branch guard arms only at typed
call sites, of which the dynamics currently carry none.
```

New exact bytes:

```tex
What each mechanism certifies, and what it does not. This is an implementation
boundary, not a theorem and not an interpretive result, so it carries a structural
label and no semantic key: nothing in the theory turns on it, and a reader auditing
the document needs it stated in one place. No silent caps --- what is certified and
what is not are stated in the same breath.

\begin{table}[htbp]
  \centering
  \captiontag{The verification contract: what is checked, and what is
    not}{tbl:verify}
  \begin{tabularx}{\textwidth}{@{} l L L @{}}
    \toprule
    \textbf{mechanism} & \textbf{certifies} & \textbf{does not certify} \\
    \midrule
    reproduction & this paper and its explainer regenerate byte for byte from a
      fixed, listed set of inputs & truth, empirical adequacy, or completeness \\
    \addlinespace
    addressing & every printed tag, reference and bibliography entry resolves to
      its target & that a tag names a certified object \\
    \addlinespace
    typed layer & ledger and registers render from one set of declarations; in
      typed results, a grade above the recorded spends, an undeclared constant, or a
      sum or comparison across dimensions stops the build & prose and displays set in
      the legacy layer; ordinary unregistered algebra \\
    \addlinespace
    human review & names, scope, interpretation, presentation & mechanical
      reproducibility by itself \\
    \bottomrule
  \end{tabularx}
\end{table}

Two scoping facts belong with the row for the typed layer, because they say what
that row means. A DEFINITION deposits nothing: naming a metric-dependent object is
free, the spend deferred into the name and cashed at every later assertion, so a
definition environment audits at the ceiling while the asserting prose after it pays.
And a subsection grade grades the subsection's OWN running prose: a result environment
audits at its own grain and its spends are consumed there, so a \texttt{nec} subsection
may contain results graded lower without the grade avalanching upward.

\PaperParagraph{stretch=4em,rightskip=0pt plus 1em}The legacy symbol layer deposits nothing: prose set in it audits vacuously, and
\verb|\RegisterVariance| supplies variance heads only, so dimensions stay loudly
unregistered there. The flagship dynamics displays (\zcref{eq:dyn:flow} and the
threshold family) are set in that layer, and the branch guard arms only at typed
call sites, of which the dynamics currently carry none.
```

## Passage 14

Anchor: `\section{The Paper and Its Checks}` in `documents/paper/explainer/explainer.tex`.

Insertion at offset 2199 within the passage: `\PaperParagraph{stretch=2em}`.

The only source change is this local layout instruction. Every word, formula, reference spelling, and claim is identical after removing it; the new block minus the marked insertion has SHA-256 `002068089d36e3135d0e0926bb745dcb8708930db2ef87de3ce24c476d2ba453`.

Old exact bytes:

```tex
\section{The Paper and Its Checks}
\label{sec:explainer:typed}

The demand this theory makes of every score --- \emph{show me your conversion rates} ---
is made of the paper itself, at every build, as far as its typed notation reaches.

Every quantity written in the paper's typed notation is declared once, with its units
and with the list of posted rates it spends. Every result carries a grade, and where its
quantities are typed the grade is \emph{checked}, not merely asserted: a result that
spends no converter is marked as standing on construction alone; a result that leans on
the public metric is marked as anchored, true within a community that shares the rod; a
result that leans on someone's private weighting is marked as settling only in that
holder's own biography, in arrears. Claim a stronger grade than your typed spends allow
and the document refuses to build. The exchange rates themselves sit in one generated
ledger --- posted, held, or openly owed --- rendered from the same declarations the
spends are checked against, so a typed debt cannot hide at a call site; a judgment call
the checker had to make would be published as a ruling rather than smuggled, and none
is posted; and the fork in the dynamics is a priced menu, so nobody inherits the chosen
branch without seeing what the other branches would have cost.

And the paper states the limit of its own trick, in the same breath: the build
certifies \emph{consistency with the declarations}, never the declarations' truth.
Spends are self-reported; a determined author could lie to the machine. Nor does the
checker reach everything: the flagship dynamics displays are set outside the typed
notation and pass unchecked, and so does ordinary algebra in the prose. Beyond that,
the build reproduces both documents byte for byte and resolves every tag and
reference; whether the reported spends are true and complete, and whether the argument
holds, is for the reader. What the machinery buys is compression --- the whole audit
surface reduced to one screen a skeptical reader can check in an afternoon --- which is
precisely the paper's own doctrine applied to itself: settlement in arrears, by other
hands.

\formalfoot{the generated constants ledger is \ptg{tbl:ledger}; the rulings register
  \ptg{tbl:rulings}; the branch prices \ptg{tbl:branches}; posted identity claims
  \ptg{tbl:equivalences}; the evidential back-edges \ptg{tbl:ev}; the two-column
  grading --- what a claim is built on versus what it is read through --- is
  \ptg{mod:facets}, with the grade register at \ptg{tbl:gen:grades}; the checker's
  stated coverage is the verification contract \ptg{tbl:verify}; no ruling row
  remains posted.}
```

New exact bytes:

```tex
\section{The Paper and Its Checks}
\label{sec:explainer:typed}

The demand this theory makes of every score --- \emph{show me your conversion rates} ---
is made of the paper itself, at every build, as far as its typed notation reaches.

Every quantity written in the paper's typed notation is declared once, with its units
and with the list of posted rates it spends. Every result carries a grade, and where its
quantities are typed the grade is \emph{checked}, not merely asserted: a result that
spends no converter is marked as standing on construction alone; a result that leans on
the public metric is marked as anchored, true within a community that shares the rod; a
result that leans on someone's private weighting is marked as settling only in that
holder's own biography, in arrears. Claim a stronger grade than your typed spends allow
and the document refuses to build. The exchange rates themselves sit in one generated
ledger --- posted, held, or openly owed --- rendered from the same declarations the
spends are checked against, so a typed debt cannot hide at a call site; a judgment call
the checker had to make would be published as a ruling rather than smuggled, and none
is posted; and the fork in the dynamics is a priced menu, so nobody inherits the chosen
branch without seeing what the other branches would have cost.

And the paper states the limit of its own trick, in the same breath: the build
certifies \emph{consistency with the declarations}, never the declarations' truth.
Spends are self-reported; a determined author could lie to the machine. Nor does the
checker reach everything: the flagship dynamics displays are set outside the typed
notation and pass unchecked, and so does ordinary algebra in the prose. Beyond that,
the build reproduces both documents byte for byte and resolves every tag and
reference; whether the reported spends are true and complete, and whether the argument
holds, is for the reader. What the machinery buys is compression --- the whole audit
surface reduced to one screen a skeptical reader can check in an afternoon --- which is
precisely the paper's own doctrine applied to itself: settlement in arrears, by other
hands.

\formalfoot{\PaperParagraph{stretch=2em}the generated constants ledger is \ptg{tbl:ledger}; the rulings register
  \ptg{tbl:rulings}; the branch prices \ptg{tbl:branches}; posted identity claims
  \ptg{tbl:equivalences}; the evidential back-edges \ptg{tbl:ev}; the two-column
  grading --- what a claim is built on versus what it is read through --- is
  \ptg{mod:facets}, with the grade register at \ptg{tbl:gen:grades}; the checker's
  stated coverage is the verification contract \ptg{tbl:verify}; no ruling row
  remains posted.}
```

## Rendered pair

| PDF | Parent SHA-256 | Candidate SHA-256 |
| --- | --- | --- |
| paper | `a9803d35a23d51f9c0367d5dd1832631e61ab75d631ad4d1851e485e1028ae7d` | `a9803d35a23d51f9c0367d5dd1832631e61ab75d631ad4d1851e485e1028ae7d` |
| explainer | `b4482f3c26ccc3ae21818e013b323d77b85ccfdef36cf15fcb41ce8372af0052` | `b4482f3c26ccc3ae21818e013b323d77b85ccfdef36cf15fcb41ce8372af0052` |

The pair was generated with the same pinned support and runtime for parent and candidate. Matching hashes show that the inserted instructions preserve the printed pair.
