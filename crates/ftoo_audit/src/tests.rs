type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keep each proof mutation, exact refusal and restoration together"
)]
fn defined_application_contract_refusals_restore_exact_proofs() -> TestResult {
    use nrz_cas::{
        Fault, Location,
        proof::{ApplicationDraft, Certificate, Primitive, Statement},
    };
    let mut frame = Frame::new(control_limits(), false)?;
    let jet = frame.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])?;
    let source = frame.polynomial_seed(&jet)?;
    let accepted = frame.defined(&source)?;
    let judgment = accepted.judgment().clone();
    let original = ApplicationDraft {
        rule: Primitive::DefinedTotal,
        context: frame.context.clone(),
        conclusion: judgment.statement.clone(),
        premises: vec![],
        substitutions: vec![],
        certificate: Certificate::None,
    };
    let subset = ApplicationDraft {
        rule: Primitive::RegionConstruct,
        context: frame.context.clone(),
        conclusion: Statement::Subset {
            subset: frame.region.clone(),
            superset: frame.region.clone(),
        },
        premises: vec![],
        substitutions: vec![],
        certificate: Certificate::None,
    };
    let other = frame.cas.check_proof(
        &frame.context,
        std::slice::from_ref(&subset),
        &mut frame.budget,
    )?;
    let weakened = frame.cas.check_proof(
        &frame.context,
        &[
            original.clone(),
            subset,
            ApplicationDraft {
                rule: Primitive::ContextWeaken,
                premises: vec![0, 1],
                ..original.clone()
            },
        ],
        &mut frame.budget,
    )?;
    assert!(other.is_unconditional());
    assert!(weakened.is_unconditional());
    assert_eq!(weakened.judgment(), &judgment);
    let x = frame.x.clone();
    let different_judgment = frame.defined(&x)?;
    for (fact, location) in [
        (&other, "source defined primitive"),
        (&weakened, "source defined premises"),
        (&different_judgment, "fixed jet contract"),
    ] {
        frame.check_defined(&source, &accepted)?;
        let snapshot = frame.cas.canonical_snapshot(&mut frame.budget)?;
        assert_eq!(
            frame.check_defined(&source, fact),
            Err(Fault::invalid(InvalidCode::Rule, Location::new(location)))
        );
        assert_eq!(frame.cas.canonical_snapshot(&mut frame.budget)?, snapshot);
        frame.check_defined(&source, &accepted)?;
        assert_eq!(accepted.judgment(), &judgment);
        let restored = frame.cas.check_proof(
            &frame.context,
            std::slice::from_ref(&original),
            &mut frame.budget,
        )?;
        assert_eq!(restored.proof(), accepted.proof());
    }
    // Malformed DefinedTotal applications cannot become VerifiedWitness values:
    // the engine checks their actual certificate and premise payloads before publication.
    for certificate in [false, true] {
        let mut changed = original.clone();
        let valid = frame.cas.check_proof(
            &frame.context,
            std::slice::from_ref(&changed),
            &mut frame.budget,
        )?;
        assert_eq!(valid.proof(), accepted.proof());
        if certificate {
            changed.certificate = Certificate::Congruence(Vec::new());
        } else {
            changed.premises = vec![0];
        }
        let steps = if certificate {
            vec![changed.clone()]
        } else {
            vec![original.clone(), changed.clone()]
        };
        let snapshot = frame.cas.canonical_snapshot(&mut frame.budget)?;
        assert_eq!(
            frame
                .cas
                .check_proof(&frame.context, &steps, &mut frame.budget)
                .unwrap_err(),
            Fault::invalid(
                InvalidCode::Rule,
                Location::new(if certificate {
                    "structural totality"
                } else {
                    "typed proof admission"
                })
            )
        );
        assert_eq!(frame.cas.canonical_snapshot(&mut frame.budget)?, snapshot);
        changed = original.clone();
        assert_eq!(changed.certificate, Certificate::None);
        assert_eq!(changed.premises, [] as [u32; 0]);
        let restored = frame
            .cas
            .check_proof(&frame.context, &[changed], &mut frame.budget)?;
        assert_eq!(restored.proof(), accepted.proof());
        frame.check_defined(&source, &restored)?;
    }
    Ok(())
}

#[test]
fn constant_and_linear_seed_emission_has_no_zero_power() -> TestResult {
    use nrz_cas::syntax::Operator;
    for constant in [(0, 1), (2, 3), (-2, 3)] {
        let mut frame = Frame::new(profile(), false)?;
        let jet = frame.rationals(&[constant])?;
        let source = frame.polynomial_seed(&jet)?;
        assert_eq!(
            frame.cas.expression(&source)?.0,
            &Operator::Rational(jet[0].clone())
        );
        let defined = frame.defined(&source)?;
        assert!(defined.is_unconditional());
        assert!(frame.source(&source, &defined)?.is_unconditional());
    }
    let mut frame = Frame::new(profile(), false)?;
    let jet = frame.rationals(&[(0, 1), (1, 3)])?;
    let source = frame.polynomial_seed(&jet)?;
    let coefficient = frame.node(Operator::Rational(jet[1].clone()), &[])?;
    let expected = frame.node(Operator::Mul, &[coefficient, frame.x.clone()])?;
    assert_eq!(source, expected);
    let defined = frame.defined(&source)?;
    assert!(defined.is_unconditional());
    assert!(frame.source(&source, &defined)?.is_unconditional());
    Ok(())
}

