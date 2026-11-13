#![allow(clippy::too_many_lines)]
use super::*;
use crate::support_bracket::mutation::FileMutation as _;
use ftoo_manifest::{Callee, ExprDraft, PrimitiveId};
use ftoo_manifest::{DefinitionId, NotationResource};
use std::{
    fmt::Write as _,
    fs,
    os::unix::{fs::symlink, process::ExitStatusExt},
    path::{Path, PathBuf},
    process::Output,
    sync::atomic::{AtomicUsize, Ordering},
};

fn exact_adapter(actual: NotationAdapterFault, expected: NotationAdapterFault) {
    let actual = NotationRenderError::Adapter(actual);
    let expected = NotationRenderError::Adapter(expected);
    assert_eq!(actual, expected);
    assert_eq!(actual.to_string(), format!("{expected:?}"));
}
fn exact_integrity(actual: SupportIntegrityFault, expected: SupportIntegrityFault) {
    exact_adapter(
        NotationAdapterFault::Integrity(actual),
        NotationAdapterFault::Integrity(expected),
    );
}
fn output(code: i32, stdout: &[u8], stderr: &[u8]) -> Output {
    Output {
        status: std::process::ExitStatus::from_raw(code),
        stdout: stdout.into(),
        stderr: stderr.into(),
    }
}
fn prepared() -> support::Support {
    let location = std::env::var_os("FTOO_NOTATION_SUPPORT").map(PathBuf::from);
    support::resolve(location.as_deref()).unwrap()
}

fn observed_identity(path: &Path) -> crate::SupportIdentity {
    use std::os::unix::fs::MetadataExt;
    let value = fs::symlink_metadata(path).unwrap();
    crate::SupportIdentity {
        device: value.dev(),
        inode: value.ino(),
        size: value.size(),
        mode: value.mode(),
        mtime_seconds: value.mtime(),
        mtime_nanoseconds: value.mtime_nsec(),
        ctime_seconds: value.ctime(),
        ctime_nanoseconds: value.ctime_nsec(),
    }
}

fn distinct_timestamp(path: &Path, probe: &Path) {
    let original = observed_identity(path);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        fs::write(probe, b"timestamp quantum").unwrap();
        let current = observed_identity(probe);
        if (current.ctime_seconds, current.ctime_nanoseconds)
            != (original.ctime_seconds, original.ctime_nanoseconds)
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "filesystem timestamp did not advance"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn support_required_bracket_identity_mutations_poison_and_restore() {
    use sha2::{Digest, Sha256};
    let fixture = Fixture::new();
    let support = fixture.support();
    let adapter = support::repository().join("tools/notation/check.cjs");
    let path = support.root.join("packages/node_modules/katex/README.md");
    let bytes = fs::read(&path).unwrap();
    let mut changed = bytes.clone();
    changed[0] ^= 1;
    for mutation in ["identical", "mtime", "rename"] {
        let before = observed_identity(&path);
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        distinct_timestamp(&path, &support.root.join("tmp/clock"));
        let mut captured = None;
        let mut repeated = None;
        let mut resumed = false;
        let result: std::result::Result<(), NotationAdapterFault> = support.scope(&adapter, || {
            let provisional = katex::batch(&support, &["x".into()], |_, _| {})?;
            assert_eq!(provisional, vec![Ok(())]); // Private raw evidence, never returned yet.
            match mutation {
                "identical" => {
                    crate::support_bracket::mutation::write(&path, &bytes).unwrap();
                    fs::File::options()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .set_support_times(fs::FileTimes::new().set_modified(modified))
                        .unwrap();
                }
                "mtime" => {
                    crate::support_bracket::mutation::write(&path, &changed).unwrap();
                    fs::File::options()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .set_support_times(fs::FileTimes::new().set_modified(modified))
                        .unwrap();
                }
                "rename" => {
                    let replacement = support.root.join("tmp/replacement");
                    crate::support_bracket::mutation::write(&replacement, &bytes).unwrap();
                    crate::support_bracket::mutation::rename(replacement, &path).unwrap();
                }
                _ => unreachable!(),
            }
            captured = Some(observed_identity(&path));
            let fault = support.observe().unwrap_err();
            repeated = Some(support.observe().unwrap_err());
            let attempt: std::result::Result<(), NotationAdapterFault> =
                support.scope(&adapter, || {
                    resumed = true;
                    Ok(())
                });
            assert_eq!(attempt, Err(fault));
            Ok(())
        });
        let actual = captured.unwrap();
        assert_ne!(before, actual);
        if mutation != "rename" {
            assert_eq!(before.size, actual.size);
            assert_ne!(
                (before.ctime_seconds, before.ctime_nanoseconds),
                (actual.ctime_seconds, actual.ctime_nanoseconds)
            );
            assert_eq!(
                (before.mtime_seconds, before.mtime_nanoseconds),
                (actual.mtime_seconds, actual.mtime_nanoseconds)
            );
        } else if mutation == "rename" {
            assert_ne!(before.inode, actual.inode);
        }
        let expected = NotationAdapterFault::SupportChanged {
            member: path.to_string_lossy().into_owned(),
            expected: before,
            actual: Some(actual),
        };
        exact_adapter(result.unwrap_err(), expected.clone());
        exact_adapter(repeated.unwrap(), expected);
        assert!(!resumed);
        if mutation == "mtime" {
            assert_eq!(
                support.closing_hash(),
                Some(Err(NotationAdapterFault::Integrity(
                    SupportIntegrityFault::DigestMismatch {
                        file: path.clone(),
                        expected: format!("{:x}", Sha256::digest(&bytes)),
                        actual: format!("{:x}", Sha256::digest(&changed)),
                    }
                )))
            );
        } else {
            assert_eq!(support.closing_hash(), Some(Ok(())));
        }
        crate::support_bracket::mutation::write(&path, &bytes).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(
            katex::batch(&support, &["x".into()], |_, _| {}).unwrap(),
            vec![Ok(())]
        );
    }
}

#[test]
fn support_required_bracket_failed_close_discards_results_and_memo() {
    use sha2::{Digest, Sha256};
    use std::{cell::Cell, rc::Rc};
    let fixture = Fixture::new();
    let support = fixture.support();
    // An admitted but distinct exact metadata key cannot hit another test's memo.
    let hidden = support
        .root
        .join("packages/node_modules/.package-lock.json");
    let mut hidden_bytes = fs::read(&hidden).unwrap();
    hidden_bytes.extend_from_slice(b"\n \t \t \t\n");
    crate::support_bracket::mutation::write(&hidden, hidden_bytes).unwrap();
    let path = support.root.join("packages/node_modules/katex/README.md");
    let bytes = fs::read(&path).unwrap();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let catalogue = ftoo_manifest::notation(&book).unwrap();
    let records = render::declarations(&catalogue).unwrap();
    let published = Rc::new(Cell::new(false));
    let publish = Rc::clone(&published);
    let result: Result<_> = support.scope(
        &support::repository().join("tools/notation/check.cjs"),
        || {
            let provisional = sweep::prepared(&catalogue, &support, &records, false)?;
            let submissions = support.submitted();
            assert!(submissions > 0);
            assert_eq!(
                sweep::prepared(&catalogue, &support, &records, false)?,
                provisional
            );
            assert_eq!(support.submitted(), submissions);
            support.after_close(move || publish.set(true));
            assert!(!published.get());
            crate::support_bracket::mutation::write(&path, b"changed after final request").unwrap();
            Ok(provisional)
        },
    );
    let expected = NotationAdapterFault::Integrity(SupportIntegrityFault::DigestMismatch {
        file: path.clone(),
        expected: format!("{:x}", Sha256::digest(&bytes)),
        actual: format!("{:x}", Sha256::digest(b"changed after final request")),
    });
    assert_eq!(support.closing_hash(), Some(Err(expected.clone())));
    let NotationRenderError::Adapter(error) = result.unwrap_err() else {
        panic!("adapter failure");
    };
    exact_adapter(error, expected);
    assert!(!published.get());
    crate::support_bracket::mutation::write(&path, &bytes).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let before = support.submitted();
    sweep::prepared(&catalogue, &support, &records, false).unwrap();
    assert!(
        support.submitted() > before,
        "failed close published memo evidence"
    );
    let before = support.submitted();
    sweep::prepared(&catalogue, &support, &records, false).unwrap();
    assert_eq!(support.submitted(), before);
}

#[test]
fn support_required_telemetry_failures_preserve_values_and_original_errors() {
    let support = prepared();
    let script = r#"
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { realm, render } = require(process.argv[1]);
const mode = process.argv[2];
const counts = { stat: 0, open: 0, write: 0, close: 0 };
const chunks = [];
process.env.FTOO_GATE_TELEMETRY = '/measurement-fixture';
fs.statSync = () => {
  counts.stat++;
  if (mode === 'stat') throw new Error('stat failure');
  return { isDirectory: () => mode !== 'not-directory' };
};
fs.openSync = () => {
  counts.open++;
  if (mode === 'open') throw new Error('open failure');
  return 42;
};
const write = fs.writeSync;
fs.writeSync = (fd, bytes, at, length) => {
  if (fd !== 42) return write(fd, bytes, at, length);
  counts.write++;
  if (mode === 'write' || mode === 'close') throw new Error('write failure');
  if (mode === 'zero') return 0;
  const n = Math.min(length, 7);
  chunks.push(Buffer.from(bytes.subarray(at, at + n)));
  return n;
};
fs.closeSync = fd => {
  assert.equal(fd, 42);
  counts.close++;
  if (mode === 'close') throw new Error('close failure');
};
assert.throws(() => realm('throw new Error("original realm error")'),
  { message: 'original realm error' });
const katex = realm(`
class ParseError extends Error {}
module.exports = { ParseError, renderToString(source) {
  if (source === 'parse') throw new ParseError('original parse error');
  if (source === 'setup') throw new Error('original setup error');
  return 'original markup';
}};
`);
assert.deepEqual(render(katex, 'valid'), [0, 'original markup']);
assert.deepEqual(render(katex, 'parse'), [2, 'original parse error']);
assert.deepEqual(render(katex, 'setup'), [3, 'original setup error']);
assert.equal(counts.stat, 1);
assert.equal(counts.open, ['stat', 'not-directory'].includes(mode) ? 0 : 1);
if (mode === 'control') {
  assert.ok(counts.write > 5);
  assert.equal(counts.close, 0);
  const records = Buffer.concat(chunks).toString('utf8').trimEnd().split('\n').map(JSON.parse);
  assert.deepEqual(records.map(r => r.phase), [
    'notation.realm_evaluation', 'notation.realm_evaluation',
    'notation.expression_evaluation', 'notation.expression_evaluation',
    'notation.expression_evaluation']);
  for (const r of records) {
    assert.equal(r.pid, process.pid);
    assert.equal(r.members, 0);
    assert.equal(r.bytes, 0);
    assert.ok(Number.isSafeInteger(r.inclusive_ns) && r.inclusive_ns >= 0);
  }
} else {
  const attempted = ['write', 'zero', 'close'].includes(mode) ? 1 : 0;
  assert.equal(counts.write, attempted);
  assert.equal(counts.close, attempted);
  assert.deepEqual(chunks, []);
}
process.stdout.write('values and original errors preserved\n');
"#;
    let outputs = support
        .scope(
            &support::repository().join("tools/notation/check.cjs"),
            || {
                let mut outputs = vec![];
                for mode in ["stat", "not-directory", "open", "write", "zero", "close"] {
                    for mode in [mode, "control"] {
                        support.observe()?;
                        let _consumer = crate::support_bracket::consumer_started();
                        let output = std::process::Command::new(&support.runtime)
                            .arg("-e")
                            .arg(script)
                            .arg(support::repository().join("tools/notation/check.cjs"))
                            .arg(mode)
                            .env_remove("NODE_PATH")
                            .env_remove("NODE_OPTIONS")
                            .output()
                            .unwrap();
                        crate::telemetry::counter("notation.process_starts", 1);
                        support.observe()?;
                        outputs.push((mode, output));
                    }
                }
                Ok::<_, NotationAdapterFault>(outputs)
            },
        )
        .unwrap();
    for (mode, output) in outputs {
        assert_eq!(output.status.code(), Some(0), "{mode}: {output:?}");
        assert_eq!(output.stderr, b"", "{mode}");
        assert_eq!(output.stdout, b"values and original errors preserved\n");
    }
}

#[test]
fn support_required_batch_matches_single_expression_markup_errors_and_isolation() {
    let support = prepared();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let catalogue = ftoo_manifest::notation(&book).unwrap();
    let records = render::declarations(&catalogue).unwrap();
    let declarations: String = records.iter().map(|r| r.source.as_str()).collect();
    let mut sources: Vec<String> = vec![
        "x".into(),
        "{x".into(),
        "α".into(),
        "x^1^2".into(),
        "x".into(),
        r"\gdef\FixtureMacro{x}\FixtureMacro".into(),
        r"\FixtureMacro".into(),
        "x".into(),
        format!("\\newcommand{{\\FtooQCoefficientOne}}[0]{{x}}{declarations}"),
        format!("{declarations}\\FtooQCoefficientOne{{}}"),
    ];
    for record in &records {
        sources.push(record.source.clone());
    }
    let (controls, results, observed, restored) = support
        .scope(
            &support::repository().join("tools/notation/check.cjs"),
            || {
                let controls = sources
                    .iter()
                    .map(|source| katex::invoke(&support, Some(source)))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                let mut observed = vec![];
                let results = katex::batch(&support, &sources, |index, result| {
                    observed.push((index, result.clone()));
                })?;
                let restored = katex::batch(&support, &["x".into()], |_, _| {})?;
                Ok::<_, NotationAdapterFault>((controls, results, observed, restored))
            },
        )
        .unwrap();
    assert_eq!(
        observed,
        controls.iter().cloned().enumerate().collect::<Vec<_>>()
    );
    assert_eq!(
        results,
        controls
            .into_iter()
            .map(|result| result.map(|_| ()))
            .collect::<Vec<_>>()
    );
    assert_eq!(restored, vec![Ok(())]);

    let first = sweep::prepared(&catalogue, &support, &records, false).unwrap();
    let warm = sweep::prepared(&catalogue, &support, &records, false).unwrap();
    assert_eq!(first.0, declarations);
    assert_eq!(warm.0, declarations);
    {
        assert_eq!(warm.1.len(), first.1.len());
        for (actual, expected) in warm.1.iter().zip(&first.1) {
            assert_eq!(
                (
                    actual.owner(),
                    actual.case(),
                    actual.macro_source(),
                    actual.unicode_source()
                ),
                (
                    expected.owner(),
                    expected.case(),
                    expected.macro_source(),
                    expected.unicode_source()
                )
            );
        }
    }
}

#[test]
fn support_required_parse_error_constructor_belongs_to_the_request_realm() {
    let support = prepared();
    let script = r"
const fs = require('node:fs');
const { realm, render } = require(process.argv[1]);
const distribution = fs.readFileSync(process.argv[2], 'utf8');
const own = realm(distribution);
const foreign = realm(distribution);
for (const error of [new own.ParseError('fixture'), new foreign.ParseError('fixture'), new Error('fixture')]) {
  own.renderToString = () => { throw error; };
  process.stdout.write(JSON.stringify(render(own, 'x')) + '\n');
}
const source = 'globalThis.count = (globalThis.count || 0) + 1; module.exports = { renderToString: () => String(globalThis.count) };';
process.stdout.write(render(realm(source), 'x')[1] + render(realm(source), 'x')[1] + '\n');
process.stdout.write(String(render(realm(distribution), 'x')[0]) + '\n');
";
    let output = support
        .scope(
            &support::repository().join("tools/notation/check.cjs"),
            || {
                support.observe()?;
                let _consumer = crate::support_bracket::consumer_started();
                let output = std::process::Command::new(&support.runtime)
                    .arg("-e")
                    .arg(script)
                    .arg(support::repository().join("tools/notation/check.cjs"))
                    .arg(&support.entry)
                    .env_remove("NODE_PATH")
                    .env_remove("NODE_OPTIONS")
                    .output()
                    .unwrap();
                crate::telemetry::counter("notation.process_starts", 1);
                support.observe()?;
                Ok::<_, NotationAdapterFault>(output)
            },
        )
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, b"");
    assert_eq!(output.stdout, b"[2,\"KaTeX parse error: fixture\"]\n[3,\"KaTeX parse error: fixture\"]\n[3,\"fixture\"]\n11\n0\n");
}

