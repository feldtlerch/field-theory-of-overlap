//! Retained linear trial and independent series-division controls; never a production fallback.
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

mod engine {
    //! Public graph admission and supplied proof construction for the fixed trial.

    use nrz_cas::{
        Cas, Fault, Handle, InvalidCode, Location, Rational, Reference,
        domain::{AmbientDomain, ContextDraft, HeaderDraft, RegionDraft, RegionExpr},
        limits::{Budget, Limits},
        proof::{ApplicationDraft, Certificate, Primitive, Statement},
        solving::{
            CertificateCoefficientIdentity, CertificateExactLinearEquivalence,
            CoefficientDomainData, CoefficientMapData, Coordinate, CoordinateRole,
            CoordinateSpaceData, IdentityMode, MapMode, MapTarget, RowOperation, SolveCertificate,
            SolveDraft, SolveProblemData, SolveSchema, SolveStatement,
            StatementCoefficientEquality, StatementLinearSystem,
        },
        syntax::{
            AtomicSort, Branch, Declaration, Draft, Environment, ExpressionDraft, Extent, Field,
            Operator, PredicateKind, Sort,
        },
    };

    use super::Submission;

    fn budget() -> Result<Budget, Fault> {
        Budget::new(Limits::default())
    }

    fn admit(cas: &mut Cas, draft: Draft) -> Result<Handle, Fault> {
        cas.admit(&[draft], &mut budget()?)?
            .into_iter()
            .next()
            .ok_or_else(|| {
                Fault::invalid(
                    InvalidCode::Reference,
                    Location::new("trial admission result"),
                )
            })
    }

    fn coordinates(
        cas: &mut Cas,
        header: &Handle,
        keys: &[String],
        role: CoordinateRole,
    ) -> Result<Handle, Fault> {
        admit(
            cas,
            Draft::Solve(SolveDraft::CoordinateSpace(CoordinateSpaceData {
                header: Reference::Existing(header.clone()),
                scalar_spaces: Vec::new(),
                coordinates: keys
                    .iter()
                    .map(|key| Coordinate {
                        key: key.clone(),
                        field: Field::Real,
                        role,
                    })
                    .collect(),
            })),
        )
    }

    fn product(cas: &mut Cas, header: &Handle, coordinates: Handle) -> Result<Handle, Fault> {
        admit(
            cas,
            Draft::Region(RegionDraft {
                header: Reference::Existing(header.clone()),
                scope: Vec::new(),
                ambient: AmbientDomain::Product { coordinates },
                expression: RegionExpr::Ambient,
            }),
        )
    }

    struct Store {
        cas: Cas,
        header: Handle,
        coefficients: Handle,
        region: Handle,
        context: Handle,
        algebra: Handle,
        map: Handle,
    }

    impl Store {
        fn new(keys: &[String]) -> Result<Self, Fault> {
            let mut cas = Cas::new(Limits::default())?;
            let header = admit(
                &mut cas,
                Draft::Header(HeaderDraft::Core(Environment {
                    field: Field::Real,
                    branch: Branch::StrictReal,
                    declarations: keys
                        .iter()
                        .map(|key| Declaration {
                            key: key.clone(),
                            display_name: key.clone(),
                            sort: Sort::Value(AtomicSort::Scalar(Field::Real)),
                        })
                        .collect(),
                })),
            )?;
            let coefficients = coordinates(&mut cas, &header, &[], CoordinateRole::Coefficient)?;
            let region = product(&mut cas, &header, coefficients.clone())?;
            let context = admit(
                &mut cas,
                Draft::Context(ContextDraft::Root {
                    header: Reference::Existing(header.clone()),
                    admissible: Reference::Existing(region.clone()),
                    assumptions: Vec::new(),
                }),
            )?;
            let algebra = admit(
                &mut cas,
                Draft::Solve(SolveDraft::CoefficientDomain(
                    CoefficientDomainData::Rational,
                )),
            )?;
            let map = admit(
                &mut cas,
                Draft::Solve(SolveDraft::CoefficientMap(CoefficientMapData {
                    source: Reference::Existing(algebra.clone()),
                    target: MapTarget::ScalarAmbient {
                        header: Reference::Existing(header.clone()),
                        field: Field::Real,
                    },
                    mode: MapMode::Embedding,
                    region: Reference::Existing(region.clone()),
                    images: Vec::new(),
                    denominators: Vec::new(),
                })),
            )?;
            Ok(Self {
                cas,
                header,
                coefficients,
                region,
                context,
                algebra,
                map,
            })
        }

        fn expression(&mut self, operator: Operator, children: &[Handle]) -> Result<Handle, Fault> {
            let sort = match &operator {
                Operator::Predicate(_) => AtomicSort::Boolean,
                Operator::Matrix { rows, columns } => AtomicSort::Matrix {
                    field: Field::Real,
                    rows: Extent::Literal(*rows),
                    columns: Extent::Literal(*columns),
                },
                _ => AtomicSort::Scalar(Field::Real),
            };
            admit(
                &mut self.cas,
                Draft::Expression(ExpressionDraft {
                    header: Reference::Existing(self.header.clone()),
                    scope: Vec::new(),
                    operator,
                    sort: Sort::Value(sort),
                    operands: children.iter().cloned().map(Reference::Existing).collect(),
                    space: None,
                    binding: None,
                }),
            )
        }

