// Full independent fault and byte oracles stay beside their restoration controls.
#![allow(clippy::too_many_lines)]
#[cfg(unix)]
mod overfull;
use super::*;
mod acceptance;
mod brackets;
mod displays;
mod historical;
mod html;
pub(super) mod native_evidence;
mod pending;
mod spacing;
mod sweep;
use brackets::CaseEvidence;

pub(super) fn inherited_native_fixture(support: &NativeSupport, root: &Path) {
    support.set_inherited(
        &fs::read(root.join(inherited_support::PATH)).unwrap(),
        &fs::read(root.join(inherited_support::TYPED_PATH)).unwrap(),
    );
}

#[test]
fn comment_spacing_writer_consumes_only_semantic_spaces() {
    use plan::Inline;
    let site = UnitSite {
        file: "fixture.toml".into(),
        path: vec![],
        anchor: ftoo_manifest::SourceAnchor::WholeFile,
        decoded: None,
        occurrence: None,
    };
    for citation in [false, true] {
        let insertion = || {
            if citation {
                Inline::Citation {
                    identity: synthetic_citation_identity(
                        "dec:fixture:source",
                        "sec:fixture:spacing",
                        "record",
                        "u0-c0",
                    ),
                    text: "dec:fixture:source".into(),
                    site: site.clone(),
                }
            } else {
                Inline::Tag {
                    handle: "c:1".into(),
                    label: "def:fixture:one".into(),
                    token: "u0-t0".into(),
                    site: site.clone(),
                }
            }
        };
        let prefix = if citation {
            "\\FtdocTag{record}{u0-c0}% cite dec:fixture:source\n"
        } else {
            "\\FtdocTag{c:1}{u0-t0}% label def:fixture:one\n"
        };
        for (following, expected) in [
            (" word", r"\space{}word"),
            (".", "."),
            (" .", r"\space{}."),
            ("’s", "'s"),
            ("", ""),
        ] {
            let runs = vec![
                insertion(),
                Inline::Text(String::new()),
                Inline::Text(following.into()),
            ];
            let mut output = String::new();
            latex::prose_inline(&mut output, &runs);
            assert_eq!(output, format!("{prefix}{expected}"));
        }
        let runs = vec![
            insertion(),
            Inline::Text(" ".into()),
            Inline::Math {
                definition_site: input::site("fixture-math.toml"),
                key: "x".into(),
                markdown: "x".into(),
                latex: "x".into(),
                pending: vec![],
                site: site.clone(),
            },
            insertion(),
            insertion(),
        ];
        let mut output = String::new();
        latex::inline(&mut output, &runs);
        assert_eq!(
            output,
            format!("{prefix}\\space{{}}\\(x\\){prefix}{prefix}")
        );
        // A new region cannot inherit the previous region's comment boundary.
        latex::prose_inline(&mut output, &[Inline::Text(" next".into())]);
        assert!(output.ends_with("\n next"));
    }
}
#[test]
fn native_preparation_refuses_unsafe_archives_and_restores()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = Directory::new();
    let script = directory.0.join("preparation-fixtures.py");
    crate::support_bracket::mutation::write(&script, PREPARATION_FIXTURES)?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .args(["-I", "-B"])
        .arg(&script)
        .arg(root.join("tools/assembly/prepare-native.py"))
        .arg(&directory.0)
        .output()?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"native preparation fixtures: ok\n");
    assert_eq!(output.stderr, b"");
    Ok(())
}
const PREPARATION_FIXTURES: &str = r"import hashlib
import importlib.util
import io
from pathlib import Path
import shutil
import sys
import tarfile

spec=importlib.util.spec_from_file_location('native_preparation',sys.argv[1])
prepare=importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)
root=Path(sys.argv[2])
archive=root/'input.tar'
destination=root/'bundle'
stage=root/'staging'
outside=root/'outside'
outside.write_bytes(b'guard')
digest='ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb'
expected={'good':digest}
def staging(*,prefix,dir):
    assert prefix=='extract-' and Path(dir)==root
    stage.mkdir()
    return str(stage)
prepare.tempfile.mkdtemp=staging
def refusal(function,expected_fault):
    try:
        function()
    except ValueError as error:
        assert error.args==(expected_fault,), (error.args,expected_fault)
    else:
        raise AssertionError(('accepted',expected_fault))
def write_archive(entries):
    with tarfile.open(archive,'w') as output:
        for name,kind,data in entries:
            entry=tarfile.TarInfo(name)
            entry.type=kind
            entry.size=len(data) if kind==tarfile.REGTYPE else 0
            if kind in (tarfile.SYMTYPE,tarfile.LNKTYPE):
                entry.linkname='../outside'
            output.addfile(entry,io.BytesIO(data) if entry.size else None)
def restore():
    write_archive([('good',tarfile.REGTYPE,b'a')])
    original=archive.read_bytes()
    prepare.download({'archive-sha256':hashlib.sha256(original).hexdigest()},archive)
    assert archive.read_bytes()==original
    prepare.extract(archive,destination,expected)
    assert prepare.inventory(destination)==expected
    assert (destination/'good').read_bytes()==b'a'
    assert outside.read_bytes()==b'guard'
    shutil.rmtree(destination)
    assert not stage.exists()
restore()
for name in ['../outside','/absolute','a//b','a\\b','./a','a/../b','a/','a\x1fb','a\x7fb','a\x80b','a\x9fb']:
    write_archive([(name,tarfile.REGTYPE,b'a')])
    refusal(lambda:prepare.extract(archive,destination,expected),('member',name))
    assert not destination.exists() and not stage.exists()
    assert outside.read_bytes()==b'guard'
    restore()
for kind in [tarfile.SYMTYPE,tarfile.LNKTYPE,tarfile.DIRTYPE,tarfile.FIFOTYPE]:
    write_archive([('good',kind,b'')])
    refusal(lambda:prepare.extract(archive,destination,expected),('archive entry','good',kind))
    assert not destination.exists() and not stage.exists()
    restore()
for entries,fault in [
    ([('good',tarfile.REGTYPE,b'a'),('good',tarfile.REGTYPE,b'a')],('archive entry','good',tarfile.REGTYPE)),
    ([('extra',tarfile.REGTYPE,b'a')],('archive entry','extra',tarfile.REGTYPE)),
    ([('good',tarfile.REGTYPE,b'x')],('member integrity','good')),
    ([],('extracted inventory/integrity',str(stage),['good'],[],[])),
]:
    write_archive(entries)
    refusal(lambda:prepare.extract(archive,destination,expected),fault)
    assert not destination.exists() and not stage.exists()
    restore()
write_archive([('good',tarfile.REGTYPE,b'a')])
original=archive.read_bytes()
refusal(lambda:prepare.download({'archive-sha256':'0'*64},archive),('archive integrity',str(archive)))
assert archive.read_bytes()==original
restore()
prepare.extract(archive,destination,expected)
(destination/'good').write_bytes(b'x')
refusal(lambda:prepare.extract(archive,destination,expected),('extracted inventory/integrity',str(destination),[],[],[('good',digest,'2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881')]))
assert (destination/'good').read_bytes()==b'x'
(destination/'good').write_bytes(b'a')
assert (destination/'good').read_bytes()==b'a'
prepare.extract(archive,destination,expected)
assert prepare.inventory(destination)==expected
shutil.rmtree(destination)
destination.symlink_to(outside)
refusal(lambda:prepare.extract(archive,destination,expected),('nonregular directory',str(destination)))
assert outside.read_bytes()==b'guard'
destination.unlink()
restore()
for url in ['http://example.invalid/archive','https://relay.fullyjustified.net/default_bundle_v33.tar','https://example.invalid/archive#fragment']:
    refusal(lambda:prepare.url(url),('unpinned URL',url))
