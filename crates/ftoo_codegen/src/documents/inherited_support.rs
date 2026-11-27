//! Native support from the source declarations admitted under M078.
use ftoo_manifest::{
    CheckedNotation, INHERITED_SOURCE_DECLARATIONS, INHERITED_SOURCE_FILES,
    INHERITED_VARIANCE_DECLARATION, inherited_decoded_declarations,
};

const TYPED_TOOLS: &str = include_str!("../../../../documents/paper/ftoo_typed_tools.tex");
const MACROS_OVERLAP: &str = include_str!("../../../../documents/paper/macros_overlap.tex");
const COMPAT: &str = include_str!("../../../../documents/paper-build/compat.tex");
const LAYOUT: &str = include_str!("../../../../documents/paper-build/reader-hooks.tex");
#[must_use]
pub(super) fn compared_macros() -> &'static str {
    MACROS_OVERLAP
}
const PREDECESSORS: &str = "\\NewDocumentCommand{\\grade}{m}\n  { {\\normalfont\\footnotesize\\textsc{#1}}\\quad }\n\\NewCommandCopy{\\secgrade}{\\grade}\n\\NewDocumentCommand{\\gradeword}{m}{ \\textsc{#1} }\n";
pub(super) const PATH: &str = "generated/native-paper-support.tex";
pub(super) const TYPED_PATH: &str = "documents/paper/ftoo_typed_tools.tex";
pub(super) const MACROS_PATH: &str = "documents/paper/macros_overlap.tex";
pub(super) const COMPARED_PATHS: [&str; 3] = [
    "documents/paper/data/ftoo_symbols.tex",
    "documents/paper/data/ftoo_symbols_historical.tex",
    "documents/paper/data/ftoo_registers.tex",
];

#[derive(Clone, Copy)]
pub(super) struct Sources<'a> {
    pub symbols: &'a str,
    pub historical: &'a str,
    pub registers: &'a str,
    pub typed_tools: &'a str,
    pub macros_overlap: &'a str,
    pub manifest: &'a str,
    pub compat: &'a str,
    pub layout: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Refusal {
    ChangedSource(&'static str),
    MissingPrecondition(&'static str),
    InvalidResultHeader(String),
}

fn source_line<'a>(source: &'a str, site: &ftoo_manifest::PaperEvidenceSite) -> Option<&'a str> {
    source.lines().nth(site.line.checked_sub(1)?)
}

fn result_support(
    macros: &str,
    manifest: &str,
    compat: &str,
    headers: &[&super::registry::ResultHeaderBinding],
) -> Result<String, Refusal> {
    use std::fmt::Write as _;
    let start = macros
        .find("\\NewDocumentCommand{\\ftoolabel}{m}")
        .ok_or(Refusal::MissingPrecondition("result header closure"))?;
    let end = macros
        .find("\\numberwithin{equation}{section}")
        .ok_or(Refusal::MissingPrecondition("result header closure"))?;
    let variant = compat
        .lines()
        .find(|line| {
            line.trim_start()
                .starts_with("\\cs_if_exist:NF \\tl_set:Ne")
        })
        .ok_or(Refusal::MissingPrecondition("result compatibility variant"))?;
    let mut output = format!("{variant}\n");
    output.push_str(&macros[start..end]);
    for prefix in [
        "\\cs_new_protected:Npn \\ftoo_tagfont:n ",
        "\\NewDocumentCommand{\\ftootagsep}",
    ] {
        let line = macros
            .lines()
            .find(|line| line.starts_with(prefix))
            .ok_or(Refusal::MissingPrecondition("result tag presentation"))?;
        writeln!(output, "{line}").expect("string write");
    }
    for row in headers {
        let lines = [
            (
                &row.tag_site,
                format!(
                    "\\prop_gput:Nnn \\g_ftoo_tag_to_id_prop {{ {} }} {{ {} }}",
                    row_key(row),
                    row.target
                ),
            ),
            (
                &row.reverse_site,
                format!(
                    "\\prop_gput:Nnn \\g_ftoo_id_to_tag_prop {{ {} }} {{ {} }}",
                    row.target,
                    row_key(row)
                ),
            ),
        ];
        for (site, expected) in lines {
            let line = source_line(manifest, site)
                .ok_or_else(|| Refusal::InvalidResultHeader(row.unit.clone()))?;
            if line != expected {
                return Err(Refusal::InvalidResultHeader(row.unit.clone()));
            }
            writeln!(output, "{line}").expect("string write");
        }
        let heading = source_line(manifest, &row.heading_site)
            .ok_or_else(|| Refusal::InvalidResultHeader(row.unit.clone()))?;
        let prefix = format!(
            "\\prop_gput:Nnn \\g_ftoo_id_to_heading_prop {{ {} }} {{ ",
            row.target
        );
        let value = heading
            .strip_prefix(&prefix)
            .and_then(|value| value.strip_suffix(" }"))
            .ok_or_else(|| Refusal::InvalidResultHeader(row.unit.clone()))?;
        if value.replace('~', " ") != row.title {
            return Err(Refusal::InvalidResultHeader(row.unit.clone()));
        }
        writeln!(output, "{heading}").expect("string write");
    }
    Ok(output)
}

