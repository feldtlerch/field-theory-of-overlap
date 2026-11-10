use ftoo_codegen::{
    UnitSource, check_projection, check_unit_files, check_weld_files, load_unit_sources,
    prepare_notation,
};
use ftoo_manifest::{
    Callee, DefinitionId, ExprDraft, RenderTarget, addressing, load_projection, notation, seed,
};

#[test]
fn complete_typesetter_consumer_requires_prepared_support() -> Result<(), Box<dyn std::error::Error>>
{
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    check_projection(&root)?;
    let bytes = std::fs::read(root.join("generated/manifest.json"))?;
    let loaded = load_projection(&bytes)?;
    let book = loaded.addressing()?;
    let n = notation(&book)?;
    let graph = seed()?;
    let direct_book = addressing(&graph)?;
    let direct = notation(&direct_book)?;
    assert_eq!(
        n.definitions()
            .iter()
            .map(|d| (d.spec(), d.command(), d.status(), d.site()))
            .collect::<Vec<_>>(),
        direct
            .definitions()
            .iter()
            .map(|d| (d.spec(), d.command(), d.status(), d.site()))
            .collect::<Vec<_>>()
    );
    let snapshot = ftoo_manifest::Snapshot::capture(book.graph());
    let prepared = prepare_notation(&n)?;
    assert_eq!(prepared.sweep_rows().len(), 237);
    assert_eq!(prepared.typesetting_calls(), 474);
    let expression = prepared.check_expression(ExprDraft::Call {
        callee: Callee::Definition(DefinitionId::Dimension),
        arguments: vec![ExprDraft::Literal { value: "x".into() }],
    })?;
    let macro_form = expression.render(RenderTarget::Macro)?;
    let expanded = expression.render(RenderTarget::Unicode)?;
    assert_eq!(macro_form.source(), r"\FtooNDimension{x}");
    assert_eq!(expanded.source(), "[{x}]");
    assert!(std::ptr::eq(expanded.notation(), &raw const prepared));
    assert_eq!(macro_form.uses(), expanded.uses());
    assert_eq!(check_unit_files(&root, &book)?.units, 2);
    assert_eq!(check_weld_files(&root, &book)?.units, 2);
    assert_eq!(ftoo_manifest::Snapshot::capture(book.graph()), snapshot);
    assert_eq!(std::fs::read(root.join("generated/manifest.json"))?, bytes);
    Ok(())
}

#[test]
fn units_and_weld_keep_their_narrow_unknown_tex_guarantee() {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let documents = UnitSource {
        file: "units/documents.toml".into(),
        bytes: b"schema=1\n[documents.main]\nrequired-languages=['en']\n".to_vec(),
    };
    let statement = UnitSource {
        file: "units/statements/private.toml".into(),
        bytes: br"schema=1
[unit]
id='sec:private:notation'
document='main'
[unit.math]
value='\NotAKnownControlWord{x}'
[unit.texts]
en='The value is @math{value}.'
"
        .to_vec(),
    };
    let corpus = load_unit_sources(documents, vec![statement]).unwrap();
    let resolved = ftoo_manifest::resolve_unit_tags(&corpus, &book).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let markdown = ftoo_codegen::load_markdown_files(&root).unwrap();
    let integrated = ftoo_manifest::integrate_labels(&resolved, &markdown).unwrap();
    let welded = ftoo_manifest::weld_units(&integrated).unwrap();
    assert_eq!(welded.units().len(), 1);
    assert_eq!(
        corpus.units()[0].math()["value"].value,
        r"\NotAKnownControlWord{x}"
    );
}
