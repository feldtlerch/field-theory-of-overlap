use std::collections::BTreeSet;

use crate::{
    Awaiting, Blocker, Calibration, ClaimAudit, Diagnostic, DraftLocation, Edge, Evidence, Guard,
    Identification, ManifestDraft, Maturity, NodeId, NodeKind, NodePayload, RULES, RelationKind,
    ReleaseGate, RuleId, RuleState, seed, seed_draft, validate,
};
use NodeId::{
    LangevinCoefficientOne as One, LangevinCoefficientOneValue as OneValue,
    LangevinCoefficientThree as Three, LangevinCoefficientThreeValue as ThreeValue,
};
use RelationKind::{CompatibleWith, DependsOn};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn edge(relation: RelationKind, source: NodeId, target: NodeId) -> Edge {
    Edge {
        relation,
        source,
        target,
        guard: Guard::Always,
    }
}

// The exact rule set matters: an unrelated refusal cannot pass a negative.
fn refused(draft: ManifestDraft, expected: &[RuleId]) -> TestResult<Vec<Diagnostic>> {
    match validate(draft) {
        Ok(_) => Err("mutation unexpectedly admitted".into()),
        Err(errors) => {
            let actual: BTreeSet<_> = errors.diagnostics().iter().map(|d| d.rule).collect();
            assert_eq!(actual, expected.iter().copied().collect());
            assert!(
                errors
                    .diagnostics()
                    .iter()
                    .all(|d| d.stage == crate::AdmissionStage::GraphAdmission)
            );
            assert!(
                errors
                    .diagnostics()
                    .iter()
                    .all(|d| !d.explanation.is_empty())
            );
            Ok(errors.diagnostics().to_vec())
        }
    }
}

#[test]
fn seed_admission_identity_kinds_edges_status_and_coverage() -> TestResult {
    let graph = validate(seed_draft())?;
    assert_eq!(graph, seed()?);
    assert_eq!(graph.nodes().len(), 4);
    let ids: BTreeSet<_> = graph.nodes().iter().map(|node| node.id).collect();
    assert_eq!(
        ids,
        [One, Three, OneValue, ThreeValue].into_iter().collect()
    );
    assert_eq!(
        graph
            .nodes()
            .iter()
            .filter(|n| n.payload.kind() == NodeKind::Quantity)
            .count(),
        2
    );
    assert_eq!(
        graph
            .nodes()
            .iter()
            .filter(|n| n.payload.kind() == NodeKind::Claim)
            .count(),
        2
    );
    let expected = [
        edge(DependsOn, Three, One),
        edge(DependsOn, OneValue, One),
        edge(DependsOn, ThreeValue, Three),
        edge(DependsOn, ThreeValue, OneValue),
        edge(CompatibleWith, One, Three),
        edge(CompatibleWith, Three, One),
    ];
    assert_eq!(graph.edges().len(), 6);
    assert_eq!(
        graph.edges().iter().copied().collect::<BTreeSet<_>>(),
        expected.into_iter().collect()
    );
    for node in graph.nodes() {
        let status = graph.status(node.id).ok_or("missing status")?;
        assert_eq!(status.maturity(), Maturity::Proposed);
        assert_eq!(status.identification(), Identification::Identified);
        assert_eq!(status.calibration(), Calibration::NotApplicable);
        match &node.payload {
            NodePayload::Quantity { .. } => assert_eq!(status.evidence(), Evidence::NotAClaim),
            NodePayload::Claim { audit, .. } => {
                assert_eq!(status.evidence(), Evidence::Deferred);
                assert!(matches!(audit, ClaimAudit::Bound { .. }));
                assert_eq!(status.blocker(), Some(Blocker::AuditRunRequired));
            }
        }
    }
    assert_eq!(graph.coverage().slice, [OneValue, ThreeValue]);
    assert!(!graph.coverage().closed_world.referents);
    assert!(!graph.coverage().closed_world.relations);
    assert!(graph.coverage().closed_world.audit);
    assert_eq!(graph.coverage().release_gate, ReleaseGate::ExplicitPartial);
    Ok(())
}

#[test]
fn semantic_name_table_and_exhaustive_payload_kinds() -> TestResult {
    assert_eq!(NodeId::ALL, [One, Three, OneValue, ThreeValue]);
    assert_eq!(NodeKind::ALL, [NodeKind::Quantity, NodeKind::Claim]);
    for id in NodeId::ALL {
        let (name, kind) = match id {
            One => ("def:langevin:coefficient-one", NodeKind::Quantity),
            Three => ("def:langevin:coefficient-three", NodeKind::Quantity),
            OneValue => ("claim:langevin:coefficient-one-value", NodeKind::Claim),
            ThreeValue => ("claim:langevin:coefficient-three-value", NodeKind::Claim),
        };
        assert_eq!(id.semantic_name(), name);
        assert_eq!(id.expected_kind(), kind);
        assert_eq!(seed()?.node(id).ok_or("missing node")?.payload.kind(), kind);
    }
    Ok(())
}

