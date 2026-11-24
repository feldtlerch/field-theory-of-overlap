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

/// Derive an appendix destination from the ordered section commands in its source.
pub(super) fn appendix_target(
    source: &str,
    label: &str,
    document: Reading,
) -> Result<Target, DestinationError> {
    let mut appendix = false;
    let mut ordinal = 0u8;
    let mut pending = None;
    let requested = format!("\\label{{{label}}}");
    for line in source.lines().map(str::trim) {
        if line == "\\appendix" {
            if appendix {
                return Err(DestinationError::UnsupportedOutline(label.into()));
            }
            appendix = true;
        } else if appendix
            && (line.starts_with("\\setcounter{")
                || line.starts_with("\\addtocounter{")
                || line.starts_with("\\stepcounter{")
                || line.starts_with("\\refstepcounter{")
                || line.starts_with("\\input")
                || line.starts_with("\\include")
                || line.starts_with("\\subfile"))
        {
            return Err(DestinationError::UnsupportedOutline(label.into()));
        } else if appendix && line.starts_with("\\section{") {
            let title = line
                .strip_prefix("\\section{")
                .and_then(|value| value.strip_suffix('}'))
                .filter(|value| {
                    !value.chars().any(|scalar| {
                        matches!(
                            scalar,
                            '{' | '}' | '\\' | '~' | '`' | '"' | '$' | '<' | '>' | '|'
                        )
                    }) && !value.contains("--")
                        && !value.contains("''")
                })
                .ok_or_else(|| DestinationError::UnsupportedOutline(label.into()))?;
            ordinal = ordinal
                .checked_add(1)
                .filter(|value| *value <= 26)
                .ok_or_else(|| DestinationError::UnsupportedOutline(label.into()))?;
            pending = Some((ordinal, title.to_owned()));
        } else if appendix && line.starts_with("\\section") {
            return Err(DestinationError::UnsupportedOutline(label.into()));
        } else if line == requested {
            let (number, title) = pending
                .take()
                .ok_or_else(|| DestinationError::MissingHeading(label.into()))?;
            let letter = char::from(b'A' + number - 1);
            return Ok(Target {
                document,
                label: label.into(),
                decoded_title: title.replace('\'', "’"),
                address: Address::Heading {
                    override_fragment: (label == "app:self-limit")
                        .then_some("method-self-limit".into()),
                },
                available: false,
                locator: format!("appendix {letter}"),
            });
        } else if !line.is_empty() {
            pending = None;
        }
    }
    Err(DestinationError::MissingHeading(label.into()))
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
    let source = inputs
        .get(file)
        .ok_or_else(|| DestinationError::MissingSource(file.clone()))?;
    let source = std::str::from_utf8(source)
        .map_err(|_| DestinationError::UnsupportedOutline(label.into()))?;
    appendix_target(source, label, Reading::Paper)
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
        let source = "\\appendix\n\\section{First}\n\\label{app:first}\n\\section{The method's self-limit}\n\\label{app:self-limit}\n";
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
        let ordinary = "\\appendix\n\\section{First}\n\\label{app:first}\n";
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
