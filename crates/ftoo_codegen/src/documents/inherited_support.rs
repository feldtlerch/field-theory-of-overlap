//! Native support from the source declarations admitted under M078.
use ftoo_manifest::{
    CheckedNotation, INHERITED_SOURCE_DECLARATIONS, INHERITED_SOURCE_FILES,
    INHERITED_VARIANCE_DECLARATION,
};

const TYPED_TOOLS: &str = include_str!("../../../../documents/paper/ftoo_typed_tools.tex");
const MACROS_OVERLAP: &str = include_str!("../../../../documents/paper/macros_overlap.tex");
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Refusal {
    ChangedSource(&'static str),
    MissingPrecondition(&'static str),
}

/// A checked catalogue has already admitted these exact source declarations under M078.
pub(super) fn generate(
    _notation: &CheckedNotation<'_, '_>,
    sources: Sources<'_>,
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
    let mut output = String::from("\\ExplSyntaxOn\n");
    output.push_str(PREDECESSORS);
    output.push_str("\\input{ftoo_typed_tools.tex}\n");
    for (_, declaration) in INHERITED_SOURCE_DECLARATIONS {
        output.push_str(declaration);
    }
    output.push_str(INHERITED_VARIANCE_DECLARATION);
    output.push_str("\\ExplSyntaxOff\n");
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources() -> Sources<'static> {
        Sources {
            symbols: INHERITED_SOURCE_FILES[0].1,
            historical: INHERITED_SOURCE_FILES[1].1,
            registers: INHERITED_SOURCE_FILES[2].1,
            typed_tools: TYPED_TOOLS,
            macros_overlap: MACROS_OVERLAP,
        }
    }

    #[test]
    fn compared_closure_has_exact_native_module() {
        let graph = ftoo_manifest::seed().unwrap();
        let book = ftoo_manifest::addressing(&graph).unwrap();
        let notation = ftoo_manifest::notation(&book).unwrap();
        let output = generate(&notation, sources()).unwrap();
        assert_eq!(
            output,
            concat!(
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
            )
        );
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
                }
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
                }
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
                }
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
                }
            ),
            Err(Refusal::ChangedSource("ftoo_typed_tools.tex"))
        );
    }
}
