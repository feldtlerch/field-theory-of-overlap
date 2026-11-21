# Self-description wording renewal

The project's owner approved the four wordings of the paper's description of its checking, and independent reading and page review accepted the resulting passages; the exact source bytes and their reversal evidence follow.

| Passage | Source | Previous span, length, SHA-256 | Renewed span, length, SHA-256 |
| --- | --- | --- | --- |
| 1 | `documents/paper/parts/front.tex` | 5034–5121; 87 bytes; `6b210cd9230c08a2fe79c439eef23e9822c186b465c1b152632117448ecbb011` | 5034–5114; 80 bytes; `cd4d1023db7f2a60ae4443b8e4d6b98cc77ed7f8879eb9a9809c56d65575fd51` |
| 4 | `documents/paper/parts/partE.tex` | 3620–3709; 89 bytes; `44eafcc9ddbb09891a8c396613dad05e47519499e80cde0772f7859a8123510c` | 3620–3717; 97 bytes; `e055762c58cf84c96f00cf5c7a90cc3f6e79116faf84c6f9bc4ae27d41698b7a` |
| 10 | `documents/paper/parts/appendices.tex` | 44771–47453; 2682 bytes; `48aa328c0967d09f96bb29cfcdd0b898b4600e20c67a9b683a08b19dd67ad115` | 44771–47440; 2669 bytes; `5dacd09748b77c3a064991c15f170beac2df2bb9ddd0a5ee1d4de0641053572a` |
| 14 | `documents/paper/explainer/explainer.tex` | 53475–56296; 2821 bytes; `cee190efbda61019f20fbec5916c13877c8f2f2d9350a86d103346d29563d7aa` | 53475–56288; 2813 bytes; `d7dc4a49e621fd741b74a948a9c9e138d277f8448193849c5fbaf35db8965ce2` |

Passage 1 changes “A grading is type-checked like a dimension but carries no independent weight in a norm.” to “A grading is typed like a dimension but carries no independent weight in a norm.”

Passage 4 changes “what is run here as a type-check stands there as theorems” to “what is argued here as a type discipline stands there as theorems”; the rest of the sentence is unchanged.

Passage 10 changes “which rest on the hand analysis recorded in the dimensional log (`\zcref{app:log}`) and are not mechanically verified” to “which rest on the hand analysis at `\ftag{P:nonorm}` and `\ftag{P:axes}` and are not mechanically verified”. `P:nonorm` links to the theorem in `documents/paper/parts/partE.tex:42–54`, whose displayed norm adds unlike dimensions; `documents/paper/parts/front.tex:223–230` cites that tag but is not its link target. `P:axes` links to the statement in `documents/paper/parts/partE.tex:56–59`, followed by the comparison argument in the same subsection.

Passage 14 changes the opening and closing frame around the three grade descriptions so that the result’s grade is stated first and the two typed refusals follow. The three descriptions, the preceding sentence, and the following sentence retain their source words.

Previous passage 14 sentences: “Every result carries a grade, and where its quantities are typed two of the grades are \emph{checked}, not merely asserted: a result that spends no converter is marked as standing on construction alone; a result that leans on the public metric is marked as anchored, true within a community that shares the rod; a result that leans on someone's private weighting is marked as settling only in that holder's own biography, in arrears. Mark a result as standing on construction alone when it records any spend, or as anchored when it leans on a held or openly owed one, and the document is refused at typesetting.”

Renewed passage 14 sentences: “Every result carries a grade: a result that spends no converter is marked as standing on construction alone; a result that leans on the public metric is marked as anchored, true within a community that shares the rod; a result that leans on someone's private weighting is marked as settling only in that holder's own biography, in arrears. Where its quantities are typed, two of these are \emph{checked}, not merely asserted: mark a result as standing on construction alone when it records any spend, or as anchored when it leans on a held or openly owed one, and the document is refused at typesetting.”

Passages 2, 11, and 12 have unchanged SHA-256 values and move only in byte position: passage 2 moves from 15108–15318 to 15101–15311, passage 11 from 52912–53028 to 52899–53015, and passage 12 from 53054–53142 to 53041–53129. The other twelve accepted intervals retain their positions and hashes.

## Exact passage bytes and reversal

The fenced content gives the recorded interval bytes; a fence-delimiting newline after an interval without a terminal newline is not part of that interval.

### Passage 1

Previous anchor: "A grading is type-checked like a dimension but carries no independent weight in a norm.".

Renewed anchor: "A grading is typed like a dimension but carries no independent weight in a norm.".

Previous exact bytes (87 bytes):

```tex
A grading is type-checked like a dimension but carries no independent weight in a norm.
```

Renewed exact bytes (80 bytes):

```tex
A grading is typed like a dimension but carries no independent weight in a norm.
```

Replacing the renewed source interval from byte 17 through byte 17 with the previous interval bytes from byte 17 through byte 24 yields the previous interval byte for byte; the restored SHA-256 is `6b210cd9230c08a2fe79c439eef23e9822c186b465c1b152632117448ecbb011`.

### Passage 4

Previous anchor: "scale-type results --- what is run here as a type-check stands there as theorems, and the".

Renewed anchor: "scale-type results --- what is argued here as a type discipline stands there as theorems, and the".

Previous exact bytes (89 bytes):

