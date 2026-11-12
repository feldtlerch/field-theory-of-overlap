use super::{
    AdapterComponent, AdapterOperation, NotationAdapterFault, ProtocolReason,
    support::{self, Support},
};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use super::BatchReason;

pub(super) const BATCH_LIMIT: usize = 4096;
static NEXT_SPOOL: AtomicU64 = AtomicU64::new(0);

pub(super) struct ChildGuard(pub(super) std::process::Child);
impl std::ops::Deref for ChildGuard {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ChildGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[allow(clippy::needless_pass_by_value)] // Used directly as the owning map_err conversion.
fn batch_io(error: io::Error) -> NotationAdapterFault {
    NotationAdapterFault::Io {
        operation: AdapterOperation::WaitOutput,
        kind: error.kind(),
    }
}

struct Spool {
    path: PathBuf,
    file: File,
}
impl Spool {
    fn new(support: &Support) -> Result<Self, NotationAdapterFault> {
        let path = support.scratch().join(format!(
            "notation-{}-{}",
            std::process::id(),
            NEXT_SPOOL.fetch_add(1, Ordering::Relaxed)
        ));
        let file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(batch_io)?;
        Ok(Self { path, file })
    }
    fn bytes(&mut self) -> Result<Vec<u8>, NotationAdapterFault> {
        self.file.seek(SeekFrom::Start(0)).map_err(batch_io)?;
        let mut bytes = vec![];
        self.file.read_to_end(&mut bytes).map_err(batch_io)?;
        Ok(bytes)
    }
}
impl Drop for Spool {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn read_frame(
    reader: &mut impl Read,
    spool: &mut impl Write,
    bytes: &mut [u8],
) -> Result<usize, NotationAdapterFault> {
    let mut at = 0;
    while at < bytes.len() {
        match reader.read(&mut bytes[at..]) {
            Ok(0) => break,
            Ok(n) => {
                spool.write_all(&bytes[at..at + n]).map_err(batch_io)?;
                at += n;
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(batch_io(e)),
        }
    }
    Ok(at)
}

fn frame_reason(index: usize, reason: BatchReason) -> ProtocolReason {
    ProtocolReason::BatchFrame { index, reason }
}

fn response_header(index: usize, bytes: &[u8]) -> Result<(u8, u64), ProtocolReason> {
    if bytes.len() != 17 {
        return Err(frame_reason(
            index,
            if bytes.is_empty() {
                BatchReason::UnexpectedEof
            } else {
                BatchReason::TruncatedHeader
            },
        ));
    }
    let actual = u64::from_be_bytes(bytes[..8].try_into().expect("index width"));
    if actual != index as u64 {
        return Err(frame_reason(
            index,
            BatchReason::WrongIndex {
                expected: index as u64,
                actual,
            },
        ));
    }
    let status = bytes[8];
    if ![0, 2, 3].contains(&status) {
        return Err(frame_reason(
            index,
            BatchReason::UnknownStatus { value: status },
        ));
    }
    Ok((
        status,
        u64::from_be_bytes(bytes[9..].try_into().expect("length width")),
    ))
}

/// Consume a bounded inventory, withholding all success until clean termination.
/// # Panics
/// Panics only if a piped descriptor is absent or the private reader panics,
/// which an expression cannot arrange.
#[allow(clippy::too_many_lines)] // Framing, integrity barriers and child reaping share one owner.
pub(super) fn batch(
    support: &Support,
    sources: &[String],
    observed: impl FnMut(usize, &Result<String, String>),
) -> Result<Vec<Result<(), String>>, NotationAdapterFault> {
    batch_with_adapter(
        support,
        &support::repository().join("tools/notation/check.cjs"),
        sources,
        observed,
    )
}

pub(super) fn batch_with_adapter(
    support: &Support,
    adapter_path: &Path,
    sources: &[String],
    observed: impl FnMut(usize, &Result<String, String>),
) -> Result<Vec<Result<(), String>>, NotationAdapterFault> {
    if sources.len() > BATCH_LIMIT {
        return Err(NotationAdapterFault::Protocol {
            status: None,
            stdout: vec![],
            stderr: vec![],
            reason: frame_reason(
                0,
                BatchReason::TooManyRequests {
                    maximum: BATCH_LIMIT,
                    actual: sources.len(),
                },
            ),
        });
    }
    support.scope(adapter_path, || {
        batch_transport(support, adapter_path, sources, observed)
    })
}

#[allow(clippy::too_many_lines)] // Retain a single reaping boundary for every protocol exit.
fn batch_transport(
    support: &Support,
    adapter_path: &Path,
    sources: &[String],
    mut observed: impl FnMut(usize, &Result<String, String>),
) -> Result<Vec<Result<(), String>>, NotationAdapterFault> {
    let mut stdout_spool = Spool::new(support)?;
    let mut stderr_spool = Spool::new(support)?;
    let mut stderr_file = stderr_spool.file.try_clone().map_err(batch_io)?;
    let startup = crate::telemetry::Phase::new("notation.process_start");
    let child = Command::new(&support.runtime)
        .arg(adapter_path)
        .arg("--batch")
        .arg(&support.entry)
        .env_remove("NODE_PATH")
        .env_remove("NODE_OPTIONS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                NotationAdapterFault::Unavailable {
                    component: AdapterComponent::Runtime,
                    kind: e.kind(),
                }
            } else {
                NotationAdapterFault::Io {
                    operation: AdapterOperation::Spawn,
                    kind: e.kind(),
                }
            }
        })?;
    let mut child = ChildGuard(child);
    drop(startup);
    crate::telemetry::counter("notation.process_starts", 1);
    crate::telemetry::counter("notation.version_probes", 1);
    let mut input = child.stdin.take().expect("piped stdin");
    let mut output = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let drain = std::thread::spawn(move || {
        // Keep draining even if storage fails, so termination never waits on a full pipe.
        let mut failed = None;
        let mut buffer = vec![0; 65_536];
        loop {
            match stderr.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if let Err(e) = stderr_file.write_all(&buffer[..n]) {
                        failed.get_or_insert(e);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => {
                    failed.get_or_insert(e);
                    break;
                }
            }
        }
        failed.map_or(Ok(()), Err)
    });
    let mut reason = None;
    let mut results = vec![];
    let consumed = (|| {
        let version_phase = crate::telemetry::Phase::new("notation.version_probe");
        let mut version = vec![];
        let mut lines = 0;
        while lines < 2 {
            let mut byte = [0];
            if read_frame(&mut output, &mut stdout_spool.file, &mut byte)? == 0 {
                reason = Some(ProtocolReason::InvalidVersionResponse);
                return Ok(());
            }
            version.push(byte[0]);
            lines += usize::from(byte[0] == b'\n');
        }
        drop(version_phase);
        support.observe()?;
        match classify(
            Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: version,
                stderr: vec![],
            },
            true,
        ) {
            Ok(_) => {}
            Err(NotationAdapterFault::Protocol { reason: fault, .. }) => {
                reason = Some(fault);
                return Ok(());
            }
            Err(e) => return Err(e),
        }
        for (index, source) in sources.iter().enumerate() {
            crate::telemetry::counter("notation.logical_requests", 1);
            support.observe()?;
            let exchange = crate::telemetry::Phase::new("notation.request_exchange");
            let request = (|| {
                let length = u32::try_from(source.len())
                    .map_err(|_| batch_io(io::Error::from(io::ErrorKind::InvalidInput)))?;
                if source.len() > super::MAX_NOTATION_OUTPUT_BYTES {
                    return Err(batch_io(io::Error::from(io::ErrorKind::InvalidInput)));
                }
                input
                    .write_all(&(index as u64).to_be_bytes())
                    .and_then(|()| input.write_all(&length.to_be_bytes()))
                    .and_then(|()| input.write_all(source.as_bytes()))
                    .and_then(|()| input.flush())
                    .map_err(|e| NotationAdapterFault::Io {
                        operation: AdapterOperation::WriteInput,
                        kind: e.kind(),
                    })?;
                crate::telemetry::counter("notation.submitted_requests", 1);
                #[cfg(test)]
                support.count_submission();
                let mut header = [0; 17];
                let count = read_frame(&mut output, &mut stdout_spool.file, &mut header)?;
                if index == 0
                    && header[..count]
                        .iter()
                        .position(|b| *b == b'\n')
                        .is_some_and(|end| header[..end].iter().all(u8::is_ascii_graphic))
                {
                    reason = Some(ProtocolReason::InvalidVersionResponse);
                    return Ok(());
                }
                let (status, length) = match response_header(index, &header[..count]) {
                    Ok(value) => value,
                    Err(fault) => {
                        reason = Some(fault);
                        return Ok(());
                    }
                };
                let _ = usize::try_from(length)
                    .map_err(|_| batch_io(io::Error::from(io::ErrorKind::FileTooLarge)))?;
                let mut payload = Spool::new(support)?;
                let mut remaining = length;
                let mut buffer = vec![0; 65_536];
                while remaining > 0 {
                    let wanted =
                        usize::try_from(remaining.min(buffer.len() as u64)).expect("buffer bound");
                    let count =
                        read_frame(&mut output, &mut stdout_spool.file, &mut buffer[..wanted])?;
                    payload.file.write_all(&buffer[..count]).map_err(batch_io)?;
                    remaining -= count as u64;
                    if count != wanted {
                        reason = Some(frame_reason(index, BatchReason::TruncatedPayload));
                        return Ok(());
                    }
                }
                let bytes = payload.bytes()?;
                match classify(
                    Output {
                        status: std::process::ExitStatus::from_raw(i32::from(status) << 8),
                        stdout: bytes,
                        stderr: vec![],
                    },
                    false,
                ) {
                    Ok(result) => {
                        observed(index, &result);
                        results.push(result.map(|_| ()));
                    }
                    Err(NotationAdapterFault::Protocol { reason: fault, .. }) => {
                        reason = Some(fault);
                    }
                    Err(e) => return Err(e),
                }
                Ok(())
            })();
            drop(exchange);
            support.observe()?;
            request?;
            if reason.is_some() {
                break;
            }
        }
        Ok::<(), NotationAdapterFault>(())
    })();
    drop(input);
    if consumed.is_err() || reason.is_some() {
        let _ = child.kill();
    }
    let mut tail = vec![0; 65_536];
    let mut trailing = 0_usize;
    let rest = (|| {
        loop {
            let n = match output.read(&mut tail) {
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(batch_io(e)),
            };
            if n == 0 {
                break;
            }
            trailing = trailing.saturating_add(n);
            let _ = child.kill();
            stdout_spool.file.write_all(&tail[..n]).map_err(batch_io)?;
        }
        Ok::<(), NotationAdapterFault>(())
    })();
    if rest.is_err() {
        let _ = child.kill();
    }
    let status = child.wait().map_err(batch_io);
    let drained = drain.join().expect("stderr reader").map_err(batch_io);
    // Termination and both readers are always reaped before returning any fault.
    consumed?;
    rest?;
    drained?;
    let status = status?;
    let stderr = stderr_spool.bytes()?;
    if status.code() == Some(3) && matches!(&reason, Some(ProtocolReason::InvalidVersionResponse)) {
        reason = Some(ProtocolReason::SetupFailure);
    }
    if reason.is_none() {
        reason = if !stderr.is_empty() {
            Some(ProtocolReason::UnexpectedStderr)
        } else if (1..17).contains(&trailing) {
            Some(frame_reason(sources.len(), BatchReason::TruncatedHeader))
        } else if trailing > 0 || !status.success() {
            Some(ProtocolReason::UnexpectedStatus)
        } else {
            None
        };
    }
    if let Some(reason) = reason {
        return Err(NotationAdapterFault::Protocol {
            status: status.code(),
            stdout: stdout_spool.bytes()?,
            stderr,
            reason,
        });
    }
    Ok(results)
}

