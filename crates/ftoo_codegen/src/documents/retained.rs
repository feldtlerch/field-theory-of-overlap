use super::{
    DocumentError,
    plan::{self, Block, Inline},
};
use ftoo_audit::{
    Verdict,
    retention::{RetainedNotation, RetentionError, exhaustion_record, stage_record},
};
use ftoo_manifest::VerdictClass::{Certified, Deferred, Refuted, Undecided};
use ftoo_manifest::{AddressBook, DocumentTarget, NodeId, ReasonPart, SourceOrigin};

fn tag(
    book: &AddressBook<'_>,
    object: NodeId,
    indices: (usize, usize),
    suffix: &str,
    document: &str,
    language: &str,
) -> Result<Inline, DocumentError> {
    let (index, report_index) = indices;
    let mut site = plan::coverage_site(Some(report_index));
    site.file = "generated/audit-report.json".into();
    site.path[0] = ftoo_manifest::PathPart::Key("outcomes".into());
    site.path.remove(1);
    let handle = book.reader_identity(object).map_err(|fault| {
        plan::reader_failure(
            fault,
            document,
            language,
            None,
            None,
            ftoo_manifest::ReaderIdentityPosition::Coverage {
                index: report_index,
                object,
            },
            vec![SourceOrigin {
                unit: None,
                language: Some(language.into()),
                target: None,
                site: site.clone(),
            }],
        )
    })?;
    Ok(Inline::Tag {
        handle,
        label: object.semantic_name().into(),
        token: format!("g-audit-{index}{suffix}"),
        site,
    })
}

#[allow(clippy::too_many_lines)] // Outcome, scope and axis prose share one accepted inventory.
pub(super) fn coverage(
    book: &AddressBook<'_>,
    view: &RetainedNotation<'_, '_, '_>,
    blocks: &[Block],
    document: &str,
    language: &str,
) -> Result<Vec<Vec<Inline>>, DocumentError> {
    let report = view.accepted().report();
    let mut selected: Vec<_> = report
        .outcomes()
        .iter()
        .enumerate()
        .filter(|(_, outcome)| book.graph().coverage().slice.contains(&outcome.claim()))
        .collect();
    selected.sort_by_key(|(_, outcome)| outcome.claim().semantic_name());
    let rows: Vec<_> = selected
        .iter()
        .enumerate()
        .map(|(index, (report_index, outcome))| {
            (index, *report_index, outcome.claim(), outcome.verdict())
        })
        .collect();
    let certified: Vec<_> = rows
        .iter()
        .filter_map(|(index, report_index, claim, verdict)| {
            if let Verdict::Certified(receipt) = verdict {
                Some((*index, *report_index, *claim, receipt.summary()))
            } else {
                None
            }
        })
        .collect();
    let mut paragraphs = vec![outcome_runs(book, &rows, document, language)?];
    paragraphs.extend(scopes(book, &certified, document, language)?);
    let pending: Vec<_> = blocks
        .iter()
        .flat_map(|block| {
            [DocumentTarget::Markdown, DocumentTarget::Latex]
                .into_iter()
                .flat_map(move |target| block.target_runs(target))
        })
        .filter_map(|run| match run {
            Inline::Math { pending, .. } => Some(pending),
            _ => None,
        })
        .flatten()
        .collect();
    if !certified.is_empty() && !pending.is_empty() {
        let proposed = certified.iter().any(|(_, _, claim, _)| {
            report
                .status(*claim)
                .is_some_and(|status| status.base().maturity() == ftoo_manifest::Maturity::Proposed)
        }) || certified.iter().any(|(_, _, claim, _)| {
            view.base_catalogue()
                .object_symbol(*claim)
                .ok()
                .and_then(|definition| view.effective_status(definition.spec().id()))
                .is_some_and(|status| {
                    status.reasons().iter().any(|reason| {
                        matches!(
                            reason,
                            ftoo_manifest::PendingReason::FormulationProposed { .. }
                        )
                    })
                })
        });
        paragraphs.push(vec![Inline::Text(if proposed { "Certification concerns the finite coefficient claim; pending notation concerns a proposed formulation." } else { "Certification concerns the finite coefficient claim; pending notation concerns separate notation eligibility conditions." }.into())]);
    }
    let outside: Vec<_> = report
        .outcomes()
        .iter()
        .filter(|outcome| !book.graph().coverage().slice.contains(&outcome.claim()))
        .collect();
    if !outside.is_empty() {
        let count = |class| {
            outside
                .iter()
                .filter(|outcome| outcome.verdict().class() == class)
                .count()
        };

        paragraphs.push(vec![Inline::Text(format!("Outside declared slice: {} claims; certified {}; refuted {}; undecided {}; deferred {}. See generated/audit-report.json for identities and blockers.", outside.len(), count(Certified), count(Refuted), count(Undecided), count(Deferred)))]);
    }
    Ok(paragraphs)
}
pub(super) fn scopes(
    book: &AddressBook<'_>,
    certified: &[(usize, usize, NodeId, &ftoo_audit::JetSummary)],
    document: &str,
    language: &str,
) -> Result<Vec<Vec<Inline>>, DocumentError> {
    let mut paragraphs = vec![];
    for (_, _, _, summary) in certified {
        // The retained producer contract supplies finite polynomial-source evidence only.
        if summary.clause != "PolynomialSeed"
            || summary.subject != "polynomial seed x/3 - x^3/45"
            || summary.source != "(1/3)*x - (1/45)*IntegerPower(x,3)"
            || summary.equation_degrees != [0, 1, 2, 3]
            || summary.jet.len() != 5
            || summary.analytic_scope
                != "The regular germ coth(x)-1/x satisfies this ODE. The triangular finite equations have leading coefficients 2,3,4,5 and uniquely fix c0..c3; c4 is free terminal candidate data. The polynomial residual has nonzero tail -2*x^5/135 + x^7/2025. No exact polynomial solution, convergence, analytic source equality or global equality is certified."
            || summary.ode != "x*y' + 2*y + x*y^2 - x"
            || summary.region != ftoo_manifest::RegionContract::AmbientProductXNoAssumptions
        {
            return Err(audit_error(RetentionError::UnsupportedComponent {
                component: "reader.summary-contract",
            }));
        }
    }
    if let Some((_, _, _, first)) = certified.first() {
        if certified.iter().all(|(_, _, _, summary)| summary == first) {
            paragraphs.push(vec![Inline::Text(format!("Audit scope: {}", scope(first)))]);
        } else {
            for (index, report_index, claim, summary) in certified {
                paragraphs.push(vec![
                    Inline::Text("Audit scope for ".into()),
                    tag(
                        book,
                        *claim,
                        (*index, *report_index),
                        "-scope",
                        document,
                        language,
                    )?,
                    Inline::Text(format!(": {}", scope(summary))),
                ]);
            }
        }
    } else {
        paragraphs.push(vec![Inline::Text("Audit scope: no selected claim is certified; analytic identification is not certified.".into())]);
    }
    Ok(paragraphs)
}
fn scope(summary: &ftoo_audit::JetSummary) -> String {
    format!(
        "finite source jet of order {}, residual order {} and derivative order {}; analytic identification is not certified.",
        summary.order, summary.residual_order, summary.derivative_order
    )
}
fn audit_error(error: RetentionError) -> DocumentError {
    DocumentError::Retention(Box::new(error.into()))
}

