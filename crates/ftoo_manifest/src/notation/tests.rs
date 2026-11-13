// Keep independent complete fault payloads and Display oracles beside each fixture.
#![allow(clippy::too_many_lines)]
use super::*;
use crate::{RuleId, addressing, seed};
use catalogue::{call, literal, primitive};

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn expression_site(fields: &[&str]) -> NotationSite {
    NotationSite::Expression {
        path: fields
            .iter()
            .map(|s| NotationPathPart::Field((*s).into()))
            .collect(),
    }
}
fn registry_site(id: DefinitionId, index: usize, fields: &[&str]) -> NotationSite {
    NotationSite::Registry {
        file: "crates/ftoo_manifest/src/notation/catalogue.rs".into(),
        entry: RegistryEntry::Definition(id),
        index,
        path: fields
            .iter()
            .map(|s| NotationPathPart::Field((*s).into()))
            .collect(),
    }
}
fn primitive_site(id: PrimitiveId, index: usize, fields: &[&str]) -> NotationSite {
    NotationSite::Registry {
        file: "crates/ftoo_manifest/src/notation/primitives.rs".into(),
        entry: RegistryEntry::Primitive(id),
        index,
        path: fields
            .iter()
            .map(|s| NotationPathPart::Field((*s).into()))
            .collect(),
    }
}
fn diag(
    rule: RuleId,
    stage: NotationStage,
    site: NotationSite,
    fault: NotationFault,
) -> NotationDiagnostic {
    NotationDiagnostic {
        rule,
        stage,
        site,
        fault,
    }
}
fn exact(error: &NotationErrors, expected: &[NotationDiagnostic], display: &str) {
    assert_eq!(error.diagnostics(), expected);
    assert_eq!(error.to_string(), display);
    let lines: Vec<_> = display.lines().collect();
    for (diagnostic, line) in error.diagnostics().iter().zip(lines) {
        assert_eq!(diagnostic.to_string(), line);
    }
}
fn body_mut(d: &mut DefinitionSpec) -> &mut ExprDraft {
    match &mut d.kind {
        DefinitionKind::Canonical { body, .. } | DefinitionKind::Constructor { body } => body,
        DefinitionKind::Alias { .. } => panic!("fixture needs a body"),
    }
}
fn backing_mut(d: &mut DefinitionSpec) -> &mut Option<NodeId> {
    match &mut d.kind {
        DefinitionKind::Canonical { backing, .. } | DefinitionKind::Alias { backing, .. } => {
            backing
        }
        DefinitionKind::Constructor { .. } => panic!("fixture needs binding"),
    }
}

