//! Strict byte and shape boundary for the documentary authority ledger.
#[cfg(test)]
mod tests;
use super::{DocumentError, input};
use crate::{UnitSource, units::toml};
use ftoo_manifest::{
    DocumentaryEventDraft, JsonValue, LocatedString, PaperEvidenceSite, PaperResolution,
    PaperTagValue, RegistryDiagnostic, RegistryErrors, RegistryFault, RegistryResource,
    RegistryStage, RuleId, ShapeKind, UnitDiagnostic, UnitErrors, UnitFault, UnitStage,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    io::Read,
    path::Path,
    process::{Command, Stdio},
};

pub(super) const FILE: &str = "registry/documentary-tags.toml";
pub(crate) const PAPER_MINTS: &str = "documents/paper-build/label-mints.tsv";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PaperLookupFault {
    AuthorityRefused,
    ProcessFailed,
    ProtocolInvalid { reason: String },
    InputChanged { path: String },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaperLookupError {
    pub rule: RuleId,
    pub site: String,
    pub fault: PaperLookupFault,
}
impl fmt::Display for PaperLookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {}: {:?}", self.rule, self.site, self.fault)
    }
}
impl std::error::Error for PaperLookupError {}
fn protocol(site: &str, reason: impl Into<String>) -> PaperLookupError {
    PaperLookupError {
        rule: RuleId::M063,
        site: site.into(),
        fault: PaperLookupFault::ProtocolInvalid {
            reason: reason.into(),
        },
    }
}
fn fields<'a>(
    value: &'a JsonValue,
    expected: &[&str],
    site: &str,
) -> Result<Vec<&'a JsonValue>, PaperLookupError> {
    let JsonValue::Object(fields) = value else {
        return Err(protocol(site, "expected object"));
    };
    if fields
        .iter()
        .map(|(key, _)| key.as_str())
        .collect::<Vec<_>>()
        != expected
    {
        return Err(protocol(site, "fields or order differ"));
    }
    Ok(fields.iter().map(|(_, value)| value).collect())
}
fn array<'a>(
    value: &'a JsonValue,
    site: &str,
    maximum: usize,
) -> Result<&'a [JsonValue], PaperLookupError> {
    let JsonValue::Array(values) = value else {
        return Err(protocol(site, "expected array"));
    };
    if values.len() > maximum {
        return Err(protocol(site, "array limit exceeded"));
    }
    Ok(values)
}
fn string<'a>(value: &'a JsonValue, site: &str) -> Result<&'a str, PaperLookupError> {
    let JsonValue::String(value) = value else {
        return Err(protocol(site, "expected string"));
    };
    Ok(value)
}
fn positive(value: &JsonValue, site: &str) -> Result<usize, PaperLookupError> {
    let JsonValue::Integer(value) = value else {
        return Err(protocol(site, "expected integer"));
    };
    let number = value
        .parse::<usize>()
        .map_err(|_| protocol(site, "invalid integer"))?;
    if number == 0 || number > 1_000_000 {
        return Err(protocol(site, "line outside bounds"));
    }
    Ok(number)
}
fn path(value: &str, site: &str) -> Result<(), PaperLookupError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.contains('\\')
        || value.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(protocol(site, "invalid relative path"));
    }
    Ok(())
}
fn evidence(value: &JsonValue, site: &str) -> Result<PaperEvidenceSite, PaperLookupError> {
    let fields = fields(value, &["file", "line"], site)?;
    let file = string(fields[0], site)?;
    path(file, site)?;
    Ok(PaperEvidenceSite {
        file: file.into(),
        line: positive(fields[1], site)?,
    })
}
fn optional_evidence(
    value: &JsonValue,
    site: &str,
) -> Result<Option<PaperEvidenceSite>, PaperLookupError> {
    if matches!(value, JsonValue::Null) {
        Ok(None)
    } else {
        evidence(value, site).map(Some)
    }
}
fn checked_file(root: &Path, name: &str) -> Result<Vec<u8>, PaperLookupError> {
    path(name, name)?;
    let mut current = root.to_path_buf();
    for part in name.split('/') {
        current.push(part);
        let metadata =
            std::fs::symlink_metadata(&current).map_err(|_| protocol(name, "input absent"))?;
        if metadata.file_type().is_symlink() {
            return Err(protocol(name, "symlink input"));
        }
    }
    if !current.is_file() {
        return Err(protocol(name, "input is not a file"));
    }
    std::fs::read(current).map_err(|_| protocol(name, "input read failed"))
}
/// A file-checked lookup can only be constructed after the checker and digest handshake.
#[derive(Debug)]
pub(crate) struct CheckedPaperLookup {
    entries: BTreeMap<String, PaperResolution>,
    virtues: BTreeMap<String, VirtueBinding>,
    inputs: BTreeMap<String, Vec<u8>>,
    departures: BTreeMap<String, String>,
    native_targets: BTreeMap<String, (String, PaperEvidenceSite)>,
    result_headers: BTreeMap<String, ResultHeaderBinding>,
    derived_bibliography: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResultHeaderBinding {
    pub key: String,
    pub unit: String,
    pub target: String,
    pub title: String,
    pub tag_site: PaperEvidenceSite,
    pub reverse_site: PaperEvidenceSite,
    pub heading_site: PaperEvidenceSite,
    pub unit_site: PaperEvidenceSite,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VirtueBinding {
    pub word: String,
    pub key: String,
    pub target: String,
    pub class_site: PaperEvidenceSite,
    pub slug_site: PaperEvidenceSite,
}
impl CheckedPaperLookup {
    #[must_use]
    pub(crate) fn resolve(&self, key: &str) -> Option<PaperResolution> {
        self.entries.get(key).cloned()
    }
    #[must_use]
    pub(crate) fn inputs(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.inputs
    }
    #[must_use]
    pub(crate) fn departure(&self, slug: &str) -> Option<&str> {
        self.departures.get(slug).map(String::as_str)
    }
    #[must_use]
    pub(crate) fn native_target(&self, key: &str) -> Option<&(String, PaperEvidenceSite)> {
        self.native_targets.get(key)
    }
    #[must_use]
    pub(crate) fn result_header(&self, key: &str) -> Option<&ResultHeaderBinding> {
        self.result_headers.get(key)
    }
    pub(crate) fn validate_results(
        &self,
        slots: &BTreeMap<String, ftoo_manifest::corpus::CheckedSlot>,
    ) -> Result<(), PaperLookupError> {
        let mut seen = BTreeSet::new();
        for (id, slot) in slots {
            let [ftoo_manifest::corpus::SlotBlock::Result { key, .. }] = slot.blocks() else {
                continue;
            };
            let row = self
                .result_header(key)
                .ok_or_else(|| protocol(key, "result header row absent"))?;
            let source = self
                .inputs
                .get(&row.unit_site.file)
                .ok_or_else(|| protocol(key, "result unit source absent"))?;
            let line = std::str::from_utf8(source)
                .map_err(|_| protocol(key, "result unit source encoding"))?
                .lines()
                .nth(row.unit_site.line.saturating_sub(1));
            if row.unit != *id
                || row.unit_site.file != slot.source()
                || line != Some(format!("key = \"{key}\"").as_str())
                || !matches!(self.entries.get(key), Some(PaperResolution::Available(value))
                    if value.key == *key && value.spelling == *key && value.target == row.target)
                || !seen.insert(key)
            {
                return Err(protocol(key, "result header owner or binding differs"));
            }
        }
        if seen.len() != self.result_headers.len() {
            return Err(protocol(
                "result_headers",
                "result header row has no checked slot",
            ));
        }
        Ok(())
    }
    #[must_use]
    pub(crate) fn derived_bibliography(&self) -> &str {
        &self.derived_bibliography
    }
    pub(crate) fn validate_econ(
        &self,
        corpus: &ftoo_manifest::LanguageCheckedUnits,
    ) -> Result<(), PaperLookupError> {
        let mut seen = BTreeSet::new();
        for unit in corpus.units() {
            let Some(econ) = unit.econ() else {
                continue;
            };
            let key = &econ.target.value;
            let declaration = unit
                .documentary_targets()
                .values()
                .find(|target| target.value == *key)
                .ok_or_else(|| protocol(key, "native target declaration absent"))?;
            let (owner, site) = self
                .native_targets
                .get(key)
                .ok_or_else(|| protocol(key, "native target row absent"))?;
            if owner != unit.label().as_str()
                || site.file != declaration.site.file
                || !seen.insert(key.clone())
            {
                return Err(protocol(key, "native target owner differs"));
            }
            let bytes = self
                .inputs
                .get(&site.file)
                .ok_or_else(|| protocol(key, "native target source absent"))?;
            let ftoo_manifest::SourceAnchor::Bytes(start, _) = &declaration.site.anchor else {
                return Err(protocol(key, "native target has no byte site"));
            };
            if *start > bytes.len()
                || site.line != bytes[..*start].split(|byte| *byte == b'\n').count()
            {
                return Err(protocol(key, "native target field site differs"));
            }
            if !matches!(self.entries.get(key), Some(PaperResolution::Available(value)) if value.target != *key && value.key == *key)
            {
                return Err(protocol(key, "native target binding is not canonical"));
            }
        }
        if seen.len() != self.native_targets.len() {
            return Err(protocol(
                "native_targets",
                "native target row has no admitted unit",
            ));
        }
        Ok(())
    }
    #[must_use]
    pub(crate) fn virtues(&self) -> &BTreeMap<String, VirtueBinding> {
        &self.virtues
    }
    pub(crate) fn check(root: &Path) -> Result<Self, PaperLookupError> {
        Self::check_with(root, "python3")
    }
    #[allow(clippy::too_many_lines)] // The bounded response is validated in wire order before retaining any binding.
    fn check_with(root: &Path, python: &str) -> Result<Self, PaperLookupError> {
        let mut child = Command::new(python)
            .arg("-B")
            .arg(root.join("tools/paper/identities.py"))
            .arg(root)
            .arg("--assembly-json")
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|_| PaperLookupError {
                rule: RuleId::M063,
                site: "tools/paper/identities.py".into(),
                fault: PaperLookupFault::ProcessFailed,
            })?;
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or_else(|| protocol("assembly-json", "stdout unavailable"))?
            .take(4_194_305)
            .read_to_end(&mut bytes)
            .map_err(|_| protocol("assembly-json", "response read failed"))?;
        if bytes.len() > 4_194_304 {
            let _ = child.kill();
            let _ = child.wait();
            return Err(protocol("assembly-json", "response limit exceeded"));
        }
        let status = child
            .wait()
            .map_err(|_| protocol("assembly-json", "process wait failed"))?;
        if !status.success() {
            return Err(PaperLookupError {
                rule: RuleId::M063,
                site: "tools/paper/identities.py".into(),
                fault: PaperLookupFault::AuthorityRefused,
            });
        }
        let value = ftoo_manifest::strict_json(&bytes)
            .map_err(|error| protocol("assembly-json", format!("{error}")))?;
        let top = fields(
            &value,
            &[
                "schema",
                "inputs",
                "bindings",
                "virtues",
                "departures",
                "native_targets",
                "result_headers",
                "bibliography",
            ],
            "assembly-json",
        )?;
        if top[0] != &JsonValue::Integer("2".into()) {
            return Err(protocol("schema", "expected integer two"));
        }
        let mut inputs = BTreeMap::new();
        for row in array(top[1], "inputs", 512)? {
            let values = fields(row, &["path", "sha256"], "inputs")?;
            let name = string(values[0], "inputs.path")?;
            path(name, "inputs.path")?;
            let digest = string(values[1], "inputs.sha256")?;
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(protocol(name, "invalid digest"));
            }
            if inputs
                .keys()
                .next_back()
                .is_some_and(|last: &String| last.as_str() >= name)
            {
                return Err(protocol(name, "input order or duplicate differs"));
            }
            let data = checked_file(root, name)?;
            if crate::hex(&Sha256::digest(&data)) != digest {
                return Err(PaperLookupError {
                    rule: RuleId::M063,
                    site: name.into(),
                    fault: PaperLookupFault::InputChanged { path: name.into() },
                });
            }
            inputs.insert(name.into(), data);
        }
        let mut entries = BTreeMap::new();
        for row in array(top[2], "bindings", 4096)? {
            let value = fields(
                row,
                &[
                    "key",
                    "spelling",
                    "status",
                    "target",
                    "target_kind",
                    "lookup_site",
                    "alias_site",
                    "declaration_sites",
                ],
                "bindings",
            )?;
            let key = string(value[0], "bindings.key")?;
            let spelling = string(value[1], "bindings.spelling")?;
            let status = string(value[2], "bindings.status")?;
            if key.is_empty()
                || spelling.is_empty()
                || entries
                    .keys()
                    .next_back()
                    .is_some_and(|last: &String| last.as_str() >= key)
            {
                return Err(protocol(key, "binding order or duplicate differs"));
            }
            let lookup_site = evidence(value[5], "bindings.lookup_site")?;
            let alias_site = optional_evidence(value[6], "bindings.alias_site")?;
            let declaration_sites = array(value[7], "bindings.declaration_sites", 4096)?
                .iter()
                .map(|site| evidence(site, "bindings.declaration_sites"))
                .collect::<Result<Vec<_>, _>>()?;
            let resolution = match status {
                "paper" | "alias" => {
                    let target = string(value[3], "bindings.target")?;
                    let kind = string(value[4], "bindings.target_kind")?;
                    if target.is_empty()
                        || !matches!(kind, "node" | "evidence")
                        || declaration_sites.is_empty()
                        || (status == "alias") != alias_site.is_some()
                    {
                        return Err(protocol(key, "paper evidence or kind differs"));
                    }
                    PaperResolution::Available(PaperTagValue {
                        key: key.into(),
                        spelling: spelling.into(),
                        target: target.into(),
                        target_kind: kind.into(),
                        lookup_site,
                        alias_site,
                        declaration_sites,
                    })
                }
                "admitted" => {
                    let target = string(value[3], "bindings.target")?;
                    if target != key
                        || value[4] != &JsonValue::String("documentary".into())
                        || alias_site.is_some()
                        || !declaration_sites.is_empty()
                    {
                        return Err(protocol(key, "admitted binding differs"));
                    }
                    PaperResolution::Unavailable {
                        status: status.into(),
                    }
                }
                "candidate" => {
                    if !matches!(value[3], JsonValue::Null)
                        || !matches!(value[4], JsonValue::Null)
                        || alias_site.is_some()
                        || !declaration_sites.is_empty()
                    {
                        return Err(protocol(key, "candidate binding differs"));
                    }
                    PaperResolution::Unavailable {
                        status: status.into(),
                    }
                }
                _ => return Err(protocol(key, "unknown binding status")),
            };
            entries.insert(key.into(), resolution);
        }
        let mut virtues = BTreeMap::new();
        for row in array(top[3], "virtues", 256)? {
            let values = fields(
                row,
                &["word", "key", "target", "class_site", "slug_site"],
                "virtues",
            )?;
            let word = string(values[0], "virtues.word")?;
            let key = string(values[1], "virtues.key")?;
            let target = string(values[2], "virtues.target")?;
            if word.is_empty()
                || !word.bytes().all(|byte| byte.is_ascii_lowercase())
                || key != format!("v:{word}")
                || virtues
                    .keys()
                    .next_back()
                    .is_some_and(|last: &String| last.as_str() >= word)
            {
                return Err(protocol(word, "virtue word, key or order differs"));
            }
            let Some(PaperResolution::Available(binding)) = entries.get(key) else {
                return Err(protocol(word, "virtue lacks a paper binding"));
            };
            if binding.target != target
                || binding.target_kind != "node"
                || binding.alias_site.is_some()
            {
                return Err(protocol(word, "virtue target differs"));
            }
            let class_site = evidence(values[3], "virtues.class_site")?;
            let slug_site = evidence(values[4], "virtues.slug_site")?;
            if class_site.file != "documents/paper/data/ftoo_manifest.tex"
                || slug_site.file != class_site.file
            {
                return Err(protocol(word, "virtue declaration site differs"));
            }
            virtues.insert(
                word.into(),
                VirtueBinding {
                    word: word.into(),
                    key: key.into(),
                    target: target.into(),
                    class_site,
                    slug_site,
                },
            );
        }
        if virtues.is_empty() {
            return Err(protocol("virtues", "virtue roster is empty"));
        }
        let mut departures = BTreeMap::new();
        for row in array(top[4], "departures", 4096)? {
            let values = fields(row, &["slug", "target", "handle", "sites"], "departures")?;
            let slug = string(values[0], "departures.slug")?;
            let target = string(values[1], "departures.target")?;
            let handle = string(values[2], "departures.handle")?;
            if slug.is_empty()
                || target.is_empty()
                || handle != format!("d:{slug}")
                || departures
                    .insert(slug.to_owned(), target.to_owned())
                    .is_some()
                || array(values[3], "departures.sites", 4096)?.is_empty()
            {
                return Err(protocol(slug, "departure evidence differs"));
            }
            for site in array(values[3], "departures.sites", 4096)? {
                evidence(site, "departures.sites")?;
            }
        }
        let mut native_targets = BTreeMap::new();
        for row in array(top[5], "native_targets", 4096)? {
            let values = fields(row, &["key", "unit", "site"], "native_targets")?;
            let key = string(values[0], "native_targets.key")?;
            let unit = string(values[1], "native_targets.unit")?;
            let site = evidence(values[2], "native_targets.site")?;
            if key.is_empty()
                || unit.is_empty()
                || native_targets
                    .insert(key.into(), (unit.into(), site))
                    .is_some()
            {
                return Err(protocol(key, "native target evidence differs"));
            }
        }
        let mut result_headers = BTreeMap::new();
        for row in array(top[6], "result_headers", 4096)? {
            let values = fields(
                row,
                &[
                    "key",
                    "unit",
                    "target",
                    "title",
                    "tag_site",
                    "reverse_site",
                    "heading_site",
                    "unit_site",
                ],
                "result_headers",
            )?;
            let key = string(values[0], "result_headers.key")?;
            let unit = string(values[1], "result_headers.unit")?;
            let target = string(values[2], "result_headers.target")?;
            let title = string(values[3], "result_headers.title")?;
            let tag_site = evidence(values[4], "result_headers.tag_site")?;
            let reverse_site = evidence(values[5], "result_headers.reverse_site")?;
            let heading_site = evidence(values[6], "result_headers.heading_site")?;
            let unit_site = evidence(values[7], "result_headers.unit_site")?;
            if key.is_empty()
                || unit.is_empty()
                || target.is_empty()
                || title.trim().is_empty()
                || heading_site.file != "documents/paper/data/ftoo_manifest.tex"
                || tag_site.file != heading_site.file
                || reverse_site.file != heading_site.file
                || !unit_site.file.starts_with("units/corpus/")
                || std::path::Path::new(&unit_site.file).extension()
                    != Some(std::ffi::OsStr::new("toml"))
                || !inputs.contains_key(&unit_site.file)
                || result_headers
                    .insert(
                        key.into(),
                        ResultHeaderBinding {
                            key: key.into(),
                            unit: unit.into(),
                            target: target.into(),
                            title: title.into(),
                            tag_site,
                            reverse_site,
                            heading_site,
                            unit_site,
                        },
                    )
                    .is_some()
            {
                return Err(protocol(key, "result header evidence differs"));
            }
        }
        let bibliography = fields(
            top[7],
            &["source", "stock_sha256", "derived_sha256", "derived_utf8"],
            "bibliography",
        )?;
        let source = string(bibliography[0], "bibliography.source")?;
        if source != "documents/paper/references.bib" {
            return Err(protocol(source, "bibliography source differs"));
        }
        let stock = inputs
            .get(source)
            .ok_or_else(|| protocol(source, "bibliography input absent"))?;
        let stock_digest = string(bibliography[1], "bibliography.stock_sha256")?;
        let derived_digest = string(bibliography[2], "bibliography.derived_sha256")?;
        let derived_bibliography = string(bibliography[3], "bibliography.derived_utf8")?;
        if crate::hex(&Sha256::digest(stock)) != stock_digest
            || crate::hex(&Sha256::digest(derived_bibliography.as_bytes())) != derived_digest
        {
            return Err(protocol(source, "bibliography digest differs"));
        }
        Ok(Self {
            entries,
            virtues,
            inputs,
            departures,
            native_targets,
            result_headers,
            derived_bibliography: derived_bibliography.into(),
        })
    }
}

