use super::{
    AdapterComponent, AdapterOperation, NotationAdapterFault, ProtocolReason,
    support::{self, Support},
};
use std::{
    io::{self, Write},
    process::{Command, Output, Stdio},
};

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
fn run(support: &Support, source: Option<&[u8]>) -> Result<Output, NotationAdapterFault> {
    let adapter = support::repository().join("tools/notation/check.cjs");
    let mut child = Command::new(&support.runtime)
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
        Ok(())
    };
    let output = child
        .wait_with_output()
        .map_err(|e| NotationAdapterFault::Io {
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
    invoke_with(support, source.is_none(), || {
        run(support, source.map(str::as_bytes))
    })
}
#[cfg(test)]
pub(super) fn invoke_bytes(
    support: &Support,
    source: &[u8],
) -> Result<Result<String, String>, NotationAdapterFault> {
    invoke_with(support, false, || run(support, Some(source)))
}
pub(super) fn invoke_with(
    support: &Support,
    version: bool,
    run: impl FnOnce() -> Result<Output, NotationAdapterFault>,
) -> Result<Result<String, String>, NotationAdapterFault> {
    support.check()?;
    let output = run();
    support.check()?;
    classify(output?, version)
}