#[test]
fn n01_primitive_inventory_and_signatures() -> TestResult {
    use PrimitiveId::{
        Add, Alpha, Beta, Dimension, Equal, Fraction, Juxtapose, Multiply, Negate, Parentheses,
        Subscript, Superscript,
    };
    let graph = seed()?;
    let book = addressing(&graph)?;
    let expected = [
        (Alpha, "alpha", 0),
        (Beta, "beta", 0),
        (Fraction, "fraction", 2),
        (Subscript, "subscript", 2),
        (Superscript, "superscript", 2),
        (Add, "add", 2),
        (Equal, "equal", 2),
        (Negate, "negate", 1),
        (Multiply, "multiply", 2),
        (Juxtapose, "juxtapose", 2),
        (Parentheses, "parentheses", 1),
        (Dimension, "dimension", 1),
    ];
    let n = notation(&book)?;
    assert_eq!(
        n.primitives()
            .iter()
            .map(|p| (p.id(), p.token(), p.signature().arity))
            .collect::<Vec<_>>(),
        expected
    );
    for (id, _, arity) in expected {
        assert_eq!(id.operation().arity(), arity);
        assert_eq!(
            Signature { arity }.parameters().collect::<Vec<_>>(),
            vec![MathSort::MathExpr; usize::from(arity)]
        );
        for actual in [
            usize::from(arity).checked_sub(1),
            Some(usize::from(arity) + 1),
        ]
        .into_iter()
        .flatten()
        {
            let error =
                check_expression(&n, primitive(id, vec![literal("x"); actual])).unwrap_err();
            exact(
                &error,
                &[diag(
                    RuleId::M047,
                    NotationStage::Ast,
                    expression_site(&["arguments"]),
                    NotationFault::CallArity {
                        callee: Callee::Primitive(id),
                        expected: arity,
                        actual,
                    },
                )],
                &format!(
                    "M047 Ast at Expression {{ path: [Field(\"arguments\")] }}: CallArity {{ callee: Primitive({id:?}), expected: {arity}, actual: {actual} }}\n"
                ),
            );
            check_expression(&n, primitive(id, vec![literal("x"); usize::from(arity)]))?;
        }
    }
    let mut p = primitives::catalogue();
    p.remove(0);
    exact(
        &construct(&book, p, catalogue::catalogue()).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            primitive_site(Alpha, 11, &["id"]),
            NotationFault::CatalogMultiplicity {
                entry: RegistryEntry::Primitive(Alpha),
                expected: 1,
                actual: 0,
                positions: vec![],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Alpha), index: 11, path: [Field(\"id\")] }: CatalogMultiplicity { entry: Primitive(Alpha), expected: 1, actual: 0, positions: [] }\n",
    );
    let mut p = primitives::catalogue();
    let mut duplicate = p[0].clone();
    duplicate.token = "other";
    p.push(duplicate);
    exact(
        &construct(&book, p, catalogue::catalogue()).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            primitive_site(Alpha, 0, &["id"]),
            NotationFault::CatalogMultiplicity {
                entry: RegistryEntry::Primitive(Alpha),
                expected: 1,
                actual: 2,
                positions: vec![0, 12],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Alpha), index: 0, path: [Field(\"id\")] }: CatalogMultiplicity { entry: Primitive(Alpha), expected: 1, actual: 2, positions: [0, 12] }\n",
    );
    let mut p = primitives::catalogue();
    p[1].token = "alpha";
    exact(
        &construct(&book, p, catalogue::catalogue()).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            primitive_site(Alpha, 0, &["token"]),
            NotationFault::DuplicateToken {
                token: "alpha".into(),
                entries: vec![
                    RegistryEntry::Primitive(Alpha),
                    RegistryEntry::Primitive(Beta),
                ],
                positions: vec![0, 1],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Alpha), index: 0, path: [Field(\"token\")] }: DuplicateToken { token: \"alpha\", entries: [Primitive(Alpha), Primitive(Beta)], positions: [0, 1] }\n",
    );
    let mut p = primitives::catalogue();
    p[2].signature.arity = 1;
    exact(
        &construct(&book, p, catalogue::catalogue()).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            primitive_site(Fraction, 2, &["signature", "arity"]),
            NotationFault::PrimitiveSignature {
                primitive: Fraction,
                expected: 2,
                actual: 1,
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Fraction), index: 2, path: [Field(\"signature\"), Field(\"arity\")] }: PrimitiveSignature { primitive: Fraction, expected: 2, actual: 1 }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n02_literal_grammar_exact_offsets() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    for value in ["a", "Z", "1", "45", "000"] {
        check_expression(&n, literal(value))?;
    }
    for (value, reason, byte, scalar) in [
        ("", LiteralReason::Empty, 0, None),
        ("ab", LiteralReason::MultipleLetters, 1, Some("b")),
        ("a1", LiteralReason::ForbiddenScalar, 1, Some("1")),
        ("1a", LiteralReason::ForbiddenScalar, 1, Some("a")),
        ("αx", LiteralReason::ForbiddenScalar, 0, Some("α")),
        (
            "a\u{301}",
            LiteralReason::ForbiddenScalar,
            1,
            Some("\u{301}"),
        ),
        ("1.2", LiteralReason::ForbiddenScalar, 1, Some(".")),
    ] {
        let scalar = scalar.map(str::to_string);
        exact(
            &check_expression(&n, literal(value)).unwrap_err(),
            &[diag(
                RuleId::M047,
                NotationStage::Ast,
                expression_site(&["value"]),
                NotationFault::InvalidLiteral {
                    value: value.into(),
                    reason,
                    byte,
                    scalar: scalar.clone(),
                },
            )],
            &format!(
                "M047 Ast at Expression {{ path: [Field(\"value\")] }}: InvalidLiteral {{ value: {value:?}, reason: {reason:?}, byte: {byte}, scalar: {scalar:?} }}\n"
            ),
        );
        check_expression(&n, literal("x"))?;
    }
    for scalar in [
        "\\", "{", "}", "_", "^", "#", "%", "$", "&", "<", ">", "+", "-", ".", " ", "\t", "\n",
        "\r", "\0", "\u{7f}", "\u{a0}", "\u{2003}", "\u{85}", "β", "é", "\u{301}",
    ] {
        exact(
            &check_expression(&n, literal(scalar)).unwrap_err(),
            &[diag(
                RuleId::M047,
                NotationStage::Ast,
                expression_site(&["value"]),
                NotationFault::InvalidLiteral {
                    value: scalar.into(),
                    reason: LiteralReason::ForbiddenScalar,
                    byte: 0,
                    scalar: Some(scalar.into()),
                },
            )],
            &format!(
                "M047 Ast at Expression {{ path: [Field(\"value\")] }}: InvalidLiteral {{ value: {scalar:?}, reason: ForbiddenScalar, byte: 0, scalar: Some({scalar:?}) }}\n"
            ),
        );
        check_expression(&n, literal("0"))?;
    }
    check_expression(&n, literal(&"0".repeat(256)))?;
    exact(
        &check_expression(&n, literal(&"0".repeat(257))).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Ast,
            expression_site(&["value"]),
            NotationFault::NotationLimit {
                resource: NotationResource::LiteralBytes,
                limit: 256,
                actual: 257,
            },
        )],
        "M047 Ast at Expression { path: [Field(\"value\")] }: NotationLimit { resource: LiteralBytes, limit: 256, actual: 257 }\n",
    );
    Ok(())
}

#[test]
fn n03_stem_grammar_collision_and_category_separation() -> TestResult {
    use DefinitionId::{CoefficientOne, CoefficientThree};
    let graph = seed()?;
    let book = addressing(&graph)?;
    for value in [
        "",
        "x",
        "X",
        "XML",
        "Coefficient1",
        "Coefficient_One",
        "Élan",
        "Bad\\Name",
        "Bad{Name}",
    ] {
        let mut d = catalogue::catalogue();
        d[0].stem = value;
        exact(
            &construct(&book, primitives::catalogue(), d).unwrap_err(),
            &[diag(
                RuleId::M048,
                NotationStage::Catalogue,
                registry_site(CoefficientOne, 0, &["stem"]),
                NotationFault::InvalidStem {
                    value: value.into(),
                },
            )],
            &format!(
                "M048 Catalogue at Registry {{ file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"stem\")] }}: InvalidStem {{ value: {value:?} }}\n"
            ),
        );
        notation(&book)?;
    }
    let mut d = catalogue::catalogue();
    d[1].stem = "CoefficientOne";
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Catalogue,
            registry_site(CoefficientOne, 0, &["stem"]),
            NotationFault::CommandCollision {
                command: "\\FtooQCoefficientOne".into(),
                definitions: vec![CoefficientOne, CoefficientThree],
            },
        )],
        "M048 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"stem\")] }: CommandCollision { command: \"\\\\FtooQCoefficientOne\", definitions: [CoefficientOne, CoefficientThree] }\n",
    );
    let mut d = catalogue::catalogue();
    d[4].stem = "CoefficientOne";
    let n = construct(&book, primitives::catalogue(), d)?;
    assert_eq!(
        n.definition(DefinitionId::FirstCoefficient).command(),
        "\\FtooACoefficientOne"
    );
    assert_eq!(
        n.definition(CoefficientOne).command(),
        "\\FtooQCoefficientOne"
    );
    let mut d = catalogue::catalogue();
    d.swap(0, 6);
    d[6].stem = "Bad1";
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Catalogue,
            registry_site(CoefficientOne, 6, &["stem"]),
            NotationFault::InvalidStem {
                value: "Bad1".into(),
            },
        )],
        "M048 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 6, path: [Field(\"stem\")] }: InvalidStem { value: \"Bad1\" }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n03_definition_shapes() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    for (i, id, arity, reason) in [
        (
            0,
            DefinitionId::CoefficientOne,
            1,
            DefinitionShapeReason::CanonicalMustBeNullary,
        ),
        (
            4,
            DefinitionId::FirstCoefficient,
            1,
            DefinitionShapeReason::AliasMustBeNullary,
        ),
        (
            5,
            DefinitionId::Dimension,
            0,
            DefinitionShapeReason::ConstructorArity,
        ),
        (
            5,
            DefinitionId::Dimension,
            10,
            DefinitionShapeReason::ConstructorArity,
        ),
    ] {
        let mut d = catalogue::catalogue();
        d[i].signature.arity = arity;
        exact(
            &construct(&book, primitives::catalogue(), d).unwrap_err(),
            &[diag(
                RuleId::M048,
                NotationStage::Catalogue,
                registry_site(id, i, &["signature", "arity"]),
                NotationFault::DefinitionShape {
                    definition: id,
                    reason: reason.clone(),
                },
            )],
            &format!(
                "M048 Catalogue at Registry {{ file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition({id:?}), index: {i}, path: [Field(\"signature\"), Field(\"arity\")] }}: DefinitionShape {{ definition: {id:?}, reason: {reason:?} }}\n"
            ),
        );
        notation(&book)?;
    }
    let mut d = catalogue::catalogue();
    *backing_mut(&mut d[0]) = Some(NodeId::LangevinCoefficientThree);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Catalogue,
            registry_site(DefinitionId::CoefficientOne, 0, &["backing"]),
            NotationFault::DefinitionShape {
                definition: DefinitionId::CoefficientOne,
                reason: DefinitionShapeReason::BackingObjectMismatch {
                    expected: NodeId::LangevinCoefficientOne,
                    actual: NodeId::LangevinCoefficientThree,
                },
            },
        )],
        "M048 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"backing\")] }: DefinitionShape { definition: CoefficientOne, reason: BackingObjectMismatch { expected: LangevinCoefficientOne, actual: LangevinCoefficientThree } }\n",
    );
    Ok(())
}

