use super::*;
use ftoo_manifest::{DefinitionId, PathPart, PrimitiveId, RenderOwner};

fn origin(owner: &str, case: &str, bytes: std::ops::Range<usize>) -> SourceOrigin {
    SourceOrigin {
        unit: None,
        language: None,
        target: Some(DocumentTarget::Latex),
        site: UnitSite {
            file: "assembly-sweep.tex".into(),
            path: vec![
                PathPart::Key("sweep".into()),
                PathPart::Key(owner.into()),
                PathPart::Key(case.into()),
            ],
            anchor: SourceAnchor::Bytes(bytes.start, bytes.end),
            decoded: None,
            occurrence: None,
        },
    }
}

fn rejection(
    case: NativeCase,
    bytes: std::ops::Range<usize>,
    origins: Vec<SourceOrigin>,
    stderr: &[u8],
) -> DocumentError {
    failure(
        RuleId::M056,
        AssemblyStage::Native,
        AssemblySite::Generated {
            path: "assembly-sweep.tex".into(),
            bytes,
            origins,
        },
        AssemblyFault::NativeRejected {
            case,
            status: Some(1),
            stdout: b"stdout\n".to_vec(),
            stderr: stderr.to_vec(),
            log: b"! Undefined control sequence.\nl.4 token\n".to_vec(),
        },
    )
}

fn exact(actual: &DocumentError, expected: &DocumentError) {
    let DocumentError::Assembly(errors) = expected else {
        panic!("independent assembly oracle")
    };
    let diagnostic = &errors.diagnostics()[0];
    let display = format!(
        "M056 Native document=None unit=None language=None target=None at {:?}: {:?}\n",
        diagnostic.site, diagnostic.fault
    );
    assert_eq!(actual, expected);
    assert_eq!(actual.to_string(), display);
}

#[test]
fn sweep_failure_lines_select_exact_rows_and_preserve_complete_evidence() {
    let mut source = "α\n".to_owned();
    let rows = vec![
        native::sweep_row(
            &mut source,
            RenderOwner::Definition(DefinitionId::CoefficientOne),
            "base",
            "x",
        ),
        native::sweep_row(
            &mut source,
            RenderOwner::Primitive(PrimitiveId::Add),
            "compound",
            "a+b",
        ),
    ];
    source.push_str("\\end{document}\n");
    assert_eq!(
        source,
        "α\n% sweep Definition(CoefficientOne) base\n\\(x\\)\\par\n% sweep Primitive(Add) compound\n\\(a+b\\)\\par\n\\end{document}\n"
    );
    let first = 3..53;
    let second = 53..97;
    assert_eq!(rows[0].bytes, first);
    assert_eq!(rows[1].bytes, second);
    let first_origin = origin("coefficient-one", "base", first.clone());
    let second_origin = origin("add", "compound", second.clone());
    assert_eq!(rows[0].origin, first_origin);
    assert_eq!(rows[1].origin, second_origin);
    let complete = NativeCase::Sweep {
        owner: RenderOwner::Expression,
        case: "complete".into(),
    };
    let all = vec![first_origin.clone(), second_origin.clone()];
    for (line, range, owner, case, selected) in [
        (
            2,
            first.clone(),
            RenderOwner::Definition(DefinitionId::CoefficientOne),
            "base",
            first_origin.clone(),
        ),
        (
            3,
            first,
            RenderOwner::Definition(DefinitionId::CoefficientOne),
            "base",
            first_origin,
        ),
        (
            4,
            second.clone(),
            RenderOwner::Primitive(PrimitiveId::Add),
            "compound",
            second_origin.clone(),
        ),
        (
            5,
            second,
            RenderOwner::Primitive(PrimitiveId::Add),
            "compound",
            second_origin,
        ),
    ] {
        let stderr = format!(
            "warning: context\nerror: document.tex:{line}: Undefined control sequence\nerror: halted\n"
        );
        let original = rejection(
            complete.clone(),
            0..source.len(),
            all.clone(),
            stderr.as_bytes(),
        );
        let expected = rejection(
            NativeCase::Sweep {
                owner,
                case: case.into(),
            },
            range,
            vec![selected],
            stderr.as_bytes(),
        );
        let actual = native::sweep_origin(original, &source, &rows);
        exact(&actual, &expected);
    }
    for stderr in [
        "error: notation.tex:4: bad\nerror: document.tex:4: later\n",
        "error: document.tex:0: bad\n",
        "error: document.tex:04: bad\n",
        "error: document.tex:1: preamble\n",
        "error: document.tex:6: closure\n",
        "error: document.tex:7: past end\n",
        "error: no source line\n",
    ] {
        let expected = rejection(
            complete.clone(),
            0..source.len(),
            all.clone(),
            stderr.as_bytes(),
        );
        let actual = native::sweep_origin(expected.clone(), &source, &rows);
        exact(&actual, &expected);
    }
}

