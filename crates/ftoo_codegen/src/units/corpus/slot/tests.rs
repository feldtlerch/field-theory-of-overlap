use super::*;
use crate::units::corpus::digest;
use ftoo_manifest::corpus::{
    SlotFault, admit_slot, admitted_intervals, check_transfer, import_source,
};
use std::{fs, path::Path};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture() -> String {
    fs::read_to_string(root().join("units/fixtures/explainer-plain.toml")).unwrap()
}

fn inherited_source() -> Vec<u8> {
    fs::read(root().join("units/fixtures/explainer-prefix-source.tex")).unwrap()
}

fn parsed(text: &str) -> Result<ftoo_manifest::corpus::SlotDraft, String> {
    parse_slot(&UnitSource {
        file: "units/fixtures/explainer-plain.toml".into(),
        bytes: text.as_bytes().to_vec(),
    })
    .map_err(|error| error.to_string())
}

fn checked(text: &str) -> Result<ftoo_manifest::corpus::CheckedSlot, SlotFault> {
    let units = crate::load_unit_files(&root()).unwrap();
    admit_slot(
        parsed(text).unwrap(),
        "units/fixtures/explainer-plain.toml",
        |unit, document, language| {
            units.units().iter().any(|entry| {
                entry.label().as_str() == unit
                    && entry.document() == document
                    && entry.texts().contains_key(language)
            })
        },
    )
}

fn imported(bytes: &[u8]) -> ftoo_manifest::corpus::SourceDraft {
    let path = "documents/paper/explainer/explainer.tex";
    let markers = fs::read_to_string(root().join("units/fixtures/explainer-prefix-inclusions.tsv"))
        .unwrap()
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            (fields.first().copied() == Some(path))
                .then(|| (fields[1].to_owned(), fields[3].to_owned()))
        })
        .collect::<Vec<_>>();
    import_source(path, bytes, digest(bytes), &markers).unwrap()
}

#[test]
fn real_prefix_admits_exact_six_children_and_all_source_pieces() {
    let slot = checked(&fixture()).unwrap();
    let bytes = inherited_source();
    let draft = imported(&bytes);
    check_transfer(
        &slot,
        &draft,
        "documents/paper/explainer/explainer.tex",
        &bytes,
        44,
        98,
    )
    .unwrap();
    assert_eq!(slot.blocks().len(), 6);
    assert_eq!(slot.id(), "sec:explainer:plain");
}

#[test]
fn slot_identity_uses_the_semantic_label_grammar() {
    for id in ["sec:x", "sec::", "sec:explainer:plain::paragraph:1"] {
        let mut draft = parsed(&fixture()).unwrap();
        draft.id = id.into();
        assert_eq!(
            admit_slot(draft, "units/fixtures/explainer-plain.toml", |_, _, _| true).unwrap_err(),
            SlotFault::Identity
        );
    }
}

#[test]
fn census_reaches_a_paragraph_after_an_existing_use() {
    let bytes = inherited_source();
    let source = imported(&bytes);
    let intervals = admitted_intervals(&source, &bytes, |_, _, _| true);
    assert!(
        intervals
            .iter()
            .any(|interval| interval.first == 44 && interval.last == 98)
    );
}

#[test]
fn unknown_fields_styles_events_and_children_refuse() {
    let source = fixture();
    let unknown_field = source.replace("language = \"en\"", "language = \"en\"\nunknown = 1");
    assert!(parsed(&unknown_field).unwrap_err().contains("UnknownField"));
    let unknown_style = source.replacen("style = \"strong\"", "style = \"smallcaps\"", 1);
    assert!(parsed(&unknown_style).unwrap_err().contains("UnknownToken"));
    let unknown_event = source.replace("kind = \"heading\"", "kind = \"event\"");
    assert!(parsed(&unknown_event).unwrap_err().contains("UnknownToken"));
    let unknown_child = source.replace("kind = \"use\"", "kind = \"nested\"");
    assert!(parsed(&unknown_child).unwrap_err().contains("UnknownToken"));
    let mut omitted = parsed(&source).unwrap();
    omitted.blocks.remove(3);
    let admitted = admit_slot(omitted, "units/fixtures/explainer-plain.toml", |_, _, _| {
        true
    })
    .unwrap();
    let bytes = inherited_source();
    assert_eq!(
        check_transfer(
            &admitted,
            &imported(&bytes),
            "documents/paper/explainer/explainer.tex",
            &bytes,
            44,
            98
        )
        .unwrap_err(),
        SlotFault::ChildOrder { index: 5 }
    );
}

