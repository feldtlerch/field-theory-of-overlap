use std::collections::BTreeSet;

use crate::{
    Awaiting, Calibration, Diagnostic, DraftLocation, Edge, Evidence, Guard, Identification,
    ManifestDraft, Maturity, NodeId, NodeKind, NodePayload, RULES, RelationKind, ReleaseGate,
    RuleId, RuleState, seed, seed_draft, validate,
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
            NodePayload::Claim { awaits, .. } => {
                assert_eq!(status.evidence(), Evidence::Deferred);
                assert_eq!(*awaits, Awaiting::AuditBinding);
            }
        }
    }
    assert_eq!(graph.coverage().slice, []);
    assert!(!graph.coverage().closed_world.referents);
    assert!(!graph.coverage().closed_world.relations);
    assert!(!graph.coverage().closed_world.audit);
    assert_eq!(graph.coverage().release_gate, ReleaseGate::ExplicitPartial);
    assert!(graph.coverage().is_explicit_partial());
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
        awaits: Awaiting::AuditBinding,
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
    assert_eq!(draft.coverage.slice, []);
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
fn m014_registered_claim_is_not_audited() -> TestResult {
    for id in [OneValue, ThreeValue] {
        let mut draft = seed_draft();
        draft.coverage.slice = vec![id];
        assert!(!draft.coverage.is_explicit_partial());
        let diagnostics = refused(draft, &[RuleId::M014])?;
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].ids, [id]);
    }
    Ok(())
}

#[test]
fn m015_each_closure_assertion_and_complete_gate() -> TestResult {
    for flag in 0..4 {
        let mut draft = seed_draft();
        match flag {
            0 => draft.coverage.closed_world.referents = true,
            1 => draft.coverage.closed_world.relations = true,
            2 => draft.coverage.closed_world.audit = true,
            _ => draft.coverage.release_gate = ReleaseGate::Complete,
        }
        assert!(!draft.coverage.is_explicit_partial());
        let diagnostics = refused(draft, &[RuleId::M015])?;
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].location, DraftLocation::Coverage);
    }
    Ok(())
}

#[test]
fn m015_empty_graph_partial_passes_complete_refused() -> TestResult {
    let mut draft = seed_draft();
    draft.nodes.clear();
    draft.edges.clear();
    let graph = validate(draft.clone())?;
    assert!(graph.coverage().is_explicit_partial());
    assert_eq!(graph.dependency_order(DependsOn)?, []);
    assert_eq!(graph.node(One), None);
    assert_eq!(graph.status(One), None);
    draft.coverage.release_gate = ReleaseGate::Complete;
    refused(draft, &[RuleId::M015])?;
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
fn register_has_exact_m1a_allocation_and_deferral_reasons() -> TestResult {
    use RuleId::{
        M001, M002, M003, M004, M005, M006, M007, M008, M009, M010, M011, M012, M013, M014, M015,
        M016, M017, M018, M019, M020, M021,
    };
    assert_eq!(
        RULES.map(|rule| rule.number),
        [
            M001, M002, M003, M004, M005, M006, M007, M008, M009, M010, M011, M012, M013, M014,
            M015, M016, M017, M018, M019, M020, M021
        ]
    );
    for rule in RULES {
        let expected = match rule.number {
            M005 | M011 => RuleState::Structural,
            M006 | M007 | M008 | M009 | M010 | M013 | M014 | M015 | M021 => RuleState::Enforced,
            M001 | M002 | M003 | M004 | M012 => RuleState::DeferredM1b,
            M016 | M017 | M018 | M019 => RuleState::DeferredM2,
            M020 => RuleState::DeferredM3,
        };
        assert_eq!(rule.state, expected);
        assert!(!rule.statement.is_empty() && !rule.scope.is_empty());
        match expected {
            RuleState::Structural | RuleState::Enforced => assert_eq!(rule.deferred_reason, None),
            RuleState::DeferredM1b | RuleState::DeferredM2 | RuleState::DeferredM3 => {
                assert_ne!(rule.deferred_reason.ok_or("missing reason")?, "");
            }
        }
    }
    Ok(())
}
