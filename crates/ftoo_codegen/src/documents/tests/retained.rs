use super::super::{latex, markdown};
use super::*;
use ftoo_audit::retention::{RetainedNotation, accept, produce};
use ftoo_manifest::NodeId::{
    LangevinCoefficientOneValue as One, LangevinCoefficientThreeValue as Three,
};

fn text(runs: &[Inline]) -> String {
    runs.iter()
        .map(|run| match run {
            Inline::Text(text) => text.clone(),
            Inline::Tag { handle, token, .. } => format!("[{handle}/{token}]"),
            _ => panic!("coverage has only text and tags"),
        })
        .collect()
}

#[test]
#[allow(clippy::too_many_lines)] // One table pins selection, census and both independent axes.
fn retained_selected_outcomes_census_and_scope_are_derived() {
    for (selection, retained) in [(2, false), (1, false), (0, false), (2, true)] {
        let partial = selection == 1;
        let mut draft = ftoo_manifest::seed_draft();
        draft.coverage.slice.truncate(selection);
        if retained {
            for node in &mut draft.nodes {
                node.maturity = ftoo_manifest::Maturity::Retained;
            }
        }
        let graph = ftoo_manifest::validate(draft).unwrap();
        let projection = ftoo_manifest::projection_bytes(&graph).unwrap();
        let production = produce(projection.clone()).unwrap();
        let accepted = accept(projection, production.bytes().to_vec()).unwrap();
        let book = ftoo_manifest::addressing(&graph).unwrap();
        let catalogue = ftoo_manifest::notation(&book).unwrap();
        let view = RetainedNotation::new(&catalogue, &accepted).unwrap();
        let runs = super::super::retained::coverage(&book, &view, &[], "fixture", "en").unwrap();
        let mut expected = vec![
            if partial { "Audit outcomes: [v:1/g-audit-0]: certified." } else { "Audit outcomes: [v:1/g-audit-0]: certified; [v:3/g-audit-1]: certified." }.to_owned(),
            "Audit scope: finite source jet of order 5, residual order 4 and derivative order 1; analytic identification is not certified.".into(),
        ];
        if selection == 0 {
            expected = vec!["Audit outcomes: (empty).".into(), "Audit scope: no selected claim is certified; analytic identification is not certified.".into(), "Outside declared slice: 2 claims; certified 0; refuted 0; undecided 0; deferred 2. See generated/audit-report.json for identities and blockers.".into()];
        }
        if partial {
            expected.push("Outside declared slice: 1 claims; certified 0; refuted 0; undecided 0; deferred 1. See generated/audit-report.json for identities and blockers.".into());
            assert_eq!(
                accepted
                    .report()
                    .outcomes()
                    .iter()
                    .find(|row| row.claim() == Three)
                    .unwrap()
                    .verdict(),
                &ftoo_audit::Verdict::Deferred(ftoo_manifest::DeferredReason::OutsideSlice)
            );
        }
        assert_eq!(
            runs.iter().map(|runs| text(runs)).collect::<Vec<_>>(),
            expected
        );
        let mut plan = DocumentPlan {
            document: "fixture".into(),
            language: "en".into(),
            coverage: Coverage::build(
                &book,
                "Evidence context: retained audit accepted.",
                "fixture",
                "en",
            )
            .unwrap(),
            blocks: vec![],
        };
        plan.coverage.audit = runs;
        for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
            let source = if target == DocumentTarget::Markdown {
                markdown::render(&plan)
            } else {
                latex::render(&plan)
            };
            assert_eq!(check_emission(&plan, &source, target), Ok(()));
            assert_eq!(
                source.len(),
                if target == DocumentTarget::Markdown {
                    markdown::size(&plan)
                } else {
                    latex::size(&plan)
                }
            );
        }
        if selection == 2 {
            scope_controls(&book, &accepted);
        }
        let site = coverage_site(None);
        let block = Block {
            pending_names: BTreeMap::default(),
            displays: BTreeMap::default(),
            unit: "sec:fixture:axis".into(),
            depth: None,
            site: site.clone(),
            runs: vec![Inline::Math {
                definition_site: site.clone(),
                site,
                key: "value".into(),
                markdown: "x".into(),
                latex: "x".into(),
                pending: vec![crate::PendingEntry::fixture(
                    ftoo_manifest::DefinitionId::CoefficientOneValue,
                    0,
                    "x",
                    vec![ftoo_manifest::PendingReason::BackingUnbound { object: One }],
                    vec![],
                )],
            }],
        };
        // Pending inventory alone does not assert a formulation state: the accepted
        // claim and its transitive catalogue backing select the explanation.
        let with_pending =
            super::super::retained::coverage(&book, &view, &[block], "fixture", "en").unwrap();
        if selection != 0 {
            expected.insert(2, if retained { "Certification concerns the finite coefficient claim; pending notation concerns separate notation eligibility conditions." } else { "Certification concerns the finite coefficient claim; pending notation concerns a proposed formulation." }.into());
        }
        assert_eq!(
            with_pending
                .iter()
                .map(|runs| text(runs))
                .collect::<Vec<_>>(),
            expected
        );
    }
}