#[test]
fn complete_call_budget_boundaries() -> TestResult {
    for producer in ProducerId::ALL {
        for claimed in [None, Some((0, 1))] {
            let total = if claimed.is_none() {
                2_202_972
            } else if producer.descriptor().target == 1 {
                2_294_475
            } else {
                2_294_507
            };
            for kind in [BudgetKind::Work, BudgetKind::ReplayWork] {
                let n = if kind == BudgetKind::Work {
                    total + 37
                } else {
                    1_190_540
                };
                let mut limits = profile();
                if kind == BudgetKind::Work {
                    limits.work = n;
                } else {
                    limits.replay_work = n;
                }
                let (accepted, measurement) = theory::checked_claim_with_prior(
                    producer.descriptor(),
                    limits.clone(),
                    false,
                    claimed,
                    37,
                )?;
                assert_eq!(measurement.total_work, total + 37);
                assert_eq!(measurement.max_replay_work, 1_190_540);
                if kind == BudgetKind::ReplayWork && claimed.is_some() {
                    assert_eq!(measurement.cumulative_replay_work, 1_248_034);
                    assert!(measurement.cumulative_replay_work > limits.replay_work);
                }
                let original = limits.clone();
                if kind == BudgetKind::Work {
                    limits.work -= 1;
                } else {
                    limits.replay_work -= 1;
                }
                let error = theory::checked_claim_with_prior(
                    producer.descriptor(),
                    limits,
                    false,
                    claimed,
                    37,
                )
                .unwrap_err();
                let contrary = kind == BudgetKind::Work && claimed.is_some();
                let expected = RunError::Execution {
                    claim: producer.descriptor().claim,
                    stage: if contrary {
                        RunStage::Projection
                    } else {
                        RunStage::Replay
                    },
                    cause: RunCause::Engine(nrz_cas::Fault::exhausted(
                        kind,
                        n - 1,
                        n,
                        nrz_cas::Location::new(if contrary {
                            "typed proof admission"
                        } else {
                            "typed witness DAG"
                        }),
                    )),
                };
                assert_eq!(error, expected);
                assert_eq!(error.to_string(), format!("{expected:?}"));
                let (restored, _) = theory::checked_claim_with_prior(
                    producer.descriptor(),
                    original.clone(),
                    false,
                    claimed,
                    37,
                )?;
                assert_eq!(restored.limits(), &original);
                assert_eq!(restored, accepted);
            }
        }
    }
    Ok(())
}

#[test]
fn replay_boundary_refusal_is_atomic_without_refunding_work() -> TestResult {
    for kind in [BudgetKind::Work, BudgetKind::ReplayWork] {
        let descriptor = ProducerId::LangevinCoefficientOneValue.descriptor();
        let (accepted, _) =
            theory::checked_claim_with_prior(descriptor, profile(), false, None, 37)?;
        let mut limits = profile();
        if kind == BudgetKind::Work {
            limits.work = 2_202_972 + 37 - 1;
        } else {
            limits.replay_work = 1_190_540 - 1;
        }
        let mut f = Frame::new(limits.clone(), false)?;
        f.budget
            .work(37, nrz_cas::Location::new("audit prior spend"))?;
        let jet = f.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])?;
        let expression = f.polynomial_seed(&jet)?;
        let defined = f.defined(&expression)?;
        let source = f.source(&expression, &defined)?;
        let (ode, rows) = f.term(&theory::ode(), &jet, 4)?;
        let (residual, member) = f.residual(&source, &ode, &rows)?;
        let evidence = [f.export(&source)?, f.export(&residual)?, f.export(&member)?];
        assert_eq!(&evidence, accepted.evidence());
        let expected = f.expectations()?;
        let mut observer = Budget::new(control_limits())?;
        let snapshot = f.cas.canonical_snapshot(&mut observer)?;
        let before = f.budget.usage().work;
        let fault = f.replay(&evidence, &expected).unwrap_err();
        let n = if kind == BudgetKind::Work {
            2_202_972 + 37
        } else {
            1_190_540
        };
        assert_eq!(
            fault,
            nrz_cas::Fault::exhausted(kind, n - 1, n, nrz_cas::Location::new("typed witness DAG"))
        );
        assert!(f.budget.usage().work > before);
        assert_eq!(f.budget.limits(), &limits);
        assert_eq!(f.cas.canonical_snapshot(&mut observer)?, snapshot);
        // A separately provisioned ledger restores the ceiling without refunding the spent ledger.
        let spent = f.budget.usage().work;
        let (restored, _) =
            theory::checked_claim_with_prior(descriptor, profile(), false, None, 37)?;
        assert_eq!(f.budget.usage().work, spent);
        assert_eq!(restored.limits(), &profile());
        assert_eq!(restored, accepted);
    }
    Ok(())
}