print('native preparation fixtures: ok')
";
#[test]
fn skeleton_source_bytes_have_an_exact_boundary_and_restore() {
    use ftoo_manifest::{LocatedString, PathPart::Key, SkeletonDraft, SkeletonSection};
    let prefix =
        "schema=1\nskeleton={document='main',sections=[{heading='sec:fixture:u0',children=[]}]}\n";
    let file = "units/skeletons/bound.toml";
    let site = |keys: &[&str], start, end| UnitSite {
        file: file.into(),
        path: keys.iter().map(|key| Key((*key).into())).collect(),
        anchor: SourceAnchor::Bytes(start, end),
        decoded: None,
        occurrence: None,
    };
    let document = prefix.find("'main'").unwrap();
    let sections = prefix.find("[{").unwrap();
    let sections_end = prefix.find("}]").unwrap() + 2;
    let heading = prefix.find("'sec:fixture:u0'").unwrap();
    let mut heading_site = site(&["skeleton", "sections"], heading, heading + 16);
    heading_site
        .path
        .extend([ftoo_manifest::PathPart::Index(0), Key("heading".into())]);
    let expected = vec![SkeletonDraft {
        document: LocatedString {
            value: "main".into(),
            site: site(&["skeleton", "document"], document, document + 6),
        },
        sections_site: site(&["skeleton", "sections"], sections, sections_end),
        sections: vec![SkeletonSection {
            heading: LocatedString {
                value: "sec:fixture:u0".into(),
                site: heading_site,
            },
            children: Vec::new(),
        }],
    }];
    let source = format!("{prefix}#{}", " ".repeat(1_048_576 - prefix.len() - 1));
    assert_eq!(source.len(), 1_048_576);
    assert_eq!(
        input::skeletons(&[crate::UnitSource {
            file: file.into(),
            bytes: source.as_bytes().to_vec()
        }]),
        Ok(expected.clone())
    );
    let fault = DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M054,
            stage: AssemblyStage::Source,
            document: None,
            unit: None,
            language: None,
            target: None,
            site: AssemblySite::Source(UnitSite {
                file: file.into(),
                path: Vec::new(),
                anchor: SourceAnchor::WholeFile,
                decoded: None,
                occurrence: None,
            }),
            fault: AssemblyFault::Limit {
                resource: ftoo_manifest::AssemblyResource::SourceBytes,
                limit: 1_048_576,
                actual: 1_048_577,
            },
        }])
        .unwrap(),
    );
    assert_eq!(
        input::skeletons(&[crate::UnitSource {
            file: file.into(),
            bytes: format!("{source} ").into_bytes()
        }]),
        Err(fault.clone())
    );
    assert_eq!(
        fault.to_string(),
        "M054 Source document=None unit=None language=None target=None at Source(UnitSite { file: \"units/skeletons/bound.toml\", path: [], anchor: WholeFile, decoded: None, occurrence: None }): Limit { resource: SourceBytes, limit: 1048576, actual: 1048577 }\n"
    );
    assert_eq!(
        input::skeletons(&[crate::UnitSource {
            file: file.into(),
            bytes: prefix.as_bytes().to_vec()
        }]),
        Ok(expected)
    );
}
#[test]
fn skeleton_position_and_heading_depth_limits_have_complete_controls()
-> Result<(), Box<dyn std::error::Error>> {
    use ftoo_manifest::{
        AssemblyInputDiagnostic, AssemblyInputFault, AssemblyResource, LocatedString,
        PathPart::{Index, Key},
        SkeletonChild, SkeletonDraft, SkeletonSection,
    };
    let sources=(0..4096).map(|index|crate::UnitSource {file:format!("units/statements/u{index}.toml"),bytes:format!("schema=1\n[unit]\nid='sec:fixture:u{index}'\ndocument='main'\nmath={{}}\n[unit.texts]\nen='Heading'\n").into_bytes()}).collect();
    let corpus = crate::load_unit_sources(
        crate::UnitSource {
            file: "units/documents.toml".into(),
            bytes: b"schema=1\n[documents.main]\nrequired-languages=['en']\n".to_vec(),
        },
        sources,
    )?;
    let source_site = |file: String, path| UnitSite {
        file,
        path,
        anchor: SourceAnchor::Unavailable,
        decoded: None,
        occurrence: None,
    };
    let metadata = (0..4096)
        .map(|index| {
            (
                format!("sec:fixture:u{index}"),
                source_site(
                    format!("units/statements/u{index}.toml"),
                    vec![Key("unit".into()), Key("id".into())],
                ),
            )
        })
        .collect();
    let declarations = BTreeMap::from([(
        "main".into(),
        source_site(
            "units/documents.toml".into(),
            vec![Key("documents".into()), Key("main".into())],
        ),
    )]);
    let reference = |index, path| LocatedString {
        value: format!("sec:fixture:u{index}"),
        site: source_site("units/skeletons/bound.toml".into(), path),
    };
    let root_path = vec![Key("skeleton".into()), Key("sections".into()), Index(0)];
    let mut heading_path = root_path.clone();
    heading_path.push(Key("heading".into()));
    let heading = reference(0, heading_path);
    let children = (1..4096)
        .map(|index| {
            let mut path = root_path.clone();
            path.extend([Key("children".into()), Index(index - 1), Key("unit".into())]);
            SkeletonChild::Unit(reference(index, path))
        })
        .collect();
    let draft = SkeletonDraft {
        document: LocatedString {
            value: "main".into(),
            site: source_site(
                "units/skeletons/bound.toml".into(),
                vec![Key("skeleton".into()), Key("document".into())],
            ),
        },
        sections_site: source_site(
            "units/skeletons/bound.toml".into(),
            vec![Key("skeleton".into()), Key("sections".into())],
        ),
        sections: vec![SkeletonSection {
            heading: heading.clone(),
            children,
        }],
    };
    let checked = ftoo_manifest::check_skeletons(
        &corpus,
        std::slice::from_ref(&draft),
        &metadata,
        &ownership_sites(&metadata),
        &declarations,
    )
    .map_err(|e| format!("{e:?}"))?;
    let expected = (0..4096)
        .map(|index| (format!("sec:fixture:u{index}"), (index == 0).then_some(1)))
        .collect::<Vec<_>>();
    assert_eq!(
        checked.documents()["main"]
            .iter()
            .map(|position| (position.unit().to_owned(), position.depth()))
            .collect::<Vec<_>>(),
        expected
    );
    let mut excessive = draft.clone();
    let mut path = root_path.clone();
    path.extend([Key("children".into()), Index(4095), Key("unit".into())]);
    let extra = reference(0, path);
    excessive.sections[0]
        .children
        .push(SkeletonChild::Unit(extra.clone()));
    assert_eq!(
        ftoo_manifest::check_skeletons(
            &corpus,
            &[excessive],
            &metadata,
            &ownership_sites(&metadata),
            &declarations
        )
        .unwrap_err(),
        vec![AssemblyInputDiagnostic {
            rule: RuleId::M054,
            stage: AssemblyStage::Ownership,
            document: Some("main".into()),
            unit: Some("sec:fixture:u0".into()),
            language: None,
            target: None,
            site: AssemblySite::Source(extra.site),
            fault: AssemblyInputFault::Limit {
                resource: AssemblyResource::UnitPositions,
                limit: 4096,
                actual: 4097
            }
        }]
    );
    for depth in [6, 7] {
        let mut sections = Vec::new();
        let mut path = root_path.clone();
        for index in 0..depth {
            let mut heading_path = path.clone();
            heading_path.push(Key("heading".into()));
            sections.push(SkeletonSection {
                heading: reference(index, heading_path),
                children: Vec::new(),
            });
            path.extend([Key("children".into()), Index(0)]);
        }
        let last = sections.last().unwrap().heading.clone();
        let mut section = sections.pop().unwrap();
        section.children = (depth..4096)
            .map(|index| {
                SkeletonChild::Unit(reference(
                    index,
                    vec![Key("skeleton".into()), Key("leaf".into()), Index(index)],
                ))
            })
            .collect();
        while let Some(mut parent) = sections.pop() {
            parent.children.push(SkeletonChild::Section(section));
            section = parent;
        }
        let mut nested = draft.clone();
        nested.sections = vec![section];
        let result = ftoo_manifest::check_skeletons(
            &corpus,
            &[nested],
            &metadata,
            &ownership_sites(&metadata),
            &declarations,
        );
        if depth == 6 {
            let checked = result.map_err(|e| format!("{e:?}"))?;
            let expected = (0..4096)
                .map(|index| {
                    (
                        format!("sec:fixture:u{index}"),
                        (index < 6).then_some(index + 1),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(
                checked.documents()["main"]
                    .iter()
                    .map(|position| (position.unit().to_owned(), position.depth()))
                    .collect::<Vec<_>>(),
                expected
            );
        } else {
            assert_eq!(
                result.unwrap_err(),
                vec![AssemblyInputDiagnostic {
                    rule: RuleId::M054,
                    stage: AssemblyStage::Ownership,
                    document: Some("main".into()),
                    unit: Some("sec:fixture:u6".into()),
                    language: None,
                    target: None,
                    site: AssemblySite::Source(last.site),
                    fault: AssemblyInputFault::Limit {
                        resource: AssemblyResource::SkeletonDepth,
                        limit: 6,
                        actual: 7
                    }
                }]
            );
        }
    }
    assert_eq!(
        ftoo_manifest::check_skeletons(
            &corpus,
            &[draft],
            &metadata,
            &ownership_sites(&metadata),
            &declarations
        )
        .map_err(|e| format!("{e:?}"))?
        .documents()["main"]
            .iter()
            .map(|position| (position.unit().to_owned(), position.depth()))
            .collect::<Vec<_>>(),
        expected
    );
    Ok(())
}
fn expected_runtime_refusal(source: &str, input: &str, message: &str) -> DocumentError {
    let start = source.find("\\FtdocTag{").unwrap();
    let end = start + source[start..].find("{u0-t0}").unwrap() + "{u0-t0}".len();
    DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Native,
            document: Some("fixture".into()),
            unit: None,
            language: Some("en".into()),
            target: Some(DocumentTarget::Latex),
            site: AssemblySite::Generated {
                path: "generated/documents/fixture/en.tex".into(),
                bytes: start..end,
                origins: Vec::new(),
            },
            fault: AssemblyFault::RuntimeTagRejected {
                input: input.into(),
                site_token: "u0-t0".into(),
                message: message.into(),
            },
        }])
        .unwrap(),
    )
}
fn support_required_native_unexpanded_live_lookup_is_refused_and_restores(
    support: &NativeSupport,
    lock: &native_support::Lock,
    checks: &mut Vec<CaseEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let original = runtime::render(&book, &[])?;
    let changed = original.replace(
        r"\prop_get:NVNTF \g_ftdoc_live_prop",
        r"\prop_get:NnNTF \g_ftdoc_live_prop",
    );
    assert_ne!(changed, original);
    let mut source = latex::start("fixture", "en");
    source.push_str("\\def\\handle{c:1}\n\\FtdocTag{\\handle}{u0-t0}\n\\end{document}\n");
    let case = NativeCase::Document {
        document: "fixture".into(),
        language: "en".into(),
    };
    let expected = expected_runtime_refusal(&source, "c:1", "tag \"c:1\": unknown");
    checks.push(CaseEvidence::new(
        "lookup/refusal",
        native::consume(
            support,
            lock,
            &source,
            "",
            &changed,
            case.clone(),
            Vec::new(),
        ),
        Err(expected),
    ));
    assert_eq!(runtime::render(&book, &[])?, original);
    checks.push(CaseEvidence::new(
        "lookup/restored",
        native::consume(support, lock, &source, "", &original, case, Vec::new()),
        Ok(()),
    ));
    Ok(())
}
#[test]
fn complete_native_inventory_distinguishes_definition_and_primitive_owners()
-> Result<(), Box<dyn std::error::Error>> {
    use ftoo_manifest::{DefinitionId as D, PrimitiveId as P, RenderOwner};
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let catalogue = ftoo_manifest::notation(&book)?;
    let mut rows = Vec::new();
    for (id, cases) in [
        (D::CoefficientOne, vec!["base"]),
        (D::CoefficientThree, vec!["base"]),
        (D::CoefficientOneValue, vec!["base"]),
        (D::CoefficientThreeValue, vec!["base"]),
        (D::FirstCoefficient, vec!["base"]),
        (
            D::Dimension,
            vec![
                "base",
                "compound",
                "slot-0-alpha",
                "slot-0-pending",
                "slot-0-nested-dimension",
                "slot-0-nested-product",
            ],
        ),
        (
            D::Product,
            vec![
                "base",
                "compound",
                "slot-0-alpha",
                "slot-0-pending",
                "slot-0-nested-dimension",
                "slot-0-nested-product",
                "slot-1-alpha",
                "slot-1-pending",
                "slot-1-nested-dimension",
                "slot-1-nested-product",
            ],
        ),
    ] {
        rows.extend(cases.into_iter().map(|case| NativeCase::Sweep {
            owner: RenderOwner::Definition(id),
            case: case.into(),
        }));
    }
    assert_eq!(rows.len(), 21);
    assert!(rows.iter().all(|row| matches!(
        row,
        NativeCase::Sweep {
            owner: RenderOwner::Definition(_),
            ..
        }
    )));
    for id in [
        D::Comovfield,
        D::Willvirtue,
        D::Gensym,
        D::Alignrate,
        D::Willnet,
        D::Willdecay,
    ] {
        rows.push(NativeCase::Sweep {
            owner: RenderOwner::Definition(id),
            case: "base".into(),
        });
    }
    let primitives = [
        (P::Alpha, 0),
        (P::Beta, 0),
        (P::Fraction, 2),
        (P::Subscript, 2),
        (P::Superscript, 2),
        (P::Add, 2),
        (P::Equal, 2),
        (P::Negate, 1),
        (P::Multiply, 2),
        (P::Juxtapose, 2),
        (P::Parentheses, 1),
        (P::Dimension, 1),
        (P::Omega, 0),
        (P::Calligraphic, 1),
        (P::UprightVir, 0),
        (P::UprightS, 0),
        (P::Dot, 1),
        (P::Sigma, 0),
        (P::Gamma, 0),
        (P::Times, 2),
        (P::Subtract, 2),
        (P::TightJoin, 2),
        (P::Underbrace, 2),
        (P::UprightNet, 0),
    ];
    for (id, arity) in primitives {
        rows.push(NativeCase::Sweep {
            owner: RenderOwner::Primitive(id),
            case: "base".into(),
        });
        for argument in 0..arity {
            for token in [
                "alpha",
                "beta",
                "fraction",
                "subscript",
                "superscript",
                "add",
                "equal",
                "negate",
                "multiply",
                "juxtapose",
                "parentheses",
                "dimension",
                "omega",
                "calligraphic",
                "upright-vir",
                "upright-s",
                "dot",
                "sigma",
                "gamma",
                "times",
                "subtract",
                "tight-join",
                "underbrace",
                "upright-net",
            ] {
                rows.push(NativeCase::Sweep {
                    owner: RenderOwner::Primitive(id),
                    case: format!("argument-{argument}-primitive-{token}"),
                });
            }
        }
    }
    assert_eq!(rows.len(), 699);
    assert_eq!(native::sweep_inventory(&catalogue, &rows), Ok(()));
    let missing = NativeCase::Sweep {
        owner: RenderOwner::Definition(D::Dimension),
        case: "base".into(),
    };
    let repeated = NativeCase::Sweep {
        owner: RenderOwner::Primitive(P::Dimension),
        case: "base".into(),
    };
    let unexpected = NativeCase::Sweep {
        owner: RenderOwner::Primitive(P::Dimension),
        case: "compound".into(),
    };
    let mut changed = rows.clone();
    changed.retain(|row| row != &missing);
    changed.push(repeated.clone());
    changed.push(unexpected.clone());
    let expected = DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M056,
            stage: AssemblyStage::Native,
            document: None,
            unit: None,
            language: None,
            target: None,
            site: AssemblySite::Inventory {
                path: "assembly-sweep.tex".into(),
            },
            fault: AssemblyFault::NativeInventory {
                missing: vec![missing],
                repeated: vec![(repeated, 2)],
                unexpected: vec![unexpected],
            },
        }])
        .unwrap(),
    );
    assert_eq!(
        native::sweep_inventory(&catalogue, &changed),
        Err(expected.clone())
    );
    assert_eq!(
        expected.to_string(),
        "M056 Native document=None unit=None language=None target=None at Inventory { path: \"assembly-sweep.tex\" }: NativeInventory { missing: [Sweep { owner: Definition(Dimension), case: \"base\" }], repeated: [(Sweep { owner: Primitive(Dimension), case: \"base\" }, 2)], unexpected: [Sweep { owner: Primitive(Dimension), case: \"compound\" }] }\n"
    );
    assert_eq!(native::sweep_inventory(&catalogue, &rows), Ok(()));
    Ok(())
}
#[test]
fn an_absent_native_environment_never_uses_path_fallback() -> Result<(), Box<dyn std::error::Error>>
{
    if std::env::var_os("FTOO_ASSEMBLY_ENV_CHILD").is_some() {
        let expected = NativeAdapterFault::Configuration {
            component: None,
            path: None,
            reason: NativeConfigurationReason::Missing,
        };
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(
            NativeSupport::from_environment(&root).unwrap_err(),
            expected
        );
        assert_eq!(
            DocumentError::Native(expected).to_string(),
            "Native(Configuration { component: None, path: None, reason: Missing })"
        );
        return Ok(());
    }
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "documents::tests::an_absent_native_environment_never_uses_path_fallback",
        ])
        .env_remove("FTOO_ASSEMBLY_SUPPORT")
        .env("FTOO_ASSEMBLY_ENV_CHILD", "1")
        .env("PATH", "/no-native-fallback")
        .output()?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    Ok(())
}
#[test]
fn skeleton_ownership_keeps_heading_and_leaf_multiplicities()
-> Result<(), Box<dyn std::error::Error>> {
    use ftoo_manifest::{
        AssemblyInputDiagnostic, AssemblyInputFault, LocatedString,
        PathPart::{Index, Key},
        SkeletonChild, SkeletonDraft, SkeletonSection,
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let corpus = crate::load_unit_sources(
        crate::UnitSource {
            file: "units/documents.toml".into(),
            bytes: b"schema = 1\n[documents.main]\nrequired-languages = [\"en\"]\n".to_vec(),
        },
        input::sources(
            &root,
            &[
                "units/statements/first-value.toml".into(),
                "units/statements/overview.toml".into(),
                "units/statements/value-context.toml".into(),
                "units/statements/value-table.toml".into(),
            ],
        )?,
    )?;
    let site = |file: &str, path: Vec<ftoo_manifest::PathPart>| UnitSite {
        file: file.into(),
        path,
        anchor: SourceAnchor::Unavailable,
        decoded: None,
        occurrence: None,
    };
    let reference = |value: &str, index| LocatedString {
        value: value.into(),
        site: site(
            "units/skeletons/private.toml",
            vec![
                Key("skeleton".into()),
                Key("sections".into()),
                Index(0),
                Key("children".into()),
                Index(index),
                Key("unit".into()),
            ],
        ),
    };
    let first = reference("sec:coefficients:first-value", 0);
    let context = reference("sec:coefficients:value-context", 1);
    let table = reference("sec:coefficients:value-table", 2);
    let heading = LocatedString {
        value: "sec:coefficients:overview".into(),
        site: site(
            "units/skeletons/private.toml",
            vec![
                Key("skeleton".into()),
                Key("sections".into()),
                Index(0),
                Key("heading".into()),
            ],
        ),
    };
    let draft = SkeletonDraft {
        document: LocatedString {
            value: "main".into(),
            site: site(
                "units/skeletons/private.toml",
                vec![Key("skeleton".into()), Key("document".into())],
            ),
        },
        sections_site: site(
            "units/skeletons/private.toml",
            vec![Key("skeleton".into()), Key("sections".into())],
        ),
        sections: vec![SkeletonSection {
            heading: heading.clone(),
            children: vec![
                SkeletonChild::Unit(first.clone()),
                SkeletonChild::Unit(context.clone()),
                SkeletonChild::Unit(table.clone()),
            ],
        }],
    };
    let unit_sites = BTreeMap::from([
        (
            "sec:coefficients:first-value".into(),
            site(
                "units/statements/first-value.toml",
                vec![Key("unit".into()), Key("id".into())],
            ),
        ),
        (
            "sec:coefficients:overview".into(),
            site(
                "units/statements/overview.toml",
                vec![Key("unit".into()), Key("id".into())],
            ),
        ),
        (
            "sec:coefficients:value-context".into(),
            site(
                "units/statements/value-context.toml",
                vec![Key("unit".into()), Key("id".into())],
            ),
        ),
        (
            "sec:coefficients:value-table".into(),
            site(
                "units/statements/value-table.toml",
                vec![Key("unit".into()), Key("id".into())],
            ),
        ),
    ]);
    let documents = BTreeMap::from([(
        "main".into(),
        site(
            "units/documents.toml",
            vec![Key("documents".into()), Key("main".into())],
        ),
    )]);
    let expected_positions = vec![
        ("sec:coefficients:overview", Some(1), heading.site.clone()),
        ("sec:coefficients:first-value", None, first.site.clone()),
        ("sec:coefficients:value-context", None, context.site.clone()),
        ("sec:coefficients:value-table", None, table.site.clone()),
    ];
    let checked = ftoo_manifest::check_skeletons(
        &corpus,
        std::slice::from_ref(&draft),
        &unit_sites,
        &ownership_sites(&unit_sites),
        &documents,
    )
    .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        checked.documents()["main"]
            .iter()
            .map(|p| (p.unit(), p.depth(), p.site().clone()))
            .collect::<Vec<_>>(),
        expected_positions
    );
    for duplicate in [false, true] {
        let mut changed = draft.clone();
        let mut sites = Vec::new();
        let primary = if duplicate {
            let repeated = reference("sec:coefficients:first-value", 3);
            sites = vec![first.site.clone(), repeated.site.clone()];
            changed.sections[0]
                .children
                .push(SkeletonChild::Unit(repeated));
            first.site.clone()
        } else {
            changed.sections[0].children.remove(0);
            unit_sites["sec:coefficients:first-value"].clone()
        };
        let expected = AssemblyInputDiagnostic {
            rule: RuleId::M054,
            stage: AssemblyStage::Ownership,
            document: Some("main".into()),
            unit: Some("sec:coefficients:first-value".into()),
            language: None,
            target: None,
            site: AssemblySite::Source(primary),
            fault: AssemblyInputFault::UnitMultiplicity {
                label: "sec:coefficients:first-value".into(),
                expected: 1,
                actual: if duplicate { 2 } else { 0 },
                sites,
            },
        };
        let actual = ftoo_manifest::check_skeletons(
            &corpus,
            &[changed],
            &unit_sites,
            &ownership_sites(&unit_sites),
            &documents,
        )
        .unwrap_err();
        assert_eq!(actual, std::slice::from_ref(&expected));
        assert_eq!(
            actual[0].to_string(),
            format!(
                "M054 Ownership document=Some(\"main\") unit=Some(\"sec:coefficients:first-value\") language=None target=None at {:?}: {:?}",
                expected.site, expected.fault
            )
        );
        let restored = ftoo_manifest::check_skeletons(
            &corpus,
            std::slice::from_ref(&draft),
            &unit_sites,
            &ownership_sites(&unit_sites),
            &documents,
        )
        .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            restored.documents()["main"]
                .iter()
                .map(|p| (p.unit(), p.depth(), p.site().clone()))
                .collect::<Vec<_>>(),
            expected_positions
        );
    }
    Ok(())
}

fn declared_positions(section: &ftoo_manifest::SkeletonSection, out: &mut Vec<(String, bool)>) {
    out.push((section.heading.value.clone(), false));
    for child in &section.children {
        match child {
            ftoo_manifest::SkeletonChild::Unit(unit)
            | ftoo_manifest::SkeletonChild::Paragraph(unit)
            | ftoo_manifest::SkeletonChild::Equation(unit) => {
                out.push((unit.value.clone(), false));
            }
            ftoo_manifest::SkeletonChild::Quotation(unit) => {
                out.push((unit.value.clone(), true));
            }
            ftoo_manifest::SkeletonChild::Section(section) => {
                declared_positions(section, out);
            }
        }
    }
}

