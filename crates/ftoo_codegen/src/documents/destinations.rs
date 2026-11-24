//! Checked screen destinations for referenced content that has no body yet.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Reading {
    Paper,
    Explainer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Address {
    Heading { override_fragment: Option<String> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Target {
    pub document: Reading,
    pub label: String,
    pub decoded_title: String,
    pub address: Address,
    pub available: bool,
    pub locator: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Reference {
    pub document: Reading,
    pub position: usize,
    pub target: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct IndexEntry {
    pub document: Reading,
    pub language: String,
    pub fragment: String,
    pub title: String,
    pub locator: String,
    pub message: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum DestinationError {
    DuplicateTarget(String),
    UnknownTarget(String),
    InvalidFragment(String),
    DuplicateFragment { document: Reading, fragment: String },
    MissingBody { document: Reading, fragment: String },
    ReaderLabel(String),
    MissingMessage(String),
    MissingHeading(String),
    MissingSource(String),
    UnsupportedOutline(String),
    ReleaseUnavailable { entries: usize, gaps: bool },
}

fn message(language: &str) -> Option<&'static str> {
    (language == "en").then_some("Not yet available in this rendering.")
}

pub(super) fn valid_fragment(fragment: &str) -> bool {
    fragment
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_lowercase)
        && fragment
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && fragment
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn title_fragment(title: &str) -> Option<String> {
    let mut fragment = String::new();
    let mut separator = false;
    for scalar in title.chars() {
        if scalar.is_ascii_alphanumeric() {
            if separator && !fragment.is_empty() {
                fragment.push('-');
            }
            fragment.push(scalar.to_ascii_lowercase());
            separator = false;
        } else if !matches!(scalar, '\'' | '’') {
            if scalar.is_whitespace() || scalar.is_ascii_punctuation() {
                separator = true;
            } else {
                return None;
            }
        }
    }
    valid_fragment(&fragment).then_some(fragment)
}

#[derive(Clone, Copy)]
enum HeadingNumber {
    Section(i32),
    Subsection(i32, i32),
    Appendix(u8),
    AppendixSubsection,
}

struct Heading {
    number: HeadingNumber,
    title: String,
    file: String,
    line: usize,
}

struct Outline<'a> {
    label: &'a str,
    document: Reading,
    section: i32,
    subsection: i32,
    appendix: Option<u8>,
    pending: Option<Heading>,
    found: Option<(Target, String)>,
    located_errors: bool,
}

impl Outline<'_> {
    fn error(&self, file: &str, line: usize) -> DestinationError {
        let detail = if self.located_errors {
            format!("{} at {file}:{line}", self.label)
        } else {
            self.label.into()
        };
        DestinationError::UnsupportedOutline(detail)
    }

    fn heading(
        &mut self,
        kind: &str,
        title: &str,
        file: &str,
        line: usize,
    ) -> Result<(), DestinationError> {
        let number = match kind {
            "section" if self.appendix.is_some() => {
                let next = self
                    .appendix
                    .unwrap()
                    .checked_add(1)
                    .filter(|n| *n <= 26)
                    .ok_or_else(|| self.error(file, line))?;
                self.appendix = Some(next);
                self.subsection = 0;
                HeadingNumber::Appendix(next)
            }
            "section" => {
                self.section = self
                    .section
                    .checked_add(1)
                    .ok_or_else(|| self.error(file, line))?;
                self.subsection = 0;
                HeadingNumber::Section(self.section)
            }
            "subsection" if self.appendix.is_some() => {
                self.subsection = self
                    .subsection
                    .checked_add(1)
                    .ok_or_else(|| self.error(file, line))?;
                HeadingNumber::AppendixSubsection
            }
            "subsection" => {
                if self.section < 0 {
                    return Err(self.error(file, line));
                }
                self.subsection = self
                    .subsection
                    .checked_add(1)
                    .ok_or_else(|| self.error(file, line))?;
                HeadingNumber::Subsection(self.section, self.subsection)
            }
            _ => return Err(self.error(file, line)),
        };
        self.pending = Some(Heading {
            number,
            title: title.into(),
            file: file.into(),
            line,
        });
        Ok(())
    }

    fn bind(&mut self, file: &str, line: usize) -> Result<(), DestinationError> {
        if self.found.is_some() {
            return Err(DestinationError::DuplicateTarget(format!(
                "{} at {file}:{line}",
                self.label
            )));
        }
        let heading = self.pending.take().ok_or_else(|| {
            DestinationError::MissingHeading(if self.located_errors {
                format!("{} at {file}:{line}", self.label)
            } else {
                self.label.into()
            })
        })?;
        let title =
            plain_title(&heading.title).ok_or_else(|| self.error(&heading.file, heading.line))?;
        let locator = match heading.number {
            HeadingNumber::Section(section) => format!("§{section}"),
            HeadingNumber::Subsection(section, subsection) => format!("§{section}.{subsection}"),
            HeadingNumber::Appendix(number) => {
                format!("appendix {}", char::from(b'A' + number - 1))
            }
            HeadingNumber::AppendixSubsection => {
                return Err(self.error(&heading.file, heading.line));
            }
        };
        self.found = Some((
            Target {
                document: self.document,
                label: self.label.into(),
                decoded_title: title.replace('\'', "’"),
                address: Address::Heading {
                    override_fragment: (self.label == "app:self-limit")
                        .then_some("method-self-limit".into()),
                },
                available: false,
                locator,
            },
            file.into(),
        ));
        Ok(())
    }
}

fn plain_title(title: &str) -> Option<&str> {
    (!title.chars().any(|scalar| {
        matches!(
            scalar,
            '{' | '}' | '\\' | '~' | '`' | '"' | '$' | '<' | '>' | '|'
        )
    }) && !title.contains("--")
        && !title.contains("''"))
    .then_some(title)
}

fn without_comment(line: &str) -> &str {
    for (index, scalar) in line.char_indices() {
        if scalar == '%'
            && line[..index]
                .bytes()
                .rev()
                .take_while(|byte| *byte == b'\\')
                .count()
                % 2
                == 0
        {
            return &line[..index];
        }
    }
    line
}

fn argument<'a>(line: &'a str, command: &str) -> Option<(&'a str, &'a str)> {
    let rest = line.strip_prefix(command)?.strip_prefix('{')?;
    let mut depth = 1usize;
    for (index, scalar) in rest.char_indices() {
        match scalar {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&rest[..index], &rest[index + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

fn counter_event(line: &str) -> bool {
    [
        "\\setcounter",
        "\\addtocounter",
        "\\stepcounter",
        "\\refstepcounter",
    ]
    .iter()
    .any(|event| line.contains(event))
}

fn unsupported_outline_form(line: &str) -> bool {
    let number_redefinition = [
        "\\renewcommand{",
        "\\renewcommand*{",
        "\\newcommand{",
        "\\newcommand*{",
        "\\def\\",
        "\\gdef\\",
        "\\xdef\\",
        "\\edef\\",
    ]
    .iter()
    .any(|command| line.contains(command))
        && (line.contains("\\thesection") || line.contains("\\thesubsection"));
    counter_event(line)
        || number_redefinition
        || [
            "\\counterwithin",
            "\\counterwithout",
            "\\@addtoreset",
            "\\@removefromreset",
            "\\numberwithin",
            "\\iffalse",
            "\\iftrue",
            "\\begin{comment}",
            "\\end{comment}",
        ]
        .iter()
        .any(|event| line.contains(event))
}

fn scan_source(
    outline: &mut Outline<'_>,
    file: &str,
    source: &str,
) -> Result<(), DestinationError> {
    let requested = format!("\\label{{{}}}", outline.label);
    let mut started = false;
    let mut ended = false;
    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let mut line = without_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if unsupported_outline_form(line)
            && !(started
                && line == "\\setcounter{subsection}{-1}"
                && file == "documents/paper/parts/partE.tex"
                && outline.appendix.is_none()
                && outline.subsection == 0)
        {
            return Err(outline.error(file, line_number));
        }
        if line == "\\begin{document}" {
            if started {
                return Err(outline.error(file, line_number));
            }
            started = true;
            continue;
        }
        if !started {
            continue;
        }
        if line == "\\end{document}" {
            ended = true;
            break;
        }
        if line == "\\appendix" {
            if outline.appendix.is_some() {
                return Err(outline.error(file, line_number));
            }
            outline.appendix = Some(0);
            outline.pending = None;
            continue;
        }
        if line == "\\setcounter{subsection}{-1}"
            && file == "documents/paper/parts/partE.tex"
            && outline.appendix.is_none()
            && outline.subsection == 0
        {
            outline.subsection = -1;
            outline.pending = None;
            continue;
        }
        if let Some(rest) = line.strip_prefix("\\PaperNeedLines") {
            let Some((count, after)) = argument(rest, "") else {
                return Err(outline.error(file, line_number));
            };
            if count.is_empty()
                || !count.bytes().all(|byte| byte.is_ascii_digit())
                || !count.parse::<u32>().is_ok_and(|value| value > 0)
            {
                return Err(outline.error(file, line_number));
            }
            line = after;
        }
        for kind in ["subsection", "section"] {
            let command = format!("\\{kind}");
            if line.starts_with(&command) {
                let Some((title, rest)) = argument(line, &command) else {
                    return Err(outline.error(file, line_number));
                };
                if !rest.trim().is_empty() {
                    return Err(outline.error(file, line_number));
                }
                outline.heading(kind, title, file, line_number)?;
                line = "";
                break;
            }
        }
        if line.is_empty() {
            continue;
        }
        if line.contains("\\appendix")
            || line.contains("\\section")
            || line.contains("\\subsection")
            || line.starts_with("\\input")
            || line.starts_with("\\include")
            || line.starts_with("\\subfile")
            || line.starts_with("\\PaperNeedLines")
        {
            return Err(outline.error(file, line_number));
        }
        if line == requested {
            outline.bind(file, line_number)?;
        } else {
            outline.pending = None;
        }
    }
    if !started || !ended {
        return Err(outline.error(file, source.lines().count()));
    }
    Ok(())
}

/// Derive an appendix destination from the same bounded heading reader used by the master.
#[cfg(test)]
pub(super) fn appendix_target(
    source: &str,
    label: &str,
    document: Reading,
) -> Result<Target, DestinationError> {
    let mut outline = Outline {
        label,
        document,
        section: -1,
        subsection: 0,
        appendix: None,
        pending: None,
        found: None,
        located_errors: false,
    };
    scan_source(&mut outline, "appendix", source)?;
    outline
        .found
        .map(|(target, _)| target)
        .ok_or_else(|| DestinationError::MissingHeading(label.into()))
}

fn preamble_input(line: &str) -> bool {
    let Some((name, rest)) = argument(line, "\\input") else {
        return false;
    };
    rest.is_empty()
        && [
            "data/ftoo_manifest.tex",
            "data/ftoo_symbols.tex",
            "data/ftoo_symbols_historical.tex",
            "macros_overlap.tex",
            "ftoo_typed_tools.tex",
            "data/ftoo_registers.tex",
            "econ_membrane.tex",
        ]
        .contains(&name)
}

fn admitted_preamble_counter(line: &str) -> bool {
    [
        "\\setcounter{section}{-1}",
        "\\setcounter{biburllcpenalty}{7000}",
        "\\setcounter{biburlucpenalty}{8000}",
    ]
    .contains(&line)
}

fn check_master_outline_form(
    label: &str,
    line: &str,
    line_number: usize,
    body: bool,
) -> Result<(), DestinationError> {
    if unsupported_outline_form(line) && (body || !admitted_preamble_counter(line)) {
        return Err(DestinationError::UnsupportedOutline(format!(
            "{label} at documents/paper/main.tex:{line_number}"
        )));
    }
    Ok(())
}

fn master_sources<'a>(
    label: &str,
    inputs: &'a BTreeMap<String, Vec<u8>>,
) -> Result<Vec<(&'a str, &'a str)>, DestinationError> {
    let master = inputs
        .get("documents/paper/main.tex")
        .ok_or_else(|| DestinationError::MissingSource("documents/paper/main.tex".into()))?;
    let master = std::str::from_utf8(master)
        .map_err(|_| DestinationError::UnsupportedOutline(label.into()))?;
    let declared = inputs
        .get("documents/paper-build/sources.tsv")
        .ok_or_else(|| {
            DestinationError::MissingSource("documents/paper-build/sources.tsv".into())
        })?;
    let declared = std::str::from_utf8(declared)
        .map_err(|_| DestinationError::UnsupportedOutline(label.into()))?;
    let declared: BTreeSet<_> = declared.lines().collect();
    let mut body = false;
    let mut initialized = false;
    let mut ended = false;
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for (index, raw) in master.lines().enumerate() {
        let line = without_comment(raw).trim();
        check_master_outline_form(label, line, index + 1, body)?;
        if !body {
            if line == "\\setcounter{section}{-1}" {
                if initialized {
                    return Err(DestinationError::UnsupportedOutline(format!(
                        "{label} at documents/paper/main.tex:{}",
                        index + 1
                    )));
                }
                initialized = true;
            } else if line.starts_with("\\input") {
                if !preamble_input(line) {
                    return Err(DestinationError::UnsupportedOutline(format!(
                        "{label} at documents/paper/main.tex:{}",
                        index + 1
                    )));
                }
            } else if line.starts_with("\\subfile")
                || line.starts_with("\\include")
                || line.contains("\\appendix")
            {
                return Err(DestinationError::UnsupportedOutline(format!(
                    "{label} at documents/paper/main.tex:{}",
                    index + 1
                )));
            } else if line == "\\begin{document}" {
                body = true;
            }
            continue;
        }
        if line == "\\end{document}" {
            ended = true;
            break;
        }
        if line.starts_with("\\subfile") {
            let Some((name, rest)) = argument(line, "\\subfile") else {
                return Err(DestinationError::UnsupportedOutline(format!(
                    "{label} at documents/paper/main.tex:{}",
                    index + 1
                )));
            };
            let path = format!("documents/paper/{name}.tex");
            if !rest.is_empty()
                || !name.starts_with("parts/")
                || name.contains("..")
                || !declared.contains(path.as_str())
                || !seen.insert(path.clone())
            {
                return Err(DestinationError::UnsupportedOutline(format!(
                    "{label} at documents/paper/main.tex:{}",
                    index + 1
                )));
            }
            let source = inputs
                .get(&path)
                .ok_or_else(|| DestinationError::MissingSource(path.clone()))?;
            let source = std::str::from_utf8(source).map_err(|_| {
                DestinationError::UnsupportedOutline(format!("{label} at {path}:1"))
            })?;
            let path = inputs.get_key_value(&path).unwrap().0.as_str();
            result.push((path, source));
        } else if line.starts_with("\\input") && line != "\\input{data/ftoo_index.tex}"
            || line.starts_with("\\include")
            || line.contains("\\appendix")
            || line.contains("\\section")
            || line.contains("\\subsection")
        {
            return Err(DestinationError::UnsupportedOutline(format!(
                "{label} at documents/paper/main.tex:{}",
                index + 1
            )));
        }
    }
    if !body || !initialized || !ended || result.is_empty() {
        return Err(DestinationError::UnsupportedOutline(format!(
            "{label} at documents/paper/main.tex"
        )));
    }
    Ok(result)
}

pub(super) fn target_from_inputs(
    label: &str,
    mints: &BTreeMap<String, ftoo_manifest::MintOrigin>,
    inputs: &BTreeMap<String, Vec<u8>>,
) -> Result<Target, DestinationError> {
    let Some(ftoo_manifest::MintOrigin {
        kind: ftoo_manifest::MintKind::PaperTex,
        site: ftoo_manifest::LabelSite::PaperMintFile { file },
    }) = mints.get(label)
    else {
        return Err(DestinationError::UnknownTarget(label.into()));
    };
    let sources = master_sources(label, inputs)?;
    let mut outline = Outline {
        label,
        document: Reading::Paper,
        section: -1,
        subsection: 0,
        appendix: None,
        pending: None,
        found: None,
        located_errors: true,
    };
    for (path, source) in sources {
        scan_source(&mut outline, path, source)?;
    }
    let (target, actual_file) = outline
        .found
        .ok_or_else(|| DestinationError::MissingHeading(label.into()))?;
    if &actual_file != file {
        return Err(DestinationError::UnsupportedOutline(format!(
            "{label} in {actual_file}, declared in {file}"
        )));
    }
    Ok(target)
}

pub(super) fn fragment(target: &Target) -> Result<String, DestinationError> {
    let candidate = match &target.address {
        Address::Heading { override_fragment } => override_fragment
            .clone()
            .or_else(|| title_fragment(&target.decoded_title)),
    }
    .ok_or_else(|| DestinationError::InvalidFragment(target.label.clone()))?;
    if !valid_fragment(&candidate) {
        return Err(DestinationError::InvalidFragment(target.label.clone()));
    }
    Ok(candidate)
}

/// Resolve each referenced unavailable target once, in first-reference order.
pub(super) fn resolve(
    targets: &[Target],
    references: &[Reference],
    body_fragments: &[(Reading, String)],
    language: &str,
) -> Result<Vec<IndexEntry>, DestinationError> {
    let message =
        message(language).ok_or_else(|| DestinationError::MissingMessage(language.to_owned()))?;
    let mut known = BTreeMap::new();
    let mut fragments = BTreeSet::new();
    for (document, fragment) in body_fragments {
        if !valid_fragment(fragment) {
            return Err(DestinationError::InvalidFragment(fragment.clone()));
        }
        if !fragments.insert((*document, fragment.clone())) {
            return Err(DestinationError::DuplicateFragment {
                document: *document,
                fragment: fragment.clone(),
            });
        }
    }
    for target in targets {
        if known.insert(target.label.as_str(), target).is_some() {
            return Err(DestinationError::DuplicateTarget(target.label.clone()));
        }
    }
    let mut references = references.to_vec();
    references.sort_by_key(|reference| (reference.document, reference.position));
    let mut first = BTreeMap::new();
    for reference in references {
        let target = known
            .get(reference.target.as_str())
            .ok_or_else(|| DestinationError::UnknownTarget(reference.target.clone()))?;
        if target.available {
            let fragment = fragment(target)?;
            if !fragments.contains(&(target.document, fragment.clone())) {
                return Err(DestinationError::MissingBody {
                    document: target.document,
                    fragment,
                });
            }
            continue;
        }
        first
            .entry(target.label.as_str())
            .or_insert((reference.document, reference.position));
    }
    let mut entries = Vec::new();
    for (label, (document, position)) in first {
        let target = known[label];
        let fragment = fragment(target)?;
        if !fragments.insert((target.document, fragment.clone())) {
            return Err(DestinationError::DuplicateFragment {
                document: target.document,
                fragment,
            });
        }
        let entry = IndexEntry {
            document: target.document,
            language: language.into(),
            fragment,
            title: target.decoded_title.clone(),
            locator: target.locator.clone(),
            message,
        };
        entries.push((document, position, entry));
    }
    entries.sort_by(|left, right| {
        (left.0, left.1, &left.2.fragment).cmp(&(right.0, right.1, &right.2.fragment))
    });
    for (_, _, entry) in &entries {
        let reader = format!("{} {} {}", entry.fragment, entry.title, entry.message);
        if let Some(label) = known.keys().find(|label| reader.contains(**label)) {
            return Err(DestinationError::ReaderLabel((*label).to_owned()));
        }
        if let Some(range) = ftoo_manifest::reader_label_literals(&reader).first() {
            return Err(DestinationError::ReaderLabel(
                reader[range.clone()].to_owned(),
            ));
        }
    }
    Ok(entries.into_iter().map(|(_, _, entry)| entry).collect())
}

/// A complete release cannot retain an unavailable destination or content gap.
pub(super) fn release(entries: &[IndexEntry], gaps: bool) -> Result<(), DestinationError> {
    if entries.is_empty() && !gaps {
        Ok(())
    } else {
        Err(DestinationError::ReleaseUnavailable {
            entries: entries.len(),
            gaps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_outline_inputs() -> BTreeMap<String, Vec<u8>> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let list = std::fs::read_to_string(root.join("documents/paper-build/sources.tsv")).unwrap();
        list.lines()
            .filter(|path| {
                *path == "documents/paper/main.tex" || path.starts_with("documents/paper/parts/")
            })
            .map(|path| (path.into(), std::fs::read(root.join(path)).unwrap()))
            .chain(std::iter::once((
                "documents/paper-build/sources.tsv".into(),
                list.as_bytes().to_vec(),
            )))
            .collect()
    }

    fn real_target(
        label: &str,
        file: &str,
        inputs: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Target, DestinationError> {
        let mints = BTreeMap::from([(
            label.into(),
            ftoo_manifest::MintOrigin {
                kind: ftoo_manifest::MintKind::PaperTex,
                site: ftoo_manifest::LabelSite::PaperMintFile { file: file.into() },
            },
        )]);
        target_from_inputs(label, &mints, inputs)
    }

    #[test]
    fn regular_headings_follow_real_master_and_counter_events() {
        let inputs = real_outline_inputs();
        for (label, file, locator, title) in [
            (
                "sec:generators",
                "documents/paper/parts/front.tex",
                "§0",
                "The object, the stance, the generators",
            ),
            (
                "subsec:ont:opposition",
                "documents/paper/parts/partA.tex",
                "§2.7",
                "Gradient opposition, and the sign that sets it",
            ),
            (
                "subsec:prof:gates",
                "documents/paper/parts/partE.tex",
                "§17.0",
                "The reachability gates",
            ),
            (
                "subsec:coda:tenses",
                "documents/paper/parts/coda.tex",
                "§22.1",
                "The three tenses",
            ),
            (
                "app:self-limit",
                "documents/paper/parts/appendices.tex",
                "appendix G",
                "The method’s self-limit",
            ),
        ] {
            let target = real_target(label, file, &inputs).unwrap();
            assert_eq!(target.locator, locator, "{label}");
            assert_eq!(target.decoded_title, title, "{label}");
        }
    }

    fn replace_input(inputs: &mut BTreeMap<String, Vec<u8>>, path: &str, old: &str, new: &str) {
        let current = String::from_utf8(inputs[path].clone()).unwrap();
        assert!(current.contains(old), "{path}: {old}");
        inputs.insert(path.into(), current.replacen(old, new, 1).into_bytes());
    }

    #[test]
    fn heading_counter_events_and_master_order_are_source_decided() {
        let base = real_outline_inputs();
        let coda = |inputs: &BTreeMap<_, _>| {
            real_target(
                "subsec:coda:tenses",
                "documents/paper/parts/coda.tex",
                inputs,
            )
            .unwrap()
            .locator
        };
        let base_coda = coda(&base);
        assert_eq!(base_coda, "§22.1");
        let (base_section, base_subsection) = base_coda
            .strip_prefix('§')
            .unwrap()
            .split_once('.')
            .unwrap();
        let base_section: i32 = base_section.parse().unwrap();
        let mut earlier = base.clone();
        replace_input(
            &mut earlier,
            "documents/paper/parts/partA.tex",
            "\\section{The epistemic basin}",
            "\\section{Inserted}\n\\section{The epistemic basin}",
        );
        assert_eq!(
            coda(&earlier),
            format!("§{}.{}", base_section + 1, base_subsection)
        );
        let mut reordered = base.clone();
        replace_input(
            &mut reordered,
            "documents/paper/main.tex",
            "\\subfile{parts/partA}\n\\subfile{parts/partB}",
            "\\subfile{parts/partB}\n\\subfile{parts/partA}",
        );
        assert_eq!(
            real_target(
                "subsec:ont:opposition",
                "documents/paper/parts/partA.tex",
                &reordered
            )
            .unwrap()
            .locator,
            "§7.7"
        );
        let mut reset = base.clone();
        replace_input(
            &mut reset,
            "documents/paper/parts/partE.tex",
            "\\setcounter{subsection}{-1}%",
            "",
        );
        let base_gates = real_target(
            "subsec:prof:gates",
            "documents/paper/parts/partE.tex",
            &base,
        )
        .unwrap()
        .locator;
        let (gates_section, gates_subsection) = base_gates
            .strip_prefix('§')
            .unwrap()
            .split_once('.')
            .unwrap();
        let gates_subsection: i32 = gates_subsection.parse().unwrap();
        assert_eq!(
            real_target(
                "subsec:prof:gates",
                "documents/paper/parts/partE.tex",
                &reset
            )
            .unwrap()
            .locator,
            format!("§{gates_section}.{}", gates_subsection + 1)
        );
    }

    #[test]
    fn master_preamble_refuses_outline_changes_but_ignores_title_format() {
        let base = real_outline_inputs();
        let mut preamble = base.clone();
        replace_input(
            &mut preamble,
            "documents/paper/main.tex",
            "\\begin{document}",
            "\\titleformat{\\subsection}[runin]\n\\titlespacing*{\\subsection}{0pt}\n\\begin{document}",
        );
        assert_eq!(
            real_target(
                "subsec:coda:tenses",
                "documents/paper/parts/coda.tex",
                &preamble
            )
            .unwrap()
            .locator,
            "§22.1"
        );
        replace_input(
            &mut preamble,
            "documents/paper/main.tex",
            "\\begin{document}",
            "\\input{unlisted-outline}\n\\begin{document}",
        );
        assert!(matches!(
            real_target(
                "subsec:coda:tenses",
                "documents/paper/parts/coda.tex",
                &preamble
            ),
            Err(DestinationError::UnsupportedOutline(site)) if site.contains("main.tex:")
        ));
        let mut depth = base.clone();
        replace_input(
            &mut depth,
            "documents/paper/main.tex",
            "\\begin{document}",
            "\\setcounter{secnumdepth}{1}\n\\begin{document}",
        );
        assert!(matches!(
            real_target(
                "subsec:coda:tenses",
                "documents/paper/parts/coda.tex",
                &depth
            ),
            Err(DestinationError::UnsupportedOutline(site)) if site.contains("main.tex:")
        ));
    }

    #[test]
    fn exact_counter_admissions_and_master_appendix_boundary_refuse_changes() {
        let base = real_outline_inputs();
        let target = |inputs: &BTreeMap<_, _>| {
            real_target(
                "subsec:coda:tenses",
                "documents/paper/parts/coda.tex",
                inputs,
            )
        };
        let mut penalty = base.clone();
        replace_input(
            &mut penalty,
            "documents/paper/main.tex",
            "\\setcounter{biburllcpenalty}{7000}",
            "\\setcounter{biburllcpenalty}{7001}",
        );
        assert!(
            matches!(target(&penalty), Err(DestinationError::UnsupportedOutline(site)) if site.contains("main.tex:61"))
        );
        let mut body_penalty = base.clone();
        replace_input(
            &mut body_penalty,
            "documents/paper/main.tex",
            "\\begin{document}",
            "\\begin{document}\n\\setcounter{biburllcpenalty}{7000}",
        );
        assert!(
            matches!(target(&body_penalty), Err(DestinationError::UnsupportedOutline(site)) if site.contains("main.tex:132"))
        );
        let mut preamble_appendix = base.clone();
        replace_input(
            &mut preamble_appendix,
            "documents/paper/main.tex",
            "\\begin{document}",
            "\\appendix\n\\begin{document}",
        );
        assert!(
            matches!(target(&preamble_appendix), Err(DestinationError::UnsupportedOutline(site)) if site.contains("main.tex:131"))
        );
        let mut misplaced_reset = base.clone();
        replace_input(
            &mut misplaced_reset,
            "documents/paper/parts/coda.tex",
            "\\subsection{The three tenses}",
            "\\setcounter{subsection}{-1}\n\\subsection{The three tenses}",
        );
        assert!(
            matches!(target(&misplaced_reset), Err(DestinationError::UnsupportedOutline(site)) if site.contains("coda.tex:8"))
        );
    }

    #[test]
    fn heading_resolution_refuses_unsupported_or_unowned_events() {
        let base = real_outline_inputs();
        let target = |inputs: &BTreeMap<_, _>| {
            real_target(
                "subsec:coda:tenses",
                "documents/paper/parts/coda.tex",
                inputs,
            )
        };
        let mut missing = base.clone();
        missing.remove("documents/paper/parts/coda.tex");
        assert_eq!(
            target(&missing),
            Err(DestinationError::MissingSource(
                "documents/paper/parts/coda.tex".into()
            ))
        );
        let mut undeclared = base.clone();
        replace_input(
            &mut undeclared,
            "documents/paper/main.tex",
            "\\subfile{parts/coda}",
            "\\subfile{parts/undeclared}",
        );
        assert!(
            matches!(target(&undeclared), Err(DestinationError::UnsupportedOutline(site)) if site.contains("main.tex:"))
        );
        let mut duplicate = base.clone();
        replace_input(
            &mut duplicate,
            "documents/paper/parts/coda.tex",
            "\\label{subsec:coda:tenses}",
            "\\label{subsec:coda:tenses}\n\\label{subsec:coda:tenses}",
        );
        assert!(
            matches!(target(&duplicate), Err(DestinationError::DuplicateTarget(site)) if site.contains("subsec:coda:tenses at documents/paper/parts/coda.tex:10"))
        );
        assert!(matches!(
            real_target(
                "subsec:coda:tenses",
                "documents/paper/parts/partB.tex",
                &base
            ),
            Err(DestinationError::UnsupportedOutline(site)) if site.contains("declared in documents/paper/parts/partB.tex")
        ));
        let mut mention = base.clone();
        replace_input(
            &mut mention,
            "documents/paper/parts/coda.tex",
            "\\label{subsec:coda:tenses}",
            "\\zcref{subsec:coda:tenses}",
        );
        assert_eq!(
            target(&mention),
            Err(DestinationError::MissingHeading(
                "subsec:coda:tenses".into()
            ))
        );
        let mut intervening = base.clone();
        replace_input(
            &mut intervening,
            "documents/paper/parts/coda.tex",
            "\\label{subsec:coda:tenses}",
            "Intervening prose.\n\\label{subsec:coda:tenses}",
        );
        assert!(
            matches!(target(&intervening), Err(DestinationError::MissingHeading(site)) if site.contains("coda.tex:10"))
        );
    }

    #[test]
    fn heading_scan_refuses_unadmitted_events_and_bounds_the_part_body() {
        let base = real_outline_inputs();
        let target = |inputs: &BTreeMap<_, _>| {
            real_target(
                "subsec:coda:tenses",
                "documents/paper/parts/coda.tex",
                inputs,
            )
        };
        for replacement in [
            "\\stepcounter{section}\n\\subsection{The three tenses}",
            "\\setcounter{secnumdepth}{1}\n\\subsection{The three tenses}",
            "\\addtocounter{secnumdepth}{-1}\n\\subsection{The three tenses}",
            "\\setcounter {secnumdepth}{1}\n\\subsection{The three tenses}",
            "\\counterwithin{subsection}{section}\n\\subsection{The three tenses}",
            "\\counterwithout{subsection}{section}\n\\subsection{The three tenses}",
            "\\@addtoreset{subsection}{section}\n\\subsection{The three tenses}",
            "\\@removefromreset{subsection}{section}\n\\subsection{The three tenses}",
            "\\numberwithin{subsection}{section}\n\\subsection{The three tenses}",
            "\\renewcommand{\\thesection}{Roman}\n\\subsection{The three tenses}",
            "\\renewcommand{\\thesubsection}{Roman}\n\\subsection{The three tenses}",
            "\\iffalse\n\\subsection{The three tenses}",
            "\\begin{comment}\n\\subsection{The three tenses}",
            "\\subsection*{The three tenses}",
            "\\subsection[Short]{The three tenses}",
            "\\subsection{The $\\beta$ tenses}",
        ] {
            let mut changed = base.clone();
            replace_input(
                &mut changed,
                "documents/paper/parts/coda.tex",
                "\\subsection{The three tenses}",
                replacement,
            );
            assert!(
                matches!(target(&changed), Err(DestinationError::UnsupportedOutline(site)) if site.contains("coda.tex:8")),
                "{replacement}"
            );
        }
        let mut prefixed = base.clone();
        replace_input(
            &mut prefixed,
            "documents/paper/parts/partA.tex",
            "\\PaperNeedLines{5}\\subsection{The two rewards}",
            "\\Unknown{5}\\subsection{The two rewards}",
        );
        assert!(
            matches!(target(&prefixed), Err(DestinationError::UnsupportedOutline(site)) if site.contains("partA.tex:529"))
        );
        let mut master_appendix = base.clone();
        replace_input(
            &mut master_appendix,
            "documents/paper/main.tex",
            "\\subfile{parts/coda}",
            "\\appendix\n\\subfile{parts/coda}",
        );
        assert!(
            matches!(target(&master_appendix), Err(DestinationError::UnsupportedOutline(site)) if site.contains("main.tex:"))
        );
        let mut before_body = base.clone();
        replace_input(
            &mut before_body,
            "documents/paper/parts/coda.tex",
            "\\begin{document}",
            "\\section{Ignored in the part preamble}\n\\begin{document}",
        );
        assert_eq!(
            target(&before_body).unwrap().locator,
            target(&base).unwrap().locator
        );
        let mut no_end = base.clone();
        replace_input(
            &mut no_end,
            "documents/paper/parts/coda.tex",
            "\\end{document}",
            "",
        );
        assert!(
            matches!(target(&no_end), Err(DestinationError::UnsupportedOutline(site)) if site.contains("coda.tex:"))
        );
    }

    fn target(label: &str, title: &str) -> Target {
        Target {
            document: Reading::Paper,
            label: label.into(),
            decoded_title: title.into(),
            address: Address::Heading {
                override_fragment: None,
            },
            available: false,
            locator: "appendix A".into(),
        }
    }

    fn reference(document: Reading, position: usize, target: &str) -> Reference {
        Reference {
            document,
            position,
            target: target.into(),
        }
    }

    #[test]
    fn first_reference_order_filters_into_owner_index() {
        let targets = [target("app:first", "First"), target("app:second", "Second")];
        let references = [
            reference(Reading::Explainer, 1, "app:first"),
            reference(Reading::Paper, 3, "app:second"),
            reference(Reading::Paper, 8, "app:first"),
        ];
        let entries = resolve(&targets, &references, &[], "en").unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.title.as_str())
                .collect::<Vec<_>>(),
            ["Second", "First"]
        );
        assert!(entries.iter().all(|entry| entry.document == Reading::Paper));
        assert_eq!(entries[0].message, "Not yet available in this rendering.");
    }

    #[test]
    fn heading_override_retains_the_checked_address() {
        let mut target = target("app:self-limit", "The method’s self-limit");
        assert_eq!(fragment(&target), Ok("the-methods-self-limit".into()));
        target.address = Address::Heading {
            override_fragment: Some("method-self-limit".into()),
        };
        assert_eq!(fragment(&target), Ok("method-self-limit".into()));
    }

    #[test]
    fn one_entry_or_one_gap_refuses_release() {
        let entries = resolve(
            &[target("app:first", "First")],
            &[reference(Reading::Paper, 0, "app:first")],
            &[],
            "en",
        )
        .unwrap();
        assert_eq!(
            release(&entries, false),
            Err(DestinationError::ReleaseUnavailable {
                entries: 1,
                gaps: false
            })
        );
        assert_eq!(
            release(&[], true),
            Err(DestinationError::ReleaseUnavailable {
                entries: 0,
                gaps: true
            })
        );
        assert_eq!(release(&[], false), Ok(()));
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn duplicate_fragment_and_reader_label_refuse() {
        let targets = [target("app:first", "First"), target("app:second", "First")];
        let refs = [
            reference(Reading::Paper, 0, "app:first"),
            reference(Reading::Paper, 1, "app:second"),
        ];
        assert_eq!(
            resolve(&targets, &refs, &[], "en"),
            Err(DestinationError::DuplicateFragment {
                document: Reading::Paper,
                fragment: "first".into()
            })
        );
        assert_eq!(
            resolve(
                &targets[..1],
                &refs[..1],
                &[(Reading::Paper, "first".into())],
                "en"
            ),
            Err(DestinationError::DuplicateFragment {
                document: Reading::Paper,
                fragment: "first".into()
            })
        );
        let leaking = target("app:first", "The label app:first");
        assert_eq!(
            resolve(&[leaking], &refs[..1], &[], "en"),
            Err(DestinationError::ReaderLabel("app:first".into()))
        );
        let unrelated = target("app:first", "The label def:fixture:one");
        assert_eq!(
            resolve(&[unrelated], &refs[..1], &[], "en"),
            Err(DestinationError::ReaderLabel("def:fixture:one".into()))
        );
    }

    #[test]
    fn available_target_moves_out_of_index() {
        let mut available = target("app:first", "First");
        available.available = true;
        let reference = reference(Reading::Paper, 0, "app:first");
        assert_eq!(
            resolve(
                std::slice::from_ref(&available),
                std::slice::from_ref(&reference),
                &[],
                "en",
            ),
            Err(DestinationError::MissingBody {
                document: Reading::Paper,
                fragment: "first".into()
            })
        );
        assert_eq!(
            resolve(
                &[available],
                &[reference],
                &[(Reading::Paper, "first".into())],
                "en"
            ),
            Ok(Vec::new())
        );
    }

    #[test]
    fn appendix_number_tracks_source_order_and_refuses_an_unsupported_edit() {
        let source = "\\begin{document}\n\\appendix\n\\section{First}\n\\label{app:first}\n\\section{The method's self-limit}\n\\label{app:self-limit}\n\\end{document}\n";
        let target = appendix_target(source, "app:self-limit", Reading::Paper).unwrap();
        assert_eq!(target.locator, "appendix B");
        assert_eq!(target.decoded_title, "The method’s self-limit");
        assert_eq!(fragment(&target), Ok("method-self-limit".into()));
        let changed = source.replace(
            "\\section{The method's self-limit}",
            "\\section{Added}\n\\label{app:added}\n\\section{The method's self-limit}",
        );
        assert_eq!(
            appendix_target(&changed, "app:self-limit", Reading::Paper)
                .unwrap()
                .locator,
            "appendix C"
        );
        assert_eq!(
            appendix_target(
                &source.replace("\\label{app:self-limit}", ""),
                "app:self-limit",
                Reading::Paper
            ),
            Err(DestinationError::MissingHeading("app:self-limit".into()))
        );
        let unsupported = source.replace(
            "\\section{First}",
            "\\setcounter{section}{3}\n\\section{First}",
        );
        assert_eq!(
            appendix_target(&unsupported, "app:self-limit", Reading::Paper),
            Err(DestinationError::UnsupportedOutline(
                "app:self-limit".into()
            ))
        );
    }

    #[test]
    fn source_outline_locates_the_first_index_target() {
        let source = include_str!("../../../../documents/paper/parts/appendices.tex");
        let target = appendix_target(source, "app:self-limit", Reading::Paper).unwrap();
        assert!(target.locator.starts_with("appendix "));
        assert_eq!(target.decoded_title, "The method’s self-limit");
    }

    #[test]
    fn unsupported_section_forms_and_titles_refuse_before_numbering() {
        let ordinary = "\\begin{document}\n\\appendix\n\\section{First}\n\\label{app:first}\n\\end{document}\n";
        for changed in [
            ordinary.replace("\\section{First}", "\\section[Short]{First}"),
            ordinary.replace("\\section{First}", "\\section {First}"),
            ordinary.replace("\\section{First}", "\\section*{First}"),
            ordinary.replace(
                "\\section{First}",
                "\\stepcounter{section}\n\\section{First}",
            ),
            ordinary.replace(
                "\\section{First}",
                "\\refstepcounter{section}\n\\section{First}",
            ),
            ordinary.replace(
                "\\section{First}",
                "\\setcounter{secnumdepth}{1}\n\\section{First}",
            ),
            ordinary.replace(
                "\\section{First}",
                "\\addtocounter{secnumdepth}{-1}\n\\section{First}",
            ),
            ordinary.replace("\\section{First}", "\\input{extra}\n\\section{First}"),
            ordinary.replace("\\section{First}", "\\include{extra}\n\\section{First}"),
            ordinary.replace("\\section{First}", "\\subfile{extra}\n\\section{First}"),
            ordinary.replace("\\section{First}", "\\section{First--part}"),
            ordinary.replace("\\section{First}", "\\section{First~part}"),
            ordinary.replace("\\section{First}", "\\section{First`part}"),
            ordinary.replace("\\section{First}", "\\section{First''part}"),
            ordinary.replace("\\section{First}", "\\section{First\"part}"),
            ordinary.replace("\\section{First}", "\\section{First$part}"),
            ordinary.replace("\\section{First}", "\\section{First<part}"),
            ordinary.replace("\\section{First}", "\\section{First>part}"),
            ordinary.replace("\\section{First}", "\\section{First|part}"),
        ] {
            assert_eq!(
                appendix_target(&changed, "app:first", Reading::Paper),
                Err(DestinationError::UnsupportedOutline("app:first".into())),
                "{changed}"
            );
        }
    }

    #[test]
    fn missing_declared_target_refuses_before_emission() {
        assert_eq!(
            target_from_inputs("app:absent", &BTreeMap::new(), &BTreeMap::new()),
            Err(DestinationError::UnknownTarget("app:absent".into()))
        );
    }
}
