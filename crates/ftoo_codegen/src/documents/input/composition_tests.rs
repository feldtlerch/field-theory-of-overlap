use super::*;

struct Fixture(std::path::PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn real_inputs() -> (
    std::path::PathBuf,
    UnitSource,
    Vec<UnitSource>,
    Vec<SkeletonDraft>,
) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let declaration = UnitSource {
        file: "units/documents.toml".into(),
        bytes: read(&root, "units/documents.toml").unwrap(),
    };
    let scopes = sources(&root, &discover(&root, "units/scopes", false).unwrap()).unwrap();
    let skeleton_sources =
        sources(&root, &discover(&root, "units/skeletons", false).unwrap()).unwrap();
    let drafts = super::skeletons(&skeleton_sources).unwrap();
    (root, declaration, scopes, drafts)
}

fn refuses(
    root: &Path,
    declaration: &UnitSource,
    scopes: &[UnitSource],
    drafts: &[SkeletonDraft],
    expected: &str,
) {
    let error = format!(
        "{:?}",
        composition(root, declaration, scopes, drafts).unwrap_err()
    );
    assert!(error.contains(expected), "{error}");
    assert!(error.contains("units/scopes/"), "{error}");
}

#[test]
fn scope_projections_refuse_changed_use_and_declaration() {
    let (root, declaration, scopes, mut drafts) = real_inputs();
    composition(&root, &declaration, &scopes, &drafts).unwrap();
    let paper = drafts
        .iter_mut()
        .find(|draft| draft.document.value == "paper-pilot")
        .unwrap();
    let extra = paper.sections[0].children[0].clone();
    paper.sections[0].children.push(extra);
    let refusal = format!(
        "{:?}",
        composition(&root, &declaration, &scopes, &drafts).unwrap_err()
    );
    assert!(
        refusal.contains("M079")
            && refusal.contains("CompositionMismatch")
            && refusal.contains("skeleton shape")
            && refusal.contains("units/skeletons/")
    );
    let (_, _, _, drafts) = real_inputs();
    let mut changed = declaration.clone();
    changed.bytes = String::from_utf8(changed.bytes)
        .unwrap()
        .replace(
            "targets = [\"fragment\", \"markdown\", \"html\"]",
            "targets = [\"fragment\"]",
        )
        .into_bytes();
    let refusal = format!(
        "{:?}",
        composition(&root, &changed, &scopes, &drafts).unwrap_err()
    );
    assert!(
        refusal.contains("M079")
            && refusal.contains("CompositionMismatch")
            && refusal.contains("declaration")
            && refusal.contains("units/documents.toml")
    );
}

#[test]
fn reordered_skeleton_child_refuses() {
    let (root, declaration, scopes, mut drafts) = real_inputs();
    let paper = drafts
        .iter_mut()
        .find(|draft| draft.document.value == "paper-pilot")
        .unwrap();
    paper.sections[0].children.swap(0, 1);
    refuses(&root, &declaration, &scopes, &drafts, "skeleton child 0");
}

#[test]
fn changed_skeleton_child_kind_refuses() {
    let (root, declaration, scopes, mut drafts) = real_inputs();
    let paper = drafts
        .iter_mut()
        .find(|draft| draft.document.value == "paper-pilot")
        .unwrap();
    let child = &mut paper.sections[0].children[0];
    *child = match child.clone() {
        SkeletonChild::Unit(value) => SkeletonChild::Paragraph(value),
        SkeletonChild::Paragraph(value)
        | SkeletonChild::CorpusSlot(value)
        | SkeletonChild::Econ(value)
        | SkeletonChild::Equation(value)
        | SkeletonChild::Quotation(value) => SkeletonChild::Unit(value),
        SkeletonChild::Section(_) => panic!("excerpt child is not a section"),
    };
    refuses(&root, &declaration, &scopes, &drafts, "skeleton child 0");
}

#[test]
fn second_record_for_one_source_refuses() {
    let (root, declaration, mut scopes, drafts) = real_inputs();
    let mut duplicate = scopes[0].clone();
    duplicate.file = "units/scopes/second.toml".into();
    scopes.push(duplicate);
    refuses(&root, &declaration, &scopes, &drafts, "duplicate source");
    let error = format!(
        "{:?}",
        composition(&root, &declaration, &scopes, &drafts).unwrap_err()
    );
    assert!(
        error.contains("record: \"units/scopes/second.toml\""),
        "{error}"
    );
}

#[test]
fn source_outside_closed_list_refuses() {
    let (root, declaration, mut scopes, drafts) = real_inputs();
    let original = String::from_utf8(scopes[0].bytes.clone()).unwrap();
    let source = scope(&scopes[0]).unwrap().source;
    scopes[0].bytes = original
        .replace(&source, "documents/paper/absent.tex")
        .into_bytes();
    refuses(&root, &declaration, &scopes, &drafts, "closed source list");
    let error = format!(
        "{:?}",
        composition(&root, &declaration, &scopes, &drafts).unwrap_err()
    );
    assert!(
        error.contains(&format!("record: {:?}", scopes[0].file)),
        "{error}"
    );
}

#[test]
fn missing_and_repeated_source_markers_refuse() {
    let (root, declaration, scopes, drafts) = real_inputs();
    let rows = composition(&root, &declaration, &scopes, &drafts).unwrap();
    let fixture =
        Fixture(std::env::temp_dir().join(format!("ftoo-composition-{}", std::process::id())));
    fs::create_dir_all(fixture.0.join("documents/paper-build")).unwrap();
    fs::copy(
        root.join("documents/paper-build/sources.tsv"),
        fixture.0.join("documents/paper-build/sources.tsv"),
    )
    .unwrap();
    for row in &rows {
        let target = fixture.0.join(&row.source);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(root.join(&row.source), target).unwrap();
    }
    let target = fixture.0.join(&rows[0].source);
    let original = fs::read_to_string(&target).unwrap();
    let marker = format!("% generated: {}", rows[0].unit);
    fs::write(&target, original.replace(&marker, "% removed marker")).unwrap();
    refuses(&fixture.0, &declaration, &scopes, &drafts, "M055");
    fs::write(&target, format!("{original}\n{marker}\n")).unwrap();
    refuses(&fixture.0, &declaration, &scopes, &drafts, "M055");
}