pub(crate) fn paper_mint_rows(data: &[u8]) -> Result<Vec<(String, String)>, DocumentError> {
    let fault = |line| DocumentError::Input(super::DocumentInputFault::PaperMintRow { line });
    let text = std::str::from_utf8(data).map_err(|_| fault(None))?;
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with('#'))
        .map(|(index, line)| {
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != 2 || fields.iter().any(|field| field.is_empty()) {
                return Err(fault(Some(index + 1)));
            }
            Ok((fields[0].into(), fields[1].into()))
        })
        .collect()
}
pub(crate) const PAPER_TAGS: &str = "documents/paper-build/paper-tags.tsv";
/// The paper's tag lookup as `(spelling, target, kind, line)` rows; a malformed row names its line.
pub(crate) fn paper_tag_rows(data: &[u8]) -> Result<Vec<(String, String, String, usize)>, usize> {
    let text = std::str::from_utf8(data).map_err(|_| 0_usize)?;
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with('#'))
        .map(|(index, line)| {
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != 3 || fields.iter().any(|f| f.is_empty()) {
                return Err(index + 1);
            }
            Ok((
                fields[0].into(),
                fields[1].into(),
                fields[2].into(),
                index + 1,
            ))
        })
        .collect()
}
pub(crate) fn read(root: &std::path::Path) -> Result<UnitSource, DocumentError> {
    use std::io::Read;
    let path = root.join(FILE);
    let metadata = std::fs::symlink_metadata(&path)
        .map_err(|e| input::io_error(super::DocumentIoOperation::Inspect, &path, &e))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DocumentError::Input(
            super::DocumentInputFault::UnexpectedEntry {
                path,
                kind: if metadata.file_type().is_symlink() {
                    super::DocumentEntryKind::Symlink
                } else if metadata.is_dir() {
                    super::DocumentEntryKind::Directory
                } else {
                    super::DocumentEntryKind::Other
                },
            },
        ));
    }
    if metadata.len() > 1_048_576 {
        return Err(limit(input::site(FILE), RegistryResource::Bytes, 1_048_576));
    }
    let file = std::fs::File::open(&path)
        .map_err(|e| input::io_error(super::DocumentIoOperation::Read, &path, &e))?;
    let mut bytes = vec![];
    file.take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|e| input::io_error(super::DocumentIoOperation::Read, &path, &e))?;
    if bytes.len() > 1_048_576 {
        return Err(limit(input::site(FILE), RegistryResource::Bytes, 1_048_576));
    }
    Ok(UnitSource {
        file: FILE.into(),
        bytes,
    })
}