#[test]
fn support_required_warm_sweep_still_checks_changed_and_restored_package_bytes() {
    use sha2::{Digest, Sha256};
    let fixture = Fixture::new();
    let support = fixture.support();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let catalogue = ftoo_manifest::notation(&book).unwrap();
    let records = render::declarations(&catalogue).unwrap();
    let original = sweep::prepared(&catalogue, &support, &records, false).unwrap();
    let path = support.entry.clone();
    let bytes = fs::read(&path).unwrap();
    let mut altered = bytes.clone();
    altered.push(b'\n');
    let expected = NotationAdapterFault::Integrity(SupportIntegrityFault::DigestMismatch {
        file: path.clone(),
        expected: format!("{:x}", Sha256::digest(&bytes)),
        actual: format!("{:x}", Sha256::digest(&altered)),
    });
    crate::support_bracket::mutation::write(&path, &altered).unwrap();
    assert_eq!(fs::read(&path).unwrap(), altered);
    let error = sweep::prepared(&catalogue, &support, &records, false).unwrap_err();
    assert_eq!(error, NotationRenderError::Adapter(expected.clone()));
    assert_eq!(
        error.to_string(),
        format!("{:?}", NotationRenderError::Adapter(expected))
    );
    crate::support_bracket::mutation::write(&path, &bytes).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    support.check().unwrap();
    let restored = sweep::prepared(&catalogue, &support, &records, false).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn support_required_batch_protocol_post_request_full_hash_refuses_mutation_and_rereads_restoration()
{
    use sha2::{Digest, Sha256};
    let fixture = Fixture::new();
    let support = fixture.support();
    let bytes = fs::read(&support.entry).unwrap();
    let mut changed = bytes.clone();
    changed.push(b'\n');
    let expected = NotationAdapterFault::Integrity(SupportIntegrityFault::DigestMismatch {
        file: support.entry.clone(),
        expected: format!("{:x}", Sha256::digest(&bytes)),
        actual: format!("{:x}", Sha256::digest(&changed)),
    });
    let before = observed_identity(&support.entry);
    let mut after = None;
    let result = katex::batch(&support, &["x".into(), "y".into()], |index, _| {
        assert_eq!(index, 0);
        crate::support_bracket::mutation::write(&support.entry, &changed).unwrap();
        assert_eq!(fs::read(&support.entry).unwrap(), changed);
        after = Some(observed_identity(&support.entry));
    });
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::SupportChanged {
            member: support.entry.to_string_lossy().into_owned(),
            expected: before,
            actual: after,
        },
    );
    assert_eq!(support.closing_hash(), Some(Err(expected)));
    crate::support_bracket::mutation::write(&support.entry, &bytes).unwrap();
    assert_eq!(fs::read(&support.entry).unwrap(), bytes);
    support.check().unwrap();
    assert_eq!(
        katex::batch(&support, &["x".into(), "y".into()], |_, _| {}).unwrap(),
        vec![Ok(()), Ok(())]
    );
}