#[test]
fn n03_alias_targets_missing_entries_and_canonical_coverage() -> TestResult {
    use DefinitionId::{CoefficientOne, Dimension, FirstCoefficient};
    use NodeId::{LangevinCoefficientOne as One, LangevinCoefficientThree as Three};
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    d.remove(0);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            registry_site(CoefficientOne, 6, &["id"]),
            NotationFault::CatalogMultiplicity {
                entry: RegistryEntry::Definition(CoefficientOne),
                expected: 1,
                actual: 0,
                positions: vec![],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 6, path: [Field(\"id\")] }: CatalogMultiplicity { entry: Definition(CoefficientOne), expected: 1, actual: 0, positions: [] }\n",
    );
    for (target, reason) in [
        (Dimension, AliasTargetReason::ConstructorTarget),
        (
            DefinitionId::CoefficientThree,
            AliasTargetReason::ObjectMismatch {
                expected: One,
                actual: Three,
            },
        ),
    ] {
        let mut d = catalogue::catalogue();
        d[4].kind = DefinitionKind::Alias {
            object: One,
            backing: Some(One),
            target,
        };
        exact(
            &construct(&book, primitives::catalogue(), d).unwrap_err(),
            &[diag(
                RuleId::M048,
                NotationStage::References,
                registry_site(FirstCoefficient, 4, &["target"]),
                NotationFault::AliasTarget {
                    alias: FirstCoefficient,
                    target,
                    reason: reason.clone(),
                },
            )],
            &format!(
                "M048 References at Registry {{ file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(FirstCoefficient), index: 4, path: [Field(\"target\")] }}: AliasTarget {{ alias: FirstCoefficient, target: {target:?}, reason: {reason:?} }}\n"
            ),
        );
        notation(&book)?;
    }
    let mut d = catalogue::catalogue();
    d[4].kind = DefinitionKind::Canonical {
        object: One,
        backing: Some(One),
        body: literal("x"),
    };
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::References,
            registry_site(CoefficientOne, 0, &["object"]),
            NotationFault::CanonicalMultiplicity {
                object: One,
                expected: 1,
                actual: 2,
                definitions: vec![CoefficientOne, FirstCoefficient],
                positions: vec![0, 4],
            },
        )],
        "M048 References at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"object\")] }: CanonicalMultiplicity { object: LangevinCoefficientOne, expected: 1, actual: 2, definitions: [CoefficientOne, FirstCoefficient], positions: [0, 4] }\n",
    );
    let mut d = catalogue::catalogue();
    d[0].kind = DefinitionKind::Alias {
        object: One,
        backing: Some(One),
        target: FirstCoefficient,
    };
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::References,
            NotationSite::Catalogue {
                file: "crates/ftoo_manifest/src/notation/catalogue.rs".into(),
            },
            NotationFault::CanonicalMultiplicity {
                object: One,
                expected: 1,
                actual: 0,
                definitions: vec![],
                positions: vec![],
            },
        )],
        "M048 References at Catalogue { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\" }: CanonicalMultiplicity { object: LangevinCoefficientOne, expected: 1, actual: 0, definitions: [], positions: [] }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n03_cycles_have_concrete_closed_paths() -> TestResult {
    use DefinitionId::{CoefficientOne, CoefficientThree, Dimension, FirstCoefficient, Product};
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[0]) = call(CoefficientOne, vec![]);
    let s = registry_site(CoefficientOne, 0, &["body", "callee"]);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Dependencies,
            s.clone(),
            NotationFault::DefinitionCycle {
                chain: vec![CoefficientOne, CoefficientOne],
                sites: vec![s],
            },
        )],
        "M048 Dependencies at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }: DefinitionCycle { chain: [CoefficientOne, CoefficientOne], sites: [Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }] }\n",
    );
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[0]) = call(CoefficientThree, vec![]);
    *body_mut(&mut d[1]) = call(CoefficientOne, vec![]);
    let s = registry_site(CoefficientOne, 0, &["body", "callee"]);
    let t = registry_site(CoefficientThree, 1, &["body", "callee"]);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Dependencies,
            s.clone(),
            NotationFault::DefinitionCycle {
                chain: vec![CoefficientOne, CoefficientThree, CoefficientOne],
                sites: vec![s, t],
            },
        )],
        "M048 Dependencies at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }: DefinitionCycle { chain: [CoefficientOne, CoefficientThree, CoefficientOne], sites: [Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }, Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientThree), index: 1, path: [Field(\"body\"), Field(\"callee\")] }] }\n",
    );
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[0]) = call(FirstCoefficient, vec![]);
    let s = registry_site(CoefficientOne, 0, &["body", "callee"]);
    let t = registry_site(FirstCoefficient, 4, &["target"]);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Dependencies,
            s.clone(),
            NotationFault::DefinitionCycle {
                chain: vec![CoefficientOne, FirstCoefficient, CoefficientOne],
                sites: vec![s, t],
            },
        )],
        "M048 Dependencies at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }: DefinitionCycle { chain: [CoefficientOne, FirstCoefficient, CoefficientOne], sites: [Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }, Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(FirstCoefficient), index: 4, path: [Field(\"target\")] }] }\n",
    );
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[5]) = call(Product, vec![ExprDraft::Slot { index: 0 }, literal("x")]);
    *body_mut(&mut d[6]) = call(
        Dimension,
        vec![primitive(
            PrimitiveId::Multiply,
            vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 1 }],
        )],
    );
    let s = registry_site(Dimension, 5, &["body", "callee"]);
    let t = registry_site(Product, 6, &["body", "callee"]);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Dependencies,
            s.clone(),
            NotationFault::DefinitionCycle {
                chain: vec![Dimension, Product, Dimension],
                sites: vec![s, t],
            },
        )],
        "M048 Dependencies at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Dimension), index: 5, path: [Field(\"body\"), Field(\"callee\")] }: DefinitionCycle { chain: [Dimension, Product, Dimension], sites: [Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Dimension), index: 5, path: [Field(\"body\"), Field(\"callee\")] }, Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Product), index: 6, path: [Field(\"body\"), Field(\"callee\")] }] }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n04_slots_and_all_arguments_are_checked() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    exact(
        &check_expression(&n, ExprDraft::Slot { index: 0 }).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Ast,
            expression_site(&["index"]),
            NotationFault::SlotOutOfScope { index: 0, arity: 0 },
        )],
        "M047 Ast at Expression { path: [Field(\"index\")] }: SlotOutOfScope { index: 0, arity: 0 }\n",
    );
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[0]) = ExprDraft::Slot { index: 0 };
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Ast,
            registry_site(DefinitionId::CoefficientOne, 0, &["body", "index"]),
            NotationFault::SlotOutOfScope { index: 0, arity: 0 },
        )],
        "M047 Ast at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"index\")] }: SlotOutOfScope { index: 0, arity: 0 }\n",
    );
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[6]) = ExprDraft::Slot { index: 2 };
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[
            diag(
                RuleId::M047,
                NotationStage::Ast,
                registry_site(DefinitionId::Product, 6, &["body"]),
                NotationFault::UnusedSlot { index: 0, arity: 2 },
            ),
            diag(
                RuleId::M047,
                NotationStage::Ast,
                registry_site(DefinitionId::Product, 6, &["body"]),
                NotationFault::UnusedSlot { index: 1, arity: 2 },
            ),
            diag(
                RuleId::M047,
                NotationStage::Ast,
                registry_site(DefinitionId::Product, 6, &["body", "index"]),
                NotationFault::SlotOutOfScope { index: 2, arity: 2 },
            ),
        ],
        "M047 Ast at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Product), index: 6, path: [Field(\"body\")] }: UnusedSlot { index: 0, arity: 2 }\nM047 Ast at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Product), index: 6, path: [Field(\"body\")] }: UnusedSlot { index: 1, arity: 2 }\nM047 Ast at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Product), index: 6, path: [Field(\"body\"), Field(\"index\")] }: SlotOutOfScope { index: 2, arity: 2 }\n",
    );
    let mut d = catalogue::catalogue();
    d[5].signature.arity = 9;
    *body_mut(&mut d[5]) = (0..9)
        .map(|index| ExprDraft::Slot { index })
        .reduce(|a, b| primitive(PrimitiveId::Add, vec![a, b]))
        .unwrap();
    let n9 = construct(&book, primitives::catalogue(), d)?;
    check_expression(&n9, call(DefinitionId::Dimension, vec![literal("x"); 9]))?;
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[5]) = primitive(
        PrimitiveId::Add,
        vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 0 }],
    );
    *body_mut(&mut d[6]) = primitive(
        PrimitiveId::Multiply,
        vec![
            call(DefinitionId::Dimension, vec![ExprDraft::Slot { index: 1 }]),
            ExprDraft::Slot { index: 0 },
        ],
    );
    let n = construct(&book, primitives::catalogue(), d)?;
    let e = check_expression(
        &n,
        call(
            DefinitionId::Product,
            vec![
                call(DefinitionId::CoefficientOne, vec![]),
                call(DefinitionId::CoefficientThree, vec![]),
            ],
        ),
    )?;
    assert_eq!(
        e.uses()
            .iter()
            .map(SymbolUse::definition)
            .collect::<Vec<_>>(),
        [
            DefinitionId::CoefficientOne,
            DefinitionId::CoefficientThree,
            DefinitionId::CoefficientThree
        ]
    );
    Ok(())
}

