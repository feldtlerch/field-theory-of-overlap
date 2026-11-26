//! Checked coarse corpus slots and their document-specific emission.

use super::{
    diagnostics::{AssemblyFault, DocumentError, DocumentIoOperation},
    failure, input,
};
use crate::UnitSource;
use ftoo_manifest::{
    AssemblySite, AssemblyStage, LanguageCheckedUnits, RuleId,
    corpus::{CheckedSlot, admit_slot},
};
use std::{collections::BTreeMap, path::Path};

pub(super) fn load(
    root: &Path,
    units: &LanguageCheckedUnits,
) -> Result<(BTreeMap<String, CheckedSlot>, Vec<UnitSource>), DocumentError> {
    let directory = "units/corpus";
    if !root.join(directory).try_exists().map_err(|error| {
        input::io_error(DocumentIoOperation::Inspect, &root.join(directory), &error)
    })? {
        return Ok((BTreeMap::new(), Vec::new()));
    }
    let paths = input::discover(root, directory, false)?;
    if paths.len() != 1 || paths[0] != "units/corpus/explainer-plain.toml" {
        return Err(failure(
            RuleId::M055,
            AssemblyStage::Source,
            AssemblySite::Source(input::site(directory)),
            AssemblyFault::UnknownToken {
                value: format!("{paths:?}"),
                expected: vec!["units/corpus/explainer-plain.toml".into()],
            },
        ));
    }
    let sources = input::sources(root, &paths)?;
    let draft = crate::parse_slot(&sources[0])
        .map_err(|errors| DocumentError::Units(crate::UnitLoadError::Admission(errors)))?;
    let checked = admit_slot(draft, |unit, document, language| {
        units.units().iter().any(|entry| {
            entry.label().as_str() == unit
                && entry.document() == document
                && entry.texts().contains_key(language)
        })
    })
    .map_err(|fault| {
        failure(
            RuleId::M055,
            AssemblyStage::Ownership,
            AssemblySite::Source(input::site(&sources[0].file)),
            AssemblyFault::UnknownToken {
                value: format!("{fault:?}"),
                expected: vec!["checked corpus slot".into()],
            },
        )
    })?;
    Ok((BTreeMap::from([(checked.id().into(), checked)]), sources))
}

pub(super) fn physical_owner<'a>(
    unit: &'a str,
    slots: &'a BTreeMap<String, CheckedSlot>,
) -> &'a str {
    for (id, slot) in slots {
        if unit == id
            || unit.starts_with(&format!("{id}::paragraph:"))
            || slot.blocks().iter().any(|child| {
                matches!(child, ftoo_manifest::corpus::SlotBlock::Use { unit: nested, .. } if unit == nested)
            })
        {
            return id;
        }
    }
    unit
}

fn paper_text(value: &str) -> String {
    super::latex::prose_text(value)
        .replace('—', "---")
        .replace('“', "``")
        .replace('”', "''")
}

fn paper_runs(output: &mut String, runs: &[ftoo_manifest::corpus::SlotRun]) {
    use std::fmt::Write as _;
    for run in runs {
        match run {
            ftoo_manifest::corpus::SlotRun::Text(value) => output.push_str(&paper_text(value)),
            ftoo_manifest::corpus::SlotRun::EnSpace => output.push_str("\\enspace "),
            ftoo_manifest::corpus::SlotRun::Style { kind, children } => {
                let command = match kind {
                    ftoo_manifest::corpus::SlotStyle::Strong => "textbf",
                    ftoo_manifest::corpus::SlotStyle::Emphasis => "emph",
                };
                write!(output, "\\{command}{{").expect("string write");
                paper_runs(output, children);
                output.push('}');
            }
        }
    }
}

pub(super) fn fragment(
    slot: &CheckedSlot,
    intro: &super::plan::Block,
) -> Result<String, DocumentError> {
    use std::fmt::Write as _;
    let ftoo_manifest::corpus::SlotBlock::Heading { title, .. } = &slot.blocks()[0] else {
        unreachable!("admitted heading");
    };
    let mut output = String::new();
    output.push_str("\\section{");
    paper_runs(&mut output, title);
    writeln!(output, "}}\n\\label{{{}}}\n", slot.id()).expect("string write");
    output.push_str(&super::paragraph_fragment_line(intro)?);
    output.push('\n');
    for (index, child) in slot.blocks().iter().enumerate().skip(2) {
        let ftoo_manifest::corpus::SlotBlock::Paragraph { runs } = child else {
            unreachable!("admitted paragraph");
        };
        paper_runs(&mut output, runs);
        output.push('\n');
        if index + 1 < slot.blocks().len() {
            output.push('\n');
        }
    }
    Ok(output)
}
