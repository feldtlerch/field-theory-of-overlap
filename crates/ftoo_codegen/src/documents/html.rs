use super::{
    NativeSupport,
    diagnostics::{AssemblyFault, DocumentError},
    failure, html_recovery,
    plan::{Block, DocumentPlan, Inline, StyleKind},
};
use crate::RenderableNotation;
use ftoo_manifest::{AssemblySite, AssemblyStage, DocumentOutput, RuleId, UnitSite};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
};

const MAX_HTML_BYTES: usize = 16_777_216;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Node {
    Text(String),
    Element {
        name: String,
        attributes: Vec<(String, String)>,
        children: Vec<Node>,
    },
}

fn element(name: &str, attributes: &[(&str, &str)], children: Vec<Node>) -> Node {
    Node::Element {
        name: name.into(),
        attributes: attributes
            .iter()
            .map(|(key, value)| ((*key).into(), (*value).into()))
            .collect(),
        children,
    }
}

#[derive(Clone, Copy)]
pub(super) struct Face {
    pub(super) file: &'static str,
    pub(super) digest: &'static str,
    pub(super) family: &'static str,
    pub(super) weight: u16,
    pub(super) italic: bool,
    pub(super) notice: &'static str,
}
impl Face {
    pub(super) fn is_math(&self) -> bool {
        self.file == MATH.file
    }
}

const REGULAR: Face = Face {
    file: "texgyrepagella-regular.otf",
    digest: "44e64260716d8f2bbe412baa1ee99b7c995190ac4573177c24def0b9200438c7",
    family: "Pagella",
    weight: 400,
    italic: false,
    notice: include_str!("../../../../licenses/fonts/notices/texgyrepagella-regular.otf.txt"),
};
const ITALIC: Face = Face {
    file: "texgyrepagella-italic.otf",
    digest: "069bb27f48eb98a741715227fc6e4b7baf3639c29fc13925f70d1ed8884ac29a",
    family: "Pagella",
    weight: 400,
    italic: true,
    notice: include_str!("../../../../licenses/fonts/notices/texgyrepagella-italic.otf.txt"),
};
const BOLD: Face = Face {
    file: "texgyrepagella-bold.otf",
    digest: "dd4f9b69f0b24f1fb9be8634abf1fce2009e5c68a30842a3e9f5d6e535900abd",
    family: "Pagella",
    weight: 700,
    italic: false,
    notice: include_str!("../../../../licenses/fonts/notices/texgyrepagella-bold.otf.txt"),
};
const BOLD_ITALIC: Face = Face {
    file: "texgyrepagella-bolditalic.otf",
    digest: "556fd64655a92bd9f5485ae2c03f7b9cdaf745a27a91d7edb3c3abde997206d2",
    family: "Pagella",
    weight: 700,
    italic: true,
    notice: include_str!("../../../../licenses/fonts/notices/texgyrepagella-bolditalic.otf.txt"),
};
const MONO: Face = Face {
    file: "Inconsolatazi4-Regular.otf",
    digest: "fa4d0728e461e228e4c3b706cc0836f72cb146852a757127c92647d8c405b638",
    family: "Inconsolata",
    weight: 400,
    italic: false,
    notice: include_str!("../../../../licenses/fonts/notices/Inconsolatazi4-Regular.otf.txt"),
};
const MATH: Face = Face {
    file: "texgyrepagella-math.otf",
    digest: "1f9e010f60e947d0e925910009b2ea85ad54edc7cefb106f8cdefb9ffd1d5f2f",
    family: "Pagella Math",
    weight: 400,
    italic: false,
    notice: include_str!("../../../../licenses/fonts/notices/texgyrepagella-math.otf.txt"),
};

#[derive(Clone)]
pub(super) struct MathRun {
    pub(super) block: usize,
    pub(super) run: usize,
    pub(super) source: String,
    pub(super) node: Node,
}

pub(super) struct Fonts(pub(super) Vec<(Face, Vec<u8>)>);

fn font_failure(plan: &DocumentPlan, file: &str) -> DocumentError {
    failure(
        RuleId::M055,
        AssemblyStage::Emission,
        AssemblySite::Source(plan.coverage.site.clone()),
        AssemblyFault::UnknownToken {
            value: file.into(),
            expected: vec!["pinned embedded font face".into()],
        },
    )
}
fn math_failure(site: &UnitSite, value: &str, expected: &str) -> DocumentError {
    failure(
        RuleId::M055,
        AssemblyStage::Emission,
        AssemblySite::Source(site.clone()),
        AssemblyFault::UnknownToken {
            value: value.into(),
            expected: vec![expected.into()],
        },
    )
}

