use crate::{
    diagnostics::{ReadingDiagnostic, ReadingErrors, ReadingFault, ReadingStage},
    explainer,
    identity::{
        ReadingDocument, ReadingId, ReadingKey, ReadingPayload, ReadingRole, ReadingSpec, key,
    },
    paper,
};
use ftoo_manifest::{READ_KIND, RuleId, is_semantic_label};
use std::collections::{BTreeMap, HashMap, HashSet};

/// One checked occurrence declaration, without a place in a document outline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadingDeclaration {
    key: ReadingKey,
    payload: ReadingPayload,
}

impl ReadingDeclaration {
    #[must_use]
    pub fn key(&self) -> ReadingKey {
        self.key
    }

    #[must_use]
    pub fn document(&self) -> ReadingDocument {
        self.key.0.document()
    }

    #[must_use]
    pub fn role(&self) -> ReadingRole {
        self.payload.role()
    }

    #[must_use]
    pub fn owned_unit(&self) -> Option<&str> {
        match self.payload {
            ReadingPayload::Refrain { unit } => Some(unit),
            ReadingPayload::Quotation { .. } => None,
        }
    }

    #[must_use]
    pub fn quotation_owner(&self) -> Option<ReadingKey> {
        match self.payload {
            ReadingPayload::Refrain { .. } => None,
            ReadingPayload::Quotation { owner } => Some(key(owner)),
        }
    }
}

/// Checked membership; the slices are sorted by semantic name, not reading order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedReadingStructure {
    declarations: Vec<ReadingDeclaration>,
    allocations: Vec<ReadingKey>,
}

impl CheckedReadingStructure {
    #[must_use]
    pub fn declarations(&self) -> &[ReadingDeclaration] {
        &self.declarations
    }