#[test]
fn quotation_use_requires_an_existing_unit_owned_by_another_document()
-> Result<(), Box<dyn std::error::Error>> {
    use ftoo_manifest::{AssemblyInputFault, SkeletonChild};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let declaration = crate::UnitSource {
        file: "units/documents.toml".into(),
        bytes: input::read(&root, "units/documents.toml")?,
    };
    let statements = input::sources(&root, &input::discover(&root, "units/statements", false)?)?;
    let corpus = crate::load_unit_sources(declaration.clone(), statements.clone())?;
    let skeleton_sources =
        input::sources(&root, &input::discover(&root, "units/skeletons", false)?)?;
    let mut drafts = input::skeletons(&skeleton_sources)?;
    let (unit_sites, owner_sites) = super::metadata(&statements);
    let document_sites = super::document_sites(&corpus);
    let checked = ftoo_manifest::check_skeletons(
        &corpus,
        &drafts,
        &unit_sites,
        &owner_sites,
        &document_sites,
    )
    .map_err(|errors| format!("{errors:?}"))?;
    for draft in &drafts {
        let mut expected = Vec::new();
        for section in &draft.sections {
            declared_positions(section, &mut expected);
        }
        assert_eq!(
            checked.documents()[&draft.document.value]
                .iter()
                .map(|position| (position.unit().to_owned(), position.quotation()))
                .collect::<Vec<_>>(),
            expected
        );
    }
    let selected = crate::units::toml::selected_targets(&declaration)
        .map_err(|errors| format!("{errors:?}"))?;
    let mut checked_fragments = 0;
    for (document, positions) in checked.documents() {
        if !selected[document].contains(&DocumentOutput::Fragment) {
            continue;
        }
        for language in &corpus.documents()[document] {
            for position in positions.iter().filter(|position| {
                position.depth().is_none() && !position.paragraph() && !position.equation()
            }) {
                let unit = corpus
                    .units()
                    .iter()
                    .find(|unit| unit.label().as_str() == position.unit())
                    .unwrap();
                let block = plan::Block {
                    pending_names: BTreeMap::new(),
                    displays: BTreeMap::new(),
                    unit: position.unit().into(),
                    depth: position.depth(),
                    quotation: position.quotation(),
                    paragraph: position.paragraph(),
                    equation: position.equation(),
                    site: position.site().clone(),
                    runs: vec![plan::Inline::Text(unit.texts()[language].text().into())],
                };
                let path = format!(
                    "generated/documents/{}/{}.tex",
                    document,
                    position.unit().replace(':', "-")
                );
                assert_eq!(
                    input::read(&root, &path)?,
                    fragment_line(&block)?.into_bytes()
                );
                checked_fragments += 1;
            }
        }
    }
    assert!(checked_fragments > 0);
    let closing = "The record can watch the hand's tremor turning. It cannot see what the righting is paid in.";
    for (document, macro_name) in [
        ("paper-pilot", "refrain"),
        ("explainer-pilot", "paperrefrain"),
    ] {
        let path = format!("generated/documents/{document}/sec-coda-record-limit.tex");
        assert_eq!(
            input::read(&root, &path)?,
            format!("\\{macro_name}{{sec:coda:record-limit}}{{{closing}}}\n").into_bytes(),
        );
        let annotated = input::read(
            &root,
            &format!("generated/documents/{document}/en.labels.md"),
        )?;
        let reader = input::read(&root, &format!("generated/documents/{document}/en.md"))?;
        let annotated = std::str::from_utf8(&annotated)?;
        assert_eq!(
            markdown::erase_annotations(annotated),
            Ok(String::from_utf8(reader.clone())?)
        );
        assert!(std::str::from_utf8(&reader)?.contains("hand’s tremor"));
        assert!(!std::str::from_utf8(&reader)?.contains("hand&#39;s"));
    }
    let mut multilingual_declaration = input::read(&root, "units/documents.toml")?;
    let declaration_text = String::from_utf8(multilingual_declaration)?;
    multilingual_declaration = declaration_text
        .replacen(
            "[documents.explainer-pilot]\nrequired-languages = [\"en\"]",
            "[documents.explainer-pilot]\nrequired-languages = [\"en\", \"fr\"]",
            1,
        )
        .into_bytes();
    let mut multilingual_statements = statements.clone();
    multilingual_statements
        .iter_mut()
        .find(|source| source.file.ends_with("pilot-explainer-heading.toml"))
        .unwrap()
        .bytes
        .extend_from_slice(b"fr = \"Extrait explicatif\"\n");
    multilingual_statements
        .iter_mut()
        .find(|source| source.file.ends_with("plain-intro.toml"))
        .unwrap()
        .bytes
        .extend_from_slice(b"fr = \"Ce paragraphe presente la theorie.\"\n");
    multilingual_statements
        .iter_mut()
        .find(|source| source.file.ends_with("explainer-overlap-flow.toml"))
        .unwrap()
        .bytes
        .extend_from_slice(b"fr = '@math{flow}.'\n");
    let multilingual = crate::load_unit_sources(
        crate::UnitSource {
            file: "units/documents.toml".into(),
            bytes: multilingual_declaration,
        },
        multilingual_statements,
    )?;
    let language_errors = ftoo_manifest::check_skeletons(
        &multilingual,
        &drafts,
        &unit_sites,
        &owner_sites,
        &document_sites,
    )
    .unwrap_err();
    assert!(language_errors.iter().any(|error| matches!(
        &error.fault, AssemblyInputFault::MissingQuotationLanguage { label, language }
        if label == "sec:say:direction" && language == "fr"
    )));
    let explainer = drafts
        .iter_mut()
        .find(|draft| draft.document.value == "explainer-pilot")
        .unwrap();
    let Some(SkeletonChild::Quotation(quoted)) = explainer.sections[0]
        .children
        .iter_mut()
        .find(|child| matches!(child, SkeletonChild::Quotation(_)))
    else {
        panic!("quotation fixture")
    };
    quoted.value = "sec:missing:quotation".into();
    let errors = ftoo_manifest::check_skeletons(
        &corpus,
        &drafts,
        &unit_sites,
        &owner_sites,
        &document_sites,
    )
    .unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(
        matches!(&errors[0].fault, AssemblyInputFault::UnknownUnit { label, .. } if label == "sec:missing:quotation")
    );
    let explainer = drafts
        .iter_mut()
        .find(|draft| draft.document.value == "explainer-pilot")
        .unwrap();
    let Some(SkeletonChild::Quotation(quoted)) = explainer.sections[0]
        .children
        .iter_mut()
        .find(|child| matches!(child, SkeletonChild::Quotation(_)))
    else {
        panic!("quotation fixture")
    };
    quoted.value = "sec:pilot:explainer-excerpt".into();
    let errors = ftoo_manifest::check_skeletons(
        &corpus,
        &drafts,
        &unit_sites,
        &owner_sites,
        &document_sites,
    )
    .unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(
        matches!(&errors[0].fault, AssemblyInputFault::WrongQuotationOwner { label, .. } if label == "sec:pilot:explainer-excerpt")
    );
    Ok(())
}
#[test]
fn typography_apostrophe_contexts() {
    let site = input::site("units/statements/typography.toml");
    let block = |unit: &str, depth, text: &str| plan::Block {
        pending_names: BTreeMap::new(),
        displays: BTreeMap::new(),
        unit: unit.into(),
        depth,
        quotation: false,
        paragraph: false,
        equation: false,
        site: site.clone(),
        runs: vec![plan::Inline::Text(text.into())],
    };
    let mut document = plan::DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: plan::Coverage::fixture("Coverage.", "Slice."),
        screen: None,
        blocks: vec![
            block("sec:fixture:heading", Some(1), "The hand’s turn"),
            block(
                "sec:fixture:body",
                None,
                "The hand’s tremor and the hand's turn.",
            ),
        ],
    };
    document.coverage.declaration = "Coverage’s claim.".into();
    document.blocks[1].runs.push(plan::Inline::Mention {
        text: "Reader's code".into(),
        site: site.clone(),
    });
    document.blocks[1].runs.push(plan::Inline::Tag {
        handle: "O'Neil".into(),
        label: "def:fixture:person".into(),
        token: "u0-t0".into(),
        site: site.clone(),
    });
    document.blocks[1]
        .runs
        .push(plan::Inline::Text("’s turn.".into()));
    document.blocks[1].displays.insert(
        DocumentTarget::Latex,
        plan::DisplayBody {
            source: "one-cell table".into(),
            layout: ftoo_manifest::DisplayLayout::Table {
                columns: vec!["l".into()],
                rows: vec![vec![0]],
                rules: vec![],
            },
            regions: vec![block("sec:fixture:body", None, "The hand’s cell.")],
            site: site.clone(),
        },
    );
    let emitted = latex::render(&document);
    assert!(emitted.contains("\\section*{The hand's turn}"));
    assert!(emitted.contains("The hand's tremor and the hand's turn."));
    assert!(emitted.contains("The hand's cell."));
    assert!(emitted.contains("Coverage’s claim."));
    assert!(emitted.contains("\\texttt{Reader's code}"));
    assert!(emitted.contains("% label def:fixture:person\n's turn."));
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let mut pending_block = block("sec:fixture:pending", None, "");
    pending_block.runs = vec![plan::Inline::Math {
        definition_site: site.clone(),
        key: "value".into(),
        markdown: "x’s".into(),
        latex: "x’s".into(),
        pending: vec![crate::PendingEntry::fixture(
            ftoo_manifest::DefinitionId::CoefficientOne,
            0,
            "x’s",
            vec![ftoo_manifest::PendingReason::FormulationProposed {
                object: ftoo_manifest::NodeId::LangevinCoefficientOne,
            }],
            vec![],
        )],
        site: site.clone(),
    }];
    pending_block
        .map_pending(&book, 0, "fixture", "en")
        .unwrap();
    let pending_wording = super::pending::paragraph(&pending_block, "en", DocumentTarget::Latex);
    assert!(pending_wording.contains("\\(x’s\\)"));
    assert_eq!(
        plan::check_emission(&document, &emitted, DocumentTarget::Latex),
        Ok(())
    );
    let altered = emitted.replacen("hand's turn", "handx turn", 1);
    let fault = plan::check_emission(&document, &altered, DocumentTarget::Latex).unwrap_err();
    let DocumentError::Assembly(errors) = fault else {
        panic!("prose recovery refusal");
    };
    let diagnostic = &errors.diagnostics()[0];
    let AssemblySite::Generated { path, bytes, .. } = &diagnostic.site else {
        panic!("generated prose site");
    };
    assert_eq!(path, "generated/documents/fixture/en.tex");
    let start = altered.find("% unit sec:fixture:heading").unwrap();
    let end = altered.find("% unit sec:fixture:body").unwrap();
    assert_eq!(*bytes, start..end);
    let AssemblyFault::EmissionMismatch {
        missing,
        unexpected,
        changed,
    } = &diagnostic.fault
    else {
        panic!("prose emission mismatch");
    };
    assert_eq!(missing.as_slice(), []);
    assert_eq!(unexpected.as_slice(), []);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].expected.kind, EmissionKind::Heading);
    assert_eq!(changed[0].expected.site, site);
    assert_eq!(changed[0].expected.value, "The hand’s turn");
    assert_eq!(changed[0].actual.value, "The handx turn");
}

#[test]
fn fragment_macro_follows_planned_quotation() {
    let mut block = plan::Block {
        pending_names: BTreeMap::new(),
        displays: BTreeMap::new(),
        unit: "sec:fixture:refrain".into(),
        depth: None,
        quotation: false,
        paragraph: false,
        equation: false,
        site: input::site("units/statements/fixture.toml"),
        runs: vec![plan::Inline::Text("Refrain.".into())],
    };
    assert_eq!(fragment_macro(&block), "refrain");
    block.quotation = true;
    assert_eq!(fragment_macro(&block), "paperrefrain");
    assert_eq!(fragment_text(&block).unwrap(), "Refrain.");
    for source in ["A { brace", "A } brace", "A ^ mark", "A ~ mark"] {
        block.runs = vec![plan::Inline::Text(source.into())];
        assert_eq!(
            fragment_text(&block),
            Err(failure(
                RuleId::M055,
                AssemblyStage::Emission,
                AssemblySite::Source(block.site.clone()),
                AssemblyFault::UnknownToken {
                    value: block.unit.clone(),
                    expected: vec!["brace-free plain refrain text".into()],
                },
            ))
        );
    }
    block.runs = vec![plan::Inline::Text("Refrain.".into())];
    block.runs.push(plan::Inline::Text("More.".into()));
    assert!(fragment_text(&block).is_err());
    let mut plan = plan::DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: plan::Coverage::fixture("Coverage.", "Slice."),
        screen: None,
        blocks: vec![block],
    };
    assert!(super::html::prose_only(&plan).is_err());
    assert!(refuse_quoted_screen_targets(&plan, &[DocumentOutput::Fragment]).is_ok());
    plan.screen = Some(plan::ScreenPlan {
        gaps: vec![],
        refrains: vec![],
        index: vec![],
        html_gaps: std::collections::BTreeMap::new(),
        end_matter: vec![],
    });
    assert_eq!(
        refuse_quoted_screen_targets(&plan, &[DocumentOutput::Markdown]),
        Ok(())
    );
    assert_eq!(
        refuse_quoted_screen_targets(&plan, &[DocumentOutput::Latex]),
        Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(plan.blocks[0].site.clone()),
            AssemblyFault::UnknownToken {
                value: "sec:fixture:refrain".into(),
                expected: vec!["quotation emitter".into()],
            },
        ))
    );
    plan.screen = None;
    assert_eq!(
        refuse_quoted_screen_targets(&plan, &[DocumentOutput::Markdown]),
        Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(plan.blocks[0].site.clone()),
            AssemblyFault::UnknownToken {
                value: "sec:fixture:refrain".into(),
                expected: vec!["quotation emitter".into()],
            },
        ))
    );
}
#[test]
fn fragment_path_collisions_refuse_before_publication() {
    let artifact = DocumentArtifact {
        path: "generated/documents/main/sec-a-b-c.tex".into(),
        bytes: b"one".to_vec(),
        origins: Vec::new(),
        emissions: Vec::new(),
        html_gap_sites: Vec::new(),
    };
    assert!(unique_artifact_paths(&[artifact.clone(), artifact]).is_err());
}
#[test]
fn wrong_owner_retains_the_actual_document_literal_and_restores()
-> Result<(), Box<dyn std::error::Error>> {
    use ftoo_manifest::{
        AssemblyInputDiagnostic, AssemblyInputFault, LocatedString, PathPart::Key, SkeletonDraft,
        SkeletonSection,
    };
    let statements: Vec<_> = [("first", "main"), ("second", "explainer")].into_iter().map(|(unit, document)| crate::UnitSource {
        file: format!("units/statements/{unit}.toml"),
        bytes: format!("schema=1\n[unit]\nid='sec:fixture:{unit}'\ndocument='{document}'\nmath={{}}\n[unit.texts]\nen='Heading'\n").into_bytes(),
    }).collect();
    let corpus = crate::load_unit_sources(crate::UnitSource {
        file: "units/documents.toml".into(),
        bytes: b"schema=1\n[documents.main]\nrequired-languages=['en']\n[documents.explainer]\nrequired-languages=['en']\n".to_vec(),
    }, statements.clone())?;
    let (metadata, owners) = super::metadata(&statements);
    let declarations = super::document_sites(&corpus);
    let mut drafts: Vec<_> = [("first", "main"), ("second", "explainer")]
        .into_iter()
        .map(|(unit, document)| {
            let site = input::site(&format!("units/skeletons/{document}.toml"));
            SkeletonDraft {
                document: LocatedString {
                    value: document.into(),
                    site: site.clone(),
                },
                sections_site: site.clone(),
                sections: vec![SkeletonSection {
                    heading: LocatedString {
                        value: format!("sec:fixture:{unit}"),
                        site,
                    },
                    children: Vec::new(),
                }],
            }
        })
        .collect();
    let valid = ftoo_manifest::check_skeletons(&corpus, &drafts, &metadata, &owners, &declarations)
        .map_err(|e| format!("{e:?}"))?;
    let expected_documents = valid.documents().clone();
    for (statement, draft) in statements.iter().zip(&drafts) {
        let source = std::str::from_utf8(&statement.bytes)?;
        let begin = source.find("document='").unwrap() + "document=".len();
        let end = begin + draft.document.value.len() + 2;
        assert_eq!(
            owners[&draft.sections[0].heading.value],
            UnitSite {
                file: statement.file.clone(),
                path: vec![Key("unit".into()), Key("document".into())],
                anchor: SourceAnchor::Bytes(begin, end),
                decoded: None,
                occurrence: None,
            }
        );
    }
    drafts[0].document.value = "explainer".into();
    drafts[1].document.value = "main".into();
    let expected: Vec<_> = [(0, "main", "explainer"), (1, "explainer", "main")]
        .into_iter()
        .map(|(index, expected, actual)| {
            let heading = &drafts[index].sections[0].heading;
            AssemblyInputDiagnostic {
                rule: RuleId::M054,
                stage: AssemblyStage::Ownership,
                document: Some(actual.into()),
                unit: Some(heading.value.clone()),
                language: None,
                target: None,
                site: AssemblySite::Source(heading.site.clone()),
                fault: AssemblyInputFault::WrongOwner {
                    label: heading.value.clone(),
                    expected: expected.into(),
                    actual: actual.into(),
                    declared_at: owners[&heading.value].clone(),
                },
            }
        })
        .collect();
    let actual =
        ftoo_manifest::check_skeletons(&corpus, &drafts, &metadata, &owners, &declarations)
            .unwrap_err();
    assert_eq!(actual, expected);
    for diagnostic in &actual {
        assert_eq!(
            diagnostic.to_string(),
            format!(
                "M054 Ownership document={:?} unit={:?} language=None target=None at {:?}: {:?}",
                diagnostic.document, diagnostic.unit, diagnostic.site, diagnostic.fault
            )
        );
    }
    drafts[0].document.value = "main".into();
    drafts[1].document.value = "explainer".into();
    let restored =
        ftoo_manifest::check_skeletons(&corpus, &drafts, &metadata, &owners, &declarations)
            .map_err(|e| format!("{e:?}"))?;
    assert_eq!(restored.documents(), &expected_documents);
    assert_eq!(super::metadata(&statements), (metadata, owners));
    Ok(())
}
#[test]
fn unknown_skeleton_kind_keeps_decoded_and_raw_literal_coordinates() {
    use ftoo_manifest::PathPart::{Index, Key};
    let text = r#"schema=1
skeleton={document='main',sections=[{heading='sec:fixture:first',children=[{kind="\u006Fther",unit='sec:fixture:second'}]}]}
"#;
    let begin = text.find('"').unwrap();
    let site = UnitSite {
        file: "units/skeletons/escaped.toml".into(),
        path: vec![
            Key("skeleton".into()),
            Key("sections".into()),
            Index(0),
            Key("children".into()),
            Index(0),
            Key("kind".into()),
        ],
        anchor: SourceAnchor::Bytes(begin, begin + 12),
        decoded: Some(0..5),
        occurrence: None,
    };
    let expected = DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M004,
            stage: AssemblyStage::Shape,
            document: None,
            unit: None,
            language: None,
            target: None,
            site: AssemblySite::Source(site),
            fault: AssemblyFault::UnknownToken {
                value: "other".into(),
                expected: vec![
                    "unit".into(),
                    "paragraph".into(),
                    "equation".into(),
                    "quotation".into(),
                    "section".into(),
                ],
            },
        }])
        .unwrap(),
    );
    let source = crate::UnitSource {
        file: "units/skeletons/escaped.toml".into(),
        bytes: text.as_bytes().to_vec(),
    };
    let actual = input::skeletons(&[source]).unwrap_err();
    assert_eq!(actual, expected);
    assert_eq!(actual.to_string(), expected.to_string());
    let restored = text.replace(r"\u006Fther", "unit");
    let checked = input::skeletons(&[crate::UnitSource {
        file: "units/skeletons/escaped.toml".into(),
        bytes: restored.as_bytes().to_vec(),
    }])
    .unwrap();
    assert_eq!(checked.len(), 1);
    assert_eq!(checked[0].sections[0].children.len(), 1);
    assert_eq!(
        restored.replace("kind=\"unit\"", "kind=\"\\u006Fther\""),
        text
    );
}
fn ownership_sites(metadata: &BTreeMap<String, UnitSite>) -> BTreeMap<String, UnitSite> {
    metadata
        .iter()
        .map(|(label, site)| {
            let mut owner = site.clone();
            owner.path.pop();
            owner
                .path
                .push(ftoo_manifest::PathPart::Key("document".into()));
            (label.clone(), owner)
        })
        .collect()
}
fn skeleton_boundary(
    source: &str,
    path: Vec<ftoo_manifest::PathPart>,
    token: &str,
    rule: RuleId,
    stage: ftoo_manifest::UnitStage,
    fault: ftoo_manifest::UnitFault,
) {
    use std::fmt::Write;
    let start = source.find(token).unwrap();
    let mut display =
        format!("{rule:?} {stage:?} unit=None language=None at \"units/skeletons/test.toml\" $");
    for part in &path {
        match part {
            ftoo_manifest::PathPart::Key(key) => write!(display, "[{key:?}]"),
            ftoo_manifest::PathPart::Index(index) => write!(display, "[{index}]"),
        }
        .unwrap();
    }
    writeln!(
        display,
        " bytes {start}..{}: {fault:?}",
        start + token.len()
    )
    .unwrap();
    let expected = DocumentError::Boundary(
        ftoo_manifest::UnitErrors::from_diagnostics(vec![ftoo_manifest::UnitDiagnostic {
            rule,
            stage,
            unit: None,
            language: None,
            site: UnitSite {
                file: "units/skeletons/test.toml".into(),
                path,
                anchor: SourceAnchor::Bytes(start, start + token.len()),
                decoded: None,
                occurrence: None,
            },
            fault,
        }])
        .unwrap(),
    );
    let actual = input::skeletons(&[crate::UnitSource {
        file: "units/skeletons/test.toml".into(),
        bytes: source.as_bytes().to_vec(),
    }])
    .unwrap_err();
    assert_eq!(actual, expected);
    assert_eq!(actual.to_string(), display);
}
#[test]
fn strict_skeleton_field_types_and_numeric_tokens() {
    use ftoo_manifest::{
        PathPart::{Index, Key},
        ShapeKind, UnitFault, UnitStage,
    };
    for (body, path, expected) in [
        (
            "skeleton=17",
            vec![Key("skeleton".into())],
            ShapeKind::Table,
        ),
        (
            "skeleton={document=17,sections=[]}",
            vec![Key("skeleton".into()), Key("document".into())],
            ShapeKind::String,
        ),
        (
            "skeleton={document='main',sections=17}",
            vec![Key("skeleton".into()), Key("sections".into())],
            ShapeKind::Array,
        ),
        (
            "skeleton={document='main',sections=[17]}",
            vec![Key("skeleton".into()), Key("sections".into()), Index(0)],
            ShapeKind::Table,
        ),
        (
            "skeleton={document='main',sections=[{heading=17,children=[]}]}",
            vec![
                Key("skeleton".into()),
                Key("sections".into()),
                Index(0),
                Key("heading".into()),
            ],
            ShapeKind::String,
        ),
        (
            "skeleton={document='main',sections=[{heading='sec:x:y',children=17}]}",
            vec![
                Key("skeleton".into()),
                Key("sections".into()),
                Index(0),
                Key("children".into()),
            ],
            ShapeKind::Array,
        ),
        (
            "skeleton={document='main',sections=[{heading='sec:x:y',children=[17]}]}",
            vec![
                Key("skeleton".into()),
                Key("sections".into()),
                Index(0),
                Key("children".into()),
                Index(0),
            ],
            ShapeKind::Table,
        ),
        (
            "skeleton={document='main',sections=[{heading='sec:x:y',children=[{kind=17,unit='sec:x:z'}]}]}",
            vec![
                Key("skeleton".into()),
                Key("sections".into()),
                Index(0),
                Key("children".into()),
                Index(0),
                Key("kind".into()),
            ],
            ShapeKind::String,
        ),
        (
            "skeleton={document='main',sections=[{heading='sec:x:y',children=[{kind='unit',unit=17}]}]}",
            vec![
                Key("skeleton".into()),
                Key("sections".into()),
                Index(0),
                Key("children".into()),
                Index(0),
                Key("unit".into()),
            ],
            ShapeKind::String,
        ),
        (
            "skeleton={document='main',sections=[{heading='sec:x:y',children=[{kind='section',heading=17,children=[]}]}]}",
            vec![
                Key("skeleton".into()),
                Key("sections".into()),
                Index(0),
                Key("children".into()),
                Index(0),
                Key("heading".into()),
            ],
            ShapeKind::String,
        ),
    ] {
        skeleton_boundary(
            &format!("schema=1\n{body}\n"),
            path,
            "17",
            RuleId::M004,
            UnitStage::Shape,
            UnitFault::WrongType {
                expected,
                actual: ShapeKind::Integer,
            },
        );
    }
    for token in ["+1", "0x1", "0o1", "0b1", "1_0", "-0", "2"] {
        skeleton_boundary(
            &format!("schema={token}\nskeleton={{document='main',sections=[]}}\n"),
            vec![Key("schema".into())],
            token,
            RuleId::M003,
            UnitStage::Shape,
            UnitFault::SchemaValue {
                expected: "1".into(),
                actual: token.into(),
            },
        );
    }
    for token in ["1.0", "1e0", "inf", "+inf", "-inf", "nan", "+nan", "-nan"] {
        skeleton_boundary(
            &format!(
                "schema=1\nunknown=[{{value={token}}}]\nskeleton={{document='main',sections=[]}}\n"
            ),
            vec![Key("unknown".into()), Index(0), Key("value".into())],
            token,
            RuleId::M003,
            UnitStage::Numeric,
            UnitFault::NonIntegerNumber {
                token: token.into(),
            },
        );
    }
}

