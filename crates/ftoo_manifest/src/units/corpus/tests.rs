use super::import::{COMMANDS, Production, production};
use super::*;

fn draft(text: &str) -> SourceDraft {
    import_source(
        "documents/paper/parts/front.tex",
        text.as_bytes(),
        "digest".into(),
        &[],
    )
    .unwrap()
}

#[test]
fn paragraph_keeps_escaped_punctuation_and_inline_structure() {
    let source = "\\label{sec:test}\n\\PaperParagraph{stretch=2em}A 20\\% share, \\emph{strong words}, and \\ftag{test:key}.\n";
    let result = draft(source);
    assert_eq!(result.byte_length, source.len());
    assert!(
        result
            .events
            .iter()
            .any(|event| event.source.starts_with("\\label"))
    );
    assert!(
        result
            .events
            .iter()
            .any(|event| event.source.starts_with("\\PaperParagraph"))
    );
    let paragraph = result
        .blocks
        .iter()
        .find(|block| block.kind == BlockKind::Paragraph)
        .unwrap();
    assert_eq!(result.blocks.len(), 1);
    let BlockFields::Paragraph { runs, .. } = &paragraph.fields else {
        panic!("paragraph fields")
    };
    assert!(matches!(
        runs.as_slice(),
        [
            Run::Text { decoded: first, .. },
            Run::Style { style, runs: children },
            Run::Text { decoded: middle, .. },
            Run::Tag { key, .. },
            Run::Text { decoded: last, .. },
        ] if first == "A 20% share, "
            && style == "emph"
            && matches!(children.as_slice(), [Run::Text { decoded, .. }] if decoded == "strong words")
            && middle == ", and "
            && key == "test:key"
            && last == ".\n"
    ));
    assert_eq!(paragraph.site.start_byte, source.find("A 20").unwrap());
    assert_eq!(paragraph.site.end_byte, source.len());
    assert_eq!(result.metadata_targets.len(), 2);
    assert_eq!(result.metadata_targets[0].source_command, "\\label");
    assert_eq!(result.metadata_targets[0].exact_key, "sec:test");
    assert_eq!(result.metadata_targets[0].site.start_byte, 0);
    assert_eq!(
        result.metadata_targets[0].site.end_byte,
        "\\label{sec:test}".len()
    );
    assert_eq!(result.metadata_targets[1].source_command, "\\ftag");
    assert_eq!(result.metadata_targets[1].exact_key, "test:key");
    assert_eq!(
        result.metadata_targets[1].site.start_byte,
        source.find("\\ftag").unwrap()
    );
    assert_eq!(result.metadata_targets[1].occurrence, 1);
    assert_eq!(result.refusals.len(), 0);
}

#[test]
fn en_space_is_a_named_intent_and_following_words_remain_content() {
    let first = draft("\\textbf{Ties.}\\enspace Next word.\n");
    let changed = draft("\\textbf{Ties.}\\enspace Other word.\n");
    let BlockFields::Paragraph { runs, .. } = &first.blocks[0].fields else {
        panic!("paragraph fields")
    };
    assert!(
        matches!(runs.as_slice(), [Run::Style { .. }, Run::Text { decoded, spacing_intent }, Run::Text { decoded: following, .. }]
        if decoded == " " && spacing_intent == "en-space" && following == "Next word.\n")
    );
    assert_ne!(first.blocks[0].fields, changed.blocks[0].fields);
    let unsupported = draft("Words \\quad follow.\n");
    assert_eq!(unsupported.refusals[0].code, "unsupported command \\quad");
    let nested = draft("\\emph{word \\quad} remains.\n");
    assert_eq!(nested.blocks[0].status, BlockStatus::Unresolved);
    let line_end = draft("\\textbf{Ties.}\\enspace\nNext word.\n");
    let BlockFields::Paragraph { runs, .. } = &line_end.blocks[0].fields else {
        panic!("paragraph fields")
    };
    assert!(
        matches!(runs.as_slice(), [Run::Style { .. }, Run::Text { decoded, spacing_intent }, Run::Text { decoded: following, .. }]
        if decoded == " " && spacing_intent == "en-space" && following == "Next word.\n")
    );
}

