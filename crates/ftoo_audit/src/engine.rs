//! One fresh store and one monotone budget per claim, including export and replay.
use nrz_cas::{
    Cas, Fault, Handle, InvalidCode, Location, Rational, Reference,
    domain::{AmbientDomain, ContextDraft, HeaderDraft, Layer, RegionDraft, RegionExpr},
    limits::{Budget, Limits},
    proof::{Judgment, Statement, VerifiedWitness},
    series::{
        ResidualReference as R, ResidualRow, ResidualSolve, ResidualSubject, SeriesObject,
        SeriesOperation, SeriesStatement,
    },
    solving::{
        CertificateMembership, CoefficientDomainData, Coordinate, CoordinateRole,
        CoordinateSpaceData, SolveCertificate, SolveDraft, SolveProblemData, SolveStatement,
        StatementMember, TupleData,
    },
    syntax::{
        AtomicSort, Branch, Declaration, Draft, Environment, ExpressionDraft, Field, Operator,
        PredicateKind, Sort,
    },
};
pub(crate) fn invalid() -> Fault {
    Fault::invalid(InvalidCode::Rule, Location::new("fixed jet contract"))
}
fn scalar() -> Sort {
    Sort::Value(AtomicSort::Scalar(Field::Real))
}
pub(crate) struct Frame {
    pub cas: Cas,
    pub budget: Budget,
    nodes: Vec<(Operator, Vec<Handle>, Handle)>,
    header: Handle,
    coefficients: Handle,
    region: Handle,
    context: Handle,
    field: Handle,
    centre: Handle,
    x: Handle,
    y: Handle,
}
fn admit(cas: &mut Cas, budget: &mut Budget, draft: Draft) -> Result<Handle, Fault> {
    cas.admit(&[draft], budget)?.pop().ok_or_else(invalid)
}
impl Frame {
    #[expect(
        clippy::too_many_lines,
        reason = "The complete residual frame is admitted together with one budget."
    )]
    pub fn new(limits: Limits, empty: bool) -> Result<Self, Fault> {
        #[cfg(test)]
        COUNTS.with(|c| {
            let (s, b) = c.get();
            c.set((s + 1, b + 1));
        });
        let mut budget = Budget::new(limits.clone())?;
        let mut cas = Cas::new(limits)?;
        let header = admit(
            &mut cas,
            &mut budget,
            Draft::Header(HeaderDraft::Core(Environment {
                field: Field::Real,
                branch: Branch::StrictReal,
                declarations: ["x", "y"]
                    .into_iter()
                    .map(|key| Declaration {
                        key: key.into(),
                        display_name: key.into(),
                        sort: scalar(),
                    })
                    .collect(),
            })),
        )?;
        let coefficients = admit(
            &mut cas,
            &mut budget,
            Draft::Solve(SolveDraft::CoordinateSpace(CoordinateSpaceData {
                header: Reference::Existing(header.clone()),
                scalar_spaces: Vec::new(),
                coordinates: vec![Coordinate {
                    key: "x".into(),
                    field: Field::Real,
                    role: CoordinateRole::Coefficient,
                }],
            })),
        )?;
        let region = admit(
            &mut cas,
            &mut budget,
            Draft::Region(RegionDraft {
                header: Reference::Existing(header.clone()),
                scope: Vec::new(),
                ambient: AmbientDomain::Product {
                    coordinates: coefficients.clone(),
                },
                expression: RegionExpr::Ambient,
            }),
        )?;
        let context = admit(
            &mut cas,
            &mut budget,
            Draft::Context(ContextDraft::Root {
                header: Reference::Existing(header.clone()),
                admissible: Reference::Existing(region.clone()),
                assumptions: Vec::new(),
            }),
        )?;
        let field = admit(
            &mut cas,
            &mut budget,
            Draft::Solve(SolveDraft::CoefficientDomain(
                CoefficientDomainData::Rational,
            )),
        )?;
        let mut expression = |operator| {
            admit(
                &mut cas,
                &mut budget,
                Draft::Expression(ExpressionDraft {
                    header: Reference::Existing(header.clone()),
                    scope: Vec::new(),
                    operator,
                    sort: scalar(),
                    operands: Vec::new(),
                    space: None,
                    binding: None,
                }),
            )
        };
        let centre = expression(Operator::Rational(Rational::zero()))?;
        let x = expression(Operator::Free("x".into()))?;
        let y = expression(Operator::Free("y".into()))?;
        let nodes = vec![
            (
                Operator::Rational(Rational::zero()),
                Vec::new(),
                centre.clone(),
            ),
            (Operator::Free("x".into()), Vec::new(), x.clone()),
            (Operator::Free("y".into()), Vec::new(), y.clone()),
        ];
        let mut frame = Self {
            cas,
            budget,
            nodes,
            header,
            coefficients,
            region,
            context,
            field,
            centre,
            x,
            y,
        };
        if empty {
            frame.empty_region()?;
        }
        Ok(frame)
    }
    fn empty_region(&mut self) -> Result<(), Fault> {
        let one = self.node(Operator::Rational(Rational::one()), &[])?;
        let predicate = self.condition(PredicateKind::Equal, &[self.centre.clone(), one])?;
        self.region = self.admit(Draft::Region(RegionDraft {
            header: Reference::Existing(self.header.clone()),
            scope: Vec::new(),
            ambient: AmbientDomain::Product {
                coordinates: self.coefficients.clone(),
            },
            expression: RegionExpr::GuardTrue {
                base: Reference::Existing(self.region.clone()),
                predicate: Reference::Existing(predicate),
            },
        }))?;
        self.context = self.admit(Draft::Context(ContextDraft::Root {
            header: Reference::Existing(self.header.clone()),
            admissible: Reference::Existing(self.region.clone()),
            assumptions: Vec::new(),
        }))?;
        Ok(())
    }
    fn admit(&mut self, draft: Draft) -> Result<Handle, Fault> {
        admit(&mut self.cas, &mut self.budget, draft)
    }
    pub fn node(&mut self, operator: Operator, operands: &[Handle]) -> Result<Handle, Fault> {
        if let Some((_, _, handle)) = self
            .nodes
            .iter()
            .find(|(op, args, _)| *op == operator && args == operands)
        {
            return Ok(handle.clone());
        }
        let original = operator.clone();
        let sort = if matches!(operator, Operator::Predicate(_)) {
            Sort::Value(AtomicSort::Boolean)
        } else {
            scalar()
        };
        let handle = self.admit(Draft::Expression(ExpressionDraft {
            header: Reference::Existing(self.header.clone()),
            scope: Vec::new(),
            operator,
            sort,
            operands: operands.iter().cloned().map(Reference::Existing).collect(),
            space: None,
            binding: None,
        }))?;
        self.nodes
            .push((original, operands.to_vec(), handle.clone()));
        Ok(handle)
    }
    fn condition(&mut self, kind: PredicateKind, arguments: &[Handle]) -> Result<Handle, Fault> {
        self.admit(Draft::Condition {
            header: Reference::Existing(self.header.clone()),
            scope: Vec::new(),
            kind,
            arguments: arguments.iter().cloned().map(Reference::Existing).collect(),
        })
    }
    pub fn rationals(&mut self, values: &[(i64, u64)]) -> Result<Vec<Rational>, Fault> {
        values
            .iter()
            .map(|(n, d)| Rational::from_ratio(*n, *d, &mut self.budget))
            .collect()
    }
    pub fn object(&mut self, values: &[Rational]) -> Result<Handle, Fault> {
        let mut coefficients = Vec::new();
        for value in values {
            coefficients.push(Reference::Existing(
                self.node(Operator::Rational(value.clone()), &[])?,
            ));
        }
        self.admit(Draft::SeriesObject(SeriesObject {
            field: Reference::Existing(self.field.clone()),
            centre: Reference::Existing(self.centre.clone()),
            variable: Reference::Existing(self.x.clone()),
            scope: Vec::new(),
            context: Reference::Existing(self.context.clone()),
            order: u32::try_from(values.len()).map_err(|_| invalid())?,
            coefficients,
        }))
    }
    pub fn compose(
        &mut self,
        jet: &[Rational],
        increment: &[Rational],
    ) -> Result<VerifiedWitness, Fault> {
        let left = self.object(jet)?;
        let right = self.object(increment)?;
        self.cas.compute_series(
            &SeriesOperation::Compose {
                left,
                right,
                order: None,
            },
            &[],
            &mut self.budget,
        )
    }
    pub fn residual(
        &mut self,
        subject: &VerifiedWitness,
        expression: &Handle,
        rows: &[ResidualRow],
    ) -> Result<(VerifiedWitness, VerifiedWitness), Fault> {
        let solving = ResidualSolve {
            statement: SolveStatement::Member(StatementMember {
                problem: R::Problem,
                tuple: R::Tuple,
                region: R::Region,
            }),
            certificate: SolveCertificate::Membership(CertificateMembership {
                problem: R::Problem,
                tuple: R::Tuple,
                region: R::Region,
                substitutions: Vec::new(),
                definedness: Vec::new(),
            }),
            premises: Vec::new(),
        };
        let (residual, member) = self.cas.check_evaluated_residual(
            subject,
            expression,
            &self.y,
            rows,
            Some(&solving),
            &mut self.budget,
        )?;
        Ok((residual, member.ok_or_else(invalid)?))
    }
    pub fn export(&mut self, witness: &VerifiedWitness) -> Result<Vec<u8>, Fault> {
        self.cas.export_witness(witness, &mut self.budget)
    }
    /// Independent expected syntax; no emitted certificate is inspected.
    #[expect(
        clippy::too_many_lines,
        reason = "Independent expected syntax keeps the complete theorem visible together."
    )]
    pub fn expectations(&mut self) -> Result<[Judgment; 2], Fault> {
        self.region = self.admit(Draft::Region(RegionDraft {
            header: Reference::Existing(self.header.clone()),
            scope: Vec::new(),
            ambient: AmbientDomain::Product {
                coordinates: self.coefficients.clone(),
            },
            expression: RegionExpr::Ambient,
        }))?;
        self.context = self.admit(Draft::Context(ContextDraft::Root {
            header: Reference::Existing(self.header.clone()),
            admissible: Reference::Existing(self.region.clone()),
            assumptions: Vec::new(),
        }))?;
        let j = self.rationals(&[(0, 1), (1, 3), (0, 1), (-1, 45), (0, 1)])?;
        let h = self.rationals(&[(0, 1), (1, 1), (0, 1), (0, 1), (0, 1)])?;
        let left = self.object(&j)?;
        let right = self.object(&h)?;
        let subject = ResidualSubject::Compose {
            left: left.clone(),
            right: right.clone(),
            output: left.clone(),
            order: 5,
            default_order: true,
        };
        // Independently specify ((x*y') + (2*y)) + (x*y^2) - x.
        let dependent = self.y.clone();
        let derivative = self.node(
            Operator::Derivative("x".into()),
            std::slice::from_ref(&dependent),
        )?;
        let a = self.node(Operator::Mul, &[self.x.clone(), derivative])?;
        let two = Rational::from_ratio(2, 1, &mut self.budget)?;
        let two = self.node(Operator::Rational(two), &[])?;
        let b = self.node(Operator::Mul, &[two, self.y.clone()])?;
        let a = self.node(Operator::Add, &[a, b])?;
        let square = self.node(Operator::IntegerPower(2), std::slice::from_ref(&dependent))?;
        let b = self.node(Operator::Mul, &[self.x.clone(), square])?;
        let sum = self.node(Operator::Add, &[a, b])?;
        let expression = self.node(Operator::Sub, &[sum, self.x.clone()])?;
        let output = self.object(&[
            Rational::zero(),
            Rational::zero(),
            Rational::zero(),
            Rational::zero(),
        ])?;
        let zero = self.centre.clone();
        let mut equations = Vec::new();
        for _ in 0..4 {
            equations.push(self.node(
                Operator::Predicate(PredicateKind::Equal),
                &[zero.clone(), zero.clone()],
            )?);
        }
        let predicate = self.condition(PredicateKind::And, &equations)?;
        let unknowns = self.admit(Draft::Solve(SolveDraft::CoordinateSpace(
            CoordinateSpaceData {
                header: Reference::Existing(self.header.clone()),
                scalar_spaces: Vec::new(),
                coordinates: Vec::new(),
            },
        )))?;
        let domain = self.admit(Draft::Region(RegionDraft {
            header: Reference::Existing(self.header.clone()),
            scope: Vec::new(),
            ambient: AmbientDomain::Product {
                coordinates: unknowns.clone(),
            },
            expression: RegionExpr::Ambient,
        }))?;
        let problem = self.admit(Draft::Solve(SolveDraft::SolveProblem(SolveProblemData {
            header: Reference::Existing(self.header.clone()),
            scope: Vec::new(),
            predicate: Reference::Existing(predicate),
            coefficients: Reference::Existing(self.coefficients.clone()),
            unknowns: Reference::Existing(unknowns.clone()),
            region: Reference::Existing(self.region.clone()),
            domain: Reference::Existing(domain),
            coefficient_domain: Reference::Existing(self.field.clone()),
            ambient: Field::Real,
            denominators: Vec::new(),
        })))?;
        let tuple = self.admit(Draft::Solve(SolveDraft::Tuple(TupleData {
            header: Reference::Existing(self.header.clone()),
            scope: Vec::new(),
            coordinates: Reference::Existing(unknowns),
            entries: Vec::new(),
        })))?;
        let residual = self.judgment(Statement::Series(Box::new(
            SeriesStatement::EvaluatedResidual {
                subject,
                expression,
                dependent: self.y.clone(),
                output,
                problem: problem.clone(),
                derivative_order: 1,
                order: 4,
            },
        )));
        let member = self.judgment(Statement::Solve(Box::new(SolveStatement::Member(
            StatementMember {
                problem,
                tuple,
                region: self.region.clone(),
            },
        ))));
        Ok([residual, member])
    }
    fn judgment(&self, statement: Statement) -> Judgment {
        Judgment {
            context: self.context.clone(),
            layer: Layer::Logic,
            space: None,
            statement,
            residuals: Vec::new(),
        }
    }
    pub fn replay(
        &mut self,
        evidence: &[Vec<u8>; 3],
        expected: &[Judgment; 2],
    ) -> Result<(), Fault> {
        self.cas.replay_evaluated_residual(
            &evidence[2],
            &expected[0],
            &expected[1],
            &mut self.budget,
        )?;
        Ok(())
    }
}
/// Untrusted trace arithmetic; the engine derives and checks every equation.
#[derive(Clone)]
pub(crate) enum Term {
    X,
    Y,
    Derivative(Box<Self>),
    Constant(i64),
    Binary(Operator, Box<Self>, Box<Self>),
    Power(Box<Self>, u32),
}
impl Frame {
    pub fn term(
        &mut self,
        term: &Term,
        jet: &[Rational],
        order: usize,
    ) -> Result<(Handle, Vec<ResidualRow>), Fault> {
        let mut rows = Vec::new();
        let (expression, _) = self.walk(term, jet, order, &mut rows)?;
        Ok((expression, rows))
    }
    fn walk(
        &mut self,
        term: &Term,
        jet: &[Rational],
        order: usize,
        rows: &mut Vec<ResidualRow>,
    ) -> Result<(Handle, Vec<Rational>), Fault> {
        let (node, values) = match term {
            Term::X => (self.x.clone(), vec![Rational::zero(), Rational::one()]),
            Term::Y => (self.y.clone(), jet.to_vec()),
            Term::Constant(n) => {
                let q = Rational::from_ratio(*n, 1, &mut self.budget)?;
                (self.node(Operator::Rational(q.clone()), &[])?, vec![q])
            }
            Term::Derivative(child) => {
                let (child, values) = self.walk(child, jet, order, rows)?;
                let mut derived = Vec::new();
                for (degree, value) in values.iter().enumerate().skip(1) {
                    let factor = Rational::from_ratio(
                        i64::try_from(degree).map_err(|_| invalid())?,
                        1,
                        &mut self.budget,
                    )?;
                    derived.push(value.mul(&factor, &mut self.budget)?);
                }
                (
                    self.node(Operator::Derivative("x".into()), &[child])?,
                    derived,
                )
            }
            Term::Binary(op, a, b) => {
                let (a, av) = self.walk(a, jet, order, rows)?;
                let (b, bv) = self.walk(b, jet, order, rows)?;
                let values = polynomial(op, &av, &bv, &mut self.budget)?;
                (self.node(op.clone(), &[a, b])?, values)
            }
            Term::Power(child, power) => {
                let (child, base) = self.walk(child, jet, order, rows)?;
                let mut values = vec![Rational::one()];
                for _ in 0..*power {
                    values = polynomial(&Operator::Mul, &values, &base, &mut self.budget)?;
                    row(rows, &values, order)?;
                }
                (
                    self.node(Operator::IntegerPower(i64::from(*power)), &[child])?,
                    values,
                )
            }
        };
        row(rows, &values, order)?;
        Ok((node, values))
    }
}
fn row(rows: &mut Vec<ResidualRow>, values: &[Rational], order: usize) -> Result<(), Fault> {
    rows.push(ResidualRow {
        ordinal: u32::try_from(rows.len()).map_err(|_| invalid())?,
        coefficients: (0..order)
            .map(|i| values.get(i).cloned().unwrap_or_else(Rational::zero))
            .collect(),
    });
    Ok(())
}
fn polynomial(
    op: &Operator,
    a: &[Rational],
    b: &[Rational],
    budget: &mut Budget,
) -> Result<Vec<Rational>, Fault> {
    if *op == Operator::Mul {
        let mut values = vec![Rational::zero(); a.len() + b.len()];
        for (i, left) in a.iter().enumerate() {
            for (j, right) in b.iter().enumerate() {
                values[i + j] = values[i + j].add(&left.mul(right, budget)?, budget)?;
            }
        }
        Ok(values)
    } else {
        (0..a.len().max(b.len()))
            .map(|i| {
                let zero = Rational::zero();
                let left = a.get(i).unwrap_or(&zero);
                let right = b.get(i).unwrap_or(&zero);
                if *op == Operator::Add {
                    left.add(right, budget)
                } else {
                    left.sub(right, budget)
                }
            })
            .collect()
    }
}

#[cfg(test)]
thread_local! { pub(crate) static COUNTS: std::cell::Cell<(usize,usize)> = const { std::cell::Cell::new((0,0)) }; }
