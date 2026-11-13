use super::*;
use std::os::unix::process::ExitStatusExt;

#[test]
fn runtime_records_and_overfull_boxes_retain_both_complete_faults() {
    let directory = Directory::new();
    let path = directory.0.join("co-occurrence.log");
    let site = AssemblySite::Generated {
        path: "document.tex".into(),
        bytes: 0..12,
        origins: vec![],
    };
    let case = NativeCase::Document {
        document: "fixture".into(),
        language: "en".into(),
    };
    for valid in [true, false] {
        let record = if valid {
            "FTDOC-R:i:0:1:61\nFTDOC-R:s:0:1:62\nFTDOC-R:m:0:1:63\nFTDOC-END\n"
        } else {
            "FTDOC-R:invalid\n"
        };
        let log =
            format!("{record}Overfull \\hbox (1.0pt too wide) detected at line 2\ncontinuation\n");
        let stderr =
            b"warning: document.tex:2: Overfull \\hbox (1.0pt too wide) detected at line 2\n"
                .to_vec();
        let stdout = b"complete output\n".to_vec();
        let diagnostic = |rule, fault| AssemblyDiagnostic {
            rule,
            stage: AssemblyStage::Native,
            document: Some("fixture".into()),
            unit: None,
            language: Some("en".into()),
            target: Some(DocumentTarget::Latex),
            site: site.clone(),
            fault,
        };
        let first = if valid {
            AssemblyFault::RuntimeTagRejected {
                input: "a".into(),
                site_token: "b".into(),
                message: "c".into(),
            }
        } else {
            AssemblyFault::InvalidRuntimeRecord {
                status: Some(0),
                stdout: stdout.clone(),
                stderr: stderr.clone(),
                log: log.as_bytes().to_vec(),
            }
        };
        let expected = DocumentError::Assembly(
            AssemblyErrors::from_diagnostics(vec![
                diagnostic(RuleId::M055, first),
                diagnostic(
                    RuleId::M056,
                    AssemblyFault::NativeRejected {
                        case: case.clone(),
                        status: Some(0),
                        stdout: stdout.clone(),
                        stderr: stderr.clone(),
                        log: log.as_bytes().to_vec(),
                    },
                ),
            ])
            .unwrap(),
        );
        fs::write(&path, &log).unwrap();
        let actual = native::classify(
            std::process::Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout,
                stderr,
            },
            fs::read(&path).unwrap(),
            site.clone(),
            case.clone(),
            true,
        )
        .unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), expected.to_string());
        fs::write(&path, "restored\n").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"restored\n");
        assert_eq!(
            native::classify(
                std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: vec![],
                    stderr: vec![],
                },
                fs::read(&path).unwrap(),
                site.clone(),
                case.clone(),
                true
            ),
            Ok(())
        );
    }
}

#[test]
fn overfull_channels_refuse_zero_exit_once_and_restore_complete_evidence() {
    let directory = Directory::new();
    let path = directory.0.join("diagnostic.log");
    let stderr_path = directory.0.join("diagnostic.stderr");
    let case = NativeCase::Document {
        document: "fixture".into(),
        language: "en".into(),
    };
    let site = AssemblySite::Generated {
        path: "document.tex".into(),
        bytes: 0..12,
        origins: vec![],
    };
    for line in [
        "Overfull \\hbox (1.0pt too wide) in paragraph at lines 2--3",
        "Overfull \\vbox (1.0pt too high) detected at line 3",
    ] {
        for (log, stderr) in [
            (
                format!("before\n{line}\ncontinuation\nafter\n"),
                String::new(),
            ),
            (String::new(), format!("warning: {line}\n")),
            (String::new(), format!("warning: document.tex:3: {line}\n")),
            (
                format!("{line}\n"),
                format!("warning: document.tex:3: {line}\nwarning: document.tex:3: {line}\n"),
            ),
            (
                format!("{line}\r\ncontinuation\r\n"),
                format!("warning: document.tex:3: {line}\r\n"),
            ),
        ] {
            fs::write(&path, &log).unwrap();
            fs::write(&stderr_path, &stderr).unwrap();
            let reread = fs::read(&path).unwrap();
            let expected = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M056,
                    stage: AssemblyStage::Native,
                    document: Some("fixture".into()),
                    unit: None,
                    language: Some("en".into()),
                    target: Some(DocumentTarget::Latex),
                    site: site.clone(),
                    fault: AssemblyFault::NativeRejected {
                        case: case.clone(),
                        status: Some(0),
                        stdout: b"complete output\n".to_vec(),
                        stderr: stderr.as_bytes().to_vec(),
                        log: reread.clone(),
                    },
                }])
                .unwrap(),
            );
            let output = |stderr: Vec<u8>| std::process::Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: b"complete output\n".to_vec(),
                stderr,
            };
            let actual = native::classify(
                output(fs::read(&stderr_path).unwrap()),
                reread,
                site.clone(),
                case.clone(),
                true,
            )
            .unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), expected.to_string());
            fs::write(&path, "restored\n").unwrap();
            fs::write(&stderr_path, "").unwrap();
            assert_eq!(fs::read(&stderr_path).unwrap(), [] as [u8; 0]);
            assert_eq!(fs::read(&path).unwrap(), b"restored\n");
            assert_eq!(
                native::classify(
                    output(fs::read(&stderr_path).unwrap()),
                    fs::read(&path).unwrap(),
                    site.clone(),
                    case.clone(),
                    true
                ),
                Ok(())
            );
        }
    }
}