#[test]
fn skeleton_source_lists_retain_duplicate_positions_and_bad_names() {
    let source = crate::UnitSource {
        file: "units/skeletons/a.toml".into(),
        bytes: b"schema=1\nskeleton={document='main',sections=[]}".to_vec(),
    };
    assert_eq!(
        input::skeletons(&[source.clone(), source.clone()]).unwrap_err(),
        DocumentError::Input(DocumentInputFault::DuplicateSource {
            file: "units/skeletons/a.toml".into(),
            positions: vec![0, 1]
        })
    );
    for name in ["", "/a", "a//b", "./a", "a/../b", "a\\b", "a/"] {
        let mut changed = source.clone();
        changed.file = name.into();
        assert_eq!(
            input::skeletons(&[changed]).unwrap_err(),
            DocumentError::Input(DocumentInputFault::InvalidSourceName { value: name.into() })
        );
    }
    assert_eq!(
        input::skeletons(&[]).unwrap_err(),
        DocumentError::Input(DocumentInputFault::EmptySkeletons {
            directory: "units/skeletons".into()
        })
    );
}
#[test]
fn support_required_mutation_during_successful_native_invocation_invalidates_success_and_restores()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original = NativeSupport::from_environment(&root).map_err(|e| format!("{e:?}"))?;
    let lock = native_support::lock(&root).map_err(|e| format!("{e:?}"))?;
    let _serial = brackets::MUTATIONS.lock().unwrap();
    let directory = Directory(
        original
            .directory()
            .join("downloads")
            .join(format!("bundle-copy-{}", std::process::id())),
    );
    let copy_phase = crate::telemetry::Phase::new("native.private_bundle_copy");
    for path in ["runtime/bin", "bundle", "downloads", "tmp"] {
        fs::create_dir_all(directory.0.join(path))?;
    }
    fs::copy(
        original.directory().join("runtime/bin/tectonic"),
        directory.0.join("runtime/bin/tectonic"),
    )?;
    for member in lock.files.keys() {
        let destination = directory.0.join("bundle").join(member);
        fs::create_dir_all(destination.parent().unwrap())?;
        fs::copy(
            original.directory().join("bundle").join(member),
            destination,
        )?;
    }
    copy_phase.finish(true);
    crate::telemetry::counter("native.private_bundle_copies", 1);
    let support =
        NativeSupport::resolve(&root, Some(&directory.0)).map_err(|e| format!("{e:?}"))?;
    inherited_native_fixture(&support, &root);
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let addressing = runtime::render(&book, &[])?;
    let mut source = latex::start("fixture", "en");
    source.push_str("Upright \\textbf{Bold} \\textit{Italic} \\textbf{\\textit{Bold italic}}.\n\\texttt{Mono \\textbf{Bold} \\textit{Italic} \\textbf{\\textit{Bold italic}}}.\n\\end{document}\n");
    let path = directory.0.join("bundle/brsgnmlminb-v.vf");
    let saved = fs::read(&path)?;
    assert_eq!(
        lock.files["brsgnmlminb-v.vf"],
        "4cac4fa71f6376629947c72e619953b6670dd74d1a3cfffefea4652c695e882e"
    );
    let expected = DocumentError::Native(NativeAdapterFault::Integrity {
        component: NativeComponent::Bundle,
        member: "brsgnmlminb-v.vf".into(),
        expected: "4cac4fa71f6376629947c72e619953b6670dd74d1a3cfffefea4652c695e882e".into(),
        actual: "3e23e8160039594a33894f6564e1b1348bbd7a0088d42c4acb73eeaed59c009d".into(),
    });
    let case = NativeCase::Document {
        document: "fixture".into(),
        language: "en".into(),
    };
    {
        use std::os::unix::fs::PermissionsExt;
        let executable = support.directory().join("runtime/bin/tectonic");
        let permissions = fs::metadata(&executable)?.permissions();
        let bytes = fs::read(&executable)?;
        crate::support_bracket::mutation::set_permissions(
            &executable,
            fs::Permissions::from_mode(0o644),
        )?;
        let fault = NativeAdapterFault::Io {
            operation: NativeOperation::Spawn,
            path: executable.clone(),
            kind: std::io::ErrorKind::PermissionDenied,
        };
        let actual = native::consume(
            &support,
            &lock,
            &source,
            "",
            &addressing,
            case.clone(),
            vec![],
        )
        .unwrap_err();
        assert_eq!(actual, DocumentError::Native(fault.clone()));
        assert_eq!(actual.to_string(), format!("Native({fault:?})"));
        assert_eq!(fs::read(&executable)?, bytes);
        assert_eq!(fs::read_dir(support.directory().join("tmp"))?.count(), 0);
        crate::support_bracket::mutation::set_permissions(&executable, permissions.clone())?;
        assert_eq!(
            fs::metadata(&executable)?.permissions().mode(),
            permissions.mode()
        );
        assert_eq!(fs::read(&executable)?, bytes);
        assert_eq!(
            native::consume(
                &support,
                &lock,
                &source,
                "",
                &addressing,
                case.clone(),
                vec![]
            ),
            Ok(())
        );
    }
    let before = brackets::identity(&path);
    let actual = native::consume_with(
        &support,
        &lock,
        &source,
        "",
        &addressing,
        case.clone(),
        Vec::new(),
        |scratch| {
            assert_eq!(fs::read_dir(scratch).unwrap().count(), 0);
        },
        |_, result| {
            assert_eq!(result, &Ok(()));
            crate::support_bracket::mutation::write(&path, b"b").unwrap();
        },
    );
    let fault = NativeAdapterFault::SupportChanged {
        member: path.to_string_lossy().into_owned(),
        expected: before,
        actual: Some(brackets::identity(&path)),
    };
    assert_eq!(actual, Err(DocumentError::Native(fault.clone())));
    assert_eq!(
        actual.unwrap_err().to_string(),
        format!("Native({fault:?})")
    );
    let DocumentError::Native(expected_hash) = expected else {
        unreachable!()
    };
    assert_eq!(support.closing_hash(), Some(Err(expected_hash)));
    assert_eq!(fs::read(&path)?, b"b");
    crate::support_bracket::mutation::write(&path, &saved)?;
    assert_eq!(fs::read(&path)?, saved);
    assert_eq!(
        native::consume_with(
            &support,
            &lock,
            &source,
            "",
            &addressing,
            case.clone(),
            Vec::new(),
            |scratch| {
                // Fontconfig's per-user font directory is under this invocation's HOME.
                // Trailing bytes leave each OTF name table intact but change its digest.
                let fonts = scratch.join(".fonts");
                fs::create_dir(&fonts).unwrap();
                for name in [
                    "lmroman10-regular.otf",
                    "lmroman10-bold.otf",
                    "lmroman10-italic.otf",
                    "lmroman10-bolditalic.otf",
                    "lmmono10-regular.otf",
                    "lmmono10-italic.otf",
                    "lmmonolt10-bold.otf",
                    "lmmonolt10-boldoblique.otf",
                ] {
                    let original = fs::read(support.directory().join("bundle").join(name)).unwrap();
                    let mut alternative = original.clone();
                    alternative.extend_from_slice(b"system-font-substitution-fixture");
                    crate::support_bracket::mutation::write(fonts.join(name), &alternative)
                        .unwrap();
                    assert_eq!(fs::read(fonts.join(name)).unwrap(), alternative);
                    assert_ne!(alternative, original);
                }
            },
            |scratch, result| {
                assert_eq!(result, &Ok(()));
                let log = fs::read_to_string(scratch.join("output/document.log"))
                    .unwrap()
                    .replace('\n', "");
                let selected = log
                    .split("[lm")
                    .skip(1)
                    .filter_map(|part| {
                        part.split_once(".otf]")
                            .map(|(name, _)| format!("lm{name}.otf"))
                    })
                    .collect::<std::collections::BTreeSet<_>>();
                assert_eq!(
                    selected,
                    [
                        "lmroman10-regular.otf",
                        "lmroman10-bold.otf",
                        "lmroman10-italic.otf",
                        "lmroman10-bolditalic.otf",
                        "lmmono10-regular.otf",
                        "lmmono10-italic.otf",
                        "lmmonolt10-bold.otf",
                        "lmmonolt10-boldoblique.otf"
                    ]
                    .map(str::to_owned)
                    .into_iter()
                    .collect()
                );
            }
        ),
        Ok(())
    );
    let font = directory.0.join("bundle/lmroman10-regular.otf");
    let saved = fs::read(&font)?;
    crate::support_bracket::mutation::remove_file(&font)?;
    let actual = native::consume(
        &support,
        &lock,
        &source,
        "",
        &addressing,
        case.clone(),
        Vec::new(),
    );
    assert_eq!(
        actual,
        Err(DocumentError::Native(NativeAdapterFault::Inventory {
            component: NativeComponent::Bundle,
            missing: vec!["lmroman10-regular.otf".into()],
            unexpected: Vec::new()
        }))
    );
    crate::support_bracket::mutation::write(&font, &saved)?;
    assert_eq!(fs::read(&font)?, saved);
    assert_eq!(
        native::consume(&support, &lock, &source, "", &addressing, case, Vec::new()),
        Ok(())
    );
    Ok(())
}
#[test]
fn native_invocation_has_exact_local_bundle_arguments_and_confined_environment() {
    use std::ffi::OsString;
    let directory = Directory::new();
    let support = NativeSupport::at(directory.0.clone());
    let scratch = directory.0.join("tmp/invocation");
    let command = native::command(&support, &scratch);
    assert_eq!(
        command.get_program(),
        directory.0.join("runtime/bin/tectonic")
    );
    assert_eq!(command.get_current_dir(), Some(scratch.as_path()));
    assert_eq!(
        command.get_args().map(OsString::from).collect::<Vec<_>>(),
        vec![
            OsString::from("--bundle"),
            directory.0.join("bundle").into_os_string(),
            OsString::from("--only-cached"),
            OsString::from("--untrusted"),
            OsString::from("--keep-logs"),
            OsString::from("--color"),
            OsString::from("never"),
            OsString::from("--outdir"),
            scratch.join("output").into_os_string(),
            scratch.join("document.tex").into_os_string(),
        ]
    );
    assert_eq!(
        command
            .get_envs()
            .map(|(key, value)| (key.to_owned(), value.map(OsString::from)))
            .collect::<BTreeMap<_, _>>(),
        BTreeMap::from([
            (
                OsString::from("FONTCONFIG_FILE"),
                Some(OsString::from("/fontconfig.conf"))
            ),
            (
                OsString::from("FONTCONFIG_SYSROOT"),
                Some(scratch.clone().into_os_string())
            ),
            (
                OsString::from("HOME"),
                Some(scratch.clone().into_os_string())
            ),
            (OsString::from("LANG"), Some(OsString::from("C.UTF-8"))),
            (
                OsString::from("TECTONIC_CACHE_DIR"),
                Some(scratch.join("bundle-cache").into_os_string())
            ),
            (
                OsString::from("XDG_CACHE_HOME"),
                Some(scratch.join("cache").into_os_string())
            ),
            (
                OsString::from("XDG_CONFIG_HOME"),
                Some(scratch.join("config").into_os_string())
            ),
        ])
    );
}
#[cfg(unix)]
#[test]
fn a_zero_status_cannot_accept_the_complete_lost_character_log() {
    use std::os::unix::process::ExitStatusExt;
    let case = NativeCase::Document {
        document: "fixture".into(),
        language: "en".into(),
    };
    let site = AssemblySite::Generated {
        path: "document.tex".into(),
        bytes: 0..4,
        origins: Vec::new(),
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
                stdout: b"complete".to_vec(),
                stderr: Vec::new(),
                log: LOST_CHARACTER_LOG.as_bytes().to_vec(),
            },
        }])
        .unwrap(),
    );
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(0),
        stdout: b"complete".to_vec(),
        stderr: Vec::new(),
    };
    assert_eq!(
        native::classify(
            output,
            LOST_CHARACTER_LOG.as_bytes().to_vec(),
            site,
            case,
            true
        ),
        Err(expected)
    );
}
fn support_required_actual_native_faces_framing_and_expanded_live_keys(
    support: &NativeSupport,
    lock: &native_support::Lock,
    checks: &mut Vec<CaseEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let addressing = runtime::render(&book, &[])?;
    let mut source = latex::start("fixture", "en");
    source.push_str(
        r"Upright \textbf{Bold} \textit{Italic} \textbf{\textit{Bold italic}}.

\texttt{Mono \textbf{Bold} \textit{Italic} \textbf{\textit{Bold italic}}}.

\def\example#1{#1}
BODYTAIL
% {#
COMMENTTAIL
\def\handle{c:1}
\FtdocTag{c:1}{direct}\FtdocTag{\handle}{composed}
\end{document}
",
    );
    checks.push(CaseEvidence::new(
        "faces/accepted",
        native::consume(
            support,
            lock,
            &source,
            "",
            &addressing,
            NativeCase::Document {
                document: "fixture".into(),
                language: "en".into(),
            },
            Vec::new(),
        ),
        Ok(()),
    ));
    Ok(())
}

fn support_required_actual_native_unknown_tag_has_complete_decoded_refusal(
    support: &NativeSupport,
    lock: &native_support::Lock,
    checks: &mut Vec<CaseEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let addressing = runtime::render(&book, &[])?;
    let mut source = latex::start("fixture", "en");
    source.push_str("\\FtdocTag{c:01}{u0-t0}\n\\end{document}\n");
    let expected = DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Native,
            document: None,
            unit: None,
            language: None,
            target: None,
            site: AssemblySite::Generated {
                path: "document.tex".into(),
                bytes: latex::start("fixture", "en").len()
                    ..latex::start("fixture", "en").len() + 22,
                origins: Vec::new(),
            },
            fault: AssemblyFault::RuntimeTagRejected {
                input: "c:01".into(),
                site_token: "u0-t0".into(),
                message: "tag \"c:01\": unknown".into(),
            },
        }])
        .unwrap(),
    );
    checks.push(CaseEvidence::new(
        "unknown/refusal",
        native::consume(
            support,
            lock,
            &source,
            "",
            &addressing,
            NativeCase::Runtime {
                site_token: "u0-t0".into(),
            },
            Vec::new(),
        ),
        Err(expected),
    ));
    Ok(())
}