fn admitted_math(source: &str, latex: &str) -> bool {
    matches!(
        source,
        "({\\underbrace{{c}_{1}}_{\\text{pending}}}={\\frac{1}{3}})" | "{c}_{1}"
    ) || (source == "S" && latex == "\\comovfield{}")
}

pub(super) fn checked_math_fragment(
    fragment: &Node,
    source: &str,
    site: &UnitSite,
) -> Result<Node, DocumentError> {
    html_recovery::math_profile(fragment, source)
        .ok_or_else(|| math_failure(site, source, "bounded exact-source MathML profile"))
}

pub(super) fn prepare_math(
    plan: &DocumentPlan,
    notation: Option<&RenderableNotation<'_, '_, '_>>,
) -> Result<Vec<MathRun>, DocumentError> {
    let mut prepared = Vec::new();
    for (block, section) in plan.blocks.iter().enumerate() {
        if !(section.paragraph || section.equation)
            || plan
                .screen
                .as_ref()
                .is_some_and(|screen| screen.html_gaps.contains_key(&block))
        {
            continue;
        }
        if section.equation {
            let (source, _) = section
                .equation_pair()
                .expect("checked equation shape before HTML emission");
            let Inline::Math { site, .. } = &section.runs[0] else {
                unreachable!("checked equation pair begins with mathematics")
            };
            if !realizable_equation(section) {
                return Err(math_failure(
                    site,
                    source,
                    "observed complete display profile",
                ));
            }
            let adapter =
                notation.ok_or_else(|| math_failure(site, source, "prepared notation adapter"))?;
            let returned = adapter
                .mathml_display(&format!("{source}."))
                .map_err(|error| {
                    math_failure(site, &format!("{error:?}"), "pinned strict MathML adapter")
                })?
                .map_err(|error| math_failure(site, &error, "strict MathML display expression"))?;
            let fragment = html_recovery::library_fragment(&returned)
                .ok_or_else(|| math_failure(site, source, "bounded MathML display fragment"))?;
            let node = html_recovery::display_math_profile(&fragment, source)
                .ok_or_else(|| math_failure(site, source, "observed complete display profile"))?;
            prepared.push(MathRun {
                block,
                run: 0,
                source: source.into(),
                node,
            });
            continue;
        }
        for (run, item) in section.runs.iter().enumerate() {
            let Inline::Math {
                markdown: source,
                latex,
                site,
                ..
            } = item
            else {
                continue;
            };
            if !admitted_math(source, latex) {
                return Err(math_failure(site, source, "realizable mathematical source"));
            }
            let adapter =
                notation.ok_or_else(|| math_failure(site, source, "prepared notation adapter"))?;
            let returned = adapter
                .mathml_inline(source)
                .map_err(|error| {
                    math_failure(site, &format!("{error:?}"), "pinned strict MathML adapter")
                })?
                .map_err(|error| math_failure(site, &error, "strict MathML expression"))?;
            let fragment = html_recovery::library_fragment(&returned)
                .ok_or_else(|| math_failure(site, source, "bounded MathML fragment"))?;
            let node = checked_math_fragment(&fragment, source, site)?;
            prepared.push(MathRun {
                block,
                run,
                source: source.clone(),
                node,
            });
        }
    }
    Ok(prepared)
}

fn need_faces(plan: &DocumentPlan) -> Vec<Face> {
    fn visit(runs: &[Inline], bold: bool, italic: bool, selected: &mut BTreeSet<&'static str>) {
        for run in runs {
            match run {
                Inline::Styled { kind, children, .. } => {
                    let (b, i) = match kind {
                        StyleKind::Emphasis => (bold, true),
                        StyleKind::Strong => (true, italic),
                        StyleKind::Smallcaps => (bold, italic),
                        StyleKind::Regular => (false, false),
                    };
                    selected.insert(match (b, i) {
                        (false, false) => REGULAR.file,
                        (false, true) => ITALIC.file,
                        (true, false) => BOLD.file,
                        (true, true) => BOLD_ITALIC.file,
                    });
                    visit(children, b, i, selected);
                }
                Inline::Mention { .. } => {
                    selected.insert(MONO.file);
                }
                Inline::Math { .. } => {
                    selected.insert(MATH.file);
                }
                _ => {}
            }
        }
    }
    let mut selected = BTreeSet::from([REGULAR.file, BOLD.file]);
    if plan.screen.as_ref().is_some_and(|screen| {
        plan.blocks.iter().enumerate().any(|(index, block)| {
            !block.paragraph
                && !block.equation
                && block.depth.is_none()
                && !screen.html_gaps.contains_key(&index)
        })
    }) {
        selected.insert(ITALIC.file);
    }
    visit(&plan.coverage.runs, false, false, &mut selected);
    for runs in &plan.coverage.audit {
        visit(runs, false, false, &mut selected);
    }
    for (index, block) in plan.blocks.iter().enumerate() {
        if plan
            .screen
            .as_ref()
            .is_some_and(|screen| screen.html_gaps.contains_key(&index))
        {
            continue;
        }
        visit(
            &block.runs,
            block.depth.is_some(),
            !block.paragraph && block.depth.is_none(),
            &mut selected,
        );
    }
    [REGULAR, ITALIC, BOLD, BOLD_ITALIC, MONO, MATH]
        .into_iter()
        .filter(|face| selected.contains(face.file))
        .collect()
}

