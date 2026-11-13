use super::{acceptance, wire, *};
use std::{collections::VecDeque, sync::Arc};

fn control() -> Arc<Produced> {
    let graph = ftoo_manifest::seed().unwrap();
    let projection = ftoo_manifest::projection_bytes(&graph).unwrap();
    let production = produce(projection.clone()).unwrap();
    assert_eq!(
        accept(projection, production.bytes.clone())
            .unwrap()
            .bytes(),
        production.bytes
    );
    production
}

#[test]
fn noncanonical_current_projection_labels_both_actual_sources() {
    let production = control();
    let mut current = production.key.projection.clone();
    current.push(b' ');
    let mut difference = super::matrix_tests::expected_difference(
        &production.key.projection,
        &current,
        InfoRecord::collect(Run::Retained),
        InfoRecord::collect(Run::Retained),
    );
    difference.path = "generated/manifest.json".into();
    let display =
        super::classifier_tests::difference_display_labels(&difference, "canonical", "current");
    let actual = produce(current).unwrap_err();
    assert_eq!(
        actual,
        RetentionError::ProvenanceMismatch {
            component: Component::ProjectionBytes,
            path: "generated/manifest.json".into(),
            difference: ProvenanceDifference::Bytes(Box::new(difference)),
        }
    );
    assert_eq!(
        actual.to_string(),
        format!(
            "retention provenance M061 at file \"generated/manifest.json\": provenance-mismatch component=projection-bytes {display}"
        )
    );
    assert_eq!(
        produce(production.key.projection.clone()).unwrap().bytes(),
        production.bytes()
    );
}
fn check(
    production: &Arc<Produced>,
    changed: &wire::Report,
    expected: &RetentionError,
    display: &str,
) {
    let bytes = wire::write(changed).unwrap();
    let file = super::file_tests::FileFixture::new(production);
    let actual = file
        .refuse_before_production(production.key.projection.clone(), &bytes)
        .unwrap_err();
    assert_eq!(actual, *expected);
    assert_eq!(actual.to_string(), display);
    let restored = acceptance::accept_with(
        production.key.projection.clone(),
        production.bytes.clone(),
        &mut VecDeque::new(),
        &mut |_, _| Ok(Arc::clone(production)),
    )
    .unwrap();
    assert_eq!(restored.bytes(), production.bytes);
}
fn mismatch(
    component: Component,
    path: &str,
    expected: ProvenanceValue,
    actual: ProvenanceValue,
) -> RetentionError {
    RetentionError::ProvenanceMismatch {
        component,
        path: path.into(),
        difference: ProvenanceDifference::Values { expected, actual },
    }
}
#[test]
fn contract_engine_digest_pin_and_target_refuse_before_allocation() {
    let production = control();
    let mut changed = production.dto.clone();
    changed.contract = 2;
    check(
        &production,
        &changed,
        &mismatch(
            Component::ContractRevision,
            "$.provenance.inputs.contract-revision",
            ProvenanceValue::U32(1),
            ProvenanceValue::U32(2),
        ),
        "retention provenance M061 at json-path \"$.provenance.inputs.contract-revision\": provenance-mismatch component=contract-revision expected=1 actual=2",
    );
    changed = production.dto.clone();
    changed.engine = "0".repeat(40);
    check(
        &production,
        &changed,
        &mismatch(
            Component::EngineRevision,
            "$.provenance.inputs.engine-revision",
            ProvenanceValue::Text(ftoo_manifest::ENGINE_PIN.0.into()),
            ProvenanceValue::Text("0".repeat(40)),
        ),
        "retention provenance M061 at json-path \"$.provenance.inputs.engine-revision\": provenance-mismatch component=engine-revision expected=\"598ff683a57caca07cf9d44e3fd28c716afb81ef\" actual=\"0000000000000000000000000000000000000000\"",
    );
    changed = production.dto.clone();
    changed.digest = "0".repeat(64);
    check(
        &production,
        &changed,
        &mismatch(
            Component::ProjectionDigest,
            "$.provenance.projection-sha256",
            ProvenanceValue::Text(production.dto.digest.clone()),
            ProvenanceValue::Text("0".repeat(64)),
        ),
        &format!(
            "retention provenance M061 at json-path \"$.provenance.projection-sha256\": provenance-mismatch component=projection-digest expected=\"{}\" actual=\"{}\"",
            production.dto.digest,
            "0".repeat(64)
        ),
    );
    receipt_mutations(&production);
}
fn receipt_mutations(production: &Arc<Produced>) {
    for index in 0..2 {
        let path = format!("$.outcomes[{index}].verdict.receipt");
        let mut changed = production.dto.clone();
        let wire::Verdict::Certified(receipt) = &mut changed.outcomes[index].verdict else {
            unreachable!()
        };
        receipt.pin = "0".repeat(40);
        check(
            production,
            &changed,
            &mismatch(
                Component::Pin,
                &format!("{path}.pin"),
                ProvenanceValue::Text(ftoo_manifest::ENGINE_PIN.0.into()),
                ProvenanceValue::Text("0".repeat(40)),
            ),
            &format!(
                "retention provenance M025 at json-path \"{path}.pin\": provenance-mismatch component=pin expected=\"598ff683a57caca07cf9d44e3fd28c716afb81ef\" actual=\"0000000000000000000000000000000000000000\""
            ),
        );
        let mut changed = production.dto.clone();
        let wire::Verdict::Certified(receipt) = &mut changed.outcomes[index].verdict else {
            unreachable!()
        };
        let target = receipt.target;
        receipt.target += 1;
        check(
            production,
            &changed,
            &mismatch(
                Component::Target,
                &format!("{path}.target"),
                ProvenanceValue::U64(target),
                ProvenanceValue::U64(target + 1),
            ),
            &format!(
                "retention provenance M025 at json-path \"{path}.target\": provenance-mismatch component=target expected={target} actual={}",
                target + 1
            ),
        );
        for limit in 0..71 {
            let mut changed = production.dto.clone();
            let wire::Verdict::Certified(receipt) = &mut changed.outcomes[index].verdict else {
                unreachable!()
            };
            let (name, value) = receipt.limits[limit].clone();
            receipt.limits[limit].1 += 1;
            let mut difference = super::matrix_tests::expected_difference(
                (value + 1).to_string().as_bytes(),
                value.to_string().as_bytes(),
                InfoRecord::collect(Run::Retained),
                production.info.clone(),
            );
            difference.claim = Some(changed.outcomes[index].claim);
            difference.path = format!("{path}.limits.{name}");
            super::verdict_tests::reject(production, &changed, vec![difference]);
        }
    }
}

