use super::{
    NotationRenderError, Result, admission, katex,
    render::{self, Declaration},
    rendered_site,
    support::Support,
    typeset,
};
use ftoo_manifest::{
    Callee, CheckedNotation, DefinitionId, DefinitionKind, ExprDraft, NotationErrors,
    NotationFault, NotationOrigin, NotationSite, NotationStage, PrimitiveId, RenderOwner,
    RenderTarget, RuleId,
};

#[derive(Debug)]
pub struct SweepRow {
    owner: RenderOwner,
    case: String,
    macro_source: String,
    unicode_source: String,
}
impl SweepRow {
    #[must_use]
    pub const fn owner(&self) -> RenderOwner {
        self.owner
    }
    #[must_use]
    pub fn case(&self) -> &str {
        &self.case
    }
    #[must_use]
    pub fn macro_source(&self) -> &str {
        &self.macro_source
    }
    #[must_use]
    pub fn unicode_source(&self) -> &str {
        &self.unicode_source
    }
}
pub(super) fn inventory(n: &CheckedNotation<'_, '_>, records: &[Declaration]) -> Result<()> {
    let mut missing = vec![];
    let mut repeated = vec![];
    for d in n.definitions() {
        let id = d.spec().id();
        let count = records.iter().filter(|r| r.id == id).count();
        if count == 0 {
            missing.push(id);
        } else if count > 1 {
            repeated.push((id, count));
        }
    }
    missing.sort_by_key(|id| id.token());
    repeated.sort_by_key(|(id, _)| id.token());
    if !missing.is_empty() || !repeated.is_empty() {
        return Err(admission(
            RuleId::M052,
            NotationStage::Names,
            NotationSite::Catalogue {
                file: "crates/ftoo_manifest/src/notation/catalogue.rs".into(),
            },
            NotationFault::SweepInventory { missing, repeated },
        ));
    }
    let mut errors = vec![];
    for record in records {
        let d = n.definition(record.id);
        let fault = if record.command != d.command() {
            Some(NotationFault::SweepCommand {
                definition: record.id,
                expected: d.command().into(),
                actual: record.command.clone(),
            })
        } else if record.arity != d.spec().signature().arity {
            Some(NotationFault::SweepSignature {
                definition: record.id,
                expected: d.spec().signature().arity,
                actual: record.arity,
            })
        } else {
            None
        };
        if let Some(fault) = fault {
            errors.push(ftoo_manifest::NotationDiagnostic {
                rule: RuleId::M052,
                stage: NotationStage::Names,
                site: declaration_site(record, 0),
                fault,
            });
        }
    }
    match NotationErrors::from_diagnostics(errors) {
        Some(e) => Err(e.into()),
        None => Ok(()),
    }
}
fn declaration_site(record: &Declaration, offset: usize) -> NotationSite {
    NotationSite::Rendered {
        owner: RenderOwner::Definition(record.id),
        case: "declaration".into(),
        bytes: record.range.start + offset..record.range.end + offset,
        origins: vec![NotationOrigin {
            definition: Some(record.id),
            path: vec![],
        }],
    }
}
pub(super) fn namespace(support: &Support, records: &[Declaration], context: &str) -> Result<()> {
    let mut prefix = context.to_string();
    for record in records {
        let site = declaration_site(record, context.len());
        let probe = render::combine(
            &prefix,
            &format!("\\newcommand{{{}}}[0]{{}}", record.command),
            site.clone(),
        )?;
        if let Err(message) = katex::invoke(support, Some(&probe))? {
            return Err(admission(
                RuleId::M050,
                NotationStage::Typesetting,
                site,
                NotationFault::HostCollision {
                    command: record.command.clone(),
                    message,
                },
            ));
        }
        let full = render::combine(&prefix, &record.source, site.clone())?;
        typeset(support, RenderTarget::Macro, &full, site)?;
        prefix = full;
    }
    Ok(())
}
fn literals(arity: u8) -> Vec<ExprDraft> {
    ["x", "y", "z", "u", "v", "w", "a", "b", "c"][..usize::from(arity)]
        .iter()
        .map(|s| ExprDraft::Literal { value: (*s).into() })
        .collect()
}
fn call(callee: Callee, arguments: Vec<ExprDraft>) -> ExprDraft {
    ExprDraft::Call { callee, arguments }
}
pub(super) fn cases(n: &CheckedNotation<'_, '_>) -> Vec<(RenderOwner, String, ExprDraft)> {
    let mut cases = vec![];
    let constructors: Vec<_> = n
        .definitions()
        .iter()
        .filter(|d| matches!(d.spec().kind(), DefinitionKind::Constructor { .. }))
        .collect();
    for d in n.definitions() {
        let id = d.spec().id();
        let callee = Callee::Definition(id);
        let owner = RenderOwner::Definition(id);
        let base = literals(d.spec().signature().arity);
        cases.push((owner, "base".into(), call(callee, base.clone())));
        if base.is_empty() {
            continue;
        }
        cases.push((
            owner,
            "compound".into(),
            call(
                callee,
                base.iter()
                    .map(|arg| {
                        call(
                            Callee::Primitive(PrimitiveId::Add),
                            vec![arg.clone(), ExprDraft::Literal { value: "1".into() }],
                        )
                    })
                    .collect(),
            ),
        ));
        for i in 0..base.len() {
            for (suffix, arg) in [
                ("alpha", call(Callee::Primitive(PrimitiveId::Alpha), vec![])),
                (
                    "pending",
                    call(Callee::Definition(DefinitionId::CoefficientOne), vec![]),
                ),
            ] {
                let mut args = base.clone();
                args[i] = arg;
                cases.push((owner, format!("slot-{i}-{suffix}"), call(callee, args)));
            }
            for inner in &constructors {
                let mut args = base.clone();
                args[i] = call(
                    Callee::Definition(inner.spec().id()),
                    literals(inner.spec().signature().arity),
                );
                cases.push((
                    owner,
                    format!("slot-{i}-nested-{}", inner.spec().token()),
                    call(callee, args),
                ));
            }
        }
    }
    for p in n.primitives() {
        let owner = RenderOwner::Primitive(p.id());
        let callee = Callee::Primitive(p.id());
        let base = literals(p.signature().arity);
        cases.push((owner, "base".into(), call(callee, base.clone())));
        for i in 0..base.len() {
            for inner in n.primitives() {
                let mut args = base.clone();
                args[i] = call(
                    Callee::Primitive(inner.id()),
                    literals(inner.signature().arity),
                );
                cases.push((
                    owner,
                    format!("argument-{i}-primitive-{}", inner.token()),
                    call(callee, args),
                ));
            }
        }
    }
    cases.sort_by(|a, b| (a.0.token(), &a.1).cmp(&(b.0.token(), &b.1)));
    cases
}
pub(super) fn row(
    n: &CheckedNotation<'_, '_>,
    support: &Support,
    declarations: &str,
    owner: RenderOwner,
    case: &str,
    draft: ExprDraft,
) -> Result<SweepRow> {
    let mut errors = vec![];
    let checked = ftoo_manifest::check_expression(n, draft)?;
    let macro_form = render::expression(n, checked.draft(), RenderTarget::Macro, owner, case)?;
    let unicode = render::expression(n, checked.draft(), RenderTarget::Unicode, owner, case)?;
    let combined = render::combine(
        declarations,
        &macro_form.source,
        macro_form.site(owner, case),
    )?;
    for (target, source, origins) in [
        (RenderTarget::Macro, &combined, &unicode.origins),
        (RenderTarget::Unicode, &unicode.source, &unicode.origins),
    ] {
        match typeset(
            support,
            target,
            source,
            rendered_site(owner, case, source, origins.clone()),
        ) {
            Ok(()) => {}
            Err(NotationRenderError::Admission(e)) => errors.extend_from_slice(e.diagnostics()),
            Err(e) => return Err(e),
        }
    }
    match NotationErrors::from_diagnostics(errors) {
        Some(e) => Err(e.into()),
        None => Ok(SweepRow {
            owner,
            case: case.into(),
            macro_source: macro_form.source,
            unicode_source: unicode.source,
        }),
    }
}
pub(super) fn run(
    n: &CheckedNotation<'_, '_>,
    support: &Support,
    declarations: &str,
) -> Result<Vec<SweepRow>> {
    let mut rows = vec![];
    let mut errors = vec![];
    for (owner, case, draft) in cases(n) {
        match row(n, support, declarations, owner, &case, draft) {
            Ok(row) => rows.push(row),
            Err(NotationRenderError::Admission(e)) => errors.extend_from_slice(e.diagnostics()),
            Err(e) => return Err(e),
        }
    }
    match NotationErrors::from_diagnostics(errors) {
        Some(e) => Err(e.into()),
        None => Ok(rows),
    }
}