pub(super) fn load_fonts(
    plan: &DocumentPlan,
    support: &NativeSupport,
) -> Result<Fonts, DocumentError> {
    let mut fonts = Vec::new();
    for face in need_faces(plan) {
        let path = support.directory().join("bundle").join(face.file);
        let fault = || {
            if face.is_math() {
                let site = plan
                    .blocks
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| {
                        !plan
                            .screen
                            .as_ref()
                            .is_some_and(|screen| screen.html_gaps.contains_key(index))
                    })
                    .flat_map(|(_, block)| &block.runs)
                    .find_map(|run| {
                        if let Inline::Math { site, .. } = run {
                            Some(site)
                        } else {
                            None
                        }
                    })
                    .expect("math face requires a math run");
                math_failure(site, face.file, "pinned embedded mathematics face")
            } else {
                font_failure(plan, face.file)
            }
        };
        let bytes = fs::read(path).map_err(|_| fault())?;
        let mut digest = String::new();
        for byte in Sha256::digest(&bytes) {
            write!(digest, "{byte:02x}").expect("string write");
        }
        if digest != face.digest {
            return Err(fault());
        }
        fonts.push((face, bytes));
    }
    Ok(Fonts(fonts))
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for (index, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            result.push(if index < chunk.len() + 1 {
                char::from(ALPHABET[((value >> shift) & 63) as usize])
            } else {
                '='
            });
        }
    }
    result
}

fn css(plan: &DocumentPlan, fonts: &Fonts) -> String {
    let mut result = String::new();
    for (face, bytes) in &fonts.0 {
        writeln!(
            result,
            "@font-face{{font-family:'{}';src:url(data:font/otf;base64,{});font-weight:{};font-style:{}}}",
            face.family,
            base64(bytes),
            face.weight,
            if face.italic { "italic" } else { "normal" }
        )
        .expect("string write");
    }
    result.push_str("body{font-family:'Pagella',serif;max-width:46rem;margin:auto;padding:0 1rem;line-height:1.5}h1{font-weight:700}a:focus-visible,summary:focus-visible{outline:3px solid #a34500;outline-offset:3px}.refrain{text-align:center;font-style:italic}.attribution{text-align:right;font-size:smaller}.gap{color:#444}");
    if plan.blocks.iter().any(|block| block.econ.is_some()) {
        result.push_str(
            ".econ{background:#f7f6f3;padding:1rem;margin:1rem 0}.econ footer{text-align:right}",
        );
    }
    result.push_str(
        ".licenses pre{white-space:pre-wrap;overflow-wrap:anywhere;font-family:'Pagella',serif}\n",
    );
    if fonts.0.iter().any(|(face, _)| face.file == MONO.file) {
        result.push_str(
            "code{font-family:'Inconsolata',monospace;font-weight:400;font-style:normal}\n",
        );
    }
    if fonts.0.iter().any(|(face, _)| face.is_math()) {
        result.push_str("math{font-family:'Pagella Math';font-weight:400;font-style:normal}math mtext{font-family:'Pagella',serif}\n");
    }
    if plan.blocks.iter().any(realizable_equation) {
        result.push_str(".equation{max-width:100%;min-width:0;overflow-x:auto}.equation:focus-visible{outline:3px solid #a34500;outline-offset:3px}\n");
    }
    result
}

