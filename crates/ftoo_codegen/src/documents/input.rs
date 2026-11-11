use super::{
    diagnostics::{
        AssemblyFault, DocumentEntryKind, DocumentError, DocumentInputFault, DocumentIoOperation,
    },
    failure,
};
use crate::{
    UnitSource,
    units::toml::{self, Node},
};
use ftoo_manifest::{
    AssemblyResource, AssemblySite, AssemblyStage, LocatedString, RuleId, ShapeKind, SkeletonChild,
    SkeletonDraft, SkeletonSection, SourceAnchor, UnitDiagnostic, UnitErrors, UnitFault, UnitSite,
    UnitStage,
};
use std::{collections::BTreeMap, fs, io, path::Path};

pub(super) fn io_error(
    operation: DocumentIoOperation,
    path: &Path,
    e: &io::Error,
) -> DocumentError {
    DocumentError::Io {
        operation,
        path: path.into(),
        kind: e.kind(),
        replaced: Vec::new(),
        removed: Vec::new(),
        cleanup: Vec::new(),
    }
}
pub(super) fn read(root: &Path, file: &str) -> Result<Vec<u8>, DocumentError> {
    let path = root.join(file);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|e| io_error(DocumentIoOperation::Inspect, &path, &e))?;
    let kind = if metadata.file_type().is_symlink() {
        Some(DocumentEntryKind::Symlink)
    } else if metadata.is_dir() {
        Some(DocumentEntryKind::Directory)
    } else if !metadata.is_file() {
        Some(DocumentEntryKind::Other)
    } else {
        None
    };
    if let Some(kind) = kind {
        return Err(DocumentError::Input(DocumentInputFault::UnexpectedEntry {
            path,
            kind,
        }));
    }
    fs::read(&path).map_err(|e| io_error(DocumentIoOperation::Read, &path, &e))
}
pub(super) fn discover(
    root: &Path,
    directory: &str,
    absent: bool,
) -> Result<Vec<String>, DocumentError> {
    let path = root.join(directory);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if absent && e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_error(DocumentIoOperation::Inspect, &path, &e)),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(DocumentError::Input(DocumentInputFault::UnexpectedEntry {
            path,
            kind: if metadata.file_type().is_symlink() {
                DocumentEntryKind::Symlink
            } else {
                DocumentEntryKind::Other
            },
        }));
    }
    let entries =
        fs::read_dir(&path).map_err(|e| io_error(DocumentIoOperation::List, &path, &e))?;
    let mut paths = entries
        .map(|e| {
            e.map(|e| e.path())
                .map_err(|e| io_error(DocumentIoOperation::List, &path, &e))
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let mut files = Vec::new();
    for path in paths {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|e| io_error(DocumentIoOperation::Inspect, &path, &e))?;
        let name = path.file_name().and_then(|n| n.to_str());
        let kind = if metadata.file_type().is_symlink() {
            Some(DocumentEntryKind::Symlink)
        } else if metadata.is_dir() {
            Some(DocumentEntryKind::Directory)
        } else if !metadata.is_file() {
            Some(DocumentEntryKind::Other)
        } else if name.is_none() {
            Some(DocumentEntryKind::NonUtf8Name)
        } else if !name.is_some_and(|n| n.as_bytes().ends_with(b".toml")) {
            Some(DocumentEntryKind::NonTomlFile)
        } else {
            None
        };
        if let Some(kind) = kind {
            return Err(DocumentError::Input(DocumentInputFault::UnexpectedEntry {
                path,
                kind,
            }));
        }
        let name = name.unwrap_or_default();
        if name.contains('\\') || name.chars().any(char::is_control) {
            return Err(DocumentError::Input(
                DocumentInputFault::InvalidSourceName {
                    value: format!("{directory}/{name}"),
                },
            ));
        }
        files.push(format!("{directory}/{name}"));
    }
    Ok(files)
}
fn boundary(error: Box<UnitDiagnostic>) -> DocumentError {
    DocumentError::Boundary(UnitErrors::from_diagnostics(vec![*error]).expect("one diagnostic"))
}
fn phase<T>(
    items: impl Iterator<Item = Result<T, Box<UnitDiagnostic>>>,
) -> Result<Vec<T>, DocumentError> {
    let mut values = Vec::new();
    let mut errors = Vec::new();
    for item in items {
        match item {
            Ok(value) => values.push(value),
            Err(e) => errors.push(*e),
        }
    }
    if let Some(errors) = UnitErrors::from_diagnostics(errors) {
        Err(DocumentError::Boundary(errors))
    } else {
        Ok(values)
    }
}
pub(super) fn site(file: &str) -> UnitSite {
    UnitSite {
        file: file.into(),
        path: Vec::new(),
        anchor: SourceAnchor::WholeFile,
        decoded: None,
        occurrence: None,
    }
}
fn section(
    node: &Node,
    nested: bool,
    depth: usize,
    count: &mut usize,
) -> Result<SkeletonSection, DocumentError> {
    if depth > 6 {
        return Err(failure(
            RuleId::M054,
            AssemblyStage::Ownership,
            AssemblySite::Source(node.site.clone()),
            AssemblyFault::Limit {
                resource: AssemblyResource::SkeletonDepth,
                limit: 6,
                actual: 7,
            },
        ));
    }
    let fields = toml::fixed(
        node,
        if nested {
            &["kind", "heading", "children"]
        } else {
            &["heading", "children"]
        },
    )
    .map_err(boundary)?;
    let offset = usize::from(nested);
    let heading = position(fields[offset], count)?;
    let children = toml::typed(fields[offset + 1], ShapeKind::Array).map_err(boundary)?;
    let children = children
        .elements
        .iter()
        .map(|child| {
            toml::typed(child, ShapeKind::Table).map_err(boundary)?;
            let discriminator = child
                .fields
                .iter()
                .find(|(key, _, _)| key == "kind")
                .map(|(_, _, n)| n);
            let discriminator = if let Some(value) = discriminator {
                toml::string(value).map_err(boundary)?
            } else {
                return Err(boundary(Box::new(UnitDiagnostic {
                    rule: RuleId::M004,
                    stage: UnitStage::Shape,
                    unit: None,
                    language: None,
                    site: child.site.parent(),
                    fault: UnitFault::MissingField {
                        name: "kind".into(),
                    },
                })));
            };
            match discriminator.value.as_str() {
                "unit" => {
                    let fields = toml::fixed(child, &["kind", "unit"]).map_err(boundary)?;
                    Ok(SkeletonChild::Unit(position(fields[1], count)?))
                }
                "section" => section(child, true, depth + 1, count).map(SkeletonChild::Section),
                _ => {
                    let mut site = discriminator.site;
                    site.decoded = Some(0..discriminator.value.len());
                    Err(failure(
                        RuleId::M004,
                        AssemblyStage::Shape,
                        AssemblySite::Source(site),
                        AssemblyFault::UnknownToken {
                            value: discriminator.value,
                            expected: vec!["unit".into(), "section".into()],
                        },
                    ))
                }
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SkeletonSection { heading, children })
}
fn position(node: &Node, count: &mut usize) -> Result<LocatedString, DocumentError> {
    let value = toml::string(node).map_err(boundary)?;
    if *count == 4096 {
        return Err(failure(
            RuleId::M054,
            AssemblyStage::Ownership,
            AssemblySite::Source(value.site),
            AssemblyFault::Limit {
                resource: AssemblyResource::UnitPositions,
                limit: 4096,
                actual: 4097,
            },
        ));
    }
    *count += 1;
    Ok(value)
}
fn root_shape<'a>(tree: &'a Node, text: &str) -> Result<&'a Node, Box<UnitDiagnostic>> {
    let fields = toml::fixed(tree, &["schema", "skeleton"])?;
    toml::typed(fields[0], ShapeKind::Integer)?;
    if toml::raw(fields[0], text) != "1" {
        return Err(Box::new(UnitDiagnostic {
            rule: RuleId::M003,
            stage: UnitStage::Shape,
            unit: None,
            language: None,
            site: fields[0].site.clone(),
            fault: UnitFault::SchemaValue {
                expected: "1".into(),
                actual: toml::raw(fields[0], text).into(),
            },
        }));
    }
    let fields = toml::fixed(fields[1], &["document", "sections"])?;
    toml::string(fields[0])?;
    toml::typed(fields[1], ShapeKind::Array)
}
fn discriminator(node: &Node) -> Result<LocatedString, Box<UnitDiagnostic>> {
    toml::typed(node, ShapeKind::Table)?;
    let value = node.fields.iter().find(|(key, _, _)| key == "kind");
    if let Some((_, _, value)) = value {
        toml::string(value)
    } else {
        Err(Box::new(UnitDiagnostic {
            rule: RuleId::M004,
            stage: UnitStage::Shape,
            unit: None,
            language: None,
            site: node.site.parent(),
            fault: UnitFault::MissingField {
                name: "kind".into(),
            },
        }))
    }
}
fn shape(tree: &Node, text: &str) -> Result<Option<LocatedString>, Box<UnitDiagnostic>> {
    let sections = root_shape(tree, text)?;
    let mut stack: Vec<_> = sections
        .elements
        .iter()
        .rev()
        .map(|node| (node, false))
        .collect();
    let mut unknown = None;
    while let Some((node, nested)) = stack.pop() {
        let kind = if nested {
            Some(discriminator(node)?)
        } else {
            None
        };
        if let Some(kind) = &kind {
            if kind.value != "unit" && kind.value != "section" {
                if unknown.is_none() {
                    unknown = Some(kind.clone());
                }
                continue;
            }
            if kind.value == "unit" {
                let fields = toml::fixed(node, &["kind", "unit"])?;
                toml::string(fields[1])?;
                continue;
            }
        }
        let fields = toml::fixed(
            node,
            if nested {
                &["kind", "heading", "children"]
            } else {
                &["heading", "children"]
            },
        )?;
        let offset = usize::from(nested);
        toml::string(fields[offset])?;
        let children = toml::typed(fields[offset + 1], ShapeKind::Array)?;
        stack.extend(children.elements.iter().rev().map(|child| (child, true)));
    }
    Ok(unknown)
}
pub(super) fn skeletons(sources: &[UnitSource]) -> Result<Vec<SkeletonDraft>, DocumentError> {
    if sources.is_empty() {
        return Err(DocumentError::Input(DocumentInputFault::EmptySkeletons {
            directory: "units/skeletons".into(),
        }));
    }
    let mut positions: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, source) in sources.iter().enumerate() {
        if source.file.is_empty()
            || source.file.contains('\\')
            || source.file.chars().any(char::is_control)
            || source
                .file
                .split('/')
                .any(|part| matches!(part, "" | "." | ".."))
        {
            return Err(DocumentError::Input(
                DocumentInputFault::InvalidSourceName {
                    value: source.file.clone(),
                },
            ));
        }
        positions.entry(&source.file).or_default().push(index);
    }
    if let Some((file, positions)) = positions
        .into_iter()
        .find(|(_, positions)| positions.len() > 1)
    {
        return Err(DocumentError::Input(DocumentInputFault::DuplicateSource {
            file: file.into(),
            positions,
        }));
    }
    let mut sources: Vec<_> = sources.iter().collect();
    sources.sort_by(|a, b| a.file.cmp(&b.file));
    for source in &sources {
        if source.bytes.len() > 1_048_576 {
            return Err(failure(
                RuleId::M054,
                AssemblyStage::Source,
                AssemblySite::Source(site(&source.file)),
                AssemblyFault::Limit {
                    resource: AssemblyResource::SourceBytes,
                    limit: 1_048_576,
                    actual: 1_048_577,
                },
            ));
        }
    }
    let texts = phase(sources.iter().map(|source| toml::encoding(source)))?;
    let trees = phase(
        sources
            .iter()
            .zip(&texts)
            .map(|(s, t)| toml::parse(&s.file, t)),
    )?;
    phase(trees.iter().zip(&texts).map(|(n, t)| toml::numeric(n, t)))?;
    let unknown = phase(
        trees
            .iter()
            .zip(&texts)
            .map(|(tree, text)| shape(tree, text)),
    )?;
    let variants = unknown.into_iter().flatten().map(unknown_kind).collect();
    if let Some(errors) = super::AssemblyErrors::from_diagnostics(variants) {
        return Err(DocumentError::Assembly(errors));
    }
    let mut count = 0;
    trees
        .iter()
        .map(|tree| {
            let fields = toml::fixed(tree, &["schema", "skeleton"]).map_err(boundary)?;
            let fields = toml::fixed(fields[1], &["document", "sections"]).map_err(boundary)?;
            let document = toml::string(fields[0]).map_err(boundary)?;
            let sections = toml::typed(fields[1], ShapeKind::Array).map_err(boundary)?;
            Ok(SkeletonDraft {
                document,
                sections_site: sections.site.clone(),
                sections: sections
                    .elements
                    .iter()
                    .map(|n| section(n, false, 1, &mut count))
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect()
}
fn unknown_kind(value: LocatedString) -> super::AssemblyDiagnostic {
    let mut site = value.site;
    site.decoded = Some(0..value.value.len());
    super::AssemblyDiagnostic {
        rule: RuleId::M004,
        stage: AssemblyStage::Shape,
        document: None,
        unit: None,
        language: None,
        target: None,
        site: AssemblySite::Source(site),
        fault: AssemblyFault::UnknownToken {
            value: value.value,
            expected: vec!["unit".into(), "section".into()],
        },
    }
}
pub(super) fn sources(root: &Path, files: &[String]) -> Result<Vec<UnitSource>, DocumentError> {
    files
        .iter()
        .map(|file| {
            Ok(UnitSource {
                file: file.clone(),
                bytes: read(root, file)?,
            })
        })
        .collect()
}
pub(super) fn reread(
    root: &Path,
    inventory: &BTreeMap<String, Vec<u8>>,
) -> Result<(), DocumentError> {
    for (file, expected) in inventory {
        let path = root.join(file);
        let actual = match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(io_error(DocumentIoOperation::Read, &path, &e)),
        };
        if actual.as_ref() != Some(expected) {
            return Err(DocumentError::Input(DocumentInputFault::SourceChanged {
                file: file.clone(),
                expected: expected.clone(),
                actual,
            }));
        }
    }
    Ok(())
}

fn display_shape(tree: &Node, text: &str) -> Result<Option<LocatedString>, Box<UnitDiagnostic>> {
    let fields = toml::fixed(tree, &["schema", "display"])?;
    toml::typed(fields[0], ShapeKind::Integer)?;
    if toml::raw(fields[0], text) != "1" {
        return Err(Box::new(UnitDiagnostic {
            rule: RuleId::M003,
            stage: UnitStage::Shape,
            unit: None,
            language: None,
            site: fields[0].site.clone(),
            fault: UnitFault::SchemaValue {
                expected: "1".into(),
                actual: toml::raw(fields[0], text).into(),
            },
        }));
    }
    let fields = toml::fixed(fields[1], &["unit", "kind", "bodies"])?;
    toml::string(fields[0])?;
    let kind = toml::string(fields[1])?;
    let targets = toml::fixed(fields[2], &["markdown", "latex"])?;
    for target in targets {
        toml::typed(target, ShapeKind::Table)?;
        for (_, _, value) in &target.fields {
            toml::string(value)?;
        }
    }
    Ok(if matches!(kind.value.as_str(), "table" | "figure") {
        None
    } else {
        Some(kind)
    })
}
pub(super) fn assembly_sources(
    skeleton_sources: &[UnitSource],
    display_sources: &[UnitSource],
) -> Result<(Vec<SkeletonDraft>, Vec<ftoo_manifest::DisplayDraft>), DocumentError> {
    let mut sources: Vec<_> = skeleton_sources
        .iter()
        .map(|s| (s, false))
        .chain(display_sources.iter().map(|s| (s, true)))
        .collect();
    let mut names = BTreeMap::<&str, Vec<usize>>::new();
    for (position, (source, display)) in sources.iter().enumerate() {
        if source.file.is_empty()
            || source.file.contains('\\')
            || source.file.chars().any(char::is_control)
            || source.file.split('/').any(|p| matches!(p, "" | "." | ".."))
        {
            return Err(DocumentError::Input(
                DocumentInputFault::InvalidSourceName {
                    value: source.file.clone(),
                },
            ));
        }
        names.entry(&source.file).or_default().push(position);
        if source.bytes.len() > 1_048_576 {
            return Err(failure(
                if *display { RuleId::M058 } else { RuleId::M054 },
                AssemblyStage::Source,
                AssemblySite::Source(site(&source.file)),
                AssemblyFault::Limit {
                    resource: AssemblyResource::SourceBytes,
                    limit: 1_048_576,
                    actual: 1_048_577,
                },
            ));
        }
    }
    if let Some((file, positions)) = names.into_iter().find(|(_, p)| p.len() > 1) {
        return Err(DocumentError::Input(DocumentInputFault::DuplicateSource {
            file: file.into(),
            positions,
        }));
    }
    sources.sort_by(|a, b| a.0.file.cmp(&b.0.file));
    let texts = phase(sources.iter().map(|(s, _)| toml::encoding(s)))?;
    let trees = phase(
        sources
            .iter()
            .zip(&texts)
            .map(|((s, _), t)| toml::parse(&s.file, t)),
    )?;
    phase(trees.iter().zip(&texts).map(|(n, t)| toml::numeric(n, t)))?;
    let unknown = phase(trees.iter().zip(&texts).zip(&sources).map(
        |((tree, text), (_, display))| {
            if *display {
                display_shape(tree, text)
            } else {
                shape(tree, text)
            }
        },
    ))?;
    let variants = unknown
        .into_iter()
        .zip(&sources)
        .filter_map(|(kind, (_, display))| {
            kind.map(|kind| {
                let mut error = unknown_kind(kind);
                if *display && let AssemblyFault::UnknownToken { expected, .. } = &mut error.fault {
                    *expected = vec!["table".into(), "figure".into()];
                }
                error
            })
        })
        .collect();
    if let Some(errors) = super::AssemblyErrors::from_diagnostics(variants) {
        return Err(DocumentError::Assembly(errors));
    }
    let mut displays = Vec::new();
    for (tree, (_, display)) in trees.iter().zip(&sources) {
        if !display {
            continue;
        }
        displays.push(display_draft(tree)?);
    }
    Ok((skeletons(skeleton_sources)?, displays))
}

fn display_draft(tree: &Node) -> Result<ftoo_manifest::DisplayDraft, DocumentError> {
    let fields = toml::fixed(tree, &["schema", "display"]).map_err(boundary)?;
    let fields = toml::fixed(fields[1], &["unit", "kind", "bodies"]).map_err(boundary)?;
    let unit = toml::string(fields[0]).map_err(boundary)?;
    let kind = if toml::string(fields[1]).map_err(boundary)?.value == "table" {
        ftoo_manifest::DisplayKind::Table
    } else {
        ftoo_manifest::DisplayKind::Figure
    };
    let targets = toml::fixed(fields[2], &["markdown", "latex"]).map_err(boundary)?;
    let mut bodies = BTreeMap::new();
    for (target, node) in [
        ftoo_manifest::DocumentTarget::Markdown,
        ftoo_manifest::DocumentTarget::Latex,
    ]
    .into_iter()
    .zip(targets)
    {
        let languages = node
            .fields
            .iter()
            .map(|(language, _, value)| {
                Ok((language.clone(), toml::string(value).map_err(boundary)?))
            })
            .collect::<Result<_, DocumentError>>()?;
        bodies.insert(
            target,
            ftoo_manifest::DisplayTargetDraft {
                site: node.site.clone(),
                languages,
            },
        );
    }
    Ok(ftoo_manifest::DisplayDraft { unit, kind, bodies })
}