#[test]
fn complete_logical_policy_accepts_21_and_refuses_20() -> TestResult {
    use nrz_cas::request::{CoveredInput, Outcome, Request, SearchPolicy, Task};
    let mut f = Frame::new(profile(), false)?;
    let root = f.cas.materialize_derived(&f.x, &f.x, &mut f.budget)?;
    let receipt = f.cas.walk_derived(&root, &mut f.budget)?.receipt;
    let request = Request {
        task: Task::Simplify,
        context: f.context.clone(),
        inputs: vec![CoveredInput { root, receipt }],
        premises: Vec::new(),
        limits: nrz_cas::limits::LimitsOverride::default(),
    };
    let mut policy = SearchPolicy::default();
    assert_eq!(policy.costs.version, 21);
    let original = policy.costs.clone();
    let Outcome::Proved { result, .. } = f.cas.run_with_policy(&request, &policy) else {
        panic!("policy acceptance");
    };
    assert!(result.witness.is_unconditional());
    let snapshot = f.cas.canonical_snapshot(&mut f.budget)?;
    policy.costs.version = 20;
    let Outcome::Invalid { fault, usage } = f.cas.run_with_policy(&request, &policy) else {
        panic!("policy refusal");
    };
    assert_eq!(
        fault,
        nrz_cas::Fault::invalid(InvalidCode::Policy, nrz_cas::Location::new("cost policy"))
    );
    assert!(usage.work > 0);
    assert_eq!(f.cas.canonical_snapshot(&mut f.budget)?, snapshot);
    policy.costs.version = 21;
    assert_eq!(policy.costs, original);
    let Outcome::Proved { result, .. } = f.cas.run_with_policy(&request, &policy) else {
        panic!("restored policy acceptance");
    };
    assert!(result.witness.is_unconditional());
    Ok(())
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keep accepted, mutated and restored structural contracts together"
)]
fn structural_seed_acceptance_and_exact_refusals() -> TestResult {
    use nrz_cas::{
        Fault, Location, Reference,
        domain::{
            AmbientDomain, AssumptionDraft, ContextDraft, HypothesisRef, RegionDraft, RegionExpr,
        },
        proof::{ApplicationDraft, Certificate, Primitive, Statement},
        syntax::{Draft, Operator, PredicateKind},
    };
    let mut f = Frame::new(profile(), false)?;
    let jet = f.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])?;
    let original = f.polynomial_seed(&jet)?;
    let accepted = f.defined(&original)?;
    let original_judgment = accepted.judgment().clone();
    let original_bytes = f.export(&accepted)?;
    let source = f.source(&original, &accepted)?;
    let (expression, rows) = f.term(&theory::ode(), &jet, 4)?;
    let (residual, member) = f.residual(&source, &expression, &rows)?;
    assert!(accepted.is_unconditional());
    assert!(source.is_unconditional());
    assert!(residual.is_unconditional());
    assert!(member.is_unconditional());
    assert_eq!(
        rows.last().ok_or("rows")?.coefficients,
        vec![nrz_cas::Rational::zero(); 4]
    );
    // Host-side diagnostic from the same term walk that builds the submitted rows.
    // This nonzero full tail is not an engine-checked fact.
    let (_, full) = f.term(&theory::ode(), &jet, 8)?;
    assert_eq!(
        full.last().ok_or("full rows")?.coefficients,
        f.rationals(&[
            (0, 1),
            (0, 1),
            (0, 1),
            (0, 1),
            (0, 1),
            (-2, 135),
            (0, 1),
            (1, 2025)
        ])?
    );
    let zero_power = f.node(Operator::IntegerPower(0), &[f.x.clone()])?;
    let snapshot = f.cas.canonical_snapshot(&mut f.budget)?;
    assert_eq!(
        f.cas
            .defined_total(&zero_power, &f.context, &mut f.budget)
            .unwrap_err(),
        Fault::invalid(
            InvalidCode::UnsupportedFragment,
            Location::new("structural totality")
        )
    );
    assert_eq!(f.cas.canonical_snapshot(&mut f.budget)?, snapshot);
    let restored = f.defined(&original)?;
    assert_eq!(restored.judgment(), &original_judgment);
    assert_eq!(f.export(&restored)?, original_bytes);
    let predicate = f.condition(PredicateKind::Defined, std::slice::from_ref(&original))?;
    let wrong_region = f.admit(Draft::Region(RegionDraft {
        header: Reference::Existing(f.header.clone()),
        scope: Vec::new(),
        ambient: AmbientDomain::Product {
            coordinates: f.coefficients.clone(),
        },
        expression: RegionExpr::GuardTrue {
            base: Reference::Existing(f.region.clone()),
            predicate: Reference::Existing(predicate.clone()),
        },
    }))?;
    let mut application = ApplicationDraft {
        rule: Primitive::DefinedTotal,
        context: f.context.clone(),
        conclusion: Statement::PredicateFact {
            region: f.region.clone(),
            predicate: predicate.clone(),
        },
        premises: Vec::new(),
        substitutions: Vec::new(),
        certificate: Certificate::None,
    };
    f.cas
        .check_proof(&f.context, &[application.clone()], &mut f.budget)?;
    application.conclusion = Statement::PredicateFact {
        region: wrong_region,
        predicate: predicate.clone(),
    };
    let snapshot = f.cas.canonical_snapshot(&mut f.budget)?;
    assert_eq!(
        f.cas
            .check_proof(&f.context, &[application.clone()], &mut f.budget)
            .unwrap_err(),
        Fault::invalid(InvalidCode::Rule, Location::new("structural totality"))
    );
    assert_eq!(f.cas.canonical_snapshot(&mut f.budget)?, snapshot);
    application.conclusion = original_judgment.statement.clone();
    let restored = f
        .cas
        .check_proof(&f.context, &[application], &mut f.budget)?;
    assert_eq!(restored.judgment(), &original_judgment);
    let original_context = f.context.clone();
    f.context = f.admit(Draft::Context(ContextDraft::Root {
        header: Reference::Existing(f.header.clone()),
        admissible: Reference::Existing(f.region.clone()),
        assumptions: vec![AssumptionDraft {
            key: "source-defined".into(),
            condition: Reference::Existing(predicate.clone()),
        }],
    }))?;
    let conditional = f.cas.check_proof(
        &f.context,
        &[ApplicationDraft {
            rule: Primitive::ContextHypothesis,
            context: f.context.clone(),
            conclusion: Statement::PredicateFact {
                region: f.region.clone(),
                predicate,
            },
            premises: Vec::new(),
            substitutions: Vec::new(),
            certificate: Certificate::Hypothesis(HypothesisRef {
                context: f.context.clone(),
                slot: 0,
            }),
        }],
        &mut f.budget,
    )?;
    assert!(!conditional.is_unconditional());
    let snapshot = f.cas.canonical_snapshot(&mut f.budget)?;
    let actual = f
        .check_defined(&original, &conditional)
        .map_err(|fault| crate::runner::fault(OneValue, RunStage::Definedness, fault));
    let expected = RunError::Execution {
        claim: OneValue,
        stage: RunStage::Definedness,
        cause: RunCause::Engine(Fault::invalid(
            InvalidCode::Rule,
            Location::new("source unconditionality"),
        )),
    };
    assert_eq!(actual, Err(expected.clone()));
    assert_eq!(expected.to_string(), format!("{expected:?}"));
    assert_eq!(f.cas.canonical_snapshot(&mut f.budget)?, snapshot);
    f.context = original_context;
    let restored = f.defined(&original)?;
    assert_eq!(restored.judgment(), &original_judgment);
    f.source(&original, &restored)?;
    Ok(())
}