fn domain(
    rule: ftoo_manifest::RuleId,
    ids: Vec<ftoo_manifest::NodeId>,
    location: ftoo_manifest::DraftLocation,
    explanation: &str,
) -> RetentionError {
    RetentionError::Domain(vec![ftoo_manifest::Diagnostic {
        stage: ftoo_manifest::AdmissionStage::ReportConsumption,
        rule,
        ids,
        edge: None,
        location,
        explanation: explanation.into(),
    }])
}
#[test]
fn retained_domain_and_binding_faults_keep_complete_existing_diagnostics() {
    use ftoo_manifest::{DraftLocation, NodeId, RuleId};
    let production = control();
    let mut changed = production.dto.clone();
    changed.reached.pop();
    check(
        &production,
        &changed,
        &domain(
            RuleId::M014,
            vec![],
            DraftLocation::Coverage,
            "reached IDs must equal the declared slice exactly",
        ),
        "retention provenance M014 at file \"generated/audit-report.json\": domain stage=report-consumption ids=[] edge=none location=coverage explanation=\"reached IDs must equal the declared slice exactly\"",
    );
    for mutation in 0..3 {
        let mut changed = production.dto.clone();
        match mutation {
            0 => {
                changed.outcomes.pop();
            }
            1 => changed.outcomes.push(changed.outcomes[0].clone()),
            2 => changed.outcomes[0].claim = NodeId::LangevinCoefficientOne,
            _ => unreachable!(),
        }
        check(
            &production,
            &changed,
            &domain(
                RuleId::M025,
                vec![],
                DraftLocation::Provenance,
                "exactly one row per present claim and none for quantities is required",
            ),
            "retention provenance M025 at file \"generated/audit-report.json\": domain stage=report-consumption ids=[] edge=none location=provenance explanation=\"exactly one row per present claim and none for quantities is required\"",
        );
    }
    for mutation in 0..3 {
        let mut changed = production.dto.clone();
        let wire::Verdict::Certified(receipt) = &mut changed.outcomes[0].verdict else {
            unreachable!()
        };
        match mutation {
            0 => receipt.revision += 1,
            1 => receipt.producer = ftoo_manifest::ProducerId::LangevinCoefficientThreeValue,
            2 => receipt.budget = ftoo_manifest::BudgetId::CoefficientThreeV1,
            _ => unreachable!(),
        }
        check(
            &production,
            &changed,
            &domain(
                RuleId::M025,
                vec![NodeId::LangevinCoefficientOneValue],
                DraftLocation::Verdict(0),
                "claim, producer, revision, budget and pin must match",
            ),
            "retention provenance M025 at file \"generated/audit-report.json\": domain stage=report-consumption ids=[\"claim:langevin:coefficient-one-value\"] edge=none location=verdict(0) explanation=\"claim, producer, revision, budget and pin must match\"",
        );
    }
    let mut changed = production.dto.clone();
    changed.outcomes[1].verdict = wire::Verdict::Deferred(ftoo_manifest::DeferredReason::Premise {
        claim: NodeId::LangevinCoefficientOneValue,
        evidence: ftoo_manifest::VerdictClass::Refuted,
    });
    check(
        &production,
        &changed,
        &domain(
            RuleId::M028,
            vec![NodeId::LangevinCoefficientThreeValue],
            DraftLocation::Verdict(1),
            "outcome must carry precisely its derived blocker",
        ),
        "retention provenance M028 at file \"generated/audit-report.json\": domain stage=report-consumption ids=[\"claim:langevin:coefficient-three-value\"] edge=none location=verdict(1) explanation=\"outcome must carry precisely its derived blocker\"",
    );
}

