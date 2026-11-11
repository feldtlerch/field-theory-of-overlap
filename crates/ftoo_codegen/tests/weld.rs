use ftoo_codegen::{
    WeldCheckSummary, check_projection, check_weld_files, load_markdown_files, load_unit_files,
};
use ftoo_manifest::{
    LabelMember, LabelUseSite, PathPart, SourceAnchor, UnitSite, integrate_labels,
    resolve_unit_tags, weld_units,
};
#[test]
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
    let expected = WeldCheckSummary {
        units: 4,
        texts: 4,
        tag_sites: 3,
        rust_mints: 4,
        markdown_mints: 117,
        unit_mints: 4,
        markdown_citations: 100,
        rust_citations: 8,
        unit_citation_sites: 1,
        mention_sites: 0,
        language_comparisons: 0,
    };
    assert_eq!(check_weld_files(&root, &book).unwrap(), expected);
    assert_eq!(check_weld_files(&root, &rust_book).unwrap(), expected);
    let corpus = load_unit_files(&root).unwrap();
    let markdown = load_markdown_files(&root).unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_labels(&resolved, &markdown).unwrap();
    let welded = weld_units(&integrated).unwrap();
    assert!(std::ptr::eq(welded.integrated(), &raw const integrated));
    assert_eq!(
        welded
            .units()
            .iter()
            .map(ftoo_manifest::WeldedUnit::unit)
            .collect::<Vec<_>>(),
        [
            "sec:coefficients:first-value",
            "sec:coefficients:overview",
            "sec:coefficients:value-context",
            "sec:coefficients:value-table"
        ]
    );
    let first = &welded.units()[0].languages()[0];
    for unit in welded.units() {
        assert_eq!(unit.languages().len(), 1);
    }
    assert_first(first);
    assert_eq!(first.labels().members, []);
    assert_eq!(first.mentions(), []);
    assert!(first.citations().is_empty());
    let heading = &welded.units()[1].languages()[0];
    assert_eq!(heading.language(), "en");
    assert_eq!(heading.tags().members, []);
    assert_eq!(heading.labels().members, []);
    assert_eq!(heading.mentions(), []);
    assert!(heading.citations().is_empty());
    let second = &welded.units()[2].languages()[0];
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
                target: ftoo_manifest::NodeId::LangevinCoefficientOne,
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
                m.target,
                m.sites
                    .iter()
                    .map(|s| (s.decoded.clone(), s.occurrence))
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "c:1",
                ftoo_manifest::NodeId::LangevinCoefficientOne,
                vec![(Some(51..60), Some(1))]
            ),
            (
                "v:1",
                ftoo_manifest::NodeId::LangevinCoefficientOneValue,
                vec![(Some(16..25), Some(0))]
            ),
        ]
    );
}