fn support_required_native_lost_character_is_a_complete_hard_error(
    support: &NativeSupport,
    lock: &native_support::Lock,
    checks: &mut Vec<CaseEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let addressing = runtime::render(&book, &[])?;
    let mut source = latex::start("fixture", "en");
    source.push_str("\u{10ffff}\n\\end{document}\n");
    let case = NativeCase::Document {
        document: "fixture".into(),
        language: "en".into(),
    };
    let mut scratch = PathBuf::new();
    let actual = native::consume_with(
        support,
        lock,
        &source,
        "",
        &addressing,
        case.clone(),
        Vec::new(),
        |path| scratch = path.into(),
        |_, _| {},
    );
    let format_progress = if support.cold_format.get() {
        "note: generating format \"latex\"\n"
    } else {
        ""
    };
    let stdout=format!("note: using only cached resource files\n{format_progress}note: Running TeX ...\nnote: Writing `{}` (12.72265625 KiB)\n",scratch.join("output/document.log").display()).into_bytes();
    let stderr = if support.cold_format.get() {
        include_bytes!("../../tests/fixtures/native/cold-lost-character-refusal.stderr").to_vec()
    } else {
        include_bytes!("../../tests/fixtures/native/lost-character-refusal.stderr").to_vec()
    };
    let expected = DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M056,
            stage: AssemblyStage::Native,
            document: Some("fixture".into()),
            unit: None,
            language: Some("en".into()),
            target: Some(DocumentTarget::Latex),
            site: AssemblySite::Generated {
                path: "generated/documents/fixture/en.tex".into(),
                bytes: 0..source.len(),
                origins: Vec::new(),
            },
            fault: AssemblyFault::NativeRejected {
                case,
                status: Some(1),
                stdout,
                stderr,
                log: LOST_CHARACTER_LOG.as_bytes().to_vec(),
            },
        }])
        .unwrap(),
    );
    checks.push(CaseEvidence::new(
        "lost-character/refusal",
        actual,
        Err(expected),
    ));
    Ok(())
}

#[test]
#[allow(clippy::too_many_lines)] // Complete independent inventories are intentionally literal.
fn independent_recovery_checks_both_targets_and_missing_blocks() {
    use plan::{Block, DocumentPlan, Inline};
    let site = UnitSite {
        file: "units/statements/a.toml".into(),
        path: vec![
            ftoo_manifest::PathPart::Key("unit".into()),
            ftoo_manifest::PathPart::Key("id".into()),
        ],
        anchor: SourceAnchor::Unavailable,
        decoded: None,
        occurrence: None,
    };
    let plan = DocumentPlan {
        document: "main".into(),
        language: "en".into(),
        coverage: plan::Coverage::fixture("Coverage.", "Slice."),
        screen: None,
        blocks: vec![Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: "sec:coefficients:overview".into(),
            depth: Some(1),
            quotation: false,
            paragraph: false,
            equation: false,
            site: site.clone(),
            runs: vec![
                Inline::Text("Coefficient ".into()),
                Inline::Tag {
                    handle: "c:1".into(),
                    label: "def:langevin:coefficient-one".into(),
                    token: "u0-t0".into(),
                    site: site.clone(),
                },
                Inline::Text("; ".into()),
                Inline::Math {
                    definition_site: input::site("fixture-math.toml"),
                    key: "value".into(),
                    markdown: "1".into(),
                    latex: "1".into(),
                    pending: vec![],
                    site: site.clone(),
                },
                Inline::Text(". See ".into()),
                Inline::Citation {
                    identity: synthetic_citation_identity(
                        "dec:weld:equal-address-sets",
                        "sec:coefficients:overview",
                        "dec:weld",
                        "u0-c0",
                    ),
                    text: "dec:weld:equal-address-sets".into(),
                    site: site.clone(),
                },
                Inline::Text(".".into()),
            ],
        }],
    };
    for (target, rendered) in [
        (DocumentTarget::Markdown, markdown::render(&plan)),
        (DocumentTarget::Latex, latex::render(&plan)),
    ] {
        assert_eq!(plan::check_emission(&plan, &rendered, target), Ok(()));
        let expected = vec![
            EmissionRecord {
                kind: EmissionKind::Heading,
                unit: "sec:coefficients:overview".into(),
                language: "en".into(),
                target,
                occurrence: None,
                value: "Coefficient @tag{c:1}; @math{value}. See `dec:weld:equal-address-sets`."
                    .into(),
                site: site.clone(),
            },
            EmissionRecord {
                kind: EmissionKind::Tag,
                unit: "sec:coefficients:overview".into(),
                language: "en".into(),
                target,
                occurrence: None,
                value: "def:langevin:coefficient-one".into(),
                site: site.clone(),
            },
            EmissionRecord {
                kind: EmissionKind::Math,
                unit: "sec:coefficients:overview".into(),
                language: "en".into(),
                target,
                occurrence: None,
                value: "value".into(),
                site: site.clone(),
            },
            EmissionRecord {
                kind: EmissionKind::Citation(Box::new(synthetic_citation_identity(
                    "dec:weld:equal-address-sets",
                    "sec:coefficients:overview",
                    "dec:weld",
                    "u0-c0",
                ))),
                unit: "sec:coefficients:overview".into(),
                language: "en".into(),
                target,
                occurrence: None,
                value: "dec:weld:equal-address-sets".into(),
                site: site.clone(),
            },
        ];
        assert_eq!(
            plan::check_emission(&plan, "", target),
            Err(failure(
                RuleId::M055,
                AssemblyStage::Emission,
                AssemblySite::Generated {
                    path: format!(
                        "generated/documents/main/en.{}",
                        if target == DocumentTarget::Markdown {
                            "labels.md"
                        } else {
                            "tex"
                        }
                    ),
                    bytes: 0..0,
                    origins: [
                        vec![
                            SourceOrigin {
                                unit: Some("sec:coefficients:overview".into()),
                                language: Some("en".into()),
                                target: Some(target),
                                site: site.clone()
                            };
                            3
                        ],
                        vec![
                            SourceOrigin {
                                unit: Some("sec:coefficients:overview".into()),
                                language: Some("en".into()),
                                target: None,
                                site: site.clone()
                            },
                            SourceOrigin {
                                unit: None,
                                language: None,
                                target: None,
                                site: UnitSite {
                                    file: "fixture.md".into(),
                                    path: vec![],
                                    anchor: SourceAnchor::Bytes(0, 27),
                                    decoded: None,
                                    occurrence: None
                                }
                            }
                        ]
                    ]
                    .concat()
                },
                AssemblyFault::EmissionMismatch {
                    missing: expected,
                    unexpected: Vec::new(),
                    changed: Vec::new()
                }
            ))
        );
        assert_eq!(plan::check_emission(&plan, &rendered, target), Ok(()));
    }
}
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(super) struct Directory(pub(super) PathBuf);
impl Directory {
    pub(super) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "assembly-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

struct NativeFixture {
    directory: Directory,
    _lock: fs::File,
}
fn native_fixture() -> NativeFixture {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| repository.join("target"), PathBuf::from);
    fs::create_dir_all(&target).unwrap();
    let root = target.canonicalize().unwrap().join("ftoo-native-fixtures");
    fs::create_dir_all(&root).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("lock"))
        .unwrap();
    lock.lock().unwrap();
    // Brackets observe every ancestor through the target and this stable root.
    // Both controls hold this per-checkout lock while changing its case entry;
    // the directory drops before the lock, including in the isolated child.
    let path = root.join("case");
    if path.exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    fs::create_dir(&path).unwrap();
    NativeFixture {
        directory: Directory(path),
        _lock: lock,
    }
}

#[test]
#[allow(clippy::too_many_lines)] // One mutation matrix shares its restoration control.
fn native_preflight_faults_restore_and_never_execute() {
    use sha2::{Digest, Sha256};
    const A: &str = "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb";
    const B: &str = "3e23e8160039594a33894f6564e1b1348bbd7a0088d42c4acb73eeaed59c009d";
    let fixture = native_fixture();
    let directory = &fixture.directory;
    let support = NativeSupport::at(directory.0.clone());
    let missing = NativeAdapterFault::Configuration {
        component: None,
        path: Some(directory.0.join("runtime")),
        reason: NativeConfigurationReason::Missing,
    };
    let digest = "d".repeat(64);
    let lock = native_support::Lock {
        source: None,
        engine: A.into(),
        files: BTreeMap::from([
            (
                "SHA256SUM".into(),
                crate::hex(&Sha256::digest(digest.as_bytes())),
            ),
            ("font.otf".into(), A.into()),
            ("package.sty".into(), A.into()),
        ]),
        bundle_digest: digest.clone(),
    };
    assert_eq!(native_support::verify(&support, &lock), Err(missing));
    for path in ["runtime/bin", "bundle", "tmp", "downloads"] {
        fs::create_dir_all(directory.0.join(path)).unwrap();
    }
    crate::support_bracket::mutation::write(directory.0.join("runtime/bin/tectonic"), b"a")
        .unwrap();
    for file in ["font.otf", "package.sty"] {
        crate::support_bracket::mutation::write(directory.0.join("bundle").join(file), b"a")
            .unwrap();
    }
    crate::support_bracket::mutation::write(directory.0.join("bundle/SHA256SUM"), digest).unwrap();
    assert_eq!(native_support::verify(&support, &lock), Ok(()));
    for (relative, component, member, expected) in [
        (
            "runtime/bin/tectonic",
            NativeComponent::Executable,
            "runtime/bin/tectonic",
            A.to_owned(),
        ),
        (
            "bundle/font.otf",
            NativeComponent::Bundle,
            "font.otf",
            A.to_owned(),
        ),
        (
            "bundle/package.sty",
            NativeComponent::Bundle,
            "package.sty",
            A.to_owned(),
        ),
        (
            "bundle/SHA256SUM",
            NativeComponent::Bundle,
            "SHA256SUM",
            lock.files["SHA256SUM"].clone(),
        ),
    ] {
        let path = directory.0.join(relative);
        let original = fs::read(&path).unwrap();
        crate::support_bracket::mutation::write(&path, b"b").unwrap();
        let expected = DocumentError::Native(NativeAdapterFault::Integrity {
            component,
            member: member.into(),
            expected,
            actual: B.into(),
        });
        let actual = native::consume(
            &support,
            &lock,
            "",
            "",
            "",
            NativeCase::Document {
                document: "main".into(),
                language: "en".into(),
            },
            Vec::new(),
        )
        .unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(fs::read_dir(directory.0.join("tmp")).unwrap().count(), 0);
        assert_eq!(fs::read(&path).unwrap(), b"b");
        crate::support_bracket::mutation::write(&path, &original).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(native_support::verify(&support, &lock), Ok(()));
    }
    let path = directory.0.join("bundle/font.otf");
    crate::support_bracket::mutation::remove_file(&path).unwrap();
    assert_eq!(
        native_support::verify(&support, &lock),
        Err(NativeAdapterFault::Inventory {
            component: NativeComponent::Bundle,
            missing: vec!["font.otf".into()],
            unexpected: Vec::new()
        })
    );
    crate::support_bracket::mutation::write(&path, b"a").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"a");
    assert_eq!(native_support::verify(&support, &lock), Ok(()));
    let refuse = |expected: NativeAdapterFault| {
        let actual = native::consume(
            &support,
            &lock,
            "",
            "",
            "",
            NativeCase::Runtime {
                site_token: "never-executed".into(),
            },
            vec![],
        )
        .unwrap_err();
        assert_eq!(actual, DocumentError::Native(expected.clone()));
        assert_eq!(actual.to_string(), format!("Native({expected:?})"));
        assert_eq!(fs::read_dir(directory.0.join("tmp")).unwrap().count(), 0);
    };
    for (relative, component) in [
        ("runtime/bin/tectonic", NativeComponent::Executable),
        ("bundle/font.otf", NativeComponent::Bundle),
    ] {
        let path = directory.0.join(relative);
        let original = fs::read(&path).unwrap();
        crate::support_bracket::mutation::remove_file(&path).unwrap();
        let expected = if component == NativeComponent::Executable {
            NativeAdapterFault::Configuration {
                component: Some(component),
                path: Some(path.clone()),
                reason: NativeConfigurationReason::Missing,
            }
        } else {
            NativeAdapterFault::Inventory {
                component,
                missing: vec!["font.otf".into()],
                unexpected: vec![],
            }
        };
        refuse(expected);
        crate::support_bracket::mutation::write(&path, &original).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(native_support::verify(&support, &lock), Ok(()));
        crate::support_bracket::mutation::remove_file(&path).unwrap();
        crate::support_bracket::mutation::symlink(directory.0.join("bundle/package.sty"), &path)
            .unwrap();
        refuse(NativeAdapterFault::Configuration {
            component: Some(component),
            path: Some(path.clone()),
            reason: NativeConfigurationReason::NonRegularEntry {
                kind: DocumentEntryKind::Symlink,
            },
        });
        assert!(
            fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        crate::support_bracket::mutation::remove_file(&path).unwrap();
        crate::support_bracket::mutation::write(&path, &original).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(native_support::verify(&support, &lock), Ok(()));
    }
    let extra = directory.0.join("bundle/extra.sty");
    crate::support_bracket::mutation::write(&extra, b"a").unwrap();
    refuse(NativeAdapterFault::Inventory {
        component: NativeComponent::Bundle,
        missing: vec![],
        unexpected: vec!["extra.sty".into()],
    });
    assert_eq!(fs::read(&extra).unwrap(), b"a");
    crate::support_bracket::mutation::remove_file(&extra).unwrap();
    assert_eq!(native_support::verify(&support, &lock), Ok(()));
    let socket_parent = fs::File::open(extra.parent().unwrap()).unwrap();
    let socket_alias = format!(
        "/proc/self/fd/{}/{}",
        std::os::fd::AsRawFd::as_raw_fd(&socket_parent),
        extra.file_name().unwrap().to_str().unwrap()
    );
    let socket = std::os::unix::net::UnixListener::bind(socket_alias).unwrap();
    refuse(NativeAdapterFault::Configuration {
        component: Some(NativeComponent::Bundle),
        path: Some(extra.clone()),
        reason: NativeConfigurationReason::NonRegularEntry {
            kind: DocumentEntryKind::Other,
        },
    });
    drop(socket);
    crate::support_bracket::mutation::remove_file(&extra).unwrap();
    assert_eq!(native_support::verify(&support, &lock), Ok(()));
}

#[test]
fn lost_character_recognition_includes_hard_errors() {
    for line in [
        b"Missing character: There is no x".as_slice(),
        b"! Missing character: There is no x",
    ] {
        assert!(native::lost_character(line));
    }
    for line in [b"ordinary text".as_slice(), b"prefix Missing character:"] {
        assert!(!native::lost_character(line));
    }
}

#[test]
fn notation_refusal_uses_the_authored_ast_range_and_restores()
-> Result<(), Box<dyn std::error::Error>> {
    use ftoo_manifest::{
        NotationDiagnostic, NotationErrors, NotationFault,
        NotationPathPart::{Argument, Field},
        NotationResource, NotationSite, NotationStage,
    };
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let catalogue = ftoo_manifest::notation(&book)?;
    let source = format!("x + {}", "1".repeat(257));
    let parsed =
        ftoo_manifest::parse_assembly_math(&catalogue, &source).map_err(|e| format!("{e:?}"))?;
    let ranges = parsed.ranges().to_vec();
    let actual: crate::NotationRenderError =
        ftoo_manifest::check_expression(&catalogue, parsed.into_draft())
            .unwrap_err()
            .into();
    let expected = crate::NotationRenderError::Admission(
        NotationErrors::from_diagnostics(vec![NotationDiagnostic {
            rule: RuleId::M047,
            stage: NotationStage::Ast,
            site: NotationSite::Expression {
                path: vec![
                    Field("arguments".into()),
                    Argument(1),
                    Field("value".into()),
                ],
            },
            fault: NotationFault::NotationLimit {
                resource: NotationResource::LiteralBytes,
                limit: 256,
                actual: 257,
            },
        }])
        .unwrap(),
    );
    assert_eq!(actual, expected);
    assert_eq!(
        actual.to_string(),
        "M047 Ast at Expression { path: [Field(\"arguments\"), Argument(1), Field(\"value\")] }: NotationLimit { resource: LiteralBytes, limit: 256, actual: 257 }\n"
    );
    assert_eq!(plan::notation_bytes(&actual, &ranges, source.len()), 4..261);
    assert_eq!(&source[4..261], "1".repeat(257));
    let restored = format!("x + {}", "1".repeat(256));
    let parsed =
        ftoo_manifest::parse_assembly_math(&catalogue, &restored).map_err(|e| format!("{e:?}"))?;
    let checked = ftoo_manifest::check_expression(&catalogue, parsed.into_draft())?;
    assert_eq!(
        checked.draft(),
        &ftoo_manifest::ExprDraft::Call {
            callee: ftoo_manifest::Callee::Primitive(ftoo_manifest::PrimitiveId::Add),
            arguments: vec![
                ftoo_manifest::ExprDraft::Literal { value: "x".into() },
                ftoo_manifest::ExprDraft::Literal {
                    value: "1".repeat(256)
                }
            ],
        }
    );
    assert_eq!(format!("{restored}1"), source);
    Ok(())
}