fn licence_nodes(fonts: &Fonts) -> Vec<Node> {
    const GUST: &str = include_str!("../../../../licenses/fonts/GUST-FONT-LICENSE.txt");
    const LPPL: &str = include_str!("../../../../licenses/fonts/LPPL-1.3c.txt");
    const README: &str = include_str!("../../../../licenses/fonts/README-TeX-Gyre-Pagella.txt");
    const MANIFEST: &str = include_str!("../../../../licenses/fonts/MANIFEST-TeX-Gyre-Pagella.txt");
    const OFL: &str = include_str!("../../../../licenses/fonts/OFL-1.1.txt");
    const INCONSOLATA_OFL: &str = include_str!("../../../../licenses/fonts/OFL-Inconsolata.txt");
    const APACHE: &str = include_str!("../../../../licenses/fonts/Apache-2.0.txt");
    const INCONSOLATA_README: &str =
        include_str!("../../../../licenses/fonts/README-Inconsolata.txt");
    const MATH_README: &str = include_str!("../../../../licenses/fonts/README-TeX-Gyre-Math.txt");
    const PAGELLA_MATH_README: &str =
        include_str!("../../../../licenses/fonts/README-TeX-Gyre-Pagella-Math.txt");
    let mut files = vec![
        ("GUST-FONT-LICENSE.txt", GUST),
        ("LPPL-1.3c.txt", LPPL),
        ("README-TeX-Gyre-Pagella.txt", README),
        ("MANIFEST-TeX-Gyre-Pagella.txt", MANIFEST),
    ];
    if fonts.0.iter().any(|(face, _)| face.file == MONO.file) {
        files.extend([
            ("OFL-1.1.txt", OFL),
            ("OFL-Inconsolata.txt", INCONSOLATA_OFL),
            ("Apache-2.0.txt", APACHE),
            ("README-Inconsolata.txt", INCONSOLATA_README),
        ]);
    }
    if fonts.0.iter().any(|(face, _)| face.is_math()) {
        files.extend([
            ("README-TeX-Gyre-Math.txt", MATH_README),
            ("README-TeX-Gyre-Pagella-Math.txt", PAGELLA_MATH_README),
        ]);
    }
    files.extend(fonts.0.iter().map(|(face, _)| (face.file, face.notice)));
    files
        .into_iter()
        .map(|(file, content)| {
            element(
                "pre",
                &[("data-file", file)],
                vec![Node::Text(content.into())],
            )
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(super) enum Destination<'a> {
    Artifact(&'a DocumentPlan),
    Quotation(&'a super::plan::ScreenQuotation),
    Virtue {
        owner: &'a str,
        language: &'a str,
        fragment: &'a str,
    },
}

pub(super) fn destination(value: Destination<'_>) -> String {
    match value {
        Destination::Artifact(plan) => {
            format!(
                "generated/documents/{}/{}.html",
                plan.document, plan.language
            )
        }
        Destination::Quotation(quotation) => format!(
            "../{}/{}.html#{}",
            quotation.owner_document, quotation.language, quotation.fragment
        ),
        Destination::Virtue {
            owner,
            language,
            fragment,
        } => {
            format!("../{owner}/{language}.html#{fragment}")
        }
    }
}

fn inline(runs: &[Inline], prepared: &[MathRun], block: usize) -> Vec<Node> {
    let mut nodes = Vec::new();
    for (run_index, run) in runs.iter().enumerate() {
        nodes.push(match run {
            Inline::Text(text) => Node::Text(text.clone()),
            Inline::Tag { handle, .. } => Node::Text(handle.clone()),
            Inline::PaperTag { value, .. } => Node::Text(value.spelling.clone()),
            Inline::Citation { identity, .. } => Node::Text(identity.handle.clone()),
            Inline::Mention { text, .. } => element("code", &[], vec![Node::Text(text.clone())]),
            Inline::Styled { kind, children, .. } => {
                let name = match kind {
                    StyleKind::Emphasis => "em",
                    StyleKind::Strong => "strong",
                    StyleKind::Smallcaps | StyleKind::Regular => "span",
                };
                let attrs = match kind {
                    StyleKind::Smallcaps => vec![("style", "font-variant:small-caps")],
                    StyleKind::Regular => {
                        vec![("style", "font-weight:normal;font-style:normal")]
                    }
                    _ => vec![],
                };
                element(name, &attrs, inline(children, &[], block))
            }
            Inline::Math { .. } => prepared
                .iter()
                .find(|item| item.block == block && item.run == run_index)
                .expect("prepared mathematics run")
                .node
                .clone(),
            Inline::BibliographyCitation { kind, entries, .. } => {
                let parenthetical = *kind == ftoo_manifest::BibliographyCitationKind::Parenthetical;
                let mut children = Vec::new();
                if parenthetical {
                    children.push(Node::Text("(".into()));
                }
                for (index, entry) in entries.iter().enumerate() {
                    if index != 0 {
                        children.push(Node::Text("; ".into()));
                    }
                    let href = format!("#{}", entry.anchor);
                    children.push(element(
                        "a",
                        &[("href", &href)],
                        vec![Node::Text(super::bibliography::screen_citation_piece(
                            *kind, entry,
                        ))],
                    ));
                }
                if parenthetical {
                    children.push(Node::Text(")".into()));
                }
                element(
                    "span",
                    &[(
                        "data-kind",
                        if parenthetical {
                            "parenthetical"
                        } else {
                            "textual"
                        },
                    )],
                    children,
                )
            }
            Inline::See { .. } => unreachable!("unrealizable block withheld before emission"),
            Inline::Virtue {
                word,
                owner,
                language,
                fragment,
                ..
            } => {
                let href = destination(Destination::Virtue {
                    owner,
                    language,
                    fragment,
                });
                element(
                    "a",
                    &[("href", &href)],
                    vec![element(
                        "strong",
                        &[],
                        vec![element("em", &[], vec![Node::Text(word.clone())])],
                    )],
                )
            }
        });
    }
    nodes
}

pub(super) fn realizable(runs: &[Inline], math_allowed: bool) -> bool {
    unrealizable_kinds(runs, math_allowed).is_empty()
}

pub(super) fn realizable_equation(block: &Block) -> bool {
    block.unit == "eq:explainer:overlap-flow"
        && block
            .equation_pair()
            .is_some_and(|(source, _)| html_recovery::admitted_flow_source(source))
}

pub(super) fn unrealizable_kinds(runs: &[Inline], math_allowed: bool) -> BTreeSet<&'static str> {
    let mut kinds = BTreeSet::new();
    for run in runs {
        match run {
            Inline::Styled { children, .. } => kinds.extend(unrealizable_kinds(children, false)),
            Inline::Math {
                markdown, latex, ..
            } if !math_allowed || !admitted_math(markdown, latex) => {
                kinds.insert("mathematics");
            }
            Inline::See { .. } => {
                // HTML needs a checked reference link emitter and recovery for its destination.
                kinds.insert("see-reference");
            }
            Inline::BibliographyCitation { .. }
            | Inline::Math { .. }
            | Inline::Text(_)
            | Inline::Tag { .. }
            | Inline::PaperTag { .. }
            | Inline::Citation { .. }
            | Inline::Virtue { .. }
            | Inline::Mention { .. } => {}
        }
    }
    kinds
}

fn bibliography_end_matter(plan: &DocumentPlan) -> Result<(), DocumentError> {
    fn first_bibliography_citation(runs: &[Inline]) -> Option<&UnitSite> {
        runs.iter().find_map(|run| match run {
            Inline::BibliographyCitation { site, .. } => Some(site),
            Inline::Styled { children, .. } => first_bibliography_citation(children),
            _ => None,
        })
    }
    let screen = plan.screen.as_ref().expect("screen checked above");
    let cited = plan
        .blocks
        .iter()
        .find_map(|block| first_bibliography_citation(&block.runs));
    if !screen
        .end_matter
        .iter()
        .any(|section| matches!(section, super::plan::ScreenEndMatter::References(_)))
        && let Some(site) = cited
    {
        return Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(site.clone()),
            AssemblyFault::UnknownToken {
                value: plan.document.clone(),
                expected: vec!["HTML References section for bibliography citations".into()],
            },
        ));
    }
    let derived = super::bibliography::entries(plan);
    let references = screen
        .end_matter
        .iter()
        .filter_map(|section| match section {
            super::plan::ScreenEndMatter::Unavailable(_) => None,
            super::plan::ScreenEndMatter::References(entries) => Some(entries),
        })
        .collect::<Vec<_>>();
    let index_match = match screen.end_matter.first() {
        Some(super::plan::ScreenEndMatter::Unavailable(entries)) => {
            !screen.index.is_empty() && *entries == screen.index
        }
        _ => screen.index.is_empty(),
    };
    let entries_match = (match references.as_slice() {
        [] => derived.is_empty(),
        [entries] => **entries == derived,
        _ => false,
    }) && index_match
        && screen.end_matter.len() == references.len() + usize::from(!screen.index.is_empty());
    if !entries_match {
        return Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(cited.unwrap_or(&plan.coverage.site).clone()),
            AssemblyFault::UnknownToken {
                value: plan.document.clone(),
                expected: vec![
                    "HTML bibliography end-matter entries derived from citations".into(),
                ],
            },
        ));
    }
    Ok(())
}

