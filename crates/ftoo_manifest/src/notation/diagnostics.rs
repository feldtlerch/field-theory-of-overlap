use super::{Callee, DefinitionId, PrimitiveId};
use crate::{NodeId, NodeKind, RuleId};
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NotationStage {
    Catalogue,
    References,
    Ast,
    Dependencies,
    Names,
    Typesetting,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderOwner {
    Definition(DefinitionId),
    Primitive(PrimitiveId),
    Expression,
}
impl RenderOwner {
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Definition(id) => id.token(),
            Self::Primitive(id) => id.token(),
            Self::Expression => "",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderTarget {
    Macro,
    Unicode,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryEntry {
    Primitive(PrimitiveId),
    Definition(DefinitionId),
}
impl RegistryEntry {
    pub(super) fn token(self) -> &'static str {
        match self {
            Self::Primitive(id) => id.token(),
            Self::Definition(id) => id.token(),
        }
    }
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NotationPathPart {
    Field(String),
    Argument(usize),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotationSite {
    Catalogue {
        file: String,
    },
    Registry {
        file: String,
        entry: RegistryEntry,
        index: usize,
        path: Vec<NotationPathPart>,
    },
    Expression {
        path: Vec<NotationPathPart>,
    },
    Rendered {
        owner: RenderOwner,
        case: String,
        bytes: std::ops::Range<usize>,
        origins: Vec<NotationOrigin>,
    },
}
impl NotationSite {
    pub(super) fn child(&self, part: NotationPathPart) -> Self {
        let mut site = self.clone();
        match &mut site {
            Self::Registry { path, .. } | Self::Expression { path } => path.push(part),
            Self::Catalogue { .. } | Self::Rendered { .. } => unreachable!("AST sites have paths"),
        }
        site
    }
    pub(super) fn field(&self, field: &str) -> Self {
        self.child(NotationPathPart::Field(field.into()))
    }
    fn key(&self) -> (&str, &[NotationPathPart], usize) {
        match self {
            Self::Registry {
                entry, index, path, ..
            } => (entry.token(), path, *index),
            Self::Expression { path } => ("", path, 0),
            Self::Catalogue { .. } => ("", &[], 0),
            Self::Rendered {
                owner,
                origins,
                bytes,
                ..
            } => (
                owner.token(),
                origins.first().map_or(&[], |o| o.path.as_slice()),
                bytes.start,
            ),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotationOrigin {
    pub definition: Option<DefinitionId>,
    pub path: Vec<NotationPathPart>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DefinitionShapeReason {
    CanonicalMustBeNullary,
    AliasMustBeNullary,
    ConstructorArity,
    BackingObjectMismatch { expected: NodeId, actual: NodeId },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AliasTargetReason {
    ConstructorTarget,
    ObjectMismatch { expected: NodeId, actual: NodeId },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralReason {
    Empty,
    MultipleLetters,
    ForbiddenScalar,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotationResource {
    Depth,
    AstNodes,
    LiteralBytes,
    ExpansionNodes,
    OutputBytes,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotationFault {
    CatalogMultiplicity {
        entry: RegistryEntry,
        expected: usize,
        actual: usize,
        positions: Vec<usize>,
    },
    DuplicateToken {
        token: String,
        entries: Vec<RegistryEntry>,
        positions: Vec<usize>,
    },
    PrimitiveSignature {
        primitive: PrimitiveId,
        expected: u8,
        actual: u8,
    },
    DefinitionShape {
        definition: DefinitionId,
        reason: DefinitionShapeReason,
    },
    CanonicalMultiplicity {
        object: NodeId,
        expected: usize,
        actual: usize,
        definitions: Vec<DefinitionId>,
        positions: Vec<usize>,
    },
    AliasTarget {
        alias: DefinitionId,
        target: DefinitionId,
        reason: AliasTargetReason,
    },
    InvalidLiteral {
        value: String,
        reason: LiteralReason,
        byte: usize,
        scalar: Option<String>,
    },
    SlotOutOfScope {
        index: u8,
        arity: u8,
    },
    UnusedSlot {
        index: u8,
        arity: u8,
    },
    CallArity {
        callee: Callee,
        expected: u8,
        actual: usize,
    },
    NotationLimit {
        resource: NotationResource,
        limit: usize,
        actual: usize,
    },
    DefinitionCycle {
        chain: Vec<DefinitionId>,
        sites: Vec<NotationSite>,
    },
    InvalidStem {
        value: String,
    },
    CommandCollision {
        command: String,
        definitions: Vec<DefinitionId>,
    },
    HostCollision {
        command: String,
        message: String,
    },
    IncompleteExpansion {
        remaining: String,
        origin: NotationOrigin,
    },
    KaTeXRejected {
        target: RenderTarget,
        source: String,
        message: String,
    },
    SweepInventory {
        missing: Vec<DefinitionId>,
        repeated: Vec<(DefinitionId, usize)>,
    },
    SweepCommand {
        definition: DefinitionId,
        expected: String,
        actual: String,
    },
    SweepSignature {
        definition: DefinitionId,
        expected: u8,
        actual: u8,
    },
}
impl NotationFault {
    fn rank(&self) -> u8 {
        match self {
            Self::CatalogMultiplicity { .. } => 0,
            Self::DuplicateToken { .. } => 1,
            Self::PrimitiveSignature { .. } => 2,
            Self::DefinitionShape { .. } => 3,
            Self::CanonicalMultiplicity { .. } => 4,
            Self::AliasTarget { .. } => 5,
            Self::InvalidLiteral { .. } => 6,
            Self::SlotOutOfScope { .. } => 7,
            Self::UnusedSlot { .. } => 8,
            Self::CallArity { .. } => 9,
            Self::NotationLimit { .. } => 10,
            Self::DefinitionCycle { .. } => 11,
            Self::InvalidStem { .. } => 12,
            Self::CommandCollision { .. } => 13,
            Self::HostCollision { .. } => 14,
            Self::IncompleteExpansion { .. } => 15,
            Self::KaTeXRejected { .. } => 16,
            Self::SweepInventory { .. } => 17,
            Self::SweepCommand { .. } => 18,
            Self::SweepSignature { .. } => 19,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotationDiagnostic {
    pub rule: RuleId,
    pub stage: NotationStage,
    pub site: NotationSite,
    pub fault: NotationFault,
}
impl fmt::Display for NotationFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl fmt::Display for NotationDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {:?} at {:?}: {}",
            self.rule, self.stage, self.site, self.fault
        )
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotationErrors {
    diagnostics: Vec<NotationDiagnostic>,
}
impl NotationErrors {
    #[must_use]
    pub fn from_diagnostics(mut diagnostics: Vec<NotationDiagnostic>) -> Option<Self> {
        if diagnostics.is_empty() {
            return None;
        }
        diagnostics.sort_by(|a, b| {
            (a.rule, a.site.key(), a.fault.rank()).cmp(&(b.rule, b.site.key(), b.fault.rank()))
        });
        Some(Self { diagnostics })
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[NotationDiagnostic] {
        &self.diagnostics
    }
}
impl fmt::Display for NotationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diagnostic in &self.diagnostics {
            writeln!(f, "{diagnostic}")?;
        }
        Ok(())
    }
}
impl Error for NotationErrors {}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SymbolLookupFault {
    NotQuantity { object: NodeId, actual: NodeKind },
    AbsentObject { object: NodeId },
}
impl fmt::Display for SymbolLookupFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for SymbolLookupFault {}

pub(super) fn finish(errors: Vec<NotationDiagnostic>) -> Result<(), NotationErrors> {
    match NotationErrors::from_diagnostics(errors) {
        Some(errors) => Err(errors),
        None => Ok(()),
    }
}
