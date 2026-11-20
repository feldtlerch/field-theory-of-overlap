//! The open-question store's strict loader, its resolution context and the
//! byte-gated register that one writer generates and checks (M067 to M071).
#[cfg(test)]
mod tests;
use crate::{
    LabelFileError, UnitLoadError, UnitSource, load_markdown_files, load_unit_files,
    units::toml::{self, Node},
};
use ftoo_manifest::{
    LabelFault, LabelSourceError, LocatedString, MarkdownSource, OpenQuestionDraft, OpenQuestions,
    QuestionConcern, QuestionDomains, QuestionErrors, QuestionReference, QuestionState,
    QuestionStateKind, ReferenceDraft, ReferenceKind, RuleId, ShapeKind, SourceAnchor, StateDraft,
    UnitDiagnostic, UnitErrors, UnitFault, UnitSite, UnitStage, admit_open_questions,
    check_markdown_sources, check_paper_tag_targets, collect_joint_mints,
};
use std::{
    collections::BTreeMap,
    fmt::{self, Write as _},
    fs,
    io::{self, Read as _, Write as _},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// The authored store.
pub const OPEN_QUESTION_STORE: &str = "registry/open-questions.toml";
/// The generated register.
pub const OPEN_QUESTION_REGISTER: &str = "generated/open-questions.md";
const MAX_BYTES: usize = 1_048_576;
const MAX_RECORDS: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuestionIoOperation {
    Inspect,
    Read,
    CreateTemporary,
    Write,
    Rename,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuestionInputFault {
    NotRegularFile { path: PathBuf },
    Limit { path: PathBuf, limit: usize },
    PaperTagRow { line: usize },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegisterFault {
    Missing {
        path: PathBuf,
    },
    NonCanonicalBytes {
        path: PathBuf,
        offset: usize,
        expected_length: usize,
        actual_length: usize,
    },
    AddressSyntax {
        mints: Vec<String>,
        citations: Vec<String>,
    },
}
/// A complete refusal of the store, its context or its register.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OpenQuestionError {
    Input(QuestionInputFault),
    Io {
        operation: QuestionIoOperation,
        path: PathBuf,
        kind: io::ErrorKind,
    },
    Boundary(UnitErrors),
    Units(UnitLoadError),
    Labels(LabelFileError),
    PaperMints(String),
    Admission(QuestionErrors),
    Register {
        rule: RuleId,
        fault: RegisterFault,
    },
    AfterReplacement(Box<Self>),
}
impl fmt::Display for OpenQuestionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boundary(errors) => write!(f, "{errors}"),
            Self::Units(error) => write!(f, "{error}"),
            Self::Labels(error) => write!(f, "{error}"),
            Self::Admission(errors) => write!(f, "{errors}"),
            Self::AfterReplacement(error) => write!(f, "register was replaced: {error}"),
            other => write!(f, "{other:?}"),
        }
    }
}
impl std::error::Error for OpenQuestionError {}
/// Counts of a checked store, as the register states them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenQuestionSummary {
    pub records: usize,
    pub states: [usize; 4],
    pub concerns: [usize; 4],
}

