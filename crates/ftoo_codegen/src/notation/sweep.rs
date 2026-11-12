#[cfg(test)]
use super::typeset;
use super::{
    NotationRenderError, Result, admission, katex,
    render::{self, Declaration},
    rendered_site,
    support::Support,
};
use ftoo_manifest::{
    Callee, CheckedNotation, DefinitionId, DefinitionKind, ExprDraft, NotationErrors,
    NotationFault, NotationOrigin, NotationSite, NotationStage, PrimitiveId, RenderOwner,
    RenderTarget, RuleId,
};
use std::sync::{Condvar, Mutex};

#[derive(Clone, Debug, Eq, PartialEq)]
struct MemoKey(Vec<Vec<u8>>);
struct MemoState {
    complete: Vec<MemoKey>,
    active: Vec<MemoKey>,
}
struct Memo {
    state: Mutex<MemoState>,
    wake: Condvar,
}
static MEMO: Memo = Memo::new();

struct Reservation<'a> {
    key: Option<MemoKey>,
    memo: &'a Memo,
}
impl Memo {
    const fn new() -> Self {
        Self {
            state: Mutex::new(MemoState {
                complete: vec![],
                active: vec![],
            }),
            wake: Condvar::new(),
        }
    }
    fn acquire(&self, key: MemoKey) -> Reservation<'_> {
        let mut state = self.state.lock().expect("sweep memo lock");
        loop {
            if state.complete.contains(&key) {
                return Reservation {
                    key: None,
                    memo: self,
                };
            }
            if !state.active.contains(&key) {
                state.active.push(key.clone());
                return Reservation {
                    key: Some(key),
                    memo: self,
                };
            }
            state = self.wake.wait(state).expect("sweep memo wait");
        }
    }
}
impl Reservation<'_> {
    fn publish(mut self) {
        if let Some(key) = self.key.take() {
            let mut state = self.memo.state.lock().expect("sweep memo lock");
            state.active.retain(|active| active != &key);
            if state.complete.len() == 8 {
                state.complete.remove(0);
            }
            state.complete.push(key);
            self.memo.wake.notify_all();
        }
    }
}
impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        if let Some(key) = &self.key {
            let mut state = self.memo.state.lock().expect("sweep memo lock");
            state.active.retain(|active| active != key);
            self.memo.wake.notify_all();
        }
    }
}

struct Request {
    source: String,
    target: RenderTarget,
    site: NotationSite,
    collision: Option<String>,
}
fn rejected(request: &Request, message: String) -> NotationRenderError {
    let (rule, fault) = if let Some(command) = &request.collision {
        (
            RuleId::M050,
            NotationFault::HostCollision {
                command: command.clone(),
                message,
            },
        )
    } else {
        (
            RuleId::M051,
            NotationFault::KaTeXRejected {
                target: request.target,
                source: request.source.clone(),
                message,
            },
        )
    };
    admission(
        rule,
        NotationStage::Typesetting,
        request.site.clone(),
        fault,
    )
}
struct Candidates {
    requests: Vec<Request>,
    declarations: String,
    rows: Vec<SweepRow>,
    key: Vec<Vec<u8>>,
}