#[test]
fn reordered_or_changed_prose_refuses_against_the_imported_source() {
    let source = fixture();
    let bytes = inherited_source();
    let imported = imported(&bytes);
    let mut swapped = parsed(&source).unwrap();
    swapped.blocks.swap(4, 5);
    let swapped = admit_slot(swapped, "units/fixtures/explainer-plain.toml", |_, _, _| {
        true
    })
    .unwrap();
    assert_eq!(
        check_transfer(
            &swapped,
            &imported,
            "documents/paper/explainer/explainer.tex",
            &bytes,
            44,
            98
        )
        .unwrap_err(),
        SlotFault::SourceContent { index: 4 }
    );
    let changed = source.replacen("Start with two lives.", "Start with three lives.", 1);
    assert_eq!(
        check_transfer(
            &checked(&changed).unwrap(),
            &imported,
            "documents/paper/explainer/explainer.tex",
            &bytes,
            44,
            98
        )
        .unwrap_err(),
        SlotFault::SourceContent { index: 2 }
    );
}

#[test]
fn duplicate_heading_mint_and_unrelated_event_refuse_at_transfer() {
    let slot = checked(&fixture()).unwrap();
    let source = String::from_utf8(inherited_source()).unwrap();
    let duplicated = source.replacen(
        "\\label{sec:explainer:plain}",
        "\\label{sec:explainer:plain}\\label{sec:explainer:plain}",
        1,
    );
    let bytes = duplicated.as_bytes();
    assert_eq!(
        check_transfer(
            &slot,
            &imported(bytes),
            "documents/paper/explainer/explainer.tex",
            bytes,
            44,
            98
        )
        .unwrap_err(),
        SlotFault::SourceMint
    );
    let event = source.replacen(
        "\\label{sec:explainer:plain}",
        "\\label{sec:explainer:plain}\\vspace{1pt}",
        1,
    );
    let bytes = event.as_bytes();
    assert_eq!(
        check_transfer(
            &slot,
            &imported(bytes),
            "documents/paper/explainer/explainer.tex",
            bytes,
            44,
            98
        )
        .unwrap_err(),
        SlotFault::SourceEvent
    );
}

#[test]
fn native_child_owner_is_required_even_when_the_marker_exists() {
    let draft = parsed(&fixture()).unwrap();
    assert_eq!(
        admit_slot(draft, "units/fixtures/explainer-plain.toml", |_, _, _| {
            false
        })
        .unwrap_err(),
        SlotFault::ChildOwner
    );
}

#[test]
fn duplicate_child_use_refuses_without_a_second_source_marker() {
    let mut draft = parsed(&fixture()).unwrap();
    draft.blocks[2] = draft.blocks[1].clone();
    assert_eq!(
        admit_slot(draft, "units/fixtures/explainer-plain.toml", |_, _, _| true).unwrap_err(),
        SlotFault::ChildOrder { index: 2 }
    );
}

#[test]
fn changed_named_spacing_refuses_source_correspondence() {
    let bytes = inherited_source();
    let mut draft = parsed(&fixture()).unwrap();
    let ftoo_manifest::corpus::SlotBlock::Paragraph { runs } = &mut draft.blocks[2] else {
        unreachable!();
    };
    runs[1] = ftoo_manifest::corpus::SlotRun::Text(" ".into());
    let checked = admit_slot(draft, "units/fixtures/explainer-plain.toml", |_, _, _| true).unwrap();
    assert_eq!(
        check_transfer(
            &checked,
            &imported(&bytes),
            "documents/paper/explainer/explainer.tex",
            &bytes,
            44,
            98
        )
        .unwrap_err(),
        SlotFault::SourceContent { index: 2 }
    );
}

#[test]
fn tex_merging_sequences_refuse_slot_admission() {
    use ftoo_manifest::corpus::{SlotBlock, SlotRun};
    for text in [
        "well--known",
        "a''b",
        "``word",
        "well-—known",
        "hand’’s",
        "‘no’”",
    ] {
        let mut draft = parsed(&fixture()).unwrap();
        let SlotBlock::Paragraph { runs } = &mut draft.blocks[2] else {
            unreachable!();
        };
        *runs = vec![SlotRun::Text(text.into())];
        assert_eq!(
            admit_slot(draft, "units/fixtures/explainer-plain.toml", |_, _, _| true).unwrap_err(),
            SlotFault::UnsupportedRun { index: 2 }
        );
    }
    let mut draft = parsed(&fixture()).unwrap();
    let SlotBlock::Paragraph { runs } = &mut draft.blocks[2] else {
        unreachable!();
    };
    *runs = vec![
        SlotRun::Text("well-".into()),
        SlotRun::Text("-known".into()),
    ];
    assert_eq!(
        admit_slot(draft, "units/fixtures/explainer-plain.toml", |_, _, _| true).unwrap_err(),
        SlotFault::UnsupportedRun { index: 2 }
    );
}