fn io_error(operation: QuestionIoOperation, path: &Path, error: &io::Error) -> OpenQuestionError {
    OpenQuestionError::Io {
        operation,
        path: path.into(),
        kind: error.kind(),
    }
}
fn read(root: &Path, file: &str) -> Result<Vec<u8>, OpenQuestionError> {
    let path = root.join(file);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|e| io_error(QuestionIoOperation::Inspect, &path, &e))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(OpenQuestionError::Input(
            QuestionInputFault::NotRegularFile { path },
        ));
    }
    let mut bytes = vec![];
    fs::File::open(&path)
        .and_then(|file| file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes))
        .map_err(|e| io_error(QuestionIoOperation::Read, &path, &e))?;
    if bytes.len() > MAX_BYTES {
        return Err(OpenQuestionError::Input(QuestionInputFault::Limit {
            path,
            limit: MAX_BYTES,
        }));
    }
    Ok(bytes)
}
fn shape(error: Box<UnitDiagnostic>) -> OpenQuestionError {
    let mut error = *error;
    if error.rule == RuleId::M004 {
        error.rule = RuleId::M067;
    }
    OpenQuestionError::Boundary(UnitErrors::from_diagnostics(vec![error]).expect("one diagnostic"))
}
fn fault(site: UnitSite, fault: UnitFault) -> OpenQuestionError {
    shape(Box::new(UnitDiagnostic {
        rule: RuleId::M067,
        stage: UnitStage::Shape,
        unit: None,
        language: None,
        site,
        fault,
    }))
}
fn field<'a>(node: &'a Node, key: &str) -> Option<&'a Node> {
    node.fields
        .iter()
        .find(|(name, _, _)| name == key)
        .map(|(_, _, value)| value)
}
fn token<T: Copy>(
    node: &Node,
    parse: impl Fn(&str) -> Option<T>,
    names: &[&str],
) -> Result<T, OpenQuestionError> {
    let value = toml::string(node).map_err(shape)?;
    parse(&value.value).ok_or_else(|| {
        fault(
            value.site,
            UnitFault::UnknownToken {
                value: value.value.clone(),
                expected: names.iter().map(|name| (*name).to_owned()).collect(),
            },
        )
    })
}
fn references(node: &Node, empty: bool) -> Result<Vec<ReferenceDraft>, OpenQuestionError> {
    let kinds: Vec<_> = ReferenceKind::ALL.iter().map(|k| k.name()).collect();
    let array = toml::typed(node, ShapeKind::Array).map_err(shape)?;
    if array.elements.is_empty() && !empty {
        return Err(fault(
            array.site.clone(),
            UnitFault::MissingField {
                name: kinds.join("|"),
            },
        ));
    }
    array
        .elements
        .iter()
        .map(|element| {
            toml::typed(element, ShapeKind::Table).map_err(shape)?;
            let Some((key, key_site, value)) = element.fields.first() else {
                return Err(fault(
                    element.site.clone(),
                    UnitFault::MissingField {
                        name: kinds.join("|"),
                    },
                ));
            };
            let Some(kind) = ReferenceKind::parse(key) else {
                return Err(fault(
                    key_site.clone(),
                    UnitFault::UnknownField {
                        name: key.clone(),
                        expected: kinds.iter().map(|k| (*k).to_owned()).collect(),
                    },
                ));
            };
            if let Some((extra, extra_site, _)) = element.fields.get(1) {
                return Err(fault(
                    extra_site.clone(),
                    UnitFault::UnknownField {
                        name: extra.clone(),
                        expected: vec![key.clone()],
                    },
                ));
            }
            Ok(ReferenceDraft {
                kind,
                value: toml::string(value).map_err(shape)?,
            })
        })
        .collect()
}
fn labels(node: &Node) -> Result<Vec<LocatedString>, OpenQuestionError> {
    let array = toml::typed(node, ShapeKind::Array).map_err(shape)?;
    if array.elements.is_empty() {
        return Err(fault(
            array.site.clone(),
            UnitFault::MissingField {
                name: "label".into(),
            },
        ));
    }
    array
        .elements
        .iter()
        .map(|element| toml::string(element).map_err(shape))
        .collect()
}
fn record(node: &Node) -> Result<OpenQuestionDraft, OpenQuestionError> {
    toml::typed(node, ShapeKind::Table).map_err(shape)?;
    let states: Vec<_> = QuestionStateKind::ALL.iter().map(|s| s.name()).collect();
    let Some(state_node) = field(node, "state") else {
        return Err(fault(
            node.site.parent(),
            UnitFault::MissingField {
                name: "state".into(),
            },
        ));
    };
    let state = token(state_node, QuestionStateKind::parse, &states)?;
    let mut required = vec![
        "label",
        "title",
        "concern",
        "question",
        "about",
        "settled-when",
        "state",
    ];
    let mut optional = vec!["blocks", "rests-on"];
    match state {
        QuestionStateKind::Open => {}
        QuestionStateKind::Settled => required.extend(["settled-by", "answer"]),
        QuestionStateKind::Succeeded => {
            required.push("succeeded-by");
            optional.push("answer");
        }
        QuestionStateKind::Withdrawn => required.push("withdrawn-because"),
    }
    let allowed: Vec<_> = required.iter().chain(&optional).copied().collect();
    let mut unknown: Vec<_> = node
        .fields
        .iter()
        .filter(|(key, _, _)| !allowed.contains(&key.as_str()))
        .collect();
    unknown.sort_by_key(|(_, site, _)| match site.anchor {
        SourceAnchor::Bytes(a, _) => a,
        _ => usize::MAX,
    });
    if let Some((key, site, _)) = unknown.first() {
        return Err(fault(
            site.clone(),
            UnitFault::UnknownField {
                name: key.clone(),
                expected: allowed.iter().map(|k| (*k).to_owned()).collect(),
            },
        ));
    }
    if let Some(missing) = required.iter().find(|key| field(node, key).is_none()) {
        return Err(fault(
            node.site.parent(),
            UnitFault::MissingField {
                name: (*missing).to_owned(),
            },
        ));
    }
    let text = |key: &str| toml::string(field(node, key).expect("required field")).map_err(shape);
    let concerns: Vec<_> = QuestionConcern::ALL.iter().map(|c| c.name()).collect();
    let optional_references =
        |key: &str| field(node, key).map_or(Ok(vec![]), |list| references(list, false));
    let state = match state {
        QuestionStateKind::Open => StateDraft::Open,
        QuestionStateKind::Settled => StateDraft::Settled {
            by: references(field(node, "settled-by").expect("required field"), false)?,
            answer: text("answer")?,
        },
        QuestionStateKind::Succeeded => StateDraft::Succeeded {
            by: references(field(node, "succeeded-by").expect("required field"), false)?,
            answer: field(node, "answer")
                .map(|answer| toml::string(answer).map_err(shape))
                .transpose()?,
        },
        QuestionStateKind::Withdrawn => StateDraft::Withdrawn {
            because: text("withdrawn-because")?,
        },
    };
    Ok(OpenQuestionDraft {
        site: node.site.clone(),
        label: text("label")?,
        title: text("title")?,
        concern: token(
            field(node, "concern").expect("required field"),
            QuestionConcern::parse,
            &concerns,
        )?,
        question: text("question")?,
        // An empty about reaches admission, which refuses it under M069.
        about: references(field(node, "about").expect("required field"), true)?,
        settled_when: text("settled-when")?,
        blocks: optional_references("blocks")?,
        rests_on: field(node, "rests-on").map_or(Ok(vec![]), labels)?,
        state,
    })
}
/// Load the store's records strictly: bytes, syntax and numbers first, then shape.
pub(crate) fn load(source: &UnitSource) -> Result<Vec<OpenQuestionDraft>, OpenQuestionError> {
    let text = toml::encoding(source).map_err(shape)?;
    let tree = toml::parse_array_bounded(&source.file, text, "question", MAX_RECORDS)
        .map_err(shape)?
        .map_err(|site| {
            fault(
                site,
                UnitFault::UnknownToken {
                    value: "more records".into(),
                    expected: vec![format!("at most {MAX_RECORDS} records")],
                },
            )
        })?;
    toml::numeric(&tree, text).map_err(shape)?;
    let fields = toml::fixed(&tree, &["schema", "question"]).map_err(shape)?;
    toml::typed(fields[0], ShapeKind::Integer).map_err(shape)?;
    if toml::raw(fields[0], text) != "1" {
        return Err(shape(Box::new(UnitDiagnostic {
            rule: RuleId::M003,
            stage: UnitStage::Shape,
            unit: None,
            language: None,
            site: fields[0].site.clone(),
            fault: UnitFault::SchemaValue {
                expected: "1".into(),
                actual: toml::raw(fields[0], text).into(),
            },
        })));
    }
    toml::typed(fields[1], ShapeKind::Array)
        .map_err(shape)?
        .elements
        .iter()
        .map(record)
        .collect()
}
/// A path the filesystem may be asked about: relative, plain components only.
fn probe(value: &str) -> bool {
    let path = Path::new(value);
    !value.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_)))
}
/// Every ancestor and the file itself are entries of their own, not symlinks.
fn regular(root: &Path, value: &str) -> bool {
    let mut path = root.to_path_buf();
    let parts: Vec<_> = Path::new(value).components().collect();
    parts.iter().enumerate().all(|(index, part)| {
        path.push(part);
        fs::symlink_metadata(&path).is_ok_and(|m| {
            !m.file_type().is_symlink()
                && if index + 1 == parts.len() {
                    m.is_file()
                } else {
                    m.is_dir()
                }
        })
    })
}
struct Prepared {
    store: OpenQuestions,
    bytes: Vec<u8>,
}
fn prepare(root: &Path) -> Result<Prepared, OpenQuestionError> {
    let source = UnitSource {
        file: OPEN_QUESTION_STORE.into(),
        bytes: read(root, OPEN_QUESTION_STORE)?,
    };
    let drafts = load(&source)?;
    let corpus = load_unit_files(root).map_err(OpenQuestionError::Units)?;
    let markdown = load_markdown_files(root).map_err(OpenQuestionError::Labels)?;
    let mint_rows = crate::documents::registry::paper_mint_rows(&read(
        root,
        crate::documents::registry::PAPER_MINTS,
    )?)
    .map_err(|e| OpenQuestionError::PaperMints(e.to_string()))?;
    let tag_rows = crate::documents::registry::paper_tag_rows(&read(
        root,
        crate::documents::registry::PAPER_TAGS,
    )?)
    .map_err(|line| OpenQuestionError::Input(QuestionInputFault::PaperTagRow { line }))?;
    let labels = |e| OpenQuestionError::Labels(LabelFileError::Labels(e));
    check_paper_tag_targets(
        crate::documents::registry::PAPER_TAGS,
        &tag_rows
            .iter()
            .map(|(key, target, _, line)| (key.clone(), target.clone(), *line))
            .collect::<Vec<_>>(),
    )
    .map_err(labels)?;
    let joint = collect_joint_mints(&corpus, &markdown)
        .map_err(labels)?
        .with_paper_mints(mint_rows.clone())
        .map_err(labels)?;
    let files = drafts
        .iter()
        .flat_map(|draft| {
            let closure = match &draft.state {
                StateDraft::Settled { by, .. } | StateDraft::Succeeded { by, .. } => by.as_slice(),
                StateDraft::Open | StateDraft::Withdrawn { .. } => &[],
            };
            draft.about.iter().chain(&draft.blocks).chain(closure)
        })
        .filter(|reference| reference.kind == ReferenceKind::File)
        .map(|reference| reference.value.value.as_str())
        .filter(|value| probe(value) && regular(root, value))
        .map(str::to_owned)
        .collect();
    let domains = QuestionDomains {
        records: markdown.mints().iter().map(|m| m.label.clone()).collect(),
        units: corpus
            .units()
            .iter()
            .map(|unit| unit.label().as_str().to_owned())
            .collect(),
        paper: tag_rows
            .iter()
            .map(|(_, target, _, _)| target.clone())
            .chain(mint_rows.iter().map(|(key, _)| key.clone()))
            .collect(),
        files,
    };
    let store = admit_open_questions(&source.file, drafts, &domains)
        .map_err(OpenQuestionError::Admission)?;
    joint.with_open_questions(&store).map_err(labels)?;
    let mut tags = BTreeMap::<String, Vec<String>>::new();
    for (spelling, target, _, _) in tag_rows {
        tags.entry(target).or_default().push(spelling);
    }
    let bytes = register(&store, &tags).into_bytes();
    address_syntax(&bytes)?;
    Ok(Prepared { store, bytes })
}
/// The register contributes no Markdown mint and no citation to any label harvest.
fn address_syntax(bytes: &[u8]) -> Result<(), OpenQuestionError> {
    let harvest = check_markdown_sources(vec![MarkdownSource {
        file: OPEN_QUESTION_REGISTER.into(),
        bytes: bytes.to_vec(),
    }]);
    let (mints, citations) = match &harvest {
        Ok(checked) => (
            checked.mints().iter().map(|m| m.label.clone()).collect(),
            checked
                .citations()
                .iter()
                .map(|c| c.label.clone())
                .collect(),
        ),
        Err(LabelSourceError::Admission(errors)) => {
            let mut mints = vec![];
            let mut citations = vec![];
            for diagnostic in errors.diagnostics() {
                match &diagnostic.fault {
                    LabelFault::DuplicateMint { label, .. } => mints.push(label.clone()),
                    LabelFault::UnresolvedMarkdownCitation { label, .. } => {
                        citations.push(label.clone());
                    }
                    other => citations.push(other.to_string()),
                }
            }
            (mints, citations)
        }
        Err(other) => (vec![], vec![other.to_string()]),
    };
    if harvest.is_err() || !mints.is_empty() || !citations.is_empty() {
        return Err(OpenQuestionError::Register {
            rule: RuleId::M071,
            fault: RegisterFault::AddressSyntax { mints, citations },
        });
    }
    Ok(())
}
fn subject(reference: &QuestionReference, tags: &BTreeMap<String, Vec<String>>) -> String {
    match reference {
        QuestionReference::Node(id) => format!("graph node `{}`", id.semantic_name()),
        QuestionReference::Rule(rule) => format!("rule {rule:?}"),
        QuestionReference::Unit(label) => format!("unit `{label}`"),
        QuestionReference::Record(label) => format!("record `{label}`"),
        QuestionReference::Paper(label) => match tags.get(label) {
            Some(spellings) => format!("paper object {}, `{label}`", spellings.join(" or ")),
            None => format!("paper label `{label}`"),
        },
        QuestionReference::File(path) => format!("file `{path}`"),
        QuestionReference::Open(label) => format!("open question `{label}`"),
    }
}
fn subjects(references: &[QuestionReference], tags: &BTreeMap<String, Vec<String>>) -> String {
    references
        .iter()
        .map(|reference| subject(reference, tags))
        .collect::<Vec<_>>()
        .join("; ")
}
fn summary(store: &OpenQuestions) -> OpenQuestionSummary {
    let mut summary = OpenQuestionSummary {
        records: store.records().len(),
        states: [0; 4],
        concerns: [0; 4],
    };
    for record in store.records() {
        let state = QuestionStateKind::ALL
            .iter()
            .position(|s| *s == record.state().kind())
            .expect("closed state");
        let concern = QuestionConcern::ALL
            .iter()
            .position(|c| *c == record.concern())
            .expect("closed concern");
        summary.states[state] += 1;
        summary.concerns[concern] += 1;
    }
    summary
}
/// The register's one writer: generated counts, summary, records and subject index.
fn register(store: &OpenQuestions, tags: &BTreeMap<String, Vec<String>>) -> String {
    let counts = summary(store);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "<!-- generated open-question register; edits are refused by the byte gate -->"
    );
    let _ = writeln!(out, "# Open questions\n");
    let _ = writeln!(
        out,
        "Generated from `{OPEN_QUESTION_STORE}`; it lists the records the store holds, not every open question of the theory, and its subject index lists what each record is about.\n"
    );
    let states = QuestionStateKind::ALL
        .iter()
        .zip(counts.states)
        .map(|(state, count)| format!("{} {count}", state.name()))
        .collect::<Vec<_>>()
        .join(", ");
    let concerns = QuestionConcern::ALL
        .iter()
        .zip(counts.concerns)
        .map(|(concern, count)| format!("{} {count}", concern.name()))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "Records: {}. By state: {states}.", counts.records);
    let _ = writeln!(out, "By concern: {concerns}.\n");
    let _ = writeln!(out, "| Record | Title | Concern | State |");
    let _ = writeln!(out, "| --- | --- | --- | --- |");
    for record in store.records() {
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} |",
            record.label(),
            record.title(),
            record.concern().name(),
            record.state().kind().name()
        );
    }
    let mut index = BTreeMap::<&QuestionReference, Vec<&str>>::new();
    for record in store.records() {
        let _ = writeln!(out, "\n## {}\n", record.title());
        let _ = writeln!(
            out,
            "Record `{}`, {}, {}.\n",
            record.label(),
            record.concern().name(),
            record.state().kind().name()
        );
        let _ = writeln!(out, "{}\n", record.question());
        let _ = writeln!(out, "- About: {}.", subjects(record.about(), tags));
        if !record.blocks().is_empty() {
            let _ = writeln!(out, "- Blocks: {}.", subjects(record.blocks(), tags));
        }
        if !record.rests_on().is_empty() {
            let rests: Vec<_> = record
                .rests_on()
                .iter()
                .map(|label| QuestionReference::Open(label.clone()))
                .collect();
            let _ = writeln!(out, "- Rests on: {}.", subjects(&rests, tags));
        }
        let _ = writeln!(out, "- Settled when: {}", record.settled_when());
        match record.state() {
            QuestionState::Open => {}
            QuestionState::Settled(settlement) => {
                let _ = writeln!(out, "- Settled by: {}.", subjects(settlement.by(), tags));
                let _ = writeln!(out, "- Answer: {}", settlement.answer());
            }
            QuestionState::Succeeded(succession) => {
                let _ = writeln!(out, "- Succeeded by: {}.", subjects(succession.by(), tags));
                if let Some(answer) = succession.answer() {
                    let _ = writeln!(out, "- Answer: {answer}");
                }
            }
            QuestionState::Withdrawn(withdrawal) => {
                let _ = writeln!(out, "- Withdrawn because: {}", withdrawal.because());
            }
        }
        for reference in record.about() {
            index.entry(reference).or_default().push(record.label());
        }
    }
    let _ = writeln!(out, "\n## Subjects\n");
    let _ = writeln!(out, "| Subject | Records |");
    let _ = writeln!(out, "| --- | --- |");
    for (reference, labels) in index {
        let labels: Vec<_> = labels.iter().map(|label| format!("`{label}`")).collect();
        let _ = writeln!(
            out,
            "| {} | {} |",
            subject(reference, tags),
            labels.join(", ")
        );
    }
    out
}
fn verify(path: &Path, actual: &[u8], expected: &[u8]) -> Result<(), OpenQuestionError> {
    if actual == expected {
        return Ok(());
    }
    let offset = actual
        .iter()
        .zip(expected)
        .position(|(a, e)| a != e)
        .unwrap_or(actual.len().min(expected.len()));
    Err(OpenQuestionError::Register {
        rule: RuleId::M071,
        fault: RegisterFault::NonCanonicalBytes {
            path: path.into(),
            offset,
            expected_length: expected.len(),
            actual_length: actual.len(),
        },
    })
}
/// Check the store and the committed register without writing anything.
/// # Errors
/// Reports input, boundary, namespace, admission or register byte-gate faults.
pub fn check_open_questions(
    root: impl AsRef<Path>,
) -> Result<OpenQuestionSummary, OpenQuestionError> {
    let root = root.as_ref();
    let prepared = prepare(root)?;
    let path = root.join(OPEN_QUESTION_REGISTER);
    let actual = match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(OpenQuestionError::Register {
                rule: RuleId::M071,
                fault: RegisterFault::Missing { path },
            });
        }
        _ => read(root, OPEN_QUESTION_REGISTER)?,
    };
    verify(&path, &actual, &prepared.bytes)?;
    Ok(summary(&prepared.store))
}
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);
/// Write the register through the same preparation and writer, then verify it.
/// # Errors
/// Errors before replacement leave the register untouched; errors after it say so.
pub fn generate_open_questions(
    root: impl AsRef<Path>,
) -> Result<OpenQuestionSummary, OpenQuestionError> {
    let root = root.as_ref();
    let prepared = prepare(root)?;
    let path = root.join(OPEN_QUESTION_REGISTER);
    let temporary = root.join(format!(
        "generated/.open-questions-{}-{}.tmp",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    let written = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| io_error(QuestionIoOperation::CreateTemporary, &temporary, &e))
        .and_then(|mut file| {
            file.write_all(&prepared.bytes)
                .map_err(|e| io_error(QuestionIoOperation::Write, &temporary, &e))
        })
        .and_then(|()| {
            fs::rename(&temporary, &path)
                .map_err(|e| io_error(QuestionIoOperation::Rename, &path, &e))
        });
    if let Err(error) = written {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    check_open_questions(root).map_err(|e| OpenQuestionError::AfterReplacement(Box::new(e)))
}
#[cfg(test)]
pub(crate) fn render_for_test(
    store: &OpenQuestions,
    tags: &BTreeMap<String, Vec<String>>,
) -> String {
    register(store, tags)
}
#[cfg(test)]
pub(crate) fn probe_for_test(root: &Path, value: &str) -> bool {
    probe(value) && regular(root, value)
}