#[test]
fn support_required_batch_protocol_frames_keep_full_streams_and_restore_a_valid_control() {
    fn frame(index: u64, status: u8, payload: &[u8]) -> Vec<u8> {
        let mut bytes = index.to_be_bytes().to_vec();
        bytes.push(status);
        bytes.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }
    fn script(stdout: &[u8], stderr: &[u8]) -> String {
        format!(
            "const fs = require('node:fs'); const timer = setInterval(() => {{}}, 1000); process.stdin.on('end', () => clearInterval(timer)); process.stdin.resume(); process.stderr.write(Buffer.from({})); const bytes = Buffer.from({}); let at = 0; while (at < bytes.length) {{ at += fs.writeSync(1, bytes, at, bytes.length - at); }} fs.closeSync(1);",
            serde_json::to_string(stderr).unwrap(),
            serde_json::to_string(stdout).unwrap()
        )
    }
    let fixture = Fixture::new();
    let support = fixture.support();
    let adapter = support.root.join("tmp/protocol.cjs");
    let handshake = b"22.16.0\n0.16.22\n";
    let mut valid = handshake.to_vec();
    valid.extend(frame(0, 0, b"<math>x</math>"));
    let mut cases = vec![];
    for count in 0..17 {
        let mut raw = handshake.to_vec();
        raw.extend_from_slice(&frame(0, 0, b"x")[..count]);
        cases.push((
            raw,
            ProtocolReason::BatchFrame {
                index: 0,
                reason: if count == 0 {
                    BatchReason::UnexpectedEof
                } else {
                    BatchReason::TruncatedHeader
                },
            },
        ));
    }
    for (bytes, reason) in [
        (
            frame(1, 0, b"<math/>"),
            ProtocolReason::BatchFrame {
                index: 0,
                reason: BatchReason::WrongIndex {
                    expected: 0,
                    actual: 1,
                },
            },
        ),
        (
            frame(0, 1, b""),
            ProtocolReason::BatchFrame {
                index: 0,
                reason: BatchReason::UnknownStatus { value: 1 },
            },
        ),
        (frame(0, 0, b""), ProtocolReason::EmptyOutput),
        (frame(0, 0, b"plain"), ProtocolReason::MissingMathml),
        (frame(0, 0, &[255]), ProtocolReason::InvalidUtf8),
        (
            frame(0, 0, "\u{feff}<math/>".as_bytes()),
            ProtocolReason::InvalidUtf8,
        ),
        (
            frame(0, 3, b"adapter failure"),
            ProtocolReason::SetupFailure,
        ),
        (
            frame(0, 0, b"123")[..18].to_vec(),
            ProtocolReason::BatchFrame {
                index: 0,
                reason: BatchReason::TruncatedPayload,
            },
        ),
    ] {
        let mut raw = handshake.to_vec();
        raw.extend(bytes);
        cases.push((raw, reason));
    }
    cases.push((
        b"22.16.0\nmalformed\n".to_vec(),
        ProtocolReason::InvalidVersionResponse,
    ));
    cases.push((
        b"22.16.0\n".to_vec(),
        ProtocolReason::InvalidVersionResponse,
    ));
    cases.push((
        [handshake.as_slice(), b"1.2.3\n", &frame(0, 0, b"<math/>")].concat(),
        ProtocolReason::InvalidVersionResponse,
    ));
    cases.push((
        [valid.as_slice(), &frame(0, 0, b"<math/>")].concat(),
        ProtocolReason::UnexpectedStatus,
    ));
    cases.push((
        [valid.as_slice(), &[0; 5]].concat(),
        ProtocolReason::BatchFrame {
            index: 1,
            reason: BatchReason::TruncatedHeader,
        },
    ));
    for (raw, reason) in cases {
        crate::support_bracket::mutation::write(
            &adapter,
            script(&raw, b"").replace("() => clearInterval(timer)", "() => {}"),
        )
        .unwrap();
        exact_adapter(
            katex::batch_with_adapter(&support, &adapter, &["x".into()], |_, _| {}).unwrap_err(),
            NotationAdapterFault::Protocol {
                status: None,
                stdout: raw,
                stderr: vec![],
                reason,
            },
        );
        let restored = script(&valid, b"");
        crate::support_bracket::mutation::write(&adapter, &restored).unwrap();
        assert_eq!(fs::read(&adapter).unwrap(), restored.as_bytes());
        assert_eq!(
            katex::batch_with_adapter(&support, &adapter, &["x".into()], |_, _| {}).unwrap(),
            vec![Ok(())]
        );
    }
    let stderr = vec![b'e'; 131_072];
    crate::support_bracket::mutation::write(&adapter, script(&valid, &stderr)).unwrap();
    exact_adapter(
        katex::batch_with_adapter(&support, &adapter, &["x".into()], |_, _| {}).unwrap_err(),
        NotationAdapterFault::Protocol {
            status: Some(0),
            stdout: valid.clone(),
            stderr,
            reason: ProtocolReason::UnexpectedStderr,
        },
    );
    let restored = script(&valid, b"");
    crate::support_bracket::mutation::write(&adapter, &restored).unwrap();
    assert_eq!(fs::read(&adapter).unwrap(), restored.as_bytes());
    assert_eq!(
        katex::batch_with_adapter(&support, &adapter, &["x".into()], |_, _| {}).unwrap(),
        vec![Ok(())]
    );
    let large = format!("<math>{}</math>", "x".repeat(1_048_576));
    let framed = [handshake.as_slice(), &frame(0, 0, large.as_bytes())].concat();
    crate::support_bracket::mutation::write(&adapter, script(&framed, b"")).unwrap();
    let mut observed = vec![];
    assert_eq!(
        katex::batch_with_adapter(&support, &adapter, &["x".into()], |index, result| {
            observed.push((index, result.clone()));
        })
        .unwrap(),
        vec![Ok(())]
    );
    assert_eq!(observed, vec![(0, Ok(large))]);
    crate::support_bracket::mutation::write(&adapter, &restored).unwrap();
    assert_eq!(fs::read(&adapter).unwrap(), restored.as_bytes());
    assert_eq!(
        katex::batch_with_adapter(&support, &adapter, &["x".into()], |_, _| {}).unwrap(),
        vec![Ok(())]
    );
    let before = script(&valid, b"")
        .replace("() => clearInterval(timer)", "() => {}")
        .into_bytes();
    let mut after = before.clone();
    after.push(b'\n');
    crate::support_bracket::mutation::write(&adapter, &before).unwrap();
    let before_identity = observed_identity(&adapter);
    let mut after_identity = None;
    let result = katex::batch_with_adapter(&support, &adapter, &["x".into()], |_, _| {
        crate::support_bracket::mutation::write(&adapter, &after).unwrap();
        assert_eq!(fs::read(&adapter).unwrap(), after);
        after_identity = Some(observed_identity(&adapter));
    });
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::SupportChanged {
            member: adapter.to_string_lossy().into_owned(),
            expected: before_identity,
            actual: after_identity,
        },
    );
    crate::support_bracket::mutation::write(&adapter, &restored).unwrap();
    assert_eq!(fs::read(&adapter).unwrap(), restored.as_bytes());
    assert_eq!(
        katex::batch_with_adapter(&support, &adapter, &["x".into()], |_, _| {}).unwrap(),
        vec![Ok(())]
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Complete protocol limits and restoration share one bracket.
fn support_required_batch_protocol_request_boundaries_and_utf8_restore() {
    use std::io::{Read, Write};
    use std::process::{Child, ChildStdout, Command, Stdio};
    use std::result::Result;
    type Evidence = Vec<(Vec<u8>, Vec<u8>)>;
    fn start(
        support: &support::Support,
        entry: &Path,
        evidence: &mut Evidence,
    ) -> Result<(katex::ChildGuard, ChildStdout), NotationAdapterFault> {
        support.observe()?;
        let _consumer = crate::support_bracket::consumer_started();
        let mut child = katex::ChildGuard(
            Command::new(&support.runtime)
                .arg(support::repository().join("tools/notation/check.cjs"))
                .arg("--batch")
                .arg(entry)
                .env_remove("NODE_PATH")
                .env_remove("NODE_OPTIONS")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        crate::telemetry::counter("notation.process_starts", 1);
        crate::telemetry::counter("notation.version_probes", 1);
        let mut stdout = child.stdout.take().unwrap();
        let mut handshake = [0; 16];
        stdout.read_exact(&mut handshake).unwrap();
        evidence.push((handshake.to_vec(), b"22.16.0\n0.16.22\n".to_vec()));
        support.observe()?;
        Ok((child, stdout))
    }
    fn request(
        support: &support::Support,
        child: &mut Child,
        index: u64,
        bytes: &[u8],
    ) -> Result<(), NotationAdapterFault> {
        support.observe()?;
        crate::telemetry::counter("notation.logical_requests", 1);
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(&index.to_be_bytes()).unwrap();
        stdin
            .write_all(&u32::try_from(bytes.len()).unwrap().to_be_bytes())
            .unwrap();
        stdin.write_all(bytes).unwrap();
        stdin.flush().unwrap();
        crate::telemetry::counter("notation.submitted_requests", 1);
        Ok(())
    }
    fn response(
        support: &support::Support,
        evidence: &mut Evidence,
        stdout: &mut ChildStdout,
        index: u64,
        status: u8,
        expected: &[u8],
    ) -> Result<(), NotationAdapterFault> {
        let mut header = [0; 17];
        stdout.read_exact(&mut header).unwrap();
        let mut wanted = index.to_be_bytes().to_vec();
        wanted.push(status);
        wanted.extend_from_slice(&(expected.len() as u64).to_be_bytes());
        let length = usize::try_from(u64::from_be_bytes(header[9..].try_into().unwrap())).unwrap();
        let mut payload = vec![0; length];
        stdout.read_exact(&mut payload).unwrap();
        evidence.push((header.to_vec(), wanted));
        evidence.push((payload, expected.to_vec()));
        support.observe()?;
        Ok(())
    }
    fn end(
        support: &support::Support,
        evidence: &mut Evidence,
        statuses: &mut Vec<(Option<i32>, i32)>,
        mut child: katex::ChildGuard,
        mut stdout: ChildStdout,
        status: i32,
        stderr: &[u8],
    ) -> Result<(), NotationAdapterFault> {
        support.observe()?;
        drop(child.stdin.take());
        let mut rest = vec![];
        stdout.read_to_end(&mut rest).unwrap();
        let mut errors = vec![];
        child
            .stderr
            .take()
            .unwrap()
            .read_to_end(&mut errors)
            .unwrap();
        statuses.push((child.wait().unwrap().code(), status));
        evidence.push((errors, stderr.to_vec()));
        evidence.push((rest, vec![]));
        support.observe()?;
        Ok(())
    }
    let fixture = Fixture::new();
    let support = fixture.support();
    let entry = support.root.join("tmp/distribution.js");
    crate::support_bracket::mutation::write(&entry, "module.exports = { version: '0.16.22', ParseError: class extends Error {}, renderToString: () => { globalThis.seen = (globalThis.seen || 0) + 1; return '<math>' + globalThis.seen + '</math>'; } };").unwrap();
    let (evidence, statuses) = support
        .scope_with_inputs(
            &support::repository().join("tools/notation/check.cjs"),
            std::slice::from_ref(&entry),
            || {
                let mut evidence = vec![];
                let mut statuses = vec![];
                for excess in [true, false] {
                    let (mut child, mut stdout) = start(&support, &entry, &mut evidence)?;
                    for index in 0..4096_u64 {
                        let bytes = match index {
                            0 => vec![255],
                            1 => vec![239, 187, 191],
                            2 => vec![b'x'; 262_144],
                            _ => vec![b'x'],
                        };
                        request(&support, &mut child, index, &bytes)?;
                        match index {
                            0 => response(
                                &support,
                                &mut evidence,
                                &mut stdout,
                                index,
                                3,
                                b"The encoded data was not valid for encoding utf-8",
                            ),
                            1 => response(
                                &support,
                                &mut evidence,
                                &mut stdout,
                                index,
                                3,
                                b"input BOM",
                            ),
                            _ => response(
                                &support,
                                &mut evidence,
                                &mut stdout,
                                index,
                                0,
                                b"<math>1</math>",
                            ),
                        }?;
                    }
                    if excess {
                        support.observe()?;
                        let stdin = child.stdin.as_mut().unwrap();
                        stdin.write_all(&4096_u64.to_be_bytes()).unwrap();
                        stdin.write_all(&0_u32.to_be_bytes()).unwrap();
                        end(
                            &support,
                            &mut evidence,
                            &mut statuses,
                            child,
                            stdout,
                            3,
                            b"batch request limit exceeded: maximum 4096, actual 4097",
                        )?;
                    } else {
                        end(
                            &support,
                            &mut evidence,
                            &mut statuses,
                            child,
                            stdout,
                            0,
                            b"",
                        )?;
                    }
                }
                for (bytes, stderr) in [
                    (vec![0], b"partial batch frame".as_slice()),
                    (
                        [
                            0_u64.to_be_bytes().as_slice(),
                            2_u32.to_be_bytes().as_slice(),
                            b"x",
                        ]
                        .concat(),
                        b"partial batch frame".as_slice(),
                    ),
                    (
                        [
                            1_u64.to_be_bytes().as_slice(),
                            0_u32.to_be_bytes().as_slice(),
                        ]
                        .concat(),
                        b"batch request index mismatch".as_slice(),
                    ),
                    (
                        [
                            0_u64.to_be_bytes().as_slice(),
                            262_145_u32.to_be_bytes().as_slice(),
                        ]
                        .concat(),
                        b"expression input limit exceeded".as_slice(),
                    ),
                ] {
                    let (mut child, stdout) = start(&support, &entry, &mut evidence)?;
                    support.observe()?;
                    child.stdin.as_mut().unwrap().write_all(&bytes).unwrap();
                    end(
                        &support,
                        &mut evidence,
                        &mut statuses,
                        child,
                        stdout,
                        3,
                        stderr,
                    )?;
                    let (mut child, mut stdout) = start(&support, &entry, &mut evidence)?;
                    request(&support, &mut child, 0, b"x")?;
                    response(
                        &support,
                        &mut evidence,
                        &mut stdout,
                        0,
                        0,
                        b"<math>1</math>",
                    )?;
                    end(
                        &support,
                        &mut evidence,
                        &mut statuses,
                        child,
                        stdout,
                        0,
                        b"",
                    )?;
                }
                Ok::<_, NotationAdapterFault>((evidence, statuses))
            },
        )
        .unwrap();
    assert_eq!(statuses.len(), 10);
    assert_eq!(evidence.len(), 16_422);
    for (actual, expected) in evidence {
        assert_eq!(actual, expected);
    }
    for (actual, expected) in statuses {
        assert_eq!(actual, Some(expected));
    }
}
struct Fixture {
    root: PathBuf,
    _serial: std::sync::MutexGuard<'static, ()>,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
        fn copy_tree(from: &Path, to: &Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let destination = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy_tree(&entry.path(), &destination);
                } else if entry.file_type().unwrap().is_file() {
                    fs::copy(entry.path(), destination).unwrap();
                }
            }
        }
        let serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let original = prepared();
        fs::create_dir_all(original.root.join("tmp/brackets")).unwrap();
        let fixtures = original.root.join("tmp/fixtures");
        fs::create_dir_all(&fixtures).unwrap();
        original.check().unwrap();
        let root = fixtures.join(format!(
            "notation-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("tmp")).unwrap();
        copy_tree(&original.root.join("packages"), &root.join("packages"));
        fs::create_dir_all(root.join("runtime/bin")).unwrap();
        fs::copy(&original.runtime, root.join("runtime/bin/node")).unwrap();
        Self {
            root,
            _serial: serial,
        }
    }
    fn support(&self) -> support::Support {
        support::resolve(Some(&self.root)).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn support_configuration_and_protocol_are_exact() {
    for (value, reason) in [
        (None, SupportReason::Missing),
        (Some(PathBuf::from("relative")), SupportReason::Relative),
        (
            Some(support::repository()),
            SupportReason::RepositoryOverlap,
        ),
        (
            Some(support::repository().parent().unwrap().to_path_buf()),
            SupportReason::RepositoryOverlap,
        ),
    ] {
        exact_adapter(
            support::resolve(value.as_deref()).unwrap_err(),
            NotationAdapterFault::SupportConfiguration { value, reason },
        );
    }
    for (status, stdout, stderr, version, reason) in [
        (0, vec![255], vec![], false, ProtocolReason::InvalidUtf8),
        (
            0,
            "\u{feff}<math>".as_bytes().to_vec(),
            vec![],
            false,
            ProtocolReason::InvalidUtf8,
        ),
        (
            3 << 8,
            vec![],
            b"setup".to_vec(),
            false,
            ProtocolReason::SetupFailure,
        ),
        (9, vec![], vec![], false, ProtocolReason::UnexpectedStatus),
        (
            7 << 8,
            vec![],
            vec![],
            false,
            ProtocolReason::UnexpectedStatus,
        ),
        (
            0,
            b"<math>".to_vec(),
            b"warning".to_vec(),
            false,
            ProtocolReason::UnexpectedStderr,
        ),
        (0, vec![], vec![], false, ProtocolReason::EmptyOutput),
        (
            0,
            b"<html>".to_vec(),
            vec![],
            false,
            ProtocolReason::MissingMathml,
        ),
        (
            0,
            b"22.16.0\n0.16.22".to_vec(),
            vec![],
            true,
            ProtocolReason::InvalidVersionResponse,
        ),
        (
            0,
            b"22.16.0\n0.16.22\nextra\n".to_vec(),
            vec![],
            true,
            ProtocolReason::InvalidVersionResponse,
        ),
        (
            0,
            b"v22.16.0\n0.16.22\n".to_vec(),
            vec![],
            true,
            ProtocolReason::InvalidVersionResponse,
        ),
    ] {
        exact_adapter(
            katex::classify(output(status, &stdout, &stderr), version).unwrap_err(),
            NotationAdapterFault::Protocol {
                status: if status == 9 { None } else { Some(status >> 8) },
                stdout,
                stderr,
                reason,
            },
        );
    }
    for (bytes, component, expected, actual) in [
        (
            "22.15.0\n0.16.22\n",
            AdapterComponent::Runtime,
            "22.16.0",
            "22.15.0",
        ),
        (
            "22.16.0\n0.16.21\n",
            AdapterComponent::KaTeX,
            "0.16.22",
            "0.16.21",
        ),
    ] {
        exact_adapter(
            katex::classify(output(0, bytes.as_bytes(), &[]), true).unwrap_err(),
            NotationAdapterFault::VersionMismatch {
                component,
                expected: expected.into(),
                actual: actual.into(),
            },
        );
    }
    assert_eq!(
        katex::classify(output(0, b"22.16.0\n0.16.22\n", &[]), true).unwrap(),
        Ok("22.16.0\n0.16.22\n".into())
    );
    assert_eq!(
        katex::classify(output(512, b"complete parse error", &[]), false).unwrap(),
        Err("complete parse error".into())
    );
}

#[test]
fn strict_metadata_bytes_and_limits() {
    use SupportMetadataReason as R;
    let file = Path::new("metadata.json");
    for (bytes, path, reason) in [
        (
            vec![b' ', 255],
            "$",
            R::InvalidUtf8 {
                offset: 1,
                error_len: Some(1),
            },
        ),
        ("\u{feff}{}".as_bytes().to_vec(), "$", R::Bom),
        (
            br#"{"a":1,"\u0061":2}"#.to_vec(),
            "$",
            R::DuplicateKey { key: "a".into() },
        ),
        (
            br#"{"a":{"b":0,"b":1}}"#.to_vec(),
            "$.a",
            R::DuplicateKey { key: "b".into() },
        ),
        (
            br#"{"a":[1.0]}"#.to_vec(),
            "$.a[0]",
            R::NonInteger {
                token: "1.0".into(),
            },
        ),
        (
            b"1e2".to_vec(),
            "$",
            R::NonInteger {
                token: "1e2".into(),
            },
        ),
        (
            b"{".to_vec(),
            "$",
            R::Syntax {
                message: "EOF while parsing an object at line 1 column 1".into(),
                line: 1,
                column: 1,
            },
        ),
        (
            vec![b' '; 1_048_577],
            "$",
            R::SizeLimit {
                limit: 1_048_576,
                actual: 1_048_577,
            },
        ),
        (
            format!("{}0{}", "[".repeat(129), "]".repeat(129)).into_bytes(),
            "$",
            R::DepthLimit {
                limit: 128,
                actual: 129,
            },
        ),
    ] {
        exact_integrity(
            support::parse(file, &bytes).unwrap_err(),
            SupportIntegrityFault::Metadata {
                file: file.into(),
                path: path.into(),
                reason,
            },
        );
    }
    let mut boundary = vec![b' '; 1_048_575];
    boundary.push(b'0');
    assert_eq!(
        support::parse(file, &boundary).unwrap(),
        support::Value::Integer("0".into())
    );
    support::parse(
        file,
        format!("{}0{}", "[".repeat(128), "]".repeat(128)).as_bytes(),
    )
    .unwrap();
    assert_eq!(
        support::parse(file, br#"{"z":true,"a":[null,"x",-1]}"#).unwrap(),
        support::parse(file, br#"{ "a": [null, "x", -1], "z": true }"#).unwrap()
    );
}

#[test]
fn digest_schema_and_provenance_are_exact() {
    use SupportMetadataReason as R;
    let original: serde_json::Value = serde_json::from_slice(support::DIGESTS).unwrap();
    let file = PathBuf::from("tools/notation/support-digests.json");
    let mut fixtures = vec![];
    let mut v = original.clone();
    v["schema"] = 2.into();
    fixtures.push((
        v,
        "$.schema",
        R::InvalidValue {
            expected: "1".into(),
            actual: "2".into(),
        },
    ));
    let mut v = original.clone();
    v["schema"] = "1".into();
    fixtures.push((
        v,
        "$.schema",
        R::WrongType {
            expected: "integer".into(),
            actual: "string".into(),
        },
    ));
    let mut v = original.clone();
    v.as_object_mut().unwrap().remove("schema");
    fixtures.push((
        v,
        "$",
        R::MissingField {
            name: "schema".into(),
        },
    ));
    let mut v = original.clone();
    v["extra"] = true.into();
    fixtures.push((
        v,
        "$",
        R::UnknownField {
            name: "extra".into(),
        },
    ));
    let mut v = original.clone();
    v["packages"][0]["sha256"] = "ABC".into();
    fixtures.push((
        v,
        "$.packages[0].sha256",
        R::InvalidValue {
            expected: "64 lowercase hexadecimal characters".into(),
            actual: "ABC".into(),
        },
    ));
    for name in [
        "/absolute",
        "node_modules/katex/../escape",
        "node_modules/katex//empty",
        "node_modules/katex/./dot",
        "node_modules/katex/back\\slash",
        "node_modules/other/file",
    ] {
        let mut v = original.clone();
        v["packages"][0]["path"] = name.into();
        fixtures.push((
            v,
            "$.packages[0].path",
            R::InvalidValue {
                expected: "relative locked-package file path".into(),
                actual: name.into(),
            },
        ));
    }
    let mut v = original.clone();
    v["packages"][0]["extra"] = true.into();
    fixtures.push((
        v,
        "$.packages[0]",
        R::UnknownField {
            name: "extra".into(),
        },
    ));
    let mut v = original.clone();
    v["packages"][0].as_object_mut().unwrap().remove("sha256");
    fixtures.push((
        v,
        "$.packages[0]",
        R::MissingField {
            name: "sha256".into(),
        },
    ));
    let mut v = original.clone();
    v["packages"][1] = v["packages"][0].clone();
    fixtures.push((
        v,
        "$.packages[1]",
        R::DuplicateEntry {
            key: "node_modules/commander/LICENSE".into(),
        },
    ));
    let mut v = original.clone();
    let runtime = v["runtimes"][0].clone();
    v["runtimes"].as_array_mut().unwrap().push(runtime);
    fixtures.push((
        v,
        "$.runtimes[1]",
        R::DuplicateEntry {
            key: "linux-x64".into(),
        },
    ));
    let mut v = original.clone();
    v["runtimes"][0]["extra"] = true.into();
    fixtures.push((
        v,
        "$.runtimes[0]",
        R::UnknownField {
            name: "extra".into(),
        },
    ));
    let mut v = original.clone();
    v["runtimes"][0]
        .as_object_mut()
        .unwrap()
        .remove("executable-sha256");
    fixtures.push((
        v,
        "$.runtimes[0]",
        R::MissingField {
            name: "executable-sha256".into(),
        },
    ));
    for (value, path, reason) in fixtures {
        exact_integrity(
            support::inventory(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
            SupportIntegrityFault::Metadata {
                file: file.clone(),
                path: path.into(),
                reason,
            },
        );
    }
    let inventory = support::inventory(support::DIGESTS).unwrap();
    assert_eq!(inventory.packages.len(), 219);
    assert_eq!(
        inventory.runtimes["linux-x64"],
        "8142d37c6f2f372ef040419e7a111a6baf17df89f8078d02447d6c639ae20c1d"
    );
    assert_eq!(
        original["runtimes"][0]["release-sha256"],
        "f4cb75bb036f0d0eddf6b79d9596df1aaab9ddccd6a20bf489be5abe9467e84e"
    );
    assert_eq!(
        original["runtimes"][0]["release-artifact"],
        "node-v22.16.0-linux-x64.tar.xz"
    );
}

#[test]
fn digest_path_byte_order_refuses_first_pair_and_restores() {
    let original: serde_json::Value = serde_json::from_slice(support::DIGESTS).unwrap();
    let admitted = support::inventory(support::DIGESTS).unwrap();
    for (from, to, previous, current) in [
        (
            133,
            135,
            "node_modules/katex/src/environments/cd.js",
            "node_modules/katex/src/environments.js",
        ),
        (
            146,
            192,
            "node_modules/katex/src/functions/verb.js",
            "node_modules/katex/src/functions.js",
        ),
    ] {
        let mut changed = original.clone();
        let records = changed["packages"].as_array_mut().unwrap();
        assert_eq!(records[from]["path"], current);
        let record = records.remove(from);
        records.insert(to, record);
        assert_eq!(records[to - 1]["path"], previous);
        exact_integrity(
            support::inventory(&serde_json::to_vec(&changed).unwrap()).unwrap_err(),
            SupportIntegrityFault::Metadata {
                file: PathBuf::from("tools/notation/support-digests.json"),
                path: format!("$.packages[{to}]"),
                reason: SupportMetadataReason::OutOfOrder {
                    previous: previous.into(),
                    current: current.into(),
                },
            },
        );
        let records = changed["packages"].as_array_mut().unwrap();
        let record = records.remove(to);
        records.insert(from, record);
        assert_eq!(changed, original);
        assert_eq!(
            support::inventory(&serde_json::to_vec(&changed).unwrap())
                .unwrap()
                .packages,
            admitted.packages
        );
    }
}

#[test]
fn support_required_every_invocation_rechecks_copies_hashes_and_postprocess_bytes() {
    let fixture = Fixture::new();
    let support = fixture.support();
    assert_eq!(
        katex::invoke(&support, None).unwrap(),
        Ok("22.16.0\n0.16.22\n".into())
    );
    assert!(
        katex::invoke(&support, Some("α"))
            .unwrap()
            .unwrap()
            .contains("<math")
    );
    for (name, copy, expected_bytes, length) in [
        (
            "package.json",
            SupportCopy::Manifest,
            support::MANIFEST,
            166,
        ),
        (
            "package-lock.json",
            SupportCopy::Lockfile,
            support::LOCK,
            1094,
        ),
    ] {
        let file = fixture.root.join("packages").join(name);
        let original = fs::read(&file).unwrap();
        assert_eq!(original, expected_bytes);
        assert_eq!(original.len(), length);
        let mut changed = original.clone();
        changed[0] ^= 1;
        crate::support_bracket::mutation::write(&file, &changed).unwrap();
        let actual = katex::invoke_with(&support, true, || {
            panic!("copy refusal must precede subprocess")
        });
        crate::support_bracket::mutation::write(&file, &original).unwrap();
        exact_adapter(
            actual.unwrap_err(),
            NotationAdapterFault::Integrity(SupportIntegrityFault::CopyMismatch {
                copy,
                external: file.clone(),
                committed: format!("tools/notation/{name}"),
                first_difference: 0,
                expected_length: length,
                actual_length: length,
            }),
        );
        support.check().unwrap();
        let mut changed = original.clone();
        changed.push(b' ');
        crate::support_bracket::mutation::write(&file, &changed).unwrap();
        let actual = support.check();
        crate::support_bracket::mutation::write(&file, &original).unwrap();
        exact_adapter(
            actual.unwrap_err(),
            NotationAdapterFault::Integrity(SupportIntegrityFault::CopyMismatch {
                copy,
                external: file,
                committed: format!("tools/notation/{name}"),
                first_difference: length,
                expected_length: length,
                actual_length: length + 1,
            }),
        );
        support.check().unwrap();
    }
    // These SHA-256 answers were measured independently with Python hashlib.
    for (name, expected, actual) in [
        (
            "packages/node_modules/katex/dist/katex.js",
            "8f1f4fb1080439c012bb8fddf424fa253e9144ab4cdd7dc0eae0e92cd04a5e33",
            "5b31ed9174171dd8d94617604c7ac0d620b8a8623bf0a1db2b80e4a05dc5b903",
        ),
        (
            "packages/node_modules/katex/package.json",
            "bd6430ca96592a055393a1ce980925162da3d40cd0231813d41ef0c56daacd91",
            "e567ce8fd5bd5b5cf7d8832b052a732a98432d6b870b734517d3e03e0273b117",
        ),
        (
            "packages/node_modules/commander/index.js",
            "d4f8af488a7a08cd71df8503fcc3e5f2fd61e9b26eb368ecd5cc8159862cde3f",
            "d7cbe63dbdc73c6d3c39617002ea6ae83d4660e5d54ecf3c75ba68e6dd516311",
        ),
        (
            "runtime/bin/node",
            "8142d37c6f2f372ef040419e7a111a6baf17df89f8078d02447d6c639ae20c1d",
            "4cec40aecc844435f116dfd5f39bec2008d39ec854e19b010a728f2f33b9daa1",
        ),
    ] {
        let file = fixture.root.join(name);
        let original = fs::read(&file).unwrap();
        let mut changed = original.clone();
        changed[0] ^= 1;
        crate::support_bracket::mutation::write(&file, &changed).unwrap();
        let result = katex::invoke_with(&support, true, || {
            panic!("digest refusal must precede subprocess")
        });
        crate::support_bracket::mutation::write(&file, &original).unwrap();
        let fault = NotationAdapterFault::Integrity(SupportIntegrityFault::DigestMismatch {
            file: file.clone(),
            expected: expected.into(),
            actual: actual.into(),
        });
        exact_adapter(result.unwrap_err(), fault.clone());
        support.check().unwrap();
        // An earlier successful call is never a trust cache. A mutation after
        // precheck also suppresses an otherwise successful subprocess result.
        assert!(
            katex::invoke(&support, Some("β"))
                .unwrap()
                .unwrap()
                .contains("<math")
        );
        let before = observed_identity(&file);
        let mut after = None;
        let result = katex::invoke_with(&support, false, || {
            crate::support_bracket::mutation::write(&file, &changed).unwrap();
            after = Some(observed_identity(&file));
            Ok(output(0, b"<math>untrusted result</math>", &[]))
        });
        crate::support_bracket::mutation::write(&file, &original).unwrap();
        exact_adapter(
            result.unwrap_err(),
            NotationAdapterFault::SupportChanged {
                member: file.to_string_lossy().into_owned(),
                expected: before,
                actual: after,
            },
        );
        assert_eq!(support.closing_hash(), Some(Err(fault)));
        support.check().unwrap();
    }
    for operation in [AdapterOperation::WriteInput, AdapterOperation::WaitOutput] {
        let fault = NotationAdapterFault::Io {
            operation,
            kind: std::io::ErrorKind::BrokenPipe,
        };
        exact_adapter(
            katex::invoke_with(&support, false, || Err(fault.clone())).unwrap_err(),
            fault,
        );
        support.check().unwrap();
    }
    let fault = NotationAdapterFault::Unavailable {
        component: AdapterComponent::Runtime,
        kind: std::io::ErrorKind::NotFound,
    };
    exact_adapter(
        katex::invoke_with(&support, true, || Err(fault.clone())).unwrap_err(),
        fault,
    );
    assert!(
        katex::invoke(&support, Some("\\alpha{}"))
            .unwrap()
            .unwrap()
            .contains("<math")
    );
    exact_adapter(
        katex::invoke_with(&support, true, || Ok(output(0, b"22.15.0\n0.16.22\n", &[])))
            .unwrap_err(),
        NotationAdapterFault::VersionMismatch {
            component: AdapterComponent::Runtime,
            expected: "22.16.0".into(),
            actual: "22.15.0".into(),
        },
    );
    for (bytes, message) in [
        (
            vec![255],
            "The encoded data was not valid for encoding utf-8",
        ),
        (vec![239, 187, 191, b'x'], "input BOM"),
    ] {
        exact_adapter(
            katex::invoke_bytes(&support, &bytes).unwrap_err(),
            NotationAdapterFault::Protocol {
                status: Some(3),
                stdout: vec![],
                stderr: message.as_bytes().to_vec(),
                reason: ProtocolReason::SetupFailure,
            },
        );
        assert!(
            katex::invoke(&support, Some("x"))
                .unwrap()
                .unwrap()
                .contains("<math")
        );
    }
}

#[test]
fn support_required_installed_record_and_file_inventory_are_complete() {
    let fixture = Fixture::new();
    let support = fixture.support();
    let packages = fixture.root.join("packages");
    let hidden = packages.join("node_modules/.package-lock.json");
    let original = fs::read(&hidden).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let commander = r#"{"engines":{"node":">= 12"},"integrity":"sha512-OkTL9umf+He2DZkUq8f8J9of7yL6RJKI24dVITBmNfZBmri9zYZQrKkuXiKhyfPSu8tUhnVBB1iKXevvnlR4Ww==","license":"MIT","resolved":"https://registry.npmjs.org/commander/-/commander-8.3.0.tgz","version":"8.3.0"}"#;
    let mut changed = value.clone();
    changed["packages"]["node_modules/commander"]["version"] = "8.3.1".into();
    crate::support_bracket::mutation::write(&hidden, serde_json::to_vec(&changed).unwrap())
        .unwrap();
    let result = katex::invoke_with(&support, true, || {
        panic!("installed record must precede subprocess")
    });
    crate::support_bracket::mutation::write(&hidden, &original).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::InstalledTreeMismatch {
            file: hidden.clone(),
            missing: vec![],
            unexpected: vec![],
            changed: vec![InstalledPackageChange {
                package: "node_modules/commander".into(),
                expected: commander.into(),
                actual: commander.replace("\"version\":\"8.3.0\"", "\"version\":\"8.3.1\""),
            }],
        }),
    );
    support.check().unwrap();
    // Whitespace and object order are insignificant; only the root project is excluded.
    support::installed(&hidden, &serde_json::to_vec(&value).unwrap()).unwrap();
    let mut changed = value.clone();
    changed["packages"]
        .as_object_mut()
        .unwrap()
        .remove("node_modules/commander");
    changed["packages"]["extra"] = serde_json::json!({});
    changed["packages"][""] = serde_json::json!({});
    exact_integrity(
        support::installed(&hidden, &serde_json::to_vec(&changed).unwrap()).unwrap_err(),
        SupportIntegrityFault::InstalledTreeMismatch {
            file: hidden.clone(),
            missing: vec!["node_modules/commander".into()],
            unexpected: vec![String::new(), "extra".into()],
            changed: vec![],
        },
    );
    let mut changed = value.clone();
    changed["packages"]["node_modules/commander"]["version"] = 8.into();
    exact_integrity(
        support::installed(&hidden, &serde_json::to_vec(&changed).unwrap()).unwrap_err(),
        SupportIntegrityFault::InstalledTreeMismatch {
            file: hidden.clone(),
            missing: vec![],
            unexpected: vec![],
            changed: vec![InstalledPackageChange {
                package: "node_modules/commander".into(),
                expected: commander.into(),
                actual: commander.replace("\"version\":\"8.3.0\"", "\"version\":8"),
            }],
        },
    );
    let mut changed = value.clone();
    changed["unknown"] = true.into();
    exact_integrity(
        support::installed(&hidden, &serde_json::to_vec(&changed).unwrap()).unwrap_err(),
        SupportIntegrityFault::Metadata {
            file: hidden.clone(),
            path: "$".into(),
            reason: SupportMetadataReason::UnknownField {
                name: "unknown".into(),
            },
        },
    );
    crate::support_bracket::mutation::remove_file(&hidden).unwrap();
    let result = support.check();
    crate::support_bracket::mutation::write(&hidden, &original).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::SupportIo {
            file: hidden,
            operation: SupportIoOperation::Inspect,
            kind: std::io::ErrorKind::NotFound,
        }),
    );
    support.check().unwrap();
    let missing_name = "node_modules/commander/index.js";
    let missing = packages.join(missing_name);
    let original = fs::read(&missing).unwrap();
    let extra_name = "node_modules/commander/unexpected.js";
    let extra = packages.join(extra_name);
    crate::support_bracket::mutation::remove_file(&missing).unwrap();
    crate::support_bracket::mutation::write(&extra, b"extra").unwrap();
    let result = support.check();
    crate::support_bracket::mutation::write(&missing, &original).unwrap();
    crate::support_bracket::mutation::remove_file(&extra).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::FileInventoryMismatch {
            directory: packages.clone(),
            missing: vec![missing_name.into()],
            unexpected: vec![extra_name.into()],
        }),
    );
    support.check().unwrap();
    crate::support_bracket::mutation::remove_file(&missing).unwrap();
    let result = support.check();
    crate::support_bracket::mutation::write(&missing, &original).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::FileInventoryMismatch {
            directory: packages.clone(),
            missing: vec![missing_name.into()],
            unexpected: vec![],
        }),
    );
    support.check().unwrap();
    crate::support_bracket::mutation::write(&extra, b"extra").unwrap();
    let result = support.check();
    crate::support_bracket::mutation::remove_file(&extra).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::FileInventoryMismatch {
            directory: packages.clone(),
            missing: vec![],
            unexpected: vec![extra_name.into()],
        }),
    );
    support.check().unwrap();
    crate::support_bracket::mutation::symlink(&missing, &extra).unwrap();
    let result = support.check();
    crate::support_bracket::mutation::remove_file(&extra).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::UnexpectedSupportEntry {
            file: extra.clone(),
            kind: SupportEntryKind::Symlink,
        }),
    );
    support.check().unwrap();
    let socket_parent = fs::File::open(extra.parent().unwrap()).unwrap();
    let socket_alias = format!(
        "/proc/self/fd/{}/{}",
        std::os::fd::AsRawFd::as_raw_fd(&socket_parent),
        extra.file_name().unwrap().to_str().unwrap()
    );
    let socket = std::os::unix::net::UnixListener::bind(socket_alias).unwrap();
    let result = support.check();
    drop(socket);
    crate::support_bracket::mutation::remove_file(&extra).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::UnexpectedSupportEntry {
            file: extra,
            kind: SupportEntryKind::Other,
        }),
    );
    support.check().unwrap();
    for (os, architecture) in [("linux", "aarch64"), ("macos", "x86_64")] {
        exact_integrity(
            support
                .check_inventory(support::DIGESTS, os, architecture)
                .unwrap_err(),
            SupportIntegrityFault::UnsupportedPlatform {
                os: os.into(),
                architecture: architecture.into(),
            },
        );
        support.check().unwrap();
    }
    let entry = packages.join("node_modules/katex/dist/katex.js");
    let bytes = fs::read(&entry).unwrap();
    crate::support_bracket::mutation::remove_file(&entry).unwrap();
    let result = katex::invoke_with(&support, true, || {
        panic!("a missing entry cannot fall back to another package")
    });
    crate::support_bracket::mutation::write(&entry, bytes).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::FileInventoryMismatch {
            directory: packages.clone(),
            missing: vec!["node_modules/katex/dist/katex.js".into()],
            unexpected: vec![],
        }),
    );
    support.check().unwrap();
    let mut archive_as_binary: serde_json::Value =
        serde_json::from_slice(support::DIGESTS).unwrap();
    archive_as_binary["runtimes"][0]["executable-sha256"] =
        "f4cb75bb036f0d0eddf6b79d9596df1aaab9ddccd6a20bf489be5abe9467e84e".into();
    exact_integrity(
        support
            .check_inventory(
                &serde_json::to_vec(&archive_as_binary).unwrap(),
                "linux",
                "x86_64",
            )
            .unwrap_err(),
        SupportIntegrityFault::DigestMismatch {
            file: support.runtime.clone(),
            expected: "f4cb75bb036f0d0eddf6b79d9596df1aaab9ddccd6a20bf489be5abe9467e84e".into(),
            actual: "8142d37c6f2f372ef040419e7a111a6baf17df89f8078d02447d6c639ae20c1d".into(),
        },
    );
    support.check().unwrap();
    let known = fixture.root.join("known-answer");
    crate::support_bracket::mutation::write(&known, b"abc").unwrap();
    assert_eq!(
        support::hash(&known).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    crate::support_bracket::mutation::write(&known, b"").unwrap();
    assert_eq!(
        support::hash(&known).unwrap(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    let absent = fixture.root.join("absent");
    exact_adapter(
        support::resolve(Some(&absent)).unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::SupportIo {
            file: absent,
            operation: SupportIoOperation::Canonicalize,
            kind: std::io::ErrorKind::NotFound,
        }),
    );
    let runtime = fixture.root.join("runtime/bin/node");
    let saved = fixture.root.join("saved-node");
    crate::support_bracket::mutation::rename(&runtime, &saved).unwrap();
    let escaped = prepared().runtime.canonicalize().unwrap();
    crate::support_bracket::mutation::symlink(&escaped, &runtime).unwrap();
    let result = support::resolve(Some(&fixture.root));
    crate::support_bracket::mutation::remove_file(&runtime).unwrap();
    crate::support_bracket::mutation::rename(&saved, &runtime).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::SupportConfiguration {
            value: Some(fixture.root.clone()),
            reason: SupportReason::EscapingEntry {
                component: AdapterComponent::Runtime,
                resolved: escaped,
            },
        },
    );
    support.check().unwrap();
}

