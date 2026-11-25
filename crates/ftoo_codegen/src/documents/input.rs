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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Inclusion {
    pub(crate) source: String,
    pub(crate) unit: String,
    pub(crate) fragment: String,
    pub(crate) role: String,
    pub(crate) line: usize,
}
const EXCERPTS: [(&str, &str, Option<&str>); 2] = [
    ("paper-pilot", "sec:pilot:paper-excerpt", Some("paper")),
    ("explainer-pilot", "sec:pilot:explainer-excerpt", None),
];
pub(super) fn fragment_path(document: &str, unit: &str) -> String {
    format!(
        "generated/documents/{}/{}.tex",
        document,
        unit.replace(':', "-")
    )
}
#[derive(Clone, Debug)]
struct ScopeRecord {
    source: String,
    document: String,
    occurrences: Vec<(String, String, UnitSite)>,
    site: UnitSite,
}
fn field<'a>(node: &'a Node, name: &str) -> Result<&'a Node, DocumentError> {
    node.fields
        .iter()
        .find(|(key, _, _)| key == name)
        .map(|(_, _, value)| value)
        .ok_or_else(|| {
            boundary(Box::new(UnitDiagnostic {
                rule: RuleId::M004,
                stage: UnitStage::Shape,
                unit: None,
                language: None,
                site: node.site.child(name),
                fault: UnitFault::MissingField { name: name.into() },
            }))
        })
}
fn string_value(node: &Node) -> Result<String, DocumentError> {
    Ok(toml::string(node).map_err(boundary)?.value)
}
fn mismatch(site: &UnitSite, record: &str, position: &str) -> DocumentError {
    failure(
        RuleId::M079,
        AssemblyStage::Ownership,
        AssemblySite::Source(site.clone()),
        AssemblyFault::CompositionMismatch {
            record: record.into(),
            position: position.into(),
        },
    )
}
fn scope(source: &UnitSource) -> Result<ScopeRecord, DocumentError> {
    let text = toml::encoding(source).map_err(boundary)?;
    let tree = toml::parse(&source.file, text).map_err(boundary)?;
    toml::numeric(&tree, text).map_err(boundary)?;
    let root = toml::fixed(&tree, &["schema", "scope"]).map_err(boundary)?;
    toml::typed(root[0], ShapeKind::Integer).map_err(boundary)?;
    if toml::raw(root[0], text) != "1" {
        return Err(boundary(Box::new(UnitDiagnostic {
            rule: RuleId::M003,
            stage: UnitStage::Shape,
            unit: None,
            language: None,
            site: root[0].site.clone(),
            fault: UnitFault::SchemaValue {
                expected: "1".into(),
                actual: toml::raw(root[0], text).into(),
            },
        })));
    }
    let scope = root[1];
    let fields = toml::fixed(scope, &["document", "source", "occurrences"]).map_err(boundary)?;
    let document = string_value(fields[0])?;
    let path = string_value(fields[1])?;
    if !path.starts_with("documents/paper/") || path.contains("..") {
        return Err(mismatch(&fields[1].site, &source.file, "source"));
    }
    let occurrences = toml::typed(fields[2], ShapeKind::Array)
        .map_err(boundary)?
        .elements
        .iter()
        .enumerate()
        .map(|(index, occurrence)| {
            let mut expected = vec!["unit", "role"];
            if occurrence
                .fields
                .iter()
                .any(|(name, _, _)| name == "numbering")
            {
                expected.push("numbering");
            }
            let fields = toml::fixed(occurrence, &expected).map_err(boundary)?;
            let unit = string_value(fields[0])?;
            let role = string_value(fields[1])?;
            if !matches!(
                role.as_str(),
                "refrain" | "paragraph" | "quotation" | "equation" | "econ"
            ) || (role == "equation") != (fields.len() == 3)
                || (role == "equation" && string_value(fields[2])? != "none")
            {
                return Err(mismatch(
                    &fields[1].site,
                    &source.file,
                    &format!("occurrence {index}"),
                ));
            }
            Ok((unit, role, occurrence.site.clone()))
        })
        .collect::<Result<Vec<_>, DocumentError>>()?;
    Ok(ScopeRecord {
        source: path,
        document,
        occurrences,
        site: scope.site.clone(),
    })
}
/// Derive excerpt uses and checked source inclusions from the source scopes.
pub(super) fn composition(
    root: &Path,
    declaration: &UnitSource,
    scope_sources: &[UnitSource],
    skeletons: &[SkeletonDraft],
) -> Result<Vec<Inclusion>, DocumentError> {
    compose(root, declaration, scope_sources, Some(skeletons))
}

