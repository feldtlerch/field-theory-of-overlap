use super::*;
use ftoo_manifest::{NodeId, ReaderIdentityPosition};

#[test]
fn reader_diagnostics_sort_families_before_owners_and_indices() {
    use ftoo_manifest::{AddressTarget, DefinitionId, ReaderIdentityReason};
    use std::fmt::Write;
    let diagnostic = |position, unit: Option<&str>| AssemblyDiagnostic {
        rule: RuleId::M055,
        stage: AssemblyStage::Emission,
        document: Some("main".into()),
        unit: unit.map(String::from),
        language: Some("en".into()),
        target: None,
        site: AssemblySite::ReaderIdentity {
            position,
            origins: vec![],
        },
        fault: AssemblyFault::ReaderIdentity {
            target: AddressTarget::Theory(NodeId::LangevinCoefficientOne),
            reason: ReaderIdentityReason::MissingTag,
            candidates: vec![],
        },
    };
    let coverage = diagnostic(
        ReaderIdentityPosition::Coverage {
            index: 0,
            object: NodeId::LangevinCoefficientOne,
        },
        None,
    );
    let pending = diagnostic(
        ReaderIdentityPosition::Pending {
            unit: "sec:z:pending".into(),
            definition: DefinitionId::FirstCoefficient,
            reason: 0,
        },
        Some("sec:z:pending"),
    );
    let first = diagnostic(
        ReaderIdentityPosition::Citation {
            unit: "sec:a:citation".into(),
            span: 0,
            target: None,
        },
        Some("sec:a:citation"),
    );
    let second = diagnostic(
        ReaderIdentityPosition::Citation {
            unit: "sec:a:citation".into(),
            span: 1,
            target: None,
        },
        Some("sec:a:citation"),
    );
    let mut no_live = first.clone();
    if let AssemblyFault::ReaderIdentity { reason, .. } = &mut no_live.fault {
        *reason = ReaderIdentityReason::NoLiveTag;
    }
    let mut documentary = first.clone();
    if let AssemblyFault::ReaderIdentity { target, .. } = &mut documentary.fault {
        *target = AddressTarget::Documentary("dec:fixture:source".into());
    }
    let expected = vec![
        coverage.clone(),
        pending.clone(),
        first.clone(),
        no_live.clone(),
        documentary.clone(),
        second.clone(),
    ];
    let actual = AssemblyErrors::from_diagnostics(vec![
        second,
        documentary,
        no_live,
        first,
        pending,
        coverage,
    ])
    .unwrap();
    assert_eq!(actual.diagnostics(), expected);
    let mut display = String::new();
    for diagnostic in expected {
        writeln!(display, "{diagnostic}").unwrap();
    }
    assert_eq!(actual.to_string(), display);
}

