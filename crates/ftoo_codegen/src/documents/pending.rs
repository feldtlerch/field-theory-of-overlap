use super::{
    PendingExplanation, PendingOccurrence,
    plan::{Block, Inline},
};
use ftoo_manifest::{DocumentTarget, SourceOrigin};
use std::{collections::BTreeMap, fmt::Write};

pub(super) fn entries(
    block: &Block,
    language: &str,
    target: DocumentTarget,
) -> Vec<PendingExplanation> {
    let mut entries: BTreeMap<usize, PendingExplanation> = BTreeMap::new();
    for run in block.target_runs(target) {
        let Inline::Math {
            pending,
            site,
            definition_site,
            ..
        } = run
        else {
            continue;
        };
        for entry in pending {
            let explanation = entries
                .entry(entry.order())
                .or_insert_with(|| PendingExplanation {
                    definition: entry.definition(),
                    body_macro: entry.body_macro().into(),
                    body_unicode: entry.body_unicode().into(),
                    reasons: entry.reasons().to_vec(),
                    names: block.pending_names[&(target, entry.definition())].clone(),
                    occurrences: vec![],
                });
            explanation.occurrences.push(PendingOccurrence {
                definition: SourceOrigin {
                    unit: Some(block.unit.clone()),
                    language: Some(language.into()),
                    target: None,
                    site: definition_site.clone(),
                },
                source: SourceOrigin {
                    unit: Some(block.unit.clone()),
                    language: Some(language.into()),
                    target: Some(target),
                    site: site.clone(),
                },
                notation: entry.origins().to_vec(),
            });
        }
    }
    entries.into_values().collect()
}

/// Break opportunities change neither the token's spelling nor its visible text.
pub(super) fn identity_text(source: &str) -> String {
    let mut output = String::new();
    for (index, token) in source.split(' ').enumerate() {
        if index != 0 {
            output.push(' ');
        }
        let mut offset = 0;
        for segment in token.split_inclusive([':', '-', '/', '.']) {
            let letters = segment.trim_end_matches([':', '-', '/', '.']);
            let long = letters.chars().count() > 24;
            for (at, scalar) in segment.char_indices() {
                output.push_str(&super::latex::text(&scalar.to_string()));
                let end = at + scalar.len_utf8();
                if (long && end < letters.len())
                    || (end == segment.len() && offset + end < token.len())
                {
                    output.push_str(r"\allowbreak{}");
                }
            }
            offset += segment.len();
        }
    }
    output
}

fn render<'a>(
    entries: impl Iterator<Item = (&'a str, &'a [ftoo_manifest::PendingReason])>,
    target: DocumentTarget,
) -> String {
    let mut output = String::new();
    for (index, (body, reasons)) in entries.enumerate() {
        debug_assert!(
            !reasons.is_empty(),
            "pending tier requires at least one reason"
        );
        output.push_str(if index == 0 {
            "Pending notation: "
        } else {
            ". "
        });
        let reasons = reasons
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        match target {
            DocumentTarget::Markdown => {
                write!(output, "${body}$: {}", super::markdown::text(&reasons))
            }
            DocumentTarget::Latex => write!(output, "\\({body}\\): {}", identity_text(&reasons)),
        }
        .expect("string write");
    }
    if !output.is_empty() {
        output.push_str(".\n\n");
    }
    output
}

pub(super) fn paragraph(block: &Block, language: &str, target: DocumentTarget) -> String {
    let entries = entries(block, language, target);
    let mut runs = vec![];
    for (index, entry) in entries.iter().enumerate() {
        debug_assert!(
            !entry.reasons.is_empty(),
            "pending tier requires at least one reason"
        );
        runs.push(Inline::Text(
            if index == 0 {
                "Pending notation: "
            } else {
                ". "
            }
            .into(),
        ));
        runs.push(Inline::Math {
            definition_site: block.site.clone(),
            key: String::new(),
            markdown: entry.body_unicode.clone(),
            latex: entry.body_macro.clone(),
            pending: vec![],
            site: block.site.clone(),
        });
        runs.push(Inline::Text(": ".into()));
        let mut names = entry.names.iter();
        for (reason, value) in entry.reasons.iter().enumerate() {
            if reason != 0 {
                runs.push(Inline::Text("; ".into()));
            }
            for part in value.text_parts() {
                match part {
                    ftoo_manifest::ReasonPart::Text(text) => runs.push(Inline::Text(text)),
                    ftoo_manifest::ReasonPart::Object { .. } => {
                        let name = names.next().expect("every reason object resolved");
                        runs.push(Inline::Tag {
                            handle: name.handle.clone(),
                            label: name.label.clone(),
                            token: name.token.clone(),
                            site: block.site.clone(),
                        });
                    }
                }
            }
        }
    }
    if runs.is_empty() {
        return String::new();
    }
    runs.push(Inline::Text(".".into()));
    let mut output = String::new();
    match target {
        DocumentTarget::Markdown => super::markdown::inline(&mut output, &runs),
        DocumentTarget::Latex => super::latex::inline(&mut output, &runs),
    }
    output.push_str("\n\n");
    output
}