#[test]
fn overfull_locations_require_explicit_input_and_all_diagnostics() {
    let source = "one\ntwo\nthree\nfour\n";
    let horizontal = b"Overfull \\hbox (1.0pt too wide) in paragraph at lines 2--3";
    let warning =
        b"warning: document.tex:3: Overfull \\hbox (1.0pt too wide) in paragraph at lines 2--3";
    assert_eq!(
        native::overfull_range(source, horizontal, warning),
        Some(4..14)
    );
    assert_eq!(native::overfull_range(source, b"", warning), Some(4..14));
    assert_eq!(native::overfull_range(source, horizontal, b""), None);
    for prefix in [
        "warning: notation.tex:3: ",
        "warning: document.tex:0: ",
        "warning: document.tex:999: ",
        "warning: ",
    ] {
        let stderr = format!("{prefix}Overfull \\vbox (1.0pt too high) detected at line 3");
        assert_eq!(native::overfull_range(source, b"", stderr.as_bytes()), None);
    }
    let extra = [
        horizontal.as_slice(),
        b"\nOverfull \\vbox (2.0pt too high) detected at line 4",
    ]
    .concat();
    assert_eq!(native::overfull_range(source, &extra, warning), None);
    let two_warnings = [
        warning.as_slice(),
        b"\nwarning: document.tex:4: Overfull \\vbox (2.0pt too high) detected at line 4",
    ]
    .concat();
    assert_eq!(
        native::overfull_range(source, &extra, &two_warnings),
        Some(4..19)
    );
    let line = "Overfull \\hbox (1234567890123456789012345678901234567890.0pt too wide) in paragraph at lines 2--3";
    let wrapped = format!("{}\n{}\n", &line[..79], &line[79..]);
    let mirror = format!("warning: document.tex:3: {line}\n");
    assert_eq!(
        native::overfull_range(source, wrapped.as_bytes(), mirror.as_bytes()),
        Some(4..14)
    );
    for (log, stderr, bytes) in [
        (extra, two_warnings, 4..19),
        (wrapped.into_bytes(), mirror.into_bytes(), 4..14),
    ] {
        let directory = Directory::new();
        let path = directory.0.join("boxes.log");
        fs::write(&path, &log).unwrap();
        let case = NativeCase::Document {
            document: "fixture".into(),
            language: "en".into(),
        };
        let site = AssemblySite::Generated {
            path: "document.tex".into(),
            bytes,
            origins: vec![],
        };
        let expected = DocumentError::Assembly(
            AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                rule: RuleId::M056,
                stage: AssemblyStage::Native,
                document: Some("fixture".into()),
                unit: None,
                language: Some("en".into()),
                target: Some(DocumentTarget::Latex),
                site: site.clone(),
                fault: AssemblyFault::NativeRejected {
                    case: case.clone(),
                    status: Some(0),
                    stdout: vec![],
                    stderr: stderr.clone(),
                    log: log.clone(),
                },
            }])
            .unwrap(),
        );
        let actual = native::classify(
            std::process::Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: vec![],
                stderr,
            },
            fs::read(&path).unwrap(),
            site.clone(),
            case.clone(),
            true,
        )
        .unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), expected.to_string());
        fs::write(&path, "restored\n").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"restored\n");
        assert_eq!(
            native::classify(
                std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: vec![],
                    stderr: vec![]
                },
                fs::read(&path).unwrap(),
                site,
                case,
                true
            ),
            Ok(())
        );
    }
    for harmless in [
        "Underfull \\hbox (badness 10000)",
        "prose Overfull \\hbox (1pt too wide)",
        "warning: prose Overfull \\hbox (1pt too wide)",
    ] {
        assert!(!native::has_overfull(
            harmless.as_bytes(),
            harmless.as_bytes()
        ));
    }
}

