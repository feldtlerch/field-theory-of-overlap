use ftoo_codegen::load_unit_files;
#[test]
fn committed_tag_free_units_use_the_shared_equation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
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
            ("sec:coefficients:value-context", "main")
        ]
    );
    let first = &corpus.units()[0];
    assert_eq!(
        first.texts().keys().map(String::as_str).collect::<Vec<_>>(),
        ["en"]
    );
    assert_eq!(
        first.texts()["en"].text(),
        "The first coefficient has the value @math{value}."
    );
    assert_eq!(
        first.math().keys().map(String::as_str).collect::<Vec<_>>(),
        ["value"]
    );
    assert_eq!(first.math()["value"].value, "c_1 = \\frac{1}{3}");
    assert_eq!(
        first.texts()["en"]
            .occurrences()
            .iter()
            .map(|o| (o.name(), o.site().decoded.clone(), o.site().occurrence))
            .collect::<Vec<_>>(),
        vec![("value", Some(36..48), Some(0))]
    );
    let second = &corpus.units()[1];
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
        "The value claim concerns the first coefficient."
    );
    assert_eq!(second.math().len(), 0);
    assert_eq!(second.texts()["en"].occurrences(), []);
    for unit in corpus.units() {
        for text in unit.texts().values() {
            assert!(!text.text().contains("@tag{"));
        }
    }
}