/// Check source-scope ownership for a diagnostic corpus read without selecting skeleton output.
pub(crate) fn import_inclusions(root: &Path) -> Result<Vec<Inclusion>, DocumentError> {
    let declaration = UnitSource {
        file: "units/documents.toml".into(),
        bytes: read(root, "units/documents.toml")?,
    };
    let files = discover(root, "units/scopes", false)?;
    let scopes = sources(root, &files)?;
    compose(root, &declaration, &scopes, None)
}

fn compose(
    root: &Path,
    declaration: &UnitSource,
    scope_sources: &[UnitSource],
    skeletons: Option<&[SkeletonDraft]>,
) -> Result<Vec<Inclusion>, DocumentError> {
    let text = toml::encoding(declaration).map_err(boundary)?;
    let tree = toml::parse(&declaration.file, text).map_err(boundary)?;
    let declared = field(&tree, "documents")?;
    let master = read(root, "documents/paper-build/sources.tsv")?;
    let master = std::str::from_utf8(&master)
        .map_err(|_| mismatch(&declared.site, "sources.tsv", "encoding"))?;
    let order: BTreeMap<_, _> = master
        .lines()
        .enumerate()
        .map(|(index, path)| (path, index))
        .collect();
    let mut grouped: BTreeMap<String, Vec<(usize, Inclusion, UnitSite)>> = BTreeMap::new();
    let mut seen_sources = std::collections::BTreeSet::new();
    let mut seen_uses = std::collections::BTreeSet::new();
    for source in scope_sources {
        let record = scope(source)?;
        if !seen_sources.insert(record.source.clone()) {
            return Err(mismatch(&record.site, &source.file, "duplicate source"));
        }
        let Some(&source_order) = order.get(record.source.as_str()) else {
            return Err(mismatch(&record.site, &source.file, "closed source list"));
        };
        if !EXCERPTS
            .iter()
            .any(|(document, _, _)| *document == record.document)
        {
            return Err(mismatch(&record.site, &source.file, "document"));
        }
        let bytes = read(root, &record.source)?;
        for (index, (unit, role, site)) in record.occurrences.iter().enumerate() {
            if !seen_uses.insert((record.document.clone(), unit.clone())) {
                return Err(mismatch(site, &source.file, &format!("occurrence {index}")));
            }
            let marker = format!("% generated: {unit}");
            let lines: Vec<_> = bytes.split(|byte| *byte == b'\n').collect();
            let positions: Vec<_> = lines
                .iter()
                .enumerate()
                .filter_map(|(line, bytes)| (*bytes == marker.as_bytes()).then_some(line + 1))
                .collect();
            if positions.len() != 1 {
                return Err(failure(
                    RuleId::M055,
                    AssemblyStage::Ownership,
                    AssemblySite::Source(site.clone()),
                    AssemblyFault::UnknownToken {
                        value: format!("marker {unit} in {}", record.source),
                        expected: vec!["one declared marker line".into()],
                    },
                ));
            }
            let fragment = fragment_path(&record.document, unit);
            let inclusion = Inclusion {
                source: record.source.clone(),
                unit: unit.clone(),
                fragment,
                role: role.clone(),
                line: positions[0],
            };
            grouped.entry(record.document.clone()).or_default().push((
                source_order,
                inclusion,
                site.clone(),
            ));
        }
    }
    check_projection(declared, skeletons, grouped)
}

