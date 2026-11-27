//! Strict native input for a bounded ordered corpus slot.

use crate::{UnitSource, units::toml};
use ftoo_manifest::{
    RuleId, ShapeKind, UnitDiagnostic, UnitErrors, UnitFault, UnitSite, UnitStage,
    corpus::{
        ResultGradeCode, ResultKind, SlotBlock, SlotDraft, SlotPaperMint, SlotRun, SlotStyle,
    },
};
use toml::Node;
#[cfg(test)]
mod tests;

fn field<'a>(node: &'a Node, name: &str) -> Result<&'a Node, Box<UnitDiagnostic>> {
    node.fields
        .iter()
        .find(|(key, _, _)| key == name)
        .map(|(_, _, value)| value)
        .ok_or_else(|| {
            Box::new(UnitDiagnostic {
                rule: RuleId::M004,
                stage: UnitStage::Shape,
                unit: None,
                language: None,
                site: node.site.child(name),
                fault: UnitFault::MissingField { name: name.into() },
            })
        })
}

fn token(site: UnitSite, value: String, expected: &[&str]) -> Box<UnitDiagnostic> {
    Box::new(UnitDiagnostic {
        rule: RuleId::M004,
        stage: UnitStage::Shape,
        unit: None,
        language: None,
        site,
        fault: UnitFault::UnknownToken {
            value,
            expected: expected.iter().map(|value| (*value).into()).collect(),
        },
    })
}

fn string(node: &Node) -> Result<String, Box<UnitDiagnostic>> {
    Ok(toml::string(node)?.value)
}

fn boolean(node: &Node, text: &str) -> Result<bool, Box<UnitDiagnostic>> {
    toml::typed(node, ShapeKind::Boolean)?;
    Ok(toml::raw(node, text) == "true")
}

fn runs(node: &Node) -> Result<Vec<SlotRun>, Box<UnitDiagnostic>> {
    toml::typed(node, ShapeKind::Array)?
        .elements
        .iter()
        .map(run)
        .collect()
}

fn run(node: &Node) -> Result<SlotRun, Box<UnitDiagnostic>> {
    toml::typed(node, ShapeKind::Table)?;
    let discriminator = field(node, "kind")?;
    let kind = string(discriminator)?;
    match kind.as_str() {
        "text" => {
            let fields = toml::fixed(node, &["kind", "decoded"])?;
            Ok(SlotRun::Text(string(fields[1])?))
        }
        "space" => {
            let fields = toml::fixed(node, &["kind", "space"])?;
            let value = string(fields[1])?;
            if value != "en-space" {
                return Err(token(fields[1].site.clone(), value, &["en-space"]));
            }
            Ok(SlotRun::EnSpace)
        }
        "style" => {
            let fields = toml::fixed(node, &["kind", "style", "children"])?;
            let value = string(fields[1])?;
            let kind = match value.as_str() {
                "strong" => SlotStyle::Strong,
                "emphasis" => SlotStyle::Emphasis,
                _ => {
                    return Err(token(
                        fields[1].site.clone(),
                        value,
                        &["strong", "emphasis"],
                    ));
                }
            };
            Ok(SlotRun::Style {
                kind,
                children: runs(fields[2])?,
            })
        }
        _ => Err(token(
            discriminator.site.clone(),
            kind,
            &["text", "space", "style"],
        )),
    }
}

