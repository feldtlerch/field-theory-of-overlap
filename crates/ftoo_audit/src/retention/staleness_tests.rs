use super::{wire, *};
use ftoo_manifest::{
    ProjectionDiagnostic, ProjectionError, ProjectionFault, ProjectionValue, RuleId,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

fn control() -> Arc<Produced> {
    let projection = ftoo_manifest::projection_bytes(&ftoo_manifest::seed().unwrap()).unwrap();
    let production = produce(projection.clone()).unwrap();
    assert_eq!(
        accept(projection, production.bytes.clone())
            .unwrap()
            .bytes(),
        production.bytes
    );
    production
}
fn assertion(path: &str, position: Option<usize>, fault: ProjectionFault) -> ProjectionError {
    ProjectionError::Assertion(vec![ProjectionDiagnostic {
        rule: RuleId::M037,
        path: path.into(),
        position,
        fault,
    }])
}
fn check(
    production: &Arc<Produced>,
    value: &serde_json::Value,
    error: &ProjectionError,
    display: &str,
) {
    let rule = match error {
        ProjectionError::Wire(error) => error.rule,
        ProjectionError::Graph(errors) => errors.diagnostics()[0].rule,
        ProjectionError::Addressing(errors) => errors[0].rule,
        ProjectionError::Assertion(errors) => errors[0].rule,
    };
    let changed_projection = format!("{}\n", serde_json::to_string(value).unwrap()).into_bytes();
    for embedded in [true, false] {
        let mut dto = production.dto.clone();
        let current = if embedded {
            dto.projection = String::from_utf8(changed_projection.clone()).unwrap();
            dto.digest = wire::hex(&Sha256::digest(&changed_projection));
            production.key.projection.clone()
        } else {
            changed_projection.clone()
        };
        let file = super::file_tests::FileFixture::new(production);
        let actual = file
            .refuse_before_production(current, &wire::write(&dto).unwrap())
            .unwrap_err();
        let (kind, path) = if embedded {
            ("json-path", "$.provenance.projection-utf8")
        } else {
            ("file", "generated/manifest.json")
        };
        assert_eq!(
            actual,
            RetentionError::Projection {
                path: path.into(),
                error: Box::new(error.clone())
            }
        );
        assert_eq!(
            actual.to_string(),
            format!("retention provenance {rule:?} at {kind} \"{path}\": {display}")
        );
        assert_eq!(
            accept(production.key.projection.clone(), production.bytes.clone())
                .unwrap()
                .bytes(),
            production.bytes
        );
    }
}

#[test]
fn each_stale_projection_limit_is_preserved_by_the_existing_projection_fault() {
    let production = control();
    let original: serde_json::Value = serde_json::from_slice(&production.key.projection).unwrap();
    for (name, value) in original["budgets"][0]["limits"].as_object().unwrap() {
        let expected = value.as_u64().unwrap();
        let mut changed = original.clone();
        changed["budgets"][0]["limits"][name] = (expected + 1).into();
        let path = format!("$.budgets[0].limits.{name}");
        check(
            &production,
            &changed,
            &assertion(
                &path,
                Some(0),
                ProjectionFault::BudgetLimitsMismatch {
                    expected,
                    actual: expected + 1,
                },
            ),
            &format!(
                "M037 at {path}: BudgetLimitsMismatch {{ expected: {expected}, actual: {} }}\n",
                expected + 1
            ),
        );
    }
}

#[test]
fn addressing_history_and_store_changes_are_not_erased_by_semantic_snapshot() {
    use ftoo_manifest::{
        AddressingDiagnostic, AddressingFault, AddressingLocation, AddressingStage, NodeId, TagId,
    };
    let production = control();
    let original: serde_json::Value = serde_json::from_slice(&production.key.projection).unwrap();
    let mut changed = original.clone();
    changed["tags"][0]["target"] = "def:langevin:coefficient-three".into();
    check(
        &production,
        &changed,
        &assertion(
            "$.tags[0].target",
            Some(0),
            ProjectionFault::TagBindingMismatch {
                tag: TagId::LangevinCoefficientOne,
                expected: NodeId::LangevinCoefficientOne,
                actual: NodeId::LangevinCoefficientThree,
            },
        ),
        "M037 at $.tags[0].target: TagBindingMismatch { tag: LangevinCoefficientOne, expected: LangevinCoefficientOne, actual: LangevinCoefficientThree }\n",
    );
    let mut changed = original.clone();
    changed["history"][0]["reason"] = "Changed reason.".into();
    check(
        &production,
        &changed,
        &assertion(
            "$.history[0].reason",
            Some(0),
            ProjectionFault::HistoryMismatch {
                expected: Some(Box::new(ProjectionValue::Text(
                    "Retrospective registration of the M1 seed identity.".into(),
                ))),
                actual: Some(Box::new(ProjectionValue::Text("Changed reason.".into()))),
            },
        ),
        "M037 at $.history[0].reason: HistoryMismatch { expected: Some(Text(\"Retrospective registration of the M1 seed identity.\")), actual: Some(Text(\"Changed reason.\")) }\n",
    );
    let mut changed = original.clone();
    changed["stores"]["live"].as_array_mut().unwrap().remove(0);
    check(
        &production,
        &changed,
        &ProjectionError::Addressing(vec![AddressingDiagnostic {
            rule: RuleId::M035,
            stage: AddressingStage::StoreConstruction,
            location: AddressingLocation::Store(TagId::LangevinCoefficientOne),
            tag: Some(TagId::LangevinCoefficientOne),
            identity: Some(NodeId::LangevinCoefficientOne),
            fault: AddressingFault::StorePartitionMismatch {
                expected: 1,
                actual: 0,
            },
        }]),
        "M035 StoreConstruction at Store(LangevinCoefficientOne): StorePartitionMismatch { expected: 1, actual: 0 }\n",
    );
}

#[test]
fn stale_pin_and_profile_are_strict_projection_faults_before_allocation() {
    use ftoo_manifest::{WireError, WireFault, WireLocation};
    let production = control();
    let original: serde_json::Value = serde_json::from_slice(&production.key.projection).unwrap();
    let mut changed = original.clone();
    changed["engine-pin"] = "0".repeat(40).into();
    check(
        &production,
        &changed,
        &assertion(
            "$.engine-pin",
            None,
            ProjectionFault::PinMismatch {
                expected: ftoo_manifest::ENGINE_PIN.0.into(),
                actual: "0".repeat(40),
            },
        ),
        "M037 at $.engine-pin: PinMismatch { expected: \"598ff683a57caca07cf9d44e3fd28c716afb81ef\", actual: \"0000000000000000000000000000000000000000\" }\n",
    );
    let mut changed = original;
    changed["budgets"][0]["profile"] = "old-profile".into();
    check(
        &production,
        &changed,
        &ProjectionError::Wire(WireError {
            rule: RuleId::M004,
            location: WireLocation::Path("$.budgets[0].profile".into()),
            fault: WireFault::UnknownToken {
                expected: vec!["pinned-trial-limits-v2".into()],
                actual: "old-profile".into(),
            },
        }),
        "M004 at Path(\"$.budgets[0].profile\"): UnknownToken { expected: [\"pinned-trial-limits-v2\"], actual: \"old-profile\" }",
    );
}

#[test]
fn stale_producer_revision_and_capability_evidence_keep_complete_contract_values() {
    let production = control();
    let original: serde_json::Value = serde_json::from_slice(&production.key.projection).unwrap();
    for index in 0..2 {
        let mut changed = original.clone();
        changed["producers"][index]["revision"] = 3.into();
        let path = format!("$.producers[{index}].revision");
        check(
            &production,
            &changed,
            &assertion(
                &path,
                Some(index),
                ProjectionFault::ContractMismatch {
                    expected: Box::new(ProjectionValue::Integer(2)),
                    actual: Box::new(ProjectionValue::Integer(3)),
                },
            ),
            &format!(
                "M037 at {path}: ContractMismatch {{ expected: Integer(2), actual: Integer(3) }}\n"
            ),
        );
    }
    for index in 0..original["capabilities"].as_array().unwrap().len() {
        let mut changed = original.clone();
        let expected = original["capabilities"][index]["evidence"]
            .as_str()
            .unwrap();
        let actual = format!("{expected}!");
        changed["capabilities"][index]["evidence"] = actual.clone().into();
        let path = format!("$.capabilities[{index}].evidence");
        check(
            &production,
            &changed,
            &assertion(
                &path,
                Some(index),
                ProjectionFault::ContractMismatch {
                    expected: Box::new(ProjectionValue::Text(expected.into())),
                    actual: Box::new(ProjectionValue::Text(actual.clone())),
                },
            ),
            &format!(
                "M037 at {path}: ContractMismatch {{ expected: Text({expected:?}), actual: Text({actual:?}) }}\n"
            ),
        );
    }
}

#[test]
fn producer_inventory_and_dependencies_cannot_be_rewritten_as_current_provenance() {
    use ftoo_manifest::NodeId;
    let production = control();
    let original: serde_json::Value = serde_json::from_slice(&production.key.projection).unwrap();
    for (index, record, dependencies) in [
        (
            0,
            "langevin-coefficient-one-value",
            vec![NodeId::LangevinCoefficientOne],
        ),
        (
            1,
            "langevin-coefficient-three-value",
            vec![
                NodeId::LangevinCoefficientOneValue,
                NodeId::LangevinCoefficientThree,
            ],
        ),
    ] {
        let mut changed = original.clone();
        changed["producers"].as_array_mut().unwrap().remove(index);
        check(
            &production,
            &changed,
            &assertion(
                "$.producers",
                None,
                ProjectionFault::ContractMultiplicity {
                    record: record.into(),
                    expected: 1,
                    actual: 0,
                    positions: vec![],
                },
            ),
            &format!(
                "M037 at $.producers: ContractMultiplicity {{ record: {record:?}, expected: 1, actual: 0, positions: [] }}\n"
            ),
        );
        let mut changed = original.clone();
        let actual = if dependencies.is_empty() {
            vec![NodeId::LangevinCoefficientThreeValue]
        } else {
            vec![]
        };
        changed["producers"][index]["dependencies"] = serde_json::json!(
            actual
                .iter()
                .map(|id| id.semantic_name())
                .collect::<Vec<_>>()
        );
        let path = format!("$.producers[{index}].dependencies");
        check(
            &production,
            &changed,
            &assertion(
                &path,
                Some(index),
                ProjectionFault::ContractMismatch {
                    expected: Box::new(ProjectionValue::Identities(dependencies.clone())),
                    actual: Box::new(ProjectionValue::Identities(actual.clone())),
                },
            ),
            &format!(
                "M037 at {path}: ContractMismatch {{ expected: Identities({dependencies:?}), actual: Identities({actual:?}) }}\n"
            ),
        );
    }
}

#[test]
fn every_edge_guard_retains_the_closed_guard_vocabulary() {
    use ftoo_manifest::{WireError, WireFault, WireLocation};
    let production = control();
    let original: serde_json::Value = serde_json::from_slice(&production.key.projection).unwrap();
    for index in 0..original["edges"].as_array().unwrap().len() {
        let mut changed = original.clone();
        changed["edges"][index]["guard"]["kind"] = "conditional".into();
        let path = format!("$.edges[{index}].guard.kind");
        check(
            &production,
            &changed,
            &ProjectionError::Wire(WireError {
                rule: RuleId::M004,
                location: WireLocation::Path(path.clone()),
                fault: WireFault::UnknownToken {
                    expected: vec!["always".into()],
                    actual: "conditional".into(),
                },
            }),
            &format!(
                "M004 at Path({path:?}): UnknownToken {{ expected: [\"always\"], actual: \"conditional\" }}"
            ),
        );
    }
}
