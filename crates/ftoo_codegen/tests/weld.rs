use ftoo_codegen::{
    UnitSource, WeldCheckSummary, check_projection, check_weld_files, load_markdown_files,
    load_unit_files, load_unit_sources,
};
use ftoo_manifest::{
    LabelMember, LabelUseSite, PathPart, SourceAnchor, UnitSite, integrate_labels,
    resolve_unit_tags, weld_units,
};

fn source_span_counts(text: &str) -> (usize, usize) {
    let backticks = text.bytes().filter(|byte| *byte == b'`').count();
    let double_delimiters = text.match_indices("``").count();
    assert_eq!(backticks % 2, 0);
    assert_eq!(double_delimiters % 2, 0);
    (backticks / 2 - double_delimiters, double_delimiters / 2)
}

fn expected_weld_summary(
    corpus: &ftoo_manifest::LanguageCheckedUnits,
    markdown: &ftoo_manifest::CheckedMarkdown,
    graph: &ftoo_manifest::ValidatedManifest,
    book: &ftoo_manifest::AddressBook<'_>,
) -> WeldCheckSummary {
    let (unit_citation_sites, mention_sites) = corpus
        .units()
        .iter()
        .flat_map(|unit| unit.texts().values())
        .map(|text| source_span_counts(text.text()))
        .fold(
            (0, 0),
            |(citations, mentions), (next_citations, next_mentions)| {
                (citations + next_citations, mentions + next_mentions)
            },
        );
    WeldCheckSummary {
        units: corpus.units().len(),
        texts: corpus.units().iter().map(|unit| unit.texts().len()).sum(),
        tag_sites: corpus
            .units()
            .iter()
            .flat_map(|unit| unit.texts().values())
            .map(|text| text.tag_occurrences().len())
            .sum(),
        rust_mints: graph.nodes().len(),
        markdown_mints: markdown.mints().len(),
        unit_mints: corpus.units().len(),
        markdown_citations: markdown.citations().len(),
        rust_citations: book.history().count(),
        unit_citation_sites,
        mention_sites,
        language_comparisons: corpus
            .units()
            .iter()
            .map(|unit| corpus.documents()[unit.document()].len().saturating_sub(1))
            .sum(),
    }
}

#[test]
#[allow(clippy::too_many_lines)] // Full-corpus and permanent-domain controls retain their independent inventories.
fn w13_committed_citation_has_complete_weld_and_unchanged_tag_indices() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    check_projection(&root).unwrap();
    let bytes = std::fs::read(root.join("generated/manifest.json")).unwrap();
    let loaded = ftoo_manifest::load_projection(&bytes).unwrap();
    let book = loaded.addressing().unwrap();
    let graph = ftoo_manifest::seed().unwrap();
    let rust_book = ftoo_manifest::addressing(&graph).unwrap();
    let before = ftoo_manifest::Snapshot::capture(&graph);
    let statuses = ftoo_manifest::NodeId::ALL.map(|id| graph.status(id));
    let corpus = load_unit_files(&root).unwrap();
    let markdown = load_markdown_files(&root).unwrap();
    let labels = ftoo_codegen::check_label_files(&root, &book).unwrap();
    let expected = expected_weld_summary(&corpus, &markdown, &graph, &book);
    assert_eq!(check_weld_files(&root, &book).unwrap(), expected);
    assert_eq!(check_weld_files(&root, &rust_book).unwrap(), expected);
    assert_eq!(labels.units, expected.units);
    assert_eq!(labels.texts, expected.texts);
    assert_eq!(labels.tag_sites, expected.tag_sites);
    let declaration = UnitSource {
        file: "units/documents.toml".into(),
        bytes: std::fs::read(root.join("units/documents.toml")).unwrap(),
    };
    let paper_labels = declaration.paper_context_unit_labels(&corpus).unwrap();
    let permanent_files: std::collections::BTreeSet<_> = corpus
        .units()
        .iter()
        .filter(|unit| !paper_labels.contains(unit.label().as_str()))
        .map(|unit| unit.texts().values().next().unwrap().site().file.clone())
        .collect();
    let permanent = load_unit_sources(
        declaration,
        std::fs::read_dir(root.join("units/statements"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension().is_some_and(|ext| ext == "toml")
                    && permanent_files.contains(&format!(
                        "units/statements/{}",
                        path.file_name().unwrap().to_string_lossy()
                    ))
            })
            .map(|path| UnitSource {
                file: format!(
                    "units/statements/{}",
                    path.file_name().unwrap().to_string_lossy()
                ),
                bytes: std::fs::read(path).unwrap(),
            })
            .collect(),
    )
    .unwrap();
    let resolved = resolve_unit_tags(&permanent, &book).unwrap();
    let integrated = integrate_labels(&resolved, &markdown).unwrap();
    let welded = weld_units(&integrated).unwrap();
    assert!(std::ptr::eq(welded.integrated(), &raw const integrated));
    assert_eq!(
        welded
            .units()
            .iter()
            .map(ftoo_manifest::WeldedUnit::unit)
            .collect::<Vec<_>>(),
        permanent
            .units()
            .iter()
            .map(|unit| unit.label().as_str())
            .collect::<Vec<_>>()
    );
    for (welded_unit, source_unit) in welded.units().iter().zip(permanent.units()) {
        assert_eq!(
            welded_unit
                .languages()
                .iter()
                .map(ftoo_manifest::WeldedLanguage::language)
                .collect::<Vec<_>>(),
            corpus.documents()[source_unit.document()]
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );
    }
    let first = &welded
        .units()
        .iter()
        .find(|unit| unit.unit() == "sec:coefficients:first-value")
        .unwrap()
        .languages()[0];
    assert_first(first);
    assert_eq!(first.labels().members, []);
    assert_eq!(first.mentions(), []);
    assert!(first.citations().is_empty());
    let heading = &welded
        .units()
        .iter()
        .find(|unit| unit.unit() == "sec:coefficients:overview")
        .unwrap()
        .languages()[0];
    assert_eq!(heading.language(), "en");
    assert_eq!(heading.tags().members, []);
    assert_eq!(heading.labels().members, []);
    assert_eq!(heading.mentions(), []);
    assert!(heading.citations().is_empty());
    let second = &welded
        .units()
        .iter()
        .find(|unit| unit.unit() == "sec:coefficients:value-context")
        .unwrap()
        .languages()[0];
    assert_eq!(second.language(), "en");
    assert_eq!(second.mentions(), []);
    assert_context(second);
    assert_eq!(
        std::fs::read(root.join("generated/manifest.json")).unwrap(),
        bytes
    );
    assert_eq!(ftoo_manifest::Snapshot::capture(&graph), before);
    assert_eq!(
        ftoo_manifest::NodeId::ALL.map(|id| graph.status(id)),
        statuses
    );
}

