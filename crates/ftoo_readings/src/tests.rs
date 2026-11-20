use crate::{
    construction::{ReadingBuilder, catalogue},
    diagnostics::{ReadingDiagnostic, ReadingFault, ReadingStage},
    explainer::{self, ExplainerReadingId},
    identity::{ReadingDocument, ReadingId, ReadingPayload, ReadingRole, ReadingSpec},
    paper::{self, PaperReadingId},
};
use ftoo_manifest::RuleId;

fn diagnostic(
    stage: ReadingStage,
    identity: &str,
    field: &str,
    index: Option<usize>,
    fault: ReadingFault,
) -> ReadingDiagnostic {
    ReadingDiagnostic {
        rule: RuleId::M074,
        stage,
        identity: identity.to_owned(),
        source: if identity.starts_with("read:paper:") {
            "crates/ftoo_readings/src/paper.rs"
        } else {
            "crates/ftoo_readings/src/explainer.rs"
        }
        .to_owned(),
        field: field.to_owned(),
        index,
        fault,
    }
}

fn replaced(id: ReadingId, spec: Option<ReadingSpec>) -> ReadingBuilder {
    let mut builder = ReadingBuilder::default();
    paper::populate(&mut builder);
    explainer::populate(&mut builder);
    builder.allocations.retain(|allocation| allocation.id != id);
    if let Some(spec) = spec {
        builder.push(id, spec);
    }
    builder
}

fn assert_faults(builder: ReadingBuilder, expected: &[ReadingDiagnostic]) {
    let Err(errors) = builder.finish() else {
        panic!("invalid fixture was accepted");
    };
    assert_eq!(errors.diagnostics(), expected);
}

#[test]
fn reading_fixed_facts_and_construction() -> Result<(), Box<dyn std::error::Error>> {
    assert_catalogue_membership()?;
    let paper_direction = ReadingId::Paper(PaperReadingId::Direction);
    let expected_direction =
        "name=\"read:paper:direction\";role=refrain;unit=\"sec:say:direction\";owner=null";
    case_invalid_name(paper_direction, expected_direction);

    let mut duplicate_identity = ReadingBuilder::default();
    paper::populate(&mut duplicate_identity);
    duplicate_identity.allocate(paper_direction);
    explainer::populate(&mut duplicate_identity);
    assert_faults(
        duplicate_identity,
        &[diagnostic(
            ReadingStage::Uniqueness,
            "read:paper:direction",
            "$",
            Some(5),
            ReadingFault::DuplicateIdentity { count: 2 },
        )],
    );

    let harm = ReadingId::Paper(PaperReadingId::HarmEnlargement);
    let duplicate_name = ReadingSpec::new(
        "read:paper:direction",
        ReadingPayload::Refrain {
            unit: "sec:say:harm-enlargement",
        },
    );
    assert_faults(replaced(harm, Some(duplicate_name)), &[
        diagnostic(ReadingStage::Uniqueness, "read:paper:direction", "$.name", None,
            ReadingFault::DuplicateName {
                name: "read:paper:direction".to_owned(),
                identities: vec!["Paper(Direction)".to_owned(), "Paper(HarmEnlargement)".to_owned()],
            }),
        diagnostic(ReadingStage::FixedFacts, "read:paper:harm-enlargement", "$", None,
            ReadingFault::FixedFactsMismatch {
                expected: "name=\"read:paper:harm-enlargement\";role=refrain;unit=\"sec:say:harm-enlargement\";owner=null".to_owned(),
                actual: "name=\"read:paper:direction\";role=refrain;unit=\"sec:say:harm-enlargement\";owner=null".to_owned(),
            }),
    ]);

    assert_faults(
        replaced(paper_direction, None),
        &[diagnostic(
            ReadingStage::Ownership,
            "read:explainer:direction",
            "$.body.owner",
            None,
            ReadingFault::MissingOwner {
                owner: "read:paper:direction".to_owned(),
            },
        )],
    );

    let explainer_direction = ReadingId::Explainer(ExplainerReadingId::Direction);
    let wrong_owner = ReadingSpec::new(
        "read:explainer:direction",
        ReadingPayload::Quotation {
            owner: ReadingId::Explainer(ExplainerReadingId::EnemyToNone),
        },
    );
    assert_faults(replaced(explainer_direction, Some(wrong_owner)), &[
        diagnostic(ReadingStage::Ownership, "read:explainer:direction", "$.body.owner", None,
            ReadingFault::WrongOwner { owner: "read:explainer:enemy-to-none".to_owned() }),
        diagnostic(ReadingStage::FixedFacts, "read:explainer:direction", "$", None,
            ReadingFault::FixedFactsMismatch {
                expected: "name=\"read:explainer:direction\";role=quotation;unit=null;owner=\"read:paper:direction\"".to_owned(),
                actual: "name=\"read:explainer:direction\";role=quotation;unit=null;owner=\"read:explainer:enemy-to-none\"".to_owned(),
            }),
    ]);

    let wrong_unit = ReadingSpec::new(
        "read:paper:direction",
        ReadingPayload::Refrain {
            unit: "sec:say:harm-enlargement",
        },
    );
    assert_faults(replaced(paper_direction, Some(wrong_unit)), &[diagnostic(
        ReadingStage::FixedFacts, "read:paper:direction", "$", None,
        ReadingFault::FixedFactsMismatch {
            expected: expected_direction.to_owned(),
            actual: "name=\"read:paper:direction\";role=refrain;unit=\"sec:say:harm-enlargement\";owner=null".to_owned(),
        },
    )]);
    Ok(())
}