        fn condition(&mut self, kind: PredicateKind, children: &[Handle]) -> Result<Handle, Fault> {
            admit(
                &mut self.cas,
                Draft::Condition {
                    header: Reference::Existing(self.header.clone()),
                    scope: Vec::new(),
                    kind,
                    arguments: children.iter().cloned().map(Reference::Existing).collect(),
                },
            )
        }

        fn matrix(&mut self, rows: u32, values: &[Vec<Rational>]) -> Result<Handle, Fault> {
            let entries = values
                .iter()
                .flatten()
                .map(|value| self.expression(Operator::Rational(value.clone()), &[]))
                .collect::<Result<Vec<_>, _>>()?;
            self.expression(
                Operator::Matrix {
                    rows,
                    columns: rows + 1,
                },
                &entries,
            )
        }

        fn problem(
            &mut self,
            keys: &[String],
            diagonal: &[Rational],
            rhs: &[Rational],
        ) -> Result<Handle, Fault> {
            let mut equations = Vec::new();
            for ((key, coefficient), value) in keys.iter().zip(diagonal).zip(rhs) {
                let unknown = self.expression(Operator::Free(key.clone()), &[])?;
                let coefficient = self.expression(Operator::Rational(coefficient.clone()), &[])?;
                let left = self.expression(Operator::Mul, &[coefficient, unknown])?;
                let right = self.expression(Operator::Rational(value.clone()), &[])?;
                // The problem root must be a Condition, but conjunction operands
                // must be Boolean expressions rather than nested Condition handles.
                let equation = if keys.len() == 1 {
                    self.condition(PredicateKind::Equal, &[left, right])?
                } else {
                    self.expression(Operator::Predicate(PredicateKind::Equal), &[left, right])?
                };
                equations.push(equation);
            }
            let predicate = if equations.len() == 1 {
                equations.remove(0)
            } else {
                self.condition(PredicateKind::And, &equations)?
            };
            let unknowns = coordinates(&mut self.cas, &self.header, keys, CoordinateRole::Unknown)?;
            let domain = product(&mut self.cas, &self.header, unknowns.clone())?;
            admit(
                &mut self.cas,
                Draft::Solve(SolveDraft::SolveProblem(SolveProblemData {
                    header: Reference::Existing(self.header.clone()),
                    scope: Vec::new(),
                    predicate: Reference::Existing(predicate),
                    coefficients: Reference::Existing(self.coefficients.clone()),
                    unknowns: Reference::Existing(unknowns),
                    region: Reference::Existing(self.region.clone()),
                    domain: Reference::Existing(domain),
                    coefficient_domain: Reference::Existing(self.algebra.clone()),
                    ambient: Field::Real,
                    denominators: Vec::new(),
                })),
            )
        }

        fn step(
            &self,
            schema: SolveSchema,
            conclusion: SolveStatement,
            certificate: SolveCertificate,
        ) -> ApplicationDraft {
            ApplicationDraft {
                rule: Primitive::Solve(schema),
                context: self.context.clone(),
                conclusion: Statement::Solve(Box::new(conclusion)),
                premises: Vec::new(),
                substitutions: Vec::new(),
                certificate: Certificate::Solve(Box::new(certificate)),
            }
        }

        fn unit(
            &mut self,
            factor: &Rational,
            inverse: &Rational,
            one: &Handle,
        ) -> Result<ApplicationDraft, Fault> {
            let factor = self.expression(Operator::Rational(factor.clone()), &[])?;
            let inverse = self.expression(Operator::Rational(inverse.clone()), &[])?;
            let left = self.expression(Operator::Mul, &[factor, inverse])?;
            Ok(self.step(
                SolveSchema::CoefficientIdentity,
                SolveStatement::CoefficientEquality(StatementCoefficientEquality {
                    left: left.clone(),
                    right: one.clone(),
                    map: self.map.clone(),
                    region: self.region.clone(),
                }),
                SolveCertificate::CoefficientIdentity(CertificateCoefficientIdentity {
                    mode: IdentityMode::Normalize,
                    left,
                    right: one.clone(),
                    map: self.map.clone(),
                    region: self.region.clone(),
                    combinations: Vec::new(),
                    denominators: Vec::new(),
                }),
            ))
        }
    }