pub(super) fn prose_only(plan: &DocumentPlan) -> Result<(), DocumentError> {
    fn first_math(runs: &[Inline]) -> Option<&UnitSite> {
        runs.iter().find_map(|run| match run {
            Inline::Math { site, .. } => Some(site),
            Inline::Styled { children, .. } => first_math(children),
            _ => None,
        })
    }
    fn first_unrealizable(runs: &[Inline]) -> Option<&UnitSite> {
        runs.iter().find_map(|run| match run {
            Inline::Styled { children, .. } => first_unrealizable(children),
            Inline::Math { site, .. }
            | Inline::BibliographyCitation { site, .. }
            | Inline::See { site, .. }
            | Inline::Virtue { site, .. } => Some(site),
            _ => None,
        })
    }
    fn first_heading_fault(runs: &[Inline]) -> Option<&UnitSite> {
        runs.iter().find_map(|run| match run {
            Inline::Tag { site, .. }
            | Inline::PaperTag { site, .. }
            | Inline::Citation { site, .. }
            | Inline::BibliographyCitation { site, .. }
            | Inline::Math { site, .. }
            | Inline::See { site, .. }
            | Inline::Virtue { site, .. } => Some(site),
            Inline::Styled { children, .. } => first_heading_fault(children),
            _ => None,
        })
    }
    let refused = plan
        .screen
        .is_none()
        .then_some(&plan.coverage.site)
        .or_else(|| first_unrealizable(&plan.coverage.runs))
        .or_else(|| (!realizable(&plan.coverage.runs, false)).then_some(&plan.coverage.site))
        .or_else(|| {
            plan.coverage
                .audit
                .iter()
                .find_map(|runs| first_unrealizable(runs))
        })
        .or_else(|| {
            plan.coverage
                .audit
                .iter()
                .any(|runs| !realizable(runs, false))
                .then_some(&plan.coverage.site)
        })
        .or_else(|| {
            plan.blocks.first().and_then(|block| {
                first_heading_fault(&block.runs)
                    .or_else(|| (!realizable(&block.runs, false)).then_some(&block.site))
            })
        })
        .or_else(|| {
            plan.blocks.iter().find_map(|block| {
                if !block.displays.is_empty() {
                    return first_math(&block.runs).or(Some(&block.site));
                }
                None
            })
        })
        .or_else(|| {
            plan.blocks
                .iter()
                .enumerate()
                .skip(1)
                .find_map(|(index, block)| {
                    (plan
                        .screen
                        .as_ref()
                        .is_some_and(|screen| screen.html_gaps.contains_key(&index))
                        == if block.equation {
                            realizable_equation(block)
                        } else {
                            realizable(&block.runs, block.paragraph)
                        })
                    .then_some(&block.site)
                })
        });
    if let Some(site) = refused {
        return Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(site.clone()),
            AssemblyFault::UnknownToken {
                value: plan.document.clone(),
                expected: vec!["checked HTML block realization and notice inventory".into()],
            },
        ));
    }
    bibliography_end_matter(plan)
}

