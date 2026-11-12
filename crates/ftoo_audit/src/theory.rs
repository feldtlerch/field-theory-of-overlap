//! The analytic identification L=coth(x)-1/x with xL'+2L+xL²-x=0
//! is scope, not an engine-checked analytic theorem. The finite equations are
//! triangular with leading coefficients 2,3,4,5 and uniquely fix c0..c3.
//! Degrees 0..3 leave c4 free; its zero is terminal candidate data only.
//! Each projection reconstructs the full polynomial source in a closed Real
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
        limits(BudgetProfile::PinnedTrialLimitsV2),
        false,
    )
}
pub(crate) fn coefficient_three() -> Result<(Receipt, Measurement), RunError> {
    checked(
        ProducerId::LangevinCoefficientThreeValue.descriptor(),
        limits(BudgetProfile::PinnedTrialLimitsV2),
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
    checked_claim(d, limits, empty, None)
}
pub(crate) fn checked_claim(
    d: ProducerDescriptor,
    limits: Limits,
    empty: bool,
    claimed: Option<(i64, u64)>,
) -> Result<(Receipt, Measurement), RunError> {
    checked_claim_with_prior(d, limits, empty, claimed, 0)
}
pub(crate) fn checked_claim_with_prior(
    d: ProducerDescriptor,
    limits: Limits,
    empty: bool,
    claimed: Option<(i64, u64)>,
    prior_work: u64,
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
    if prior_work != 0 {
        frame
            .budget
            .work(prior_work, nrz_cas::Location::new("audit prior spend"))
            .map_err(error(RunStage::Setup))?;
    }
    let jet = frame
        .rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])
        .map_err(error(RunStage::Candidate))?;
    let source = frame
        .polynomial_seed(&jet)
        .map_err(error(RunStage::Candidate))?;
    let defined = frame
        .defined(&source)
        .map_err(error(RunStage::Definedness))?;
    let source = frame
        .source(&source, &defined)
        .map_err(error(RunStage::Source))?;
    let (expression, rows) = frame
        .term(&ode(), &jet, 4)
        .map_err(error(RunStage::Candidate))?;
    let (residual, member) = frame
        .residual(&source, &expression, &rows)
        .map_err(error(RunStage::ResidualMember))?;
    let member_work = frame.budget.usage().work;
    let evidence = [
        frame.export(&source).map_err(error(RunStage::Export))?,
        frame.export(&residual).map_err(error(RunStage::Export))?,
        frame.export(&member).map_err(error(RunStage::Export))?,
    ];
    let expected = frame.expectations().map_err(error(RunStage::Expectation))?;
    let before = frame.budget.usage().work;
    frame
        .replay(&evidence, &expected)
        .map_err(error(RunStage::Replay))?;
    let mut cumulative_replay_work = frame.budget.usage().work - before;
    let mut max_replay_work = cumulative_replay_work;
    let contrary = if let Some((numerator, denominator)) = claimed {
        let claimed = nrz_cas::Rational::from_ratio(numerator, denominator, &mut frame.budget)
            .map_err(error(RunStage::Projection))?;
        if claimed == jet[d.target] {
            None
        } else {
            let (witness, contrary_replay) = frame
                .counterstatement(&jet[d.target], &claimed)
                .map_err(error(RunStage::Projection))?;
            cumulative_replay_work += contrary_replay;
            max_replay_work = max_replay_work.max(contrary_replay);
            Some(crate::Counterstatement { claimed, witness })
        }
    } else {
        None
    };
    let total_work = frame.budget.usage().work;
    let summary = JetSummary {
        subject: "polynomial seed x/3 - x^3/45",
        jet,
        source: "(1/3)*x - (1/45)*IntegerPower(x,3)",
        clause: "PolynomialSeed",
        order: 5,
        derivative_order: 1,
        residual_order: 4,
        equation_degrees: [0, 1, 2, 3],
        ode: "x*y' + 2*y + x*y^2 - x",
        region: d.region,
        analytic_scope: "The regular germ coth(x)-1/x satisfies this ODE. The triangular finite equations have leading coefficients 2,3,4,5 and uniquely fix c0..c3; c4 is free terminal candidate data. The polynomial residual has nonzero tail -2*x^5/135 + x^7/2025. No exact polynomial solution, convergence, analytic source equality or global equality is certified.",
    };
    let mut receipt = Receipt::accepted(d, limits, summary, evidence);
    receipt.contrary = contrary;
    Ok((
        receipt,
        Measurement {
            claim: d.claim,
            execution: frame.execution,
            member_work,
            total_work,
            cumulative_replay_work,
            max_replay_work,
        },
    ))
}
