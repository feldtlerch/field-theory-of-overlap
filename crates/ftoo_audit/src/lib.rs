//! Exact residual-system trial for field-theory-of-overlap against nrz.
//!
//! With dimension three, coefficient matching gives
//! `(m + 2) c_m = [m = 1] - sum(c_i c_j, i + j = m - 1)`.
//! Lower coefficients are specialized before each submission. Candidate values
//! come from exact rational division; only the returned engine verdict certifies
//! the supplied elimination. A refusal remains evidence and is never bypassed.
//!
//! The public route is `Cas::check_proof` with `ApplicationDraft`,
//! `Primitive::Solve(ExactLinearEquivalence)`, `Statement::Solve(LinearSystem)`
//! and `Certificate::Solve(ExactLinearEquivalence)`. Each scale cites an earlier
//! coefficient-identity proof of its inverse. The coefficient algebra is Q;
//! unknowns have Real ambient, with no assumptions or denominator exclusions.
//! Every submission uses a fresh store and the pinned engine's default limits.

mod engine;

use nrz_cas::{
    Fault, Rational,
    limits::{Budget, Limits},
};

/// Exact submitted data and the checker result, independent of store-local IDs.
#[derive(Clone, Debug)]
pub struct Submission {
    /// Original unknown names in column order.
    pub unknowns: Vec<String>,
    /// Original augmented matrix, retaining every residual equation.
    pub matrix: Vec<Vec<Rational>>,
    /// Claimed reduced augmented matrix.
    pub reduced: Vec<Vec<Rational>>,
    /// Scale factors in row order; each operation cites the same-numbered premise.
    pub scales: Vec<Rational>,
    /// Inverse-identity factors `(scale, inverse)` in premise order.
    pub units: Vec<(Rational, Rational)>,
    /// Accepted portable witness bytes, or the actual typed proof-checker fault.
    pub verdict: Result<Vec<u8>, Fault>,
}

/// Sequential candidates and all nine submissions, including the negative control.
#[derive(Clone, Debug)]
pub struct Trial {
    /// Exact sequential candidates, from c1 through c7.
    pub coefficients: Vec<Rational>,
    /// Seven scalar systems, the joint system, then the tampered scalar certificate.
    pub submissions: Vec<Submission>,
}

/// Run the fixed residual trial without replacing any checker refusal.
///
/// # Errors
/// Returns setup, arithmetic or witness-export faults. Proof-checking refusals
/// are retained in each submission instead of terminating the trial.
pub fn solving_trial() -> Result<Trial, Fault> {
    let mut arithmetic = Budget::new(Limits::default())?;
    let mut coefficients = vec![Rational::zero()];
    let mut submissions = Vec::new();
    for order in 1_u32..=7 {
        let mut convolution = Rational::zero();
        for (i, left) in coefficients.iter().enumerate().skip(1) {
            if let Some(j) = usize::try_from(order - 1)
                .ok()
                .and_then(|m| m.checked_sub(i))
                .filter(|j| *j >= 1)
            {
                let product = left.mul(&coefficients[j], &mut arithmetic)?;
                convolution = convolution.add(&product, &mut arithmetic)?;
            }
        }
        let forcing = if order == 1 {
            Rational::one()
        } else {
            Rational::zero()
        };
        let rhs = forcing.sub(&convolution, &mut arithmetic)?;
        let diagonal = Rational::from_ratio(i64::from(order + 2), 1, &mut arithmetic)?;
        coefficients.push(rhs.div(&diagonal, &mut arithmetic)?);
        submissions.push(engine::submit(&[order], &[diagonal], &[rhs], false)?);
    }
    submissions.push(engine::submit(
        &[1, 2],
        &[
            Rational::from_ratio(3, 1, &mut arithmetic)?,
            Rational::from_ratio(4, 1, &mut arithmetic)?,
        ],
        &[Rational::one(), Rational::zero()],
        false,
    )?);
    submissions.push(engine::submit(
        &[1],
        &[Rational::from_ratio(3, 1, &mut arithmetic)?],
        &[Rational::one()],
        true,
    )?);
    Ok(Trial {
        coefficients: coefficients.into_iter().skip(1).collect(),
        submissions,
    })
}

#[cfg(test)]
mod tests;