pub(super) fn plain(runs: &[Inline]) -> String {
    let mut result = String::new();
    for run in runs {
        match run {
            Inline::Text(value) | Inline::Mention { text: value, .. } => result.push_str(value),
            Inline::Tag { handle, .. } => result.push_str(handle),
            Inline::PaperTag { value, .. } => result.push_str(&value.spelling),
            Inline::Citation { identity, .. } => result.push_str(&identity.handle),
            Inline::Styled { children, .. } => result.push_str(&plain(children)),
            Inline::Math { .. }
            | Inline::BibliographyCitation { .. }
            | Inline::See { .. }
            | Inline::Virtue { .. } => {
                unreachable!("unrealizable heading refused before title")
            }
        }
    }
    result
}

fn labels(language: &str) -> Option<(&'static str, &'static str)> {
    (language == "en").then_some(("Contents", "Font licences"))
}

pub(super) fn check_destinations(
    plan: &DocumentPlan,
    plans: &[DocumentPlan],
    selected: &BTreeMap<String, Vec<DocumentOutput>>,
) -> Result<(), DocumentError> {
    if let Some((refrain, quotation)) = plan.screen.iter().find_map(|screen| {
        plan.blocks
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(index, _)| {
                if screen.html_gaps.contains_key(&index) {
                    return None;
                }
                let refrain = super::plan::refrain_index(&plan.blocks, index)
                    .and_then(|position| screen.refrains.get(position))?;
                refrain.quotation.as_ref().and_then(|quotation| {
                    (!selected
                        .get(&quotation.owner_document)
                        .is_some_and(|targets| targets.contains(&DocumentOutput::Html))
                        || !plans.iter().any(|owner| {
                            owner.document == quotation.owner_document
                                && owner.language == quotation.language
                                && owner.screen.as_ref().is_some_and(|screen| {
                                    owner
                                        .blocks
                                        .iter()
                                        .enumerate()
                                        .skip(1)
                                        .any(|(index, block)| {
                                            super::plan::refrain_index(&owner.blocks, index)
                                                .and_then(|refrain_index| {
                                                    screen.refrains.get(refrain_index)
                                                })
                                                .is_some_and(|candidate| {
                                                    candidate.quotation.is_none()
                                                        && candidate.fragment == quotation.fragment
                                                        && !screen.html_gaps.contains_key(&index)
                                                        && realizable(&block.runs, block.paragraph)
                                                })
                                        })
                                })
                        }))
                    .then_some((refrain, quotation))
                })
            })
    }) {
        return Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(refrain.site.clone()),
            AssemblyFault::UnknownToken {
                value: quotation.owner_document.clone(),
                expected: vec!["HTML owner destination".into()],
            },
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Each document section is assembled in its visible order.
pub(super) fn model(
    plan: &DocumentPlan,
    fonts: &Fonts,
    prepared: &[MathRun],
) -> Result<Node, DocumentError> {
    prose_only(plan)?;
    let screen = plan.screen.as_ref().expect("checked screen");
    let heading = &plan.blocks[0];
    let (contents_label, licences_label) = labels(&plan.language).ok_or_else(|| {
        failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(plan.coverage.site.clone()),
            AssemblyFault::UnknownToken {
                value: plan.language.clone(),
                expected: vec!["HTML screen labels".into()],
            },
        )
    })?;
    let stylesheet = css(plan, fonts);
    if stylesheet.contains('<') || stylesheet.contains('&') {
        return Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(plan.coverage.site.clone()),
            AssemblyFault::UnknownToken {
                value: "HTML stylesheet raw text".into(),
                expected: vec!["CSS without raw HTML delimiters".into()],
            },
        ));
    }
    let mut coverage = vec![element(
        "p",
        &[],
        vec![Node::Text(plan.coverage.declaration.clone())],
    )];
    coverage.push(element("p", &[], inline(&plan.coverage.runs, &[], 0)));
    for runs in &plan.coverage.audit {
        coverage.push(element("p", &[], inline(runs, &[], 0)));
    }
    let heading_nodes = inline(&heading.runs, &[], 0);
    let title = plain(&heading.runs);
    let mut body = vec![element("section", &[("id", "coverage")], coverage)];
    body.push(element(
        "nav",
        &[("aria-label", contents_label)],
        vec![element(
            "a",
            &[("href", "#content")],
            vec![Node::Text(title.clone())],
        )],
    ));
    let mut main = vec![element("h1", &[("id", "content")], heading_nodes)];
    for (index, block) in plan.blocks.iter().enumerate().skip(1) {
        if !screen.gaps[index - 1].message.is_empty() {
            main.push(element(
                "p",
                &[("class", "gap")],
                vec![Node::Text(screen.gaps[index - 1].message.clone())],
            ));
        }
        if let Some(gap) = screen.html_gaps.get(&index) {
            main.push(element(
                "p",
                &[("class", "gap")],
                vec![Node::Text(gap.message.clone())],
            ));
            continue;
        }
        if block.depth == Some(2) {
            let fragment = super::plan::public_fragment(plan, &block.site, &block.unit)?;
            main.push(element(
                "h2",
                &[("id", &fragment)],
                inline(&block.runs, prepared, index),
            ));
            continue;
        }
        if let Some(econ) = &block.econ {
            let header_id = format!("{}-header", econ.fragment);
            main.push(element(
                "section",
                &[
                    ("class", "econ"),
                    ("id", &econ.fragment),
                    ("aria-labelledby", &header_id),
                ],
                vec![
                    element(
                        "header",
                        &[],
                        vec![element(
                            "p",
                            &[("id", &header_id)],
                            vec![
                                element(
                                    "strong",
                                    &[],
                                    vec![Node::Text(format!(
                                        "Economics {}. {}",
                                        econ.number, econ.title
                                    ))],
                                ),
                                Node::Text(format!(" (on {})", econ.reference)),
                            ],
                        )],
                    ),
                    element("p", &[], inline(&block.runs, prepared, index)),
                    element(
                        "footer",
                        &[],
                        vec![element(
                            "p",
                            &[],
                            vec![Node::Text(format!(
                                "{} · d:{}",
                                econ.target, econ.departure
                            ))],
                        )],
                    ),
                ],
            ));
            continue;
        }
        if block.paragraph {
            main.push(element("p", &[], inline(&block.runs, prepared, index)));
            continue;
        }
        if block.equation {
            let math = prepared
                .iter()
                .find(|item| item.block == index && item.run == 0)
                .expect("prepared admitted equation");
            main.push(element(
                "div",
                &[("class", "equation"), ("tabindex", "0")],
                vec![math.node.clone()],
            ));
            continue;
        }
        let refrain =
            &screen.refrains[super::plan::refrain_index(&plan.blocks, index).expect("refrain")];
        let attrs = if refrain.quotation.is_none() {
            vec![("class", "refrain"), ("id", refrain.fragment.as_str())]
        } else {
            vec![("class", "refrain")]
        };
        main.push(element("p", &attrs, inline(&block.runs, &[], index)));
        let handle = &refrain.handle;
        let attribution = if let Some(quotation) = &refrain.quotation {
            let href = destination(Destination::Quotation(quotation));
            element("a", &[("href", &href)], vec![Node::Text(handle.clone())])
        } else {
            Node::Text(handle.clone())
        };
        main.push(element("p", &[("class", "attribution")], vec![attribution]));
    }
    main.push(element(
        "p",
        &[("class", "gap")],
        vec![Node::Text(screen.gaps.last().expect("gap").message.clone())],
    ));
    for section in &screen.end_matter {
        match section {
            super::plan::ScreenEndMatter::Unavailable(entries) => {
                main.push(element(
                    "h2",
                    &[("id", "references-to-unavailable-content")],
                    vec![Node::Text("References to unavailable content".into())],
                ));
                for entry in entries {
                    main.push(element(
                        "p",
                        &[],
                        vec![
                            element("a", &[("id", &entry.fragment)], vec![]),
                            Node::Text(if entry.locator.is_empty() {
                                format!("{}. {}", entry.title, entry.message)
                            } else {
                                format!("{} — {}. {}", entry.locator, entry.title, entry.message)
                            }),
                        ],
                    ));
                }
            }
            super::plan::ScreenEndMatter::References(entries) => {
                main.push(element(
                    "h2",
                    &[("id", "references")],
                    vec![Node::Text("References".into())],
                ));
                for entry in entries {
                    main.push(element(
                        "p",
                        &[],
                        vec![
                            element("a", &[("id", &entry.anchor)], vec![]),
                            Node::Text(super::bibliography::screen_text(entry)),
                        ],
                    ));
                }
            }
        }
    }
    body.push(element("main", &[], main));
    let mut licences = vec![element(
        "summary",
        &[],
        vec![Node::Text(licences_label.into())],
    )];
    licences.extend(licence_nodes(fonts));
    body.push(element("details", &[("class", "licenses")], licences));
    Ok(element(
        "html",
        &[("lang", &plan.language)],
        vec![
            element(
                "head",
                &[],
                vec![
                    element("meta", &[("charset", "utf-8")], vec![]),
                    element(
                        "meta",
                        &[
                            ("name", "viewport"),
                            ("content", "width=device-width, initial-scale=1"),
                        ],
                        vec![],
                    ),
                    element("title", &[], vec![Node::Text(title)]),
                    element("style", &[], vec![Node::Text(stylesheet)]),
                ],
            ),
            element("body", &[], body),
        ],
    ))
}