#[test]
fn sweep_explanation_group_is_exact_and_restores() {
    let entries = vec![crate::PendingEntry::fixture(
        ftoo_manifest::DefinitionId::CoefficientOne,
        0,
        r"{c}_{1}",
        vec![ftoo_manifest::PendingReason::FormulationProposed {
            object: ftoo_manifest::NodeId::LangevinCoefficientOne,
        }],
        vec![],
    )];
    let correct = "{\\raggedright\nPending notation: \\({c}_{1}\\): retained formulation def:\\allowbreak{}langevin:\\allowbreak{}coefficient-\\allowbreak{}one.\\par}\n\n";
    assert_eq!(super::super::pending::sweep(&entries), correct);
    let directory = Directory::new();
    let path = directory.0.join("sweep.tex");
    for bad in [
        correct.replace("{\\raggedright\n", ""),
        correct.replace("\\par}", "}\\par"),
        correct.replace("\\raggedright", "\\raggedleft"),
        correct.replacen(r"\allowbreak{}", "", 1),
        correct.replace("retained", "changed"),
        format!("{correct}extra"),
    ] {
        fs::write(&path, &bad).unwrap();
        let reread = fs::read_to_string(&path).unwrap();
        assert_eq!(reread, bad);
        assert!(!plan::check_sweep_pending(&reread, &entries));
        fs::write(&path, correct).unwrap();
        let restored = fs::read_to_string(&path).unwrap();
        assert_eq!(restored, correct);
        assert!(plan::check_sweep_pending(&restored, &entries));
    }
    assert!(plan::check_sweep_pending("", &[]));
    assert!(!plan::check_sweep_pending(correct, &[]));
}

#[test]
fn overfull_sweep_origin_retains_all_affected_rows() {
    use ftoo_manifest::{DefinitionId, RenderOwner};
    let mut source = String::new();
    let first = native::sweep_row(
        &mut source,
        RenderOwner::Definition(DefinitionId::CoefficientOne),
        "base",
        "x",
    );
    let second = native::sweep_row(
        &mut source,
        RenderOwner::Definition(DefinitionId::CoefficientThree),
        "base",
        "y",
    );
    let third = native::sweep_row(
        &mut source,
        RenderOwner::Definition(DefinitionId::Dimension),
        "base",
        "z",
    );
    let rows = vec![first, second, third];
    for (first_line, selected) in [(2, 0..2), (4, 1..2)] {
        let line =
            format!("Overfull \\hbox (1.0pt too wide) in paragraph at lines {first_line}--4");
        let stderr = format!("warning: document.tex:4: {line}\n").into_bytes();
        let log = format!("{line}\n").into_bytes();
        let case = NativeCase::Sweep {
            owner: RenderOwner::Expression,
            case: "complete".into(),
        };
        let site = AssemblySite::Generated {
            path: "assembly-sweep.tex".into(),
            bytes: 0..source.len(),
            origins: rows.iter().map(|row| row.origin.clone()).collect(),
        };
        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: vec![],
            stderr: stderr.clone(),
        };
        let actual =
            native::classify(output, log.clone(), site.clone(), case.clone(), true).unwrap_err();
        let expected = failure(
            RuleId::M056,
            AssemblyStage::Native,
            AssemblySite::Generated {
                path: "assembly-sweep.tex".into(),
                bytes: rows[selected.start].bytes.start..rows[selected.end - 1].bytes.end,
                origins: rows[selected.clone()]
                    .iter()
                    .map(|row| row.origin.clone())
                    .collect(),
            },
            AssemblyFault::NativeRejected {
                case: if selected.len() == 1 {
                    rows[selected.start].case.clone()
                } else {
                    case
                },
                status: Some(0),
                stdout: vec![],
                stderr,
                log,
            },
        );
        assert_eq!(native::sweep_origin(actual, &source, &rows), expected);
    }
}