/// Derive all owned evidence anew; the memo retains only successful consumption.
#[allow(clippy::too_many_lines)] // The ordered inventory and both emitted targets are derived together.
fn candidates(n: &CheckedNotation<'_, '_>, records: &[Declaration]) -> Result<Candidates> {
    let admission_phase = crate::telemetry::Phase::new("notation.source_admission");
    inventory(n, records)?;
    let mut requests = vec![];
    let mut declarations = String::new();
    for record in records {
        let site = declaration_site(record, 0);
        requests.push(Request {
            source: render::combine(
                &declarations,
                &format!("\\newcommand{{{}}}[0]{{}}", record.command),
                site.clone(),
            )?,
            target: RenderTarget::Macro,
            site: site.clone(),
            collision: Some(record.command.clone()),
        });
        declarations = render::combine(&declarations, &record.source, site.clone())?;
        requests.push(Request {
            source: declarations.clone(),
            target: RenderTarget::Macro,
            site,
            collision: None,
        });
    }
    let mut rows = vec![];
    let mut key = vec![
        super::support::MANIFEST.to_vec(),
        super::support::LOCK.to_vec(),
        super::support::DIGESTS.to_vec(),
        super::support::NODE_PIN.as_bytes().to_vec(),
        Vec::new(),
    ];
    for record in records {
        key.push(record.id.token().as_bytes().to_vec());
        key.push(record.command.as_bytes().to_vec());
        key.push(vec![record.arity]);
        key.push(record.source.as_bytes().to_vec());
    }
    for (owner, case, draft) in cases(n) {
        let checked = ftoo_manifest::check_expression(n, draft)?;
        let macro_form = render::expression(n, checked.draft(), RenderTarget::Macro, owner, &case)?;
        let unicode = render::expression(n, checked.draft(), RenderTarget::Unicode, owner, &case)?;
        let combined = render::combine(
            &declarations,
            &macro_form.source,
            macro_form.site(owner, &case),
        )?;
        key.push(owner.token().as_bytes().to_vec());
        key.push(case.as_bytes().to_vec());
        for (target, source) in [
            (RenderTarget::Macro, combined),
            (RenderTarget::Unicode, unicode.source.clone()),
        ] {
            requests.push(Request {
                site: rendered_site(owner, &case, &source, unicode.origins.clone()),
                source,
                target,
                collision: None,
            });
        }
        rows.push(SweepRow {
            owner,
            case,
            macro_source: macro_form.source,
            unicode_source: unicode.source,
        });
    }
    for request in &requests {
        key.push(match request.target {
            RenderTarget::Macro => b"macro".to_vec(),
            RenderTarget::Unicode => b"unicode".to_vec(),
        });
        key.push(request.source.as_bytes().to_vec());
    }
    drop(admission_phase);
    Ok(Candidates {
        requests,
        declarations,
        rows,
        key,
    })
}

#[allow(clippy::too_many_lines)] // Keep the complete key and publication boundary together.
pub(super) fn prepared(
    n: &CheckedNotation<'_, '_>,
    support: &Support,
    records: &[Declaration],
    bypass: bool,
) -> Result<(String, Vec<SweepRow>)> {
    with_prepared(n, support, records, bypass, Ok)
}

pub(super) fn with_prepared<T, E>(
    n: &CheckedNotation<'_, '_>,
    support: &Support,
    records: &[Declaration],
    bypass: bool,
    run: impl FnOnce((String, Vec<SweepRow>)) -> std::result::Result<T, E>,
) -> std::result::Result<T, E>
where
    E: From<super::NotationRenderError> + From<super::NotationAdapterFault>,
{
    let candidates = candidates(n, records)?;
    support.scope(
        &super::support::repository().join("tools/notation/check.cjs"),
        || run(prepared_inner(support, records, bypass, candidates)?),
    )
}

