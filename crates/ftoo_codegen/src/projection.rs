//! Independent semantic inventory; comparison never serializes its operands.
use crate::ProjectionCheckError;
use ftoo_manifest::{
    CAPABILITIES, ClaimAudit, ENGINE_PIN, LoadedProjection, NodePayload, ProducerId,
    ProjectionDiagnostic, ProjectionError, ProjectionFault, ProjectionValue, ResolvedLimits,
    RuleId, SourceDifference, ValidatedManifest, load_projection, projection_bytes,
    registry_diagnostics, seed,
};

pub(crate) struct Prepared {
    pub graph: ValidatedManifest,
    pub bytes: Vec<u8>,
}
pub(crate) fn prepare_projection() -> Result<Prepared, ProjectionCheckError> {
    let graph = seed().map_err(ProjectionCheckError::SourceGraph)?;
    registry(&graph)?;
    let bytes = projection_bytes(&graph).map_err(ProjectionCheckError::Projection)?;
    compare(
        &graph,
        &load_projection(&bytes).map_err(ProjectionCheckError::Projection)?,
    )?;
    Ok(Prepared { graph, bytes })
}
pub(crate) fn registry(graph: &ValidatedManifest) -> Result<(), ProjectionCheckError> {
    let errors = registry_diagnostics(graph, &ProducerId::ALL.map(ProducerId::descriptor));
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ProjectionCheckError::Registry(errors))
    }
}
fn changed(
    rows: &mut Vec<SourceDifference>,
    path: String,
    expected: ProjectionValue,
    actual: ProjectionValue,
) {
    if expected != actual {
        rows.push(SourceDifference {
            position: None,
            path,
            expected: Some(expected),
            actual: Some(actual),
        });
    }
}
fn text(value: impl std::fmt::Debug) -> ProjectionValue {
    ProjectionValue::Text(format!("{value:?}"))
}
fn string(value: &str) -> ProjectionValue {
    ProjectionValue::Text(value.into())
}
fn member<T: Copy + Ord + PartialEq>(
    rows: &mut Vec<SourceDifference>,
    path: &str,
    expected: &[T],
    actual: &[T],
    value: impl Fn(T) -> ProjectionValue,
) {
    let mut expected = expected.to_vec();
    expected.sort();
    let mut actual = actual.to_vec();
    actual.sort();
    for item in expected.iter().filter(|x| !actual.contains(x)) {
        rows.push(SourceDifference {
            position: None,
            path: path.into(),
            expected: Some(value(*item)),
            actual: None,
        });
    }
    for item in actual.iter().filter(|x| !expected.contains(x)) {
        rows.push(SourceDifference {
            position: None,
            path: path.into(),
            expected: None,
            actual: Some(value(*item)),
        });
    }
}
fn binding(rows: &mut Vec<SourceDifference>, path: &str, expected: ClaimAudit, actual: ClaimAudit) {
    match (expected, actual) {
        (
            ClaimAudit::Bound {
                producer: ep,
                budget: eb,
                routes: er,
            },
            ClaimAudit::Bound {
                producer: ap,
                budget: ab,
                routes: ar,
            },
        ) => {
            changed(rows, format!("{path}.producer"), text(ep), text(ap));
            changed(rows, format!("{path}.budget"), text(eb), text(ab));
            changed(rows, format!("{path}.routes"), text(er), text(ar));
        }
        (ClaimAudit::Awaiting(e), ClaimAudit::Awaiting(a)) => {
            changed(rows, format!("{path}.awaits"), text(e), text(a));
        }
        (e, a) => changed(rows, path.into(), text(e), text(a)),
    }
}
pub(crate) fn compare(
    expected: &ValidatedManifest,
    loaded: &LoadedProjection,
) -> Result<(), ProjectionCheckError> {
    let actual = loaded.manifest();
    let mut rows = Vec::new();
    changed(
        &mut rows,
        "$.engine-pin".into(),
        string(ENGINE_PIN.0),
        string(loaded.engine_pin()),
    );
    compare_nodes(&mut rows, expected, actual);
    compare_coverage(&mut rows, expected, actual);
    compare_budgets(&mut rows, expected, loaded);
    compare_producers(&mut rows, loaded);
    compare_capabilities(&mut rows, loaded);
    for difference in &mut rows {
        difference.position = source_position(difference, loaded);
    }
    rows.sort_by(|a, b| (component(&a.path), &a.path).cmp(&(component(&b.path), &b.path)));
    if rows.is_empty() {
        Ok(())
    } else {
        Err(ProjectionCheckError::Projection(
            ProjectionError::Assertion(vec![ProjectionDiagnostic {
                rule: RuleId::M037,
                path: "$".into(),
                position: None,
                fault: ProjectionFault::SourceMismatch { differences: rows },
            }]),
        ))
    }
}