fn plan(book: &ftoo_manifest::AddressBook<'_>) -> DocumentPlan {
    DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: Coverage::build(book, "Coverage.", "fixture", "en").unwrap(),
        blocks: vec![Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: "sec:fixture:reader".into(),
            depth: None,
            site: super::super::input::site("fixture.toml"),
            runs: vec![Inline::Text("Body.".into())],
        }],
    }
}
fn render(plan: &DocumentPlan, target: DocumentTarget) -> String {
    if target == DocumentTarget::Latex {
        super::super::latex::render(plan)
    } else {
        super::super::markdown::render(plan)
    }
}
fn identity(target: DocumentTarget, index: usize, object: NodeId, handle: &str) -> EmissionRecord {
    EmissionRecord {
        kind: EmissionKind::ReaderIdentity {
            position: Some(ReaderIdentityPosition::Coverage { index, object }),
            object: Some(object),
            handle: handle.into(),
            token: format!("g-slice-{index}"),
        },
        unit: String::new(),
        language: "en".into(),
        target,
        occurrence: None,
        value: object.semantic_name().into(),
        site: coverage_site(Some(index)),
    }
}
#[test]
#[allow(clippy::too_many_lines)] // Keep complete byte and mutation oracles together.
fn reader_coverage_seed_empty_bytes_sizes_and_complete_mutation_faults() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let plan = plan(&book);
    let path = std::env::temp_dir().join(format!("reader-coverage-{}.txt", std::process::id()));
    for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
        let source = render(&plan, target);
        let slice = if target == DocumentTarget::Latex {
            "Declared slice: \\FtdocTag{v:1}{g-slice-0}% label claim:langevin:coefficient-one-value\n, \\FtdocTag{v:3}{g-slice-1}% label claim:langevin:coefficient-three-value\n."
        } else {
            "Declared slice: v:1<!-- label claim:langevin:coefficient-one-value -->, v:3<!-- label claim:langevin:coefficient-three-value -->."
        };
        let complete = if target == DocumentTarget::Latex {
            format!(
                "{}Coverage.\n\n{slice}\n\n% unit sec:fixture:reader document fixture language en\nBody.\n\n\\end{{document}}\n",
                coverage_prefix(&plan, target)
            )
        } else {
            format!(
                "<!-- generated document fixture language en -->\nCoverage.\n{slice}\n\n<!-- unit sec:fixture:reader document fixture language en -->\nBody.\n"
            )
        };
        assert_eq!(source, complete);
        assert_eq!(
            source.len(),
            if target == DocumentTarget::Latex {
                super::super::latex::size(&plan)
            } else {
                super::super::markdown::size(&plan)
            }
        );
        assert_eq!(check_emission(&plan, &source, target), Ok(()));
        let expected = identity(target, 0, NodeId::LangevinCoefficientOneValue, "v:1");
        // A changed bound-label comment is not repaired from the handle.
        let mut actual = expected.clone();
        actual.value = "claim:langevin:coefficient-three-value".into();
        if let EmissionKind::ReaderIdentity { object, .. } = &mut actual.kind {
            *object = Some(NodeId::LangevinCoefficientThreeValue);
        }
        let mut cases = vec![(
            source.replacen(
                "claim:langevin:coefficient-one-value",
                "claim:langevin:coefficient-three-value",
                1,
            ),
            actual,
        )];
        if target == DocumentTarget::Latex {
            let mut actual = expected.clone();
            if let EmissionKind::ReaderIdentity { token, .. } = &mut actual.kind {
                *token = "g-slice-9".into();
            }
            cases.push((source.replacen("g-slice-0", "g-slice-9", 1), actual));
        }
        for (mutated, actual) in cases {
            std::fs::write(&path, &mutated).unwrap();
            let reread = std::fs::read_to_string(&path).unwrap();
            let start = coverage_prefix(&plan, target).len();
            let end = reread.find("\n\n").unwrap();
            let end = if target == DocumentTarget::Latex {
                reread[end + 2..].find("\n\n").unwrap() + end + 2
            } else {
                end
            };
            let fault = emission_failure(
                &plan,
                target,
                start..end,
                AssemblyFault::EmissionMismatch {
                    missing: vec![expected.clone()],
                    unexpected: vec![actual],
                    changed: vec![],
                },
            );
            let actual = check_emission(&plan, &reread, target).unwrap_err();
            assert_eq!(actual, fault);
            assert_eq!(actual.to_string(), fault.to_string());
            std::fs::write(&path, &source).unwrap();
            let restored = std::fs::read_to_string(&path).unwrap();
            assert_eq!(restored, source);
            assert_eq!(check_emission(&plan, &restored, target), Ok(()));
        }
    }
    std::fs::remove_file(path).unwrap();
    let mut draft = ftoo_manifest::seed_draft();
    draft.coverage.slice.clear();
    let graph = ftoo_manifest::validate(draft).unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let plan = self::plan(&book);
    assert_eq!(
        render(&plan, DocumentTarget::Markdown),
        "<!-- generated document fixture language en -->\nCoverage.\nDeclared slice: &#40;empty&#41;.\n\n<!-- unit sec:fixture:reader document fixture language en -->\nBody.\n"
    );
    assert_eq!(
        render(&plan, DocumentTarget::Latex),
        format!(
            "{}\\FtdocContext{{fixture}}{{en}}\n\\begin{{document}}\nCoverage.\n\nDeclared slice: (empty).\n\n% unit sec:fixture:reader document fixture language en\nBody.\n\n\\end{{document}}\n",
            super::super::latex::PREAMBLE
        )
    );
    for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
        assert_eq!(
            check_emission(&plan, &render(&plan, target), target),
            Ok(())
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // Complete ordered mutation and restoration evidence stays together.
fn reader_coverage_order_multiplicity_and_handle_faults_are_complete() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let plan = plan(&book);
    let path = std::env::temp_dir().join(format!("reader-order-{}.txt", std::process::id()));
    for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
        let source = render(&plan, target);
        let first = identity(target, 0, NodeId::LangevinCoefficientOneValue, "v:1");
        let second = identity(target, 1, NodeId::LangevinCoefficientThreeValue, "v:3");
        let first_bytes = if target == DocumentTarget::Latex {
            "\\FtdocTag{v:1}{g-slice-0}% label claim:langevin:coefficient-one-value\n"
        } else {
            "v:1<!-- label claim:langevin:coefficient-one-value -->"
        };
        let second_bytes = if target == DocumentTarget::Latex {
            "\\FtdocTag{v:3}{g-slice-1}% label claim:langevin:coefficient-three-value\n"
        } else {
            "v:3<!-- label claim:langevin:coefficient-three-value -->"
        };
        let expected_slice = EmissionRecord {
            kind: EmissionKind::CoverageSlice,
            unit: String::new(),
            language: "en".into(),
            target,
            occurrence: None,
            value: "Declared slice: @tag{v:1}, @tag{v:3}.".into(),
            site: coverage_site(None),
        };
        for case in 0..4 {
            let mut one = first.clone();
            let mut two = second.clone();
            let (new_slice, fragment, missing, unexpected) = match case {
                0 => {
                    if let EmissionKind::ReaderIdentity { handle, .. } = &mut one.kind {
                        *handle = "v:3".into();
                    }
                    (
                        "Declared slice: @tag{v:3}, @tag{v:3}.",
                        format!("{}, {second_bytes}", first_bytes.replacen("v:1", "v:3", 1)),
                        vec![first.clone()],
                        vec![one],
                    )
                }
                1 => {
                    if let EmissionKind::ReaderIdentity {
                        position, token, ..
                    } = &mut two.kind
                    {
                        *position = Some(ReaderIdentityPosition::Coverage {
                            index: 0,
                            object: NodeId::LangevinCoefficientOneValue,
                        });
                        if target == DocumentTarget::Markdown {
                            *token = "g-slice-0".into();
                        }
                    }
                    two.site = coverage_site(Some(0));
                    (
                        "Declared slice: @tag{v:3}.",
                        second_bytes.into(),
                        vec![first.clone(), second.clone()],
                        vec![two],
                    )
                }
                2 => {
                    if let EmissionKind::ReaderIdentity {
                        position, token, ..
                    } = &mut one.kind
                    {
                        *position = Some(ReaderIdentityPosition::Coverage {
                            index: 1,
                            object: NodeId::LangevinCoefficientThreeValue,
                        });
                        if target == DocumentTarget::Markdown {
                            *token = "g-slice-1".into();
                        }
                    }
                    one.site = coverage_site(Some(1));
                    if let EmissionKind::ReaderIdentity {
                        position, token, ..
                    } = &mut two.kind
                    {
                        *position = None;
                        if target == DocumentTarget::Markdown {
                            *token = "g-slice-2".into();
                        }
                    }
                    two.site = coverage_site(Some(2));
                    (
                        "Declared slice: @tag{v:1}, @tag{v:1}, @tag{v:3}.",
                        format!("{first_bytes}, {first_bytes}, {second_bytes}"),
                        vec![second.clone()],
                        vec![one, two],
                    )
                }
                _ => {
                    if let EmissionKind::ReaderIdentity {
                        position, token, ..
                    } = &mut one.kind
                    {
                        *position = Some(ReaderIdentityPosition::Coverage {
                            index: 1,
                            object: NodeId::LangevinCoefficientThreeValue,
                        });
                        if target == DocumentTarget::Markdown {
                            *token = "g-slice-1".into();
                        }
                    }
                    one.site = coverage_site(Some(1));
                    if let EmissionKind::ReaderIdentity {
                        position, token, ..
                    } = &mut two.kind
                    {
                        *position = Some(ReaderIdentityPosition::Coverage {
                            index: 0,
                            object: NodeId::LangevinCoefficientOneValue,
                        });
                        if target == DocumentTarget::Markdown {
                            *token = "g-slice-0".into();
                        }
                    }
                    two.site = coverage_site(Some(0));
                    (
                        "Declared slice: @tag{v:3}, @tag{v:1}.",
                        format!("{second_bytes}, {first_bytes}"),
                        vec![first.clone(), second.clone()],
                        vec![two, one],
                    )
                }
            };
            let mutated = source.replace(&format!("{first_bytes}, {second_bytes}"), &fragment);
            std::fs::write(&path, &mutated).unwrap();
            let reread = std::fs::read_to_string(&path).unwrap();
            let start = coverage_prefix(&plan, target).len();
            let end = reread.find("\n\n").unwrap();
            let end = if target == DocumentTarget::Latex {
                reread[end + 2..].find("\n\n").unwrap() + end + 2
            } else {
                end
            };
            let mut actual_slice = expected_slice.clone();
            actual_slice.value = new_slice.into();
            let expected = emission_failure(
                &plan,
                target,
                start..end,
                AssemblyFault::EmissionMismatch {
                    missing,
                    unexpected,
                    changed: vec![super::super::diagnostics::EmissionChange {
                        expected: expected_slice.clone(),
                        actual: actual_slice,
                    }],
                },
            );
            let actual = check_emission(&plan, &reread, target).unwrap_err();
            assert_eq!(actual, expected, "case {case}");
            assert_eq!(actual.to_string(), expected.to_string());
            std::fs::write(&path, &source).unwrap();
            assert_eq!(
                check_emission(&plan, &std::fs::read_to_string(&path).unwrap(), target),
                Ok(())
            );
        }
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn reader_fault_mapping_retains_context_candidates_and_projection_origin() {
    for reason in [
        ftoo_manifest::ReaderIdentityReason::MissingTag,
        ftoo_manifest::ReaderIdentityReason::NoLiveTag,
        ftoo_manifest::ReaderIdentityReason::AmbiguousLiveTags,
    ] {
        let candidates = if reason == ftoo_manifest::ReaderIdentityReason::MissingTag {
            vec![]
        } else {
            vec![ftoo_manifest::ReaderTagCandidate {
                handle: "v:1".into(),
                outcome: Err(ftoo_manifest::TagResolutionFault::OutOfSliceTag {
                    input: "v:1".into(),
                    tag: ftoo_manifest::TagId::LangevinCoefficientOneValue,
                    target: NodeId::LangevinCoefficientOneValue,
                    cause: ftoo_manifest::OutOfSliceCause::ClaimNotSelected,
                }),
            }]
        };
        let origins = vec![SourceOrigin {
            unit: None,
            language: Some("en".into()),
            target: None,
            site: coverage_site(Some(0)),
        }];
        let position = ReaderIdentityPosition::Coverage {
            index: 0,
            object: NodeId::LangevinCoefficientOneValue,
        };
        let expected = DocumentError::Assembly(
            AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                rule: RuleId::M055,
                stage: AssemblyStage::Emission,
                document: Some("fixture".into()),
                unit: None,
                language: Some("en".into()),
                target: None,
                site: AssemblySite::ReaderIdentity {
                    position: position.clone(),
                    origins: origins.clone(),
                },
                fault: AssemblyFault::ReaderIdentity {
                    target: ftoo_manifest::AddressTarget::Theory(
                        NodeId::LangevinCoefficientOneValue,
                    ),
                    reason,
                    candidates: candidates.clone(),
                },
            }])
            .unwrap(),
        );
        let actual = reader_failure(
            ftoo_manifest::ReaderIdentityFault {
                target: ftoo_manifest::AddressTarget::Theory(NodeId::LangevinCoefficientOneValue),
                reason,
                candidates,
            },
            "fixture",
            "en",
            None,
            None,
            position,
            origins,
        );
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), expected.to_string());
    }
}