#[allow(clippy::too_many_lines)]
fn prepared_inner(
    support: &Support,
    records: &[Declaration],
    bypass: bool,
    candidates: Candidates,
) -> Result<(String, Vec<SweepRow>)> {
    let Candidates {
        requests,
        declarations,
        rows,
        mut key,
    } = candidates;
    let adapter_path = super::support::repository().join("tools/notation/check.cjs");
    let adapter = std::fs::read(&adapter_path).map_err(|e| super::NotationAdapterFault::Io {
        operation: super::AdapterOperation::WaitOutput,
        kind: e.kind(),
    })?;
    key.push(adapter);
    for name in [
        ".node-version",
        "package.json",
        "package-lock.json",
        "support-digests.json",
    ] {
        key.push(
            std::fs::read(
                super::support::repository()
                    .join("tools/notation")
                    .join(name),
            )
            .map_err(|e| super::NotationAdapterFault::Io {
                operation: super::AdapterOperation::WaitOutput,
                kind: e.kind(),
            })?,
        );
    }
    // Actual metadata bytes participate even when the strict checker admits a
    // semantically equivalent installed lock with different object ordering.
    key.push(
        super::support::read(
            &support
                .root
                .join("packages/node_modules/.package-lock.json"),
        )
        .map_err(super::NotationAdapterFault::Integrity)?,
    );
    let private_hit = !bypass && support.has_evidence(&key);
    let reservation = if bypass || private_hit {
        None
    } else {
        Some(MEMO.acquire(MemoKey(key.clone())))
    };
    let hit = private_hit || reservation.as_ref().is_some_and(|r| r.key.is_none());
    let memo_phase = crate::telemetry::Phase::new(if hit {
        "notation.memo_hit"
    } else {
        "notation.memo_miss"
    });
    let result: Result<()> = if hit {
        crate::telemetry::counter("notation.logical_requests", requests.len());
        Ok(())
    } else {
        let sources: Vec<_> = requests.iter().map(|r| r.source.clone()).collect();
        let mut errors = vec![];
        let mut failed_namespace = false;
        for (chunk, sources) in sources.chunks(katex::BATCH_LIMIT).enumerate() {
            let outcomes = katex::batch(support, sources, |index, _| {
                let request = &requests[chunk * katex::BATCH_LIMIT + index];
                if request.collision.is_some() {
                    crate::telemetry::counter("notation.namespace_probes", 1);
                }
            })?;
            for (offset, outcome) in outcomes.into_iter().enumerate() {
                let request = &requests[chunk * katex::BATCH_LIMIT + offset];
                if let Err(message) = outcome {
                    let error = rejected(request, message);
                    if !failed_namespace && let NotationRenderError::Admission(e) = error {
                        errors.extend_from_slice(e.diagnostics());
                    }
                    if chunk * katex::BATCH_LIMIT + offset < records.len() * 2 {
                        failed_namespace = true;
                    }
                }
            }
        }
        match NotationErrors::from_diagnostics(errors) {
            Some(e) => Err(e.into()),
            None => Ok(()),
        }
    };
    result?;
    if !bypass {
        support.remember(key);
    }
    if let Some(reservation) = reservation {
        support.after_close(move || reservation.publish());
    }
    drop(memo_phase);
    Ok((declarations, rows))
}