#[test]
fn publication_io_failures_retain_completed_paths_cleanup_and_restoration() {
    use DocumentIoOperation::{
        CreateDirectory, CreateTemporary, Flush, Read, Remove, Rename, SyncDirectory, SyncFile,
        Write,
    };
    use std::io::{Error, ErrorKind};
    let directory = Directory::new();
    let paths = [
        "generated/documents/main/en.labels.md",
        "generated/documents/main/en.tex",
        "generated/notation.tex",
        "generated/addressing.tex",
    ];
    let obsolete = [
        "generated/documents/main/de.md",
        "generated/documents/main/fr.tex",
        "generated/documents/main/zz.html",
    ];
    let prepared = PreparedDocuments {
        snapshot: None,
        html_gap_rows: Vec::new(),
        screen_gap_rows: Vec::new(),
        artifacts: paths
            .iter()
            .map(|path| DocumentArtifact {
                path: (*path).into(),
                bytes: b"new\n".to_vec(),
                origins: Vec::new(),
                emissions: Vec::new(),
                html_gap_sites: Vec::new(),
            })
            .collect(),
        inputs: BTreeMap::new(),
        observed_outputs: BTreeMap::new(),
        unavailable: Vec::new(),
    };
    fs::create_dir_all(directory.0.join("generated/documents/main")).unwrap();
    let reset = || {
        for path in paths.into_iter().chain(obsolete) {
            fs::write(directory.0.join(path), b"old\n").unwrap();
            assert_eq!(fs::read(directory.0.join(path)).unwrap(), b"old\n");
        }
    };
    let restored = || {
        assert_eq!(
            artifacts::publish(&directory.0, &prepared, |_, _| Ok(())),
            Ok(())
        );
        assert_eq!(artifacts::verify(&directory.0, &prepared), Ok(()));
        for path in paths {
            assert_eq!(fs::read(directory.0.join(path)).unwrap(), b"new\n");
        }
        for path in obsolete {
            assert_eq!(
                fs::read(directory.0.join(path)).unwrap_err().kind(),
                ErrorKind::NotFound
            );
        }
    };
    for operation in [
        CreateDirectory,
        CreateTemporary,
        Write,
        Flush,
        SyncFile,
        SyncDirectory,
        Rename,
        Remove,
        Read,
    ] {
        for failure_index in 0..2 {
            reset();
            let mut seen = 0;
            let mut failed_path = None;
            let actual = artifacts::publish(&directory.0, &prepared, |candidate, path| {
                if candidate == operation {
                    let fail = seen == failure_index;
                    seen += 1;
                    if fail {
                        failed_path = Some(path.to_path_buf());
                        return Err(Error::from(ErrorKind::PermissionDenied));
                    }
                }
                Ok(())
            });
            let replaced_count = if matches!(operation, Remove | Read) {
                paths.len()
            } else if operation == SyncDirectory {
                failure_index + 1
            } else {
                failure_index
            };
            let removed_count = if operation == Read {
                obsolete.len()
            } else if operation == Remove {
                failure_index
            } else {
                0
            };
            let expected = DocumentError::Io {
                operation,
                path: failed_path.unwrap(),
                kind: ErrorKind::PermissionDenied,
                replaced: paths[..replaced_count]
                    .iter()
                    .map(|p| (*p).into())
                    .collect(),
                removed: obsolete[..removed_count]
                    .iter()
                    .map(|p| (*p).into())
                    .collect(),
                cleanup: Vec::new(),
            };
            assert_eq!(actual, Err(expected.clone()));
            assert_eq!(actual.unwrap_err().to_string(), format!("{expected:?}"));
            for (index, path) in paths.iter().enumerate() {
                assert_eq!(
                    fs::read(directory.0.join(path)).unwrap(),
                    if index < replaced_count {
                        b"new\n"
                    } else {
                        b"old\n"
                    }
                );
            }
            for (index, path) in obsolete.iter().enumerate() {
                if index < removed_count {
                    assert_eq!(
                        fs::read(directory.0.join(path)).unwrap_err().kind(),
                        ErrorKind::NotFound
                    );
                } else {
                    assert_eq!(fs::read(directory.0.join(path)).unwrap(), b"old\n");
                }
            }
            restored();
        }
    }
    reset();
    let mut writes = 0;
    let mut temporary = None;
    let actual = artifacts::publish(&directory.0, &prepared, |operation, path| {
        if operation == Write {
            writes += 1;
            if writes == 2 {
                temporary = Some(path.to_path_buf());
                return Err(Error::from(ErrorKind::WriteZero));
            }
        }
        if operation == Remove {
            assert_eq!(Some(path), temporary.as_deref());
            return Err(Error::from(ErrorKind::PermissionDenied));
        }
        Ok(())
    });
    let temporary = temporary.unwrap();
    let expected = DocumentError::Io {
        operation: Write,
        path: temporary.clone(),
        kind: ErrorKind::WriteZero,
        replaced: vec![paths[0].into()],
        removed: Vec::new(),
        cleanup: vec![IoFailure {
            operation: Remove,
            path: temporary.clone(),
            kind: ErrorKind::PermissionDenied,
        }],
    };
    assert_eq!(actual, Err(expected.clone()));
    assert_eq!(actual.unwrap_err().to_string(), format!("{expected:?}"));
    assert_eq!(fs::read(&temporary).unwrap(), b"");
    fs::remove_file(&temporary).unwrap();
    assert_eq!(
        fs::read(&temporary).unwrap_err().kind(),
        ErrorKind::NotFound
    );
    assert_eq!(fs::read(directory.0.join(paths[0])).unwrap(), b"new\n");
    for path in &paths[1..] {
        assert_eq!(fs::read(directory.0.join(path)).unwrap(), b"old\n");
    }
    restored();
}

#[test]
fn artifact_byte_faults_pin_offsets_and_restore_every_role() {
    let directory = Directory::new();
    let paths = [
        "generated/documents/main/en.labels.md",
        "generated/documents/main/en.tex",
        "generated/notation.tex",
        "generated/addressing.tex",
    ];
    let prepared = PreparedDocuments {
        snapshot: None,
        html_gap_rows: Vec::new(),
        screen_gap_rows: Vec::new(),
        artifacts: paths
            .iter()
            .map(|path| DocumentArtifact {
                path: (*path).into(),
                bytes: b"a\n".to_vec(),
                origins: Vec::new(),
                emissions: Vec::new(),
                html_gap_sites: Vec::new(),
            })
            .collect(),
        inputs: BTreeMap::new(),
        observed_outputs: BTreeMap::new(),
        unavailable: Vec::new(),
    };
    fs::create_dir_all(directory.0.join("generated/documents/main")).unwrap();
    for path in paths {
        fs::write(directory.0.join(path), b"a\n").unwrap();
    }
    assert_eq!(artifacts::verify(&directory.0, &prepared), Ok(()));
    for path in paths {
        for (actual, offset, end, diff) in [
            (
                b"b\n".as_slice(),
                0,
                1,
                "byte 0: expected length=2 window@0=610a EOF; actual length=2 window@0=620a EOF\n",
            ),
            (
                b"a".as_slice(),
                1,
                1,
                "byte 1: expected length=2 window@0=610a EOF; actual length=1 window@0=61 EOF\n",
            ),
            (
                b"a\r\n".as_slice(),
                1,
                2,
                "byte 1: expected length=2 window@0=610a EOF; actual length=3 window@0=610d0a EOF\n",
            ),
        ] {
            fs::write(directory.0.join(path), actual).unwrap();
            let expected = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M057,
                    stage: AssemblyStage::Bytes,
                    document: None,
                    unit: None,
                    language: None,
                    target: None,
                    site: AssemblySite::Generated {
                        path: path.into(),
                        bytes: offset..end,
                        origins: Vec::new(),
                    },
                    fault: AssemblyFault::NonCanonicalDocument {
                        path: path.into(),
                        offset,
                        diff: diff.into(),
                    },
                }])
                .unwrap(),
            );
            assert_eq!(artifacts::verify(&directory.0, &prepared), Err(expected));
            assert_eq!(fs::read(directory.0.join(path)).unwrap(), actual);
            fs::write(directory.0.join(path), b"a\n").unwrap();
            assert_eq!(artifacts::verify(&directory.0, &prepared), Ok(()));
        }
    }
}

#[test]
fn skeleton_encoding_precedes_shape_globally() {
    use ftoo_manifest::{UnitDiagnostic, UnitErrors, UnitFault, UnitStage};
    for (bytes, anchor, fault) in [
        (vec![255], SourceAnchor::Bytes(0, 1), UnitFault::InvalidUtf8),
        (
            b"\xef\xbb\xbfschema=1".to_vec(),
            SourceAnchor::Bytes(0, 3),
            UnitFault::Bom,
        ),
    ] {
        let sources = [
            crate::UnitSource {
                file: "units/skeletons/a.toml".into(),
                bytes: b"schema=2".to_vec(),
            },
            crate::UnitSource {
                file: "units/skeletons/b.toml".into(),
                bytes,
            },
        ];
        let expected = DocumentError::Boundary(
            UnitErrors::from_diagnostics(vec![UnitDiagnostic {
                rule: RuleId::M001,
                stage: UnitStage::Encoding,
                unit: None,
                language: None,
                site: UnitSite {
                    file: "units/skeletons/b.toml".into(),
                    path: Vec::new(),
                    anchor,
                    decoded: None,
                    occurrence: None,
                },
                fault,
            }])
            .unwrap(),
        );
        assert_eq!(input::skeletons(&sources).unwrap_err(), expected);
    }
}

#[test]
fn display_discovery_is_flat_and_absence_equals_empty() {
    let directory = Directory::new();
    assert_eq!(
        input::discover(&directory.0, "units/displays", true),
        Ok(Vec::new())
    );
    fs::create_dir_all(directory.0.join("units/displays")).unwrap();
    assert_eq!(
        input::discover(&directory.0, "units/displays", true),
        Ok(Vec::new())
    );
    fs::write(directory.0.join("units/displays/z.toml"), b"schema=1").unwrap();
    fs::write(directory.0.join("units/displays/a.toml"), b"schema=1").unwrap();
    assert_eq!(
        input::discover(&directory.0, "units/displays", true),
        Ok(vec![
            "units/displays/a.toml".into(),
            "units/displays/z.toml".into()
        ])
    );
    let path = directory.0.join("units/displays/b.txt");
    fs::write(&path, b"x").unwrap();
    assert_eq!(
        input::discover(&directory.0, "units/displays", true),
        Err(DocumentError::Input(DocumentInputFault::UnexpectedEntry {
            path: path.clone(),
            kind: DocumentEntryKind::NonTomlFile
        }))
    );
    fs::remove_file(path).unwrap();
    assert_eq!(
        input::discover(&directory.0, "units/displays", true),
        Ok(vec![
            "units/displays/a.toml".into(),
            "units/displays/z.toml".into()
        ])
    );
}

#[test]
fn source_snapshot_mutation_and_removal_are_distinct_and_restore() {
    let directory = Directory::new();
    let inventory = BTreeMap::from([("source.toml".into(), b"original".to_vec())]);
    let path = directory.0.join("source.toml");
    fs::write(&path, b"original").unwrap();
    assert_eq!(input::reread(&directory.0, &inventory), Ok(()));
    fs::write(&path, b"changed").unwrap();
    assert_eq!(
        input::reread(&directory.0, &inventory),
        Err(DocumentError::Input(DocumentInputFault::SourceChanged {
            file: "source.toml".into(),
            expected: b"original".to_vec(),
            actual: Some(b"changed".to_vec())
        }))
    );
    assert_eq!(fs::read(&path).unwrap(), b"changed");
    fs::write(&path, b"original").unwrap();
    assert_eq!(input::reread(&directory.0, &inventory), Ok(()));
    fs::remove_file(&path).unwrap();
    assert_eq!(
        input::reread(&directory.0, &inventory),
        Err(DocumentError::Input(DocumentInputFault::SourceChanged {
            file: "source.toml".into(),
            expected: b"original".to_vec(),
            actual: None
        }))
    );
    fs::write(&path, b"original").unwrap();
    assert_eq!(input::reread(&directory.0, &inventory), Ok(()));
}
#[test]
fn raw_framing_retains_both_tails() {
    for source in ["\\def\\example{{#}}\nTAIL\n", "% {#\nTAIL\n"] {
        let mut actual = String::new();
        latex::append(&mut actual, latex::Piece::Raw(latex::RawTex(source)));
        assert_eq!(actual, source);
    }
}
#[test]
fn runtime_record_decodes_complete_chunks() {
    assert_eq!(native::decode(b"FTDOC-R:i:0:1:633a3031\nFTDOC-R:s:0:1:75302d7430\nFTDOC-R:m:0:2:7461672022633a3031223a20756e6b6e\nFTDOC-R:m:1:2:6f776e\nFTDOC-END\n"),Some(("c:01".into(),"u0-t0".into(),"tag \"c:01\": unknown".into())));
}

#[test]
fn native_location_faults_are_exact() {
    use std::path::{Path, PathBuf};
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    for (value, expected) in [
        (
            None,
            NativeAdapterFault::Configuration {
                component: None,
                path: None,
                reason: NativeConfigurationReason::Missing,
            },
        ),
        (
            Some(PathBuf::from("relative")),
            NativeAdapterFault::Configuration {
                component: None,
                path: Some(PathBuf::from("relative")),
                reason: NativeConfigurationReason::RelativeSupport {
                    value: PathBuf::from("relative"),
                },
            },
        ),
        (
            Some(root.clone()),
            NativeAdapterFault::Configuration {
                component: None,
                path: Some(root.clone()),
                reason: NativeConfigurationReason::OverlappingSupport {
                    repository: root.clone(),
                    support: root.clone(),
                },
            },
        ),
        (
            Some(root.parent().unwrap().to_owned()),
            NativeAdapterFault::Configuration {
                component: None,
                path: Some(root.parent().unwrap().to_owned()),
                reason: NativeConfigurationReason::OverlappingSupport {
                    repository: root.clone(),
                    support: root.parent().unwrap().to_owned(),
                },
            },
        ),
    ] {
        let actual = NativeSupport::resolve(&root, value.as_deref()).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(
            DocumentError::Native(actual).to_string(),
            format!("Native({expected:?})")
        );
    }
}

#[test]
fn runtime_record_rejects_noncanonical_chunks() {
    for record in [
        b"FTDOC-R:i:00:1:61\nFTDOC-END\n".as_slice(),
        b"FTDOC-R:i:0:0:61\nFTDOC-END\n",
        b"FTDOC-R:i:0:1:AF\nFTDOC-END\n",
        b"FTDOC-R:i:0:2:61\nFTDOC-R:i:1:2:62\nFTDOC-END\n",
        b"FTDOC-R:s:0:1:61\nFTDOC-END\n",
        b"FTDOC-R:i:0:1:\r\nFTDOC-R:s:0:1:\r\nFTDOC-R:m:0:1:\r\nFTDOC-END\r\n",
        b"FTDOC-R:i:0:1:\nFTDOC-R:s:0:1:\nFTDOC-R:m:0:1:\nFTDOC-END",
        b"FTDOC-R:i:0:1:\nFTDOC-R:s:0:1:\nFTDOC-R:m:0:1:\nFTDOC-END\nFTDOC-END\n",
    ] {
        assert_eq!(native::decode(record), None);
    }
    assert_eq!(
        native::decode(b"FTDOC-R:i:0:1:\nFTDOC-R:s:0:1:\nFTDOC-R:m:0:1:\nFTDOC-END\n"),
        Some((String::new(), String::new(), String::new()))
    );
}

#[test]
fn prose_escaping_has_independent_literal_goldens() {
    assert_eq!(
        markdown::text("&<>()[]{}$#_%\\^~`\"'*/+-=|@,.:;?!"),
        "&#38;&#60;&#62;&#40;&#41;&#91;&#93;&#123;&#125;&#36;&#35;&#95;&#37;&#92;&#94;&#126;&#96;&#34;&#39;&#42;&#47;&#43;&#45;&#61;&#124;&#64;,.:;?!"
    );
    assert_eq!(
        latex::text("\\{}%#$&_^~α"),
        r"\textbackslash{}\{\}\%\#\$\&\_\textasciicircum{}\textasciitilde{}α"
    );
}

const LOST_CHARACTER_LOG: &str = include_str!("../../tests/fixtures/native/lost-character.log");

fn synthetic_citation_identity(
    label: &str,
    unit: &str,
    handle: &str,
    token: &str,
) -> CitationIdentity {
    CitationIdentity {
        label: label.into(),
        resolved: Some(ftoo_manifest::AddressTarget::Documentary(label.into())),
        handle: handle.into(),
        token: token.into(),
        position: ftoo_manifest::ReaderIdentityPosition::Citation {
            unit: unit.into(),
            span: 0,
            target: None,
        },
        mint: Some(ftoo_manifest::MintOrigin {
            kind: ftoo_manifest::MintKind::Markdown,
            site: ftoo_manifest::LabelSite::Markdown {
                file: "fixture.md".into(),
                bytes: 0..label.len(),
            },
        }),
    }
}