fn check_projection(
    declared: &Node,
    skeletons: Option<&[SkeletonDraft]>,
    mut grouped: BTreeMap<String, Vec<(usize, Inclusion, UnitSite)>>,
) -> Result<Vec<Inclusion>, DocumentError> {
    let mut result = Vec::new();
    for (document, heading, context) in EXCERPTS {
        let entry = field(declared, document)?;
        let mut fields = vec!["required-languages", "targets"];
        if context.is_some() {
            fields.push("paper-context");
        }
        toml::fixed(entry, &fields).map_err(boundary)?;
        let expected_languages = vec!["en".to_owned()];
        let expected_targets = vec![
            "fragment".to_owned(),
            "markdown".to_owned(),
            "html".to_owned(),
        ];
        let actual_languages = toml::typed(field(entry, "required-languages")?, ShapeKind::Array)
            .map_err(boundary)?
            .elements
            .iter()
            .map(string_value)
            .collect::<Result<Vec<_>, _>>()?;
        let actual_targets = toml::typed(field(entry, "targets")?, ShapeKind::Array)
            .map_err(boundary)?
            .elements
            .iter()
            .map(string_value)
            .collect::<Result<Vec<_>, _>>()?;
        let actual_context = entry
            .fields
            .iter()
            .find(|(name, _, _)| name == "paper-context")
            .map(|(_, _, value)| string_value(value))
            .transpose()?;
        if actual_languages != expected_languages
            || actual_targets != expected_targets
            || actual_context.as_deref() != context
        {
            return Err(mismatch(&entry.site, document, "declaration"));
        }
        let mut uses = grouped.remove(document).unwrap_or_default();
        uses.sort_by_key(|(order, inclusion, _)| (*order, inclusion.line));
        if uses.is_empty() {
            return Err(mismatch(&entry.site, document, "empty excerpt"));
        }
        if let Some(skeletons) = skeletons {
            let draft = skeletons
                .iter()
                .find(|draft| draft.document.value == document)
                .ok_or_else(|| mismatch(&declared.site, document, "skeleton"))?;
            if draft.sections.len() != 1
                || draft.sections[0].heading.value != heading
                || draft.sections[0].children.len() != uses.len()
            {
                return Err(mismatch(&draft.document.site, document, "skeleton shape"));
            }
            for (index, (child, (_, inclusion, site))) in
                draft.sections[0].children.iter().zip(&uses).enumerate()
            {
                let matches = match child {
                    SkeletonChild::Unit(value) => {
                        inclusion.role == "refrain" && value.value == inclusion.unit
                    }
                    SkeletonChild::Paragraph(value) => {
                        inclusion.role == "paragraph" && value.value == inclusion.unit
                    }
                    SkeletonChild::Econ(value) => {
                        inclusion.role == "econ" && value.value == inclusion.unit
                    }
                    SkeletonChild::Equation(value) => {
                        inclusion.role == "equation" && value.value == inclusion.unit
                    }
                    SkeletonChild::Quotation(value) => {
                        inclusion.role == "quotation" && value.value == inclusion.unit
                    }
                    SkeletonChild::Section(_) => false,
                };
                if !matches {
                    return Err(mismatch(site, document, &format!("skeleton child {index}")));
                }
            }
        }
        result.extend(uses.into_iter().map(|(_, inclusion, _)| inclusion));
    }
    Ok(result)
}