fn compare_limits(
    rows: &mut Vec<SourceDifference>,
    path: &str,
    expected: &ResolvedLimits,
    actual: &ResolvedLimits,
) {
    macro_rules! fields {
        ($($name:ident => $wire:literal),* $(,)?) => { $(changed(rows, format!("{path}.limits.{}", $wire), ProjectionValue::Integer(expected.$name()), ProjectionValue::Integer(actual.$name()));)* };
    }
    fields! {
        source_bytes => "source-bytes", name_bytes => "name-bytes", original_occurrences => "original-occurrences",
        depth => "depth", vertices => "vertices", edges => "edges", staged_vertices => "staged-vertices", staged_edges => "staged-edges",
        payload_bytes => "payload-bytes", staged_payload_bytes => "staged-payload-bytes", work => "work", arithmetic_steps => "arithmetic-steps",
        component_bits => "component-bits", intermediate_bits => "intermediate-bits", literal_bytes => "literal-bytes", decimal_exponent => "decimal-exponent",
        integer_exponent => "integer-exponent", polynomial_degree => "polynomial-degree", polynomial_variables => "polynomial-variables", polynomial_terms => "polynomial-terms",
        matrix_entries => "matrix-entries", condition_atoms => "condition-atoms", condition_steps => "condition-steps", match_attempts => "match-attempts", match_batch => "match-batch",
        frontier_labels => "frontier-labels", queue_entries => "queue-entries", transitions => "transitions", expansion_credits => "expansion-credits", peak_expression_size => "peak-expression-size",
        proof_nodes => "proof-nodes", witness_bytes => "witness-bytes", output_bytes => "output-bytes", diagnostic_records => "diagnostic-records", replay_work => "replay-work",
        cost_quota => "cost-quota", progress_quota => "progress-quota", case_arms => "case-arms", region_nodes => "region-nodes", region_cells => "region-cells",
        refinement_pairs => "refinement-pairs", local_contexts => "local-contexts", hypothesis_depth => "hypothesis-depth", boundary_nodes => "boundary-nodes",
        jet_order => "jet-order", jet_entries => "jet-entries", delta_order => "delta-order", test_nodes => "test-nodes", pairing_steps => "pairing-steps", pullback_zeros => "pullback-zeros",
        coefficient_ops => "coefficient-ops", denominator_factors => "denominator-factors", coefficient_maps => "coefficient-maps", tower_levels => "tower-levels",
        tower_degree => "tower-degree", tower_reductions => "tower-reductions", gcd_steps => "gcd-steps", component_splits => "component-splits", orbit_permutations => "orbit-permutations",
        orbit_members => "orbit-members", orbit_terms => "orbit-terms", symmetric_steps => "symmetric-steps", resolvent_depth => "resolvent-depth", resolvent_degree => "resolvent-degree",
        selection_slots => "selection-slots", solution_branches => "solution-branches", family_parameters => "family-parameters", solve_identities => "solve-identities",
        linear_pivots => "linear-pivots", isolation_steps => "isolation-steps", local_branches => "local-branches",
    }
}