#[test]
fn dependency_order_is_independently_stated() -> TestResult {
    assert_eq!(
        seed()?.dependency_order(DependsOn)?,
        [One, OneValue, Three, ThreeValue]
    );
    let mut reordered = seed_draft();
    reordered.nodes.reverse();
    reordered.edges.reverse();
    assert_eq!(
        validate(reordered)?.dependency_order(DependsOn)?,
        [One, OneValue, Three, ThreeValue]
    );
    match seed()?.dependency_order(CompatibleWith) {
        Err(error) => assert_eq!(error.relation, CompatibleWith),
        Ok(_) => return Err("compatibility order unexpectedly supported".into()),
    }
    Ok(())
}

#[test]
fn m006_duplicate_identity() -> TestResult {
    let mut draft = seed_draft();
    draft.nodes.push(draft.nodes[0].clone());
    let diagnostics = refused(draft, &[RuleId::M006])?;
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|d| d.ids == [One]));
    Ok(())
}

#[test]
fn m006_payload_identity_mismatch_suppresses_dependent_checks() -> TestResult {
    let mut draft = seed_draft();
    draft.nodes[0].payload = NodePayload::Claim {
        description: "Wrong payload".into(),
        audit: ClaimAudit::Awaiting(Awaiting::AuditBinding),
    };
    let diagnostics = refused(draft, &[RuleId::M006])?;
    assert_eq!(diagnostics[0].ids, [One]);
    assert_eq!(diagnostics[0].location, DraftLocation::Node(0));
    Ok(())
}

#[test]
fn m006_blank_description_for_each_payload() -> TestResult {
    for id in NodeId::ALL {
        let mut draft = seed_draft();
        let node = draft
            .nodes
            .iter_mut()
            .find(|node| node.id == id)
            .ok_or("missing node")?;
        match &mut node.payload {
            NodePayload::Quantity { description } | NodePayload::Claim { description, .. } => {
                *description = " \t\n\u{2003}".into();
            }
        }
        let diagnostics = refused(draft, &[RuleId::M006])?;
        assert_eq!(diagnostics[0].ids, [id]);
    }
    Ok(())
}

#[test]
fn m007_missing_endpoints_ignore_false_closure_flags() -> TestResult {
    let mut draft = seed_draft();
    draft.nodes.retain(|node| node.id != One);
    let diagnostics = refused(draft, &[RuleId::M007])?;
    assert_eq!(diagnostics.len(), 4);
    assert!(
        diagnostics
            .iter()
            .all(|d| d.ids == [One] && d.edge.is_some())
    );
    Ok(())
}

#[test]
fn m007_absent_slice_member_precedes_shape_and_eligibility() -> TestResult {
    let mut draft = seed_draft();
    draft.nodes.retain(|node| node.id != OneValue);
    draft
        .edges
        .retain(|e| e.source != OneValue && e.target != OneValue);
    draft.coverage.slice = vec![OneValue, OneValue];
    let diagnostics = refused(draft, &[RuleId::M007])?;
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|d| d.ids == [OneValue]));
    Ok(())
}

#[test]
fn m008_reverse_claim_dependency() -> TestResult {
    let mut draft = seed_draft();
    draft.edges[1] = draft.edges[1].reverse();
    let diagnostics = refused(draft, &[RuleId::M008])?;
    assert_eq!(diagnostics[0].edge, Some(edge(DependsOn, One, OneValue)));
    Ok(())
}

#[test]
fn m008_claim_compatibility_suppresses_m021() -> TestResult {
    let mut draft = seed_draft();
    draft.edges.push(edge(CompatibleWith, OneValue, One));
    let diagnostics = refused(draft, &[RuleId::M008])?;
    assert_eq!(
        diagnostics[0].edge,
        Some(edge(CompatibleWith, OneValue, One))
    );
    Ok(())
}

