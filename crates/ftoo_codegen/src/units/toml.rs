use super::UnitSource;
use ftoo_manifest::{
    DocumentDraft, DocumentEntryDraft, LocatedEntryDraft, LocatedString, PathPart, RuleId,
    ShapeKind, SourceAnchor, UnitDiagnostic, UnitDraft, UnitFault, UnitSite, UnitStage,
};
use std::{collections::BTreeMap, ops::Range};
use toml_edit::{ImDocument, Item, TableLike, Value};

pub(crate) struct Node {
    pub site: UnitSite,
    pub(crate) kind: ShapeKind,
    pub(crate) string: Option<String>,
    pub(crate) fields: Vec<(String, UnitSite, Node)>,
    pub(crate) elements: Vec<Node>,
}
fn anchor(span: Option<Range<usize>>) -> SourceAnchor {
    span.map_or(SourceAnchor::WholeFile, |r| {
        SourceAnchor::Bytes(r.start, r.end)
    })
}
fn site(file: &str) -> UnitSite {
    UnitSite {
        file: file.to_owned(),
        path: vec![],
        anchor: SourceAnchor::WholeFile,
        decoded: None,
        occurrence: None,
    }
}
fn error(rule: RuleId, stage: UnitStage, site: UnitSite, fault: UnitFault) -> Box<UnitDiagnostic> {
    Box::new(UnitDiagnostic {
        rule,
        stage,
        unit: None,
        language: None,
        site,
        fault,
    })
}
pub(crate) fn encoding(source: &UnitSource) -> Result<&str, Box<UnitDiagnostic>> {
    let mut location = site(&source.file);
    let text = std::str::from_utf8(&source.bytes).map_err(|e| {
        let start = e.valid_up_to();
        location.anchor = SourceAnchor::Bytes(
            start,
            e.error_len().map_or(source.bytes.len(), |n| start + n),
        );
        error(
            RuleId::M001,
            UnitStage::Encoding,
            location.clone(),
            UnitFault::InvalidUtf8,
        )
    })?;
    if text.starts_with('\u{feff}') {
        location.anchor = SourceAnchor::Bytes(0, 3);
        return Err(error(
            RuleId::M001,
            UnitStage::Encoding,
            location,
            UnitFault::Bom,
        ));
    }
    Ok(text)
}
pub(crate) fn parse(file: &str, text: &str) -> Result<Node, Box<UnitDiagnostic>> {
    let document = ImDocument::parse(text).map_err(|e| {
        let mut location = site(file);
        location.anchor = e.span().map_or(SourceAnchor::Unavailable, |r| {
            SourceAnchor::Bytes(r.start, r.end)
        });
        error(
            RuleId::M002,
            UnitStage::Syntax,
            location,
            UnitFault::TomlSyntax {
                message: e.message().to_owned(),
            },
        )
    })?;
    Ok(table(document.as_table(), site(file)))
}
fn table(table: &dyn TableLike, location: UnitSite) -> Node {
    let fields = table
        .iter()
        .map(|(key, value)| {
            let mut key_site = location.child(key);
            key_site.anchor = anchor(table.key(key).and_then(toml_edit::Key::span));
            (key.to_owned(), key_site, node(value, location.child(key)))
        })
        .collect();
    Node {
        site: location,
        kind: ShapeKind::Table,
        string: None,
        fields,
        elements: vec![],
    }
}
fn node(item: &Item, mut location: UnitSite) -> Node {
    location.anchor = anchor(item.span());
    match item {
        Item::Table(t) => table(t, location),
        Item::Value(v) => value(v, location),
        Item::ArrayOfTables(tables) => {
            let elements = tables
                .iter()
                .enumerate()
                .map(|(index, t)| {
                    let mut child = location.clone();
                    child.path.push(PathPart::Index(index));
                    child.anchor = anchor(t.span());
                    table(t, child)
                })
                .collect();
            Node {
                site: location,
                kind: ShapeKind::Array,
                string: None,
                fields: vec![],
                elements,
            }
        }
        Item::None => unreachable!("immutable parsed documents have no empty items"),
    }
}
fn value(value: &Value, mut location: UnitSite) -> Node {
    location.anchor = anchor(value.span());
    if let Value::InlineTable(t) = value {
        return table(t, location);
    }
    let kind = match value {
        Value::String(_) => ShapeKind::String,
        Value::Integer(_) => ShapeKind::Integer,
        Value::Float(_) => ShapeKind::Float,
        Value::Boolean(_) => ShapeKind::Boolean,
        Value::Datetime(_) => ShapeKind::Datetime,
        Value::Array(_) => ShapeKind::Array,
        Value::InlineTable(_) => unreachable!("handled above"),
    };
    let elements = value.as_array().map_or_else(Vec::new, |array| {
        array
            .iter()
            .enumerate()
            .map(|(index, v)| {
                let mut child = location.clone();
                child.path.push(PathPart::Index(index));
                self::value(v, child)
            })
            .collect()
    });
    Node {
        site: location,
        kind,
        string: value.as_str().map(str::to_owned),
        fields: vec![],
        elements,
    }
}
fn first_float(node: &Node) -> Option<&Node> {
    let own = (node.kind == ShapeKind::Float).then_some(node);
    own.into_iter()
        .chain(node.fields.iter().filter_map(|(_, _, v)| first_float(v)))
        .chain(node.elements.iter().filter_map(first_float))
        .min_by_key(|n| match n.site.anchor {
            SourceAnchor::Bytes(a, _) => a,
            _ => usize::MAX,
        })
}
pub(crate) fn raw<'a>(node: &Node, text: &'a str) -> &'a str {
    match node.site.anchor {
        SourceAnchor::Bytes(a, b) => &text[a..b],
        _ => "",
    }
}
pub(crate) fn numeric(tree: &Node, text: &str) -> Result<(), Box<UnitDiagnostic>> {
    if let Some(node) = first_float(tree) {
        Err(error(
            RuleId::M003,
            UnitStage::Numeric,
            node.site.clone(),
            UnitFault::NonIntegerNumber {
                token: raw(node, text).to_owned(),
            },
        ))
    } else {
        Ok(())
    }
}
pub(crate) fn typed(node: &Node, expected: ShapeKind) -> Result<&Node, Box<UnitDiagnostic>> {
    if node.kind == expected {
        Ok(node)
    } else {
        Err(error(
            RuleId::M004,
            UnitStage::Shape,
            node.site.clone(),
            UnitFault::WrongType {
                expected,
                actual: node.kind,
            },
        ))
    }
}
pub(crate) fn fixed<'a>(
    node: &'a Node,
    keys: &[&str],
) -> Result<Vec<&'a Node>, Box<UnitDiagnostic>> {
    typed(node, ShapeKind::Table)?;
    let mut unknown: Vec<_> = node
        .fields
        .iter()
        .filter(|(key, _, _)| !keys.contains(&key.as_str()))
        .collect();
    unknown.sort_by_key(|(_, site, _)| match site.anchor {
        SourceAnchor::Bytes(a, _) => a,
        _ => usize::MAX,
    });
    if let Some((key, site, _)) = unknown.first() {
        return Err(error(
            RuleId::M004,
            UnitStage::Shape,
            site.clone(),
            UnitFault::UnknownField {
                name: key.clone(),
                expected: keys.iter().map(|k| (*k).to_owned()).collect(),
            },
        ));
    }
    keys.iter()
        .map(|key| {
            node.fields
                .iter()
                .find(|(k, _, _)| k == key)
                .map(|(_, _, n)| n)
                .ok_or_else(|| {
                    error(
                        RuleId::M004,
                        UnitStage::Shape,
                        node.site.parent(),
                        UnitFault::MissingField {
                            name: (*key).to_owned(),
                        },
                    )
                })
        })
        .collect()
}
pub(crate) fn string(node: &Node) -> Result<LocatedString, Box<UnitDiagnostic>> {
    typed(node, ShapeKind::String)?;
    Ok(LocatedString {
        value: node.string.clone().unwrap_or_default(),
        site: node.site.clone(),
    })
}
fn dynamic(node: &Node) -> Result<Vec<&(String, UnitSite, Node)>, Box<UnitDiagnostic>> {
    typed(node, ShapeKind::Table)?;
    let mut fields: Vec<_> = node.fields.iter().collect();
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(fields)
}
fn entries(node: &Node) -> Result<BTreeMap<String, LocatedEntryDraft>, Box<UnitDiagnostic>> {
    dynamic(node)?
        .into_iter()
        .map(|(key, site, value)| {
            Ok((
                key.clone(),
                LocatedEntryDraft {
                    key_site: site.clone(),
                    value: string(value)?,
                },
            ))
        })
        .collect()
}
pub(super) enum Draft {
    Documents(DocumentDraft),
    Unit(Box<UnitDraft>),
}
fn shape_inner(tree: &Node, text: &str, declarations: bool) -> Result<Draft, Box<UnitDiagnostic>> {
    let keys = if declarations {
        ["schema", "documents"]
    } else {
        ["schema", "unit"]
    };
    let root = fixed(tree, &keys)?;
    typed(root[0], ShapeKind::Integer)?;
    if raw(root[0], text) != "1" {
        return Err(error(
            RuleId::M003,
            UnitStage::Shape,
            root[0].site.clone(),
            UnitFault::SchemaValue {
                expected: "1".to_owned(),
                actual: raw(root[0], text).to_owned(),
            },
        ));
    }
    if declarations {
        let mut documents = BTreeMap::new();
        for (name, key_site, table) in dynamic(root[1])? {
            let fields = fixed(table, &["required-languages"])?;
            let array = typed(fields[0], ShapeKind::Array)?;
            let required_languages = array
                .elements
                .iter()
                .map(string)
                .collect::<Result<Vec<_>, _>>()?;
            documents.insert(
                name.clone(),
                DocumentEntryDraft {
                    key_site: key_site.clone(),
                    table_site: table.site.clone(),
                    languages_site: array.site.clone(),
                    required_languages,
                },
            );
        }
        Ok(Draft::Documents(DocumentDraft {
            documents_site: root[1].site.clone(),
            documents,
        }))
    } else {
        let fields = fixed(root[1], &["id", "document", "math", "texts"])?;
        Ok(Draft::Unit(Box::new(UnitDraft {
            id: string(fields[0])?,
            document: string(fields[1])?,
            table_site: root[1].site.clone(),
            math_site: fields[2].site.clone(),
            texts_site: fields[3].site.clone(),
            math: entries(fields[2])?,
            texts: entries(fields[3])?,
        })))
    }
}
pub(super) fn shape(
    tree: &Node,
    text: &str,
    declarations: bool,
) -> Result<Draft, Box<UnitDiagnostic>> {
    shape_inner(tree, text, declarations).map_err(|mut fault| {
        if !declarations {
            fault.unit = tree
                .fields
                .iter()
                .find(|(k, _, _)| k == "unit")
                .and_then(|(_, _, u)| {
                    u.fields
                        .iter()
                        .find(|(k, _, _)| k == "id")
                        .and_then(|(_, _, id)| id.string.clone())
                });
            if let [
                PathPart::Key(unit),
                PathPart::Key(texts),
                PathPart::Key(language),
                ..,
            ] = fault.site.path.as_slice()
                && unit == "unit"
                && texts == "texts"
            {
                fault.language = Some(language.clone());
            }
        }
        fault
    })
}
