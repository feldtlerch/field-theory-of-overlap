use super::{
    diagnostics::{
        AssemblyFault, DocumentError, NativeAdapterFault, NativeCase, NativeOperation,
        NativeProtocolReason,
    },
    failure,
    native_support::{self, Lock, NativeSupport},
};
use ftoo_manifest::{AssemblySite, AssemblyStage, RuleId, SourceOrigin};
use std::{
    fs,
    path::Path,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

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
            if let AssemblyFault::NativeRejected { case, stderr, .. } = &mut diagnostic.fault
                && matches!(case, NativeCase::Sweep { .. })
                && let Some(offset) = failure_offset(source, stderr)
                && let Some(row) = rows.iter().find(|row| row.bytes.contains(&offset))
            {
                *case = row.case.clone();
                diagnostic.site = AssemblySite::Generated {
                    path: "assembly-sweep.tex".into(),
                    bytes: row.bytes.clone(),
                    origins: vec![row.origin.clone()],
                };
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
    .map_err(|error| runtime_origin(error, plan))
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
                for block in &plan.blocks {
                    for run in block.target_runs(ftoo_manifest::DocumentTarget::Latex) {
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
    consume_with(
        support,
        lock,
        source,
        notation,
        addressing,
        case,
        origins,
        |_| {},
        |_, _| {},
    )
}
#[allow(clippy::too_many_arguments)] // Private injection point observes the same invocation.
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
    native_support::verify(support, lock)?;
    let scratch = support.root.join("tmp").join(format!(
        "document-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&scratch)
        .map_err(|e| native_support::io_error(NativeOperation::WriteInput, &scratch, &e))?;
    before(&scratch);
    let result = invoke(
        support, &scratch, source, notation, addressing, case, origins,
    );
    completed(&scratch, &result);
    let checked = native_support::verify(support, lock);
    let cleanup_phase = crate::telemetry::Phase::new("native.cleanup");
    let cleanup = fs::remove_dir_all(&scratch)
        .map_err(|e| native_support::io_error(NativeOperation::WriteInput, &scratch, &e));
    drop(cleanup_phase);
    checked?;
    cleanup?;
    result
}
fn invoke(
    support: &NativeSupport,
    scratch: &Path,
    source: &str,
    notation: &str,
    addressing: &str,
    case: NativeCase,
    origins: Vec<SourceOrigin>,
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
    let invocation = crate::telemetry::Phase::new(match &case {
        NativeCase::Document { .. } => "native.document",
        NativeCase::Sweep { .. } => "native.sweep",
        NativeCase::Runtime { .. } => "native.runtime",
        NativeCase::Collision { .. } => "native.collision",
    });
    let output = measured_output(&mut command(support, scratch))
        .map_err(|e| native_support::io_error(NativeOperation::Spawn, &executable, &e))?;
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
    let bytes = decode(&log)
        .and_then(|(_, token, _)| tag_range(source, &token))
        .unwrap_or(0..source.len());
    let site = AssemblySite::Generated {
        path,
        bytes,
        origins,
    };
    classify(
        output,
        log,
        site,
        case,
        scratch.join("output/document.pdf").is_file(),
    )
}

fn measured_output(command: &mut Command) -> std::io::Result<std::process::Output> {
    use std::io::{BufRead, BufReader, Read};
    use std::process::Stdio;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let _started = crate::telemetry::Phase::new("native.process_started");
    let stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    std::thread::scope(|scope| {
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
        let status = child.wait();
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
    if log
        .split(|byte| *byte == b'\n')
        .any(|line| line.starts_with(b"FTDOC-"))
        || (matches!(&case, NativeCase::Runtime { .. }) && !output.status.success())
    {
        let record = decode(&log);
        if let Some((input, site_token, message)) = record {
            return Err(located_failure(
                RuleId::M055,
                site,
                &case,
                AssemblyFault::RuntimeTagRejected {
                    input,
                    site_token,
                    message,
                },
            ));
        }
        return Err(DocumentError::Native(NativeAdapterFault::Protocol {
            reason: NativeProtocolReason::InvalidRuntimeRecord,
            status: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
            log,
        }));
    }
    let lost = log.split(|b| *b == b'\n').any(lost_character);
    if !output.status.success() || lost {
        let context = case.clone();
        return Err(located_failure(
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

pub(super) fn command(support: &NativeSupport, scratch: &Path) -> Command {
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
    command
}
