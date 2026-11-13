use super::{
    FormatPhase,
    diagnostics::{
        AssemblyFault, DocumentError, NativeAdapterFault, NativeCase, NativeOperation,
        NativeProtocolReason,
    },
    failure,
    native_format::{self, Seed},
    native_support::{self, Lock, NativeSupport},
};
use ftoo_manifest::{AssemblySite, AssemblyStage, RuleId, SourceOrigin};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
#[derive(Clone, Debug)]
pub(super) struct Evidence {
    pub scratch: std::path::PathBuf,
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub log: Vec<u8>,
    pub pdf_bytes: Option<u64>,
}
#[cfg(test)]
impl Evidence {
    fn new(scratch: &Path, output: &std::process::Output, log: &[u8]) -> Self {
        Self {
            scratch: scratch.into(),
            status: output.status.code(),
            stdout: output.stdout.clone(),
            stderr: output.stderr.clone(),
            log: log.into(),
            pdf_bytes: fs::metadata(scratch.join("output/document.pdf"))
                .ok()
                .map(|value| value.len()),
        }
    }
}
struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Process(std::process::Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) struct SweepOrigin {
    pub case: NativeCase,
    pub origin: SourceOrigin,
    pub bytes: std::ops::Range<usize>,
}

pub(super) fn sweep_row(
    source: &mut String,
    owner: ftoo_manifest::RenderOwner,
    case: &str,
    expression: &str,
) -> SweepOrigin {
    use std::fmt::Write;
    let start = source.len();
    writeln!(source, "% sweep {owner:?} {case}").expect("string write");
    writeln!(source, "\\({expression}\\)\\par").expect("string write");
    let bytes = start..source.len();
    SweepOrigin {
        case: NativeCase::Sweep {
            owner,
            case: case.into(),
        },
        origin: SourceOrigin {
            unit: None,
            language: None,
            target: Some(ftoo_manifest::DocumentTarget::Latex),
            site: ftoo_manifest::UnitSite {
                file: "assembly-sweep.tex".into(),
                path: vec![
                    ftoo_manifest::PathPart::Key("sweep".into()),
                    ftoo_manifest::PathPart::Key(owner.token().into()),
                    ftoo_manifest::PathPart::Key(case.into()),
                ],
                anchor: ftoo_manifest::SourceAnchor::Bytes(bytes.start, bytes.end),
                decoded: None,
                occurrence: None,
            },
        },
        bytes,
    }
}

pub(super) fn sweep_origin(
    error: DocumentError,
    source: &str,
    rows: &[SweepOrigin],
) -> DocumentError {
    let DocumentError::Assembly(errors) = error else {
        return error;
    };
    let diagnostics = errors
        .diagnostics()
        .iter()
        .cloned()
        .map(|mut diagnostic| {
            if let AssemblyFault::NativeRejected {
                case, stderr, log, ..
            } = &mut diagnostic.fault
                && matches!(case, NativeCase::Sweep { .. })
                && let Some(range) = if has_overfull(log, stderr) {
                    overfull_range(source, log, stderr)
                } else {
                    failure_offset(source, stderr).map(|offset| offset..offset + 1)
                }
            {
                let selected: Vec<_> = rows
                    .iter()
                    .filter(|row| row.bytes.start < range.end && range.start < row.bytes.end)
                    .collect();
                if let (Some(first), Some(last)) = (selected.first(), selected.last()) {
                    if selected.len() == 1 {
                        *case = first.case.clone();
                    }
                    diagnostic.site = AssemblySite::Generated {
                        path: "assembly-sweep.tex".into(),
                        bytes: first.bytes.start..last.bytes.end,
                        origins: selected.iter().map(|row| row.origin.clone()).collect(),
                    };
                }
            }
            diagnostic
        })
        .collect();
    DocumentError::Assembly(
        super::AssemblyErrors::from_diagnostics(diagnostics).expect("nonempty diagnostics"),
    )
}