fn assert_first(first: &ftoo_manifest::WeldedLanguage) {
    let text_site = UnitSite {
        file: "units/statements/first-value.toml".into(),
        path: ["unit", "texts", "en"]
            .map(|s| PathPart::Key(s.into()))
            .to_vec(),
        anchor: SourceAnchor::Bytes(152, 213),
        decoded: None,
        occurrence: None,
    };
    assert_eq!(
        first.labels(),
        &ftoo_manifest::LabelSide {
            language: "en".into(),
            text_site: text_site.clone(),
            members: vec![]
        }
    );
    let mut tag_site = text_site.clone();
    tag_site.decoded = Some(22..31);
    tag_site.occurrence = Some(0);
    assert_eq!(
        first.tags(),
        &ftoo_manifest::TagSide {
            language: "en".into(),
            text_site,
            members: vec![ftoo_manifest::TagMember {
                input: "c:1".into(),
                target: ftoo_manifest::ResolvedTagTarget::Theory(
                    ftoo_manifest::NodeId::LangevinCoefficientOne,
                ),
                sites: vec![tag_site]
            }]
        }
    );
}

fn assert_context(second: &ftoo_manifest::WeldedLanguage) {
    let location = LabelUseSite {
        span_index: 0,
        site: UnitSite {
            file: "units/statements/value-context.toml".into(),
            path: ["unit", "texts", "en"]
                .map(|s| PathPart::Key(s.into()))
                .to_vec(),
            anchor: SourceAnchor::Bytes(104, 202),
            decoded: Some(66..95),
            occurrence: None,
        },
    };
    let mut text_site = location.site.clone();
    text_site.decoded = None;
    assert_eq!(second.labels().language, "en");
    assert_eq!(second.labels().text_site, text_site);
    assert_eq!(second.tags().language, "en");
    assert_eq!(second.tags().text_site, text_site);
    for tag in &second.tags().members {
        for site in &tag.sites {
            assert_eq!(site.file, text_site.file);
            assert_eq!(site.path, text_site.path);
            assert_eq!(site.anchor, text_site.anchor);
        }
    }
    assert_eq!(
        second.labels().members,
        [LabelMember {
            label: "dec:weld:equal-address-sets".into(),
            sites: vec![location.clone()]
        }]
    );
    assert_eq!(second.citations().len(), 1);
    assert_eq!(second.citations()[0].label(), "dec:weld:equal-address-sets");
    assert_eq!(second.citations()[0].site(), &location);
    assert_eq!(
        second.citations()[0].origin(),
        &ftoo_manifest::MintOrigin {
            kind: ftoo_manifest::MintKind::Markdown,
            site: ftoo_manifest::LabelSite::Markdown {
                file: "docs/adr/012-the-weld.md".into(),
                bytes: 117..149,
            },
        }
    );
    assert_eq!(
        second
            .tags()
            .members
            .iter()
            .map(|m| (
                m.input.as_str(),
                m.target.clone(),
                m.sites
                    .iter()
                    .map(|s| (s.decoded.clone(), s.occurrence))
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "c:1",
                ftoo_manifest::ResolvedTagTarget::Theory(
                    ftoo_manifest::NodeId::LangevinCoefficientOne,
                ),
                vec![(Some(51..60), Some(1))]
            ),
            (
                "v:1",
                ftoo_manifest::ResolvedTagTarget::Theory(
                    ftoo_manifest::NodeId::LangevinCoefficientOneValue,
                ),
                vec![(Some(16..25), Some(0))]
            ),
        ]
    );
}
