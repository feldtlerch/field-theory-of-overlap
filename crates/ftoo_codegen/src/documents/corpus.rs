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

pub(crate) fn load(
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
    let sources = input::sources(root, &paths)?;
    let mut slots = BTreeMap::new();
    for source in &sources {
        let draft = crate::parse_slot(source)
            .map_err(|errors| DocumentError::Units(crate::UnitLoadError::Admission(errors)))?;
        let checked = admit_slot(draft, &source.file, |unit, document, language| {
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
                AssemblySite::Source(input::site(&source.file)),
                AssemblyFault::UnknownToken {
                    value: format!("{fault:?}"),
                    expected: vec!["checked corpus slot".into()],
                },
            )
        })?;
        let id = checked.id().to_owned();
        if slots.insert(id.clone(), checked).is_some() {
            return Err(failure(
                RuleId::M055,
                AssemblyStage::Ownership,
                AssemblySite::Source(input::site(&source.file)),
                AssemblyFault::UnknownToken {
                    value: id,
                    expected: vec!["unique corpus slot identity".into()],
                },
            ));
        }
    }
    Ok((slots, sources))
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
    blocks: &[super::plan::Block],
) -> Result<String, DocumentError> {
    use std::fmt::Write as _;
    if blocks.len() != slot.blocks().len() {
        return Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(input::site(slot.source())),
            AssemblyFault::UnknownToken {
                value: blocks.len().to_string(),
                expected: vec![format!("{} ordered slot children", slot.blocks().len())],
            },
        ));
    }
    let mut output = String::new();
    for (index, (child, block)) in slot.blocks().iter().zip(blocks).enumerate() {
        if index > 0 {
            output.push('\n');
        }
        match child {
            ftoo_manifest::corpus::SlotBlock::Heading { title, .. } => {
                output.push_str("\\section{");
                paper_runs(&mut output, title);
                writeln!(output, "}}\n\\label{{{}}}", slot.id()).expect("string write");
            }
            ftoo_manifest::corpus::SlotBlock::Use { unit, .. } => {
                if block.unit != *unit {
                    return Err(failure(
                        RuleId::M055,
                        AssemblyStage::Emission,
                        AssemblySite::Source(input::site(slot.source())),
                        AssemblyFault::UnknownToken {
                            value: block.unit.clone(),
                            expected: vec![unit.clone()],
                        },
                    ));
                }
                output.push_str(&super::paragraph_fragment_line(block)?);
            }
            ftoo_manifest::corpus::SlotBlock::Paragraph { runs } => {
                paper_runs(&mut output, runs);
                output.push('\n');
            }
        }
    }
    output.push_str("\\par\n");
    Ok(output)
}
