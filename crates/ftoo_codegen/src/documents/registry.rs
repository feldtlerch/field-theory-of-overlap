//! Strict byte and shape boundary for the documentary authority ledger.
#[cfg(test)]
mod tests;
use super::{DocumentError, input};
use crate::{UnitSource, units::toml};
use ftoo_manifest::{
    DocumentaryEventDraft, LocatedString, RegistryDiagnostic, RegistryErrors, RegistryFault,
    RegistryResource, RegistryStage, RuleId, ShapeKind, UnitDiagnostic, UnitErrors, UnitFault,
    UnitStage,
};

pub(super) const FILE: &str = "registry/documentary-tags.toml";
pub(super) fn read(root: &std::path::Path) -> Result<UnitSource, DocumentError> {
    use std::io::Read;
    let path = root.join(FILE);
    let metadata = std::fs::symlink_metadata(&path)
        .map_err(|e| input::io_error(super::DocumentIoOperation::Inspect, &path, &e))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DocumentError::Input(
            super::DocumentInputFault::UnexpectedEntry {
                path,
                kind: if metadata.file_type().is_symlink() {
                    super::DocumentEntryKind::Symlink
                } else if metadata.is_dir() {
                    super::DocumentEntryKind::Directory
                } else {
                    super::DocumentEntryKind::Other
                },
            },
        ));
    }
    if metadata.len() > 1_048_576 {
        return Err(limit(input::site(FILE), RegistryResource::Bytes, 1_048_576));
    }
    let file = std::fs::File::open(&path)
        .map_err(|e| input::io_error(super::DocumentIoOperation::Read, &path, &e))?;
    let mut bytes = vec![];
    file.take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|e| input::io_error(super::DocumentIoOperation::Read, &path, &e))?;
    if bytes.len() > 1_048_576 {
        return Err(limit(input::site(FILE), RegistryResource::Bytes, 1_048_576));
    }
    Ok(UnitSource {
        file: FILE.into(),
        bytes,
    })
}

fn boundary(error: Box<UnitDiagnostic>) -> DocumentError {
    DocumentError::Boundary(UnitErrors::from_diagnostics(vec![*error]).expect("one diagnostic"))
}
fn limit(
    site: ftoo_manifest::UnitSite,
    resource: RegistryResource,
    maximum: usize,
) -> DocumentError {
    DocumentError::DocumentaryRegistry(
        RegistryErrors::from_diagnostics(vec![RegistryDiagnostic {
            rule: RuleId::M060,
            stage: RegistryStage::Source,
            site,
            fault: RegistryFault::Limit {
                resource,
                limit: maximum,
                actual: maximum + 1,
            },
        }])
        .expect("one diagnostic"),
    )
}
#[allow(clippy::too_many_lines)] // Keep the closed event schema and its field provenance together.
pub(super) fn load(source: &UnitSource) -> Result<Vec<DocumentaryEventDraft>, DocumentError> {
    if source.bytes.len() > 1_048_576 {
        return Err(limit(
            input::site(&source.file),
            RegistryResource::Bytes,
            1_048_576,
        ));
    }
    let text = toml::encoding(source).map_err(boundary)?;
    let tree = toml::parse_array_bounded(&source.file, text, "events", 4096)
        .map_err(boundary)?
        .map_err(|site| limit(site, RegistryResource::Events, 4096))?;
    toml::numeric(&tree, text).map_err(boundary)?;
    let fields = toml::fixed(&tree, &["schema", "events"]).map_err(boundary)?;
    toml::typed(fields[0], ShapeKind::Integer).map_err(boundary)?;
    if toml::raw(fields[0], text) != "1" {
        return Err(boundary(Box::new(UnitDiagnostic {
            rule: RuleId::M003,
            stage: UnitStage::Shape,
            unit: None,
            language: None,
            site: fields[0].site.clone(),
            fault: UnitFault::SchemaValue {
                expected: "1".into(),
                actual: toml::raw(fields[0], text).into(),
            },
        })));
    }
    let events = toml::typed(fields[1], ShapeKind::Array).map_err(boundary)?;
    if events.elements.len() > 4096 {
        return Err(limit(events.site.clone(), RegistryResource::Events, 4096));
    }
    events
        .elements
        .iter()
        .map(|event| {
            toml::typed(event, ShapeKind::Table).map_err(boundary)?;
            let change = event
                .fields
                .iter()
                .find(|(key, _, _)| key == "change")
                .map(|(_, _, value)| value);
            let Some(change) = change else {
                return Err(boundary(Box::new(UnitDiagnostic {
                    rule: RuleId::M004,
                    stage: UnitStage::Shape,
                    unit: None,
                    language: None,
                    site: event.site.parent(),
                    fault: UnitFault::MissingField {
                        name: "change".into(),
                    },
                })));
            };
            let change = toml::string(change).map_err(boundary)?;
            let admit = match change.value.as_str() {
                "admit" => true,
                "retire" => false,
                _ => {
                    return Err(boundary(Box::new(UnitDiagnostic {
                        rule: RuleId::M004,
                        stage: UnitStage::Shape,
                        unit: None,
                        language: None,
                        site: change.site,
                        fault: UnitFault::UnknownToken {
                            value: change.value,
                            expected: vec!["admit".into(), "retire".into()],
                        },
                    })));
                }
            };
            let keys: &[&str] = if admit {
                &[
                    "ordinal",
                    "change",
                    "handle",
                    "authority-record",
                    "reason",
                    "label",
                ]
            } else {
                &["ordinal", "change", "handle", "authority-record", "reason"]
            };
            let fields = toml::fixed(event, keys).map_err(boundary)?;
            toml::typed(fields[0], ShapeKind::Integer).map_err(boundary)?;
            Ok(DocumentaryEventDraft {
                site: event.site.clone(),
                ordinal: LocatedString {
                    value: toml::raw(fields[0], text).into(),
                    site: fields[0].site.clone(),
                },
                handle: toml::string(fields[2]).map_err(boundary)?,
                authority_record: toml::string(fields[3]).map_err(boundary)?,
                reason: toml::string(fields[4]).map_err(boundary)?,
                label: if admit {
                    Some(toml::string(fields[5]).map_err(boundary)?)
                } else {
                    None
                },
            })
        })
        .collect()
}
