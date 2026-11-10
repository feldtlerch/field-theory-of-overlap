use super::*;
use NodeId::{
    LangevinCoefficientOne as Q1, LangevinCoefficientOneValue as C1, LangevinCoefficientThree as Q3,
};
use ftoo_manifest::{
    AddressingDiagnostic, AddressingFault, AddressingLocation, AddressingStage, AdmissionStage,
    Diagnostic, DraftLocation, Edge, Guard, NodeId, OutOfSliceCause, ProjectionDiagnostic,
    ProjectionError, ProjectionFault, ProjectionValue, RelationKind, RuleId, SourceDifference,
    StoreState, TagId, WireError, WireFault, WireLocation,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn addressing_fault(
    rule: RuleId,
    stage: AddressingStage,
    location: AddressingLocation,
    tag: Option<TagId>,
    identity: NodeId,
    fault: AddressingFault,
) -> ProjectionCheckError {
    ProjectionCheckError::Projection(ProjectionError::Addressing(vec![AddressingDiagnostic {
        rule,
        stage,
        location,
        tag,
        identity: Some(identity),
        fault,
    }]))
}
#[test]
fn t08_history_ordinal_retains_original_index() {
    let f = Fixture::new();
    f.check(
        &f.clean.replacen("\"ordinal\":8", "\"ordinal\":9", 1),
        addressing_fault(
            RuleId::M032,
            AddressingStage::HistoryAdmission,
            AddressingLocation::History(7),
            Some(TagId::LangevinCoefficientThreeValue),
            NodeId::LangevinCoefficientThreeValue,
            AddressingFault::InvalidHistoryOrder {
                expected: 8,
                actual: 9,
            },
        ),
    );
}
#[test]
fn t09_unknown_descriptor_handle_is_not_runtime_unknown_tag() {
    let f = Fixture::new();
    f.check(
        &f.clean.replacen("\"tag\":\"c:1\"", "\"tag\":\"c:01\"", 1),
        shape(
            "$.tags[0].tag",
            WireFault::UnknownToken {
                expected: vec!["c:1".into(), "c:3".into(), "v:1".into(), "v:3".into()],
                actual: "c:01".into(),
            },
        ),
    );
}
#[test]
fn t10_known_handle_cannot_rebind_target() {
    let f = Fixture::new();
    f.check(
        &f.clean.replacen(
            "\"tag\":\"c:1\",\"target\":\"def:langevin:coefficient-one\"",
            "\"tag\":\"c:1\",\"target\":\"def:langevin:coefficient-three\"",
            1,
        ),
        assertion(
            "$.tags[0].target",
            Some(0),
            ProjectionFault::TagBindingMismatch {
                tag: TagId::LangevinCoefficientOne,
                expected: Q1,
                actual: Q3,
            },
        ),
    );
}
#[test]
fn t11_false_out_of_slice_store_cannot_replace_live() {
    let f = Fixture::new();
    let bytes = f
        .clean
        .replacen(
            "\"live\":[{\"tag\":\"c:1\"},{\"tag\":\"c:3\"}",
            "\"live\":[{\"tag\":\"c:1\"}",
            1,
        )
        .replacen(
            "\"out-of-slice\":[]",
            "\"out-of-slice\":[{\"tag\":\"c:3\",\"cause\":\"quantity-outside-support\"}]",
            1,
        );
    f.check(
        &bytes,
        addressing_fault(
            RuleId::M035,
            AddressingStage::StoreConstruction,
            AddressingLocation::Store(TagId::LangevinCoefficientThree),
            Some(TagId::LangevinCoefficientThree),
            Q3,
            AddressingFault::StoreStateMismatch {
                expected: StoreState::Live,
                actual: StoreState::OutOfSlice(OutOfSliceCause::QuantityOutsideSupport),
            },
        ),
    );
}
#[test]
fn t12_repeated_live_entry_retains_multiplicity() {
    let f = Fixture::new();
    f.check(
        &f.clean
            .replacen("\"live\":[", "\"live\":[{\"tag\":\"c:1\"},", 1),
        addressing_fault(
            RuleId::M035,
            AddressingStage::StoreConstruction,
            AddressingLocation::Store(TagId::LangevinCoefficientOne),
            Some(TagId::LangevinCoefficientOne),
            Q1,
            AddressingFault::StorePartitionMismatch {
                expected: 1,
                actual: 2,
            },
        ),
    );
}
#[test]
fn t13_history_reason_is_fixed_authority() {
    let f = Fixture::new();
    let old = "Retrospective registration of the M1 seed identity.";
    let new = "A different nonblank reason.";
    f.check(
        &f.clean.replacen(old, new, 1),
        assertion(
            "$.history[0].reason",
            Some(0),
            ProjectionFault::HistoryMismatch {
                expected: Some(Box::new(ProjectionValue::Text(old.into()))),
                actual: Some(Box::new(ProjectionValue::Text(new.into()))),
            },
        ),
    );
}
#[test]
fn t14_original_formulation_is_not_current_description() {
    let f = Fixture::new();
    let old = "The coefficient of k in the three-dimensional Langevin expansion.";
    let new = "A different nonblank historical formulation.";
    let bytes = f.clean.replacen(
        &format!("\"original-formulation\":\"{old}\""),
        &format!("\"original-formulation\":\"{new}\""),
        1,
    );
    f.check(
        &bytes,
        assertion(
            "$.history[0].change.original-formulation",
            Some(0),
            ProjectionFault::HistoryMismatch {
                expected: Some(Box::new(ProjectionValue::Text(old.into()))),
                actual: Some(Box::new(ProjectionValue::Text(new.into()))),
            },
        ),
    );
}

#[test]
fn io_faults_preserve_old_bytes_before_replacement_and_report_afterward() {
    use std::io::{Error, ErrorKind};
    for operation in [
        IoOperation::CreateDirectory,
        IoOperation::CreateTemporary,
        IoOperation::Write,
        IoOperation::Rename,
        IoOperation::Read,
    ] {
        let fixture = Fixture::new();
        fs::write(&fixture.path, b"old artifact").unwrap();
        let mut failed_path = None;
        let mut hook = |op, path: &std::path::Path, _| {
            if op == operation {
                failed_path = Some(path.to_path_buf());
                Err(Error::from(ErrorKind::PermissionDenied))
            } else {
                Ok(())
            }
        };
        let actual = crate::artifact::generate_with(&fixture.root, &mut hook);
        assert_eq!(
            actual,
            Err(ProjectionCheckError::Io {
                operation,
                path: failed_path.unwrap(),
                kind: ErrorKind::PermissionDenied,
                replaced: operation == IoOperation::Read
            })
        );
        let expected = if operation == IoOperation::Read {
            fixture.clean.as_bytes()
        } else {
            b"old artifact"
        };
        assert_eq!(fs::read(&fixture.path).unwrap(), expected);
        let names: Vec<_> = fs::read_dir(fixture.root.join("generated"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("manifest.json")]);
        generate_projection(&fixture.root).unwrap();
        check_projection(&fixture.root).unwrap();
    }
}
#[test]
fn check_missing_and_read_failure_never_write() {
    let fixture = Fixture::new();
    let mut operations = Vec::new();
    let error = crate::artifact::check_with(&fixture.root, &mut |operation, _, _| {
        operations.push(operation);
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
    });
    assert_eq!(
        error,
        Err(ProjectionCheckError::Io {
            operation: IoOperation::Read,
            path: fixture.path.clone(),
            kind: std::io::ErrorKind::PermissionDenied,
            replaced: false
        })
    );
    assert_eq!(operations, vec![IoOperation::Read]);
    assert_eq!(fs::read_to_string(&fixture.path).unwrap(), fixture.clean);
    fs::remove_file(&fixture.path).unwrap();
    assert_eq!(
        check_projection(&fixture.root),
        Err(ProjectionCheckError::MissingArtifact {
            path: fixture.path.clone()
        })
    );
    assert!(!fixture.path.exists());
    generate_projection(&fixture.root).unwrap();
    check_projection(&fixture.root).unwrap();
}
#[test]
fn post_replacement_verification_failure_never_claims_rollback() {
    let fixture = Fixture::new();
    let actual = crate::artifact::generate_with(&fixture.root, &mut |operation, path, replaced| {
        if operation == IoOperation::Read && replaced {
            fs::write(path, b"null")?;
        }
        Ok(())
    });
    assert_eq!(
        actual,
        Err(ProjectionCheckError::AfterReplacement(Box::new(shape(
            "$",
            WireFault::Null {
                expected: "object".into()
            }
        ))))
    );
    assert_eq!(fs::read(&fixture.path).unwrap(), b"null");
    generate_projection(&fixture.root).unwrap();
    check_projection(&fixture.root).unwrap();
}
#[test]
fn explicit_root_is_independent_of_process_working_directory() {
    let fixture = Fixture::new();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("tests::alternate_directory_child")
        .env("FTOO_PROJECTION_TEST_ROOT", &fixture.root)
        .current_dir(fixture.root.join("generated"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&fixture.path).unwrap(), fixture.clean);
    check_projection(&fixture.root).unwrap();
}
#[test]
fn alternate_directory_child() {
    if let Some(root) = std::env::var_os("FTOO_PROJECTION_TEST_ROOT") {
        check_projection(&root).unwrap();
        generate_projection(&root).unwrap();
        check_projection(&root).unwrap();
    }
}
#[test]
fn byte_gate_names_missing_terminal_lf_and_eof_exactly() {
    let fixture = Fixture::new();
    let bytes = fixture.clean.trim_end_matches('\n');
    let offset = bytes.len();
    let escape = |s: &str| s.as_bytes().escape_ascii().to_string();
    let diff = format!(
        "--- expected/{}\n+++ actual/{}\n-{}<EOF>\n+{}<EOF>\n@@ byte {offset} @@\n-{}<EOF>\n+{}<EOF>\n",
        fixture.path.display(),
        fixture.path.display(),
        escape(&fixture.clean),
        escape(bytes),
        escape(&fixture.clean[offset - 24..]),
        escape(&bytes[offset - 24..])
    );
    fixture.check(
        bytes,
        ProjectionCheckError::ByteGate {
            rule: RuleId::M038,
            fault: ByteGateFault::NonCanonicalBytes {
                path: fixture.path.clone(),
                offset,
                diff,
            },
        },
    );
}

#[test]
fn semantic_comparison_covers_coverage_maturity_and_binding_budget() {
    let f = Fixture::new();
    for (from, to, path, expected, actual) in [
        (
            "\"referents\":false",
            "\"referents\":true",
            "$.coverage.closed-world.referents",
            ProjectionValue::Boolean(false),
            ProjectionValue::Boolean(true),
        ),
        (
            "\"relations\":false",
            "\"relations\":true",
            "$.coverage.closed-world.relations",
            ProjectionValue::Boolean(false),
            ProjectionValue::Boolean(true),
        ),
        (
            "\"audit\":true",
            "\"audit\":false",
            "$.coverage.closed-world.audit",
            ProjectionValue::Boolean(true),
            ProjectionValue::Boolean(false),
        ),
        (
            "\"release-gate\":\"explicit-partial\"",
            "\"release-gate\":\"complete\"",
            "$.coverage.release-gate",
            ProjectionValue::Text("ExplicitPartial".into()),
            ProjectionValue::Text("Complete".into()),
        ),
        (
            "\"maturity\":\"proposed\"",
            "\"maturity\":\"retained\"",
            "$.nodes[claim:langevin:coefficient-one-value].status.maturity",
            ProjectionValue::Text("Proposed".into()),
            ProjectionValue::Text("Retained".into()),
        ),
        (
            "\"budget\":\"coefficient-one-v1\"",
            "\"budget\":\"coefficient-three-v1\"",
            "$.nodes[claim:langevin:coefficient-one-value].audit.budget",
            ProjectionValue::Text("CoefficientOneV1".into()),
            ProjectionValue::Text("CoefficientThreeV1".into()),
        ),
    ] {
        f.check(
            &f.clean.replacen(from, to, 1),
            source(vec![SourceDifference {
                position: path.starts_with("$.nodes[").then_some(0),
                path: path.into(),
                expected: Some(expected),
                actual: Some(actual),
            }]),
        );
    }
}
#[test]
fn semantic_comparison_covers_slice_and_derived_stores() {
    let f = Fixture::new();
    let bytes=f.clean.replacen("\"slice\":[\"claim:langevin:coefficient-one-value\",\"claim:langevin:coefficient-three-value\"]","\"slice\":[]",1)
        .replacen("\"live\":[{\"tag\":\"c:1\"},{\"tag\":\"c:3\"},{\"tag\":\"v:1\"},{\"tag\":\"v:3\"}]", "\"live\":[]", 1)
        .replacen("\"out-of-slice\":[]", "\"out-of-slice\":[{\"tag\":\"c:1\",\"cause\":\"quantity-outside-support\"},{\"tag\":\"c:3\",\"cause\":\"quantity-outside-support\"},{\"tag\":\"v:1\",\"cause\":\"claim-not-selected\"},{\"tag\":\"v:3\",\"cause\":\"claim-not-selected\"}]", 1);
    f.check(
        &bytes,
        source(vec![
            SourceDifference {
                position: None,
                path: "$.coverage.slice".into(),
                expected: Some(ProjectionValue::Identity(C1)),
                actual: None,
            },
            SourceDifference {
                position: None,
                path: "$.coverage.slice".into(),
                expected: Some(ProjectionValue::Identity(
                    NodeId::LangevinCoefficientThreeValue,
                )),
                actual: None,
            },
            SourceDifference {
                path: "$.stores[c:1]".into(),
                position: Some(0),
                expected: Some(ProjectionValue::Store(StoreState::Live)),
                actual: Some(ProjectionValue::Store(StoreState::OutOfSlice(
                    OutOfSliceCause::QuantityOutsideSupport,
                ))),
            },
            SourceDifference {
                path: "$.stores[c:3]".into(),
                position: Some(1),
                expected: Some(ProjectionValue::Store(StoreState::Live)),
                actual: Some(ProjectionValue::Store(StoreState::OutOfSlice(
                    OutOfSliceCause::QuantityOutsideSupport,
                ))),
            },
            SourceDifference {
                path: "$.stores[v:1]".into(),
                position: Some(2),
                expected: Some(ProjectionValue::Store(StoreState::Live)),
                actual: Some(ProjectionValue::Store(StoreState::OutOfSlice(
                    OutOfSliceCause::ClaimNotSelected,
                ))),
            },
            SourceDifference {
                path: "$.stores[v:3]".into(),
                position: Some(3),
                expected: Some(ProjectionValue::Store(StoreState::Live)),
                actual: Some(ProjectionValue::Store(StoreState::OutOfSlice(
                    OutOfSliceCause::ClaimNotSelected,
                ))),
            },
        ]),
    );
}
#[test]
fn semantic_multi_defect_collection_has_complete_stable_order() {
    let f = Fixture::new();
    let bytes = f
        .clean
        .replacen("The first coefficient is exactly 1/3.", "Changed.", 1)
        .replacen(
            "\"producer\":\"langevin-coefficient-one-value\"",
            "\"producer\":\"langevin-coefficient-three-value\"",
            1,
        );
    f.check(
        &bytes,
        source(vec![
            changed(
                "$.nodes[claim:langevin:coefficient-one-value].audit.producer",
                "LangevinCoefficientOneValue",
                "LangevinCoefficientThreeValue",
            ),
            changed(
                "$.nodes[claim:langevin:coefficient-one-value].description",
                "The first coefficient is exactly 1/3.",
                "Changed.",
            ),
        ]),
    );
}
struct Fixture {
    root: PathBuf,
    path: PathBuf,
    clean: String,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ftoo-projection-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        generate_projection(&root).unwrap();
        let path = root.join("generated/manifest.json");
        let clean = fs::read_to_string(&path).unwrap();
        Self { root, path, clean }
    }
    fn check(&self, bytes: &str, expected: ProjectionCheckError) {
        fs::write(&self.path, bytes).unwrap();
        assert_eq!(check_projection(&self.root), Err(expected));
        assert_eq!(fs::read_to_string(&self.path).unwrap(), bytes);
        generate_projection(&self.root).unwrap();
        assert_eq!(fs::read_to_string(&self.path).unwrap(), self.clean);
        check_projection(&self.root).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn wire(rule: RuleId, location: WireLocation, fault: WireFault) -> ProjectionCheckError {
    ProjectionCheckError::Projection(ProjectionError::Wire(WireError {
        rule,
        location,
        fault,
    }))
}
fn shape(path: &str, fault: WireFault) -> ProjectionCheckError {
    wire(RuleId::M004, WireLocation::Path(path.into()), fault)
}
fn assertion(path: &str, position: Option<usize>, fault: ProjectionFault) -> ProjectionCheckError {
    ProjectionCheckError::Projection(ProjectionError::Assertion(vec![ProjectionDiagnostic {
        rule: RuleId::M037,
        path: path.into(),
        position,
        fault,
    }]))
}
fn source(differences: Vec<SourceDifference>) -> ProjectionCheckError {
    assertion("$", None, ProjectionFault::SourceMismatch { differences })
}
fn missing(edge: Edge) -> SourceDifference {
    SourceDifference {
        position: None,
        path: "$.edges".into(),
        expected: Some(ProjectionValue::Edge(edge)),
        actual: None,
    }
}
fn edge(relation: RelationKind, source: NodeId, target: NodeId) -> Edge {
    Edge {
        relation,
        source,
        target,
        guard: Guard::Always,
    }
}
fn edge_text(edge: Edge) -> String {
    format!(
        "{{\"relation\":\"{}\",\"source\":\"{}\",\"target\":\"{}\",\"guard\":{{\"kind\":\"always\"}}}}",
        edge.relation.name(),
        edge.source.semantic_name(),
        edge.target.semantic_name()
    )
}
fn remove_edge(bytes: &str, edge: Edge) -> String {
    let text = edge_text(edge);
    if bytes.contains(&format!("{text},")) {
        bytes.replacen(&format!("{text},"), "", 1)
    } else {
        bytes.replacen(&format!(",{text}"), "", 1)
    }
}
fn changed(path: &str, expected: &str, actual: &str) -> SourceDifference {
    SourceDifference {
        position: if path.contains("claim:langevin:coefficient-one-value") {
            Some(0)
        } else if path.contains("def:langevin:coefficient-one") {
            Some(2)
        } else {
            None
        },
        path: path.into(),
        expected: Some(ProjectionValue::Text(expected.into())),
        actual: Some(ProjectionValue::Text(actual.into())),
    }
}

#[test]
fn t01_deleted_quantity_dependency() {
    let f = Fixture::new();
    let e = edge(RelationKind::DependsOn, Q3, Q1);
    f.check(&remove_edge(&f.clean, e), source(vec![missing(e)]));
}
#[test]
fn t02_each_missing_compatibility_orientation() {
    let f = Fixture::new();
    for removed in [
        edge(RelationKind::CompatibleWith, Q1, Q3),
        edge(RelationKind::CompatibleWith, Q3, Q1),
    ] {
        let remaining = removed.reverse();
        let bytes = remove_edge(&f.clean, removed);
        fs::write(&f.path, &bytes).unwrap();
        let error = check_projection(&f.root).unwrap_err();
        let ProjectionCheckError::Projection(ProjectionError::Graph(errors)) = error else {
            panic!("expected graph refusal: {error:?}");
        };
        assert_eq!(
            errors.diagnostics(),
            &[Diagnostic {
                rule: RuleId::M021,
                stage: AdmissionStage::GraphAdmission,
                ids: vec![remaining.source, remaining.target],
                edge: Some(remaining),
                location: DraftLocation::Edge(0),
                explanation: format!(
                    "missing reverse compatible-with: {} -> {} with Always",
                    removed.source.semantic_name(),
                    removed.target.semantic_name()
                )
            }]
        );
        generate_projection(&f.root).unwrap();
        check_projection(&f.root).unwrap();
    }
}
#[test]
fn t03_both_compatibility_orientations_missing() {
    let f = Fixture::new();
    let a = edge(RelationKind::CompatibleWith, Q1, Q3);
    let b = a.reverse();
    f.check(
        &remove_edge(&remove_edge(&f.clean, a), b),
        source(vec![missing(a), missing(b)]),
    );
}
#[test]
fn t04_known_false_evidence_assertion() {
    let f = Fixture::new();
    f.check(
        &f.clean.replacen(
            "\"evidence\":\"deferred\"",
            "\"evidence\":\"not-a-claim\"",
            1,
        ),
        wire(
            RuleId::M012,
            WireLocation::Path("$.nodes[0].status.evidence".into()),
            WireFault::StatusMismatch {
                node: C1,
                expected: "deferred".into(),
                actual: "not-a-claim".into(),
            },
        ),
    );
}
#[test]
fn t05_noncanonical_space_has_exact_offset_and_diff() {
    let f = Fixture::new();
    let bytes = f.clean.replacen('{', "{ ", 1);
    let escaped = |s: &str| s.as_bytes().escape_ascii().to_string();
    let diff = format!(
        "--- expected/{}\n+++ actual/{}\n-{}<EOF>\n+{}<EOF>\n@@ byte 1 @@\n-{}\n+{}\n",
        f.path.display(),
        f.path.display(),
        escaped(&f.clean),
        escaped(&bytes),
        escaped(&f.clean[..26]),
        escaped(&bytes[..26])
    );
    f.check(
        &bytes,
        ProjectionCheckError::ByteGate {
            rule: RuleId::M038,
            fault: ByteGateFault::NonCanonicalBytes {
                path: f.path.clone(),
                offset: 1,
                diff,
            },
        },
    );
}
#[test]
fn t06_budget_work_assertion() {
    let f = Fixture::new();
    f.check(
        &f.clean.replacen("\"work\":8000000", "\"work\":8000001", 1),
        assertion(
            "$.budgets[0].limits.work",
            Some(0),
            ProjectionFault::BudgetLimitsMismatch {
                expected: 8_000_000,
                actual: 8_000_001,
            },
        ),
    );
}
#[test]
fn t07_missing_budget_limit() {
    let f = Fixture::new();
    f.check(
        &f.clean.replacen("\"local-branches\":64,", "", 1),
        shape(
            "$.budgets[0].limits",
            WireFault::MissingField {
                name: "local-branches".into(),
            },
        ),
    );
}
#[test]
fn t15_binding_source_mismatch_precedes_registry() {
    let f = Fixture::new();
    let bytes = f.clean.replacen(
        "\"producer\":\"langevin-coefficient-one-value\"",
        "\"producer\":\"langevin-coefficient-three-value\"",
        1,
    );
    f.check(
        &bytes,
        source(vec![changed(
            "$.nodes[claim:langevin:coefficient-one-value].audit.producer",
            "LangevinCoefficientOneValue",
            "LangevinCoefficientThreeValue",
        )]),
    );
}
#[test]
fn t16_producer_revision_assertion() {
    let f = Fixture::new();
    f.check(
        &f.clean.replacen("\"revision\":1", "\"revision\":2", 1),
        assertion(
            "$.producers[0].revision",
            Some(0),
            ProjectionFault::ContractMismatch {
                expected: Box::new(ProjectionValue::Integer(1)),
                actual: Box::new(ProjectionValue::Integer(2)),
            },
        ),
    );
}
#[test]
fn t17_adapter_implementation_assertion() {
    let f = Fixture::new();
    f.check(
        &f.clean.replacen(
            "\"adapter-implemented\":false",
            "\"adapter-implemented\":true",
            1,
        ),
        assertion(
            "$.capabilities[0].adapter-implemented",
            Some(0),
            ProjectionFault::ContractMismatch {
                expected: Box::new(ProjectionValue::Boolean(false)),
                actual: Box::new(ProjectionValue::Boolean(true)),
            },
        ),
    );
}
#[test]
fn t18_engine_pin_assertion() {
    let f = Fixture::new();
    let old = "a5cb0ab0d1763e8ed620f7c765fdd8d3b26b65ad";
    let new = "b5cb0ab0d1763e8ed620f7c765fdd8d3b26b65ad";
    f.check(
        &f.clean.replacen(old, new, 1),
        assertion(
            "$.engine-pin",
            None,
            ProjectionFault::PinMismatch {
                expected: old.into(),
                actual: new.into(),
            },
        ),
    );
}
#[test]
fn t19_escaped_equivalent_node_key() {
    let f = Fixture::new();
    let prefix = "\"id\":\"claim:langevin:coefficient-one-value\"";
    let bytes = f
        .clean
        .replacen(prefix, &format!("{prefix},\"\\u0069d\":0"), 1);
    let column = bytes.find("\\u0069d\":0").unwrap() + 10;
    f.check(
        &bytes,
        wire(
            RuleId::M002,
            WireLocation::LineColumn {
                line: 1,
                column,
                path: "$.nodes[0]".into(),
            },
            WireFault::DuplicateKey { key: "id".into() },
        ),
    );
}
#[test]
fn t20_duplicate_in_unknown_object() {
    let f = Fixture::new();
    let bytes = f.clean.replacen('{', "{\"unknown\":{\"x\":0,\"x\":1},", 1);
    f.check(
        &bytes,
        wire(
            RuleId::M002,
            WireLocation::LineColumn {
                line: 1,
                column: 23,
                path: "$.unknown".into(),
            },
            WireFault::DuplicateKey { key: "x".into() },
        ),
    );
}
#[test]
fn t21_noninteger_in_unknown_field() {
    let f = Fixture::new();
    let bytes = f.clean.replacen('{', "{\"unknown\":1e0,", 1);
    f.check(
        &bytes,
        wire(
            RuleId::M003,
            WireLocation::LineColumn {
                line: 1,
                column: 12,
                path: "$.unknown".into(),
            },
            WireFault::NonIntegerNumber {
                token: "1e0".into(),
            },
        ),
    );
}
#[test]
fn t22_null_guard() {
    let f = Fixture::new();
    f.check(
        &f.clean
            .replacen("\"guard\":{\"kind\":\"always\"}", "\"guard\":null", 1),
        shape(
            "$.edges[0].guard",
            WireFault::Null {
                expected: "object".into(),
            },
        ),
    );
}
#[test]
fn t23_certified_is_not_projection_evidence() {
    let f = Fixture::new();
    f.check(
        &f.clean
            .replacen("\"evidence\":\"deferred\"", "\"evidence\":\"certified\"", 1),
        shape(
            "$.nodes[0].status.evidence",
            WireFault::UnknownToken {
                expected: vec!["not-a-claim".into(), "deferred".into()],
                actual: "certified".into(),
            },
        ),
    );
}
#[test]
fn t24_description_source_mismatch() {
    let f = Fixture::new();
    let old = "The coefficient of k in the three-dimensional Langevin expansion.";
    let new = "A different nonblank description.";
    f.check(
        &f.clean.replacen(old, new, 1),
        source(vec![changed(
            "$.nodes[def:langevin:coefficient-one].description",
            old,
            new,
        )]),
    );
}
#[test]
fn generate_and_check_actual_fixture() {
    let fixture = Fixture::new();
    generate_projection(&fixture.root).unwrap();
    check_projection(&fixture.root).unwrap();
    assert_eq!(fs::read_to_string(&fixture.path).unwrap(), fixture.clean);
}