pub(super) fn support_required_native_sweep_row_has_complete_evidence_and_restores(
    support: &NativeSupport,
    lock: &native_support::Lock,
    checks: &mut Vec<CaseEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let addressing = runtime::render(&book, &[])?;
    let mut source = latex::start("assembly-sweep", "en");
    let rows = vec![
        native::sweep_row(
            &mut source,
            RenderOwner::Primitive(PrimitiveId::Add),
            "base",
            "1+2",
        ),
        native::sweep_row(
            &mut source,
            RenderOwner::Definition(DefinitionId::CoefficientOne),
            "base",
            "\\FtdocStored",
        ),
        native::sweep_row(
            &mut source,
            RenderOwner::Definition(DefinitionId::Dimension),
            "compound",
            "x",
        ),
    ];
    source.push_str("\\end{document}\n");
    let notation = "\\def\\FtdocStored{\\UndefinedStoredCommand}\n";
    let mut scratch = PathBuf::new();
    let error = native::consume_with(
        support,
        lock,
        &source,
        notation,
        &addressing,
        NativeCase::Sweep {
            owner: RenderOwner::Expression,
            case: "complete".into(),
        },
        rows.iter().map(|row| row.origin.clone()).collect(),
        |path| scratch = path.into(),
        |_, _| {},
    );
    let (stdout, stderr, log): (String, String, String) = serde_json::from_str(include_str!(
        "../../../tests/fixtures/assembly/sweep-rejection.json"
    ))?;
    assert_eq!(
        &source[535..596],
        "% sweep Definition(CoefficientOne) base\n\\(\\FtdocStored\\)\\par\n"
    );
    let expected = failure(
        RuleId::M056,
        AssemblyStage::Native,
        AssemblySite::Generated {
            path: "assembly-sweep.tex".into(),
            bytes: 535..596,
            origins: vec![origin("coefficient-one", "base", 535..596)],
        },
        AssemblyFault::NativeRejected {
            case: NativeCase::Sweep {
                owner: RenderOwner::Definition(DefinitionId::CoefficientOne),
                case: "base".into(),
            },
            status: Some(1),
            stdout: stdout
                .replace("@SCRATCH@", scratch.to_str().unwrap())
                .into_bytes(),
            stderr: stderr.into_bytes(),
            log: log.into_bytes(),
        },
    );
    checks.push(CaseEvidence::new(
        "sweep/refusal",
        error.map_err(|error| native::sweep_origin(error, &source, &rows)),
        Err(expected),
    ));
    let restored = notation.replace("\\UndefinedStoredCommand", "1");
    checks.push(CaseEvidence::new(
        "sweep/restored",
        native::consume(
            support,
            lock,
            &source,
            &restored,
            &addressing,
            NativeCase::Sweep {
                owner: RenderOwner::Expression,
                case: "complete".into(),
            },
            rows.iter().map(|row| row.origin.clone()).collect(),
        )
        .map_err(|error| native::sweep_origin(error, &source, &rows)),
        Ok(()),
    ));
    Ok(())
}