pub(super) fn generate_projections(root: &Path) -> Result<(), DocumentError> {
    use std::fmt::Write as _;
    let declaration = UnitSource {
        file: "units/documents.toml".into(),
        bytes: read(root, "units/documents.toml")?,
    };
    let files = discover(root, "units/scopes", false)?;
    let scopes = sources(root, &files)?;
    let inclusions = compose(root, &declaration, &scopes, None)?;
    for (document, heading, _) in EXCERPTS {
        let mut text = format!(
            "schema = 1\n\n[skeleton]\ndocument = \"{document}\"\n\n[[skeleton.sections]]\nheading = \"{heading}\"\nchildren = [\n"
        );
        let selected: Vec<_> = inclusions
            .iter()
            .filter(|row| row.fragment == fragment_path(document, &row.unit))
            .collect();
        for (index, inclusion) in selected.iter().enumerate() {
            let kind = if inclusion.role == "refrain" {
                "unit"
            } else {
                &inclusion.role
            };
            let comma = if index + 1 == selected.len() { "" } else { "," };
            if kind == "equation" {
                writeln!(
                    text,
                    "  {{ kind = \"equation\", unit = \"{}\", numbering = \"none\" }}{comma}",
                    inclusion.unit
                )
                .expect("string write");
            } else {
                writeln!(
                    text,
                    "  {{ kind = \"{kind}\", unit = \"{}\" }}{comma}",
                    inclusion.unit
                )
                .expect("string write");
            }
        }
        text.push_str("]\n");
        let path = root.join(format!("units/skeletons/{document}.toml"));
        if fs::read(&path).map_err(|error| io_error(DocumentIoOperation::Read, &path, &error))?
            != text.as_bytes()
        {
            fs::write(&path, text)
                .map_err(|error| io_error(DocumentIoOperation::Write, &path, &error))?;
        }
    }
    Ok(())
}

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
                "paragraph" => {
                    let fields = toml::fixed(child, &["kind", "unit"]).map_err(boundary)?;
                    Ok(SkeletonChild::Paragraph(position(fields[1], count)?))
                }
                "econ" => {
                    let fields = toml::fixed(child, &["kind", "unit"]).map_err(boundary)?;
                    Ok(SkeletonChild::Econ(position(fields[1], count)?))
                }
                "equation" => {
                    let fields =
                        toml::fixed(child, &["kind", "unit", "numbering"]).map_err(boundary)?;
                    if string_value(fields[2])? != "none" {
                        return Err(mismatch(&fields[2].site, "skeleton", "numbering"));
                    }
                    Ok(SkeletonChild::Equation(position(fields[1], count)?))
                }
                "quotation" => {
                    let fields = toml::fixed(child, &["kind", "unit"]).map_err(boundary)?;
                    Ok(SkeletonChild::Quotation(position(fields[1], count)?))
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
                            expected: vec![
                                "unit".into(),
                                "paragraph".into(),
                                "econ".into(),
                                "equation".into(),
                                "quotation".into(),
                                "section".into(),
                            ],
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
            if kind.value != "unit"
                && kind.value != "paragraph"
                && kind.value != "econ"
                && kind.value != "equation"
                && kind.value != "quotation"
                && kind.value != "section"
            {
                if unknown.is_none() {
                    unknown = Some(kind.clone());
                }
                continue;
            }
            if matches!(
                kind.value.as_str(),
                "unit" | "paragraph" | "econ" | "quotation" | "equation"
            ) {
                let fields = if kind.value == "equation" {
                    toml::fixed(node, &["kind", "unit", "numbering"])?
                } else {
                    toml::fixed(node, &["kind", "unit"])?
                };
                toml::string(fields[1])?;
                if kind.value == "equation" {
                    toml::string(fields[2])?;
                }
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
            expected: vec![
                "unit".into(),
                "paragraph".into(),
                "econ".into(),
                "equation".into(),
                "quotation".into(),
                "section".into(),
            ],
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
        let actual = if file == super::registry::FILE {
            match super::registry::read(root) {
                Ok(source) => Some(source.bytes),
                Err(DocumentError::Io {
                    kind: io::ErrorKind::NotFound,
                    ..
                }) => None,
                Err(error) => return Err(error),
            }
        } else {
            match fs::read(&path) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => return Err(io_error(DocumentIoOperation::Read, &path, &e)),
            }
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

#[cfg(test)]
mod composition_tests;