#[test]
#[allow(clippy::too_many_lines)] // One bracket carries coverage and both pending-object controls.
fn support_required_reader_coverage_runtime_has_empty_unit_and_exact_origin()
-> Result<(), Box<dyn std::error::Error>> {
    use super::super::{NativeSupport, native, native_support, runtime};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let support = NativeSupport::from_environment(&root).map_err(|error| format!("{error:?}"))?;
    let lock = native_support::lock(&root).map_err(|error| format!("{error:?}"))?;
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let plan = plan(&book);
    let addressing = runtime::render(&book, std::slice::from_ref(&plan))?;
    let source = render(&plan, DocumentTarget::Latex);
    let mutated = source.replacen("{v:1}{g-slice-0}", "{c:01}{g-slice-0}", 1);
    let file = std::env::temp_dir().join(format!("reader-native-{}.tex", std::process::id()));
    *support.evidence.borrow_mut() = Some(vec![]);
    let results = support.scope(&lock, || -> Result<_, DocumentError> {
        let good = native::consume_plan(&support, &lock, &source, "", &addressing, &plan);
        std::fs::write(&file, &mutated).unwrap();
        let reread = std::fs::read_to_string(&file).unwrap();
        let bad = native::consume_plan(&support, &lock, &reread, "", &addressing, &plan);
        std::fs::write(&file, &source).unwrap();
        let restored = std::fs::read_to_string(&file).unwrap();
        assert_eq!(restored, source);
        let restored = native::consume_plan(&support, &lock, &restored, "", &addressing, &plan);
        let pending = pending_runtime_control(&support, &lock)?;
        let retained = retained_premise_native_controls(&support, &lock)?;
        Ok(([good, bad, restored], pending, retained))
    })?;
    assert_eq!(support.closing_hash(), Some(Ok(())));
    std::fs::remove_file(file)?;
    let begin = mutated.find(r"\FtdocTag{c:01}{g-slice-0}").unwrap();
    let expected = DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Native,
            document: Some("fixture".into()),
            unit: None,
            language: Some("en".into()),
            target: Some(DocumentTarget::Latex),
            site: AssemblySite::Generated {
                path: "generated/documents/fixture/en.tex".into(),
                bytes: begin..begin + r"\FtdocTag{c:01}{g-slice-0}".len(),
                origins: vec![SourceOrigin {
                    unit: None,
                    language: Some("en".into()),
                    target: Some(DocumentTarget::Latex),
                    site: coverage_site(Some(0)),
                }],
            },
            fault: AssemblyFault::RuntimeTagRejected {
                input: "c:01".into(),
                site_token: "g-slice-0".into(),
                message: "tag \"c:01\": unknown".into(),
            },
        }])
        .unwrap(),
    );
    let (results, (pending_results, pending_expected), retained_results) = results;
    for (actual, expected, display) in retained_results {
        assert_eq!(actual, expected);
        if let Err(error) = actual {
            assert_eq!(error.to_string(), display);
        }
    }
    assert_eq!(
        pending_results,
        [Ok(()), Err(pending_expected.clone()), Ok(())]
    );
    assert_eq!(
        pending_results[1].as_ref().unwrap_err().to_string(),
        pending_expected.to_string()
    );
    assert_eq!(results, [Ok(()), Err(expected.clone()), Ok(())]);
    assert_eq!(
        results[1].as_ref().unwrap_err().to_string(),
        expected.to_string()
    );
    let evidence = support.evidence.borrow_mut().take().unwrap();
    assert_eq!(evidence.len(), 15);
    let pending = &evidence[4];
    assert_eq!(pending.status, Some(1));
    assert_eq!(
        pending.stdout,
        include_str!("../../../tests/fixtures/native/reader-pending.stdout")
            .replace("@SCRATCH@", pending.scratch.to_str().unwrap())
            .as_bytes()
    );
    assert_eq!(
        pending.stderr,
        include_bytes!("../../../tests/fixtures/native/reader-pending.stderr")
    );
    assert_eq!(
        pending.log,
        include_bytes!("../../../tests/fixtures/native/reader-pending.log")
    );
    let evidence = &evidence[1];
    assert_eq!(evidence.status, Some(1));
    let stdout = include_str!("../../../tests/fixtures/native/reader-coverage.stdout")
        .replace("@SCRATCH@", evidence.scratch.to_str().unwrap());
    assert_eq!(evidence.stdout, stdout.as_bytes());
    assert_eq!(
        evidence.stderr,
        include_bytes!("../../../tests/fixtures/native/reader-coverage.stderr")
    );
    assert_eq!(
        evidence.log,
        include_bytes!("../../../tests/fixtures/native/reader-coverage.log")
    );
    Ok(())
}