#[test]
fn exhaustive_relation_endpoint_matrix() -> TestResult {
    assert_eq!(RelationKind::ALL, [DependsOn, CompatibleWith]);
    for relation in RelationKind::ALL {
        let contract = relation.contract();
        assert!(!contract.allow_self);
        assert_eq!(contract.acyclic, relation == DependsOn);
        assert_eq!(contract.symmetric, relation == CompatibleWith);
        for source_kind in NodeKind::ALL {
            for target_kind in NodeKind::ALL {
                let allowed = match (relation, source_kind, target_kind) {
                    (DependsOn | CompatibleWith, NodeKind::Quantity, NodeKind::Quantity)
                    | (DependsOn, NodeKind::Claim, NodeKind::Quantity | NodeKind::Claim) => true,
                    (DependsOn | CompatibleWith, NodeKind::Quantity, NodeKind::Claim)
                    | (CompatibleWith, NodeKind::Claim, NodeKind::Quantity | NodeKind::Claim) => {
                        false
                    }
                };
                assert_eq!(
                    contract.endpoints.contains(&(source_kind, target_kind)),
                    allowed
                );
                let source = match source_kind {
                    NodeKind::Quantity => One,
                    NodeKind::Claim => OneValue,
                };
                let target = match target_kind {
                    NodeKind::Quantity => Three,
                    NodeKind::Claim => ThreeValue,
                };
                let mut draft = seed_draft();
                draft.edges = vec![edge(relation, source, target)];
                if contract.symmetric {
                    draft.edges.push(edge(relation, target, source));
                }
                if allowed {
                    validate(draft)?;
                } else {
                    let diagnostics = refused(draft, &[RuleId::M008])?;
                    assert!(diagnostics.iter().all(|d| d.edge.is_some()));
                }
            }
        }
    }
    Ok(())
}

#[test]
fn m009_duplicate_tuple_for_each_relation() -> TestResult {
    for relation in RelationKind::ALL {
        let mut draft = seed_draft();
        let duplicate = *draft
            .edges
            .iter()
            .find(|e| e.relation == relation)
            .ok_or("missing relation")?;
        draft.edges.push(duplicate);
        let diagnostics = refused(draft, &[RuleId::M009])?;
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|d| d.edge == Some(duplicate)));
    }
    Ok(())
}

#[test]
fn m009_self_edges_precede_cycles_and_symmetry() -> TestResult {
    for relation in RelationKind::ALL {
        let mut draft = seed_draft();
        draft.edges.push(edge(relation, One, One));
        let diagnostics = refused(draft, &[RuleId::M009])?;
        assert_eq!(diagnostics[0].edge, Some(edge(relation, One, One)));
    }
    Ok(())
}

#[test]
fn m010_dependency_cycle_outside_empty_slice() -> TestResult {
    let mut draft = seed_draft();
    draft.coverage.slice.clear();
    draft.edges.push(edge(DependsOn, One, Three));
    let diagnostics = refused(draft, &[RuleId::M010])?;
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].ids, [One, Three, One]);
    assert_eq!(
        diagnostics[0].location,
        DraftLocation::Cycle {
            relation: DependsOn,
            edges: vec![6, 0]
        }
    );
    assert_eq!(
        diagnostics[0].explanation,
        "depends-on cycle: def:langevin:coefficient-one -> def:langevin:coefficient-three -> def:langevin:coefficient-one"
    );
    Ok(())
}

#[test]
fn m010_multiple_cycles_select_smallest_semantic_member() -> TestResult {
    let mut draft = seed_draft();
    draft.edges.push(edge(DependsOn, One, Three));
    draft.edges.push(edge(DependsOn, OneValue, ThreeValue));
    let first = refused(draft.clone(), &[RuleId::M010])?;
    assert_eq!(first[0].ids, [OneValue, ThreeValue, OneValue]);
    draft.nodes.reverse();
    draft.edges.reverse();
    let second = refused(draft, &[RuleId::M010])?;
    assert_eq!(semantic_diagnostics(&first), semantic_diagnostics(&second));
    Ok(())
}

#[test]
fn m009_multiplicity_does_not_hide_independent_cycle() -> TestResult {
    let mut draft = seed_draft();
    draft.edges.push(edge(DependsOn, One, Three));
    draft.edges.push(edge(DependsOn, One, Three));
    let diagnostics = refused(draft, &[RuleId::M009, RuleId::M010])?;
    let cycle = diagnostics
        .iter()
        .find(|d| d.rule == RuleId::M010)
        .ok_or("missing cycle")?;
    assert_eq!(cycle.ids, [One, Three, One]);
    Ok(())
}

#[test]
fn permitted_compatibility_cycle_and_cross_relation_cycle() -> TestResult {
    let graph = seed()?;
    // Three -> One is shared by two relations, while One -> Three is only
    // compatibility. The union cycles but the dependency subgraph does not.
    assert!(graph.edges().contains(&edge(DependsOn, Three, One)));
    assert!(graph.edges().contains(&edge(CompatibleWith, Three, One)));
    assert!(graph.edges().contains(&edge(CompatibleWith, One, Three)));
    Ok(())
}

#[test]
fn disconnected_dependency_component_and_isolated_nodes() -> TestResult {
    let mut draft = seed_draft();
    draft.edges.retain(|e| {
        *e != edge(DependsOn, Three, One) && *e != edge(DependsOn, ThreeValue, OneValue)
    });
    assert_eq!(
        validate(draft.clone())?.dependency_order(DependsOn)?,
        [One, OneValue, Three, ThreeValue]
    );
    draft.edges.clear();
    assert_eq!(
        validate(draft)?.dependency_order(DependsOn)?,
        [OneValue, ThreeValue, One, Three]
    );
    Ok(())
}

