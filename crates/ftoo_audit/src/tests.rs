type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
#[ignore = "test-only oversized budgets used to measure the complete route; production uses the reviewed profile"]
fn diagnostic_complete_route_work() -> TestResult {
    let mut refused = false;
    for (target, work) in [(1, 10_000_000), (3, 10_000_000)] {
        let mut f = crate::engine::Frame::new(
            nrz_cas::limits::Limits {
                work,
                replay_work: 10_000_000,
                ..nrz_cas::limits::Limits::default()
            },
            false,
        )?;
        let jet = f.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])?;
        let h = f.rationals(&[(0, 1), (1, 1), (0, 1), (0, 1), (0, 1)])?;
        let compose = f.compose(&jet, &h)?;
        let (ode, rows) = f.term(&crate::theory::ode(), &jet, 4)?;
        let (residual, member) = f.residual(&compose, &ode, &rows)?;
        println!(
            "MEASURE target={target} member_work={}",
            f.budget.usage().work
        );
        let evidence = [
            f.export(&compose)?,
            f.export(&residual)?,
            f.export(&member)?,
        ];
        let expected = f.expectations()?;
        println!("MEASURE before replay Work={}", f.budget.usage().work);
        let before_replay = f.budget.usage().work;
        match f.replay(&evidence, &expected) {
            Ok(()) => println!("MEASURE target={target} profile_work={work} replay accepted"),
            Err(fault) => {
                println!(
                    "MEASURE target={target} profile_work={work} refused={fault:?}, consumed_work={}",
                    f.budget.usage().work
                );
                refused = true;
            }
        }
        println!(
            "MEASURE target={target} consumed_work={} replay_work={} usage={:?}",
            f.budget.usage().work,
            f.budget.usage().work - before_replay,
            f.budget.usage()
        );
    }
    if refused {
        Err("complete route blocked by reviewed ceilings".into())
    } else {
        Ok(())
    }
}