#[test]
fn complete_routes_reproduce_across_fresh_and_warm_stores() -> TestResult {
    for producer in ProducerId::ALL {
        for repetition in 0..3 {
            if repetition == 2 {
                crate::test_support::solving_trial()?;
            }
            let mut f = Frame::new(profile(), false)?;
            let jet = f.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])?;
            let expression = f.polynomial_seed(&jet)?;
            let defined = f.defined(&expression)?;
            let source = f.source(&expression, &defined)?;
            let (ode, rows) = f.term(&theory::ode(), &jet, 4)?;
            let (residual, member) = f.residual(&source, &ode, &rows)?;
            let member_work = f.budget.usage().work;
            let evidence = [f.export(&source)?, f.export(&residual)?, f.export(&member)?];
            let payload = f.cas.counts().payload_bytes;
            let expected = f.expectations()?;
            let before = f.budget.usage().work;
            f.replay(&evidence, &expected)?;
            let measured = (
                member_work,
                f.budget.usage().work,
                f.budget.usage().work - before,
                evidence.each_ref().map(Vec::len),
                payload,
            );
            println!(
                "REPRO target={} repetition={repetition} measured={measured:?}",
                producer.descriptor().target
            );
            assert_eq!(
                measured,
                (993_246, 2_202_972, 1_190_540, [1_131, 3_000, 3_157], 24_788)
            );
            for claimed in [None, Some((0, 1))] {
                let (receipt, measurement) =
                    theory::checked_claim(producer.descriptor(), profile(), false, claimed)?;
                let total = if claimed.is_none() {
                    2_202_972
                } else if receipt.target() == 1 {
                    2_294_475
                } else {
                    2_294_507
                };
                let replay = if claimed.is_none() {
                    1_190_540
                } else {
                    1_248_034
                };
                println!(
                    "REPRO target={} repetition={repetition} contrary={} total={} cumulative_replay_work={}",
                    receipt.target(),
                    claimed.is_some(),
                    measurement.total_work,
                    measurement.cumulative_replay_work
                );
                assert_eq!(measurement.max_replay_work, 1_190_540);
                assert_eq!(
                    (
                        measurement.member_work,
                        measurement.total_work,
                        measurement.cumulative_replay_work
                    ),
                    (993_246, total, replay)
                );
                assert_eq!(
                    receipt.evidence().each_ref().map(Vec::len),
                    [1_131, 3_000, 3_157]
                );
            }
        }
    }
    Ok(())
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
    InvalidCode, Rational,
    limits::{Budget, BudgetKind, Limits},
};
fn profile() -> Limits {
    theory::limits(BudgetProfile::PinnedTrialLimitsV2)
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
            receipt.summary().source,
            "(1/3)*x - (1/45)*IntegerPower(x,3)"
        );
        assert_eq!(receipt.summary().clause, "PolynomialSeed");
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
        assert!(receipt.summary().subject.contains("polynomial seed"));
        assert_eq!(receipt.pin(), ftoo_manifest::ENGINE_PIN);
        assert_eq!(receipt.limits(), &profile());
        assert_eq!(receipt.revision(), 2);
        assert_eq!(
            graph.status(receipt.claim()).ok_or("status")?.evidence(),
            ftoo_manifest::Evidence::Deferred
        );
    }
    for m in run.measurements() {
        assert_eq!(
            (m.member_work, m.total_work, m.cumulative_replay_work),
            (993_246, 2_202_972, 1_190_540)
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
fn m023_real_exhaustion_preserves_resource_and_stage() -> TestResult {
    let descriptor = ProducerId::LangevinCoefficientOneValue.descriptor();
    theory::checked(descriptor, profile(), false)?;
    let fault = theory::checked(
        descriptor,
        Limits {
            work: 1,
            ..profile()
        },
        false,
    )
    .unwrap_err();
    assert_eq!(
        fault,
        RunError::Execution {
            claim: OneValue,
            stage: RunStage::Setup,
            cause: RunCause::Engine(nrz_cas::Fault::exhausted(
                BudgetKind::Work,
                1,
                2,
                nrz_cas::Location::new("draft validation")
            ))
        }
    );
    let (restored, _) = theory::checked(descriptor, profile(), false)?;
    assert_eq!(restored.limits(), &profile());
    Ok(())
}
#[test]
fn empty_guarded_region_never_yields_accepted_receipt() -> TestResult {
    let descriptor = ProducerId::LangevinCoefficientOneValue.descriptor();
    let (original, _) = theory::checked(descriptor, profile(), false)?;
    let fault = theory::checked(descriptor, profile(), true).unwrap_err();
    assert_eq!(
        fault,
        RunError::Execution {
            claim: OneValue,
            stage: RunStage::Replay,
            cause: RunCause::Engine(nrz_cas::Fault::invalid(
                InvalidCode::EndpointMismatch,
                nrz_cas::Location::new("formal series arithmetic")
            ))
        }
    );
    let (restored, _) = theory::checked(descriptor, profile(), false)?;
    assert_eq!(restored, original);
    Ok(())
}
#[test]
fn wrong_target_never_yields_accepted_receipt() -> TestResult {
    let original = ProducerId::LangevinCoefficientThreeValue.descriptor();
    theory::checked(original, profile(), false)?;
    let mut descriptor = original;
    descriptor.target = 1;
    assert_eq!(
        theory::checked(descriptor, profile(), false).unwrap_err(),
        RunError::Execution {
            claim: ThreeValue,
            stage: RunStage::Projection,
            cause: RunCause::ProjectionMismatch
        }
    );
    descriptor = original;
    assert_eq!(descriptor, original);
    let (restored, _) = theory::checked(descriptor, profile(), false)?;
    assert_eq!(restored.target(), 3);
    Ok(())
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
#[expect(
    clippy::too_many_lines,
    reason = "independent full 71-field fixture and measured headroom derivation"
)]
fn pinned_trial_limits_full_field_fixture() -> TestResult {
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
        work: 3_000_000,
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
        replay_work: 1_500_000,
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
            work: 3_000_000,
            replay_work: 1_500_000,
            ..defaults
        }
    );
    // Complete-route measurements for both targets; each field uses the larger.
    // Round upward to 500,000, then step by 500,000 until headroom is at least 25%.
    let mut largest = [0, 0];
    for producer in ProducerId::ALL {
        for claimed in [None, Some((0, 1))] {
            let (_, measured) =
                theory::checked_claim(producer.descriptor(), profile(), false, claimed)?;
            largest[0] = largest[0].max(measured.total_work);
            largest[1] = largest[1].max(measured.max_replay_work);
        }
    }
    assert_eq!(largest, [2_294_507, 1_190_540]);
    for (measured, reviewed) in largest
        .into_iter()
        .zip([expected.work, expected.replay_work])
    {
        let mut derived = measured.div_ceil(500_000) * 500_000;
        while (derived - measured) * 4 < measured {
            derived += 500_000;
        }
        assert_eq!(derived, reviewed);
    }
    Ok(())
}