fn boundary(error: Box<UnitDiagnostic>) -> DocumentError {
    DocumentError::Boundary(UnitErrors::from_diagnostics(vec![*error]).expect("one diagnostic"))
}
fn limit(
    site: ftoo_manifest::UnitSite,
    resource: RegistryResource,
    maximum: usize,
) -> DocumentError {
    DocumentError::DocumentaryRegistry(
        RegistryErrors::from_diagnostics(vec![RegistryDiagnostic {
            rule: RuleId::M060,
            stage: RegistryStage::Source,
            site,
            fault: RegistryFault::Limit {
                resource,
                limit: maximum,
                actual: maximum + 1,
            },
        }])
        .expect("one diagnostic"),
    )
}
#[allow(clippy::too_many_lines)] // Keep the closed event schema and its field provenance together.
pub(crate) fn load(source: &UnitSource) -> Result<Vec<DocumentaryEventDraft>, DocumentError> {
    if source.bytes.len() > 1_048_576 {
        return Err(limit(
            input::site(&source.file),
            RegistryResource::Bytes,
            1_048_576,
        ));
    }
    let text = toml::encoding(source).map_err(boundary)?;
    let tree = toml::parse_array_bounded(&source.file, text, "events", 4096)
        .map_err(boundary)?
        .map_err(|site| limit(site, RegistryResource::Events, 4096))?;
    toml::numeric(&tree, text).map_err(boundary)?;
    let fields = toml::fixed(&tree, &["schema", "events"]).map_err(boundary)?;
    toml::typed(fields[0], ShapeKind::Integer).map_err(boundary)?;
    if toml::raw(fields[0], text) != "1" {
        return Err(boundary(Box::new(UnitDiagnostic {
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
    let events = toml::typed(fields[1], ShapeKind::Array).map_err(boundary)?;
    if events.elements.len() > 4096 {
        return Err(limit(events.site.clone(), RegistryResource::Events, 4096));
    }
    events
        .elements
        .iter()
        .map(|event| {
            toml::typed(event, ShapeKind::Table).map_err(boundary)?;
            let change = event
                .fields
                .iter()
                .find(|(key, _, _)| key == "change")
                .map(|(_, _, value)| value);
            let Some(change) = change else {
                return Err(boundary(Box::new(UnitDiagnostic {
                    rule: RuleId::M004,
                    stage: UnitStage::Shape,
                    unit: None,
                    language: None,
                    site: event.site.parent(),
                    fault: UnitFault::MissingField {
                        name: "change".into(),
                    },
                })));
            };
            let change = toml::string(change).map_err(boundary)?;
            let admit = match change.value.as_str() {
                "admit" => true,
                "retire" => false,
                _ => {
                    return Err(boundary(Box::new(UnitDiagnostic {
                        rule: RuleId::M004,
                        stage: UnitStage::Shape,
                        unit: None,
                        language: None,
                        site: change.site,
                        fault: UnitFault::UnknownToken {
                            value: change.value,
                            expected: vec!["admit".into(), "retire".into()],
                        },
                    })));
                }
            };
            let keys: &[&str] = if admit {
                &[
                    "ordinal",
                    "change",
                    "handle",
                    "authority-record",
                    "reason",
                    "label",
                ]
            } else {
                &["ordinal", "change", "handle", "authority-record", "reason"]
            };
            let fields = toml::fixed(event, keys).map_err(boundary)?;
            toml::typed(fields[0], ShapeKind::Integer).map_err(boundary)?;
            Ok(DocumentaryEventDraft {
                site: event.site.clone(),
                ordinal: LocatedString {
                    value: toml::raw(fields[0], text).into(),
                    site: fields[0].site.clone(),
                },
                handle: toml::string(fields[2]).map_err(boundary)?,
                authority_record: toml::string(fields[3]).map_err(boundary)?,
                reason: toml::string(fields[4]).map_err(boundary)?,
                label: if admit {
                    Some(toml::string(fields[5]).map_err(boundary)?)
                } else {
                    None
                },
            })
        })
        .collect()
}