fn row_key(row: &super::registry::ResultHeaderBinding) -> &str {
    row.key.as_str()
}

fn result_compat_preamble(source: &str) -> Result<&str, Refusal> {
    let start = source
        .find("\\makeatletter\n\\ExplSyntaxOn\n\\AddToHook{package/zref-clever/after}{")
        .ok_or(Refusal::MissingPrecondition(
            "result reference type compatibility",
        ))?;
    let tail = &source[start..];
    let closing = "\\ExplSyntaxOff\n\\makeatother\n";
    let end = tail.find(closing).ok_or(Refusal::MissingPrecondition(
        "result reference type compatibility",
    ))? + closing.len();
    Ok(&tail[..end])
}

/// A checked catalogue has already admitted these exact source declarations under M078.
pub(super) fn generate(
    _notation: &CheckedNotation<'_, '_>,
    sources: Sources<'_>,
    headers: &[&super::registry::ResultHeaderBinding],
) -> Result<String, Refusal> {
    if !sources.macros_overlap.contains(PREDECESSORS) {
        return Err(Refusal::MissingPrecondition("grade definitions"));
    }
    for ((path, expected), actual) in INHERITED_SOURCE_FILES.into_iter().zip([
        sources.symbols,
        sources.historical,
        sources.registers,
    ]) {
        if actual != expected {
            return Err(Refusal::ChangedSource(path));
        }
    }
    for (path, actual, expected) in [
        ("ftoo_typed_tools.tex", sources.typed_tools, TYPED_TOOLS),
        ("macros_overlap.tex", sources.macros_overlap, MACROS_OVERLAP),
    ] {
        if actual != expected {
            return Err(Refusal::ChangedSource(path));
        }
    }
    let mut output = String::new();
    if !headers.is_empty() {
        if sources.compat != COMPAT {
            return Err(Refusal::ChangedSource("compat.tex"));
        }
        if sources.layout != LAYOUT {
            return Err(Refusal::ChangedSource("reader-hooks.tex"));
        }
        output.push_str(result_compat_preamble(sources.compat)?);
        output.push_str("\\RequirePackage{amsthm}\n\\RequirePackage{hyperref}\n\\RequirePackage{zref-clever}\n\\RequirePackage{zref-titleref}\n");
    }
    output.push_str("\\ExplSyntaxOn\n");
    output.push_str(PREDECESSORS);
    output.push_str("\\input{ftoo_typed_tools.tex}\n");
    for (_, declaration) in INHERITED_SOURCE_DECLARATIONS {
        output.push_str(declaration);
    }
    for (_, declaration) in inherited_decoded_declarations() {
        output.push_str(&declaration);
    }
    output.push_str(INHERITED_VARIANCE_DECLARATION);
    if !headers.is_empty() {
        output.push_str(&result_support(
            sources.macros_overlap,
            sources.manifest,
            sources.compat,
            headers,
        )?);
        let grade_gap = sources
            .layout
            .lines()
            .find(|line| line.starts_with("\\NewDocumentCommand\\PaperGradeGap{m}{"))
            .ok_or(Refusal::MissingPrecondition("result grade gap"))?;
        output.push_str(grade_gap);
        output.push('\n');
    }
    output.push_str("\\ExplSyntaxOff\n");
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_header_support_uses_the_compared_source_lines() {
        let manifest = include_str!("../../../../documents/paper/data/ftoo_manifest.tex");
        let target = "sem:claim:witness-pooling-principle";
        let key = "f:pool";
        let located = |line: &str| ftoo_manifest::PaperEvidenceSite {
            file: "documents/paper/data/ftoo_manifest.tex".into(),
            line: manifest
                .lines()
                .position(|candidate| candidate == line)
                .unwrap()
                + 1,
        };
        let tag = format!("\\prop_gput:Nnn \\g_ftoo_tag_to_id_prop {{ {key} }} {{ {target} }}");
        let reverse = format!("\\prop_gput:Nnn \\g_ftoo_id_to_tag_prop {{ {target} }} {{ {key} }}");
        let heading = format!(
            "\\prop_gput:Nnn \\g_ftoo_id_to_heading_prop {{ {target} }} {{ Coherence~pools,~independence~weighs }}"
        );
        let row = super::super::registry::ResultHeaderBinding {
            key: key.into(),
            unit: "result:f:pool".into(),
            target: target.into(),
            title: "Coherence pools, independence weighs".into(),
            tag_site: located(&tag),
            reverse_site: located(&reverse),
            heading_site: located(&heading),
            unit_site: ftoo_manifest::PaperEvidenceSite {
                file: "units/corpus/result-pool.toml".into(),
                line: 1,
            },
        };
        let module = result_support(MACROS_OVERLAP, manifest, COMPAT, &[&row]).unwrap();
        assert!(module.contains("\\newtheorem{insight}[theorem]{Insight}"));
        assert!(module.contains(&tag));
        assert!(module.contains(&reverse));
        assert!(module.contains(&heading));
        let changed = manifest.replacen(&heading, "\\prop_gput:Nnn \\g_ftoo_id_to_heading_prop { sem:claim:witness-pooling-principle } { Changed }", 1);
        assert_eq!(
            result_support(MACROS_OVERLAP, &changed, COMPAT, &[&row]),
            Err(Refusal::InvalidResultHeader(row.unit))
        );
    }

    fn sources() -> Sources<'static> {
        Sources {
            symbols: INHERITED_SOURCE_FILES[0].1,
            historical: INHERITED_SOURCE_FILES[1].1,
            registers: INHERITED_SOURCE_FILES[2].1,
            typed_tools: TYPED_TOOLS,
            macros_overlap: MACROS_OVERLAP,
            manifest: "",
            compat: "",
            layout: "",
        }
    }

    #[test]
    fn compared_closure_has_exact_native_module() {
        let graph = ftoo_manifest::seed().unwrap();
        let book = ftoo_manifest::addressing(&graph).unwrap();
        let notation = ftoo_manifest::notation(&book).unwrap();
        let output = generate(&notation, sources(), &[]).unwrap();
        let mut expected = String::from(concat!(
            "\\ExplSyntaxOn\n",
            "\\NewDocumentCommand{\\grade}{m}\n  { {\\normalfont\\footnotesize\\textsc{#1}}\\quad }\n",
            "\\NewCommandCopy{\\secgrade}{\\grade}\n",
            "\\NewDocumentCommand{\\gradeword}{m}{ \\textsc{#1} }\n",
            "\\input{ftoo_typed_tools.tex}\n",
            "\\NewDocumentCommand\n  { \\comovfield }\n  {  }\n  {\n    \\ensuremath{S}\n  }\n",
            "\\NewDocumentCommand\n  { \\readout }\n  {  }\n  {\n    \\ensuremath{\\symcal{M}}\n  }\n",
            "\\NewDocumentCommand\n  { \\willvirtue }\n  {  }\n  {\n    \\ensuremath{\\omega\\sp{\\mathrm{vir}}}\n  }\n",
            "\\DeclareQuantity{\\gensym}{0}{ dim={\\Time[-1]}, variance=operator, spends={} }\n  { A\\sb{\\mathrm S} }\n",
            "\\NewDocumentCommand\n  { \\alignrate }\n  {  }\n  {\n    \\ensuremath{\\dot{\\sigma}}\n  }\n",
            "\\NewDocumentCommand\n  { \\willnet }\n  {  }\n  {\n    \\ensuremath{\\omega\\sb{\\mathrm{net}}}\n  }\n",
            "\\NewDocumentCommand\n  { \\willdecay }\n  {  }\n  {\n    \\ensuremath{\\gamma\\sigma}\n  }\n",
            "\\RegisterVariance{\\comovfield}{operator}\n",
            "\\ExplSyntaxOff\n"
        ));
        let decoded = inherited_decoded_declarations();
        let ending = "\\RegisterVariance{\\comovfield}{operator}\n\\ExplSyntaxOff\n";
        let position = expected.find(ending).expect("fixed ending");
        let additional = decoded
            .iter()
            .map(|(_, source)| source.as_str())
            .collect::<String>();
        expected.insert_str(position, &additional);
        assert_eq!(output, expected);
    }

    #[test]
    fn changed_inputs_and_missing_grade_precondition_refuse() {
        let graph = ftoo_manifest::seed().unwrap();
        let book = ftoo_manifest::addressing(&graph).unwrap();
        let notation = ftoo_manifest::notation(&book).unwrap();
        let changed = INHERITED_SOURCE_FILES[0]
            .1
            .replacen("\\ensuremath{S}", "\\ensuremath{X}", 1);
        assert_eq!(
            generate(
                &notation,
                Sources {
                    symbols: &changed,
                    ..sources()
                },
                &[]
            ),
            Err(Refusal::ChangedSource("ftoo_symbols.tex"))
        );
        let duplicate = format!(
            "{}\\NewDocumentCommand{{\\comovfield}}{{}}{{X}}\n",
            INHERITED_SOURCE_FILES[0].1
        );
        assert_eq!(
            generate(
                &notation,
                Sources {
                    symbols: &duplicate,
                    ..sources()
                },
                &[]
            ),
            Err(Refusal::ChangedSource("ftoo_symbols.tex"))
        );
        let missing = MACROS_OVERLAP.replacen(PREDECESSORS, "", 1);
        assert_ne!(missing, MACROS_OVERLAP);
        assert_eq!(
            generate(
                &notation,
                Sources {
                    macros_overlap: &missing,
                    ..sources()
                },
                &[]
            ),
            Err(Refusal::MissingPrecondition("grade definitions"))
        );
        let changed_tools = TYPED_TOOLS.replacen(
            "\\RenewDocumentCommand{\\grade}",
            "\\RenewDocumentCommand{\\grading}",
            1,
        );
        assert_ne!(changed_tools, TYPED_TOOLS);
        assert_eq!(
            generate(
                &notation,
                Sources {
                    typed_tools: &changed_tools,
                    ..sources()
                },
                &[]
            ),
            Err(Refusal::ChangedSource("ftoo_typed_tools.tex"))
        );
    }
}