    #[must_use]
    pub fn allocations(&self) -> &[ReadingKey] {
        &self.allocations
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Allocation {
    pub(crate) id: ReadingId,
    spec: ReadingSpec,
    document_index: usize,
}

#[derive(Default)]
pub(crate) struct ReadingBuilder {
    pub(crate) allocations: Vec<Allocation>,
    paper_count: usize,
    explainer_count: usize,
}

impl ReadingBuilder {
    pub(crate) fn allocate(&mut self, id: ReadingId) {
        self.push(id, id.spec());
    }

    pub(crate) fn push(&mut self, id: ReadingId, spec: ReadingSpec) {
        let document_index = match id.document() {
            ReadingDocument::Paper => {
                let index = self.paper_count;
                self.paper_count += 1;
                index
            }
            ReadingDocument::Explainer => {
                let index = self.explainer_count;
                self.explainer_count += 1;
                index
            }
        };
        self.allocations.push(Allocation {
            id,
            spec,
            document_index,
        });
    }

    pub(crate) fn finish(self) -> Result<CheckedReadingStructure, ReadingErrors> {
        let mut diagnostics = Vec::new();
        let mut by_id: HashMap<ReadingId, Vec<Allocation>> = HashMap::new();
        let mut by_name: BTreeMap<&str, Vec<Allocation>> = BTreeMap::new();
        for allocation in &self.allocations {
            if !is_semantic_label(allocation.spec.name)
                || allocation.spec.name.split(':').next() != Some(READ_KIND)
            {
                diagnostics.push(diagnostic(
                    allocation,
                    ReadingStage::Name,
                    "$.name",
                    None,
                    ReadingFault::InvalidName {
                        value: allocation.spec.name.to_owned(),
                    },
                ));
            }
            by_id.entry(allocation.id).or_default().push(*allocation);
            by_name
                .entry(allocation.spec.name)
                .or_default()
                .push(*allocation);
        }

        check_uniqueness(&by_id, by_name, &mut diagnostics);
        check_ownership_and_facts(&self.allocations, &by_id, &mut diagnostics);

        if let Some(errors) = ReadingErrors::from_diagnostics(diagnostics) {
            return Err(errors);
        }
        let mut declarations: Vec<_> = self
            .allocations
            .into_iter()
            .map(|allocation| ReadingDeclaration {
                key: key(allocation.id),
                payload: allocation.spec.payload,
            })
            .collect();
        declarations.sort_by_key(|declaration| declaration.key.semantic_name());
        let allocations = declarations.iter().map(ReadingDeclaration::key).collect();
        Ok(CheckedReadingStructure {
            declarations,
            allocations,
        })
    }
}

fn check_uniqueness(
    by_id: &HashMap<ReadingId, Vec<Allocation>>,
    by_name: BTreeMap<&str, Vec<Allocation>>,
    diagnostics: &mut Vec<ReadingDiagnostic>,
) {
    for occurrences in by_id.values().filter(|items| items.len() > 1) {
        let allocation = occurrences[1];
        diagnostics.push(diagnostic(
            &allocation,
            ReadingStage::Uniqueness,
            "$",
            Some(allocation.document_index),
            ReadingFault::DuplicateIdentity {
                count: occurrences.len(),
            },
        ));
    }
    for (name, occurrences) in by_name {
        let mut distinct = HashSet::new();
        let mut members: Vec<_> = occurrences
            .into_iter()
            .filter(|allocation| distinct.insert(allocation.id))
            .collect();
        if members.len() > 1 {
            members.sort_by_key(|allocation| format!("{:?}", allocation.id));
            let identities = members
                .iter()
                .map(|allocation| format!("{:?}", allocation.id))
                .collect();
            diagnostics.push(diagnostic(
                &members[0],
                ReadingStage::Uniqueness,
                "$.name",
                None,
                ReadingFault::DuplicateName {
                    name: name.to_owned(),
                    identities,
                },
            ));
        }
    }
}

fn check_ownership_and_facts(
    allocations: &[Allocation],
    by_id: &HashMap<ReadingId, Vec<Allocation>>,
    diagnostics: &mut Vec<ReadingDiagnostic>,
) {
    for allocation in allocations {
        if let ReadingPayload::Quotation { owner } = allocation.spec.payload {
            match by_id.get(&owner).map(Vec::as_slice) {
                None | Some([]) => diagnostics.push(diagnostic(
                    allocation,
                    ReadingStage::Ownership,
                    "$.body.owner",
                    None,
                    ReadingFault::MissingOwner {
                        owner: owner.spec().name.to_owned(),
                    },
                )),
                Some([owner_allocation]) => {
                    if owner.document() == allocation.id.document()
                        || !matches!(
                            owner_allocation.spec.payload,
                            ReadingPayload::Refrain { .. }
                        )
                    {
                        diagnostics.push(diagnostic(
                            allocation,
                            ReadingStage::Ownership,
                            "$.body.owner",
                            None,
                            ReadingFault::WrongOwner {
                                owner: owner.spec().name.to_owned(),
                            },
                        ));
                    }
                }
                Some(_) => {}
            }
        }
        let expected = allocation.id.spec();
        if allocation.spec != expected {
            diagnostics.push(diagnostic(
                allocation,
                ReadingStage::FixedFacts,
                "$",
                None,
                ReadingFault::FixedFactsMismatch {
                    expected: description(expected),
                    actual: description(allocation.spec),
                },
            ));
        }
    }
}

fn diagnostic(
    allocation: &Allocation,
    stage: ReadingStage,
    field: &str,
    index: Option<usize>,
    fault: ReadingFault,
) -> ReadingDiagnostic {
    ReadingDiagnostic {
        rule: RuleId::M074,
        stage,
        identity: allocation.id.spec().name.to_owned(),
        source: allocation.id.source().to_owned(),
        field: field.to_owned(),
        index,
        fault,
    }
}

fn description(spec: ReadingSpec) -> String {
    let (role, unit, owner) = match spec.payload {
        ReadingPayload::Refrain { unit } => ("refrain", format!("{unit:?}"), "null".to_owned()),
        ReadingPayload::Quotation { owner } => (
            "quotation",
            "null".to_owned(),
            format!("{:?}", owner.spec().name),
        ),
    };
    format!("name={:?};role={role};unit={unit};owner={owner}", spec.name)
}

/// Build the present checked catalogue from closed code declarations.
///
/// # Errors
///
/// Returns every located name, uniqueness, ownership and fixed-fact refusal.
pub fn catalogue() -> Result<CheckedReadingStructure, ReadingErrors> {
    let mut builder = ReadingBuilder::default();
    paper::populate(&mut builder);
    explainer::populate(&mut builder);
    builder.finish()
}