use crate::{RunCause, RunError, RunStage, engine::Frame, theory};
use NodeId::{
    LangevinCoefficientOne as One, LangevinCoefficientOneValue as OneValue,
    LangevinCoefficientThreeValue as ThreeValue,
};
use ftoo_manifest::{
    AdmissionStage, Awaiting, BudgetProfile, ClaimAudit, Diagnostic, EnginePin, NodeId,
    NodePayload, ProducerId, RuleId, seed, seed_draft, validate,
};
use nrz_cas::{
    FaultKind, InvalidCode, Rational,
    limits::{Budget, BudgetKind, Limits},
};
fn profile() -> Limits {
    theory::limits(BudgetProfile::PinnedTrialLimitsV1)
}
fn counts() -> (usize, usize) {
    crate::engine::COUNTS.with(std::cell::Cell::get)
}
fn admission(
    graph: &ftoo_manifest::ValidatedManifest,
    entries: &[crate::registry::Entry],
    rule: RuleId,
) -> Vec<Diagnostic> {
    let before = counts();
    let Err(RunError::Admission(errors)) = crate::runner::execute(graph, entries) else {
        panic!("expected admission failure")
    };
    assert_eq!(
        errors
            .iter()
            .map(|d| d.rule)
            .collect::<std::collections::BTreeSet<_>>(),
        [rule].into_iter().collect()
    );
    assert!(
        errors
            .iter()
            .all(|d| d.stage == AdmissionStage::RegistryAdmission)
    );
    assert_eq!(counts(), before);
    errors
}
#[test]
fn both_producers_use_fresh_stores_shared_budgets_and_preserve_base_status() -> TestResult {
    let graph = seed()?;
    let before = counts();
    let run = crate::run(&graph)?;
    assert_eq!(counts(), (before.0 + 2, before.1 + 2));
    assert!(std::ptr::eq(run.manifest(), &raw const graph));
    assert_eq!(run.selected(), [OneValue, ThreeValue]);
    assert_eq!(run.receipts().len(), 2);
    let mut b = Budget::new(Limits::default())?;
    let j = [(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)]
        .into_iter()
        .map(|(n, d)| Rational::from_ratio(n, d, &mut b))
        .collect::<Result<Vec<_>, _>>()?;
    for (receipt, target) in run.receipts().iter().zip([1, 3]) {
        assert_eq!(receipt.target(), target);
        assert_eq!(receipt.summary().jet, j);
        assert_eq!(receipt.coefficient(), &j[target]);
        assert_eq!(
            receipt.summary().increment,
            [
                Rational::zero(),
                Rational::one(),
                Rational::zero(),
                Rational::zero(),
                Rational::zero()
            ]
        );
        assert_eq!(receipt.summary().input_orders, [5, 5]);
        assert!(receipt.summary().default_order);
        assert_eq!(
            (
                receipt.summary().order,
                receipt.summary().derivative_order,
                receipt.summary().residual_order
            ),
            (5, 1, 4)
        );
        assert_eq!(receipt.summary().equation_degrees, [0, 1, 2, 3]);
        assert_eq!(
            receipt.summary().region,
            ftoo_manifest::RegionContract::AmbientProductXNoAssumptions
        );
        assert!(receipt.summary().subject.contains("identity composition"));
        assert_eq!(receipt.pin(), ftoo_manifest::ENGINE_PIN);
        assert_eq!(receipt.limits(), &profile());
        assert_eq!(receipt.revision(), 1);
        assert_eq!(
            graph.status(receipt.claim()).ok_or("status")?.evidence(),
            ftoo_manifest::Evidence::Deferred
        );
    }
    for m in run.measurements() {
        assert_eq!(
            (m.member_work, m.total_work, m.replay_work),
            (2_574_024, 6_073_177, 3_465_362)
        );
    }
    Ok(())
}
#[test]
fn empty_and_one_only_slice_have_no_unselected_rows() -> TestResult {
    for slice in [vec![], vec![OneValue]] {
        let mut draft = seed_draft();
        draft.coverage.slice.clone_from(&slice);
        let graph = validate(draft)?;
        let run = crate::run(&graph)?;
        assert_eq!(run.selected(), slice);
        assert_eq!(run.receipts().len(), slice.len());
    }
    Ok(())
}
#[test]
fn m014_three_only_refuses_at_registry_admission() -> TestResult {
    let mut draft = seed_draft();
    draft.coverage.slice = vec![ThreeValue];
    let errors = admission(&validate(draft)?, &crate::registry::REGISTRY, RuleId::M014);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].ids, [ThreeValue, OneValue]);
    Ok(())
}
#[test]
fn m020_missing_duplicate_swapped_orphan_and_awaiting_implementations() -> TestResult {
    let graph = seed()?;
    let entries = crate::registry::REGISTRY;
    admission(&graph, &entries[..1], RuleId::M020);
    admission(&graph, &[entries[0], entries[0], entries[1]], RuleId::M020);
    let mut swapped = entries;
    swapped[0].descriptor.claim = ThreeValue;
    admission(&graph, &swapped, RuleId::M020);
    let mut draft = seed_draft();
    draft.coverage.slice.clear();
    draft.nodes.retain(|n| n.id != ThreeValue);
    draft
        .edges
        .retain(|e| e.source != ThreeValue && e.target != ThreeValue);
    draft.budgets.pop();
    admission(&validate(draft)?, &entries, RuleId::M020);
    let mut draft = seed_draft();
    draft.coverage.slice.clear();
    draft.coverage.closed_world.audit = false;
    if let NodePayload::Claim { audit, .. } = &mut draft.nodes[3].payload {
        *audit = ClaimAudit::Awaiting(Awaiting::AuditBinding);
    }
    draft.budgets.pop();
    admission(&validate(draft)?, &entries, RuleId::M020);
    Ok(())
}
#[test]
fn m022_missing_and_invented_dependencies() -> TestResult {
    let mut draft = seed_draft();
    draft
        .edges
        .retain(|e| !(e.source == ThreeValue && e.target == OneValue));
    admission(&validate(draft)?, &crate::registry::REGISTRY, RuleId::M022);
    let mut draft = seed_draft();
    draft.edges.push(ftoo_manifest::Edge {
        source: ThreeValue,
        target: One,
        relation: ftoo_manifest::RelationKind::DependsOn,
        guard: ftoo_manifest::Guard::Always,
    });
    admission(&validate(draft)?, &crate::registry::REGISTRY, RuleId::M022);
    Ok(())
}
#[test]
fn m023_missing_duplicate_swapped_wrong_pin_and_orphan_budgets() -> TestResult {
    for mutation in 0..5 {
        let mut draft = seed_draft();
        match mutation {
            0 => {
                draft.budgets.pop();
            }
            1 => draft.budgets.push(draft.budgets[0]),
            2 => draft.budgets[0].producer = ProducerId::LangevinCoefficientThreeValue,
            3 => draft.budgets[0].pin = EnginePin("wrong-pin"),
            _ => {
                draft.coverage.slice.clear();
                draft.coverage.closed_world.audit = false;
                for n in &mut draft.nodes {
                    if let NodePayload::Claim { audit, .. } = &mut n.payload {
                        *audit = ClaimAudit::Awaiting(Awaiting::AuditBinding);
                    }
                }
                let graph = validate(draft)?;
                admission(&graph, &[], RuleId::M023);
                continue;
            }
        }
        admission(&validate(draft)?, &crate::registry::REGISTRY, RuleId::M023);
    }
    Ok(())
}
#[test]
fn m024_unsupported_dispatch_and_m022_missing_receipt_allocate_nothing() {
    let before = counts();
    let mut d = ProducerId::LangevinCoefficientOneValue.descriptor();
    d.capabilities = &[ftoo_manifest::CapabilityId::ExactRationalLinearEquivalence];
    assert!(matches!(
        crate::runner::ready(d, &[]),
        Err(RunError::Execution {
            stage: RunStage::Capability,
            cause: RunCause::CapabilityUnavailable(_),
            ..
        })
    ));
    assert!(matches!(
        crate::runner::ready(ProducerId::LangevinCoefficientThreeValue.descriptor(), &[]),
        Err(RunError::Execution {
            stage: RunStage::Premise,
            cause: RunCause::MissingAcceptedPremise(OneValue),
            ..
        })
    ));
    assert_eq!(before, counts());
}
fn failed_first() -> Result<(crate::Receipt, crate::Measurement), RunError> {
    Err(RunError::Execution {
        claim: OneValue,
        stage: RunStage::Projection,
        cause: RunCause::ProjectionMismatch,
    })
}
fn must_not_execute() -> Result<(crate::Receipt, crate::Measurement), RunError> {
    panic!("dependent executed after failed premise")
}
#[test]
fn m022_failed_premise_stops_before_dependent() -> TestResult {
    let mut entries = crate::registry::REGISTRY;
    entries[0].produce = failed_first;
    entries[1].produce = must_not_execute;
    assert!(matches!(
        crate::runner::execute(&seed()?, &entries),
        Err(RunError::Execution {
            claim: OneValue,
            ..
        })
    ));
    Ok(())
}
#[test]
fn m023_real_exhaustion_preserves_resource_and_stage() {
    let limits = Limits {
        work: 1,
        ..profile()
    };
    let result = theory::checked(
        ProducerId::LangevinCoefficientOneValue.descriptor(),
        limits,
        false,
    );
    assert!(matches!(
        result,
        Err(RunError::Execution {
            claim: OneValue,
            stage: RunStage::Setup,
            cause: RunCause::Engine(nrz_cas::Fault {
                kind: FaultKind::Exhausted {
                    budget: BudgetKind::Work,
                    reason: nrz_cas::Exhaustion::Limit,
                    limit: 1,
                    needed: 2
                },
                ..
            })
        })
    ));
}
#[test]
fn empty_guarded_region_never_yields_accepted_receipt() {
    let result = theory::checked(
        ProducerId::LangevinCoefficientOneValue.descriptor(),
        profile(),
        true,
    );
    assert!(
        matches!(
            result,
            Err(RunError::Execution {
                stage: RunStage::Replay,
                cause: RunCause::Engine(nrz_cas::Fault {
                    kind: FaultKind::Invalid(InvalidCode::EndpointMismatch),
                    ..
                }),
                ..
            })
        ),
        "{result:?}"
    );
}
#[test]
fn wrong_target_never_yields_accepted_receipt() {
    let mut d = ProducerId::LangevinCoefficientThreeValue.descriptor();
    d.target = 1;
    assert!(matches!(
        theory::checked(d, profile(), false),
        Err(RunError::Execution {
            stage: RunStage::Projection,
            ..
        })
    ));
}
#[test]
fn dependency_pin_matches_manifest_cargo_and_lock() {
    let pin = ftoo_manifest::ENGINE_PIN.0;
    assert!(include_str!("../Cargo.toml").contains(&format!("rev = \"{pin}\"")));
    assert!(include_str!("../../../Cargo.lock").contains(&format!("?rev={pin}#{pin}")));
    assert_eq!(
        ftoo_manifest::CAPABILITIES
            .iter()
            .filter(|c| c.adapter_implemented)
            .map(|c| c.id)
            .collect::<Vec<_>>(),
        [ftoo_manifest::CapabilityId::FormalTruncatedPowerSeries]
    );
}