fn reader_fixture_book(graph: &ftoo_manifest::ValidatedManifest) -> ftoo_manifest::AddressBook<'_> {
    reader_fixture_book_state(graph, false)
}
fn reader_fixture_book_state(
    graph: &ftoo_manifest::ValidatedManifest,
    retired: bool,
) -> ftoo_manifest::AddressBook<'_> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let corpus = reader_fixture_corpus("See `dec:weld:equal-address-sets`.");
    let production = crate::load_markdown_files(&root).unwrap();
    let mut sources = production.sources().to_vec();
    sources.push(ftoo_manifest::MarkdownSource {
        file: "fixture.md".into(),
        bytes: "# Fixture · `dec:fixture:source`\n# Encoded · `dec:fixture:encoded`\n"
            .as_bytes()
            .to_vec(),
    });
    let markdown = ftoo_manifest::check_markdown_sources(sources).unwrap();
    let joint = ftoo_manifest::collect_joint_mints(&corpus, &markdown).unwrap();
    let mut events = registry::load(&registry::read(&root).unwrap()).unwrap();
    events.truncate(1); // This fixture exercises the weld decision and two local records.
    let located = |value: &str| ftoo_manifest::LocatedString {
        value: value.into(),
        site: input::site("fixture-registry.toml"),
    };
    events.push(ftoo_manifest::DocumentaryEventDraft {
        site: input::site("fixture-registry.toml"),
        ordinal: located("2"),
        handle: located("record"),
        label: Some(located("dec:fixture:source")),
        authority_record: located("dec:tags:bound-to-labels"),
        reason: located("Fixture citation."),
    });
    events.push(ftoo_manifest::DocumentaryEventDraft {
        site: input::site("fixture-registry.toml"),
        ordinal: located("3"),
        handle: located("é_#$&^~"),
        label: Some(located("dec:fixture:encoded")),
        authority_record: located("dec:tags:bound-to-labels"),
        reason: located("Encoded fixture citation."),
    });
    if retired {
        events.push(ftoo_manifest::DocumentaryEventDraft {
            site: input::site("fixture-registry.toml"),
            ordinal: located("4"),
            handle: located("dec:weld"),
            label: None,
            authority_record: located("dec:tags:bound-to-labels"),
            reason: located("Fixture retirement."),
        });
    }
    ftoo_manifest::addressing(graph)
        .unwrap()
        .with_documentary(&joint, events)
        .unwrap()
}

fn support_required_native_citation_keys_and_retirement(
    support: &NativeSupport,
    lock: &native_support::Lock,
    checks: &mut Vec<CaseEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ftoo_manifest::seed()?;
    let book = reader_fixture_book(&graph);
    let mut plan = reader_citation_plan(&book, "dec:weld:equal-address-sets");
    let source = latex::render(&plan);
    let addressing = runtime::render(&book, std::slice::from_ref(&plan))?;
    checks.push(CaseEvidence::new(
        "citation/literal",
        native::consume_plan(support, lock, &source, "", &addressing, &plan),
        Ok(()),
    ));
    let composed = source.replace(
        "{dec:weld}",
        r"{\FtdocHandle{100,101,99,58,119,101,108,100}}",
    );
    checks.push(CaseEvidence::new(
        "citation/composed",
        native::consume_plan(support, lock, &composed, "", &addressing, &plan),
        Ok(()),
    ));
    let retired = reader_fixture_book_state(&graph, true);
    let retired_addressing = runtime::render(&retired, std::slice::from_ref(&plan))?;
    let site = vec![
        SourceOrigin {
            unit: Some("sec:fixture:citation".into()),
            language: Some("en".into()),
            target: None,
            site: UnitSite {
                decoded: Some(4..33),
                ..input::site("fixture.toml")
            },
        },
        SourceOrigin {
            unit: None,
            language: None,
            target: None,
            site: UnitSite {
                file: "docs/adr/012-the-weld.md".into(),
                path: vec![],
                anchor: SourceAnchor::Bytes(117, 149),
                decoded: None,
                occurrence: None,
            },
        },
    ];
    let expected=DocumentError::Assembly(AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {rule:RuleId::M055,stage:AssemblyStage::Native,document:Some("fixture".into()),unit:Some("sec:fixture:citation".into()),language:Some("en".into()),target:Some(DocumentTarget::Latex),site:AssemblySite::Generated {path:"generated/documents/fixture/en.tex".into(),bytes:source.find(r"\FtdocTag").unwrap()..source.find("% cite").unwrap(),origins:site},fault:AssemblyFault::RuntimeTagRejected {input:"dec:weld".into(),site_token:"u0-c0".into(),message:"tag \"dec:weld\": retired; label dec:weld:equal-address-sets; event 4; authority dec:tags:bound-to-labels; reason \"Fixture retirement.\"".into()}}]).unwrap());
    checks.push(CaseEvidence::new(
        "citation/retired",
        native::consume_plan(support, lock, &source, "", &retired_addressing, &plan),
        Err(expected),
    ));
    checks.push(CaseEvidence::new(
        "citation/restored",
        native::consume_plan(support, lock, &source, "", &addressing, &plan),
        Ok(()),
    ));
    plan = reader_citation_plan(&book, "dec:fixture:encoded");
    let source = latex::render(&plan);
    let addressing = runtime::render(&book, std::slice::from_ref(&plan))?;
    checks.push(CaseEvidence::new(
        "citation/encoded",
        native::consume_plan(support, lock, &source, "", &addressing, &plan),
        Ok(()),
    ));
    Ok(())
}

fn reader_fixture_corpus(text: &str) -> ftoo_manifest::LanguageCheckedUnits {
    use ftoo_manifest::*;
    let site = input::site("fixture.toml");
    let value = |text: &str| LocatedString {
        value: text.into(),
        site: site.clone(),
    };
    admit_unit_drafts(
        DocumentDraft {
            documents_site: site.clone(),
            documents: ["main", "fixture"]
                .into_iter()
                .map(|document| {
                    (
                        document.into(),
                        DocumentEntryDraft {
                            key_site: site.clone(),
                            table_site: site.clone(),
                            languages_site: site.clone(),
                            required_languages: vec![value("en")],
                        },
                    )
                })
                .collect(),
        },
        vec![UnitDraft {
            id: value("sec:fixture:citation"),
            document: value("fixture"),
            table_site: site.clone(),
            math_site: site.clone(),
            texts_site: site.clone(),
            math: BTreeMap::new(),
            texts: BTreeMap::from([(
                "en".into(),
                LocatedEntryDraft {
                    key_site: site.clone(),
                    value: value(text),
                },
            )]),
        }],
    )
    .unwrap()
}
fn reader_citation_plan(book: &ftoo_manifest::AddressBook<'_>, label: &str) -> plan::DocumentPlan {
    fixture_plan_from_text(book, &format!("See `{label}`."))
}
fn fixture_plan_from_text(book: &ftoo_manifest::AddressBook<'_>, text: &str) -> plan::DocumentPlan {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let corpus = reader_fixture_corpus(text);
    let mut sources = crate::load_markdown_files(&root)
        .unwrap()
        .sources()
        .to_vec();
    sources.push(ftoo_manifest::MarkdownSource {
        file: "fixture.md".into(),
        bytes: "# Fixture · `dec:fixture:source`\n# Encoded · `dec:fixture:encoded`\n"
            .as_bytes()
            .to_vec(),
    });
    let markdown = ftoo_manifest::check_markdown_sources(sources).unwrap();
    let resolved = ftoo_manifest::resolve_unit_tags(&corpus, book).unwrap();
    let integrated = ftoo_manifest::integrate_labels(&resolved, &markdown).unwrap();
    let welded = ftoo_manifest::weld_units(&integrated).unwrap();
    plan::fixture_citation_plan(&welded)
}

#[test]
fn equation_display_boundaries_and_attachments_recover_from_both_writers() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    let mut plan = fixture_plan_from_text(&book, "A display.");
    let block = &mut plan.blocks[0];
    block.unit = "eq:fixture:flow".into();
    block.depth = None;
    block.paragraph = false;
    block.equation = true;
    let site = input::site("units/statements/fixture.toml");
    block.runs = vec![
        plan::Inline::Math {
            definition_site: site.clone(),
            key: "flow".into(),
            markdown: r"\underbrace{σ}_{\text{will, signed}}\;+\;\underbrace{γ}_{\text{decay}}"
                .into(),
            latex:
                r"\underbrace{\sigma}_{\text{will, signed}}\;+\;\underbrace{\gamma}_{\text{decay}}"
                    .into(),
            pending: vec![],
            site,
        },
        plan::Inline::Text(".".into()),
    ];
    for target in [DocumentTarget::Latex, DocumentTarget::Markdown] {
        let source = if target == DocumentTarget::Latex {
            latex::render(&plan)
        } else {
            markdown::render(&plan)
        };
        plan::check_emission(&plan, &source, target).unwrap();
        if target == DocumentTarget::Markdown {
            assert!(source.contains(r"\hspace{0.2778em}+\hspace{0.2778em}"));
            assert!(!source.contains(r"\;+\;"));
        }
        let mut changed_sources = vec![
            source.replacen("will, signed", "decay", 1),
            source
                .replacen("will, signed", "swapped attachment", 1)
                .replacen("decay", "will, signed", 1)
                .replacen("swapped attachment", "decay", 1),
            source.replacen("decay}}.\n", "decay}}\n", 1),
        ];
        if target == DocumentTarget::Markdown {
            changed_sources.push(source.replacen(r"\hspace{0.2778em}", r"\;", 1));
            changed_sources.push(source.replacen(r"\hspace{0.2778em}", ";", 1));
        }
        for changed in changed_sources {
            assert_ne!(changed, source);
            let error = plan::check_emission(&plan, &changed, target).unwrap_err();
            assert!(format!("{error:?}").contains("EmissionMismatch"));
        }
    }
    let plan::Inline::Math { markdown, .. } = &mut plan.blocks[0].runs[0] else {
        unreachable!()
    };
    markdown.push(':');
    let error = markdown::check_math_admission(&plan).unwrap_err();
    assert!(format!("{error:?}").contains("ForbiddenBodyCharacter"));
    assert!(format!("{error:?}").contains("top-level display"));
    let DocumentError::Assembly(errors) = error else {
        panic!("located display refusal")
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(input::site("units/statements/fixture.toml"))
    );
}

#[test]
fn equation_shape_refuses_missing_stop_at_source_under_m054() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    let mut plan = fixture_plan_from_text(&book, "A display.");
    let block = &mut plan.blocks[0];
    block.unit = "eq:fixture:flow".into();
    block.equation = true;
    block.runs = vec![plan::Inline::Math {
        definition_site: block.site.clone(),
        key: "flow".into(),
        markdown: "x".into(),
        latex: "x".into(),
        pending: vec![],
        site: block.site.clone(),
    }];
    let error = plan::check_equation_shape(block).unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("located refusal")
    };
    assert_eq!(errors.diagnostics()[0].rule, RuleId::M054);
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(block.site.clone())
    );
}

#[test]
fn inline_adapter_markup_is_classified_and_helper_builds_located_equation_diagnostic() {
    let inline_output = "<span class=\"katex\"><span class=\"katex-html\">x</span></span>";
    let returned = crate::notation::returned_display_mode(inline_output);
    let (bytes, fault) =
        plan::check_equation_mode("flow", 1, true, returned, returned).unwrap_err();
    let site = input::site("units/statements/explainer-overlap-flow.toml");
    let diagnostic = plan::equation_diagnostic(
        "explainer-pilot",
        "eq:explainer:overlap-flow",
        &site,
        "flow",
        vec![],
        bytes,
        fault,
    );
    assert_eq!(diagnostic.rule, RuleId::M053);
    assert_eq!(
        diagnostic.unit.as_deref(),
        Some("eq:explainer:overlap-flow")
    );
    assert_eq!(
        diagnostic.site,
        AssemblySite::Math {
            definition: site,
            key: "flow".into(),
            bytes: 0..1,
            uses: vec![],
        }
    );
    let AssemblyFault::UnknownToken { expected, .. } = diagnostic.fault else {
        panic!("mode refusal")
    };
    assert_eq!(expected, vec!["matching rendered display mode"]);
}

#[test]
fn screen_excerpt_equation_recovery_rejects_attachment_swap() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    let mut plan = fixture_plan_from_text(&book, "A display.");
    let site = input::site("units/statements/fixture.toml");
    plan.blocks[0].unit = "eq:fixture:flow".into();
    plan.blocks[0].depth = None;
    plan.blocks[0].paragraph = false;
    plan.blocks[0].equation = true;
    plan.blocks[0].runs = vec![
        plan::Inline::Math {
            definition_site: site.clone(),
            key: "flow".into(),
            markdown: r"\underbrace{σ}_{\text{first}}+\underbrace{γ}_{\text{second}}".into(),
            latex: r"\underbrace{\sigma}_{\text{first}}+\underbrace{\gamma}_{\text{second}}".into(),
            pending: vec![],
            site: site.clone(),
        },
        plan::Inline::Text(".".into()),
    ];
    plan.blocks.insert(
        0,
        plan::Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: "sec:fixture:heading".into(),
            depth: Some(1),
            quotation: false,
            paragraph: false,
            equation: false,
            site: site.clone(),
            runs: vec![plan::Inline::Text("A reading".into())],
        },
    );
    plan.screen = Some(plan::ScreenPlan {
        gaps: vec![
            plan::ScreenGap {
                position: GapPosition::Before,
                message: String::new(),
                site: site.clone(),
                boundaries: vec![],
            },
            plan::ScreenGap {
                position: GapPosition::After,
                message: String::new(),
                site: site.clone(),
                boundaries: vec![],
            },
        ],
        refrains: vec![],
        index: vec![],
        html_gaps: BTreeMap::new(),
        end_matter: vec![],
    });
    let annotated = format!("{}\n", markdown::render(&plan).trim_end_matches('\n'));
    plan::check_emission(&plan, &annotated, DocumentTarget::Markdown).unwrap();
    let changed = annotated
        .replacen("first", "temporary annotation", 1)
        .replacen("second", "first", 1)
        .replacen("temporary annotation", "second", 1);
    assert_ne!(changed, annotated);
    let error = plan::check_emission(&plan, &changed, DocumentTarget::Markdown).unwrap_err();
    assert!(format!("{error:?}").contains("EmissionMismatch"));
}

#[test]
fn checked_styles_cross_scanner_plan_writers_and_recovery() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    for kind in ["emphasis", "strong", "smallcaps", "regular"] {
        let source = format!("See @{kind}{{A & B}}.");
        let plan = fixture_plan_from_text(&book, &source);
        let plan::Inline::Styled {
            kind: actual,
            children,
            child_sites,
            site,
        } = &plan.blocks[0].runs[1]
        else {
            panic!("checked style missing")
        };
        assert_eq!(actual.as_str(), kind);
        assert_eq!(children.len(), 1);
        assert_eq!(
            child_sites[0].decoded,
            Some(4 + kind.len() + 2..4 + kind.len() + 7)
        );
        assert_eq!(child_sites[0].occurrence, None);
        assert_eq!(site.decoded, Some(4..source.len() - 1));
        assert_eq!(site.occurrence, Some(0));
        let tex = latex::render(&plan);
        let opener = match kind {
            "emphasis" => r"\emph{",
            "strong" => r"\textbf{",
            "smallcaps" => r"{\scshape ",
            "regular" => r"{\normalfont ",
            _ => unreachable!("closed test kinds"),
        };
        assert!(tex.contains(&format!(r"{opener}A \& B}}")));
        plan::check_emission(&plan, &tex, DocumentTarget::Latex).unwrap();
        if matches!(kind, "emphasis" | "strong") {
            markdown::check_math_admission(&plan).unwrap();
            let md = markdown::render(&plan);
            assert!(md.contains(if kind == "emphasis" {
                "See *A &#38; B*."
            } else {
                "See **A &#38; B**."
            }));
            plan::check_emission(&plan, &md, DocumentTarget::Markdown).unwrap();
        } else {
            let error = markdown::check_math_admission(&plan).unwrap_err();
            let DocumentError::Assembly(errors) = error else {
                panic!("style diagnostic")
            };
            assert!(matches!(
                errors.diagnostics()[0].fault,
                AssemblyFault::UnsupportedMarkdownStyle {
                    reason: diagnostics::MarkdownStyleReason::UnsupportedKind,
                    ..
                }
            ));
            assert_eq!(
                errors.diagnostics()[0].site,
                AssemblySite::Source(site.clone())
            );
        }
    }
}

#[test]
fn checked_style_recovery_refuses_boundary_and_identity_mutations() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    let plan = fixture_plan_from_text(&book, "See @emphasis{one} @emphasis{two}.");
    let md = markdown::render(&plan);
    plan::check_emission(&plan, &md, DocumentTarget::Markdown).unwrap();
    for mutated in [
        md.replace("*one*", "o*ne*"),
        md.replace("*one*", "one"),
        md.replace("*one* *two*", "*one two*"),
    ] {
        let error = plan::check_emission(&plan, &mutated, DocumentTarget::Markdown).unwrap_err();
        let DocumentError::Assembly(errors) = error else {
            panic!("emission diagnostic")
        };
        assert!(matches!(
            errors.diagnostics()[0].fault,
            AssemblyFault::EmissionMismatch { .. }
        ));
    }
    let plan = fixture_plan_from_text(
        &book,
        "See @emphasis{`dec:weld:equal-address-sets` and ``x&*``}.",
    );
    let plan::Inline::Styled {
        children,
        child_sites,
        ..
    } = &plan.blocks[0].runs[1]
    else {
        panic!("style")
    };
    assert!(matches!(children[0], plan::Inline::Citation { .. }));
    assert!(matches!(children[2], plan::Inline::Mention { .. }));
    let source = "See @emphasis{`dec:weld:equal-address-sets` and ``x&*``}.";
    let cite_at = source.find("`dec:weld:equal-address-sets`").unwrap();
    let mention_at = source.find("``x&*``").unwrap();
    assert_eq!(child_sites[0].decoded, Some(cite_at..cite_at + 29));
    assert_eq!(child_sites[2].decoded, Some(mention_at..mention_at + 7));
    let tex = latex::render(&plan);
    plan::check_emission(&plan, &tex, DocumentTarget::Latex).unwrap();
    let md = markdown::render(&plan);
    assert!(md.contains("<code>x&#38;&#42;</code>"));
    plan::check_emission(&plan, &md, DocumentTarget::Markdown).unwrap();
    let mutated = md.replace(
        "<!-- cite dec:weld:equal-address-sets -->",
        "<!-- cite dec:weld:other -->",
    );
    let error = plan::check_emission(&plan, &mutated, DocumentTarget::Markdown).unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("identity diagnostic")
    };
    assert!(matches!(
        errors.diagnostics()[0].fault,
        AssemblyFault::EmissionMismatch { .. }
    ));
}