fn failure_offset(source: &str, stderr: &[u8]) -> Option<usize> {
    // Use the engine's explicit filename, never an unqualified log line that
    // could instead refer to an included package or notation declaration.
    let error = std::str::from_utf8(stderr)
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("error: "))?;
    let (line, _) = error.strip_prefix("document.tex:")?.split_once(':')?;
    let line = decimal(line)?.checked_sub(1)?;
    let mut offset = 0;
    for (index, text) in source.split_inclusive('\n').enumerate() {
        if index == line {
            return Some(offset);
        }
        offset += text.len();
    }
    None
}
pub(super) fn consume_plan(
    support: &NativeSupport,
    lock: &Lock,
    source: &str,
    notation: &str,
    addressing: &str,
    plan: &super::plan::DocumentPlan,
) -> Result<(), DocumentError> {
    consume(
        support,
        lock,
        source,
        notation,
        addressing,
        NativeCase::Document {
            document: plan.document.clone(),
            language: plan.language.clone(),
        },
        plan.origins(ftoo_manifest::DocumentTarget::Latex),
    )
    .map_err(|error| overfull_origin(runtime_origin(error, plan), source, plan))
}
fn overfull_origin(
    error: DocumentError,
    source: &str,
    plan: &super::plan::DocumentPlan,
) -> DocumentError {
    let DocumentError::Assembly(errors) = error else {
        return error;
    };
    let diagnostics = errors
        .diagnostics()
        .iter()
        .cloned()
        .map(|mut diagnostic| {
            if let AssemblyFault::NativeRejected { stderr, log, .. } = &diagnostic.fault
                && let Some(range) = overfull_range(source, log, stderr)
            {
                let blocks: Vec<_> = plan
                    .blocks
                    .iter()
                    .filter(|block| {
                        let marker = format!(
                            "% unit {} document {} language {}\n",
                            block.unit, plan.document, plan.language
                        );
                        source.find(&marker).is_some_and(|start| {
                            let end = source[start + marker.len()..]
                                .find("% unit ")
                                .map_or(source.len(), |n| start + marker.len() + n);
                            start < range.end && range.start < end
                        })
                    })
                    .collect();
                if !blocks.is_empty()
                    && let AssemblySite::Generated { origins, .. } = &mut diagnostic.site
                {
                    origins.retain(|origin| {
                        blocks
                            .iter()
                            .any(|block| origin.unit.as_ref() == Some(&block.unit))
                    });
                    if blocks.len() == 1 {
                        diagnostic.unit = Some(blocks[0].unit.clone());
                    }
                }
            }
            diagnostic
        })
        .collect();
    DocumentError::Assembly(
        super::AssemblyErrors::from_diagnostics(diagnostics).expect("nonempty diagnostics"),
    )
}
pub(super) fn runtime_origin(
    error: DocumentError,
    plan: &super::plan::DocumentPlan,
) -> DocumentError {
    let DocumentError::Assembly(errors) = error else {
        return error;
    };
    let diagnostics = errors
        .diagnostics()
        .iter()
        .cloned()
        .map(|mut diagnostic| {
            if let AssemblyFault::RuntimeTagRejected { site_token, .. } = &diagnostic.fault {
                for run in &plan.coverage.runs {
                    if let super::plan::Inline::Tag { token, site, .. } = run
                        && token == site_token
                    {
                        diagnostic.unit = None;
                        if let AssemblySite::Generated { origins, .. } = &mut diagnostic.site {
                            *origins = vec![SourceOrigin {
                                unit: None,
                                language: Some(plan.language.clone()),
                                target: Some(ftoo_manifest::DocumentTarget::Latex),
                                site: site.clone(),
                            }];
                        }
                    }
                }
                for block in &plan.blocks {
                    for ((target, definition), names) in &block.pending_names {
                        if *target != ftoo_manifest::DocumentTarget::Latex
                            || !names.iter().any(|name| name.token == *site_token)
                        {
                            continue;
                        }
                        diagnostic.unit = Some(block.unit.clone());
                        if let AssemblySite::Generated { origins, .. } = &mut diagnostic.site {
                            *origins =
                                block.pending_origins(*definition, &plan.language, Some(*target));
                        }
                    }
                    for run in block.target_runs(ftoo_manifest::DocumentTarget::Latex) {
                        if let super::plan::Inline::Citation { identity, site, .. } = run
                            && identity.token == *site_token
                        {
                            diagnostic.unit = Some(block.unit.clone());
                            if let AssemblySite::Generated { origins, .. } = &mut diagnostic.site {
                                *origins = super::plan::citation_origins(
                                    identity,
                                    site,
                                    &block.unit,
                                    &plan.language,
                                );
                            }
                        }
                        if let super::plan::Inline::Tag { token, site, .. } = run
                            && token == site_token
                        {
                            diagnostic.unit = Some(block.unit.clone());
                            if let AssemblySite::Generated { origins, .. } = &mut diagnostic.site {
                                *origins = vec![SourceOrigin {
                                    unit: Some(block.unit.clone()),
                                    language: Some(plan.language.clone()),
                                    target: Some(ftoo_manifest::DocumentTarget::Latex),
                                    site: site.clone(),
                                }];
                            }
                        }
                    }
                }
            }
            diagnostic
        })
        .collect();
    DocumentError::Assembly(
        super::AssemblyErrors::from_diagnostics(diagnostics).expect("nonempty diagnostics"),
    )
}
pub(super) fn tag_range(source: &str, token: &str) -> Option<std::ops::Range<usize>> {
    for (start, _) in source.match_indices("\\FtdocTag{") {
        let mut at = start + "\\FtdocTag".len();
        let mut arguments = Vec::new();
        for _ in 0..2 {
            if source.as_bytes().get(at) != Some(&b'{') {
                break;
            }
            let begin = at + 1;
            let mut depth = 1;
            at += 1;
            while let Some(&byte) = source.as_bytes().get(at) {
                at += 1;
                match byte {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    break;
                }
            }
            if depth != 0 {
                break;
            }
            arguments.push(&source[begin..at - 1]);
        }
        if arguments.len() == 2 && arguments[1] == token {
            return Some(start..at);
        }
    }
    None
}
pub(super) fn sweep_inventory(
    catalogue: &ftoo_manifest::CheckedNotation<'_, '_>,
    actual: &[NativeCase],
) -> Result<(), DocumentError> {
    use ftoo_manifest::{DefinitionKind, RenderOwner};
    let mut expected = Vec::new();
    for definition in catalogue.definitions() {
        let owner = RenderOwner::Definition(definition.spec().id());
        expected.push(NativeCase::Sweep {
            owner,
            case: "base".into(),
        });
        let arity = definition.spec().signature().arity;
        if arity > 0 {
            expected.push(NativeCase::Sweep {
                owner,
                case: "compound".into(),
            });
        }
        for slot in 0..arity {
            for suffix in ["alpha", "pending"] {
                expected.push(NativeCase::Sweep {
                    owner,
                    case: format!("slot-{slot}-{suffix}"),
                });
            }
            for inner in catalogue
                .definitions()
                .iter()
                .filter(|d| matches!(d.spec().kind(), DefinitionKind::Constructor { .. }))
            {
                expected.push(NativeCase::Sweep {
                    owner,
                    case: format!("slot-{slot}-nested-{}", inner.spec().token()),
                });
            }
        }
    }
    for primitive in catalogue.primitives() {
        let owner = RenderOwner::Primitive(primitive.id());
        expected.push(NativeCase::Sweep {
            owner,
            case: "base".into(),
        });
        for slot in 0..primitive.signature().arity {
            for inner in catalogue.primitives() {
                expected.push(NativeCase::Sweep {
                    owner,
                    case: format!("argument-{slot}-primitive-{}", inner.token()),
                });
            }
        }
    }
    expected.sort_by_key(|case| match case {
        NativeCase::Sweep { owner, case } => (owner.token(), case.clone()),
        _ => unreachable!("sweep only"),
    });
    let missing = expected
        .iter()
        .filter(|case| !actual.contains(case))
        .cloned()
        .collect::<Vec<_>>();
    let repeated = expected
        .iter()
        .filter_map(|case| {
            let count = actual.iter().filter(|value| *value == case).count();
            (count > 1).then(|| (case.clone(), count))
        })
        .collect::<Vec<_>>();
    let unexpected = actual
        .iter()
        .filter(|case| !expected.contains(case))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() && repeated.is_empty() && unexpected.is_empty() {
        return Ok(());
    }
    Err(failure(
        RuleId::M056,
        AssemblyStage::Native,
        AssemblySite::Inventory {
            path: "assembly-sweep.tex".into(),
        },
        AssemblyFault::NativeInventory {
            missing,
            repeated,
            unexpected,
        },
    ))
}
pub(super) fn consume(
    support: &NativeSupport,
    lock: &Lock,
    source: &str,
    notation: &str,
    addressing: &str,
    case: NativeCase,
    origins: Vec<SourceOrigin>,
) -> Result<(), DocumentError> {
    consume_observed(
        support,
        lock,
        source,
        notation,
        addressing,
        case,
        origins,
        false,
        |_| {},
        |_, _| {},
    )
}
#[allow(clippy::too_many_arguments)] // Private injection point observes the same invocation.
#[cfg(test)]
pub(super) fn consume_with(
    support: &NativeSupport,
    lock: &Lock,
    source: &str,
    notation: &str,
    addressing: &str,
    case: NativeCase,
    origins: Vec<SourceOrigin>,
    before: impl FnOnce(&Path),
    completed: impl FnOnce(&Path, &Result<(), DocumentError>),
) -> Result<(), DocumentError> {
    consume_observed(
        support, lock, source, notation, addressing, case, origins, true, before, completed,
    )
}
#[allow(clippy::too_many_arguments)] // The explicit seam invalidates a shared boundary observation.
fn consume_observed(
    support: &NativeSupport,
    lock: &Lock,
    source: &str,
    notation: &str,
    addressing: &str,
    case: NativeCase,
    origins: Vec<SourceOrigin>,
    injected: bool,
    before: impl FnOnce(&Path),
    completed: impl FnOnce(&Path, &Result<(), DocumentError>),
) -> Result<(), DocumentError> {
    support.scope(lock, || {
        support.ensure_open(lock)?;
        if injected {
            support.invalidate_observation();
        }
        let scratch = support.root.join("tmp").join(format!(
            "document-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&scratch)
            .map_err(|e| native_support::io_error(NativeOperation::WriteInput, &scratch, &e))?;
        let guard = Scratch(scratch.clone());
        before(&scratch);
        let mut seed = None;
        let result = invoke(
            support, lock, &scratch, source, notation, addressing, case, origins, &mut seed,
        );
        completed(&scratch, &result);
        let checked = support.after_consumer(lock);
        let format = seed.map_or(Ok(()), |seed: std::sync::Arc<Seed>| {
            seed.check(&scratch.join("cache"), FormatPhase::AfterCase)
                .map_err(|fault| support.poison(fault))
        });
        let cleanup_phase = crate::telemetry::Phase::new("native.cleanup");
        let cleanup = fs::remove_dir_all(&scratch)
            .map_err(|e| native_support::io_error(NativeOperation::WriteInput, &scratch, &e));
        drop(guard);
        drop(cleanup_phase);
        checked?;
        format?;
        cleanup?;
        result
    })
}
#[allow(clippy::too_many_arguments)] // Lock belongs to the invocation's owning bracket.
fn invoke(
    support: &NativeSupport,
    lock: &Lock,
    scratch: &Path,
    source: &str,
    notation: &str,
    addressing: &str,
    case: NativeCase,
    origins: Vec<SourceOrigin>,
    seed: &mut Option<std::sync::Arc<Seed>>,
) -> Result<(), DocumentError> {
    for name in ["output", "cache", "config", "bundle-cache"] {
        let path = scratch.join(name);
        fs::create_dir(&path)
            .map_err(|e| native_support::io_error(NativeOperation::WriteInput, &path, &e))?;
    }
    for (name, bytes) in [
        ("document.tex", source),
        ("notation.tex", notation),
        ("addressing.tex", addressing),
    ] {
        let path = scratch.join(name);
        fs::write(&path, bytes)
            .map_err(|e| native_support::io_error(NativeOperation::WriteInput, &path, &e))?;
    }
    let executable = support.root.join("runtime/bin/tectonic");
    #[cfg(test)]
    let reuse = !support.cold_format.get();
    #[cfg(not(test))]
    let reuse = true;
    if reuse {
        let format = support.format_seed(lock, native_format::BOOTSTRAP)?;
        format
            .copy(&scratch.join("cache"))
            .map_err(|fault| support.poison(fault))?;
        crate::telemetry::counter("native.format_copies", 1);
        *seed = Some(format);
    }
    support.before_consumer(lock)?;
    let invocation = crate::telemetry::Phase::new(match &case {
        NativeCase::Document { .. } => "native.document",
        NativeCase::Sweep { .. } => "native.sweep",
        NativeCase::Runtime { .. } => "native.runtime",
        NativeCase::Collision { .. } => "native.collision",
    });
    let mut invocation_command = command(support, scratch);
    let output = measured_output(
        &mut invocation_command.command,
        &invocation_command.writable,
    )
    .map_err(|e| support.spawn_error(&executable, &e))?;
    drop(invocation);
    let path = scratch.join("output/document.log");
    let log = match fs::read(&path) {
        Ok(log) => log,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            return Err(native_support::io_error(NativeOperation::ReadOutput, &path, &e).into());
        }
    };
    let path = match &case {
        NativeCase::Document { document, language } => {
            format!("generated/documents/{document}/{language}.tex")
        }
        NativeCase::Sweep { .. } => "assembly-sweep.tex".into(),
        NativeCase::Runtime { .. } | NativeCase::Collision { .. } => "document.tex".into(),
    };
    let bytes = if has_overfull(&log, &output.stderr) {
        overfull_range(source, &log, &output.stderr)
    } else {
        decode(&log).and_then(|(_, token, _)| tag_range(source, &token))
    }
    .unwrap_or(0..source.len());
    let site = AssemblySite::Generated {
        path,
        bytes,
        origins,
    };
    #[cfg(test)]
    if let Some(evidence) = support.evidence.borrow_mut().as_mut() {
        evidence.push(Evidence::new(scratch, &output, &log));
    }
    classify(
        output,
        log,
        site,
        case,
        scratch.join("output/document.pdf").is_file(),
    )
    .map_err(|error| runtime_ranges(error, source))
}

fn runtime_ranges(error: DocumentError, source: &str) -> DocumentError {
    let DocumentError::Assembly(errors) = error else {
        return error;
    };
    let diagnostics = errors
        .diagnostics()
        .iter()
        .cloned()
        .map(|mut diagnostic| {
            let range = match &diagnostic.fault {
                AssemblyFault::RuntimeTagRejected { site_token, .. } => {
                    Some(tag_range(source, site_token).unwrap_or(0..source.len()))
                }
                AssemblyFault::InvalidRuntimeRecord { .. } => Some(0..source.len()),
                _ => None,
            };
            if let (Some(range), AssemblySite::Generated { bytes, .. }) =
                (range, &mut diagnostic.site)
            {
                *bytes = range;
            }
            diagnostic
        })
        .collect();
    DocumentError::Assembly(
        super::AssemblyErrors::from_diagnostics(diagnostics).expect("native diagnostics"),
    )
}

pub(super) fn bootstrap(
    support: &NativeSupport,
    lock: &Lock,
    source: &str,
) -> Result<Seed, DocumentError> {
    let phase = crate::telemetry::Phase::new("native.format_bootstrap");
    let scratch = support.root.join("tmp").join(format!(
        "format-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&scratch)
        .map_err(|e| native_support::io_error(NativeOperation::WriteInput, &scratch, &e))?;
    let guard = Scratch(scratch.clone());
    let result = (|| {
        for name in ["output", "cache", "config", "bundle-cache"] {
            let path = scratch.join(name);
            fs::create_dir(&path)
                .map_err(|e| native_support::io_error(NativeOperation::WriteInput, &path, &e))?;
        }
        let input = scratch.join("document.tex");
        fs::write(&input, source)
            .map_err(|e| native_support::io_error(NativeOperation::WriteInput, &input, &e))?;
        support.before_consumer(lock)?;
        let mut invocation_command = command(support, &scratch);
        let output = measured_output(
            &mut invocation_command.command,
            &invocation_command.writable,
        )
        .map_err(|e| support.spawn_error(&support.root.join("runtime/bin/tectonic"), &e));
        support.after_consumer(lock)?;
        let output = output?;
        let path = scratch.join("output/document.log");
        let log = match fs::read(&path) {
            Ok(log) => log,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => vec![],
            Err(error) => {
                return Err(
                    native_support::io_error(NativeOperation::ReadOutput, &path, &error).into(),
                );
            }
        };
        #[cfg(test)]
        {
            *support.bootstrap_evidence.borrow_mut() = Some(Evidence::new(&scratch, &output, &log));
        }
        let reason =
            if !output.status.success() || log.split(|byte| *byte == b'\n').any(lost_character) {
                Some(NativeProtocolReason::UnexpectedStatus)
            } else if !scratch.join("output/document.pdf").is_file() {
                Some(NativeProtocolReason::MissingOutput)
            } else {
                None
            };
        if let Some(reason) = reason {
            return Err(NativeAdapterFault::Protocol {
                reason,
                status: output.status.code(),
                stdout: output.stdout,
                stderr: output.stderr,
                log,
            }
            .into());
        }
        Seed::capture(&scratch.join("cache"), &lock.bundle_digest)
            .map_err(|fault| support.poison(fault).into())
    })();
    drop(guard);
    phase.finish(result.is_ok());
    result
}

pub(super) fn measured_output(
    command: &mut Command,
    directories: &[PathBuf],
) -> std::io::Result<std::process::Output> {
    use std::io::{BufRead, BufReader, Read};
    use std::process::Stdio;
    let _consumer = crate::support_bracket::consumer_started();
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let writable: Vec<_> = directories
        .iter()
        .map(std::path::PathBuf::as_path)
        .collect();
    let mut child = crate::support_bracket::confinement::spawn_measured(
        command,
        &writable,
        "native.confinement.setup",
    )?;
    let _started = crate::telemetry::Phase::new("native.process_started");
    let stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    std::thread::scope(|scope| {
        let mut child = Process(child);
        let stdout = scope.spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut bytes = vec![];
            let mut line = vec![];
            let mut format = None;
            while reader.read_until(b'\n', &mut line)? != 0 {
                if line == b"note: generating format \"latex\"\n" {
                    format = Some(crate::telemetry::Phase::new("native.format_build"));
                } else if line == b"note: Running TeX ...\n"
                    && let Some(format) = format.take()
                {
                    format.finish(true);
                }
                bytes.extend_from_slice(&line);
                line.clear();
            }
            Ok::<_, std::io::Error>(bytes)
        });
        let stderr = scope.spawn(move || {
            let mut bytes = vec![];
            stderr.read_to_end(&mut bytes)?;
            Ok::<_, std::io::Error>(bytes)
        });
        let status = child.0.wait();
        let stdout = stdout.join().expect("native stdout reader");
        let stderr = stderr.join().expect("native stderr reader");
        Ok(std::process::Output {
            status: status?,
            stdout: stdout?,
            stderr: stderr?,
        })
    })
}
pub(super) fn classify(
    output: std::process::Output,
    log: Vec<u8>,
    site: AssemblySite,
    case: NativeCase,
    pdf_exists: bool,
) -> Result<(), DocumentError> {
    let overfull = has_overfull(&log, &output.stderr);
    let mut diagnostics = Vec::new();
    let mut append = |error| {
        let DocumentError::Assembly(errors) = error else {
            unreachable!("located assembly fault")
        };
        diagnostics.extend_from_slice(errors.diagnostics());
    };
    if log
        .split(|byte| *byte == b'\n')
        .any(|line| line.starts_with(b"FTDOC-"))
        || (matches!(&case, NativeCase::Runtime { .. }) && !output.status.success())
    {
        if let Some((input, site_token, message)) = decode(&log) {
            let error = located_failure(
                RuleId::M055,
                site.clone(),
                &case,
                AssemblyFault::RuntimeTagRejected {
                    input,
                    site_token,
                    message,
                },
            );
            if !overfull {
                return Err(error);
            }
            append(error);
        } else {
            if !overfull {
                return Err(DocumentError::Native(NativeAdapterFault::Protocol {
                    reason: NativeProtocolReason::InvalidRuntimeRecord,
                    status: output.status.code(),
                    stdout: output.stdout,
                    stderr: output.stderr,
                    log,
                }));
            }
            append(located_failure(
                RuleId::M055,
                site.clone(),
                &case,
                AssemblyFault::InvalidRuntimeRecord {
                    status: output.status.code(),
                    stdout: output.stdout.clone(),
                    stderr: output.stderr.clone(),
                    log: log.clone(),
                },
            ));
        }
    }
    let lost = log.split(|b| *b == b'\n').any(lost_character);
    if !output.status.success() || lost || overfull {
        let context = case.clone();
        append(located_failure(
            RuleId::M056,
            site,
            &context,
            AssemblyFault::NativeRejected {
                case,
                status: output.status.code(),
                stdout: output.stdout,
                stderr: output.stderr,
                log,
            },
        ));
        return Err(DocumentError::Assembly(
            super::AssemblyErrors::from_diagnostics(diagnostics).expect("native refusal"),
        ));
    }
    if !pdf_exists {
        return Err(DocumentError::Native(NativeAdapterFault::Protocol {
            reason: NativeProtocolReason::MissingOutput,
            status: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
            log,
        }));
    }
    Ok(())
}
fn located_failure(
    rule: RuleId,
    site: AssemblySite,
    case: &NativeCase,
    fault: AssemblyFault,
) -> DocumentError {
    let (document, language, target) = match case {
        NativeCase::Document { document, language } => (
            Some(document.clone()),
            Some(language.clone()),
            Some(ftoo_manifest::DocumentTarget::Latex),
        ),
        _ => (None, None, None),
    };
    DocumentError::Assembly(
        super::AssemblyErrors::from_diagnostics(vec![super::AssemblyDiagnostic {
            rule,
            stage: AssemblyStage::Native,
            document,
            unit: None,
            language,
            target,
            site,
            fault,
        }])
        .expect("one diagnostic"),
    )
}
pub(super) fn lost_character(line: &[u8]) -> bool {
    line.starts_with(b"Missing character:") || line.starts_with(b"! Missing character:")
}
fn box_prefix(line: &[u8]) -> bool {
    line.starts_with(b"Overfull \\hbox (") || line.starts_with(b"Overfull \\vbox (")
}
type BoxWarning<'a> = (&'a [u8], Option<(&'a str, usize)>);
fn box_warning(line: &[u8]) -> Option<BoxWarning<'_>> {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let warning = line.strip_prefix(b"warning: ")?;
    if box_prefix(warning) {
        return Some((warning, None));
    }
    let split = warning.windows(2).position(|pair| pair == b": ")?;
    let location = std::str::from_utf8(&warning[..split]).ok()?;
    let (file, line) = location.rsplit_once(':')?;
    if line.is_empty() || !line.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let diagnostic = &warning[split + 2..];
    (!file.is_empty() && box_prefix(diagnostic))
        .then_some((diagnostic, line.parse().ok().map(|line| (file, line))))
}
pub(super) fn has_overfull(log: &[u8], stderr: &[u8]) -> bool {
    log.split(|b| *b == b'\n').any(box_prefix)
        || stderr
            .split(|b| *b == b'\n')
            .any(|line| box_warning(line).is_some())
}
pub(super) fn overfull_range(
    source: &str,
    log: &[u8],
    stderr: &[u8],
) -> Option<std::ops::Range<usize>> {
    let warnings: Vec<_> = stderr
        .split(|byte| *byte == b'\n')
        .filter_map(box_warning)
        .collect();
    // A log-only box has no reliable input-file attribution. Do not borrow the
    // location of a different box merely because both occur in one invocation.
    let mut lines = log
        .split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line));
    while let Some(line) = lines.next() {
        if !box_prefix(line) {
            continue;
        }
        let mut diagnostic = line.to_vec();
        let mut width = line.len();
        // The pinned engine wraps its log at 79 columns. Evidence is retained
        // verbatim; only matching a located mirror joins these display lines.
        while width == 79 {
            let Some(continuation) = lines.next() else {
                break;
            };
            diagnostic.extend_from_slice(continuation);
            width = continuation.len();
        }
        if !warnings
            .iter()
            .any(|(warning, location)| *warning == diagnostic.as_slice() && location.is_some())
        {
            return None;
        }
    }
    let mut result: Option<std::ops::Range<usize>> = None;
    for (diagnostic, location) in warnings {
        let Some(("document.tex", line)) = location else {
            return None;
        };
        let paragraph = std::str::from_utf8(diagnostic)
            .ok()
            .and_then(|s| s.split_once(" at lines "))
            .and_then(|(_, s)| s.split_once("--"))
            .and_then(|(a, b)| Some((decimal(a)?, decimal(b)?)));
        let (first, last) = paragraph.unwrap_or((line, line));
        if first == 0 || first > last {
            return None;
        }
        let mut offset = 0;
        let mut start = None;
        let mut end = None;
        for (index, text) in source.split_inclusive('\n').enumerate() {
            if index + 1 == first {
                start = Some(offset);
            }
            offset += text.len();
            if index + 1 == last {
                end = Some(offset);
                break;
            }
        }
        let (Some(start), Some(end)) = (start, end) else {
            return None;
        };
        result = Some(result.map_or(start..end, |old| old.start.min(start)..old.end.max(end)));
    }
    result
}