fn scope_controls(
    book: &ftoo_manifest::AddressBook<'_>,
    accepted: &ftoo_audit::retention::AcceptedRetention,
) {
    let ftoo_audit::Verdict::Certified(receipt) = accepted.report().outcomes()[0].verdict() else {
        panic!("certified seed")
    };
    let original = receipt.summary();
    let mut divergent = original.clone();
    // A private admitted-contract fixture tests complete summary equality, including
    // data that does not occur in the three-order reader sentence.
    divergent.jet.swap(0, 1);
    let cases = [
        (vec![], vec!["Audit scope: no selected claim is certified; analytic identification is not certified.".to_owned()]),
        (vec![(0, 0, One, original), (1, 1, Three, original)], vec!["Audit scope: finite source jet of order 5, residual order 4 and derivative order 1; analytic identification is not certified.".into()]),
        (vec![(0, 0, One, original), (1, 1, Three, &divergent)], vec![
            "Audit scope for [v:1/g-audit-0-scope]: finite source jet of order 5, residual order 4 and derivative order 1; analytic identification is not certified.".into(),
            "Audit scope for [v:3/g-audit-1-scope]: finite source jet of order 5, residual order 4 and derivative order 1; analytic identification is not certified.".into(),
        ]),
    ];
    for (summaries, expected) in cases {
        let actual = super::super::retained::scopes(book, &summaries, "fixture", "en").unwrap();
        assert_eq!(
            actual.iter().map(|runs| text(runs)).collect::<Vec<_>>(),
            expected
        );
    }
    let mut unsupported = original.clone();
    unsupported.clause = "Unknown";
    let actual =
        super::super::retained::scopes(book, &[(0, 0, One, &unsupported)], "fixture", "en")
            .err()
            .expect("unsupported contract");
    let expected = DocumentError::Retention(Box::new(
        crate::retention::RetentionCheckError::Audit(Box::new(
            ftoo_audit::retention::RetentionError::UnsupportedComponent {
                component: "reader.summary-contract",
            },
        )),
    ));
    assert_eq!(actual, expected);
    assert_eq!(
        actual.to_string(),
        "retention reproduction M061 at file \"generated/audit-report.json\": unsupported-component \"reader.summary-contract\""
    );
    assert_eq!(
        super::super::retained::scopes(book, &[(0, 0, One, original)], "fixture", "en")
            .unwrap()
            .len(),
        1
    );
}

pub(super) fn outcome_plan(
    book: &ftoo_manifest::AddressBook<'_>,
    verdict: &ftoo_audit::Verdict,
) -> DocumentPlan {
    let mut plan = DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: Coverage::build(
            book,
            "Evidence context: retained audit accepted.",
            "fixture",
            "en",
        )
        .unwrap(),
        blocks: vec![],
    };
    // The first selected row belongs to the second report outcome. Presentation
    // fixtures do not manufacture an accepted report or effective notation status.
    plan.coverage.audit = vec![
        super::super::retained::outcome_runs(book, &[(0, 1, Three, verdict)], "fixture", "en")
            .unwrap(),
    ];
    plan
}