const ONE: &str = r"\underbrace{{c}_{1}}_{\text{pending}}";
const THREE: &str = r"\underbrace{{c}_{3}}_{\text{pending}}";
const ONE_REASON: &str = "retained formulation claim:langevin:coefficient-one-value; retained formulation def:langevin:coefficient-one; claim evidence claim:langevin:coefficient-one-value awaits audit-run-required";
fn literal(s: &str) -> ExprDraft {
    ExprDraft::Literal { value: s.into() }
}
fn primitive(id: PrimitiveId, arguments: Vec<ExprDraft>) -> ExprDraft {
    ExprDraft::Call {
        callee: Callee::Primitive(id),
        arguments,
    }
}
fn definition(id: DefinitionId, arguments: Vec<ExprDraft>) -> ExprDraft {
    ExprDraft::Call {
        callee: Callee::Definition(id),
        arguments,
    }
}
fn pending(body: &str) -> String {
    format!("\\underbrace{{{body}}}_{{\\text{{pending}}}}")
}
fn declaration_goldens() -> Vec<(DefinitionId, String)> {
    use DefinitionId::{
        CoefficientOne, CoefficientOneValue, CoefficientThree, CoefficientThreeValue, Dimension,
        FirstCoefficient, Product,
    };
    vec![
        (
            CoefficientOne,
            format!("\\newcommand{{\\FtooQCoefficientOne}}[0]{{{ONE}}}\n"),
        ),
        (
            CoefficientOneValue,
            format!(
                "\\newcommand{{\\FtooOCoefficientOneValue}}[0]{{{}}}\n",
                pending(r"({\FtooQCoefficientOne{}}={\frac{1}{3}})")
            ),
        ),
        (
            CoefficientThree,
            format!("\\newcommand{{\\FtooQCoefficientThree}}[0]{{{THREE}}}\n"),
        ),
        (
            CoefficientThreeValue,
            format!(
                "\\newcommand{{\\FtooOCoefficientThreeValue}}[0]{{{}}}\n",
                pending(r"({\FtooQCoefficientThree{}}={(-{\frac{1}{45}})})")
            ),
        ),
        (
            Dimension,
            "\\newcommand{\\FtooNDimension}[1]{[{#1}]}\n".into(),
        ),
        (
            FirstCoefficient,
            format!(
                "\\newcommand{{\\FtooAFirstCoefficient}}[0]{{{}}}\n",
                pending(r"\FtooQCoefficientOne{}")
            ),
        ),
        (
            Product,
            "\\newcommand{\\FtooNProduct}[2]{({#1}\\cdot{}{#2})}\n".into(),
        ),
    ]
}