fn retained_draft() -> crate::ManifestDraft {
    let mut draft = crate::seed_draft();
    for node in &mut draft.nodes {
        node.maturity = crate::Maturity::Retained;
    }
    draft
}
#[test]
fn n05_every_own_binding_reason_from_real_graph() -> TestResult {
    use crate::{Awaiting, Blocker, CapabilityId, ClaimAudit, NodePayload};
    use NodeId::{LangevinCoefficientOne as One, LangevinCoefficientOneValue as Value};
    let graph = crate::validate(retained_draft())?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    assert_eq!(
        n.quantity_symbol(One)?.status().unwrap().tier(),
        SymbolTier::Authorised
    );
    assert_eq!(n.quantity_symbol(One)?.status().unwrap().reasons(), []);
    assert_eq!(
        n.object_symbol(Value)?.status().unwrap().reasons(),
        [PendingReason::ClaimDeferred {
            object: Value,
            blocker: Blocker::AuditRunRequired
        }]
    );
    for awaits in [
        Awaiting::AuditBinding,
        Awaiting::Capability(CapabilityId::ExactRationalLinearEquivalence),
        Awaiting::Capability(CapabilityId::SymbolicTrigonometry),
        Awaiting::Capability(CapabilityId::TrigonometricIntegration),
    ] {
        let mut draft = retained_draft();
        draft.coverage.slice.clear();
        draft.coverage.closed_world.audit = false;
        draft
            .budgets
            .retain(|b| b.producer.descriptor().claim != Value);
        for node in &mut draft.nodes {
            if node.id == Value
                && let NodePayload::Claim { audit, .. } = &mut node.payload
            {
                *audit = ClaimAudit::Awaiting(awaits);
            }
        }
        let graph = crate::validate(draft)?;
        let book = addressing(&graph)?;
        let n = notation(&book)?;
        let reason = PendingReason::ClaimDeferred {
            object: Value,
            blocker: Blocker::Awaiting(awaits),
        };
        assert_eq!(
            n.object_symbol(Value)?.status().unwrap().tier(),
            SymbolTier::Pending
        );
        assert_eq!(
            n.object_symbol(Value)?.status().unwrap().reasons(),
            [reason]
        );
    }
    let mut d = catalogue::catalogue();
    *backing_mut(&mut d[0]) = None;
    let n = construct(&book, primitives::catalogue(), d)?;
    let status = n.quantity_symbol(One)?.status().unwrap();
    assert_eq!(status.tier(), SymbolTier::Pending);
    assert_eq!(
        status.reasons(),
        [PendingReason::BackingUnbound { object: One }]
    );
    assert_eq!(
        status.reasons()[0].to_string(),
        "backing object def:langevin:coefficient-one is unbound"
    );
    let mut draft = retained_draft();
    draft.coverage.slice.clear();
    draft.coverage.closed_world.audit = false;
    draft.nodes.retain(|node| node.id != One);
    draft.edges.retain(|e| e.source != One && e.target != One);
    let graph = crate::validate(draft)?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    assert_eq!(
        n.definition(DefinitionId::CoefficientOne)
            .status()
            .unwrap()
            .reasons(),
        [PendingReason::BackingAbsent { object: One }]
    );
    assert_eq!(
        n.definition(DefinitionId::CoefficientOne)
            .status()
            .unwrap()
            .reasons()[0]
            .to_string(),
        "backing object def:langevin:coefficient-one is absent"
    );
    assert_eq!(
        n.object_symbol(One).unwrap_err(),
        SymbolLookupFault::AbsentObject { object: One }
    );
    assert_eq!(
        n.object_symbol(One).unwrap_err().to_string(),
        "AbsentObject { object: LangevinCoefficientOne }"
    );
    assert_eq!(
        n.quantity_symbol(One).unwrap_err(),
        SymbolLookupFault::AbsentObject { object: One }
    );
    Ok(())
}

#[test]
fn n06_recursive_alias_minima_and_transitive_reason_union() -> TestResult {
    use DefinitionId::{CoefficientOne, Dimension, FirstCoefficient};
    use NodeId::LangevinCoefficientOne as One;
    for (proposed, missing, expected) in [
        (false, false, vec![]),
        (
            false,
            true,
            vec![PendingReason::BackingUnbound { object: One }],
        ),
        (
            true,
            false,
            vec![PendingReason::FormulationProposed { object: One }],
        ),
        (
            true,
            true,
            vec![
                PendingReason::BackingUnbound { object: One },
                PendingReason::FormulationProposed { object: One },
            ],
        ),
    ] {
        let graph = crate::validate(if proposed {
            crate::seed_draft()
        } else {
            retained_draft()
        })?;
        let book = addressing(&graph)?;
        let mut d = catalogue::catalogue();
        if missing {
            *backing_mut(&mut d[4]) = None;
        }
        let n = construct(&book, primitives::catalogue(), d)?;
        let status = n.definition(FirstCoefficient).status().unwrap();
        assert_eq!(status.reasons(), expected);
        assert_eq!(
            status.tier(),
            if expected.is_empty() {
                SymbolTier::Authorised
            } else {
                SymbolTier::Pending
            }
        );
        assert_eq!(n.quantity_symbol(One)?.spec().id(), CoefficientOne);
    }
    let graph = crate::validate(retained_draft())?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    *backing_mut(&mut d[4]) = None;
    d[5].signature.arity = 0;
    d[5].kind = DefinitionKind::Alias {
        object: One,
        backing: Some(One),
        target: FirstCoefficient,
    };
    let n = construct(&book, primitives::catalogue(), d)?;
    assert_eq!(
        n.definition(CoefficientOne).status().unwrap().tier(),
        SymbolTier::Authorised
    );
    for id in [FirstCoefficient, Dimension] {
        assert_eq!(
            n.definition(id).status().unwrap().tier(),
            SymbolTier::Pending
        );
        assert_eq!(
            n.definition(id).status().unwrap().reasons(),
            [PendingReason::BackingUnbound { object: One }]
        );
    }
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[1]) = call(CoefficientOne, vec![]);
    let n = construct(&book, primitives::catalogue(), d)?;
    assert_eq!(
        n.definition(DefinitionId::CoefficientThreeValue)
            .status()
            .unwrap()
            .reasons(),
        [
            PendingReason::FormulationProposed {
                object: NodeId::LangevinCoefficientThreeValue
            },
            PendingReason::FormulationProposed { object: One },
            PendingReason::FormulationProposed {
                object: NodeId::LangevinCoefficientThree
            },
            PendingReason::ClaimDeferred {
                object: NodeId::LangevinCoefficientThreeValue,
                blocker: crate::Blocker::AuditRunRequired
            },
        ]
    );
    Ok(())
}