#[test]
fn checked_style_tex_boundaries_and_markdown_context_are_checked() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    for (kind, opener) in [
        ("emphasis", r"\emph{"),
        ("strong", r"\textbf{"),
        ("smallcaps", r"{\scshape "),
        ("regular", r"{\normalfont "),
    ] {
        let plan = fixture_plan_from_text(&book, &format!("See @{kind}{{one}}."));
        let tex = latex::render(&plan);
        plan::check_emission(&plan, &tex, DocumentTarget::Latex).unwrap();
        for changed in [
            tex.replacen(&format!("{opener}one}}"), "one", 1),
            tex.replacen("one}", "o}ne", 1),
            tex.replacen(&format!("{opener}one}}"), &format!("{opener} one}}"), 1),
        ] {
            let error = plan::check_emission(&plan, &changed, DocumentTarget::Latex).unwrap_err();
            let DocumentError::Assembly(errors) = error else {
                panic!("emission diagnostic")
            };
            assert!(matches!(
                errors.diagnostics()[0].fault,
                AssemblyFault::EmissionMismatch { .. }
            ));
        }
    }
    let plan = fixture_plan_from_text(&book, "See @strong{one} @strong{two}.");
    let md = markdown::render(&plan);
    plan::check_emission(&plan, &md, DocumentTarget::Markdown).unwrap();
    for changed in [
        md.replace("**one**", "o**ne**"),
        md.replace("**one**", "one"),
        md.replace("**one** **two**", "**one two**"),
    ] {
        let error = plan::check_emission(&plan, &changed, DocumentTarget::Markdown).unwrap_err();
        let DocumentError::Assembly(errors) = error else {
            panic!("emission diagnostic")
        };
        assert!(matches!(
            errors.diagnostics()[0].fault,
            AssemblyFault::EmissionMismatch { .. }
        ));
    }
    let pair = fixture_plan_from_text(&book, "See @emphasis{one} @emphasis{two}.");
    let tex = latex::render(&pair);
    let merged = tex.replace(r"\emph{one} \emph{two}", r"\emph{one two}");
    assert_ne!(merged, tex);
    let error = plan::check_emission(&pair, &merged, DocumentTarget::Latex).unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("merged boundary diagnostic")
    };
    assert!(matches!(
        errors.diagnostics()[0].fault,
        AssemblyFault::EmissionMismatch { .. }
    ));
    let escaped = fixture_plan_from_text(&book, "See @emphasis{A @@ B & C}.");
    let tex = latex::render(&escaped);
    assert!(tex.contains("A @ B"));
    plan::check_emission(&escaped, &tex, DocumentTarget::Latex).unwrap();
    let md = markdown::render(&escaped);
    assert!(md.contains("A &#64; B &#38; C"));
    plan::check_emission(&escaped, &md, DocumentTarget::Markdown).unwrap();
}

#[test]
fn styled_children_and_canonical_markdown_are_compared() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    let plan = fixture_plan_from_text(
        &book,
        "See @emphasis{`dec:weld:equal-address-sets` and ``x&*``}.",
    );
    let tex = latex::render(&plan);
    let changed_cite = tex.replace(
        "% cite dec:weld:equal-address-sets",
        "% cite dec:fixture:source",
    );
    assert_ne!(changed_cite, tex);
    let error = plan::check_emission(&plan, &changed_cite, DocumentTarget::Latex).unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("cite identity diagnostic")
    };
    assert!(matches!(
        errors.diagnostics()[0].fault,
        AssemblyFault::EmissionMismatch { .. }
    ));
    let md = markdown::render(&plan);
    for changed in [
        md.replace("<code>x&#38;&#42;</code>", "<code>y&#38;&#42;</code>"),
        md.replace("<code>x&#38;&#42;</code>", "<code>x&amp;&#42;</code>"),
    ] {
        assert_ne!(changed, md);
        let error = plan::check_emission(&plan, &changed, DocumentTarget::Markdown).unwrap_err();
        let DocumentError::Assembly(errors) = error else {
            panic!("mention diagnostic")
        };
        assert!(matches!(
            errors.diagnostics()[0].fault,
            AssemblyFault::EmissionMismatch { .. }
        ));
    }
    let plan = fixture_plan_from_text(&book, "See @emphasis{one}.");
    let md = markdown::render(&plan);
    let changed = md.replace("*one*", "_one_");
    assert_ne!(changed, md);
    let error = plan::check_emission(&plan, &changed, DocumentTarget::Markdown).unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("spelling diagnostic")
    };
    assert!(matches!(
        errors.diagnostics()[0].fault,
        AssemblyFault::EmissionMismatch { .. }
    ));
}

#[test]
fn styled_tag_keeps_its_checked_identity_through_both_writers() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    let source = "See @strong{A @tag{c:1} B}.";
    let plan = fixture_plan_from_text(&book, source);
    let plan::Inline::Styled {
        children,
        child_sites,
        ..
    } = &plan.blocks[0].runs[1]
    else {
        panic!("styled run")
    };
    let tag_at = source.find("@tag{c:1}").unwrap();
    assert_eq!(child_sites[1].decoded, Some(tag_at..tag_at + 9));
    assert!(
        matches!(&children[1], plan::Inline::Tag { label, .. } if label == "def:langevin:coefficient-one")
    );
    let tex = latex::render(&plan);
    plan::check_emission(&plan, &tex, DocumentTarget::Latex).unwrap();
    let md = markdown::render(&plan);
    plan::check_emission(&plan, &md, DocumentTarget::Markdown).unwrap();
    let changed = md.replace(
        "<!-- label def:langevin:coefficient-one -->",
        "<!-- label def:langevin:other -->",
    );
    assert_ne!(changed, md);
    let error = plan::check_emission(&plan, &changed, DocumentTarget::Markdown).unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("tag identity diagnostic")
    };
    assert!(matches!(
        errors.diagnostics()[0].fault,
        AssemblyFault::EmissionMismatch { .. }
    ));
}

#[test]
fn markdown_styles_refuse_unobserved_context_edges_and_math_adjacency() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    for kind in ["emphasis", "strong"] {
        let mut plan = fixture_plan_from_text(&book, &format!("See @{kind}{{one}}."));
        let plan::Inline::Styled { site, .. } = &plan.blocks[0].runs[1] else {
            panic!("style")
        };
        let site = site.clone();
        plan.blocks[0].depth = Some(1);
        let error = markdown::check_math_admission(&plan).unwrap_err();
        let DocumentError::Assembly(errors) = error else {
            panic!("heading diagnostic")
        };
        assert_eq!(errors.diagnostics()[0].site, AssemblySite::Source(site));
        assert!(matches!(
            errors.diagnostics()[0].fault,
            AssemblyFault::UnsupportedMarkdownStyle {
                reason: diagnostics::MarkdownStyleReason::UnobservedContext,
                ..
            }
        ));
    }
    for (source, side) in [
        (
            "See x@emphasis{(a)} now.",
            diagnostics::StyleBoundarySide::Start,
        ),
        ("@emphasis{a ``b``}x", diagnostics::StyleBoundarySide::End),
    ] {
        let plan = fixture_plan_from_text(&book, source);
        let style = plan.blocks[0]
            .runs
            .iter()
            .find_map(|run| match run {
                plan::Inline::Styled { site, .. } => Some(site),
                _ => None,
            })
            .unwrap();
        let error = markdown::check_math_admission(&plan).unwrap_err();
        let DocumentError::Assembly(errors) = error else {
            panic!("edge diagnostic")
        };
        assert_eq!(
            errors.diagnostics()[0].site,
            AssemblySite::Source(style.clone())
        );
        assert!(matches!(
            errors.diagnostics()[0].fault,
            AssemblyFault::UnsupportedMarkdownStyle {
                reason: diagnostics::MarkdownStyleReason::PunctuationEdge { side: actual },
                ..
            } if actual == side
        ));
    }
    let mut plan = fixture_plan_from_text(&book, "See @emphasis{one}.");
    let style_site = match &plan.blocks[0].runs[1] {
        plan::Inline::Styled { site, .. } => site.clone(),
        _ => panic!("style"),
    };
    plan.blocks[0].runs.insert(
        1,
        plan::Inline::Math {
            definition_site: style_site.clone(),
            key: "x".into(),
            markdown: "x".into(),
            latex: "x".into(),
            pending: vec![],
            site: style_site.clone(),
        },
    );
    let error = markdown::check_math_admission(&plan).unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("math adjacency diagnostic")
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(style_site)
    );
    assert!(matches!(
        errors.diagnostics()[0].fault,
        AssemblyFault::UnsupportedMarkdownStyle {
            reason: diagnostics::MarkdownStyleReason::DirectAdjacency {
                neighbor: diagnostics::StyleNeighbor::Mathematics
            },
            ..
        }
    ));
}

#[test]
fn markdown_style_adjacencies_and_whitespace_edge_are_checked() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    let positive = fixture_plan_from_text(&book, "See @emphasis{(a)} now.");
    markdown::check_math_admission(&positive).unwrap();
    let emitted = markdown::render(&positive);
    assert!(emitted.contains("See *&#40;a&#41;* now."));
    plan::check_emission(&positive, &emitted, DocumentTarget::Markdown).unwrap();
    for (source, neighbor) in [
        (
            "See @strong{a.}@emphasis{b} now.",
            diagnostics::StyleNeighbor::Style,
        ),
        (
            "@emphasis{(a)}@strong{b}",
            diagnostics::StyleNeighbor::Style,
        ),
        (
            "See @tag{c:1}@emphasis{(a)} now.",
            diagnostics::StyleNeighbor::TagOrCitation,
        ),
        (
            "See `dec:weld:equal-address-sets`@emphasis{(a)} now.",
            diagnostics::StyleNeighbor::TagOrCitation,
        ),
        (
            "See @emphasis{(a)}@tag{c:1} now.",
            diagnostics::StyleNeighbor::TagOrCitation,
        ),
    ] {
        let plan = fixture_plan_from_text(&book, source);
        let first_style = plan.blocks[0]
            .runs
            .iter()
            .find_map(|run| match run {
                plan::Inline::Styled { site, .. } => Some(site),
                _ => None,
            })
            .unwrap();
        let error = markdown::check_math_admission(&plan).unwrap_err();
        let DocumentError::Assembly(errors) = error else {
            panic!("style adjacency diagnostic")
        };
        assert_eq!(
            errors.diagnostics()[0].site,
            AssemblySite::Source(first_style.clone())
        );
        assert!(matches!(
            errors.diagnostics()[0].fault,
            AssemblyFault::UnsupportedMarkdownStyle {
                reason: diagnostics::MarkdownStyleReason::DirectAdjacency { neighbor: actual },
                ..
            } if actual == neighbor
        ));
    }
    for before in [true, false] {
        let mut plan = fixture_plan_from_text(&book, "See @emphasis{one}.");
        let site = match &plan.blocks[0].runs[1] {
            plan::Inline::Styled { site, .. } => site.clone(),
            _ => panic!("styled run"),
        };
        plan.blocks[0].runs.insert(
            if before { 1 } else { 2 },
            plan::Inline::Math {
                definition_site: site.clone(),
                key: "x".into(),
                markdown: "x".into(),
                latex: "x".into(),
                pending: vec![],
                site: site.clone(),
            },
        );
        let error = markdown::check_math_admission(&plan).unwrap_err();
        let DocumentError::Assembly(errors) = error else {
            panic!("math adjacency diagnostic")
        };
        assert_eq!(errors.diagnostics()[0].site, AssemblySite::Source(site));
        assert!(matches!(
            errors.diagnostics()[0].fault,
            AssemblyFault::UnsupportedMarkdownStyle {
                reason: diagnostics::MarkdownStyleReason::DirectAdjacency {
                    neighbor: diagnostics::StyleNeighbor::Mathematics
                },
                ..
            }
        ));
    }
}

fn assert_style_punctuation_outer_neighbor(source: &str, side: diagnostics::StyleBoundarySide) {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    let plan = fixture_plan_from_text(&book, source);
    let style_site = plan.blocks[0]
        .runs
        .iter()
        .find_map(|run| match run {
            plan::Inline::Styled { site, .. } => Some(site),
            _ => None,
        })
        .unwrap();
    let error = markdown::check_math_admission(&plan).unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("punctuation edge diagnostic")
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(style_site.clone())
    );
    assert!(matches!(
        errors.diagnostics()[0].fault,
        AssemblyFault::UnsupportedMarkdownStyle {
            reason: diagnostics::MarkdownStyleReason::PunctuationEdge { side: actual },
            ..
        } if actual == side
    ));
}

#[test]
fn markdown_style_degree_outer_neighbor_refuses() {
    assert_style_punctuation_outer_neighbor(
        "5°@emphasis{(a)} now.",
        diagnostics::StyleBoundarySide::Start,
    );
}

#[test]
fn markdown_style_arrow_outer_neighbor_refuses() {
    assert_style_punctuation_outer_neighbor(
        "@emphasis{a ``b``}→ now.",
        diagnostics::StyleBoundarySide::End,
    );
}

#[test]
fn markdown_style_soft_hyphen_outer_neighbor_refuses() {
    assert_style_punctuation_outer_neighbor(
        "5\u{00ad}@emphasis{(a)} now.",
        diagnostics::StyleBoundarySide::Start,
    );
}

#[test]
fn markdown_style_screen_paragraph_is_observed() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    let mut plan = fixture_plan_from_text(&book, "See @emphasis{one}.");
    let site = match &plan.blocks[0].runs[1] {
        plan::Inline::Styled { site, .. } => site.clone(),
        _ => panic!("styled run"),
    };
    let heading = plan::Block {
        pending_names: std::collections::BTreeMap::new(),
        displays: std::collections::BTreeMap::new(),
        unit: "sec:fixture:screen-heading".into(),
        depth: Some(1),
        quotation: false,
        paragraph: false,
        equation: false,
        site: site.clone(),
        runs: vec![plan::Inline::Text("Screen".into())],
    };
    plan.blocks.insert(0, heading);
    plan.blocks[1].paragraph = true;
    assert!(plan.blocks[1].paragraph);
    plan.screen = Some(plan::ScreenPlan {
        gaps: vec![],
        refrains: vec![],
        index: vec![],
        html_gaps: std::collections::BTreeMap::new(),
        end_matter: vec![],
    });
    assert_eq!(markdown::check_math_admission(&plan), Ok(()));
}

#[test]
fn citation_fixture_plans_retain_real_checked_mint_origins() {
    use ftoo_manifest::*;
    let graph = seed().unwrap();
    let book = reader_fixture_book(&graph);
    let plan = reader_citation_plan(&book, "dec:weld:equal-address-sets");
    let plan::Inline::Citation { identity, site, .. } = &plan.blocks[0].runs[1] else {
        panic!("citation")
    };
    assert_eq!(
        identity,
        &CitationIdentity {
            label: "dec:weld:equal-address-sets".into(),
            resolved: Some(AddressTarget::Documentary(
                "dec:weld:equal-address-sets".into()
            )),
            handle: "dec:weld".into(),
            token: "u0-c0".into(),
            position: ReaderIdentityPosition::Citation {
                unit: "sec:fixture:citation".into(),
                span: 0,
                target: None
            },
            mint: Some(MintOrigin {
                kind: MintKind::Markdown,
                site: LabelSite::Markdown {
                    file: "docs/adr/012-the-weld.md".into(),
                    bytes: 117..149
                }
            })
        }
    );
    assert_eq!(
        site,
        &UnitSite {
            decoded: Some(4..33),
            ..input::site("fixture.toml")
        }
    );
    assert_eq!(
        plan::check_emission(&plan, &latex::render(&plan), DocumentTarget::Latex),
        Ok(())
    );
    assert_eq!(
        plan::check_emission(&plan, &markdown::render(&plan), DocumentTarget::Markdown),
        Ok(())
    );
}
#[test]
fn release_destination_bridge_refuses_entries_and_gaps_independently() {
    let mut prepared = PreparedDocuments {
        snapshot: None,
        artifacts: Vec::new(),
        html_gap_rows: Vec::new(),
        screen_gap_rows: Vec::new(),
        inputs: BTreeMap::new(),
        observed_outputs: BTreeMap::new(),
        unavailable: Vec::new(),
    };
    prepared.unavailable.push(destinations::IndexEntry {
        document: destinations::Reading::Paper,
        language: "en".into(),
        fragment: "method-self-limit".into(),
        title: "The method’s self-limit".into(),
        locator: "appendix G".into(),
        message: "Not yet available in this rendering.",
    });
    assert!(
        prepared
            .check_release_destinations()
            .unwrap_err()
            .contains("entries: 1, gaps: false")
    );
    prepared.unavailable.clear();
    prepared.screen_gap_rows.push(ScreenGapRow {
        document: "paper-pilot".into(),
        language: "en".into(),
        position: 1,
        before_unit: Some("sec:first".into()),
        after_unit: Some("sec:second".into()),
        message: "Intervening content is unavailable.".into(),
    });
    assert!(
        prepared
            .check_release_destinations()
            .unwrap_err()
            .contains("entries: 0, gaps: true")
    );
    prepared.screen_gap_rows.clear();
    assert_eq!(prepared.check_release_destinations(), Ok(()));
}