fn after_production(
    production: &Arc<Produced>,
    changed: &wire::Report,
    expected: &RetentionError,
    display: &str,
) {
    let mut calls = Vec::new();
    let mut memo = VecDeque::new();
    let file = super::file_tests::FileFixture::new(production);
    let actual = acceptance::accept_with(
        production.key.projection.clone(),
        file.write_read(&wire::write(changed).unwrap()),
        &mut memo,
        &mut |_, run| {
            calls.push(run);
            Ok(Arc::clone(production))
        },
    )
    .unwrap_err();
    assert_eq!(actual, *expected);
    assert_eq!(actual.to_string(), display);
    assert_eq!(calls, vec![Run::Fresh1]);
    assert_eq!(memo.len(), 0);
    let restored = acceptance::accept_with(
        production.key.projection.clone(),
        file.write_read(&production.bytes),
        &mut memo,
        &mut |_, _| Ok(Arc::clone(production)),
    )
    .unwrap();
    assert_eq!(restored.bytes(), production.bytes);
}
#[test]
fn status_and_assessment_assertions_never_promote_file_fields() {
    let production = control();
    let mut changed = production.dto.clone();
    changed.statuses[0].maturity = ftoo_manifest::Maturity::Retained;
    after_production(
        &production,
        &changed,
        &RetentionError::StatusAssertion {
            index: 0,
            axis: StatusAxis::Maturity,
            expected: production.dto.statuses[0].clone(),
            actual: changed.statuses[0].clone(),
        },
        "retention assertions M012 at json-path \"$.statuses[0]\": assertion-mismatch axis=maturity node=\"claim:langevin:coefficient-one-value\" expected={\"id\":\"claim:langevin:coefficient-one-value\",\"maturity\":\"proposed\",\"identification\":\"identified\",\"calibration\":\"not-applicable\",\"evidence\":\"certified\"} actual={\"id\":\"claim:langevin:coefficient-one-value\",\"maturity\":\"retained\",\"identification\":\"identified\",\"calibration\":\"not-applicable\",\"evidence\":\"certified\"}",
    );
    changed = production.dto.clone();
    changed.statuses[0].evidence = crate::AuditedEvidence::Deferred;
    after_production(
        &production,
        &changed,
        &RetentionError::StatusAssertion {
            index: 0,
            axis: StatusAxis::Evidence,
            expected: production.dto.statuses[0].clone(),
            actual: changed.statuses[0].clone(),
        },
        "retention assertions M012 at json-path \"$.statuses[0]\": assertion-mismatch axis=evidence node=\"claim:langevin:coefficient-one-value\" expected={\"id\":\"claim:langevin:coefficient-one-value\",\"maturity\":\"proposed\",\"identification\":\"identified\",\"calibration\":\"not-applicable\",\"evidence\":\"certified\"} actual={\"id\":\"claim:langevin:coefficient-one-value\",\"maturity\":\"proposed\",\"identification\":\"identified\",\"calibration\":\"not-applicable\",\"evidence\":\"deferred\"}",
    );
    changed = production.dto.clone();
    changed.assessment.exit_code = 4;
    after_production(
        &production,
        &changed,
        &RetentionError::AssessmentAssertion {
            expected: production.dto.assessment.clone(),
            actual: changed.assessment.clone(),
        },
        "retention assertions M015 at json-path \"$.assessment\": assertion-mismatch expected={\"disposition\":\"pass\",\"exit-code\":0} actual={\"disposition\":\"pass\",\"exit-code\":4}",
    );
}