fn pending_runtime_control(
    support: &super::super::NativeSupport,
    lock: &super::super::native_support::Lock,
) -> Result<([Result<(), DocumentError>; 3], DocumentError), DocumentError> {
    use super::super::{native, runtime};
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let mut plan = self::plan(&book);
    let definition_site = super::super::input::site("units/math.toml");
    let use_site = super::super::input::site("units/use.toml");
    plan.blocks[0].runs = vec![Inline::Math {
        definition_site: definition_site.clone(),
        site: use_site.clone(),
        key: "value".into(),
        markdown: "x".into(),
        latex: "x".into(),
        pending: vec![crate::PendingEntry::fixture(
            ftoo_manifest::DefinitionId::CoefficientOne,
            0,
            "x",
            vec![ftoo_manifest::PendingReason::FormulationProposed {
                object: NodeId::LangevinCoefficientOne,
            }],
            vec![],
        )],
    }];
    plan.blocks[0].map_pending(&book, 0, "fixture", "en")?;
    let addressing = runtime::render(&book, std::slice::from_ref(&plan))?;
    let source = render(&plan, DocumentTarget::Latex);
    let bad = source.replace("{c:1}{u0-p0-r0}", "{c:01}{u0-p0-r0}");
    let begin = bad.find(r"\FtdocTag{c:01}{u0-p0-r0}").unwrap();
    let expected = DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Native,
            document: Some("fixture".into()),
            unit: Some("sec:fixture:reader".into()),
            language: Some("en".into()),
            target: Some(DocumentTarget::Latex),
            site: AssemblySite::Generated {
                path: "generated/documents/fixture/en.tex".into(),
                bytes: begin..begin + r"\FtdocTag{c:01}{u0-p0-r0}".len(),
                origins: vec![definition_site, use_site]
                    .into_iter()
                    .map(|site| SourceOrigin {
                        unit: Some("sec:fixture:reader".into()),
                        language: Some("en".into()),
                        target: None,
                        site,
                    })
                    .collect(),
            },
            fault: AssemblyFault::RuntimeTagRejected {
                input: "c:01".into(),
                site_token: "u0-p0-r0".into(),
                message: "tag \"c:01\": unknown".into(),
            },
        }])
        .unwrap(),
    );
    let path = std::env::temp_dir().join(format!("pending-runtime-{}.tex", std::process::id()));
    let good = native::consume_plan(support, lock, &source, "", &addressing, &plan);
    std::fs::write(&path, &bad).unwrap();
    let bad = native::consume_plan(
        support,
        lock,
        &std::fs::read_to_string(&path).unwrap(),
        "",
        &addressing,
        &plan,
    );
    std::fs::write(&path, &source).unwrap();
    let reread = std::fs::read_to_string(&path).unwrap();
    assert_eq!(reread, source);
    let restored = native::consume_plan(support, lock, &reread, "", &addressing, &plan);
    std::fs::remove_file(path).unwrap();
    Ok(([good, bad, restored], expected))
}