#[test]
fn n07_addressing_independence_and_snapshot_preservation() -> TestResult {
    use crate::{OutOfSliceCause, TagId, TagResolutionFault};
    use NodeId::{
        LangevinCoefficientOne as One, LangevinCoefficientOneValue as Value,
        LangevinCoefficientThree as Three,
    };
    let mut snapshots = vec![];
    for slice in [
        vec![Value, NodeId::LangevinCoefficientThreeValue],
        vec![Value],
        vec![],
    ] {
        let mut draft = retained_draft();
        draft.coverage.slice = slice.clone();
        let graph = crate::validate(draft)?;
        let before = crate::projection_bytes(&graph)?;
        let book = addressing(&graph)?;
        let history: Vec<_> = book.history().cloned().collect();
        let n = notation(&book)?;
        snapshots.push(
            n.definitions()
                .iter()
                .map(|d| d.status().cloned())
                .collect::<Vec<_>>(),
        );
        if slice.is_empty() {
            let error = book.resolve("c:1").unwrap_err();
            assert_eq!(
                error,
                TagResolutionFault::OutOfSliceTag {
                    input: "c:1".into(),
                    tag: TagId::LangevinCoefficientOne,
                    target: One,
                    cause: OutOfSliceCause::QuantityOutsideSupport
                }
            );
            assert_eq!(
                error.to_string(),
                "tag \"c:1\": out of slice; label def:langevin:coefficient-one; cause quantity-outside-support"
            );
        } else {
            assert_eq!(book.resolve("c:1")?, One);
        }
        if slice.len() == 1 {
            let error = book.resolve("c:3").unwrap_err();
            assert_eq!(
                error,
                TagResolutionFault::OutOfSliceTag {
                    input: "c:3".into(),
                    tag: TagId::LangevinCoefficientThree,
                    target: Three,
                    cause: OutOfSliceCause::QuantityOutsideSupport
                }
            );
            assert_eq!(
                error.to_string(),
                "tag \"c:3\": out of slice; label def:langevin:coefficient-three; cause quantity-outside-support"
            );
        }
        assert_eq!(
            n.quantity_symbol(Value).unwrap_err(),
            SymbolLookupFault::NotQuantity {
                object: Value,
                actual: NodeKind::Claim
            }
        );
        assert_eq!(
            n.quantity_symbol(Value).unwrap_err().to_string(),
            "NotQuantity { object: LangevinCoefficientOneValue, actual: Claim }"
        );
        for object in NodeId::ALL {
            check_expression(&n, call(n.object_symbol(object)?.spec().id(), vec![]))?;
        }
        assert_eq!(book.history().cloned().collect::<Vec<_>>(), history);
        assert_eq!(crate::projection_bytes(&graph)?, before);
    }
    assert_eq!(snapshots[0], snapshots[1]);
    assert_eq!(snapshots[1], snapshots[2]);
    Ok(())
}

fn chain(count: usize, mut expr: ExprDraft) -> ExprDraft {
    for _ in 0..count {
        expr = primitive(PrimitiveId::Parentheses, vec![expr]);
    }
    expr
}
fn full_binary(levels: usize) -> ExprDraft {
    if levels == 0 {
        literal("x")
    } else {
        primitive(
            PrimitiveId::Add,
            vec![full_binary(levels - 1), full_binary(levels - 1)],
        )
    }
}
fn argument_path(indices: &[usize]) -> Vec<NotationPathPart> {
    indices
        .iter()
        .flat_map(|i| {
            [
                NotationPathPart::Field("arguments".into()),
                NotationPathPart::Argument(*i),
            ]
        })
        .collect()
}
#[test]
fn n02_depth_and_node_boundaries() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    check_expression(&n, chain(63, literal("x")))?;
    let path = argument_path(&[0; 64]);
    let expected_path = std::iter::repeat_n("Field(\"arguments\"), Argument(0)", 64)
        .collect::<Vec<_>>()
        .join(", ");
    exact(
        &check_expression(&n, chain(64, literal("x"))).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Ast,
            NotationSite::Expression { path },
            NotationFault::NotationLimit {
                resource: NotationResource::Depth,
                limit: 64,
                actual: 65,
            },
        )],
        &format!(
            "M047 Ast at Expression {{ path: [{expected_path}] }}: NotationLimit {{ resource: Depth, limit: 64, actual: 65 }}\n"
        ),
    );
    check_expression(&n, chain(1, full_binary(11)))?;
    let mut indices = vec![0, 0];
    indices.extend([1; 11]);
    let path = argument_path(&indices);
    let expected_path = [
        vec!["Field(\"arguments\"), Argument(0)"; 2],
        vec!["Field(\"arguments\"), Argument(1)"; 11],
    ]
    .concat()
    .join(", ");
    exact(
        &check_expression(&n, chain(2, full_binary(11))).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Ast,
            NotationSite::Expression { path },
            NotationFault::NotationLimit {
                resource: NotationResource::AstNodes,
                limit: 4096,
                actual: 4097,
            },
        )],
        &format!(
            "M047 Ast at Expression {{ path: [{expected_path}] }}: NotationLimit {{ resource: AstNodes, limit: 4096, actual: 4097 }}\n"
        ),
    );
    check_expression(&n, literal("x"))?;
    Ok(())
}
// Independent visit arithmetic: a duplicating invocation visits its call, the
// binary primitive and two slots, plus two visits of its argument tree.
fn expanded_size(size: usize) -> ExprDraft {
    if size == 1 {
        literal("x")
    } else if size >= 6 && size.is_multiple_of(2) {
        call(DefinitionId::Dimension, vec![expanded_size((size - 4) / 2)])
    } else {
        chain(1, expanded_size(size - 1))
    }
}
#[test]
fn n02_substitution_counts_repeated_occurrences_at_exact_limit() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[5]) = primitive(
        PrimitiveId::Add,
        vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 0 }],
    );
    let n = construct(&book, primitives::catalogue(), d)?;
    check_expression(&n, expanded_size(65_536))?;
    let error = check_expression(&n, chain(1, expanded_size(65_536))).unwrap_err();
    // Each argument remains at its authored site, even on the final duplicate.
    let mut size: usize = 65_536;
    let mut indices = vec![0];
    while size != 1 {
        indices.push(0);
        size = if size >= 6 && size.is_multiple_of(2) {
            (size - 4) / 2
        } else {
            size - 1
        };
    }
    let path = argument_path(&indices);
    let display_path = std::iter::repeat_n("Field(\"arguments\"), Argument(0)", indices.len())
        .collect::<Vec<_>>()
        .join(", ");
    exact(
        &error,
        &[diag(
            RuleId::M047,
            NotationStage::Dependencies,
            NotationSite::Expression { path },
            NotationFault::NotationLimit {
                resource: NotationResource::ExpansionNodes,
                limit: 65_536,
                actual: 65_537,
            },
        )],
        &format!(
            "M047 Dependencies at Expression {{ path: [{display_path}] }}: NotationLimit {{ resource: ExpansionNodes, limit: 65536, actual: 65537 }}\n"
        ),
    );
    check_expression(&n, expanded_size(65_536))?;
    Ok(())
}