#[test]
fn exact_projection_identity_includes_descriptions_edges_coverage_and_maturity() {
    use ftoo_manifest::{Maturity, NodePayload};
    let production = control();
    let mut drafts = Vec::new();
    for index in 0..4 {
        let mut draft = ftoo_manifest::seed_draft();
        match &mut draft.nodes[index].payload {
            NodePayload::Quantity { description } | NodePayload::Claim { description, .. } => {
                description.push('!');
            }
        }
        drafts.push(draft);
        let mut draft = ftoo_manifest::seed_draft();
        draft.nodes[index].maturity = Maturity::Retained;
        drafts.push(draft);
    }
    let mut draft = ftoo_manifest::seed_draft();
    draft.coverage.slice.clear();
    drafts.push(draft);
    let mut draft = ftoo_manifest::seed_draft();
    draft
        .edges
        .retain(|edge| edge.relation != ftoo_manifest::RelationKind::CompatibleWith);
    drafts.push(draft);
    for edge in ftoo_manifest::seed_draft().edges {
        if edge.relation == ftoo_manifest::RelationKind::DependsOn {
            let mut draft = ftoo_manifest::seed_draft();
            draft.edges.retain(|candidate| *candidate != edge);
            drafts.push(draft);
        }
    }
    for draft in drafts {
        let graph = ftoo_manifest::validate(draft).unwrap();
        let current = ftoo_manifest::projection_bytes(&graph).unwrap();
        let file = super::file_tests::FileFixture::new(&production);
        let actual = file
            .refuse_before_production(current.clone(), &production.bytes)
            .unwrap_err();
        let mut difference = super::matrix_tests::expected_difference(
            &current,
            &production.key.projection,
            InfoRecord::collect(Run::Retained),
            InfoRecord::collect(Run::Retained),
        );
        difference.path = "$.provenance.projection-utf8".into();
        let display =
            super::classifier_tests::difference_display_labels(&difference, "current", "retained");
        let expected = RetentionError::ProvenanceMismatch {
            component: Component::ProjectionBytes,
            path: difference.path.clone(),
            difference: ProvenanceDifference::Bytes(Box::new(difference)),
        };
        assert_eq!(actual, expected);
        assert_eq!(
            actual.to_string(),
            format!(
                "retention provenance M061 at json-path \"$.provenance.projection-utf8\": provenance-mismatch component=projection-bytes {display}"
            )
        );
        let restored = accept(production.key.projection.clone(), production.bytes.clone()).unwrap();
        assert_eq!(restored.bytes(), production.bytes);
    }
}