#[test]
fn pinned_trial_limits_full_field_fixture() {
    let expected = Limits {
        source_bytes: 262_144,
        name_bytes: 256,
        original_occurrences: 10_000,
        depth: 128,
        vertices: 50_000,
        edges: 200_000,
        staged_vertices: 50_000,
        staged_edges: 200_000,
        payload_bytes: 16_777_216,
        staged_payload_bytes: 16_777_216,
        work: 8_000_000,
        arithmetic_steps: 500_000,
        component_bits: 4_096,
        intermediate_bits: 8_192,
        literal_bytes: 256,
        decimal_exponent: 128,
        integer_exponent: 256,
        polynomial_degree: 64,
        polynomial_variables: 16,
        polynomial_terms: 4_096,
        matrix_entries: 4_096,
        condition_atoms: 256,
        condition_steps: 100_000,
        match_attempts: 50_000,
        match_batch: 32,
        frontier_labels: 8_192,
        queue_entries: 16_384,
        transitions: 20_000,
        expansion_credits: 20_000,
        peak_expression_size: 50_000,
        proof_nodes: 20_000,
        witness_bytes: 1_048_576,
        output_bytes: 1_048_576,
        diagnostic_records: 128,
        replay_work: 4_500_000,
        cost_quota: 4,
        progress_quota: 1,
        case_arms: 256,
        region_nodes: 2_048,
        region_cells: 256,
        refinement_pairs: 4_096,
        local_contexts: 128,
        hypothesis_depth: 32,
        boundary_nodes: 128,
        jet_order: 8,
        jet_entries: 4_096,
        delta_order: 8,
        test_nodes: 256,
        pairing_steps: 4_096,
        pullback_zeros: 64,
        coefficient_ops: 2_000_000,
        denominator_factors: 256,
        coefficient_maps: 256,
        tower_levels: 8,
        tower_degree: 256,
        tower_reductions: 100_000,
        gcd_steps: 100_000,
        component_splits: 128,
        orbit_permutations: 40_320,
        orbit_members: 256,
        orbit_terms: 262_144,
        symmetric_steps: 4_096,
        resolvent_depth: 8,
        resolvent_degree: 32,
        selection_slots: 64,
        solution_branches: 256,
        family_parameters: 64,
        solve_identities: 8_192,
        linear_pivots: 256,
        isolation_steps: 100_000,
        local_branches: 64,
    };
    assert_eq!(profile(), expected);
    let defaults = Limits::default();
    assert_eq!(
        expected,
        Limits {
            work: 8_000_000,
            replay_work: 4_500_000,
            ..defaults
        }
    );
    // Complete-route measurements for both targets; each field uses the larger.
    // Round upward to 500,000, then step by 500,000 until headroom is at least 25%.
    for (measured, reviewed) in [
        (6_073_177_u64, expected.work),
        (3_465_362, expected.replay_work),
    ] {
        let mut derived = measured.div_ceil(500_000) * 500_000;
        while (derived - measured) * 4 < measured {
            derived += 500_000;
        }
        assert_eq!(derived, reviewed);
    }
}

