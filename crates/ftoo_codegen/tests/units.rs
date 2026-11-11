use ftoo_codegen::{UnitCheckSummary, check_projection, check_unit_files, load_unit_files};
use ftoo_manifest::{NodeId, load_projection, resolve_unit_tags};
#[test]
fn committed_tagged_units_resolve_through_the_checked_projection() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    check_projection(&root).unwrap();
    let loaded =
        load_projection(&std::fs::read(root.join("generated/manifest.json")).unwrap()).unwrap();
    let book = loaded.addressing().unwrap();
    assert_eq!(
        check_unit_files(&root, &book).unwrap(),
        UnitCheckSummary {
            documents: 1,
            units: 4,
            texts: 4,
            tag_sites: 3
        }
    );
    let corpus = load_unit_files(&root).unwrap();
    assert_eq!(
        corpus.documents(),
        &std::collections::BTreeMap::from([("main".to_owned(), vec!["en".to_owned()])])
    );
    assert_eq!(
        corpus
            .units()
            .iter()
            .map(|u| (u.label().as_str(), u.document()))
            .collect::<Vec<_>>(),
        vec![
            ("sec:coefficients:first-value", "main"),
            ("sec:coefficients:overview", "main"),
            ("sec:coefficients:value-context", "main"),
            ("sec:coefficients:value-table", "main")
        ]
    );
    let first = &corpus.units()[0];
    assert_eq!(
        first.texts().keys().map(String::as_str).collect::<Vec<_>>(),
        ["en"]
    );
    assert_eq!(
        first.texts()["en"].text(),
        "The first coefficient @tag{c:1} has the value @math{value}."
    );
    assert_eq!(
        first.math().keys().map(String::as_str).collect::<Vec<_>>(),
        ["value"]
    );
    assert_eq!(
        first.math()["value"].value,
        "\\FtooQCoefficientOne{} = \\frac{1}{3}"
    );
    assert_eq!(
        first.texts()["en"]
            .occurrences()
            .iter()
            .map(|o| (o.name(), o.site().decoded.clone(), o.site().occurrence))
            .collect::<Vec<_>>(),
        vec![("value", Some(46..58), Some(1))]
    );
    let second = &corpus.units()[2];
    assert_eq!(
        second
            .texts()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["en"]
    );
    assert_eq!(
        second.texts()["en"].text(),
        "The value claim @tag{v:1} concerns the coefficient @tag{c:1}. See `dec:weld:equal-address-sets`."
    );
    assert_eq!(second.math().len(), 0);
    assert_eq!(second.texts()["en"].occurrences(), []);
    assert_tag_inventory(&corpus, &book);
}

fn assert_tag_inventory(
    corpus: &ftoo_manifest::LanguageCheckedUnits,
    book: &ftoo_manifest::AddressBook<'_>,
) {
    let resolved = resolve_unit_tags(corpus, book).unwrap();
    assert_eq!(
        resolved
            .tags()
            .iter()
            .map(|t| (
                t.unit().as_str(),
                t.language(),
                t.occurrence().input(),
                t.occurrence().site().decoded.clone(),
                t.occurrence().site().occurrence,
                t.target()
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "sec:coefficients:first-value",
                "en",
                "c:1",
                Some(22..31),
                Some(0),
                NodeId::LangevinCoefficientOne
            ),
            (
                "sec:coefficients:value-context",
                "en",
                "v:1",
                Some(16..25),
                Some(0),
                NodeId::LangevinCoefficientOneValue
            ),
            (
                "sec:coefficients:value-context",
                "en",
                "c:1",
                Some(51..60),
                Some(1),
                NodeId::LangevinCoefficientOne
            ),
        ]
    );
    for tag in resolved.tags() {
        assert!(resolved.book().graph().node(tag.target()).is_some());
    }
}

#[test]
fn u10_projection_book_resolves_all_tags_at_every_position_and_language() {
    use ftoo_codegen::{UnitSource, load_unit_sources};
    use ftoo_manifest::{PathPart, SourceAnchor, UnitSite};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    check_projection(&root).unwrap();
    let loaded =
        load_projection(&std::fs::read(root.join("generated/manifest.json")).unwrap()).unwrap();
    let book = loaded.addressing().unwrap();
    let before = ftoo_manifest::Snapshot::capture(book.graph());
    let statuses = NodeId::ALL.map(|id| book.graph().status(id));
    for (input, target) in [
        ("c:1", NodeId::LangevinCoefficientOne),
        ("c:3", NodeId::LangevinCoefficientThree),
        ("v:1", NodeId::LangevinCoefficientOneValue),
        ("v:3", NodeId::LangevinCoefficientThreeValue),
    ] {
        for (text, starts) in [
            (format!("@tag{{{input}}} begins."), vec![0]),
            (format!("See @tag{{{input}}} here."), vec![4]),
            (format!("See @tag{{{input}}}"), vec![4]),
            (
                format!("See @tag{{{input}}} and @tag{{{input}}}."),
                vec![4, 18],
            ),
        ] {
            let statement = format!(
                "schema=1\n[unit]\nid='sec:coefficients:first-value'\ndocument='explainer'\nmath={{}}\n[unit.texts]\nen='{text}'\nfr='{text}'\n"
            );
            let corpus=load_unit_sources(UnitSource {file:"units/documents.toml".into(),bytes:b"schema=1\n[documents.main]\nrequired-languages=['en']\n[documents.explainer]\nrequired-languages=['en','fr']\n".to_vec()},vec![UnitSource {file:"units/statements/one.toml".into(),bytes:statement.as_bytes().to_vec()}]).unwrap();
            let resolved = resolve_unit_tags(&corpus, &book).unwrap();
            let mut expected = Vec::new();
            for language in ["en", "fr"] {
                let raw_start = statement.find(&format!("{language}=")).unwrap() + 3;
                for (index, start) in starts.iter().copied().enumerate() {
                    expected.push((
                        "sec:coefficients:first-value",
                        language,
                        input,
                        UnitSite {
                            file: "units/statements/one.toml".into(),
                            path: ["unit", "texts", language]
                                .map(|s| PathPart::Key(s.into()))
                                .to_vec(),
                            anchor: SourceAnchor::Bytes(raw_start, raw_start + text.len() + 2),
                            decoded: Some(start..start + 9),
                            occurrence: Some(index),
                        },
                        target,
                    ));
                }
            }
            assert_eq!(
                resolved
                    .tags()
                    .iter()
                    .map(|tag| (
                        tag.unit().as_str(),
                        tag.language(),
                        tag.occurrence().input(),
                        tag.occurrence().site().clone(),
                        tag.target()
                    ))
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(std::ptr::eq(resolved.book(), &raw const book));
            assert!(std::ptr::eq(resolved.corpus(), &raw const corpus));
            assert_eq!(book.graph().node(target).unwrap().id, target);
        }
    }
    assert_eq!(ftoo_manifest::Snapshot::capture(book.graph()), before);
    assert_eq!(NodeId::ALL.map(|id| book.graph().status(id)), statuses);
}