```tex
scale-type results --- what is run here as a type-check stands there as theorems, and the
```

Renewed exact bytes (97 bytes):

```tex
scale-type results --- what is argued here as a type discipline stands there as theorems, and the
```

Replacing the renewed source interval from byte 31 through byte 63 with the previous interval bytes from byte 31 through byte 55 yields the previous interval byte for byte; the restored SHA-256 is `44eafcc9ddbb09891a8c396613dad05e47519499e80cde0772f7859a8123510c`.

### Passage 10

Previous anchor: "What each mechanism certifies, and what it does not. This is an implementation".

Renewed anchor: "What each mechanism certifies, and what it does not. This is an implementation".

Previous exact bytes (2682 bytes):

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
      typed results, a necessary result with any recorded spend, an anchored-contingent
      result with a held or open spend, or an undeclared constant is refused at
      typesetting & prose and displays set in the legacy layer; ordinary
      unregistered algebra; sums and comparisons across dimensions, which rest on the
      hand analysis recorded in the dimensional log (\zcref{app:log}) and are not
      mechanically verified \\
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

Renewed exact bytes (2669 bytes):

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
      typed results, a necessary result with any recorded spend, an anchored-contingent
      result with a held or open spend, or an undeclared constant is refused at
      typesetting & prose and displays set in the legacy layer; ordinary
      unregistered algebra; sums and comparisons across dimensions, which rest on the
      hand analysis at \ftag{P:nonorm} and \ftag{P:axes} and are not
      mechanically verified \\
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

Replacing the renewed source interval from byte 1408 through byte 1444 with the previous interval bytes from byte 1408 through byte 1457 yields the previous interval byte for byte; the restored SHA-256 is `48aa328c0967d09f96bb29cfcdd0b898b4600e20c67a9b683a08b19dd67ad115`.

### Passage 14

Previous anchor: "\\section{The Paper and Its Checks}".

Renewed anchor: "\\section{The Paper and Its Checks}".

Previous exact bytes (2821 bytes):

```tex
\section{The Paper and Its Checks}
\label{sec:explainer:typed}

The demand this theory makes of every score --- \emph{show me your conversion rates} ---
is made of the paper itself, at every build, as far as its typed notation reaches.

Every quantity written in the paper's typed notation is declared once, with its units
and with the list of posted rates it spends. Every result carries a grade, and where its
quantities are typed two of the grades are \emph{checked}, not merely asserted: a result that
spends no converter is marked as standing on construction alone; a result that leans on
the public metric is marked as anchored, true within a community that shares the rod; a
result that leans on someone's private weighting is marked as settling only in that
holder's own biography, in arrears. Mark a result as standing on construction alone when it records any spend, or as anchored
when it leans on a held or openly owed one, and the document is refused at typesetting. The exchange rates themselves sit in one generated
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

Renewed exact bytes (2813 bytes):

```tex
\section{The Paper and Its Checks}
\label{sec:explainer:typed}

The demand this theory makes of every score --- \emph{show me your conversion rates} ---
is made of the paper itself, at every build, as far as its typed notation reaches.

Every quantity written in the paper's typed notation is declared once, with its units
and with the list of posted rates it spends. Every result carries a grade: a result that
spends no converter is marked as standing on construction alone; a result that leans on
the public metric is marked as anchored, true within a community that shares the rod; a
result that leans on someone's private weighting is marked as settling only in that
holder's own biography, in arrears. Where its quantities are typed, two of these are
\emph{checked}, not merely asserted: mark a result as standing on construction alone when it records any spend, or as anchored
when it leans on a held or openly owed one, and the document is refused at typesetting. The exchange rates themselves sit in one generated
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

Replacing the renewed source interval from byte 396 through byte 795 with the previous interval bytes from byte 396 through byte 803 yields the previous interval byte for byte; the restored SHA-256 is `cee190efbda61019f20fbec5916c13877c8f2f2d9350a86d103346d29563d7aa`.

## Rendered pair

| Artifact | Previous SHA-256 | Renewed SHA-256 |
| --- | --- | --- |
| `generated/paper/paper.pdf` | `d3313cb86a5afced0c7f4320ac84e6084df395f25bf172d1d8756bb41b5c3aab` | `1744907b8e65e7cc6129bc5db94c1ba83bbc2751854c4eb0d74333439ffc17f8` |
| `generated/paper/explainer.pdf` | `4d7b9793f2a615391ce9f7a44ef504a1337043c7c73c4e3bdaada7e9acd41bad` | `73594d6cdc321098c212f3a557277c23a6a1a59a2a1c7fec159b4e03f9954fee` |
| `generated/paper/inputs.json` | `c10e64f3e8e5dae942a22c832d429588bf8251c4d1131abb14adc7d2a2233986` | `c5f600f91a9fca691b85a41df5e3b1738e0e2851801d7d0f67bdd1a24768f2e9` |

Raw extracted page text changes on physical paper pages 7, 69, and 109 and explainer page 23 only. The paper remains 131 pages and the explainer 24; all 658 paper auxiliary destinations, 24 explainer auxiliary destinations, and bookmark page mappings retain their pages. The table prints the two references as `P:NONORM` and `P:AXES`, both resolving to their body targets. The receipt changes only the four authored source entries listed above.