#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
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
#[cfg(test)]
pub(super) fn namespace(support: &Support, records: &[Declaration], context: &str) -> Result<()> {
    let _phase = crate::telemetry::Phase::new("notation.namespace");
    let mut prefix = context.to_string();
    for record in records {
        let site = declaration_site(record, context.len());
        let probe = render::combine(
            &prefix,
            &format!("\\newcommand{{{}}}[0]{{}}", record.command),
            site.clone(),
        )?;
        let probe_result = katex::invoke(support, Some(&probe));
        if probe_result.is_ok() {
            crate::telemetry::counter("notation.namespace_probes", 1);
        }
        if let Err(message) = probe_result? {
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
#[cfg(test)]
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

#[cfg(test)]
mod memo_tests {
    use super::*;
    use std::sync::{Arc, mpsc};

    #[test]
    fn batch_refusals_keep_the_retained_target_source_and_current_site() {
        let site = NotationSite::Rendered {
            owner: RenderOwner::Definition(DefinitionId::Dimension),
            case: "current case".into(),
            bytes: 11..19,
            origins: vec![NotationOrigin {
                definition: Some(DefinitionId::Dimension),
                path: vec![],
            }],
        };
        for (target, collision, rule, fault) in [
            (
                RenderTarget::Macro,
                None,
                RuleId::M051,
                NotationFault::KaTeXRejected {
                    target: RenderTarget::Macro,
                    source: "current source".into(),
                    message: "full refusal".into(),
                },
            ),
            (
                RenderTarget::Unicode,
                None,
                RuleId::M051,
                NotationFault::KaTeXRejected {
                    target: RenderTarget::Unicode,
                    source: "current source".into(),
                    message: "full refusal".into(),
                },
            ),
            (
                RenderTarget::Macro,
                Some("\\occupied".into()),
                RuleId::M050,
                NotationFault::HostCollision {
                    command: "\\occupied".into(),
                    message: "full refusal".into(),
                },
            ),
        ] {
            let request = Request {
                source: "current source".into(),
                target,
                site: site.clone(),
                collision,
            };
            let expected =
                NotationErrors::from_diagnostics(vec![ftoo_manifest::NotationDiagnostic {
                    rule,
                    stage: NotationStage::Typesetting,
                    site: site.clone(),
                    fault,
                }])
                .unwrap();
            let actual = rejected(&request, "full refusal".into());
            assert_eq!(actual.to_string(), expected.to_string());
            let NotationRenderError::Admission(actual) = actual else {
                panic!("expected complete admission evidence")
            };
            assert_eq!(actual.diagnostics(), expected.diagnostics());
        }
    }

    #[test]
    fn catalogue_status_and_each_owned_key_component_require_new_evidence() {
        let seed = ftoo_manifest::seed().unwrap();
        let mut draft = ftoo_manifest::seed_draft();
        for node in &mut draft.nodes {
            node.maturity = ftoo_manifest::Maturity::Retained;
        }
        let retained = ftoo_manifest::validate(draft).unwrap();
        let mut keys = vec![];
        for graph in [&seed, &retained, &seed] {
            let book = ftoo_manifest::addressing(graph).unwrap();
            let catalogue = ftoo_manifest::notation(&book).unwrap();
            let declarations = render::declarations(&catalogue).unwrap();
            keys.push(MemoKey(candidates(&catalogue, &declarations).unwrap().key));
        }
        assert_eq!(keys[0], keys[2]);
        assert_ne!(keys[0], keys[1]);
        let memo = Memo::new();
        memo.acquire(keys[0].clone()).publish();
        assert_eq!(memo.acquire(keys[1].clone()).key, Some(keys[1].clone()));
        assert_eq!(memo.acquire(keys[2].clone()).key, None);
        for index in [0, 1, 2, 3, 4, 8, keys[0].0.len() - 1] {
            let mut changed = keys[0].clone();
            changed.0[index].push(0);
            assert_eq!(memo.acquire(changed.clone()).key, Some(changed));
            assert_eq!(memo.acquire(keys[0].clone()).key, None);
        }
    }

    #[test]
    fn complete_keys_preserve_boundaries_and_eviction_only_discards_success() {
        let memo = Memo::new();
        let first = MemoKey(vec![b"ab".to_vec(), b"c".to_vec()]);
        let collision = MemoKey(vec![b"a".to_vec(), b"bc".to_vec()]);
        memo.acquire(first.clone()).publish();
        assert_eq!(memo.acquire(first.clone()).key, None);
        let different = memo.acquire(collision.clone());
        assert_eq!(different.key, Some(collision.clone()));
        drop(different);
        assert_eq!(memo.acquire(collision.clone()).key, Some(collision));
        for byte in 0..10 {
            memo.acquire(MemoKey(vec![vec![byte]])).publish();
        }
        assert_eq!(
            memo.state.lock().unwrap().complete,
            (2..10)
                .map(|byte| MemoKey(vec![vec![byte]]))
                .collect::<Vec<_>>()
        );
        assert_eq!(memo.acquire(first.clone()).key, Some(first));
    }

    #[test]
    fn concurrent_waiters_observe_only_complete_success_or_become_the_next_creator() {
        for success in [true, false] {
            let memo = Arc::new(Memo::new());
            let key = MemoKey(vec![b"same owned bytes".to_vec()]);
            let creator = memo.acquire(key.clone());
            assert_eq!(creator.key, Some(key.clone()));
            let (started, ready) = mpsc::channel();
            let (finished, result) = mpsc::channel();
            let waiter_memo = Arc::clone(&memo);
            let waiter_key = key.clone();
            let waiter = std::thread::spawn(move || {
                started.send(()).unwrap();
                let reservation = waiter_memo.acquire(waiter_key);
                finished.send(reservation.key.clone()).unwrap();
                reservation.publish();
            });
            ready.recv().unwrap();
            assert_eq!(result.try_recv(), Err(mpsc::TryRecvError::Empty));
            if success {
                creator.publish();
            } else {
                drop(creator);
            }
            assert_eq!(
                result.recv().unwrap(),
                if success { None } else { Some(key.clone()) }
            );
            waiter.join().unwrap();
            assert_eq!(memo.acquire(key.clone()).key, None);
            assert_eq!(memo.state.lock().unwrap().complete, vec![key]);
            assert_eq!(memo.state.lock().unwrap().active, vec![]);
        }
    }
}