#[test]
fn equation_window_leaves_terminal_coefficient_free() -> TestResult {
    for terminal in [0, 7] {
        let mut f = Frame::new(profile(), false)?;
        let j = f.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (terminal, 1)])?;
        let h = f.rationals(&[(0, 1), (1, 1), (0, 1), (0, 1), (0, 1)])?;
        let compose = f.compose(&j, &h)?;
        let (ode, rows) = f.term(&theory::ode(), &j, 4)?;
        // Only degrees 0..3 enter: c4 is unconstrained terminal candidate data.
        assert!(rows.iter().all(|row| row.coefficients.len() == 4));
        assert_eq!(
            rows.last().ok_or("trace")?.coefficients,
            vec![Rational::zero(); 4]
        );
        f.residual(&compose, &ode, &rows)?;
        // Independent trial recurrence: 2c0; 3c1+c0²-1;
        // 4c2+2c0c1; 5c3+c1²+2c0c2. No c4 enters.
        let mut budget = Budget::new(Limits::default())?;
        let two = Rational::from_ratio(2, 1, &mut budget)?;
        let three = Rational::from_ratio(3, 1, &mut budget)?;
        let four = Rational::from_ratio(4, 1, &mut budget)?;
        let five = Rational::from_ratio(5, 1, &mut budget)?;
        let equations = [
            two.mul(&j[0], &mut budget)?,
            three
                .mul(&j[1], &mut budget)?
                .add(&j[0].mul(&j[0], &mut budget)?, &mut budget)?
                .sub(&Rational::one(), &mut budget)?,
            four.mul(&j[2], &mut budget)?.add(
                &two.mul(&j[0], &mut budget)?.mul(&j[1], &mut budget)?,
                &mut budget,
            )?,
            five.mul(&j[3], &mut budget)?
                .add(&j[1].mul(&j[1], &mut budget)?, &mut budget)?
                .add(
                    &two.mul(&j[0], &mut budget)?.mul(&j[2], &mut budget)?,
                    &mut budget,
                )?,
        ];
        assert_eq!(rows.last().ok_or("trace")?.coefficients, equations);
    }
    Ok(())
}
#[test]
fn changed_cubic_coefficient_refuses_member() -> TestResult {
    let mut f = Frame::new(profile(), false)?;
    let j = f.rationals(&[(0, 1), (1, 3), (0, 1), (1, 45), (0, 1)])?;
    let h = f.rationals(&[(0, 1), (1, 1), (0, 1), (0, 1), (0, 1)])?;
    let compose = f.compose(&j, &h)?;
    let (ode, rows) = f.term(&theory::ode(), &j, 4)?;
    let fault = f
        .residual(&compose, &ode, &rows)
        .expect_err("Member must refuse");
    assert_eq!(fault.kind, FaultKind::Invalid(InvalidCode::Rule));
    Ok(())
}
#[test]
fn nonzero_increment_constant_refuses_composition() -> TestResult {
    let mut f = Frame::new(profile(), false)?;
    let j = f.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])?;
    let h = f.rationals(&[(1, 1), (1, 1), (0, 1), (0, 1), (0, 1)])?;
    let fault = f.compose(&j, &h).expect_err("composition must refuse");
    assert_eq!(
        fault.kind,
        FaultKind::Invalid(InvalidCode::SeriesNonzeroConstant)
    );
    // Evidence owns the coefficient even after the entire store is dropped.
    drop(f);
    let evidence = fault
        .series_constant()
        .ok_or("missing owned constant evidence")?;
    assert_eq!(evidence.index, 0);
    assert_eq!(evidence.value, Rational::one());
    Ok(())
}

