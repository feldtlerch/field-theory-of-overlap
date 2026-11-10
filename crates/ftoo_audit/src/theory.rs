//! The analytic identification L=coth(x)-1/x with xL'+2L+xL²-x=0
//! is scope, not an engine-checked analytic theorem. The finite equations are
//! triangular with leading coefficients 2,3,4,5 and uniquely fix c0..c3.
//! Degrees 0..3 leave c4 free; its zero is terminal candidate data only.
//! Each projection reconstructs the full identity composition in a closed Real
//! header, empty scope, ambient coefficient Product(x), and no assumptions.
use crate::engine::Term;
use nrz_cas::syntax::Operator;
pub(crate) fn ode() -> Term {
    use Term::{Constant, Derivative, Power, X, Y};
    fn b(op: Operator, a: Term, b: Term) -> Term {
        Term::Binary(op, Box::new(a), Box::new(b))
    }
    b(
        Operator::Sub,
        b(
            Operator::Add,
            b(
                Operator::Add,
                b(Operator::Mul, X, Derivative(Box::new(Y))),
                b(Operator::Mul, Constant(2), Y),
            ),
            b(Operator::Mul, X, Power(Box::new(Y), 2)),
        ),
        X,
    )
}

use crate::{
    JetSummary, Measurement, Receipt, RunCause, RunError, RunStage, engine::Frame, runner::fault,
};
use ftoo_manifest::{BudgetProfile, ProducerDescriptor, ProducerId};
use nrz_cas::limits::Limits;

pub(crate) fn limits(profile: BudgetProfile) -> Limits {
    Limits {
        work: profile.work(),
        replay_work: profile.replay_work(),
        ..Limits::default()
    }
}
pub(crate) fn coefficient_one() -> Result<(Receipt, Measurement), RunError> {
    checked(
        ProducerId::LangevinCoefficientOneValue.descriptor(),
        limits(BudgetProfile::PinnedTrialLimitsV1),
        false,
    )
}
pub(crate) fn coefficient_three() -> Result<(Receipt, Measurement), RunError> {
    checked(
        ProducerId::LangevinCoefficientThreeValue.descriptor(),
        limits(BudgetProfile::PinnedTrialLimitsV1),
        false,
    )
}
// Private parameters permit negative fixtures; public callers select neither
// expressions nor targets, regions, budgets, callbacks or supplied proof bytes.
pub(crate) fn checked(
    d: ProducerDescriptor,
    limits: Limits,
    empty: bool,
) -> Result<(Receipt, Measurement), RunError> {
    if d.target != d.producer.descriptor().target || d.region != d.producer.descriptor().region {
        return Err(RunError::Execution {
            claim: d.claim,
            stage: RunStage::Projection,
            cause: RunCause::ProjectionMismatch,
        });
    }
    let error = |stage| move |e| fault(d.claim, stage, e);
    let mut frame = Frame::new(limits.clone(), empty).map_err(error(RunStage::Setup))?;
    let jet = frame
        .rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])
        .map_err(error(RunStage::Candidate))?;
    let increment = frame
        .rationals(&[(0, 1), (1, 1), (0, 1), (0, 1), (0, 1)])
        .map_err(error(RunStage::Candidate))?;
    let compose = frame
        .compose(&jet, &increment)
        .map_err(error(RunStage::Compose))?;
    let (expression, rows) = frame
        .term(&ode(), &jet, 4)
        .map_err(error(RunStage::Candidate))?;
    let (residual, member) = frame
        .residual(&compose, &expression, &rows)
        .map_err(error(RunStage::ResidualMember))?;
    let member_work = frame.budget.usage().work;
    let evidence = [
        frame.export(&compose).map_err(error(RunStage::Export))?,
        frame.export(&residual).map_err(error(RunStage::Export))?,
        frame.export(&member).map_err(error(RunStage::Export))?,
    ];
    let expected = frame.expectations().map_err(error(RunStage::Expectation))?;
    let before = frame.budget.usage().work;
    frame
        .replay(&evidence, &expected)
        .map_err(error(RunStage::Replay))?;
    let total_work = frame.budget.usage().work;
    let summary = JetSummary {
        subject: "identity composition Compose(J,h)=J",
        jet,
        increment,
        input_orders: [5, 5],
        default_order: true,
        order: 5,
        derivative_order: 1,
        residual_order: 4,
        equation_degrees: [0, 1, 2, 3],
        ode: "x*y' + 2*y + x*y^2 - x",
        region: d.region,
        analytic_scope: "The regular germ coth(x)-1/x satisfies this ODE. The triangular finite equations have leading coefficients 2,3,4,5 and uniquely fix c0..c3; c4 is free terminal candidate data. No convergence or global equality is certified.",
    };
    Ok((
        Receipt::accepted(d, limits, summary, evidence),
        Measurement {
            claim: d.claim,
            member_work,
            total_work,
            replay_work: total_work - before,
        },
    ))
}
