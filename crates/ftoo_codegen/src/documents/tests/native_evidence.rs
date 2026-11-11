// Complete native evidence at the locked engine, bundle and fixed preamble.
pub(super) fn evidence(name: &str) -> (&'static str, &'static str, &'static str) {
    match name {
        "stored" => (
            r#"note: using only cached resource files
note: generating format "latex"
note: Running TeX ...
note: Writing `@SCRATCH@/output/document.log` (12.64 KiB)
"#,
            r"warning: open of input /dev/null failed
caused by: access to the path `/dev/null` is forbidden
error: document.tex:12: Undefined control sequence
error: halted on potentially-recoverable error as specified
",
            r#"**
(document.tex
LaTeX2e <2021-11-15> patch level 1
L3 programming layer <2022-02-24> (article.cls
Document Class: article 2021/10/04 v1.4n Standard LaTeX document class
(size10.clo
File: size10.clo 2021/10/04 v1.4n Standard LaTeX file (size option)
)
\c@part=\count181
\c@section=\count182
\c@subsection=\count183
\c@subsubsection=\count184
\c@paragraph=\count185
\c@subparagraph=\count186
\c@figure=\count187
\c@table=\count188
\abovecaptionskip=\skip47
\belowcaptionskip=\skip48
\bibindent=\dimen138
) (amsmath.sty
Package: amsmath 2021/10/15 v2.17l AMS math features
\@mathmargin=\skip49

For additional information on amsmath, use the `?' option.
(amstext.sty
Package: amstext 2021/08/26 v2.01 AMS text
 (amsgen.sty
File: amsgen.sty 1999/11/30 v2.0 generic functions
\@emptytoks=\toks16
\ex@=\dimen139
)) (amsbsy.sty
Package: amsbsy 1999/11/29 v1.2d Bold Symbols
\pmbraise@=\dimen140
) (amsopn.sty
Package: amsopn 2021/08/26 v2.02 operator names
)
\inf@bad=\count189
LaTeX Info: Redefining \frac on input line 234.
\uproot@=\count190
\leftroot@=\count191
LaTeX Info: Redefining \overline on input line 399.
\classnum@=\count192
\DOTSCASE@=\count193
LaTeX Info: Redefining \ldots on input line 496.
LaTeX Info: Redefining \dots on input line 499.
LaTeX Info: Redefining \cdots on input line 620.
\Mathstrutbox@=\box50
\strutbox@=\box51
\big@size=\dimen141
LaTeX Font Info:    Redeclaring font encoding OML on input line 743.
LaTeX Font Info:    Redeclaring font encoding OMS on input line 744.
\macc@depth=\count194
\c@MaxMatrixCols=\count195
\dotsspace@=\muskip16
\c@parentequation=\count196
\dspbrk@lvl=\count197
\tag@help=\toks17
\row@=\count198
\column@=\count199
\maxfields@=\count266
\andhelp@=\toks18
\eqnshift@=\dimen142
\alignsep@=\dimen143
\tagshift@=\dimen144
\tagwidth@=\dimen145
\totwidth@=\dimen146
\lineht@=\dimen147
\@envbody=\toks19
\multlinegap=\skip50
\multlinetaggap=\skip51
\mathdisplay@stack=\toks20
LaTeX Info: Redefining \[ on input line 2938.
LaTeX Info: Redefining \] on input line 2939.
) (amssymb.sty
Package: amssymb 2013/01/14 v3.01 AMS font symbols

(amsfonts.sty
Package: amsfonts 2013/01/14 v3.01 Basic AMSFonts support
\symAMSa=\mathgroup4
\symAMSb=\mathgroup5
LaTeX Font Info:    Redeclaring math symbol \hbar on input line 98.
LaTeX Font Info:    Overwriting math alphabet `\mathfrak' in version `bold'
(Font)                  U/euf/m/n --> U/euf/b/n on input line 106.
)) (fontspec.sty (xparse.sty (expl3.sty
Package: expl3 2022-02-24 L3 programming layer (loader) 
 (l3backend-xetex.def
File: l3backend-xetex.def 2022-02-07 L3 backend support: XeTeX
\c__kernel_sys_dvipdfmx_version_int=\count267
\l__color_backend_stack_int=\count268
\g__color_backend_stack_int=\count269
\g__graphics_track_int=\count270
\l__pdf_internal_box=\box52
\g__pdf_backend_object_int=\count271
\g__pdf_backend_annotation_int=\count272
\g__pdf_backend_link_int=\count273
))
Package: xparse 2022-01-12 L3 Experimental document command parser
)
Package: fontspec 2022/01/15 v2.8a Font selection for XeLaTeX and LuaLaTeX

(fontspec-xetex.sty
Package: fontspec-xetex 2022/01/15 v2.8a Font selection for XeLaTeX and LuaLaTe
X
\l__fontspec_script_int=\count274
\l__fontspec_language_int=\count275
\l__fontspec_strnum_int=\count276
\l__fontspec_tmp_int=\count277
\l__fontspec_tmpa_int=\count278
\l__fontspec_tmpb_int=\count279
\l__fontspec_tmpc_int=\count280
\l__fontspec_em_int=\count281
\l__fontspec_emdef_int=\count282
\l__fontspec_strong_int=\count283
\l__fontspec_strongdef_int=\count284
\l__fontspec_tmpa_dim=\dimen148
\l__fontspec_tmpb_dim=\dimen149
\l__fontspec_tmpc_dim=\dimen150
 (fontenc.sty
Package: fontenc 2021/04/29 v2.0v Standard LaTeX package
) (fontspec.cfg)))

Package fontspec Info: Font family 'lmroman10(0)' created for font 'lmroman10'
(fontspec)             with options
(fontspec)             [Ligatures=TeX,Extension=.otf,UprightFont=*-regular,Bold
Font=*-bold,ItalicFont=*-italic,BoldItalicFont=*-bolditalic].
(fontspec)              
(fontspec)              This font family consists of the following NFSS
(fontspec)             series/shapes:
(fontspec)              
(fontspec)             - 'normal' (m/n) with NFSS spec.:
(fontspec)             <->"[lmroman10-regular.otf]/OT:script=latn;language=dflt
;mapping=tex-text;"
(fontspec)             - 'small caps'  (m/sc) with NFSS spec.: 
(fontspec)             - 'bold' (b/n) with NFSS spec.:
(fontspec)             <->"[lmroman10-bold.otf]/OT:script=latn;language=dflt;ma
pping=tex-text;"
(fontspec)             - 'bold small caps'  (b/sc) with NFSS spec.: 
(fontspec)             - 'italic' (m/it) with NFSS spec.:
(fontspec)             <->"[lmroman10-italic.otf]/OT:script=latn;language=dflt;
mapping=tex-text;"
(fontspec)             - 'italic small caps'  (m/scit) with NFSS spec.: 
(fontspec)             - 'bold italic' (b/it) with NFSS spec.:
(fontspec)             <->"[lmroman10-bolditalic.otf]/OT:script=latn;language=d
flt;mapping=tex-text;"
(fontspec)             - 'bold italic small caps'  (b/scit) with NFSS spec.: 


Package fontspec Info: Font family 'lmmono10(0)' created for font 'lmmono10'
(fontspec)             with options
(fontspec)             [WordSpace={1,0,0},HyphenChar=None,PunctuationSpace=Word
Space,Extension=.otf,UprightFont=*-regular,ItalicFont=*-italic,BoldFont=lmmonol
t10-bold,BoldItalicFont=lmmonolt10-boldoblique].
(fontspec)              
(fontspec)              This font family consists of the following NFSS
(fontspec)             series/shapes:
(fontspec)              
(fontspec)             - 'normal' (m/n) with NFSS spec.:
(fontspec)             <->"[lmmono10-regular.otf]/OT:script=latn;language=dflt;
"
(fontspec)             - 'small caps'  (m/sc) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'bold' (b/n) with NFSS spec.:
(fontspec)             <->"[lmmonolt10-bold.otf]/OT:script=latn;language=dflt;"

(fontspec)             - 'bold small caps'  (b/sc) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'italic' (m/it) with NFSS spec.:
(fontspec)             <->"[lmmono10-italic.otf]/OT:script=latn;language=dflt;"

(fontspec)             - 'italic small caps'  (m/scit) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'bold italic' (b/it) with NFSS spec.:
(fontspec)             <->"[lmmonolt10-boldoblique.otf]/OT:script=latn;language
=dflt;"
(fontspec)             - 'bold italic small caps'  (b/scit) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 

 (notation.tex)
(addressing.tex
\l_ftdoc_chunks_int=\count285
)
No file document.aux.
\openout1 = `document.aux'.

LaTeX Font Info:    Checking defaults for OML/cmm/m/it on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for OMS/cmsy/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for OT1/cmr/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for T1/cmr/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for TS1/cmr/m/n on input line 11.
LaTeX Font Info:    Trying to load font information for TS1+cmr on input line 1
1.
(ts1cmr.fd
File: ts1cmr.fd 2019/12/16 v2.5j Standard LaTeX font definitions
)
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for TU/lmr/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for OMX/cmex/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for U/cmr/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.

Package fontspec Info: Adjusting the maths setup (use [no-math] to avoid
(fontspec)             this).

\symlegacymaths=\mathgroup6
LaTeX Font Info:    Overwriting symbol font `legacymaths' in version `bold'
(Font)                  OT1/cmr/m/n --> OT1/cmr/bx/n on input line 11.
LaTeX Font Info:    Redeclaring math accent \acute on input line 11.
LaTeX Font Info:    Redeclaring math accent \grave on input line 11.
LaTeX Font Info:    Redeclaring math accent \ddot on input line 11.
LaTeX Font Info:    Redeclaring math accent \tilde on input line 11.
LaTeX Font Info:    Redeclaring math accent \bar on input line 11.
LaTeX Font Info:    Redeclaring math accent \breve on input line 11.
LaTeX Font Info:    Redeclaring math accent \check on input line 11.
LaTeX Font Info:    Redeclaring math accent \hat on input line 11.
LaTeX Font Info:    Redeclaring math accent \dot on input line 11.
LaTeX Font Info:    Redeclaring math accent \mathring on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Gamma on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Delta on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Theta on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Lambda on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Xi on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Pi on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Sigma on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Upsilon on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Phi on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Psi on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Omega on input line 11.
LaTeX Font Info:    Redeclaring math symbol \mathdollar on input line 11.
LaTeX Font Info:    Redeclaring symbol font `operators' on input line 11.
LaTeX Font Info:    Encoding `OT1' has changed to `TU' for symbol font
(Font)              `operators' in the math version `normal' on input line 11.
LaTeX Font Info:    Overwriting symbol font `operators' in version `normal'
(Font)                  OT1/cmr/m/n --> TU/lmroman10(0)/m/n on input line 11.
LaTeX Font Info:    Encoding `OT1' has changed to `TU' for symbol font
(Font)              `operators' in the math version `bold' on input line 11.
LaTeX Font Info:    Overwriting symbol font `operators' in version `bold'
(Font)                  OT1/cmr/bx/n --> TU/lmroman10(0)/m/n on input line 11.
LaTeX Font Info:    Overwriting symbol font `operators' in version `normal'
(Font)                  TU/lmroman10(0)/m/n --> TU/lmroman10(0)/m/n on input li
ne 11.
LaTeX Font Info:    Overwriting math alphabet `\mathit' in version `normal'
(Font)                  OT1/cmr/m/it --> TU/lmroman10(0)/m/it on input line 11.

LaTeX Font Info:    Overwriting math alphabet `\mathbf' in version `normal'
(Font)                  OT1/cmr/bx/n --> TU/lmroman10(0)/b/n on input line 11.
LaTeX Font Info:    Overwriting math alphabet `\mathsf' in version `normal'
(Font)                  OT1/cmss/m/n --> TU/lmss/m/n on input line 11.
LaTeX Font Info:    Overwriting math alphabet `\mathtt' in version `normal'
(Font)                  OT1/cmtt/m/n --> TU/lmmono10(0)/m/n on input line 11.
LaTeX Font Info:    Overwriting symbol font `operators' in version `bold'
(Font)                  TU/lmroman10(0)/m/n --> TU/lmroman10(0)/b/n on input li
ne 11.
LaTeX Font Info:    Overwriting math alphabet `\mathit' in version `bold'
(Font)                  OT1/cmr/bx/it --> TU/lmroman10(0)/b/it on input line 11
.
LaTeX Font Info:    Overwriting math alphabet `\mathsf' in version `bold'
(Font)                  OT1/cmss/bx/n --> TU/lmss/b/n on input line 11.
LaTeX Font Info:    Overwriting math alphabet `\mathtt' in version `bold'
(Font)                  OT1/cmtt/m/n --> TU/lmmono10(0)/b/n on input line 11.

! Undefined control sequence.
\FtdocStored ->\UndefinedStoredCommand 
                                       
l.12 \FtdocStored
                 
No pages of output.
"#,
        ),
        "collision" => (
            r#"note: using only cached resource files
note: generating format "latex"
note: Running TeX ...
note: Writing `@SCRATCH@/output/document.log` (7.95 KiB)
"#,
            r"warning: open of input /dev/null failed
caused by: access to the path `/dev/null` is forbidden
error: notation.tex:1: LaTeX Error: Command \FtooQCoefficientOne already defined.
               Or name \end... illegal, see p.192 of the manual.

See the LaTeX manual or LaTeX Companion for explanation.
Type  H <return>  for immediate help
error: halted on potentially-recoverable error as specified
",
            r#"**
(document.tex
LaTeX2e <2021-11-15> patch level 1
L3 programming layer <2022-02-24> (article.cls
Document Class: article 2021/10/04 v1.4n Standard LaTeX document class
(size10.clo
File: size10.clo 2021/10/04 v1.4n Standard LaTeX file (size option)
)
\c@part=\count181
\c@section=\count182
\c@subsection=\count183
\c@subsubsection=\count184
\c@paragraph=\count185
\c@subparagraph=\count186
\c@figure=\count187
\c@table=\count188
\abovecaptionskip=\skip47
\belowcaptionskip=\skip48
\bibindent=\dimen138
) (amsmath.sty
Package: amsmath 2021/10/15 v2.17l AMS math features
\@mathmargin=\skip49

For additional information on amsmath, use the `?' option.
(amstext.sty
Package: amstext 2021/08/26 v2.01 AMS text
 (amsgen.sty
File: amsgen.sty 1999/11/30 v2.0 generic functions
\@emptytoks=\toks16
\ex@=\dimen139
)) (amsbsy.sty
Package: amsbsy 1999/11/29 v1.2d Bold Symbols
\pmbraise@=\dimen140
) (amsopn.sty
Package: amsopn 2021/08/26 v2.02 operator names
)
\inf@bad=\count189
LaTeX Info: Redefining \frac on input line 234.
\uproot@=\count190
\leftroot@=\count191
LaTeX Info: Redefining \overline on input line 399.
\classnum@=\count192
\DOTSCASE@=\count193
LaTeX Info: Redefining \ldots on input line 496.
LaTeX Info: Redefining \dots on input line 499.
LaTeX Info: Redefining \cdots on input line 620.
\Mathstrutbox@=\box50
\strutbox@=\box51
\big@size=\dimen141
LaTeX Font Info:    Redeclaring font encoding OML on input line 743.
LaTeX Font Info:    Redeclaring font encoding OMS on input line 744.
\macc@depth=\count194
\c@MaxMatrixCols=\count195
\dotsspace@=\muskip16
\c@parentequation=\count196
\dspbrk@lvl=\count197
\tag@help=\toks17
\row@=\count198
\column@=\count199
\maxfields@=\count266
\andhelp@=\toks18
\eqnshift@=\dimen142
\alignsep@=\dimen143
\tagshift@=\dimen144
\tagwidth@=\dimen145
\totwidth@=\dimen146
\lineht@=\dimen147
\@envbody=\toks19
\multlinegap=\skip50
\multlinetaggap=\skip51
\mathdisplay@stack=\toks20
LaTeX Info: Redefining \[ on input line 2938.
LaTeX Info: Redefining \] on input line 2939.
) (amssymb.sty
Package: amssymb 2013/01/14 v3.01 AMS font symbols

(amsfonts.sty
Package: amsfonts 2013/01/14 v3.01 Basic AMSFonts support
\symAMSa=\mathgroup4
\symAMSb=\mathgroup5
LaTeX Font Info:    Redeclaring math symbol \hbar on input line 98.
LaTeX Font Info:    Overwriting math alphabet `\mathfrak' in version `bold'
(Font)                  U/euf/m/n --> U/euf/b/n on input line 106.
)) (fontspec.sty (xparse.sty (expl3.sty
Package: expl3 2022-02-24 L3 programming layer (loader) 
 (l3backend-xetex.def
File: l3backend-xetex.def 2022-02-07 L3 backend support: XeTeX
\c__kernel_sys_dvipdfmx_version_int=\count267
\l__color_backend_stack_int=\count268
\g__color_backend_stack_int=\count269
\g__graphics_track_int=\count270
\l__pdf_internal_box=\box52
\g__pdf_backend_object_int=\count271
\g__pdf_backend_annotation_int=\count272
\g__pdf_backend_link_int=\count273
))
Package: xparse 2022-01-12 L3 Experimental document command parser
)
Package: fontspec 2022/01/15 v2.8a Font selection for XeLaTeX and LuaLaTeX

(fontspec-xetex.sty
Package: fontspec-xetex 2022/01/15 v2.8a Font selection for XeLaTeX and LuaLaTe
X
\l__fontspec_script_int=\count274
\l__fontspec_language_int=\count275
\l__fontspec_strnum_int=\count276
\l__fontspec_tmp_int=\count277
\l__fontspec_tmpa_int=\count278
\l__fontspec_tmpb_int=\count279
\l__fontspec_tmpc_int=\count280
\l__fontspec_em_int=\count281
\l__fontspec_emdef_int=\count282
\l__fontspec_strong_int=\count283
\l__fontspec_strongdef_int=\count284
\l__fontspec_tmpa_dim=\dimen148
\l__fontspec_tmpb_dim=\dimen149
\l__fontspec_tmpc_dim=\dimen150
 (fontenc.sty
Package: fontenc 2021/04/29 v2.0v Standard LaTeX package
) (fontspec.cfg)))

Package fontspec Info: Font family 'lmroman10(0)' created for font 'lmroman10'
(fontspec)             with options
(fontspec)             [Ligatures=TeX,Extension=.otf,UprightFont=*-regular,Bold
Font=*-bold,ItalicFont=*-italic,BoldItalicFont=*-bolditalic].
(fontspec)              
(fontspec)              This font family consists of the following NFSS
(fontspec)             series/shapes:
(fontspec)              
(fontspec)             - 'normal' (m/n) with NFSS spec.:
(fontspec)             <->"[lmroman10-regular.otf]/OT:script=latn;language=dflt
;mapping=tex-text;"
(fontspec)             - 'small caps'  (m/sc) with NFSS spec.: 
(fontspec)             - 'bold' (b/n) with NFSS spec.:
(fontspec)             <->"[lmroman10-bold.otf]/OT:script=latn;language=dflt;ma
pping=tex-text;"
(fontspec)             - 'bold small caps'  (b/sc) with NFSS spec.: 
(fontspec)             - 'italic' (m/it) with NFSS spec.:
(fontspec)             <->"[lmroman10-italic.otf]/OT:script=latn;language=dflt;
mapping=tex-text;"
(fontspec)             - 'italic small caps'  (m/scit) with NFSS spec.: 
(fontspec)             - 'bold italic' (b/it) with NFSS spec.:
(fontspec)             <->"[lmroman10-bolditalic.otf]/OT:script=latn;language=d
flt;mapping=tex-text;"
(fontspec)             - 'bold italic small caps'  (b/scit) with NFSS spec.: 


Package fontspec Info: Font family 'lmmono10(0)' created for font 'lmmono10'
(fontspec)             with options
(fontspec)             [WordSpace={1,0,0},HyphenChar=None,PunctuationSpace=Word
Space,Extension=.otf,UprightFont=*-regular,ItalicFont=*-italic,BoldFont=lmmonol
t10-bold,BoldItalicFont=lmmonolt10-boldoblique].
(fontspec)              
(fontspec)              This font family consists of the following NFSS
(fontspec)             series/shapes:
(fontspec)              
(fontspec)             - 'normal' (m/n) with NFSS spec.:
(fontspec)             <->"[lmmono10-regular.otf]/OT:script=latn;language=dflt;
"
(fontspec)             - 'small caps'  (m/sc) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'bold' (b/n) with NFSS spec.:
(fontspec)             <->"[lmmonolt10-bold.otf]/OT:script=latn;language=dflt;"

(fontspec)             - 'bold small caps'  (b/sc) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'italic' (m/it) with NFSS spec.:
(fontspec)             <->"[lmmono10-italic.otf]/OT:script=latn;language=dflt;"

(fontspec)             - 'italic small caps'  (m/scit) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'bold italic' (b/it) with NFSS spec.:
(fontspec)             <->"[lmmonolt10-boldoblique.otf]/OT:script=latn;language
=dflt;"
(fontspec)             - 'bold italic small caps'  (b/scit) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 

 (notation.tex

! LaTeX Error: Command \FtooQCoefficientOne already defined.
               Or name \end... illegal, see p.192 of the manual.

See the LaTeX manual or LaTeX Companion for explanation.
Type  H <return>  for immediate help.
 ...                                              
                                                  
l.1 \newcommand{\FtooQCoefficientOne}[0]{1}
                                           
No pages of output.
"#,
        ),
        "include" => (
            r#"note: using only cached resource files
note: generating format "latex"
note: Running TeX ...
note: Writing `@SCRATCH@/output/document.log` (7.77 KiB)
"#,
            r"warning: open of input /dev/null failed
caused by: access to the path `/dev/null` is forbidden
error: document.tex:9: ! LaTeX Error: File `missing-addressing.tex' not found.
error: halted on potentially-recoverable error as specified
",
            r#"**
(document.tex
LaTeX2e <2021-11-15> patch level 1
L3 programming layer <2022-02-24> (article.cls
Document Class: article 2021/10/04 v1.4n Standard LaTeX document class
(size10.clo
File: size10.clo 2021/10/04 v1.4n Standard LaTeX file (size option)
)
\c@part=\count181
\c@section=\count182
\c@subsection=\count183
\c@subsubsection=\count184
\c@paragraph=\count185
\c@subparagraph=\count186
\c@figure=\count187
\c@table=\count188
\abovecaptionskip=\skip47
\belowcaptionskip=\skip48
\bibindent=\dimen138
) (amsmath.sty
Package: amsmath 2021/10/15 v2.17l AMS math features
\@mathmargin=\skip49

For additional information on amsmath, use the `?' option.
(amstext.sty
Package: amstext 2021/08/26 v2.01 AMS text
 (amsgen.sty
File: amsgen.sty 1999/11/30 v2.0 generic functions
\@emptytoks=\toks16
\ex@=\dimen139
)) (amsbsy.sty
Package: amsbsy 1999/11/29 v1.2d Bold Symbols
\pmbraise@=\dimen140
) (amsopn.sty
Package: amsopn 2021/08/26 v2.02 operator names
)
\inf@bad=\count189
LaTeX Info: Redefining \frac on input line 234.
\uproot@=\count190
\leftroot@=\count191
LaTeX Info: Redefining \overline on input line 399.
\classnum@=\count192
\DOTSCASE@=\count193
LaTeX Info: Redefining \ldots on input line 496.
LaTeX Info: Redefining \dots on input line 499.
LaTeX Info: Redefining \cdots on input line 620.
\Mathstrutbox@=\box50
\strutbox@=\box51
\big@size=\dimen141
LaTeX Font Info:    Redeclaring font encoding OML on input line 743.
LaTeX Font Info:    Redeclaring font encoding OMS on input line 744.
\macc@depth=\count194
\c@MaxMatrixCols=\count195
\dotsspace@=\muskip16
\c@parentequation=\count196
\dspbrk@lvl=\count197
\tag@help=\toks17
\row@=\count198
\column@=\count199
\maxfields@=\count266
\andhelp@=\toks18
\eqnshift@=\dimen142
\alignsep@=\dimen143
\tagshift@=\dimen144
\tagwidth@=\dimen145
\totwidth@=\dimen146
\lineht@=\dimen147
\@envbody=\toks19
\multlinegap=\skip50
\multlinetaggap=\skip51
\mathdisplay@stack=\toks20
LaTeX Info: Redefining \[ on input line 2938.
LaTeX Info: Redefining \] on input line 2939.
) (amssymb.sty
Package: amssymb 2013/01/14 v3.01 AMS font symbols

(amsfonts.sty
Package: amsfonts 2013/01/14 v3.01 Basic AMSFonts support
\symAMSa=\mathgroup4
\symAMSb=\mathgroup5
LaTeX Font Info:    Redeclaring math symbol \hbar on input line 98.
LaTeX Font Info:    Overwriting math alphabet `\mathfrak' in version `bold'
(Font)                  U/euf/m/n --> U/euf/b/n on input line 106.
)) (fontspec.sty (xparse.sty (expl3.sty
Package: expl3 2022-02-24 L3 programming layer (loader) 
 (l3backend-xetex.def
File: l3backend-xetex.def 2022-02-07 L3 backend support: XeTeX
\c__kernel_sys_dvipdfmx_version_int=\count267
\l__color_backend_stack_int=\count268
\g__color_backend_stack_int=\count269
\g__graphics_track_int=\count270
\l__pdf_internal_box=\box52
\g__pdf_backend_object_int=\count271
\g__pdf_backend_annotation_int=\count272
\g__pdf_backend_link_int=\count273
))
Package: xparse 2022-01-12 L3 Experimental document command parser
)
Package: fontspec 2022/01/15 v2.8a Font selection for XeLaTeX and LuaLaTeX

(fontspec-xetex.sty
Package: fontspec-xetex 2022/01/15 v2.8a Font selection for XeLaTeX and LuaLaTe
X
\l__fontspec_script_int=\count274
\l__fontspec_language_int=\count275
\l__fontspec_strnum_int=\count276
\l__fontspec_tmp_int=\count277
\l__fontspec_tmpa_int=\count278
\l__fontspec_tmpb_int=\count279
\l__fontspec_tmpc_int=\count280
\l__fontspec_em_int=\count281
\l__fontspec_emdef_int=\count282
\l__fontspec_strong_int=\count283
\l__fontspec_strongdef_int=\count284
\l__fontspec_tmpa_dim=\dimen148
\l__fontspec_tmpb_dim=\dimen149
\l__fontspec_tmpc_dim=\dimen150
 (fontenc.sty
Package: fontenc 2021/04/29 v2.0v Standard LaTeX package
) (fontspec.cfg)))

Package fontspec Info: Font family 'lmroman10(0)' created for font 'lmroman10'
(fontspec)             with options
(fontspec)             [Ligatures=TeX,Extension=.otf,UprightFont=*-regular,Bold
Font=*-bold,ItalicFont=*-italic,BoldItalicFont=*-bolditalic].
(fontspec)              
(fontspec)              This font family consists of the following NFSS
(fontspec)             series/shapes:
(fontspec)              
(fontspec)             - 'normal' (m/n) with NFSS spec.:
(fontspec)             <->"[lmroman10-regular.otf]/OT:script=latn;language=dflt
;mapping=tex-text;"
(fontspec)             - 'small caps'  (m/sc) with NFSS spec.: 
(fontspec)             - 'bold' (b/n) with NFSS spec.:
(fontspec)             <->"[lmroman10-bold.otf]/OT:script=latn;language=dflt;ma
pping=tex-text;"
(fontspec)             - 'bold small caps'  (b/sc) with NFSS spec.: 
(fontspec)             - 'italic' (m/it) with NFSS spec.:
(fontspec)             <->"[lmroman10-italic.otf]/OT:script=latn;language=dflt;
mapping=tex-text;"
(fontspec)             - 'italic small caps'  (m/scit) with NFSS spec.: 
(fontspec)             - 'bold italic' (b/it) with NFSS spec.:
(fontspec)             <->"[lmroman10-bolditalic.otf]/OT:script=latn;language=d
flt;mapping=tex-text;"
(fontspec)             - 'bold italic small caps'  (b/scit) with NFSS spec.: 


Package fontspec Info: Font family 'lmmono10(0)' created for font 'lmmono10'
(fontspec)             with options
(fontspec)             [WordSpace={1,0,0},HyphenChar=None,PunctuationSpace=Word
Space,Extension=.otf,UprightFont=*-regular,ItalicFont=*-italic,BoldFont=lmmonol
t10-bold,BoldItalicFont=lmmonolt10-boldoblique].
(fontspec)              
(fontspec)              This font family consists of the following NFSS
(fontspec)             series/shapes:
(fontspec)              
(fontspec)             - 'normal' (m/n) with NFSS spec.:
(fontspec)             <->"[lmmono10-regular.otf]/OT:script=latn;language=dflt;
"
(fontspec)             - 'small caps'  (m/sc) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'bold' (b/n) with NFSS spec.:
(fontspec)             <->"[lmmonolt10-bold.otf]/OT:script=latn;language=dflt;"

(fontspec)             - 'bold small caps'  (b/sc) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'italic' (m/it) with NFSS spec.:
(fontspec)             <->"[lmmono10-italic.otf]/OT:script=latn;language=dflt;"

(fontspec)             - 'italic small caps'  (m/scit) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'bold italic' (b/it) with NFSS spec.:
(fontspec)             <->"[lmmonolt10-boldoblique.otf]/OT:script=latn;language
=dflt;"
(fontspec)             - 'bold italic small caps'  (b/scit) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 

 (notation.tex)
! ! LaTeX Error: File `missing-addressing.tex' not found..
\@missingfileerror ...or: File `#1.#2' not found.}
                                                  
l.9 \input{missing-addressing.tex}
                                  
No pages of output.
"#,
        ),
        "closure" => (
            r#"note: using only cached resource files
note: generating format "latex"
note: Running TeX ...
note: Writing `@SCRATCH@/output/document.log` (12.55 KiB)
"#,
            r"warning: open of input /dev/null failed
caused by: access to the path `/dev/null` is forbidden
error: !Emergency stop
*** (job aborted, no legal \end found)
error: *** (job aborted, no legal \end found)
",
            r#"**
(document.tex
LaTeX2e <2021-11-15> patch level 1
L3 programming layer <2022-02-24> (article.cls
Document Class: article 2021/10/04 v1.4n Standard LaTeX document class
(size10.clo
File: size10.clo 2021/10/04 v1.4n Standard LaTeX file (size option)
)
\c@part=\count181
\c@section=\count182
\c@subsection=\count183
\c@subsubsection=\count184
\c@paragraph=\count185
\c@subparagraph=\count186
\c@figure=\count187
\c@table=\count188
\abovecaptionskip=\skip47
\belowcaptionskip=\skip48
\bibindent=\dimen138
) (amsmath.sty
Package: amsmath 2021/10/15 v2.17l AMS math features
\@mathmargin=\skip49

For additional information on amsmath, use the `?' option.
(amstext.sty
Package: amstext 2021/08/26 v2.01 AMS text
 (amsgen.sty
File: amsgen.sty 1999/11/30 v2.0 generic functions
\@emptytoks=\toks16
\ex@=\dimen139
)) (amsbsy.sty
Package: amsbsy 1999/11/29 v1.2d Bold Symbols
\pmbraise@=\dimen140
) (amsopn.sty
Package: amsopn 2021/08/26 v2.02 operator names
)
\inf@bad=\count189
LaTeX Info: Redefining \frac on input line 234.
\uproot@=\count190
\leftroot@=\count191
LaTeX Info: Redefining \overline on input line 399.
\classnum@=\count192
\DOTSCASE@=\count193
LaTeX Info: Redefining \ldots on input line 496.
LaTeX Info: Redefining \dots on input line 499.
LaTeX Info: Redefining \cdots on input line 620.
\Mathstrutbox@=\box50
\strutbox@=\box51
\big@size=\dimen141
LaTeX Font Info:    Redeclaring font encoding OML on input line 743.
LaTeX Font Info:    Redeclaring font encoding OMS on input line 744.
\macc@depth=\count194
\c@MaxMatrixCols=\count195
\dotsspace@=\muskip16
\c@parentequation=\count196
\dspbrk@lvl=\count197
\tag@help=\toks17
\row@=\count198
\column@=\count199
\maxfields@=\count266
\andhelp@=\toks18
\eqnshift@=\dimen142
\alignsep@=\dimen143
\tagshift@=\dimen144
\tagwidth@=\dimen145
\totwidth@=\dimen146
\lineht@=\dimen147
\@envbody=\toks19
\multlinegap=\skip50
\multlinetaggap=\skip51
\mathdisplay@stack=\toks20
LaTeX Info: Redefining \[ on input line 2938.
LaTeX Info: Redefining \] on input line 2939.
) (amssymb.sty
Package: amssymb 2013/01/14 v3.01 AMS font symbols

(amsfonts.sty
Package: amsfonts 2013/01/14 v3.01 Basic AMSFonts support
\symAMSa=\mathgroup4
\symAMSb=\mathgroup5
LaTeX Font Info:    Redeclaring math symbol \hbar on input line 98.
LaTeX Font Info:    Overwriting math alphabet `\mathfrak' in version `bold'
(Font)                  U/euf/m/n --> U/euf/b/n on input line 106.
)) (fontspec.sty (xparse.sty (expl3.sty
Package: expl3 2022-02-24 L3 programming layer (loader) 
 (l3backend-xetex.def
File: l3backend-xetex.def 2022-02-07 L3 backend support: XeTeX
\c__kernel_sys_dvipdfmx_version_int=\count267
\l__color_backend_stack_int=\count268
\g__color_backend_stack_int=\count269
\g__graphics_track_int=\count270
\l__pdf_internal_box=\box52
\g__pdf_backend_object_int=\count271
\g__pdf_backend_annotation_int=\count272
\g__pdf_backend_link_int=\count273
))
Package: xparse 2022-01-12 L3 Experimental document command parser
)
Package: fontspec 2022/01/15 v2.8a Font selection for XeLaTeX and LuaLaTeX

(fontspec-xetex.sty
Package: fontspec-xetex 2022/01/15 v2.8a Font selection for XeLaTeX and LuaLaTe
X
\l__fontspec_script_int=\count274
\l__fontspec_language_int=\count275
\l__fontspec_strnum_int=\count276
\l__fontspec_tmp_int=\count277
\l__fontspec_tmpa_int=\count278
\l__fontspec_tmpb_int=\count279
\l__fontspec_tmpc_int=\count280
\l__fontspec_em_int=\count281
\l__fontspec_emdef_int=\count282
\l__fontspec_strong_int=\count283
\l__fontspec_strongdef_int=\count284
\l__fontspec_tmpa_dim=\dimen148
\l__fontspec_tmpb_dim=\dimen149
\l__fontspec_tmpc_dim=\dimen150
 (fontenc.sty
Package: fontenc 2021/04/29 v2.0v Standard LaTeX package
) (fontspec.cfg)))

Package fontspec Info: Font family 'lmroman10(0)' created for font 'lmroman10'
(fontspec)             with options
(fontspec)             [Ligatures=TeX,Extension=.otf,UprightFont=*-regular,Bold
Font=*-bold,ItalicFont=*-italic,BoldItalicFont=*-bolditalic].
(fontspec)              
(fontspec)              This font family consists of the following NFSS
(fontspec)             series/shapes:
(fontspec)              
(fontspec)             - 'normal' (m/n) with NFSS spec.:
(fontspec)             <->"[lmroman10-regular.otf]/OT:script=latn;language=dflt
;mapping=tex-text;"
(fontspec)             - 'small caps'  (m/sc) with NFSS spec.: 
(fontspec)             - 'bold' (b/n) with NFSS spec.:
(fontspec)             <->"[lmroman10-bold.otf]/OT:script=latn;language=dflt;ma
pping=tex-text;"
(fontspec)             - 'bold small caps'  (b/sc) with NFSS spec.: 
(fontspec)             - 'italic' (m/it) with NFSS spec.:
(fontspec)             <->"[lmroman10-italic.otf]/OT:script=latn;language=dflt;
mapping=tex-text;"
(fontspec)             - 'italic small caps'  (m/scit) with NFSS spec.: 
(fontspec)             - 'bold italic' (b/it) with NFSS spec.:
(fontspec)             <->"[lmroman10-bolditalic.otf]/OT:script=latn;language=d
flt;mapping=tex-text;"
(fontspec)             - 'bold italic small caps'  (b/scit) with NFSS spec.: 


Package fontspec Info: Font family 'lmmono10(0)' created for font 'lmmono10'
(fontspec)             with options
(fontspec)             [WordSpace={1,0,0},HyphenChar=None,PunctuationSpace=Word
Space,Extension=.otf,UprightFont=*-regular,ItalicFont=*-italic,BoldFont=lmmonol
t10-bold,BoldItalicFont=lmmonolt10-boldoblique].
(fontspec)              
(fontspec)              This font family consists of the following NFSS
(fontspec)             series/shapes:
(fontspec)              
(fontspec)             - 'normal' (m/n) with NFSS spec.:
(fontspec)             <->"[lmmono10-regular.otf]/OT:script=latn;language=dflt;
"
(fontspec)             - 'small caps'  (m/sc) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'bold' (b/n) with NFSS spec.:
(fontspec)             <->"[lmmonolt10-bold.otf]/OT:script=latn;language=dflt;"

(fontspec)             - 'bold small caps'  (b/sc) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'italic' (m/it) with NFSS spec.:
(fontspec)             <->"[lmmono10-italic.otf]/OT:script=latn;language=dflt;"

(fontspec)             - 'italic small caps'  (m/scit) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 
(fontspec)             - 'bold italic' (b/it) with NFSS spec.:
(fontspec)             <->"[lmmonolt10-boldoblique.otf]/OT:script=latn;language
=dflt;"
(fontspec)             - 'bold italic small caps'  (b/scit) with NFSS spec.: 
(fontspec)             and font adjustment code:
(fontspec)             \fontdimen 2\font =1\fontdimen 2\font \fontdimen 3\font
(fontspec)             =0\fontdimen 3\font \fontdimen 4\font =0\fontdimen
(fontspec)             4\font \fontdimen 7\font =0\fontdimen 2\font
(fontspec)             \tex_hyphenchar:D \font =-1\scan_stop: 

 (notation.tex)
(addressing.tex
\l_ftdoc_chunks_int=\count285
)
No file document.aux.
\openout1 = `document.aux'.

LaTeX Font Info:    Checking defaults for OML/cmm/m/it on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for OMS/cmsy/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for OT1/cmr/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for T1/cmr/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for TS1/cmr/m/n on input line 11.
LaTeX Font Info:    Trying to load font information for TS1+cmr on input line 1
1.
(ts1cmr.fd
File: ts1cmr.fd 2019/12/16 v2.5j Standard LaTeX font definitions
)
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for TU/lmr/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for OMX/cmex/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.
LaTeX Font Info:    Checking defaults for U/cmr/m/n on input line 11.
LaTeX Font Info:    ... okay on input line 11.

Package fontspec Info: Adjusting the maths setup (use [no-math] to avoid
(fontspec)             this).

\symlegacymaths=\mathgroup6
LaTeX Font Info:    Overwriting symbol font `legacymaths' in version `bold'
(Font)                  OT1/cmr/m/n --> OT1/cmr/bx/n on input line 11.
LaTeX Font Info:    Redeclaring math accent \acute on input line 11.
LaTeX Font Info:    Redeclaring math accent \grave on input line 11.
LaTeX Font Info:    Redeclaring math accent \ddot on input line 11.
LaTeX Font Info:    Redeclaring math accent \tilde on input line 11.
LaTeX Font Info:    Redeclaring math accent \bar on input line 11.
LaTeX Font Info:    Redeclaring math accent \breve on input line 11.
LaTeX Font Info:    Redeclaring math accent \check on input line 11.
LaTeX Font Info:    Redeclaring math accent \hat on input line 11.
LaTeX Font Info:    Redeclaring math accent \dot on input line 11.
LaTeX Font Info:    Redeclaring math accent \mathring on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Gamma on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Delta on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Theta on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Lambda on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Xi on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Pi on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Sigma on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Upsilon on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Phi on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Psi on input line 11.
LaTeX Font Info:    Redeclaring math symbol \Omega on input line 11.
LaTeX Font Info:    Redeclaring math symbol \mathdollar on input line 11.
LaTeX Font Info:    Redeclaring symbol font `operators' on input line 11.
LaTeX Font Info:    Encoding `OT1' has changed to `TU' for symbol font
(Font)              `operators' in the math version `normal' on input line 11.
LaTeX Font Info:    Overwriting symbol font `operators' in version `normal'
(Font)                  OT1/cmr/m/n --> TU/lmroman10(0)/m/n on input line 11.
LaTeX Font Info:    Encoding `OT1' has changed to `TU' for symbol font
(Font)              `operators' in the math version `bold' on input line 11.
LaTeX Font Info:    Overwriting symbol font `operators' in version `bold'
(Font)                  OT1/cmr/bx/n --> TU/lmroman10(0)/m/n on input line 11.
LaTeX Font Info:    Overwriting symbol font `operators' in version `normal'
(Font)                  TU/lmroman10(0)/m/n --> TU/lmroman10(0)/m/n on input li
ne 11.
LaTeX Font Info:    Overwriting math alphabet `\mathit' in version `normal'
(Font)                  OT1/cmr/m/it --> TU/lmroman10(0)/m/it on input line 11.

LaTeX Font Info:    Overwriting math alphabet `\mathbf' in version `normal'
(Font)                  OT1/cmr/bx/n --> TU/lmroman10(0)/b/n on input line 11.
LaTeX Font Info:    Overwriting math alphabet `\mathsf' in version `normal'
(Font)                  OT1/cmss/m/n --> TU/lmss/m/n on input line 11.
LaTeX Font Info:    Overwriting math alphabet `\mathtt' in version `normal'
(Font)                  OT1/cmtt/m/n --> TU/lmmono10(0)/m/n on input line 11.
LaTeX Font Info:    Overwriting symbol font `operators' in version `bold'
(Font)                  TU/lmroman10(0)/m/n --> TU/lmroman10(0)/b/n on input li
ne 11.
LaTeX Font Info:    Overwriting math alphabet `\mathit' in version `bold'
(Font)                  OT1/cmr/bx/it --> TU/lmroman10(0)/b/it on input line 11
.
LaTeX Font Info:    Overwriting math alphabet `\mathsf' in version `bold'
(Font)                  OT1/cmss/bx/n --> TU/lmss/b/n on input line 11.
LaTeX Font Info:    Overwriting math alphabet `\mathtt' in version `bold'
(Font)                  OT1/cmtt/m/n --> TU/lmmono10(0)/b/n on input line 11.
)
! Emergency stop
*** (job aborted, no legal \end found)
No pages of output.
"#,
        ),
        _ => unreachable!("four fixed native cases"),
    }
}