fn assert_catalogue_membership() -> Result<(), Box<dyn std::error::Error>> {
    let structure = catalogue()?;
    let declarations: Vec<_> = structure
        .declarations()
        .iter()
        .map(|entry| {
            (
                entry.key().semantic_name(),
                entry.document(),
                entry.role(),
                entry.owned_unit(),
                entry
                    .quotation_owner()
                    .map(crate::ReadingKey::semantic_name),
            )
        })
        .collect();
    assert_eq!(
        declarations,
        [
            (
                "read:explainer:direction",
                ReadingDocument::Explainer,
                ReadingRole::Quotation,
                None,
                Some("read:paper:direction")
            ),
            (
                "read:explainer:enemy-to-none",
                ReadingDocument::Explainer,
                ReadingRole::Quotation,
                None,
                Some("read:paper:enemy-to-none")
            ),
            (
                "read:explainer:record-limit",
                ReadingDocument::Explainer,
                ReadingRole::Quotation,
                None,
                Some("read:paper:record-limit")
            ),
            (
                "read:paper:direction",
                ReadingDocument::Paper,
                ReadingRole::Refrain,
                Some("sec:say:direction"),
                None
            ),
            (
                "read:paper:enemy-to-none",
                ReadingDocument::Paper,
                ReadingRole::Refrain,
                Some("sec:say:enemy-to-none"),
                None
            ),
            (
                "read:paper:harm-enlargement",
                ReadingDocument::Paper,
                ReadingRole::Refrain,
                Some("sec:say:harm-enlargement"),
                None
            ),
            (
                "read:paper:record-limit",
                ReadingDocument::Paper,
                ReadingRole::Refrain,
                Some("sec:coda:record-limit"),
                None
            ),
            (
                "read:paper:wonder-beauty",
                ReadingDocument::Paper,
                ReadingRole::Refrain,
                Some("sec:say:wonder-beauty"),
                None
            ),
        ]
    );
    assert_eq!(
        structure
            .allocations()
            .iter()
            .map(|key| key.semantic_name())
            .collect::<Vec<_>>(),
        declarations.iter().map(|item| item.0).collect::<Vec<_>>()
    );

    Ok(())
}

fn case_invalid_name(paper_direction: ReadingId, expected_direction: &str) {
    let invalid_name = ReadingSpec::new(
        "read:paper:Direction",
        ReadingPayload::Refrain {
            unit: "sec:say:direction",
        },
    );
    assert_faults(
        replaced(paper_direction, Some(invalid_name)),
        &[
            diagnostic(ReadingStage::Name, "read:paper:direction", "$.name", None,
                ReadingFault::InvalidName { value: "read:paper:Direction".to_owned() }),
            diagnostic(ReadingStage::FixedFacts, "read:paper:direction", "$", None,
                ReadingFault::FixedFactsMismatch {
                    expected: expected_direction.to_owned(),
                    actual: "name=\"read:paper:Direction\";role=refrain;unit=\"sec:say:direction\";owner=null".to_owned(),
                }),
        ],
    );
}