#[test]
fn declarations_claims_aliases_and_nested_grouping_goldens() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let records = render::declarations(&n).unwrap();
    assert_eq!(
        records
            .iter()
            .map(|r| (r.id, r.source.clone()))
            .collect::<Vec<_>>(),
        declaration_goldens()
    );
    for (expr, macro_bytes, expanded) in [
        (
            definition(DefinitionId::CoefficientOneValue, vec![]),
            r"\FtooOCoefficientOneValue{}",
            pending(&format!("({{{ONE}}}={{\\frac{{1}}{{3}}}})")),
        ),
        (
            definition(DefinitionId::CoefficientThreeValue, vec![]),
            r"\FtooOCoefficientThreeValue{}",
            pending(&format!("({{{THREE}}}={{(−{{\\frac{{1}}{{45}}}})}})")),
        ),
        (
            definition(DefinitionId::FirstCoefficient, vec![]),
            r"\FtooAFirstCoefficient{}",
            pending(ONE),
        ),
        (
            definition(
                DefinitionId::Product,
                vec![
                    definition(DefinitionId::Dimension, vec![literal("x")]),
                    definition(DefinitionId::Product, vec![literal("y"), literal("z")]),
                ],
            ),
            r"\FtooNProduct{\FtooNDimension{x}}{\FtooNProduct{y}{z}}",
            "({[{x}]}⋅{({y}⋅{z})})".into(),
        ),
        (
            primitive(
                PrimitiveId::Superscript,
                vec![
                    primitive(PrimitiveId::Subscript, vec![literal("x"), literal("1")]),
                    primitive(PrimitiveId::Superscript, vec![literal("y"), literal("2")]),
                ],
            ),
            r"{{x}_{1}}^{{y}^{2}}",
            "{{x}_{1}}^{{y}^{2}}".into(),
        ),
        (
            definition(
                DefinitionId::Dimension,
                vec![primitive(
                    PrimitiveId::Add,
                    vec![literal("x"), literal("1")],
                )],
            ),
            r"\FtooNDimension{({x}+{1})}",
            "[{({x}+{1})}]".into(),
        ),
    ] {
        let checked = ftoo_manifest::check_expression(&n, expr).unwrap();
        for (target, expected) in [
            (RenderTarget::Macro, macro_bytes),
            (RenderTarget::Unicode, expanded.as_str()),
        ] {
            assert_eq!(
                render::expression(
                    &n,
                    checked.draft(),
                    target,
                    RenderOwner::Expression,
                    "golden"
                )
                .unwrap()
                .source,
                expected
            );
        }
    }
}