#[test]
fn forged_current_provenance_does_not_promote_old_status_assertions() {
    let original = control();
    let mut draft = ftoo_manifest::seed_draft();
    draft
        .nodes
        .iter_mut()
        .find(|node| node.id == ftoo_manifest::NodeId::LangevinCoefficientOneValue)
        .unwrap()
        .maturity = ftoo_manifest::Maturity::Retained;
    let graph = ftoo_manifest::validate(draft).unwrap();
    let projection = ftoo_manifest::projection_bytes(&graph).unwrap();
    let fresh = produce(projection).unwrap();
    let mut forged = original.dto.clone();
    forged.projection.clone_from(&fresh.dto.projection);
    forged.digest.clone_from(&fresh.dto.digest);
    after_production(
        &fresh,
        &forged,
        &RetentionError::StatusAssertion {
            index: 0,
            axis: StatusAxis::Maturity,
            expected: fresh.dto.statuses[0].clone(),
            actual: original.dto.statuses[0].clone(),
        },
        "retention assertions M012 at json-path \"$.statuses[0]\": assertion-mismatch axis=maturity node=\"claim:langevin:coefficient-one-value\" expected={\"id\":\"claim:langevin:coefficient-one-value\",\"maturity\":\"retained\",\"identification\":\"identified\",\"calibration\":\"not-applicable\",\"evidence\":\"certified\"} actual={\"id\":\"claim:langevin:coefficient-one-value\",\"maturity\":\"proposed\",\"identification\":\"identified\",\"calibration\":\"not-applicable\",\"evidence\":\"certified\"}",
    );
}

#[test]
fn status_domain_keeps_missing_duplicate_and_extra_rows() {
    let production = control();
    let expected: Vec<_> = production
        .dto
        .statuses
        .iter()
        .map(|status| status.id)
        .collect();
    for index in 0..3 {
        let mut changed = production.dto.clone();
        match index {
            0 => {
                changed.statuses.pop();
            }
            1 => changed.statuses.push(changed.statuses[0].clone()),
            2 => changed.statuses[3].id = changed.statuses[2].id,
            _ => unreachable!(),
        }
        changed.statuses.sort_by_key(|status| status.id);
        let actual: Vec<_> = changed.statuses.iter().map(|status| status.id).collect();
        let names = |ids: &[ftoo_manifest::NodeId]| {
            serde_json::to_string(&ids.iter().map(|id| id.semantic_name()).collect::<Vec<_>>())
                .unwrap()
        };
        check(
            &production,
            &changed,
            &RetentionError::StatusDomain {
                expected: expected.clone(),
                actual: actual.clone(),
            },
            &format!(
                "retention provenance M012 at json-path \"$.statuses\": status-domain expected={} actual={}",
                names(&expected),
                names(&actual)
            ),
        );
    }
}

