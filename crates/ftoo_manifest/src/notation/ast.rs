use super::{
    Callee, DefinitionId, DefinitionKind, DefinitionSpec, LiteralReason, NotationDiagnostic,
    NotationFault, NotationOrigin, NotationPathPart, NotationResource, NotationSite, NotationStage,
    PrimitiveSpec,
};
use crate::RuleId;
use std::ops::Range;
use std::rc::Rc;

pub const MAX_DEPTH: usize = 64;
pub const MAX_AST_NODES: usize = 4_096;
pub const MAX_LITERAL_BYTES: usize = 256;
pub const MAX_EXPANSION_NODES: usize = 65_536;
pub const MAX_TEXT_BYTES: usize = 256;
pub const MAX_TEXT_RUNS: usize = 64;
pub const MAX_EXPRESSION_TEXT_BYTES: usize = 4_096;

/// Decoded mathematical words and their source and decoded byte ranges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub source: Range<usize>,
    pub decoded: Range<usize>,
}

pub(super) fn merge_plain_runs(expr: &mut ExprDraft) {
    let mut pending = vec![expr];
    while let Some(expr) = pending.pop() {
        match expr {
            ExprDraft::Text { runs } if runs.len() <= MAX_TEXT_RUNS => {
                let mut merged: Vec<TextRun> = Vec::new();
                for run in std::mem::take(runs) {
                    if let Some(last) = merged.last_mut()
                        && last.source.end == run.source.start
                        && last.decoded.end == run.decoded.start
                    {
                        last.text.push_str(&run.text);
                        last.source.end = run.source.end;
                        last.decoded.end = run.decoded.end;
                        continue;
                    }
                    merged.push(run);
                }
                *runs = merged;
            }
            ExprDraft::Call { arguments, .. } => pending.extend(arguments.iter_mut()),
            _ => {}
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MathSort {
    MathExpr,
}
/// All parameters and results have the one syntactic sort, never a quantity type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Signature {
    pub arity: u8,
}
impl Signature {
    #[must_use]
    pub const fn result(self) -> MathSort {
        MathSort::MathExpr
    }
    pub fn parameters(self) -> impl Iterator<Item = MathSort> {
        (0..self.arity).map(|_| MathSort::MathExpr)
    }
}

/// Untrusted syntax; checked values never expose mutable drafts.
/// ```compile_fail,E0599
/// let expression = ftoo_manifest::ExprDraft::Raw("anything".into());
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExprDraft {
    Literal {
        value: String,
    },
    Text {
        runs: Vec<TextRun>,
    },
    Slot {
        index: u8,
    },
    Call {
        callee: Callee,
        arguments: Vec<Self>,
    },
}
impl Drop for ExprDraft {
    fn drop(&mut self) {
        let Self::Call { arguments, .. } = self else {
            return;
        };
        let mut pending = std::mem::take(arguments);
        while let Some(mut expr) = pending.pop() {
            if let Self::Call { arguments, .. } = &mut expr {
                pending.append(arguments);
            }
        }
    }
}

fn literal_fault(value: &str) -> Option<NotationFault> {
    let mut scalars = value.char_indices();
    let Some((_, first)) = scalars.next() else {
        return Some(NotationFault::InvalidLiteral {
            value: value.into(),
            reason: LiteralReason::Empty,
            byte: 0,
            scalar: None,
        });
    };
    let bad = if first.is_ascii_alphabetic() {
        scalars.next().map(|(byte, c)| {
            (
                byte,
                c,
                if c.is_ascii_alphabetic() {
                    LiteralReason::MultipleLetters
                } else {
                    LiteralReason::ForbiddenScalar
                },
            )
        })
    } else if first.is_ascii_digit() {
        scalars
            .find(|(_, c)| !c.is_ascii_digit())
            .map(|(byte, c)| (byte, c, LiteralReason::ForbiddenScalar))
    } else {
        Some((0, first, LiteralReason::ForbiddenScalar))
    };
    bad.map(|(byte, c, reason)| NotationFault::InvalidLiteral {
        value: value.into(),
        reason,
        byte,
        scalar: Some(c.to_string()),
    })
}

pub(super) fn limit(
    site: NotationSite,
    stage: NotationStage,
    resource: NotationResource,
    maximum: usize,
) -> NotationDiagnostic {
    NotationDiagnostic {
        rule: RuleId::M047,
        stage,
        site,
        fault: NotationFault::NotationLimit {
            resource,
            limit: maximum,
            actual: maximum + 1,
        },
    }
}

fn declared_arity(
    callee: Callee,
    primitives: &[PrimitiveSpec],
    definitions: &[DefinitionSpec],
) -> u8 {
    match callee {
        Callee::Primitive(id) => {
            primitives
                .iter()
                .find(|p| p.id == id)
                .expect("complete primitives")
                .signature
                .arity
        }
        Callee::Definition(id) => {
            definitions
                .iter()
                .find(|d| d.id == id)
                .expect("complete definitions")
                .signature
                .arity
        }
    }
}

#[allow(clippy::too_many_lines)] // One walk enforces shared depth, node and text bounds.
pub(super) fn validate(
    expr: &ExprDraft,
    arity: u8,
    site: &NotationSite,
    primitives: &[PrimitiveSpec],
    definitions: &[DefinitionSpec],
    allow_text: bool,
) -> Vec<NotationDiagnostic> {
    let mut errors = Vec::new();
    let mut stack = vec![(expr, site.clone(), 1)];
    let mut nodes = 0;
    let mut used = [false; 9];
    let mut text_bytes = 0;
    while let Some((expr, site, depth)) = stack.pop() {
        if nodes >= MAX_AST_NODES {
            errors.push(limit(
                site,
                NotationStage::Ast,
                NotationResource::AstNodes,
                MAX_AST_NODES,
            ));
            break;
        }
        nodes += 1;
        if depth > MAX_DEPTH {
            errors.push(limit(
                site,
                NotationStage::Ast,
                NotationResource::Depth,
                MAX_DEPTH,
            ));
            continue;
        }
        let mut push = |site, fault| {
            errors.push(NotationDiagnostic {
                rule: RuleId::M047,
                stage: NotationStage::Ast,
                site,
                fault,
            });
        };
        match expr {
            ExprDraft::Literal { value } => {
                if value.len() > MAX_LITERAL_BYTES {
                    errors.push(limit(
                        site.field("value"),
                        NotationStage::Ast,
                        NotationResource::LiteralBytes,
                        MAX_LITERAL_BYTES,
                    ));
                } else if let Some(fault) = literal_fault(value) {
                    push(site.field("value"), fault);
                }
            }
            ExprDraft::Text { runs } => {
                if !allow_text {
                    push(site.clone(), NotationFault::TextOutsideDocument);
                }
                if runs.is_empty() || runs.len() > MAX_TEXT_RUNS {
                    push(
                        site.clone(),
                        NotationFault::InvalidTextRuns { count: runs.len() },
                    );
                }
                if nodes.saturating_add(runs.len()) > MAX_AST_NODES {
                    push(
                        site.clone(),
                        NotationFault::NotationLimit {
                            resource: NotationResource::AstNodes,
                            limit: MAX_AST_NODES,
                            actual: nodes.saturating_add(runs.len()),
                        },
                    );
                    break;
                }
                nodes += runs.len();
                for (index, run) in runs.iter().enumerate() {
                    let run_site = site.field("runs").child(NotationPathPart::Argument(index));
                    if run.text.len() > MAX_TEXT_BYTES {
                        push(
                            run_site,
                            NotationFault::NotationLimit {
                                resource: NotationResource::TextBytes,
                                limit: MAX_TEXT_BYTES,
                                actual: run.text.len(),
                            },
                        );
                        continue;
                    }
                    text_bytes += run.text.len();
                    if text_bytes > MAX_EXPRESSION_TEXT_BYTES {
                        push(
                            run_site,
                            NotationFault::NotationLimit {
                                resource: NotationResource::ExpressionTextBytes,
                                limit: MAX_EXPRESSION_TEXT_BYTES,
                                actual: text_bytes,
                            },
                        );
                        continue;
                    }
                    if run.decoded.end.checked_sub(run.decoded.start) != Some(run.text.len())
                        || run.source.end.checked_sub(run.source.start) != Some(run.text.len())
                        || run.text.trim_matches(' ').is_empty()
                        || run
                            .text
                            .bytes()
                            .any(|b| !b.is_ascii_alphabetic() && !matches!(b, b' ' | b',' | b'-'))
                        || run
                            .text
                            .as_bytes()
                            .windows(2)
                            .any(|pair| pair[0] == pair[1] && matches!(pair[0], b' ' | b',' | b'-'))
                    {
                        push(
                            run_site,
                            NotationFault::InvalidText {
                                value: run.text.clone(),
                            },
                        );
                    }
                }
                for (index, pair) in runs.windows(2).enumerate() {
                    if pair[0].source.end > pair[1].source.start
                        || pair[0].decoded.end > pair[1].decoded.start
                    {
                        push(
                            site.field("runs")
                                .child(NotationPathPart::Argument(index + 1)),
                            NotationFault::InvalidText {
                                value: pair[1].text.clone(),
                            },
                        );
                    }
                    let (Some(&left), Some(&right)) = (
                        pair[0].text.as_bytes().last(),
                        pair[1].text.as_bytes().first(),
                    ) else {
                        continue;
                    };
                    if left == right && matches!(left, b' ' | b',' | b'-') {
                        push(
                            site.field("runs")
                                .child(NotationPathPart::Argument(index + 1)),
                            NotationFault::InvalidText {
                                value: format!("{}{}", left as char, right as char),
                            },
                        );
                    }
                }
            }
            ExprDraft::Slot { index } => {
                if *index >= arity {
                    push(
                        site.field("index"),
                        NotationFault::SlotOutOfScope {
                            index: *index,
                            arity,
                        },
                    );
                } else {
                    used[usize::from(*index)] = true;
                }
            }
            ExprDraft::Call { callee, arguments } => {
                if matches!(callee, Callee::Primitive(super::PrimitiveId::Underbrace))
                    && arguments.get(1).is_some_and(|annotation| {
                        matches!(annotation, ExprDraft::Text { runs } if {
                            let mut bytes = runs.iter().flat_map(|run| run.text.bytes()).skip_while(|byte| *byte == b' ');
                            b"pending".iter().all(|byte| bytes.next() == Some(*byte)) && bytes.all(|byte| byte == b' ')
                        })
                    })
                {
                    push(site.field("arguments").child(NotationPathPart::Argument(1)), NotationFault::PendingAnnotation);
                }
                let expected = declared_arity(*callee, primitives, definitions);
                if arguments.len() != usize::from(expected) {
                    push(
                        site.field("arguments"),
                        NotationFault::CallArity {
                            callee: *callee,
                            expected,
                            actual: arguments.len(),
                        },
                    );
                }
                // A malformed variadic draft cannot allocate an unbounded work stack.
                // The extra scheduled node retains the exact first-excess site.
                for (index, arg) in arguments
                    .iter()
                    .take((MAX_AST_NODES + 1).saturating_sub(nodes))
                    .enumerate()
                    .rev()
                {
                    stack.push((
                        arg,
                        site.field("arguments")
                            .child(NotationPathPart::Argument(index)),
                        depth + 1,
                    ));
                }
            }
        }
    }
    for index in 0..arity {
        if !used[usize::from(index)] {
            errors.push(NotationDiagnostic {
                rule: RuleId::M047,
                stage: NotationStage::Ast,
                site: site.clone(),
                fault: NotationFault::UnusedSlot { index, arity },
            });
        }
    }
    errors
}

pub(super) fn calls(expr: &ExprDraft, site: NotationSite) -> Vec<(DefinitionId, NotationSite)> {
    let mut stack = vec![(expr, site)];
    let mut calls = Vec::new();
    while let Some((expr, site)) = stack.pop() {
        if let ExprDraft::Call { callee, arguments } = expr {
            if let Callee::Definition(id) = callee {
                calls.push((*id, site.field("callee")));
            }
            for (i, arg) in arguments.iter().enumerate().rev() {
                stack.push((
                    arg,
                    site.field("arguments").child(NotationPathPart::Argument(i)),
                ));
            }
        }
    }
    calls
}

#[derive(Clone)]
struct Bound<'a> {
    expr: &'a ExprDraft,
    environment: Rc<Vec<Bound<'a>>>,
    site: NotationSite,
    origins: Vec<NotationOrigin>,
}

/// Iterative substitution counts visits, including every repeated argument occurrence.
/// Arguments retain their original binder, preventing capture by a nested constructor.
pub(super) fn expand(
    expr: &ExprDraft,
    site: NotationSite,
    definitions: &[DefinitionSpec],
    arity: u8,
) -> Result<Vec<(DefinitionId, Vec<NotationOrigin>)>, Box<NotationDiagnostic>> {
    let placeholders: Vec<_> = (0..arity)
        .map(|_| ExprDraft::Literal { value: "x".into() })
        .collect();
    let env = Rc::new(
        placeholders
            .iter()
            .map(|expr| Bound {
                expr,
                environment: Rc::new(Vec::new()),
                site: site.clone(),
                origins: Vec::new(),
            })
            .collect(),
    );
    let mut stack = vec![Bound {
        expr,
        environment: env,
        site,
        origins: Vec::new(),
    }];
    let mut visited = 0;
    let mut uses = Vec::new();
    while let Some(bound) = stack.pop() {
        if visited == MAX_EXPANSION_NODES {
            return Err(Box::new(limit(
                bound.site,
                NotationStage::Dependencies,
                NotationResource::ExpansionNodes,
                MAX_EXPANSION_NODES,
            )));
        }
        visited += 1;
        match bound.expr {
            ExprDraft::Literal { .. } | ExprDraft::Text { .. } => {}
            ExprDraft::Slot { index } => {
                let mut argument = bound.environment[usize::from(*index)].clone();
                let mut origins = bound.origins;
                if let NotationSite::Registry {
                    entry: super::RegistryEntry::Definition(id),
                    path,
                    ..
                } = bound.site
                {
                    origins.push(NotationOrigin {
                        definition: Some(id),
                        path,
                    });
                }
                argument.origins = origins;
                stack.push(argument);
            }
            ExprDraft::Call {
                callee: Callee::Primitive(_),
                arguments,
            } => {
                for (i, expr) in arguments.iter().enumerate().rev() {
                    stack.push(Bound {
                        expr,
                        environment: bound.environment.clone(),
                        site: bound
                            .site
                            .field("arguments")
                            .child(NotationPathPart::Argument(i)),
                        origins: bound.origins.clone(),
                    });
                }
            }
            ExprDraft::Call {
                callee: Callee::Definition(id),
                arguments,
            } => {
                stack.push(definition_body(
                    &bound,
                    *id,
                    arguments,
                    definitions,
                    &mut visited,
                    &mut uses,
                )?);
            }
        }
    }
    uses.sort_by_key(|(id, _)| id.token());
    Ok(uses)
}

fn definition_body<'a>(
    bound: &Bound<'a>,
    id: DefinitionId,
    arguments: &'a [ExprDraft],
    definitions: &'a [DefinitionSpec],
    visited: &mut usize,
    uses: &mut Vec<(DefinitionId, Vec<NotationOrigin>)>,
) -> Result<Bound<'a>, Box<NotationDiagnostic>> {
    let mut current = id;
    let mut origins = bound.origins.clone();
    let (definition, path) = match &bound.site {
        NotationSite::Registry {
            entry: super::RegistryEntry::Definition(id),
            path,
            ..
        } => (Some(*id), path.clone()),
        NotationSite::Expression { path } => (None, path.clone()),
        _ => unreachable!("expression site"),
    };
    origins.push(NotationOrigin { definition, path });
    loop {
        let (i, spec) = definitions
            .iter()
            .enumerate()
            .find(|(_, d)| d.id == current)
            .expect("complete definitions");
        if !matches!(spec.kind, DefinitionKind::Constructor { .. }) {
            uses.push((current, origins.clone()));
        }
        let site = super::catalogue::site(i, current);
        match &spec.kind {
            DefinitionKind::Alias { target, .. } => {
                if *visited == MAX_EXPANSION_NODES {
                    return Err(Box::new(limit(
                        site.field("target"),
                        NotationStage::Dependencies,
                        NotationResource::ExpansionNodes,
                        MAX_EXPANSION_NODES,
                    )));
                }
                *visited += 1;
                origins.push(NotationOrigin {
                    definition: Some(current),
                    path: vec![NotationPathPart::Field("target".into())],
                });
                current = *target;
            }
            DefinitionKind::Canonical { body, .. }
            | DefinitionKind::Constructor { body }
            | DefinitionKind::Inherited { body } => {
                let environment = Rc::new(
                    arguments
                        .iter()
                        .enumerate()
                        .map(|(i, expr)| Bound {
                            expr,
                            environment: bound.environment.clone(),
                            site: bound
                                .site
                                .field("arguments")
                                .child(NotationPathPart::Argument(i)),
                            origins: bound.origins.clone(),
                        })
                        .collect(),
                );
                return Ok(Bound {
                    expr: body,
                    environment,
                    site: site.field("body"),
                    origins,
                });
            }
        }
    }
}
