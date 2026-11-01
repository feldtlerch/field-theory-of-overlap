use std::fmt::Write;

use nrz_cas::{
    Fault, FaultKind, InvalidCode, Rational,
    limits::{Budget, Limits},
};

use crate::{Submission, solving_trial};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

// L(k) = (cosh(k) - sinh(k)/k) / sinh(k). Divide after removing
// the common k: numerator coefficients at k^(2j-1) are 2j/(2j+1)!,
// denominator coefficients at k^(2j) are 1/(2j+1)!. This calculation
// uses neither the differential residual nor the sequential candidates.
fn langevin_oracle(budget: &mut Budget) -> Result<Vec<Rational>, Fault> {
    let mut numerator = vec![Rational::zero(); 8];
    let mut denominator = vec![Rational::zero(); 8];
    let mut factorial = 1_u64;
    for degree in 1_u32..=9 {
        factorial *= u64::from(degree);
        if degree % 2 == 1 {
            let power = usize::try_from(degree - 1).unwrap_or(8);
            if power < denominator.len() {
                denominator[power] = Rational::from_ratio(1, factorial, budget)?;
            }
            if power > 0 {
                numerator[power - 1] =
                    Rational::from_ratio(i64::from(degree - 1), factorial, budget)?;
            }
        }
    }
    let mut quotient = Vec::new();
    for (degree, coefficient) in numerator.iter().enumerate() {
        let mut remainder = coefficient.clone();
        for (previous, value) in quotient.iter().enumerate() {
            remainder =
                remainder.sub(&denominator[degree - previous].mul(value, budget)?, budget)?;
        }
        quotient.push(remainder.div(&denominator[0], budget)?);
    }
    Ok(quotient.into_iter().skip(1).collect())
}

fn row_text(row: &[Rational], budget: &mut Budget) -> Result<String, Fault> {
    Ok(format!(
        "[{}]",
        row.iter()
            .map(|value| value.exact_string(budget))
            .collect::<Result<Vec<_>, _>>()?
            .join(",")
    ))
}

fn matrix_text(matrix: &[Vec<Rational>], budget: &mut Budget) -> Result<String, Fault> {
    Ok(format!(
        "[{}]",
        matrix
            .iter()
            .map(|row| row_text(row, budget))
            .collect::<Result<Vec<_>, _>>()?
            .join(",")
    ))
}

fn transcript(index: usize, submission: &Submission, budget: &mut Budget) -> TestResult<String> {
    let mut text = String::new();
    let size = submission.unknowns.len();
    let label = if index == 8 {
        "tampered"
    } else if index == 7 {
        "joint"
    } else {
        "sequential"
    };
    writeln!(
        text,
        "{label} {}: Q, Real unknowns [{}], no assumptions/exclusions",
        index + 1,
        submission.unknowns.join(",")
    )?;
    writeln!(
        text,
        "statement: LinearSystem A|b={}, reduced={}, rank={size}, augmented_rank={size}, free_columns=[]",
        matrix_text(&submission.matrix, budget)?,
        matrix_text(&submission.reduced, budget)?
    )?;
    for (slot, (factor, inverse)) in submission.units.iter().enumerate() {
        writeln!(
            text,
            "premise {slot}: CoefficientIdentity Normalize {}*{}=1, map Q->Real, region=ambient(empty coefficient product)",
            factor.exact_string(budget)?,
            inverse.exact_string(budget)?
        )?;
    }
    let mut operations = Vec::new();
    for (row, value) in submission.scales.iter().enumerate() {
        operations.push(format!(
            "Scale(row={row},value={},unit={row})",
            value.exact_string(budget)?
        ));
    }
    writeln!(
        text,
        "certificate: ExactLinearEquivalence matrix=A|b, augmented_columns=1, operations=[{}], pivots={:?}, minor=1, reduced=statement.reduced, region=ambient(empty coefficient product)",
        operations.join(","),
        (0..size).collect::<Vec<_>>()
    )?;
    match &submission.verdict {
        Ok(_) => writeln!(text, "verdict: accepted (portable witness retained)")?,
        Err(fault) => writeln!(text, "verdict: refused {fault}")?,
    }
    Ok(text)
}

#[test]
fn solving_trial_evidence() -> TestResult {
    let trial = solving_trial()?;
    let mut budget = Budget::new(Limits::default())?;
    println!("consumer route: Cas::check_proof(ApplicationDraft with Solve statement/certificate)");
    for (index, submission) in trial.submissions.iter().enumerate() {
        print!("{}", transcript(index, submission, &mut budget)?);
    }
    let oracle = langevin_oracle(&mut budget)?;
    let expected = [
        (1, 3),
        (0, 1),
        (-1, 45),
        (0, 1),
        (2, 945),
        (0, 1),
        (-1, 4725),
    ]
    .into_iter()
    .map(|(n, d)| Rational::from_ratio(n, d, &mut budget))
    .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(oracle, expected);
    assert_eq!(trial.coefficients, oracle);
    assert_eq!(trial.submissions.len(), 9);
    // Every valid system must retain accepted evidence, including the diagonal
    // joint system; an unrelated refusal cannot masquerade as tamper detection.
    assert!(
        trial.submissions[..8]
            .iter()
            .all(|submission| submission.verdict.is_ok())
    );
    assert!(
        matches!(&trial.submissions[8].verdict, Err(fault) if fault.kind == FaultKind::Invalid(InvalidCode::Rule))
    );
    for submission in &trial.submissions[..8] {
        if let Ok(witness) = &submission.verdict {
            assert_ne!(witness.as_slice(), []);
        }
    }
    let joint_witness = trial.submissions[7]
        .verdict
        .as_ref()
        .map_err(Clone::clone)?;
    let replay = crate::engine::replay_joint(joint_witness);
    match &replay {
        Ok(()) => println!(
            "joint replay: accepted in a fresh store against an independently built Judgment"
        ),
        Err(fault) => println!("joint replay: refused {fault}"),
    }
    assert!(replay.is_ok(), "joint replay: {replay:?}");
    println!(
        "oracle: exact cosh/sinh series division agrees with {}",
        row_text(&oracle, &mut budget)?
    );
    Ok(())
}