fn escape(source: &str, output: &mut String) {
    for character in source.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(character),
        }
    }
}

pub(super) fn serialize(node: &Node, output: &mut String) {
    match node {
        Node::Text(value) => escape(value, output),
        Node::Element {
            name,
            attributes,
            children,
        } => {
            write!(output, "<{name}").expect("string write");
            for (key, value) in attributes {
                write!(output, " {key}=\"").expect("string write");
                escape(value, output);
                output.push('"');
            }
            output.push('>');
            if name != "meta" {
                for child in children {
                    if name == "style" {
                        if let Node::Text(value) = child {
                            output.push_str(value);
                        }
                    } else {
                        serialize(child, output);
                    }
                }
                write!(output, "</{name}>").expect("string write");
            }
        }
    }
}

pub(super) fn render(
    plan: &DocumentPlan,
    support: &NativeSupport,
    known_labels: &[&str],
    notation: Option<&RenderableNotation<'_, '_, '_>>,
) -> Result<String, DocumentError> {
    prose_only(plan)?;
    let prepared = prepare_math(plan, notation)?;
    let fonts = load_fonts(plan, support)?;
    let tree = model(plan, &fonts, &prepared)?;
    let mut output = String::from("<!doctype html>");
    serialize(&tree, &mut output);
    if output.len() > MAX_HTML_BYTES {
        return Err(font_failure(plan, "HTML document byte limit"));
    }
    super::html_recovery::check(plan, &fonts, &prepared, known_labels, &output)?;
    Ok(output)
}
