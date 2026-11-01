//! Public graph admission and supplied proof construction for the fixed trial.

use nrz_cas::{
    Cas, Fault, Handle, InvalidCode, Location, Rational, Reference,
    domain::{AmbientDomain, ContextDraft, HeaderDraft, RegionDraft, RegionExpr},
    limits::{Budget, Limits},
    proof::{ApplicationDraft, Certificate, Primitive, Statement},
    solving::{
        CertificateCoefficientIdentity, CertificateExactLinearEquivalence, CoefficientDomainData,
        CoefficientMapData, Coordinate, CoordinateRole, CoordinateSpaceData, IdentityMode, MapMode,
        MapTarget, RowOperation, SolveCertificate, SolveDraft, SolveProblemData, SolveSchema,
        SolveStatement, StatementCoefficientEquality, StatementLinearSystem,
    },
    syntax::{
        AtomicSort, Branch, Declaration, Draft, Environment, ExpressionDraft, Extent, Field,
        Operator, PredicateKind, Sort,
    },
};

use crate::Submission;

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