#[test]
fn independent_replay_pins_subject_derivative_and_window() -> TestResult {
    use nrz_cas::{proof::Statement, series::SeriesStatement};
    let graph = seed()?;
    let run = crate::run(&graph)?;
    for receipt in run.receipts() {
        for mutation in 0..4 {
            let mut frame = Frame::new(profile(), false)?;
            let mut expected = frame.expectations()?;
            if mutation != 0 {
                let Statement::Series(series) = &mut expected[0].statement else {
                    panic!("series")
                };
                let SeriesStatement::EvaluatedResidual {
                    subject,
                    derivative_order,
                    order,
                    ..
                } = series.as_mut()
                else {
                    panic!("residual")
                };
                match mutation {
                    1 => *derivative_order = 2,
                    2 => *order = 3,
                    _ => {
                        let nrz_cas::series::ResidualSubject::Compose { left, right, .. } = subject
                        else {
                            panic!("compose")
                        };
                        std::mem::swap(left, right);
                    }
                }
            }
            let result = frame.replay(receipt.evidence(), &expected);
            if mutation == 0 {
                result?;
            } else {
                // Invalid order parameters refuse before endpoint comparison;
                // swapping the well-formed subject reaches endpoint matching.
                let code = match mutation {
                    1 | 2 => InvalidCode::Rule,
                    3 => InvalidCode::EndpointMismatch,
                    _ => unreachable!(),
                };
                let fault = result.expect_err("mutated replay must refuse");
                assert_eq!(fault.kind, FaultKind::Invalid(code));
                assert_eq!(
                    fault.location,
                    nrz_cas::Location::new("formal series arithmetic")
                );
            }
        }
    }
    Ok(())
}

#[test]
fn compose_and_residual_exports_replay_independently() -> TestResult {
    let graph = seed()?;
    let run = crate::run(&graph)?;
    for receipt in run.receipts() {
        for index in 0..2 {
            let mut frame = Frame::new(profile(), false)?;
            let [residual, _member] = frame.expectations()?;
            let nrz_cas::proof::Statement::Series(series) = &residual.statement else {
                panic!("series")
            };
            let nrz_cas::series::SeriesStatement::EvaluatedResidual {
                subject:
                    nrz_cas::series::ResidualSubject::Compose {
                        left,
                        right,
                        output,
                        order,
                        default_order,
                    },
                ..
            } = series.as_ref()
            else {
                panic!("identity composition")
            };
            // Test-only Compose expectation, from freshly specified syntax, not evidence.
            let compose = nrz_cas::proof::Judgment {
                statement: nrz_cas::proof::Statement::Series(Box::new(
                    nrz_cas::series::SeriesStatement::Compose {
                        left: left.clone(),
                        right: right.clone(),
                        output: output.clone(),
                        order: *order,
                        default_order: *default_order,
                    },
                )),
                ..residual.clone()
            };
            let expected = [compose, residual];
            frame.cas.replay(
                &receipt.evidence()[index],
                &expected[index],
                &mut frame.budget,
            )?;
        }
    }
    Ok(())
}
#[test]
fn changed_residual_sign_refuses_member() -> TestResult {
    let mut f = Frame::new(profile(), false)?;
    let j = f.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])?;
    let h = f.rationals(&[(0, 1), (1, 1), (0, 1), (0, 1), (0, 1)])?;
    let compose = f.compose(&j, &h)?;
    let mut ode = theory::ode();
    let crate::engine::Term::Binary(op, ..) = &mut ode else {
        panic!("ODE")
    };
    *op = nrz_cas::syntax::Operator::Add;
    let (ode, rows) = f.term(&ode, &j, 4)?;
    let fault = f
        .residual(&compose, &ode, &rows)
        .expect_err("Member must refuse");
    assert_eq!(fault.kind, FaultKind::Invalid(InvalidCode::Rule));
    Ok(())
}

