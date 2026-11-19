use super::*;
use ftoo_manifest::{QuestionFault, QuestionList};
use std::sync::atomic::AtomicUsize;

static NEXT: AtomicUsize = AtomicUsize::new(0);
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
/// A disposable repository holding exactly the inputs the store's check reads.
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ftoo-questions-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for directory in ["units", "documents", "registry"] {
            copy_tree(&root().join(directory), &path.join(directory));
        }
        for source in load_markdown_files(&root()).unwrap().sources() {
            let file = path.join(&source.file);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, &source.bytes).unwrap();
        }
        Self { root: path }
    }
    fn store(&self) -> String {
        fs::read_to_string(self.root.join(OPEN_QUESTION_STORE)).unwrap()
    }
    fn write_store(&self, text: &str) {
        fs::write(self.root.join(OPEN_QUESTION_STORE), text).unwrap();
    }
    fn check(&self) -> Result<OpenQuestionSummary, OpenQuestionError> {
        check_open_questions(&self.root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn committed_summary() -> OpenQuestionSummary {
    OpenQuestionSummary {
        records: 9,
        states: [9, 0, 0, 0],
        concerns: [6, 0, 1, 2],
    }
}

#[test]
fn committed_register_is_checked_by_the_ordinary_workspace_gate() {
    assert_eq!(check_open_questions(root()), Ok(committed_summary()));
    let register = fs::read_to_string(root().join(OPEN_QUESTION_REGISTER)).unwrap();
    assert!(
        register.contains("Records: 9. By state: open 9, settled 0, succeeded 0, withdrawn 0.\n")
    );
    assert!(register.contains("By concern: theory 6, model 0, documents 1, machinery 2.\n"));
}
#[test]
fn strict_reader_admits_arrays_of_inline_tables_unchanged() {
    let source = UnitSource {
        file: OPEN_QUESTION_STORE.into(),
        bytes: fs::read(root().join(OPEN_QUESTION_STORE)).unwrap(),
    };
    let drafts = load(&source).unwrap();
    assert_eq!(drafts.len(), 9);
    let tables = drafts
        .iter()
        .find(|d| d.label.value == "open:paper:fixed-data-tables")
        .unwrap();
    assert_eq!(tables.about.len(), 7);
    assert_eq!(tables.about[0].kind, ReferenceKind::Record);
    assert_eq!(tables.about[0].value.value, "dec:paper:authored-latex");
    assert_eq!(
        tables.about[1].value.site.path,
        [
            ftoo_manifest::PathPart::Key("question".into()),
            ftoo_manifest::PathPart::Index(4),
            ftoo_manifest::PathPart::Key("about".into()),
            ftoo_manifest::PathPart::Index(1),
            ftoo_manifest::PathPart::Key("file".into()),
        ]
    );
}

const ONE: &str = "schema = 1\n\n[[question]]\nlabel = \"open:test:a\"\ntitle = \"the a subject\"\nconcern = \"theory\"\nquestion = \"Is the a subject settled?\"\nabout = [{ record = \"dec:paper:authored-latex\" }]\nsettled-when = \"A derivation answers it.\"\nstate = \"open\"\n";
fn loaded(text: &str) -> Result<Vec<OpenQuestionDraft>, OpenQuestionError> {
    load(&UnitSource {
        file: OPEN_QUESTION_STORE.into(),
        bytes: text.as_bytes().to_vec(),
    })
}
fn boundary(text: &str) -> (RuleId, UnitFault) {
    match loaded(text) {
        Err(OpenQuestionError::Boundary(errors)) => (
            errors.diagnostics()[0].rule,
            errors.diagnostics()[0].fault.clone(),
        ),
        other => panic!("expected a boundary refusal, got {other:?}"),
    }
}
#[test]
fn m067_minimal_store_loads() {
    let drafts = loaded(ONE).unwrap();
    assert_eq!(drafts[0].state, StateDraft::Open);
    assert_eq!(drafts[0].concern, QuestionConcern::Theory);
}
#[test]
fn m067_unknown_field_refuses() {
    let (rule, fault) =
        boundary(&ONE.replace("state = \"open\"", "state = \"open\"\nowner = \"x\""));
    assert_eq!(rule, RuleId::M067);
    assert!(matches!(fault, UnitFault::UnknownField { name, .. } if name == "owner"));
}
#[test]
fn m067_closure_field_on_an_open_record_refuses() {
    let (rule, fault) = boundary(&ONE.replace(
        "state = \"open\"",
        "state = \"open\"\nanswer = \"It is settled.\"",
    ));
    assert_eq!(rule, RuleId::M067);
    assert!(matches!(fault, UnitFault::UnknownField { name, .. } if name == "answer"));
}
#[test]
fn m067_settled_record_without_its_answer_refuses() {
    let (rule, fault) = boundary(&ONE.replace(
        "state = \"open\"",
        "state = \"settled\"\nsettled-by = [{ rule = \"M001\" }]",
    ));
    assert_eq!(rule, RuleId::M067);
    assert_eq!(
        fault,
        UnitFault::MissingField {
            name: "answer".into()
        }
    );
}
#[test]
fn m067_withdrawn_record_without_its_reason_refuses() {
    let (rule, fault) = boundary(&ONE.replace("state = \"open\"", "state = \"withdrawn\""));
    assert_eq!(rule, RuleId::M067);
    assert_eq!(
        fault,
        UnitFault::MissingField {
            name: "withdrawn-because".into()
        }
    );
}
#[test]
fn m067_missing_state_and_missing_required_field_refuse() {
    let (rule, fault) = boundary(&ONE.replace("state = \"open\"\n", ""));
    assert_eq!(rule, RuleId::M067);
    assert_eq!(
        fault,
        UnitFault::MissingField {
            name: "state".into()
        }
    );
    let (rule, fault) = boundary(&ONE.replace("title = \"the a subject\"\n", ""));
    assert_eq!(rule, RuleId::M067);
    assert_eq!(
        fault,
        UnitFault::MissingField {
            name: "title".into()
        }
    );
}
#[test]
fn m067_unknown_tokens_refuse() {
    for (from, to, value) in [
        ("state = \"open\"", "state = \"reopened\"", "reopened"),
        ("concern = \"theory\"", "concern = \"system\"", "system"),
    ] {
        let (rule, fault) = boundary(&ONE.replace(from, to));
        assert_eq!(rule, RuleId::M067);
        assert!(matches!(fault, UnitFault::UnknownToken { value: v, .. } if v == value));
    }
}
#[test]
fn m067_reference_tables_have_exactly_one_known_kind() {
    let (rule, fault) = boundary(&ONE.replace("{ record =", "{ url ="));
    assert_eq!(rule, RuleId::M067);
    assert!(matches!(fault, UnitFault::UnknownField { name, .. } if name == "url"));
    let (rule, fault) = boundary(&ONE.replace(
        "{ record = \"dec:paper:authored-latex\" }",
        "{ record = \"dec:paper:authored-latex\", file = \"docs/glossary.md\" }",
    ));
    assert_eq!(rule, RuleId::M067);
    assert!(
        matches!(fault, UnitFault::UnknownField { name, expected } if name == "file" && expected == ["record"])
    );
    let (rule, fault) = boundary(&ONE.replace("{ record = \"dec:paper:authored-latex\" }", "{}"));
    assert_eq!(rule, RuleId::M067);
    assert!(matches!(fault, UnitFault::MissingField { .. }));
    let (rule, fault) = boundary(&ONE.replace(
        "{ record = \"dec:paper:authored-latex\" }",
        "\"dec:paper:authored-latex\"",
    ));
    assert_eq!(rule, RuleId::M067);
    assert!(matches!(fault, UnitFault::WrongType { .. }));
}
#[test]
fn m067_empty_lists_refuse() {
    for (from, to) in [
        ("state = \"open\"", "state = \"open\"\nblocks = []"),
        (
            "state = \"open\"",
            "state = \"settled\"\nsettled-by = []\nanswer = \"It is settled.\"",
        ),
        ("state = \"open\"", "state = \"open\"\nrests-on = []"),
    ] {
        let (rule, fault) = boundary(&ONE.replace(from, to));
        assert_eq!(rule, RuleId::M067, "{to}");
        assert!(matches!(fault, UnitFault::MissingField { .. }), "{to}");
    }
}
#[test]
fn byte_faults_keep_their_boundary_rules() {
    assert_eq!(
        boundary(&ONE.replace("schema = 1", "schema = 2")).0,
        RuleId::M003
    );
    assert_eq!(
        boundary(&ONE.replace("schema = 1", "schema = 1.0")).0,
        RuleId::M003
    );
    assert_eq!(
        boundary(&ONE.replace("state = \"open\"", "state = \"open\"\nstate = \"open\"")).0,
        RuleId::M002
    );
    assert_eq!(boundary(&format!("\u{feff}{ONE}")).0, RuleId::M001);
    assert_eq!(boundary("schema = 1\n").0, RuleId::M067);
}

#[test]
fn m071_generate_and_check_share_the_writer_and_restore_the_committed_bytes() {
    let fixture = Fixture::new();
    let committed = fs::read(fixture.root.join(OPEN_QUESTION_REGISTER)).unwrap();
    fs::remove_file(fixture.root.join(OPEN_QUESTION_REGISTER)).unwrap();
    assert!(matches!(
        fixture.check(),
        Err(OpenQuestionError::Register {
            rule: RuleId::M071,
            fault: RegisterFault::Missing { .. }
        })
    ));
    assert_eq!(
        generate_open_questions(&fixture.root),
        Ok(committed_summary())
    );
    assert_eq!(
        fs::read(fixture.root.join(OPEN_QUESTION_REGISTER)).unwrap(),
        committed
    );
    assert_eq!(fixture.check(), Ok(committed_summary()));
}
#[test]
fn m071_edited_register_refuses_at_the_first_differing_byte() {
    let fixture = Fixture::new();
    let path = fixture.root.join(OPEN_QUESTION_REGISTER);
    let committed = fs::read_to_string(&path).unwrap();
    let edited = committed.replacen("Records: 9.", "Records: 8.", 1);
    let offset = committed.find("Records: 9.").unwrap() + "Records: ".len();
    fs::write(&path, &edited).unwrap();
    assert_eq!(
        fixture.check(),
        Err(OpenQuestionError::Register {
            rule: RuleId::M071,
            fault: RegisterFault::NonCanonicalBytes {
                path,
                offset,
                expected_length: committed.len(),
                actual_length: edited.len(),
            }
        })
    );
}
#[test]
fn m071_register_text_that_would_harvest_a_citation_refuses_before_writing() {
    let fixture = Fixture::new();
    let store = fixture
        .store()
        .replacen(
            "title = \"the baseline converter\"",
            "title = \"the baseline converter (calibration\"",
            1,
        )
        .replacen(
            "settled-when = \"A calibration is posted",
            "settled-when = \"A calibration, now closed) is posted",
            1,
        );
    fixture.write_store(&store);
    let before = fs::read(fixture.root.join(OPEN_QUESTION_REGISTER)).unwrap();
    let error = generate_open_questions(&fixture.root).unwrap_err();
    assert!(
        matches!(
            &error,
            OpenQuestionError::Register {
                rule: RuleId::M071,
                fault: RegisterFault::AddressSyntax { citations, .. }
            } if citations.contains(&"open:converter:baseline-calibration".to_owned())
        ),
        "{error:?}"
    );
    assert_eq!(
        fs::read(fixture.root.join(OPEN_QUESTION_REGISTER)).unwrap(),
        before
    );
}
fn label_rule(error: &OpenQuestionError) -> RuleId {
    match error {
        OpenQuestionError::Labels(LabelFileError::Labels(errors)) => errors.diagnostics()[0].rule,
        other => panic!("expected a label refusal, got {other:?}"),
    }
}
#[test]
fn m068_markdown_mint_of_the_reserved_kind_refuses_the_store() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root.join("docs/extra.md"),
        "# Extra · `open:converter:baseline-calibration`\n",
    )
    .unwrap();
    assert_eq!(label_rule(&fixture.check().unwrap_err()), RuleId::M068);
}
#[test]
fn m068_paper_tag_target_and_mint_key_of_the_reserved_kind_refuse() {
    for (file, row, key) in [
        (
            crate::documents::registry::PAPER_TAGS,
            "x:open\topen:paper:tag\tnode\n",
            "open:paper:tag",
        ),
        (
            crate::documents::registry::PAPER_MINTS,
            "open:paper:key\tdocuments/paper/main.tex\n",
            "open:paper:key",
        ),
    ] {
        let fixture = Fixture::new();
        let path = fixture.root.join(file);
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str(row);
        fs::write(&path, text).unwrap();
        let error = fixture.check().unwrap_err();
        assert_eq!(label_rule(&error), RuleId::M068, "{file}");
        let OpenQuestionError::Labels(LabelFileError::Labels(errors)) = error else {
            unreachable!("the label rule identifies a label refusal")
        };
        assert!(matches!(
            &errors.diagnostics()[0].fault,
            ftoo_manifest::LabelFault::ReservedOpenKind { label, .. } if label == key
        ));
    }
}
fn admission(error: &OpenQuestionError) -> (RuleId, QuestionFault) {
    match error {
        OpenQuestionError::Admission(errors) => (
            errors.diagnostics()[0].rule,
            errors.diagnostics()[0].fault.clone(),
        ),
        other => panic!("expected an admission refusal, got {other:?}"),
    }
}
#[test]
fn m069_file_references_need_a_regular_file_without_symlinks() {
    let fixture = Fixture::new();
    std::os::unix::fs::symlink(
        "explainer/explainer.tex",
        fixture.root.join("documents/paper/linked.tex"),
    )
    .unwrap();
    for value in [
        "documents/paper/linked.tex",
        "documents/paper/explainer",
        "documents/paper/absent.tex",
    ] {
        fixture.write_store(&fixture.store().replacen(
            "documents/paper/explainer/explainer.tex",
            value,
            1,
        ));
        assert_eq!(
            admission(&fixture.check().unwrap_err()),
            (
                RuleId::M069,
                QuestionFault::UnresolvedReference {
                    list: QuestionList::About,
                    kind: ReferenceKind::File,
                    value: value.into()
                }
            ),
            "{value}"
        );
        fixture.write_store(&fixture.store().replacen(
            value,
            "documents/paper/explainer/explainer.tex",
            1,
        ));
    }
    assert!(probe_for_test(
        &fixture.root,
        "documents/paper/explainer/explainer.tex"
    ));
    assert_eq!(fixture.check(), Ok(committed_summary()));
}
#[test]
fn m069_paper_reference_resolves_against_tag_targets_and_mint_keys_only() {
    let fixture = Fixture::new();
    fixture.write_store(&fixture.store().replacen(
        "{ paper = \"app:ledger\" }",
        "{ paper = \"app:missing\" }",
        1,
    ));
    assert_eq!(
        admission(&fixture.check().unwrap_err()),
        (
            RuleId::M069,
            QuestionFault::UnresolvedReference {
                list: QuestionList::About,
                kind: ReferenceKind::Paper,
                value: "app:missing".into()
            }
        )
    );
}
#[test]
fn register_names_paper_objects_by_tag_beside_their_label() {
    let register = fs::read_to_string(root().join(OPEN_QUESTION_REGISTER)).unwrap();
    assert!(register.contains("paper object c:nu, `sem:converter:funded-gradient-to-rate`"));
    assert!(register.contains("paper label `app:ledger`"));
    assert!(register.contains("record `dec:paper:authored-latex`"));
    assert!(register.contains("- Rests on: open question `open:paper:fixed-data-tables`."));
    let rendered = render_for_test(
        &admit_open_questions(
            OPEN_QUESTION_STORE,
            loaded(ONE).unwrap(),
            &QuestionDomains {
                records: ["dec:paper:authored-latex".to_owned()].into(),
                ..QuestionDomains::default()
            },
        )
        .unwrap(),
        &BTreeMap::new(),
    );
    assert!(rendered.contains("| `open:test:a` | the a subject | theory | open |\n"));
    assert!(rendered.contains("| record `dec:paper:authored-latex` | `open:test:a` |\n"));
}
#[test]
fn m069_empty_about_in_the_store_reaches_admission_and_refuses() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let start = store
        .find("about = [\n  { paper = \"sem:claim:verdict-model-scope\" },\n]")
        .unwrap();
    let end = start + store[start..].find(']').unwrap() + 1;
    fixture.write_store(&format!("{}about = []{}", &store[..start], &store[end..]));
    assert_eq!(
        admission(&fixture.check().unwrap_err()),
        (
            RuleId::M069,
            QuestionFault::EmptyList {
                list: QuestionList::About
            }
        )
    );
}
#[test]
fn open_labels_enter_no_citation_domain_yet() {
    // A unit text citing a record is refused as an unresolved unit citation.
    let fixture = Fixture::new();
    let unit = fixture.root.join("units/statements/value-context.toml");
    let text = fs::read_to_string(&unit).unwrap();
    fs::write(
        &unit,
        text.replace(
            "`dec:weld:equal-address-sets`",
            "`open:converter:baseline-calibration`",
        ),
    )
    .unwrap();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let error = crate::check_weld_files(&fixture.root, &book).unwrap_err();
    let crate::WeldFileError::Weld(errors) = &error else {
        panic!("expected a weld refusal, got {error:?}");
    };
    assert!(
        errors.diagnostics().iter().any(|d| d.rule == RuleId::M045
            && matches!(&d.fault, ftoo_manifest::WeldFault::UnresolvedUnitCitation { label, .. }
                if label == "open:converter:baseline-calibration")),
        "{error:?}"
    );
    // A Markdown citation of a record is refused as unresolved in the Markdown domain.
    let fixture = Fixture::new();
    fs::write(
        fixture.root.join("docs/extra.md"),
        "A mention (`open:converter:baseline-calibration`).\n",
    )
    .unwrap();
    let error = load_markdown_files(&fixture.root).unwrap_err();
    let LabelFileError::Source(ftoo_manifest::LabelSourceError::Admission(errors)) = &error else {
        panic!("expected a Markdown refusal, got {error:?}");
    };
    assert!(matches!(
        &errors.diagnostics()[0].fault,
        LabelFault::UnresolvedMarkdownCitation { label, .. }
            if label == "open:converter:baseline-calibration"
    ));
}
/// The register's regeneration command, named in ADR 015:
/// `cargo test -p ftoo_codegen --locked -- --ignored regenerate_open_question_register`.
#[test]
#[ignore = "writes generated/open-questions.md; run after editing the store"]
fn regenerate_open_question_register() {
    generate_open_questions(root()).unwrap();
}
