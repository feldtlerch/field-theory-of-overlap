use super::*;

#[test]
fn support_required_literal_historical_body_under_current_contracts_and_pending_paragraphs()
-> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    let old = include_str!("../../../tests/fixtures/native/historical-en.tex");
    assert_eq!(
        format!("{:x}", Sha256::digest(old.as_bytes())),
        "b0e2d86f4052058e5dab8d26fc50b87aafc2e24dc6518a7f19ebc2ebbd5de338"
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let projection = fs::read(root.join("generated/manifest.json"))?;
    let loaded = ftoo_manifest::load_projection(&projection)?;
    let book = loaded.addressing().unwrap();
    let declaration = crate::UnitSource {
        file: "units/documents.toml".into(),
        bytes: input::read(&root, "units/documents.toml")?,
    };
    let statements = input::sources(&root, &input::discover(&root, "units/statements", false)?)?;
    let corpus = crate::load_unit_sources(declaration, statements.clone()).unwrap();
    let markdown = crate::load_markdown_files(&root).unwrap();
    let joint = ftoo_manifest::collect_joint_mints(&corpus, &markdown).unwrap();
    let book = book
        .with_documentary(
            &joint,
            registry::load(&registry::read(&root).unwrap()).unwrap(),
        )
        .unwrap();
    let resolved = ftoo_manifest::resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = ftoo_manifest::integrate_joint_labels(&resolved, &joint).unwrap();
    let welded = ftoo_manifest::weld_units(&integrated).unwrap();
    let sources = input::sources(&root, &input::discover(&root, "units/skeletons", false)?)?;
    let displays = input::sources(&root, &input::discover(&root, "units/displays", true)?)?;
    let (drafts, display_drafts) = input::assembly_sources(&sources, &displays)?;
    let (metadata, owners) = super::super::metadata(&statements);
    let document_sites = super::super::document_sites(&corpus);
    let skeletons =
        ftoo_manifest::check_skeletons(&corpus, &drafts, &metadata, &owners, &document_sites)
            .unwrap();
    let displays = ftoo_manifest::check_displays(&display_drafts, &skeletons, &welded).unwrap();
    let catalogue = ftoo_manifest::notation(&book).unwrap();
    let directory = Directory::new();
    let path = directory.0.join("historical.tex");
    crate::notation::with_preparation(&catalogue, |notation| -> Result<(), DocumentError> {
        plan::citation_tests::support_required_literal_build_controls(notation);
        super::pending::support_required_pending_paragraphs_are_complete_label_neutral_and_independently_consumed(&book, notation)?;
        let plans = plan::build(&skeletons, &welded, notation, &metadata, &displays)?;
        let plan = plans
            .iter()
            .find(|plan| plan.document == "main" && plan.language == "en")
            .unwrap();
        let records = plan.records(DocumentTarget::Latex);
        let error = |bytes, missing, changed| {
            failure(
                RuleId::M055,
                AssemblyStage::Emission,
                AssemblySite::Generated {
                    path: "generated/documents/main/en.tex".into(),
                    bytes,
                    origins: plan.origins(DocumentTarget::Latex),
                },
                AssemblyFault::EmissionMismatch {
                    missing,
                    unexpected: vec![],
                    changed,
                },
            )
        };
        // The literal historical file also predates the current fixed envelope.
        // Envelope rejection precedes body recovery, so the complete missing
        // inventory at EOF is the most specific unchanged-byte refusal.
        fs::write(&path, old).unwrap();
        let reread = fs::read_to_string(&path).unwrap();
        assert_eq!(reread, old);
        assert_eq!(
            plan::check_emission(plan, &reread, DocumentTarget::Latex),
            Err(error(old.len()..old.len(), records.clone(), vec![]))
        );

        let modern = latex::render(plan);
        // Retain historical body bytes, including the two endangered spaces.
        // Only the later envelope, live-citation and pending-paragraph contracts
        // are supplied, so their independent refusals cannot mask the spacing.
        let body = &old[old.find("% unit ").unwrap()..];
        let body = body.replace(
            r"\texttt{dec:weld:equal-address-sets}",
            r"\FtdocTag{dec:weld}{u2-c0}",
        );
        let body = body.replace("\n\n% unit sec:coefficients:value-table", "\n\nPending notation: \\({c}_{1}\\): retained formulation \\FtdocTag{c:1}{u0-p0-r0}% label def:langevin:coefficient-one\n.\n\n% unit sec:coefficients:value-table");
        let candidate = format!("{}{body}", &modern[..modern.find("% unit ").unwrap()]);
        let changed = [
            (
                "sec:coefficients:value-context",
                "@tag{v:1} concerns",
                "@tag{v:1}concerns",
            ),
            (
                "sec:coefficients:first-value",
                "@tag{c:1} has",
                "@tag{c:1}has",
            ),
        ]
        .into_iter()
        .map(|(unit, before, after)| {
            let expected = records
                .iter()
                .find(|record| record.unit == unit && record.kind == EmissionKind::Paragraph)
                .unwrap()
                .clone();
            assert!(expected.value.contains(before));
            let mut actual = expected.clone();
            actual.value = actual.value.replacen(before, after, 1);
            EmissionChange { expected, actual }
        })
        .collect();
        let start = candidate
            .find("% unit sec:coefficients:value-context ")
            .unwrap();
        let end = candidate
            .find("% unit sec:coefficients:first-value ")
            .unwrap();
        let expected = error(start..end, vec![], changed);
        fs::write(&path, &candidate).unwrap();
        let actual = plan::check_emission(
            plan,
            &fs::read_to_string(&path).unwrap(),
            DocumentTarget::Latex,
        )
        .unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), expected.to_string());
        fs::write(&path, &modern).unwrap();
        let restored = fs::read_to_string(&path).unwrap();
        assert_eq!(restored, modern);
        assert_eq!(
            plan::check_emission(plan, &restored, DocumentTarget::Latex),
            Ok(())
        );
        Ok(())
    })?;
    Ok(())
}