fn report_rules(error: crate::AuditError, expected: &[RuleId]) {
    let crate::AuditError::Report(errors) = error else {
        panic!("expected report diagnostics")
    };
    assert_eq!(
        errors
            .iter()
            .map(|d| d.rule)
            .collect::<std::collections::BTreeSet<_>>(),
        expected.iter().copied().collect()
    );
    assert!(
        errors
            .iter()
            .all(|d| d.stage == AdmissionStage::ReportConsumption)
    );
}
#[test]
fn m027_cold_warm_and_reordered_reports_have_identical_gated_evidence() -> TestResult {
    let graph = seed()?;
    let before = counts();
    let cold = crate::audit(&graph)?;
    assert_eq!(counts(), (before.0 + 2, before.1 + 2));
    crate::test_support::solving_trial()?;
    let warm = crate::audit(&graph)?;
    assert_eq!(cold.gated(), warm.gated());
    let mut draft = seed_draft();
    draft.nodes.reverse();
    draft.edges.reverse();
    draft.budgets.reverse();
    draft.coverage.slice.reverse();
    let reordered = crate::audit(&validate(draft)?)?;
    assert_eq!(cold.gated(), reordered.gated());
    let mut measurements = warm.measurements().to_vec();
    measurements[0].total_work += 999;
    let observed = crate::AuditReport::from_outcomes(
        &graph,
        warm.snapshot().clone(),
        warm.reached().to_vec(),
        warm.outcomes().to_vec(),
        measurements,
    )?;
    assert_eq!(cold.gated(), observed.gated());
    assert_eq!(
        cold.status(OneValue).unwrap().evidence(),
        crate::AuditedEvidence::Certified
    );
    assert_eq!(
        cold.status(One).unwrap().evidence(),
        crate::AuditedEvidence::NotAClaim
    );
    assert_eq!(
        cold.status(OneValue).unwrap().base().maturity(),
        ftoo_manifest::Maturity::Proposed
    );
    assert_eq!(
        graph.status(OneValue).unwrap().evidence(),
        ftoo_manifest::Evidence::Deferred
    );
    Ok(())
}
#[test]
fn m027_reused_execution_identity_is_refused() -> TestResult {
    let graph = seed()?;
    let report = crate::audit(&graph)?;
    let mut measurements = report.measurements().to_vec();
    measurements[1].execution = measurements[0].execution;
    report_rules(
        crate::AuditReport::from_outcomes(
            &graph,
            report.snapshot().clone(),
            report.reached().to_vec(),
            report.outcomes().to_vec(),
            measurements,
        )
        .expect_err("reused store"),
        &[RuleId::M027],
    );
    Ok(())
}
fn refuted_first() -> Result<(crate::Receipt, crate::Measurement), RunError> {
    theory::checked_claim(
        ProducerId::LangevinCoefficientOneValue.descriptor(),
        profile(),
        false,
        Some((0, 1)),
    )
}
fn exhausted_first() -> Result<(crate::Receipt, crate::Measurement), RunError> {
    theory::checked(
        ProducerId::LangevinCoefficientOneValue.descriptor(),
        Limits {
            work: 1,
            ..profile()
        },
        false,
    )
}
#[test]
fn m026_checked_counterstatement_refutes_and_partial_release_refuses() -> TestResult {
    let graph = seed()?;
    let mut entries = crate::registry::REGISTRY;
    entries[0].produce = refuted_first;
    entries[1].produce = must_not_execute;
    let report = crate::runner::audit_entries(&graph, &entries)?;
    let crate::Verdict::Refuted(receipt) = report.outcomes()[0].verdict() else {
        panic!("expected checked refutation: {:?}", report.outcomes())
    };
    assert_eq!(
        receipt.counterstatement().unwrap().claimed(),
        &Rational::zero()
    );
    assert_ne!(receipt.counterstatement().unwrap().witness(), [0u8; 0]);
    assert_eq!(
        report.outcomes()[1].verdict(),
        &crate::Verdict::Deferred(ftoo_manifest::DeferredReason::Premise {
            claim: OneValue,
            evidence: ftoo_manifest::VerdictClass::Refuted
        })
    );
    assert_eq!(
        report.assessment().disposition(),
        ftoo_manifest::Disposition::MathematicalFailure
    );
    assert_eq!(report.assessment().exit_code(), 4);
    assert_eq!(
        report
            .assessment()
            .diagnostics()
            .iter()
            .map(|d| d.rule)
            .collect::<Vec<_>>(),
        [RuleId::M015]
    );
    assert_eq!(
        report.status(OneValue).unwrap().evidence(),
        crate::AuditedEvidence::Refuted
    );
    println!(
        "COUNTERSTATEMENT complete work={} total replay={}",
        report.measurements()[0].total_work,
        report.measurements()[0].replay_work
    );
    assert!(report.measurements()[0].total_work <= 8_000_000);
    Ok(())
}
#[test]
fn m026_typed_exhaustion_is_undecided_and_partial_release_allows_it() -> TestResult {
    let graph = seed()?;
    let mut entries = crate::registry::REGISTRY;
    entries[0].produce = exhausted_first;
    entries[1].produce = must_not_execute;
    let report = crate::runner::audit_entries(&graph, &entries)?;
    let crate::Verdict::Undecided { stage, exhaustion } = report.outcomes()[0].verdict() else {
        panic!("Undecided")
    };
    assert_eq!(*stage, RunStage::Setup);
    assert_eq!(
        exhaustion.kind,
        FaultKind::Exhausted {
            reason: nrz_cas::Exhaustion::Limit,
            budget: BudgetKind::Work,
            limit: 1,
            needed: 2
        }
    );
    assert_eq!(
        report.outcomes()[1].verdict(),
        &crate::Verdict::Deferred(ftoo_manifest::DeferredReason::Premise {
            claim: OneValue,
            evidence: ftoo_manifest::VerdictClass::Undecided
        })
    );
    assert_eq!(report.assessment().exit_code(), 0);
    assert_eq!(report.reached(), [OneValue, ThreeValue]);
    assert_eq!(report.measurements(), []);
    Ok(())
}
#[test]
fn m028_deferred_premise_and_capability_never_execute() -> TestResult {
    let first = crate::ClaimOutcome {
        claim: OneValue,
        verdict: crate::Verdict::Deferred(ftoo_manifest::DeferredReason::Capability(
            ftoo_manifest::CapabilityId::ExactRationalLinearEquivalence,
        )),
    };
    let mut entry = crate::registry::REGISTRY[1];
    entry.produce = must_not_execute;
    let before = counts();
    let (verdict, measurement) = crate::runner::dispatch(&entry, &[first])?;
    assert_eq!(
        verdict,
        crate::Verdict::Deferred(ftoo_manifest::DeferredReason::Premise {
            claim: OneValue,
            evidence: ftoo_manifest::VerdictClass::Deferred
        })
    );
    assert!(measurement.is_none());
    let mut entry = crate::registry::REGISTRY[0];
    entry.produce = must_not_execute;
    entry.descriptor.capabilities = &[ftoo_manifest::CapabilityId::ExactRationalLinearEquivalence];
    let (verdict, measurement) = crate::runner::dispatch(&entry, &[])?;
    assert_eq!(
        verdict,
        crate::Verdict::Deferred(ftoo_manifest::DeferredReason::Capability(
            ftoo_manifest::CapabilityId::ExactRationalLinearEquivalence
        ))
    );
    assert!(measurement.is_none());
    assert_eq!(counts(), before);
    Ok(())
}
#[test]
fn m025_missing_premise_is_not_a_blocker_fallback() {
    report_rules(
        crate::runner::dispatch(&crate::registry::REGISTRY[1], &[]).expect_err("missing row"),
        &[RuleId::M025],
    );
}
#[test]
fn m026_invalid_construction_never_becomes_refutation() -> TestResult {
    let mut f = Frame::new(profile(), false)?;
    let j = f.rationals(&[(0, 1), (1, 3), (0, 1), (1, 45), (0, 1)])?;
    let h = f.rationals(&[(0, 1), (1, 1), (0, 1), (0, 1), (0, 1)])?;
    let compose = f.compose(&j, &h)?;
    let (ode, rows) = f.term(&theory::ode(), &j, 4)?;
    let fault = f.residual(&compose, &ode, &rows).expect_err("bad member");
    assert_eq!(fault.kind, FaultKind::Invalid(InvalidCode::Rule));
    let error = crate::runner::fault(OneValue, RunStage::ResidualMember, fault);
    assert_eq!(
        crate::runner::interpret(error.clone()).expect_err("invalid is not refutation"),
        crate::AuditError::Execution(error)
    );
    let graph = seed()?;
    let report = crate::audit(&graph)?;
    let mut outcomes = report.outcomes().to_vec();
    let crate::Verdict::Certified(receipt) = outcomes.remove(0).verdict else {
        panic!("certified")
    };
    outcomes.insert(
        0,
        crate::ClaimOutcome {
            claim: OneValue,
            verdict: crate::Verdict::Refuted(receipt),
        },
    );
    report_rules(
        crate::AuditReport::from_outcomes(
            &graph,
            report.snapshot().clone(),
            report.reached().to_vec(),
            outcomes,
            report.measurements().to_vec(),
        )
        .expect_err("no contrary proof"),
        &[RuleId::M026],
    );
    Ok(())
}
#[test]
fn m015_real_partial_complete_and_incomplete_assessments() -> TestResult {
    use ftoo_manifest::{Disposition as D, ReleaseGate};
    for slice in [vec![], vec![OneValue], vec![OneValue, ThreeValue]] {
        let mut draft = seed_draft();
        draft.coverage.slice = slice.clone();
        let graph = validate(draft)?;
        let report = crate::audit(&graph)?;
        assert_eq!(report.assessment().disposition(), D::Pass);
        assert_eq!(report.outcomes().len(), 2);
        for row in report.outcomes() {
            if !slice.contains(&row.claim()) {
                assert_eq!(
                    row.verdict(),
                    &crate::Verdict::Deferred(ftoo_manifest::DeferredReason::OutsideSlice)
                );
            }
        }
    }
    let mut draft = seed_draft();
    draft.coverage.release_gate = ReleaseGate::Complete;
    let report = crate::audit(&validate(draft.clone())?)?;
    assert_eq!(report.assessment().exit_code(), 5);
    draft.coverage.closed_world.referents = true;
    draft.coverage.closed_world.relations = true;
    let report = crate::audit(&validate(draft)?)?;
    assert_eq!(report.assessment().exit_code(), 0);
    for (class, code) in [
        (D::Pass, 0),
        (D::InputError, 2),
        (D::SemanticInvalidity, 3),
        (D::MathematicalFailure, 4),
        (D::Incomplete, 5),
    ] {
        assert_eq!(class.exit_code(), code);
    }
    Ok(())
}