fn decimal(value: &str) -> Option<usize> {
    let n = value.parse::<usize>().ok()?;
    (n.to_string() == value).then_some(n)
}
pub(super) fn decode(log: &[u8]) -> Option<(String, String, String)> {
    let text = std::str::from_utf8(log).ok()?;
    let lines = text
        .split_inclusive('\n')
        .filter(|line| line.starts_with("FTDOC-"))
        .map(|line| line.strip_suffix('\n'))
        .collect::<Option<Vec<_>>>()?;
    let mut at = 0;
    let mut values = Vec::new();
    for field in ['i', 's', 'm'] {
        let mut bytes = Vec::new();
        let mut count = None;
        let mut ordinal = 0;
        loop {
            let fields: Vec<_> = lines.get(at)?.split(':').collect();
            if fields.len() != 5
                || fields[0] != "FTDOC-R"
                || fields[1] != field.to_string()
                || decimal(fields[2])? != ordinal
            {
                return None;
            }
            let total = decimal(fields[3])?;
            if total == 0 || count.is_some_and(|c| c != total) {
                return None;
            }
            count = Some(total);
            let hex = fields[4];
            if hex.len() > 32
                || hex.len() % 2 != 0
                || (ordinal + 1 < total && hex.len() != 32)
                || (hex.is_empty() && total != 1)
            {
                return None;
            }
            for pair in hex.as_bytes().as_chunks::<2>().0 {
                if !pair
                    .iter()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
                {
                    return None;
                }
                bytes.push(u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?);
            }
            at += 1;
            ordinal += 1;
            if ordinal == total {
                break;
            }
        }
        values.push(String::from_utf8(bytes).ok()?);
    }
    if lines.get(at) != Some(&"FTDOC-END") || at + 1 != lines.len() {
        return None;
    }
    let mut values = values.into_iter();
    Some((values.next()?, values.next()?, values.next()?))
}

pub(super) struct Invocation {
    command: Command,
    writable: Vec<PathBuf>,
}
impl std::ops::Deref for Invocation {
    type Target = Command;
    fn deref(&self) -> &Command {
        &self.command
    }
}

pub(super) fn command(support: &NativeSupport, scratch: &Path) -> Invocation {
    let executable = support.root.join("runtime/bin/tectonic");
    let mut command = Command::new(executable);

    command
        .current_dir(scratch)
        .env_clear()
        .env("HOME", scratch)
        .env("LANG", "C.UTF-8")
        .env("TECTONIC_CACHE_DIR", scratch.join("bundle-cache"))
        .env("XDG_CACHE_HOME", scratch.join("cache"))
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .arg("--bundle")
        .arg(support.root.join("bundle"))
        .arg("--only-cached")
        .arg("--untrusted")
        .arg("--keep-logs")
        .arg("--color")
        .arg("never")
        .arg("--outdir")
        .arg(scratch.join("output"))
        .arg(scratch.join("document.tex"));
    Invocation {
        command,
        writable: std::iter::once(scratch.to_owned())
            .chain(["output", "cache", "config", "bundle-cache"].map(|name| scratch.join(name)))
            .collect(),
    }
}