#[test]
#[allow(clippy::too_many_lines)] // All reason templates retain complete literal target expectations together.
fn reader_pending_templates_keep_all_blockers_and_actual_targets() {
    use ftoo_manifest::{Awaiting, Blocker, CapabilityId, DefinitionId, PendingReason};
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let object = NodeId::LangevinCoefficientOneValue;
    let mut cases = vec![
        (
            PendingReason::BackingUnbound { object },
            "backing object ",
            " is unbound",
        ),
        (
            PendingReason::FormulationProposed { object },
            "retained formulation ",
            "",
        ),
    ];
    for (blocker, after) in [
        (Blocker::AuditRunRequired, " awaits audit-run-required"),
        (
            Blocker::Awaiting(Awaiting::AuditBinding),
            " awaits audit-binding",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(
                CapabilityId::ExactRationalLinearEquivalence,
            )),
            " awaits capability exact-rational-linear-equivalence",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(
                CapabilityId::FormalTruncatedPowerSeries,
            )),
            " awaits capability formal-truncated-power-series",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(CapabilityId::SymbolicTrigonometry)),
            " awaits capability symbolic-trigonometry",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(CapabilityId::TrigonometricIntegration)),
            " awaits capability trigonometric-integration",
        ),
    ] {
        cases.push((
            PendingReason::ClaimDeferred { object, blocker },
            "claim evidence ",
            after,
        ));
    }
    for (reason, before, after) in cases {
        let mut plan = plan(&book);
        let block = &mut plan.blocks[0];
        block.runs = vec![Inline::Math {
            definition_site: super::super::input::site("math.toml"),
            key: "value".into(),
            markdown: "x".into(),
            latex: "x".into(),
            pending: vec![crate::PendingEntry::fixture(
                DefinitionId::CoefficientOne,
                0,
                "x",
                vec![reason.clone()],
                vec![],
            )],
            site: block.site.clone(),
        }];
        block.map_pending(&book, 7, "fixture", "en").unwrap();
        for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
            let expected = if target == DocumentTarget::Latex {
                format!(
                    "Pending notation: \\(x\\): {before}\\FtdocTag{{v:1}}{{u7-p0-r0}}% label claim:langevin:coefficient-one-value\n{}{}.\n\n",
                    if after.is_empty() { "" } else { r"\space{}" },
                    after.strip_prefix(' ').unwrap_or(after)
                )
            } else {
                format!(
                    "Pending notation: $x$: {before}v:1<!-- label claim:langevin:coefficient-one-value -->{}.\n\n",
                    after.replace('-', "&#45;")
                )
            };
            assert_eq!(
                super::super::pending::paragraph(&plan.blocks[0], "en", target),
                expected
            );
            assert_eq!(
                check_emission(&plan, &render(&plan, target), target),
                Ok(())
            );
            let entries = pending_records(&plan, &plan.blocks[0], target);
            let EmissionKind::PendingExplanation(entry) = &entries[0].kind else {
                panic!("complete entry")
            };
            assert_eq!(entry.reasons, vec![reason.clone()]);
            assert_eq!(
                entry.names,
                vec![super::super::PendingReaderName {
                    reason: 0,
                    role: ftoo_manifest::ReasonRole::Subject,
                    object,
                    handle: "v:1".into(),
                    label: "claim:langevin:coefficient-one-value".into(),
                    token: "u7-p0-r0".into()
                }]
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // Pin every origin in the complete diagnostic family.
fn reader_pending_collects_unavailable_objects_with_real_mixed_origins() {
    use ftoo_manifest::{
        DefinitionId, OutOfSliceCause, ReaderIdentityReason, ReaderTagCandidate, TagId,
        TagResolutionFault,
    };
    for empty in [false, true] {
        let mut draft = ftoo_manifest::seed_draft();
        draft.coverage.slice = if empty {
            vec![]
        } else {
            vec![NodeId::LangevinCoefficientOneValue]
        };
        let graph = ftoo_manifest::validate(draft).unwrap();
        let book = ftoo_manifest::addressing(&graph).unwrap();
        for target in [
            None,
            Some(DocumentTarget::Latex),
            Some(DocumentTarget::Markdown),
        ] {
            let definition_site = super::super::input::site("definition.toml");
            let uses = [
                super::super::input::site("first-use.toml"),
                super::super::input::site("second-use.toml"),
            ];
            let reasons = vec![
                ftoo_manifest::PendingReason::FormulationProposed {
                    object: NodeId::LangevinCoefficientThreeValue,
                },
                ftoo_manifest::PendingReason::FormulationProposed {
                    object: NodeId::LangevinCoefficientOneValue,
                },
            ];
            let runs: Vec<_> = uses
                .iter()
                .map(|site| Inline::Math {
                    definition_site: definition_site.clone(),
                    site: site.clone(),
                    key: "value".into(),
                    markdown: "x".into(),
                    latex: "x".into(),
                    pending: vec![crate::PendingEntry::fixture(
                        DefinitionId::FirstCoefficient,
                        0,
                        "x",
                        reasons.clone(),
                        vec![],
                    )],
                })
                .collect();
            let mut block = Block {
                pending_names: BTreeMap::new(),
                displays: BTreeMap::new(),
                unit: "sec:fixture:pending".into(),
                depth: None,
                site: definition_site.clone(),
                runs: vec![],
            };
            if let Some(target) = target {
                block.displays.insert(
                    target,
                    DisplayBody {
                        source: "@math{value}".into(),
                        layout: ftoo_manifest::DisplayLayout::Quote {
                            rows: vec![Some(0)],
                        },
                        site: definition_site.clone(),
                        regions: vec![Block {
                            pending_names: BTreeMap::new(),
                            displays: BTreeMap::new(),
                            unit: block.unit.clone(),
                            depth: None,
                            site: definition_site.clone(),
                            runs,
                        }],
                    },
                );
            } else {
                block.runs = runs;
            }
            let origins: Vec<_> = uses
                .iter()
                .flat_map(|site| {
                    [
                        SourceOrigin {
                            unit: Some(block.unit.clone()),
                            language: Some("en".into()),
                            target: None,
                            site: definition_site.clone(),
                        },
                        SourceOrigin {
                            unit: Some(block.unit.clone()),
                            language: Some("en".into()),
                            target,
                            site: site.clone(),
                        },
                    ]
                })
                .collect();
            let mut diagnostics = vec![];
            for (reason, object, handle, tag) in [
                (
                    0,
                    NodeId::LangevinCoefficientThreeValue,
                    "v:3",
                    TagId::LangevinCoefficientThreeValue,
                ),
                (
                    1,
                    NodeId::LangevinCoefficientOneValue,
                    "v:1",
                    TagId::LangevinCoefficientOneValue,
                ),
            ] {
                if !empty && reason == 1 {
                    continue;
                }
                diagnostics.push(AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("fixture".into()),
                    unit: Some(block.unit.clone()),
                    language: Some("en".into()),
                    target,
                    site: AssemblySite::ReaderIdentity {
                        position: ReaderIdentityPosition::Pending {
                            unit: block.unit.clone(),
                            definition: DefinitionId::FirstCoefficient,
                            reason,
                        },
                        origins: origins.clone(),
                    },
                    fault: AssemblyFault::ReaderIdentity {
                        target: ftoo_manifest::AddressTarget::Theory(object),
                        reason: ReaderIdentityReason::NoLiveTag,
                        candidates: vec![ReaderTagCandidate {
                            handle: handle.into(),
                            outcome: Err(TagResolutionFault::OutOfSliceTag {
                                input: handle.into(),
                                tag,
                                target: object,
                                cause: OutOfSliceCause::ClaimNotSelected,
                            }),
                        }],
                    },
                });
            }
            if empty && target.is_none() {
                let ordered: Vec<_> = diagnostics
                    .iter()
                    .flat_map(|diagnostic| {
                        [DocumentTarget::Markdown, DocumentTarget::Latex].map(|target| {
                            let mut diagnostic = diagnostic.clone();
                            diagnostic.target = Some(target);
                            diagnostic
                        })
                    })
                    .collect();
                let reversed = ordered.iter().rev().cloned().collect();
                assert_eq!(
                    AssemblyErrors::from_diagnostics(reversed)
                        .unwrap()
                        .diagnostics(),
                    ordered
                );
            }
            let expected =
                DocumentError::Assembly(AssemblyErrors::from_diagnostics(diagnostics).unwrap());
            let actual = block.map_pending(&book, 0, "fixture", "en").unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), expected.to_string());
            assert_eq!(block.pending_names, BTreeMap::new());
            let restored = ftoo_manifest::seed().unwrap();
            let restored = ftoo_manifest::addressing(&restored).unwrap();
            assert_eq!(block.map_pending(&restored, 0, "fixture", "en"), Ok(()));
        }
    }
}

