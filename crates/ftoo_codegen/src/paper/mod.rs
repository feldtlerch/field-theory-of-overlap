//! Whole-pair fixed point and exact delivered-byte gate for the paper and explainer.
mod registry;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A complete refusal from paper preparation or byte checking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaperFault {
    Input(String),
    PdfMismatch(Vec<ByteDifference>),
    CycleCap {
        cycles: u64,
    },
    PassCounts {
        job: String,
        tex: usize,
        bibtex: usize,
        exit: Option<i32>,
    },
    StaleReceipt {
        entry: String,
        expected: Option<String>,
        actual: Option<String>,
    },
}

/// Located bytes around the first difference in one delivered file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ByteDifference {
    pub path: String,
    pub expected_length: usize,
    pub actual_length: usize,
    pub first_difference: usize,
    pub window_start: usize,
    pub expected_hex: String,
    pub actual_hex: String,
    pub expected_text: String,
    pub actual_text: String,
}

impl std::fmt::Display for PaperFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(message) => f.write_str(message),
            Self::PdfMismatch(rows) => {
                f.write_str("Paper byte gate failed")?;
                for row in rows {
                    write!(
                        f,
                        "; {}: offset {}, lengths {} / {}, window {}: expected {} ({:?}), actual {} ({:?})",
                        row.path,
                        row.first_difference,
                        row.expected_length,
                        row.actual_length,
                        row.window_start,
                        row.expected_hex,
                        row.expected_text,
                        row.actual_hex,
                        row.actual_text
                    )?;
                }
                Ok(())
            }
            Self::CycleCap { cycles } => write!(
                f,
                "No joint fixed point after {cycles} pair cycles; retained auxiliary bytes and windows in cycles.json"
            ),
            Self::PassCounts {
                job,
                tex,
                bibtex,
                exit,
            } => write!(
                f,
                "Explicit pass recipe failed for {job}: TeX {tex} (expected 3), bibliography {bibtex} (expected 1), exit {exit:?}"
            ),
            Self::StaleReceipt {
                entry,
                expected,
                actual,
            } => write!(
                f,
                "Paper receipt stale at {entry}: committed {expected:?}, fresh {actual:?}; regenerate"
            ),
        }
    }
}
impl std::error::Error for PaperFault {}
impl From<String> for PaperFault {
    fn from(message: String) -> Self {
        Self::Input(message)
    }
}
impl From<&str> for PaperFault {
    fn from(message: &str) -> Self {
        Self::Input(message.into())
    }
}
type Result<T> = std::result::Result<T, PaperFault>;
type Snapshot = BTreeMap<String, Vec<u8>>;
const OUTPUTS: [(&str, &str); 2] = [("main", "paper.pdf"), ("main_explainer", "explainer.pdf")];
const RECEIPT: &str = "generated/paper/inputs.json";