fn block(node: &Node, text: &str) -> Result<SlotBlock, Box<UnitDiagnostic>> {
    toml::typed(node, ShapeKind::Table)?;
    let discriminator = field(node, "kind")?;
    let kind = string(discriminator)?;
    match kind.as_str() {
        "heading" => {
            let fields = toml::fixed(node, &["kind", "level", "star", "title", "label"])?;
            let level = string(fields[1])?;
            if level != "section" {
                return Err(token(fields[1].site.clone(), level, &["section"]));
            }
            Ok(SlotBlock::Heading {
                level,
                star: boolean(fields[2], text)?,
                title: runs(fields[3])?,
                label: string(fields[4])?,
            })
        }
        "paragraph" => {
            let fields = toml::fixed(node, &["kind", "runs"])?;
            Ok(SlotBlock::Paragraph {
                runs: runs(fields[1])?,
            })
        }
        "use" => {
            let fields = toml::fixed(node, &["kind", "unit", "role"])?;
            Ok(SlotBlock::Use {
                unit: string(fields[1])?,
                role: string(fields[2])?,
            })
        }
        "result" => {
            let mut keys = vec!["kind", "result-kind", "target", "key", "children"];
            if node.fields.iter().any(|(name, _, _)| name == "grade") {
                keys.push("grade");
            }
            if node.fields.iter().any(|(name, _, _)| name == "grade-gap") {
                keys.push("grade-gap");
            }
            toml::fixed(node, &keys)?;
            let kind_node = field(node, "result-kind")?;
            let kind_name = string(kind_node)?;
            let result_kind = ResultKind::from_name(&kind_name).ok_or_else(|| {
                token(
                    kind_node.site.clone(),
                    kind_name,
                    &["insight", "remark", "definition"],
                )
            })?;
            let grade = node
                .fields
                .iter()
                .find(|(name, _, _)| name == "grade")
                .map(|(_, _, value)| {
                    let name = string(value)?;
                    ResultGradeCode::from_name(&name).ok_or_else(|| {
                        token(
                            value.site.clone(),
                            name,
                            &["nec", "cont-anch", "feas", "cont-comm", "cont"],
                        )
                    })
                })
                .transpose()?;
            let grade_gap = node
                .fields
                .iter()
                .find(|(name, _, _)| name == "grade-gap")
                .map(|(_, _, value)| boolean(value, text))
                .transpose()?
                .unwrap_or(false);
            let children = toml::typed(field(node, "children")?, ShapeKind::Array)?
                .elements
                .iter()
                .map(|child| match block(child, text)? {
                    SlotBlock::Paragraph { runs } => Ok(runs),
                    _ => Err(token(
                        child.site.clone(),
                        "result child".into(),
                        &["paragraph"],
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(SlotBlock::Result {
                kind: result_kind,
                target: string(field(node, "target")?)?,
                key: string(field(node, "key")?)?,
                grade,
                grade_gap,
                children,
            })
        }
        _ => Err(token(
            discriminator.site.clone(),
            kind,
            &["heading", "paragraph", "use", "result"],
        )),
    }
}

fn parse(source: &UnitSource) -> Result<SlotDraft, Box<UnitDiagnostic>> {
    let text = toml::encoding(source)?;
    let tree = toml::parse(&source.file, text)?;
    toml::numeric(&tree, text)?;
    let root = toml::fixed(&tree, &["schema", "unit"])?;
    toml::typed(root[0], ShapeKind::Integer)?;
    if toml::raw(root[0], text) != "1" {
        return Err(Box::new(UnitDiagnostic {
            rule: RuleId::M003,
            stage: UnitStage::Shape,
            unit: None,
            language: None,
            site: root[0].site.clone(),
            fault: UnitFault::SchemaValue {
                expected: "1".into(),
                actual: toml::raw(root[0], text).into(),
            },
        }));
    }
    let mut keys = vec!["id", "document", "composition", "language", "blocks"];
    if root[1]
        .fields
        .iter()
        .any(|(key, _, _)| key == "documentary-targets")
    {
        keys.push("documentary-targets");
    }
    let fields = toml::fixed(root[1], &keys)?;
    let blocks = toml::typed(fields[4], ShapeKind::Array)?
        .elements
        .iter()
        .map(|node| block(node, text))
        .collect::<Result<Vec<_>, _>>()?;
    let paper_mint = if let [SlotBlock::Result { target, key, .. }] = blocks.as_slice() {
        let targets = field(root[1], "documentary-targets")?;
        toml::typed(targets, ShapeKind::Table)?;
        let [(_, _, value)] = targets.fields.as_slice() else {
            return Err(token(
                targets.site.clone(),
                "documentary-targets".into(),
                &["one target"],
            ));
        };
        let declared = field(targets, target)?;
        let declared_key = string(declared)?;
        if !std::ptr::eq(value, declared) || declared_key != *key {
            return Err(token(declared.site.clone(), declared_key, &[key]));
        }
        let result_node = &toml::typed(fields[4], ShapeKind::Array)?.elements[0];
        Some(SlotPaperMint {
            selector: target.clone(),
            key: key.clone(),
            declaration: declared.site.clone(),
            placement: field(result_node, "target")?.site.clone(),
        })
    } else {
        None
    };
    Ok(SlotDraft {
        schema: 1,
        id: string(fields[0])?,
        document: string(fields[1])?,
        composition: string(fields[2])?,
        language: string(fields[3])?,
        paper_mint,
        blocks,
    })
}

/// Parse one native corpus slot with exclusive fields and closed block and run kinds.
/// # Errors
/// Refuses invalid UTF-8, TOML, schema, field shape, block kind, style or spacing kind.
/// # Panics
/// The nonempty diagnostic conversion cannot fail because parse supplies one diagnostic.
pub fn parse_slot(source: &UnitSource) -> Result<SlotDraft, UnitErrors> {
    parse(source)
        .map_err(|error| UnitErrors::from_diagnostics(vec![*error]).expect("one diagnostic"))
}