#[test]
fn n16_exact_appended_register() {
    use crate::{RULES, RuleState};
    let rules = &RULES[..49];
    assert_eq!(
        rules[46..]
            .iter()
            .map(|r| (r.number, r.statement, r.scope, r.state, r.deferred_reason))
            .collect::<Vec<_>>(),
        [
            (
                RuleId::M047,
                "Notation expressions use only the closed registry, exact signatures, bounded literal grammar and scoped constructor parameters, and every formal is used, within declared notation limits.",
                "Every primitive descriptor, definition body and submitted notation expression.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M048,
                "Definitions have their kind's signature shape and any backing equals their object; stems obey the reserved alphabetic grammar and generated names are unique; every permanent object has exactly one canonical symbol; aliases preserve that object and terminate at its canonical symbol; all definition dependencies are acyclic.",
                "The complete notation definition catalogue, including absent objects and unused definitions.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M049,
                "Symbol tiers are derived from the supplied graph and transitive symbol uses; an alias takes the lesser of its own binding tier and its target tier, retaining every pending reason.",
                "Checked notation catalogues and all exposed symbol-status values.",
                RuleState::Structural,
                None
            ),
        ]
    );
    assert_eq!(
        (
            rules.len(),
            rules
                .iter()
                .filter(|r| r.state == RuleState::Enforced)
                .count(),
            rules
                .iter()
                .filter(|r| r.state == RuleState::Structural)
                .count(),
            rules.iter().filter(|r| r.deferred_reason.is_some()).count()
        ),
        (49, 40, 4, 5)
    );
    assert_eq!(
        (
            RULES[..46].len(),
            RULES[..46]
                .iter()
                .filter(|r| r.state == RuleState::Enforced)
                .count(),
            RULES[..46]
                .iter()
                .filter(|r| r.state == RuleState::Structural)
                .count(),
            RULES[..46]
                .iter()
                .filter(|r| r.deferred_reason.is_some())
                .count()
        ),
        (46, 38, 3, 5)
    );
}

#[test]
fn n16_renderer_register_has_no_existing_transitions() {
    use crate::{RULES, RuleState};
    assert_eq!(
        RULES[49..52]
            .iter()
            .map(|r| (r.number, r.statement, r.scope, r.state, r.deferred_reason))
            .collect::<Vec<_>>(),
        [
            (
                RuleId::M050,
                "Generated notation declarations refuse collisions in the consuming typesetting context.",
                "Every emitted notation declaration and its checked KaTeX context.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M051,
                "Both notation forms derive from the same checked AST with explicit token boundaries and within the output limit; full Unicode expansion contains no project commands and every returned expression passes the pinned KaTeX grammar.",
                "Generated declaration bodies, macro invocations and fully expanded notation expressions.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M052,
                "A generated sweep typesets every emitted notation command with its declared arguments and checks its fully expanded counterpart.",
                "The complete emitted command inventory, including aliases, constructors and pending symbols.",
                RuleState::Enforced,
                None
            ),
        ]
    );
    assert_eq!(
        (
            RULES.len(),
            RULES
                .iter()
                .filter(|r| r.state == RuleState::Enforced)
                .count(),
            RULES
                .iter()
                .filter(|r| r.state == RuleState::Structural)
                .count(),
            RULES.iter().filter(|r| r.deferred_reason.is_some()).count()
        ),
        (59, 50, 4, 5)
    );
}