fn compare_nodes(
    rows: &mut Vec<SourceDifference>,
    expected: &ValidatedManifest,
    actual: &ValidatedManifest,
) {
    let mut nodes: Vec<_> = expected.nodes().iter().collect();
    nodes.sort_by_key(|n| n.id);
    for node in nodes {
        let path = format!("$.nodes[{}]", node.id.semantic_name());
        let Some(other) = actual.node(node.id) else {
            rows.push(SourceDifference {
                position: None,
                path,
                expected: Some(ProjectionValue::Node(node.clone())),
                actual: None,
            });
            continue;
        };
        changed(
            rows,
            format!("{path}.kind"),
            text(node.payload.kind()),
            text(other.payload.kind()),
        );
        changed(
            rows,
            format!("{path}.description"),
            string(node.payload.description()),
            string(other.payload.description()),
        );
        changed(
            rows,
            format!("{path}.status.maturity"),
            text(node.maturity),
            text(other.maturity),
        );
        if let (NodePayload::Claim { audit: e, .. }, NodePayload::Claim { audit: a, .. }) =
            (&node.payload, &other.payload)
        {
            binding(rows, &format!("{path}.audit"), *e, *a);
        }
    }
    let mut extras: Vec<_> = actual
        .nodes()
        .iter()
        .filter(|n| expected.node(n.id).is_none())
        .collect();
    extras.sort_by_key(|n| n.id);
    for node in extras {
        rows.push(SourceDifference {
            position: None,
            path: format!("$.nodes[{}]", node.id.semantic_name()),
            expected: None,
            actual: Some(ProjectionValue::Node(node.clone())),
        });
    }
    member(
        rows,
        "$.edges",
        expected.edges(),
        actual.edges(),
        ProjectionValue::Edge,
    );
}

fn compare_coverage(
    rows: &mut Vec<SourceDifference>,
    expected: &ValidatedManifest,
    actual: &ValidatedManifest,
) {
    let e = expected.coverage();
    let a = actual.coverage();
    member(
        rows,
        "$.coverage.slice",
        &e.slice,
        &a.slice,
        ProjectionValue::Identity,
    );
    for (name, e, a) in [
        (
            "referents",
            e.closed_world.referents,
            a.closed_world.referents,
        ),
        (
            "relations",
            e.closed_world.relations,
            a.closed_world.relations,
        ),
        ("audit", e.closed_world.audit, a.closed_world.audit),
    ] {
        changed(
            rows,
            format!("$.coverage.closed-world.{name}"),
            ProjectionValue::Boolean(e),
            ProjectionValue::Boolean(a),
        );
    }
    changed(
        rows,
        "$.coverage.release-gate".into(),
        text(e.release_gate),
        text(a.release_gate),
    );
}

fn compare_budgets(
    rows: &mut Vec<SourceDifference>,
    expected: &ValidatedManifest,
    loaded: &LoadedProjection,
) {
    for budget in expected.budgets() {
        let path = format!("$.budgets[{:?}]", budget.id);
        let Some(other) = loaded.budgets().iter().find(|b| b.spec().id == budget.id) else {
            rows.push(SourceDifference {
                position: None,
                path,
                expected: Some(ProjectionValue::Budget(*budget)),
                actual: None,
            });
            continue;
        };
        let actual = other.spec();
        changed(
            rows,
            format!("{path}.producer"),
            text(budget.producer),
            text(actual.producer),
        );
        changed(
            rows,
            format!("{path}.profile"),
            text(budget.profile),
            text(actual.profile),
        );
        changed(
            rows,
            format!("{path}.pin"),
            string(budget.pin.0),
            string(actual.pin.0),
        );
        compare_limits(rows, &path, &budget.profile.resolved(), &other.limits());
    }
    for other in loaded
        .budgets()
        .iter()
        .filter(|b| !expected.budgets().iter().any(|e| e.id == b.spec().id))
    {
        rows.push(SourceDifference {
            position: None,
            path: format!("$.budgets[{:?}]", other.spec().id),
            expected: None,
            actual: Some(ProjectionValue::Budget(other.spec())),
        });
    }
}