// These rows describe presentation only and cannot construct an accepted context.
pub(super) fn outcome_runs(
    book: &AddressBook<'_>,
    rows: &[(usize, usize, NodeId, &Verdict)],
    document: &str,
    language: &str,
) -> Result<Vec<Inline>, DocumentError> {
    let mut runs = vec![Inline::Text("Audit outcomes: ".into())];
    for (position, (index, report_index, claim, verdict)) in rows.iter().enumerate() {
        if position != 0 {
            runs.push(Inline::Text("; ".into()));
        }
        runs.push(tag(
            book,
            *claim,
            (*index, *report_index),
            "",
            document,
            language,
        )?);
        let parts = match verdict {
            Verdict::Certified(_) => vec![ReasonPart::Text(": certified".into())],
            Verdict::Refuted(_) => vec![ReasonPart::Text(": refuted".into())],
            Verdict::Undecided { stage, exhaustion } => vec![ReasonPart::Text(format!(
                ": undecided, {}",
                exhaustion_record(exhaustion)
                    .map_err(audit_error)?
                    .description(stage_record(*stage))
            ))],
            Verdict::Deferred(reason) => {
                let mut parts = vec![ReasonPart::Text(": deferred, awaiting ".into())];
                parts.extend(ftoo_manifest::deferred_parts(*reason));
                parts
            }
        };
        for part in parts {
            match part {
                ReasonPart::Text(text) => runs.push(Inline::Text(text)),
                ReasonPart::Object { id, .. } => runs.push(tag(
                    book,
                    id,
                    (*index, *report_index),
                    "-premise",
                    document,
                    language,
                )?),
            }
        }
    }
    if rows.is_empty() {
        runs.push(Inline::Text("(empty)".into()));
    }
    runs.push(Inline::Text(".".into()));
    Ok(runs)
}