pub(super) fn support_required_overfull_boxes_refuse_complete_frames_and_restore_at_boundary(
    support: &NativeSupport,
    lock: &native_support::Lock,
    checks: &mut Vec<brackets::CaseEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let plan = plan::DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: plan::Coverage::fixture("Coverage.", "Slice."),
        blocks: vec![plan::Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: "sec:fixture:box".into(),
            depth: None,
            site: input::site("units/box.toml"),
            runs: vec![plan::Inline::Text("BOUNDARY".into())],
        }],
    };
    let addressing = runtime::render(&book, std::slice::from_ref(&plan))?;
    let template = latex::render(&plan);
    let good = template.replace(
        "BOUNDARY",
        r"\noindent\hbox to\hsize{\vrule width\hsize height1pt}",
    );
    let directory = Directory::new();
    let path = directory.0.join("box.tex");
    let start = support.evidence.borrow().as_ref().unwrap().len();
    let results = {
        let mut results = vec![];
        for body in [
            r"\hbox to0pt{\vrule width1pt height1pt}",
            r"\vbox to0pt{\hrule height1pt}",
        ] {
            let bad = template.replace("BOUNDARY", body);
            fs::write(&path, &bad).unwrap();
            let reread = fs::read_to_string(&path).unwrap();
            let result = native::consume_plan(support, lock, &reread, "", &addressing, &plan);
            fs::write(&path, &good).unwrap();
            let restored = fs::read_to_string(&path).unwrap();
            assert_eq!(restored, good);
            let accepted = native::consume_plan(support, lock, &restored, "", &addressing, &plan);
            results.push((bad, result, accepted));
        }
        results
    };
    let evidence = support.evidence.borrow();
    let evidence = &evidence.as_ref().unwrap()[start..];
    let frames: Vec<(String, String, String)> =
        serde_json::from_str(include_str!("../../../tests/fixtures/native/overfull.json"))?;
    assert_eq!(frames.len(), 2);
    for (index, ((source, actual, restored), (stdout, stderr, log))) in
        results.into_iter().zip(frames).enumerate()
    {
        let raw = &evidence[index * 2];
        let stdout = stdout.replace("@SCRATCH@", raw.scratch.to_str().unwrap());
        let stdout = stdout.replace(
            "@PDF-KIB@",
            &format!(
                "{:.2}",
                f64::from(u32::try_from(raw.pdf_bytes.unwrap()).unwrap()) / 1024.0
            ),
        );
        let mut alternate: Vec<_> = stdout.split_inclusive('\n').collect();
        let writes: Vec<_> = alternate
            .iter()
            .enumerate()
            .filter_map(|(index, line)| line.starts_with("note: Writing `").then_some(index))
            .collect();
        assert_eq!(writes.len(), 2);
        assert_eq!(writes[1], writes[0] + 1);
        alternate.swap(writes[0], writes[1]);
        let alternate = alternate.concat();
        let stdout = if raw.stdout == alternate.as_bytes() {
            alternate
        } else {
            stdout
        };
        let begin = source
            .find(if index == 0 {
                r"\hbox to0pt"
            } else {
                r"\vbox to0pt"
            })
            .unwrap();
        let end = begin + source[begin..].find('\n').unwrap() + 1;
        let expected = DocumentError::Assembly(
            AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                rule: RuleId::M056,
                stage: AssemblyStage::Native,
                document: Some("fixture".into()),
                unit: Some("sec:fixture:box".into()),
                language: Some("en".into()),
                target: Some(DocumentTarget::Latex),
                site: AssemblySite::Generated {
                    path: "generated/documents/fixture/en.tex".into(),
                    bytes: begin..end,
                    origins: plan
                        .origins(DocumentTarget::Latex)
                        .into_iter()
                        .filter(|origin| origin.unit.as_deref() == Some("sec:fixture:box"))
                        .collect(),
                },
                fault: AssemblyFault::NativeRejected {
                    case: NativeCase::Document {
                        document: "fixture".into(),
                        language: "en".into(),
                    },
                    status: Some(0),
                    stdout: stdout.into_bytes(),
                    stderr: stderr.into_bytes(),
                    log: log.into_bytes(),
                },
            }])
            .unwrap(),
        );
        checks.push(brackets::CaseEvidence::new(
            format!("overfull/{index}/refusal"),
            actual,
            Err(expected),
        ));
        checks.push(brackets::CaseEvidence::new(
            format!("overfull/{index}/restored"),
            restored,
            Ok(()),
        ));
    }
    assert_eq!(evidence.len(), 4);
    Ok(())
}