fn exact_admission(
    actual: NotationRenderError,
    rule: RuleId,
    stage: NotationStage,
    site: NotationSite,
    fault: NotationFault,
) {
    let text = format!("{rule:?} {stage:?} at {site:?}: {fault:?}\n");
    let expected = NotationRenderError::Admission(
        NotationErrors::from_diagnostics(vec![NotationDiagnostic {
            rule,
            stage,
            site,
            fault,
        }])
        .unwrap(),
    );
    assert_eq!(actual.to_string(), text);
    let NotationRenderError::Admission(errors) = actual else {
        panic!("expected numbered admission refusal");
    };
    assert_eq!(NotationRenderError::Admission(errors), expected);
}
#[test]
fn sweep_inventory_mutations_are_exact_and_restore() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let original = render::declarations(&n).unwrap();
    let catalogue = NotationSite::Catalogue {
        file: "crates/ftoo_manifest/src/notation/catalogue.rs".into(),
    };
    let mut records = original.clone();
    records.retain(|r| r.id != DefinitionId::CoefficientThree);
    exact_admission(
        sweep::inventory(&n, &records).unwrap_err(),
        RuleId::M052,
        NotationStage::Names,
        catalogue.clone(),
        NotationFault::SweepInventory {
            missing: vec![DefinitionId::CoefficientThree],
            repeated: vec![],
        },
    );
    sweep::inventory(&n, &original).unwrap();
    for count in [2, 3] {
        let mut records = original.clone();
        for _ in 1..count {
            records.push(original[0].clone());
        }
        exact_admission(
            sweep::inventory(&n, &records).unwrap_err(),
            RuleId::M052,
            NotationStage::Names,
            catalogue.clone(),
            NotationFault::SweepInventory {
                missing: vec![],
                repeated: vec![(DefinitionId::CoefficientOne, count)],
            },
        );
        sweep::inventory(&n, &original).unwrap();
    }
    let mut records = original.clone();
    records.retain(|r| r.id != DefinitionId::CoefficientThree);
    records.push(original[0].clone());
    exact_admission(
        sweep::inventory(&n, &records).unwrap_err(),
        RuleId::M052,
        NotationStage::Names,
        catalogue,
        NotationFault::SweepInventory {
            missing: vec![DefinitionId::CoefficientThree],
            repeated: vec![(DefinitionId::CoefficientOne, 2)],
        },
    );
    sweep::inventory(&n, &original).unwrap();
    let site = NotationSite::Rendered {
        owner: RenderOwner::Definition(DefinitionId::CoefficientOne),
        case: "declaration".into(),
        bytes: 0..declaration_goldens()[0].1.len(),
        origins: vec![NotationOrigin {
            definition: Some(DefinitionId::CoefficientOne),
            path: vec![],
        }],
    };
    let mut records = original.clone();
    records[0].command = "\\Wrong".into();
    exact_admission(
        sweep::inventory(&n, &records).unwrap_err(),
        RuleId::M052,
        NotationStage::Names,
        site.clone(),
        NotationFault::SweepCommand {
            definition: DefinitionId::CoefficientOne,
            expected: "\\FtooQCoefficientOne".into(),
            actual: "\\Wrong".into(),
        },
    );
    sweep::inventory(&n, &original).unwrap();
    let mut records = original.clone();
    records[0].arity = 1;
    exact_admission(
        sweep::inventory(&n, &records).unwrap_err(),
        RuleId::M052,
        NotationStage::Names,
        site,
        NotationFault::SweepSignature {
            definition: DefinitionId::CoefficientOne,
            expected: 0,
            actual: 1,
        },
    );
    sweep::inventory(&n, &original).unwrap();
}

#[test]
fn incomplete_and_output_boundaries_have_exact_sites() {
    let origin = NotationOrigin {
        definition: None,
        path: vec![],
    };
    for (source, remaining) in [
        (r"\FtooQCoefficientOne{}", r"\FtooQCoefficientOne"),
        ("#1", "#1"),
    ] {
        let candidate = render::Candidate {
            source: source.into(),
            origins: vec![origin.clone()],
        };
        exact_admission(
            render::complete(&candidate, RenderOwner::Expression, "mutation").unwrap_err(),
            RuleId::M051,
            NotationStage::Names,
            NotationSite::Rendered {
                owner: RenderOwner::Expression,
                case: "mutation".into(),
                bytes: 0..source.len(),
                origins: vec![origin.clone()],
            },
            NotationFault::IncompleteExpansion {
                remaining: remaining.into(),
                origin: origin.clone(),
            },
        );
    }
    let candidate = render::Candidate {
        source: "α".into(),
        origins: vec![origin.clone()],
    };
    render::complete(&candidate, RenderOwner::Expression, "mutation").unwrap();
    let prefix = "0".repeat(262_143);
    let site = NotationSite::Rendered {
        owner: RenderOwner::Expression,
        case: "output".into(),
        bytes: 0..2,
        origins: vec![origin],
    };
    assert_eq!(
        render::combine(&prefix, "1", site.clone()).unwrap().len(),
        262_144
    );
    exact_admission(
        render::combine(&prefix, "12", site.clone()).unwrap_err(),
        RuleId::M051,
        NotationStage::Names,
        site.clone(),
        NotationFault::NotationLimit {
            resource: NotationResource::OutputBytes,
            limit: 262_144,
            actual: 262_145,
        },
    );
    assert_eq!(render::combine(&prefix, "1", site).unwrap().len(), 262_144);
    let origin = NotationOrigin {
        definition: None,
        path: vec![],
    };
    let mut candidate = render::Candidate {
        source: prefix.clone(),
        origins: vec![],
    };
    exact_admission(
        render::append(
            &mut candidate,
            "α",
            vec![origin.clone()],
            RenderOwner::Expression,
            "writer-limit",
        )
        .unwrap_err(),
        RuleId::M051,
        NotationStage::Names,
        NotationSite::Rendered {
            owner: RenderOwner::Expression,
            case: "writer-limit".into(),
            bytes: 262_143..262_145,
            origins: vec![origin.clone()],
        },
        NotationFault::NotationLimit {
            resource: NotationResource::OutputBytes,
            limit: 262_144,
            actual: 262_145,
        },
    );
    assert_eq!(candidate.source, prefix);
    candidate.source.pop();
    render::append(
        &mut candidate,
        "α",
        vec![origin],
        RenderOwner::Expression,
        "writer-limit",
    )
    .unwrap();
    assert_eq!(candidate.source.len(), 262_144);
}

// This oracle has no descriptor, declaration or writer reads. Its closed input
// inventory and exact strings deliberately duplicate the reviewed specification.
const PRIMITIVES: [(PrimitiveId, &str, usize); 12] = [
    (PrimitiveId::Alpha, "alpha", 0),
    (PrimitiveId::Beta, "beta", 0),
    (PrimitiveId::Fraction, "fraction", 2),
    (PrimitiveId::Subscript, "subscript", 2),
    (PrimitiveId::Superscript, "superscript", 2),
    (PrimitiveId::Add, "add", 2),
    (PrimitiveId::Equal, "equal", 2),
    (PrimitiveId::Negate, "negate", 1),
    (PrimitiveId::Multiply, "multiply", 2),
    (PrimitiveId::Juxtapose, "juxtapose", 2),
    (PrimitiveId::Parentheses, "parentheses", 1),
    (PrimitiveId::Dimension, "dimension", 1),
];
const DEFINITIONS: [(DefinitionId, &str, usize); 7] = [
    (DefinitionId::CoefficientOne, "coefficient-one", 0),
    (DefinitionId::CoefficientThree, "coefficient-three", 0),
    (
        DefinitionId::CoefficientOneValue,
        "coefficient-one-value",
        0,
    ),
    (
        DefinitionId::CoefficientThreeValue,
        "coefficient-three-value",
        0,
    ),
    (DefinitionId::FirstCoefficient, "first-coefficient", 0),
    (DefinitionId::Dimension, "dimension", 1),
    (DefinitionId::Product, "product", 2),
];
fn oracle(expr: &ExprDraft, unicode: bool) -> String {
    let ExprDraft::Call { callee, arguments } = expr else {
        return match expr {
            ExprDraft::Literal { value } => value.clone(),
            _ => panic!("oracle consumes closed ground calls"),
        };
    };
    let a: Vec<_> = arguments.iter().map(|e| oracle(e, unicode)).collect();
    match callee {
        Callee::Primitive(id) => match id {
            PrimitiveId::Alpha => if unicode { "α" } else { r"\alpha{}" }.into(),
            PrimitiveId::Beta => if unicode { "β" } else { r"\beta{}" }.into(),
            PrimitiveId::Fraction => format!("\\frac{{{}}}{{{}}}", a[0], a[1]),
            PrimitiveId::Subscript => format!("{{{}}}_{{{}}}", a[0], a[1]),
            PrimitiveId::Superscript => format!("{{{}}}^{{{}}}", a[0], a[1]),
            PrimitiveId::Add => format!("({{{}}}+{{{}}})", a[0], a[1]),
            PrimitiveId::Equal => format!("({{{}}}={{{}}})", a[0], a[1]),
            PrimitiveId::Negate => format!("({}{{{}}})", if unicode { "−" } else { "-" }, a[0]),
            PrimitiveId::Multiply => format!(
                "({{{}}}{}{{{}}})",
                a[0],
                if unicode { "⋅" } else { r"\cdot{}" },
                a[1]
            ),
            PrimitiveId::Juxtapose => format!("{{{}}}\\,{{{}}}", a[0], a[1]),
            PrimitiveId::Parentheses => format!("({{{}}})", a[0]),
            PrimitiveId::Dimension => format!("[{{{}}}]", a[0]),
        },
        Callee::Definition(id) if !unicode => {
            let command = match id {
                DefinitionId::CoefficientOne => r"\FtooQCoefficientOne",
                DefinitionId::CoefficientThree => r"\FtooQCoefficientThree",
                DefinitionId::CoefficientOneValue => r"\FtooOCoefficientOneValue",
                DefinitionId::CoefficientThreeValue => r"\FtooOCoefficientThreeValue",
                DefinitionId::FirstCoefficient => r"\FtooAFirstCoefficient",
                DefinitionId::Dimension => r"\FtooNDimension",
                DefinitionId::Product => r"\FtooNProduct",
            };
            format!(
                "{command}{}",
                if a.is_empty() {
                    "{}".into()
                } else {
                    a.iter().fold(String::new(), |mut out, s| {
                        write!(out, "{{{s}}}").unwrap();
                        out
                    })
                }
            )
        }
        Callee::Definition(id) => match id {
            DefinitionId::CoefficientOne => ONE.into(),
            DefinitionId::CoefficientThree => THREE.into(),
            DefinitionId::CoefficientOneValue => {
                pending(&format!("({{{ONE}}}={{\\frac{{1}}{{3}}}})"))
            }
            DefinitionId::CoefficientThreeValue => {
                pending(&format!("({{{THREE}}}={{(−{{\\frac{{1}}{{45}}}})}})"))
            }
            DefinitionId::FirstCoefficient => pending(ONE),
            DefinitionId::Dimension => format!("[{{{}}}]", a[0]),
            DefinitionId::Product => format!("({{{}}}⋅{{{}}})", a[0], a[1]),
        },
    }
}

#[test]
fn complete_row_key_invocation_and_both_form_inventory() {
    use std::collections::BTreeMap;
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let ground = |arity: usize| {
        ["x", "y"][..arity]
            .iter()
            .map(|s| literal(s))
            .collect::<Vec<_>>()
    };
    let mut expected = BTreeMap::new();
    for (id, token, arity) in DEFINITIONS {
        expected.insert((token, "base".to_string()), definition(id, ground(arity)));
        if arity > 0 {
            expected.insert(
                (token, "compound".into()),
                definition(
                    id,
                    ground(arity)
                        .into_iter()
                        .map(|e| primitive(PrimitiveId::Add, vec![e, literal("1")]))
                        .collect(),
                ),
            );
            for index in 0..arity {
                for (name, arg) in [
                    ("alpha", primitive(PrimitiveId::Alpha, vec![])),
                    ("pending", definition(DefinitionId::CoefficientOne, vec![])),
                    (
                        "nested-dimension",
                        definition(DefinitionId::Dimension, vec![literal("x")]),
                    ),
                    (
                        "nested-product",
                        definition(DefinitionId::Product, vec![literal("x"), literal("y")]),
                    ),
                ] {
                    let mut args = ground(arity);
                    args[index] = arg;
                    expected.insert(
                        (token, format!("slot-{index}-{name}")),
                        definition(id, args),
                    );
                }
            }
        }
    }
    // Owners distinguish the primitive dimension from its project constructor.
    let mut expected: BTreeMap<_, _> = expected
        .into_iter()
        .map(|((token, case), expr)| ((format!("definition:{token}"), case), expr))
        .collect();
    for (id, token, arity) in PRIMITIVES {
        expected.insert(
            (format!("primitive:{token}"), "base".into()),
            primitive(id, ground(arity)),
        );
        for index in 0..arity {
            for (inner, name, inner_arity) in PRIMITIVES {
                let mut args = ground(arity);
                args[index] = primitive(inner, ground(inner_arity));
                expected.insert(
                    (
                        format!("primitive:{token}"),
                        format!("argument-{index}-primitive-{name}"),
                    ),
                    primitive(id, args),
                );
            }
        }
    }
    let actual: BTreeMap<_, _> = sweep::cases(&n)
        .into_iter()
        .map(|(owner, case, expr)| {
            let name = match owner {
                RenderOwner::Definition(id) => format!("definition:{}", id.token()),
                RenderOwner::Primitive(id) => format!("primitive:{}", id.token()),
                RenderOwner::Expression => unreachable!(),
            };
            ((name, case), expr)
        })
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 237);
    for (owner, case, expr) in sweep::cases(&n) {
        let checked = ftoo_manifest::check_expression(&n, expr).unwrap();
        for target in [RenderTarget::Macro, RenderTarget::Unicode] {
            assert_eq!(
                render::expression(&n, checked.draft(), target, owner, &case)
                    .unwrap()
                    .source,
                oracle(checked.draft(), target == RenderTarget::Unicode),
                "{owner:?}/{case}/{target:?}"
            );
        }
    }
}