pub(super) fn sweep(entries: &[crate::PendingEntry]) -> String {
    let paragraph = render(
        entries
            .iter()
            .map(|entry| (entry.body_macro(), entry.reasons())),
        DocumentTarget::Latex,
    );
    if paragraph.is_empty() {
        paragraph
    } else {
        format!(
            "{{\\raggedright\n{}\\par}}\n\n",
            paragraph.trim_end_matches('\n')
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_multiple_reason_text_stays_in_order() {
        use ftoo_manifest::{Blocker, NodeId, PendingReason};
        let reasons = [
            PendingReason::FormulationProposed {
                object: NodeId::LangevinCoefficientOneValue,
            },
            PendingReason::FormulationProposed {
                object: NodeId::LangevinCoefficientOne,
            },
            PendingReason::ClaimDeferred {
                object: NodeId::LangevinCoefficientOneValue,
                blocker: Blocker::AuditRunRequired,
            },
        ];
        assert_eq!(
            render(
                std::iter::once((r"({{c}_{1}}={\frac{1}{3}})", reasons.as_slice())),
                DocumentTarget::Markdown
            ),
            "Pending notation: $({{c}_{1}}={\\frac{1}{3}})$: retained formulation claim:langevin:coefficient&#45;one&#45;value; retained formulation def:langevin:coefficient&#45;one; claim evidence claim:langevin:coefficient&#45;one&#45;value awaits audit&#45;run&#45;required.\n\n"
        );
        assert_eq!(
            render(
                std::iter::once((r"({{c}_{1}}={\frac{1}{3}})", reasons.as_slice())),
                DocumentTarget::Latex
            ),
            "Pending notation: \\(({{c}_{1}}={\\frac{1}{3}})\\): retained formulation claim:\\allowbreak{}langevin:\\allowbreak{}coefficient-\\allowbreak{}one-\\allowbreak{}value; retained formulation def:\\allowbreak{}langevin:\\allowbreak{}coefficient-\\allowbreak{}one; claim evidence claim:\\allowbreak{}langevin:\\allowbreak{}coefficient-\\allowbreak{}one-\\allowbreak{}value awaits audit-\\allowbreak{}run-\\allowbreak{}required.\n\n"
        );
    }
    #[test]
    fn breaks_preserve_complete_identity_tokens() {
        assert_eq!(
            identity_text("retained formulation def:example:coefficient-one"),
            r"retained formulation def:\allowbreak{}example:\allowbreak{}coefficient-\allowbreak{}one"
        );
        assert_eq!(
            identity_text("a/b.c-d:e"),
            r"a/\allowbreak{}b.\allowbreak{}c-\allowbreak{}d:\allowbreak{}e"
        );
        assert_eq!(
            identity_text("abcdefghijklmnopqrstuvwx"),
            "abcdefghijklmnopqrstuvwx"
        );
        assert_eq!(
            identity_text("abcdefghijklmnopqrstuvwxy"),
            r"a\allowbreak{}b\allowbreak{}c\allowbreak{}d\allowbreak{}e\allowbreak{}f\allowbreak{}g\allowbreak{}h\allowbreak{}i\allowbreak{}j\allowbreak{}k\allowbreak{}l\allowbreak{}m\allowbreak{}n\allowbreak{}o\allowbreak{}p\allowbreak{}q\allowbreak{}r\allowbreak{}s\allowbreak{}t\allowbreak{}u\allowbreak{}v\allowbreak{}w\allowbreak{}x\allowbreak{}y"
        );
    }
}
