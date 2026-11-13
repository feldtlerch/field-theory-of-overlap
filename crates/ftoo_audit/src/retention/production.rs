//! Checked-result conversion has no public authority-injection seam.
use super::wire;
use crate::{AuditReport, Receipt, Verdict};
fn receipt(receipt: &Receipt) -> wire::Receipt {
    let summary = receipt.summary();
    wire::Receipt {
        producer: receipt.producer(),
        revision: receipt.revision(),
        budget: receipt.budget(),
        pin: receipt.pin().0.into(),
        target: receipt.target() as u64,
        limits: limits(receipt.limits()),
        summary: wire::Summary {
            subject: summary.subject.into(),
            jet: receipt
                .rational_components
                .clone()
                .map(|(numerator, denominator)| wire::Rational {
                    numerator,
                    denominator,
                }),
            source: summary.source.into(),
            clause: summary.clause.into(),
            order: summary.order,
            derivative_order: summary.derivative_order,
            residual_order: summary.residual_order,
            equation_degrees: summary.equation_degrees,
            ode: summary.ode.into(),
            region: summary.region,
            analytic_scope: summary.analytic_scope.into(),
        },
        source: receipt.evidence()[0].clone(),
        residual: receipt.evidence()[1].clone(),
        member: receipt.evidence()[2].clone(),
    }
}
pub(super) fn report(
    report: &AuditReport,
    projection: String,
    digest: String,
    engine: String,
) -> Result<wire::Report, super::RetentionError> {
    let mut identities = ftoo_manifest::NodeId::ALL;
    identities.sort();
    Ok(wire::Report {
        projection,
        digest,
        contract: super::CONTRACT_REVISION,
        engine,
        reached: report.reached().to_vec(),
        outcomes: report
            .outcomes()
            .iter()
            .map(|row| {
                Ok(wire::Outcome {
                    claim: row.claim(),
                    verdict: match row.verdict() {
                        Verdict::Certified(value) => {
                            wire::Verdict::Certified(Box::new(receipt(value)))
                        }
                        Verdict::Refuted(value) => {
                            let contrary = value
                                .counterstatement()
                                .expect("checked refutation includes counterstatement");
                            wire::Verdict::Refuted {
                                receipt: Box::new(receipt(value)),
                                counterstatement: wire::Counterstatement {
                                    claimed: wire::Rational {
                                        numerator: contrary.components.0.clone(),
                                        denominator: contrary.components.1.clone(),
                                    },
                                    witness: contrary.witness().to_vec(),
                                },
                            }
                        }
                        Verdict::Undecided { stage, exhaustion } => wire::Verdict::Undecided {
                            stage: *stage,
                            exhaustion: exhaustion_record(exhaustion)?,
                        },
                        Verdict::Deferred(reason) => wire::Verdict::Deferred(*reason),
                    },
                })
            })
            .collect::<Result<_, super::RetentionError>>()?,
        statuses: identities
            .into_iter()
            .filter_map(|id| {
                report.status(id).map(|status| wire::Status {
                    id,
                    maturity: status.base().maturity(),
                    identification: status.base().identification(),
                    calibration: status.base().calibration(),
                    evidence: status.evidence(),
                })
            })
            .collect(),
        assessment: wire::Assessment {
            disposition: report.assessment().disposition(),
            exit_code: u32::from(report.assessment().exit_code()),
        },
    })
}

pub(super) fn exhaustion_record(
    fault: &nrz_cas::Fault,
) -> Result<wire::Exhaustion, super::RetentionError> {
    let nrz_cas::Fault {
        kind,
        location,
        series_evidence,
    } = fault;
    let nrz_cas::Location {
        operation,
        logical_id,
    } = location;
    let (reason, budget, limit, needed) = match kind {
        nrz_cas::FaultKind::Exhausted {
            reason,
            budget,
            limit,
            needed,
        } => (reason, budget, limit, needed),
        nrz_cas::FaultKind::Invalid(_) => {
            return Err(super::RetentionError::UnsupportedComponent {
                component: "exhaustion.kind",
            });
        }
    };
    match series_evidence.as_deref() {
        None => (),
        Some(
            nrz_cas::SeriesRefusalEvidence::Constant(nrz_cas::SeriesConstantEvidence {
                input: _,
                coefficient: _,
                index: _,
                value: _,
            })
            | nrz_cas::SeriesRefusalEvidence::Precision([_, _, _]),
        ) => {
            return Err(super::RetentionError::UnsupportedComponent {
                component: "exhaustion.series-evidence",
            });
        }
    }
    Ok(wire::Exhaustion {
        resource: *budget,
        reason: *reason,
        limit: *limit,
        attempted: *needed,
        location: wire::Location {
            operation: (*operation).into(),
            logical_id: *logical_id,
        },
    })
}

pub(super) use super::wire::limits;