#[test]
fn nullary_boundaries_operators_and_nested_contexts() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let mut operands = vec![
        primitive(PrimitiveId::Alpha, vec![]),
        primitive(PrimitiveId::Beta, vec![]),
    ];
    operands.extend(
        DEFINITIONS
            .into_iter()
            .filter(|(_, _, arity)| *arity == 0)
            .map(|(id, _, _)| definition(id, vec![])),
    );
    for operand in operands {
        for suffix in [
            literal("x"),
            literal("1"),
            primitive(PrimitiveId::Alpha, vec![]),
            primitive(PrimitiveId::Superscript, vec![literal("x"), literal("2")]),
            primitive(PrimitiveId::Fraction, vec![literal("1"), literal("3")]),
        ] {
            let expr = primitive(PrimitiveId::Juxtapose, vec![operand.clone(), suffix]);
            for target in [RenderTarget::Macro, RenderTarget::Unicode] {
                assert_eq!(
                    render::expression(&n, &expr, target, RenderOwner::Expression, "boundary")
                        .unwrap()
                        .source,
                    oracle(&expr, target == RenderTarget::Unicode)
                );
            }
        }
    }
    for id in [PrimitiveId::Add, PrimitiveId::Multiply, PrimitiveId::Equal] {
        for args in [
            vec![primitive(PrimitiveId::Alpha, vec![]), literal("x")],
            vec![literal("x"), primitive(PrimitiveId::Beta, vec![])],
        ] {
            let expr = primitive(id, args);
            for target in [RenderTarget::Macro, RenderTarget::Unicode] {
                assert_eq!(
                    render::expression(&n, &expr, target, RenderOwner::Expression, "operator")
                        .unwrap()
                        .source,
                    oracle(&expr, target == RenderTarget::Unicode)
                );
            }
        }
    }
    let chars: Vec<_> = r"\alpha{}#1".chars().collect();
    assert_eq!(chars, ['\\', 'a', 'l', 'p', 'h', 'a', '{', '}', '#', '1']);
}

#[test]
fn substitution_repeats_formals_without_capturing_nested_arguments() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let body = primitive(
        PrimitiveId::Add,
        vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 0 }],
    );
    let args = vec![definition(
        DefinitionId::Product,
        vec![literal("x"), literal("y")],
    )];
    assert_eq!(
        render::fixture_body(&n, &body, &args).unwrap().source,
        "({({x}⋅{y})}+{({x}⋅{y})})"
    );
    let body = definition(
        DefinitionId::Product,
        vec![ExprDraft::Slot { index: 1 }, ExprDraft::Slot { index: 0 }],
    );
    let args = vec![
        definition(DefinitionId::Dimension, vec![literal("x")]),
        definition(DefinitionId::Product, vec![literal("y"), literal("z")]),
    ];
    assert_eq!(
        render::fixture_body(&n, &body, &args).unwrap().source,
        "({({y}⋅{z})}⋅{[{x}]})"
    );
}

#[test]
fn support_required_retained_quantity_examples_and_adversarial_atom_concatenation() {
    let mut draft = ftoo_manifest::seed_draft();
    for node in &mut draft.nodes {
        node.maturity = ftoo_manifest::Maturity::Retained;
    }
    let graph = ftoo_manifest::validate(draft).unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let records = render::declarations(&n).unwrap();
    for (id, source) in [
        (
            DefinitionId::CoefficientOne,
            "\\newcommand{\\FtooQCoefficientOne}[0]{{c}_{1}}\n",
        ),
        (
            DefinitionId::FirstCoefficient,
            "\\newcommand{\\FtooAFirstCoefficient}[0]{\\FtooQCoefficientOne{}}\n",
        ),
        (
            DefinitionId::Dimension,
            "\\newcommand{\\FtooNDimension}[1]{[{#1}]}\n",
        ),
        (
            DefinitionId::Product,
            "\\newcommand{\\FtooNProduct}[2]{({#1}\\cdot{}{#2})}\n",
        ),
    ] {
        assert_eq!(records.iter().find(|r| r.id == id).unwrap().source, source);
    }
    for (expr, macro_bytes, unicode) in [
        (
            definition(DefinitionId::CoefficientOne, vec![]),
            r"\FtooQCoefficientOne{}",
            "{c}_{1}",
        ),
        (
            definition(DefinitionId::FirstCoefficient, vec![]),
            r"\FtooAFirstCoefficient{}",
            "{c}_{1}",
        ),
        (
            definition(
                DefinitionId::Dimension,
                vec![definition(DefinitionId::CoefficientOne, vec![])],
            ),
            r"\FtooNDimension{\FtooQCoefficientOne{}}",
            "[{{c}_{1}}]",
        ),
        (
            definition(
                DefinitionId::Product,
                vec![primitive(PrimitiveId::Alpha, vec![]), literal("x")],
            ),
            r"\FtooNProduct{\alpha{}}{x}",
            "({α}⋅{x})",
        ),
        (
            primitive(
                PrimitiveId::Juxtapose,
                vec![primitive(PrimitiveId::Alpha, vec![]), literal("x")],
            ),
            r"{\alpha{}}\,{x}",
            r"{α}\,{x}",
        ),
        (
            primitive(
                PrimitiveId::Equal,
                vec![
                    definition(DefinitionId::CoefficientOne, vec![]),
                    primitive(PrimitiveId::Fraction, vec![literal("1"), literal("3")]),
                ],
            ),
            r"({\FtooQCoefficientOne{}}={\frac{1}{3}})",
            r"({{c}_{1}}={\frac{1}{3}})",
        ),
        (
            primitive(
                PrimitiveId::Subscript,
                vec![
                    primitive(PrimitiveId::Add, vec![literal("x"), literal("y")]),
                    literal("1"),
                ],
            ),
            "{({x}+{y})}_{1}",
            "{({x}+{y})}_{1}",
        ),
        (
            primitive(
                PrimitiveId::Negate,
                vec![primitive(
                    PrimitiveId::Fraction,
                    vec![literal("1"), literal("45")],
                )],
            ),
            r"(-{\frac{1}{45}})",
            r"(−{\frac{1}{45}})",
        ),
    ] {
        for (target, expected) in [
            (RenderTarget::Macro, macro_bytes),
            (RenderTarget::Unicode, unicode),
        ] {
            assert_eq!(
                render::expression(&n, &expr, target, RenderOwner::Expression, "example")
                    .unwrap()
                    .source,
                expected
            );
        }
    }
    let support = prepared();
    let seed = ftoo_manifest::seed().unwrap();
    let seed_book = ftoo_manifest::addressing(&seed).unwrap();
    let n = ftoo_manifest::notation(&seed_book).unwrap();
    let declarations = render::declarations(&n)
        .unwrap()
        .iter()
        .map(|r| r.source.as_str())
        .collect::<String>();
    let mut atoms = vec![
        primitive(PrimitiveId::Alpha, vec![]),
        primitive(PrimitiveId::Beta, vec![]),
    ];
    atoms.extend(
        DEFINITIONS
            .into_iter()
            .filter(|(_, _, a)| *a == 0)
            .map(|(id, _, _)| definition(id, vec![])),
    );
    let results = support
        .scope(
            &support::repository().join("tools/notation/check.cjs"),
            || {
                let mut results = vec![];
                for target in [RenderTarget::Macro, RenderTarget::Unicode] {
                    let mut concatenations = vec![];
                    for atom in &atoms {
                        for suffix in ["x", "1", r"\beta{}", "^2", "{x}"] {
                            let actual = render::expression(
                                &n,
                                atom,
                                target,
                                RenderOwner::Expression,
                                "atom",
                            )
                            .unwrap()
                            .source;
                            let joined = format!("{actual}{suffix}");
                            assert_eq!(
                                joined,
                                format!(
                                    "{}{suffix}",
                                    oracle(atom, target == RenderTarget::Unicode)
                                )
                            );
                            concatenations.push(format!("{{{joined}}}"));
                        }
                    }
                    let source = concatenations.join(r"\,");
                    let source = if target == RenderTarget::Macro {
                        format!("{declarations}{source}")
                    } else {
                        source
                    };
                    results.push(typeset(
                        &support,
                        target,
                        &source,
                        rendered_site(
                            RenderOwner::Expression,
                            "concatenated-atoms",
                            &source,
                            vec![],
                        ),
                    ));
                }
                Ok::<_, NotationAdapterFault>(results)
            },
        )
        .unwrap();
    assert_eq!(results.len(), 2);
    for result in results {
        result.unwrap();
    }
}

#[test]
fn pending_annotations_keep_compact_markers_in_both_targets() {
    use ftoo_manifest::{Awaiting, CapabilityId, ClaimAudit, Maturity, NodeId, NodePayload};
    for (awaits, text) in [
        (Awaiting::AuditBinding, "audit-binding"),
        (
            Awaiting::Capability(CapabilityId::ExactRationalLinearEquivalence),
            "capability exact-rational-linear-equivalence",
        ),
        (
            Awaiting::Capability(CapabilityId::SymbolicTrigonometry),
            "capability symbolic-trigonometry",
        ),
        (
            Awaiting::Capability(CapabilityId::TrigonometricIntegration),
            "capability trigonometric-integration",
        ),
    ] {
        let mut draft = ftoo_manifest::seed_draft();
        draft.coverage.slice.clear();
        draft.coverage.closed_world.audit = false;
        draft
            .budgets
            .retain(|b| b.producer.descriptor().claim != NodeId::LangevinCoefficientOneValue);
        for node in &mut draft.nodes {
            node.maturity = Maturity::Retained;
            if node.id == NodeId::LangevinCoefficientOneValue
                && let NodePayload::Claim { audit, .. } = &mut node.payload
            {
                *audit = ClaimAudit::Awaiting(awaits);
            }
        }
        let graph = ftoo_manifest::validate(draft).unwrap();
        let book = ftoo_manifest::addressing(&graph).unwrap();
        let n = ftoo_manifest::notation(&book).unwrap();
        let reason = format!("claim evidence claim:langevin:coefficient-one-value awaits {text}");
        let source = render::expression(
            &n,
            &definition(DefinitionId::CoefficientOneValue, vec![]),
            RenderTarget::Unicode,
            RenderOwner::Expression,
            "reason",
        )
        .unwrap()
        .source;
        assert_eq!(source, pending(r"({{c}_{1}}={\frac{1}{3}})"));
        let declarations = render::declarations(&n).unwrap();
        assert_eq!(
            declarations
                .iter()
                .find(|d| d.id == DefinitionId::CoefficientOneValue)
                .unwrap()
                .source,
            format!(
                "\\newcommand{{\\FtooOCoefficientOneValue}}[0]{{{}}}\n",
                pending(r"({\FtooQCoefficientOne{}}={\frac{1}{3}})")
            )
        );
        assert_eq!(
            n.definition(DefinitionId::CoefficientOneValue)
                .status()
                .unwrap()
                .reasons()[0]
                .to_string(),
            reason
        );
    }
}

#[test]
fn support_required_external_markdown_and_links_do_not_change_either_source_inventory() {
    use sha2::{Digest, Sha256};
    use std::process::Command;
    let fixture = Fixture::new();
    let root = support::repository();
    let python = || {
        let result = Command::new("python3").arg("-I").arg("-c").arg("import pathlib,json; print(json.dumps([(p.as_posix(),p.read_text(encoding='utf-8')) for p in sorted(pathlib.Path('.').rglob('*.md'))]))").current_dir(&root).output().unwrap();
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        result.stdout
    };
    let before = crate::load_markdown_files(&root).unwrap();
    let py_before = python();
    let external = fixture.root.join("external.md");
    crate::support_bracket::mutation::write(&external, b"unrelated external prose\n").unwrap();
    symlink(&external, fixture.root.join("external-link.md")).unwrap();
    let after = crate::load_markdown_files(&root).unwrap();
    assert_eq!(after.sources(), before.sources());
    assert_eq!(python(), py_before);
    let expected: Vec<(String, String)> = before
        .sources()
        .iter()
        .map(|s| (s.file.clone(), String::from_utf8(s.bytes.clone()).unwrap()))
        .collect();
    assert_eq!(
        serde_json::from_slice::<Vec<(String, String)>>(&py_before).unwrap(),
        expected
    );
    let mut digest = Sha256::new();
    for source in before.sources() {
        digest.update(source.file.as_bytes());
        digest.update([0]);
        digest.update(Sha256::digest(&source.bytes));
    }
    eprintln!(
        "RUST_SOURCE_INVENTORY {} {:x}",
        before.sources().len(),
        digest.finalize()
    );
}