#[test]
fn equation_window_leaves_terminal_coefficient_free() -> TestResult {
    for terminal in [(0, 1), (7, 2), (-2, 3)] {
        let mut f = Frame::new(profile(), false)?;
        let j = f.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), terminal])?;
        let expression = f.polynomial_seed(&j)?;
        let defined = f.defined(&expression)?;
        let source = f.source(&expression, &defined)?;
        let nrz_cas::proof::Statement::Series(statement) = &source.judgment().statement else {
            panic!("source");
        };
        let nrz_cas::series::SeriesStatement::Source(statement) = statement.as_ref() else {
            panic!("source");
        };
        let output = f.cas.series_object(&statement.output)?;
        assert_eq!(output.order, 5);
        assert_eq!(
            f.cas.expression(&output.coefficients[4])?.0,
            &nrz_cas::syntax::Operator::Rational(j[4].clone())
        );
        let (ode, rows) = f.term(&theory::ode(), &j, 4)?;
        // Only degrees 0..3 enter: c4 is unconstrained terminal candidate data.
        assert!(rows.iter().all(|row| row.coefficients.len() == 4));
        assert_eq!(
            rows.last().ok_or("trace")?.coefficients,
            vec![Rational::zero(); 4]
        );
        let (residual, _) = f.residual(&source, &ode, &rows)?;
        let nrz_cas::proof::Statement::Series(statement) = &residual.judgment().statement else {
            panic!("residual");
        };
        let nrz_cas::series::SeriesStatement::EvaluatedResidual {
            output,
            derivative_order,
            order,
            ..
        } = statement.as_ref()
        else {
            panic!("residual");
        };
        assert_eq!((*derivative_order, *order), (1, 4));
        for coefficient in &f.cas.series_object(output)?.coefficients {
            assert_eq!(
                f.cas.expression(coefficient)?.0,
                &nrz_cas::syntax::Operator::Rational(Rational::zero())
            );
        }
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
    changed_member_input(false).map(|_| ())
}
fn changed_member_input(sign: bool) -> Result<nrz_cas::Fault, Box<dyn std::error::Error>> {
    let mut f = Frame::new(control_limits(), false)?;
    let original = f.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])?;
    let expression = f.polynomial_seed(&original)?;
    let defined = f.defined(&expression)?;
    let accepted = f.source(&expression, &defined)?;
    let (ode, rows) = f.term(&theory::ode(), &original, 4)?;
    let (_, original_member) = f.residual(&accepted, &ode, &rows)?;
    let judgment = original_member.judgment().clone();
    let mut jet = original.clone();
    let mut term = theory::ode();
    if sign {
        let crate::engine::Term::Binary(op, ..) = &mut term else {
            panic!("ODE");
        };
        *op = nrz_cas::syntax::Operator::Add;
    } else {
        jet[3] = Rational::from_ratio(1, 45, &mut f.budget)?;
    }
    let expression = f.polynomial_seed(&jet)?;
    let defined = f.defined(&expression)?;
    let source = f.source(&expression, &defined)?;
    let (expression, rows) = f.term(&term, &jet, 4)?;
    let snapshot = f.cas.canonical_snapshot(&mut f.budget)?;
    let fault = f.residual(&source, &expression, &rows).unwrap_err();
    assert_eq!(
        fault,
        nrz_cas::Fault::invalid(
            InvalidCode::Rule,
            nrz_cas::Location::new("elementary solving")
        )
    );
    assert_eq!(f.cas.canonical_snapshot(&mut f.budget)?, snapshot);
    jet = original.clone();
    term = theory::ode();
    assert_eq!(jet, original);
    let expression = f.polynomial_seed(&jet)?;
    let defined = f.defined(&expression)?;
    let source = f.source(&expression, &defined)?;
    let (expression, rows) = f.term(&term, &jet, 4)?;
    let (_, restored) = f.residual(&source, &expression, &rows)?;
    assert_eq!(restored.judgment(), &judgment);
    Ok(fault)
}
#[test]
fn independent_replay_pins_subject_derivative_and_window() -> TestResult {
    use nrz_cas::{proof::Statement, series::SeriesStatement};
    let graph = seed()?;
    let run = crate::run(&graph)?;
    for receipt in run.receipts() {
        for mutation in 1..=5 {
            let mut frame = Frame::new(control_limits(), false)?;
            let original = frame.expectations()?;
            let alternate = alternate_context(&mut frame)?;
            frame.replay(receipt.evidence(), &original)?;
            let mut expected = original.clone();
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
                3 | 4 => {
                    let nrz_cas::series::ResidualSubject::Source(source) = subject else {
                        panic!("source")
                    };
                    if mutation == 3 {
                        source.source = frame.x.clone();
                    } else {
                        source.domain = alternate.0.clone();
                    }
                }
                _ => expected[0].context = alternate.1.clone(),
            }
            let snapshot = frame.cas.canonical_snapshot(&mut frame.budget)?;
            let (code, location) = match mutation {
                3 => (InvalidCode::EndpointMismatch, "formal series arithmetic"),
                5 => (InvalidCode::Context, "scalar statement view"),
                _ => (InvalidCode::Rule, "formal series arithmetic"),
            };
            assert_eq!(
                frame.replay(receipt.evidence(), &expected),
                Err(nrz_cas::Fault::invalid(
                    code,
                    nrz_cas::Location::new(location)
                ))
            );
            assert_eq!(frame.cas.canonical_snapshot(&mut frame.budget)?, snapshot);
            expected = original.clone();
            assert_eq!(expected, original);
            frame.replay(receipt.evidence(), &expected)?;
        }
    }
    Ok(())
}