fn compare_producers(rows: &mut Vec<SourceDifference>, loaded: &LoadedProjection) {
    changed(
        rows,
        "$.producers.length".into(),
        ProjectionValue::Integer(2),
        ProjectionValue::Integer(loaded.producers().len() as u64),
    );
    for id in ProducerId::ALL {
        let e = id.descriptor();
        let path = format!("$.producers[{id:?}]");
        let Some(a) = loaded.producers().iter().find(|p| p.id() == id) else {
            rows.push(SourceDifference {
                position: None,
                path,
                expected: Some(text(e)),
                actual: None,
            });
            continue;
        };
        changed(rows, format!("{path}.id"), text(e.producer), text(a.id()));
        changed(
            rows,
            format!("{path}.claim"),
            text(e.claim),
            text(a.claim()),
        );
        member(
            rows,
            &format!("{path}.dependencies"),
            e.dependencies,
            a.dependencies(),
            ProjectionValue::Identity,
        );
        let mut ec: Vec<_> = e.capabilities.iter().map(|c| format!("{c:?}")).collect();
        ec.sort();
        let mut ac: Vec<_> = a.capabilities().iter().map(|c| format!("{c:?}")).collect();
        ac.sort();
        changed(rows, format!("{path}.capabilities"), text(ec), text(ac));
        changed(
            rows,
            format!("{path}.budget"),
            text(e.budget),
            text(a.budget()),
        );
        changed(
            rows,
            format!("{path}.revision"),
            ProjectionValue::Integer(u64::from(e.revision)),
            ProjectionValue::Integer(u64::from(a.revision())),
        );
        changed(
            rows,
            format!("{path}.target"),
            ProjectionValue::Integer(e.target as u64),
            ProjectionValue::Integer(a.target()),
        );
        changed(
            rows,
            format!("{path}.region"),
            text(e.region),
            text(a.region()),
        );
    }
}

fn compare_capabilities(rows: &mut Vec<SourceDifference>, loaded: &LoadedProjection) {
    changed(
        rows,
        "$.capabilities.length".into(),
        ProjectionValue::Integer(4),
        ProjectionValue::Integer(loaded.capabilities().len() as u64),
    );
    for e in CAPABILITIES {
        let path = format!("$.capabilities[{:?}]", e.id);
        let Some(a) = loaded.capabilities().iter().find(|c| c.id() == e.id) else {
            rows.push(SourceDifference {
                position: None,
                path,
                expected: Some(text(e)),
                actual: None,
            });
            continue;
        };
        changed(rows, format!("{path}.id"), text(e.id), text(a.id()));
        changed(
            rows,
            format!("{path}.availability"),
            text(e.availability),
            text(a.availability()),
        );
        changed(
            rows,
            format!("{path}.adapter-implemented"),
            ProjectionValue::Boolean(e.adapter_implemented),
            ProjectionValue::Boolean(a.adapter_implemented()),
        );
        changed(
            rows,
            format!("{path}.evidence"),
            string(e.evidence),
            string(a.evidence()),
        );
    }
}

fn component(path: &str) -> usize {
    [
        "$.engine-pin",
        "$.nodes",
        "$.edges",
        "$.coverage",
        "$.budgets",
        "$.producers",
        "$.capabilities",
    ]
    .iter()
    .position(|prefix| path.starts_with(prefix))
    .unwrap_or(7)
}

fn source_position(difference: &SourceDifference, loaded: &LoadedProjection) -> Option<usize> {
    let path = &difference.path;
    if path.starts_with("$.nodes[") {
        return loaded
            .manifest()
            .nodes()
            .iter()
            .position(|node| path.starts_with(&format!("$.nodes[{}]", node.id.semantic_name())));
    }
    if path == "$.edges" {
        return match &difference.actual {
            Some(ProjectionValue::Edge(edge)) => {
                loaded.manifest().edges().iter().position(|e| e == edge)
            }
            _ => None,
        };
    }
    if path == "$.coverage.slice" {
        return match &difference.actual {
            Some(ProjectionValue::Identity(id)) => loaded
                .manifest()
                .coverage()
                .slice
                .iter()
                .position(|i| i == id),
            _ => None,
        };
    }
    if path.starts_with("$.budgets[") {
        return loaded
            .budgets()
            .iter()
            .position(|b| path.starts_with(&format!("$.budgets[{:?}]", b.spec().id)));
    }
    if path.starts_with("$.producers[") {
        return loaded
            .producers()
            .iter()
            .position(|p| path.starts_with(&format!("$.producers[{:?}]", p.id())));
    }
    if path.starts_with("$.capabilities[") {
        return loaded
            .capabilities()
            .iter()
            .position(|c| path.starts_with(&format!("$.capabilities[{:?}]", c.id())));
    }
    None
}
