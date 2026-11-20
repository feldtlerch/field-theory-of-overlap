use ftoo_manifest::RuleId;
use std::{error::Error, fmt};

/// The validation phase that located a reading fault.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadingStage {
    Name,
    Uniqueness,
    Ownership,
    FixedFacts,
}

impl ReadingStage {
    fn rank(self) -> u8 {
        match self {
            Self::Name => 0,
            Self::Uniqueness => 1,
            Self::Ownership => 2,
            Self::FixedFacts => 3,
        }
    }
}

/// A precise failure of constructed reading declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadingFault {
    InvalidName {
        value: String,
    },
    DuplicateIdentity {
        count: usize,
    },
    DuplicateName {
        name: String,
        identities: Vec<String>,
    },
    MissingOwner {
        owner: String,
    },
    WrongOwner {
        owner: String,
    },
    FixedFactsMismatch {
        expected: String,
        actual: String,
    },
}

impl ReadingFault {
    fn rank(&self) -> u8 {
        match self {
            Self::InvalidName { .. } => 0,
            Self::DuplicateIdentity { .. } => 1,
            Self::DuplicateName { .. } => 2,
            Self::MissingOwner { .. } => 3,
            Self::WrongOwner { .. } => 4,
            Self::FixedFactsMismatch { .. } => 5,
        }
    }
}

/// One located refusal under the reading-construction rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadingDiagnostic {
    pub rule: RuleId,
    pub stage: ReadingStage,
    pub identity: String,
    pub source: String,
    pub field: String,
    pub index: Option<usize>,
    pub fault: ReadingFault,
}

/// Nonempty, canonically ordered reading-construction refusals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadingErrors {
    diagnostics: Vec<ReadingDiagnostic>,
}

impl ReadingErrors {
    /// Sort a nonempty diagnostic set without admitting a reading structure.
    #[must_use]
    pub fn from_diagnostics(mut diagnostics: Vec<ReadingDiagnostic>) -> Option<Self> {
        if diagnostics.is_empty() {
            return None;
        }
        diagnostics.sort_by(|left, right| {
            (
                left.stage.rank(),
                left.identity.as_str(),
                left.source.as_str(),
                left.field.as_str(),
                left.index,
                left.fault.rank(),
            )
                .cmp(&(
                    right.stage.rank(),
                    right.identity.as_str(),
                    right.source.as_str(),
                    right.field.as_str(),
                    right.index,
                    right.fault.rank(),
                ))
        });
        Some(Self { diagnostics })
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[ReadingDiagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for ReadingErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (at, diagnostic) in self.diagnostics.iter().enumerate() {
            if at != 0 {
                writeln!(f)?;
            }
            write!(
                f,
                "{:?} {:?} {} {} {} {:?} {:?}",
                diagnostic.rule,
                diagnostic.stage,
                diagnostic.identity,
                diagnostic.source,
                diagnostic.field,
                diagnostic.index,
                diagnostic.fault
            )?;
        }
        Ok(())
    }
}

impl Error for ReadingErrors {}