#[test]
fn reader_pending_shared_selection_retains_both_target_bodies_and_definition_sites() {
    use ftoo_manifest::DefinitionId;
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let mut plan = plan(&book);
    let run = |name: &str| Inline::Math {
        definition_site: super::super::input::site("definition.toml"),
        key: "value".into(),
        markdown: "x".into(),
        latex: "x".into(),
        pending: vec![crate::PendingEntry::fixture(
            DefinitionId::CoefficientOne,
            0,
            "x",
            vec![ftoo_manifest::PendingReason::BackingUnbound {
                object: NodeId::LangevinCoefficientOne,
            }],
            vec![],
        )],
        site: super::super::input::site(name),
    };
    let block = &mut plan.blocks[0];
    block.runs = vec![run("statement.toml")];
    for (target, name) in [
        (DocumentTarget::Markdown, "markdown.toml"),
        (DocumentTarget::Latex, "latex.toml"),
    ] {
        block.displays.insert(
            target,
            DisplayBody {
                source: String::new(),
                layout: ftoo_manifest::DisplayLayout::Quote {
                    rows: vec![Some(0)],
                },
                regions: vec![Block {
                    pending_names: BTreeMap::new(),
                    displays: BTreeMap::new(),
                    unit: block.unit.clone(),
                    depth: None,
                    site: super::super::input::site(name),
                    runs: vec![run(name)],
                }],
                site: super::super::input::site(name),
            },
        );
    }
    let origin = |target, file| SourceOrigin {
        unit: Some("sec:fixture:reader".into()),
        language: Some("en".into()),
        target,
        site: super::super::input::site(file),
    };
    let shared = vec![
        origin(None, "definition.toml"),
        origin(None, "statement.toml"),
        origin(None, "definition.toml"),
        origin(Some(DocumentTarget::Markdown), "markdown.toml"),
        origin(None, "definition.toml"),
        origin(Some(DocumentTarget::Latex), "latex.toml"),
    ];
    assert_eq!(
        block.pending_origins(DefinitionId::CoefficientOne, "en", None),
        shared
    );
    assert_eq!(
        block.pending_origins(
            DefinitionId::CoefficientOne,
            "en",
            Some(DocumentTarget::Latex)
        ),
        vec![
            origin(None, "definition.toml"),
            origin(None, "statement.toml"),
            origin(None, "definition.toml"),
            origin(Some(DocumentTarget::Latex), "latex.toml")
        ]
    );
}

