//! Whole-pair fixed point and exact delivered-byte gate for the paper and explainer.
pub(crate) mod html_gaps;
mod registry;
mod release;
use crate::hex;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A complete refusal from paper preparation or byte checking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaperFault {
    Input(String),
    ReadingAcceptance {
        passage: u64,
    },
    PassageSet {
        ids: Vec<u64>,
    },
    Release {
        part: &'static str,
        reason: String,
    },
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
            Self::Release { part, reason } => {
                write!(f, "Paper release M065 refused at {part}: {reason}")
            }
            Self::ReadingAcceptance { passage } => write!(
                f,
                "Paper passage {passage}: accepted bytes must occur exactly once with the recorded hash and a filled reading acceptance"
            ),
            Self::PassageSet { ids } => write!(
                f,
                "Paper acceptance requires exactly passage ids 1 through 19; found {ids:?}"
            ),
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
const PAIR_FILES: [&str; 4] = [
    "paper.pdf",
    "explainer.pdf",
    "inputs.json",
    "identity-ledger.json",
];

fn bytes(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| PaperFault::Input(format!("{}: {e}", path.display())))
}
fn save(path: &Path, data: &Value) -> Result<()> {
    let mut output = serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?;
    output.push(b'\n');
    fs::write(path, output).map_err(|e| PaperFault::Input(format!("{}: {e}", path.display())))
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
    check_reading_acceptance(root)?;
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
fn check_reading_acceptance(root: &Path) -> Result<()> {
    let record: Value = serde_json::from_slice(&bytes(
        &root.join("documents/paper-build/self-description.json"),
    )?)
    .map_err(|e| e.to_string())?;
    let rows = record["passages"].as_array().ok_or("Missing passages")?;
    let mut ids = rows
        .iter()
        .map(|row| number(row, "id"))
        .collect::<Result<Vec<_>>>()?;
    ids.sort_unstable();
    if ids != (1..=19).collect::<Vec<_>>() {
        return Err(PaperFault::PassageSet { ids });
    }
    for row in rows {
        let fault = PaperFault::ReadingAcceptance {
            passage: number(row, "id")?,
        };
        let data = bytes(&root.join(text(row, "file")?))?;
        let anchor = text(row, "anchor")?.as_bytes();
        let length = usize::try_from(number(row, "end")?.saturating_sub(number(row, "start")?))
            .map_err(|_| fault.clone())?;
        let matches = data
            .windows(anchor.len().max(1))
            .enumerate()
            .filter(|(i, b)| {
                *b == anchor
                    && data.get(*i..i.saturating_add(length)).is_some_and(|span| {
                        hex(&Sha256::digest(span)) == row["sha256"].as_str().unwrap_or("")
                    })
            })
            .count();
        if length == 0 || matches != 1 || text(row, "reading_acceptance")?.trim().is_empty() {
            return Err(fault);
        }
    }
    Ok(())
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
fn validate_output(work: &Path, job: &str, cycle: u64) -> Result<()> {
    let log = String::from_utf8(bytes(&work.join("paper").join(format!("{job}.log")))?)
        .map_err(|e| e.to_string())?;
    let diagnostic = log.to_ascii_lowercase();
    let stream = String::from_utf8(bytes(&work.join(format!("{job}-{cycle}.txt")))?)
        .map_err(|e| e.to_string())?;
    if diagnostic.contains("multiply defined")
        || stream.lines().any(|line| {
            line.starts_with("warning: Object @") && line.ends_with(" already defined.")
        })
    {
        return Err(format!("Duplicate label or named output object: {job}").into());
    }
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
                validate_output(work, job, cycle)?;
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

fn collect_files(root: &Path, dir: &Path, rows: &mut BTreeMap<String, String>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            collect_files(root, &path, rows)?;
        } else if path.is_file() {
            let name = path.strip_prefix(root).map_err(|e| e.to_string())?;
            rows.insert(
                name.to_string_lossy().into_owned(),
                hex(&Sha256::digest(bytes(&path)?)),
            );
        }
    }
    Ok(())
}

fn check_built(built: &Path) -> Result<Value> {
    let record: Value =
        serde_json::from_slice(&bytes(&built.join("pair.json"))?).map_err(|e| e.to_string())?;
    let private = record["private_dir"]
        .as_str()
        .ok_or("Missing pair private directory")?;
    if private.contains('/') || private == "." || private == ".." {
        return Err("Invalid pair private directory".into());
    }
    let outputs = record["outputs"]
        .as_object()
        .ok_or("Missing pair outputs")?;
    for (name, hash) in outputs {
        if !PAIR_FILES.contains(&name.as_str()) {
            return Err(format!("Unknown pair output: {name}").into());
        }
        if hex(&Sha256::digest(bytes(&built.join(name))?))
            != hash.as_str().ok_or("Invalid pair hash")?
        {
            return Err(format!("Pair output differs: {name}").into());
        }
    }
    if outputs.len() != PAIR_FILES.len() {
        return Err("Incomplete pair outputs".into());
    }
    let work = record["work"]
        .as_object()
        .ok_or("Missing pair work membership")?;
    let work_root = built.join(private).join("work");
    let mut actual = BTreeMap::new();
    collect_files(&work_root, &work_root, &mut actual)?;
    if actual.len() != work.len()
        || actual
            .iter()
            .any(|(name, hash)| work.get(name).and_then(Value::as_str) != Some(hash))
    {
        return Err("Pair work evidence differs".into());
    }
    serde_json::from_slice(&bytes(&built.join("inputs.json"))?).map_err(|e| e.to_string().into())
}

fn produce(root: &Path, support: &Path, work: &Path, output: &Path) -> Result<()> {
    if let Some(private) = work.parent() {
        let previous = private.join("work.previous");
        if previous.exists() {
            fs::remove_dir_all(&previous).map_err(|e| e.to_string())?;
        }
        if work.exists() {
            fs::rename(work, previous).map_err(|e| e.to_string())?;
        }
    }
    build(root, support, work)?;
    publish(work, output)
}

fn publish(work: &Path, output: &Path) -> Result<()> {
    let mut work_files = BTreeMap::new();
    collect_files(work, work, &mut work_files)?;
    let mut outputs = BTreeMap::new();
    for name in PAIR_FILES {
        outputs.insert(name, hex(&Sha256::digest(bytes(&work.join(name))?)));
    }
    let private = work.parent().ok_or("Missing private work parent")?;
    if private.parent() != Some(output) {
        return Err("Private work must be inside pair output directory".into());
    }
    let private_name = private
        .file_name()
        .ok_or("Missing private directory name")?
        .to_string_lossy();
    let completion = json!({"private_dir":private_name,"outputs":outputs,"work":work_files});
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    for name in PAIR_FILES {
        let staged = private.join(format!("{name}.next"));
        fs::copy(work.join(name), &staged).map_err(|e| e.to_string())?;
        fs::rename(staged, output.join(name)).map_err(|e| e.to_string())?;
    }
    let staged = private.join("pair.json.next");
    save(&staged, &completion)?;
    fs::rename(staged, output.join("pair.json")).map_err(|e| e.to_string())?;
    Ok(())
}

struct Request {
    operation: String,
    root: PathBuf,
    values: BTreeMap<String, OsString>,
    release: bool,
    registered_html_gaps: bool,
}
fn request(args: &[OsString]) -> Result<Request> {
    let mut iter = args.iter();
    let operation = iter
        .next()
        .ok_or("Expected generate, check, documents, import-dry-run or import-correspondence operation")?
        .to_str()
        .ok_or("Non-UTF8 command")?
        .to_owned();
    if ![
        "generate",
        "check",
        "check-registered-html-gaps",
        "produce",
        "publish",
        "documents-generate",
        "documents-check",
        "documents-recovery",
        "import-dry-run",
        "import-correspondence",
        "documents-check-gaps",
    ]
    .contains(&operation.as_str())
    {
        return Err("Unknown paper command".into());
    }
    let mut values = BTreeMap::new();
    let mut release = false;
    while let Some(arg) = iter.next() {
        if arg == "--release" {
            release = true;
            continue;
        }
        let key = arg.to_str().ok_or("Non-UTF8 option")?;
        if ![
            "--repository",
            "--support",
            "--work",
            "--built",
            "--output-dir",
            "--assignment",
            "--side",
        ]
        .contains(&key)
        {
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
    if operation != "import-correspondence"
        && (values.contains_key("--assignment") || values.contains_key("--side"))
    {
        return Err("Assignment and side belong only to import-correspondence".into());
    }
    let root = values.get("--repository").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let registered_html_gaps = operation == "check-registered-html-gaps";
    if registered_html_gaps {
        if release || !values.contains_key("--built") {
            return Err(
                "Registered development gaps require the built product check without a release flag"
                    .into(),
            );
        }
        release = true;
    }
    Ok(Request {
        operation,
        root: root.canonicalize().map_err(|e| e.to_string())?,
        values,
        release,
        registered_html_gaps,
    })
}
fn require_notation_support(value: Option<&std::ffi::OsStr>) -> Result<()> {
    if value.is_none() {
        return Err("Notation support required".into());
    }
    Ok(())
}
fn assembly_support(req: &Request) -> Result<crate::NativeSupport> {
    let path = req
        .values
        .get("--support")
        .cloned()
        .or_else(|| std::env::var_os("FTOO_ASSEMBLY_SUPPORT"))
        .map(PathBuf::from);
    crate::NativeSupport::resolve(&req.root, path.as_deref())
        .map_err(|error| PaperFault::Input(crate::DocumentError::Native(error).to_string()))
}
fn run_documents(req: &Request) -> Result<()> {
    if req.release {
        return Err("Release flag applies only to the paper pair".into());
    }
    if req.values.contains_key("--work") {
        return Err("Documents commands take no work area".into());
    }
    let assembly = assembly_support(req)?;
    require_notation_support(std::env::var_os("FTOO_NOTATION_SUPPORT").as_deref())?;
    if req.operation == "documents-generate" {
        let prepared = crate::generate_documents(&req.root, &assembly)
            .map_err(|error| PaperFault::Input(error.to_string()))?;
        release::write_registers(&req.root, &prepared)
    } else if req.operation == "documents-check-gaps" {
        let prepared = crate::check_documents(&req.root, &assembly)
            .map_err(|error| PaperFault::Input(error.to_string()))?;
        release::screen_parts(&req.root, &prepared, true)
    } else if req.operation == "documents-recovery" {
        let output = req
            .values
            .get("--output-dir")
            .ok_or("Recovery output directory required")?;
        let output = PathBuf::from(output);
        if !output.is_absolute() || output.starts_with(&req.root) {
            return Err("Recovery output must be absolute and outside the repository".into());
        }
        let prepared = crate::check_documents(&req.root, &assembly)
            .map_err(|error| PaperFault::Input(error.to_string()))?;
        fs::create_dir_all(&output).map_err(|e| e.to_string())?;
        for medium in ["markdown", "html"] {
            let export = prepared
                .corpus_slot_recovery_export(
                    medium,
                    "documents/paper/explainer/explainer.tex",
                    44,
                    98,
                    "sec:explainer:plain",
                )
                .ok_or("Checked corpus-slot recovery unavailable")?;
            let bytes =
                serde_json::to_vec_pretty(&export).map_err(|e| PaperFault::Input(e.to_string()))?;
            fs::write(output.join(format!("{medium}-recovery.json")), bytes)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    } else {
        crate::check_documents(&req.root, &assembly)
            .map(|_| ())
            .map_err(|error| PaperFault::Input(error.to_string()))
    }
}
fn check_release_documents(req: &Request) -> Result<PathBuf> {
    let assembly = assembly_support(req)?;
    require_notation_support(std::env::var_os("FTOO_NOTATION_SUPPORT").as_deref())?;
    let prepared = crate::check_documents(&req.root, &assembly)
        .map_err(|error| PaperFault::Input(error.to_string()))?;
    release::screen_parts(&req.root, &prepared, req.registered_html_gaps)?;
    Ok(assembly.directory().to_path_buf())
}
fn pair_support(req: &Request, release_support: Option<PathBuf>) -> Result<PathBuf> {
    release_support
        .or_else(|| {
            req.values
                .get("--support")
                .cloned()
                .or_else(|| std::env::var_os("FTOO_ASSEMBLY_SUPPORT"))
                .map(PathBuf::from)
        })
        .ok_or_else(|| "Pinned support required".into())
}
/// Run the explicit whole-paper command and support-required byte gate.
///
/// # Errors
/// Returns a located input, subprocess, fixed-point or byte-comparison failure.
pub fn run(args: &[OsString]) -> Result<()> {
    let req = request(args)?;
    if let Some(result) = run_import_operation(&req) {
        return result;
    }
    let built = req.values.contains_key("--built");
    if req.operation.starts_with("documents-") {
        return run_documents(&req);
    }
    let release_support = if req.release {
        Some(check_release_documents(&req)?)
    } else {
        None
    };
    if req.release && !built {
        release::inputs(&req.root)?;
    }
    if req.release || built {
        if !req.release {
            check_reading_acceptance(&req.root)?;
        }
        if req.release {
            release::part("registry", registry::check(&req.root))?;
        } else {
            registry::check(&req.root)?;
        }
        let mut identities = Command::new("python3");
        identities
            .current_dir(&req.root)
            .args(["-B", "tools/paper/identities.py"])
            .arg(&req.root);
        if req.release {
            identities.arg("--release");
            release::part("printed identities", command_output(&mut identities))?;
        } else {
            command_output(&mut identities)?;
        }
    }
    if let Some(built) = req.values.get("--built") {
        if req.operation != "check" && req.operation != "check-registered-html-gaps" {
            return Err("Built pair is only valid for check".into());
        }
        let built = PathBuf::from(built);
        let receipt = check_built(&built)?;
        return if req.release {
            release::check(&req.root, &built, &receipt)
        } else {
            compare(&req.root, &built, &receipt)
        };
    }
    let mut work = PathBuf::from(req.values.get("--work").ok_or("Clean work area required")?);
    if ["produce", "publish"].contains(&req.operation.as_str()) && !work.is_absolute() {
        work = std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(work);
    }
    if !work.is_absolute() {
        return Err("Work area must be absolute".into());
    }
    if ["produce", "publish"].contains(&req.operation.as_str()) {
        let mut output = PathBuf::from(
            req.values
                .get("--output-dir")
                .ok_or("Pair output directory required")?,
        );
        if !output.is_absolute() {
            output = std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(output);
        }
        if req.operation == "publish" {
            return publish(&work, &output);
        }
        let support = req
            .values
            .get("--support")
            .cloned()
            .or_else(|| std::env::var_os("FTOO_ASSEMBLY_SUPPORT"))
            .ok_or("Pinned support required")?;
        return produce(&req.root, &PathBuf::from(support), &work, &output);
    }
    let support = pair_support(&req, release_support)?;
    let built = build(&req.root, &support, &work);
    let receipt = if req.release {
        release::part("paper byte gate", built)?
    } else {
        built?
    };
    if req.release {
        return release::check(&req.root, &work, &receipt);
    }
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

fn run_import_operation(req: &Request) -> Option<Result<()>> {
    match req.operation.as_str() {
        "import-dry-run" => Some(run_import(req)),
        "import-correspondence" => Some(run_correspondence(req)),
        _ => None,
    }
}

fn run_import(req: &Request) -> Result<()> {
    if req.release
        || req.values.contains_key("--support")
        || req.values.contains_key("--work")
        || req.values.contains_key("--built")
        || req.values.contains_key("--assignment")
        || req.values.contains_key("--side")
    {
        return Err("Import dry run takes only repository and output directory".into());
    }
    let output = req
        .values
        .get("--output-dir")
        .ok_or("Import output directory required")?;
    let report =
        crate::units::corpus::dry_run(&req.root, Path::new(output)).map_err(PaperFault::Input)?;
    eprintln!("import dry run: {}", report["fully_decoded_outer"]);
    Ok(())
}

fn run_correspondence(req: &Request) -> Result<()> {
    if req.release
        || req.values.keys().any(|key| {
            !matches!(
                key.as_str(),
                "--repository" | "--assignment" | "--side" | "--output-dir"
            )
        })
    {
        return Err(
            "Import correspondence takes only repository, assignment, side and output directory"
                .into(),
        );
    }
    let assignment = req
        .values
        .get("--assignment")
        .ok_or("Assignment required")?;
    let side = req.values.get("--side").ok_or("Side required")?;
    let output = req
        .values
        .get("--output-dir")
        .ok_or("Output directory required")?;
    let side = side.to_str().ok_or("Non-UTF8 side")?;
    crate::units::corpus::export_correspondence(
        &req.root,
        Path::new(assignment),
        side,
        Path::new(output),
    )
    .map_err(PaperFault::Input)
}

#[cfg(test)]
mod tests;