#[test]
fn one_command_table_distinguishes_exact_names() {
    assert_eq!(production("\\enspace"), Some(Production::Spacing));
    assert_eq!(production("\\gradeword"), None);
    assert_eq!(production("\\parencite"), Some(Production::Citation));
    let names: std::collections::BTreeSet<_> = COMMANDS.iter().map(|(name, _)| name).collect();
    assert_eq!(names.len(), COMMANDS.len());
    let prefix =
        draft("\\PaperDisplayParagraphX{equation}{lastline}\\begin{equation}x\\end{equation}\n");
    assert_eq!(
        prefix.refusals[0].code,
        "unsupported command \\PaperDisplayParagraphX"
    );
}

#[test]
fn malformed_hook_argument_refuses_instead_of_absorbing_content() {
    let source = "\\spend{open \\unknown\nNext.\n";
    let result = draft(source);
    assert_eq!(result.refusals[0].code, "unbalanced argument of \\spend");
    assert!(
        result
            .refusals
            .iter()
            .any(|fault| fault.code == "unsupported command \\unknown")
    );
    assert_eq!(result.accounting.last().unwrap().end_byte, source.len());
}

#[test]
fn comment_braces_do_not_close_a_hook_argument() {
    let source = "\\grade{nec% } ignored\nplus}\n";
    let result = draft(source);
    assert_eq!(result.events[0].source, "\\grade{nec% } ignored\nplus}");
}

#[test]
fn refused_block_keeps_each_located_fault() {
    let result = draft("A \\unknown{x}, $x$, and \\unknown{y}.\n");
    assert_eq!(result.blocks.len(), 1);
    assert_eq!(result.refusals.len(), 3);
    assert_eq!(
        result
            .refusals
            .iter()
            .filter(|fault| fault.is_primary)
            .count(),
        1
    );
    assert_eq!(
        result
            .refusals
            .iter()
            .filter(|fault| fault.code == "unsupported command \\unknown")
            .count(),
        2
    );
    assert_eq!(
        result
            .refusals
            .iter()
            .filter(|fault| fault.code == "unresolved mathematics")
            .count(),
        1
    );
    assert!(
        result
            .refusals
            .iter()
            .all(|fault| fault.outer_block_index == 0)
    );
    let comment = draft("A \\emph{first% } ignored\nsecond}.\n");
    assert!(
        comment
            .refusals
            .iter()
            .any(|fault| fault.code == "comment in content")
    );
    let display = draft("\\begin{equation}x\\end{equation}\n");
    assert_eq!(display.refusals.len(), 1);
    assert_eq!(display.refusals[0].code, "unsupported environment equation");
    assert_eq!(display.math.len(), 1);
}

#[test]
fn container_children_begin_after_header_and_group_paragraph_lines() {
    let source =
        "\\begin{remark}[A title]First line\ncontinues.\n\nSecond paragraph.\n\\end{remark}\n";
    let result = draft(source);
    assert_eq!(result.blocks[0].children.len(), 2);
    assert_eq!(
        result.blocks[0].children[0].site.start_byte,
        source.find("First line").unwrap()
    );
    assert_eq!(result.blocks[0].children[0].site.end_line, 2);
    assert_eq!(result.blocks[0].children[1].site.start_line, 4);
    assert_eq!(result.blocks[0].status, BlockStatus::Unresolved);
    let hooked = draft("\\begin{remark}[T]\\grade{nec}\nProse.\n\\end{remark}\n");
    assert_eq!(hooked.blocks[0].children.len(), 1);
    assert_eq!(hooked.blocks[0].children[0].status, BlockStatus::Decoded);
    assert_eq!(hooked.blocks[0].children[0].site.start_line, 2);
    let display = draft(
        "\\begin{remark}[T]\\PaperDisplayParagraph{equation}{lastline}\\begin{equation}\nx\n\\end{equation}\nAfter.\n\\end{remark}\n",
    );
    assert_eq!(display.blocks[0].children[0].kind, BlockKind::Display);
    assert_eq!(display.blocks[0].children[1].kind, BlockKind::Paragraph);
}

#[test]
fn handle_target_is_located_and_refused_runs_propose_nothing() {
    let source = "A \\handle*{act:one} follows.\n";
    let result = draft(source);
    assert_eq!(result.metadata_targets.len(), 1);
    assert_eq!(result.metadata_targets[0].source_command, "\\handle*");
    assert_eq!(result.metadata_targets[0].exact_key, "act:one");
    assert_eq!(result.metadata_targets[0].site.start_byte, 2);
    assert_eq!(
        result.metadata_targets[0].site.end_byte,
        2 + "\\handle*{act:one}".len()
    );
    let refused = draft("A \\handle*{act:one} and \\unknown{word}.\n");
    assert_eq!(refused.metadata_targets.len(), 0);
    assert_eq!(refused.refusals[0].reason, "unsupported command \\unknown");
    let heading = draft("\\section*{A \\handle{act:one}}\n");
    assert_eq!(heading.metadata_targets.len(), 1);
    assert_eq!(
        heading.metadata_targets[0].site.start_byte,
        "\\section*{A ".len()
    );
}