#[test]
fn m011_retained_maturity_does_not_promote_derived_axes() -> TestResult {
    let before = seed()?;
    let mut draft = seed_draft();
    for node in &mut draft.nodes {
        node.maturity = Maturity::Retained;
    }
    let after = validate(draft)?;
    for id in NodeId::ALL {
        let old = before.status(id).ok_or("missing old status")?;
        let new = after.status(id).ok_or("missing new status")?;
        assert_eq!(old.maturity(), Maturity::Proposed);
        assert_eq!(new.maturity(), Maturity::Retained);
        assert_eq!(new.identification(), old.identification());
        assert_eq!(new.calibration(), old.calibration());
        assert_eq!(new.evidence(), old.evidence());
    }
    Ok(())
}

#[test]
fn m013_duplicate_slice_precedes_eligibility() -> TestResult {
    let mut draft = seed_draft();
    draft.coverage.slice = vec![OneValue, OneValue];
    let diagnostics = refused(draft, &[RuleId::M013])?;
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|d| d.ids == [OneValue]));
    Ok(())
}

#[test]
fn m013_quantity_slice_precedes_eligibility() -> TestResult {
    let mut draft = seed_draft();
    draft.coverage.slice = vec![One];
    let diagnostics = refused(draft, &[RuleId::M013])?;
    assert_eq!(diagnostics[0].ids, [One]);
    Ok(())
}

#[test]
fn m014_awaiting_selected_claim_refused_at_graph_admission() -> TestResult {
    for id in [OneValue, ThreeValue] {
        let mut draft = seed_draft();
        draft.coverage.slice = vec![id];
        draft.coverage.closed_world.audit = false;
        awaiting(&mut draft, id);

        let diagnostics = refused(draft, &[RuleId::M014])?;
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].ids, [id]);
    }
    Ok(())
}

#[test]
fn m015_each_closure_assertion_and_complete_gate() -> TestResult {
    for flag in 0..3 {
        let mut draft = seed_draft();
        match flag {
            0 => draft.coverage.closed_world.referents = true,
            1 => draft.coverage.closed_world.relations = true,
            _ => draft.coverage.release_gate = ReleaseGate::Complete,
        }

        validate(draft)?; // Editorial assertions are admitted, not inferred.
    }
    Ok(())
}

#[test]
fn m015_empty_graph_declaration_defers_complete_to_consumer() -> TestResult {
    let mut draft = seed_draft();
    draft.coverage.slice.clear();
    draft.budgets.clear();
    draft.nodes.clear();
    draft.edges.clear();
    let graph = validate(draft.clone())?;
    assert_eq!(graph.dependency_order(DependsOn)?, []);
    assert_eq!(graph.node(One), None);
    assert_eq!(graph.status(One), None);
    draft.coverage.release_gate = ReleaseGate::Complete;
    validate(draft)?;
    Ok(())
}

#[test]
fn m021_each_missing_reverse_is_refused_restoration_passes() -> TestResult {
    for removed in [
        edge(CompatibleWith, One, Three),
        edge(CompatibleWith, Three, One),
    ] {
        let mut draft = seed_draft();
        draft.edges.retain(|e| *e != removed);
        let diagnostics = refused(draft.clone(), &[RuleId::M021])?;
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].edge, Some(removed.reverse()));
        assert!(diagnostics[0].explanation.contains(&format!(
            "{} -> {}",
            removed.source.semantic_name(),
            removed.target.semantic_name()
        )));
        draft.edges.push(removed);
        validate(draft)?;
    }
    let mut dependencies_only = seed_draft();
    dependencies_only.edges.retain(|e| e.relation == DependsOn);
    validate(dependencies_only)?;
    Ok(())
}

fn semantic_diagnostics(
    diagnostics: &[Diagnostic],
) -> Vec<(RuleId, Vec<NodeId>, Option<Edge>, String)> {
    diagnostics
        .iter()
        .map(|d| (d.rule, d.ids.clone(), d.edge, d.explanation.clone()))
        .collect()
}

#[test]
fn diagnostics_are_semantic_and_independent_defects_accumulate() -> TestResult {
    let mut draft = seed_draft();
    draft.edges.push(edge(DependsOn, One, Three));
    draft.edges.push(edge(DependsOn, One, OneValue));
    draft
        .edges
        .retain(|e| *e != edge(CompatibleWith, One, Three));
    draft.coverage.slice = vec![One, OneValue];
    awaiting(&mut draft, OneValue);
    draft.coverage.closed_world.audit = true;
    let rules = [
        RuleId::M008,
        RuleId::M010,
        RuleId::M013,
        RuleId::M014,
        RuleId::M015,
        RuleId::M021,
    ];
    let first = refused(draft.clone(), &rules)?;
    draft.nodes.reverse();
    draft.edges.reverse();
    draft.coverage.slice.reverse();
    let second = refused(draft, &rules)?;
    assert_eq!(semantic_diagnostics(&first), semantic_diagnostics(&second));
    assert_ne!(first, second); // Locations still refer to the actual drafts.
    assert!(first.windows(2).all(|pair| pair[0].rule <= pair[1].rule));
    Ok(())
}