#[test]
fn n01_definition_multiplicity_tokens_and_every_call_arity() -> TestResult {
    use DefinitionId::{CoefficientOne, CoefficientThree};
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    for (id, arity) in [
        (CoefficientOne, 0),
        (CoefficientThree, 0),
        (DefinitionId::CoefficientOneValue, 0),
        (DefinitionId::CoefficientThreeValue, 0),
        (DefinitionId::FirstCoefficient, 0),
        (DefinitionId::Dimension, 1),
        (DefinitionId::Product, 2),
    ] {
        for actual in [
            usize::from(arity).checked_sub(1),
            Some(usize::from(arity) + 1),
        ]
        .into_iter()
        .flatten()
        {
            exact(
                &check_expression(&n, call(id, vec![literal("x"); actual])).unwrap_err(),
                &[diag(
                    RuleId::M047,
                    NotationStage::Ast,
                    expression_site(&["arguments"]),
                    NotationFault::CallArity {
                        callee: Callee::Definition(id),
                        expected: arity,
                        actual,
                    },
                )],
                &format!(
                    "M047 Ast at Expression {{ path: [Field(\"arguments\")] }}: CallArity {{ callee: Definition({id:?}), expected: {arity}, actual: {actual} }}\n"
                ),
            );
            check_expression(&n, call(id, vec![literal("x"); usize::from(arity)]))?;
        }
    }
    let mut d = catalogue::catalogue();
    let mut copy = d[0].clone();
    copy.stem = "Another";
    copy.token = "another";
    d.push(copy);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            registry_site(CoefficientOne, 0, &["id"]),
            NotationFault::CatalogMultiplicity {
                entry: RegistryEntry::Definition(CoefficientOne),
                expected: 1,
                actual: 2,
                positions: vec![0, 7],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"id\")] }: CatalogMultiplicity { entry: Definition(CoefficientOne), expected: 1, actual: 2, positions: [0, 7] }\n",
    );
    let mut d = catalogue::catalogue();
    d[1].token = "coefficient-one";
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            registry_site(CoefficientOne, 0, &["token"]),
            NotationFault::DuplicateToken {
                token: "coefficient-one".into(),
                entries: vec![
                    RegistryEntry::Definition(CoefficientOne),
                    RegistryEntry::Definition(CoefficientThree),
                ],
                positions: vec![0, 1],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"token\")] }: DuplicateToken { token: \"coefficient-one\", entries: [Definition(CoefficientOne), Definition(CoefficientThree)], positions: [0, 1] }\n",
    );
    let mut p = primitives::catalogue();
    p[1].operation = PrimitiveOperation::Alpha;
    exact(
        &construct(&book, p, catalogue::catalogue()).unwrap_err(),
        &[
            diag(
                RuleId::M047,
                NotationStage::Catalogue,
                primitive_site(PrimitiveId::Alpha, 0, &["operation"]),
                NotationFault::CatalogMultiplicity {
                    entry: RegistryEntry::Primitive(PrimitiveId::Alpha),
                    expected: 1,
                    actual: 2,
                    positions: vec![0, 1],
                },
            ),
            diag(
                RuleId::M047,
                NotationStage::Catalogue,
                primitive_site(PrimitiveId::Beta, 12, &["operation"]),
                NotationFault::CatalogMultiplicity {
                    entry: RegistryEntry::Primitive(PrimitiveId::Beta),
                    expected: 1,
                    actual: 0,
                    positions: vec![],
                },
            ),
        ],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Alpha), index: 0, path: [Field(\"operation\")] }: CatalogMultiplicity { entry: Primitive(Alpha), expected: 1, actual: 2, positions: [0, 1] }\nM047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Beta), index: 12, path: [Field(\"operation\")] }: CatalogMultiplicity { entry: Primitive(Beta), expected: 1, actual: 0, positions: [] }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n03_all_argument_errors_and_alias_self_cycle() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    let expression = primitive(
        PrimitiveId::Fraction,
        vec![literal("α"), ExprDraft::Slot { index: 0 }, literal("x")],
    );
    let mut first = argument_path(&[0]);
    first.push(NotationPathPart::Field("value".into()));
    let mut second = argument_path(&[1]);
    second.push(NotationPathPart::Field("index".into()));
    exact(
        &check_expression(&n, expression).unwrap_err(),
        &[
            diag(
                RuleId::M047,
                NotationStage::Ast,
                expression_site(&["arguments"]),
                NotationFault::CallArity {
                    callee: Callee::Primitive(PrimitiveId::Fraction),
                    expected: 2,
                    actual: 3,
                },
            ),
            diag(
                RuleId::M047,
                NotationStage::Ast,
                NotationSite::Expression { path: first },
                NotationFault::InvalidLiteral {
                    value: "α".into(),
                    reason: LiteralReason::ForbiddenScalar,
                    byte: 0,
                    scalar: Some("α".into()),
                },
            ),
            diag(
                RuleId::M047,
                NotationStage::Ast,
                NotationSite::Expression { path: second },
                NotationFault::SlotOutOfScope { index: 0, arity: 0 },
            ),
        ],
        "M047 Ast at Expression { path: [Field(\"arguments\")] }: CallArity { callee: Primitive(Fraction), expected: 2, actual: 3 }\nM047 Ast at Expression { path: [Field(\"arguments\"), Argument(0), Field(\"value\")] }: InvalidLiteral { value: \"α\", reason: ForbiddenScalar, byte: 0, scalar: Some(\"α\") }\nM047 Ast at Expression { path: [Field(\"arguments\"), Argument(1), Field(\"index\")] }: SlotOutOfScope { index: 0, arity: 0 }\n",
    );
    let mut d = catalogue::catalogue();
    if let DefinitionKind::Alias { target, .. } = &mut d[4].kind {
        *target = DefinitionId::FirstCoefficient;
    }
    let site = registry_site(DefinitionId::FirstCoefficient, 4, &["target"]);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Dependencies,
            site.clone(),
            NotationFault::DefinitionCycle {
                chain: vec![
                    DefinitionId::FirstCoefficient,
                    DefinitionId::FirstCoefficient,
                ],
                sites: vec![site],
            },
        )],
        "M048 Dependencies at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(FirstCoefficient), index: 4, path: [Field(\"target\")] }: DefinitionCycle { chain: [FirstCoefficient, FirstCoefficient], sites: [Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(FirstCoefficient), index: 4, path: [Field(\"target\")] }] }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n05_seed_exact_statuses_and_catalogue_bodies() -> TestResult {
    use DefinitionId::{
        CoefficientOne, CoefficientOneValue, CoefficientThree, CoefficientThreeValue, Dimension,
        FirstCoefficient, Product,
    };
    use NodeId::{
        LangevinCoefficientOne as One, LangevinCoefficientOneValue as OneValue,
        LangevinCoefficientThree as Three, LangevinCoefficientThreeValue as ThreeValue,
    };
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    let expected = [
        (
            CoefficientOne,
            "coefficient-one",
            "CoefficientOne",
            "\\FtooQCoefficientOne",
            0,
            DefinitionKind::Canonical {
                object: One,
                backing: Some(One),
                body: primitive(PrimitiveId::Subscript, vec![literal("c"), literal("1")]),
            },
        ),
        (
            CoefficientThree,
            "coefficient-three",
            "CoefficientThree",
            "\\FtooQCoefficientThree",
            0,
            DefinitionKind::Canonical {
                object: Three,
                backing: Some(Three),
                body: primitive(PrimitiveId::Subscript, vec![literal("c"), literal("3")]),
            },
        ),
        (
            CoefficientOneValue,
            "coefficient-one-value",
            "CoefficientOneValue",
            "\\FtooOCoefficientOneValue",
            0,
            DefinitionKind::Canonical {
                object: OneValue,
                backing: Some(OneValue),
                body: primitive(
                    PrimitiveId::Equal,
                    vec![
                        call(CoefficientOne, vec![]),
                        primitive(PrimitiveId::Fraction, vec![literal("1"), literal("3")]),
                    ],
                ),
            },
        ),
        (
            CoefficientThreeValue,
            "coefficient-three-value",
            "CoefficientThreeValue",
            "\\FtooOCoefficientThreeValue",
            0,
            DefinitionKind::Canonical {
                object: ThreeValue,
                backing: Some(ThreeValue),
                body: primitive(
                    PrimitiveId::Equal,
                    vec![
                        call(CoefficientThree, vec![]),
                        primitive(
                            PrimitiveId::Negate,
                            vec![primitive(
                                PrimitiveId::Fraction,
                                vec![literal("1"), literal("45")],
                            )],
                        ),
                    ],
                ),
            },
        ),
        (
            FirstCoefficient,
            "first-coefficient",
            "FirstCoefficient",
            "\\FtooAFirstCoefficient",
            0,
            DefinitionKind::Alias {
                object: One,
                backing: Some(One),
                target: CoefficientOne,
            },
        ),
        (
            Dimension,
            "dimension",
            "Dimension",
            "\\FtooNDimension",
            1,
            DefinitionKind::Constructor {
                body: primitive(PrimitiveId::Dimension, vec![ExprDraft::Slot { index: 0 }]),
            },
        ),
        (
            Product,
            "product",
            "Product",
            "\\FtooNProduct",
            2,
            DefinitionKind::Constructor {
                body: primitive(
                    PrimitiveId::Multiply,
                    vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 1 }],
                ),
            },
        ),
    ];
    assert_eq!(
        n.definitions()
            .iter()
            .map(|d| (
                d.spec().id(),
                d.spec().token(),
                d.spec().stem(),
                d.command(),
                d.spec().signature().arity,
                d.spec().kind().clone()
            ))
            .collect::<Vec<_>>(),
        expected
    );
    for (id, objects, claim) in [
        (CoefficientOne, vec![One], None),
        (CoefficientThree, vec![Three], None),
        (CoefficientOneValue, vec![OneValue, One], Some(OneValue)),
        (
            CoefficientThreeValue,
            vec![ThreeValue, Three],
            Some(ThreeValue),
        ),
        (FirstCoefficient, vec![One], None),
    ] {
        let mut reasons: Vec<_> = objects
            .into_iter()
            .map(|object| PendingReason::FormulationProposed { object })
            .collect();
        if let Some(object) = claim {
            reasons.push(PendingReason::ClaimDeferred {
                object,
                blocker: crate::Blocker::AuditRunRequired,
            });
        }
        assert_eq!(n.definition(id).status().unwrap().reasons(), reasons);
        assert_eq!(
            n.definition(id).status().unwrap().tier(),
            SymbolTier::Pending
        );
    }
    assert_eq!(n.definition(Dimension).status(), None);
    assert_eq!(n.definition(Product).status(), None);
    Ok(())
}