#[test]
fn source_and_residual_exports_replay_independently() -> TestResult {
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
                subject: nrz_cas::series::ResidualSubject::Source(source),
                ..
            } = series.as_ref()
            else {
                panic!("polynomial source")
            };
            // Test-only Source expectation, from freshly specified syntax, not evidence.
            let source = nrz_cas::proof::Judgment {
                statement: nrz_cas::proof::Statement::Series(Box::new(
                    nrz_cas::series::SeriesStatement::Source(source.clone()),
                )),
                ..residual.clone()
            };
            let expected = [source, residual];
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
fn defined_export_replays_in_proof_empty_reordered_receivers() -> TestResult {
    use nrz_cas::{
        domain::Layer,
        proof::{Judgment, Statement},
        syntax::{Operator, PredicateKind},
    };
    let mut producer = Frame::new(profile(), false)?;
    let jet = producer.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])?;
    let source = producer.polynomial_seed(&jet)?;
    let defined = producer.defined(&source)?;
    let bytes = producer.export(&defined)?;
    for reordered in [false, true] {
        let mut receiver = Frame::new(profile(), false)?;
        if reordered {
            let values = receiver.rationals(&[(1, 45), (1, 3)])?;
            for value in values {
                receiver.node(Operator::Rational(value), &[])?;
            }
        }
        let values = receiver.rationals(&[(1, 3), (1, 45)])?;
        let third = receiver.node(Operator::Rational(values[0].clone()), &[])?;
        let linear = receiver.node(Operator::Mul, &[third, receiver.x.clone()])?;
        let power = receiver.node(Operator::IntegerPower(3), &[receiver.x.clone()])?;
        let forty_fifth = receiver.node(Operator::Rational(values[1].clone()), &[])?;
        let cubic = receiver.node(Operator::Mul, &[forty_fifth, power])?;
        let original = receiver.node(Operator::Sub, &[linear, cubic])?;
        let predicate = receiver.condition(PredicateKind::Defined, &[original])?;
        let expected = Judgment {
            context: receiver.context.clone(),
            layer: Layer::Logic,
            space: None,
            statement: Statement::PredicateFact {
                region: receiver.region.clone(),
                predicate,
            },
            residuals: Vec::new(),
        };
        let replayed = receiver
            .cas
            .replay(&bytes, &expected, &mut receiver.budget)?;
        assert_eq!(replayed.judgment(), &expected);
        assert!(replayed.is_unconditional());
        let mut altered = bytes.clone();
        altered[0] ^= 1;
        let snapshot = receiver.cas.canonical_snapshot(&mut receiver.budget)?;
        assert_eq!(
            receiver
                .cas
                .replay(&altered, &expected, &mut receiver.budget)
                .unwrap_err(),
            nrz_cas::Fault::invalid(
                InvalidCode::UnsupportedVersion,
                nrz_cas::Location::new("typed witness import")
            )
        );
        assert_eq!(
            receiver.cas.canonical_snapshot(&mut receiver.budget)?,
            snapshot
        );
        altered.clone_from(&bytes);
        assert_eq!(altered, bytes);
        let restored = receiver
            .cas
            .replay(&altered, &expected, &mut receiver.budget)?;
        assert_eq!(restored.judgment(), &expected);
    }
    Ok(())
}
#[test]
fn changed_residual_sign_refuses_member() -> TestResult {
    changed_member_input(true).map(|_| ())
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
fn work_boundary_first() -> Result<(crate::Receipt, crate::Measurement), RunError> {
    theory::checked_claim_with_prior(
        ProducerId::LangevinCoefficientOneValue.descriptor(),
        Limits {
            work: 2_202_972 + 37 - 1,
            ..profile()
        },
        false,
        None,
        37,
    )
}
fn replay_boundary_first() -> Result<(crate::Receipt, crate::Measurement), RunError> {
    theory::checked_claim_with_prior(
        ProducerId::LangevinCoefficientOneValue.descriptor(),
        Limits {
            replay_work: 1_190_540 - 1,
            ..profile()
        },
        false,
        None,
        37,
    )
}
#[test]
fn complete_boundary_exhaustion_reports_undecided() -> TestResult {
    let graph = seed()?;
    let accepted = crate::audit(&graph)?;
    for (produce, kind, n) in [
        (
            work_boundary_first as fn() -> Result<(crate::Receipt, crate::Measurement), RunError>,
            BudgetKind::Work,
            2_202_972 + 37,
        ),
        (
            replay_boundary_first as fn() -> Result<(crate::Receipt, crate::Measurement), RunError>,
            BudgetKind::ReplayWork,
            1_190_540,
        ),
    ] {
        let mut entries = crate::registry::REGISTRY;
        entries[0].produce = produce;
        entries[1].produce = must_not_execute;
        let report = crate::runner::audit_entries(&graph, &entries)?;
        assert_eq!(
            report.outcomes()[0].verdict(),
            &crate::Verdict::Undecided {
                stage: RunStage::Replay,
                exhaustion: nrz_cas::Fault::exhausted(
                    kind,
                    n - 1,
                    n,
                    nrz_cas::Location::new("typed witness DAG")
                ),
            }
        );
        assert_eq!(
            report.outcomes()[1].verdict(),
            &crate::Verdict::Deferred(ftoo_manifest::DeferredReason::Premise {
                claim: OneValue,
                evidence: ftoo_manifest::VerdictClass::Undecided,
            })
        );
        assert_eq!(report.measurements(), []);
        entries = crate::registry::REGISTRY;
        let restored = crate::runner::audit_entries(&graph, &entries)?;
        assert_eq!(restored.gated(), accepted.gated());
    }
    Ok(())
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
        "COUNTERSTATEMENT complete work={} cumulative replay Work={}",
        report.measurements()[0].total_work,
        report.measurements()[0].cumulative_replay_work
    );
    assert!(report.measurements()[0].total_work <= 3_000_000);
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
        exhaustion,
        &nrz_cas::Fault::exhausted(
            BudgetKind::Work,
            1,
            2,
            nrz_cas::Location::new("draft validation")
        )
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
    let fault = changed_member_input(false)?;
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
    let graph = seed()?;
    let run = crate::run(&graph)?;
    let original = run.receipts()[0].evidence()[1].clone();
    let mut frame = Frame::new(control_limits(), false)?;
    let expected = frame.expectations()?;
    frame
        .cas
        .replay(&original, &expected[0], &mut frame.budget)?;
    let snapshot = frame.cas.canonical_snapshot(&mut frame.budget)?;
    let fault = frame
        .cas
        .replay(&[], &expected[0], &mut frame.budget)
        .unwrap_err();
    assert_eq!(
        fault,
        nrz_cas::Fault::invalid(
            InvalidCode::UnsupportedVersion,
            nrz_cas::Location::new("typed witness import")
        )
    );
    assert_eq!(frame.cas.canonical_snapshot(&mut frame.budget)?, snapshot);
    let restored = original.clone();
    assert_eq!(restored, original);
    frame
        .cas
        .replay(&restored, &expected[0], &mut frame.budget)?;
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
    for mutation in 0..2 {
        let original = report.snapshot().clone();
        let mut snapshot = original.clone();
        if mutation == 0 {
            snapshot.pin = EnginePin("stale");
        } else {
            snapshot.descriptors[0].revision = 1;
        }
        let error = crate::AuditReport::from_outcomes(
            &graph,
            snapshot,
            report.reached().to_vec(),
            report.outcomes().to_vec(),
            report.measurements().to_vec(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            crate::AuditError::Report(vec![Diagnostic {
                stage: AdmissionStage::ReportConsumption,
                rule: RuleId::M025,
                ids: vec![],
                edge: None,
                location: ftoo_manifest::DraftLocation::Provenance,
                explanation: "semantic snapshot differs from authority".into(),
            }])
        );
        let restored = crate::AuditReport::from_outcomes(
            &graph,
            original.clone(),
            report.reached().to_vec(),
            report.outcomes().to_vec(),
            report.measurements().to_vec(),
        )?;
        assert_eq!(restored.snapshot(), &original);
        assert_eq!(restored.gated(), report.gated());
    }
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
            "COUNTERSTATEMENT target={} work={} cumulative_replay_work={}",
            receipt.target(),
            measurement.total_work,
            measurement.cumulative_replay_work
        );
        assert!(measurement.total_work <= 3_000_000);
        assert!(measurement.max_replay_work <= 1_500_000);
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

fn control_limits() -> Limits {
    // Several complete accept/refuse/restore calls share this test-only ledger.
    Limits {
        work: 20_000_000,
        replay_work: 5_000_000,
        ..profile()
    }
}
fn alternate_context(
    frame: &mut Frame,
) -> Result<(nrz_cas::Handle, nrz_cas::Handle), nrz_cas::Fault> {
    use nrz_cas::{
        Reference,
        domain::{AmbientDomain, ContextDraft, RegionDraft, RegionExpr},
        syntax::{Draft, PredicateKind},
    };
    let x = frame.x.clone();
    let predicate = frame.condition(PredicateKind::Defined, &[x])?;
    let region = frame.admit(Draft::Region(RegionDraft {
        header: Reference::Existing(frame.header.clone()),
        scope: vec![],
        ambient: AmbientDomain::Product {
            coordinates: frame.coefficients.clone(),
        },
        expression: RegionExpr::GuardTrue {
            base: Reference::Existing(frame.region.clone()),
            predicate: Reference::Existing(predicate),
        },
    }))?;
    let context = frame.admit(Draft::Context(ContextDraft::Root {
        header: Reference::Existing(frame.header.clone()),
        admissible: Reference::Existing(region.clone()),
        assumptions: vec![],
    }))?;
    Ok((region, context))
}
#[test]
fn portable_bytes_restore_and_reread_at_each_receipt_boundary() -> TestResult {
    use nrz_cas::{
        proof::Statement,
        series::{ResidualSubject, SeriesStatement},
        syntax::Operator,
    };
    let graph = seed()?;
    let run = crate::run(&graph)?;
    for receipt in run.receipts() {
        for index in 0..3 {
            let mut frame = Frame::new(control_limits(), false)?;
            // Admit coefficient leaves in the opposite order before constructing expectations.
            for value in frame.rationals(&[(1, 45), (1, 3)])? {
                frame.node(Operator::Rational(value), &[])?;
            }
            let expected = frame.expectations()?;
            let Statement::Series(series) = &expected[0].statement else {
                panic!("series");
            };
            let SeriesStatement::EvaluatedResidual {
                subject: ResidualSubject::Source(source),
                ..
            } = series.as_ref()
            else {
                panic!("source");
            };
            let subject = nrz_cas::proof::Judgment {
                statement: Statement::Series(Box::new(SeriesStatement::Source(source.clone()))),
                ..expected[0].clone()
            };
            let original = receipt.evidence()[index].clone();
            let standalone = if index == 0 { &subject } else { &expected[0] };
            if index < 2 {
                frame.cas.replay(&original, standalone, &mut frame.budget)?;
            } else {
                frame.replay(receipt.evidence(), &expected)?;
            }
            let mut bytes = original.clone();
            bytes[0] ^= 1;
            let snapshot = frame.cas.canonical_snapshot(&mut frame.budget)?;
            let fault = if index < 2 {
                frame
                    .cas
                    .replay(&bytes, standalone, &mut frame.budget)
                    .unwrap_err()
            } else {
                let mut evidence = receipt.evidence().clone();
                evidence[2] = bytes.clone();
                frame.replay(&evidence, &expected).unwrap_err()
            };
            assert_eq!(
                fault,
                nrz_cas::Fault::invalid(
                    InvalidCode::UnsupportedVersion,
                    nrz_cas::Location::new("typed witness import")
                )
            );
            assert_eq!(frame.cas.canonical_snapshot(&mut frame.budget)?, snapshot);
            bytes.clone_from(&original);
            assert_eq!(bytes, original);
            if index < 2 {
                frame.cas.replay(&bytes, standalone, &mut frame.budget)?;
            } else {
                let mut evidence = receipt.evidence().clone();
                evidence[2] = bytes;
                frame.replay(&evidence, &expected)?;
            }
        }
    }
    Ok(())
}
#[test]
fn exact_dependency_and_budget_binding_mutations_restore_authority() -> TestResult {
    use ftoo_manifest::{DraftLocation, RelationKind};
    let original = seed_draft();
    for mutation in 0..3 {
        crate::run(&validate(original.clone())?)?;
        let mut draft = original.clone();
        let (rule, claim, location, message) = match mutation {
            0 => {
                draft.edges.retain(|e| {
                    !(e.source == ThreeValue
                        && e.target == OneValue
                        && e.relation == RelationKind::DependsOn)
                });
                (
                    RuleId::M022,
                    ThreeValue,
                    DraftLocation::Binding(3),
                    "direct dependencies must equal the fixed descriptor",
                )
            }
            1 => {
                draft.budgets.pop();
                (
                    RuleId::M023,
                    ThreeValue,
                    DraftLocation::Binding(3),
                    "missing, duplicate, swapped or unreviewed budget",
                )
            }
            _ => {
                draft.budgets[0].pin = EnginePin("stale");
                (
                    RuleId::M023,
                    OneValue,
                    DraftLocation::Binding(2),
                    "missing, duplicate, swapped or unreviewed budget",
                )
            }
        };
        let mut expected = vec![Diagnostic {
            stage: AdmissionStage::RegistryAdmission,
            rule,
            ids: vec![claim],
            edge: None,
            location,
            explanation: message.into(),
        }];
        if mutation == 2 {
            expected.push(Diagnostic { stage:AdmissionStage::RegistryAdmission,rule,ids:vec![OneValue],edge:None,location:DraftLocation::Budget(0),explanation:"budget is unused, duplicated or does not match the reviewed pin/profile".into() });
        }
        let graph = validate(draft)?;
        let before = counts();
        assert_eq!(
            crate::runner::execute(&graph, &crate::registry::REGISTRY).unwrap_err(),
            RunError::Admission(expected)
        );
        assert_eq!(counts(), before);
        let restored = validate(original.clone())?;
        assert_eq!(restored.budgets(), original.budgets);
        crate::run(&restored)?;
    }
    Ok(())
}