#[test]
fn committed_adapter_and_ci_pins_are_fixed() {
    assert_eq!(support::NODE_PIN.as_bytes(), b"22.16.0\n");
    let ci = include_str!("../../../../.github/workflows/ci.yml");
    assert!(ci.contains("actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11"));
    assert!(ci.contains("name: bounded smoke — support-dependent suites require the full battery"));
    assert!(ci.contains("test --workspace --locked -- --skip support_required"));
    assert!(ci.contains("unshare --user --map-root-user --net"));
    assert!(!ci.contains("FTOO_NOTATION_SUPPORT"));
    assert!(!ci.contains("FTOO_ASSEMBLY_SUPPORT"));
    assert!(!ci.contains("prepare-native.py"));
    assert!(!ci.contains("npm ci"));
    let script = include_str!("../../../../tools/notation/check.cjs");
    assert!(script.contains("require(entry)"));
    assert!(script.contains("throwOnError: true, strict: 'error', trust: false"));
    assert!(
        script.contains("output: 'htmlAndMathml', displayMode: false, maxExpand: 1000, macros: {}")
    );
}

#[test]
fn support_required_actual_grammar_refusals_and_host_collision_restore() {
    let support = prepared();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let records = render::declarations(&n).unwrap();
    let origin = NotationOrigin {
        definition: None,
        path: vec![],
    };
    let checks = support.scope(&support::repository().join("tools/notation/check.cjs"), || {
    let mut checks: Vec<Box<dyn FnOnce() + '_>> = vec![];
    for (source, message) in [
        (
            r"\NotDefined",
            "KaTeX parse error: Undefined control sequence: \\NotDefined at position 1: \\\u{332}N\u{332}o\u{332}t\u{332}D\u{332}e\u{332}f\u{332}i\u{332}n\u{332}e\u{332}d\u{332}",
        ),
        (
            "{x",
            "KaTeX parse error: Expected '}', got 'EOF' at end of input: {x",
        ),
        (
            "x^1^2",
            "KaTeX parse error: Double superscript at position 4: x^1^\u{332}2",
        ),
        (
            "☃",
            "KaTeX parse error: LaTeX-incompatible input and strict mode is set to 'error': Unrecognized Unicode character \"☃\" (9731) [unknownSymbol] at position 1: ☃\u{332}",
        ),
        (
            r"\alphax",
            "KaTeX parse error: Undefined control sequence: \\alphax at position 1: \\\u{332}a\u{332}l\u{332}p\u{332}h\u{332}a\u{332}x\u{332}",
        ),
    ] {
        let target = if source == r"\alphax" {
            RenderTarget::Macro
        } else {
            RenderTarget::Unicode
        };
        let alpha = primitive(PrimitiveId::Alpha, vec![]);
        let mut emitted =
            render::expression(&n, &alpha, target, RenderOwner::Expression, "grammar").unwrap();
        let restored = if target == RenderTarget::Macro {
            assert_eq!(emitted.source, r"\alpha{}");
            let adjacent = format!("{}x", emitted.source);
            assert_eq!(adjacent, r"\alpha{}x");
            assert_eq!(adjacent.replace("{}", ""), source);
            adjacent
        } else {
            assert_eq!(emitted.source, "α");
            "α".into()
        };
        emitted.source = source.into();
        let site = NotationSite::Rendered {
            owner: RenderOwner::Expression,
            case: "grammar".into(),
            bytes: 0..source.len(),
            origins: vec![origin.clone()],
        };
        assert_eq!(emitted.site(RenderOwner::Expression, "grammar"), site);
        let refused = typeset(&support, target, &emitted.source, site.clone());
        checks.push(Box::new(move || exact_admission(
            refused.unwrap_err(),
            RuleId::M051,
            NotationStage::Typesetting,
            site,
            NotationFault::KaTeXRejected {
                target,
                source: source.into(),
                message: message.into(),
            },
        )));
        emitted.source = restored;
        let restored = typeset(
            &support,
            target,
            &emitted.source,
            emitted.site(RenderOwner::Expression, "restored"),
        );
        checks.push(Box::new(move || { restored.unwrap(); }));
    }
    let context = r"\newcommand{\FtooQCoefficientOne}[0]{x}";
    let collision = sweep::namespace(&support, &records, context);
    checks.push(Box::new(move || exact_admission(collision.unwrap_err(),
        RuleId::M050, NotationStage::Typesetting,
        NotationSite::Rendered { owner: RenderOwner::Definition(DefinitionId::CoefficientOne), case: "declaration".into(), bytes: 39..115, origins: vec![NotationOrigin { definition: Some(DefinitionId::CoefficientOne), path: vec![] }] },
        NotationFault::HostCollision { command: r"\FtooQCoefficientOne".into(), message: r"KaTeX parse error: \newcommand{\FtooQCoefficientOne} attempting to redefine \FtooQCoefficientOne; use \renewcommand".into() })));
    let restored_namespace = sweep::namespace(&support, &records, "");
    checks.push(Box::new(move || { restored_namespace.unwrap(); }));
    let declaration = "\\newcommand{\\FtooQCoefficientOne}[0]{\\NotDefined}\n";
    let declared = katex::invoke(&support, Some(declaration));
    checks.push(Box::new(move || assert!(declared.unwrap().unwrap().contains("<math"))));
    let mut mutated = records.clone();
    mutated[0].source = declaration.into();
    sweep::inventory(&n, &mutated).unwrap();
    let bundle = mutated
        .iter()
        .map(|r| r.source.as_str())
        .collect::<String>();
    let mut expected_declarations = declaration_goldens();
    expected_declarations[0].1 = declaration.into();
    let expected_bundle = expected_declarations
        .iter()
        .map(|(_, s)| s.as_str())
        .collect::<String>();
    assert_eq!(bundle, expected_bundle);
    let full = format!("{expected_bundle}\\FtooQCoefficientOne{{}}");
    let site = NotationSite::Rendered {
        owner: RenderOwner::Definition(DefinitionId::CoefficientOne),
        case: "base".into(),
        bytes: 0..full.len(),
        origins: vec![
            NotationOrigin {
                definition: None,
                path: vec![],
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![ftoo_manifest::NotationPathPart::Field("body".into())],
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![
                    ftoo_manifest::NotationPathPart::Field("body".into()),
                    ftoo_manifest::NotationPathPart::Field("arguments".into()),
                    ftoo_manifest::NotationPathPart::Argument(0),
                ],
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![
                    ftoo_manifest::NotationPathPart::Field("body".into()),
                    ftoo_manifest::NotationPathPart::Field("arguments".into()),
                    ftoo_manifest::NotationPathPart::Argument(1),
                ],
            },
        ],
    };
    let refused = sweep::row(&n, &support, &bundle, RenderOwner::Definition(DefinitionId::CoefficientOne), "base", definition(DefinitionId::CoefficientOne, vec![]));
    checks.push(Box::new(move || exact_admission(refused.unwrap_err(),
        RuleId::M051, NotationStage::Typesetting, site,
        NotationFault::KaTeXRejected { target: RenderTarget::Macro, source: full,
            message: "KaTeX parse error: Undefined control sequence: \\NotDefined at position 38: …ficientOne}[0]{\\\u{332}N\u{332}o\u{332}t\u{332}D\u{332}e\u{332}f\u{332}i\u{332}n\u{332}e\u{332}d\u{332}}\n\\newcommand{\\…".into() })));
    mutated[0] = records[0].clone();
    sweep::inventory(&n, &mutated).unwrap();
    let bundle = mutated
        .iter()
        .map(|r| r.source.as_str())
        .collect::<String>();
    let restored_row = sweep::row(
        &n,
        &support,
        &bundle,
        RenderOwner::Definition(DefinitionId::CoefficientOne),
        "base",
        definition(DefinitionId::CoefficientOne, vec![]),
    );
    checks.push(Box::new(move || {
        let restored_row = restored_row.unwrap();
        assert_eq!(restored_row.macro_source(), r"\FtooQCoefficientOne{}");
        assert_eq!(restored_row.unicode_source(), ONE);
    }));
    let restored = format!(
        "{}\\FtooQCoefficientOne{{}}\\alpha{{}}",
        declaration_goldens()[0].1
    );
    let restored = typeset(
        &support,
        RenderTarget::Macro,
        &restored,
        rendered_site(
            RenderOwner::Definition(DefinitionId::CoefficientOne),
            "restored",
            &restored,
            vec![],
        ),
    );
    checks.push(Box::new(move || { restored.unwrap(); }));
    Ok::<_, NotationAdapterFault>(checks)
    }).unwrap();
    assert_eq!(checks.len(), 16);
    for check in checks {
        check();
    }
}

#[test]
fn expanded_origins_retain_constructor_argument_and_definition_paths() {
    use ftoo_manifest::NotationPathPart::{Argument, Field};
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let expr = definition(
        DefinitionId::Dimension,
        vec![definition(DefinitionId::CoefficientOne, vec![])],
    );
    let expanded = render::expression(
        &n,
        &expr,
        RenderTarget::Unicode,
        RenderOwner::Expression,
        "origins",
    )
    .unwrap();
    assert_eq!(
        expanded.origins,
        [
            NotationOrigin {
                definition: None,
                path: vec![]
            },
            NotationOrigin {
                definition: Some(DefinitionId::Dimension),
                path: vec![Field("body".into())]
            },
            NotationOrigin {
                definition: Some(DefinitionId::Dimension),
                path: vec![Field("body".into()), Field("arguments".into()), Argument(0)]
            },
            NotationOrigin {
                definition: None,
                path: vec![Field("arguments".into()), Argument(0)]
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![Field("body".into())]
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![Field("body".into()), Field("arguments".into()), Argument(0)]
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![Field("body".into()), Field("arguments".into()), Argument(1)]
            },
        ]
    );
}

#[test]
fn primitive_goldens_are_independent() {
    use PrimitiveId::{
        Add, Alpha, Beta, Dimension, Equal, Fraction, Juxtapose, Multiply, Negate, Parentheses,
        Subscript, Superscript,
    };
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    for (id, args, macro_source, unicode) in [
        (Alpha, vec![], "\\alpha{}", "α"),
        (Beta, vec![], "\\beta{}", "β"),
        (Fraction, vec!["x", "y"], "\\frac{x}{y}", "\\frac{x}{y}"),
        (Subscript, vec!["x", "y"], "{x}_{y}", "{x}_{y}"),
        (Superscript, vec!["x", "y"], "{x}^{y}", "{x}^{y}"),
        (Add, vec!["x", "y"], "({x}+{y})", "({x}+{y})"),
        (Equal, vec!["x", "y"], "({x}={y})", "({x}={y})"),
        (Negate, vec!["x"], "(-{x})", "(−{x})"),
        (Multiply, vec!["x", "y"], "({x}\\cdot{}{y})", "({x}⋅{y})"),
        (Juxtapose, vec!["1", "2"], "{1}\\,{2}", "{1}\\,{2}"),
        (Parentheses, vec!["x"], "({x})", "({x})"),
        (Dimension, vec!["x"], "[{x}]", "[{x}]"),
    ] {
        let expr = ExprDraft::Call {
            callee: Callee::Primitive(id),
            arguments: args
                .iter()
                .map(|v| ExprDraft::Literal { value: (*v).into() })
                .collect(),
        };
        for (target, expected) in [
            (RenderTarget::Macro, macro_source),
            (RenderTarget::Unicode, unicode),
        ] {
            assert_eq!(
                render::expression(&n, &expr, target, RenderOwner::Expression, "golden")
                    .unwrap()
                    .source,
                expected
            );
        }
    }
}

#[test]
fn support_required_pending_entries_preserve_order_reasons_and_transitive_origins() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let catalogue = ftoo_manifest::notation(&book).unwrap();
    super::with_preparation(&catalogue, |prepared| -> Result<()> {
        let expression = prepared.check_expression(primitive(
            PrimitiveId::Add,
            vec![
                definition(DefinitionId::CoefficientOneValue, vec![]),
                definition(DefinitionId::FirstCoefficient, vec![]),
            ],
        ))?;
        for target in [RenderTarget::Macro, RenderTarget::Unicode] {
            let rendered = expression.render(target)?;
            let entries = rendered.pending_entries();
            let expected: Vec<_> = catalogue
                .definitions()
                .iter()
                .map(|d| d.spec().id())
                .filter(|id| {
                    matches!(
                        id,
                        DefinitionId::CoefficientOne
                            | DefinitionId::FirstCoefficient
                            | DefinitionId::CoefficientOneValue
                    )
                })
                .collect();
            assert_eq!(
                entries
                    .iter()
                    .map(PendingEntry::definition)
                    .collect::<Vec<_>>(),
                expected
            );
            for entry in entries {
                let used = rendered
                    .uses()
                    .iter()
                    .find(|u| u.definition() == entry.definition())
                    .unwrap();
                assert_eq!(entry.origins(), used.origins());
                assert_ne!(entry.origins(), []);
                let reasons = entry
                    .reasons()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                let body = if entry.definition() == DefinitionId::CoefficientOneValue {
                    assert_eq!(reasons, ONE_REASON);
                    r"({{c}_{1}}={\frac{1}{3}})"
                } else {
                    assert_eq!(reasons, "retained formulation def:langevin:coefficient-one");
                    r"{c}_{1}"
                };
                assert_eq!(entry.body(RenderTarget::Macro), body);
                assert_eq!(entry.body(RenderTarget::Unicode), body);
            }
        }
        Ok(())
    })
    .unwrap();
}