fn bytes(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| PaperFault::Input(format!("{}: {e}", path.display())))
}
fn save(path: &Path, data: &Value) -> Result<()> {
    let mut output = serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?;
    output.push(b'\n');
    fs::write(path, output).map_err(|e| PaperFault::Input(format!("{}: {e}", path.display())))
}
fn hex(data: &[u8]) -> String {
    let mut out = String::new();
    for byte in data {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| PaperFault::Input(format!("Missing string {key}")))
}
fn number(value: &Value, key: &str) -> Result<u64> {
    value[key]
        .as_u64()
        .ok_or_else(|| PaperFault::Input(format!("Missing integer {key}")))
}
fn strings(value: &Value, key: &str) -> Result<Vec<String>> {
    value[key]
        .as_array()
        .ok_or_else(|| format!("Missing array {key}"))?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| PaperFault::Input(format!("Non-string {key}")))
        })
        .collect()
}
fn command_output(command: &mut Command) -> Result<Vec<u8>> {
    let start = Instant::now();
    let output = command.output().map_err(|e| e.to_string())?;
    eprintln!(
        "paper subprocess: wall={:.6}s",
        start.elapsed().as_secs_f64()
    );
    if !output.status.success() {
        return Err(format!(
            "command failed {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    Ok(output.stdout)
}
fn inputs(root: &Path, support_work: Option<(&Path, &Path)>) -> Result<Value> {
    let mut command = Command::new("python3");
    command
        .args(["-B", "tools/paper/inputs.py"])
        .arg(root)
        .current_dir(root);
    if let Some((support, work)) = support_work {
        command
            .arg("--support")
            .arg(support)
            .arg("--work")
            .arg(work);
    }
    serde_json::from_slice(&command_output(&mut command)?)
        .map_err(|e| PaperFault::Input(e.to_string()))
}
fn byte_difference(path: &str, expected: &[u8], actual: &[u8]) -> ByteDifference {
    let offset = expected
        .iter()
        .zip(actual)
        .position(|(a, b)| a != b)
        .unwrap_or(expected.len().min(actual.len()));
    let lo = offset.saturating_sub(16);
    let a = &expected[lo.min(expected.len())..(offset + 32).min(expected.len())];
    let b = &actual[lo.min(actual.len())..(offset + 32).min(actual.len())];
    ByteDifference {
        path: path.into(),
        expected_length: expected.len(),
        actual_length: actual.len(),
        first_difference: offset,
        window_start: lo,
        expected_hex: hex(a),
        actual_hex: hex(b),
        expected_text: String::from_utf8_lossy(a).into_owned(),
        actual_text: String::from_utf8_lossy(b).into_owned(),
    }
}
fn difference(path: &str, expected: &[u8], actual: &[u8]) -> Value {
    difference_json(&byte_difference(path, expected, actual))
}
fn difference_json(row: &ByteDifference) -> Value {
    json!({"path":row.path, "expected_length":row.expected_length,
        "actual_length":row.actual_length, "first_difference":row.first_difference,
        "window_start":row.window_start, "expected_hex":row.expected_hex,
        "actual_hex":row.actual_hex, "expected_text":row.expected_text,
        "actual_text":row.actual_text})
}
fn pass_counts(job: &str, output: &str, exit: Option<i32>) -> Result<(usize, usize)> {
    let tex = output.matches("Running TeX").count() + output.matches("Rerunning TeX").count();
    let bibtex = output.matches("Running BibTeX").count();
    if exit != Some(0) || tex != 3 || bibtex != 1 {
        return Err(PaperFault::PassCounts {
            job: job.into(),
            tex,
            bibtex,
            exit,
        });
    }
    Ok((tex, bibtex))
}
fn fixed_point(
    previous: Option<&Snapshot>,
    current: &Snapshot,
    cycle: u64,
    cap: u64,
) -> Result<bool> {
    if previous == Some(current) {
        return Ok(true);
    }
    if cycle == cap {
        return Err(PaperFault::CycleCap { cycles: cap });
    }
    Ok(false)
}
fn snapshot(root: &Path, directory: &Path, out: &mut Snapshot) -> Result<()> {
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            snapshot(root, &path, out)?;
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| ["aux", "out", "bbl", "toc", "identities"].contains(&s))
        {
            let name = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .into_owned();
            out.insert(name, bytes(&path)?);
        }
    }
    Ok(())
}
fn auxiliary_delta(a: &Snapshot, b: &Snapshot) -> Vec<Value> {
    a.keys()
        .chain(b.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|p| a.get(*p) != b.get(*p))
        .map(|p| {
            let mut row = difference(
                p,
                a.get(p).map_or(&[], Vec::as_slice),
                b.get(p).map_or(&[], Vec::as_slice),
            );
            row["expected_present"] = json!(a.contains_key(p));
            row["actual_present"] = json!(b.contains_key(p));
            row
        })
        .collect()
}
fn invoke(plan: &Value, work: &Path, job: &str, cycle: u64) -> Result<Value> {
    let settings = &plan["settings"];
    let log = work.join(format!("{job}-{cycle}.txt"));
    let file = fs::File::create(&log).map_err(|e| e.to_string())?;
    let mut command = Command::new(text(plan, "engine")?);
    command
        .env_clear()
        .arg("--bundle")
        .arg(text(plan, "bundle")?)
        .args(strings(settings, "flags")?)
        .arg(format!("{job}.tex"))
        .current_dir(work.join("paper"))
        .stdout(Stdio::from(file.try_clone().map_err(|e| e.to_string())?))
        .stderr(Stdio::from(file));
    command
        .env(
            "SOURCE_DATE_EPOCH",
            number(settings, "source-date-epoch")?.to_string(),
        )
        .env("FORCE_SOURCE_DATE", text(settings, "force-source-date")?);
    for (key, value) in settings["environment"]
        .as_object()
        .ok_or("Missing environment")?
    {
        command.env(key, value.as_str().ok_or("Non-string environment")?);
    }
    for (key, value) in settings["work-environment"]
        .as_object()
        .ok_or("Missing work environment")?
    {
        let path = work.join(value.as_str().ok_or("Non-string work environment")?);
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        command.env(key, path);
    }
    let start = Instant::now();
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(number(settings, "invocation-timeout")?) {
            child.kill().map_err(|e| e.to_string())?;
            child.wait().map_err(|e| e.to_string())?;
            return Err(format!("Typesetting timeout; log {}", log.display()).into());
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let output = bytes(&log)?;
    let output = String::from_utf8_lossy(&output);
    let (tex, bib) = pass_counts(job, &output, status.code())?;
    let row = json!({"job":job,"cycle":cycle,"wall":start.elapsed().as_secs_f64(),"tex":tex,"bibtex":bib,"exit":status.code()});
    println!("{row}");
    Ok(row)
}
fn validate_output(work: &Path, job: &str) -> Result<()> {
    let log = String::from_utf8(bytes(&work.join("paper").join(format!("{job}.log")))?)
        .map_err(|e| e.to_string())?;
    if log.lines().any(|s| s.starts_with('!'))
        || log.contains("undefined references")
        || log.contains("undefined citations")
        || log.contains("Undefined control sequence")
    {
        return Err(format!("Unresolved typesetting input: {job}").into());
    }
    let blg = String::from_utf8(bytes(&work.join("paper").join(format!("{job}.blg")))?)
        .map_err(|e| e.to_string())?
        .to_lowercase();
    if blg.contains("error") || blg.contains("warning") || blg.contains("you can't mess") {
        return Err(format!("Bibliography diagnostic: {job}").into());
    }
    Ok(())
}
fn build(root: &Path, support: &Path, work: &Path) -> Result<Value> {
    if work.exists() {
        return Err(format!("Work area must not exist: {}", work.display()).into());
    }
    fs::create_dir_all(work).map_err(|e| e.to_string())?;
    let plan = inputs(root, Some((support, &work.join("paper"))))?;
    registry::check(root)?;
    let mut previous = None;
    let mut passes = Vec::new();
    let mut cycles = Vec::new();
    let cap = number(&plan["settings"], "max-pair-cycles")?;
    for cycle in 1..=cap {
        for (job, _) in OUTPUTS {
            passes.push(invoke(&plan, work, job, cycle)?);
        }
        let mut current = Snapshot::new();
        snapshot(&work.join("paper"), &work.join("paper"), &mut current)?;
        for (name, data) in &current {
            let target = work.join("auxiliary").join(cycle.to_string()).join(name);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(target, data).map_err(|e| e.to_string())?;
        }
        cycles.push(json!({"cycle":cycle,"differences":auxiliary_delta(previous.as_ref().unwrap_or(&Snapshot::new()), &current)}));
        save(&work.join("cycles.json"), &json!(cycles))?;
        save(&work.join("passes.json"), &json!(passes))?;
        if fixed_point(previous.as_ref(), &current, cycle, cap)? {
            for (job, name) in OUTPUTS {
                validate_output(work, job)?;
                let data = bytes(&work.join("paper").join(format!("{job}.pdf")))?;
                fs::write(work.join(name), data).map_err(|e| e.to_string())?;
            }
            let receipt = json!({"inputs":plan["inputs"]});
            command_output(
                Command::new("python3")
                    .current_dir(root)
                    .args(["-B", "tools/paper/ledger.py"])
                    .arg(root)
                    .arg(work),
            )?;
            save(&work.join("inputs.json"), &receipt)?;
            println!("Joint fixed point: {cycle} pair cycles");
            return Ok(receipt);
        }
        previous = Some(current);
    }
    Err(PaperFault::CycleCap { cycles: cap })
}
fn changed_keys(a: &Value, b: &Value) -> Result<Vec<String>> {
    let a = a.as_object().ok_or("Expected inventory object")?;
    let b = b.as_object().ok_or("Actual inventory object")?;
    Ok(a.keys()
        .chain(b.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect())
}
fn compare(root: &Path, work: &Path, receipt: &Value) -> Result<()> {
    let expected: Value =
        serde_json::from_slice(&bytes(&root.join(RECEIPT))?).map_err(|e| e.to_string())?;
    let mut failures = Vec::new();
    for (_, name) in OUTPUTS {
        let path = format!("generated/paper/{name}");
        let a = bytes(&root.join(&path))?;
        let b = bytes(&work.join(name))?;
        if a != b {
            failures.push(byte_difference(&path, &a, &b));
            fs::create_dir_all(work.join("expected")).map_err(|e| e.to_string())?;
            fs::write(work.join("expected").join(name), a).map_err(|e| e.to_string())?;
        }
    }
    let report = json!({"changed_inputs":changed_keys(&expected["inputs"], &receipt["inputs"])?,"pdf_differences":failures.iter().map(difference_json).collect::<Vec<_>>(),
        "auxiliary_evidence":"Actual bytes and between-cycle byte windows retained in auxiliary/ and cycles.json."});
    save(&work.join("comparison.json"), &report)?;
    println!("{report}");
    if !failures.is_empty() {
        return Err(PaperFault::PdfMismatch(failures));
    }
    check_receipt(&expected, receipt)?;
    let mut fresh = serde_json::to_vec_pretty(receipt).map_err(|e| e.to_string())?;
    fresh.push(b'\n');
    if bytes(&root.join(RECEIPT))? != fresh {
        return Err(PaperFault::StaleReceipt {
            entry: "(encoding)".into(),
            expected: None,
            actual: None,
        });
    }
    println!("Paper byte gate: both PDFs and receipt equal");
    Ok(())
}

fn check_receipt(expected: &Value, actual: &Value) -> Result<()> {
    if let Some(entry) = changed_keys(&expected["inputs"], &actual["inputs"])?
        .into_iter()
        .next()
    {
        return Err(PaperFault::StaleReceipt {
            expected: expected["inputs"][&entry].as_str().map(str::to_owned),
            actual: actual["inputs"][&entry].as_str().map(str::to_owned),
            entry,
        });
    }
    if expected != actual {
        return Err(PaperFault::StaleReceipt {
            entry: "(structure)".into(),
            expected: None,
            actual: None,
        });
    }
    Ok(())
}

struct Request {
    operation: String,
    root: PathBuf,
    values: BTreeMap<String, OsString>,
}
fn request(args: &[OsString]) -> Result<Request> {
    let mut iter = args.iter();
    let operation = iter
        .next()
        .ok_or("Expected generate or check")?
        .to_str()
        .ok_or("Non-UTF8 command")?
        .to_owned();
    if !["generate", "check"].contains(&operation.as_str()) {
        return Err("Unknown paper command".into());
    }
    let mut values = BTreeMap::new();
    while let Some(arg) = iter.next() {
        let key = arg.to_str().ok_or("Non-UTF8 option")?;
        if !["--repository", "--support", "--work"].contains(&key) {
            return Err(format!("Unknown option {key}").into());
        }
        if values
            .insert(
                key.to_owned(),
                iter.next().ok_or("Missing option value")?.clone(),
            )
            .is_some()
        {
            return Err(format!("Duplicate option {key}").into());
        }
    }
    let root = values.get("--repository").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    Ok(Request {
        operation,
        root: root.canonicalize().map_err(|e| e.to_string())?,
        values,
    })
}
/// Run the explicit whole-paper command and support-required byte gate.
///
/// # Errors
/// Returns a located input, subprocess, fixed-point or byte-comparison failure.
pub fn run(args: &[OsString]) -> Result<()> {
    let req = request(args)?;
    let work = PathBuf::from(req.values.get("--work").ok_or("Clean work area required")?);
    if !work.is_absolute() {
        return Err("Work area must be absolute".into());
    }
    let support = req
        .values
        .get("--support")
        .cloned()
        .or_else(|| std::env::var_os("FTOO_ASSEMBLY_SUPPORT"))
        .ok_or("Pinned support required")?;
    let receipt = build(&req.root, &PathBuf::from(support), &work)?;
    if req.operation == "generate" {
        fs::create_dir_all(req.root.join("generated/paper")).map_err(|e| e.to_string())?;
        for name in ["paper.pdf", "explainer.pdf", "inputs.json"] {
            fs::copy(work.join(name), req.root.join("generated/paper").join(name))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    } else {
        compare(&req.root, &work, &receipt)
    }
}

#[cfg(test)]
mod tests;