pub(super) const KATEX_VERSION: &str = "0.16.22";
fn protocol(output: Output, reason: ProtocolReason) -> NotationAdapterFault {
    NotationAdapterFault::Protocol {
        status: output.status.code(),
        stdout: output.stdout,
        stderr: output.stderr,
        reason,
    }
}
pub(super) fn classify(
    output: Output,
    version: bool,
) -> Result<Result<String, String>, NotationAdapterFault> {
    let Ok(stdout) = std::str::from_utf8(&output.stdout) else {
        return Err(protocol(output, ProtocolReason::InvalidUtf8));
    };
    if stdout.starts_with('\u{feff}') {
        return Err(protocol(output, ProtocolReason::InvalidUtf8));
    }
    if output.status.code() == Some(3) {
        return Err(protocol(output, ProtocolReason::SetupFailure));
    }
    if !output.stderr.is_empty() {
        return Err(protocol(output, ProtocolReason::UnexpectedStderr));
    }
    match output.status.code() {
        Some(0) => {
            if version {
                let parts: Vec<_> = stdout.split('\n').collect();
                if parts.len() != 3
                    || !parts[2].is_empty()
                    || !parts[..2].iter().all(|s| {
                        let p: Vec<_> = s.split('.').collect();
                        p.len() == 3
                            && p.iter()
                                .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                    })
                {
                    return Err(protocol(output, ProtocolReason::InvalidVersionResponse));
                }
                for (component, expected, actual) in [
                    (
                        AdapterComponent::Runtime,
                        support::NODE_PIN.trim_end(),
                        parts[0],
                    ),
                    (AdapterComponent::KaTeX, KATEX_VERSION, parts[1]),
                ] {
                    if actual != expected {
                        return Err(NotationAdapterFault::VersionMismatch {
                            component,
                            expected: expected.into(),
                            actual: actual.into(),
                        });
                    }
                }
            } else {
                if stdout.is_empty() {
                    return Err(protocol(output, ProtocolReason::EmptyOutput));
                }
                if !stdout.contains("<math") {
                    return Err(protocol(output, ProtocolReason::MissingMathml));
                }
            }
            Ok(Ok(stdout.into()))
        }
        Some(2) if !version => Ok(Err(stdout.into())),
        _ => Err(protocol(output, ProtocolReason::UnexpectedStatus)),
    }
}
fn collected_output(child: ChildGuard) -> io::Result<Output> {
    std::thread::scope(|scope| {
        // Own the child inside the scope so unwinding kills it before reader joins.
        let mut child = child;
        let mut stdout = child.stdout.take().expect("piped stdout");
        let mut stderr = child.stderr.take().expect("piped stderr");
        let stdout = scope.spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes)?;
            Ok::<_, io::Error>(bytes)
        });
        let stderr = scope.spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes)?;
            Ok::<_, io::Error>(bytes)
        });
        let status = child.wait();
        let stdout = stdout.join().expect("stdout reader");
        let stderr = stderr.join().expect("stderr reader");
        Ok(Output {
            status: status?,
            stdout: stdout?,
            stderr: stderr?,
        })
    })
}
fn run(support: &Support, source: Option<&[u8]>) -> Result<Output, NotationAdapterFault> {
    let _phase = crate::telemetry::Phase::new(if source.is_some() {
        "notation.expression_process"
    } else {
        "notation.version_process"
    });
    let adapter = support::repository().join("tools/notation/check.cjs");
    let startup = crate::telemetry::Phase::new("notation.process_start");
    let child = Command::new(&support.runtime)
        .arg(adapter)
        .arg(if source.is_some() {
            "--expression"
        } else {
            "--version"
        })
        .arg(&support.entry)
        .env_remove("NODE_PATH")
        .env_remove("NODE_OPTIONS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                NotationAdapterFault::Unavailable {
                    component: AdapterComponent::Runtime,
                    kind: e.kind(),
                }
            } else {
                NotationAdapterFault::Io {
                    operation: AdapterOperation::Spawn,
                    kind: e.kind(),
                }
            }
        })?;
    let mut child = ChildGuard(child);
    drop(startup);
    crate::telemetry::counter("notation.process_starts", 1);
    let result = if let Some(source) = source {
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(source)
            .map_err(|e| NotationAdapterFault::Io {
                operation: AdapterOperation::WriteInput,
                kind: e.kind(),
            })
    } else {
        drop(child.stdin.take());
        crate::telemetry::counter("notation.version_probes", 1);
        Ok(())
    };
    if result.is_ok() && source.is_some() {
        crate::telemetry::counter("notation.submitted_requests", 1);
    }
    let output = collected_output(child).map_err(|e| NotationAdapterFault::Io {
        operation: AdapterOperation::WaitOutput,
        kind: e.kind(),
    });
    result?;
    output
}
pub(super) fn invoke(
    support: &Support,
    source: Option<&str>,
) -> Result<Result<String, String>, NotationAdapterFault> {
    if source.is_some() {
        crate::telemetry::counter("notation.logical_requests", 1);
    }
    invoke_with(support, source.is_none(), || {
        run(support, source.map(str::as_bytes))
    })
}
#[cfg(test)]
pub(super) fn invoke_bytes(
    support: &Support,
    source: &[u8],
) -> Result<Result<String, String>, NotationAdapterFault> {
    crate::telemetry::counter("notation.logical_requests", 1);
    invoke_with(support, false, || run(support, Some(source)))
}
pub(super) fn invoke_with(
    support: &Support,
    version: bool,
    run: impl FnOnce() -> Result<Output, NotationAdapterFault>,
) -> Result<Result<String, String>, NotationAdapterFault> {
    support.scope(
        &support::repository().join("tools/notation/check.cjs"),
        || {
            support.observe()?;
            let output = run();
            support.observe()?;
            classify(output?, version)
        },
    )
}