#[test]
fn m026_junk_witness_is_semantic_invalidity_not_refutation() -> TestResult {
    let mut frame = Frame::new(profile(), false)?;
    let expected = frame.expectations()?;
    let fault = frame
        .cas
        .replay(&[], &expected[0], &mut frame.budget)
        .expect_err("junk witness");
    assert_eq!(
        fault.kind,
        FaultKind::Invalid(InvalidCode::UnsupportedVersion)
    );
    let error = crate::runner::fault(OneValue, RunStage::Replay, fault);
    let classified = crate::runner::interpret(error.clone()).expect_err("invalid must abort");
    assert_eq!(classified, crate::AuditError::Execution(error));
    assert_eq!(classified.exit_code(), 3);
    let mut entries = crate::registry::REGISTRY;
    entries[0].produce = failed_first;
    entries[1].produce = must_not_execute;
    assert_eq!(
        crate::runner::audit_entries(&seed()?, &entries).expect_err("invalid producer must abort"),
        crate::AuditError::Execution(RunError::Execution {
            claim: OneValue,
            stage: RunStage::Projection,
            cause: RunCause::ProjectionMismatch
        })
    );
    Ok(())
}
#[test]
fn m025_stale_provenance_cannot_construct_audited_status() -> TestResult {
    let graph = seed()?;
    let report = crate::audit(&graph)?;
    let mut snapshot = report.snapshot().clone();
    snapshot.pin = EnginePin("stale");
    report_rules(
        crate::AuditReport::from_outcomes(
            &graph,
            snapshot,
            report.reached().to_vec(),
            report.outcomes().to_vec(),
            report.measurements().to_vec(),
        )
        .expect_err("stale report"),
        &[RuleId::M025],
    );
    Ok(())
}
#[test]
fn m026_both_counterstatements_fit_the_fixed_profile() -> TestResult {
    for producer in ProducerId::ALL {
        let (receipt, measurement) =
            theory::checked_claim(producer.descriptor(), profile(), false, Some((0, 1)))?;
        assert!(receipt.counterstatement().is_some());
        assert_eq!(receipt.limits(), &profile());
        println!(
            "COUNTERSTATEMENT target={} work={} total_replay={}",
            receipt.target(),
            measurement.total_work,
            measurement.replay_work
        );
        assert!(measurement.total_work <= 8_000_000);
        assert!(measurement.replay_work <= 4_500_000);
    }
    Ok(())
}

#[test]
fn m015_empty_authority_is_partial_but_never_complete() -> TestResult {
    let mut draft = seed_draft();
    draft.nodes.clear();
    draft.edges.clear();
    draft.budgets.clear();
    draft.coverage.slice.clear();
    draft.coverage.closed_world = ftoo_manifest::ClosedWorld {
        referents: true,
        relations: true,
        audit: true,
    };
    let before = counts();
    for (gate, code) in [
        (ftoo_manifest::ReleaseGate::ExplicitPartial, 0),
        (ftoo_manifest::ReleaseGate::Complete, 5),
    ] {
        draft.coverage.release_gate = gate;
        let graph = validate(draft.clone())?;
        let report = crate::runner::audit_entries(&graph, &[])?;
        assert_eq!(report.outcomes(), []);
        assert_eq!(report.assessment().exit_code(), code);
    }
    assert_eq!(counts(), before);
    Ok(())
}