#[test]
fn markdown_handles_are_local_to_the_recovered_region() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let mut plan = self::plan(&book);
    let site = plan.blocks[0].site.clone();
    plan.blocks[0].runs = vec![
        Inline::Text("prefix".into()),
        Inline::Tag {
            handle: "v:1".into(),
            label: "claim:langevin:coefficient-one-value".into(),
            token: "u0-t0".into(),
            site,
        },
    ];
    plan.blocks[0].pending_names.insert(
        (
            DocumentTarget::Markdown,
            ftoo_manifest::DefinitionId::CoefficientOne,
        ),
        vec![super::super::PendingReaderName {
            reason: 0,
            role: ftoo_manifest::ReasonRole::Subject,
            object: NodeId::LangevinCoefficientOneValue,
            handle: "prefixv:1".into(),
            label: "claim:langevin:coefficient-one-value".into(),
            token: "u0-p0-r0".into(),
        }],
    );
    let source = render(&plan, DocumentTarget::Markdown);
    assert_eq!(
        check_emission(&plan, &source, DocumentTarget::Markdown),
        Ok(())
    );
}

fn premise_plan(book: &ftoo_manifest::AddressBook<'_>) -> DocumentPlan {
    let mut plan = plan(book);
    let block = &mut plan.blocks[0];
    block.runs = vec![Inline::Math {
        definition_site: block.site.clone(),
        site: block.site.clone(),
        key: "value".into(),
        markdown: "x".into(),
        latex: "x".into(),
        pending: vec![crate::PendingEntry::fixture(
            ftoo_manifest::DefinitionId::CoefficientThreeValue,
            0,
            "x",
            vec![ftoo_manifest::PendingReason::ClaimAuditDeferred {
                object: NodeId::LangevinCoefficientThreeValue,
                reason: ftoo_manifest::DeferredReason::Premise {
                    claim: NodeId::LangevinCoefficientOneValue,
                    evidence: ftoo_manifest::VerdictClass::Undecided,
                },
            }],
            vec![],
        )],
    }];
    block.map_pending(book, 0, "fixture", "en").unwrap();
    plan
}

#[test]
fn retained_premise_selectors_refuse_each_missing_object_and_restore() {
    use NodeId::{LangevinCoefficientOneValue as One, LangevinCoefficientThreeValue as Three};
    use ftoo_manifest::{
        AddressTarget, DeferredReason, DefinitionId, OutOfSliceCause, PendingReason,
        ReaderIdentityReason, ReaderTagCandidate, TagId, TagResolutionFault, VerdictClass,
    };
    let graph = ftoo_manifest::seed().unwrap();
    let full = ftoo_manifest::addressing(&graph).unwrap();
    let mut draft = ftoo_manifest::seed_draft();
    draft.coverage.slice = vec![One];
    let partial = ftoo_manifest::validate(draft).unwrap();
    let partial = ftoo_manifest::addressing(&partial).unwrap();
    for (subject, premise) in [(Three, One), (One, Three)] {
        let mut plan = premise_plan(&full);
        let block = &mut plan.blocks[0];
        let Inline::Math { pending, .. } = &mut block.runs[0] else {
            panic!("math")
        };
        *pending = vec![crate::PendingEntry::fixture(
            DefinitionId::CoefficientThreeValue,
            0,
            "x",
            vec![PendingReason::ClaimAuditDeferred {
                object: subject,
                reason: DeferredReason::Premise {
                    claim: premise,
                    evidence: VerdictClass::Undecided,
                },
            }],
            vec![],
        )];
        block.pending_names.clear();
        let site = AssemblySite::ReaderIdentity {
            position: ReaderIdentityPosition::Pending {
                unit: block.unit.clone(),
                definition: DefinitionId::CoefficientThreeValue,
                reason: 0,
            },
            origins: vec![
                SourceOrigin {
                    unit: Some(block.unit.clone()),
                    language: Some("en".into()),
                    target: None,
                    site: block.site.clone()
                };
                2
            ],
        };
        let fault = AssemblyFault::ReaderIdentity {
            target: AddressTarget::Theory(Three),
            reason: ReaderIdentityReason::NoLiveTag,
            candidates: vec![ReaderTagCandidate {
                handle: "v:3".into(),
                outcome: Err(TagResolutionFault::OutOfSliceTag {
                    input: "v:3".into(),
                    tag: TagId::LangevinCoefficientThreeValue,
                    target: Three,
                    cause: OutOfSliceCause::ClaimNotSelected,
                }),
            }],
        };
        let expected = DocumentError::Assembly(
            AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                rule: RuleId::M055,
                stage: AssemblyStage::Emission,
                document: Some("fixture".into()),
                unit: Some(block.unit.clone()),
                language: Some("en".into()),
                target: None,
                site: site.clone(),
                fault: fault.clone(),
            }])
            .unwrap(),
        );
        let actual = block.map_pending(&partial, 0, "fixture", "en").unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(
            actual.to_string(),
            format!(
                "M055 Emission document=Some(\"fixture\") unit=Some(\"sec:fixture:reader\") language=Some(\"en\") target=None at {site:?}: {fault:?}\n"
            )
        );
        assert_eq!(block.map_pending(&full, 0, "fixture", "en"), Ok(()));
        for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
            assert_eq!(
                check_emission(&plan, &render(&plan, target), target),
                Ok(())
            );
        }
    }
}

