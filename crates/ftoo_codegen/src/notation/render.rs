use super::{MAX_NOTATION_OUTPUT_BYTES, Result, admission, rendered_site};
use ftoo_manifest::{
    Callee, CheckedNotation, DefinitionId, DefinitionKind, ExprDraft, NotationFault,
    NotationOrigin, NotationPathPart, NotationResource, NotationSite, NotationStage,
    PrimitiveOperation, RenderOwner, RenderTarget, RuleId, SymbolTier,
};
use std::{collections::BTreeSet, rc::Rc};

#[derive(Debug)]
pub(super) struct Candidate {
    pub source: String,
    pub origins: Vec<NotationOrigin>,
}
impl Candidate {
    pub fn site(&self, owner: RenderOwner, case: &str) -> NotationSite {
        rendered_site(owner, case, &self.source, self.origins.clone())
    }
}
#[derive(Clone, Debug)]
pub(super) struct Declaration {
    pub id: DefinitionId,
    pub command: String,
    pub arity: u8,
    pub source: String,
    pub range: std::ops::Range<usize>,
}
#[derive(Clone)]
struct Frame<'a> {
    expr: &'a ExprDraft,
    environment: Rc<Vec<Self>>,
    definition: Option<DefinitionId>,
    path: Vec<NotationPathPart>,
    origins: Vec<NotationOrigin>,
}
impl<'a> Frame<'a> {
    fn child(&self, expr: &'a ExprDraft, index: usize) -> Self {
        let mut path = self.path.clone();
        path.extend([
            NotationPathPart::Field("arguments".into()),
            NotationPathPart::Argument(index),
        ]);
        Self {
            expr,
            environment: self.environment.clone(),
            definition: self.definition,
            path,
            origins: self.origins.clone(),
        }
    }
    fn origin(&self) -> NotationOrigin {
        NotationOrigin {
            definition: self.definition,
            path: self.path.clone(),
        }
    }
}
enum Action<'a> {
    Text(String, Vec<NotationOrigin>),
    Visit(Frame<'a>),
}
enum Part {
    Fixed(&'static str),
    Argument(usize),
}
fn parts(operation: PrimitiveOperation, target: RenderTarget) -> Vec<Part> {
    use Part::{Argument as A, Fixed as F};
    use PrimitiveOperation::{
        Add, Alpha, Beta, Dimension, Equal, Fraction, Juxtapose, Multiply, Negate, Parentheses,
        Subscript, Superscript,
    };
    match operation {
        Alpha | Beta => {
            if target == RenderTarget::Macro {
                vec![F(operation.host_tokens()[0]), F("{}")]
            } else {
                vec![F(operation
                    .unicode_scalar()
                    .expect("closed scalar operation"))]
            }
        }
        Fraction => vec![
            F(operation.host_tokens()[0]),
            F("{"),
            A(0),
            F("}{"),
            A(1),
            F("}"),
        ],
        Subscript => vec![F("{"), A(0), F("}_{"), A(1), F("}")],
        Superscript => vec![F("{"), A(0), F("}^{"), A(1), F("}")],
        Add => vec![F("({"), A(0), F("}+{"), A(1), F("})")],
        Equal => vec![F("({"), A(0), F("}={"), A(1), F("})")],
        Negate => vec![
            F("("),
            F(if target == RenderTarget::Macro {
                "-"
            } else {
                operation.unicode_scalar().expect("closed scalar operation")
            }),
            F("{"),
            A(0),
            F("})"),
        ],
        Multiply => {
            let mut parts = vec![F("({"), A(0), F("}")];
            if target == RenderTarget::Macro {
                parts.extend([F(operation.host_tokens()[0]), F("{}")]);
            } else {
                parts.push(F(operation
                    .unicode_scalar()
                    .expect("closed scalar operation")));
            }
            parts.extend([F("{"), A(1), F("})")]);
            parts
        }
        Juxtapose => vec![
            F("{"),
            A(0),
            F("}"),
            F(operation.host_tokens()[0]),
            F("{"),
            A(1),
            F("}"),
        ],
        Parentheses => vec![F("({"), A(0), F("})")],
        Dimension => vec![F("[{"), A(0), F("}]")],
    }
}
pub(super) fn append(
    output: &mut Candidate,
    text: &str,
    origins: Vec<NotationOrigin>,
    owner: RenderOwner,
    case: &str,
) -> Result<()> {
    if output.source.len() + text.len() > MAX_NOTATION_OUTPUT_BYTES {
        return Err(admission(
            RuleId::M051,
            NotationStage::Names,
            NotationSite::Rendered {
                owner,
                case: case.into(),
                bytes: output.source.len()..output.source.len() + text.len(),
                origins,
            },
            NotationFault::NotationLimit {
                resource: NotationResource::OutputBytes,
                limit: MAX_NOTATION_OUTPUT_BYTES,
                actual: MAX_NOTATION_OUTPUT_BYTES + 1,
            },
        ));
    }
    output.source.push_str(text);
    for origin in origins {
        if !output.origins.contains(&origin) {
            output.origins.push(origin);
        }
    }
    Ok(())
}
fn pending(
    effective: Option<&ftoo_audit::retention::RetainedNotation<'_, '_, '_>>,
    n: &CheckedNotation<'_, '_>,
    id: DefinitionId,
) -> bool {
    if let Some(view) = effective {
        return view
            .effective_status(id)
            .is_some_and(|s| s.tier() == SymbolTier::Pending);
    }
    n.definition(id)
        .status()
        .is_some_and(|s| s.tier() == SymbolTier::Pending)
}
#[derive(Clone, Copy)]
enum Mode {
    Expression,
    Declaration,
    PendingBody,
}
fn definition<'a>(
    effective: Option<&ftoo_audit::retention::RetainedNotation<'_, '_, '_>>,
    n: &'a CheckedNotation<'_, '_>,
    id: DefinitionId,
    environment: Rc<Vec<Frame<'a>>>,
    mut origins: Vec<NotationOrigin>,
    stack: &mut Vec<Action<'a>>,
    unmarked: bool,
) {
    if !unmarked && pending(effective, n, id) {
        stack.push(Action::Text("}_{\\text{pending}}".into(), origins.clone()));
    }
    match n.definition(id).spec().kind() {
        DefinitionKind::Alias { target, .. } => {
            origins.push(NotationOrigin {
                definition: Some(id),
                path: vec![NotationPathPart::Field("target".into())],
            });
            definition(
                effective,
                n,
                *target,
                environment,
                origins.clone(),
                stack,
                unmarked,
            );
        }
        DefinitionKind::Canonical { body, .. } | DefinitionKind::Constructor { body } => stack
            .push(Action::Visit(Frame {
                expr: body,
                environment,
                definition: Some(id),
                path: vec![NotationPathPart::Field("body".into())],
                origins: origins.clone(),
            })),
    }
    if !unmarked && pending(effective, n, id) {
        stack.push(Action::Text("\\underbrace{".into(), origins));
    }
}
fn emit<'a>(
    effective: Option<&ftoo_audit::retention::RetainedNotation<'_, '_, '_>>,
    n: &'a CheckedNotation<'_, '_>,
    mut stack: Vec<Action<'a>>,
    target: RenderTarget,
    owner: RenderOwner,
    case: &str,
    mode: Mode,
) -> Result<Candidate> {
    let unmarked = matches!(mode, Mode::PendingBody);
    let mut output = Candidate {
        source: String::new(),
        origins: vec![],
    };
    while let Some(action) = stack.pop() {
        match action {
            Action::Text(text, origins) => append(&mut output, &text, origins, owner, case)?,
            Action::Visit(frame) => {
                let mut origins = frame.origins.clone();
                origins.push(frame.origin());
                match frame.expr {
                    ExprDraft::Literal { value } => {
                        append(&mut output, value, origins, owner, case)?;
                    }
                    ExprDraft::Slot { index } if matches!(mode, Mode::Declaration) => append(
                        &mut output,
                        &format!("#{}", index + 1),
                        origins,
                        owner,
                        case,
                    )?,
                    ExprDraft::Slot { index } => {
                        let mut argument = frame.environment[usize::from(*index)].clone();
                        argument.origins = origins;
                        stack.push(Action::Visit(argument));
                    }
                    ExprDraft::Call {
                        callee: Callee::Primitive(id),
                        arguments,
                    } => {
                        let operation = n
                            .primitives()
                            .iter()
                            .find(|p| p.id() == *id)
                            .expect("checked primitive")
                            .operation();
                        for part in parts(operation, target).into_iter().rev() {
                            stack.push(match part {
                                Part::Fixed(text) => Action::Text(text.into(), origins.clone()),
                                Part::Argument(i) => Action::Visit(frame.child(&arguments[i], i)),
                            });
                        }
                    }
                    ExprDraft::Call {
                        callee: Callee::Definition(id),
                        arguments,
                    } if target == RenderTarget::Macro && !unmarked => {
                        if arguments.is_empty() {
                            stack.push(Action::Text("{}".into(), origins.clone()));
                        }
                        for (i, arg) in arguments.iter().enumerate().rev() {
                            stack.push(Action::Text("}".into(), origins.clone()));
                            stack.push(Action::Visit(frame.child(arg, i)));
                            stack.push(Action::Text("{".into(), origins.clone()));
                        }
                        stack.push(Action::Text(n.definition(*id).command().into(), origins));
                    }
                    ExprDraft::Call {
                        callee: Callee::Definition(id),
                        arguments,
                    } => {
                        let environment = Rc::new(
                            arguments
                                .iter()
                                .enumerate()
                                .map(|(i, arg)| frame.child(arg, i))
                                .collect(),
                        );
                        definition(
                            effective,
                            n,
                            *id,
                            environment,
                            origins,
                            &mut stack,
                            unmarked,
                        );
                    }
                }
            }
        }
    }
    if target == RenderTarget::Unicode {
        complete(&output, owner, case)?;
    }
    Ok(output)
}
pub(super) fn complete(output: &Candidate, owner: RenderOwner, case: &str) -> Result<()> {
    let position = output
        .source
        .find("\\Ftoo")
        .or_else(|| output.source.find('#'));
    if let Some(start) = position {
        let remaining = output.source[start..]
            .chars()
            .take_while(|c| c.is_ascii_alphabetic() || matches!(c, '\\' | '#' | '0'..='9'))
            .collect();
        let origin = output.origins.first().cloned().unwrap_or(NotationOrigin {
            definition: None,
            path: vec![],
        });
        return Err(admission(
            RuleId::M051,
            NotationStage::Names,
            output.site(owner, case),
            NotationFault::IncompleteExpansion { remaining, origin },
        ));
    }
    Ok(())
}
pub(super) fn expression(
    effective: Option<&ftoo_audit::retention::RetainedNotation<'_, '_, '_>>,
    n: &CheckedNotation<'_, '_>,
    expr: &ExprDraft,
    target: RenderTarget,
    owner: RenderOwner,
    case: &str,
) -> Result<Candidate> {
    emit(
        effective,
        n,
        vec![Action::Visit(Frame {
            expr,
            environment: Rc::new(vec![]),
            definition: None,
            path: vec![],
            origins: vec![],
        })],
        target,
        owner,
        case,
        Mode::Expression,
    )
}
#[cfg(test)]
pub(super) fn fixture_body(
    n: &CheckedNotation<'_, '_>,
    body: &ExprDraft,
    arguments: &[ExprDraft],
) -> Result<Candidate> {
    let effective = None;
    let environment = Rc::new(
        arguments
            .iter()
            .enumerate()
            .map(|(i, expr)| Frame {
                expr,
                environment: Rc::new(vec![]),
                definition: None,
                path: vec![
                    NotationPathPart::Field("arguments".into()),
                    NotationPathPart::Argument(i),
                ],
                origins: vec![],
            })
            .collect(),
    );
    emit(
        effective,
        n,
        vec![Action::Visit(Frame {
            expr: body,
            environment,
            definition: Some(DefinitionId::Product),
            path: vec![NotationPathPart::Field("body".into())],
            origins: vec![],
        })],
        RenderTarget::Unicode,
        RenderOwner::Definition(DefinitionId::Product),
        "fixture",
        Mode::Expression,
    )
}
pub(super) fn combine(prefix: &str, source: &str, site: NotationSite) -> Result<String> {
    if prefix.len() + source.len() > MAX_NOTATION_OUTPUT_BYTES {
        return Err(admission(
            RuleId::M051,
            NotationStage::Names,
            site,
            NotationFault::NotationLimit {
                resource: NotationResource::OutputBytes,
                limit: MAX_NOTATION_OUTPUT_BYTES,
                actual: MAX_NOTATION_OUTPUT_BYTES + 1,
            },
        ));
    }
    Ok(format!("{prefix}{source}"))
}
fn dependencies(kind: &DefinitionKind) -> BTreeSet<DefinitionId> {
    let mut dependencies = BTreeSet::new();
    let mut stack = match kind {
        DefinitionKind::Alias { target, .. } => {
            dependencies.insert(*target);
            vec![]
        }
        DefinitionKind::Canonical { body, .. } | DefinitionKind::Constructor { body } => vec![body],
    };
    while let Some(expr) = stack.pop() {
        if let ExprDraft::Call { callee, arguments } = expr {
            if let Callee::Definition(id) = callee {
                dependencies.insert(*id);
            }
            stack.extend(arguments);
        }
    }
    dependencies
}
pub(super) fn declarations(
    effective: Option<&ftoo_audit::retention::RetainedNotation<'_, '_, '_>>,
    n: &CheckedNotation<'_, '_>,
) -> Result<Vec<Declaration>> {
    let mut remaining: Vec<_> = n.definitions().iter().collect();
    remaining.sort_by_key(|d| d.spec().token());
    let mut emitted = BTreeSet::new();
    let mut records = vec![];
    let mut offset = 0;
    while !remaining.is_empty() {
        let i = remaining
            .iter()
            .position(|d| dependencies(d.spec().kind()).is_subset(&emitted))
            .expect("checked acyclic definitions");
        let d = remaining.remove(i);
        let id = d.spec().id();
        let alias;
        let body = match d.spec().kind() {
            DefinitionKind::Alias { target, .. } => {
                alias = ExprDraft::Call {
                    callee: Callee::Definition(*target),
                    arguments: vec![],
                };
                &alias
            }
            DefinitionKind::Canonical { body, .. } | DefinitionKind::Constructor { body } => body,
        };
        let field = if matches!(d.spec().kind(), DefinitionKind::Alias { .. }) {
            "target"
        } else {
            "body"
        };
        let mut body = emit(
            effective,
            n,
            vec![Action::Visit(Frame {
                expr: body,
                environment: Rc::new(vec![]),
                definition: Some(id),
                path: vec![NotationPathPart::Field(field.into())],
                origins: vec![],
            })],
            RenderTarget::Macro,
            RenderOwner::Definition(id),
            "declaration",
            Mode::Declaration,
        )?;
        if pending(effective, n, id) {
            body.source = combine(
                "\\underbrace{",
                &body.source,
                body.site(RenderOwner::Definition(id), "declaration"),
            )?;
            body.source = combine(
                &body.source,
                "}_{\\text{pending}}",
                body.site(RenderOwner::Definition(id), "declaration"),
            )?;
        }
        let source = format!(
            "\\newcommand{{{}}}[{}]{{{}}}\n",
            d.command(),
            d.spec().signature().arity,
            body.source
        );
        if offset + source.len() > MAX_NOTATION_OUTPUT_BYTES {
            return Err(admission(
                RuleId::M051,
                NotationStage::Names,
                rendered_site(
                    RenderOwner::Definition(id),
                    "declaration",
                    &source,
                    body.origins,
                ),
                NotationFault::NotationLimit {
                    resource: NotationResource::OutputBytes,
                    limit: MAX_NOTATION_OUTPUT_BYTES,
                    actual: MAX_NOTATION_OUTPUT_BYTES + 1,
                },
            ));
        }
        let end = offset + source.len();
        records.push(Declaration {
            id,
            command: d.command().into(),
            arity: d.spec().signature().arity,
            source,
            range: offset..end,
        });
        offset = end;
        emitted.insert(id);
    }
    Ok(records)
}

/// Private explanation mode: expand every project call without status wrappers.
pub(super) fn pending_body(
    n: &CheckedNotation<'_, '_>,
    id: DefinitionId,
    target: RenderTarget,
) -> Result<Candidate> {
    let effective = None;
    let mut stack = vec![];
    definition(effective, n, id, Rc::new(vec![]), vec![], &mut stack, true);
    emit(
        effective,
        n,
        stack,
        target,
        RenderOwner::Definition(id),
        "pending-body",
        Mode::PendingBody,
    )
}