#[test]
fn register_has_exact_m6a_allocation_and_deferral_reasons() -> TestResult {
    use RuleId::{
        M001, M002, M003, M004, M005, M006, M007, M008, M009, M010, M011, M012, M013, M014, M015,
        M016, M017, M018, M019, M020, M021, M022, M023, M024, M025, M026, M027, M028, M029, M030,
        M031, M032, M033, M034, M035, M036, M037, M038, M039, M040, M041, M042, M043, M044, M045,
        M046, M047, M048, M049, M050, M051, M052, M053, M054, M055, M056, M057, M058, M059, M060,
        M061, M062,
    };
    assert_eq!(
        RULES.map(|rule| rule.number),
        [
            M001, M002, M003, M004, M005, M006, M007, M008, M009, M010, M011, M012, M013, M014,
            M015, M016, M017, M018, M019, M020, M021, M022, M023, M024, M025, M026, M027, M028,
            M029, M030, M031, M032, M033, M034, M035, M036, M037, M038, M039, M040, M041, M042,
            M043, M044, M045, M046, M047, M048, M049, M050, M051, M052, M053, M054, M055, M056,
            M057, M058, M059, M060, M061, M062
        ]
    );
    for rule in RULES {
        let expected = match rule.number {
            M005 | M011 | M030 | M049 => RuleState::Structural,
            M006 | M007 | M008 | M009 | M010 | M013 | M014 | M015 | M020 | M021 | M022 | M023
            | M024 | M025 | M026 | M027 | M028 | M031 | M032 | M033 | M034 | M035 | M036 | M001
            | M002 | M003 | M004 | M012 | M037 | M038 | M039 | M040 | M041 | M042 | M043 | M044
            | M045 | M046 | M047 | M048 | M050 | M051 | M052 | M053 | M054 | M055 | M056 | M057
            | M058 | M059 | M060 | M061 | M062 => RuleState::Enforced,
            M016 | M017 | M018 | M019 => RuleState::DeferredM2,
            M029 => RuleState::DeferredExclusiveRoutes,
        };
        assert_eq!(rule.state, expected);
        assert!(!rule.statement.is_empty() && !rule.scope.is_empty());
        match expected {
            RuleState::Structural | RuleState::Enforced => assert_eq!(rule.deferred_reason, None),
            RuleState::DeferredM2 | RuleState::DeferredExclusiveRoutes => {
                assert_ne!(rule.deferred_reason.ok_or("missing reason")?, "");
            }
        }
    }
    Ok(())
}