    pub(super) fn submit(
        orders: &[u32],
        diagonal: &[Rational],
        rhs: &[Rational],
        tamper: bool,
    ) -> Result<Submission, Fault> {
        let mut arithmetic = budget()?;
        let unknowns: Vec<_> = orders.iter().map(|order| format!("c{order}")).collect();
        let mut store = Store::new(&unknowns)?;
        let mut matrix = Vec::new();
        let mut reduced = Vec::new();
        let mut scales = Vec::new();
        let mut units = Vec::new();
        for (row, (coefficient, value)) in diagonal.iter().zip(rhs).enumerate() {
            let mut original_row = vec![Rational::zero(); orders.len()];
            original_row[row] = coefficient.clone();
            original_row.push(value.clone());
            matrix.push(original_row);
            let mut reduced_row = vec![Rational::zero(); orders.len()];
            reduced_row[row] = Rational::one();
            reduced_row.push(value.div(coefficient, &mut arithmetic)?);
            reduced.push(reduced_row);
            let scale = Rational::one().div(coefficient, &mut arithmetic)?;
            units.push((scale.clone(), coefficient.clone()));
            scales.push(scale);
        }
        if tamper {
            scales[0] = Rational::from_ratio(1, 2, &mut arithmetic)?;
        }
        let size = u32::try_from(orders.len())
            .map_err(|_| Fault::invalid(InvalidCode::Sort, Location::new("trial dimensions")))?;
        let problem = store.problem(&unknowns, diagonal, rhs)?;
        let original = store.matrix(size, &matrix)?;
        let elimination = store.matrix(size, &reduced)?;
        let one = store.expression(Operator::Rational(Rational::one()), &[])?;
        let mut steps = units
            .iter()
            .map(|(factor, inverse)| store.unit(factor, inverse, &one))
            .collect::<Result<Vec<_>, _>>()?;
        let mut operations = Vec::new();
        for (row, scale) in (0..size).zip(&scales) {
            operations.push(RowOperation::Scale {
                row,
                value: store.expression(Operator::Rational(scale.clone()), &[])?,
                unit: row,
            });
        }
        let mut step = store.step(
            SolveSchema::ExactLinearEquivalence,
            SolveStatement::LinearSystem(StatementLinearSystem {
                problem,
                reduced: elimination.clone(),
                rank: size,
                augmented_rank: size,
                region: store.region.clone(),
                free_columns: Vec::new(),
            }),
            SolveCertificate::ExactLinearEquivalence(CertificateExactLinearEquivalence {
                matrix: original,
                augmented_columns: 1,
                operations,
                pivots: (0..size).collect(),
                minor: one,
                reduced: elimination,
                region: store.region.clone(),
            }),
        );
        step.premises = (0..size).collect();
        steps.push(step);
        let verdict = match store
            .cas
            .check_proof(&store.context, &steps, &mut budget()?)
        {
            Ok(witness) => Ok(store.cas.export_witness(&witness, &mut budget()?)?),
            Err(fault) => Err(fault),
        };
        Ok(Submission {
            unknowns,
            matrix,
            reduced,
            scales,
            units,
            verdict,
        })
    }

    #[cfg(test)]
    pub(super) fn replay_joint(bytes: &[u8]) -> Result<(), Fault> {
        use nrz_cas::{domain::Layer, proof::Judgment};

        // Build the expected diagonal theorem from fixed mathematical data, without
        // inspecting the exported proof or copying the producer's submitted values.
        let keys = ["c1".to_owned(), "c2".to_owned()];
        let mut store = Store::new(&keys)?;
        let mut arithmetic = budget()?;
        let diagonal = [
            Rational::from_ratio(3, 1, &mut arithmetic)?,
            Rational::from_ratio(4, 1, &mut arithmetic)?,
        ];
        let problem = store.problem(&keys, &diagonal, &[Rational::one(), Rational::zero()])?;
        let reduced = store.matrix(
            2,
            &[
                vec![
                    Rational::one(),
                    Rational::zero(),
                    Rational::from_ratio(1, 3, &mut arithmetic)?,
                ],
                vec![Rational::zero(), Rational::one(), Rational::zero()],
            ],
        )?;
        let expected = Judgment {
            context: store.context.clone(),
            layer: Layer::Logic,
            space: None,
            statement: Statement::Solve(Box::new(SolveStatement::LinearSystem(
                StatementLinearSystem {
                    problem,
                    reduced,
                    rank: 2,
                    augmented_rank: 2,
                    region: store.region.clone(),
                    free_columns: Vec::new(),
                },
            ))),
            residuals: Vec::new(),
        };
        store.cas.replay(bytes, &expected, &mut budget()?)?;
        Ok(())
    }
}
mod tests {
    use std::fmt::Write;

    use nrz_cas::{
        Fault, FaultKind, InvalidCode, Rational,
        limits::{Budget, Limits},
    };

    use super::{Submission, solving_trial};

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

    fn transcript(
        index: usize,
        submission: &Submission,
        budget: &mut Budget,
    ) -> TestResult<String> {
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
        println!(
            "consumer route: Cas::check_proof(ApplicationDraft with Solve statement/certificate)"
        );
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
                assert_ne!(witness.as_slice(), [0u8; 0]);
            }
        }
        let joint_witness = trial.submissions[7]
            .verdict
            .as_ref()
            .map_err(Clone::clone)?;
        let replay = super::engine::replay_joint(joint_witness);
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
}