#[test]
#[allow(clippy::too_many_lines)] // One table drives every verdict through both emission consumers.
fn retained_outcome_runs_have_exact_bytes_recovery_and_runtime_sites() {
    use ftoo_audit::{RunStage, Verdict};
    use ftoo_manifest::{CapabilityId, DeferredReason, VerdictClass};
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let production = produce(ftoo_manifest::projection_bytes(&graph).unwrap()).unwrap();
    let Verdict::Certified(receipt) = production.report().outcomes()[0].verdict() else {
        panic!("certified seed");
    };
    let undecided = |logical_id| {
        let mut exhaustion = nrz_cas::Fault::exhausted(
            nrz_cas::limits::BudgetKind::Work,
            7,
            8,
            nrz_cas::Location::new("source_jet"),
        );
        exhaustion.location.logical_id = logical_id;
        Verdict::Undecided {
            stage: RunStage::Setup,
            exhaustion,
        }
    };
    let cases = [
        (
            Verdict::Certified(receipt.clone()),
            ": certified.",
            ": certified.",
        ),
        (
            Verdict::Refuted(receipt.clone()),
            ": refuted.",
            ": refuted.",
        ),
        (
            undecided(None),
            ": undecided, setup exhausted work with limit 7 and attempted 8 at source&#95;jet.",
            ": undecided, setup exhausted work with limit 7 and attempted 8 at source\\_jet.",
        ),
        (
            undecided(Some(17)),
            ": undecided, setup exhausted work with limit 7 and attempted 8 at source&#95;jet &#40;logical id 17&#41;.",
            ": undecided, setup exhausted work with limit 7 and attempted 8 at source\\_jet (logical id 17).",
        ),
        (
            Verdict::Deferred(DeferredReason::AuditBinding),
            ": deferred, awaiting audit binding.",
            ": deferred, awaiting audit binding.",
        ),
        (
            Verdict::Deferred(DeferredReason::Capability(
                CapabilityId::SymbolicTrigonometry,
            )),
            ": deferred, awaiting capability symbolic&#45;trigonometry.",
            ": deferred, awaiting capability symbolic-trigonometry.",
        ),
        (
            Verdict::Deferred(DeferredReason::OutsideSlice),
            ": deferred, awaiting outside slice.",
            ": deferred, awaiting outside slice.",
        ),
        (
            Verdict::Deferred(DeferredReason::Premise {
                claim: One,
                evidence: VerdictClass::Undecided,
            }),
            ": deferred, awaiting premise v:1<!-- label claim:langevin:coefficient-one-value --> with evidence undecided.",
            ": deferred, awaiting premise \\FtdocTag{v:1}{g-audit-0-premise}% label claim:langevin:coefficient-one-value\n\\space{}with evidence undecided.",
        ),
    ];
    for (verdict, md, tex) in cases {
        let plan = outcome_plan(&book, &verdict);
        for (target, suffix) in [(DocumentTarget::Markdown, md), (DocumentTarget::Latex, tex)] {
            let mut fragment = String::new();
            let expected = if target == DocumentTarget::Markdown {
                markdown::inline(&mut fragment, &plan.coverage.audit[0]);
                format!(
                    "Audit outcomes: v:3<!-- label claim:langevin:coefficient-three-value -->{suffix}"
                )
            } else {
                latex::inline(&mut fragment, &plan.coverage.audit[0]);
                format!(
                    "Audit outcomes: \\FtdocTag{{v:3}}{{g-audit-0}}% label claim:langevin:coefficient-three-value\n{suffix}"
                )
            };
            assert_eq!(fragment, expected);
            let source = if target == DocumentTarget::Markdown {
                markdown::render(&plan)
            } else {
                latex::render(&plan)
            };
            assert_eq!(check_emission(&plan, &source, target), Ok(()));
            assert_eq!(
                source.len(),
                if target == DocumentTarget::Markdown {
                    markdown::size(&plan)
                } else {
                    latex::size(&plan)
                }
            );
        }
        let runtime = super::super::runtime::render(&book, std::slice::from_ref(&plan)).unwrap();
        let rows: Vec<_> = runtime
            .lines()
            .filter(|line| line.starts_with("\\prop_gput:Nxx \\g_ftdoc_sites_prop "))
            .collect();
        let mut expected = vec![AUDIT_SITE];
        if matches!(verdict, Verdict::Deferred(DeferredReason::Premise { .. })) {
            expected.push(PREMISE_SITE);
        }
        assert_eq!(rows, expected);
    }
}

const AUDIT_SITE: &str = r"\prop_gput:Nxx \g_ftdoc_sites_prop {\char_generate:nn {102}{12}\char_generate:nn {105}{12}\char_generate:nn {120}{12}\char_generate:nn {116}{12}\char_generate:nn {117}{12}\char_generate:nn {114}{12}\char_generate:nn {101}{12}\char_generate:nn {47}{12}\char_generate:nn {101}{12}\char_generate:nn {110}{12}\char_generate:nn {47}{12}\char_generate:nn {103}{12}\char_generate:nn {45}{12}\char_generate:nn {97}{12}\char_generate:nn {117}{12}\char_generate:nn {100}{12}\char_generate:nn {105}{12}\char_generate:nn {116}{12}\char_generate:nn {45}{12}\char_generate:nn {48}{12}} {}";

const PREMISE_SITE: &str = r"\prop_gput:Nxx \g_ftdoc_sites_prop {\char_generate:nn {102}{12}\char_generate:nn {105}{12}\char_generate:nn {120}{12}\char_generate:nn {116}{12}\char_generate:nn {117}{12}\char_generate:nn {114}{12}\char_generate:nn {101}{12}\char_generate:nn {47}{12}\char_generate:nn {101}{12}\char_generate:nn {110}{12}\char_generate:nn {47}{12}\char_generate:nn {103}{12}\char_generate:nn {45}{12}\char_generate:nn {97}{12}\char_generate:nn {117}{12}\char_generate:nn {100}{12}\char_generate:nn {105}{12}\char_generate:nn {116}{12}\char_generate:nn {45}{12}\char_generate:nn {48}{12}\char_generate:nn {45}{12}\char_generate:nn {112}{12}\char_generate:nn {114}{12}\char_generate:nn {101}{12}\char_generate:nn {109}{12}\char_generate:nn {105}{12}\char_generate:nn {115}{12}\char_generate:nn {101}{12}} {}";