fn awaiting(draft: &mut ManifestDraft, id: NodeId) {
    for node in &mut draft.nodes {
        if node.id == id
            && let NodePayload::Claim { audit, .. } = &mut node.payload
        {
            *audit = ClaimAudit::Awaiting(Awaiting::AuditBinding);
        }
    }
}
#[test]
fn m015_awaiting_outside_empty_slice_refuses_audit_closure() -> TestResult {
    let mut draft = seed_draft();
    draft.coverage.slice.clear();
    awaiting(&mut draft, OneValue);
    draft
        .budgets
        .retain(|b| b.producer.descriptor().claim != OneValue);
    let diagnostics = refused(draft, &[RuleId::M015])?;
    assert_eq!(diagnostics[0].stage, crate::AdmissionStage::GraphAdmission);
    Ok(())
}
#[test]
fn m024_adapter_support_is_not_engine_availability() -> TestResult {
    for capability in [
        crate::CapabilityId::FormalTruncatedPowerSeries,
        crate::CapabilityId::ExactRationalLinearEquivalence,
    ] {
        let mut draft = seed_draft();
        draft.coverage.slice.clear();
        draft.coverage.closed_world.audit = false;
        if let NodePayload::Claim { audit, .. } = &mut draft.nodes[2].payload {
            *audit = ClaimAudit::Awaiting(Awaiting::Capability(capability));
        }
        draft.budgets.remove(0);
        if capability == crate::CapabilityId::FormalTruncatedPowerSeries {
            let d = refused(draft, &[RuleId::M024])?;
            assert_eq!(d[0].stage, crate::AdmissionStage::GraphAdmission);
        } else {
            validate(draft)?;
        }
    }
    Ok(())
}
#[test]
fn register_pins_admission_statements_and_counts() {
    assert_eq!(RULES.len(), 62);
    assert_eq!(
        (
            RULES[..44].len(),
            RULES[..44]
                .iter()
                .filter(|r| r.state == RuleState::Enforced)
                .count(),
            RULES[..44]
                .iter()
                .filter(|r| r.state == RuleState::Structural)
                .count(),
            RULES[..44]
                .iter()
                .filter(|r| r.deferred_reason.is_some())
                .count(),
        ),
        (44, 36, 3, 5)
    );
    assert_eq!(
        RULES[44..46]
            .iter()
            .map(|r| (r.number, r.statement, r.scope, r.state, r.deferred_reason))
            .collect::<Vec<_>>(),
        vec![
            (
                RuleId::M045,
                "Unit texts never mint labels; bare label spans cite uniquely minted labels and doubled spans mention without harvesting or resolution.",
                "Every admitted statement text in every required language, with citations resolved in the joint label domain.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M046,
                "Within each statement unit, every language has equal addressed tag sets and equal cited label sets, compared separately from the shared metadata mint and independently of position or multiplicity.",
                "All language texts of each resolved unit after joint label and citation admission.",
                RuleState::Enforced,
                None
            ),
        ]
    );
    assert_eq!(
        RULES[42..44]
            .iter()
            .map(|r| (r.number, r.statement, r.scope, r.state, r.deferred_reason))
            .collect::<Vec<_>>(),
        vec![
            (
                RuleId::M043,
                "Every label has one mint across the permanent Rust identity catalog, Markdown mint occurrences and admitted unit metadata; Markdown citations resolve within the Markdown mint domain.",
                "The complete supplied Markdown corpus, every permanent NodeId and every admitted unit metadata mint.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M044,
                "Every Rust-held authority-record citation resolves to exactly one Markdown mint.",
                "Every history event in the supplied resolved corpus's AddressBook, including admission and retirement records.",
                RuleState::Enforced,
                None
            ),
        ]
    );
    assert_eq!(
        RULES[12].statement,
        "Every declared slice member is unique and is a claim; M007 resolves membership before this shape check."
    );
    assert_eq!(
        RULES[13].statement,
        "GraphAdmission requires each slice claim Bound; RegistryAdmission requires transitive claim premises selected; ReportConsumption requires reached IDs equal the declared slice."
    );
    assert_eq!(
        RULES[14].statement,
        "Audit closure true requires every claim Bound to its implementation; ExplicitPartial and Complete satisfy the stated report predicates, including nonvacuity."
    );
    assert_eq!(
        RULES
            .iter()
            .filter(|r| r.state == RuleState::Enforced)
            .count(),
        53
    );
    assert_eq!(
        RULES
            .iter()
            .filter(|r| r.state == RuleState::Structural)
            .count(),
        4
    );
    assert_eq!(
        RULES.iter().filter(|r| r.deferred_reason.is_some()).count(),
        5
    );
}

#[test]
fn exact_deferral_reasons() {
    use RuleId::*;
    let expected = [
        (
            M016,
            "M2 must admit typed weighting paths and their consumer.",
        ),
        (M017, "M2 must admit anchored readings and route checking."),
        (
            M018,
            "M2 must admit converters and their checking consumer.",
        ),
        (M019, "M2 must admit indexed spaces and operation typing."),
        (
            M029,
            "Exclusive routes await their first claim and isolation consumer.",
        ),
    ];
    assert_eq!(
        RULES
            .iter()
            .filter_map(|r| r.deferred_reason.map(|reason| (r.number, reason)))
            .collect::<Vec<_>>(),
        expected
    );
}