#[test]
fn retained_premise_reason_has_two_independent_names_and_exact_fragments() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let plan = premise_plan(&book);
    for (target, expected) in [
        (
            DocumentTarget::Markdown,
            "Pending notation: $x$: claim evidence v:3<!-- label claim:langevin:coefficient-three-value --> awaits premise v:1<!-- label claim:langevin:coefficient-one-value --> with evidence undecided.\n\n",
        ),
        (
            DocumentTarget::Latex,
            "Pending notation: \\(x\\): claim evidence \\FtdocTag{v:3}{u0-p0-r0}% label claim:langevin:coefficient-three-value\n\\space{}awaits premise \\FtdocTag{v:1}{u0-p0-r0-premise}% label claim:langevin:coefficient-one-value\n\\space{}with evidence undecided.\n\n",
        ),
    ] {
        assert_eq!(
            super::super::pending::paragraph(&plan.blocks[0], "en", target),
            expected
        );
        let records = pending_records(&plan, &plan.blocks[0], target);
        let EmissionKind::PendingExplanation(entry) = &records[0].kind else {
            panic!("pending explanation");
        };
        assert_eq!(
            entry.names,
            vec![
                super::super::PendingReaderName {
                    reason: 0,
                    role: ftoo_manifest::ReasonRole::Subject,
                    object: NodeId::LangevinCoefficientThreeValue,
                    handle: "v:3".into(),
                    label: "claim:langevin:coefficient-three-value".into(),
                    token: "u0-p0-r0".into()
                },
                super::super::PendingReaderName {
                    reason: 0,
                    role: ftoo_manifest::ReasonRole::Premise,
                    object: NodeId::LangevinCoefficientOneValue,
                    handle: "v:1".into(),
                    label: "claim:langevin:coefficient-one-value".into(),
                    token: "u0-p0-r0-premise".into()
                },
            ]
        );
        let source = render(&plan, target);
        assert_eq!(check_emission(&plan, &source, target), Ok(()));
        assert_eq!(
            if target == DocumentTarget::Markdown {
                super::super::markdown::size(&plan)
            } else {
                super::super::latex::size(&plan)
            },
            source.len()
        );
    }
}

type RetainedNativeCase = (Result<(), DocumentError>, Result<(), DocumentError>, String);

#[allow(clippy::too_many_lines)] // One bracket table preserves all subject, premise and audit origins.
fn retained_premise_native_controls(
    support: &super::super::NativeSupport,
    lock: &super::super::native_support::Lock,
) -> Result<Vec<RetainedNativeCase>, DocumentError> {
    use super::super::{native, runtime};
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let verdict = ftoo_audit::Verdict::Deferred(ftoo_manifest::DeferredReason::Premise {
        claim: NodeId::LangevinCoefficientOneValue,
        evidence: ftoo_manifest::VerdictClass::Undecided,
    });
    let directory = super::super::tests::Directory::new();
    let path = directory.0.join("premise.tex");
    let mut results = vec![];
    for (plan, handle, token) in [
        (premise_plan(&book), "v:3", "u0-p0-r0"),
        (premise_plan(&book), "v:1", "u0-p0-r0-premise"),
        (
            super::retained_tests::outcome_plan(&book, &verdict),
            "v:1",
            "g-audit-0-premise",
        ),
    ] {
        let addressing = runtime::render(&book, std::slice::from_ref(&plan))?;
        let source = render(&plan, DocumentTarget::Latex);
        let unit = plan.blocks.first().map(|block| block.unit.clone());
        std::fs::write(&path, &source).unwrap();
        let reread = std::fs::read_to_string(&path).unwrap();
        assert_eq!(reread, source);
        results.push((
            native::consume_plan(support, lock, &reread, "", &addressing, &plan),
            Ok(()),
            String::new(),
        ));
        let bad = source.replace(
            &format!("{{{handle}}}{{{token}}}"),
            &format!("{{c:01}}{{{token}}}"),
        );
        std::fs::write(&path, &bad).unwrap();
        let reread = std::fs::read_to_string(&path).unwrap();
        assert_eq!(reread, bad);
        let invocation = format!("\\FtdocTag{{c:01}}{{{token}}}");
        let begin = bad.find(&invocation).unwrap();
        let site = AssemblySite::Generated {
            path: "generated/documents/fixture/en.tex".into(),
            bytes: begin..begin + invocation.len(),
            origins: if let Some(block) = plan.blocks.first() {
                vec![
                    SourceOrigin {
                        unit: unit.clone(),
                        language: Some("en".into()),
                        target: None,
                        site: block.site.clone()
                    };
                    2
                ]
            } else {
                vec![SourceOrigin {
                    unit: None,
                    language: Some("en".into()),
                    target: Some(DocumentTarget::Latex),
                    site: UnitSite {
                        file: "generated/audit-report.json".into(),
                        path: vec![
                            ftoo_manifest::PathPart::Key("outcomes".into()),
                            ftoo_manifest::PathPart::Index(1),
                        ],
                        anchor: ftoo_manifest::SourceAnchor::Unavailable,
                        decoded: None,
                        occurrence: None,
                    },
                }]
            },
        };
        let fault = AssemblyFault::RuntimeTagRejected {
            input: "c:01".into(),
            site_token: token.into(),
            message: "tag \"c:01\": unknown".into(),
        };
        let display = format!(
            "M055 Native document=Some(\"fixture\") unit={unit:?} language=Some(\"en\") target=Some(Latex) at {site:?}: {fault:?}\n"
        );
        let expected = DocumentError::Assembly(
            AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                rule: RuleId::M055,
                stage: AssemblyStage::Native,
                document: Some("fixture".into()),
                unit: unit.clone(),
                language: Some("en".into()),
                target: Some(DocumentTarget::Latex),
                site,
                fault,
            }])
            .unwrap(),
        );
        results.push((
            native::consume_plan(support, lock, &reread, "", &addressing, &plan),
            Err(expected),
            display,
        ));
        std::fs::write(&path, &source).unwrap();
        let reread = std::fs::read_to_string(&path).unwrap();
        assert_eq!(reread, source);
        results.push((
            native::consume_plan(support, lock, &reread, "", &addressing, &plan),
            Ok(()),
            String::new(),
        ));
    }
    Ok(results)
}