#[test]
fn fixed_preamble_inputs_cannot_reset_zero_fuzz() {
    let inputs = [
        (
            "notation.tex",
            include_str!("../../../../../generated/notation.tex"),
        ),
        (
            "addressing.tex",
            include_str!("../../../../../generated/addressing.tex"),
        ),
    ];
    let assignments: Vec<_> = inputs
        .into_iter()
        .flat_map(|(name, text)| {
            [r"\hfuzz", r"\vfuzz"].into_iter().flat_map(move |token| {
                text.match_indices(token)
                    .map(move |(offset, _)| (name, token, offset))
            })
        })
        .collect();
    assert_eq!(assignments, vec![]);
}

#[test]
fn exact_sweep_explanation_is_refused_in_a_reader_pending_paragraph() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let site = input::site("units/pending.toml");
    let entry = crate::PendingEntry::fixture(
        ftoo_manifest::DefinitionId::CoefficientOne,
        0,
        "x",
        vec![ftoo_manifest::PendingReason::FormulationProposed {
            object: ftoo_manifest::NodeId::LangevinCoefficientOne,
        }],
        vec![],
    );
    let mut block = plan::Block {
        pending_names: BTreeMap::new(),
        displays: BTreeMap::new(),
        unit: "sec:fixture:pending".into(),
        depth: None,
        site: site.clone(),
        runs: vec![plan::Inline::Math {
            definition_site: site.clone(),
            key: "value".into(),
            markdown: "x".into(),
            latex: "x".into(),
            pending: vec![entry.clone()],
            site,
        }],
    };
    block.map_pending(&book, 0, "fixture", "en").unwrap();
    let plan = plan::DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: plan::Coverage::fixture("Coverage.", "Slice."),
        blocks: vec![block],
    };
    let source = latex::render(&plan);
    let reader = "Pending notation: \\(x\\): retained formulation \\FtdocTag{c:1}{u0-p0-r0}% label def:langevin:coefficient-one\n.\n\n";
    let sweep = "{\\raggedright\nPending notation: \\(x\\): retained formulation def:\\allowbreak{}langevin:\\allowbreak{}coefficient-\\allowbreak{}one.\\par}\n\n";
    assert_eq!(super::super::pending::sweep(&[entry]), sweep);
    assert_eq!(source.matches(reader).count(), 1);
    let bad = source.replace(reader, sweep);
    let records = plan.records(DocumentTarget::Latex);
    let mut actual_record = records[0].clone();
    actual_record.value = format!("\\(x\\)\n\n{}", sweep.trim_end_matches('\n'));
    let expected = DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Emission,
            document: None,
            unit: None,
            language: None,
            target: None,
            site: AssemblySite::Generated {
                path: "generated/documents/fixture/en.tex".into(),
                bytes: bad.find("% unit ").unwrap()..bad.len(),
                origins: plan.origins(DocumentTarget::Latex),
            },
            fault: AssemblyFault::EmissionMismatch {
                missing: records[1..].to_vec(),
                unexpected: vec![],
                changed: vec![EmissionChange {
                    expected: records[0].clone(),
                    actual: actual_record,
                }],
            },
        }])
        .unwrap(),
    );
    let directory = Directory::new();
    let path = directory.0.join("reader.tex");
    fs::write(&path, bad).unwrap();
    let actual = plan::check_emission(
        &plan,
        &fs::read_to_string(&path).unwrap(),
        DocumentTarget::Latex,
    )
    .unwrap_err();
    assert_eq!(actual, expected);
    assert_eq!(actual.to_string(), expected.to_string());
    fs::write(&path, &source).unwrap();
    let restored = fs::read_to_string(&path).unwrap();
    assert_eq!(restored, source);
    assert_eq!(
        plan::check_emission(&plan, &restored, DocumentTarget::Latex),
        Ok(())
    );
}
