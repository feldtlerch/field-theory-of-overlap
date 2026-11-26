//! Strict native input for a bounded ordered corpus slot.

use crate::{UnitSource, units::toml};
use ftoo_manifest::{
    RuleId, ShapeKind, UnitDiagnostic, UnitErrors, UnitFault, UnitSite, UnitStage,
    corpus::{SlotBlock, SlotDraft, SlotRun, SlotStyle},
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
        _ => Err(token(
            discriminator.site.clone(),
            kind,
            &["heading", "paragraph", "use"],
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
    let fields = toml::fixed(
        root[1],
        &["id", "document", "composition", "language", "blocks"],
    )?;
    let blocks = toml::typed(fields[4], ShapeKind::Array)?
        .elements
        .iter()
        .map(|node| block(node, text))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SlotDraft {
        schema: 1,
        id: string(fields[0])?,
        document: string(fields[1])?,
        composition: string(fields[2])?,
        language: string(fields[3])?,
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