// These are explicitly synthetic classifications. Pure acceptance proves no mathematics.
fn synthetic_rows(graph: &crate::ValidatedManifest) -> Vec<crate::OutcomeDraft> {
    let mut rows = Vec::new();
    for id in graph.dependency_order(DependsOn).unwrap() {
        let node = graph.node(id).unwrap();
        if node.payload.kind() != NodeKind::Claim {
            continue;
        }
        let deferred = crate::expected_deferral(graph, node, &rows);
        rows.push(crate::OutcomeDraft::new(
            node,
            if deferred.is_some() {
                crate::VerdictClass::Deferred
            } else {
                crate::VerdictClass::Certified
            },
            deferred,
        ));
    }
    rows.sort_by_key(|r| r.claim);
    rows
}
fn exact_report_rules(
    graph: &crate::ValidatedManifest,
    snapshot: &crate::Snapshot,
    reached: &[NodeId],
    rows: &[crate::OutcomeDraft],
    expected: &[RuleId],
) {
    let errors = crate::report_diagnostics(graph, snapshot, reached, rows);
    assert_eq!(
        errors.iter().map(|d| d.rule).collect::<BTreeSet<_>>(),
        expected.iter().copied().collect()
    );
    assert!(
        errors
            .iter()
            .all(|d| d.stage == crate::AdmissionStage::ReportConsumption)
    );
}
#[test]
fn m025_missing_duplicate_and_quantity_rows_are_domain_errors_only() -> TestResult {
    let graph = seed()?;
    let snapshot = crate::Snapshot::capture(&graph);
    let rows = synthetic_rows(&graph);
    for mutation in 0..3 {
        let mut changed = rows.clone();
        match mutation {
            0 => {
                changed.pop();
            }
            1 => changed.push(changed[0].clone()),
            _ => changed[0].claim = One,
        }
        exact_report_rules(
            &graph,
            &snapshot,
            &graph.coverage().slice,
            &changed,
            &[RuleId::M025],
        );
    }
    Ok(())
}
#[test]
fn m025_each_snapshot_component_and_row_identity_is_pinned() -> TestResult {
    let graph = seed()?;
    let snapshot = crate::Snapshot::capture(&graph);
    let rows = synthetic_rows(&graph);
    for mutation in 0..12 {
        let mut changed = snapshot.clone();
        match mutation {
            0 => {
                let (NodePayload::Claim { description, .. }
                | NodePayload::Quantity { description }) = &mut changed.nodes[0].payload;
                description.push('!');
            }
            1 => changed.nodes[0].maturity = Maturity::Retained,
            2 => changed
                .edges
                .retain(|e| *e != edge(CompatibleWith, One, Three)),
            3 => changed
                .edges
                .retain(|e| *e != edge(CompatibleWith, Three, One)),
            4 => {
                let n = changed.nodes.iter_mut().find(|n| n.id == OneValue).unwrap();
                if let NodePayload::Claim { audit, .. } = &mut n.payload {
                    *audit = ClaimAudit::Awaiting(Awaiting::AuditBinding);
                }
            }
            5 => changed.budgets[0].pin = crate::EnginePin("stale"),
            6 => changed.capabilities[0].adapter_implemented = true,
            7 => changed.coverage.slice.pop().map(|_| ()).unwrap(),
            8 => changed.coverage.closed_world.referents = true,
            9 => changed.descriptors[0].revision += 1,
            10 => changed.pin = crate::EnginePin("stale"),
            _ => changed.descriptors[0].capabilities = &[crate::CapabilityId::SymbolicTrigonometry],
        }
        exact_report_rules(
            &graph,
            &changed,
            &graph.coverage().slice,
            &rows,
            &[RuleId::M025],
        );
    }
    for mutation in 0..4 {
        let mut changed = rows.clone();
        match mutation {
            0 => changed[0].producer = Some(crate::ProducerId::LangevinCoefficientThreeValue),
            1 => changed[0].revision = Some(99),
            2 => changed[0].budget = Some(crate::BudgetId::CoefficientThreeV1),
            _ => changed[0].pin = crate::EnginePin("stale"),
        }
        exact_report_rules(
            &graph,
            &snapshot,
            &graph.coverage().slice,
            &changed,
            &[RuleId::M025],
        );
    }
    let mut reordered = seed_draft();
    reordered.nodes.reverse();
    reordered.edges.reverse();
    reordered.budgets.reverse();
    reordered.coverage.slice.reverse();
    assert_eq!(snapshot, crate::Snapshot::capture(&validate(reordered)?));
    Ok(())
}
#[test]
fn m028_wrong_blocker_on_present_valid_row_is_not_m025() -> TestResult {
    let mut draft = seed_draft();
    draft.coverage.slice = vec![OneValue];
    let graph = validate(draft)?;
    let snapshot = crate::Snapshot::capture(&graph);
    let mut rows = synthetic_rows(&graph);
    exact_report_rules(&graph, &snapshot, &graph.coverage().slice, &rows, &[]);
    let row = rows.iter_mut().find(|r| r.claim == ThreeValue).unwrap();
    row.deferred = Some(crate::DeferredReason::Premise {
        claim: OneValue,
        evidence: crate::VerdictClass::Certified,
    });
    exact_report_rules(
        &graph,
        &snapshot,
        &graph.coverage().slice,
        &rows,
        &[RuleId::M028],
    );
    rows.iter_mut()
        .find(|r| r.claim == ThreeValue)
        .unwrap()
        .deferred = Some(crate::DeferredReason::OutsideSlice);
    exact_report_rules(&graph, &snapshot, &graph.coverage().slice, &rows, &[]);
    Ok(())
}
#[test]
fn m014_missing_reached_claim_fails_only_report_consumption() -> TestResult {
    let graph = seed()?;
    exact_report_rules(
        &graph,
        &crate::Snapshot::capture(&graph),
        &[OneValue],
        &synthetic_rows(&graph),
        &[RuleId::M014],
    );
    Ok(())
}
fn synthetic_assessment(
    graph: &crate::ValidatedManifest,
    rows: &[crate::OutcomeDraft],
    expected: crate::Disposition,
) {
    let result = crate::assess_draft(
        graph,
        &crate::Snapshot::capture(graph),
        &graph.coverage().slice,
        rows,
    );
    assert_eq!(result.disposition, expected);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.rule)
            .collect::<BTreeSet<_>>(),
        if expected == crate::Disposition::Pass {
            BTreeSet::new()
        } else {
            [RuleId::M015].into_iter().collect()
        }
    );
}
#[test]
fn m015_synthetic_complete_requires_each_closure_nonempty_and_total_certification() -> TestResult {
    use crate::{DeferredReason as B, Disposition as D, VerdictClass as V};
    let mut base = seed_draft();
    base.coverage.release_gate = ReleaseGate::Complete;
    base.coverage.closed_world = crate::ClosedWorld {
        referents: true,
        relations: true,
        audit: true,
    };
    let graph = validate(base.clone())?;
    synthetic_assessment(&graph, &synthetic_rows(&graph), D::Pass);
    for flag in 0..3 {
        let mut draft = base.clone();
        match flag {
            0 => draft.coverage.closed_world.referents = false,
            1 => draft.coverage.closed_world.relations = false,
            _ => draft.coverage.closed_world.audit = false,
        }
        let graph = validate(draft)?;
        synthetic_assessment(&graph, &synthetic_rows(&graph), D::Incomplete);
    }
    let mut draft = base.clone();
    draft.coverage.slice = vec![OneValue];
    let graph = validate(draft)?;
    synthetic_assessment(&graph, &synthetic_rows(&graph), D::Incomplete);
    for verdict in [V::Refuted, V::Undecided] {
        let graph = validate(base.clone())?;
        let mut rows = synthetic_rows(&graph);
        rows[0].verdict = verdict;
        rows[1].verdict = V::Deferred;
        rows[1].deferred = Some(B::Premise {
            claim: OneValue,
            evidence: verdict,
        });
        synthetic_assessment(
            &graph,
            &rows,
            if verdict == V::Refuted {
                D::MathematicalFailure
            } else {
                D::Incomplete
            },
        );
        let mut draft = base.clone();
        draft.coverage.release_gate = ReleaseGate::ExplicitPartial;
        let graph = validate(draft)?;
        synthetic_assessment(
            &graph,
            &rows,
            if verdict == V::Refuted {
                D::MathematicalFailure
            } else {
                D::Pass
            },
        );
    }
    let mut draft = base.clone();
    draft.coverage.slice.clear();
    let graph = validate(draft)?;
    synthetic_assessment(&graph, &synthetic_rows(&graph), D::Incomplete);
    let mut empty = base;
    empty.nodes.clear();
    empty.edges.clear();
    empty.budgets.clear();
    empty.coverage.slice.clear();
    let graph = validate(empty.clone())?;
    synthetic_assessment(&graph, &[], D::Incomplete);
    empty.coverage.release_gate = ReleaseGate::ExplicitPartial;
    let graph = validate(empty)?;
    synthetic_assessment(&graph, &[], D::Pass);
    Ok(())
}
#[test]
fn m028_awaiting_reasons_survive_empty_selection() -> TestResult {
    let mut draft = seed_draft();
    draft.coverage.slice.clear();
    draft.coverage.closed_world.audit = false;
    draft.budgets.clear();
    awaiting(&mut draft, OneValue);
    awaiting(&mut draft, ThreeValue);
    if let NodePayload::Claim { audit, .. } = &mut draft
        .nodes
        .iter_mut()
        .find(|n| n.id == ThreeValue)
        .unwrap()
        .payload
    {
        *audit = ClaimAudit::Awaiting(Awaiting::Capability(
            crate::CapabilityId::ExactRationalLinearEquivalence,
        ));
    }
    let graph = validate(draft)?;
    let rows = synthetic_rows(&graph);
    assert_eq!(rows[0].deferred, Some(crate::DeferredReason::AuditBinding));
    assert_eq!(
        rows[1].deferred,
        Some(crate::DeferredReason::Capability(
            crate::CapabilityId::ExactRationalLinearEquivalence
        ))
    );
    synthetic_assessment(&graph, &rows, crate::Disposition::Pass);
    Ok(())
}

#[test]
fn m015_awaiting_claim_prevents_complete_even_outside_slice() -> TestResult {
    let mut draft = seed_draft();
    draft.coverage.release_gate = ReleaseGate::Complete;
    draft.coverage.closed_world = crate::ClosedWorld {
        referents: true,
        relations: true,
        audit: false,
    };
    draft.coverage.slice = vec![OneValue];
    awaiting(&mut draft, ThreeValue);
    draft.budgets.pop();
    let graph = validate(draft)?;
    synthetic_assessment(
        &graph,
        &synthetic_rows(&graph),
        crate::Disposition::Incomplete,
    );
    Ok(())
}