#[cfg(test)]
mod batch_tests {
    use super::*;
    use std::{io::Cursor, os::unix::process::ExitStatusExt};

    #[test]
    fn every_header_boundary_retains_exact_raw_evidence() {
        let mut valid = vec![0; 17];
        valid[7] = 5;
        valid[16] = 7;
        for length in 0..17 {
            let raw = valid[..length].to_vec();
            let mut captured = vec![];
            let mut header = [0; 17];
            let count = read_frame(&mut Cursor::new(&raw), &mut captured, &mut header).unwrap();
            assert_eq!(count, length);
            assert_eq!(captured, raw);
            let expected = frame_reason(
                5,
                if length == 0 {
                    BatchReason::UnexpectedEof
                } else {
                    BatchReason::TruncatedHeader
                },
            );
            let fault = protocol(
                Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: captured,
                    stderr: b"fixture".to_vec(),
                },
                response_header(5, &header[..count]).unwrap_err(),
            );
            let expected = NotationAdapterFault::Protocol {
                status: Some(0),
                stdout: raw,
                stderr: b"fixture".to_vec(),
                reason: expected,
            };
            assert_eq!(fault, expected);
            assert_eq!(fault.to_string(), format!("{expected:?}"));
            assert_eq!(response_header(5, &valid), Ok((0, 7)));
        }
        for actual in [0, 4, 6, u64::MAX] {
            let mut changed = valid.clone();
            changed[..8].copy_from_slice(&actual.to_be_bytes());
            assert_eq!(
                response_header(5, &changed),
                Err(frame_reason(
                    5,
                    BatchReason::WrongIndex {
                        expected: 5,
                        actual
                    }
                ))
            );
            assert_eq!(response_header(5, &valid), Ok((0, 7)));
        }
        for status in 0..=u8::MAX {
            let mut changed = valid.clone();
            changed[8] = status;
            let expected = if [0, 2, 3].contains(&status) {
                Ok((status, 7))
            } else {
                Err(frame_reason(
                    5,
                    BatchReason::UnknownStatus { value: status },
                ))
            };
            assert_eq!(response_header(5, &changed), expected);
            assert_eq!(response_header(5, &valid), Ok((0, 7)));
        }
    }

    #[test]
    fn spool_write_failure_is_an_exact_io_fault() {
        struct Failed;
        impl Write for Failed {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::from(io::ErrorKind::StorageFull))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let expected = NotationAdapterFault::Io {
            operation: AdapterOperation::WaitOutput,
            kind: io::ErrorKind::StorageFull,
        };
        let fault = read_frame(&mut Cursor::new(b"raw"), &mut Failed, &mut [0; 3]).unwrap_err();
        assert_eq!(fault, expected);
        assert_eq!(fault.to_string(), format!("{expected:?}"));
        let mut restored = vec![];
        assert_eq!(
            read_frame(&mut Cursor::new(b"raw"), &mut restored, &mut [0; 3]).unwrap(),
            3
        );
        assert_eq!(restored, b"raw");
    }
}