#[test]
fn balanced_container_is_located_residue() {
    let source =
        "\\begin{remark}\n\\begin{equation}x=1\\end{equation}\nThen continues.\n\\end{remark}\n";
    let result = draft(source);
    assert_eq!(result.blocks.len(), 1);
    assert_eq!(result.blocks[0].kind, BlockKind::TheoremContainer);
    assert_eq!(result.refusals[0].reason, "unsupported environment remark");
    assert_eq!(result.blocks[0].children.len(), 2);
    assert_eq!(result.blocks[0].children[0].kind, BlockKind::Display);
    assert!(matches!(
        result.blocks[0].children[1].fields,
        BlockFields::Paragraph {
            continuation: true,
            ..
        }
    ));
    assert_eq!(result.accounting[0].end_byte, source.len());
}

#[test]
fn braced_table_cell_keeps_its_column() {
    let source = "\\begin{tabular}{cc}\na & {b & c} \\\\\n\\end{tabular}\n";
    let result = draft(source);
    let BlockFields::Table { rows, .. } = &result.blocks[0].fields else {
        panic!("table fields")
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 2);
    assert_eq!(rows[0][1].text, "b & c");
}

#[test]
fn paragraph_hooks_join_following_lines_and_prefix_commands_do_not_split() {
    let joined = draft("\\PaperParagraph{stretch=2em}The first line\ncontinues here.\n");
    assert_eq!(joined.blocks.len(), 1);
    assert_eq!(joined.refusals.len(), 0);
    assert_eq!(joined.blocks[0].site.end_line, 2);
    let prefix = draft("The first line\n\\gradeword{nec} appears here\nand continues.\n");
    assert_eq!(prefix.blocks.len(), 1);
    assert_eq!(prefix.refusals.len(), 1);
    assert_eq!(prefix.refusals[0].reason, "unsupported command \\gradeword");
    assert_eq!(prefix.blocks[0].site.end_line, 3);
    let citation = draft("The first line\n\\parencite{a:key} continues here.\n");
    assert_eq!(citation.blocks.len(), 1);
    assert_eq!(citation.refusals.len(), 0);
}

#[test]
fn display_continuation_and_heading_star_are_retained() {
    let result = draft("\\begin{equation}x=1\\end{equation}\nAfterwards.\n");
    assert!(matches!(
        result.blocks[1].fields,
        BlockFields::Paragraph {
            continuation: true,
            ..
        }
    ));
    let heading = draft("\\part*{Unnumbered}\n");
    assert!(matches!(
        heading.blocks[0].fields,
        BlockFields::Heading { star: true, .. }
    ));
}

#[test]
fn environment_names_and_trailing_table_bytes_refuse() {
    for (name, kind) in [
        ("econ", BlockKind::EconomicsBox),
        ("epist", BlockKind::EpistemologyBox),
        ("definition", BlockKind::TheoremContainer),
        ("postulate", BlockKind::TheoremContainer),
        ("insight", BlockKind::TheoremContainer),
    ] {
        let source = format!("\\begin{{{name}}}A\\end{{{name}}}\n");
        let result = draft(&source);
        assert_eq!(result.blocks[0].kind, kind);
        assert_eq!(
            result.refusals[0].code,
            format!("unsupported environment {name}")
        );
    }
    let table = draft("\\begin{tabular}{c}a\\\\\\end{tabular}trailing\n");
    assert_eq!(table.refusals[0].code, "unsupported environment tabular");
}

#[test]
fn unadmitted_virtue_runs_and_unicode_command_names_refuse() {
    for command in ["\\willvirtue{love}", "\\gensym{love}", "\\é{word}"] {
        let result = draft(&format!("A {command}.\n"));
        assert_eq!(result.refusals.len(), 1);
        assert!(result.refusals[0].code.starts_with("unsupported command"));
    }
}

