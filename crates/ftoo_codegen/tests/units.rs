use ftoo_codegen::{
    UnitCheckSummary, UnitSource, check_projection, check_unit_files, load_unit_files,
    load_unit_sources,
};
use ftoo_manifest::{NodeId, load_projection, resolve_unit_tags};
use std::collections::BTreeMap;

fn declared_units(root: &std::path::Path) -> Vec<(String, String)> {
    let mut declared = Vec::new();
    for entry in std::fs::read_dir(root.join("units/statements")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "toml") {
            continue;
        }
        let source: toml_edit::DocumentMut =
            std::fs::read_to_string(path).unwrap().parse().unwrap();
        declared.push((
            source["unit"]["id"].as_str().unwrap().to_owned(),
            source["unit"]["document"].as_str().unwrap().to_owned(),
        ));
    }
    declared.sort();
    declared
}

fn declared_documents(root: &std::path::Path) -> BTreeMap<String, Vec<String>> {
    let source: toml_edit::DocumentMut = std::fs::read_to_string(root.join("units/documents.toml"))
        .unwrap()
        .parse()
        .unwrap();
    source["documents"]
        .as_table()
        .unwrap()
        .iter()
        .map(|(name, item)| {
            (
                name.to_owned(),
                item["required-languages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|language| language.as_str().unwrap().to_owned())
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn committed_tagged_units_resolve_through_the_checked_projection() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    check_projection(&root).unwrap();
    let loaded =
        load_projection(&std::fs::read(root.join("generated/manifest.json")).unwrap()).unwrap();
    let book = loaded.addressing().unwrap();
    let corpus = load_unit_files(&root).unwrap();
    let source_units = declared_units(&root);
    assert_eq!(corpus.documents(), &declared_documents(&root));
    assert_eq!(
        corpus
            .units()
            .iter()
            .map(|unit| (unit.label().as_str().to_owned(), unit.document().to_owned()))
            .collect::<Vec<_>>(),
        source_units
    );
    let checked = check_unit_files(&root, &book).unwrap();
    assert_eq!(
        checked,
        UnitCheckSummary {
            documents: corpus.documents().len(),
            units: source_units.len(),
            texts: corpus.units().iter().map(|unit| unit.texts().len()).sum(),
            tag_sites: corpus
                .units()
                .iter()
                .flat_map(|unit| unit.texts().values())
                .map(|text| text.tag_occurrences().len())
                .sum(),
        }
    );
    let first = corpus
        .units()
        .iter()
        .find(|u| u.label().as_str() == "sec:coefficients:first-value")
        .unwrap();
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
    let second = corpus
        .units()
        .iter()
        .find(|u| u.label().as_str() == "sec:coefficients:value-context")
        .unwrap();
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
    assert_tag_inventory(&corpus, &book, checked.tag_sites);
}

fn expected_tag_targets(root: &std::path::Path) -> BTreeMap<String, String> {
    let projection: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("generated/manifest.json")).unwrap())
            .unwrap();
    projection["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["tag"].as_str().unwrap().to_owned(),
                row["target"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[allow(clippy::too_many_lines)] // Independent source, declaration and resolved-target inventories are compared together.
fn assert_tag_inventory(
    corpus: &ftoo_manifest::LanguageCheckedUnits,
    book: &ftoo_manifest::AddressBook<'_>,
    checked_tag_sites: usize,
) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let permanent = load_unit_sources(
        UnitSource {
            file: "units/documents.toml".into(),
            bytes: std::fs::read(root.join("units/documents.toml")).unwrap(),
        },
        std::fs::read_dir(root.join("units/statements"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension().is_some_and(|ext| ext == "toml")
                    && path.file_name().unwrap() != "reading-shadow.toml"
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
    let resolved = resolve_unit_tags(&permanent, book).unwrap();
    let declared: Vec<_> = corpus
        .units()
        .iter()
        .filter(|unit| unit.label().as_str() != "sec:reading:shadow")
        .flat_map(|unit| {
            unit.texts().iter().flat_map(move |(language, text)| {
                text.tag_occurrences().iter().map(move |occurrence| {
                    (
                        unit.label().as_str(),
                        language.as_str(),
                        occurrence.input(),
                        occurrence.site().decoded.clone(),
                        occurrence.site().occurrence,
                    )
                })
            })
        })
        .collect();
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
            ))
            .collect::<Vec<_>>(),
        declared
    );
    for tag in resolved.tags() {
        assert!(resolved.book().graph().node(tag.target()).is_some());
    }
    let mut expected_targets = expected_tag_targets(&root);
    let paper_tags =
        std::fs::read_to_string(root.join("documents/paper-build/paper-tags.tsv")).unwrap();
    let paper_keys: std::collections::BTreeSet<_> = paper_tags
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 3);
            expected_targets.insert(fields[0].to_owned(), fields[1].to_owned());
            fields[0].to_owned()
        })
        .collect();
    let mut expected = Vec::new();
    for entry in std::fs::read_dir(root.join("units/statements")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "toml") {
            continue;
        }
        let source: toml_edit::DocumentMut =
            std::fs::read_to_string(path).unwrap().parse().unwrap();
        let unit = source["unit"]["id"].as_str().unwrap();
        for (language, value) in source["unit"]["texts"].as_table().unwrap() {
            let text = value.as_str().unwrap();
            for (at, _) in text.match_indices("@tag{") {
                let tail = &text[at + 5..];
                let input = &tail[..tail.find('}').unwrap()];
                if let Some(target) = expected_targets.get(input) {
                    expected.push((
                        unit.to_owned(),
                        language.to_owned(),
                        input.to_owned(),
                        target.clone(),
                    ));
                }
            }
        }
    }
    expected.sort();
    assert_eq!(expected.len(), checked_tag_sites);
    assert!(expected.iter().any(|(unit, language, input, target)| {
        unit == "sec:reading:shadow"
            && language == "en"
            && input == "src:nd"
            && expected_targets.get(input) == Some(target)
    }));
    let expected_permanent: Vec<_> = expected
        .into_iter()
        .filter(|(_, _, input, _)| !paper_keys.contains(input))
        .collect();
    let mut actual: Vec<_> = resolved
        .tags()
        .iter()
        .map(|tag| {
            (
                tag.unit().as_str().to_owned(),
                tag.language().to_owned(),
                tag.occurrence().input().to_owned(),
                tag.target().semantic_name().to_owned(),
            )
        })
        .collect();
    actual.sort();
    assert_eq!(actual, expected_permanent);
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
