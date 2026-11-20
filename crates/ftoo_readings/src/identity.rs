use crate::{explainer::ExplainerReadingId, paper::PaperReadingId};

/// The document in which an occurrence is allocated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadingDocument {
    Paper,
    Explainer,
}

/// The occurrence's ownership role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadingRole {
    Refrain,
    Quotation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum ReadingId {
    Paper(PaperReadingId),
    Explainer(ExplainerReadingId),
}

impl ReadingId {
    pub(crate) fn spec(self) -> ReadingSpec {
        match self {
            Self::Paper(id) => id.spec(),
            Self::Explainer(id) => id.spec(),
        }
    }

    pub(crate) fn document(self) -> ReadingDocument {
        match self {
            Self::Paper(_) => ReadingDocument::Paper,
            Self::Explainer(_) => ReadingDocument::Explainer,
        }
    }

    pub(crate) fn source(self) -> &'static str {
        match self {
            Self::Paper(_) => "crates/ftoo_readings/src/paper.rs",
            Self::Explainer(_) => "crates/ftoo_readings/src/explainer.rs",
        }
    }
}

/// A checked identity with no constructor from untrusted text.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ReadingKey(pub(crate) ReadingId);

impl ReadingKey {
    /// Return the stable semantic name declared for this identity.
    #[must_use]
    pub fn semantic_name(self) -> &'static str {
        self.0.spec().name
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReadingSpec {
    pub(crate) name: &'static str,
    pub(crate) payload: ReadingPayload,
}

impl ReadingSpec {
    pub(crate) const fn new(name: &'static str, payload: ReadingPayload) -> Self {
        Self { name, payload }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReadingPayload {
    Refrain { unit: &'static str },
    Quotation { owner: ReadingId },
}

impl ReadingPayload {
    pub(crate) fn role(self) -> ReadingRole {
        match self {
            Self::Refrain { .. } => ReadingRole::Refrain,
            Self::Quotation { .. } => ReadingRole::Quotation,
        }
    }
}

pub(crate) fn key(id: ReadingId) -> ReadingKey {
    ReadingKey(id)
}