#[test]
fn accounting_mutations_refuse_their_own_fault() {
    let source = "Café.\n\nMore.\n";
    let original = draft(source);
    let mut hole = original.clone();
    hole.accounting[0].start_byte = 1;
    assert!(matches!(
        validate_accounting(&hole, source.as_bytes()),
        Err(ImportFault::Accounting { reason: "hole", .. })
    ));
    let mut overlap = original.clone();
    overlap.accounting[1].start_byte = 0;
    assert!(matches!(
        validate_accounting(&overlap, source.as_bytes()),
        Err(ImportFault::Accounting {
            reason: "overlap",
            ..
        })
    ));
    let mut split = original.clone();
    split.accounting[0].end_byte = 4;
    split.accounting[1].start_byte = 4;
    assert!(matches!(
        validate_accounting(&split, source.as_bytes()),
        Err(ImportFault::Accounting {
            reason: "unicode boundary",
            ..
        })
    ));
    let mut owner = original;
    owner.accounting[0].owner = AccountOwner::Content(100);
    assert!(matches!(
        validate_accounting(&owner, source.as_bytes()),
        Err(ImportFault::Accounting {
            reason: "missing owner",
            ..
        })
    ));
}

#[test]
fn unsupported_command_and_broken_environment_are_not_decoded() {
    let command = draft("A \\unknown{value} word.\n");
    assert_eq!(command.refusals[0].reason, "unsupported command \\unknown");
    assert_eq!(command.refusals[0].site.start_byte, 2);
    assert_eq!(command.refusals[0].site.end_byte, 2 + "\\unknown".len());
    let option = draft("A \\emph[unknown]{value} word.\n");
    assert_eq!(
        option.refusals[0].reason,
        "unsupported optional argument of \\emph"
    );
    assert_eq!(option.refusals[0].site.start_byte, 2);
    assert_eq!(option.refusals[0].site.end_byte, 2 + "\\emph[".len());
    let broken = draft("\\begin{remark}\nunfinished\n");
    assert_eq!(broken.refusals[0].reason, "unbalanced environment");
    assert_eq!(
        broken.accounting.last().unwrap().end_byte,
        broken.byte_length
    );
}

#[test]
fn known_marker_is_not_copied_as_prose() {
    let source = "% generated: sec:test:body\n";
    let result = import_source(
        "documents/paper/parts/front.tex",
        source.as_bytes(),
        "digest".into(),
        &[("sec:test:body".into(), "paragraph".into())],
    )
    .unwrap();
    assert_eq!(result.blocks[0].kind, BlockKind::ExistingNativeInclusion);
    assert!(matches!(
        result.accounting[0].owner,
        AccountOwner::ExistingNative(0)
    ));
    let unowned = draft(source);
    assert_eq!(unowned.refusals[0].reason, "unowned generated marker");
    assert!(matches!(
        unowned.accounting[0].owner,
        AccountOwner::Refusal(0)
    ));
    for altered in [
        "  % generated: sec:test:body\n",
        "% generated: sec:test:body\r\n",
        "\\label{sec:other}% generated: sec:test:body\n",
    ] {
        let result = import_source(
            "documents/paper/parts/front.tex",
            altered.as_bytes(),
            "digest".into(),
            &[("sec:test:body".into(), "paragraph".into())],
        )
        .unwrap();
        assert_eq!(result.refusals[0].code, "unowned generated marker");
    }
}

#[test]
fn changed_captured_input_refuses() {
    assert!(matches!(
        ensure_unchanged("source.tex", b"same", b"sane"),
        Err(ImportFault::InputChanged { .. })
    ));
}

#[test]
fn accounting_refusal_owner_must_name_the_first_fault() {
    let source = "A \\unknown{x}, $x$.\n";
    let mut result = draft(source);
    assert_eq!(result.refusals.len(), 2);
    assert!(result.refusals[0].is_primary);
    assert!(!result.refusals[1].is_primary);
    result.accounting[0].owner = AccountOwner::Refusal(1);
    assert!(matches!(
        validate_accounting(&result, source.as_bytes()),
        Err(ImportFault::Accounting {
            reason: "secondary refusal owner",
            ..
        })
    ));
}

#[test]
fn crossing_group_is_located_residue_and_a_broken_group_claims_the_suffix() {
    let crossing = draft("A \\emph{one\n\n two}.\nAfter.\n");
    assert_eq!(
        crossing.refusals[0].reason,
        "argument crosses block boundary"
    );
    assert!(
        crossing
            .blocks
            .iter()
            .any(|block| block.status == BlockStatus::Decoded)
    );
    let broken = draft("A \\emph{one\n\nStill inside.\n");
    assert_eq!(broken.refusals[0].reason, "unbalanced argument of \\emph");
    assert_eq!(
        broken.accounting.last().unwrap().end_byte,
        broken.byte_length
    );
}