fn summary_text(summary: &wire::Summary) -> String {
    let q = |text: &str| serde_json::to_string(text).unwrap();
    let jet = summary
        .jet
        .iter()
        .map(|rational| {
            format!(
                "{{\"numerator\":{},\"denominator\":{}}}",
                q(&rational.numerator),
                q(&rational.denominator)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"subject\":{},\"jet\":[{jet}],\"source\":{},\"clause\":{},\"order\":{},\"derivative-order\":{},\"residual-order\":{},\"equation-degrees\":{},\"ode\":{},\"region\":\"ambient-product-x-no-assumptions\",\"analytic-scope\":{}}}",
        q(&summary.subject),
        q(&summary.source),
        q(&summary.clause),
        summary.order,
        summary.derivative_order,
        summary.residual_order,
        serde_json::to_string(&summary.equation_degrees).unwrap(),
        q(&summary.ode),
        q(&summary.analytic_scope)
    )
}
#[test]
fn fixed_summary_fields_are_assertions_not_engine_inputs() {
    let production = control();
    let wire::Verdict::Certified(receipt) = &production.dto.outcomes[0].verdict else {
        unreachable!()
    };
    let expected = receipt.summary.clone();
    for index in 0..14 {
        let mut changed = production.dto.clone();
        let wire::Verdict::Certified(receipt) = &mut changed.outcomes[0].verdict else {
            unreachable!()
        };
        let summary = &mut receipt.summary;
        match index {
            0 => summary.subject.push('!'),
            1 => summary.source.push('!'),
            2 => summary.clause.push('!'),
            3 => summary.order += 1,
            4 => summary.derivative_order += 1,
            5 => summary.residual_order += 1,
            6 => summary.ode.push('!'),
            7 => summary.analytic_scope.push('!'),
            8 => summary.equation_degrees[0] += 1,
            9..=13 => {
                summary.jet[index - 9] = wire::Rational {
                    numerator: "2".into(),
                    denominator: "7".into(),
                }
            }
            _ => unreachable!(),
        }
        let actual = summary.clone();
        after_production(
            &production,
            &changed,
            &mismatch(
                Component::Summary,
                "$.outcomes[0].verdict.receipt.summary",
                ProvenanceValue::Summary(Box::new(expected.clone())),
                ProvenanceValue::Summary(Box::new(actual.clone())),
            ),
            &format!(
                "retention provenance M025 at json-path \"$.outcomes[0].verdict.receipt.summary\": provenance-mismatch component=summary expected={} actual={}",
                summary_text(&expected),
                summary_text(&actual)
            ),
        );
    }
}

#[test]
fn independent_receipt_faults_are_complete_and_sorted_by_owning_rule() {
    let production = control();
    let mut changed = production.dto.clone();
    let wire::Verdict::Certified(receipt) = &mut changed.outcomes[0].verdict else {
        unreachable!()
    };
    receipt.pin = "0".repeat(40);
    receipt.target = 2;
    receipt
        .limits
        .iter_mut()
        .find(|(name, _)| name == "work")
        .unwrap()
        .1 = 3_000_001;
    let root = "$.outcomes[0].verdict.receipt";
    let faults = [
        (
            ftoo_manifest::RuleId::M025,
            "pin",
            Component::Pin,
            ProvenanceValue::Text(ftoo_manifest::ENGINE_PIN.0.into()),
            ProvenanceValue::Text("0".repeat(40)),
        ),
        (
            ftoo_manifest::RuleId::M025,
            "target",
            Component::Target,
            ProvenanceValue::U64(1),
            ProvenanceValue::U64(2),
        ),
    ];
    let diagnostics = faults
        .into_iter()
        .map(|(rule, field, component, expected, actual)| {
            let path = format!("{root}.{field}");
            RetentionDiagnostic {
                rule,
                stage: Stage::Provenance,
                site: Site::JsonPath(path.clone()),
                fault: mismatch(component, &path, expected, actual),
            }
        })
        .collect();
    let display = "retention provenance M025 at json-path \"$.outcomes[0].verdict.receipt.pin\": provenance-mismatch component=pin expected=\"598ff683a57caca07cf9d44e3fd28c716afb81ef\" actual=\"0000000000000000000000000000000000000000\"\nretention provenance M025 at json-path \"$.outcomes[0].verdict.receipt.target\": provenance-mismatch component=target expected=1 actual=2";
    check(
        &production,
        &changed,
        &RetentionError::Multiple(diagnostics),
        display,
    );
}