#[test]
fn n04_repeated_substitution_keeps_each_origin_chain() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[5]) = primitive(
        PrimitiveId::Add,
        vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 0 }],
    );
    let n = construct(&book, primitives::catalogue(), d)?;
    let e = check_expression(
        &n,
        call(
            DefinitionId::Dimension,
            vec![call(DefinitionId::CoefficientOne, vec![])],
        ),
    )?;
    let expected: Vec<_> = [0, 1]
        .into_iter()
        .map(|index| {
            vec![
                NotationOrigin {
                    definition: None,
                    path: vec![],
                },
                NotationOrigin {
                    definition: Some(DefinitionId::Dimension),
                    path: vec![
                        NotationPathPart::Field("body".into()),
                        NotationPathPart::Field("arguments".into()),
                        NotationPathPart::Argument(index),
                    ],
                },
                NotationOrigin {
                    definition: None,
                    path: vec![
                        NotationPathPart::Field("arguments".into()),
                        NotationPathPart::Argument(0),
                    ],
                },
            ]
        })
        .collect();
    assert_eq!(
        e.uses()
            .iter()
            .map(|u| u.origins().to_vec())
            .collect::<Vec<_>>(),
        expected
    );
    for usage in e.uses() {
        assert_eq!(usage.definition(), DefinitionId::CoefficientOne);
        assert_eq!(
            usage.status().reasons(),
            [PendingReason::FormulationProposed {
                object: NodeId::LangevinCoefficientOne
            }]
        );
    }
    Ok(())
}

#[test]
fn n02_constructor_body_expansion_boundary() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut definitions = catalogue::catalogue();
    *body_mut(&mut definitions[5]) = primitive(
        PrimitiveId::Add,
        vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 0 }],
    );
    let body = primitive(
        PrimitiveId::Add,
        vec![
            ExprDraft::Slot { index: 0 },
            primitive(
                PrimitiveId::Add,
                vec![ExprDraft::Slot { index: 1 }, expanded_size(65_530)],
            ),
        ],
    );
    *body_mut(&mut definitions[6]) = body.clone();
    construct(&book, primitives::catalogue(), definitions.clone())?;
    *body_mut(&mut definitions[6]) = chain(1, body);
    let mut indices = vec![0, 1, 1];
    let mut size: usize = 65_530;
    while size != 1 {
        indices.push(0);
        size = if size >= 6 && size.is_multiple_of(2) {
            (size - 4) / 2
        } else {
            size - 1
        };
    }
    let mut path = vec![NotationPathPart::Field("body".into())];
    path.extend(argument_path(&indices));
    let location = NotationSite::Registry {
        file: "crates/ftoo_manifest/src/notation/catalogue.rs".into(),
        entry: RegistryEntry::Definition(DefinitionId::Product),
        index: 6,
        path,
    };
    let display_path = indices
        .iter()
        .map(|i| format!("Field(\"arguments\"), Argument({i})"))
        .collect::<Vec<_>>()
        .join(", ");
    exact(
        &construct(&book, primitives::catalogue(), definitions).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Dependencies,
            location,
            NotationFault::NotationLimit {
                resource: NotationResource::ExpansionNodes,
                limit: 65_536,
                actual: 65_537,
            },
        )],
        &format!(
            "M047 Dependencies at Registry {{ file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Product), index: 6, path: [Field(\"body\"), {display_path}] }}: NotationLimit {{ resource: ExpansionNodes, limit: 65536, actual: 65537 }}\n"
        ),
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn complete_seed_consumer() {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let n = notation(&book).unwrap();
    assert_eq!(n.definitions().len(), 7);
    assert_eq!(n.primitives().len(), 12);
    assert_eq!(
        n.definitions()
            .iter()
            .filter(|d| d.status().is_some())
            .count(),
        5
    );
    for object in NodeId::ALL {
        assert_eq!(
            n.object_symbol(object).unwrap().status().unwrap().tier(),
            SymbolTier::Pending
        );
    }
    for d in n.definitions() {
        let args = (0..d.spec().signature().arity)
            .map(|_| literal("x"))
            .collect();
        check_expression(&n, call(d.spec().id(), args)).unwrap();
    }
    for p in n.primitives() {
        let args = (0..p.signature().arity).map(|_| literal("x")).collect();
        check_expression(&n, primitive(p.id(), args)).unwrap();
    }
}

#[test]
fn exact_fraction_arity_and_display() {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let n = notation(&book).unwrap();
    let error =
        check_expression(&n, primitive(PrimitiveId::Fraction, vec![literal("1")])).unwrap_err();
    assert_eq!(
        error.diagnostics(),
        &[NotationDiagnostic {
            rule: RuleId::M047,
            stage: NotationStage::Ast,
            site: NotationSite::Expression {
                path: vec![NotationPathPart::Field("arguments".into())]
            },
            fault: NotationFault::CallArity {
                callee: Callee::Primitive(PrimitiveId::Fraction),
                expected: 2,
                actual: 1
            },
        }]
    );
    assert_eq!(
        error.to_string(),
        "M047 Ast at Expression { path: [Field(\"arguments\")] }: CallArity { callee: Primitive(Fraction), expected: 2, actual: 1 }\n"
    );
    check_expression(
        &n,
        primitive(PrimitiveId::Fraction, vec![literal("1"), literal("3")]),
    )
    .unwrap();
}

#[test]
fn pending_text_parts_keep_every_template_and_blocker_spelling() {
    use crate::{Awaiting, Blocker, CapabilityId, NodeId, PendingReason};
    let object = NodeId::LangevinCoefficientOne;
    let mut cases = vec![
        (
            PendingReason::BackingUnbound { object },
            "backing object ",
            " is unbound",
        ),
        (
            PendingReason::BackingAbsent { object },
            "backing object ",
            " is absent",
        ),
        (
            PendingReason::FormulationProposed { object },
            "retained formulation ",
            "",
        ),
    ];
    for (blocker, after) in [
        (Blocker::AuditRunRequired, " awaits audit-run-required"),
        (
            Blocker::Awaiting(Awaiting::AuditBinding),
            " awaits audit-binding",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(
                CapabilityId::ExactRationalLinearEquivalence,
            )),
            " awaits capability exact-rational-linear-equivalence",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(
                CapabilityId::FormalTruncatedPowerSeries,
            )),
            " awaits capability formal-truncated-power-series",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(CapabilityId::SymbolicTrigonometry)),
            " awaits capability symbolic-trigonometry",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(CapabilityId::TrigonometricIntegration)),
            " awaits capability trigonometric-integration",
        ),
    ] {
        cases.push((
            PendingReason::ClaimDeferred { object, blocker },
            "claim evidence ",
            after,
        ));
    }
    for (reason, before, after) in cases {
        assert_eq!(reason.text_parts(), (before, object, after.into()));
        assert_eq!(
            reason.to_string(),
            format!("{before}def:langevin:coefficient-one{after}")
        );
    }
}
